use std::error::Error;
use std::fs::File;
use std::io::{self, BufWriter, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::{iter, thread};
use std::time::{Duration, Instant};
use snes_apu_spcp::{Apu, search_for_script700_file};
use spc_spcp::spc::Spc;

const BUFFER_SIZE: usize = 1024;

struct WavWriter {
    writer: BufWriter<File>,
    riff_offset: u64,
    data_offset: u64
}

impl WavWriter {
    pub fn new<P: AsRef<Path>>(wav_path: P) -> io::Result<Self> {
        Ok(Self {
            writer: BufWriter::new(File::create(wav_path)?),
            riff_offset: 0,
            data_offset: 0
        })
    }

    fn write_chunk(&mut self, chunk: &'static [u8], subtype: Option<&'static [u8]>) -> io::Result<u64> {
        debug_assert_eq!(chunk.len(), 4);
        self.writer.write_all(chunk)?;
        self.write_u32(0)?;

        let result = self.writer.stream_position()?;

        if let Some(subtype) = subtype {
            debug_assert_eq!(subtype.len(), 4);
            self.writer.write_all(subtype)?;
        }

        Ok(result)
    }

    fn update_chunk(&mut self, chunk_offset: u64) -> io::Result<()> {
        let mut current_offset = self.writer.stream_position()?;
        debug_assert!(current_offset > chunk_offset);

        if current_offset % 2 == 1 {
            // align
            self.write_u8(0)?;
            current_offset = self.writer.stream_position()?;
        }
        let chunk_size = (current_offset - chunk_offset) as u32;

        self.writer.seek(SeekFrom::Start(chunk_offset - 4))?;
        self.write_u32(chunk_size)?;
        self.writer.seek(SeekFrom::Start(current_offset))?;
        Ok(())
    }

    fn write_u8(&mut self, value: u8) -> io::Result<()> {
        self.writer.write_all(&[value])
    }

    fn write_u16(&mut self, value: u16) -> io::Result<()> {
        self.writer.write_all(value.to_le_bytes().as_slice())
    }

    fn write_u32(&mut self, value: u32) -> io::Result<()> {
        self.writer.write_all(value.to_le_bytes().as_slice())
    }

    fn write_i16(&mut self, value: i16) -> io::Result<()> {
        self.writer.write_all(value.to_le_bytes().as_slice())
    }

    fn write_i32(&mut self, value: i32) -> io::Result<()> {
        self.writer.write_all(value.to_le_bytes().as_slice())
    }

    fn write_list_item<S: AsRef<str>>(&mut self, key: &'static [u8], value: S) -> io::Result<()> {
        debug_assert_eq!(key.len(), 4);

        if value.as_ref().is_empty() {
            return Ok(());
        }
        let value = value.as_ref().as_bytes();

        self.writer.write_all(key)?;
        self.write_u32(1 + value.len() as u32)?;
        self.writer.write_all(value)?;
        self.write_u8(0)
    }

    pub fn start(&mut self) -> io::Result<()> {
        self.riff_offset = self.write_chunk(b"RIFF", Some(b"WAVE"))?;

        let fmt_offset = self.write_chunk(b"fmt ", None)?;
        self.write_u16(1)?;  // Sample format: PCM
        self.write_u16(2)?;  // Channels: 2
        self.write_u32(32000)?;  // Sample rate: 32 kHz
        self.write_u32(128000)?;  // Sample bytes per second: (32000 samples/second * 2 channels * 16 bits/sample) / (8 bits/byte)
        self.write_u16(4)?;  // Sample bytes per sample: (2 channels * 16 bits/sample) / (8 bits/byte)
        self.write_u16(16)?;  // Sample bit depth: 16-bit
        self.update_chunk(fmt_offset)?;

        Ok(())
    }

    pub fn write_metadata(&mut self, spc: &Spc) -> io::Result<()> {
        let metadata = spc.metadata();

        let info_list_offset = self.write_chunk(b"LIST", Some(b"INFO"))?;

        self.write_list_item(b"INAM", metadata.song_title().unwrap_or_default())?;
        self.write_list_item(b"IPRD", metadata.game_title().unwrap_or_default())?;
        self.write_list_item(b"IART", metadata.artist_name().unwrap_or_default())?;
        self.write_list_item(b"ITCH", metadata.dumper_name().unwrap_or_default())?;
        self.write_list_item(b"ICMT", metadata.comments().unwrap_or_default())?;
        self.write_list_item(b"ISFT", "SPCPresenter Mini")?;

        self.update_chunk(info_list_offset)
    }

    pub fn write_samples(&mut self, l_audio_buffer: &[i16], r_audio_buffer: &[i16]) -> io::Result<()> {
        if self.data_offset == 0 {
            self.data_offset = self.write_chunk(b"data", None)?;
        }
        for (l, r) in iter::zip(l_audio_buffer, r_audio_buffer) {
            self.write_i16(*l)?;
            self.write_i16(*r)?;
        }
        Ok(())
    }

    pub fn finish(&mut self) -> io::Result<()> {
        self.update_chunk(self.data_offset)?;
        self.update_chunk(self.riff_offset)?;
        self.writer.flush()
    }
}

#[derive(Clone)]
enum WavExporterThreadRequest {
    Export {
        wav_path: PathBuf,
        spc_path: PathBuf
    },
    Cancel,
    Terminate
}

#[derive(Clone)]
pub enum WavExporterMessage {
    Progress {
        current_time: Duration,
        total_time: Duration
    },
    Finished,
    Error(Arc<dyn Error>)
}

macro_rules! we_unwrap {
    ($v: expr, $cb: tt, $lbl: tt) => {
        match $v {
            Ok(v) => v,
            Err(e) => {
                $cb(WavExporterMessage::Error(Arc::new(e)));
                continue $lbl;
            }
        }
    };
}

pub fn spawn_wav_exporter_thread<F>(request_channel: mpsc::Receiver<WavExporterThreadRequest>, mut cb: F) -> thread::JoinHandle<()>
    where
        F: FnMut(WavExporterMessage) + Send + 'static
{
    thread::spawn(move || {
        'main: loop {
            let (wav_path, spc_path) = match request_channel.recv().unwrap() {
                WavExporterThreadRequest::Export { wav_path, spc_path } => (wav_path, spc_path),
                WavExporterThreadRequest::Cancel => continue 'main,
                WavExporterThreadRequest::Terminate => break 'main
            };

            let spc = we_unwrap!(Spc::load(&spc_path), cb, 'main);

            let mut wav_writer = we_unwrap!(WavWriter::new(wav_path), cb, 'main);
            we_unwrap!(wav_writer.start(), cb, 'main);
            we_unwrap!(wav_writer.write_metadata(&spc), cb, 'main);

            let mut apu = Apu::from_spc(&spc);
            apu.clear_echo_buffer();

            if let Some(script700_path) = search_for_script700_file(&spc_path) {
                we_unwrap!(apu.load_script700(script700_path), cb, 'main);
            }

            let mut l_audio_buffer = [0i16; BUFFER_SIZE];
            let mut r_audio_buffer = [0i16; BUFFER_SIZE];

            let (play_time, fadeout_time) = spc.metadata().play_time(None).unwrap_or((Duration::from_secs(300), Duration::from_secs(10)));
            let total_time = play_time + fadeout_time;
            let mut current_time = Duration::ZERO;
            let mut samples_left = (total_time.as_secs_f64() * 32000.0) as usize;
            let mut last_updated = Instant::now();

            cb(WavExporterMessage::Progress {
                current_time,
                total_time
            });

            'encode: while samples_left > 0 {
                let n = samples_left.min(BUFFER_SIZE);
                apu.render(&mut l_audio_buffer[..n], &mut r_audio_buffer[..n], n);
                we_unwrap!(wav_writer.write_samples(&l_audio_buffer[..n], &r_audio_buffer[..n]), cb, 'main);
                samples_left -= n;
                current_time += Duration::from_secs_f64(n as f64 / 32000.0);

                match request_channel.try_recv() {
                    Ok(WavExporterThreadRequest::Cancel) => break 'encode,
                    Ok(WavExporterThreadRequest::Terminate) => break 'main,
                    _ => ()
                };

                if last_updated.elapsed() > Duration::from_millis(100) {
                    cb(WavExporterMessage::Progress {
                        current_time,
                        total_time
                    });
                    last_updated = Instant::now();
                }
            }

            we_unwrap!(wav_writer.finish(), cb, 'main);
            cb(WavExporterMessage::Finished);
        }
    })
}

pub struct WavExporter {
    handle: thread::JoinHandle<()>,
    request_channel: mpsc::Sender<WavExporterThreadRequest>,
    spc_path: Option<PathBuf>
}

impl WavExporter {
    pub fn new<F: FnMut(WavExporterMessage) + Send + 'static>(cb: F) -> Self {
        let (request_channel, request_channel_rx) = mpsc::channel();
        let handle = spawn_wav_exporter_thread(request_channel_rx, cb);

        Self {
            handle,
            request_channel,
            spc_path: None
        }
    }

    pub fn set_spc_path<P: AsRef<Path>>(&mut self, spc_path: P) {
        self.spc_path = Some(spc_path.as_ref().to_path_buf());
    }

    pub fn export<P: AsRef<Path>>(&self, wav_path: P) {
        let wav_path = wav_path.as_ref().to_path_buf();
        let spc_path = match &self.spc_path {
            Some(spc_path) => spc_path.clone(),
            None => return
        };

        self.request_channel.send(WavExporterThreadRequest::Export {
            wav_path,
            spc_path
        }).unwrap();
    }

    pub fn cancel(&self) {
        self.request_channel.send(WavExporterThreadRequest::Cancel).unwrap();
    }
}

impl Drop for WavExporter {
    fn drop(&mut self) {
        self.request_channel.send(WavExporterThreadRequest::Cancel).unwrap();
        self.request_channel.send(WavExporterThreadRequest::Terminate).unwrap();
    }
}
