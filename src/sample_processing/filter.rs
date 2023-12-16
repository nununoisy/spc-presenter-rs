use std::f64::consts::PI;
use rustfft::num_complex::ComplexFloat;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FilterType {
    LowPass,
    HighPass,
    BandPass,
    Notch,
    Peak,
    LowShelf,
    HighShelf
}

#[derive(Debug, Clone)]
pub struct BiquadIIRFilter {
    filter_type: FilterType,
    gain: f64,
    q_factor: f64,
    f_c: f64,
    sample_rate: f64,

    a0: f64,
    a1: f64,
    a2: f64,
    b1: f64,
    b2: f64
}

impl BiquadIIRFilter {
    pub fn new(filter_type: FilterType, gain: f64, q_factor: f64, f_c: f64, sample_rate: f64) -> Self {
        let mut result = Self {
            filter_type,
            gain,
            q_factor,
            f_c,
            sample_rate,

            a0: 1.0,
            a1: 0.0,
            a2: 0.0,
            b1: 0.0,
            b2: 0.0,
        };

        result.recalculate_filter_coefficients();

        result
    }

    fn recalculate_filter_coefficients(&mut self) {
        let v = (self.gain.abs() / 20.0).expf(10.0);
        let k = (PI * self.f_c).tan();
        let kk = k.powi(2);
        let (c1, c2) = if self.gain >= 0.0 {
            // boost
            (1.0, v)
        } else {
            // cut
            (v, 1.0)
        };

        match self.filter_type {
            FilterType::LowPass => {
                let norm = 1.0 / (1.0 + k / self.q_factor + kk);
                self.a0 = kk * norm;
                self.a1 = 2.0 * self.a0;
                self.a2 = self.a0;
                self.b1 = 2.0 * (kk - 1.0) * norm;
                self.b2 = (1.0 - k / self.q_factor + kk) * norm;
            }
            FilterType::HighPass => {
                let norm = 1.0 / (1.0 + k / self.q_factor + kk);
                self.a0 = norm;
                self.a1 = -2.0 * self.a0;
                self.a2 = self.a0;
                self.b1 = 2.0 * (kk - 1.0) * norm;
                self.b2 = (1.0 - k / self.q_factor + kk) * norm;
            }
            FilterType::BandPass => {
                let norm = 1.0 / (1.0 + k / self.q_factor + kk);
                self.a0 = k / self.q_factor * norm;
                self.a1 = 0.0;
                self.a2 = -self.a0;
                self.b1 = 2.0 * (kk - 1.0) * norm;
                self.b2 = (1.0 - k / self.q_factor + kk) * norm;
            }
            FilterType::Notch => {
                let norm = 1.0 / (1.0 + k / self.q_factor + kk);
                self.a0 = (kk + 1.0) * norm;
                self.a1 = 2.0 * (kk - 1.0) * norm;
                self.a2 = self.a0;
                self.b1 = self.a1;
                self.b2 = (1.0 - k / self.q_factor + kk) * norm;
            }
            FilterType::Peak => {
                let norm = 1.0 / (1.0 + (c1 / self.q_factor) * k + kk);
                self.a0 = (1.0 + (c2 / self.q_factor) * k + kk) * norm;
                self.a1 = 2.0 * (kk - 1.0) * norm;
                self.a2 = (1.0 - (c2 / self.q_factor) * k + kk) * norm;
                self.b1 = self.a1;
                self.b2 = (1.0 - (c1 / self.q_factor) * k + kk) * norm;
            }
            FilterType::LowShelf => {
                let norm = 1.0 / (1.0 + (2.0 * c1).sqrt() * k + c1 * kk);
                self.a0 = (1.0 + (2.0 * c2).sqrt() * k + c2 * kk) * norm;
                self.a1 = 2.0 * (c2 * kk - 1.0) * norm;
                self.a2 = (1.0 - (2.0 * c2).sqrt() * k + c2 * kk) * norm;
                self.b1 = 2.0 * (c1 * kk - 1.0) * norm;
                self.b2 = (1.0 - (2.0 * c1).sqrt() * k + c1 * kk) * norm;
            }
            FilterType::HighShelf => {
                let norm = 1.0 / (c1 + (2.0 * c1).sqrt() * k + kk);
                self.a0 = (c2 + (2.0 * c2).sqrt() * k + kk) * norm;
                self.a1 = 2.0 * (kk - c2) * norm;
                self.a2 = (c2 - (2.0 * c2).sqrt() * k + kk) * norm;
                self.b1 = 2.0 * (kk - c1) * norm;
                self.b2 = (c1 - (2.0 * c1).sqrt() * k + kk) * norm;
            }
        }
    }

    pub fn process(&self, signal: &[f64]) -> Vec<f64> {
        let mut result: Vec<f64> = vec![0.0; signal.len() + 2];

        unsafe {
            for i in 0..signal.len() {
                *result.get_unchecked_mut(i) = signal[i] * self.a0 + *result.get_unchecked(i + 1);
                *result.get_unchecked_mut(i + 1) = signal[i] * self.a1 + *result.get_unchecked(i + 2) - self.b1 * *result.get_unchecked(i);
                *result.get_unchecked_mut(i + 2) = signal[i] * self.a2 - self.b2 * *result.get_unchecked(i);
            }
        }

        result.truncate(signal.len());

        result
    }
}
