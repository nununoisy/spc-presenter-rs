use crate::emulator::snes_apu::dsp::brr_stream_decoder::BrrStreamDecoder;
use crate::emulator::snes_apu::dsp::gaussian::construct_accurate_gaussian_table;
use super::dsp::Dsp;
use super::super::apu::Apu;
use super::envelope::Envelope;
use super::dsp_helpers;
use super::gaussian::{HALF_KERNEL_SIZE, HALF_KERNEL};

const RESAMPLE_BUFFER_LEN: usize = 12;

#[derive(Clone, Copy)]
pub enum ResamplingMode {
    Linear,
    Gaussian,
    AccurateGaussian
}

#[derive(Clone, Copy)]
pub struct VoiceOutput {
    pub left_out: i32,
    pub right_out: i32,
    pub last_voice_out: i32,
}

impl VoiceOutput {
    pub fn default() -> VoiceOutput {
        VoiceOutput {
            left_out: 0,
            right_out: 0,
            last_voice_out: 0,
        }
    }
}

pub const VOICE_BUFFER_LEN: usize = 128;

pub struct VoiceBuffer {
    pub buffer: Box<[VoiceOutput]>,
    pub pos: i32,
}

impl VoiceBuffer {
    pub fn new() -> VoiceBuffer {
        VoiceBuffer {
            buffer: vec![VoiceOutput::default(); VOICE_BUFFER_LEN].into_boxed_slice(),
            pos: 0,
        }
    }

    pub fn write(&mut self, value: VoiceOutput) {
        self.buffer[self.pos as usize] = value;
        self.pos = (self.pos + 1) % (VOICE_BUFFER_LEN as i32);
    }

    pub fn read(&self) -> VoiceOutput {
        self.buffer.get(self.pos as usize).cloned().unwrap()
    }
}

pub struct Voice {
    dsp: *mut Dsp,
    emulator: *mut Apu,

    pub envelope: Envelope,

    pub vol_left: u8,
    pub vol_right: u8,
    pub pitch_low: u8,
    pub pitch_high: u8,
    pub source: u8,
    pub pitch_mod: bool,
    pub noise_on: bool,
    pub echo_on: bool,

    sample_start_address: u32,
    loop_start_address: u32,
    brr_decoder: BrrStreamDecoder,
    sample_address: u32,
    sample_offset: u32,
    sample_pos: i32,
    pub(super) sample_block_index: usize,

    pub(crate) edge_hit: bool,
    pub(crate) sample_frame: usize,
    endx_bit: bool,
    endx_latch: bool,
    pub outx_value: u8,
    kon_delay: u8,

    pub resampling_mode: ResamplingMode,
    resample_buffer: [i32; RESAMPLE_BUFFER_LEN],
    resample_buffer_pos: usize,
    accurate_gaussian_table: [i16; 512],

    pub output_buffer: VoiceBuffer,
    pub is_muted: bool,
    pub is_solod: bool,
}

impl Voice {
    pub fn new(dsp: *mut Dsp, emulator: *mut Apu, resampling_mode: ResamplingMode) -> Voice {
        Voice {
            dsp: dsp,
            emulator: emulator,

            envelope: Envelope::new(dsp),

            vol_left: 0,
            vol_right: 0,
            pitch_low: 0,
            pitch_high: 0x10,
            source: 0,
            pitch_mod: false,
            noise_on: false,
            echo_on: false,

            sample_start_address: 0,
            loop_start_address: 0,
            brr_decoder: BrrStreamDecoder::new(),
            sample_address: 0,
            sample_offset: 0,
            sample_pos: 0,
            sample_block_index: 0,

            edge_hit: false,
            sample_frame: 0,
            endx_bit: false,
            endx_latch: false,
            outx_value: 0,
            kon_delay: 0,

            resampling_mode: resampling_mode,
            resample_buffer: [0; RESAMPLE_BUFFER_LEN],
            resample_buffer_pos: 0,
            accurate_gaussian_table: construct_accurate_gaussian_table(),

            output_buffer: VoiceBuffer::new(),
            is_muted: false,
            is_solod: false,
        }
    }

    #[inline]
    fn dsp(&self) -> &mut Dsp {
        unsafe {
            &mut (*self.dsp)
        }
    }

    #[inline]
    fn emulator(&self) -> &mut Apu {
        unsafe {
            &mut (*self.emulator)
        }
    }

    pub fn render_sample(&mut self, last_voice_out: i32, noise: i32, are_any_voices_solod: bool) -> VoiceOutput {
        let mut pitch = ((self.pitch_high as i32) << 8) | (self.pitch_low as i32);
        if self.pitch_mod {
            pitch += ((last_voice_out >> 5) * pitch) >> 10;
        }
        pitch = ((pitch as u32) & 0x7fff) as i32;

        if self.kon_delay > 0 {
            self.envelope.kon_delay_tick();

            self.sample_pos = 0;
            self.kon_delay -= 1;
        };

        let mut sample = if !self.noise_on {
            let base_pos = match self.resampling_mode {
                ResamplingMode::AccurateGaussian => self.resample_buffer_pos + (self.sample_pos >> 12) as usize,
                _ => self.resample_buffer_pos
            };

            let s1 = self.resample_buffer[base_pos % RESAMPLE_BUFFER_LEN];
            let s2 = self.resample_buffer[(base_pos + 1) % RESAMPLE_BUFFER_LEN];
            let s3 = self.resample_buffer[(base_pos + 2) % RESAMPLE_BUFFER_LEN];
            let s4 = self.resample_buffer[(base_pos + 3) % RESAMPLE_BUFFER_LEN];
            let resampled = match self.resampling_mode {
                ResamplingMode::Linear => {
                    let p1 = self.sample_pos;
                    let p2 = 0x1000 - p1;
                    (s1 * p1 + s2 * p2) >> 12
                },
                ResamplingMode::Gaussian => {
                    let kernel_index = (self.sample_pos >> 2) as usize;
                    let p1 = HALF_KERNEL[kernel_index] as i32;
                    let p2 = HALF_KERNEL[kernel_index + HALF_KERNEL_SIZE / 2] as i32;
                    let p3 = HALF_KERNEL[HALF_KERNEL_SIZE - 1 - kernel_index] as i32;
                    let p4 = HALF_KERNEL[HALF_KERNEL_SIZE - 1 - (kernel_index + HALF_KERNEL_SIZE / 2)] as i32;
                    (s1 * p1 + s2 * p2 + s3 * p3 + s4 * p4) >> 11
                },
                ResamplingMode::AccurateGaussian => {
                    let kernel_index = (self.sample_pos >> 4) as usize;
                    let p1 = self.accurate_gaussian_table[255 - kernel_index] as i32;
                    let p2 = self.accurate_gaussian_table[511 - kernel_index] as i32;
                    let p3 = self.accurate_gaussian_table[kernel_index + 256] as i32;
                    let p4 = self.accurate_gaussian_table[kernel_index] as i32;

                    let c1 = (s4 * p1) >> 11;
                    let c2 = (s3 * p2) >> 11;
                    let c3 = (s2 * p3) >> 11;
                    let c4 = (s1 * p4) >> 11;

                    ((((c1 + c2 + c3) & 0xFFFF) as i16) as i32) + c4
                }
            };
            dsp_helpers::clamp(resampled) & !1
        } else {
            ((noise << 1) as i16) as i32
        };

        let env_level = self.envelope.level;

        sample = ((sample * env_level) >> 11) & !1;
        self.outx_value = ((sample >> 8) as i8) as u8;

        if self.kon_delay < 5 && self.brr_decoder.is_end && !self.brr_decoder.is_looping {
            self.envelope.key_off();
            self.envelope.level = 0;
        }

        if self.kon_delay == 0 {
            self.envelope.tick();
            self.sample_pos = (self.sample_pos & 0x3fff) + pitch;
            if self.sample_pos > 0x7fff {
                self.sample_pos = 0x7fff;
            }
        }

        self.endx_latch = false;
        while self.sample_pos >= 0x1000 {
            self.sample_pos -= 0x1000;
            self.read_next_sample();

            if self.brr_decoder.is_finished() {
                if self.kon_delay <= 3 {
                    if self.brr_decoder.is_end && self.brr_decoder.is_looping {
                        self.edge_hit = true;
                        self.read_entry();
                        if self.kon_delay == 0 {
                            self.sample_address = self.loop_start_address;
                        } else {
                            self.sample_address = self.sample_start_address;
                        }
                    } else {
                        self.sample_address += 9;
                    }
                    self.read_next_block();
                    self.sample_block_index += 1;
                } else {
                    self.brr_decoder.restart();
                    self.sample_offset = 0;
                }
            }
        }

        let ret =
            if self.is_solod || (!self.is_muted && !are_any_voices_solod) {
                VoiceOutput {
                    left_out: dsp_helpers::multiply_volume(sample, self.vol_left),
                    right_out: dsp_helpers::multiply_volume(sample, self.vol_right),
                    last_voice_out: sample
                }
            } else {
                VoiceOutput {
                    left_out: 0,
                    right_out: 0,
                    last_voice_out: 0
                }
            };
        self.output_buffer.write(ret);

        if self.kon_delay >= 5 {
            self.endx_bit = false;
        } else if self.endx_latch {
            self.endx_bit = true;
        }

        ret
    }

    pub fn set_pitch_high(&mut self, value: u8) {
        self.pitch_high = value & 0x3f;
    }

    pub fn key_on(&mut self) {
        self.read_entry();
        self.sample_address = self.sample_start_address;
        // self.brr_decoder.reset(0, 0);
        self.read_next_block();
        self.sample_block_index = 0;
        self.sample_pos = 0;
        for i in 0..RESAMPLE_BUFFER_LEN {
            self.resample_buffer[i] = 0;
        }
        self.read_next_sample();
        self.envelope.key_on();
        self.edge_hit = true;
        self.sample_frame = 0;
        self.endx_bit = false;
        self.endx_latch = false;
        self.kon_delay = 5;
    }

    pub fn key_off(&mut self) {
        self.envelope.key_off();
        self.edge_hit = true;
    }

    fn read_entry(&mut self) {
        self.sample_start_address = self.dsp().read_source_dir_start_address(self.source as i32);
        self.loop_start_address = self.dsp().read_source_dir_loop_address(self.source as i32);
    }

    fn read_next_block(&mut self) {
        // println!("Read BRR block header: ${:04x}", self.sample_address);
        self.brr_decoder.read_header(self.emulator().read_u8(self.sample_address));
        self.sample_offset = 0;

        if self.brr_decoder.is_end {
            self.endx_latch = true;
        }
    }

    fn read_next_sample(&mut self) {
        if self.brr_decoder.needs_more_samples() {
            assert!(self.sample_offset < 7, "OOB sample access: offset={}", self.sample_offset);
            let buf = vec![
                self.emulator().read_u8(self.sample_address + self.sample_offset + 1),
                self.emulator().read_u8(self.sample_address + self.sample_offset + 2)
            ];
            self.sample_offset += 2;

            self.brr_decoder.read(&buf);
        }

        self.resample_buffer_pos = match self.resample_buffer_pos {
            0 => RESAMPLE_BUFFER_LEN - 1,
            x => x - 1
        };
        self.resample_buffer[self.resample_buffer_pos] = self.brr_decoder.read_next_sample() as i32;
    }

    pub fn pitch(&self) -> u16 {
        (((self.pitch_high as u16) << 8) | (self.pitch_low as u16)) & 0x3FFF
    }

    pub fn edge_detected(&mut self) -> bool {
        let result = self.edge_hit;
        self.edge_hit = false;
        result
    }

    pub fn get_sample_frame(&mut self) -> usize {
        let result = self.sample_frame;
        self.sample_frame += 1;
        result
    }

    pub fn get_endx_bit(&mut self) -> bool {
        self.endx_bit
    }

    pub fn clear_endx_bit(&mut self) {
        self.endx_bit = false;
        self.endx_latch = false;
    }
}
