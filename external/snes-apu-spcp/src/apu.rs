use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};
use crate::smp::Smp;
use crate::timer::Timer;
use crate::dsp::dsp::Dsp;
use crate::script700::runtime::Runtime;
use spc_spcp::spc::{Spc, RAM_LEN, IPL_ROM_LEN};
use crate::ResamplingMode;

#[derive(Copy, Clone, Default, Debug)]
pub struct ApuChannelState {
    pub source: u8,
    pub muted: bool,
    pub envelope_level: i32,
    pub volume: (i8, i8),
    pub amplitude: (i32, i32),
    pub pitch: u16,
    pub noise_clock: Option<u8>,
    pub edge: bool,
    pub kon_frames: usize,
    pub sample_block_index: usize,
    pub echo_delay: Option<u8>,
    pub pitch_modulation: bool
}

#[derive(Copy, Clone, Default, Debug)]
pub struct ApuMasterState {
    pub master_volume: (i8, i8),
    pub echo_volume: (i8, i8),
    pub echo_delay: u8,
    pub echo_feedback: i8,
    pub amplitude: (i32, i32)
}

pub trait ApuStateReceiver {
    fn receive(&mut self, channel: usize, state: ApuChannelState);
    fn receive_master(&mut self, state: ApuMasterState);
}

static DEFAULT_IPL_ROM: [u8; IPL_ROM_LEN] = [
    0xcd, 0xef, 0xbd, 0xe8, 0x00, 0xc6, 0x1d, 0xd0,
    0xfc, 0x8f, 0xaa, 0xf4, 0x8f, 0xbb, 0xf5, 0x78,
    0xcc, 0xf4, 0xd0, 0xfb, 0x2f, 0x19, 0xeb, 0xf4,
    0xd0, 0xfc, 0x7e, 0xf4, 0xd0, 0x0b, 0xe4, 0xf5,
    0xcb, 0xf4, 0xd7, 0x00, 0xfc, 0xd0, 0xf3, 0xab,
    0x01, 0x10, 0xef, 0x7e, 0xf4, 0x10, 0xeb, 0xba,
    0xf6, 0xda, 0x00, 0xba, 0xf4, 0xc4, 0xf4, 0xdd,
    0x5d, 0xd0, 0xdb, 0x1f, 0x00, 0x00, 0xc0, 0xff,
];

pub struct Apu {
    ram: Box<[u8]>,
    pub(crate) ipl_rom: Box<[u8]>,
    pub(crate) output_ports: [u8; 4],

    pub(crate) smp: Option<Box<Smp>>,
    pub(crate) dsp: Option<Box<Dsp>>,
    pub(crate) script700_runtime: Option<Box<Runtime>>,

    timers: [Timer; 3],

    is_ipl_rom_enabled: bool,
    dsp_reg_address: u8
}

impl Apu {
    pub fn new() -> Box<Apu> {
        let mut ret = Box::new(Apu {
            ram: vec![0; RAM_LEN].into_boxed_slice(),
            ipl_rom: DEFAULT_IPL_ROM.iter().cloned().collect::<Vec<_>>().into_boxed_slice(),
            output_ports: [0u8; 4],

            smp: None,
            dsp: None,
            script700_runtime: None,

            timers: [Timer::new(256), Timer::new(256), Timer::new(32)],

            is_ipl_rom_enabled: true,
            dsp_reg_address: 0
        });
        let ret_ptr = &mut *ret as *mut _;
        ret.smp = Some(Box::new(Smp::new(ret_ptr)));
        ret.dsp = Some(Dsp::new(ret_ptr));
        ret.script700_runtime = Some(Runtime::new(ret_ptr));
        ret
    }

    pub fn read_spc(&mut self, spc: &Spc) {
        for i in 0..RAM_LEN {
            self.ram[i] = spc.ram[i];
        }
        for i in 0..IPL_ROM_LEN {
            self.ipl_rom[i] = spc.ipl_rom[i];
        }

        {
            let smp = self.smp.as_mut().unwrap();
            smp.reg_pc = spc.pc;
            smp.reg_a = spc.a;
            smp.reg_x = spc.x;
            smp.reg_y = spc.y;
            smp.set_psw(spc.psw);
            smp.reg_sp = spc.sp;
        }

        self.dsp.as_mut().unwrap().set_state(spc);

        for i in 0..3 {
            let target = self.ram[0xfa + i];
            self.timers[i].set_target(target);
        }
        let control_reg = self.ram[0xf1];
        self.set_control_reg(control_reg);

        self.dsp_reg_address = self.ram[0xf2];

        // Restore APUIO registers
        for a in 0xf4..=0xf7 {
            self.ram[a] = spc.ram[a];
        }

        self.script700_runtime.as_mut().unwrap().reset();
    }

    pub fn from_spc(spc: &Spc) -> Box<Apu> {
        let mut ret = Apu::new();
        ret.read_spc(spc);
        ret
    }

    pub fn read_anonymous_script700(&mut self, script: &str) {
        self.script700_runtime.as_mut().unwrap().read_anonymous_script(script);
    }

    pub fn load_script700<P: AsRef<Path>>(&mut self, script_path: P) -> io::Result<()> {
        self.script700_runtime.as_mut().unwrap().load_script(script_path)
    }

    pub fn render(&mut self, left_buffer: &mut [i16], right_buffer: &mut [i16], num_samples: i32) {
        let smp = self.smp.as_mut().unwrap();
        let dsp = self.dsp.as_mut().unwrap();
        while dsp.output_buffer.get_sample_count() < num_samples {
            smp.run(num_samples * 64);
            dsp.flush();
        }

        dsp.output_buffer.read(left_buffer, right_buffer, num_samples);
    }

    pub fn cpu_cycles_callback(&mut self, num_cycles: i32) {
        self.script700_runtime.as_mut().unwrap().cycles_callback(num_cycles);
        self.dsp.as_mut().unwrap().cycles_callback(num_cycles);
        for timer in self.timers.iter_mut() {
            timer.cpu_cycles_callback(num_cycles);
        }
    }

    pub fn read_u8(&mut self, address: u32) -> u8 {
        let address = address & 0xffff;
        if address >= 0xf0 && address < 0x0100 {
            match address {
                0xf0 | 0xf1 => 0,

                0xf2 => self.dsp_reg_address,
                0xf3 => self.dsp.as_mut().unwrap().get_register(self.dsp_reg_address),

                0xf4 ..= 0xf7 => {
                    self.script700_runtime.as_mut().unwrap().trigger_port_event(false, (address - 0xf4) as u8);
                    self.ram[address as usize]
                }
                0xfa ..= 0xfc => 0,

                0xfd => self.timers[0].read_counter(),
                0xfe => self.timers[1].read_counter(),
                0xff => self.timers[2].read_counter(),

                _ => self.ram[address as usize]
            }
        } else if address >= 0xffc0 && self.is_ipl_rom_enabled {
            self.ipl_rom[(address - 0xffc0) as usize]
        } else {
            self.ram[address as usize]
        }
    }

    pub fn write_u8(&mut self, address: u32, value: u8) {
        let address = address & 0xffff;
        if address >= 0x00f0 && address < 0x0100 {
            match address {
                0xf0 => { self.set_test_reg(value); },
                0xf1 => { self.set_control_reg(value); },
                0xf2 => { self.dsp_reg_address = value; },
                0xf3 => { self.dsp.as_mut().unwrap().set_register(self.dsp_reg_address, value); },

                0xf4 ..= 0xf7 => {
                    self.output_ports[(address - 0xf4) as usize] = value;
                    self.script700_runtime.as_mut().unwrap().trigger_port_event(true, (address - 0xf4) as u8);
                },
                0xf8 ..= 0xf9 => { self.ram[address as usize] = value; },

                0xfa => { self.timers[0].set_target(value); },
                0xfb => { self.timers[1].set_target(value); },
                0xfc => { self.timers[2].set_target(value); },

                _ => () // Do nothing
            }
        } else {
            self.ram[address as usize] = value;
        }
    }

    pub fn clear_echo_buffer(&mut self) {
        let dsp = self.dsp.as_mut().unwrap();
        // Check FLG to see if echo writes are disabled, and skip clearing if they are.
        if (dsp.get_register(0x6c) & 0x20) != 0 {
            return;
        }
        let length = dsp.calculate_echo_length();
        let mut end_addr = dsp.get_echo_start_address() as i32 + length;
        if end_addr > RAM_LEN as i32 {
            end_addr = RAM_LEN as i32;
        }
        for i in dsp.get_echo_start_address() as i32..end_addr {
            self.ram[i as usize] = 0xff;
        }
    }

    fn set_test_reg(&mut self, value: u8) {
        if value != 0x0A {
            let pc = self.smp.as_ref().unwrap().reg_pc;
            let ir = self.read_u8(pc as u32);
            panic!("Test reg not implemented (pc=${:04x}, ir=${:02x}, echo_start=${:04x}, value=${:02x})", pc, ir, self.dsp.as_ref().unwrap().get_echo_start_address(), value);
        }
    }

    fn set_control_reg(&mut self, value: u8) {
        self.is_ipl_rom_enabled = (value & 0x80) != 0;
        if (value & 0x20) != 0 {
            self.ram[0xf6] = 0;
            self.ram[0xf7] = 0;
        }
        if (value & 0x10) != 0 {
            self.ram[0xf4] = 0;
            self.ram[0xf5] = 0;
        }
        self.timers[0].set_start_stop_bit((value & 0x01) != 0);
        self.timers[1].set_start_stop_bit((value & 0x02) != 0);
        self.timers[2].set_start_stop_bit((value & 0x04) != 0);
    }

    pub fn set_resampling_mode(&mut self, resampling_mode: ResamplingMode) {
        self.dsp.as_mut().unwrap().set_resampling_mode(resampling_mode);
    }

    pub fn set_state_receiver(&mut self, state_receiver: Option<Arc<Mutex<dyn ApuStateReceiver>>>) {
        self.dsp.as_mut().unwrap().state_receiver = state_receiver;
    }

    pub fn read_sample_directory(&mut self, source: u8) -> (u32, u32) {
        let sample_address = self.dsp.as_mut().unwrap().read_source_dir_start_address(source as i32);
        let loop_address = self.dsp.as_mut().unwrap().read_source_dir_loop_address(source as i32);
        (sample_address, loop_address)
    }

    pub fn write_to_input_port(&mut self, port: usize, value: u8) {
        debug_assert!(port < 4);
        self.ram[0xf4 + port] = value;
    }
}

unsafe impl Send for Apu {}
unsafe impl Sync for Apu {}
