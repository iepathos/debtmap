//! Unified Lines of Code (LOC) counting module
//!
//! This module provides a single source of truth for LOC calculation across all analysis modes.
//! It ensures consistent counting whether coverage data is provided or not.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Counts lines of code using a consistent methodology across all analysis modes.
///
/// # Line Counting Rules
///
/// - **Physical Lines**: Raw line count from file
/// - **Code Lines**: Lines containing executable code (excludes comments, blanks)
/// - **Comment Lines**: Lines that are primarily comments
/// - **Blank Lines**: Lines containing only whitespace
///
/// # File Filtering
///
/// Files are excluded if they match:
/// - Test file patterns: `*_test.rs`, `tests/**/*`
/// - Generated file markers: `@generated`, `DO NOT EDIT`
/// - Custom exclusion patterns from config
///
/// # Examples
///
/// ```rust
/// use debtmap::metrics::loc_counter::LocCounter;
/// use std::path::Path;
///
/// let counter = LocCounter::default();
/// let count = counter.count_file(Path::new("src/lib.rs")).unwrap();
/// println!("Code lines: {}", count.code_lines);
/// ```
#[derive(Debug, Clone, Default)]
pub struct LocCounter {
    config: LocCountingConfig,
}

impl LocCounter {
    /// Create a new LOC counter with custom configuration
    pub fn new(config: LocCountingConfig) -> Self {
        Self { config }
    }

    /// Count lines in a single file
    pub fn count_file(&self, path: &Path) -> Result<LocCount, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        Ok(self.count_content(&content))
    }

    /// Count lines in file content (pure function)
    pub fn count_content(&self, content: &str) -> LocCount {
        let physical_lines = content.lines().count();
        let mut code_lines = 0;
        let mut comment_lines = 0;
        let mut blank_lines = 0;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                blank_lines += 1;
            } else if is_comment_line(trimmed) {
                comment_lines += 1;
            } else {
                code_lines += 1;
            }
        }

        LocCount {
            physical_lines,
            code_lines,
            comment_lines,
            blank_lines,
        }
    }

    /// Determine if file should be included in LOC count
    pub fn should_include(&self, path: &Path) -> bool {
        if !self.config.include_tests && self.is_test_file(path) {
            log::debug!("Excluding test file: {}", path.display());
            return false;
        }

        if !self.config.include_generated && self.is_generated(path) {
            log::debug!("Excluding generated file: {}", path.display());
            return false;
        }

        if self.is_excluded_by_pattern(path) {
            log::debug!("Excluding file by pattern: {}", path.display());
            return false;
        }

        log::debug!("Including file in LOC count: {}", path.display());
        true
    }

    /// Check if path is a test file
    pub fn is_test_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Check if in tests directory
        if path_str.contains("/tests/") || path_str.contains("/test/") {
            return true;
        }

        // Check if filename has _test suffix
        if let Some(file_name) = path.file_stem() {
            let name = file_name.to_string_lossy();
            if name.ends_with("_test") || name.ends_with("_tests") {
                return true;
            }
        }

        false
    }

    /// Check if path is a generated file
    pub fn is_generated(&self, path: &Path) -> bool {
        // Check file content for generation markers
        if let Ok(content) = std::fs::read_to_string(path) {
            let first_100_lines: String = content.lines().take(100).collect::<Vec<_>>().join("\n");

            if first_100_lines.contains("@generated")
                || first_100_lines.contains("DO NOT EDIT")
                || first_100_lines.contains("automatically generated")
            {
                return true;
            }
        }

        // Check filename patterns
        let path_str = path.to_string_lossy();
        path_str.contains(".generated.") || path_str.ends_with(".g.rs")
    }

    /// Check if excluded by custom patterns
    fn is_excluded_by_pattern(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        for pattern in &self.config.exclude_patterns {
            if path_str.contains(pattern.as_str()) {
                return true;
            }
        }

        false
    }

    /// Count LOC for an entire project from file metrics
    ///
    /// This ensures each file is counted exactly once by using a HashMap
    /// to track unique files.
    pub fn count_from_file_paths(&self, files: &[PathBuf]) -> ProjectLocCount {
        let mut file_counts = HashMap::new();

        for file_path in files {
            if !self.should_include(file_path) {
                continue;
            }

            if let Ok(count) = self.count_file(file_path) {
                file_counts.insert(file_path.clone(), count);
            }
        }

        let total = self.aggregate_counts(&file_counts);

        ProjectLocCount {
            total,
            by_file: file_counts,
        }
    }

    /// Aggregate individual file counts into total
    fn aggregate_counts(&self, file_counts: &HashMap<PathBuf, LocCount>) -> LocCount {
        let mut total = LocCount::default();

        for count in file_counts.values() {
            total.physical_lines += count.physical_lines;
            total.code_lines += count.code_lines;
            total.comment_lines += count.comment_lines;
            total.blank_lines += count.blank_lines;
        }

        total
    }
}

/// Individual file line count
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LocCount {
    /// Raw line count from file
    pub physical_lines: usize,
    /// Lines containing executable code
    pub code_lines: usize,
    /// Lines that are primarily comments
    pub comment_lines: usize,
    /// Lines containing only whitespace
    pub blank_lines: usize,
}

/// Project-level LOC count with file-by-file breakdown
#[derive(Debug, Clone)]
pub struct ProjectLocCount {
    /// Aggregated total across all files
    pub total: LocCount,
    /// Per-file breakdown
    pub by_file: HashMap<PathBuf, LocCount>,
}

/// Configuration for LOC counting
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct LocCountingConfig {
    /// Include test files in LOC count (default: false)
    #[serde(default)]
    pub include_tests: bool,
    /// Include generated files in LOC count (default: false)
    #[serde(default)]
    pub include_generated: bool,
    /// Count comments as code lines (default: false)
    #[serde(default)]
    pub count_comments: bool,
    /// Count blank lines as code lines (default: false)
    #[serde(default)]
    pub count_blanks: bool,
    /// Additional exclusion patterns
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
}

/// Check if a line is primarily a comment
fn is_comment_line(trimmed_line: &str) -> bool {
    // Rust comments
    if trimmed_line.starts_with("//")
        || trimmed_line.starts_with("/*")
        || trimmed_line.starts_with('*') && !trimmed_line.starts_with("*/")
    {
        return true;
    }

    // Python comments
    if trimmed_line.starts_with('#') {
        return true;
    }

    // JavaScript/TypeScript comments
    // (already covered by // and /*)

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loc_counter_consistent() {
        let counter = LocCounter::default();
        let content = "fn main() {\n    println!(\"Hello\");\n}\n";

        // Count twice, should be identical
        let count1 = counter.count_content(content);
        let count2 = counter.count_content(content);
        assert_eq!(count1, count2);
    }

    #[test]
    fn test_count_content() {
        let counter = LocCounter::default();
        let content = "// Comment\nfn main() {\n\n    println!(\"Hello\");\n}\n";

        let count = counter.count_content(content);

        assert_eq!(count.physical_lines, 5);
        assert_eq!(count.comment_lines, 1);
        assert_eq!(count.blank_lines, 1);
        assert_eq!(count.code_lines, 3); // fn main(), println, }
    }

    #[test]
    fn test_test_file_exclusion() {
        let counter = LocCounter::default();
        assert!(!counter.should_include(Path::new("tests/integration_test.rs")));
        assert!(!counter.should_include(Path::new("src/foo_test.rs")));
        assert!(counter.should_include(Path::new("src/main.rs")));
    }

    #[test]
    fn test_is_comment_line() {
        assert!(is_comment_line("// This is a comment"));
        assert!(is_comment_line("/* Block comment */"));
        assert!(is_comment_line("# Python comment"));
        assert!(!is_comment_line("let x = 5; // inline comment"));
        assert!(!is_comment_line("let x = 5;"));
    }

    #[test]
    fn test_count_from_unique_files() {
        // This test demonstrates that counting from a list of files
        // ensures each file is only counted once
        let counter = LocCounter::default();

        // Create a test scenario (would need actual files in real test)
        let files = vec![PathBuf::from("src/main.rs"), PathBuf::from("src/lib.rs")];

        let result = counter.count_from_file_paths(&files);

        // Each file should appear exactly once in by_file
        assert_eq!(result.by_file.len(), files.len());
    }
}
