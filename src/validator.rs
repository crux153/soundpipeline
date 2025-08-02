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

    fn normalize_path(path: &Path) -> Vec<String> {
        let mut components = Vec::new();
        
        for component in path.components() {
            match component {
                std::path::Component::Normal(name) => {
                    if let Some(name_str) = name.to_str() {
                        components.push(name_str.to_string());
                    }
                }
                std::path::Component::CurDir => {
                    // Skip current directory components
                }
                std::path::Component::ParentDir => {
                    // Handle parent directory - could add this logic if needed
                    components.push("..".to_string());
                }
                _ => {
                    // Skip other components like root, prefix
                }
            }
        }
        
        components
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
            match current.get(component) {
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
            match current.get(component) {
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
            components: &[String],
            index: usize,
        ) -> bool {
            if index >= components.len() {
                return false;
            }

            let component = &components[index];
            
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
        let components = Self::normalize_path(dir);
        if components.is_empty() {
            String::new()
        } else {
            components.join("/") // Always use / for glob patterns
        }
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
    if !selected_format.format.is_empty() && selected_format.format != "wav" && !config.has_transcode_step() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use std::fs;

    fn create_test_config() -> Config {
        Config {
            syntax: "soundpipeline".to_string(),
            syntax_version: 1,
            formats: crate::config::FormatsConfig {
                available: vec![
                    crate::config::FormatOption {
                        format: "mp3".to_string(),
                        bitrates: Some(vec!["320k".to_string(), "256k".to_string()]),
                        default_bitrate: Some("320k".to_string()),
                        bit_depths: None,
                        default_bit_depth: None,
                    },
                    crate::config::FormatOption {
                        format: "flac".to_string(),
                        bitrates: None,
                        default_bitrate: None,
                        bit_depths: Some(vec![24, 16]),
                        default_bit_depth: Some(24),
                    },
                ],
                default: Some("mp3".to_string()),
            },
            steps: vec![],
        }
    }

    fn create_test_format() -> SelectedFormat {
        SelectedFormat {
            format: "mp3".to_string(),
            bitrate: Some("320k".to_string()),
            bit_depth: None,
        }
    }

    #[test]
    fn test_file_tree_normalize_path() {
        assert_eq!(FileTree::normalize_path(Path::new("file.txt")), vec!["file.txt"]);
        assert_eq!(FileTree::normalize_path(Path::new("./file.txt")), vec!["file.txt"]);
        assert_eq!(FileTree::normalize_path(Path::new("dir/file.txt")), vec!["dir", "file.txt"]);
        assert_eq!(FileTree::normalize_path(Path::new("./dir/file.txt")), vec!["dir", "file.txt"]);
        assert_eq!(FileTree::normalize_path(Path::new("dir/subdir/file.txt")), vec!["dir", "subdir", "file.txt"]);
        assert_eq!(FileTree::normalize_path(Path::new("")), Vec::<String>::new());
        assert_eq!(FileTree::normalize_path(Path::new(".")), Vec::<String>::new());
        
        // Test Windows-style paths (only on Windows where backslash is path separator)
        #[cfg(windows)]
        {
            assert_eq!(FileTree::normalize_path(Path::new("dir\\file.txt")), vec!["dir", "file.txt"]);
            assert_eq!(FileTree::normalize_path(Path::new(".\\dir\\file.txt")), vec!["dir", "file.txt"]);
        }
    }

    #[test]
    fn test_file_tree_normalize_directory_for_pattern() {
        assert_eq!(FileTree::normalize_directory_for_pattern(Path::new("dir")), "dir");
        assert_eq!(FileTree::normalize_directory_for_pattern(Path::new("./dir")), "dir");
        assert_eq!(FileTree::normalize_directory_for_pattern(Path::new("dir/")), "dir");
        assert_eq!(FileTree::normalize_directory_for_pattern(Path::new("./dir/")), "dir");
        assert_eq!(FileTree::normalize_directory_for_pattern(Path::new(".")), "");
        assert_eq!(FileTree::normalize_directory_for_pattern(Path::new("")), "");
    }

    #[test]
    fn test_file_tree_add_and_exists() {
        let mut tree = FileTree::new();
        
        // Test adding files
        tree.add_file(Path::new("file1.txt"));
        tree.add_file(Path::new("./file2.txt"));
        tree.add_file(Path::new("dir/file3.txt"));
        tree.add_file(Path::new("./dir/subdir/file4.txt"));
        
        // Test file existence
        assert!(tree.exists(Path::new("file1.txt")));
        assert!(tree.exists(Path::new("./file1.txt")));
        assert!(tree.exists(Path::new("file2.txt")));
        assert!(tree.exists(Path::new("dir/file3.txt")));
        assert!(tree.exists(Path::new("./dir/file3.txt")));
        assert!(tree.exists(Path::new("dir/subdir/file4.txt")));
        
        // Test directory existence (created implicitly)
        assert!(tree.exists(Path::new("dir")));
        assert!(tree.exists(Path::new("./dir")));
        assert!(tree.exists(Path::new("dir/subdir")));
        
        // Test non-existent files
        assert!(!tree.exists(Path::new("nonexistent.txt")));
        assert!(!tree.exists(Path::new("dir/nonexistent.txt")));
    }

    #[test]
    fn test_file_tree_add_directory() {
        let mut tree = FileTree::new();
        
        // Test adding directories
        tree.add_directory(Path::new("dir1"));
        tree.add_directory(Path::new("./dir2"));
        tree.add_directory(Path::new("parent/child"));
        
        // Test directory existence
        assert!(tree.exists(Path::new("dir1")));
        assert!(tree.exists(Path::new("./dir1")));
        assert!(tree.exists(Path::new("dir2")));
        assert!(tree.exists(Path::new("parent")));
        assert!(tree.exists(Path::new("parent/child")));
        
        // Test is_directory
        assert!(tree.is_directory(Path::new("dir1")));
        assert!(tree.is_directory(Path::new("parent")));
        assert!(tree.is_directory(Path::new("parent/child")));
    }

    #[test]
    fn test_file_tree_is_file_vs_is_directory() {
        let mut tree = FileTree::new();
        
        tree.add_file(Path::new("file.txt"));
        tree.add_file(Path::new("dir/file.txt"));
        tree.add_directory(Path::new("empty_dir"));
        
        // Test file identification
        assert!(tree.is_file(Path::new("file.txt")));
        assert!(tree.is_file(Path::new("dir/file.txt")));
        assert!(!tree.is_file(Path::new("dir")));
        assert!(!tree.is_file(Path::new("empty_dir")));
        
        // Test directory identification
        assert!(tree.is_directory(Path::new("dir")));
        assert!(tree.is_directory(Path::new("empty_dir")));
        assert!(!tree.is_directory(Path::new("file.txt")));
        assert!(!tree.is_directory(Path::new("dir/file.txt")));
    }

    #[test]
    fn test_file_tree_remove() {
        let mut tree = FileTree::new();
        
        tree.add_file(Path::new("file1.txt"));
        tree.add_file(Path::new("dir/file2.txt"));
        tree.add_directory(Path::new("empty_dir"));
        
        assert!(tree.exists(Path::new("file1.txt")));
        assert!(tree.exists(Path::new("dir/file2.txt")));
        assert!(tree.exists(Path::new("empty_dir")));
        
        // Remove files and directories
        tree.remove(Path::new("file1.txt"));
        tree.remove(Path::new("dir/file2.txt"));
        tree.remove(Path::new("empty_dir"));
        
        assert!(!tree.exists(Path::new("file1.txt")));
        assert!(!tree.exists(Path::new("dir/file2.txt")));
        assert!(!tree.exists(Path::new("empty_dir")));
        
        // Directory should still exist (it was created implicitly for file2.txt)
        assert!(tree.exists(Path::new("dir")));
    }

    #[test]
    fn test_file_tree_find_matching() {
        let mut tree = FileTree::new();
        
        tree.add_file(Path::new("track_01.mp3"));
        tree.add_file(Path::new("track_02.mp3"));
        tree.add_file(Path::new("track_01.flac"));
        tree.add_file(Path::new("cover.jpg"));
        tree.add_file(Path::new("dir/track_03.mp3"));
        tree.add_file(Path::new("dir/other.txt"));
        
        // Test exact matches
        let matches = tree.find_matching("track_01.mp3");
        assert_eq!(matches.len(), 1);
        assert!(matches.contains(&PathBuf::from("track_01.mp3")));
        
        // Test wildcard patterns
        let matches = tree.find_matching("track_*.mp3");
        assert_eq!(matches.len(), 2);
        assert!(matches.contains(&PathBuf::from("track_01.mp3")));
        assert!(matches.contains(&PathBuf::from("track_02.mp3")));
        
        // Test extension patterns
        let matches = tree.find_matching("track_01.*");
        assert_eq!(matches.len(), 2);
        assert!(matches.contains(&PathBuf::from("track_01.mp3")));
        assert!(matches.contains(&PathBuf::from("track_01.flac")));
        
        // Test directory patterns
        let matches = tree.find_matching("dir/*");
        assert_eq!(matches.len(), 2);
        assert!(matches.contains(&PathBuf::from("dir/track_03.mp3")));
        assert!(matches.contains(&PathBuf::from("dir/other.txt")));
    }

    #[test]
    fn test_file_tree_find_in_directory() {
        let mut tree = FileTree::new();
        
        tree.add_file(Path::new("track_01.mp3"));
        tree.add_file(Path::new("dir/track_02.mp3"));
        tree.add_file(Path::new("dir/track_03.flac"));
        tree.add_file(Path::new("dir/subdir/track_04.mp3"));
        
        // Test finding in root directory
        let matches = tree.find_in_directory(Path::new("."), "track_*.mp3");
        assert_eq!(matches.len(), 1);
        assert!(matches.contains(&PathBuf::from("track_01.mp3")));
        
        // Test finding in subdirectory
        let matches = tree.find_in_directory(Path::new("dir"), "track_*.*");
        assert_eq!(matches.len(), 2);
        assert!(matches.contains(&PathBuf::from("dir/track_02.mp3")));
        assert!(matches.contains(&PathBuf::from("dir/track_03.flac")));
        
        // Test with ./ prefix
        let matches = tree.find_in_directory(Path::new("./dir"), "track_*.*");
        assert_eq!(matches.len(), 2);
        
        // Test non-existent directory
        let matches = tree.find_in_directory(Path::new("nonexistent"), "*.mp3");
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_validate_timestamp() {
        // Valid timestamps
        assert!(validate_timestamp("0:00:00.000"));
        assert!(validate_timestamp("1:23:45.678"));
        assert!(validate_timestamp("12:59:59.999"));
        assert!(validate_timestamp("0:00:00.000000"));
        assert!(validate_timestamp("1:23:45.678901"));
        
        // Invalid timestamps
        assert!(!validate_timestamp("00:00.000"));      // Missing hour
        assert!(!validate_timestamp("0:0:00.000"));     // Single digit minute
        assert!(!validate_timestamp("0:00:0.000"));     // Single digit second
        assert!(!validate_timestamp("0:00:00.00"));     // Wrong millisecond length
        assert!(!validate_timestamp("0:00:00.0000"));   // Wrong millisecond length
        assert!(!validate_timestamp("0:00:00"));        // Missing milliseconds
        assert!(!validate_timestamp("0:60:00.000"));    // Invalid minute
        assert!(!validate_timestamp("0:00:60.000"));    // Invalid second
        assert!(!validate_timestamp("0:00:00.abc"));    // Non-numeric milliseconds
        assert!(!validate_timestamp("a:00:00.000"));    // Non-numeric hour
    }

    #[test]
    fn test_validate_pipeline_success() {
        let mut config = create_test_config();
        config.steps = vec![
            crate::config::StepConfig::Ffmpeg {
                input: "input.mkv".to_string(),
                output: "audio.wav".to_string(),
                args: vec!["-vn".to_string()],
            },
            crate::config::StepConfig::Split {
                input: "audio.wav".to_string(),
                output_dir: "split".to_string(),
                files: vec![
                    crate::config::SplitFile {
                        file: "track_01.wav".to_string(),
                        start: "0:00:00.000".to_string(),
                        end: "0:03:00.000".to_string(),
                        start_seconds: 0.0,
                        end_seconds: 180.0,
                    },
                ],
            },
            crate::config::StepConfig::Transcode {
                input_dir: "split".to_string(),
                output_dir: "output".to_string(),
                files: vec!["track_01.wav".to_string()],
            },
            crate::config::StepConfig::Tag {
                input_dir: "output".to_string(),
                files: vec![
                    crate::config::TagFile {
                        file: "track_01.*".to_string(),
                        title: Some("Track 1".to_string()),
                        artist: None,
                        album: None,
                        album_artist: None,
                        track: None,
                        track_total: None,
                        disk: None,
                        disk_total: None,
                        album_art: None,
                        genre: None,
                        year: None,
                        comment: None,
                    },
                ],
            },
        ];
        
        let format = create_test_format();
        let temp_dir = TempDir::new().unwrap();
        
        // Create input file
        fs::write(temp_dir.path().join("input.mkv"), "dummy content").unwrap();
        
        let result = validate_pipeline(&config, &format, temp_dir.path()).unwrap();
        
        assert!(result.is_valid);
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_validate_pipeline_missing_input() {
        let mut config = create_test_config();
        config.steps = vec![
            crate::config::StepConfig::Ffmpeg {
                input: "nonexistent.mkv".to_string(),
                output: "audio.wav".to_string(),
                args: vec![],
            },
        ];
        
        let format = create_test_format();
        let temp_dir = TempDir::new().unwrap();
        
        let result = validate_pipeline(&config, &format, temp_dir.path()).unwrap();
        
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("nonexistent.mkv"));
        assert!(result.errors[0].contains("does not exist"));
    }

    #[test]
    fn test_validate_pipeline_invalid_timestamp() {
        let mut config = create_test_config();
        config.steps = vec![
            crate::config::StepConfig::Ffmpeg {
                input: "input.mkv".to_string(),
                output: "audio.wav".to_string(),
                args: vec![],
            },
            crate::config::StepConfig::Split {
                input: "audio.wav".to_string(),
                output_dir: "split".to_string(),
                files: vec![
                    crate::config::SplitFile {
                        file: "track_01.wav".to_string(),
                        start: "0:00:00".to_string(), // Invalid timestamp
                        end: "0:03:00.000".to_string(),
                        start_seconds: 0.0,
                        end_seconds: 180.0,
                    },
                ],
            },
        ];
        
        let format = create_test_format();
        let temp_dir = TempDir::new().unwrap();
        
        fs::write(temp_dir.path().join("input.mkv"), "dummy content").unwrap();
        
        let result = validate_pipeline(&config, &format, temp_dir.path()).unwrap();
        
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("Invalid start timestamp"));
        assert!(result.errors[0].contains("0:00:00"));
    }

    #[test]
    fn test_validate_pipeline_broken_dependency_chain() {
        let mut config = create_test_config();
        config.steps = vec![
            crate::config::StepConfig::Ffmpeg {
                input: "input.mkv".to_string(),
                output: "audio.wav".to_string(),
                args: vec![],
            },
            crate::config::StepConfig::Split {
                input: "different_audio.wav".to_string(), // Wrong input
                output_dir: "split".to_string(),
                files: vec![
                    crate::config::SplitFile {
                        file: "track_01.wav".to_string(),
                        start: "0:00:00.000".to_string(),
                        end: "0:03:00.000".to_string(),
                        start_seconds: 0.0,
                        end_seconds: 180.0,
                    },
                ],
            },
        ];
        
        let format = create_test_format();
        let temp_dir = TempDir::new().unwrap();
        
        fs::write(temp_dir.path().join("input.mkv"), "dummy content").unwrap();
        
        let result = validate_pipeline(&config, &format, temp_dir.path()).unwrap();
        
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("different_audio.wav"));
        assert!(result.errors[0].contains("does not exist"));
    }

    #[test]
    fn test_validate_pipeline_tag_no_matching_files() {
        let mut config = create_test_config();
        config.steps = vec![
            crate::config::StepConfig::Ffmpeg {
                input: "input.mkv".to_string(),
                output: "audio.wav".to_string(),
                args: vec![],
            },
            crate::config::StepConfig::Tag {
                input_dir: ".".to_string(),
                files: vec![
                    crate::config::TagFile {
                        file: "nonexistent_*.mp3".to_string(),
                        title: Some("Track 1".to_string()),
                        artist: None,
                        album: None,
                        album_artist: None,
                        track: None,
                        track_total: None,
                        disk: None,
                        disk_total: None,
                        album_art: None,
                        genre: None,
                        year: None,
                        comment: None,
                    },
                ],
            },
        ];
        
        let format = create_test_format();
        let temp_dir = TempDir::new().unwrap();
        
        fs::write(temp_dir.path().join("input.mkv"), "dummy content").unwrap();
        
        let result = validate_pipeline(&config, &format, temp_dir.path()).unwrap();
        
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("No files matching pattern"));
        assert!(result.errors[0].contains("nonexistent_*.mp3"));
    }

    #[test]
    fn test_validate_pipeline_cleanup_warnings() {
        let mut config = create_test_config();
        config.steps = vec![
            crate::config::StepConfig::Cleanup {
                files: vec![
                    "nonexistent_file.wav".to_string(),
                    "nonexistent_dir".to_string(),
                ],
            },
        ];
        
        // Use empty format to avoid format-related warnings
        let format = SelectedFormat {
            format: String::new(),
            bitrate: None,
            bit_depth: None,
        };
        let temp_dir = TempDir::new().unwrap();
        
        let result = validate_pipeline(&config, &format, temp_dir.path()).unwrap();
        
        assert!(result.is_valid); // Cleanup warnings don't fail validation
        assert_eq!(result.errors.len(), 0);
        assert_eq!(result.warnings.len(), 2);
        assert!(result.warnings[0].contains("may not exist when cleanup runs"));
        assert!(result.warnings[1].contains("may not exist when cleanup runs"));
    }

    #[test]
    fn test_validate_pipeline_missing_album_art() {
        let mut config = create_test_config();
        config.steps = vec![
            crate::config::StepConfig::Ffmpeg {
                input: "input.mkv".to_string(),
                output: "audio.wav".to_string(),
                args: vec![],
            },
            crate::config::StepConfig::Tag {
                input_dir: ".".to_string(),
                files: vec![
                    crate::config::TagFile {
                        file: "audio.wav".to_string(),
                        title: Some("Track 1".to_string()),
                        artist: None,
                        album: None,
                        album_artist: None,
                        track: None,
                        track_total: None,
                        disk: None,
                        disk_total: None,
                        album_art: Some("nonexistent_cover.jpg".to_string()),
                        genre: None,
                        year: None,
                        comment: None,
                    },
                ],
            },
        ];
        
        // Use empty format to avoid format-related warnings
        let format = SelectedFormat {
            format: String::new(),
            bitrate: None,
            bit_depth: None,
        };
        let temp_dir = TempDir::new().unwrap();
        
        fs::write(temp_dir.path().join("input.mkv"), "dummy content").unwrap();
        
        let result = validate_pipeline(&config, &format, temp_dir.path()).unwrap();
        
        assert!(result.is_valid); // Missing album art is just a warning
        assert_eq!(result.errors.len(), 0);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("Album art file"));
        assert!(result.warnings[0].contains("nonexistent_cover.jpg"));
    }

    #[test]
    fn test_validate_pipeline_format_without_transcode() {
        let mut config = create_test_config();
        config.steps = vec![
            crate::config::StepConfig::Ffmpeg {
                input: "input.mkv".to_string(),
                output: "audio.wav".to_string(),
                args: vec![],
            },
        ];
        
        let format = create_test_format(); // mp3 format but no transcode step
        let temp_dir = TempDir::new().unwrap();
        
        fs::write(temp_dir.path().join("input.mkv"), "dummy content").unwrap();
        
        let result = validate_pipeline(&config, &format, temp_dir.path()).unwrap();
        
        assert!(result.is_valid);
        assert_eq!(result.errors.len(), 0);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("Output format 'mp3' specified but no transcode step found"));
    }

    #[test]
    fn test_validate_pipeline_complex_workflow() {
        let mut config = create_test_config();
        config.steps = vec![
            crate::config::StepConfig::Ffmpeg {
                input: "input.mkv".to_string(),
                output: "extracted.wav".to_string(),
                args: vec!["-vn".to_string()],
            },
            crate::config::StepConfig::Split {
                input: "extracted.wav".to_string(),
                output_dir: "./split_output".to_string(),
                files: vec![
                    crate::config::SplitFile {
                        file: "track_01.wav".to_string(),
                        start: "0:00:00.000".to_string(),
                        end: "0:03:30.500".to_string(),
                        start_seconds: 0.0,
                        end_seconds: 210.5,
                    },
                    crate::config::SplitFile {
                        file: "track_02.wav".to_string(),
                        start: "0:03:30.500".to_string(),
                        end: "0:07:15.750".to_string(),
                        start_seconds: 210.5,
                        end_seconds: 435.75,
                    },
                ],
            },
            crate::config::StepConfig::Transcode {
                input_dir: "./split_output".to_string(),
                output_dir: "./final_output".to_string(),
                files: vec!["track_01.wav".to_string(), "track_02.wav".to_string()],
            },
            crate::config::StepConfig::Tag {
                input_dir: "./final_output".to_string(),
                files: vec![
                    crate::config::TagFile {
                        file: "track_*.*".to_string(),
                        title: None,
                        artist: Some("Artist Name".to_string()),
                        album: Some("Album Name".to_string()),
                        album_artist: None,
                        track: None,
                        track_total: None,
                        disk: None,
                        disk_total: None,
                        album_art: Some("cover.jpg".to_string()),
                        genre: None,
                        year: None,
                        comment: None,
                    },
                ],
            },
            crate::config::StepConfig::Cleanup {
                files: vec![
                    "extracted.wav".to_string(),
                    "split_output".to_string(),
                ],
            },
        ];
        
        let format = create_test_format();
        let temp_dir = TempDir::new().unwrap();
        
        // Create required files
        fs::write(temp_dir.path().join("input.mkv"), "dummy video content").unwrap();
        fs::write(temp_dir.path().join("cover.jpg"), "dummy image content").unwrap();
        
        let result = validate_pipeline(&config, &format, temp_dir.path()).unwrap();
        
        assert!(result.is_valid);
        assert_eq!(result.errors.len(), 0);
        assert_eq!(result.warnings.len(), 0);
    }
}