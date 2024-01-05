use super::brr_stream_decoder::BrrStreamDecoder;
use super::dsp::Dsp;
use super::super::apu::Apu;
use super::envelope::Envelope;
use super::dsp_helpers;
use super::gaussian::{HALF_KERNEL, HALF_KERNEL_SIZE, ACCURATE_KERNEL, ACCURATE_KERNEL_SIZE};

const RESAMPLE_BUFFER_LEN: usize = 12;

#[derive(Clone, Copy, Default, PartialEq)]
pub enum ResamplingMode {
    Linear,
    Gaussian,
    #[default]
    Accurate
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
    pub envx_value: u8,
    kon_delay: u8,

    pub resampling_mode: ResamplingMode,
    resample_buffer: [i32; RESAMPLE_BUFFER_LEN],
    resample_buffer_pos: usize,

    pub output_buffer: VoiceBuffer,
    pub is_muted: bool,
    pub is_solod: bool,

    every_other_sample: bool,
    pub kon_queued: bool,
    pub kof_queued: bool,
    samples_buffered: usize
}

impl Voice {
    pub fn new(dsp: *mut Dsp, emulator: *mut Apu, resampling_mode: ResamplingMode) -> Voice {
        Voice {
            dsp,
            emulator,

            envelope: Envelope::new(dsp),

            vol_left: 0,
            vol_right: 0,
            pitch_low: 0,
            pitch_high: 0,
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
            envx_value: 0,
            kon_delay: 0,

            resampling_mode,
            resample_buffer: [0; RESAMPLE_BUFFER_LEN],
            resample_buffer_pos: 0,

            output_buffer: VoiceBuffer::new(),
            is_muted: false,
            is_solod: false,

            every_other_sample: false,
            kon_queued: false,
            kof_queued: false,
            samples_buffered: 0
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
        // last misc29
        self.every_other_sample = !self.every_other_sample;

        // voice1
        self.read_entry();

        // voice2
        let next_sample_address = if self.kon_delay != 0 {
            self.sample_start_address
        } else {
            self.loop_start_address
        };

        // voice3a
        let mut pitch = ((self.pitch_high as i32) << 8) | (self.pitch_low as i32);

        // voice3b

        // voice3c
        if self.pitch_mod {
            pitch += ((last_voice_out >> 5) * pitch) >> 10;
        }
        pitch = ((pitch as u32) & 0x7fff) as i32;

        if self.kon_delay > 0 {
            if self.kon_delay == 5 {
                self.sample_address = next_sample_address;
                self.resample_buffer_pos = 0;
                self.sample_block_index = 0;
                self.sample_frame = 0;
                self.edge_hit = true;
                self.read_brr_header();
            }

            self.envelope.kon_delay_tick();

            self.kon_delay -= 1;
            self.sample_pos = if (self.kon_delay & 3) != 0 { 0x4000 } else { 0 };

            pitch = 0;
        };

        let mut sample = if !self.noise_on {
            let base_pos = self.resample_buffer_pos + (self.sample_pos >> 12) as usize;

            let s1 = self.resample_buffer[base_pos % RESAMPLE_BUFFER_LEN];
            let s2 = self.resample_buffer[(base_pos + 1) % RESAMPLE_BUFFER_LEN];
            let s3 = self.resample_buffer[(base_pos + 2) % RESAMPLE_BUFFER_LEN];
            let s4 = self.resample_buffer[(base_pos + 3) % RESAMPLE_BUFFER_LEN];

            let resampled = match self.resampling_mode {
                ResamplingMode::Linear => {
                    let p1 = self.sample_pos & 0xFFF;
                    let p2 = 0x1000 - p1;

                    (s1 * p1 + s2 * p2) >> 12
                },
                ResamplingMode::Gaussian => {
                    let kernel_index = ((self.sample_pos & 0xFFF) >> 2) as usize;
                    let p1 = HALF_KERNEL[HALF_KERNEL_SIZE - 1 - (kernel_index + HALF_KERNEL_SIZE / 2)] as i32;
                    let p2 = HALF_KERNEL[HALF_KERNEL_SIZE - 1 - kernel_index] as i32;
                    let p3 = HALF_KERNEL[kernel_index + HALF_KERNEL_SIZE / 2] as i32;
                    let p4 = HALF_KERNEL[kernel_index] as i32;

                    (s1 * p1 + s2 * p2 + s3 * p3 + s4 * p4) >> 11
                },
                ResamplingMode::Accurate => {
                    let kernel_index = ((self.sample_pos & 0xFFF) >> 4) as usize;
                    let p1 = ACCURATE_KERNEL[ACCURATE_KERNEL_SIZE - 1 - (kernel_index + ACCURATE_KERNEL_SIZE / 2)] as i32;
                    let p2 = ACCURATE_KERNEL[ACCURATE_KERNEL_SIZE - 1 - kernel_index] as i32;
                    let p3 = ACCURATE_KERNEL[kernel_index + ACCURATE_KERNEL_SIZE / 2] as i32;
                    let p4 = ACCURATE_KERNEL[kernel_index] as i32;

                    let c1 = (s1 * p1) >> 11;
                    let c2 = (s2 * p2) >> 11;
                    let c3 = (s3 * p3) >> 11;
                    let c4 = (s4 * p4) >> 11;

                    ((((c1 + c2 + c3) & 0xFFFF) as i16) as i32) + c4
                }
            };
            dsp_helpers::clamp(resampled) & !1
        } else {
            ((noise << 1) as i16) as i32
        };

        sample = ((sample * self.envelope.level) >> 11) & !1;
        let envx_value = (self.envelope.level >> 4) as u8;

        if self.brr_decoder.is_end && !self.brr_decoder.is_looping {
            self.envelope.key_off();
            self.envelope.level = 0;
        }

        if self.every_other_sample {
            if self.kof_queued {
                self.envelope.key_off();
            }
            if self.kon_queued {
                self.kon_queued = false;
                self.envelope.key_on();
                self.kon_delay = 5;
            }
        }

        if self.kon_delay == 0 {
            self.envelope.tick();
        }

        // voice4
        let mut looped = false;
        if self.sample_pos >= 0x4000 {
            self.decode_brr_samples();

            if self.brr_decoder.is_finished() {
                if self.brr_decoder.is_end {
                    self.sample_address = next_sample_address;
                    self.edge_hit = true;
                    looped = true;
                } else {
                    self.sample_address += 9;
                }
                self.read_brr_header();
                self.sample_block_index += 1;
            }
        }

        self.sample_pos = (self.sample_pos & 0x3fff) + pitch;
        if self.sample_pos > 0x7fff {
            self.sample_pos = 0x7fff;
        }

        // voice5
        if looped {
            self.endx_latch = true;
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

        // voice6
        let outx_value = ((sample >> 8) as i8) as u8;

        // voice7
        self.endx_bit = self.endx_latch;

        // voice8
        self.outx_value = outx_value;

        // voice9
        self.envx_value = envx_value;

        ret
    }

    pub fn set_pitch_high(&mut self, value: u8) {
        self.pitch_high = value & 0x3f;
    }

    fn read_entry(&mut self) {
        self.sample_start_address = self.dsp().read_source_dir_start_address(self.source as i32);
        self.loop_start_address = self.dsp().read_source_dir_loop_address(self.source as i32);
    }

    fn read_brr_header(&mut self) {
        self.brr_decoder.read_header(self.emulator().read_u8(self.sample_address));
        self.sample_offset = 0;
    }

    fn decode_brr_samples(&mut self) {
        debug_assert!(self.sample_offset < 7, "OOB sample access: offset={}", self.sample_offset);
        let buf = vec![
            self.emulator().read_u8(self.sample_address + self.sample_offset + 1),
            self.emulator().read_u8(self.sample_address + self.sample_offset + 2)
        ];
        self.sample_offset += 2;

        self.brr_decoder.read(&buf);

        for _ in 0..4 {
            self.resample_buffer[self.resample_buffer_pos] = self.brr_decoder.read_next_sample() as i32;
            self.resample_buffer_pos = (self.resample_buffer_pos + 1) % RESAMPLE_BUFFER_LEN;
        }
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
