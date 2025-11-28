//! Test helper functions for creating test data.
//!
//! This module provides factory functions and builders for creating common
//! test objects like ASTs, configurations, and coverage data.
//!
//! # Quick Reference
//!
//! | Helper | Purpose |
//! |--------|---------|
//! | [`parse_test_code`] | Parse Rust code string to AST |
//! | [`create_test_ast`] | Create synthetic AST with N if-statements |
//! | [`ConfigBuilder`] | Build test configurations fluently |
//! | [`create_test_coverage`] | Create coverage data with specific values |
//! | [`create_test_project`] | Create realistic project file structure |
//!
//! # Examples
//!
//! ## Creating Test ASTs
//!
//! ```rust,ignore
//! use debtmap::testkit::helpers::{parse_test_code, create_test_ast};
//!
//! // Parse specific code
//! let ast = parse_test_code("fn foo() { if x { } }");
//!
//! // Create synthetic AST with 5 if-statements (complexity ~6)
//! let ast = create_test_ast(5);
//! ```
//!
//! ## Creating Test Configs
//!
//! ```rust,ignore
//! use debtmap::testkit::helpers::ConfigBuilder;
//!
//! let config = ConfigBuilder::new()
//!     .complexity_threshold(15.0)
//!     .coverage_threshold(80.0)
//!     .build();
//! ```
//!
//! ## Creating Test Coverage
//!
//! ```rust,ignore
//! use debtmap::testkit::helpers::create_test_coverage;
//!
//! // 80% coverage: 80 lines hit out of 100
//! let coverage = create_test_coverage(100, 80);
//! ```

use crate::config::{DebtmapConfig, ThresholdsConfig};
use crate::io::traits::{CoverageData, FileCoverage};
use crate::testkit::DebtmapTestEnv;
use std::path::PathBuf;

/// Parse inline Rust code into an AST.
///
/// This is a convenience wrapper around `syn::parse_str` that panics
/// on parse errors with a helpful message.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::testkit::helpers::parse_test_code;
///
/// let ast = parse_test_code(r#"
///     fn example() {
///         if x { println!("x"); }
///     }
/// "#);
/// ```
///
/// # Panics
///
/// Panics if the code cannot be parsed as valid Rust.
pub fn parse_test_code(code: &str) -> syn::File {
    syn::parse_str(code).unwrap_or_else(|e| {
        panic!(
            "Failed to parse test code:\n{}\n\nError: {}",
            code.lines()
                .enumerate()
                .map(|(i, line)| format!("{:3} | {}", i + 1, line))
                .collect::<Vec<_>>()
                .join("\n"),
            e
        )
    })
}

/// Create a synthetic AST with a specific number of if-statements.
///
/// Useful for testing complexity calculations where you want
/// a predictable complexity value.
///
/// # Complexity Calculation
///
/// Cyclomatic complexity = 1 + number of decision points.
/// Each `if` adds 1 decision point, so:
/// - `create_test_ast(0)` has complexity 1
/// - `create_test_ast(5)` has complexity 6
/// - `create_test_ast(10)` has complexity 11
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::testkit::helpers::create_test_ast;
///
/// let ast = create_test_ast(5);
/// // ast contains: fn test_function() { if x0 {} if x1 {} ... if x4 {} }
/// ```
pub fn create_test_ast(if_count: u32) -> syn::File {
    let mut code = "fn test_function() {\n".to_string();

    for i in 0..if_count {
        code.push_str(&format!("    if x{} {{ }}\n", i));
    }

    code.push_str("}\n");

    parse_test_code(&code)
}

/// Create an AST with nested conditionals for testing nesting depth.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::testkit::helpers::create_nested_ast;
///
/// let ast = create_nested_ast(3);
/// // Creates: if x { if y { if z { } } }
/// ```
pub fn create_nested_ast(nesting_depth: u32) -> syn::File {
    let mut code = "fn test_function() {\n".to_string();

    for i in 0..nesting_depth {
        code.push_str(&"    ".repeat(i as usize + 1));
        code.push_str(&format!("if x{} {{\n", i));
    }

    // Add a statement at the deepest level
    code.push_str(&"    ".repeat(nesting_depth as usize + 1));
    code.push_str("println!(\"deep\");\n");

    // Close all braces
    for i in (0..nesting_depth).rev() {
        code.push_str(&"    ".repeat(i as usize + 1));
        code.push_str("}\n");
    }

    code.push_str("}\n");

    parse_test_code(&code)
}

/// Create an AST with multiple functions for testing multi-function analysis.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::testkit::helpers::create_multi_function_ast;
///
/// let ast = create_multi_function_ast(3);
/// // Creates: fn func_0() {} fn func_1() {} fn func_2() {}
/// ```
pub fn create_multi_function_ast(function_count: u32) -> syn::File {
    let mut code = String::new();

    for i in 0..function_count {
        code.push_str(&format!(
            "fn func_{}() {{ println!(\"func {}\"); }}\n",
            i, i
        ));
    }

    parse_test_code(&code)
}

/// Builder for creating test configurations.
///
/// Provides a fluent API for setting up `DebtmapConfig` with commonly
/// needed test values.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::testkit::helpers::ConfigBuilder;
///
/// let config = ConfigBuilder::new()
///     .complexity_threshold(15)
///     .max_function_length(200)
///     .ignore_patterns(vec!["tests/**/*".to_string()])
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct ConfigBuilder {
    config: DebtmapConfig,
}

impl ConfigBuilder {
    /// Create a new config builder with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the complexity threshold.
    pub fn complexity_threshold(mut self, threshold: u32) -> Self {
        let thresholds = self
            .config
            .thresholds
            .get_or_insert_with(ThresholdsConfig::default);
        thresholds.complexity = Some(threshold);
        self
    }

    /// Set the maximum function length threshold.
    pub fn max_function_length(mut self, length: usize) -> Self {
        let thresholds = self
            .config
            .thresholds
            .get_or_insert_with(ThresholdsConfig::default);
        thresholds.max_function_length = Some(length);
        self
    }

    /// Set the maximum file length threshold.
    pub fn max_file_length(mut self, length: usize) -> Self {
        let thresholds = self
            .config
            .thresholds
            .get_or_insert_with(ThresholdsConfig::default);
        thresholds.max_file_length = Some(length);
        self
    }

    /// Set the minimum debt score threshold.
    pub fn minimum_debt_score(mut self, score: f64) -> Self {
        let thresholds = self
            .config
            .thresholds
            .get_or_insert_with(ThresholdsConfig::default);
        thresholds.minimum_debt_score = Some(score);
        self
    }

    /// Set the minimum risk score threshold.
    pub fn minimum_risk_score(mut self, score: f64) -> Self {
        let thresholds = self
            .config
            .thresholds
            .get_or_insert_with(ThresholdsConfig::default);
        thresholds.minimum_risk_score = Some(score);
        self
    }

    /// Set ignore patterns.
    pub fn ignore_patterns(mut self, patterns: Vec<String>) -> Self {
        self.config.ignore = Some(crate::config::IgnoreConfig { patterns });
        self
    }

    /// Build the final configuration.
    pub fn build(self) -> DebtmapConfig {
        self.config
    }
}

/// Create coverage data with specific line and hit counts.
///
/// This creates synthetic coverage data where the first `hits` lines
/// are marked as hit, and the remaining lines are not hit.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::testkit::helpers::create_test_coverage;
///
/// // 80% coverage
/// let coverage = create_test_coverage(100, 80);
/// assert!((coverage.percentage - 80.0).abs() < 0.01);
/// ```
pub fn create_test_coverage(total_lines: usize, hit_lines: usize) -> FileCoverage {
    let mut fc = FileCoverage::new();

    for i in 1..=total_lines {
        fc.add_line(i, if i <= hit_lines { 1 } else { 0 });
    }

    fc
}

/// Create coverage data for a file with a specific percentage.
///
/// Returns a CoverageData instance with the file's coverage set.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::testkit::helpers::create_coverage_data;
///
/// let coverage = create_coverage_data("src/main.rs", 75.0);
/// let pct = coverage.get_file_coverage(Path::new("src/main.rs")).unwrap();
/// assert!((pct - 75.0).abs() < 1.0);
/// ```
pub fn create_coverage_data(path: impl Into<PathBuf>, percentage: f64) -> CoverageData {
    let mut data = CoverageData::new();
    let fc = create_test_coverage(100, percentage as usize);
    data.add_file_coverage(path.into(), fc);
    data
}

/// Create a realistic test project environment.
///
/// Returns a `DebtmapTestEnv` with a standard Rust project structure:
/// - `src/main.rs` with a main function
/// - `src/lib.rs` with a public function
/// - `src/utils.rs` with a helper function
/// - `tests/integration_test.rs` with a test
///
/// Also includes coverage data for each source file.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::testkit::helpers::create_test_project;
///
/// let env = create_test_project();
/// assert!(env.has_file("src/main.rs"));
/// assert!(env.has_file("src/lib.rs"));
/// ```
pub fn create_test_project() -> DebtmapTestEnv {
    DebtmapTestEnv::new()
        .with_files(vec![
            ("src/main.rs", "fn main() { println!(\"Hello\"); }"),
            ("src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }"),
            ("src/utils.rs", "pub fn helper() { /* ... */ }"),
            (
                "tests/integration_test.rs",
                "#[test] fn test_main() { assert!(true); }",
            ),
        ])
        .with_coverage_percentage("src/main.rs", 80.0)
        .with_coverage_percentage("src/lib.rs", 100.0)
        .with_coverage_percentage("src/utils.rs", 50.0)
        .with_config(DebtmapConfig::default())
}

/// Create a complex test project with various code patterns.
///
/// Returns a `DebtmapTestEnv` with files exhibiting different complexity levels:
/// - Simple functions (low complexity)
/// - Complex functions with nested conditionals (high complexity)
/// - Functions with loops
/// - Test files
///
/// Useful for integration testing of the analysis pipeline.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::testkit::helpers::create_complex_project;
///
/// let env = create_complex_project();
/// // Analyze and verify different complexity levels
/// ```
pub fn create_complex_project() -> DebtmapTestEnv {
    DebtmapTestEnv::new()
        .with_file(
            "src/simple.rs",
            r#"
fn simple_function() {
    println!("Hello");
}

fn another_simple() -> i32 {
    42
}
"#,
        )
        .with_file(
            "src/complex.rs",
            r#"
fn complex_function(x: i32, y: i32, z: bool) -> i32 {
    if x > 0 {
        if y > 0 {
            if z {
                return x + y;
            } else {
                return x - y;
            }
        } else {
            return x;
        }
    } else if y > 0 {
        return y;
    } else {
        return 0;
    }
}

fn with_loop(items: &[i32]) -> i32 {
    let mut sum = 0;
    for item in items {
        if *item > 0 {
            sum += item;
        }
    }
    sum
}
"#,
        )
        .with_file(
            "src/lib.rs",
            r#"
pub mod simple;
pub mod complex;
"#,
        )
        .with_coverage_percentage("src/simple.rs", 100.0)
        .with_coverage_percentage("src/complex.rs", 60.0)
        .with_config(ConfigBuilder::new().complexity_threshold(10).build())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_test_code() {
        let ast = parse_test_code("fn foo() {}");
        assert_eq!(ast.items.len(), 1);
    }

    #[test]
    fn test_create_test_ast() {
        let ast = create_test_ast(3);
        // Should have one function
        assert_eq!(ast.items.len(), 1);

        // The code should contain the if statements
        let code = quote::quote!(#ast).to_string();
        assert!(code.contains("if x0"));
        assert!(code.contains("if x1"));
        assert!(code.contains("if x2"));
    }

    #[test]
    fn test_create_nested_ast() {
        let ast = create_nested_ast(3);
        assert_eq!(ast.items.len(), 1);

        let code = quote::quote!(#ast).to_string();
        assert!(code.contains("if x0"));
        assert!(code.contains("if x1"));
        assert!(code.contains("if x2"));
    }

    #[test]
    fn test_create_multi_function_ast() {
        let ast = create_multi_function_ast(5);
        assert_eq!(ast.items.len(), 5);
    }

    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new()
            .complexity_threshold(15)
            .max_function_length(200)
            .minimum_debt_score(2.0)
            .build();

        let thresholds = config.thresholds.unwrap();
        assert_eq!(thresholds.complexity, Some(15));
        assert_eq!(thresholds.max_function_length, Some(200));
        assert_eq!(thresholds.minimum_debt_score, Some(2.0));
    }

    #[test]
    fn test_config_builder_ignore_patterns() {
        let config = ConfigBuilder::new()
            .ignore_patterns(vec!["tests/**/*".to_string()])
            .build();

        let ignore = config.ignore.unwrap();
        assert_eq!(ignore.patterns, vec!["tests/**/*"]);
    }

    #[test]
    fn test_create_test_coverage() {
        let fc = create_test_coverage(100, 75);
        assert_eq!(fc.total_lines, 100);
        assert_eq!(fc.hit_lines, 75);
    }

    #[test]
    fn test_create_coverage_data() {
        let data = create_coverage_data("test.rs", 80.0);
        let pct = data
            .get_file_coverage(std::path::Path::new("test.rs"))
            .unwrap();
        assert!((pct - 80.0).abs() < 1.0);
    }

    #[test]
    fn test_create_test_project() {
        let env = create_test_project();
        assert!(env.has_file("src/main.rs"));
        assert!(env.has_file("src/lib.rs"));
        assert!(env.has_file("src/utils.rs"));
        assert!(env.has_file("tests/integration_test.rs"));
    }

    #[test]
    fn test_create_complex_project() {
        use crate::env::AnalysisEnv;

        let env = create_complex_project();
        assert!(env.has_file("src/simple.rs"));
        assert!(env.has_file("src/complex.rs"));
        assert!(env.has_file("src/lib.rs"));

        // Verify config was applied
        let config = env.config();
        assert!(config.thresholds.is_some());
    }
}
