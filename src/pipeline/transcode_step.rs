use crate::pipeline::Step;
use anyhow::Result;
use async_trait::async_trait;
use ffmpeg_sidecar::command::FfmpegCommand;
use std::path::Path;
use tracing::{info, debug, warn};

pub struct TranscodeStep {
    pub input_dir: String,
    pub output_dir: String,
    pub files: Vec<String>,
    pub format: String,
    pub bitrate: Option<String>,
}

impl TranscodeStep {
    pub fn new(
        input_dir: String,
        output_dir: String,
        files: Vec<String>,
        format: String,
        bitrate: Option<String>,
    ) -> Self {
        Self {
            input_dir,
            output_dir,
            files,
            format,
            bitrate,
        }
    }

    fn get_codec_args(&self) -> Result<Vec<String>> {
        match self.format.as_str() {
            "mp3" => {
                let mut args = vec!["-acodec".to_string(), "libmp3lame".to_string()];
                if let Some(bitrate) = &self.bitrate {
                    args.extend(["-ab".to_string(), bitrate.clone()]);
                }
                Ok(args)
            }
            "aac" => {
                let mut args = vec!["-acodec".to_string(), "aac".to_string()];
                if let Some(bitrate) = &self.bitrate {
                    args.extend(["-ab".to_string(), bitrate.clone()]);
                }
                Ok(args)
            }
            "flac" => {
                Ok(vec!["-acodec".to_string(), "flac".to_string()])
            }
            "alac" => {
                Ok(vec!["-acodec".to_string(), "alac".to_string()])
            }
            _ => {
                anyhow::bail!("Unsupported format: {}", self.format);
            }
        }
    }

    fn get_output_extension(&self) -> &str {
        match self.format.as_str() {
            "mp3" => "mp3",
            "aac" => "m4a",
            "flac" => "flac",
            "alac" => "m4a",
            _ => "unknown",
        }
    }

    fn get_output_filename(&self, input_filename: &str) -> String {
        let stem = std::path::Path::new(input_filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(input_filename);
        
        format!("{}.{}", stem, self.get_output_extension())
    }
}

#[async_trait]
impl Step for TranscodeStep {
    async fn execute(&self, working_dir: &Path) -> Result<()> {
        info!(
            "Executing Transcode step: {} -> {} (format: {}, bitrate: {:?})",
            self.input_dir, self.output_dir, self.format, self.bitrate
        );

        let input_dir_path = working_dir.join(&self.input_dir);
        let output_dir_path = working_dir.join(&self.output_dir);

        debug!("Input directory: {}", input_dir_path.display());
        debug!("Output directory: {}", output_dir_path.display());

        // Check if input directory exists
        if !input_dir_path.exists() {
            anyhow::bail!("Input directory does not exist: {}", input_dir_path.display());
        }

        // Create output directory if it doesn't exist
        if !output_dir_path.exists() {
            std::fs::create_dir_all(&output_dir_path)?;
            debug!("Created output directory: {}", output_dir_path.display());
        }

        // Get codec arguments
        let codec_args = self.get_codec_args()?;

        // Process each file
        for (i, file_pattern) in self.files.iter().enumerate() {
            info!("Processing file {}/{}: {}", i + 1, self.files.len(), file_pattern);

            // Find matching files (support wildcards)
            let matching_files = if file_pattern.contains('*') {
                // Use glob pattern matching
                let pattern_path = input_dir_path.join(file_pattern);
                let pattern_str = pattern_path.to_string_lossy();
                
                glob::glob(&pattern_str)?
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .filter(|p| p.is_file())
                    .collect()
            } else {
                // Direct file path
                let file_path = input_dir_path.join(file_pattern);
                if file_path.exists() && file_path.is_file() {
                    vec![file_path]
                } else {
                    warn!("File not found: {}", file_path.display());
                    continue;
                }
            };

            if matching_files.is_empty() {
                warn!("No files found matching pattern: {}", file_pattern);
                continue;
            }

            // Transcode each matching file
            for input_file_path in matching_files {
                let input_filename = input_file_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");

                let output_filename = self.get_output_filename(input_filename);
                let output_file_path = output_dir_path.join(&output_filename);

                debug!("Transcoding: {} -> {}", input_file_path.display(), output_file_path.display());

                // Build FFmpeg command
                let mut command = FfmpegCommand::new();
                command
                    .input(input_file_path.to_string_lossy())
                    .overwrite();

                // Add codec arguments
                for arg in &codec_args {
                    command.arg(arg);
                }

                command.output(output_file_path.to_string_lossy());

                // Execute FFmpeg command
                info!("Running FFmpeg transcode for: {}", input_filename);
                let mut child = command.spawn()?;
                let result = child.wait()?;
                
                if !result.success() {
                    anyhow::bail!("FFmpeg failed for file: {}, exit code: {:?}", input_filename, result.code());
                }

                let file_size = std::fs::metadata(&output_file_path)?.len();
                info!("Created: {} ({} bytes)", output_file_path.display(), file_size);
            }
        }

        info!("Transcode step completed successfully");
        Ok(())
    }

    fn name(&self) -> &str {
        "Transcode"
    }
}