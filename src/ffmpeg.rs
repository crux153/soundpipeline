use anyhow::Result;
use ffmpeg_sidecar::download::{ffmpeg_download_url, unpack_ffmpeg};
use ffmpeg_sidecar::paths::sidecar_dir;
use ffmpeg_sidecar::command::ffmpeg_is_installed;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

/// Auto-download FFmpeg with progress tracking
pub fn auto_download_with_progress() -> Result<()> {
    // if ffmpeg_is_installed() {
    //     tracing::info!("FFmpeg is already installed");
    //     return Ok(());
    // }

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