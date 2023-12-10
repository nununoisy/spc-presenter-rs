use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::rc::Rc;
use anyhow::{Result, anyhow};
use crate::emulator::{ApuStateReceiver, Emulator};
use super::{sample_loudness, util, Yin};

const F_MIN: f64 = 55.0;
const F_MAX: f64 = 4000.0;
const SAMPLE_RATE: f64 = 32000.0;
const BASE_PITCH_TROUGH_THRESHOLD: f64 = 0.2;
const FRAME_LENGTH: usize = 2048;
const HOP_LENGTH: usize = 256;

#[derive(Clone)]
pub struct SampleData {
    signal: Vec<i16>,
    base_pitch: f64,
    temporal_pitch: Vec<f64>,
    custom_pitch: Option<f64>,
    loudness: Vec<f64>
}

fn process_sample(signal: &[f64], start_block_count: usize, loop_block_count: usize) -> Result<(f64, f64)> {
    let mut yin = Yin::new(F_MIN, F_MAX, SAMPLE_RATE, signal.len(), None, None)?;
    let yin_result = yin.yin(signal, Some(BASE_PITCH_TROUGH_THRESHOLD));
    let yin_result = yin_result
        .get(0)
        .ok_or(anyhow!("YIN did not return any results"))?;

    if yin_result.voiced {
        Ok((yin_result.f_0, yin_result.periodicity))
    } else {
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
}

fn process_sample_temporal(signal: &[f64], base_pitch: f64, base_clarity: f64) -> Result<(f64, f64, Vec<f64>, Vec<f64>)> {
    if signal.len() <= FRAME_LENGTH {
        return Ok((base_pitch, base_clarity, vec![], vec![]));
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

    let loudness = sample_loudness(signal, SAMPLE_RATE, FRAME_LENGTH, HOP_LENGTH);

    Ok((median_pitch, clarity, temporal_pitch, loudness))
}

impl SampleData {
    pub fn new(signal: &[i16], start_block_count: usize, loop_block_count: usize) -> Result<Self> {
        let signal_f64: Vec<f64> = signal
            .iter()
            .map(|x| *x as f64)
            .collect();

        let (base_pitch, base_clarity) = process_sample(&signal_f64, start_block_count, loop_block_count)?;
        let (base_pitch, base_clarity, temporal_pitch, loudness) = process_sample_temporal(&signal_f64, base_pitch, base_clarity)?;

        println!("Sample analysis results: f_0={} Hz, clarity={}", base_pitch, base_clarity);

        Ok(Self {
            signal: signal.to_vec(),
            base_pitch,
            temporal_pitch,
            custom_pitch: None,
            loudness
        })
    }

    pub fn signal(&self) -> Vec<i16> {
        self.signal.clone()
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
        util::linear_interpolate(&self.loudness, x, 1.0)
    }

    pub fn set_custom_tuning(&mut self, custom_pitch: Option<f64>) {
        self.custom_pitch = custom_pitch;
    }
}

pub enum SampleProcessorProgress {
    DetectingSamples { current_frame: usize, total_frames: usize, detected_samples: usize },
    ProcessedSample { current_sample: usize, total_samples: usize, source: u8 },
    Finished
}

struct SampleDetector(HashSet<u8>);

impl SampleDetector {
    pub fn new() -> Self {
        Self(HashSet::new())
    }

    pub fn sources(&mut self) -> Vec<u8> {
        let mut result = self.0.iter().cloned().collect();
        self.0.clear();
        result
    }
}

impl ApuStateReceiver for SampleDetector {
    fn receive(
        &mut self,
        _channel: usize,
        source: u8,
        _muted: bool,
        _envelope_level: i32,
        _volume: (u8, u8),
        _amplitude: (i32, i32),
        _pitch: u16,
        _noise_clock: Option<u8>,
        _edge: bool,
        _kon_frames: usize,
        _sample_block_index: usize
    ) {
        self.0.insert(source);
    }
}

pub struct SampleProcessor {
    emulator: Emulator,
    total_frames: usize,
    current_frame: usize,
    total_samples: Option<usize>,
    current_sample: usize,
    sample_data: HashMap<u8, SampleData>,
    sample_detector: Rc<RefCell<SampleDetector>>,
    detected_sources: HashSet<u8>,
    processing_queue: VecDeque<(u8, Vec<i16>, usize, usize)>
}

impl SampleProcessor {
    pub fn from_spc<P: AsRef<Path>>(spc_path: P) -> Result<Self> {
        let mut emulator = Emulator::from_spc(spc_path)?;
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
            total_samples: None,
            current_sample: 0,
            sample_data: HashMap::new(),
            sample_detector,
            detected_sources: HashSet::new(),
            processing_queue: VecDeque::new()
        })
    }

    pub fn step(&mut self) -> Result<SampleProcessorProgress> {
        if self.current_frame < self.total_frames {
            // Play more frames to detect samples
            self.emulator.step()?;
            self.current_frame += 1;

            for source in self.sample_detector.borrow_mut().sources() {
                if self.detected_sources.contains(&source) {
                    continue;
                }

                let (sample, start_block_count, loop_block_count) = self.emulator.dump_sample(source, 30 * 32000);
                println!("Discovered new sample ${:x}, length={}:{} blocks", source, start_block_count, loop_block_count);
                self.processing_queue.push_back((source, sample, start_block_count, loop_block_count));
                self.detected_sources.insert(source);
            }

            Ok(SampleProcessorProgress::DetectingSamples {
                current_frame: self.current_frame,
                total_frames: self.total_frames,
                detected_samples: self.detected_sources.len()
            })
        } else {
            // Process detected samples
            if self.total_samples.is_none() {
                self.total_samples = Some(self.detected_sources.len());
            }
            let total_samples = self.total_samples.unwrap();

            if let Some((source, sample, start_block_count, loop_block_count)) = self.processing_queue.pop_front() {
                let sample_data = SampleData::new(&sample, start_block_count, loop_block_count)?;
                self.sample_data.insert(source, sample_data);
                self.current_sample += 1;

                Ok(SampleProcessorProgress::ProcessedSample {
                    current_sample: self.current_sample,
                    total_samples,
                    source,
                })
            } else {
                Ok(SampleProcessorProgress::Finished)
            }
        }
    }

    pub fn finish(self) -> HashMap<u8, SampleData> {
        self.sample_data
    }
}
