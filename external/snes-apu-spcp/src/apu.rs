use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};
use crate::smp::Smp;
use crate::timer::Timer;
use crate::dsp::dsp::Dsp;
use crate::script700::runtime::Runtime;
use spc_spcp::spc::{Spc, RAM_LEN, IPL_ROM_LEN};
use crate::blargg_spc_filter::BlarggSpcFilter;
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
    pub master_reset: bool,
    pub master_mute: bool,
    pub echo_writes_enabled: bool,
    pub noise_clock: u8,
    pub fir: [u8; 8],
    pub input_ports: [u8; 4],
    pub output_ports: [u8; 4],
    pub amplitude: (i32, i32)
}

#[derive(Copy, Clone, Default, Debug)]
pub struct ApuSmpState {
    pub reg_pc: u16,
    pub reg_a: u8,
    pub reg_x: u8,
    pub reg_y: u8,
    pub reg_sp: u8,
    pub reg_psw: u8
}

#[derive(Copy, Clone, Default, Debug)]
pub struct ApuScript700State {
    pub wait_cycles: u32,
    pub cmp1: u32,
    pub cmp2: u32,
    pub working_memory: [u32; 8],
    pub pc: usize,
    pub input_ports_unbuffered: bool,
    pub call_stack_size: usize,
    pub call_stack_enabled: bool
}

pub trait ApuStateReceiver {
    fn receive_channel(&mut self, channel: usize, state: ApuChannelState) {
        let _ = (channel, state);
    }

    fn receive_master(&mut self, state: ApuMasterState) {
        let _ = state;
    }

    fn receive_smp(&mut self, state: ApuSmpState) {
        let _ = state;
    }

    fn receive_script700(&mut self, state: ApuScript700State) {
        let _ = state;
    }
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
    pub(crate) ram: Box<[u8; RAM_LEN]>,
    pub(crate) ipl_rom: Box<[u8]>,
    pub(crate) output_ports: [u8; 4],

    pub(crate) smp: Option<Box<Smp>>,
    pub(crate) dsp: Option<Box<Dsp>>,
    pub(crate) script700_runtime: Option<Box<Runtime>>,

    timer0: Timer<128>,
    timer1: Timer<128>,
    timer2: Timer<16>,

    is_ipl_rom_enabled: bool,
    dsp_reg_address: u8,

    output_filter: BlarggSpcFilter,
    output_filter_enabled: bool
}

impl Apu {
    pub fn new() -> Box<Apu> {
        let mut ret = Box::new(Apu {
            ram: Box::new([0; RAM_LEN]),
            ipl_rom: Box::new(DEFAULT_IPL_ROM),
            output_ports: [0u8; 4],

            smp: None,
            dsp: None,
            script700_runtime: None,

            timer0: Timer::new(),
            timer1: Timer::new(),
            timer2: Timer::new(),

            is_ipl_rom_enabled: true,
            dsp_reg_address: 0,

            output_filter: BlarggSpcFilter::default(),
            output_filter_enabled: true
        });
        let ret_ptr = &mut *ret as *mut _;
        ret.smp = Some(Box::new(Smp::new(ret_ptr)));
        ret.dsp = Some(Dsp::new(ret_ptr));
        ret.script700_runtime = Some(Runtime::new(ret_ptr));
        ret
    }

    pub fn set_output_filter_enabled(&mut self, enabled: bool) {
        self.output_filter.clear();
        self.output_filter_enabled = enabled;
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

        self.timer0.set_target(self.ram[0xfa]);
        self.timer1.set_target(self.ram[0xfb]);
        self.timer2.set_target(self.ram[0xfc]);

        let control_reg = self.ram[0xf1];
        self.set_control_reg(control_reg);

        self.dsp_reg_address = self.ram[0xf2];

        // Restore APUIO registers
        for a in 0xf4..=0xf7 {
            self.ram[a] = spc.ram[a];
        }

        self.script700_runtime.as_mut().unwrap().reset();
        self.output_filter.clear();
        self.dsp.as_mut().unwrap().output_buffer.clear();
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

    pub fn render(&mut self, left_buffer: &mut [i16], right_buffer: &mut [i16], num_samples: usize) {
        let smp = self.smp.as_mut().unwrap();
        let dsp = self.dsp.as_mut().unwrap();
        while dsp.output_buffer.get_sample_count() < num_samples {
            smp.run(64);
        }

        dsp.output_buffer.read(left_buffer, right_buffer, num_samples);
        if self.output_filter_enabled {
            self.output_filter.run(left_buffer, 0);
            self.output_filter.run(right_buffer, 1);
        }
    }

    pub fn cpu_cycles_callback(&mut self, num_cycles: i32) {
        self.timer0.cpu_cycles_callback(num_cycles);
        self.timer1.cpu_cycles_callback(num_cycles);
        self.timer2.cpu_cycles_callback(num_cycles);
        self.dsp.as_mut().unwrap().cycles_callback(num_cycles);
        self.script700_runtime.as_mut().unwrap().cycles_callback(num_cycles);
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

                0xfd => self.timer0.read(),
                0xfe => self.timer1.read(),
                0xff => self.timer2.read(),

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
                0xf2 => { self.dsp_reg_address = value & 0x7F; },
                0xf3 => { self.dsp.as_mut().unwrap().set_register(self.dsp_reg_address, value); },

                0xf4 ..= 0xf7 => {
                    self.output_ports[(address - 0xf4) as usize] = value;
                    self.script700_runtime.as_mut().unwrap().trigger_port_event(true, (address - 0xf4) as u8);
                },
                0xf8 ..= 0xf9 => { self.ram[address as usize] = value; },

                0xfa => { self.timer0.set_target(value); },
                0xfb => { self.timer1.set_target(value); },
                0xfc => { self.timer2.set_target(value); },

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

        self.timer0.synchronize_stage1();
        self.timer1.synchronize_stage1();
        self.timer2.synchronize_stage1();
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
        self.timer0.set_enable((value & 0x01) != 0, false);
        self.timer1.set_enable((value & 0x02) != 0, false);
        self.timer2.set_enable((value & 0x04) != 0, true);
    }

    pub fn resampling_mode(&self) -> ResamplingMode {
        self.dsp.as_ref().unwrap().resampling_mode()
    }

    pub fn set_resampling_mode(&mut self, resampling_mode: ResamplingMode) {
        self.dsp.as_mut().unwrap().set_resampling_mode(resampling_mode);
    }

    pub fn set_state_receiver(&mut self, state_receiver: Option<Arc<Mutex<dyn ApuStateReceiver>>>) {
        self.smp.as_mut().unwrap().state_receiver = state_receiver.clone();
        self.dsp.as_mut().unwrap().state_receiver = state_receiver.clone();
        self.script700_runtime.as_mut().unwrap().state_receiver = state_receiver.clone();
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
