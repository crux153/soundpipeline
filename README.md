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

Create a YAML configuration file defining your extraction parameters:

```yaml
# example.yml
source: "/path/to/video.mkv"
output_dir: "/path/to/output"
audio_stream: 0  # Audio stream index in video file

tracks:
  - title: "Track 1"
    artist: "Artist Name"
    album: "Album Name"
    start: "00:00:00"
    end: "00:05:23"
    track_number: 1
  
  - title: "Track 2"
    artist: "Artist Name"
    album: "Album Name"
    start: "00:05:23"
    end: "00:09:45"
    track_number: 2

formats:
  - mp3:
      bitrate: "320k"
  - flac:
      compression_level: 8
  - aac:
      bitrate: "256k"
  - alac
```

Run the conversion:

```bash
soundpipeline config.yml
```

## Configuration Format

### Root Fields

- `source`: Path to the source video file
- `output_dir`: Directory where output files will be saved
- `audio_stream`: Audio stream index to extract (default: 0)
- `tracks`: Array of track definitions
- `formats`: Array of output format configurations

### Track Definition

Each track can include:
- `title`: Track title
- `artist`: Artist name
- `album`: Album name
- `start`: Start timestamp (HH:MM:SS or HH:MM:SS.mmm)
- `end`: End timestamp
- `track_number`: Track number
- `genre`: (Optional) Genre
- `year`: (Optional) Release year
- `comment`: (Optional) Additional comments

### Format Configuration

Supported formats and their options:
- `mp3`: `bitrate` (e.g., "320k", "256k", "192k")
- `aac`: `bitrate` (e.g., "256k", "192k")
- `flac`: `compression_level` (0-12, default: 5)
- `alac`: No additional options

## Examples

### Basic Usage

```yaml
source: "video.mp4"
output_dir: "./output"
tracks:
  - title: "Full Audio"
    artist: "Artist Name"
    album: "Album Name"
    start: "00:00:00"
    end: "01:30:00"
formats:
  - mp3:
      bitrate: "320k"
```

### Multiple Tracks with Full Metadata

```yaml
source: "recording.mkv"
output_dir: "./split_tracks"
audio_stream: 1  # Use second audio stream

tracks:
  - title: "Part 1"
    artist: "Artist Name"
    album: "Album Title"
    start: "00:00:00"
    end: "00:02:15"
    track_number: 1
    genre: "Genre"
    year: 2024
    
  - title: "Part 2"
    artist: "Artist Name"
    album: "Album Title"
    start: "00:02:15"
    end: "00:06:45"
    track_number: 2
    genre: "Genre"
    year: 2024

formats:
  - flac:
      compression_level: 8
  - mp3:
      bitrate: "V0"  # Variable bitrate
```

## License

MIT License - see LICENSE file for details

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

- Built with Rust
- Audio processing powered by FFmpeg
- Metadata handling via id3, metaflac, and mp4ameta crates