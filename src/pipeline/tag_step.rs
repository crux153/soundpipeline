use crate::config::TagFile;
use crate::pipeline::Step;
use anyhow::Result;
use async_trait::async_trait;
use lofty::{prelude::*, probe::Probe, tag::{Tag, TagItem, ItemValue}, picture::{Picture, PictureType, MimeType}, config::WriteOptions};
use std::path::Path;
use tracing::{info, debug, warn};

pub struct TagStep {
    pub input_dir: String,
    pub files: Vec<TagFile>,
}

impl TagStep {
    pub fn new(input_dir: String, files: Vec<TagFile>) -> Self {
        Self { input_dir, files }
    }

    fn apply_metadata_to_file(&self, file_path: &Path, tag_config: &TagFile) -> Result<()> {
        debug!("Applying metadata to: {}", file_path.display());

        // Probe the file to get its type and load it
        let mut tagged_file = Probe::open(file_path)?.read()?;

        // Get or create a tag for the file
        let tag = match tagged_file.primary_tag_mut() {
            Some(primary_tag) => primary_tag,
            None => {
                // If no primary tag exists, create a new one
                let tag_type = tagged_file.primary_tag_type();
                tagged_file.insert_tag(Tag::new(tag_type));
                tagged_file.primary_tag_mut().unwrap()
            }
        };

        // Apply metadata fields
        if let Some(title) = &tag_config.title {
            tag.set_title(title.clone());
            debug!("Set title: {}", title);
        }

        if let Some(artist) = &tag_config.artist {
            tag.set_artist(artist.clone());
            debug!("Set artist: {}", artist);
        }

        if let Some(album) = &tag_config.album {
            tag.set_album(album.clone());
            debug!("Set album: {}", album);
        }

        if let Some(album_artist) = &tag_config.album_artist {
            tag.insert(TagItem::new(ItemKey::AlbumArtist, ItemValue::Text(album_artist.clone())));
            debug!("Set album artist: {}", album_artist);
        }

        if let Some(track) = tag_config.track {
            tag.set_track(track);
            debug!("Set track: {}", track);
        }

        if let Some(track_total) = tag_config.track_total {
            tag.set_track_total(track_total);
            debug!("Set track total: {}", track_total);
        }

        if let Some(disk) = tag_config.disk {
            tag.set_disk(disk);
            debug!("Set disk: {}", disk);
        }

        if let Some(disk_total) = tag_config.disk_total {
            tag.set_disk_total(disk_total);
            debug!("Set disk total: {}", disk_total);
        }

        if let Some(genre) = &tag_config.genre {
            tag.set_genre(genre.clone());
            debug!("Set genre: {}", genre);
        }

        if let Some(year) = tag_config.year {
            tag.set_year(year);
            debug!("Set year: {}", year);
        }

        if let Some(comment) = &tag_config.comment {
            tag.set_comment(comment.clone());
            debug!("Set comment: {}", comment);
        }

        // Handle album art if specified
        if let Some(album_art_path) = &tag_config.album_art {
            let art_path = Path::new(album_art_path);
            if art_path.exists() {
                match std::fs::read(art_path) {
                    Ok(art_data) => {
                        // Determine MIME type based on file extension
                        let mime_type = match art_path.extension().and_then(|ext| ext.to_str()) {
                            Some("jpg") | Some("jpeg") => MimeType::Jpeg,
                            Some("png") => MimeType::Png,
                            Some("gif") => MimeType::Gif,
                            Some("bmp") => MimeType::Bmp,
                            _ => {
                                warn!("Unknown image format for album art: {}", album_art_path);
                                MimeType::Jpeg // Default to JPEG
                            }
                        };

                        let picture = Picture::new_unchecked(
                            PictureType::CoverFront,
                            Some(mime_type),
                            None,
                            art_data,
                        );

                        tag.set_picture(0, picture);
                        debug!("Set album art from: {}", album_art_path);
                    }
                    Err(e) => {
                        warn!("Failed to read album art file {}: {}", album_art_path, e);
                    }
                }
            } else {
                warn!("Album art file not found: {}", album_art_path);
            }
        }

        // Save the changes
        tagged_file.save_to_path(file_path, WriteOptions::default())?;
        info!("Successfully tagged: {}", file_path.display());

        Ok(())
    }
}

#[async_trait]
impl Step for TagStep {
    async fn execute(&self, working_dir: &Path) -> Result<()> {
        info!("Executing Tag step with {} files", self.files.len());

        let input_dir_path = working_dir.join(&self.input_dir);

        debug!("Input directory: {}", input_dir_path.display());

        // Check if input directory exists
        if !input_dir_path.exists() {
            anyhow::bail!("Input directory does not exist: {}", input_dir_path.display());
        }

        // Process each file configuration
        for (i, tag_config) in self.files.iter().enumerate() {
            info!("Processing file {}/{}: {}", i + 1, self.files.len(), tag_config.file);

            // Find matching files (support wildcards)
            let matching_files = if tag_config.file.contains('*') {
                // Use glob pattern matching
                let pattern_path = input_dir_path.join(&tag_config.file);
                let pattern_str = pattern_path.to_string_lossy();
                
                glob::glob(&pattern_str)?
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .filter(|p| p.is_file())
                    .collect()
            } else {
                // Direct file path
                let file_path = input_dir_path.join(&tag_config.file);
                if file_path.exists() && file_path.is_file() {
                    vec![file_path]
                } else {
                    warn!("File not found: {}", file_path.display());
                    continue;
                }
            };

            if matching_files.is_empty() {
                warn!("No files found matching pattern: {}", tag_config.file);
                continue;
            }

            // Apply metadata to each matching file
            for file_path in matching_files {
                match self.apply_metadata_to_file(&file_path, tag_config) {
                    Ok(()) => {
                        debug!("Successfully tagged: {}", file_path.display());
                    }
                    Err(e) => {
                        warn!("Failed to tag file {}: {}", file_path.display(), e);
                        // Continue with other files instead of failing the entire step
                    }
                }
            }
        }

        info!("Tag step completed successfully");
        Ok(())
    }

    fn name(&self) -> &str {
        "Tag"
    }
}