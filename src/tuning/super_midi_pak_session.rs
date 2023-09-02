use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use serde::Deserialize;
use serde_json;
use base64::Engine as _;

#[derive(Deserialize, Clone)]
pub struct SampleCatalogEntry {
    pub id: String,
    pub name: String,
    pub loop_start: Option<i32>,
    pub brr: String
}

#[derive(Deserialize, Clone)]
pub struct SampleDirectoryEntry {
    pub idx: i32,
    pub sample_id: String,

    pub base_frequency: Option<f64>,

    pub volume_envelope_attack_time: i32,
    pub volume_envelope_decay_time: i32,
    pub volume_envelope_sustain_level: i32,
    pub volume_envelope_sustain_time: i32,

    pub pitch_envelope_attack_time: i32,
    pub pitch_envelope_decay_time: i32,
    pub pitch_envelope_amount: i32,

    pub i18: i32,
    pub i19: i32,
    pub selected: bool
}

#[derive(Deserialize, Clone)]
pub struct CustomTuningEntry {
    pub semitone: i32
}

#[derive(Deserialize, Clone)]
pub struct CustomTunings {
    pub tunings: Vec<CustomTuningEntry>
}

#[derive(Clone)]
pub struct SuperMidiPakSample {
    pub id: String,
    pub source: u8,
    pub name: String,
    pub loop_start: Option<i32>,
    pub brr: Vec<u8>,
    pub pitch: Option<f64>
}

impl fmt::Display for SuperMidiPakSample {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "${:02x} {} :: id='{}', f0={} Hz", self.source, self.name, self.id, self.pitch.unwrap_or(0.0))
    }
}

#[derive(Deserialize, Clone)]
pub struct SuperMidiPakSession {
    version: i32,
    #[serde(rename = "type")]
    session_type: String,

    #[serde(rename = "sample_catalogue")]
    sample_catalog: Vec<SampleCatalogEntry>,
    sample_directory: Vec<SampleDirectoryEntry>,

    global_settings: HashMap<String, i32>,
    channel_settings: Vec<HashMap<String, i32>>,
    custom_tunings: CustomTunings
}

impl SuperMidiPakSession {
    pub fn from_json(j: &str) -> Result<Self, String> {
        let result: Self = serde_json::from_str(j).map_err(|e| e.to_string())?;
        if result.session_type != "super_midi_pak_sample_uploader_session" {
            return Err("Invalid session file".to_string());
        }
        Ok(result)
    }

    pub fn version(&self) -> i32 {
        self.version
    }

    pub fn samples(&self) -> Result<Vec<SuperMidiPakSample>, String> {
        let mut result: Vec<SuperMidiPakSample> = Vec::new();

        for directory_entry in &self.sample_directory {
            let catalog_entry = self.sample_catalog
                .iter()
                .find(|&ce| ce.id.clone() == directory_entry.sample_id.clone())
                .ok_or(format!("No catalog entry for sample '{}'", directory_entry.sample_id))?;

            let brr = base64::engine::general_purpose::STANDARD_NO_PAD.decode(catalog_entry.brr.as_str())
                .map_err(|e| e.to_string())?;

            result.push(SuperMidiPakSample {
                id: directory_entry.sample_id.clone(),
                source: directory_entry.idx as u8,
                name: catalog_entry.name.clone(),
                loop_start: catalog_entry.loop_start,
                brr,
                pitch: directory_entry.base_frequency
            });
        }

        Ok(result)
    }
}
