use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, mpsc};
use std::thread;
use hound::{WavSpec, WavWriter};
use snes_apu_spcp::Apu;
use spc_spcp::spc::Spc;

#[derive(Clone)]
enum WavExporterThreadRequest {
    Export {
        wav_path: PathBuf,
        spc: Box<Spc>,
        script700_path: Option<PathBuf>
    },
    Cancel,
    Terminate
}

#[derive(Clone)]
enum WavExporterThreadResponse {
    Progress {

    }
}

fn spawn_wav_exporter_thread(request_channel: mpsc::Receiver<WavExporterThreadRequest>, current_time: Arc<AtomicU64>, end_time: Arc<AtomicU64>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let progress = progress.clone();

        let mut apu = Apu::new();
    })
}

pub struct WavExporter {

}
