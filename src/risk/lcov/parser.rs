//! LCOV file parser - the imperative shell.
//!
//! This module provides the I/O operations for parsing LCOV coverage files.
//! Following Stillwater philosophy, this is the "imperative shell" that
//! reads files and dispatches to pure handlers.
//!
//! # Stillwater Philosophy
//!
//! This module is the boundary between I/O and pure computation:
//! - Opens and reads LCOV files (I/O)
//! - Iterates over records (I/O)
//! - Dispatches to pure handlers (delegation)
//! - Reports progress (side effect)
//!
//! The actual record processing is delegated to the pure handlers in
//! the `handlers` module.
//!
//! # Example
//!
//! ```ignore
//! use std::path::Path;
//! use debtmap::risk::lcov::{parse_lcov_file, CoverageProgress};
//!
//! // Simple parsing
//! let data = parse_lcov_file(Path::new("coverage.info"))?;
//!
//! // Parsing with progress callback
//! let data = parse_lcov_file_with_callback(Path::new("coverage.info"), |progress| {
//!     match progress {
//!         CoverageProgress::Parsing { current, .. } => {
//!             println!("Processing file {}", current);
//!         }
//!         _ => {}
//!     }
//! })?;
//! ```

use super::handlers::{
    handle_end_of_record, handle_function_data, handle_function_name, handle_incomplete_file,
    handle_line_data, handle_lines_found, handle_lines_hit, handle_source_file, LcovParserState,
};
use super::types::{CoverageProgress, LcovData};
use anyhow::{Context, Result};
use indicatif::ProgressBar;
use std::path::Path;

/// Parse LCOV file.
///
/// Simple API that parses an LCOV file without progress reporting.
///
/// # Arguments
///
/// * `path` - Path to the LCOV file
///
/// # Returns
///
/// Parsed `LcovData` containing coverage information.
///
/// # Errors
///
/// Returns an error if the file cannot be opened or parsed.
///
/// # Example
///
/// ```ignore
/// use std::path::Path;
/// use debtmap::risk::lcov::parse_lcov_file;
///
/// let data = parse_lcov_file(Path::new("coverage.info"))?;
/// println!("Total lines: {}", data.total_lines);
/// ```
pub fn parse_lcov_file(path: &Path) -> Result<LcovData> {
    parse_lcov_file_with_progress(path, &ProgressBar::hidden())
}

/// Parse LCOV file with progress callback.
///
/// Parses an LCOV file and calls the provided callback with progress updates.
/// This is the primary parsing API that supports custom progress handling.
///
/// # Arguments
///
/// * `path` - Path to the LCOV file
/// * `progress_callback` - Function called with progress updates
///
/// # Returns
///
/// Parsed `LcovData` containing coverage information.
///
/// # Errors
///
/// Returns an error if the file cannot be opened or parsed.
///
/// # Example
///
/// ```ignore
/// use debtmap::risk::lcov::{parse_lcov_file_with_callback, CoverageProgress};
///
/// let data = parse_lcov_file_with_callback(Path::new("coverage.info"), |progress| {
///     if let CoverageProgress::Parsing { current, total } = progress {
///         println!("Progress: {}/{}", current, total);
///     }
/// })?;
/// ```
pub fn parse_lcov_file_with_callback<F>(path: &Path, mut progress_callback: F) -> Result<LcovData>
where
    F: FnMut(CoverageProgress),
{
    use lcov::{Reader, Record};

    progress_callback(CoverageProgress::Initializing);

    let reader = Reader::open_file(path)
        .with_context(|| format!("Failed to open LCOV file: {}", path.display()))?;

    let mut state = LcovParserState::new();

    progress_callback(CoverageProgress::Parsing {
        current: 0,
        total: 0,
    });

    // Imperative shell: dispatch to pure handlers
    for record in reader {
        let record = record.with_context(|| "Failed to parse LCOV record")?;

        match record {
            Record::SourceFile { path } => handle_source_file(&mut state, path),
            Record::FunctionName { start_line, name } => {
                handle_function_name(&mut state, start_line, name)
            }
            Record::FunctionData { name, count } => handle_function_data(&mut state, name, count),
            Record::LineData { line, count, .. } => handle_line_data(&mut state, line, count),
            Record::LinesFound { found } => handle_lines_found(&mut state, found),
            Record::LinesHit { hit } => handle_lines_hit(&mut state, hit),
            Record::EndOfRecord => {
                handle_end_of_record(&mut state);
                // Throttle: update every 10 files
                if state.file_count % 10 == 0 {
                    progress_callback(CoverageProgress::Parsing {
                        current: state.file_count,
                        total: state.file_count,
                    });
                }
            }
            _ => {} // Ignore other record types
        }
    }

    // Handle case where file doesn't end with EndOfRecord
    handle_incomplete_file(&mut state);

    progress_callback(CoverageProgress::ComputingStats);
    state.data.build_index();
    progress_callback(CoverageProgress::Complete);

    Ok(state.data)
}

/// Legacy API: Parse LCOV file with ProgressBar.
///
/// Parses an LCOV file using an indicatif ProgressBar for progress display.
/// This is maintained for backward compatibility with existing code.
///
/// # Arguments
///
/// * `path` - Path to the LCOV file
/// * `progress` - ProgressBar to update during parsing
///
/// # Returns
///
/// Parsed `LcovData` containing coverage information.
///
/// # Errors
///
/// Returns an error if the file cannot be opened or parsed.
pub fn parse_lcov_file_with_progress(path: &Path, progress: &ProgressBar) -> Result<LcovData> {
    use lcov::{Reader, Record};

    progress.set_message("Loading coverage data");

    let reader = Reader::open_file(path)
        .with_context(|| format!("Failed to open LCOV file: {}", path.display()))?;

    let mut state = LcovParserState::new();

    // Imperative shell: dispatch to pure handlers
    for record in reader {
        let record = record.with_context(|| "Failed to parse LCOV record")?;

        match record {
            Record::SourceFile { path } => handle_source_file(&mut state, path),
            Record::FunctionName { start_line, name } => {
                handle_function_name(&mut state, start_line, name)
            }
            Record::FunctionData { name, count } => handle_function_data(&mut state, name, count),
            Record::LineData { line, count, .. } => handle_line_data(&mut state, line, count),
            Record::LinesFound { found } => handle_lines_found(&mut state, found),
            Record::LinesHit { hit } => handle_lines_hit(&mut state, hit),
            Record::EndOfRecord => {
                handle_end_of_record(&mut state);
                progress.set_position(state.file_count as u64);
            }
            _ => {} // Ignore other record types
        }
    }

    // Handle case where file doesn't end with EndOfRecord
    handle_incomplete_file(&mut state);

    progress.set_message("Building coverage index");
    state.data.build_index();
    progress.finish_and_clear();

    Ok(state.data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_lcov_file() {
        let lcov_content = r#"TN:
SF:/path/to/file.rs
FN:10,test_function
FNDA:5,test_function
FNF:1
FNH:1
DA:10,5
DA:11,5
DA:12,0
LF:3
LH:2
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();

        assert_eq!(data.total_lines, 3);
        assert_eq!(data.lines_hit, 2);

        let file_path = PathBuf::from("/path/to/file.rs");
        assert!(data.functions.contains_key(&file_path));

        let funcs = &data.functions[&file_path];
        assert_eq!(funcs.len(), 1);
        assert_eq!(funcs[0].name, "test_function");
        assert_eq!(funcs[0].execution_count, 5);
    }

    #[test]
    fn test_parse_lcov_file_multiple_files() {
        let lcov_content = r#"TN:
SF:/path/to/file1.rs
FN:10,func1
FNDA:5,func1
DA:10,5
LF:1
LH:1
end_of_record
SF:/path/to/file2.rs
FN:20,func2
FNDA:0,func2
DA:20,0
LF:1
LH:0
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();

        assert_eq!(data.total_lines, 2);
        assert_eq!(data.lines_hit, 1);
        assert_eq!(data.functions.len(), 2);
    }

    #[test]
    fn test_parse_lcov_file_with_callback() {
        let lcov_content = r#"TN:
SF:/path/to/file.rs
FN:10,test_function
FNDA:5,test_function
DA:10,5
LF:1
LH:1
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let mut progress_states = Vec::new();
        let data = parse_lcov_file_with_callback(temp_file.path(), |progress| {
            progress_states.push(format!("{:?}", progress));
        })
        .unwrap();

        assert_eq!(data.total_lines, 1);

        // Should have received progress updates
        assert!(progress_states.iter().any(|s| s.contains("Initializing")));
        assert!(progress_states.iter().any(|s| s.contains("Complete")));
    }

    #[test]
    fn test_parse_lcov_file_empty() {
        let lcov_content = r#"TN:
SF:/path/to/empty.rs
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();

        assert_eq!(data.get_overall_coverage(), 0.0);
        assert_eq!(data.functions.len(), 0);
    }

    #[test]
    fn test_parse_lcov_file_consolidates_duplicates() {
        // Test that duplicate mangled functions with different crate hashes
        // are consolidated into a single entry
        let lcov_content = r#"TN:
SF:/path/to/file.rs
FN:18,_RNvMNtNtNtCs9MAeJIiYlOV_7debtmap8analysis11attribution14change_trackerNtB2_13ChangeTracker13track_changes
FNDA:5,_RNvMNtNtNtCs9MAeJIiYlOV_7debtmap8analysis11attribution14change_trackerNtB2_13ChangeTracker13track_changes
FN:18,_RNvMNtNtNtCs5ZpFxq88JTF_7debtmap8analysis11attribution14change_trackerNtB2_13ChangeTracker13track_changes
FNDA:3,_RNvMNtNtNtCs5ZpFxq88JTF_7debtmap8analysis11attribution14change_trackerNtB2_13ChangeTracker13track_changes
DA:18,5
DA:19,5
LF:2
LH:2
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();
        let file_path = PathBuf::from("/path/to/file.rs");

        // Should consolidate to single function
        let funcs = &data.functions[&file_path];
        assert_eq!(funcs.len(), 1, "Expected 1 function after consolidation");

        // Should keep max execution count (5 vs 3)
        assert_eq!(funcs[0].execution_count, 5, "Expected max execution count");

        // Function name should be demangled
        assert!(
            funcs[0].name.contains("ChangeTracker") || funcs[0].name.contains("track_changes"),
            "Expected demangled name, got: {}",
            funcs[0].name
        );
    }

    #[test]
    fn test_parse_lcov_file_without_end_of_record() {
        // Test that data is preserved even without end_of_record
        let lcov_content = r#"TN:
SF:/path/to/incomplete.rs
FN:10,orphan_func
FNDA:2,orphan_func
DA:10,2
LF:1
LH:1
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();
        let file_path = PathBuf::from("/path/to/incomplete.rs");

        assert!(data.functions.contains_key(&file_path));
        let funcs = &data.functions[&file_path];
        assert_eq!(funcs.len(), 1);
        assert_eq!(funcs[0].name, "orphan_func");
    }
}
