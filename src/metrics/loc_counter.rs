//! Unified Lines of Code (LOC) counting module
//!
//! This module provides a single source of truth for LOC calculation across all analysis modes.
//! It ensures consistent counting whether coverage data is provided or not.
//!
//! # LOC Counting Methodology (Spec 201)
//!
//! - **Physical Lines**: Raw line count from file (includes all lines)
//! - **Code Lines**: Lines containing executable code (excludes comments and blanks)
//! - **Comment Lines**: Lines that are primarily comments (single-line or multi-line)
//! - **Blank Lines**: Lines containing only whitespace
//!
//! The invariant `physical_lines == code_lines + comment_lines + blank_lines` always holds.
//!
//! ## Multi-line Comment Handling
//!
//! This module correctly tracks multi-line block comments (`/* ... */`) using state tracking.
//! Lines inside block comments are counted as comments even if they don't start with `*`.
//!
//! ## Language-Aware Detection
//!
//! The module is language-aware and handles:
//! - Rust: `//`, `/* */` (nestable), and attributes `#[...]` (counted as code, not comments)
//! - Python: `#` comments
//! - JavaScript/TypeScript: `//` and `/* */` comments
//!
//! ## Limitations
//!
//! - Comment markers inside string literals are not detected (would require full parsing)
//! - Raw strings with comment markers (e.g., `r#"/* not a comment */"#`) may be miscounted

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Language identifier for language-aware comment detection (Spec 201)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LocLanguage {
    /// Rust: `//`, `/* */` (nestable), `#[...]` are code (not comments)
    Rust,
    /// Python: `#` comments, `"""` docstrings (simplified as comments)
    Python,
    /// JavaScript: `//` and `/* */` comments
    JavaScript,
    /// TypeScript: `//` and `/* */` comments
    TypeScript,
    /// Unknown language - uses conservative comment detection
    #[default]
    Unknown,
}

impl LocLanguage {
    /// Detect language from file extension
    pub fn from_extension(ext: Option<&OsStr>) -> Self {
        match ext.and_then(|e| e.to_str()) {
            Some("rs") => Self::Rust,
            Some("py") | Some("pyi") => Self::Python,
            Some("js") | Some("jsx") | Some("mjs") | Some("cjs") => Self::JavaScript,
            Some("ts") | Some("tsx") | Some("mts") | Some("cts") => Self::TypeScript,
            _ => Self::Unknown,
        }
    }

    /// Detect language from file path
    pub fn from_path(path: &Path) -> Self {
        Self::from_extension(path.extension())
    }
}

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

    /// Count lines in a single file with auto-detected language (Spec 201)
    pub fn count_file(&self, path: &Path) -> Result<LocCount, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        let language = LocLanguage::from_path(path);
        Ok(self.count_content_with_language(&content, Some(language)))
    }

    /// Count lines in file content (pure function) - backward compatible
    ///
    /// Uses conservative comment detection (Unknown language).
    /// For accurate counting, use `count_content_with_language` instead.
    pub fn count_content(&self, content: &str) -> LocCount {
        self.count_content_with_language(content, None)
    }

    /// Count lines in file content with language-aware comment detection (Spec 201)
    ///
    /// This method correctly handles:
    /// - Multi-line block comments (`/* ... */`)
    /// - Nested block comments in Rust
    /// - Rust attributes (`#[...]`) as code, not comments
    /// - Python `#` comments
    pub fn count_content_with_language(
        &self,
        content: &str,
        language: Option<LocLanguage>,
    ) -> LocCount {
        let language = language.unwrap_or_default();
        let physical_lines = content.lines().count();
        let mut code_lines = 0;
        let mut comment_lines = 0;
        let mut blank_lines = 0;

        // State for multi-line comment tracking (Spec 201)
        let mut comment_state = CommentState::default();

        for line in content.lines() {
            let trimmed = line.trim();

            // Classify the line with multi-line comment state tracking
            let line_type = classify_line(trimmed, &mut comment_state, language);

            match line_type {
                LineType::Blank => blank_lines += 1,
                LineType::Comment => comment_lines += 1,
                LineType::Code => code_lines += 1,
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

/// Line type classification for LOC counting
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LineType {
    Blank,
    Comment,
    Code,
}

/// State tracking for multi-line comment detection (Spec 201)
#[derive(Clone, Debug, Default)]
struct CommentState {
    /// Whether we're inside a block comment
    in_block_comment: bool,
    /// Nesting depth for Rust's nested block comments (/* /* */ */)
    block_depth: usize,
}

/// Classify a line considering multi-line comment state (Spec 201)
///
/// This function handles:
/// - Single-line comments (`//`, `#` for Python)
/// - Block comments (`/* ... */`) with state tracking
/// - Nested block comments for Rust
/// - Rust attributes (`#[...]`) as code, not comments
fn classify_line(trimmed: &str, state: &mut CommentState, language: LocLanguage) -> LineType {
    if trimmed.is_empty() {
        return LineType::Blank;
    }

    // If we're inside a block comment, check for exit
    if state.in_block_comment {
        update_block_comment_state(trimmed, state, language);
        return LineType::Comment;
    }

    // Check for block comment start
    if let Some(start_idx) = trimmed.find("/*") {
        // Check if there's code before the comment
        let before_comment = &trimmed[..start_idx];
        let has_code_before = !before_comment.trim().is_empty()
            && !is_single_line_comment_start(before_comment.trim(), language);

        // Track entering block comment
        enter_block_comment(trimmed, start_idx, state, language);

        // If there was code before the comment, it's a code line
        if has_code_before {
            return LineType::Code;
        }

        // If the block comment closes on the same line with no code after, it's a comment
        if !state.in_block_comment {
            // Find where the comment ends
            if let Some(end_idx) = trimmed.rfind("*/") {
                let after_comment = &trimmed[end_idx + 2..];
                if after_comment.trim().is_empty() {
                    return LineType::Comment;
                }
                // There's code after the comment
                return LineType::Code;
            }
        }

        return LineType::Comment;
    }

    // Check for single-line comments
    if is_single_line_comment(trimmed, language) {
        return LineType::Comment;
    }

    LineType::Code
}

/// Check if a line starts with a single-line comment marker
fn is_single_line_comment_start(trimmed: &str, language: LocLanguage) -> bool {
    // Rust/JS/TS line comments
    if trimmed.starts_with("//") {
        return true;
    }

    // Python comments (but not Rust attributes!)
    if trimmed.starts_with('#') {
        match language {
            LocLanguage::Rust => {
                // Rust: #[...] and #![...] are attributes (code), not comments
                // Only # followed by something other than [ or ! is considered a comment
                // But in Rust, bare # is typically part of raw strings, not comments
                // So we treat # as NOT a comment in Rust
                false
            }
            LocLanguage::Python => true,
            // For Unknown/JS/TS, # is not a standard comment marker
            _ => false,
        }
    } else {
        false
    }
}

/// Check if a trimmed line is primarily a single-line comment
fn is_single_line_comment(trimmed: &str, language: LocLanguage) -> bool {
    // Check for line comment start
    if is_single_line_comment_start(trimmed, language) {
        return true;
    }

    // Lines starting with * (inside block comments - but we handle state separately)
    // This handles the case where a line in a multi-line comment starts with *
    // But since we track state, this is mainly for lines like:
    //   * This is documentation
    // which are formatted comment continuations
    if trimmed.starts_with('*') && !trimmed.starts_with("*/") && !trimmed.starts_with("**") {
        // Could be a comment continuation, but without state tracking we can't be sure
        // With state tracking enabled, we handle this in the block comment path
        // For single-line check, be conservative - don't count as comment
        return false;
    }

    false
}

/// Enter block comment and update state
fn enter_block_comment(
    trimmed: &str,
    start_idx: usize,
    state: &mut CommentState,
    language: LocLanguage,
) {
    // Start from after the first /*
    let mut idx = start_idx + 2;
    let bytes = trimmed.as_bytes();
    let mut depth = 1;

    while idx < bytes.len() {
        if idx + 1 < bytes.len() {
            // Check for nested /* in Rust
            if bytes[idx] == b'/' && bytes[idx + 1] == b'*' {
                if language == LocLanguage::Rust {
                    depth += 1;
                }
                idx += 2;
                continue;
            }
            // Check for closing */
            if bytes[idx] == b'*' && bytes[idx + 1] == b'/' {
                depth -= 1;
                if depth == 0 {
                    state.in_block_comment = false;
                    state.block_depth = 0;
                    return;
                }
                idx += 2;
                continue;
            }
        }
        idx += 1;
    }

    // Didn't find matching close - we're in a block comment
    state.in_block_comment = true;
    state.block_depth = depth;
}

/// Update block comment state when inside a block comment
fn update_block_comment_state(trimmed: &str, state: &mut CommentState, language: LocLanguage) {
    let bytes = trimmed.as_bytes();
    let mut idx = 0;
    let mut depth = state.block_depth;

    while idx < bytes.len() {
        if idx + 1 < bytes.len() {
            // Check for nested /* in Rust
            if bytes[idx] == b'/' && bytes[idx + 1] == b'*' {
                if language == LocLanguage::Rust {
                    depth += 1;
                }
                idx += 2;
                continue;
            }
            // Check for closing */
            if bytes[idx] == b'*' && bytes[idx + 1] == b'/' {
                depth -= 1;
                if depth == 0 {
                    state.in_block_comment = false;
                    state.block_depth = 0;
                    return;
                }
                idx += 2;
                continue;
            }
        }
        idx += 1;
    }

    // Still in block comment
    state.block_depth = depth;
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
    fn test_single_line_comments() {
        let counter = LocCounter::default();

        // Test Rust line comments
        let rust_content = "// This is a comment\nlet x = 5;";
        let count = counter.count_content_with_language(rust_content, Some(LocLanguage::Rust));
        assert_eq!(count.comment_lines, 1);
        assert_eq!(count.code_lines, 1);

        // Test Python comments
        let python_content = "# This is a comment\nx = 5";
        let count = counter.count_content_with_language(python_content, Some(LocLanguage::Python));
        assert_eq!(count.comment_lines, 1);
        assert_eq!(count.code_lines, 1);
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

    // ===== Spec 201: Multi-line Comment Tests =====

    #[test]
    fn test_multiline_block_comment() {
        let counter = LocCounter::default();
        let code = "/* comment\nstill comment\nend */\ncode";
        let count = counter.count_content_with_language(code, Some(LocLanguage::Rust));
        assert_eq!(count.comment_lines, 3);
        assert_eq!(count.code_lines, 1);
        assert_eq!(count.physical_lines, 4);
    }

    #[test]
    fn test_multiline_block_comment_no_asterisk_prefix() {
        // Lines in block comments that don't start with * should still be comments
        let counter = LocCounter::default();
        let code = "/*\nThis line starts with text, not *\nso it should still be a comment\n*/\nfn main() {}";
        let count = counter.count_content_with_language(code, Some(LocLanguage::Rust));
        assert_eq!(count.comment_lines, 4);
        assert_eq!(count.code_lines, 1);
    }

    #[test]
    fn test_nested_block_comments_rust() {
        // Rust supports nested block comments: /* /* */ */
        let counter = LocCounter::default();
        let code = "/* outer /* inner */ still outer */\ncode";
        let count = counter.count_content_with_language(code, Some(LocLanguage::Rust));
        assert_eq!(count.comment_lines, 1);
        assert_eq!(count.code_lines, 1);
    }

    #[test]
    fn test_nested_block_comments_multiline_rust() {
        let counter = LocCounter::default();
        let code = "/*\n/* nested\n*/\nstill in outer\n*/\ncode";
        let count = counter.count_content_with_language(code, Some(LocLanguage::Rust));
        assert_eq!(count.comment_lines, 5);
        assert_eq!(count.code_lines, 1);
    }

    // ===== Spec 201: Rust Attribute Tests =====

    #[test]
    fn test_rust_attributes_are_code() {
        let counter = LocCounter::default();
        let code = "#[derive(Debug)]\nstruct Foo;";
        let count = counter.count_content_with_language(code, Some(LocLanguage::Rust));
        assert_eq!(count.code_lines, 2);
        assert_eq!(count.comment_lines, 0);
    }

    #[test]
    fn test_rust_inner_attributes_are_code() {
        let counter = LocCounter::default();
        let code = "#![allow(unused)]\nfn main() {}";
        let count = counter.count_content_with_language(code, Some(LocLanguage::Rust));
        assert_eq!(count.code_lines, 2);
        assert_eq!(count.comment_lines, 0);
    }

    #[test]
    fn test_rust_cfg_attributes_are_code() {
        let counter = LocCounter::default();
        let code = "#[cfg(test)]\nmod tests {\n    #[test]\n    fn test_foo() {}\n}";
        let count = counter.count_content_with_language(code, Some(LocLanguage::Rust));
        assert_eq!(count.code_lines, 5);
        assert_eq!(count.comment_lines, 0);
    }

    #[test]
    fn test_python_hash_still_comments() {
        // Python # should still be comments
        let counter = LocCounter::default();
        let code = "# This is a Python comment\ndef foo():\n    pass";
        let count = counter.count_content_with_language(code, Some(LocLanguage::Python));
        assert_eq!(count.comment_lines, 1);
        assert_eq!(count.code_lines, 2);
    }

    // ===== Spec 201: LOC Invariant Tests =====

    #[test]
    fn test_loc_invariant_simple() {
        let counter = LocCounter::default();
        let code = "fn main() {\n    // comment\n\n    let x = 5;\n}";
        let count = counter.count_content_with_language(code, Some(LocLanguage::Rust));
        assert_eq!(
            count.physical_lines,
            count.code_lines + count.comment_lines + count.blank_lines,
            "Invariant: physical = code + comment + blank"
        );
    }

    #[test]
    fn test_loc_invariant_multiline_comments() {
        let counter = LocCounter::default();
        let code = "/*\nMulti\nline\ncomment\n*/\nfn foo() {}\n\n// single\nlet x = 1;";
        let count = counter.count_content_with_language(code, Some(LocLanguage::Rust));
        assert_eq!(
            count.physical_lines,
            count.code_lines + count.comment_lines + count.blank_lines,
            "Invariant: physical = code + comment + blank"
        );
    }

    #[test]
    fn test_loc_invariant_mixed_content() {
        let counter = LocCounter::default();
        let code = r#"
// Header comment
#[derive(Debug)]
struct Foo {
    /* inline */ x: i32,
}

/*
Block
comment
*/
fn main() {
    println!("hello");
}
"#;
        let count = counter.count_content_with_language(code, Some(LocLanguage::Rust));
        assert_eq!(
            count.physical_lines,
            count.code_lines + count.comment_lines + count.blank_lines,
            "Invariant: physical = code + comment + blank"
        );
    }

    // ===== Spec 201: Language Detection Tests =====

    #[test]
    fn test_language_from_extension() {
        assert_eq!(
            LocLanguage::from_path(Path::new("foo.rs")),
            LocLanguage::Rust
        );
        assert_eq!(
            LocLanguage::from_path(Path::new("bar.py")),
            LocLanguage::Python
        );
        assert_eq!(
            LocLanguage::from_path(Path::new("baz.js")),
            LocLanguage::JavaScript
        );
        assert_eq!(
            LocLanguage::from_path(Path::new("qux.ts")),
            LocLanguage::TypeScript
        );
        assert_eq!(
            LocLanguage::from_path(Path::new("file.tsx")),
            LocLanguage::TypeScript
        );
        assert_eq!(
            LocLanguage::from_path(Path::new("unknown.xyz")),
            LocLanguage::Unknown
        );
    }

    #[test]
    fn test_inline_block_comment() {
        let counter = LocCounter::default();
        let code = "let x = /* inline */ 5;";
        let count = counter.count_content_with_language(code, Some(LocLanguage::Rust));
        // Line has code before and after comment, so it's code
        assert_eq!(count.code_lines, 1);
        assert_eq!(count.comment_lines, 0);
    }

    #[test]
    fn test_block_comment_single_line() {
        let counter = LocCounter::default();
        let code = "/* just a comment */";
        let count = counter.count_content_with_language(code, Some(LocLanguage::Rust));
        assert_eq!(count.comment_lines, 1);
        assert_eq!(count.code_lines, 0);
    }

    #[test]
    fn test_code_before_block_comment() {
        let counter = LocCounter::default();
        let code = "let x = 5; /* comment";
        let count = counter.count_content_with_language(code, Some(LocLanguage::Rust));
        // Has code before comment, so it's code
        assert_eq!(count.code_lines, 1);
        assert_eq!(count.comment_lines, 0);
    }

    #[test]
    fn test_doc_comments_are_comments() {
        let counter = LocCounter::default();
        let code = "/// This is a doc comment\n//! Module doc\nfn foo() {}";
        let count = counter.count_content_with_language(code, Some(LocLanguage::Rust));
        assert_eq!(count.comment_lines, 2);
        assert_eq!(count.code_lines, 1);
    }
}
