pub mod render_options;

use anyhow::{Result};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use ringbuf::{HeapRb, Rb};
use ringbuf::ring_buffer::RbBase;
use render_options::RendererOptions;
use crate::emulator::Emulator;
use crate::renderer::render_options::StopCondition;
use crate::video_builder;
use crate::video_builder::VideoBuilder;
use crate::visualizer::Visualizer;

pub struct Renderer {
    options: RendererOptions,
    emulator: Emulator,
    viz: Arc<Mutex<Visualizer>>,
    vb: VideoBuilder,

    cur_frame: u64,
    encode_start: Instant,
    frame_timestamp: f64,
    frame_times: HeapRb<f64>,
    loop_count: u64,
    loop_duration: Option<u64>,
    fadeout_timer: Option<u64>,
    expected_duration: Option<usize>
}

impl Renderer {
    pub fn new(options: RendererOptions) -> Result<Self> {
        let emulator = Emulator::from_spc(options.input_path.clone(), options.video_options.sample_rate as u32)?;
        let viz = Arc::new(Mutex::new(Visualizer::new(
            8,
            options.video_options.resolution_in.0,
            options.video_options.resolution_in.1,
            32000,
            options.config.piano_roll.clone(),
            options.sample_tunings.clone()
        )));

        let mut video_options = options.video_options.clone();

        if let Some(metadata) = emulator.get_spc_metadata() {
            video_options.metadata.insert("title".to_string(), metadata.title);
            video_options.metadata.insert("artist".to_string(), metadata.artist);
            video_options.metadata.insert("album".to_string(), metadata.game);
            video_options.metadata.insert("comment".to_string(), "Encoded with SPCPresenter".to_string());
        }

        let vb = VideoBuilder::new(video_options)?;

        Ok(Self {
            options: options.clone(),
            emulator,
            viz,
            vb,
            cur_frame: 0,
            encode_start: Instant::now(),
            frame_timestamp: 0.0,
            frame_times: HeapRb::new(600),
            loop_count: 0,
            loop_duration: None,
            fadeout_timer: None,
            expected_duration: None
        })
    }

    pub fn start_encoding(&mut self) -> Result<()> {
        self.emulator.init();

        if !self.options.script700_path.is_empty() {
            self.emulator.load_script700(&self.options.script700_path)?;
        }

        self.emulator.set_state_receiver(Some(self.viz.clone()));
        self.emulator.set_resampling_mode(self.options.config.emulator.resampling_mode);
        self.emulator.set_filter_enabled(self.options.config.emulator.filter_enabled);
        self.emulator.set_frame_delay(6);

        if !self.options.per_sample_colors.is_empty() {
            self.viz.lock().unwrap().settings_manager_mut().put_per_sample_colors(self.options.per_sample_colors.clone());
        }

        let roi_w = self.options.video_options.resolution_out.0 as i32;
        let roi_h = self.options.config.piano_roll.waveform_height as i32 + self.options.config.piano_roll.key_length as i32;
        self.vb.set_roi_region(0, 0, roi_w, roi_h);

        self.vb.start_encoding()?;
        self.encode_start = Instant::now();

        Ok(())
    }

    pub fn step(&mut self) -> Result<bool> {
        self.emulator.step()?;

        {
            let mut viz = self.viz.lock().unwrap();
            viz.draw();
            self.vb.push_video_data(viz.get_canvas_buffer())?;
        }

        if let Some(audio) = self.emulator.get_audio_samples(Some(self.vb.audio_frame_size())) {
            let adjusted_audio = match self.fadeout_timer {
                Some(t) => {
                    let volume_divisor = (self.options.fadeout_length as f64 / t as f64) as i16;
                    audio.iter().map(|s| s / volume_divisor).collect()
                },
                None => audio
            };
            self.vb.push_audio_data(video_builder::as_u8_slice(&adjusted_audio))?;
        }

        self.vb.step_encoding()?;

        let elapsed_secs = self.elapsed().as_secs_f64();
        let frame_time = elapsed_secs - self.frame_timestamp;
        self.frame_timestamp = elapsed_secs;

        self.frame_times.push_overwrite(frame_time);

        self.expected_duration = self.next_expected_duration();
        self.fadeout_timer = self.next_fadeout_timer();

        if let Some(t) = self.fadeout_timer {
            if t == 0 {
                return Ok(false)
            }
        }

        self.cur_frame += 1;
        Ok(true)
    }

    pub fn finish_encoding(&mut self) -> Result<()> {
        self.vb.finish_encoding()?;

        Ok(())
    }

    pub fn current_frame(&self) -> u64 {
        self.cur_frame
    }

    pub fn elapsed(&self) -> Duration {
        self.encode_start.elapsed()
    }

    fn next_expected_duration(&self) -> Option<usize> {
        if self.expected_duration.is_some() {
            return self.expected_duration;
        }

        match self.options.stop_condition {
            StopCondition::Frames(stop_frames) => Some((stop_frames + self.options.fadeout_length) as usize),
            StopCondition::Loops(stop_loop_count) => {
                match self.loop_duration {
                    Some(d) => Some(self.options.fadeout_length as usize + d as usize * stop_loop_count),
                    None => None
                }
            },
            StopCondition::SpcDuration => {
                Some((self.emulator.get_spc_metadata()?.duration_frames + self.options.fadeout_length) as usize)
            }
        }
    }

    fn next_fadeout_timer(&self) -> Option<u64> {
        match self.fadeout_timer {
            Some(0) => Some(0),
            Some(t) => Some(t - 1),
            None => {
                // if self.last_position.end {
                //     return Some(self.options.fadeout_length);
                // }

                match self.options.stop_condition {
                    StopCondition::Loops(stop_loop_count) => {
                        if self.loop_count >= stop_loop_count as u64 {
                            Some(self.options.fadeout_length)
                        } else {
                            None
                        }
                    },
                    StopCondition::Frames(stop_frames) => {
                        if self.current_frame() >= stop_frames {
                            Some(self.options.fadeout_length)
                        } else {
                            None
                        }
                    },
                    StopCondition::SpcDuration => {
                        let duration = match self.emulator.get_spc_metadata() {
                            Some(metadata) => metadata.duration_frames,
                            None => panic!("SPC file does not have duration tag")
                        };

                        if self.current_frame() >= duration {
                            Some(self.options.fadeout_length)
                        } else {
                            None
                        }
                    }
                }
            }
        }
    }

    pub fn loop_count(&self) -> u64 {
        self.loop_count
    }

    pub fn instantaneous_fps(&self) -> u32 {
        match self.frame_times.iter().last().cloned() {
            Some(ft) => (1.0 / ft) as u32,
            None => 0
        }
    }

    pub fn average_fps(&self) -> u32 {
        if self.frame_times.is_empty() {
            return 0;
        }
        (self.frame_times.len() as f64 / self.frame_times.iter().sum::<f64>()) as u32
    }

    pub fn encode_rate(&self) -> f64 {
        self.average_fps() as f64 / 60.0
    }

    pub fn encoded_duration(&self) -> Duration {
        self.vb.encoded_video_duration()
    }

    pub fn encoded_size(&self) -> usize {
        self.vb.encoded_video_size()
    }

    pub fn expected_duration_frames(&self) -> Option<usize> {
        self.expected_duration
    }

    pub fn expected_duration(&self) -> Option<Duration> {
        match self.expected_duration {
            Some(d) => {
                let secs = d as f64 / 60.0;
                Some(Duration::from_secs_f64(secs))
            },
            None => None
        }
    }

    pub fn eta_duration(&self) -> Option<Duration> {
        match self.expected_duration {
            Some(expected_duration) => {
                let remaining_frames = expected_duration - self.current_frame() as usize;
                let average_fps = u32::max(self.average_fps(), 1) as f64;
                let remaining_secs = remaining_frames as f64 / average_fps;
                Some(Duration::from_secs_f64(self.elapsed().as_secs_f64() + remaining_secs))
            },
            None => None
        }
    }
}
