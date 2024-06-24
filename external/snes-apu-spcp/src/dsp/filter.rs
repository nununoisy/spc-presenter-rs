use super::dsp_helpers;

const NUM_TAPS: usize = 8;

pub struct Filter {
    pub coefficients: [u8; NUM_TAPS],

    buffer: [i16; NUM_TAPS],
    buffer_pos: usize
}

impl Filter {
    pub fn new() -> Filter {
        Filter {
            coefficients: [0; NUM_TAPS],

            buffer: [0; NUM_TAPS],
            buffer_pos: 0
        }
    }

    // pub fn next(&mut self, value: i32, is_right: bool) -> i32 {
    //     self.buffer_pos = (self.buffer_pos + 1) % NUM_TAPS as i32;
    //
    //     let mut ret = 0;
    //     for i in 0..(NUM_TAPS - 1) {
    //         // Right echo read occurs after taps 0, 1, and 2 are processed
    //         if (i == 0 && !is_right) || (i == 3 && is_right) {
    //             self.buffer[self.buffer_pos as usize] = (((value >> 1) & 0xFFFF) as i16) as i32;
    //         }
    //
    //         ret += (self.buffer[((self.buffer_pos + (i as i32) + 1) as usize) % NUM_TAPS] * ((self.coefficients[i] as i8) as i32)) >> 6;
    //     }
    //     ret = ((ret & 0xFFFF) as i16) as i32;
    //     ret += ((((self.buffer[(self.buffer_pos as usize) % NUM_TAPS] * ((self.coefficients[7] as i8) as i32)) >> 6) & 0xFFFF) as i16) as i32;
    //
    //     ret
    // }

        let mut ret = 0;
        for i in 0..(NUM_TAPS - 1) {
            // Right echo read occurs after taps 0, 1, and 2 are processed
            if (i == 0 && !is_right) || (i == 3 && is_right) {
                self.buffer[self.buffer_pos as usize] = dsp_helpers::cast_arb_int(value >> 1, 16);
            }

            ret += (self.buffer[((self.buffer_pos + (i as i32) + 1) as usize) % NUM_TAPS] * ((self.coefficients[i] as i8) as i32)) >> 6;
            if i == 0 || i == 2 || i == 5 {
                ret = dsp_helpers::cast_arb_int(ret, 17);
            }
        }
        ret = dsp_helpers::cast_arb_int(ret, 16);
        ret += dsp_helpers::cast_arb_int((self.buffer[(self.buffer_pos as usize) % NUM_TAPS] * ((self.coefficients[7] as i8) as i32)) >> 6, 16);

    pub fn echo22(&mut self) {

    }
}

impl Default for Filter {
    fn default() -> Self {
        Self::new()
    }
}
