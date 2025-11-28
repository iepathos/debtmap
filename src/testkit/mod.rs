//! Testing infrastructure for debtmap using stillwater's MockEnv.
//!
//! This module provides testing utilities that enable fast, deterministic tests
//! without real I/O operations. It includes:
//!
//! - **[`DebtmapTestEnv`]**: Mock environment implementing [`AnalysisEnv`](crate::env::AnalysisEnv)
//!   with in-memory file system, coverage data, and cache
//! - **Assertion macros**: Extended assertions for Result and Validation types
//! - **Test helpers**: Factory functions for creating test ASTs, configs, and coverage
//! - **Property testing**: Generators for proptest-based testing
//!
//! # Why Use This Module?
//!
//! Traditional tests using TempDir have several issues:
//! - **Slow**: File I/O adds 50-100ms per test
//! - **Brittle**: File system state can leak between tests
//! - **Complex setup**: Requires managing temp directories and cleanup
//!
//! With `DebtmapTestEnv`, tests are:
//! - **Fast**: In-memory operations complete in microseconds
//! - **Isolated**: Each test gets fresh, independent state
//! - **Simple**: Fluent API for setting up test scenarios
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use debtmap::testkit::{DebtmapTestEnv, assert_result_ok};
//! use debtmap::env::AnalysisEnv;
//!
//! #[test]
//! fn test_file_reading() {
//!     let env = DebtmapTestEnv::new()
//!         .with_file("test.rs", "fn main() {}");
//!
//!     let content = env.file_system().read_to_string("test.rs".as_ref());
//!     let content = assert_result_ok!(content);
//!     assert!(content.contains("fn main"));
//! }
//! ```
//!
//! # Migration from TempDir
//!
//! ## Before (slow, ~50ms)
//!
//! ```rust,ignore
//! #[test]
//! fn test_analyze_file() {
//!     let temp_dir = TempDir::new().unwrap();
//!     let test_file = temp_dir.path().join("test.rs");
//!     fs::write(&test_file, "fn main() {}").unwrap();
//!
//!     let result = analyze_file(&test_file).unwrap();
//!     assert_eq!(result.functions.len(), 1);
//! }
//! ```
//!
//! ## After (fast, ~0.5ms)
//!
//! ```rust,ignore
//! use debtmap::testkit::{DebtmapTestEnv, assert_result_ok};
//!
//! #[test]
//! fn test_analyze_file() {
//!     let env = DebtmapTestEnv::new()
//!         .with_file("test.rs", "fn main() {}");
//!
//!     let result = analyze_file_with_env("test.rs", &env);
//!     let result = assert_result_ok!(result);
//!     assert_eq!(result.functions.len(), 1);
//! }
//! ```
//!
//! # Spec 200 Implementation
//!
//! This module implements Specification 200: Testing Infrastructure with MockEnv.
//! It completes the stillwater integration series (Specs 195-200).

pub mod assertions;
pub mod helpers;
pub mod mock_env;

// proptest_generators is only available in tests (proptest is a dev-dependency)
#[cfg(test)]
pub mod proptest_generators;

// Re-export main types
// Note: Assertion macros are exported at crate root via #[macro_export]
pub use helpers::{
    create_complex_project, create_coverage_data, create_multi_function_ast, create_nested_ast,
    create_test_ast, create_test_coverage, create_test_project, parse_test_code, ConfigBuilder,
};
pub use mock_env::DebtmapTestEnv;
