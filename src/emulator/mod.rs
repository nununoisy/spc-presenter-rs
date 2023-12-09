mod snes_apu;
mod resampler;
mod filter;

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::rc::Rc;
use spc::spc::{Id666Tag, Spc};
use snes_apu::apu::Apu;
pub use snes_apu::dsp::voice::ResamplingMode;
use crate::emulator::snes_apu::dsp::pitch_detection::VoicePitch;

pub trait ApuStateReceiver {
    fn receive(&mut self, channel: usize, volume: u8, amplitude: i16, frequency: f64, timbre: usize, balance: f64, edge: bool, kon_frames: usize);
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
    pub fn from_spc<P: AsRef<Path>>(spc_path: P) -> Result<Self, String> {
        let spc_file = Spc::load(spc_path)
            .map_err(|e| format!("Failed to load SPC! {}", e))?;
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

    pub fn step(&mut self) -> Result<(), String> {
        let sample_count = if self.frame_count % 3 == 0 { 534 } else { 533 };

        let mut l_sample_buffer = vec![0i16; sample_count];
        let mut r_sample_buffer = vec![0i16; sample_count];
        self.apu.render(&mut l_sample_buffer, &mut r_sample_buffer, sample_count as i32);

        let mut combined_sample_buffer: Vec<i16> = Vec::new();
        for sample in self.resampler.run(&l_sample_buffer, &r_sample_buffer)? {
            combined_sample_buffer.push(sample);
        }
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
                let result: Vec<_> = self.sample_buffer.drain(0..(frame_size * 2)).collect();
                Some(result)
            },
            None => {
                let result: Vec<_> = self.sample_buffer.clone().into_iter().collect();
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

    pub fn set_manual_sample_tuning(&mut self, source: u8, pitch: f64) {
        self.apu.dsp.as_mut().unwrap().source_pitches.insert(source, VoicePitch {
            pitch,
            piecewise_pitch: vec![],
            clarity: 1.0,
            loudness: vec![1.0]
        });
    }
}
