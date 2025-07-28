use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub formats: FormatsConfig,
    pub steps: Vec<StepConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatsConfig {
    pub available: Vec<FormatOption>,
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatOption {
    pub format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrates: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_bitrate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bit_depths: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_bit_depth: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitFile {
    pub file: String,
    pub start: String,
    pub end: String,
    #[serde(skip)]
    pub start_seconds: f64,
    #[serde(skip)]
    pub end_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagFile {
    pub file: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track: Option<u32>,
    pub track_total: Option<u32>,
    pub disk: Option<u32>,
    pub disk_total: Option<u32>,
    pub album_art: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StepConfig {
    Ffmpeg {
        input: String,
        output: String,
        #[serde(default)]
        args: Vec<String>,
    },
    Split {
        input: String,
        output_dir: String,
        files: Vec<SplitFile>,
    },
    Transcode {
        input_dir: String,
        output_dir: String,
        files: Vec<String>,
    },
    Tag {
        input_dir: String,
        files: Vec<TagFile>,
    },
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    pub fn has_transcode_step(&self) -> bool {
        self.steps.iter().any(|step| matches!(step, StepConfig::Transcode { .. }))
    }
}

#[derive(Debug, Clone)]
pub struct SelectedFormat {
    pub format: String,
    pub bitrate: Option<String>,
    pub bit_depth: Option<u8>,
}