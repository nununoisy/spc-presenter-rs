mod oscilloscope;
pub mod channel_settings;
mod tile_map;
mod filters;
mod piano_roll;

use ringbuf::{HeapRb, Rb, StaticRb};
use raqote::{DrawTarget, SolidSource};
use crate::emulator::ApuStateReceiver;
use crate::visualizer::channel_settings::ChannelSettingsManager;
use crate::visualizer::filters::HighPassIIR;
use crate::visualizer::tile_map::TileMap;

#[derive(Copy, Clone, Default)]
pub struct ChannelState {
    pub channel: usize,
    pub volume: u8,
    pub amplitude: f32,
    pub frequency: f64,
    pub timbre: usize,
    pub balance: f64,
    pub edge: bool,
    pub kon_frames: usize
}

pub struct Visualizer {
    canvas: DrawTarget,
    settings: ChannelSettingsManager,
    channel_states: Vec<HeapRb<ChannelState>>,
    channel_filters: Vec<HighPassIIR>,
    state_slices: HeapRb<ChannelState>,
    font: TileMap
}

const APU_STATE_BUF_SIZE: usize = 8192;
const FONT_IMAGE: &'static [u8] = include_bytes!("8x8_font.png");
const FONT_CHAR_MAP: &'static str = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~";

impl Visualizer {
    pub fn new() -> Self {
        let mut channel_states: Vec<HeapRb<ChannelState>> = Vec::new();
        let mut channel_filters: Vec<HighPassIIR> = Vec::new();
        for _ in 0..8 {
            channel_states.push(HeapRb::new(APU_STATE_BUF_SIZE));
            channel_filters.push(HighPassIIR::new(44100.0, 300.0));
        }

        Self {
            canvas: DrawTarget::new(960, 540),
            settings: ChannelSettingsManager::default(),
            channel_states,
            channel_filters,
            state_slices: HeapRb::new(APU_STATE_BUF_SIZE),
            font: TileMap::new(FONT_IMAGE, 8, 8, FONT_CHAR_MAP).unwrap()
        }
    }

    /// Get canvas buffer as BGRA data (little endian) or ARGB data (big endian)
    pub fn get_canvas_buffer(&self) -> Vec<u8> {
        self.canvas.get_data_u8().to_vec()
    }

    pub fn clear(&mut self) {
        self.canvas.clear(SolidSource::from_unpremultiplied_argb(0, 0, 0, 0));
    }

    pub fn settings_manager(&self) -> ChannelSettingsManager {
        self.settings.clone()
    }

    pub fn settings_manager_mut(&mut self) -> &mut ChannelSettingsManager {
        &mut self.settings
    }
}

impl ApuStateReceiver for Visualizer {
    fn receive(&mut self, channel: usize, volume: u8, amplitude: i16, frequency: f64, timbre: usize, balance: f64, edge: bool, kon_frames: usize) {
        const C_0: f64 = 16.351597831287;

        let buf = self.channel_states.get_mut(channel).unwrap();
        let filter = self.channel_filters.get_mut(channel).unwrap();

        filter.consume(amplitude as f32);

        let timbre_max = self.settings.settings(channel).num_colors();

        let state = ChannelState {
            channel,
            volume,
            amplitude: filter.output(),
            frequency, //: frequency.max(C_0),
            timbre: timbre % timbre_max,
            balance,
            edge,
            kon_frames
        };

        buf.push_overwrite(state);
    }
}