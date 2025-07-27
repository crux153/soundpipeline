use crate::pipeline::Step;
use anyhow::Result;
use async_trait::async_trait;
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::{FfmpegEvent, LogLevel};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::time::Duration;
use tracing::{info, debug};

pub struct FfmpegStep {
    pub input: String,
    pub output: String,
    pub args: Vec<String>,
}

impl FfmpegStep {
    pub fn new(input: String, output: String, args: Vec<String>) -> Self {
        Self { input, output, args }
    }
}

#[async_trait]
impl Step for FfmpegStep {
    async fn execute(&self, working_dir: &Path) -> Result<()> {
        info!("Executing FFmpeg step: {} -> {}", self.input, self.output);
        
        let input_path = working_dir.join(&self.input);
        let output_path = working_dir.join(&self.output);
        
        debug!("Input path: {}", input_path.display());
        debug!("Output path: {}", output_path.display());
        debug!("FFmpeg args: {:?}", self.args);
        
        // Check if input file exists
        if !input_path.exists() {
            anyhow::bail!("Input file does not exist: {}", input_path.display());
        }
        
        // Create output directory if it doesn't exist
        if let Some(output_dir) = output_path.parent() {
            if !output_dir.exists() {
                std::fs::create_dir_all(output_dir)?;
                debug!("Created output directory: {}", output_dir.display());
            }
        }
        
        // Build FFmpeg command
        let mut command = FfmpegCommand::new();
        command
            .input(input_path.to_string_lossy())
            .overwrite(); // Add -y flag for file overwrite
            
        // Add custom arguments before output
        for arg in &self.args {
            command.arg(arg);
        }
        
        // Add progress reporting flag
        command.args(["-progress", "pipe:1", "-stats"]);
        
        command.output(output_path.to_string_lossy());
        
        debug!("Full FFmpeg command will be executed with args: {:?}", self.args);
        
        // Execute FFmpeg command with progress tracking
        info!("Starting FFmpeg conversion...");
        
        // Create progress bar
        let progress_bar = ProgressBar::new_spinner();
        progress_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg} [{elapsed_precise}]")
                .unwrap()
        );
        progress_bar.set_message("Processing...");
        progress_bar.enable_steady_tick(Duration::from_millis(100));
        
        let mut child = command.spawn()?;
        let iter = child.iter()?;
        
        progress_bar.set_length(100);
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}% {msg}")
                .unwrap()
                .progress_chars("#>-")
        );
        
        let mut total_duration_ms: Option<f64> = None;
        
        for event in iter {
            debug!("Received event: {:?}", event);
            match event {
                FfmpegEvent::ParsedDuration(duration) => {
                    // Duration is in seconds, convert to milliseconds
                    total_duration_ms = Some(duration.duration * 1000.0);
                    debug!("Total duration: {:.2} seconds ({:.0} ms)", duration.duration, duration.duration * 1000.0);
                }
                FfmpegEvent::Progress(progress) => {
                    debug!("Progress struct: {:?}", progress);
                    if let Some(current_ms) = parse_time_to_ms(&progress.time) {
                        debug!("Parsed current time: {} ms", current_ms);
                        if let Some(total_ms) = total_duration_ms {
                            let percentage = (current_ms / total_ms * 100.0) as u64;
                            debug!("Progress calculation: {} / {} = {}%", current_ms, total_ms, percentage);
                            progress_bar.set_position(percentage.min(100));
                            progress_bar.set_message(format!("Speed: {:.2}x", progress.speed));
                        } else {
                            let minutes = current_ms / 60000.0;
                            progress_bar.set_message(format!("{:.1}m Speed: {:.2}x", minutes, progress.speed));
                        }
                    } else {
                        debug!("Failed to parse time: {}", progress.time);
                    }
                }
                FfmpegEvent::Log(LogLevel::Info, msg) => {
                    // Parse progress from log messages like:
                    // "size=  685824KiB time=00:40:38.68 bitrate=2303.8kbits/s speed= 254x"
                    if msg.contains("time=") && msg.contains("speed=") {
                        debug!("Progress log: {}", msg);
                        if let Some(time_str) = extract_time_from_log(&msg) {
                            if let Some(current_ms) = parse_time_to_ms(&time_str) {
                                if let Some(total_ms) = total_duration_ms {
                                    let percentage = (current_ms / total_ms * 100.0) as u64;
                                    progress_bar.set_position(percentage.min(100));
                                    
                                    // Extract speed if available
                                    if let Some(speed_str) = extract_speed_from_log(&msg) {
                                        progress_bar.set_message(format!("Speed: {}", speed_str));
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        
        progress_bar.finish_with_message("Conversion completed");
        
        let result = child.wait()?;
        if !result.success() {
            anyhow::bail!("FFmpeg conversion failed with exit code: {:?}", result.code());
        }
        
        info!("FFmpeg conversion completed successfully");
        
        // Verify output file was created
        if !output_path.exists() {
            anyhow::bail!("FFmpeg failed to create output file: {}", output_path.display());
        }
        
        let file_size = std::fs::metadata(&output_path)?.len();
        info!("Output file created: {} ({} bytes)", output_path.display(), file_size);
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "FFmpeg"
    }
}

// Helper function to parse FFmpeg time format "hh:mm:ss.ff" to milliseconds
fn parse_time_to_ms(time_str: &str) -> Option<f64> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    
    let hours: f64 = parts[0].parse().ok()?;
    let minutes: f64 = parts[1].parse().ok()?;
    let seconds: f64 = parts[2].parse().ok()?;
    
    Some((hours * 3600.0 + minutes * 60.0 + seconds) * 1000.0)
}

// Extract time from log message like "size=  685824KiB time=00:40:38.68 bitrate=2303.8kbits/s speed= 254x"
fn extract_time_from_log(log_msg: &str) -> Option<String> {
    if let Some(time_start) = log_msg.find("time=") {
        let time_part = &log_msg[time_start + 5..];
        if let Some(time_end) = time_part.find(' ') {
            return Some(time_part[..time_end].to_string());
        }
    }
    None
}

// Extract speed from log message like "speed= 254x"
fn extract_speed_from_log(log_msg: &str) -> Option<String> {
    if let Some(speed_start) = log_msg.find("speed=") {
        let speed_part = &log_msg[speed_start + 6..].trim_start();
        // Find the end by looking for the closing ']' or '"' since the log message ends there
        if let Some(speed_end) = speed_part.find('"').or_else(|| speed_part.find(']')) {
            return Some(speed_part[..speed_end].trim().to_string());
        } else {
            // If no end delimiter, take the whole remaining part
            return Some(speed_part.trim().to_string());
        }
    }
    None
}