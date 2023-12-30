use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::rc::Rc;
use std::thread;
use std::time::Duration;
use anyhow::{Result, anyhow};
use crate::emulator::{ApuStateReceiver, Emulator, BrrSample};
use super::{sample_loudness, util, Yin};

const F_MIN: f64 = 62.5;
const F_MAX: f64 = 4000.0;
const SAMPLE_RATE: f64 = 32000.0;
const BASE_PITCH_TROUGH_THRESHOLD: f64 = 0.2;
const FRAME_LENGTH: usize = 2048;
const HOP_LENGTH: usize = 256;

#[derive(Clone)]
pub struct SampleData {
    sample: BrrSample,
    base_pitch: f64,
    temporal_pitch: Vec<f64>,
    custom_pitch: Option<f64>,
    loudness: Vec<f64>
}

impl Default for SampleData {
    fn default() -> Self {
        Self {
            sample: BrrSample::new(),
            base_pitch: 0.0,
            temporal_pitch: vec![],
            custom_pitch: None,
            loudness: vec![]
        }
    }
}

fn process_sample(signal: &[f64], start_block_count: usize, loop_block_count: usize) -> Result<(f64, f64)> {
    if signal.len() >= 64 {
        let mut yin = Yin::new(F_MIN, F_MAX, SAMPLE_RATE, signal.len(), None, None)?;
        let yin_result = yin.yin(signal, Some(BASE_PITCH_TROUGH_THRESHOLD));
        let yin_result = yin_result
            .get(0)
            .ok_or(anyhow!("YIN did not return any results"))?;

        if yin_result.voiced {
            return Ok((yin_result.f_0, yin_result.periodicity))
        }
    }

    let mut period_blocks = match loop_block_count {
        0 => start_block_count,
        _ => loop_block_count
    }.max(1);

    while period_blocks > 16 {
        period_blocks /= 2;
    }

    let f_0 = 32000.0 / (period_blocks * 16) as f64;

    println!("WARNING: YIN did not return a suitable candidate! Assuming base period is {} BRR blocks (f_0={} Hz)", period_blocks, f_0);

    Ok((f_0, 0.0))
}

fn process_sample_temporal(signal: &[f64], base_pitch: f64, base_clarity: f64) -> Result<(f64, f64, Vec<f64>, Vec<f64>)> {
    let loudness = sample_loudness(signal, SAMPLE_RATE, FRAME_LENGTH, HOP_LENGTH);

    if signal.len() <= FRAME_LENGTH {
        return Ok((base_pitch, base_clarity, vec![], loudness));
    }

    let mut yin = Yin::new(F_MIN, F_MAX, SAMPLE_RATE, FRAME_LENGTH, None, Some(HOP_LENGTH))?;
    let result = yin.pyin(signal, Default::default(), None, None);

    let clarity_norm = 1.0 / result.len() as f64;
    let clarity = result
        .iter()
        .map(|frame| frame.periodicity * clarity_norm)
        .sum::<f64>();

    let temporal_pitch: Vec<f64> = result
        .into_iter()
        .map(|frame| frame.f_0)
        .collect();
    let median_pitch = temporal_pitch[temporal_pitch.len() / 2];

    Ok((median_pitch, clarity, temporal_pitch, loudness))
}

impl SampleData {
    pub fn new(sample: BrrSample, sample_count: usize) -> Result<Self> {
        let signal: Vec<f64> = sample
            .clone()
            .into_iter()
            .take(sample_count)
            .map(|x| x as f64)
            .collect();

        let (base_pitch, base_clarity) = process_sample(&signal, sample.start_block_count(), sample.loop_block_count())?;
        let (base_pitch, base_clarity, temporal_pitch, loudness) = process_sample_temporal(&signal, base_pitch, base_clarity)?;

        println!("Sample analysis results: f_0={} Hz, clarity={}", base_pitch, base_clarity);

        Ok(Self {
            sample,
            base_pitch,
            temporal_pitch,
            custom_pitch: None,
            loudness
        })
    }

    pub fn sample(&self) -> &BrrSample {
        &self.sample
    }

    pub fn base_pitch(&self) -> f64 {
        self.base_pitch
    }

    pub fn pitch_at(&self, sample_block_index: usize) -> f64 {
        if let Some(pitch) = self.custom_pitch {
            return pitch;
        }

        let x = sample_block_index as f64 * 16.0 / HOP_LENGTH as f64;
        util::linear_interpolate(&self.temporal_pitch, x, self.base_pitch)
    }

    pub fn loudness_at(&self, sample_block_index: usize) -> f64 {
        let x = sample_block_index as f64 * 16.0 / HOP_LENGTH as f64;
        util::linear_interpolate(&self.loudness, x, 0.0)
    }

    pub fn set_custom_tuning(&mut self, custom_pitch: Option<f64>) {
        self.custom_pitch = custom_pitch;
    }
}

pub enum SampleProcessorProgress {
    DetectingSamples { current_frame: usize, total_frames: usize, detected_samples: usize },
    ProcessingSamples { current_sample: usize, total_samples: usize, source: u8 },
    Finished
}

struct SampleDetector(HashMap<u8, usize>);

impl SampleDetector {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn sources(&mut self) -> Vec<(u8, usize)> {
        let result = self.0.clone().into_iter().collect();
        self.0.clear();
        result
    }
}

impl ApuStateReceiver for SampleDetector {
    fn receive(
        &mut self,
        _channel: usize,
        source: u8,
        muted: bool,
        _envelope_level: i32,
        volume: (i8, i8),
        amplitude: (i32, i32),
        _pitch: u16,
        _noise_clock: Option<u8>,
        _edge: bool,
        _kon_frames: usize,
        sample_block_index: usize
    ) {
        if muted || (volume.0 == 0 && volume.1 == 0 && amplitude.0 == 0 && amplitude.1 == 0) {
            return;
        }

        self.0.entry(source)
            .and_modify(|last_block_index| {
                if *last_block_index < sample_block_index + 1 {
                    *last_block_index = sample_block_index + 1;
                }
            })
            .or_insert(sample_block_index + 1);
    }
}

pub struct SampleProcessor {
    emulator: Emulator,
    total_frames: usize,
    current_frame: usize,
    current_sample: usize,
    sample_data: HashMap<u8, SampleData>,
    sample_detector: Rc<RefCell<SampleDetector>>,
    detected_sources: HashMap<u8, usize>,
    processing_queue: VecDeque<(u8, BrrSample)>
}

impl SampleProcessor {
    pub fn from_spc<P: AsRef<Path>>(spc_path: P) -> Result<Self> {
        let mut emulator = Emulator::from_spc(spc_path, 44_100)?;
        emulator.init();

        let total_frames = match emulator.get_spc_metadata() {
            Some(spc_metadata) => spc_metadata.duration_frames as usize,
            None => 300 * 60
        };

        let sample_detector = Rc::new(RefCell::new(SampleDetector::new()));
        emulator.set_state_receiver(Some(sample_detector.clone()));

        Ok(Self {
            emulator,
            total_frames,
            current_frame: 0,
            current_sample: 0,
            sample_data: HashMap::new(),
            sample_detector,
            detected_sources: HashMap::new(),
            processing_queue: VecDeque::new()
        })
    }

    pub fn set_frame_count(&mut self, frame_count: usize) {
        self.total_frames = frame_count;
    }

    fn determine_progress(&self) -> SampleProcessorProgress {
        if self.current_frame < self.total_frames {
            SampleProcessorProgress::DetectingSamples {
                current_frame: self.current_frame,
                total_frames: self.total_frames,
                detected_samples: self.detected_sources.len()
            }
        } else if let Some((source, _sample)) = self.processing_queue.front() {
            SampleProcessorProgress::ProcessingSamples {
                current_sample: self.current_sample,
                total_samples: self.detected_sources.len(),
                source: *source
            }
        } else {
            SampleProcessorProgress::Finished
        }
    }

    pub fn step(&mut self) -> Result<SampleProcessorProgress> {
        if self.current_frame < self.total_frames {
            // Play more frames to detect samples
            self.emulator.step()?;
            self.current_frame += 1;

            for (source, max_length) in self.sample_detector.borrow_mut().sources() {
                if let Some(last_max_length) = self.detected_sources.get_mut(&source) {
                    if *last_max_length < max_length {
                        *last_max_length = max_length;
                    }
                    continue;
                }

                let sample = self.emulator.dump_sample(source);
                println!("Discovered new sample ${:x}, length={}:{} blocks", source, sample.start_block_count(), sample.loop_block_count());
                self.processing_queue.push_back((source, sample));
                self.detected_sources.insert(source, max_length);
            }

            if self.current_frame >= self.total_frames {
                thread::sleep(Duration::from_millis(500));
            }
        } else {
            // Process detected samples
            if let Some((source, sample)) = self.processing_queue.pop_front() {
                let sample_count = (self.detected_sources.get(&source).cloned().unwrap_or(60000) + 2000) * 16;
                println!("Processing sample ${:x} for {} samples...", source, sample_count);
                let sample_data = SampleData::new(sample, sample_count)?;
                self.sample_data.insert(source, sample_data);
                self.current_sample += 1;
            }
        }

        Ok(self.determine_progress())
    }

    pub fn finish(self) -> HashMap<u8, SampleData> {
        self.sample_data
    }
}
