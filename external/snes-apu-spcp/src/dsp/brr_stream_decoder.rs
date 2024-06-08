use super::dsp_helpers;

pub struct BrrStreamDecoder {
    pub is_end: bool,
    pub is_looping: bool,
    filter: u8,
    shift: u8,
    samples: [i16; 16],
    decode_pos: usize,

    sample_index: i32,
    last_sample: i16,
    last_last_sample: i16
}

impl BrrStreamDecoder {
    pub fn new() -> BrrStreamDecoder {
        BrrStreamDecoder {
            is_end: false,
            is_looping: false,
            filter: 0,
            shift: 0,
            samples: [0; 16],
            decode_pos: 0,

            sample_index: 0,
            last_sample: 0,
            last_last_sample: 0
        }
    }

    pub fn read_header(&mut self, raw_header: u8) {
        self.is_end = (raw_header & 0x01) != 0;
        self.is_looping = (raw_header & 0x02) != 0;
        self.filter = (raw_header >> 2) & 0x03;
        self.shift = raw_header >> 4;

        self.sample_index = 0;
        self.decode_pos = 0;
    }

    pub fn read(&mut self, buf: &[u8]) {
        let mut nybbles = buf[0] as i32;
        nybbles = (nybbles << 8) | (buf[1] as i32);

        for _ in 0..4 {
            let mut sample = dsp_helpers::cast_arb_int(nybbles, 16) >> 12;
            nybbles <<= 4;

            if self.shift <= 12 {
                sample <<= self.shift;
                sample >>= 1;
            } else {
                sample &= !0x07ff;
            }

            let p1 = self.last_sample as i32;
            let p2 = (self.last_last_sample >> 1) as i32;

            match self.filter {
                0 => (),
                1 => {
                    // sample += p1 * 0.46875
                    sample += p1 >> 1;
                    sample += (-p1) >> 5;
                },
                2 => {
                    // sample += p1 * 0.953125 - p2 * 0.46875
                    sample += p1;
                    sample -= p2;
                    sample += p2 >> 4;
                    sample += (p1 * -3) >> 6;
                },
                3 => {
                    // sample += p1 * 0.8984375 - p2 * 0.40625
                    sample += p1;
                    sample -= p2;
                    sample += (p1 * -13) >> 7;
                    sample += (p2 * 3) >> 4;
                },
                _ => unreachable!()
            }

            sample = dsp_helpers::clamp(sample);
            let sample_16 = dsp_helpers::cast_arb_int(sample << 1, 16) as i16;
            self.samples[self.decode_pos] = sample_16;
            self.decode_pos += 1;
            self.last_last_sample = self.last_sample;
            self.last_sample = sample_16;
        }
    }

    pub fn read_next_sample(&mut self) -> i16 {
        let ret = self.samples[self.sample_index as usize];
        self.sample_index += 1;
        ret
    }

    pub fn is_finished(&self) -> bool {
        self.sample_index >= 16
    }
}
