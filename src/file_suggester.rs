use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;
use ffmpeg_sidecar::ffprobe::ffprobe_path;
use dialoguer::Confirm;

/// Information about a potential replacement file
#[derive(Debug, Clone)]
pub struct FileSuggestion {
    pub file_path: PathBuf,
    pub duration_seconds: f64,
    pub difference_seconds: f64,
}

/// Scan working directory for MKV files and get their durations
pub fn scan_mkv_files(working_dir: &Path) -> Result<Vec<(PathBuf, f64)>> {
    let mut mkv_files = Vec::new();
    
    // Read directory entries
    let entries = std::fs::read_dir(working_dir)?;
    
    for entry in entries.flatten() {
        let path = entry.path();
        
        // Check if it's an MKV file
        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension.to_string_lossy().to_lowercase() == "mkv" {
                    // Get duration using ffprobe
                    match get_file_duration(&path) {
                        Ok(duration) => {
                            tracing::debug!("Found MKV file: {} (duration: {:.2}s)", 
                                          path.display(), duration);
                            mkv_files.push((path, duration));
                        }
                        Err(e) => {
                            tracing::warn!("Failed to get duration for {}: {}", 
                                         path.display(), e);
                        }
                    }
                }
            }
        }
    }
    
    tracing::info!("Scanned working directory: found {} MKV files", mkv_files.len());
    Ok(mkv_files)
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

/// Find the best matching MKV file by duration
pub fn find_best_match(
    mkv_files: &[(PathBuf, f64)], 
    target_duration: f64, 
    tolerance: f64
) -> Option<FileSuggestion> {
    let mut best_match: Option<FileSuggestion> = None;
    
    for (file_path, duration) in mkv_files {
        let difference = (target_duration - duration).abs();
        
        // Only consider files within tolerance
        if difference < tolerance {
            let suggestion = FileSuggestion {
                file_path: file_path.clone(),
                duration_seconds: *duration,
                difference_seconds: difference,
            };
            
            // Update best match if this one is closer
            match &best_match {
                None => best_match = Some(suggestion),
                Some(current_best) => {
                    if difference < current_best.difference_seconds {
                        best_match = Some(suggestion);
                    }
                }
            }
        }
    }
    
    best_match
}

/// Ask user for confirmation to replace the file
pub fn confirm_file_replacement(
    original_file: &str,
    suggested_file: &FileSuggestion,
    expected_duration: f64,
    file_exists: bool,
) -> Result<bool> {
    println!();
    if file_exists {
        println!("🔍 Duration mismatch detected!");
        println!("   Original file: {}", original_file);
        println!("   Expected duration: {:.2}s", expected_duration);
    } else {
        println!("❌ Missing file detected!");
        println!("   Original file: {} (not found)", original_file);
        println!("   Expected duration: {:.2}s", expected_duration);
    }
    println!();
    println!("💡 Found a potential replacement:");
    println!("   Suggested file: {}", suggested_file.file_path.display());
    println!("   File duration: {:.2}s", suggested_file.duration_seconds);
    println!("   Difference from expected: {:.2}s", suggested_file.difference_seconds);
    println!();
    
    let prompt = if file_exists {
        "Would you like to use this file instead?"
    } else {
        "Would you like to use this file?"
    };
    
    let confirmed = Confirm::new()
        .with_prompt(prompt)
        .default(true)
        .interact()?;
    
    Ok(confirmed)
}

/// Get suggested replacement for a file with duration mismatch or missing file
pub fn suggest_replacement(
    working_dir: &Path,
    original_file: &str, 
    expected_duration: f64,
    tolerance: f64,
) -> Result<Option<PathBuf>> {
    // Check if original file exists
    let original_path = if Path::new(original_file).is_absolute() {
        Path::new(original_file).to_path_buf()
    } else {
        working_dir.join(original_file)
    };
    let file_exists = original_path.exists();
    
    if file_exists {
        tracing::info!("Searching for alternative MKV files with similar duration (file exists but duration mismatch)...");
    } else {
        tracing::info!("Searching for alternative MKV files to replace missing file '{}'...", original_file);
    }
    
    // Scan working directory for MKV files
    let mkv_files = scan_mkv_files(working_dir)?;
    
    if mkv_files.is_empty() {
        tracing::info!("No MKV files found in working directory for replacement suggestion");
        return Ok(None);
    }
    
    // Find best matching file
    let best_match = find_best_match(&mkv_files, expected_duration, tolerance);
    
    match best_match {
        Some(suggestion) => {
            tracing::info!("Found potential replacement: {} (duration: {:.2}s, diff: {:.2}s)",
                          suggestion.file_path.display(), 
                          suggestion.duration_seconds, 
                          suggestion.difference_seconds);
            
            // Ask user for confirmation
            if confirm_file_replacement(original_file, &suggestion, expected_duration, file_exists)? {
                let action = if file_exists { "replaced" } else { "selected" };
                tracing::info!("User confirmed replacement: {} {} with {}", 
                              original_file, action, suggestion.file_path.display());
                Ok(Some(suggestion.file_path))
            } else {
                tracing::info!("User declined replacement suggestion");
                Ok(None)
            }
        }
        None => {
            tracing::info!("No suitable replacement files found within {:.1}s tolerance", tolerance);
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_best_match() {
        let mkv_files = vec![
            (PathBuf::from("file1.mkv"), 100.0),
            (PathBuf::from("file2.mkv"), 150.0),
            (PathBuf::from("file3.mkv"), 152.5),
            (PathBuf::from("file4.mkv"), 200.0),
        ];
        
        // Test finding best match within tolerance
        let result = find_best_match(&mkv_files, 151.0, 3.0);
        assert!(result.is_some());
        let suggestion = result.unwrap();
        assert_eq!(suggestion.file_path, PathBuf::from("file2.mkv"));
        assert_eq!(suggestion.duration_seconds, 150.0);
        assert_eq!(suggestion.difference_seconds, 1.0);
    }

    #[test]
    fn test_find_best_match_exact() {
        let mkv_files = vec![
            (PathBuf::from("exact.mkv"), 150.0),
            (PathBuf::from("close.mkv"), 152.0),
        ];
        
        let result = find_best_match(&mkv_files, 150.0, 3.0);
        assert!(result.is_some());
        let suggestion = result.unwrap();
        assert_eq!(suggestion.file_path, PathBuf::from("exact.mkv"));
        assert_eq!(suggestion.difference_seconds, 0.0);
    }

    #[test]
    fn test_find_best_match_no_match() {
        let mkv_files = vec![
            (PathBuf::from("too_short.mkv"), 100.0),
            (PathBuf::from("too_long.mkv"), 200.0),
        ];
        
        let result = find_best_match(&mkv_files, 150.0, 3.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_best_match_empty() {
        let mkv_files = vec![];
        let result = find_best_match(&mkv_files, 150.0, 3.0);
        assert!(result.is_none());
    }
}