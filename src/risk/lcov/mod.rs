//! LCOV coverage data parsing and querying.
//!
//! This module provides functionality for parsing LCOV coverage files and
//! querying coverage information. It follows the Stillwater philosophy of
//! separating pure logic from I/O operations.
//!
//! # Module Structure
//!
//! The module is organized into focused submodules:
//!
//! - [`types`] - Core data structures (pure data)
//! - [`demangle`] - Rust symbol demangling (pure functions)
//! - [`normalize`] - Function name normalization (pure functions)
//! - [`diagnostics`] - Debug statistics (boundary effects)
//! - [`handlers`] - Pure record handlers
//! - [`coverage`] - Coverage calculation (pure functions)
//! - [`parser`] - LCOV file parsing (I/O boundary)
//! - [`query`] - LcovData query methods
//!
//! # Architecture
//!
//! ```text
//!                     types.rs (foundation)
//!                        ↑
//!           ┌────────────┼────────────┐
//!           ↓            ↓            ↓
//!     demangle.rs   normalize.rs   diagnostics.rs
//!           ↓            ↓
//!           └────────────┼────────────┐
//!                        ↓            ↓
//!                 handlers.rs    coverage.rs
//!                        ↓
//!                     parser.rs (I/O boundary)
//!                        ↓
//!                     query.rs (uses types + diagnostics)
//!                        ↓
//!                     mod.rs (composition)
//! ```
//!
//! # Stillwater Philosophy
//!
//! **Pure Core (Still Water):**
//! - `types.rs` - Immutable data definitions
//! - `demangle.rs` - String transformations
//! - `normalize.rs` - Name normalization
//! - `handlers.rs` - State transformations
//! - `coverage.rs` - Coverage calculations
//!
//! **Imperative Shell (Flowing Water):**
//! - `parser.rs` - File I/O, iteration
//! - `diagnostics.rs` - Global statistics (side effect)
//!
//! # Quick Start
//!
//! ```ignore
//! use std::path::Path;
//! use debtmap::risk::lcov::{parse_lcov_file, LcovData};
//!
//! // Parse coverage file
//! let data = parse_lcov_file(Path::new("coverage.info"))?;
//!
//! // Query coverage
//! let coverage = data.get_function_coverage(
//!     Path::new("src/lib.rs"),
//!     "my_function",
//! );
//!
//! println!("Coverage: {:?}", coverage);
//! println!("Overall: {:.1}%", data.get_overall_coverage());
//! ```
//!
//! # Progress Reporting
//!
//! For long-running parsing operations, use the callback API:
//!
//! ```ignore
//! use debtmap::risk::lcov::{parse_lcov_file_with_callback, CoverageProgress};
//!
//! let data = parse_lcov_file_with_callback(
//!     Path::new("coverage.info"),
//!     |progress| {
//!         match progress {
//!             CoverageProgress::Parsing { current, total } => {
//!                 println!("Processing file {} of {}", current, total);
//!             }
//!             _ => {}
//!         }
//!     }
//! )?;
//! ```

// Module declarations
pub mod coverage;
pub mod demangle;
pub mod diagnostics;
pub mod handlers;
pub mod normalize;
pub mod parser;
pub mod query;
pub mod types;

// Re-exports for backward compatibility and convenience
pub use diagnostics::print_coverage_statistics;
pub use normalize::{normalize_demangled_name, strip_trailing_generics};
pub use parser::{parse_lcov_file, parse_lcov_file_with_callback, parse_lcov_file_with_progress};
pub use types::{CoverageProgress, FunctionCoverage, LcovData, NormalizedFunctionName};

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    /// Integration test for the full LCOV parsing pipeline
    #[test]
    fn test_full_parsing_pipeline() {
        let lcov_content = r#"TN:
SF:/path/to/file.rs
FN:10,test_function
FNDA:5,test_function
DA:10,5
DA:11,5
LF:2
LH:2
end_of_record
"#;

        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp.path()).unwrap();

        assert_eq!(data.total_lines, 2);
        assert_eq!(data.lines_hit, 2);
        assert!(data
            .get_function_coverage(std::path::Path::new("/path/to/file.rs"), "test_function")
            .is_some());
    }

    /// Test backward compatibility of re-exports
    #[test]
    fn test_reexports_available() {
        // These should all be accessible from the module root
        let _ = LcovData::new();
        let _ = CoverageProgress::Initializing;
        let _ = NormalizedFunctionName::simple("test");
    }

    /// Test function name normalization via public API
    #[test]
    fn test_normalize_function_name_public_api() {
        let result = normalize_demangled_name("HashMap<K,V>::insert");
        assert_eq!(result.full_path, "HashMap::insert");
        assert_eq!(result.method_name, "insert");
    }

    /// Test strip_trailing_generics via public API
    #[test]
    fn test_strip_generics_public_api() {
        let result = strip_trailing_generics("method::<T>");
        assert_eq!(result, "method");
    }

    /// Property-based test for normalization idempotence
    #[test]
    fn test_normalize_idempotent() {
        let inputs = vec![
            "simple_function",
            "Module::function",
            "HashMap<K,V>::insert",
            "<Type>::method",
            "method::<T>",
        ];

        for input in inputs {
            let result1 = normalize_demangled_name(input);
            let result2 = normalize_demangled_name(&result1.full_path);

            // Normalizing an already-normalized name should be stable
            assert_eq!(
                result1.method_name, result2.method_name,
                "Method name changed for: {}",
                input
            );
        }
    }

    /// Test coverage percentage calculation
    #[test]
    fn test_coverage_percentage_calculation() {
        let lcov_content = r#"TN:
SF:/path/to/file.rs
FN:10,fully_covered
FN:20,partially_covered
FN:30,not_covered
FNDA:10,fully_covered
FNDA:5,partially_covered
FNDA:0,not_covered
DA:10,10
DA:11,10
DA:12,10
DA:20,5
DA:21,5
DA:22,0
DA:23,0
DA:30,0
DA:31,0
DA:32,0
LF:10
LH:5
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();
        let file_path = PathBuf::from("/path/to/file.rs");

        // Test fully covered function (100%)
        let coverage = data.get_function_coverage(&file_path, "fully_covered");
        assert_eq!(coverage, Some(1.0));

        // Test partially covered function (50%)
        let coverage = data.get_function_coverage(&file_path, "partially_covered");
        assert_eq!(coverage, Some(0.5));

        // Test uncovered function (0%)
        let coverage = data.get_function_coverage(&file_path, "not_covered");
        assert_eq!(coverage, Some(0.0));
    }

    /// Test function name demangling in parsing
    #[test]
    fn test_demangling_in_parsing() {
        let lcov_content = r#"TN:
SF:/path/to/file.rs
FN:18,_RNvMNtNtNtCs9MAeJIiYlOV_7debtmap8analysis11attribution14change_trackerNtB2_13ChangeTracker13track_changes
FNDA:5,_RNvMNtNtNtCs9MAeJIiYlOV_7debtmap8analysis11attribution14change_trackerNtB2_13ChangeTracker13track_changes
DA:18,5
LF:1
LH:1
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();
        let file_path = PathBuf::from("/path/to/file.rs");

        let funcs = &data.functions[&file_path];
        assert_eq!(funcs.len(), 1);

        // Function name should be demangled and normalized
        assert!(
            funcs[0].name.contains("ChangeTracker") || funcs[0].name.contains("track_changes"),
            "Expected demangled name, got: {}",
            funcs[0].name
        );
    }
}
