use anyhow::Result;
use std::path::Path;
use std::process::Command;
use ffmpeg_sidecar::ffprobe::ffprobe_path;
use crate::config::{Config, StepConfig};

/// Information about a single duration check
#[derive(Debug, Clone)]
pub struct DurationCheckInfo {
    pub step_index: usize,
    pub input_file: String,
    pub expected_duration: String,
    pub expected_seconds: f64,
    pub actual_seconds: f64,
    pub difference_seconds: f64,
    pub is_valid: bool,
}

/// Result of duration checking
#[derive(Debug)]
pub struct DurationCheckResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub checks: Vec<DurationCheckInfo>,
}

impl DurationCheckResult {
    pub fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            checks: Vec::new(),
        }
    }

    pub fn add_check(&mut self, check: DurationCheckInfo) {
        self.checks.push(check);
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
        self.is_valid = false;
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

/// Parse time string in h:mm:ss format to seconds
fn parse_time_to_seconds(time_str: &str) -> Result<f64> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 3 {
        anyhow::bail!("Invalid time format '{}'. Expected h:mm:ss", time_str);
    }

    let hours: f64 = parts[0].parse()
        .map_err(|_| anyhow::anyhow!("Invalid hours in time '{}'", time_str))?;
    let minutes: f64 = parts[1].parse()
        .map_err(|_| anyhow::anyhow!("Invalid minutes in time '{}'", time_str))?;
    let seconds: f64 = parts[2].parse()
        .map_err(|_| anyhow::anyhow!("Invalid seconds in time '{}'", time_str))?;

    if minutes >= 60.0 {
        anyhow::bail!("Invalid minutes '{}' in time '{}'. Must be less than 60", minutes, time_str);
    }
    if seconds >= 60.0 {
        anyhow::bail!("Invalid seconds '{}' in time '{}'. Must be less than 60", seconds, time_str);
    }

    Ok(hours * 3600.0 + minutes * 60.0 + seconds)
}

/// Get duration of a media file using ffprobe
fn get_file_duration(file_path: &Path) -> Result<f64> {
    let ffprobe_path = ffprobe_path();
    
    let output = Command::new(ffprobe_path)
        .args([
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(file_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ffprobe failed for file '{}': {}", file_path.display(), stderr);
    }

    let duration_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let duration: f64 = duration_str.parse()
        .map_err(|_| anyhow::anyhow!("Failed to parse duration '{}' for file '{}'", duration_str, file_path.display()))?;

    Ok(duration)
}

/// Check durations for ffmpeg steps that have input_duration specified
pub fn check_durations(config: &Config, working_dir: &Path, tolerance: f64) -> Result<DurationCheckResult> {
    let mut result = DurationCheckResult::new();

    tracing::info!("Starting duration check for ffmpeg steps...");

    for (idx, step) in config.steps.iter().enumerate() {
        if let StepConfig::Ffmpeg { input, input_duration: Some(expected_duration), .. } = step {
            tracing::debug!("Checking duration for step {} (ffmpeg): input='{}', expected_duration='{}'", 
                           idx + 1, input, expected_duration);

            // Parse expected duration
            let expected_seconds = match parse_time_to_seconds(expected_duration) {
                Ok(seconds) => seconds,
                Err(e) => {
                    result.add_error(format!(
                        "Step {} (ffmpeg): Invalid input_duration format '{}': {}",
                        idx + 1, expected_duration, e
                    ));
                    continue;
                }
            };

            // Resolve input file path
            let input_path = if Path::new(input).is_absolute() {
                Path::new(input).to_path_buf()
            } else {
                working_dir.join(input)
            };

            // Check if file exists
            if !input_path.exists() {
                tracing::warn!("Step {} (ffmpeg): Input file '{}' does not exist, will be handled by file suggester",
                               idx + 1, input_path.display());
                
                // Create a check info with invalid status but no actual duration
                let check_info = DurationCheckInfo {
                    step_index: idx + 1,
                    input_file: input.clone(),
                    expected_duration: expected_duration.clone(),
                    expected_seconds,
                    actual_seconds: 0.0, // File doesn't exist
                    difference_seconds: expected_seconds, // Max difference since file doesn't exist
                    is_valid: false,
                };
                
                result.add_check(check_info);
                result.add_error(format!(
                    "Step {} (ffmpeg): Input file '{}' does not exist",
                    idx + 1, input_path.display()
                ));
                continue;
            }

            // Get actual duration using ffprobe
            let actual_seconds = match get_file_duration(&input_path) {
                Ok(duration) => duration,
                Err(e) => {
                    result.add_error(format!(
                        "Step {} (ffmpeg): Failed to get duration for '{}': {}",
                        idx + 1, input_path.display(), e
                    ));
                    continue;
                }
            };

            // Check if durations match within tolerance
            let duration_diff = (expected_seconds - actual_seconds).abs();
            let is_valid = duration_diff < tolerance;

            // Create duration check info
            let check_info = DurationCheckInfo {
                step_index: idx + 1,
                input_file: input.clone(),
                expected_duration: expected_duration.clone(),
                expected_seconds,
                actual_seconds,
                difference_seconds: duration_diff,
                is_valid,
            };
            
            result.add_check(check_info);

            if !is_valid {
                result.add_error(format!(
                    "Step {} (ffmpeg): Duration mismatch for '{}'. Expected: {:.2}s ({}) vs Actual: {:.2}s (difference: {:.2}s)",
                    idx + 1, input, expected_seconds, expected_duration, actual_seconds, duration_diff
                ));
            } else {
                tracing::info!(
                    "Step {} (ffmpeg): Duration check passed for '{}'. Expected: {:.2}s vs Actual: {:.2}s (difference: {:.2}s)",
                    idx + 1, input, expected_seconds, actual_seconds, duration_diff
                );
            }
        }
    }

    tracing::info!("Duration check completed: {} errors, {} warnings", 
                  result.errors.len(), result.warnings.len());

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_to_seconds() {
        assert_eq!(parse_time_to_seconds("0:00:00").unwrap(), 0.0);
        assert_eq!(parse_time_to_seconds("0:01:00").unwrap(), 60.0);
        assert_eq!(parse_time_to_seconds("1:00:00").unwrap(), 3600.0);
        assert_eq!(parse_time_to_seconds("1:23:45").unwrap(), 5025.0);
        assert_eq!(parse_time_to_seconds("0:00:30").unwrap(), 30.0);
        assert_eq!(parse_time_to_seconds("2:30:15").unwrap(), 9015.0);
    }

    #[test]
    fn test_parse_time_to_seconds_with_decimals() {
        assert_eq!(parse_time_to_seconds("0:00:30.5").unwrap(), 30.5);
        assert_eq!(parse_time_to_seconds("1:23:45.25").unwrap(), 5025.25);
    }

    #[test]
    fn test_parse_time_to_seconds_invalid_format() {
        assert!(parse_time_to_seconds("00:00").is_err());
        assert!(parse_time_to_seconds("1:2:3:4").is_err());
        assert!(parse_time_to_seconds("abc:def:ghi").is_err());
    }

    #[test]
    fn test_parse_time_to_seconds_invalid_values() {
        assert!(parse_time_to_seconds("0:60:00").is_err()); // 60 minutes
        assert!(parse_time_to_seconds("0:00:60").is_err()); // 60 seconds
        assert!(parse_time_to_seconds("0:99:30").is_err()); // 99 minutes
    }

    #[test]
    fn test_duration_check_result() {
        let mut result = DurationCheckResult::new();
        assert!(result.is_valid);
        assert_eq!(result.errors.len(), 0);
        assert_eq!(result.warnings.len(), 0);
        assert_eq!(result.checks.len(), 0);

        result.add_warning("Test warning".to_string());
        assert!(result.is_valid);
        assert_eq!(result.warnings.len(), 1);

        result.add_error("Test error".to_string());
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);

        // Test adding check info
        let check_info = DurationCheckInfo {
            step_index: 1,
            input_file: "test.mkv".to_string(),
            expected_duration: "0:02:30".to_string(),
            expected_seconds: 150.0,
            actual_seconds: 152.5,
            difference_seconds: 2.5,
            is_valid: true,
        };
        result.add_check(check_info);
        assert_eq!(result.checks.len(), 1);
        assert_eq!(result.checks[0].step_index, 1);
        assert_eq!(result.checks[0].input_file, "test.mkv");
        assert_eq!(result.checks[0].expected_duration, "0:02:30");
        assert_eq!(result.checks[0].expected_seconds, 150.0);
        assert_eq!(result.checks[0].actual_seconds, 152.5);
        assert_eq!(result.checks[0].difference_seconds, 2.5);
        assert!(result.checks[0].is_valid);
    }
}