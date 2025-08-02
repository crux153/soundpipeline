use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub syntax: String,
    pub syntax_version: u32,
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
        #[serde(skip_serializing_if = "Option::is_none")]
        input_duration: Option<String>,
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
    Cleanup {
        files: Vec<String>,
    },
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        
        // Validate syntax
        if config.syntax != "soundpipeline" {
            anyhow::bail!(
                "Invalid configuration syntax. Expected 'soundpipeline', found '{}'", 
                config.syntax
            );
        }
        
        // Validate version
        if config.syntax_version != 1 {
            anyhow::bail!(
                "Unsupported configuration version {}. This version of soundpipeline only supports syntax_version: 1", 
                config.syntax_version
            );
        }
        
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    fn create_test_config_yaml() -> &'static str {
        r#"
syntax: soundpipeline
syntax_version: 1
formats:
  available:
    - format: mp3
      bitrates: ["320k", "256k", "192k"]
      default_bitrate: "320k"
    - format: flac
      bit_depths: [16, 24]
      default_bit_depth: 24
    - format: alac
      bit_depths: [16, 24, 32]
      default_bit_depth: 24
  default: mp3
steps:
  - type: ffmpeg
    input: "input.mkv"
    output: "audio.wav"
    args: ["-vn", "-acodec", "pcm_s16le"]
  - type: split
    input: "audio.wav"
    output_dir: "split"
    files:
      - file: "track_01.wav"
        start: "0:00:00.000"
        end: "0:03:30.000"
      - file: "track_02.wav"
        start: "0:03:30.000"
        end: "0:07:15.500"
  - type: transcode
    input_dir: "split"
    output_dir: "output"
    files: ["track_01.wav", "track_02.wav"]
  - type: tag
    input_dir: "output"
    files:
      - file: "track_01.*"
        title: "First Track"
        artist: "Test Artist"
        album: "Test Album"
        track: 1
        track_total: 2
      - file: "track_02.*"
        title: "Second Track"
        artist: "Test Artist"
        album: "Test Album"
        track: 2
        track_total: 2
        album_art: "cover.jpg"
  - type: cleanup
    files: ["split", "audio.wav"]
"#
    }

    fn create_invalid_syntax_yaml() -> &'static str {
        r#"
syntax: invalid
syntax_version: 1
formats:
  available: []
steps: []
"#
    }

    fn create_invalid_version_yaml() -> &'static str {
        r#"
syntax: soundpipeline
syntax_version: 2
formats:
  available: []
steps: []
"#
    }

    #[test]
    fn test_config_from_file_success() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "{}", create_test_config_yaml()).unwrap();
        
        let config = Config::from_file(temp_file.path()).unwrap();
        
        assert_eq!(config.syntax, "soundpipeline");
        assert_eq!(config.syntax_version, 1);
        assert_eq!(config.formats.available.len(), 3);
        assert_eq!(config.formats.default, Some("mp3".to_string()));
        assert_eq!(config.steps.len(), 5);
    }

    #[test]
    fn test_config_from_file_invalid_syntax() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "{}", create_invalid_syntax_yaml()).unwrap();
        
        let result = Config::from_file(temp_file.path());
        
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Invalid configuration syntax"));
        assert!(error.contains("Expected 'soundpipeline', found 'invalid'"));
    }

    #[test]
    fn test_config_from_file_invalid_version() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "{}", create_invalid_version_yaml()).unwrap();
        
        let result = Config::from_file(temp_file.path());
        
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Unsupported configuration version 2"));
        assert!(error.contains("only supports syntax_version: 1"));
    }

    #[test]
    fn test_config_from_file_nonexistent() {
        let result = Config::from_file("nonexistent.yml");
        
        assert!(result.is_err());
        // Should be a file not found error (ErrorKind::NotFound)
        let error = result.unwrap_err();
        if let Some(io_error) = error.downcast_ref::<std::io::Error>() {
            assert_eq!(io_error.kind(), std::io::ErrorKind::NotFound);
        } else {
            panic!("Expected std::io::Error with NotFound kind");
        }
    }

    #[test]
    fn test_config_from_file_invalid_yaml() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "invalid: yaml: content: [").unwrap();
        
        let result = Config::from_file(temp_file.path());
        
        assert!(result.is_err());
        // Should be a YAML parsing error
    }

    #[test]
    fn test_has_transcode_step() {
        let mut config = Config {
            syntax: "soundpipeline".to_string(),
            syntax_version: 1,
            formats: FormatsConfig {
                available: vec![],
                default: None,
            },
            steps: vec![],
        };

        // Initially no transcode step
        assert!(!config.has_transcode_step());

        // Add ffmpeg step - still no transcode
        config.steps.push(StepConfig::Ffmpeg {
            input: "input.mkv".to_string(),
            output: "audio.wav".to_string(),
            args: vec![],
            input_duration: None,
        });
        assert!(!config.has_transcode_step());

        // Add split step - still no transcode
        config.steps.push(StepConfig::Split {
            input: "audio.wav".to_string(),
            output_dir: "split".to_string(),
            files: vec![],
        });
        assert!(!config.has_transcode_step());

        // Add transcode step - now has transcode
        config.steps.push(StepConfig::Transcode {
            input_dir: "split".to_string(),
            output_dir: "output".to_string(),
            files: vec!["track.wav".to_string()],
        });
        assert!(config.has_transcode_step());

        // Add more steps - still has transcode
        config.steps.push(StepConfig::Tag {
            input_dir: "output".to_string(),
            files: vec![],
        });
        config.steps.push(StepConfig::Cleanup {
            files: vec!["split".to_string()],
        });
        assert!(config.has_transcode_step());
    }

    #[test]
    fn test_format_option_deserialization() {
        let yaml = r#"
format: mp3
bitrates: ["320k", "256k"]
default_bitrate: "320k"
"#;
        
        let format_option: FormatOption = serde_yaml::from_str(yaml).unwrap();
        
        assert_eq!(format_option.format, "mp3");
        assert_eq!(format_option.bitrates, Some(vec!["320k".to_string(), "256k".to_string()]));
        assert_eq!(format_option.default_bitrate, Some("320k".to_string()));
        assert_eq!(format_option.bit_depths, None);
        assert_eq!(format_option.default_bit_depth, None);
    }

    #[test]
    fn test_format_option_serialization() {
        let format_option = FormatOption {
            format: "flac".to_string(),
            bitrates: None,
            default_bitrate: None,
            bit_depths: Some(vec![16, 24]),
            default_bit_depth: Some(24),
        };
        
        let yaml = serde_yaml::to_string(&format_option).unwrap();
        
        assert!(yaml.contains("format: flac"));
        assert!(yaml.contains("bit_depths:"));
        assert!(yaml.contains("- 16"));
        assert!(yaml.contains("- 24"));
        assert!(yaml.contains("default_bit_depth: 24"));
        // Optional fields should be omitted when None
        assert!(!yaml.contains("bitrates"));
        assert!(!yaml.contains("default_bitrate"));
    }

    #[test]
    fn test_step_config_ffmpeg_deserialization() {
        let yaml = r#"
type: ffmpeg
input: "video.mkv"
output: "audio.wav"
args: ["-vn", "-acodec", "pcm_s16le"]
"#;
        
        let step: StepConfig = serde_yaml::from_str(yaml).unwrap();
        
        if let StepConfig::Ffmpeg { input, output, args, input_duration } = step {
            assert_eq!(input, "video.mkv");
            assert_eq!(output, "audio.wav");
            assert_eq!(args, vec!["-vn", "-acodec", "pcm_s16le"]);
            assert_eq!(input_duration, None);
        } else {
            panic!("Expected Ffmpeg step");
        }
    }

    #[test]
    fn test_step_config_ffmpeg_default_args() {
        let yaml = r#"
type: ffmpeg
input: "video.mkv"
output: "audio.wav"
"#;
        
        let step: StepConfig = serde_yaml::from_str(yaml).unwrap();
        
        if let StepConfig::Ffmpeg { input, output, args, input_duration } = step {
            assert_eq!(input, "video.mkv");
            assert_eq!(output, "audio.wav");
            assert_eq!(args, Vec::<String>::new()); // Default empty args
            assert_eq!(input_duration, None);
        } else {
            panic!("Expected Ffmpeg step");
        }
    }

    #[test]
    fn test_step_config_ffmpeg_with_input_duration() {
        let yaml = r#"
type: ffmpeg
input: "video.mkv"
output: "audio.wav"
args: ["-vn"]
input_duration: "1:23:45"
"#;
        
        let step: StepConfig = serde_yaml::from_str(yaml).unwrap();
        
        if let StepConfig::Ffmpeg { input, output, args, input_duration } = step {
            assert_eq!(input, "video.mkv");
            assert_eq!(output, "audio.wav");
            assert_eq!(args, vec!["-vn"]);
            assert_eq!(input_duration, Some("1:23:45".to_string()));
        } else {
            panic!("Expected Ffmpeg step");
        }
    }

    #[test]
    fn test_step_config_split_deserialization() {
        let yaml = r#"
type: split
input: "audio.wav"
output_dir: "split"
files:
  - file: "track_01.wav"
    start: "0:00:00.000"
    end: "0:03:30.000"
  - file: "track_02.wav"
    start: "0:03:30.000"
    end: "0:07:15.500"
"#;
        
        let step: StepConfig = serde_yaml::from_str(yaml).unwrap();
        
        if let StepConfig::Split { input, output_dir, files } = step {
            assert_eq!(input, "audio.wav");
            assert_eq!(output_dir, "split");
            assert_eq!(files.len(), 2);
            assert_eq!(files[0].file, "track_01.wav");
            assert_eq!(files[0].start, "0:00:00.000");
            assert_eq!(files[0].end, "0:03:30.000");
            assert_eq!(files[1].file, "track_02.wav");
            assert_eq!(files[1].start, "0:03:30.000");
            assert_eq!(files[1].end, "0:07:15.500");
        } else {
            panic!("Expected Split step");
        }
    }

    #[test]
    fn test_step_config_transcode_deserialization() {
        let yaml = r#"
type: transcode
input_dir: "split"
output_dir: "output"
files: ["track_01.wav", "track_02.wav"]
"#;
        
        let step: StepConfig = serde_yaml::from_str(yaml).unwrap();
        
        if let StepConfig::Transcode { input_dir, output_dir, files } = step {
            assert_eq!(input_dir, "split");
            assert_eq!(output_dir, "output");
            assert_eq!(files, vec!["track_01.wav", "track_02.wav"]);
        } else {
            panic!("Expected Transcode step");
        }
    }

    #[test]
    fn test_step_config_tag_deserialization() {
        let yaml = r#"
type: tag
input_dir: "output"
files:
  - file: "track_01.*"
    title: "First Track"
    artist: "Test Artist"
    album: "Test Album"
    track: 1
    track_total: 2
    album_art: "cover.jpg"
"#;
        
        let step: StepConfig = serde_yaml::from_str(yaml).unwrap();
        
        if let StepConfig::Tag { input_dir, files } = step {
            assert_eq!(input_dir, "output");
            assert_eq!(files.len(), 1);
            let tag_file = &files[0];
            assert_eq!(tag_file.file, "track_01.*");
            assert_eq!(tag_file.title, Some("First Track".to_string()));
            assert_eq!(tag_file.artist, Some("Test Artist".to_string()));
            assert_eq!(tag_file.album, Some("Test Album".to_string()));
            assert_eq!(tag_file.track, Some(1));
            assert_eq!(tag_file.track_total, Some(2));
            assert_eq!(tag_file.album_art, Some("cover.jpg".to_string()));
        } else {
            panic!("Expected Tag step");
        }
    }

    #[test]
    fn test_step_config_cleanup_deserialization() {
        let yaml = r#"
type: cleanup
files: ["split", "audio.wav", "temp"]
"#;
        
        let step: StepConfig = serde_yaml::from_str(yaml).unwrap();
        
        if let StepConfig::Cleanup { files } = step {
            assert_eq!(files, vec!["split", "audio.wav", "temp"]);
        } else {
            panic!("Expected Cleanup step");
        }
    }

    #[test]
    fn test_split_file_skipped_fields() {
        let split_file = SplitFile {
            file: "track.wav".to_string(),
            start: "0:00:00.000".to_string(),
            end: "0:03:00.000".to_string(),
            start_seconds: 0.0,
            end_seconds: 180.0,
        };
        
        let yaml = serde_yaml::to_string(&split_file).unwrap();
        
        // The #[serde(skip)] fields should not be serialized
        assert!(yaml.contains("file: track.wav"));
        assert!(yaml.contains("start: 0:00:00.000"));
        assert!(yaml.contains("end: 0:03:00.000"));
        assert!(!yaml.contains("start_seconds"));
        assert!(!yaml.contains("end_seconds"));
    }

    #[test]
    fn test_tag_file_optional_fields() {
        let tag_file = TagFile {
            file: "track.*".to_string(),
            title: Some("Test".to_string()),
            artist: None,
            album: None,
            album_artist: None,
            track: Some(1),
            track_total: None,
            disk: None,
            disk_total: None,
            album_art: None,
            genre: None,
            year: None,
            comment: None,
        };
        
        let yaml = serde_yaml::to_string(&tag_file).unwrap();
        
        assert!(yaml.contains("file: track.*"));
        assert!(yaml.contains("title: Test"));
        assert!(yaml.contains("track: 1"));
        // None fields are serialized as null
        assert!(yaml.contains("artist: null"));
        assert!(yaml.contains("album: null"));
        // Some fields should be present, some should be null
        assert!(yaml.contains("track: 1"));
        assert!(yaml.contains("track_total: null"));
    }

    #[test]
    fn test_config_full_round_trip() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "{}", create_test_config_yaml()).unwrap();
        
        // Load config from file
        let config = Config::from_file(temp_file.path()).unwrap();
        
        // Serialize back to YAML
        let serialized = serde_yaml::to_string(&config).unwrap();
        
        // Parse the serialized YAML back
        let config2: Config = serde_yaml::from_str(&serialized).unwrap();
        
        // Should be equivalent
        assert_eq!(config.syntax, config2.syntax);
        assert_eq!(config.syntax_version, config2.syntax_version);
        assert_eq!(config.formats.available.len(), config2.formats.available.len());
        assert_eq!(config.steps.len(), config2.steps.len());
    }
}