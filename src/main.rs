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

fn main() {
    video_builder::init().unwrap();

    match env::args().len() {
        1 => gui::run(),
        _ => cli::run()
    };
}
