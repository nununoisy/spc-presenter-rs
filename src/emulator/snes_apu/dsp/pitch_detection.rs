use super::dsp::Dsp;
use super::brr_block_decoder::BrrBlockDecoder;
use crate::sample_processing::{sample_loudness, Yin};

const PDA_WINDOW_LENGTH: usize = 2048;
const PDA_WINDOW_STEP: usize = 512;

#[derive(Clone)]
pub struct VoicePitch {
    pub pitch: f64,
    pub piecewise_pitch: Vec<f64>,
    pub clarity: f64,
    pub loudness: Vec<f64>
}

impl VoicePitch {
    pub fn pitch_at(&self, sample_block_index: usize) -> f64 {
        let index = sample_block_index as f64 * 16.0 / PDA_WINDOW_STEP as f64;
        let left_index = index.floor() as usize;
        let right_index = index.ceil() as usize;
        let delta = index.ceil() - index;

        let mut left = self.piecewise_pitch.get(left_index).cloned().unwrap_or(self.pitch);
        let mut right = self.piecewise_pitch.get(right_index).cloned().unwrap_or(self.pitch);

        (left * delta) + (right * (1.0 - delta))
    }

    pub fn loudness_at(&self, sample_block_index: usize) -> f64 {
        let index = sample_block_index as f64 * 16.0 / PDA_WINDOW_STEP as f64;
        let left_index = index.floor() as usize;
        let right_index = index.ceil() as usize;
        let delta = index.ceil() - index;

        let mut left = self.loudness.get(left_index).cloned().unwrap_or(0.0);
        let mut right = self.loudness.get(right_index).cloned().unwrap_or(0.0);

        (left * delta) + (right * (1.0 - delta))
    }
}

pub fn temporal_sample_pitch(sample: &[f64], base_pitch: f64, base_clarity: f64) -> VoicePitch {
    if sample.len() <= PDA_WINDOW_LENGTH {
        return VoicePitch {
            pitch: base_pitch,
            piecewise_pitch: vec![],
            clarity: base_clarity,
            loudness: vec![1.0]
        }
    }

    let mut yin = Yin::new(55.0, 4000.0, 32000.0, PDA_WINDOW_LENGTH, None, Some(PDA_WINDOW_STEP)).unwrap();
    let result = yin.pyin(sample, Default::default(), None, None);

    let clarity = result
        .iter()
        .map(|x| x.periodicity / result.len() as f64)
        .sum::<f64>();

    let pitches: Vec<f64> = result
        .into_iter()
        .map(|x| x.f_0)
        .collect();

    let median_pitch = pitches[pitches.len() / 2];

    let loudness = sample_loudness(sample, 32000.0, PDA_WINDOW_LENGTH, PDA_WINDOW_STEP);

    VoicePitch {
        pitch: median_pitch,
        piecewise_pitch: pitches,
        clarity,
        loudness
    }
}

impl Dsp {
    pub(super) fn detect_voice_pitch(&mut self, channel: usize) -> (f64, f64) {
        if self.voices[channel].noise_on {
            const C_0: f64 = 16.351597831287;

            return (C_0 * (2.0_f64).powf((self.noise_clock as f64) / 12.0), 1.0);
        }

        let source = self.voices[channel].source;
        let sample_block_index = self.voices[channel].sample_block_index;

        if !self.source_pitches.contains_key(&source) {
            let mut decoded_sample: Vec<f64> = Vec::new();
            let mut sample_address = self.read_source_dir_start_address(source as i32);
            let loop_address = self.read_source_dir_loop_address(source as i32);

            let mut brr_block_decoder = BrrBlockDecoder::new();
            let mut loop_count = 0;
            let mut start_block_count = 0;
            let mut loop_block_count = 0;

            brr_block_decoder.reset(0, 0);

            loop {
                let mut buf = [0; 9];
                for i in 0..9 {
                    buf[i] = self.emulator().read_u8(sample_address + i as u32);
                }
                brr_block_decoder.read(&buf);
                sample_address += 9;

                match loop_count {
                    0 => start_block_count += 1,
                    1 => loop_block_count += 1,
                    _ => ()
                };

                while !brr_block_decoder.is_finished() {
                    decoded_sample.push(brr_block_decoder.read_next_sample() as f64);
                }

                if brr_block_decoder.is_end {
                    if brr_block_decoder.is_looping && decoded_sample.len() < (30 * 32000) {
                        sample_address = loop_address;
                        loop_count += 1;
                    } else {
                        break;
                    }
                }
            }

            let mut yin = Yin::new(55.0, 4000.0, 32000.0, decoded_sample.len(), None, None).unwrap();
            let yin_result = yin.yin(&decoded_sample, Some(0.2))[0];

            let (base_pitch, base_clarity) = if yin_result.voiced {
                (yin_result.f_0, yin_result.periodicity)
            } else {
                let mut period_blocks = match loop_block_count {
                    0 => start_block_count,
                    _ => loop_block_count
                }.max(1);

                while period_blocks > 16 {
                    period_blocks /= 2;
                }

                println!("WARNING: YIN failure! Assuming base period is {} BRR blocks", period_blocks);

                (32000.0 / (period_blocks * 16) as f64, 0.0)
            };

            let voice_pitch = temporal_sample_pitch(&decoded_sample, base_pitch, base_clarity);

            println!("Detected new source ${:x}, f_0={} Hz, clarity={}, length={}:{}", source, voice_pitch.pitch, voice_pitch.clarity, start_block_count, loop_block_count);
            self.source_pitches.insert(source, voice_pitch);
        }

        (self.source_pitches.get(&source).unwrap().pitch_at(sample_block_index), self.source_pitches.get(&source).unwrap().loudness_at(sample_block_index))
    }
}
