use crate::dsp::stereo::{Stereo, StereoChannel};
use super::brr_stream_decoder::BrrStreamDecoder;
use super::dsp::Dsp;
use super::super::apu::Apu;
use super::envelope::Envelope;
use super::dsp_helpers;
use super::interpolation_tables::{interp_dot, ACCURATE_GAUSSIAN_KERNEL, GAUSSIAN_KERNEL, CUBIC_SPLINE_KERNEL, SINC_KERNEL};

const RESAMPLE_BUFFER_LEN: usize = 12;

#[derive(Clone, Copy, Default, PartialEq)]
pub enum ResamplingMode {
    Linear,
    Gaussian,
    Cubic,
    Sinc,
    #[default]
    Accurate
}

#[derive(Clone, Copy, Default)]
pub struct VoiceOutput {
    pub left_out: i32,
    pub right_out: i32,
    pub last_voice_out: i32,
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

    pub volume: Stereo<u8>,
    pub pitch_low: u8,
    pub pitch_high: u8,
    pub source: u8,
    pub l_source: u8,
    pub pitch_mod: bool,
    pub l_pitch_mod: bool,
    pub noise_on: bool,
    pub l_noise_on: bool,
    pub echo_on: bool,
    pub l_echo_on: bool,

    sample_start_address: u32,
    loop_start_address: u32,
    next_sample_address: u32,
    brr_decoder: BrrStreamDecoder,
    sample_address: u32,
    sample_offset: u32,
    sample_pos: i32,
    pub(super) sample_block_index: usize,

    pub(super) edge_hit: bool,
    pub(super) sample_frame: usize,
    endx_bit: bool,
    l_endx_bit: bool,
    looped: bool,
    pub outx_value: u8,
    pub envx_value: u8,
    l_envx_value: u8,
    kon_delay: u8,

    pub resampling_mode: ResamplingMode,
    resample_buffer: [i32; 2 * RESAMPLE_BUFFER_LEN],
    resample_buffer_pos: usize,

    pub(super) amplitude: Stereo<i32>,
    pub output_buffer: VoiceBuffer,
    pub is_muted: bool,
    pub is_solod: bool,

    pub kon: bool,
    pub l_kon: bool,
    pub kof: bool,
    pub l_kof: bool
}

impl Voice {
    pub fn new(dsp: *mut Dsp, emulator: *mut Apu, resampling_mode: ResamplingMode) -> Voice {
        Voice {
            dsp,
            emulator,

            envelope: Envelope::new(dsp),

            volume: Stereo::default(),
            pitch_low: 0,
            pitch_high: 0,
            source: 0,
            l_source: 0,
            pitch_mod: false,
            l_pitch_mod: false,
            noise_on: false,
            l_noise_on: false,
            echo_on: false,
            l_echo_on: false,

            sample_start_address: 0,
            loop_start_address: 0,
            next_sample_address: 0,
            brr_decoder: BrrStreamDecoder::new(),
            sample_address: 0,
            sample_offset: 0,
            sample_pos: 0,
            sample_block_index: 0,

            edge_hit: false,
            sample_frame: 0,
            endx_bit: false,
            l_endx_bit: false,
            looped: false,
            outx_value: 0,
            envx_value: 0,
            l_envx_value: 0,
            kon_delay: 0,

            resampling_mode,
            resample_buffer: [0; 2 * RESAMPLE_BUFFER_LEN],
            resample_buffer_pos: 0,

            amplitude: Stereo::default(),
            output_buffer: VoiceBuffer::new(),
            is_muted: false,
            is_solod: false,

            kon: false,
            l_kon: false,
            kof: false,
            l_kof: false
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

    fn output(&mut self, channel: StereoChannel) {
        let amp = dsp_helpers::multiply_volume(self.dsp().l_output, self.volume.into_inner(channel));

        let master = dsp_helpers::clamp(dsp_helpers::cast_arb_int(self.dsp().master_output.into_inner(channel) + amp, 17));
        self.dsp().master_output.set(channel, master);
        if self.echo_on {
            let echo = dsp_helpers::clamp(dsp_helpers::cast_arb_int(self.dsp().echo_output.into_inner(channel) + amp, 17));
            self.dsp().echo_output.set(channel, echo);
        }
        self.amplitude.set(channel, amp);
    }

    pub fn voice1(&mut self) {
        // voice1
        self.sample_start_address = self.dsp().read_source_dir_start_address(self.source as i32);
        self.loop_start_address = self.dsp().read_source_dir_loop_address(self.source as i32);
        self.source = self.l_source;
    }

    pub fn voice2(&mut self) {
        // voice2
        self.next_sample_address = if self.kon_delay != 0 {
            self.sample_start_address
        } else {
            self.loop_start_address
        };
        self.dsp().l_adsr0 = self.envelope.l_adsr0;
        self.dsp().l_pitch = self.pitch_low as i32;
    }

    pub fn voice3a(&mut self) {
        // voice3a
        self.dsp().l_pitch |= (self.pitch_high as i32) << 8;
    }

    pub fn voice3b(&mut self) {
        // voice3b
        self.brr_decoder.read_first_byte(self.emulator().read_u8(self.sample_address + self.sample_offset + 1));
        self.brr_decoder.read_header(self.emulator().read_u8(self.sample_address));
    }

    pub fn voice3c(&mut self) {
        // voice3c
        if self.pitch_mod {
            self.dsp().l_pitch += ((self.dsp().l_output >> 5) * self.dsp().l_pitch) >> 10;
        }
        self.dsp().l_pitch = dsp_helpers::cast_arb_uint(self.dsp().l_pitch, 15);

        if self.kon_delay > 0 {
            if self.kon_delay == 5 {
                self.sample_address = self.next_sample_address;
                self.sample_offset = 0;
                self.resample_buffer_pos = 0;
                self.sample_block_index = 0;
                self.sample_frame = 0;
                self.edge_hit = true;
                self.brr_decoder.read_header(0);
                self.brr_decoder.reset();
            }

            self.envelope.reset_level();

            self.kon_delay -= 1;
            self.sample_pos = if (self.kon_delay & 3) != 0 { 0x4000 } else { 0 };

            self.dsp().l_pitch = 0;
        };

        let sample = if !self.noise_on {
            let base_pos = (self.resample_buffer_pos + (self.sample_pos >> 12) as usize) % RESAMPLE_BUFFER_LEN;
            let resampled = match self.resampling_mode {
                ResamplingMode::Linear => {
                    let p1 = (self.sample_pos & 0xFFF) as i16;
                    let p2 = 0x1000 - p1;
                    interp_dot(&self.resample_buffer[base_pos..(base_pos + 2)], &[p1, p2]) >> 12
                },
                ResamplingMode::Gaussian => {
                    let kernel_index = (self.sample_pos & 0xFFC) as usize;
                    interp_dot(&self.resample_buffer[base_pos..(base_pos + 4)], &GAUSSIAN_KERNEL[kernel_index..(kernel_index + 4)]) >> 11
                },
                ResamplingMode::Cubic => {
                    let kernel_index = ((self.sample_pos & 0xFF0) >> 2) as usize;
                    interp_dot(&self.resample_buffer[base_pos..(base_pos + 4)], &CUBIC_SPLINE_KERNEL[kernel_index..(kernel_index + 4)]) >> 15
                },
                ResamplingMode::Sinc => {
                    let kernel_index = ((self.sample_pos & 0xFF0) >> 1) as usize;
                    interp_dot(&self.resample_buffer[base_pos..(base_pos + 8)], &SINC_KERNEL[kernel_index..(kernel_index + 8)]) >> 15
                },
                ResamplingMode::Accurate => {
                    let kernel_index = ((self.sample_pos & 0xFF0) >> 2) as usize;
                    let mut sum = 0;
                    for i in 0..4 {
                        sum += (self.resample_buffer[base_pos + i] * ACCURATE_GAUSSIAN_KERNEL[kernel_index + i] as i32) >> 11;
                        if i == 2 {
                            sum = ((sum & 0xFFFF) as i16) as i32;
                        }
                    }
                    sum
                }
            };
            dsp_helpers::clamp(resampled) & !1
        } else {
            dsp_helpers::cast_arb_int(self.dsp().noise << 1, 16)
        };

        self.dsp().l_output = ((sample * self.envelope.level) >> 11) & !1;
        self.l_envx_value = ((self.envelope.level >> 4) & 0xFF) as u8;

        if self.dsp().master_reset || (self.brr_decoder.is_end && !self.brr_decoder.is_looping) {
            self.envelope.key_off();
            self.envelope.level = 0;
        }

        if self.dsp().every_other_sample {
            if self.kof {
                self.envelope.key_off();
            }
            if self.kon {
                self.envelope.key_on();
                self.kon_delay = 5;
            }
        }

        if self.kon_delay == 0 {
            self.envelope.tick(self.dsp().l_adsr0);
        }
    }

    pub fn voice3(&mut self) {
        self.voice3a();
        self.voice3b();
        self.voice3c();
    }

    pub fn voice4(&mut self) {
        // voice4
        self.looped = false;
        if self.sample_pos >= 0x4000 {
            self.decode_brr_samples();

            if self.brr_decoder.is_finished() {
                if self.brr_decoder.is_end {
                    self.sample_address = self.next_sample_address;
                    self.edge_hit = true;
                    self.looped = true;
                } else {
                    self.sample_address += 9;
                }
                self.brr_decoder.reset();
                self.sample_offset = 0;
                self.sample_block_index += 1;
            }
        }

        self.sample_pos = (self.sample_pos & 0x3fff) + self.dsp().l_pitch;
        if self.sample_pos > 0x7fff {
            self.sample_pos = 0x7fff;
        }

        self.output(StereoChannel::Left);
    }

    pub fn voice5(&mut self) {
        // voice5
        self.output(StereoChannel::Right);

        if self.looped {
            self.l_endx_bit = true;
        }
        if self.kon_delay == 5 {
            self.l_endx_bit = false;
        }
    }

    pub fn voice6(&mut self) {
        // voice6
        self.dsp().l_outx = ((self.dsp().l_output >> 8) & 0xFF) as u8;
    }

    pub fn voice7(&mut self) {
        // voice7
        self.endx_bit = self.l_endx_bit;
        self.dsp().l_envx = self.l_envx_value;
    }

    pub fn voice8(&mut self) {
        // voice8
        self.outx_value = self.dsp().l_outx;
    }

    pub fn voice9(&mut self) {
        // voice9
        self.envx_value = self.dsp().l_envx;
    }

    pub fn misc27(&mut self) {
        self.pitch_mod = self.l_pitch_mod;
    }

    pub fn misc28(&mut self) {
        self.noise_on = self.l_noise_on;
        self.echo_on = self.l_echo_on;
    }

    pub fn misc29(&mut self) {
        if self.dsp().every_other_sample && self.kon {
            self.l_kon = false;
        }
    }

    pub fn misc30(&mut self) {
        if self.dsp().every_other_sample {
            self.kon = self.l_kon;
            self.kof = self.l_kof;
        }
    }

    pub fn set_pitch_high(&mut self, value: u8) {
        self.pitch_high = value & 0x3f;
    }

    fn decode_brr_samples(&mut self) {
        debug_assert!(self.sample_offset < 7, "OOB sample access: offset={}", self.sample_offset);
        let second_byte = self.emulator().read_u8(self.sample_address + self.sample_offset + 2);
        self.sample_offset += 2;

        self.brr_decoder.read(second_byte);

        for _ in 0..4 {
            let next_sample = self.brr_decoder.read_next_sample() as i32;
            self.resample_buffer[self.resample_buffer_pos] = next_sample;
            self.resample_buffer[self.resample_buffer_pos + RESAMPLE_BUFFER_LEN] = next_sample;
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
    }

    pub fn tick_latches(&mut self) {
        self.source = self.l_source;
        self.pitch_mod = self.l_pitch_mod;
        self.noise_on = self.l_noise_on;
        self.echo_on = self.l_echo_on;
    }
}
