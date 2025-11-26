//! Effect wrappers for complexity analysis with I/O.
//!
//! This module provides effect-based wrappers around the pure complexity
//! functions in `pure.rs`. These wrappers compose:
//!
//! 1. File reading (I/O effect)
//! 2. Parsing (fallible operation)
//! 3. Pure complexity calculation
//!
//! # Design
//!
//! The key insight is separating concerns:
//! - **Pure functions** (`pure.rs`): Fast, testable, no I/O
//! - **Effect wrappers** (this module): Handle I/O and error cases
//!
//! This separation enables:
//! - Fast unit tests for pure functions (no I/O overhead)
//! - Integration tests for effect wrappers (real I/O)
//! - Clear error handling and context
//!
//! # Usage
//!
//! ```rust,ignore
//! use debtmap::complexity::effects_wrappers::*;
//! use debtmap::effects::run_effect;
//! use std::path::PathBuf;
//!
//! let path = PathBuf::from("src/main.rs");
//! let effect = calculate_cyclomatic_effect(path);
//! let complexity = run_effect(effect, config)?;
//! ```

use super::pure::{
    calculate_cognitive_pure, calculate_cyclomatic_pure, detect_patterns_pure, Pattern,
};
use crate::effects::{effect_fail, effect_pure, AnalysisEffect};
use crate::env::{AnalysisEnv, RealEnv};
use crate::errors::AnalysisError;
use std::path::PathBuf;
use stillwater::Effect;

/// Calculate cyclomatic complexity for a file with I/O.
///
/// This effect reads the file, parses it, and calculates complexity.
///
/// # Errors
///
/// - `AnalysisError::IoError` if the file cannot be read
/// - `AnalysisError::ParseError` if the file cannot be parsed as Rust
///
/// # Example
///
/// ```rust,ignore
/// let effect = calculate_cyclomatic_effect(PathBuf::from("src/main.rs"));
/// let complexity = run_effect(effect, config)?;
/// ```
pub fn calculate_cyclomatic_effect(path: PathBuf) -> AnalysisEffect<u32> {
    Effect::from_fn(move |env: &RealEnv| {
        // Read file content
        let content = env.file_system().read_to_string(&path).map_err(|e| {
            AnalysisError::io_with_path(format!("Failed to read file: {}", e), path.clone())
        })?;

        // Parse as Rust
        let ast = syn::parse_file(&content).map_err(|e| {
            AnalysisError::parse(format!("Failed to parse {}: {}", path.display(), e))
        })?;

        // Calculate complexity using pure function
        Ok(calculate_cyclomatic_pure(&ast))
    })
}

/// Calculate cognitive complexity for a file with I/O.
///
/// This effect reads the file, parses it, and calculates cognitive complexity.
///
/// # Errors
///
/// - `AnalysisError::IoError` if the file cannot be read
/// - `AnalysisError::ParseError` if the file cannot be parsed as Rust
///
/// # Example
///
/// ```rust,ignore
/// let effect = calculate_cognitive_effect(PathBuf::from("src/main.rs"));
/// let complexity = run_effect(effect, config)?;
/// ```
pub fn calculate_cognitive_effect(path: PathBuf) -> AnalysisEffect<u32> {
    Effect::from_fn(move |env: &RealEnv| {
        // Read file content
        let content = env.file_system().read_to_string(&path).map_err(|e| {
            AnalysisError::io_with_path(format!("Failed to read file: {}", e), path.clone())
        })?;

        // Parse as Rust
        let ast = syn::parse_file(&content).map_err(|e| {
            AnalysisError::parse(format!("Failed to parse {}: {}", path.display(), e))
        })?;

        // Calculate complexity using pure function
        Ok(calculate_cognitive_pure(&ast))
    })
}

/// Detect patterns in a file with I/O.
///
/// This effect reads the file, parses it, and detects code patterns.
///
/// # Errors
///
/// - `AnalysisError::IoError` if the file cannot be read
/// - `AnalysisError::ParseError` if the file cannot be parsed as Rust
///
/// # Example
///
/// ```rust,ignore
/// let effect = detect_patterns_effect(PathBuf::from("src/main.rs"));
/// let patterns = run_effect(effect, config)?;
/// ```
pub fn detect_patterns_effect(path: PathBuf) -> AnalysisEffect<Vec<Pattern>> {
    Effect::from_fn(move |env: &RealEnv| {
        // Read file content
        let content = env.file_system().read_to_string(&path).map_err(|e| {
            AnalysisError::io_with_path(format!("Failed to read file: {}", e), path.clone())
        })?;

        // Parse as Rust
        let ast = syn::parse_file(&content).map_err(|e| {
            AnalysisError::parse(format!("Failed to parse {}: {}", path.display(), e))
        })?;

        // Detect patterns using pure function
        Ok(detect_patterns_pure(&ast))
    })
}

/// Combined complexity analysis result.
#[derive(Debug, Clone)]
pub struct ComplexityResult {
    /// Cyclomatic complexity
    pub cyclomatic: u32,
    /// Cognitive complexity
    pub cognitive: u32,
    /// Detected patterns
    pub patterns: Vec<Pattern>,
}

/// Analyze a file for all complexity metrics with I/O.
///
/// This combines cyclomatic, cognitive, and pattern analysis in one effect.
/// It's more efficient than running three separate effects since it only
/// reads and parses the file once.
///
/// # Example
///
/// ```rust,ignore
/// let effect = analyze_complexity_effect(PathBuf::from("src/main.rs"));
/// let result = run_effect(effect, config)?;
/// println!("Cyclomatic: {}, Cognitive: {}", result.cyclomatic, result.cognitive);
/// ```
pub fn analyze_complexity_effect(path: PathBuf) -> AnalysisEffect<ComplexityResult> {
    Effect::from_fn(move |env: &RealEnv| {
        // Read file content
        let content = env.file_system().read_to_string(&path).map_err(|e| {
            AnalysisError::io_with_path(format!("Failed to read file: {}", e), path.clone())
        })?;

        // Parse as Rust
        let ast = syn::parse_file(&content).map_err(|e| {
            AnalysisError::parse(format!("Failed to parse {}: {}", path.display(), e))
        })?;

        // Calculate all metrics using pure functions
        Ok(ComplexityResult {
            cyclomatic: calculate_cyclomatic_pure(&ast),
            cognitive: calculate_cognitive_pure(&ast),
            patterns: detect_patterns_pure(&ast),
        })
    })
}

/// Calculate complexity from a string (for testing or processing in-memory content).
///
/// This is useful when you already have the source code as a string and
/// don't need to read from disk.
///
/// # Example
///
/// ```rust,ignore
/// let code = "fn foo() { if x { } }";
/// let effect = calculate_cyclomatic_from_string(code.to_string());
/// let complexity = run_effect(effect, config)?;
/// ```
pub fn calculate_cyclomatic_from_string(content: String) -> AnalysisEffect<u32> {
    match syn::parse_file(&content) {
        Ok(ast) => effect_pure(calculate_cyclomatic_pure(&ast)),
        Err(e) => effect_fail(AnalysisError::parse(format!("Parse error: {}", e))),
    }
}

/// Calculate cognitive complexity from a string.
pub fn calculate_cognitive_from_string(content: String) -> AnalysisEffect<u32> {
    match syn::parse_file(&content) {
        Ok(ast) => effect_pure(calculate_cognitive_pure(&ast)),
        Err(e) => effect_fail(AnalysisError::parse(format!("Parse error: {}", e))),
    }
}

/// Detect patterns from a string.
pub fn detect_patterns_from_string(content: String) -> AnalysisEffect<Vec<Pattern>> {
    match syn::parse_file(&content) {
        Ok(ast) => effect_pure(detect_patterns_pure(&ast)),
        Err(e) => effect_fail(AnalysisError::parse(format!("Parse error: {}", e))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DebtmapConfig;
    use crate::effects::run_effect;

    #[test]
    fn test_cyclomatic_from_string() {
        let code = "fn foo() { if true { } }".to_string();
        let effect = calculate_cyclomatic_from_string(code);
        let result = run_effect(effect, DebtmapConfig::default());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2); // base 1 + if 1
    }

    #[test]
    fn test_cognitive_from_string() {
        let code = "fn foo() { if true { if false { } } }".to_string();
        let effect = calculate_cognitive_from_string(code);
        let result = run_effect(effect, DebtmapConfig::default());
        assert!(result.is_ok());
        // Outer if: 1 + 0 = 1, Inner if: 1 + 1 = 2, Total: 3
        assert_eq!(result.unwrap(), 3);
    }

    #[test]
    fn test_patterns_from_string() {
        let code = r#"
            struct Big { a: i32, b: i32, c: i32, d: i32, e: i32, f: i32 }
        "#
        .to_string();
        let effect = detect_patterns_from_string(code);
        let result = run_effect(effect, DebtmapConfig::default());
        assert!(result.is_ok());
        let patterns = result.unwrap();
        assert!(!patterns.is_empty());
        assert!(matches!(&patterns[0], Pattern::GodObject { .. }));
    }

    #[test]
    fn test_parse_error_handling() {
        let invalid_code = "fn foo( {".to_string(); // Invalid syntax
        let effect = calculate_cyclomatic_from_string(invalid_code);
        let result = run_effect(effect, DebtmapConfig::default());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Parse error"));
    }
}
