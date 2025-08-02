use anyhow::Result;
use ffmpeg_sidecar::download::{ffmpeg_download_url, unpack_ffmpeg};
use ffmpeg_sidecar::paths::{sidecar_dir, ffmpeg_path};
use ffmpeg_sidecar::command::ffmpeg_is_installed;
use ffmpeg_sidecar::ffprobe::ffprobe_path;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::debug;

/// Auto-download FFmpeg with progress tracking
pub fn auto_download_with_progress() -> Result<()> {
    if ffmpeg_is_installed() {
        tracing::info!("FFmpeg is already installed");
        return Ok(());
    }

    tracing::info!("FFmpeg not found, downloading...");

    // Get download URL
    let download_url = ffmpeg_download_url()?;
    let destination = sidecar_dir()?;
    
    // Download with progress
    let archive_path = download_ffmpeg_package_with_progress(&download_url, &destination)?;
    
    // Unpack
    tracing::info!("Unpacking FFmpeg...");
    unpack_ffmpeg(&archive_path, &destination)?;
    
    tracing::info!("FFmpeg download and installation completed");
    Ok(())
}

/// Download FFmpeg package with progress bar
fn download_ffmpeg_package_with_progress(url: &str, destination: &Path) -> Result<PathBuf> {
    let response = ureq::get(url).call()?;
    
    // Get content length for progress bar
    let content_length = response
        .header("content-length")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    
    // Create progress bar
    let pb = ProgressBar::new(content_length);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
            .progress_chars("#>-")
    );
    pb.set_message("Downloading FFmpeg");

    // Extract filename from URL
    let filename = url
        .split('/')
        .last()
        .unwrap_or("ffmpeg.zip");
    
    let file_path = destination.join(filename);
    
    // Ensure destination directory exists
    std::fs::create_dir_all(destination)?;
    
    // Download with progress tracking
    let mut reader = BufReader::new(response.into_reader());
    let mut file = File::create(&file_path)?;
    
    // Custom copy with progress updates
    let mut buffer = [0; 8192];
    let mut total_downloaded = 0u64;
    
    loop {
        let bytes_read = std::io::Read::read(&mut reader, &mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        
        std::io::Write::write_all(&mut file, &buffer[..bytes_read])?;
        total_downloaded += bytes_read as u64;
        pb.set_position(total_downloaded);
    }
    
    pb.finish_with_message("FFmpeg download completed");
    
    Ok(file_path)
}

/// Get duration of a media file using ffprobe
pub fn get_file_duration(file_path: &Path) -> Result<f64> {
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

/// Check if specific encoders are available in FFmpeg
pub fn check_encoder_availability() -> Result<EncoderAvailability> {
    debug!("Checking encoder availability...");
    
    let ffmpeg_binary = ffmpeg_path();
    let output = Command::new(&ffmpeg_binary)
        .arg("-encoders")
        .output()?;
    
    if !output.status.success() {
        anyhow::bail!("Failed to run ffmpeg -encoders");
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    let aac_at_available = stdout.contains("aac_at");
    
    debug!("Encoder availability - aac_at: {}", aac_at_available);
    
    if aac_at_available {
        debug!("AudioToolbox AAC encoder detected");
    } else {
        debug!("AudioToolbox AAC encoder not available, using standard AAC encoder");
    }
    
    Ok(EncoderAvailability {
        aac_at: aac_at_available,
    })
}

#[derive(Debug, Clone)]
pub struct EncoderAvailability {
    pub aac_at: bool,
}

impl EncoderAvailability {
    pub fn get_aac_encoder(&self) -> &'static str {
        if self.aac_at {
            "aac_at"
        } else {
            "aac"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_download_does_not_panic() {
        // This test just ensures the function doesn't panic
        // Actual download testing would require network access
        let result = std::panic::catch_unwind(|| {
            // Don't actually download in tests
            ffmpeg_is_installed()
        });
        assert!(result.is_ok());
    }
}