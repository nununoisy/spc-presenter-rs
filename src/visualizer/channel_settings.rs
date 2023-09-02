use std::collections::HashMap;
use raqote::Color;
use super::ChannelState;

#[derive(Clone)]
pub struct ChannelSettings(String, bool, Vec<Color>);

impl ChannelSettings {
    pub fn new(name: &str, colors: &[Color]) -> Self {
        Self(name.to_string(), false, colors.to_vec())
    }

    pub fn name(&self) -> String {
        self.0.clone()
    }

    pub fn hidden(&self) -> bool {
        self.1
    }

    pub fn color(&self, state: &ChannelState) -> Option<Color> {
        let result = self.2.get(state.timbre).cloned();
        if let Some(color) = &result {
            if state.volume == 0 {
                return Some(Color::new(color.a(), color.r() / 2 + 0x10, color.g() / 2 + 0x10, color.b() / 2 + 0x10));
            }
        }
        result
    }

    pub fn colors(&self) -> Vec<Color> {
        self.2.clone()
    }

    pub fn num_colors(&self) -> usize {
        self.2.len()
    }

    pub fn set_hidden(&mut self, hidden: bool) {
        self.1 = hidden;
    }

    pub fn set_colors(&mut self, colors: &[Color]) {
        self.2 = colors.to_vec();
    }
}

#[derive(Clone)]
pub struct ChannelSettingsManager {
    settings: Vec<ChannelSettings>
}

impl ChannelSettingsManager {
    pub fn new(settings: Vec<ChannelSettings>) -> Self {
        if settings.len() != 8 {
            panic!("Invalid settings instantiation!");
        }

        Self {
            settings
        }
    }

    pub fn settings(&self, channel: usize) -> ChannelSettings {
        self.settings.get(channel).cloned().unwrap()
    }

    pub fn settings_mut(&mut self, channel: usize) -> &mut ChannelSettings {
        self.settings.get_mut(channel).unwrap()
    }

    pub fn put_per_sample_colors(&mut self, sample_colors: HashMap<u8, Color>) {
        for settings in self.settings.iter_mut() {
            let base_color = settings.colors().first().cloned().unwrap_or(Color::new(0xFF, 0xFF, 0xA0, 0xA0));

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
        Self {
            settings: vec![
                ChannelSettings::new("Channel 1", &[
                    Color::new(0xFF, 0xFF, 0xA0, 0xA0)
                ]),
                ChannelSettings::new("Channel 2", &[
                    Color::new(0xFF, 0xFF, 0xE0, 0xA0)
                ]),
                ChannelSettings::new("Channel 3", &[
                    Color::new(0xFF, 0x40, 0xFF, 0x40)
                ]),
                ChannelSettings::new("Channel 4", &[
                    Color::new(0xFF, 0xC0, 0xC0, 0xC0)
                ]),
                ChannelSettings::new("Channel 5", &[
                    Color::new(0xFF, 0x9A, 0x4F, 0xFF)
                ]),
                ChannelSettings::new("Channel 6", &[
                    Color::new(0xFF, 0x38, 0xAB, 0xF2)
                ]),
                ChannelSettings::new("Channel 7", &[
                    Color::new(0xFF, 0xAC, 0xED, 0x32)
                ]),
                ChannelSettings::new("Channel 8", &[
                    Color::new(0xFF, 0x24, 0x7B, 0xA0)
                ])
            ]
        }
    }
}
