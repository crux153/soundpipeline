use super::Step;
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use tracing::{info, warn};

pub struct CleanupStep {
    files: Vec<String>,
}

impl CleanupStep {
    pub fn new(files: Vec<String>) -> Self {
        Self {
            files,
        }
    }
}

#[async_trait]
impl Step for CleanupStep {
    async fn execute(&self, working_dir: &Path) -> Result<()> {
        info!("Executing Cleanup step: {} files/directories to remove", self.files.len());
        
        let mut removed_count = 0;
        let mut failed_count = 0;
        
        for file_pattern in &self.files {
            let path = working_dir.join(file_pattern);
            
            if path.exists() {
                if path.is_dir() {
                    match std::fs::remove_dir_all(&path) {
                        Ok(_) => {
                            info!("Removed directory: {}", path.display());
                            removed_count += 1;
                        }
                        Err(e) => {
                            warn!("Failed to remove directory {}: {}", path.display(), e);
                            failed_count += 1;
                        }
                    }
                } else {
                    match std::fs::remove_file(&path) {
                        Ok(_) => {
                            info!("Removed file: {}", path.display());
                            removed_count += 1;
                        }
                        Err(e) => {
                            warn!("Failed to remove file {}: {}", path.display(), e);
                            failed_count += 1;
                        }
                    }
                }
            } else {
                // Check if it's a glob pattern
                let pattern_str = path.to_string_lossy();
                match glob::glob(&pattern_str) {
                    Ok(paths) => {
                        let mut pattern_matched = false;
                        for entry in paths {
                            match entry {
                                Ok(path) => {
                                    pattern_matched = true;
                                    if path.is_dir() {
                                        match std::fs::remove_dir_all(&path) {
                                            Ok(_) => {
                                                info!("Removed directory: {}", path.display());
                                                removed_count += 1;
                                            }
                                            Err(e) => {
                                                warn!("Failed to remove directory {}: {}", path.display(), e);
                                                failed_count += 1;
                                            }
                                        }
                                    } else {
                                        match std::fs::remove_file(&path) {
                                            Ok(_) => {
                                                info!("Removed file: {}", path.display());
                                                removed_count += 1;
                                            }
                                            Err(e) => {
                                                warn!("Failed to remove file {}: {}", path.display(), e);
                                                failed_count += 1;
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Glob pattern error for {}: {}", pattern_str, e);
                                    failed_count += 1;
                                }
                            }
                        }
                        if !pattern_matched {
                            warn!("No files matched pattern: {}", file_pattern);
                        }
                    }
                    Err(e) => {
                        warn!("Invalid glob pattern {}: {}", file_pattern, e);
                        failed_count += 1;
                    }
                }
            }
        }
        
        info!("Cleanup step completed: {} removed, {} failed", removed_count, failed_count);
        
        if failed_count > 0 && removed_count == 0 {
            anyhow::bail!("All cleanup operations failed");
        }
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "Cleanup"
    }
}