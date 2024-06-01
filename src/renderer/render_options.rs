use std::collections::HashMap;
use std::str::FromStr;
use std::ffi::OsStr;
use tiny_skia::Color;
use crate::config::Config;
use crate::sample_processing::SampleData;
use crate::video_builder::video_options::VideoOptions;

pub const FRAME_RATE: i32 = 60;

macro_rules! extra_str_traits {
    ($t: ty) => {
        impl From<&OsStr> for $t {
            fn from(value: &OsStr) -> Self {
                <$t>::from_str(value.to_str().unwrap()).unwrap()
            }
        }

        impl From<String> for $t {
            fn from(value: String) -> Self {
                <$t>::from_str(value.as_str()).unwrap()
            }
        }
    }
}

#[derive(Copy, Clone)]
pub enum StopCondition {
    Frames(u64),
    Loops(usize),
    SpcDuration
}

impl FromStr for StopCondition {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split(':').collect();
        if parts.len() != 2 {
            return Err("Stop condition format invalid, try one of 'time:30', 'time:spc', or 'frames:1800'.".to_string());
        }

        match parts[0] {
            "time" => {
                if parts[1] == "spc" {
                    Ok(StopCondition::SpcDuration)
                } else {
                    let time = u64::from_str(parts[1]).map_err( | e | e.to_string()) ?;
                    Ok(StopCondition::Frames(time * FRAME_RATE as u64))
                }
            },
            "frames" => {
                let frames = u64::from_str(parts[1]).map_err(|e| e.to_string())?;
                Ok(StopCondition::Frames(frames))
            },
            "loops" => {
                let loops = usize::from_str(parts[1]).map_err(|e| e.to_string())?;
                Ok(StopCondition::Loops(loops))
            },
            _ => Err(format!("Unknown condition type {}. Valid types are 'time', 'frames', and 'loops'", parts[0]))
        }
    }
}

extra_str_traits!(StopCondition);

#[derive(Clone)]
pub struct RendererOptions {
    pub input_path: String,
    pub video_options: VideoOptions,

    pub stop_condition: StopCondition,
    pub fadeout_length: u64,

    pub config: Config,
    pub sample_tunings: HashMap<u8, SampleData>,
    pub per_sample_colors: HashMap<u8, Color>
}

impl Default for RendererOptions {
    fn default() -> Self {
        Self {
            input_path: "".to_string(),
            video_options: VideoOptions {
                output_path: "".to_string(),
                metadata: Default::default(),
                background_path: None,
                dim_background: true,
                video_time_base: (1, 60).into(),
                video_codec: "libx264".to_string(),
                video_codec_params: Default::default(),
                pixel_format_in: "rgba".to_string(),
                pixel_format_out: "yuv420p".to_string(),
                resolution_in: (960, 540),
                resolution_out: (1920, 1080),
                audio_time_base: (1, 44_100).into(),
                audio_codec: "aac".to_string(),
                audio_codec_params: Default::default(),
                audio_channels: 2,
                sample_format_in: "s16".to_string(),
                sample_format_out: "fltp".to_string(),
                sample_rate: 44_100,
            },
            stop_condition: StopCondition::Frames(300 * FRAME_RATE as u64),
            fadeout_length: 180,
            config: Config::default(),
            sample_tunings: HashMap::new(),
            per_sample_colors: HashMap::new()
        }
    }
}
