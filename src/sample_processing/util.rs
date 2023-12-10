use std::cmp::Ordering;
use std::mem;
use rustfft::num_complex::Complex;
use super::simd_sum::SimdSumAdapter;

pub fn complex_frames(signal: &[f64], frame_length: usize, hop_length: usize) -> Vec<Vec<Complex<f64>>> {
    (0..=(signal.len() - frame_length))
        .step_by(hop_length)
        .map(|t| {
            signal.iter()
                .skip(t)
                .take(frame_length)
                .map(|x| Complex::new(*x, 0.0))
                .collect()
        })
        .collect()
}

pub fn next_minimum(signal: &[f64], threshold: f64) -> (usize, bool) {
    match signal.iter().position(|&v| v < threshold) {
        Some(below_threshold_index) => (
            signal.windows(2)
                .enumerate()
                .skip(below_threshold_index)
                .find(|(_, s)| s[0] < s[1])
                .map(|(i, _)| i)
                .unwrap_or(below_threshold_index),
            true
        ),
        None => (
            signal.iter()
                .enumerate()
                .min_by(|(_, &a), (_, b)| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                .map(|(i, _)| i)
                .unwrap_or(0),
            false
        )
    }
}

pub fn linear_interpolate(signal: &[f64], x: f64, padding: f64) -> f64 {
    if x.fract() == 0.0 {
        return signal.get(x as usize).cloned().unwrap_or(padding);
    }

    let x0 = x.floor();
    let x1 = x.ceil();
    let y0 = signal.get(x0 as usize).cloned().unwrap_or(padding);
    let y1 = signal.get(x1 as usize).cloned().unwrap_or(padding);

    (y0 * (x1 - x) + y1 * (x - x0)) / (x1 - x0)
}

pub fn parabolic_interpolate(signal: &[f64], x1: usize) -> f64 {
    if x1 >= signal.len() {
        return x1 as f64;
    }

    let x0 = x1.saturating_sub(1);
    let x2 = (x1 + 1).min(signal.len() - 1);

    if x0 == x1 {
        if signal[x1] <= signal[x2] {
            x1 as f64
        } else {
            x2 as f64
        }
    } else if x2 == x1 {
        if signal[x1] <= signal[x0] {
            x1 as f64
        } else {
            x0 as f64
        }
    } else {
        let s0 = signal[x0];
        let s1 = signal[x1];
        let s2 = signal[x2];

        x1 as f64 + (s2 - s0) / (2.0 * (2.0 * s1 - s2 - s0))
    }
}

pub fn normalize(signal: &mut [f64]) {
    let signal_sum = signal.simd_sum::<16>();

    if signal_sum > 0.0 {
        let d = 1.0 / signal_sum;
        for x in signal.iter_mut() {
            *x *= d;
        }
    } else {
        signal.fill(1.0 / signal.len() as f64);
    }
}

pub fn normalize_max(signal: &mut [f64]) {
    let signal_max = signal
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
        .cloned()
        .unwrap_or(0.0);

    if signal_max != 0.0 {
        let d = 1.0 / signal_max;
        for x in signal.iter_mut() {
            *x *= d;
        }
    }
}

pub fn localized_transition_matrix(state_count: usize, transition_width: usize) -> Vec<(usize, usize, f64)> {
    let mut result: Vec<(usize, usize, f64)> = Vec::with_capacity(state_count * transition_width * 4);

    for state in 0..state_count {
        let theoretical_min_next_state = state as f64 - (transition_width as f64 / 2.0);
        let min_next_state = state.saturating_sub(transition_width / 2);
        let max_next_state = (state + (transition_width / 2)).min(state_count - 1);

        let mut weights: Vec<f64> = (min_next_state..max_next_state)
            .map(|i| {
                if i <= state {
                    i as f64 - theoretical_min_next_state + 1.0
                } else {
                    state as f64 - theoretical_min_next_state + 1.0 - (i - state) as f64
                }
            })
            .collect();
        normalize(&mut weights);

        for (i, weight) in weights.into_iter().enumerate() {
            result.push((state, i + min_next_state, weight * 0.99));
            result.push((state, i + min_next_state + state_count, weight * 0.01));
            result.push((state + state_count, i + min_next_state, weight * 0.99));
            result.push((state + state_count, i + min_next_state + state_count, weight * 0.01));
        }
    }

    result
}

pub fn viterbi<P, V>(prob: P, init: &[f64], transition: &[(usize, usize, f64)]) -> Vec<usize>
where
    P: AsRef<[V]>,
    V: AsRef<[f64]>
{
    let mut psi: Vec<Vec<usize>> = vec![vec![0; init.len()]];
    let mut path: Vec<usize> = vec![init.len() - 1; prob.as_ref().len()];

    let mut t_1: Vec<f64> = init.iter()
        .enumerate()
        .map(|(i, &b)| b * prob.as_ref()[0].as_ref()[i])
        .collect();
    normalize(&mut t_1);

    let mut t_2: Vec<f64> = vec![0.0; init.len()];

    for frame in 1..prob.as_ref().len() {
        psi.push(vec![0; init.len()]);

        for (from_state, to_state, transition_prob) in transition.iter().cloned() {
            let current_value = t_1[from_state] * transition_prob;
            if current_value > t_2[to_state] {
                t_2[to_state] = current_value;
                psi[frame][to_state] = from_state;
            }
        }

        for (state, t) in t_2.iter_mut().enumerate() {
            *t *= prob.as_ref()[frame].as_ref()[state];
        }
        normalize(&mut t_2);

        mem::swap(&mut t_1, &mut t_2);
        t_2.fill(0.0);
    }

    let mut best_value = 0.0;
    for (state, value) in t_1.into_iter().enumerate() {
        if value > best_value {
            best_value = value;
            *path.last_mut().unwrap() = state;
        }
    }

    if path.len() > 1 {
        for frame in (0..=(path.len() - 2)).rev() {
            path[frame] = psi[frame + 1][path[frame + 1]];
        }
    }

    path
}

pub fn moving_rms(signal: &[f64], frame_length: usize, hop_length: usize) -> Vec<f64> {
    let reciprocal = 1.0 / frame_length as f64;
    let mean_square: Vec<f64> = signal.iter()
        .map(|x| (*x).powi(2) * reciprocal)
        .collect();

    (0..=(signal.len() - frame_length))
        .step_by(hop_length)
        .map(|t| {
            mean_square[t..(t + frame_length)]
                .simd_sum::<16>()
                .sqrt()
        })
        .collect()
}

