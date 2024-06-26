pub struct RingBuffer<const N: usize> {
    left_buffer: Box<[i16; N]>,
    right_buffer: Box<[i16; N]>,
    write_pos: usize,
    read_pos: usize,
    sample_count: usize
}

impl<const N: usize> RingBuffer<N> {
    pub fn new() -> Self {
        Self {
            left_buffer: Box::new([0i16; N]),
            right_buffer: Box::new([0i16; N]),
            write_pos: 0,
            read_pos: 0,
            sample_count: 0
        }
    }

    pub fn write_sample(&mut self, left: i16, right: i16) {
        self.left_buffer[self.write_pos] = left;
        self.right_buffer[self.write_pos] = right;
        self.write_pos = (self.write_pos + 1) % N;
        self.sample_count += 1;
    }

    pub fn read(&mut self, left: &mut [i16], right: &mut [i16], num_samples: usize) {
        debug_assert!(num_samples <= self.sample_count);

        let read1_size = std::cmp::min(num_samples, N - self.read_pos);
        let read2_size = num_samples - read1_size;

        left[..read1_size].copy_from_slice(&self.left_buffer[self.read_pos..(self.read_pos + read1_size)]);
        right[..read1_size].copy_from_slice(&self.right_buffer[self.read_pos..(self.read_pos + read1_size)]);
        if read2_size > 0 {
            left[read1_size..(read1_size + read2_size)].copy_from_slice(&self.left_buffer[..read2_size]);
            right[read1_size..(read1_size + read2_size)].copy_from_slice(&self.right_buffer[..read2_size]);
            self.read_pos = read2_size;
        } else {
            self.read_pos += read1_size;
        }

        self.sample_count -= num_samples;
    }

    pub fn get_sample_count(&self) -> usize {
        self.sample_count
    }

    pub fn clear(&mut self) {
        self.read_pos = 0;
        self.write_pos = 0;
        self.sample_count = 0;
    }
}
