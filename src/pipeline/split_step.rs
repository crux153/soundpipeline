use crate::config::SplitFile;
use crate::pipeline::Step;
use anyhow::Result;
use async_trait::async_trait;
use hound::{WavReader, WavWriter, SampleFormat};
use std::path::Path;
use tracing::{info, debug, warn};

pub struct SplitStep {
    pub input: String,
    pub output_dir: String,
    pub files: Vec<SplitFile>,
}

impl SplitStep {
    pub fn new(input: String, output_dir: String, files: Vec<SplitFile>) -> Self {
        Self {
            input,
            output_dir,
            files,
        }
    }

    fn parse_timestamp(&self, timestamp: &str) -> Result<f64> {
        let parts: Vec<&str> = timestamp.split(':').collect();
        
        match parts.len() {
            2 => {
                // MM:SS.sss or MM:SS.ssssss format
                let minutes: u32 = parts[0].parse()?;
                let seconds: f64 = parts[1].parse()?;
                Ok(minutes as f64 * 60.0 + seconds)
            }
            3 => {
                // H:MM:SS.sss or H:MM:SS.ssssss format
                let hours: u32 = parts[0].parse()?;
                let minutes: u32 = parts[1].parse()?;
                let seconds: f64 = parts[2].parse()?;
                Ok(hours as f64 * 3600.0 + minutes as f64 * 60.0 + seconds)
            }
            _ => {
                anyhow::bail!("Invalid timestamp format: {}. Expected h:mm:ss.SSS or h:mm:ss.SSSSSS", timestamp);
            }
        }
    }

    fn seconds_to_sample_index(&self, seconds: f64, sample_rate: u32) -> usize {
        (seconds * sample_rate as f64).round() as usize
    }
}

#[async_trait]
impl Step for SplitStep {
    async fn execute(&self, working_dir: &Path) -> Result<()> {
        info!("Executing Split step: {} -> {}", self.input, self.output_dir);
        
        let input_path = working_dir.join(&self.input);
        let output_dir_path = working_dir.join(&self.output_dir);
        
        debug!("Input path: {}", input_path.display());
        debug!("Output directory: {}", output_dir_path.display());
        
        // Check if input file exists
        if !input_path.exists() {
            anyhow::bail!("Input file does not exist: {}", input_path.display());
        }
        
        // Create output directory if it doesn't exist
        if !output_dir_path.exists() {
            std::fs::create_dir_all(&output_dir_path)?;
            debug!("Created output directory: {}", output_dir_path.display());
        }
        
        // Open input WAV file
        info!("Opening WAV file: {}", input_path.display());
        let mut reader = WavReader::open(&input_path)?;
        let spec = reader.spec();
        
        info!("WAV format: {} channels, {} Hz, {} bits, {} samples", 
              spec.channels, spec.sample_rate, spec.bits_per_sample, reader.len());
        
        // Read all samples into memory
        info!("Reading WAV samples into memory...");
        let samples: Vec<i32> = match spec.sample_format {
            SampleFormat::Float => {
                reader.samples::<f32>()
                    .collect::<Result<Vec<f32>, _>>()?
                    .into_iter()
                    .map(|s| (s * i32::MAX as f32) as i32)
                    .collect()
            }
            SampleFormat::Int => {
                reader.samples::<i32>()
                    .collect::<Result<Vec<i32>, _>>()?
            }
        };
        
        info!("Loaded {} samples", samples.len());
        
        // Process each split file
        for (i, split_file) in self.files.iter().enumerate() {
            info!("Processing split {}/{}: {}", i + 1, self.files.len(), split_file.file);
            
            // Parse timestamps
            let start_seconds = self.parse_timestamp(&split_file.start)?;
            let end_seconds = self.parse_timestamp(&split_file.end)?;
            
            debug!("Time range: {:.6}s to {:.6}s", start_seconds, end_seconds);
            
            // Convert to sample indices
            let start_sample = self.seconds_to_sample_index(start_seconds, spec.sample_rate);
            let end_sample = self.seconds_to_sample_index(end_seconds, spec.sample_rate);
            
            // Adjust for multi-channel audio
            let start_frame = start_sample * spec.channels as usize;
            let end_frame = end_sample * spec.channels as usize;
            
            debug!("Sample range: {} to {} (frames {} to {})", 
                   start_sample, end_sample, start_frame, end_frame);
            
            // Validate range
            if start_frame >= samples.len() {
                warn!("Start sample {} exceeds file length {}, skipping", start_frame, samples.len());
                continue;
            }
            
            let actual_end_frame = std::cmp::min(end_frame, samples.len());
            if end_frame > samples.len() {
                warn!("End sample {} exceeds file length {}, truncating to {}", 
                      end_frame, samples.len(), actual_end_frame);
            }
            
            // Extract samples for this segment
            let segment_samples = &samples[start_frame..actual_end_frame];
            let duration_samples = (actual_end_frame - start_frame) / spec.channels as usize;
            let duration_seconds = duration_samples as f64 / spec.sample_rate as f64;
            
            info!("Extracting {:.3}s ({} samples)", duration_seconds, duration_samples);
            
            // Create output file
            let output_file_path = output_dir_path.join(&split_file.file);
            debug!("Writing to: {}", output_file_path.display());
            
            let mut writer = WavWriter::create(&output_file_path, spec)?;
            
            // Write samples
            for &sample in segment_samples {
                writer.write_sample(sample)?;
            }
            
            writer.finalize()?;
            
            let file_size = std::fs::metadata(&output_file_path)?.len();
            info!("Created: {} ({} bytes)", output_file_path.display(), file_size);
        }
        
        info!("Split step completed successfully");
        Ok(())
    }
    
    fn name(&self) -> &str {
        "Split"
    }
}