use std::iter;
use anyhow::{Result, ensure};
use rustfft::{FftPlanner, num_complex::Complex};
use crate::sample_processing::simd_sum::SimdSumAdapter;
use crate::sample_processing::threshold_distribution::ThresholdDistribution;
use crate::sample_processing::util;

#[derive(Debug, Copy, Clone)]
pub struct YinFrame {
    pub f_0: f64,
    pub voiced: bool,
    pub periodicity: f64
}

pub struct Yin {
    f_min: f64,
    f_max: f64,
    sample_rate: f64,
    frame_length: usize,
    win_length: usize,
    hop_length: usize,

    planner: FftPlanner<f64>
}

impl Yin {
    pub fn new(
        f_min: f64,
        f_max: f64,
        sample_rate: f64,
        frame_length: usize,
        win_length: Option<usize>,
        hop_length: Option<usize>
    ) -> Result<Self> {
        let win_length = win_length.unwrap_or(frame_length / 2);
        let hop_length = hop_length.unwrap_or(frame_length / 4);

        ensure!(f_min > 0.0, "Min. frequency must be positive");
        ensure!(f_max > f_min, "Max frequency must be greater than min. frequency");
        ensure!(sample_rate > 0.0, "Sample rate must be positive");
        ensure!(frame_length > 0, "Frame length must be positive");
        ensure!(win_length > 0, "Window length must be positive");
        ensure!(hop_length > 0, "Hop length must be positive");

        Ok(Self {
            f_min,
            f_max,
            sample_rate,
            frame_length,
            win_length,
            hop_length,

            planner: FftPlanner::new()
        })
    }

    fn cumulative_mean_normalized_difference(&mut self, signal: &[f64], min_period: usize, max_period: usize) -> Vec<Vec<f64>> {
        let fft = self.planner.plan_fft_forward(self.frame_length);
        let ifft = self.planner.plan_fft_inverse(self.frame_length);

        let fft_scratch_len = fft.get_inplace_scratch_len();
        let ifft_scratch_len = ifft.get_inplace_scratch_len();
        let mut scratch: Vec<Complex<f64>> = vec![Complex::new(0.0, 0.0); fft_scratch_len.max(ifft_scratch_len)];
        let mut kernel: Vec<Complex<f64>> = vec![Complex::new(0.0, 0.0); self.frame_length];

        let mut result: Vec<Vec<f64>> = Vec::new();

        for mut frame in util::complex_frames(signal, self.frame_length, self.hop_length) {
            // Compute energy terms
            let mut energy_terms: Vec<f64> = frame
                .iter()
                .take(self.win_length)
                .map(|&x| x.re.powi(2))
                .collect();
            let first_energy_term = energy_terms.simd_sum::<16>();
            for i in 0..self.win_length {
                energy_terms[i] = match i {
                    0 => first_energy_term,
                    _ => energy_terms[i - 1] - frame[i - 1].re.powi(2) + frame[i + self.win_length].re.powi(2)
                };
            }

            // Create a kernel from half of the framed signal data
            kernel[..self.win_length].copy_from_slice(&frame[..self.win_length]);
            kernel[self.win_length..].fill(Complex::new(0.0, 0.0));

            // Compute the DFT of the frame and kernel for convolution
            fft.process_with_scratch(&mut frame, &mut scratch[..fft_scratch_len]);
            fft.process_with_scratch(&mut kernel, &mut scratch[..fft_scratch_len]);
            // Backwards normalize the results
            let norm_const = Complex::new(1.0 / self.frame_length as f64, 0.0);
            // Convolve the frame and kernel in the frequency domain
            for (f, h) in iter::zip(frame.iter_mut(), kernel.iter()) {
                *f *= norm_const * h.conj();
            }
            ifft.process_with_scratch(&mut frame, &mut scratch[..ifft_scratch_len]);

            // Cumulative mean normalized difference function
            let mut yin_frame: Vec<f64> = vec![0.0; (min_period..=max_period).count()];
            let mut cum_sum = 0.0;

            for (tau, (frame_term, energy_term)) in iter::zip(&frame[0..=max_period], &energy_terms[0..=max_period]).enumerate() {
                let difference = first_energy_term + energy_term - 2.0 * frame_term.re;
                cum_sum += difference;

                if tau < min_period {
                    continue;
                }

                let cum_mean = match tau {
                    0 => 1.0,
                    _ => (cum_sum / tau as f64) + f64::EPSILON
                };

                yin_frame[tau - min_period] = difference / cum_mean;
            }

            result.push(yin_frame);
        }

        result
    }

    pub fn yin(&mut self, signal: &[f64], trough_threshold: Option<f64>) -> Vec<YinFrame> {
        let min_period = (self.sample_rate / self.f_max).floor() as usize;
        let max_period = ((self.sample_rate / self.f_min).ceil() as usize)
            .min(self.frame_length - self.win_length - 1);
        let trough_threshold = trough_threshold.unwrap_or(0.1);

        let mut result: Vec<YinFrame> = Vec::new();

        for yin_frame in self.cumulative_mean_normalized_difference(signal, min_period, max_period) {
            let (x1, hit_threshold) = util::next_minimum(&yin_frame, trough_threshold);
            let f_0 = self.sample_rate / (util::parabolic_interpolate(&yin_frame, x1) + min_period as f64);
            let periodicity = 1.0 - yin_frame[x1];

            debug_assert!(f_0 >= self.f_min, "f_0 too small! {} < {}", f_0, self.f_min);
            debug_assert!(f_0 <= self.f_max, "f_0 too big! {} > {}", f_0, self.f_max);

            result.push(YinFrame {
                f_0,
                voiced: hit_threshold,
                periodicity
            });
        }

        result
    }

    pub fn pyin(&mut self, signal: &[f64], prior: ThresholdDistribution, resolution: Option<f64>, max_transition_rate: Option<f64>) -> Vec<YinFrame> {
        let min_period = (self.sample_rate / self.f_max).floor() as usize;
        let max_period = ((self.sample_rate / self.f_min).ceil() as usize)
            .min(self.frame_length - self.win_length - 1);
        let distribution = prior.distribution();

        // Compute the number of pitch bins
        let resolution = resolution.unwrap_or(0.1);
        let pitch_bins_per_semitone = (1.0 / resolution).ceil();
        let pitch_bin_count = (12.0 * pitch_bins_per_semitone * (self.f_max / self.f_min).log2()).floor() as usize + 1;

        // Compute the emission matrix
        let mut emission_matrix: Vec<Vec<f64>> = Vec::new();
        let mut voiced_probs: Vec<f64> = Vec::new();
        for yin_frame in self.cumulative_mean_normalized_difference(signal, min_period, max_period) {
            let mut peak_prob: Vec<f64> = vec![0.0; yin_frame.len()];

            let mut threshold_index = distribution.len() - 1;
            let mut index = 2;
            let mut min_index = 0;
            let mut min_val = 42.0;
            while index < yin_frame.len() {
                if yin_frame[index] < (0.01 * (threshold_index + 1) as f64) {
                    index += util::next_minimum(&yin_frame[index..], f64::MAX).0;

                    if yin_frame[index] < min_val && index > 2 {
                        min_val = yin_frame[index];
                        min_index = index;
                    }

                    peak_prob[index] += distribution[threshold_index];
                    if threshold_index == 0 {
                        break;
                    }
                    threshold_index -= 1;
                } else {
                    index += 1;
                }
            }

            if min_index > 0 {
                peak_prob[min_index] += (1.0 - peak_prob.simd_sum::<16>()) * 0.01;
            }

            let mut voiced_prob_states: Vec<f64> = vec![0.0; 2 * pitch_bin_count];
            for (x1, p) in peak_prob.into_iter().enumerate() {
                // Filter out any zero probabilities
                if p == 0.0 {
                    continue;
                }

                // Refine the frequency candidate by parabolic interpolation
                let f_0 = self.sample_rate / (util::parabolic_interpolate(&yin_frame, x1) + min_period as f64);
                if f_0 < self.f_min || f_0 > self.f_max {
                    continue;
                }

                // Determine the pitch bin for this candidate
                let pitch_bin = (12.0 * pitch_bins_per_semitone * (f_0 / self.f_min).log2())
                    .round()
                    .clamp(0.0, pitch_bin_count as f64) as usize;
                // The voiced probability state for this candidate is just p
                voiced_prob_states[pitch_bin] = p;
            }

            // Compute the unvoiced probability states
            let voiced_prob = voiced_prob_states[..pitch_bin_count]
                .simd_sum::<16>()
                .clamp(0.0, 1.0);
            voiced_prob_states[pitch_bin_count..].fill((1.0 - voiced_prob) / pitch_bin_count as f64);

            emission_matrix.push(voiced_prob_states);
            voiced_probs.push(voiced_prob);
        }

        // Compute the transition matrix
        let max_transition_rate = max_transition_rate.unwrap_or(35.92);
        let max_semitones_per_frame = (max_transition_rate * 12.0 * self.hop_length as f64 / self.sample_rate).round() as usize;
        let transition_width = max_semitones_per_frame * pitch_bins_per_semitone as usize + 1;
        let transition_matrix = util::localized_transition_matrix(pitch_bin_count, transition_width);

        // Compute the initial observations
        let viterbi_init: Vec<f64> = vec![1.0 / pitch_bin_count as f64; 2 * pitch_bin_count];

        // Viterbi decode the observation probabilities
        let path = util::viterbi(&emission_matrix, &viterbi_init, &transition_matrix);

        let mut result: Vec<YinFrame> = Vec::new();

        for (state, pitch_bin) in path.into_iter().enumerate() {
            let f_0 = self.f_min * ((pitch_bin as f64).rem_euclid(pitch_bin_count as f64) / (12.0 * pitch_bins_per_semitone)).exp2();
            let voiced = pitch_bin < pitch_bin_count;
            let periodicity = 1.0 - voiced_probs[state];

            debug_assert!(f_0 >= self.f_min, "f_0 too small! {} < {}", f_0, self.f_min);
            debug_assert!(f_0 <= self.f_max, "f_0 too big! {} > {}", f_0, self.f_max);

            result.push(YinFrame {
                f_0,
                voiced,
                periodicity
            })
        }

        result
    }
}
