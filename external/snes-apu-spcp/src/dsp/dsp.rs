use std::sync::{Arc, Mutex};
use crate::apu::{Apu, ApuChannelState, ApuMasterState, ApuStateReceiver};
use super::voice::{Voice, ResamplingMode};
use super::ring_buffer::RingBuffer;
use super::stereo::Stereo;
use spc_spcp::spc::{Spc, REG_LEN};

pub const SAMPLE_RATE: usize = 32000;
pub const BUFFER_LEN: usize = SAMPLE_RATE * 2;

const NUM_VOICES: usize = 8;

const COUNTER_RANGE: i32 = 30720;
static COUNTER_RATES: [i32; 32] = [
    COUNTER_RANGE + 1, // Never fires
    2048, 1536, 1280, 1024, 768, 640, 512, 384, 320, 256, 192, 160, 128, 96,
    80, 64, 48, 40, 32, 24, 20, 16, 12, 10, 8, 6, 5, 4, 3, 2, 1];

static COUNTER_OFFSETS: [i32; 32] = [
    0, 0, 1040, 536, 0, 1040, 536, 0, 1040, 536, 0, 1040, 536, 0, 1040,
    536, 0, 1040, 536, 0, 1040, 536, 0, 1040, 536, 0, 1040, 536, 0, 1040, 0, 0];

pub struct Dsp {
    emulator: *mut Apu,

    pub voices: Vec<Box<Voice>>,

    pub output_buffer: RingBuffer,

    pub(super) master_volume: Stereo<u8>,
    pub(super) echo_volume: Stereo<u8>,
    pub(super) noise_clock: u8,
    pub(super) echo_write_enabled: bool,
    pub(super) l_echo_write_enabled: bool,
    pub(super) echo_feedback: u8,
    pub(super) source_dir: u8,
    pub(super) l_source_dir: u8,
    pub(super) echo_start_address: u16,
    pub(super) l_echo_start_address: u16,
    pub(super) echo_address: u16,
    pub(super) echo_delay: u8,
    kon_cache: u8,
    kof_cache: u8,

    pub(super) counter: i32,

    cycle_count: i32,
    pub(super) echo_pos: i32,
    pub(super) echo_length: i32,
    pub(super) fir: [u8; 8],
    pub(super) echo_history: Stereo<[i16; 8]>,
    pub(super) echo_history_offset: usize,

    pub(super) master_reset: bool,
    pub(super) master_mute: bool,
    pub(super) master_output: Stereo<i32>,
    pub(super) echo_input: Stereo<i32>,
    pub(super) echo_output: Stereo<i32>,
    pub(super) every_other_sample: bool,
    pub(super) noise: i32,
    pub(super) l_adsr0: u8,
    pub(super) l_envx: u8,
    pub(super) l_outx: u8,
    pub(super) l_pitch: i32,
    pub(super) l_output: i32,

    resampling_mode: ResamplingMode,

    pub state_receiver: Option<Arc<Mutex<dyn ApuStateReceiver>>>
}

impl Dsp {
    pub fn new(emulator: *mut Apu) -> Box<Dsp> {
        let mut ret = Box::new(Dsp {
            emulator,

            voices: Vec::with_capacity(NUM_VOICES),

            output_buffer: RingBuffer::new(),

            master_volume: Stereo::default(),
            echo_volume: Stereo::default(),
            noise_clock: 0,
            echo_write_enabled: false,
            l_echo_write_enabled: false,
            echo_feedback: 0,
            source_dir: 0,
            l_source_dir: 0,
            echo_start_address: 0,
            l_echo_start_address: 0,
            echo_address: 0,
            echo_delay: 0,
            kon_cache: 0,
            kof_cache: 0,

            counter: 0,

            cycle_count: 0,
            echo_pos: 0,
            echo_length: 0,
            fir: [0u8; 8],
            echo_history: Stereo::new([0i16; 8], [0i16; 8]),
            echo_history_offset: 0,

            master_reset: true,
            master_mute: true,
            master_output: Stereo::default(),
            echo_input: Stereo::default(),
            echo_output: Stereo::default(),
            every_other_sample: true,
            noise: 0x4000,
            l_adsr0: 0,
            l_envx: 0,
            l_outx: 0,
            l_pitch: 0,
            l_output: 0,

            resampling_mode: ResamplingMode::Accurate,

            state_receiver: None
        });
        let ret_ptr = &mut *ret as *mut _;
        for _ in 0..NUM_VOICES {
            ret.voices.push(Box::new(Voice::new(ret_ptr, emulator, ResamplingMode::Accurate)));
        }
        ret.set_filter_coefficient(0x00, 0x80);
        ret.set_filter_coefficient(0x01, 0xff);
        ret.set_filter_coefficient(0x02, 0x9a);
        ret.set_filter_coefficient(0x03, 0xff);
        ret.set_filter_coefficient(0x04, 0x67);
        ret.set_filter_coefficient(0x05, 0xff);
        ret.set_filter_coefficient(0x06, 0x0f);
        ret.set_filter_coefficient(0x07, 0xff);
        ret.set_resampling_mode(ResamplingMode::Accurate);
        ret
    }

    #[inline]
    pub(super) fn emulator(&self) -> &mut Apu {
        unsafe {
            &mut (*self.emulator)
        }
    }

    fn set_filter_coefficient(&mut self, index: i32, value: u8) {
        self.fir[index as usize] = value;
    }

    fn get_filter_coefficient(&self, index: i32) -> u8 {
        self.fir[index as usize]
    }

    pub fn resampling_mode(&self) -> ResamplingMode {
        self.resampling_mode
    }

    pub fn set_resampling_mode(&mut self, resampling_mode: ResamplingMode) {
        self.resampling_mode = resampling_mode;
        for voice in self.voices.iter_mut() {
            voice.resampling_mode = resampling_mode;
        }
    }

    pub fn set_state(&mut self, spc: &Spc) {
        for i in 0..REG_LEN {
            match i {
                0x4c | 0x5c => (), // Don't key on/off anything yet
                _ => { self.set_register(i as u8, spc.regs[i]); }
            }
        }

        self.set_kon(spc.regs[0x4c]);
        self.set_kof(spc.regs[0x5c]);

        // Tick some latches now
        self.source_dir = self.l_source_dir;
        self.echo_start_address = self.l_echo_start_address;
        self.echo_length = self.calculate_echo_length();
        for voice in self.voices.iter_mut() {
            voice.tick_latches();
        }
    }

    pub fn cycles_callback(&mut self, num_cycles: i32) {
        for _ in 0..num_cycles {
            self.cycle_count = (self.cycle_count + 1) % 32;

            // if (self.cycle_count % 2) != 0 {
            //     continue;
            // }

            match self.cycle_count {
                0 => {
                    self.voices[0].voice5();
                    self.voices[1].voice2();
                },
                1 => {
                    self.voices[0].voice6();
                    self.voices[1].voice3();
                },
                2 => {
                    self.voices[0].voice7();
                    self.voices[1].voice4();
                    self.voices[3].voice1();
                },
                3 => {
                    self.voices[0].voice8();
                    self.voices[1].voice5();
                    self.voices[2].voice2();
                },
                4 => {
                    self.voices[0].voice9();
                    self.voices[1].voice6();
                    self.voices[2].voice3();
                },
                5 => {
                    self.voices[1].voice7();
                    self.voices[2].voice4();
                    self.voices[4].voice1();
                },
                6 => {
                    self.voices[1].voice8();
                    self.voices[2].voice5();
                    self.voices[3].voice2();
                },
                7 => {
                    self.voices[1].voice9();
                    self.voices[2].voice6();
                    self.voices[3].voice3();
                },
                8 => {
                    self.voices[2].voice7();
                    self.voices[3].voice4();
                    self.voices[5].voice1();
                },
                9 => {
                    self.voices[2].voice8();
                    self.voices[3].voice5();
                    self.voices[4].voice2();
                },
                10 => {
                    self.voices[2].voice9();
                    self.voices[3].voice6();
                    self.voices[4].voice3();
                },
                11 => {
                    self.voices[3].voice7();
                    self.voices[4].voice4();
                    self.voices[6].voice1();
                },
                12 => {
                    self.voices[3].voice8();
                    self.voices[4].voice5();
                    self.voices[5].voice2();
                },
                13 => {
                    self.voices[3].voice9();
                    self.voices[4].voice6();
                    self.voices[5].voice3();
                },
                14 => {
                    self.voices[4].voice7();
                    self.voices[5].voice4();
                    self.voices[7].voice1();
                },
                15 => {
                    self.voices[4].voice8();
                    self.voices[5].voice5();
                    self.voices[6].voice2();
                },
                16 => {
                    self.voices[4].voice9();
                    self.voices[5].voice6();
                    self.voices[6].voice3();
                },
                17 => {
                    self.voices[0].voice1();
                    self.voices[5].voice7();
                    self.voices[6].voice4();
                },
                18 => {
                    self.voices[5].voice8();
                    self.voices[6].voice5();
                    self.voices[7].voice2();
                },
                19 => {
                    self.voices[5].voice9();
                    self.voices[6].voice6();
                    self.voices[7].voice3();
                },
                20 => {
                    self.voices[1].voice1();
                    self.voices[6].voice7();
                    self.voices[7].voice4();
                },
                21 => {
                    self.voices[6].voice8();
                    self.voices[7].voice5();
                    self.voices[0].voice2();
                },
                22 => {
                    self.voices[0].voice3a();
                    self.voices[6].voice9();
                    self.voices[7].voice6();
                    self.echo22();
                    self.state22();
                },
                23 => {
                    self.voices[7].voice7();
                    self.echo23();
                },
                24 => {
                    self.voices[7].voice8();
                    self.echo24();
                },
                25 => {
                    self.voices[0].voice3b();
                    self.voices[7].voice9();
                    self.echo25();
                },
                26 => {
                    self.echo26();
                },
                27 => {
                    self.echo27();
                    self.misc27();
                },
                28 => {
                    self.echo28();
                    self.misc28();
                },
                29 => {
                    self.echo29();
                    self.misc29();
                },
                30 => {
                    self.misc30();
                    self.voices[0].voice3c();
                    self.echo30();
                },
                31 => {
                    self.voices[0].voice4();
                    self.voices[2].voice1();
                },

                _ => unreachable!()
            }
        }
    }

    fn state22(&mut self) {
        if self.state_receiver.is_some() {
            for channel in 0..NUM_VOICES {
                let voice = self.voices.get_mut(channel).unwrap();

                let state = ApuChannelState {
                    source: voice.source,
                    muted: voice.is_muted,
                    envelope_level: voice.envelope.level,
                    volume: (*voice.volume.left() as i8, *voice.volume.right() as i8),
                    amplitude: (voice.amplitude.into_inner_left(), voice.amplitude.into_inner_right()),
                    pitch: voice.pitch(),
                    noise_clock: voice.noise_on.then_some(self.noise_clock),
                    edge: voice.edge_detected(),
                    kon_frames: voice.get_sample_frame(),
                    sample_block_index: voice.sample_block_index,
                    echo_delay: (self.echo_write_enabled && voice.echo_on).then_some(self.echo_delay),
                    pitch_modulation: voice.pitch_mod
                };

                self.state_receiver
                    .as_ref()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .receive(channel, state);
            }

            let state = ApuMasterState {
                master_volume: (*self.master_volume.left() as i8, *self.master_volume.right() as i8),
                echo_volume: (*self.echo_volume.left() as i8, *self.echo_volume.right() as i8),
                echo_delay: self.echo_delay,
                echo_feedback: self.echo_feedback as i8,
                amplitude: (self.master_output.into_inner_left(), self.master_output.into_inner_right()),
            };

            self.state_receiver
                .as_ref()
                .unwrap()
                .lock()
                .unwrap()
                .receive_master(state);
        }
    }

    pub fn get_echo_start_address(&self) -> u16 {
        self.echo_start_address
    }

    pub(crate) fn calculate_echo_length(&self) -> i32 {
        (self.echo_delay as i32) * 0x800
    }

    // pub fn flush(&mut self) {
    //     self.is_flushing = true;
    //
    //     while self.cycles_since_last_flush > 64 {
    //         if !self.read_counter(self.noise_clock as i32) {
    //             let feedback = (self.noise << 13) ^ (self.noise << 14);
    //             self.noise = (feedback & 0x4000) ^ (self.noise >> 1);
    //         }
    //
    //         let mut are_any_voices_solod = false;
    //         for voice in self.voices.iter() {
    //             if voice.is_solod {
    //                 are_any_voices_solod = true;
    //                 break;
    //             }
    //         }
    //
    //         let mut left_out = 0;
    //         let mut right_out = 0;
    //         let mut left_echo_out = 0;
    //         let mut right_echo_out = 0;
    //         let mut last_voice_out = 0;
    //         for voice in self.voices.iter_mut() {
    //             let output = voice.render_sample(last_voice_out, self.noise, are_any_voices_solod);
    //
    //             left_out = dsp_helpers::clamp(dsp_helpers::cast_arb_int(left_out + output.left_out, 17));
    //             right_out = dsp_helpers::clamp(dsp_helpers::cast_arb_int(right_out + output.right_out, 17));
    //
    //             if voice.echo_on {
    //                 left_echo_out = dsp_helpers::clamp(dsp_helpers::cast_arb_int(left_echo_out + output.left_out, 17));
    //                 right_echo_out = dsp_helpers::clamp(dsp_helpers::cast_arb_int(right_echo_out + output.right_out, 17));
    //             }
    //
    //             last_voice_out = ((output.last_voice_out & 0xFFFF) as i16) as i32;
    //         }
    //
    //         left_out = dsp_helpers::multiply_volume(left_out, self.vol_left);
    //         right_out = dsp_helpers::multiply_volume(right_out, self.vol_right);
    //
    //         let echo_address = (self.echo_start_address.wrapping_add(self.echo_pos as u16)) as u32;
    //         // println!("ECHO_ADDR=${:04x} ECHO_LEN=${:04x} ESA=${:04x} EDL=${:02x}\n\n\n", echo_address, self.echo_length, self.echo_start_address, self.echo_delay);
    //         let mut left_echo_in = ((((self.emulator().read_u8(echo_address + 1) as i32) << 8) | (self.emulator().read_u8(echo_address) as i32)) as i16) as i32;
    //         let mut right_echo_in = ((((self.emulator().read_u8(echo_address + 3) as i32) << 8) | (self.emulator().read_u8(echo_address + 2) as i32)) as i16) as i32;
    //
    //         left_echo_in = dsp_helpers::clamp(self.left_filter.next(left_echo_in, false)) & !1;
    //         right_echo_in = dsp_helpers::clamp(self.right_filter.next(right_echo_in, true)) & !1;
    //
    //         let left_out = dsp_helpers::clamp(dsp_helpers::cast_arb_int(left_out + dsp_helpers::multiply_volume(left_echo_in, self.echo_vol_left), 17)) as i16;
    //         let right_out = dsp_helpers::clamp(dsp_helpers::cast_arb_int(right_out + dsp_helpers::multiply_volume(right_echo_in, self.echo_vol_right), 17)) as i16;
    //         self.output_buffer.write_sample(left_out, right_out);
    //
    //         if self.echo_pos == 0 {
    //             self.echo_length = self.calculate_echo_length();
    //         }
    //         self.echo_pos += 4;
    //         if self.echo_pos >= self.echo_length {
    //             self.echo_pos = 0;
    //         }
    //
    //         if self.echo_write_enabled {
    //             left_echo_out = dsp_helpers::clamp(dsp_helpers::cast_arb_int(left_echo_out + ((((left_echo_in * ((self.echo_feedback as i8) as i32)) >> 7) as i16) as i32), 17)) & !1;
    //             right_echo_out = dsp_helpers::clamp(dsp_helpers::cast_arb_int(right_echo_out + ((((right_echo_in * ((self.echo_feedback as i8) as i32)) >> 7) as i16) as i32), 17)) & !1;
    //
    //             self.emulator().write_u8(echo_address + 0, left_echo_out as u8);
    //             self.emulator().write_u8(echo_address + 1, (left_echo_out >> 8) as u8);
    //             self.emulator().write_u8(echo_address + 2, right_echo_out as u8);
    //             self.emulator().write_u8(echo_address + 3, (right_echo_out >> 8) as u8);
    //         }
    //
    //         if self.counter == 0 {
    //             self.counter = COUNTER_RANGE;
    //         }
    //         self.counter -= 1;
    //         self.cycles_since_last_flush -= 64;
    //
    //         if self.state_receiver.is_some() {
    //             for channel in 0..NUM_VOICES {
    //                 let voice = self.voices.get_mut(channel).unwrap();
    //
    //                 let state = ApuChannelState {
    //                     source: voice.source,
    //                     muted: voice.is_muted,
    //                     envelope_level: voice.envelope.level,
    //                     volume: (*voice.volume.left() as i8, *voice.volume.right() as i8),
    //                     amplitude: voice.amplitude,
    //                     pitch: voice.pitch(),
    //                     noise_clock: voice.noise_on.then_some(self.noise_clock),
    //                     edge: voice.edge_detected(),
    //                     kon_frames: voice.get_sample_frame(),
    //                     sample_block_index: voice.sample_block_index,
    //                     echo_delay: (self.echo_write_enabled && voice.echo_on).then_some(self.echo_delay),
    //                     pitch_modulation: voice.pitch_mod
    //                 };
    //
    //                 self.state_receiver
    //                     .as_ref()
    //                     .unwrap()
    //                     .lock()
    //                     .unwrap()
    //                     .receive(channel, state);
    //             }
    //
    //             let state = ApuMasterState {
    //                 master_volume: (*self.master_volume.left() as i8, *self.master_volume.right() as i8),
    //                 echo_volume: (*self.echo_volume.left() as i8, *self.echo_volume.right() as i8),
    //                 echo_delay: self.echo_delay,
    //                 echo_feedback: self.echo_feedback as i8,
    //                 amplitude: (left_out as i32, right_out as i32),
    //             };
    //
    //             self.state_receiver
    //                 .as_ref()
    //                 .unwrap()
    //                 .lock()
    //                 .unwrap()
    //                 .receive_master(state);
    //         }
    //     }
    //
    //     self.is_flushing = false;
    // }

    pub fn set_register(&mut self, address: u8, value: u8) {
        if (address & 0x80) != 0 {
            return;
        }

        // if !self.is_flushing {
        //     self.flush();
        // }

        let voice_index = address >> 4;
        let voice_address = address & 0x0f;
        if voice_address < 0x0a {
            if voice_address < 8 {
                let voice = &mut self.voices[voice_index as usize];
                match voice_address {
                    0x00 => { voice.volume.set_left(value); },
                    0x01 => { voice.volume.set_right(value); },
                    0x02 => { voice.pitch_low = value; },
                    0x03 => { voice.set_pitch_high(value); },
                    0x04 => { voice.l_source = value; voice.edge_hit = true; },
                    0x05 => { voice.envelope.l_adsr0 = value; },
                    0x06 => { voice.envelope.adsr1 = value; },
                    0x07 => { voice.envelope.gain = value; },

                    _ => ()
                }
            }
        } else if voice_address == 0x0f {
            self.set_filter_coefficient(voice_index as i32, value);
        } else {
            match address {
                0x0c => { self.master_volume.set_left(value); },
                0x1c => { self.master_volume.set_right(value); },
                0x2c => { self.echo_volume.set_left(value); },
                0x3c => { self.echo_volume.set_right(value); },
                0x4c => { self.set_kon(value); },
                0x5c => { self.set_kof(value); },
                0x6c => { self.set_flg(value); },
                0x7c => { self.set_endx(); },

                0x0d => { self.echo_feedback = value; },

                0x2d => { self.set_pmon(value); },
                0x3d => { self.set_non(value); },
                0x4d => { self.set_eon(value); },
                0x5d => { self.l_source_dir = value; },
                0x6d => { self.l_echo_start_address = (value as u16) << 8; },
                0x7d => { self.echo_delay = value & 0x0f; },

                _ => ()
            }
        }
    }

    pub fn get_register(&mut self, address: u8) -> u8 {
        // if !self.is_flushing {
        //     self.flush();
        // }

        let voice_index = address >> 4;
        let voice_address = address & 0x0f;
        if voice_address < 0x0a {
            let voice = &mut self.voices[voice_index as usize];
            match voice_address {
                0x00 => voice.volume.into_inner_left(),
                0x01 => voice.volume.into_inner_right(),
                0x02 => voice.pitch_low,
                0x03 => voice.pitch_high & 0x3F,
                0x04 => voice.source,
                0x05 => voice.envelope.l_adsr0,
                0x06 => voice.envelope.adsr1,
                0x07 => voice.envelope.gain,
                0x08 => voice.envx_value,
                0x09 => voice.outx_value,

                _ => unreachable!()
            }
        } else if voice_address == 0x0f {
            self.get_filter_coefficient(voice_index as i32)
        } else {
            match address {
                0x0c => self.master_volume.into_inner_left(),
                0x1c => self.master_volume.into_inner_right(),
                0x2c => self.echo_volume.into_inner_left(),
                0x3c => self.echo_volume.into_inner_right(),
                0x4c => self.kon_cache,
                0x5c => self.kof_cache,
                0x6c => self.get_flg(),
                0x7c => self.get_endx(),

                0x2d => self.get_pmon(),
                0x3d => self.get_non(),
                0x4d => self.get_eon(),
                0x5d => self.l_source_dir,
                0x6d => (self.l_echo_start_address >> 8) as u8,
                0x7d => self.echo_delay,

                _ => 0
            }
        }
    }

    pub fn read_counter(&self, rate: i32) -> bool {
        if rate == 0 {
            return true;
        }
        ((self.counter + COUNTER_OFFSETS[rate as usize]) % COUNTER_RATES[rate as usize]) != 0
    }

    pub fn read_source_dir_start_address(&self, index: i32) -> u32 {
        self.read_source_dir_address(index, 0)
    }

    pub fn read_source_dir_loop_address(&self, index: i32) -> u32 {
        self.read_source_dir_address(index, 2)
    }

    fn read_source_dir_address(&self, index: i32, offset: i32) -> u32 {
        let dir_address = (self.source_dir as i32) * 0x100;
        let entry_address = dir_address + index * 4;
        let mut ret = self.emulator().read_u8((entry_address as u32) + (offset as u32)) as u32;
        ret |= (self.emulator().read_u8((entry_address as u32) + (offset as u32) + 1) as u32) << 8;
        ret
    }

    fn set_kon(&mut self, voice_mask: u8) {
        self.kon_cache = voice_mask;
        for i in 0..NUM_VOICES {
            if ((voice_mask as usize) & (1 << i)) != 0 {
                self.voices[i].l_kon = true;
            }
        }
    }

    fn set_kof(&mut self, voice_mask: u8) {
        self.kof_cache = voice_mask;
        for i in 0..NUM_VOICES {
            self.voices[i].l_kof = ((voice_mask as usize) & (1 << i)) != 0;
        }
    }

    fn set_flg(&mut self, value: u8) {
        self.noise_clock = value & 0x1f;
        self.l_echo_write_enabled = (value & 0x20) == 0;
        self.master_mute = (value & 0x40) != 0;
        self.master_reset = (value & 0x80) != 0;
    }

    fn get_flg(&self) -> u8 {
        let mut result = self.noise_clock;
        if !self.l_echo_write_enabled {
            result |= 0x20;
        }
        if self.master_mute {
            result |= 0x40;
        }
        if self.master_reset {
            result |= 0x80;
        }
        result
    }

    fn set_pmon(&mut self, voice_mask: u8) {
        self.voices[0].l_pitch_mod = false;
        for i in 1..NUM_VOICES {
            self.voices[i].l_pitch_mod = ((voice_mask as usize) & (1 << i)) != 0;
        }
    }

    fn get_pmon(&self) -> u8 {
        let mut result = 0u8;
        for i in 1..NUM_VOICES {
            if self.voices[i].l_pitch_mod {
                result |= (1 << i) as u8;
            }
        }
        result
    }

    fn set_non(&mut self, voice_mask: u8) {
        for i in 0..NUM_VOICES {
            self.voices[i].l_noise_on = ((voice_mask as usize) & (1 << i)) != 0;
        }
    }

    fn get_non(&self) -> u8 {
        let mut result = 0u8;
        for i in 1..NUM_VOICES {
            if self.voices[i].l_noise_on {
                result |= (1 << i) as u8;
            }
        }
        result
    }

    fn set_eon(&mut self, voice_mask: u8) {
        for i in 0..NUM_VOICES {
            self.voices[i].l_echo_on = ((voice_mask as usize) & (1 << i)) != 0;
        }
    }

    fn get_eon(&self) -> u8 {
        let mut result = 0u8;
        for i in 1..NUM_VOICES {
            if self.voices[i].l_echo_on {
                result |= (1 << i) as u8;
            }
        }
        result
    }

    fn set_endx(&mut self) {
        for i in 0..NUM_VOICES {
            self.voices[i].clear_endx_bit();
        }
    }

    fn get_endx(&mut self) -> u8 {
        let mut result = 0u8;
        for i in 0..NUM_VOICES {
            if self.voices[i].get_endx_bit() {
                result |= 1 << i;
            }
        }
        result
    }
}
