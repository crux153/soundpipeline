use anyhow::Result;
use clap::Parser;
use ffmpeg_sidecar::download::auto_download;
use soundpipeline::{config::Config, encoders, format_selector, format_parser, pipeline::Pipeline, validator::validate_pipeline};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "soundpipeline",
    about = "Extract and convert audio from video files",
    version
)]
struct Args {
    /// Path to the YAML configuration file (defaults to soundpipeline.yml)
    #[arg(value_name = "CONFIG")]
    config: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Output format (e.g., mp3, mp3:320k, flac, flac:16bit, alac:24bit)
    #[arg(long)]
    format: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let filter = if args.verbose {
        "soundpipeline=debug,info"
    } else {
        "soundpipeline=info"
    };
    
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();

    // Ensure FFmpeg is available by auto-downloading if needed
    tracing::info!("Checking FFmpeg availability...");
    auto_download()?;
    tracing::info!("FFmpeg is ready");

    // Check encoder availability
    let encoder_availability = encoders::check_encoder_availability()?;

    // Determine config file path
    let config_path = match args.config {
        Some(path) => path,
        None => {
            let default_config = PathBuf::from("soundpipeline.yml");
            if !default_config.exists() {
                anyhow::bail!("No config file specified and default 'soundpipeline.yml' not found in current directory");
            }
            default_config
        }
    };

    tracing::info!("Starting SoundPipeline with config: {}", config_path.display());

    // Load configuration
    let config = Config::from_file(&config_path)?;
    tracing::debug!("Loaded configuration: {:#?}", config);

    // Format selection - only if transcode step exists
    let selected_format = if config.has_transcode_step() {
        if let Some(format_str) = &args.format {
            tracing::info!("Using format specified via CLI: {}", format_str);
            format_parser::parse_format_string(format_str, &config.formats)?
        } else {
            tracing::info!("No format specified, launching interactive selection");
            format_selector::select_format(&config.formats)?
        }
    } else {
        tracing::info!("No transcode step found in configuration, skipping format selection");
        // Create a default format that won't be used
        soundpipeline::config::SelectedFormat {
            format: String::new(),
            bitrate: None,
            bit_depth: None,
        }
    };
    
    if config.has_transcode_step() {
        tracing::info!("Selected format: {} with bitrate: {:?}, bit depth: {:?}", 
                       selected_format.format, selected_format.bitrate, selected_format.bit_depth);
    }

    // Get working directory
    let working_dir = std::env::current_dir()?;
    
    // Validate pipeline before execution
    tracing::info!("Validating pipeline configuration...");
    let validation_result = validate_pipeline(&config, &selected_format, &working_dir)?;
    
    // Handle validation results
    if !validation_result.warnings.is_empty() {
        for warning in &validation_result.warnings {
            tracing::warn!("{}", warning);
        }
    }
    
    if !validation_result.is_valid {
        tracing::error!("Pipeline validation failed with {} error(s):", validation_result.errors.len());
        for error in &validation_result.errors {
            tracing::error!("  - {}", error);
        }
        anyhow::bail!("Pipeline validation failed. Please fix the errors above and try again.");
    }
    
    tracing::info!("Pipeline validation successful");

    // Create and execute pipeline
    let pipeline = Pipeline::from_config(&config, &selected_format, &working_dir, &encoder_availability)?;
    pipeline.execute().await?;

    tracing::info!("SoundPipeline completed successfully");
    Ok(())
}