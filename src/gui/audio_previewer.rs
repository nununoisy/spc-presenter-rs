use std::time::Duration;
use slint::{Timer, TimerMode};
use std::sync::{Arc, Mutex, atomic::{AtomicU16, AtomicBool, Ordering}};
use rodio::{OutputStreamHandle, OutputStream, Sink, Source};
use crate::emulator::BrrSample;
use crate::visualizer::C_0;

pub const PITCH_MIN: u16 = 1;
pub const PITCH_MAX: u16 = 0x3FFF;
pub const PITCH_IDENTITY: u16 = 0x1000;

macro_rules! audio_try {
    ($x: expr) => {{
        match $x {
            Ok(v) => Some(v),
            Err(e) => {
                println!("Warning: audio failure: {}", e);
                None
            }
        }
    }};
}

pub struct AudioPreviewer {
    stream: Option<(OutputStream, OutputStreamHandle)>,
    sink: Option<Sink>,
    sample_pitch: Arc<AtomicU16>,
    was_playing: Arc<AtomicBool>
}

impl AudioPreviewer {
    pub fn new() -> Self {
        Self {
            stream: None,
            sink: None,
            sample_pitch: Arc::new(AtomicU16::new(PITCH_IDENTITY)),
            was_playing: Arc::new(AtomicBool::new(false))
        }
    }

    pub fn audio_playing(&self) -> bool {
        self.sink
            .as_ref()
            .map(|s| !s.empty() && !s.is_paused())
            .unwrap_or(false)
    }

    pub fn audio_stopped_playing(&self) -> bool {
        let is_playing = self.audio_playing();
        match self.was_playing.compare_exchange(!is_playing, is_playing, Ordering::SeqCst, Ordering::Acquire) {
            Ok(true) => true,
            _ => false
        }
    }

    #[inline]
    pub fn pitch(&self) -> u16 {
        self.sample_pitch.load(Ordering::SeqCst)
    }

    #[inline]
    pub fn set_pitch(&mut self, pitch: u16) -> u16 {
        let new_pitch = pitch.clamp(PITCH_MIN, PITCH_MAX);
        self.sample_pitch.store(new_pitch, Ordering::SeqCst);
        new_pitch
    }

    pub fn set_pitch_to_midi_note(&mut self, f_0: f64, note: i32) -> u16 {
        let note_freq = C_0 * ((note - 12) as f64 / 12.0).exp2();
        let pitch_factor = note_freq / f_0;
        let new_pitch = (pitch_factor * PITCH_IDENTITY as f64).round().clamp(PITCH_MIN as f64, PITCH_MAX as f64) as u16;
        self.set_pitch(new_pitch)
    }

    pub fn play(&mut self, sample: &BrrSample) -> bool {
        self.stop();

        self.stream = audio_try!(OutputStream::try_default());
        self.sink = self.stream.as_ref().and_then(|(_stream, handle)| audio_try!(Sink::try_new(handle)));

        if let Some(sink) = self.sink.as_mut() {
            let sample_pitch = self.sample_pitch.clone();
            let source = sample
                .clone()
                .into_iter()
                .amplify(0.5)
                .speed(1.0)
                .periodic_access(Duration::from_millis(1), move |source| {
                    let pitch = sample_pitch.load(Ordering::SeqCst);
                    let factor = (pitch as f32) / (0x1000 as f32);
                    source.set_factor(factor);
                });

            sink.append(source);
            sink.play();
            true
        } else {
            false
        }
    }

    pub fn stop(&mut self) {
        if let Some(sink) = self.sink.as_mut() {
            sink.clear();
            sink.stop();
        }
    }
}

pub fn audio_stopped_timer<F: FnMut() + 'static>(audio_previewer: Arc<Mutex<AudioPreviewer>>, mut cb: F) -> Timer {
    let result = Timer::default();

    result.start(TimerMode::Repeated, Duration::from_millis(10), move || {
        if audio_previewer.lock().unwrap().audio_stopped_playing() {
            cb();
        }
    });

    result
}
