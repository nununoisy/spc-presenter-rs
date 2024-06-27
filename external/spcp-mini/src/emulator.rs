use std::path::{Path, PathBuf};
use std::{iter, thread};
use std::ops::Deref;
use std::sync::{Arc, atomic::AtomicUsize, mpsc, Mutex, RwLock};
use std::sync::atomic::Ordering;
use std::time::Duration;
use rodio::Source;
use rodio::source::SeekError;
use snes_apu_spcp::{Apu, ApuChannelState, ApuMasterState, ApuStateReceiver, search_for_script700_file};
use spc_spcp::spc::Spc;

type BufferedAudio = Arc<RwLock<Vec<i16>>>;
type BufferedStates = Arc<RwLock<Vec<EmulatorState>>>;
type SeekPosition = Arc<AtomicUsize>;

#[derive(Clone)]
pub struct EmulatorState([ApuChannelState; 8], ApuMasterState);

impl EmulatorState {
    pub(self) fn new() -> Self {
        Self([ApuChannelState::default(); 8], ApuMasterState::default())
    }

    pub(self) fn reset(&mut self) {
        self.0.fill(ApuChannelState::default());
        self.1 = ApuMasterState::default();
    }

    pub fn master(&self) -> ApuMasterState {
        self.1
    }
}

impl ApuStateReceiver for EmulatorState {
    fn receive_channel(&mut self, channel: usize, state: ApuChannelState) {
        self.0[channel] = state;
    }

    fn receive_master(&mut self, state: ApuMasterState) {
        self.1 = state;
    }
}

impl Deref for EmulatorState {
    type Target = [ApuChannelState];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
enum EmulatorThreadMessage {
    LoadSpc(Box<Spc>, Option<PathBuf>),
    Terminate
}

const SAMPLES_PER_STATE: usize = 320;

fn spawn_emulator_thread(channel: mpsc::Receiver<EmulatorThreadMessage>, buffered_audio: BufferedAudio, buffered_states: BufferedStates, seek_position: SeekPosition) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let buffered_audio = buffered_audio.clone();
        let buffered_states = buffered_states.clone();
        let seek_position = seek_position.clone();

        let mut apu = Apu::new();
        let mut spc_loaded = false;
        let state = Arc::new(Mutex::new(EmulatorState::new()));
        let mut minimum_end_position = 0usize;

        'main: loop {
            match channel.try_recv() {
                Ok(EmulatorThreadMessage::LoadSpc(spc, script700_path)) => {
                    apu = Apu::from_spc(&spc);
                    apu.clear_echo_buffer();

                    if let Some(script700_path) = script700_path {
                        let _ = apu.load_script700(script700_path);
                    }

                    if let Some(id666_tag) = &spc.id666_tag {
                        minimum_end_position = 64 * (id666_tag.play_time + id666_tag.fadeout_time).as_millis() as usize;
                    } else {
                        minimum_end_position = 64 * 60 * 1000;
                    }

                    state.lock().unwrap().reset();
                    apu.set_state_receiver(Some(state.clone()));

                    seek_position.store(0, Ordering::Release);
                    buffered_audio.write().unwrap().clear();
                    buffered_audio.write().unwrap().shrink_to(minimum_end_position);
                    buffered_states.write().unwrap().clear();
                    buffered_states.write().unwrap().shrink_to(minimum_end_position);
                    spc_loaded = true;
                },
                Ok(EmulatorThreadMessage::Terminate) => break 'main,
                _ => ()
            }

            if spc_loaded && seek_position.load(Ordering::Acquire).max(minimum_end_position) >= buffered_audio.read().unwrap().len().saturating_sub(1024000) {
                for _ in 0..100 {
                    let mut l_audio_buffer = [0i16; SAMPLES_PER_STATE];
                    let mut r_audio_buffer = [0i16; SAMPLES_PER_STATE];
                    apu.render(&mut l_audio_buffer, &mut r_audio_buffer, SAMPLES_PER_STATE);

                    let mut nonplanar_audio = vec![0; 2 * SAMPLES_PER_STATE];
                    unsafe {
                        for (i, (l, r)) in iter::zip(l_audio_buffer, r_audio_buffer).enumerate() {
                            *nonplanar_audio.get_unchecked_mut(2 * i) = r;
                            *nonplanar_audio.get_unchecked_mut(2 * i + 1) = l;
                        }
                    }

                    buffered_audio.write().unwrap().extend_from_slice(&nonplanar_audio);
                    buffered_states.write().unwrap().push(state.lock().unwrap().clone());
                }
            } else {
                thread::sleep(Duration::from_millis(16));
            }
        }
    })
}

pub struct Emulator {
    handle: thread::JoinHandle<()>,
    channel_tx: mpsc::Sender<EmulatorThreadMessage>,
    buffered_audio: BufferedAudio,
    buffered_states: BufferedStates,
    seek_position: SeekPosition,
}

impl Emulator {
    pub fn new() -> Self {
        let (channel_tx, channel_rx) = mpsc::channel();
        let buffered_audio: BufferedAudio = Arc::new(RwLock::new(Vec::new()));
        let buffered_states: BufferedStates = Arc::new(RwLock::new(Vec::new()));
        let seek_position: SeekPosition = Arc::new(AtomicUsize::default());

        Self {
            handle: spawn_emulator_thread(channel_rx, buffered_audio.clone(), buffered_states.clone(), seek_position.clone()),
            channel_tx,
            buffered_audio,
            buffered_states,
            seek_position
        }
    }

    pub fn iter(&self) -> EmulatorSource {
        EmulatorSource::new(self.buffered_audio.clone(), self.buffered_states.clone(), self.seek_position.clone())
    }

    pub fn load_spc<P: AsRef<Path>>(&mut self, spc_path: P) -> Option<Box<Spc>> {
        let spc = Box::new(Spc::load(&spc_path).ok()?);
        self.channel_tx.send(EmulatorThreadMessage::LoadSpc(spc.clone(), search_for_script700_file(&spc_path))).unwrap();
        Some(spc)
    }
}

impl Drop for Emulator {
    fn drop(&mut self) {
        self.channel_tx.send(EmulatorThreadMessage::Terminate).unwrap();
    }
}

#[derive(Clone)]
pub struct EmulatorSource {
    buffered_audio: BufferedAudio,
    buffered_states: BufferedStates,
    seek_position: SeekPosition
}

impl EmulatorSource {
    pub(self) fn new(buffered_audio: BufferedAudio, buffered_states: BufferedStates, seek_position: SeekPosition) -> Self {
        Self {
            buffered_audio,
            buffered_states,
            seek_position
        }
    }

    pub fn position(&self) -> Duration {
        let pos = (self.seek_position.load(Ordering::Relaxed) / 2) as f64;
        Duration::from_secs_f64(pos / 32000.0)
    }

    pub fn buffer_length(&self) -> Duration {
        let buffer_length = (self.buffered_audio.read().unwrap().len() / 2) as f64;
        Duration::from_secs_f64(buffer_length / 32000.0)
    }

    pub fn apu_state(&self) -> Option<EmulatorState> {
        let pos = self.seek_position.load(Ordering::Relaxed);
        if pos == 0 {
            Some(EmulatorState::new())
        } else {
            self.buffered_states.read().unwrap().get(pos / (2 * SAMPLES_PER_STATE)).cloned()
        }
    }

    pub fn ensure_left(&mut self) {
        let seek_position = self.seek_position.load(Ordering::Acquire);
        self.seek_position.store(seek_position & !1, Ordering::Release);
    }
}

impl Iterator for EmulatorSource {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        let pos = self.seek_position.load(Ordering::Acquire);
        let buffered_audio = self.buffered_audio.read().unwrap();
        if pos >= buffered_audio.len() {
            Some(buffered_audio.last().copied().unwrap_or_default())
        } else {
            self.seek_position.store(pos + 1, Ordering::Release);
            Some(buffered_audio[pos])
        }
    }
}

impl Source for EmulatorSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        2
    }

    fn sample_rate(&self) -> u32 {
        32000
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.seek_position.store(64 * pos.as_millis() as usize, Ordering::SeqCst);
        Ok(())
    }
}
