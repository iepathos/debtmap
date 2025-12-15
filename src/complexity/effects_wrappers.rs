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

use super::pure::Pattern;
use crate::effects::{effect_fail, effect_pure, AnalysisEffect};
use crate::env::{AnalysisEnv, RealEnv};
use crate::errors::AnalysisError;
use crate::extraction::{DetectedPattern, UnifiedFileExtractor};
use std::path::PathBuf;
use stillwater::effect::prelude::*;

/// Convert extracted patterns to complexity patterns.
///
/// Spec 204: Patterns are now pre-computed during extraction to avoid re-parsing.
fn convert_extracted_patterns(patterns: &[DetectedPattern]) -> Vec<Pattern> {
    patterns
        .iter()
        .map(|p| match p {
            DetectedPattern::GodObject { name, field_count } => Pattern::GodObject {
                name: name.clone(),
                field_count: *field_count,
            },
            DetectedPattern::LongFunction { name, lines } => Pattern::LongFunction {
                name: name.clone(),
                lines: *lines,
            },
            DetectedPattern::ManyParameters { name, param_count } => Pattern::ManyParameters {
                name: name.clone(),
                param_count: *param_count,
            },
            DetectedPattern::DeepNesting {
                function_name,
                depth,
            } => Pattern::DeepNesting {
                function_name: function_name.clone(),
                depth: *depth,
            },
        })
        .collect()
}

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
/// Spec 204: Patterns are now pre-computed during extraction to avoid re-parsing.
/// Uses UnifiedFileExtractor which handles SourceMap reset internally.
pub fn detect_patterns_effect(path: PathBuf) -> AnalysisEffect<Vec<Pattern>> {
    from_fn(move |env: &RealEnv| {
        // Read file content
        let content = env.file_system().read_to_string(&path).map_err(|e| {
            AnalysisError::io_with_path(format!("Failed to read file: {}", e), path.clone())
        })?;

        // Use extracted data which includes detected patterns (spec 204)
        let extracted = UnifiedFileExtractor::extract(&path, &content).map_err(|e| {
            AnalysisError::parse(format!("Failed to parse {}: {}", path.display(), e))
        })?;

        // Convert extracted patterns to complexity patterns
        let patterns = convert_extracted_patterns(&extracted.detected_patterns);

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
/// Spec 204: Uses UnifiedFileExtractor for all metrics including pattern detection.
/// No separate parsing needed - all data is extracted in a single pass.
pub fn analyze_complexity_effect(path: PathBuf) -> AnalysisEffect<ComplexityResult> {
    from_fn(move |env: &RealEnv| {
        // Read file content
        let content = env.file_system().read_to_string(&path).map_err(|e| {
            AnalysisError::io_with_path(format!("Failed to read file: {}", e), path.clone())
        })?;

        // Use extracted data for all metrics (spec 204)
        let extracted = UnifiedFileExtractor::extract(&path, &content).map_err(|e| {
            AnalysisError::parse(format!("Failed to parse {}: {}", path.display(), e))
        })?;

        // Get complexity from extracted data
        let cyclomatic: u32 = extracted.functions.iter().map(|f| f.cyclomatic).sum();
        let cognitive: u32 = extracted.functions.iter().map(|f| f.cognitive).sum();

        // Patterns are now pre-computed during extraction (spec 204)
        let patterns = convert_extracted_patterns(&extracted.detected_patterns);

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
/// Spec 204: Uses UnifiedFileExtractor for complexity calculation.
pub fn calculate_cyclomatic_from_string(content: String) -> AnalysisEffect<u32> {
    use std::path::Path;
    let path = Path::new("<string>");
    match UnifiedFileExtractor::extract(path, &content) {
        Ok(extracted) => {
            let total: u32 = extracted.functions.iter().map(|f| f.cyclomatic).sum();
            effect_pure(total)
        }
        Err(e) => effect_fail(AnalysisError::parse(format!("Parse error: {}", e))),
    }
}

/// Calculate cognitive complexity from a string.
///
/// Spec 204: Uses UnifiedFileExtractor for complexity calculation.
pub fn calculate_cognitive_from_string(content: String) -> AnalysisEffect<u32> {
    use std::path::Path;
    let path = Path::new("<string>");
    match UnifiedFileExtractor::extract(path, &content) {
        Ok(extracted) => {
            let total: u32 = extracted.functions.iter().map(|f| f.cognitive).sum();
            effect_pure(total)
        }
        Err(e) => effect_fail(AnalysisError::parse(format!("Parse error: {}", e))),
    }
}

/// Detect patterns from a string.
///
/// Spec 204: Uses UnifiedFileExtractor for pattern detection.
pub fn detect_patterns_from_string(content: String) -> AnalysisEffect<Vec<Pattern>> {
    use std::path::Path;
    let path = Path::new("<string>");
    match UnifiedFileExtractor::extract(path, &content) {
        Ok(extracted) => effect_pure(convert_extracted_patterns(&extracted.detected_patterns)),
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
