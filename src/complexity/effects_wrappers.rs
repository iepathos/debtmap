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
use crate::extraction::UnifiedFileExtractor;
use std::path::PathBuf;
use stillwater::effect::prelude::*;

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
///
/// # Note
///
/// Spec 202: Uses UnifiedFileExtractor for parsing to prevent SourceMap overflow.
/// The extracted data includes pre-computed cyclomatic complexity, but we
/// re-calculate it here for consistency with the pure function interface.
pub fn calculate_cyclomatic_effect(path: PathBuf) -> AnalysisEffect<u32> {
    from_fn(move |env: &RealEnv| {
        // Read file content
        let content = env.file_system().read_to_string(&path).map_err(|e| {
            AnalysisError::io_with_path(format!("Failed to read file: {}", e), path.clone())
        })?;

        // Spec 202: Use extracted data for pre-computed complexity
        let extracted = UnifiedFileExtractor::extract(&path, &content).map_err(|e| {
            AnalysisError::parse(format!("Failed to parse {}: {}", path.display(), e))
        })?;

        // Sum cyclomatic complexity from all functions
        let total_complexity: u32 = extracted.functions.iter().map(|f| f.cyclomatic).sum();
        Ok(total_complexity)
    })
    .boxed()
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
///
/// Spec 202: Uses UnifiedFileExtractor for parsing to prevent SourceMap overflow.
pub fn calculate_cognitive_effect(path: PathBuf) -> AnalysisEffect<u32> {
    from_fn(move |env: &RealEnv| {
        // Read file content
        let content = env.file_system().read_to_string(&path).map_err(|e| {
            AnalysisError::io_with_path(format!("Failed to read file: {}", e), path.clone())
        })?;

        // Spec 202: Use extracted data for pre-computed cognitive complexity
        let extracted = UnifiedFileExtractor::extract(&path, &content).map_err(|e| {
            AnalysisError::parse(format!("Failed to parse {}: {}", path.display(), e))
        })?;

        // Sum cognitive complexity from all functions
        let total_cognitive: u32 = extracted.functions.iter().map(|f| f.cognitive).sum();
        Ok(total_cognitive)
    })
    .boxed()
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
///
/// Note: Pattern detection requires AST access and is not pre-computed in
/// ExtractedFileData. We use syn::parse_file here but reset SourceMap after (spec 202).
pub fn detect_patterns_effect(path: PathBuf) -> AnalysisEffect<Vec<Pattern>> {
    from_fn(move |env: &RealEnv| {
        // Read file content
        let content = env.file_system().read_to_string(&path).map_err(|e| {
            AnalysisError::io_with_path(format!("Failed to read file: {}", e), path.clone())
        })?;

        // Parse as Rust (pattern detection needs AST access)
        let ast = syn::parse_file(&content).map_err(|e| {
            AnalysisError::parse(format!("Failed to parse {}: {}", path.display(), e))
        })?;

        // Detect patterns using pure function
        let patterns = detect_patterns_pure(&ast);

        // Reset SourceMap after parsing to prevent overflow (spec 202)
        crate::core::parsing::reset_span_locations();

        Ok(patterns)
    })
    .boxed()
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
///
/// Spec 202: Uses UnifiedFileExtractor for complexity metrics but still needs
/// AST for pattern detection. SourceMap is reset after each parsing operation.
pub fn analyze_complexity_effect(path: PathBuf) -> AnalysisEffect<ComplexityResult> {
    from_fn(move |env: &RealEnv| {
        // Read file content
        let content = env.file_system().read_to_string(&path).map_err(|e| {
            AnalysisError::io_with_path(format!("Failed to read file: {}", e), path.clone())
        })?;

        // Use extracted data for complexity metrics (spec 202)
        let extracted = UnifiedFileExtractor::extract(&path, &content).map_err(|e| {
            AnalysisError::parse(format!("Failed to parse {}: {}", path.display(), e))
        })?;

        // Get complexity from extracted data
        let cyclomatic: u32 = extracted.functions.iter().map(|f| f.cyclomatic).sum();
        let cognitive: u32 = extracted.functions.iter().map(|f| f.cognitive).sum();

        // Pattern detection requires AST access (not in ExtractedFileData)
        let patterns = syn::parse_file(&content)
            .map(|ast| {
                let result = detect_patterns_pure(&ast);
                crate::core::parsing::reset_span_locations();
                result
            })
            .unwrap_or_default();

        Ok(ComplexityResult {
            cyclomatic,
            cognitive,
            patterns,
        })
    })
    .boxed()
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
///
/// Spec 202: Resets SourceMap after parsing.
pub fn calculate_cyclomatic_from_string(content: String) -> AnalysisEffect<u32> {
    let result = syn::parse_file(&content);
    crate::core::parsing::reset_span_locations();
    match result {
        Ok(ast) => effect_pure(calculate_cyclomatic_pure(&ast)),
        Err(e) => effect_fail(AnalysisError::parse(format!("Parse error: {}", e))),
    }
}

/// Calculate cognitive complexity from a string.
///
/// Spec 202: Resets SourceMap after parsing.
pub fn calculate_cognitive_from_string(content: String) -> AnalysisEffect<u32> {
    let result = syn::parse_file(&content);
    crate::core::parsing::reset_span_locations();
    match result {
        Ok(ast) => effect_pure(calculate_cognitive_pure(&ast)),
        Err(e) => effect_fail(AnalysisError::parse(format!("Parse error: {}", e))),
    }
}

/// Detect patterns from a string.
///
/// Spec 202: Resets SourceMap after parsing.
pub fn detect_patterns_from_string(content: String) -> AnalysisEffect<Vec<Pattern>> {
    let result = syn::parse_file(&content);
    crate::core::parsing::reset_span_locations();
    match result {
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
