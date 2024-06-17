pub struct Timer {
    resolution: i32,
    is_running: bool,
    ticks: i32,
    target: u8,
    counter_low: u8,
    counter_high: u8
}

impl Timer {
    pub fn new(resolution: i32) -> Timer {
        Timer {
            resolution,
            is_running: false,
            ticks: 0,
            target: 0,
            counter_low: 0,
            counter_high: 0
        }
    }

    pub fn cpu_cycles_callback(&mut self, num_cycles: i32) {
        if !self.is_running {
            return;
        }
        self.ticks += num_cycles;
        while self.ticks > self.resolution {
            self.ticks -= self.resolution;

            self.counter_low = self.counter_low.wrapping_add(1);
            if self.target != 0 && self.counter_low == self.target {
                self.counter_high = self.counter_high.wrapping_add(1);
                self.counter_low = 0;
            }
        }
    }

    pub fn set_start_stop_bit(&mut self, value: bool) {
        if value && !self.is_running {
            self.ticks = 0;
            self.counter_low = 0;
        }
        self.is_running = value;
    }

    pub fn set_target(&mut self, value: u8) {
        self.target = value;
    }

    pub fn read_counter(&mut self) -> u8 {
        let ret = self.counter_high & 0x0f;
        self.counter_high = 0;
        ret
    }
}
