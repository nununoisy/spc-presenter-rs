use super::dsp::Dsp;

impl Dsp {
    pub(super) fn misc27(&mut self) {
        for voice in self.voices.iter_mut() {
            voice.misc27();
        }
    }

    pub(super) fn misc28(&mut self) {
        for voice in self.voices.iter_mut() {
            voice.misc28();
        }
        self.source_dir = self.l_source_dir;
    }

    pub(super) fn misc29(&mut self) {
        self.every_other_sample = !self.every_other_sample;
        for voice in self.voices.iter_mut() {
            voice.misc29();
        }
    }

    pub(super) fn misc30(&mut self) {
        for voice in self.voices.iter_mut() {
            voice.misc30();
        }

        self.counter = match self.counter {
            0 => (2048 * 5 * 3) - 1,
            _ => self.counter - 1
        };

        if !self.read_counter(self.noise_clock as i32) {
            let feedback = (self.noise << 13) ^ (self.noise << 14);
            self.noise = (feedback & 0x4000) ^ (self.noise >> 1);
        }
    }
}