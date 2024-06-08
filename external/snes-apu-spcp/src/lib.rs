mod apu;
mod smp;
mod dsp;
mod timer;

pub use apu::{Apu, ApuState, ApuStateReceiver};
pub use dsp::voice::ResamplingMode;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
