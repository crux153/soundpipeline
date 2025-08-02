use crate::config::{Config, StepConfig, SelectedFormat};
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use tracing::{info, debug};
use glob::Pattern;

#[derive(Debug)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn add_error(&mut self, error: String) {
        self.is_valid = false;
        self.errors.push(error);
    }

    fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

#[derive(Debug, Clone)]
enum FileSystemEntry {
    File,
    Directory(HashMap<String, FileSystemEntry>),
}

#[derive(Debug, Clone)]
struct FileTree {
    root: HashMap<String, FileSystemEntry>,
}

impl FileTree {
    fn new() -> Self {
        Self {
            root: HashMap::new(),
        }
    }

    fn normalize_path(path: &Path) -> Vec<&str> {
        let path_str = path.to_str().unwrap_or("");
        let normalized = path_str.trim_start_matches("./");
        normalized
            .split('/')
            .filter(|s| !s.is_empty() && *s != ".")
            .collect()
    }

    fn add_file(&mut self, path: &Path) {
        let components = Self::normalize_path(path);
        
        if components.is_empty() {
            return;
        }

        let mut current = &mut self.root;
        
        // Navigate/create directories up to the parent
        for (i, component) in components.iter().enumerate() {
            if i == components.len() - 1 {
                // Last component is the file
                current.insert(component.to_string(), FileSystemEntry::File);
                debug!("FileTree: Added file {:?}", path);
            } else {
                // Create directory if it doesn't exist
                let entry = current.entry(component.to_string())
                    .or_insert_with(|| FileSystemEntry::Directory(HashMap::new()));
                
                if let FileSystemEntry::Directory(dir) = entry {
                    current = dir;
                } else {
                    // Path component exists as file, can't create directory
                    return;
                }
            }
        }
    }

    fn add_directory(&mut self, path: &Path) {
        let components = Self::normalize_path(path);
        
        if components.is_empty() {
            return;
        }

        let mut current = &mut self.root;
        
        for component in components {
            let entry = current.entry(component.to_string())
                .or_insert_with(|| FileSystemEntry::Directory(HashMap::new()));
            
            if let FileSystemEntry::Directory(dir) = entry {
                current = dir;
            } else {
                // Path component exists as file, can't create directory
                return;
            }
        }
        
        debug!("FileTree: Added directory {:?}", path);
    }

    fn exists(&self, path: &Path) -> bool {
        let components = Self::normalize_path(path);
        
        if components.is_empty() {
            return false;
        }

        let mut current = &self.root;
        
        for (i, component) in components.iter().enumerate() {
            match current.get(*component) {
                Some(FileSystemEntry::File) => return i == components.len() - 1,
                Some(FileSystemEntry::Directory(dir)) => {
                    if i == components.len() - 1 {
                        return true; // Found the directory
                    }
                    current = dir;
                }
                None => return false,
            }
        }
        
        false
    }

    fn is_file(&self, path: &Path) -> bool {
        let components = Self::normalize_path(path);
        
        if components.is_empty() {
            return false;
        }

        let mut current = &self.root;
        
        for (i, component) in components.iter().enumerate() {
            match current.get(*component) {
                Some(FileSystemEntry::File) => return i == components.len() - 1,
                Some(FileSystemEntry::Directory(dir)) => {
                    if i == components.len() - 1 {
                        return false; // It's a directory, not a file
                    }
                    current = dir;
                }
                None => return false,
            }
        }
        
        false
    }

    fn is_directory(&self, path: &Path) -> bool {
        self.exists(path) && !self.is_file(path)
    }

    fn remove(&mut self, path: &Path) {
        let components = Self::normalize_path(path);
        
        if components.is_empty() {
            return;
        }

        fn remove_recursive(
            current: &mut HashMap<String, FileSystemEntry>,
            components: &[&str],
            index: usize,
        ) -> bool {
            if index >= components.len() {
                return false;
            }

            let component = components[index];
            
            if index == components.len() - 1 {
                // Remove the target
                current.remove(component).is_some()
            } else {
                // Navigate to parent directory
                if let Some(FileSystemEntry::Directory(dir)) = current.get_mut(component) {
                    remove_recursive(dir, components, index + 1)
                } else {
                    false
                }
            }
        }

        if remove_recursive(&mut self.root, &components, 0) {
            debug!("FileTree: Removed {:?}", path);
        }
    }

    fn find_matching(&self, pattern: &str) -> Vec<PathBuf> {
        let mut results = Vec::new();
        
        fn collect_paths(
            current: &HashMap<String, FileSystemEntry>,
            current_path: PathBuf,
            pattern: &Pattern,
            results: &mut Vec<PathBuf>,
        ) {
            for (name, entry) in current {
                let path = if current_path.as_os_str().is_empty() {
                    PathBuf::from(name)
                } else {
                    current_path.join(name)
                };
                
                match entry {
                    FileSystemEntry::File => {
                        if let Some(path_str) = path.to_str() {
                            if pattern.matches(path_str) {
                                results.push(path);
                            }
                        }
                    }
                    FileSystemEntry::Directory(dir) => {
                        // Check if directory itself matches
                        if let Some(path_str) = path.to_str() {
                            if pattern.matches(path_str) {
                                results.push(path.clone());
                            }
                        }
                        // Recursively check contents
                        collect_paths(dir, path, pattern, results);
                    }
                }
            }
        }
        
        if let Ok(glob_pattern) = Pattern::new(pattern) {
            collect_paths(&self.root, PathBuf::new(), &glob_pattern, &mut results);
        }
        
        results
    }

    fn normalize_directory_for_pattern(dir: &Path) -> String {
        let dir_str = dir.to_string_lossy();
        let normalized = dir_str.trim_start_matches("./").trim_end_matches('/');
        normalized.to_string()
    }

    fn find_in_directory(&self, dir: &Path, pattern: &str) -> Vec<PathBuf> {
        let normalized_dir = Self::normalize_directory_for_pattern(dir);
        
        let full_pattern = if normalized_dir.is_empty() || normalized_dir == "." {
            pattern.to_string()
        } else {
            format!("{}/{}", normalized_dir, pattern)
        };
        
        self.find_matching(&full_pattern)
    }
}

pub fn validate_pipeline(
    config: &Config,
    selected_format: &SelectedFormat,
    working_dir: &Path,
) -> Result<ValidationResult> {
    let mut result = ValidationResult::new();
    let mut file_tree = FileTree::new();
    
    info!("Starting pipeline validation");
    
    // Check if working directory exists and is accessible
    if !working_dir.exists() {
        debug!("Working directory {} will be created during execution", working_dir.display());
    } else if !working_dir.is_dir() {
        result.add_error(format!("Working directory {} is not a directory", working_dir.display()));
    }
    
    // Track initial files in working directory by scanning recursively
    if working_dir.exists() {
        scan_directory_recursive(working_dir, working_dir, &mut file_tree)?;
    }
    
    // Validate each step
    for (idx, step) in config.steps.iter().enumerate() {
        debug!("Validating step {}: {:?}", idx + 1, step);
        
        match step {
            StepConfig::Ffmpeg { input, output, args: _ } => {
                // Check if input file exists
                if !file_tree.exists(Path::new(input)) {
                    let input_path = working_dir.join(input);
                    if !input_path.exists() {
                        result.add_error(format!(
                            "Step {} (ffmpeg): Input file '{}' does not exist and will not be created by previous steps",
                            idx + 1, input
                        ));
                    }
                }
                
                // Simulate output file creation
                file_tree.add_file(Path::new(output));
            }
            
            StepConfig::Split { input, output_dir, files } => {
                // Check if input file exists
                if !file_tree.exists(Path::new(input)) {
                    let input_path = working_dir.join(input);
                    if !input_path.exists() {
                        result.add_error(format!(
                            "Step {} (split): Input file '{}' does not exist and will not be created by previous steps",
                            idx + 1, input
                        ));
                    }
                }
                
                // Create output directory
                if output_dir != "." && !output_dir.is_empty() {
                    file_tree.add_directory(Path::new(output_dir));
                }
                
                // Validate timestamps and simulate output file creation
                for file in files {
                    if file.file.is_empty() {
                        result.add_error(format!(
                            "Step {} (split): Empty filename in split configuration",
                            idx + 1
                        ));
                    }
                    
                    // Validate timestamp format
                    if !validate_timestamp(&file.start) {
                        result.add_error(format!(
                            "Step {} (split): Invalid start timestamp '{}' for file '{}'. Expected format: h:mm:ss.SSS or h:mm:ss.SSSSSS",
                            idx + 1, file.start, file.file
                        ));
                    }
                    
                    if !validate_timestamp(&file.end) {
                        result.add_error(format!(
                            "Step {} (split): Invalid end timestamp '{}' for file '{}'. Expected format: h:mm:ss.SSS or h:mm:ss.SSSSSS",
                            idx + 1, file.end, file.file
                        ));
                    }
                    
                    // Simulate output file creation
                    let output_file = if output_dir == "." || output_dir.is_empty() {
                        PathBuf::from(&file.file)
                    } else {
                        PathBuf::from(output_dir).join(&file.file)
                    };
                    file_tree.add_file(&output_file);
                }
            }
            
            StepConfig::Transcode { input_dir, output_dir, files } => {
                // Create output directory
                if output_dir != "." && !output_dir.is_empty() {
                    file_tree.add_directory(Path::new(output_dir));
                }
                
                // Check if specified files exist in file tree
                for file in files {
                    let input_file = if input_dir == "." || input_dir.is_empty() {
                        PathBuf::from(file)
                    } else {
                        PathBuf::from(input_dir).join(file)
                    };
                    
                    if !file_tree.exists(&input_file) {
                        let input_file_path = working_dir.join(&input_file);
                        if !input_file_path.exists() {
                            result.add_error(format!(
                                "Step {} (transcode): Input file '{}' does not exist and will not be created by previous steps",
                                idx + 1, input_file.display()
                            ));
                        }
                    }
                    
                    // Simulate output file creation based on selected format
                    let base_name = file.trim_end_matches(".wav");
                    let output_file_name = format!("{}.{}", base_name, selected_format.format);
                    let output_file = if output_dir == "." || output_dir.is_empty() {
                        PathBuf::from(output_file_name)
                    } else {
                        PathBuf::from(output_dir).join(output_file_name)
                    };
                    file_tree.add_file(&output_file);
                }
            }
            
            StepConfig::Tag { input_dir, files } => {
                // Check if files to tag exist using glob matching
                for tag_file in files {
                    let matches = file_tree.find_in_directory(Path::new(input_dir), &tag_file.file);
                    
                    if matches.is_empty() {
                        result.add_error(format!(
                            "Step {} (tag): No files matching pattern '{}' in directory '{}'",
                            idx + 1, tag_file.file, input_dir
                        ));
                    } else {
                        debug!("Step {} (tag): Found {} files matching pattern '{}' in directory '{}'", 
                              idx + 1, matches.len(), tag_file.file, input_dir);
                    }
                    
                    // Check if album art file exists if specified
                    if let Some(album_art) = &tag_file.album_art {
                        if !file_tree.exists(Path::new(album_art)) {
                            result.add_warning(format!(
                                "Step {} (tag): Album art file '{}' does not exist",
                                idx + 1, album_art
                            ));
                        }
                    }
                }
            }
            
            StepConfig::Cleanup { files } => {
                // Check if files or directories exist in the simulated file tree
                for file in files {
                    let path = Path::new(file);
                    if !file_tree.exists(path) {
                        result.add_warning(format!(
                            "Step {} (cleanup): Path '{}' may not exist when cleanup runs",
                            idx + 1, file
                        ));
                    } else {
                        // Determine the type for better messages
                        let entry_type = if file_tree.is_directory(path) {
                            "directory"
                        } else {
                            "file"
                        };
                        debug!("Step {} (cleanup): Will remove {} '{}'", idx + 1, entry_type, file);
                    }
                    // Simulate removal
                    file_tree.remove(path);
                }
            }
        }
    }
    
    // Check if output format requires transcoding
    if selected_format.format != "wav" && !config.has_transcode_step() {
        result.add_warning(format!(
            "Output format '{}' specified but no transcode step found in pipeline",
            selected_format.format
        ));
    }
    
    info!("Pipeline validation completed: {} errors, {} warnings", 
          result.errors.len(), result.warnings.len());
    
    Ok(result)
}

fn validate_timestamp(timestamp: &str) -> bool {
    // Expected format: h:mm:ss.SSS or h:mm:ss.SSSSSS
    let parts: Vec<&str> = timestamp.split(':').collect();
    if parts.len() != 3 {
        return false;
    }
    
    // Validate hours
    if parts[0].parse::<u32>().is_err() {
        return false;
    }
    
    // Validate minutes
    if parts[1].len() != 2 || parts[1].parse::<u32>().map(|m| m >= 60).unwrap_or(true) {
        return false;
    }
    
    // Validate seconds and milliseconds/microseconds
    let seconds_parts: Vec<&str> = parts[2].split('.').collect();
    if seconds_parts.len() != 2 {
        return false;
    }
    
    if seconds_parts[0].len() != 2 || seconds_parts[0].parse::<u32>().map(|s| s >= 60).unwrap_or(true) {
        return false;
    }
    
    // Milliseconds should be 3 or 6 digits
    let millis_len = seconds_parts[1].len();
    if millis_len != 3 && millis_len != 6 {
        return false;
    }
    
    seconds_parts[1].parse::<u32>().is_ok()
}

fn scan_directory_recursive(base_dir: &Path, current_dir: &Path, file_tree: &mut FileTree) -> Result<()> {
    if let Ok(entries) = std::fs::read_dir(current_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(relative_path) = path.strip_prefix(base_dir) {
                if path.is_file() {
                    file_tree.add_file(relative_path);
                } else if path.is_dir() {
                    file_tree.add_directory(relative_path);
                    scan_directory_recursive(base_dir, &path, file_tree)?;
                }
            }
        }
    }
    Ok(())
}