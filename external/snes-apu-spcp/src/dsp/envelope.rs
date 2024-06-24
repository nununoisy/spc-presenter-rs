use super::dsp::Dsp;

enum Mode {
    Attack,
    Decay,
    Sustain,
    Release
}

pub struct Envelope {
    dsp: *mut Dsp,

    pub l_adsr0: u8,
    pub adsr1: u8,
    pub gain: u8,

    mode: Mode,
    pub level: i32,
    hidden_level: i32
}

impl Envelope {
    pub fn new(dsp: *mut Dsp) -> Envelope {
        Envelope {
            dsp,

            l_adsr0: 0,
            adsr1: 0,
            gain: 0,

            mode: Mode::Release,
            level: 0,
            hidden_level: 0
        }
    }

    #[inline]
    fn dsp(&self) -> &mut Dsp {
        unsafe {
            &mut (*self.dsp)
        }
    }

    pub fn key_on(&mut self) {
        self.mode = Mode::Attack;
    }

    pub fn key_off(&mut self) {
        self.mode = Mode::Release;
    }

    pub fn reset_level(&mut self) {
        self.level = 0;
        self.hidden_level = 0;
    }

    pub fn tick(&mut self, adsr0: u8) {
        let mut env = self.level;
        if let Mode::Release = self.mode {
            self.level = (env - 8).max(0);
            return;
        }

        let rate: i32;
        let env_data: i32;
        if (adsr0 & 0x80) != 0 {
            // ADSR mode
            env_data = self.adsr1 as i32;
            match self.mode {
                Mode::Attack => {
                    rate = ((adsr0 as i32) & 0x0f) * 2 + 1;
                    env += if rate < 31 { 0x20 } else { 0x400 };
                },
                _ => {
                    env -= 1;
                    env -= env >> 8;
                    match self.mode {
                        Mode::Decay => {
                            rate = (((adsr0 as i32) >> 3) & 0x0e) + 0x10;
                        },
                        _ => {
                            rate = env_data & 0x1f;
                        }
                    }
                }
            }
        } else {
            // Gain mode
            env_data = self.gain as i32;
            let mode = env_data >> 5;
            if mode < 4 {
                // Direct
                env = env_data * 0x10;
                rate = 31;
            } else {
                rate = env_data & 0x1f;
                if mode == 4 {
                    // Linear decrease
                    env -= 0x20;
                } else if mode < 6 {
                    // Exponential decrease
                    env -= 1;
                    env -= env >> 8;
                } else {
                    // Linear increase
                    env += 0x20;
                    if mode > 6 && (self.hidden_level as u32) >= 0x600 {
                        env += 0x08 - 0x20;
                    }
                }
            }
        }

        if let Mode::Decay = self.mode {
            if (env >> 8) == (env_data >> 5) {
                self.mode = Mode::Sustain;
            }
        }

        self.hidden_level = env; // Super obscure quirk thingy here

        if (env as u32) > 0x07ff {
            env = env.clamp(0, 0x7ff);
            if let Mode::Attack = self.mode {
                self.mode = Mode::Decay;
            }
        }

        if self.dsp().read_counter(rate) {
            return;
        }
        self.level = env;
    }
}
