use std::error::Error;
use std::fs::File;
use std::io::{self, BufWriter, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::{iter, thread};
use std::time::Duration;
use snes_apu_spcp::Apu;
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
        let current_offset = self.writer.stream_position()?;
        debug_assert!(current_offset > chunk_offset);

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

    fn write_list_item(&mut self, key: &'static [u8], value: &str) -> io::Result<()> {
        debug_assert_eq!(key.len(), 4);

        if value.is_empty() {
            return Ok(());
        }

        self.writer.write_all(key)?;
        self.write_u32(1 + value.as_bytes().len() as u32)?;
        self.writer.write_all(value.as_bytes())?;
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
        let metadata = match spc.metadata() {
            Some(metadata) => metadata,
            None => return Ok(())
        };

        let info_list_offset = self.write_chunk(b"LIST", Some(b"INFO"))?;

        self.write_list_item(b"INAM", metadata.title().as_str())?;
        self.write_list_item(b"IPRD", metadata.game_title().as_str())?;
        self.write_list_item(b"IART", metadata.artist_name().as_str())?;
        self.write_list_item(b"ITCH", metadata.dumper_name().as_str())?;
        self.write_list_item(b"ICMT", metadata.comments().as_str())?;
        self.write_list_item(b"ISFT", "SPCPresenter Mini")?;

        self.update_chunk(info_list_offset)
    }

    pub fn write_samples(&mut self, l_audio_buffer: &[i16], r_audio_buffer: &[i16]) -> io::Result<()> {
        if self.data_offset = 0 {
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
        self.update_chunk(self.riff_offset)
    }
}

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
        current_time: Duration,
        total_time: Duration
    },
    Finished,
    Error(Box<dyn Error>)
}

macro_rules! we_unwrap {
    ($v: expr, $response_channel: tt, $lbl: tt) => {
        match $v {
            Ok(v) => v,
            Err(e) => {
                $response_channel.send(WavExporterThreadResponse::Error(Box::new(e))).unwrap();
                continue $lbl;
            }
        }
    };
}

fn spawn_wav_exporter_thread(request_channel: mpsc::Receiver<WavExporterThreadRequest>, response_channel: mpsc::Sender<WavExporterThreadResponse>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut apu = Apu::new();

        'main: loop {
            let (wav_path, spc, script700_path) = match request_channel.recv().unwrap() {
                WavExporterThreadRequest::Export { wav_path, spc, script700_path } => (wav_path, spc, script700_path),
                WavExporterThreadRequest::Cancel => continue 'main,
                WavExporterThreadRequest::Terminate => break 'main
            };

            let mut wav_writer = we_unwrap!(WavWriter::new(wav_path), response_channel, 'main);
            we_unwrap!(wav_writer.start(), response_channel, 'main);
            we_unwrap!(wav_writer.write_metadata(&spc), response_channel, 'main);

            apu = Apu::from_spc(&spc);
            apu.clear_echo_buffer();

            if let Some(script700_path) = script700_path {
                let _ = apu.load_script700(script700_path);
            }

            let mut l_audio_buffer = [0i16; BUFFER_SIZE];
            let mut r_audio_buffer = [0i16; BUFFER_SIZE];

            let (play_time, fadeout_time) = spc.metadata().play_time(None).unwrap_or((Duration::from_secs(300), Duration::from_secs(10)));
            let total_time = play_time + fadeout_time;
            let mut current_time = Duration::ZERO;
            let mut samples_left = (total_time.as_secs_f64() * 32000.0) as usize;

            while samples_left > 0 {
                let n = samples_left.min(BUFFER_SIZE);
                apu.render(&mut l_audio_buffer[..n], &mut r_audio_buffer[..n], n as i32);
                we_unwrap!(wav_writer.write_samples(&l_audio_buffer[..n], &r_audio_buffer[..n]), response_channel, 'main);
                samples_left -= n;
                current_time += Duration::from_secs_f64(n as f64 / 64000.0);

                response_channel.send(WavExporterThreadResponse::Progress {
                    current_time,
                    total_time
                }).unwrap();
            }

            we_unwrap!(wav_writer.finish(), response_channel, 'main);
            drop(wav_writer);

            response_channel.send(WavExporterThreadResponse::Finished).unwrap();
        }
    })
}

pub struct WavExporter {
    handle: thread::JoinHandle<()>,
    request_channel: mpsc::Sender<WavExporterThreadResponse>
}
