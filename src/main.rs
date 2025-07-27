use anyhow::Result;
use clap::Parser;
use ffmpeg_sidecar::download::auto_download;
use soundpipeline::{config::Config, encoders, format_selector, format_parser, pipeline::Pipeline};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "soundpipeline",
    about = "Extract and convert audio from video files",
    version
)]
struct Args {
    /// Path to the YAML configuration file
    #[arg(value_name = "CONFIG")]
    config: PathBuf,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Output format (e.g., mp3, mp3:320k, flac)
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

    tracing::info!("Starting SoundPipeline with config: {}", args.config.display());

    // Load configuration
    let config = Config::from_file(&args.config)?;
    tracing::debug!("Loaded configuration: {:#?}", config);

    // Format selection - CLI argument or interactive
    let selected_format = if let Some(format_str) = &args.format {
        tracing::info!("Using format specified via CLI: {}", format_str);
        format_parser::parse_format_string(format_str, &config.formats)?
    } else {
        tracing::info!("No format specified, launching interactive selection");
        format_selector::select_format(&config.formats)?
    };
    tracing::info!("Selected format: {} with bitrate: {:?}", 
                   selected_format.format, selected_format.bitrate);

    // Create and execute pipeline
    let working_dir = std::env::current_dir()?;
    let pipeline = Pipeline::from_config(&config, &selected_format, &working_dir, &encoder_availability)?;
    pipeline.execute().await?;

    tracing::info!("SoundPipeline completed successfully");
    Ok(())
}