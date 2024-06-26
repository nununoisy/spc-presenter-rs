use anyhow::{Context, Result};
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use tiny_skia::Color;
use csscolorparser::Color as CssColor;
use snes_apu_spcp::ResamplingMode;
use crate::visualizer::channel_settings::ChannelSettingsManager;

fn serialize_color<S: Serializer>(color: &Color, serializer: S) -> Result<S::Ok, S::Error> {
    let color_u8 = color.to_color_u8();
    let hex_color = match color_u8.alpha() {
        0xFF => format!("#{:02X}{:02X}{:02X}", color_u8.red(), color_u8.green(), color_u8.blue()),
        _ => format!("#{:02X}{:02X}{:02X}{:02X}", color_u8.red(), color_u8.green(), color_u8.blue(), color_u8.alpha())
    };
    serializer.serialize_str(hex_color.as_str())
}

fn deserialize_color<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Color, D::Error> {
    let css_color = CssColor::deserialize(deserializer)?;
    Ok(Color::from_rgba(
        css_color.r as _,
        css_color.g as _,
        css_color.b as _,
        css_color.a as _
    ).unwrap())
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct PianoRollConfig {
    pub settings: ChannelSettingsManager,
    pub key_length: f32,
    pub key_thickness: f32,
    pub divider_width: u32,
    pub octave_count: u32,
    pub speed_multiplier: u32,
    pub starting_octave: i32,
    pub waveform_height: u32,
    pub oscilloscope_glow_thickness: f32,
    pub oscilloscope_line_thickness: f32,
    pub draw_piano_strings: bool,
    pub draw_text_labels: bool,
    #[serde(serialize_with = "serialize_color", deserialize_with = "deserialize_color")]
    pub outline_color: Color,
    #[serde(serialize_with = "serialize_color", deserialize_with = "deserialize_color")]
    pub divider_color: Color
}

impl Default for PianoRollConfig {
    fn default() -> Self {
        Self {
            settings: ChannelSettingsManager::default(),
            key_length: 24.0,
            key_thickness: 5.0,
            divider_width: 5,
            octave_count: 9,
            speed_multiplier: 1,
            starting_octave: 0,
            waveform_height: 48,
            oscilloscope_glow_thickness: 2.0,
            oscilloscope_line_thickness: 0.75,
            draw_piano_strings: false,
            draw_text_labels: true,
            outline_color: Color::BLACK,
            divider_color: Color::BLACK
        }
    }
}

#[derive(Serialize, Deserialize, Copy, Clone)]
enum SerializableResamplingMode {
    #[serde(rename = "accurate")]
    Accurate,
    #[serde(rename = "gaussian")]
    Gaussian,
    #[serde(rename = "linear")]
    Linear,
    #[serde(rename = "cubic")]
    Cubic,
    #[serde(rename = "sinc")]
    Sinc
}

fn serialize_resampling_mode<S: Serializer>(resampling_mode: &ResamplingMode, serializer: S) -> Result<S::Ok, S::Error> {
    let resampling_mode = match resampling_mode {
        ResamplingMode::Accurate => SerializableResamplingMode::Accurate,
        ResamplingMode::Gaussian => SerializableResamplingMode::Gaussian,
        ResamplingMode::Linear => SerializableResamplingMode::Linear,
        ResamplingMode::Cubic => SerializableResamplingMode::Cubic,
        ResamplingMode::Sinc => SerializableResamplingMode::Sinc
    };
    resampling_mode.serialize(serializer)
}

fn deserialize_resampling_mode<'de, D: Deserializer<'de>>(deserializer: D) -> Result<ResamplingMode, D::Error> {
    let resampling_mode = match SerializableResamplingMode::deserialize(deserializer)? {
        SerializableResamplingMode::Accurate => ResamplingMode::Accurate,
        SerializableResamplingMode::Gaussian => ResamplingMode::Gaussian,
        SerializableResamplingMode::Linear => ResamplingMode::Linear,
        SerializableResamplingMode::Cubic => ResamplingMode::Cubic,
        SerializableResamplingMode::Sinc => ResamplingMode::Sinc
    };
    Ok(resampling_mode)
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct EmulatorConfig {
    pub filter_enabled: bool,
    #[serde(serialize_with = "serialize_resampling_mode", deserialize_with = "deserialize_resampling_mode")]
    pub resampling_mode: ResamplingMode
}

impl Default for EmulatorConfig {
    fn default() -> Self {
        Self {
            filter_enabled: true,
            resampling_mode: ResamplingMode::default()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Config {
    pub emulator: EmulatorConfig,
    pub piano_roll: PianoRollConfig
}

impl Config {
    pub fn from_toml(config: &str) -> Result<Self> {
        toml::from_str(config).context("Importing configuration")
    }

    pub fn export(&self) -> Result<String> {
        toml::to_string(&self).context("Exporting configuration")
    }
}
