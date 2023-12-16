use std::collections::{BTreeMap, HashMap};
use tiny_skia::Color;
use csscolorparser::Color as CssColor;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use super::ChannelState;

#[derive(Clone)]
pub struct ChannelSettings(String, String, bool, Vec<Color>);

impl ChannelSettings {
    pub fn new(chip: &str, name: &str, colors: &[Color]) -> Self {
        Self(chip.to_string(), name.to_string(), false, colors.to_vec())
    }

    pub fn chip(&self) -> String {
        self.0.clone()
    }

    pub fn name(&self) -> String {
        self.1.clone()
    }

    pub fn hidden(&self) -> bool {
        self.2
    }

    pub fn color(&self, state: &ChannelState) -> Option<Color> {
        let color_index = match self.3.len() {
            0 => state.timbre,
            max_index => state.timbre % max_index
        };

        let result = self.3.get(color_index).cloned();
        if let Some(color) = &result {
            if state.volume == 0.0 {
                return Some(Color::from_rgba(
                    color.red() / 2.0 + 0.0625,
                    color.green() / 2.0 + 0.0625,
                    color.blue() / 2.0 + 0.0625,
                    color.alpha()
                ).unwrap());
            }
        }
        result
    }

    pub fn colors(&self) -> Vec<Color> {
        self.3.clone()
    }

    pub fn num_colors(&self) -> usize {
        self.3.len()
    }

    pub fn set_hidden(&mut self, hidden: bool) {
        self.2 = hidden;
    }

    pub fn set_colors(&mut self, colors: &[Color]) {
        self.3 = colors.to_vec();
    }
}

impl Default for ChannelSettings {
    fn default() -> Self {
        Self::new("<?>", "<?>", &[Color::from_rgba8(0x90, 0x90, 0x90, 0xFF)])
    }
}

#[derive(Clone)]
pub struct ChannelSettingsManager(Vec<ChannelSettings>);

impl ChannelSettingsManager {
    pub fn settings(&self, channel: usize) -> Option<&ChannelSettings> {
        self.0.get(channel)
    }

    pub fn settings_mut(&mut self, channel: usize) -> Option<&mut ChannelSettings> {
        self.0.get_mut(channel)
    }

    // pub fn settings_by_name(&self, chip: &str, channel: &str) -> Option<&ChannelSettings> {
    //     self.0
    //         .iter()
    //         .find(|settings| settings.chip().as_str() == chip && settings.name().as_str() == channel)
    // }

    pub fn settings_mut_by_name(&mut self, chip: &str, channel: &str) -> Option<&mut ChannelSettings> {
        self.0
            .iter_mut()
            .find(|settings| settings.chip().as_str() == chip && settings.name().as_str() == channel)
    }

    pub fn put_per_sample_colors(&mut self, sample_colors: HashMap<u8, Color>) {
        for settings in self.0.iter_mut() {
            let base_color = settings.colors().first().cloned().unwrap_or(Color::from_rgba8(0xFF, 0xA0, 0xA0, 0xFF));

            let mut new_colors = vec![base_color; 0x100];
            for (sample, sample_color) in sample_colors.iter() {
                new_colors[*sample as usize] = sample_color.clone();
            }

            settings.set_colors(&new_colors);
        }
    }
}

impl Default for ChannelSettingsManager {
    fn default() -> Self {
        Self(vec![
            ChannelSettings::new("S-DSP", "Channel 1", &[
                Color::from_rgba8(0xFF, 0xA0, 0xA0, 0xFF)
            ]),
            ChannelSettings::new("S-DSP", "Channel 2", &[
                Color::from_rgba8(0xFF, 0xE0, 0xA0, 0xFF)
            ]),
            ChannelSettings::new("S-DSP", "Channel 3", &[
                Color::from_rgba8(0x40, 0xFF, 0x40, 0xFF)
            ]),
            ChannelSettings::new("S-DSP", "Channel 4", &[
                Color::from_rgba8(0xC0, 0xC0, 0xC0, 0xFF)
            ]),
            ChannelSettings::new("S-DSP", "Channel 5", &[
                Color::from_rgba8(0x9A, 0x4F, 0xFF, 0xFF)
            ]),
            ChannelSettings::new("S-DSP", "Channel 6", &[
                Color::from_rgba8(0x38, 0xAB, 0xF2, 0xFF)
            ]),
            ChannelSettings::new("S-DSP", "Channel 7", &[
                Color::from_rgba8(0xAC, 0xED, 0x32, 0xFF)
            ]),
            ChannelSettings::new("S-DSP", "Channel 8", &[
                Color::from_rgba8(0x24, 0x7B, 0xA0, 0xFF)
            ])
        ])
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
struct PianoRollChannelConfig {
    pub hidden: bool,
    #[serde(flatten)]
    pub colors: BTreeMap<String, CssColor>
}

impl Serialize for ChannelSettingsManager {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut settings: BTreeMap<String, BTreeMap<String, PianoRollChannelConfig>> = BTreeMap::new();
        for channel_settings in self.0.iter() {
            let config = PianoRollChannelConfig {
                hidden: channel_settings.hidden(),
                colors: BTreeMap::from_iter(
                    channel_settings.colors()
                        .iter()
                        .map(|c| {
                            let css_color = CssColor::new(c.red() as _, c.green() as _, c.blue() as _, c.alpha() as _);
                            ("static".to_string(), css_color)
                        })
                )
            };

            settings.entry(channel_settings.chip())
                .or_insert(BTreeMap::new())
                .insert(channel_settings.name(), config);
        }

        settings.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ChannelSettingsManager {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut result = Self::default();
        let settings: BTreeMap<String, BTreeMap<String, PianoRollChannelConfig>> = BTreeMap::deserialize(deserializer)?;

        for (chip, chip_settings) in settings {
            for (channel, channel_settings) in chip_settings {
                if let Some(settings) = result.settings_mut_by_name(&chip, &channel) {
                    let mut colors = settings.colors();
                    for (color_key, css_color) in channel_settings.colors.iter() {
                        let index = match (channel.as_str(), color_key.as_str()) {
                            ("Channel 1", "static") => 0,
                            ("Channel 2", "static") => 0,
                            ("Channel 3", "static") => 0,
                            ("Channel 4", "static") => 0,
                            ("Channel 5", "static") => 0,
                            ("Channel 6", "static") => 0,
                            ("Channel 7", "static") => 0,
                            ("Channel 8", "static") => 0,
                            _ => continue
                        };
                        colors.get_mut(index).map(|c| {
                            c.set_red(css_color.r as _);
                            c.set_green(css_color.g as _);
                            c.set_blue(css_color.b as _);
                            c.set_alpha(1.0);
                        });
                    }
                    settings.set_colors(&colors);
                    settings.set_hidden(channel_settings.hidden);
                }
            }
        }

        Ok(result)
    }
}