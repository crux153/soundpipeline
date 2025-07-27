use anyhow::Result;
use ffmpeg_sidecar::paths::ffmpeg_path;
use std::process::Command;
use tracing::debug;

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
    let alac_at_available = stdout.contains("alac_at");
    
    debug!("Encoder availability - aac_at: {}, alac_at: {}", aac_at_available, alac_at_available);
    
    if aac_at_available || alac_at_available {
        debug!("AudioToolbox encoders detected");
    } else {
        debug!("AudioToolbox encoders not available, using standard encoders");
    }
    
    Ok(EncoderAvailability {
        aac_at: aac_at_available,
        alac_at: alac_at_available,
    })
}

#[derive(Debug, Clone)]
pub struct EncoderAvailability {
    pub aac_at: bool,
    pub alac_at: bool,
}

impl EncoderAvailability {
    pub fn get_aac_encoder(&self) -> &'static str {
        if self.aac_at {
            "aac_at"
        } else {
            "aac"
        }
    }
    
    pub fn get_alac_encoder(&self) -> &'static str {
        if self.alac_at {
            "alac_at"
        } else {
            "alac"
        }
    }
}