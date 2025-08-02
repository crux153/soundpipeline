use clap::Parser;
use serde::{Deserialize, Serialize};

/// Default duration tolerance in seconds
const DEFAULT_DURATION_TOLERANCE: f64 = 3.0;

/// Default file scan pattern for file suggester
const DEFAULT_FILE_SCAN_PATTERN: &str = "*.mkv";

/// Application settings that can be configured via YAML, environment variables, or CLI flags
#[derive(Debug, Clone, Serialize, Deserialize, Parser)]
#[serde(default)]
pub struct Settings {
    /// Duration tolerance in seconds for ffmpeg step validation
    #[arg(
        long = "duration-tolerance", 
        env = "DURATION_TOLERANCE", 
        default_value_t = DEFAULT_DURATION_TOLERANCE,
        help = "Duration tolerance in seconds for ffmpeg step validation"
    )]
    #[serde(default = "default_duration_tolerance")]
    pub duration_tolerance: f64,

    /// File scan pattern for file suggester (glob pattern)
    #[arg(
        long = "file-scan-pattern",
        env = "FILE_SCAN_PATTERN",
        default_value = DEFAULT_FILE_SCAN_PATTERN,
        help = "Glob pattern for scanning files in file suggester"
    )]
    #[serde(default = "default_file_scan_pattern")]
    pub file_scan_pattern: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            duration_tolerance: DEFAULT_DURATION_TOLERANCE,
            file_scan_pattern: DEFAULT_FILE_SCAN_PATTERN.to_string(),
        }
    }
}

/// Default value function for serde
fn default_duration_tolerance() -> f64 {
    DEFAULT_DURATION_TOLERANCE
}

/// Default value function for serde
fn default_file_scan_pattern() -> String {
    DEFAULT_FILE_SCAN_PATTERN.to_string()
}

impl Settings {
    /// Merge settings from different sources with proper priority
    /// CLI/env settings override YAML settings
    pub fn merge_with_yaml(&mut self, yaml_settings: &Settings) {
        // CLI/env values are already parsed by clap, so only override if CLI used default
        // We'll compare with the default to see if CLI/env was actually provided
        if (self.duration_tolerance - DEFAULT_DURATION_TOLERANCE).abs() < f64::EPSILON {
            // CLI/env used default value, so use YAML if different from default
            if (yaml_settings.duration_tolerance - DEFAULT_DURATION_TOLERANCE).abs() >= f64::EPSILON {
                self.duration_tolerance = yaml_settings.duration_tolerance;
            }
        }
        
        if self.file_scan_pattern == DEFAULT_FILE_SCAN_PATTERN {
            // CLI/env used default value, so use YAML if different from default
            if yaml_settings.file_scan_pattern != DEFAULT_FILE_SCAN_PATTERN {
                self.file_scan_pattern = yaml_settings.file_scan_pattern.clone();
            }
        }
        // If CLI/env provided a non-default value, keep it (it takes priority)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.duration_tolerance, DEFAULT_DURATION_TOLERANCE);
    }

    #[test]
    fn test_default_duration_tolerance_function() {
        assert_eq!(default_duration_tolerance(), DEFAULT_DURATION_TOLERANCE);
    }

    #[test]
    fn test_merge_with_yaml_default_cli() {
        let mut cli_settings = Settings {
            duration_tolerance: DEFAULT_DURATION_TOLERANCE, // CLI used default
            file_scan_pattern: DEFAULT_FILE_SCAN_PATTERN.to_string(),
        };
        
        let yaml_settings = Settings {
            duration_tolerance: 6.0, // YAML provided different value
            file_scan_pattern: "*.mp4".to_string(),
        };
        
        cli_settings.merge_with_yaml(&yaml_settings);
        assert_eq!(cli_settings.duration_tolerance, 6.0); // YAML used since CLI was default
        assert_eq!(cli_settings.file_scan_pattern, "*.mp4"); // YAML used since CLI was default
    }

    #[test]
    fn test_merge_with_yaml_custom_cli() {
        let mut cli_settings = Settings {
            duration_tolerance: 4.0, // CLI provided custom value
            file_scan_pattern: "*.avi".to_string(), // CLI provided custom value
        };
        
        let yaml_settings = Settings {
            duration_tolerance: 6.0, // YAML provided different value
            file_scan_pattern: "*.mp4".to_string(), // YAML provided different value
        };
        
        cli_settings.merge_with_yaml(&yaml_settings);
        assert_eq!(cli_settings.duration_tolerance, 4.0); // CLI wins
        assert_eq!(cli_settings.file_scan_pattern, "*.avi"); // CLI wins
    }

    #[test]
    fn test_merge_with_yaml_both_default() {
        let mut cli_settings = Settings {
            duration_tolerance: DEFAULT_DURATION_TOLERANCE, // CLI used default
            file_scan_pattern: DEFAULT_FILE_SCAN_PATTERN.to_string(), // CLI used default
        };
        
        let yaml_settings = Settings {
            duration_tolerance: DEFAULT_DURATION_TOLERANCE, // YAML also has default
            file_scan_pattern: DEFAULT_FILE_SCAN_PATTERN.to_string(), // YAML also has default
        };
        
        cli_settings.merge_with_yaml(&yaml_settings);
        assert_eq!(cli_settings.duration_tolerance, DEFAULT_DURATION_TOLERANCE); // Default value kept
        assert_eq!(cli_settings.file_scan_pattern, DEFAULT_FILE_SCAN_PATTERN); // Default value kept
    }
}