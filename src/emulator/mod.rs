mod resampler;
mod filter;
mod brr_sample;

use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use spc_spcp::spc::Spc;
use snes_apu_spcp::{Apu, ApuStateReceiver, ResamplingMode};
pub use brr_sample::{BrrSample, BrrSampleBuilder};

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
    frame_delay: usize,
    sample_buffer: VecDeque<i16>,
    resampler: resampler::Resampler,
    filter: filter::BlarggSpcFilter,
    filter_enabled: bool
}

impl Emulator {
    pub fn from_spc<P: AsRef<Path>>(spc_path: P, sample_rate: u32) -> Result<Self> {
        let spc_file = Spc::load(spc_path)?;
        let apu = Apu::from_spc(&spc_file);

        Ok(Self {
            spc_file,
            apu,
            frame_count: 0,
            frame_delay: 0,
            sample_buffer: VecDeque::new(),
            resampler: resampler::Resampler::new(sample_rate)?,
            filter: filter::BlarggSpcFilter::default(),
            filter_enabled: false
        })
    }

    pub fn load_script700<P: AsRef<Path>>(&mut self, script_path: P) -> Result<()> {
        Ok(self.apu.load_script700(script_path)?)
    }

    pub fn init(&mut self) {
        self.apu.clear_echo_buffer();
        self.filter.clear();
    }

    pub fn step(&mut self) -> Result<()> {
        let sample_count = if self.frame_count % 3 == 0 { 534 } else { 533 };

        let mut l_sample_buffer = vec![0i16; sample_count];
        let mut r_sample_buffer = vec![0i16; sample_count];
        if self.frame_count >= self.frame_delay {
            self.apu.render(&mut l_sample_buffer, &mut r_sample_buffer, sample_count as i32);
        }

        let mut combined_sample_buffer = self.resampler.run(&l_sample_buffer, &r_sample_buffer)?;
        if self.filter_enabled {
            self.filter.run(&mut combined_sample_buffer);
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

    pub fn set_state_receiver(&mut self, state_receiver: Option<Arc<Mutex<dyn ApuStateReceiver>>>) {
        self.apu.set_state_receiver(state_receiver);
    }

    pub fn set_frame_delay(&mut self, frame_delay: usize) {
        self.frame_delay = frame_delay;
    }

    pub fn set_resampling_mode(&mut self, resampling_mode: ResamplingMode) {
        self.apu.set_resampling_mode(resampling_mode);
    }

    pub fn set_filter_enabled(&mut self, filter_enabled: bool) {
        self.filter_enabled = filter_enabled;
    }

    pub fn get_spc_metadata(&self) -> Option<SpcMetadata> {
        if self.spc_file.id666_tag.is_none() {
            return None;
        }

        let title = self.spc_file.id666_tag.as_ref().unwrap().song_title.clone();
        let artist = self.spc_file.id666_tag.as_ref().unwrap().artist_name.clone();
        let game = self.spc_file.id666_tag.as_ref().unwrap().game_title.clone();
        let duration_frames = (60.0 * self.spc_file.id666_tag.as_ref().unwrap().play_time.as_secs_f64()).round() as u64;
        let fadeout_frames = (60.0 * self.spc_file.id666_tag.as_ref().unwrap().fadeout_time.as_secs_f64()).round() as u64;

        Some(SpcMetadata {
            title,
            artist,
            game,
            duration_frames,
            fadeout_frames
        })
    }

    pub fn dump_sample(&mut self, source: u8) -> BrrSample {
        let mut result = BrrSampleBuilder::new();

        let (mut sample_address, loop_address) = self.apu.read_sample_directory(source);
        println!("Dumping sample ${:x}, start=${:04x}, loop=${:04x}", source, sample_address, loop_address);
        let mut did_loop = false;
        let mut buf = [0u8; 9];

        loop {
            for i in 0..9 {
                buf[i] = self.apu.read_u8(sample_address + i as u32);
            }
            sample_address += 9;

            let loop_flag = (buf[0] & 0b0000_0010) != 0;
            let end_flag = (buf[0] & 0b0000_0001) != 0;

            if did_loop {
                result.add_loop_block(&buf);
            } else {
                result.add_start_block(&buf);
            }

            if end_flag {
                if loop_flag && !did_loop {
                    sample_address = loop_address;
                    did_loop = true;
                } else {
                    break;
                }
            }
        }

        result.simplify();
        result.into_inner()
    }
}
