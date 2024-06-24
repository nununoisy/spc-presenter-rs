// pub struct Timer {
//     resolution: i32,
//     is_running: bool,
//     ticks: i32,
//     target: u8,
//     counter_low: u8,
//     counter_high: u8
// }
//
// impl Timer {
//     pub fn new(resolution: i32) -> Timer {
//         Timer {
//             resolution,
//             is_running: false,
//             ticks: 0,
//             target: 0,
//             counter_low: 0,
//             counter_high: 0
//         }
//     }
//
//     pub fn cpu_cycles_callback(&mut self, num_cycles: i32) {
//         if !self.is_running {
//             return;
//         }
//         self.ticks += num_cycles;
//         while self.ticks > self.resolution {
//             self.ticks -= self.resolution;
//
//             self.counter_low = self.counter_low.wrapping_add(1);
//             if self.target != 0 && self.counter_low == self.target {
//                 self.counter_high = self.counter_high.wrapping_add(1);
//                 self.counter_low = 0;
//             }
//         }
//     }
//
//     pub fn set_start_stop_bit(&mut self, value: bool) {
//         if value && !self.is_running {
//             self.ticks = 0;
//             self.counter_low = 0;
//         }
//         self.is_running = value;
//     }
//
//     pub fn set_target(&mut self, value: u8) {
//         self.target = value;
//     }
//
//     pub fn read_counter(&mut self) -> u8 {
//         let ret = self.counter_high & 0x0f;
//         self.counter_high = 0;
//         ret
//     }
// }

#[derive(Default)]
pub struct Timer<const F: u8> {
    stage0: u8,
    stage1: u8,
    stage2: u8,
    stage3: u8,
    line: bool,
    enable: bool,
    target: u8
}

impl<const F: u8> Timer<F> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cpu_cycles_callback(&mut self, num_cycles: i32) {
        // The 2 is from the internal SPC700 clock divider.
        self.stage0 += (num_cycles * 2) as u8;
        if self.stage0 < F {
            return;
        }

        self.stage0 -= F;
        self.stage1 ^= 1;
        self.synchronize_stage1();
    }

    pub fn synchronize_stage1(&mut self) {
        let line = self.line;
        self.line = self.stage1 != 0;
        if !line || self.line {
            return;
        }

        if !self.enable {
            return;
        }
        self.stage2 = self.stage2.wrapping_add(1);
        if self.stage2 != self.target {
            return;
        }

        self.stage2 = 0;
        self.stage3 = (self.stage3 + 1) & 0x0f;
    }

    pub fn read(&mut self) -> u8 {
        let result = self.stage3;
        self.stage3 = 0;
        result
    }

    pub fn set_enable(&mut self, enable: bool, inv_raised: bool) {
        if (!self.enable && enable) ^ inv_raised  {
            self.stage2 = 0;
            self.stage3 = 0;
        }
        self.enable = enable;
    }

    pub fn set_target(&mut self, target: u8) {
        self.target = target;
    }
}
