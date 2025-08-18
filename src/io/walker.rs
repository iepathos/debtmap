use crate::core::Language;
use anyhow::Result;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

pub struct FileWalker {
    root: PathBuf,
    languages: Vec<Language>,
    ignore_patterns: Vec<String>,
}

impl FileWalker {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            languages: vec![
                Language::Rust,
                Language::Python,
                Language::JavaScript,
                Language::TypeScript,
            ],
            ignore_patterns: vec![],
        }
    }

    pub fn with_languages(mut self, languages: Vec<Language>) -> Self {
        self.languages = languages;
        self
    }

    pub fn with_ignore_patterns(mut self, patterns: Vec<String>) -> Self {
        self.ignore_patterns = patterns;
        self
    }

    pub fn walk(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let walker = WalkBuilder::new(&self.root)
            .hidden(false)
            .git_ignore(true)
            .build();

        for entry in walker {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && self.should_process(path) {
                files.push(path.to_path_buf());
            }
        }

        Ok(files)
    }

    fn should_process(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy();
            let lang = Language::from_extension(&ext_str);

            if !self.languages.contains(&lang) {
                return false;
            }

            // Check ignore patterns against both absolute and relative paths
            let path_str = path.to_string_lossy();
            let relative_path = path.strip_prefix(&self.root)
                .unwrap_or(path)
                .to_string_lossy();
            
            for pattern in &self.ignore_patterns {
                if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                    // Check against absolute path
                    if glob_pattern.matches(&path_str) {
                        return false;
                    }
                    // Check against relative path
                    if glob_pattern.matches(&relative_path) {
                        return false;
                    }
                    // Check against filename for patterns like "*.test.rs"
                    if let Some(file_name) = path.file_name() {
                        if glob_pattern.matches(file_name.to_string_lossy().as_ref()) {
                            return false;
                        }
                    }
                }
            }

            true
        } else {
            false
        }
    }
}

pub fn find_project_files(root: &Path, languages: Vec<Language>) -> Result<Vec<PathBuf>> {
    if root.is_file() {
        // Handle single file case
        if let Some(ext) = root.extension() {
            let ext_str = ext.to_string_lossy();
            let lang = Language::from_extension(&ext_str);
            if languages.contains(&lang) || languages.is_empty() {
                return Ok(vec![root.to_path_buf()]);
            }
        }
        Ok(vec![])
    } else {
        // Handle directory case
        FileWalker::new(root.to_path_buf())
            .with_languages(languages)
            .walk()
    }
}

/// Find project files with configuration-based ignore patterns
pub fn find_project_files_with_config(
    root: &Path,
    languages: Vec<Language>,
    config: &crate::config::DebtmapConfig,
) -> Result<Vec<PathBuf>> {
    if root.is_file() {
        // Handle single file case - ignore patterns don't apply to explicitly specified files
        if let Some(ext) = root.extension() {
            let ext_str = ext.to_string_lossy();
            let lang = Language::from_extension(&ext_str);
            if languages.contains(&lang) || languages.is_empty() {
                return Ok(vec![root.to_path_buf()]);
            }
        }
        Ok(vec![])
    } else {
        // Handle directory case with ignore patterns from config
        FileWalker::new(root.to_path_buf())
            .with_languages(languages)
            .with_ignore_patterns(config.get_ignore_patterns())
            .walk()
    }
}

pub fn count_lines(path: &Path) -> Result<usize> {
    let content = std::fs::read_to_string(path)?;
    Ok(content.lines().count())
}

pub fn get_file_size(path: &Path) -> Result<u64> {
    let metadata = std::fs::metadata(path)?;
    Ok(metadata.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_project() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        // Create some test files
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("tests")).unwrap();
        fs::create_dir_all(root.join("benches")).unwrap();

        // Production source files
        let mut main_file = fs::File::create(root.join("src/main.rs")).unwrap();
        writeln!(main_file, "fn main() {{}}").unwrap();

        let mut lib_file = fs::File::create(root.join("src/lib.rs")).unwrap();
        writeln!(lib_file, "pub fn hello() {{}}").unwrap();

        // Test files
        let mut test_file = fs::File::create(root.join("tests/test_main.rs")).unwrap();
        writeln!(test_file, "#[test] fn test() {{}}").unwrap();

        let mut unit_test = fs::File::create(root.join("src/foo.test.rs")).unwrap();
        writeln!(unit_test, "#[test] fn unit_test() {{}}").unwrap();

        // Benchmark files
        let mut bench_file = fs::File::create(root.join("benches/bench.rs")).unwrap();
        writeln!(bench_file, "fn bench() {{}}").unwrap();

        (temp_dir, root)
    }

    #[test]
    fn test_find_files_without_ignore_patterns() {
        let (_temp_dir, root) = create_test_project();

        let walker = FileWalker::new(root.clone())
            .with_languages(vec![Language::Rust]);

        let files = walker.walk().unwrap();

        // Should find all Rust files
        assert_eq!(files.len(), 5);
        let file_names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(file_names.contains(&"main.rs".to_string()));
        assert!(file_names.contains(&"lib.rs".to_string()));
        assert!(file_names.contains(&"test_main.rs".to_string()));
        assert!(file_names.contains(&"foo.test.rs".to_string()));
        assert!(file_names.contains(&"bench.rs".to_string()));
    }

    #[test]
    fn test_find_files_with_ignore_patterns() {
        let (_temp_dir, root) = create_test_project();

        let walker = FileWalker::new(root.clone())
            .with_languages(vec![Language::Rust])
            .with_ignore_patterns(vec![
                "tests/**/*".to_string(),
                "*.test.rs".to_string(),
                "benches/**/*".to_string(),
            ]);

        let files = walker.walk().unwrap();

        // Should only find production source files
        assert_eq!(files.len(), 2);
        let file_names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(file_names.contains(&"main.rs".to_string()));
        assert!(file_names.contains(&"lib.rs".to_string()));
        assert!(!file_names.contains(&"test_main.rs".to_string()));
        assert!(!file_names.contains(&"foo.test.rs".to_string()));
        assert!(!file_names.contains(&"bench.rs".to_string()));
    }

    #[test]
    fn test_find_project_files_with_config() {
        let (_temp_dir, root) = create_test_project();

        let config = crate::config::DebtmapConfig {
            ignore: Some(crate::config::IgnoreConfig {
                patterns: vec![
                    "tests/**/*".to_string(),
                    "**/*.test.rs".to_string(),
                ],
            }),
            ..Default::default()
        };

        let files = find_project_files_with_config(&root, vec![Language::Rust], &config).unwrap();

        // Should exclude test files
        assert_eq!(files.len(), 3); // main.rs, lib.rs, bench.rs
        let file_names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(file_names.contains(&"main.rs".to_string()));
        assert!(file_names.contains(&"lib.rs".to_string()));
        assert!(file_names.contains(&"bench.rs".to_string()));
        assert!(!file_names.contains(&"test_main.rs".to_string()));
        assert!(!file_names.contains(&"foo.test.rs".to_string()));
    }

    #[test]
    fn test_single_file_ignores_patterns() {
        let (_temp_dir, root) = create_test_project();
        let test_file = root.join("tests/test_main.rs");

        let config = crate::config::DebtmapConfig {
            ignore: Some(crate::config::IgnoreConfig {
                patterns: vec!["tests/**/*".to_string()],
            }),
            ..Default::default()
        };

        // When a single file is specified directly, ignore patterns don't apply
        let files = find_project_files_with_config(&test_file, vec![Language::Rust], &config).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0], test_file);
    }
}
