use anyhow::Result;
use clap::Parser;
use soundpipeline::{config::Config, format_selector};
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

    /// Dry run - show what would be done without executing
    #[arg(long)]
    dry_run: bool,
}

fn main() -> Result<()> {
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

    tracing::info!("Starting SoundPipeline with config: {}", args.config.display());

    if args.dry_run {
        tracing::info!("Running in dry-run mode - no files will be created");
    }

    // Load configuration
    let config = Config::from_file(&args.config)?;
    tracing::debug!("Loaded configuration: {:#?}", config);

    // Interactive format selection
    let selected_format = format_selector::select_format(&config.formats)?;
    tracing::info!("Selected format: {} with bitrate: {:?}", 
                   selected_format.format, selected_format.bitrate);

    // TODO: Implement pipeline execution with selected format
    // TODO: Implement audio extraction
    // TODO: Implement track splitting
    // TODO: Implement format conversion
    // TODO: Implement metadata tagging

    tracing::info!("SoundPipeline completed successfully");
    Ok(())
}