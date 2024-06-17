mod apu;
mod smp;
mod dsp;
mod timer;
mod script700;

pub use apu::{Apu, ApuChannelState, ApuMasterState, ApuStateReceiver};
pub use dsp::voice::ResamplingMode;
pub use script700::search_for_script700_file;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
