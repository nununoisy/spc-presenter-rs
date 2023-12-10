mod snes_apu;
mod resampler;
mod filter;

use anyhow::{Result};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::rc::Rc;
use spc::spc::{Id666Tag, Spc};
use snes_apu::apu::Apu;
pub use snes_apu::dsp::voice::ResamplingMode;

pub trait ApuStateReceiver {
    fn receive(
        &mut self,
        channel: usize,
        source: u8,
        muted: bool,
        envelope_level: i32,
        volume: (u8, u8),
        amplitude: (i32, i32),
        pitch: u16,
        noise_clock: Option<u8>,
        edge: bool,
        kon_frames: usize,
        sample_block_index: usize
    );
}

pub struct SpcMetadata {
    pub title: String,
    pub artist: String,
    pub game: String,
    pub duration_frames: u64,
    pub fadeout_frames: u64
}

pub struct Emulator {
    spc_file: Spc,
    apu: Box<Apu>,
    frame_count: usize,
    sample_buffer: VecDeque<i16>,
    resampler: resampler::Resampler,
    filter: filter::BlarggSpcFilter,
    filter_enabled: bool
}

impl Emulator {
    pub fn from_spc<P: AsRef<Path>>(spc_path: P) -> Result<Self> {
        let spc_file = Spc::load(spc_path)?;
        let apu = Apu::from_spc(&spc_file);

        Ok(Self {
            spc_file,
            apu,
            frame_count: 0,
            sample_buffer: VecDeque::new(),
            resampler: resampler::Resampler::new(44_100)?,
            filter: filter::BlarggSpcFilter::default(),
            filter_enabled: false
        })
    }

    pub fn set_filter_enabled(&mut self, filter_enabled: bool) {
        self.filter_enabled = filter_enabled;
    }

    pub fn init(&mut self) {
        self.apu.clear_echo_buffer();
        self.filter.clear();
    }

    pub fn step(&mut self) -> Result<()> {
        let sample_count = if self.frame_count % 3 == 0 { 534 } else { 533 };

        let mut l_sample_buffer = vec![0i16; sample_count];
        let mut r_sample_buffer = vec![0i16; sample_count];
        self.apu.render(&mut l_sample_buffer, &mut r_sample_buffer, sample_count as i32);

        let mut combined_sample_buffer = self.resampler.run(&l_sample_buffer, &r_sample_buffer)?;
        if self.filter_enabled {
            self.filter.run(&mut combined_sample_buffer)?;
        }
        self.sample_buffer.extend(combined_sample_buffer.iter());

        self.frame_count += 1;

        Ok(())
    }

    pub fn get_audio_samples(&mut self, frame_size: Option<usize>) -> Option<Vec<i16>> {
        match frame_size {
            Some(frame_size) => {
                if self.sample_buffer.len() < frame_size * 2 {
                    return None;
                }
                let result: Vec<i16> = self.sample_buffer.drain(0..(frame_size * 2)).collect();
                Some(result)
            },
            None => {
                let result: Vec<i16> = self.sample_buffer.clone().into_iter().collect();
                self.sample_buffer.clear();
                Some(result)
            }
        }
    }

    pub fn set_state_receiver(&mut self, state_receiver: Option<Rc<RefCell<dyn ApuStateReceiver>>>) {
        self.apu.dsp.as_mut().unwrap().state_receiver = state_receiver;
    }

    pub fn set_resampling_mode(&mut self, resampling_mode: ResamplingMode) {
        self.apu.dsp.as_mut().unwrap().set_resampling_mode(resampling_mode);
    }

    pub fn get_spc_metadata(&self) -> Option<SpcMetadata> {
        if self.spc_file.id666_tag.is_none() {
            return None;
        }

        let title = self.spc_file.id666_tag.as_ref().unwrap().song_title.clone();
        let artist = self.spc_file.id666_tag.as_ref().unwrap().artist_name.clone();
        let game = self.spc_file.id666_tag.as_ref().unwrap().game_title.clone();
        let duration_frames = 60 * (self.spc_file.id666_tag.as_ref().unwrap().seconds_to_play_before_fading_out as u64);
        let fadeout_frames = 60 * (self.spc_file.id666_tag.as_ref().unwrap().fade_out_length as u64) / 1000;

        Some(SpcMetadata {
            title,
            artist,
            game,
            duration_frames,
            fadeout_frames
        })
    }

    pub fn dump_sample(&mut self, source: u8, sample_count: usize) -> (Vec<i16>, usize, usize) {
        self.apu.dump_sample(source, sample_count)
    }
}
