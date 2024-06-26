use rodio::{Device, DeviceTrait, OutputStream, OutputStreamHandle, Sink, Source};
use std::time::Duration;
use rodio::cpal::traits::HostTrait;
use crate::emulator::EmulatorSource;


pub struct AudioManager {
    stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    sink: Option<Sink>
}

impl AudioManager {
    pub fn new() -> Self {
        Self {
            stream: None,
            stream_handle: None,
            sink: None
        }
    }

    pub fn device_names() -> Vec<String> {
        let devices = match rodio::cpal::default_host().output_devices() {
            Ok(devices) => devices,
            Err(e) => {
                println!("Error enumerating audio devices: {}", e);
                return vec![];
            }
        };

        devices.map(|d| d.name()).collect::<Result<Vec<_>, _>>().unwrap_or_else(|e| {
            println!("Error enumerating audio devices: {}", e);
            vec![]
        })
    }

    fn find_device(device_name: &str) -> Option<Device> {
        let mut devices = match rodio::cpal::default_host().output_devices() {
            Ok(devices) => devices,
            Err(e) => {
                println!("Error enumerating audio devices: {}", e);
                return None;
            }
        };

        devices.find(|device| device.name().unwrap_or("".to_string()).as_str() == device_name)
    }

    pub fn init(&mut self, mut source: EmulatorSource, device_name: Option<&str>) {
        let was_paused = self.is_paused();

        self.stream = None;
        self.stream_handle = None;
        self.sink = None;

        let new_stream = match device_name {
            Some(device_name) => {
                match Self::find_device(device_name) {
                    Some(device) => OutputStream::try_from_device(&device),
                    None => {
                        println!("Could not find device named '{}'.", device_name);
                        return;
                    }
                }
            },
            None => OutputStream::try_default()
        };
        match new_stream {
            Ok((stream, stream_handle)) => {
                self.stream = Some(stream);
                self.stream_handle = Some(stream_handle);
            },
            Err(e) => {
                println!("Error initializing audio: {}", e);
                return;
            }
        }

        match Sink::try_new(self.stream_handle.as_ref().unwrap()) {
            Ok(sink) => {
                source.ensure_left();
                sink.append(rodio::source::Zero::<i16>::new(2, 32000).take_duration(Duration::from_millis(500)));
                sink.append(source);
                if was_paused {
                    sink.pause();
                } else {
                    sink.play();
                }
                self.sink = Some(sink);
            },
            Err(e) => {
                println!("Error initializing audio: {}", e);
                return;
            }
        }
    }

    pub fn play(&self) {
        if let Some(sink) = &self.sink {
            sink.play();
        }
    }

    pub fn pause(&self) {
        if let Some(sink) = &self.sink {
            sink.pause();
        }
    }

    pub fn is_paused(&self) -> bool {
        match &self.sink {
            Some(sink) => sink.is_paused(),
            None => true
        }
    }

    pub fn seek(&self, pos: Duration) {
        if let Some(sink) = &self.sink {
            sink.try_seek(pos).unwrap();
        }
    }

    pub fn set_volume(&self, volume: f32) {
        if let Some(sink) = &self.sink {
            sink.set_volume(volume);
        }
    }
}
