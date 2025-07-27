use crate::pipeline::Step;
use anyhow::Result;
use async_trait::async_trait;
use ffmpeg_sidecar::command::FfmpegCommand;
use std::path::Path;
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
            .input(input_path.to_string_lossy());
            
        // Add custom arguments before output
        for arg in &self.args {
            command.arg(arg);
        }
        
        command.output(output_path.to_string_lossy());
        
        debug!("Full FFmpeg command will be executed with args: {:?}", self.args);
        
        // Execute FFmpeg command and wait for completion
        info!("Starting FFmpeg conversion...");
        let mut child = command.spawn()?;
        let result = child.wait()?;
        
        if result.success() {
            info!("FFmpeg conversion completed successfully");
        } else {
            anyhow::bail!("FFmpeg conversion failed with exit code: {:?}", result.code());
        }
        
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