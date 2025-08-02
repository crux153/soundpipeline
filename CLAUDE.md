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

## Project Structure

### Module Organization

```
src/
├── lib.rs              # Library root - exports all public modules
├── main.rs             # CLI entry point
├── config.rs           # Configuration structures and parsing
├── settings.rs         # Application settings with CLI/env/YAML support
├── encoders.rs         # Encoder availability checking
├── format_parser.rs    # Format string parsing (e.g., "mp3:320k")
├── format_selector.rs  # Interactive format selection
├── validator.rs        # Pipeline validation after duration check
├── duration_checker.rs # Duration validation using FFprobe
├── file_suggester.rs   # Alternative file suggestion for duration mismatches
├── pipeline.rs         # Pipeline module definition (NOT pipeline/mod.rs)
└── pipeline/           # Pipeline implementation modules
    ├── executor.rs     # Main Pipeline struct and execution logic
    ├── step.rs         # Step trait definition
    ├── ffmpeg_step.rs  # FFmpeg step implementation
    ├── split_step.rs   # Split step implementation
    ├── transcode_step.rs # Transcode step implementation
    ├── tag_step.rs     # Tag step implementation
    └── cleanup_step.rs # Cleanup step implementation
```

**Important**: The pipeline module is defined in `src/pipeline.rs`, not `src/pipeline/mod.rs`. This is a single-file module declaration pattern where `pipeline.rs` declares the submodules.

### Architecture Overview

#### Core Components

1. **Pipeline System** (`src/pipeline/`)
   - `executor.rs`: Contains the main `Pipeline` struct and execution logic
   - `step.rs`: Defines the `Step` trait that all pipeline steps implement
   - Executes steps sequentially with proper error handling

2. **Step Implementations** (`src/pipeline/`)
   - `ffmpeg_step.rs`: Direct FFmpeg command execution via ffmpeg-sidecar
   - `split_step.rs`: Audio file splitting based on timestamps
   - `transcode_step.rs`: Format conversion with configurable codecs
   - `tag_step.rs`: Metadata tagging for various formats using lofty-rs
   - `cleanup_step.rs`: Remove temporary files and directories

3. **Configuration** (`src/config.rs`)
   - YAML parsing with serde_yaml
   - Step-based configuration structure
   - Optional settings section for application configuration
   - Validates timestamps in h:mm:ss.SSS or h:mm:ss.SSSSSS format

4. **Settings Management** (`src/settings.rs`)
   - Configurable application settings with multiple override methods
   - Duration tolerance for ffmpeg step validation (default: 3.0 seconds)
   - Priority: CLI flags > environment variables > YAML config > defaults
   - Automatic parsing from `--duration-tolerance` CLI flag and `DURATION_TOLERANCE` env var

5. **Pipeline Validation** (`src/validator.rs`)
   - Pre-execution validation of pipeline configuration
   - Simulates file system changes through a virtual file tree
   - Validates file dependencies between steps
   - Supports glob pattern matching (e.g., `*.mp3`, `track_*.*`)
   - Checks timestamp formats and required files

6. **Metadata Handling** (`src/pipeline/tag_step.rs`)
   - Unified metadata handling using lofty-rs crate
   - Supports all major audio formats: MP3, FLAC, M4A, AAC, ALAC
   - Handles album artwork with automatic MIME type detection
   - Graceful error handling for individual file failures

7. **Duration Checker** (`src/duration_checker.rs`)
   - Validates input file durations against expected values using FFprobe
   - Optional `input_duration` field in ffmpeg steps (format: h:mm:ss)
   - Configurable tolerance for duration matching via settings system
   - Runs before pipeline validation to enable file replacements

8. **File Suggester** (`src/file_suggester.rs`)
   - Scans working directory for alternative MKV files when duration mismatches occur
   - Finds best matching files by duration within tolerance
   - Prompts user for confirmation before replacing files in configuration
   - Enables automatic file correction for duration-related issues

9. **CLI Interface** (`src/main.rs`)
   - clap for argument parsing
   - Accepts format specifications via --format flags
   - Supports settings configuration via CLI flags and environment variables
   - Progress tracking with indicatif
   - Runs duration check and file suggestion before pipeline validation

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
3. **Check duration and suggest alternatives** (for ffmpeg steps with input_duration specified):
   - Use FFprobe to get actual file duration
   - Compare with expected duration from configuration
   - If mismatch exceeds tolerance (3 seconds):
     - Scan working directory for alternative MKV files
     - Find best matching file by duration within tolerance
     - Prompt user for confirmation to replace file in configuration
     - Update configuration with confirmed replacement
4. **Validate pipeline configuration** (after potential file replacements):
   - Check if all input files exist or will be created by previous steps
   - Validate timestamp formats
   - Simulate file/directory creation and deletion
   - Verify glob patterns will match expected files
5. Execute pipeline steps sequentially:
   - **ffmpeg**: Extract audio or process video/audio files
   - **split**: Divide audio based on timestamps
   - **transcode**: Convert to specified formats (MP3, FLAC, M4A, etc.)
   - **tag**: Apply metadata to output files
   - **cleanup**: Remove temporary files and directories
5. Handle errors and clean up temporary files

## Important Implementation Notes

- ffmpeg-sidecar automatically downloads and manages FFmpeg binaries
- Use WAV (PCM) as intermediate format to preserve quality
- Timestamp format must be h:mm:ss.SSS or h:mm:ss.SSSSSS
- File patterns in tag step support wildcards (e.g., "track_01.*")
- Output formats are specified via CLI flags, not config files
- Ensure proper cleanup of temporary files even on errors
- Use structured logging with tracing for debugging
- Each step operates on files relative to a working directory
- Metadata tagging uses lofty-rs for unified handling across all audio formats
- Pipeline validation runs before execution to catch configuration errors early
- Validation simulates the entire pipeline execution to verify file dependencies
- The validator uses a tree-based file system representation to track files and directories
- Path normalization handles `./` prefixes consistently throughout the codebase
- Duration checking validates media file lengths against expected values using FFprobe
- FFmpeg steps can include optional `input_duration: "h:mm:ss"` for duration validation
- File suggestion automatically finds alternative MKV files when duration mismatches occur
- Duration check and file suggestion run before pipeline validation to enable configuration corrections
