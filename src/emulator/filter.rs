

pub const GAIN_UNIT: i32 = 0x100;
const GAIN_BITS: u8 = 8;

pub const BASS_NONE: i32 = 0;
pub const BASS_NORM: i32 = 8;
pub const BASS_MAX: i32 = 31;

#[derive(Copy, Clone, Default)]
struct FilterChannel {
    pub p1: i32,
    pub pp1: i32,
    pub sum: i32
}

#[derive(Copy, Clone)]
pub struct BlarggSpcFilter {
    gain: i32,
    bass: i32,
    ch: [FilterChannel; 2]
}

impl BlarggSpcFilter {
    pub fn new(gain: i32, bass: i32) -> Self {
        Self {
            gain,
            bass,
            ch: [FilterChannel::default(), FilterChannel::default()]
        }
    }

    pub fn clear(&mut self) {
        self.ch = [FilterChannel::default(), FilterChannel::default()];
    }

    pub fn set_gain(&mut self, gain: i32) {
        self.gain = gain;
    }

    pub fn set_bass(&mut self, bass: i32) {
        self.bass = bass;
    }

    pub fn run(&mut self, io: &mut [i16]) -> Result<(), String> {
        if io.len() % 2 != 0 {
            return Err("Filter input length must be even".to_string());
        }

        for (i, c) in self.ch.iter_mut().enumerate() {
            let mut sum = c.sum;
            let mut pp1 = c.pp1;
            let mut p1 = c.p1;

            for sm_ref in io.iter_mut().skip(i).step_by(2) {
                let sm = *sm_ref as i32;

                // Low-pass filter (two point FIR with coeffs 0.25, 0.75)
                let f = sm + p1;
                p1 = sm * 3;

                // High-pass filter ("leaky integrator")
                let delta = f - pp1;
                pp1 = f;
                let s = sum >> (GAIN_BITS + 2);
                sum += (delta * self.gain) - (sum >> self.bass);

                // Clamp to 16 bits
                if s > 0x7FFF {
                    *sm_ref = 0x7FFF;
                } else if s < -0x8000 {
                    *sm_ref = -0x8000;
                } else {
                    *sm_ref = s as i16;
                }
            }

            c.sum = sum;
            c.pp1 = pp1;
            c.p1 = p1;
        }

        Ok(())
    }
}

impl Default for BlarggSpcFilter {
    fn default() -> Self {
        Self::new(GAIN_UNIT, BASS_NORM)
    }
}
