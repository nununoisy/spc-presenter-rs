use std::iter;
use super::util;
use super::filter::{BiquadIIRFilter, FilterType};

const MIN_ACTIVE_WINDOWS: usize = 4;

pub fn sample_loudness(signal: &[f64], sample_rate: f64, frame_length: usize, hop_length: usize) -> Vec<f64> {
    // K-weighting filter chain to quantize the head effects. From ITU-R BS.1770-5
    let high_shelf = BiquadIIRFilter::new(FilterType::HighShelf, 4.0, 0.5_f64.sqrt(), 1500.0, sample_rate);
    let high_pass = BiquadIIRFilter::new(FilterType::HighPass, 0.0, 0.5, 38.0, sample_rate);

    let filtered_signal = high_shelf.process(&signal);
    let mut filtered_signal = high_pass.process(&filtered_signal);
    filtered_signal.extend(iter::repeat(0.0).take(frame_length));

    // Take the normalized moving root mean square
    let mut moving_rms = util::moving_rms(&filtered_signal, frame_length, hop_length);
    util::normalize_max(&mut moving_rms);

    let active_windows = moving_rms.iter().filter(|s| **s != 0.0).take(MIN_ACTIVE_WINDOWS).count();
    if active_windows < MIN_ACTIVE_WINDOWS {
        println!("Sample is too quiet for precise loudness");
        vec![1.0; MIN_ACTIVE_WINDOWS]
    } else {
        moving_rms
    }
}
