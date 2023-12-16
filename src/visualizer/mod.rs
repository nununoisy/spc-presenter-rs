mod filters;
pub mod channel_settings;
mod oscilloscope;
mod piano_roll;
mod tile_map;

use std::collections::HashMap;
use tiny_skia::{Color, Pixmap, Rect};
use channel_settings::{ChannelSettingsManager, ChannelSettings};
use filters::HighPassIIR;
use oscilloscope::OscilloscopeState;
use piano_roll::PianoRollState;
use crate::emulator::ApuStateReceiver;
use tile_map::TileMap;
use crate::config::PianoRollConfig;
use crate::sample_processing::SampleData;

pub const C_0: f64 = 16.351597831287;
pub const APU_STATE_BUF_SIZE: usize = 4096;
const FONT_IMAGE: &'static [u8] = include_bytes!("8x8_font.png");
const FONT_CHAR_MAP: &'static str = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~";

#[derive(Debug, Copy, Clone, Default)]
pub struct ChannelState {
    pub volume: f32,
    pub amplitude: f32,
    pub frequency: f64,
    pub timbre: usize,
    pub balance: f64,
    pub edge: bool,
    pub kon_frames: usize
}

pub struct Visualizer {
    channels: usize,
    canvas: Pixmap,
    config: PianoRollConfig,

    channel_last_states: Vec<ChannelState>,
    channel_filters: Vec<HighPassIIR>,
    oscilloscope_states: Vec<OscilloscopeState>,
    piano_roll_states: Vec<PianoRollState>,

    font: TileMap,
    oscilloscope_divider_cache: Option<(f32, Pixmap)>,
    sample_data: HashMap<u8, SampleData>
}

impl Visualizer {
    pub fn new(channels: usize, width: u32, height: u32, sample_rate: u32, config: PianoRollConfig, sample_data: HashMap<u8, SampleData>) -> Self {
        let mut oscilloscope_states: Vec<OscilloscopeState> = Vec::with_capacity(channels);
        let mut piano_roll_states: Vec<PianoRollState> = Vec::with_capacity(channels);
        for _ in 0..channels {
            oscilloscope_states.push(OscilloscopeState::new());
            piano_roll_states.push(PianoRollState::new(sample_rate as f32, config.speed_multiplier as f32 * 4.0, config.starting_octave as f32));
        }

        Self {
            channels,
            canvas: Pixmap::new(width, height).unwrap(),
            config,
            channel_last_states: vec![ChannelState::default(); channels],
            channel_filters: vec![HighPassIIR::new(sample_rate as f32, 300.0); channels],
            oscilloscope_states,
            piano_roll_states,
            font: TileMap::new(Pixmap::decode_png(FONT_IMAGE).unwrap(), 8, 8, FONT_CHAR_MAP),
            oscilloscope_divider_cache: None,
            sample_data
        }
    }

    pub fn get_canvas_buffer(&self) -> &[u8] {
        self.canvas.data()
    }

    pub fn clear(&mut self) {
        self.canvas.fill(Color::TRANSPARENT);
    }

    pub fn draw(&mut self) {
        self.clear();

        let oscilloscopes_pos = Rect::from_xywh(
            0.0,
            0.0,
            self.canvas.width() as f32,
            self.config.waveform_height as f32
        ).unwrap();

        let max_channels_per_row = if self.is_vertical_layout() {
            4
        } else {
            8
        };
        self.draw_oscilloscopes(oscilloscopes_pos, max_channels_per_row);

        let piano_roll_pos = Rect::from_xywh(
            0.0,
            oscilloscopes_pos.bottom(),
            self.canvas.width() as f32,
            self.canvas.height() as f32 - oscilloscopes_pos.height()
        ).unwrap();

        self.draw_piano_roll(piano_roll_pos);
    }

    pub fn settings_manager(&self) -> &ChannelSettingsManager {
        &self.config.settings
    }

    pub fn settings_manager_mut(&mut self) -> &mut ChannelSettingsManager {
        &mut self.config.settings
    }

    pub fn is_vertical_layout(&self) -> bool {
        self.canvas.height() > self.canvas.width()
    }
}

impl ApuStateReceiver for Visualizer {
    fn receive(
        &mut self,
        channel: usize,
        source: u8,
        muted: bool,
        envelope_level: i32,
        volume: (i8, i8),
        amplitude: (i32, i32),
        pitch: u16,
        noise_clock: Option<u8>,
        edge: bool,
        kon_frames: usize,
        sample_block_index: usize
    ) {
        let sample_data = self.sample_data.get(&source).expect("Missing sample data!");
        let source_pitch = sample_data.pitch_at(sample_block_index);
        let source_loudness = match noise_clock {
            Some(_) => 1.0,
            None => sample_data.loudness_at(sample_block_index)
        };
        
        let (l_volume, r_volume) = volume;
        let (l_amplitude, r_amplitude) = amplitude;
        
        let volume = if muted {
            0.0
        } else {
            let mean_volume = (l_volume as f32 / 2.0).abs() + (r_volume as f32 / 2.0).abs();
            (source_loudness as f32 * 2.8 * (mean_volume / 3.0 + 1.0).log2() * (envelope_level as f32 / 2047.0)).ceil()
        };

        let amplitude_pre = {
            if l_volume < 0 && r_volume > 0 {
                ((r_amplitude - l_amplitude) / 2) as i16
            } else if l_volume > 0 && r_volume < 0 {
                ((l_amplitude - r_amplitude) / 2) as i16
            } else {
                ((l_amplitude + r_amplitude) / 2) as i16
            }
        };

        let frequency = match noise_clock {
            Some(t) => C_0 * (t as f64 / 12.0).exp2(),
            None => source_pitch * pitch as f64 / 0x1000 as f64
        }.max(f64::EPSILON);

        let balance = ((l_volume as f64).abs() / -128.0) + ((r_volume as f64).abs() / 128.0) + 0.5;

        let filter = self.channel_filters.get_mut(channel).unwrap();
        filter.consume(amplitude_pre as f32);

        let settings = self.config.settings.settings(channel).unwrap();
        let timbre_max = settings.num_colors();

        let state = ChannelState {
            volume,
            amplitude: filter.output(),
            frequency,
            timbre: source as usize % timbre_max,
            balance,
            edge,
            kon_frames
        };

        self.oscilloscope_states[channel].consume(&state, settings);
        self.piano_roll_states[channel].consume(&state, settings);
        self.channel_last_states[channel] = state;
    }
}
