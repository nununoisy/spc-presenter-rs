mod emulator;
mod visualizer;
mod video_builder;
mod renderer;
mod cli;
mod tuning;
mod gui;
mod config;
mod sample_processing;

use std::env;
use build_time::build_time_utc;

fn main() {
    println!("SPCPresenter started! (built {})", build_time_utc!("%Y-%m-%dT%H:%M:%S"));
    video_builder::init().unwrap();

    match env::args().len() {
        1 => gui::run(),
        _ => cli::run()
    };
}
