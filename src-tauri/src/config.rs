use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// MIDI note that maps to Medium octave, degree 1 (default: C4 = 60)
    pub reference_midi_note: u8,

    /// Tempo multiplier (1.0 = normal speed)
    pub tempo_factor: f64,

    /// Transpose in semitones (-24 to +24)
    pub transpose: i32,

    /// Maximum simultaneous notes (1-3)
    pub max_polyphony: u8,

    /// Delay before playback starts (ms)
    pub start_delay_ms: u64,

    /// Key mappings for each octave
    pub key_mapping: KeyMapping,

    /// Global hotkey bindings
    pub hotkeys: Hotkeys,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMapping {
    pub high: Vec<String>,
    pub medium: Vec<String>,
    pub low: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hotkeys {
    pub play_pause: String,
    pub stop: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            reference_midi_note: 60, // C4
            tempo_factor: 1.0,
            transpose: 0,
            max_polyphony: 2,
            start_delay_ms: 500,
            key_mapping: KeyMapping::default(),
            hotkeys: Hotkeys::default(),
        }
    }
}

impl Default for KeyMapping {
    fn default() -> Self {
        Self {
            high: vec!["Q", "W", "E", "R", "T", "Y", "U"]
                .into_iter()
                .map(String::from)
                .collect(),
            medium: vec!["A", "S", "D", "F", "G", "H", "J"]
                .into_iter()
                .map(String::from)
                .collect(),
            low: vec!["Z", "X", "C", "V", "B", "N", "M"]
                .into_iter()
                .map(String::from)
                .collect(),
        }
    }
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            play_pause: "F7".to_string(),
            stop: "F8".to_string(),
        }
    }
}

impl AppConfig {
    /// Get the config directory path
    fn config_dir() -> Result<PathBuf> {
        let proj_dirs = directories::ProjectDirs::from("com", "wwmp", "WWMP")
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
        Ok(proj_dirs.config_dir().to_path_buf())
    }

    /// Get the config file path
    fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.json"))
    }

    /// Load config from disk, or return default if not found
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let config: AppConfig = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Save config to disk
    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir()?;
        fs::create_dir_all(&dir)?;

        let path = Self::config_path()?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}
