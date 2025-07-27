# SoundPipeline

Extract and convert audio from video files with automated splitting and tagging.

## Overview

SoundPipeline is a command-line tool that automates the process of:
- Extracting audio tracks from video files (MKV, MP4, AVI, etc.)
- Splitting audio into individual tracks based on timestamps
- Converting to multiple audio formats (MP3, AAC, FLAC, ALAC)
- Applying ID3/metadata tags automatically

## Features

- **Multiple Format Support**: Convert to MP3, AAC, FLAC, and ALAC formats
- **Automated Splitting**: Split audio based on timestamp definitions
- **Metadata Tagging**: Automatically apply ID3 tags and metadata
- **YAML Configuration**: Define all processing parameters in a simple YAML file
- **FFmpeg Integration**: Leverages FFmpeg for reliable audio processing
- **Progress Tracking**: Visual progress indicators for long operations

## Installation

### Prerequisites

- Rust 1.70 or later
- FFmpeg installed and available in PATH

### Building from Source

```bash
git clone https://github.com/yourusername/soundpipeline
cd soundpipeline
cargo build --release
```

The binary will be available at `target/release/soundpipeline`.

## Usage

Create a YAML configuration file defining your processing steps:

```yaml
# example.yml
formats:
  available:
    - format: mp3
      bitrates: ["320k", "256k", "192k", "128k"]
      default_bitrate: "320k"
    - format: aac
      bitrates: ["256k", "192k", "128k"]
      default_bitrate: "256k"
    - format: flac
    - format: alac
  default: mp3

steps:
  # Extract audio from video file
  - type: ffmpeg
    input: "/path/to/video.mkv"
    output: "extracted_audio.wav"
    args: ["-vn", "-acodec", "pcm_s16le"]
  
  # Split into individual tracks
  - type: split
    input: "extracted_audio.wav"
    output_dir: "./splits"
    files:
      - file: "track_01.wav"
        start: "0:00:00.000"
        end: "0:05:23.000"
      - file: "track_02.wav"
        start: "0:05:23.000"
        end: "0:09:45.000"
  
  # Convert to desired formats (format specified at runtime)
  - type: transcode
    input_dir: "./splits"
    output_dir: "./transcoded"
    files:
      - "track_01.wav"
      - "track_02.wav"
  
  # Add metadata tags
  - type: tag
    input_dir: "./transcoded"
    files:
      - file: "track_01.*"
        title: "Track 1"
        artist: "Artist Name"
        album: "Album Name"
        track: 1
        album_art: "cover.jpg"
      - file: "track_02.*"
        title: "Track 2"
        artist: "Artist Name"
        album: "Album Name"
        track: 2
        album_art: "cover.jpg"
```

Run the conversion:

```bash
soundpipeline config.yml
```

The tool will interactively ask you to select output formats from those defined in your configuration file.

## Configuration Format

### Step Types

#### ffmpeg
Execute FFmpeg commands directly:
- `input`: Input file path
- `output`: Output file path
- `args`: Array of FFmpeg arguments

#### split
Split audio files based on timestamps:
- `input`: Source audio file
- `output_dir`: Directory for output files
- `files`: Array of output definitions
  - `file`: Output filename
  - `start`: Start timestamp (h:mm:ss.SSS format)
  - `end`: End timestamp

#### transcode
Convert audio files to different formats:
- `input_dir`: Directory containing input files
- `output_dir`: Directory for output files
- `files`: Array of input filenames
- Output format is specified via command-line flags

#### tag
Apply metadata tags to audio files:
- `input_dir`: Directory containing files to tag
- `files`: Array of tag definitions
  - `file`: File pattern (supports wildcards)
  - `title`: Track title
  - `artist`: Artist name
  - `album`: Album name
  - `track`: Track number
  - `album_art`: (Optional) Album artwork image file
  - `genre`: (Optional) Genre
  - `year`: (Optional) Year
  - `comment`: (Optional) Comment

## Examples

### Basic Usage

```yaml
formats:
  available:
    - format: mp3
      bitrates: ["320k", "256k", "192k", "128k"]
      default_bitrate: "320k"
    - format: flac
  default: mp3

steps:
  - type: ffmpeg
    input: "video.mp4"
    output: "audio.wav"
    args: ["-vn", "-acodec", "pcm_s16le"]
  
  - type: transcode
    input_dir: "."
    output_dir: "./output"
    files: ["audio.wav"]
  
  - type: tag
    input_dir: "./output"
    files:
      - file: "audio.*"
        title: "Full Audio"
        artist: "Artist Name"
        album: "Album Name"
        album_art: "artwork.jpg"
```

### Multiple Tracks from Video

```yaml
formats:
  available:
    - format: mp3
      bitrates: ["320k", "256k", "192k", "128k"]
      default_bitrate: "320k"
    - format: aac
      bitrates: ["256k", "192k", "128k"]
      default_bitrate: "256k"
    - format: flac
    - format: alac
  default: mp3

steps:
  # Extract second audio stream
  - type: ffmpeg
    input: "recording.mkv"
    output: "full_audio.wav"
    args: ["-map", "0:a:1", "-vn", "-acodec", "pcm_s16le"]
  
  # Split based on timestamps
  - type: split
    input: "full_audio.wav"
    output_dir: "./splits"
    files:
      - file: "part_01.wav"
        start: "0:00:00.000"
        end: "0:02:15.000"
      - file: "part_02.wav"
        start: "0:02:15.000"
        end: "0:06:45.000"
  
  # Convert to formats (specified at runtime)
  - type: transcode
    input_dir: "./splits"
    output_dir: "./final"
    files:
      - "part_01.wav"
      - "part_02.wav"
  
  # Apply metadata
  - type: tag
    input_dir: "./final"
    files:
      - file: "part_01.*"
        title: "Part 1"
        artist: "Artist Name"
        album: "Album Title"
        track: 1
        genre: "Genre"
        year: 2024
        album_art: "cover.jpg"
      - file: "part_02.*"
        title: "Part 2"
        artist: "Artist Name"
        album: "Album Title"
        track: 2
        genre: "Genre"
        year: 2024
        album_art: "cover.jpg"
```

### Command-line Usage

```bash
# Run with configuration file
soundpipeline pipeline.yml

# Run with verbose output
soundpipeline -v pipeline.yml

# Dry run to see what would be done
soundpipeline --dry-run pipeline.yml
```

When you run the tool, it will:
1. Load your configuration file
2. Show available formats from your config
3. Let you select one or more output formats
4. If a format has multiple bitrates, ask you to choose one
5. Process your audio files accordingly

## License

MIT License - see LICENSE file for details

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

- Built with Rust
- Audio processing powered by FFmpeg
- Metadata handling via id3, metaflac, and mp4ameta crates