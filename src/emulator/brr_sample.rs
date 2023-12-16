use std::time::Duration;
use rodio::Source;
use anyhow::{Result, bail};
use super::snes_apu::dsp::brr_block_decoder::BrrBlockDecoder;

#[derive(Clone, PartialEq)]
pub struct BrrSample(Vec<u8>, Vec<u8>);

impl BrrSample {
    pub fn new() -> Self {
        Self(Vec::new(), Vec::new())
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut result = BrrSampleBuilder::new();
        if data.is_empty() {
            return Ok(result.into_inner());
        }

        let mut ptr = data.len() % 9;
        let loop_offset = match ptr {
            2 => Some(u16::from_le_bytes(data[..2].try_into().unwrap()) as usize + 2),
            0 => None,
            invalid => bail!("Invalid BRR length {}", invalid)
        };

        while ptr < data.len() {
            let block = &data[ptr..(ptr + 9)];
            ptr += 9;

            if let Some(loop_offset) = loop_offset {
                if loop_offset <= ptr {
                    result.add_loop_block(block);
                }
            }
            result.add_start_block(block);
        }

        result.simplify();
        Ok(result.into_inner())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result: Vec<u8> = Vec::with_capacity(self.0.len() + self.1.len() + 2);

        if !self.1.is_empty() {
            let loop_offset = self.0.len() as u16;
            result.extend_from_slice(loop_offset.to_le_bytes().as_slice());
        }
        result.extend_from_slice(&self.0);
        result.extend_from_slice(&self.1);

        result
    }

    pub fn start_block_count(&self) -> usize {
        self.0.len() / 9
    }

    pub fn loop_block_count(&self) -> usize {
        self.1.len() / 9
    }

    pub fn loop_offset(&self) -> Option<usize> {
        (!self.1.is_empty()).then(|| self.0.len())
    }

    pub fn get_start_block(&self, block_index: usize) -> Option<&[u8]> {
        let start_offset = block_index * 9;
        let end_offset = start_offset + 9;
        if end_offset > self.0.len() {
            None
        } else {
            Some(&self.0[start_offset..end_offset])
        }
    }

    pub fn get_loop_block(&self, block_index: usize) -> Option<&[u8]> {
        let start_offset = block_index * 9;
        let end_offset = start_offset + 9;
        if end_offset > self.1.len() {
            None
        } else {
            Some(&self.1[start_offset..end_offset])
        }
    }
}

pub struct BrrSampleBuilder(BrrSample);

impl BrrSampleBuilder {
    pub fn new() -> Self {
        Self(BrrSample::new())
    }

    pub fn add_start_block(&mut self, block: &[u8]) {
        debug_assert_eq!(block.len(), 9);
        self.0.0.extend_from_slice(block);
    }

    pub fn add_loop_block(&mut self, block: &[u8]) {
        debug_assert_eq!(block.len(), 9);
        self.0.1.extend_from_slice(block);
    }

    pub fn simplify(&mut self) {
        if self.0.start_block_count() < self.0.loop_block_count() || self.0.loop_block_count() == 0 {
            return;
        }

        let start = self.0.0.len() - self.0.1.len();
        if &self.0.0[start..] == &self.0.1 {
            println!("Simplified sample");
            let _ = self.0.0.drain(start..);
        }
    }

    pub fn into_inner(self) -> BrrSample {
        self.0
    }
}

pub struct BrrSampleIntoIter {
    sample: BrrSample,
    decoder: BrrBlockDecoder,
    loaded_first_block: bool,
    block_index: usize,
    sample_index: usize,
    loop_count: usize
}

impl BrrSampleIntoIter {
    pub fn new(sample: BrrSample) -> Self {
        Self {
            sample,
            decoder: BrrBlockDecoder::new(),
            loaded_first_block: false,
            block_index: 0,
            sample_index: 0,
            loop_count: 0
        }
    }

    pub fn loop_count(&self) -> usize {
        self.loop_count
    }
}

impl Iterator for BrrSampleIntoIter {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        // Load the next block
        if !self.loaded_first_block || self.decoder.is_finished() {
            if self.decoder.is_end {
                if self.decoder.is_looping {
                    self.block_index = 0;
                    self.loop_count += 1;
                } else {
                    return None;
                }
            }

            let block = match self.loop_count {
                0 => self.sample.get_start_block(self.block_index).or_else(|| self.sample.get_loop_block(self.block_index - self.sample.start_block_count()))?,
                _ => self.sample.get_loop_block(self.block_index)?
            };
            self.decoder.read(block);
            self.loaded_first_block = true;
            self.block_index += 1;
            self.sample_index = 0;
        } else {
            self.sample_index += 1;
        }

        Some(self.decoder.read_next_sample())
    }
}

impl Source for BrrSampleIntoIter {
    fn current_frame_len(&self) -> Option<usize> {
        match self.sample.loop_offset() {
            Some(_) => None,  // sound plays infinitely
            None => {
                let blocks_remaining = self.sample.start_block_count() - self.block_index.saturating_sub(1);
                Some(16 * blocks_remaining + self.sample_index)
            }
        }
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        32000
    }

    fn total_duration(&self) -> Option<Duration> {
        match self.sample.loop_offset() {
            Some(_) => None, // sound plays infinitely
            None => {
                let samples = (self.sample.start_block_count() * 16) as f64;
                Some(Duration::from_secs_f64(samples / 32000.0))
            }
        }
    }
}

impl<'a> IntoIterator for BrrSample {
    type Item = i16;
    type IntoIter = BrrSampleIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        BrrSampleIntoIter::new(self)
    }
}
