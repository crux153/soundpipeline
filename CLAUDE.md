# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SoundPipeline is a Rust command-line tool that extracts audio from video files, splits them into individual tracks, converts to various audio formats (MP3, AAC, FLAC, ALAC), and applies metadata tags. The tool uses FFmpeg for audio processing and is configured via YAML files.

## Key Commands

### Build and Run
```bash
# Build the project
cargo build

# Build release version
cargo build --release

# Run with a config file
cargo run -- config.yml

# Run tests
cargo test

# Run with verbose logging
cargo run -- -v config.yml
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

1. **Configuration System** (`src/config.rs`)
   - YAML parsing using serde_yaml
   - Validates track timestamps and format specifications
   - Defines the structure for input configuration files

2. **Audio Processing** (`src/audio/`)
   - `extractor.rs`: Handles audio extraction from video files using FFmpeg
   - `splitter.rs`: Splits audio files based on timestamps
   - `converter.rs`: Converts between audio formats
   - All operations use FFmpeg via command execution

3. **Metadata Handling** (`src/metadata/`)
   - `tagger.rs`: Abstract interface for metadata tagging
   - Format-specific implementations:
     - MP3: Uses id3 crate
     - FLAC: Uses metaflac crate
     - AAC/ALAC: Uses mp4ameta crate

4. **CLI Interface** (`src/main.rs`)
   - Uses clap for argument parsing
   - Orchestrates the entire pipeline
   - Provides progress feedback via indicatif

### Key Design Decisions

1. **FFmpeg Integration**: Uses command-line FFmpeg rather than bindings for better compatibility
2. **Error Handling**: Uses anyhow for application errors, thiserror for library errors
3. **Async Operations**: Uses tokio for concurrent processing of multiple tracks/formats
4. **Progress Tracking**: indicatif for visual progress during long operations

### Typical Workflow

1. Parse YAML configuration file
2. Validate all timestamps and paths
3. Extract audio stream from video file to temporary WAV
4. Split WAV into individual tracks based on timestamps
5. Convert each track to requested formats in parallel
6. Apply metadata tags to each output file
7. Clean up temporary files

## Important Implementation Notes

- Always validate that FFmpeg is available in PATH before operations
- Use temporary directories for intermediate files (WAV extraction)
- Preserve original audio quality during extraction (use PCM/WAV as intermediate)
- Handle timestamp formats flexibly (HH:MM:SS or HH:MM:SS.mmm)
- Ensure proper cleanup of temporary files even on errors
- Use structured logging with tracing for debugging