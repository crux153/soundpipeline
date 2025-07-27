use crate::config::{Config, StepConfig, SelectedFormat};
use crate::encoders::EncoderAvailability;
use crate::pipeline::{Step, ffmpeg_step::FfmpegStep, split_step::SplitStep, transcode_step::TranscodeStep};
use anyhow::Result;
use std::path::Path;
use tracing::{info, debug};

pub struct Pipeline {
    steps: Vec<Box<dyn Step>>,
    working_dir: std::path::PathBuf,
}

impl Pipeline {
    pub fn from_config(config: &Config, selected_format: &SelectedFormat, working_dir: impl AsRef<Path>, encoder_availability: &EncoderAvailability) -> Result<Self> {
        let working_dir = working_dir.as_ref().to_path_buf();
        let mut steps: Vec<Box<dyn Step>> = Vec::new();
        
        for step_config in &config.steps {
            match step_config {
                StepConfig::Ffmpeg { input, output, args } => {
                    let step = FfmpegStep::new(
                        input.clone(),
                        output.clone(),
                        args.clone(),
                    );
                    steps.push(Box::new(step));
                }
                StepConfig::Split { input, output_dir, files } => {
                    let step = SplitStep::new(
                        input.clone(),
                        output_dir.clone(),
                        files.clone(),
                    );
                    steps.push(Box::new(step));
                }
                StepConfig::Transcode { input_dir, output_dir, files } => {
                    let step = TranscodeStep::new(
                        input_dir.clone(),
                        output_dir.clone(),
                        files.clone(),
                        selected_format.format.clone(),
                        selected_format.bitrate.clone(),
                        encoder_availability.clone(),
                    );
                    steps.push(Box::new(step));
                }
                StepConfig::Tag { .. } => {
                    // TODO: Implement tag step
                    info!("Tag step not yet implemented, skipping");
                }
            }
        }
        
        Ok(Pipeline {
            steps,
            working_dir,
        })
    }
    
    pub async fn execute(&self) -> Result<()> {
        info!("Starting pipeline execution with {} steps", self.steps.len());
        debug!("Working directory: {}", self.working_dir.display());
        
        // Ensure working directory exists
        if !self.working_dir.exists() {
            std::fs::create_dir_all(&self.working_dir)?;
            info!("Created working directory: {}", self.working_dir.display());
        }
        
        for (i, step) in self.steps.iter().enumerate() {
            info!("Executing step {}/{}: {}", i + 1, self.steps.len(), step.name());
            
            match step.execute(&self.working_dir).await {
                Ok(()) => {
                    info!("Step {}/{} completed successfully", i + 1, self.steps.len());
                }
                Err(e) => {
                    anyhow::bail!("Step {}/{} failed: {}", i + 1, self.steps.len(), e);
                }
            }
        }
        
        info!("Pipeline execution completed successfully");
        Ok(())
    }
}