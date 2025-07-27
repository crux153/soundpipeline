# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SoundPipeline is a Rust command-line tool that processes audio/video files through a configurable pipeline of steps. It supports extracting audio from video files, splitting audio based on timestamps, converting between formats, and applying metadata tags. The tool uses FFmpeg (via ffmpeg-sidecar) and is configured through YAML files with a step-based architecture similar to CI/CD pipelines.

## Key Commands

### Build and Run
```bash
# Build the project
cargo build

# Build release version
cargo build --release

# Run with a config file and output formats
cargo run -- config.yml --format mp3:320k --format flac

# Run tests
cargo test

# Run with verbose logging
cargo run -- -v config.yml --format mp3
```

### Development Commands
```bash
# Format code
cargo fmt

# Lint code
cargo clippy -- -D warnings

# Check for compilation errors
cargo check

# Run a specific test
cargo test test_name

# Generate documentation
cargo doc --open
```

## Architecture Overview

### Core Components

1. **Pipeline System** (`src/pipeline/`)
   - `pipeline.rs`: Main pipeline orchestrator
   - `step.rs`: Step trait and implementations
   - Executes steps sequentially with proper error handling

2. **Step Implementations** (`src/steps/`)
   - `ffmpeg.rs`: Direct FFmpeg command execution via ffmpeg-sidecar
   - `split.rs`: Audio file splitting based on timestamps
   - `transcode.rs`: Format conversion with configurable codecs
   - `tag.rs`: Metadata tagging for various formats

3. **Configuration** (`src/config.rs`)
   - YAML parsing with serde_yaml
   - Step-based configuration structure
   - Validates timestamps in h:mm:ss.SSS or h:mm:ss.SSSSSS format

4. **Metadata Handling** (`src/metadata/`)
   - Format-specific implementations:
     - MP3: id3 crate
     - FLAC: metaflac crate
     - M4A/AAC/ALAC: mp4ameta crate

5. **CLI Interface** (`src/main.rs`)
   - clap for argument parsing
   - Accepts format specifications via --format flags
   - Progress tracking with indicatif

### Key Design Decisions

1. **FFmpeg Integration**: Uses ffmpeg-sidecar for FFmpeg binary management and execution
2. **Pipeline Architecture**: Step-based design allows flexible audio processing workflows
3. **Error Handling**: anyhow for application errors, thiserror for library errors
4. **Async Operations**: tokio for concurrent file processing
5. **Format Selection**: Output formats specified at runtime, not in config files
6. **Timestamp Format**: Uses h:mm:ss.SSS or h:mm:ss.SSSSSS format

### Typical Workflow

1. Parse YAML configuration with step definitions
2. Parse command-line format specifications
3. Execute pipeline steps sequentially:
   - **ffmpeg**: Extract audio or process video/audio files
   - **split**: Divide audio based on timestamps
   - **transcode**: Convert to specified formats (MP3, FLAC, M4A, etc.)
   - **tag**: Apply metadata to output files
4. Handle errors and clean up temporary files

## Important Implementation Notes

- ffmpeg-sidecar automatically downloads and manages FFmpeg binaries
- Use WAV (PCM) as intermediate format to preserve quality
- Timestamp format must be h:mm:ss.SSS or h:mm:ss.SSSSSS
- File patterns in tag step support wildcards (e.g., "track_01.*")
- Output formats are specified via CLI flags, not config files
- Ensure proper cleanup of temporary files even on errors
- Use structured logging with tracing for debugging
- Each step operates on files relative to a working directory