use crate::config::SplitFile;
use crate::pipeline::Step;
use anyhow::Result;
use async_trait::async_trait;
use hound::{WavReader, WavWriter};
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
        
        // Sort files by start time and validate no overlaps
        let mut sorted_files = self.files.clone();
        for file in &mut sorted_files {
            file.start_seconds = self.parse_timestamp(&file.start)?;
            file.end_seconds = self.parse_timestamp(&file.end)?;
        }
        sorted_files.sort_by(|a, b| a.start_seconds.partial_cmp(&b.start_seconds).unwrap());
        
        // Validate no overlapping segments
        for i in 1..sorted_files.len() {
            if sorted_files[i-1].end_seconds > sorted_files[i].start_seconds {
                anyhow::bail!(
                    "Overlapping segments detected: '{}' ends at {:.6}s but '{}' starts at {:.6}s",
                    sorted_files[i-1].file, sorted_files[i-1].end_seconds,
                    sorted_files[i].file, sorted_files[i].start_seconds
                );
            }
        }
        
        info!("Processing {} splits in chronological order", sorted_files.len());
        
        // Open input WAV file
        info!("Opening WAV file: {}", input_path.display());
        let mut reader = WavReader::open(&input_path)?;
        let spec = reader.spec();
        
        info!("WAV format: {} channels, {} Hz, {} bits, {} samples", 
              spec.channels, spec.sample_rate, spec.bits_per_sample, reader.len());
        
        // Process splits sequentially
        let mut current_sample_index = 0;
        let mut samples_iter = reader.samples::<i32>();
        
        for (i, split_file) in sorted_files.iter().enumerate() {
            info!("Processing split {}/{}: {}", i + 1, sorted_files.len(), split_file.file);
            
            let start_seconds = split_file.start_seconds;
            let end_seconds = split_file.end_seconds;
            
            debug!("Time range: {:.6}s to {:.6}s", start_seconds, end_seconds);
            
            // Convert to sample indices
            let start_sample = self.seconds_to_sample_index(start_seconds, spec.sample_rate);
            let end_sample = self.seconds_to_sample_index(end_seconds, spec.sample_rate);
            
            // Adjust for multi-channel audio
            let start_frame = start_sample * spec.channels as usize;
            let end_frame = end_sample * spec.channels as usize;
            
            debug!("Sample range: {} to {} (frames {} to {})", 
                   start_sample, end_sample, start_frame, end_frame);
            
            // Skip samples until we reach the start of this segment
            while current_sample_index < start_frame {
                if samples_iter.next().is_none() {
                    anyhow::bail!("Unexpected end of file while seeking to sample {}", start_frame);
                }
                current_sample_index += 1;
            }
            
            // Create output file
            let output_file_path = output_dir_path.join(&split_file.file);
            debug!("Writing to: {}", output_file_path.display());
            
            let mut writer = WavWriter::create(&output_file_path, spec)?;
            
            // Read and write samples for this segment
            let mut samples_written = 0;
            let target_samples = end_frame - start_frame;
            
            while samples_written < target_samples {
                match samples_iter.next() {
                    Some(Ok(sample)) => {
                        writer.write_sample(sample)?;
                        samples_written += 1;
                        current_sample_index += 1;
                    }
                    Some(Err(e)) => return Err(e.into()),
                    None => {
                        warn!("End of file reached after {} samples, expected {}", 
                              samples_written, target_samples);
                        break;
                    }
                }
            }
            
            writer.finalize()?;
            
            let duration_samples = samples_written / spec.channels as usize;
            let duration_seconds = duration_samples as f64 / spec.sample_rate as f64;
            let file_size = std::fs::metadata(&output_file_path)?.len();
            
            info!("Created: {} ({:.3}s, {} bytes)", 
                  output_file_path.display(), duration_seconds, file_size);
        }
        
        info!("Split step completed successfully");
        Ok(())
    }
    
    fn name(&self) -> &str {
        "Split"
    }
}