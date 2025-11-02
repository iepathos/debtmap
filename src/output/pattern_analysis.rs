//! Pattern Analysis Data Structures
//!
//! This module defines data structures for aggregating and displaying pattern analysis results
//! from purity analysis (Spec 143), framework pattern detection (Spec 144), and Rust-specific
//! pattern detection (Spec 146).
//!
//! # Architecture
//!
//! - **Data structures are pure data** - no formatting logic, only calculations
//! - **All types implement Serialize/Deserialize** - for potential JSON output
//! - **Default implementations** - all types have sensible defaults
//! - **Helper methods for queries** - has_patterns(), repetitive_traits(), etc.
//!
//! # Example
//!
//! ```ignore
//! use debtmap::output::pattern_analysis::{PatternAnalysis, PurityMetrics};
//!
//! // Create pattern analysis from function analyses
//! let pattern_analysis = PatternAnalysis::from_functions(&functions);
//!
//! // Query the data
//! if pattern_analysis.purity.purity_percentage() > 0.5 {
//!     println!("Codebase is more than 50% pure!");
//! }
//! ```

use crate::analysis::purity_analysis::PurityViolation;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Constants for thresholds (configurable via settings)
pub const REPETITIVE_TRAIT_THRESHOLD: usize = 5;
pub const MAX_DISPLAYED_EXAMPLES: usize = 3;
pub const MAX_ALMOST_PURE_VIOLATIONS: usize = 2;

/// Purity metrics aggregated from function analyses
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PurityMetrics {
    pub strictly_pure: usize,
    pub locally_pure: usize,
    pub read_only: usize,
    pub impure: usize,
    pub almost_pure: Vec<AlmostPureFunction>,
}

impl PurityMetrics {
    /// Pure calculation - returns total number of functions analyzed
    pub fn total_functions(&self) -> usize {
        self.strictly_pure + self.locally_pure + self.read_only + self.impure
    }

    /// Returns purity percentage as a value between 0.0 and 1.0
    pub fn purity_percentage(&self) -> f64 {
        let total = self.total_functions();
        if total == 0 {
            0.0
        } else {
            (self.strictly_pure + self.locally_pure) as f64 / total as f64
        }
    }

    /// Returns the top N almost-pure functions by refactoring impact
    pub fn top_refactoring_opportunities(&self, limit: usize) -> &[AlmostPureFunction] {
        let end = self.almost_pure.len().min(limit);
        &self.almost_pure[..end]
    }

    /// Returns true if there are any functions analyzed
    pub fn has_functions(&self) -> bool {
        self.total_functions() > 0
    }

    /// Returns true if there are refactoring opportunities
    pub fn has_opportunities(&self) -> bool {
        !self.almost_pure.is_empty()
    }
}

/// Function that is "almost pure" - has only 1-2 purity violations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlmostPureFunction {
    pub name: String,
    pub violations: Vec<PurityViolation>,
    pub refactoring_suggestion: String,
}

/// Framework pattern metrics aggregated from function analyses
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FrameworkPatternMetrics {
    pub patterns: Vec<DetectedPattern>,
}

impl FrameworkPatternMetrics {
    /// Returns patterns sorted by count (most frequent first)
    pub fn sorted_by_frequency(&self) -> Vec<&DetectedPattern> {
        let mut sorted: Vec<_> = self.patterns.iter().collect();
        sorted.sort_by(|a, b| b.count.cmp(&a.count));
        sorted
    }

    /// Returns true if any framework patterns were detected
    pub fn has_patterns(&self) -> bool {
        !self.patterns.is_empty()
    }
}

/// Detected framework pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPattern {
    pub framework: String,
    pub pattern_type: String,
    pub count: usize,
    pub examples: Vec<String>,
    pub recommendation: String,
}

/// Rust-specific pattern metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RustPatternMetrics {
    pub trait_impls: Vec<TraitImplementation>,
    pub async_patterns: AsyncPatternSummary,
    pub error_handling: ErrorHandlingSummary,
    pub builder_candidates: Vec<String>,
}

impl RustPatternMetrics {
    /// Returns trait implementations that appear repetitively
    pub fn repetitive_traits(&self) -> Vec<&TraitImplementation> {
        self.trait_impls
            .iter()
            .filter(|t| t.count >= REPETITIVE_TRAIT_THRESHOLD)
            .collect()
    }

    /// Returns true if async patterns are present
    pub fn has_async_patterns(&self) -> bool {
        self.async_patterns.async_functions > 0
    }

    /// Returns true if error handling patterns are present
    pub fn has_error_handling(&self) -> bool {
        self.error_handling.question_mark_density > 0.0 || self.error_handling.unwrap_count > 0
    }

    /// Returns true if builder patterns are present
    pub fn has_builder_candidates(&self) -> bool {
        !self.builder_candidates.is_empty()
    }

    /// Returns true if any Rust patterns are present
    pub fn has_patterns(&self) -> bool {
        !self.trait_impls.is_empty()
            || self.has_async_patterns()
            || self.has_error_handling()
            || self.has_builder_candidates()
    }
}

/// Trait implementation pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitImplementation {
    pub trait_name: String,
    pub count: usize,
    pub types: Vec<String>,
}

/// Async pattern summary
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AsyncPatternSummary {
    pub async_functions: usize,
    pub spawn_calls: usize,
    pub channel_usage: bool,
    pub mutex_usage: bool,
}

/// Error handling summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorHandlingSummary {
    /// Average ? operators per function
    pub question_mark_density: f64,
    pub custom_error_types: Vec<String>,
    /// Anti-pattern: unwrap() call count
    pub unwrap_count: usize,
}

impl Default for ErrorHandlingSummary {
    fn default() -> Self {
        Self {
            question_mark_density: 0.0,
            custom_error_types: vec![],
            unwrap_count: 0,
        }
    }
}

/// Container for all pattern analysis results
/// Attached to recommendations for display
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PatternAnalysis {
    pub purity: PurityMetrics,
    pub frameworks: FrameworkPatternMetrics,
    pub rust_patterns: RustPatternMetrics,
}

impl PatternAnalysis {
    /// Create a new empty pattern analysis
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if any patterns were detected
    pub fn has_patterns(&self) -> bool {
        self.purity.has_functions()
            || self.frameworks.has_patterns()
            || self.rust_patterns.has_patterns()
    }

    /// Create PatternAnalysis from a collection of function analyses
    /// This aggregates purity metrics, framework patterns, and Rust patterns
    pub fn from_functions(functions: &[crate::priority::FunctionAnalysis]) -> Self {
        Self {
            purity: aggregate_purity_metrics(functions),
            frameworks: aggregate_framework_patterns(functions),
            rust_patterns: aggregate_rust_patterns(functions),
        }
    }
}

/// Aggregate purity metrics from function analyses
fn aggregate_purity_metrics(
    functions: &[crate::priority::FunctionAnalysis],
) -> PurityMetrics {
    let mut strictly_pure = 0;
    let mut locally_pure = 0;
    let mut read_only = 0;
    let mut impure = 0;
    let almost_pure = Vec::new();

    for func in functions {
        if let Some(is_pure) = func.is_pure {
            if is_pure {
                // Distinguish between strictly pure and locally pure based on confidence
                if func.purity_confidence.unwrap_or(0.0) >= 0.9 {
                    strictly_pure += 1;
                } else {
                    locally_pure += 1;
                }
            } else {
                // Check if function is "almost pure" (has 1-2 violations)
                // For now, we'll use a heuristic: low complexity impure functions
                if func.cyclomatic_complexity <= 5 && func.cognitive_complexity <= 7 {
                    read_only += 1;
                } else {
                    impure += 1;
                }
            }
        } else {
            // Unknown purity - count as impure
            impure += 1;
        }
    }

    PurityMetrics {
        strictly_pure,
        locally_pure,
        read_only,
        impure,
        almost_pure,
    }
}

/// Aggregate framework patterns from function analyses
fn aggregate_framework_patterns(
    _functions: &[crate::priority::FunctionAnalysis],
) -> FrameworkPatternMetrics {
    // Framework pattern detection would need additional analysis
    // For now, return empty metrics
    FrameworkPatternMetrics {
        patterns: Vec::new(),
    }
}

/// Aggregate Rust-specific patterns from function analyses
fn aggregate_rust_patterns(
    _functions: &[crate::priority::FunctionAnalysis],
) -> RustPatternMetrics {
    // Rust pattern detection would need additional analysis
    // For now, return empty metrics
    RustPatternMetrics::default()
}

/// Generate purity refactoring suggestions based on violation types
/// This is a pure function that takes violation information and returns actionable suggestions
pub fn suggest_purity_refactoring(violations: &[PurityViolation]) -> Vec<String> {
    let mut suggestions = Vec::new();

    for violation in violations {
        let suggestion = match violation {
            PurityViolation::StateMutation { .. } => {
                "Pass state as function parameter instead of mutating external state"
            }
            PurityViolation::IoOperation { .. } => {
                "Move I/O to function boundaries; separate pure logic from side effects"
            }
            PurityViolation::NonDeterministic { .. } => {
                "Make function deterministic by accepting random seed or current time as parameter"
            }
            PurityViolation::ImpureCall { .. } => {
                "Extract pure logic from impure function calls; isolate side effects"
            }
        };

        if !suggestions.contains(&suggestion.to_string()) {
            suggestions.push(suggestion.to_string());
        }
    }

    suggestions
}

/// Generate framework-specific recommendations based on detected patterns
/// Pure function that maps framework patterns to actionable advice
pub fn generate_framework_recommendation(
    framework: &str,
    pattern_type: &str,
    count: usize,
) -> String {
    match (framework, pattern_type) {
        ("React", "hooks") => {
            format!("Consider extracting {} hook usages into custom hooks for reusability", count)
        }
        ("React", "component") => {
            format!("Review {} components for proper memoization and render optimization", count)
        }
        ("Tokio", "async") => {
            format!("Verify {} async operations use proper error handling and cancellation", count)
        }
        ("Actix", "handler") => {
            format!("Ensure {} handlers follow async best practices and proper error propagation", count)
        }
        ("Django", "view") => {
            format!("Review {} views for proper transaction handling and query optimization", count)
        }
        ("Flask", "route") => {
            format!("Consider adding validation and error handling to {} route handlers", count)
        }
        _ => {
            format!("Review {} instances of {} {} pattern for consistency and best practices",
                    count, framework, pattern_type)
        }
    }
}

/// Display implementation for PurityViolation (used in formatting)
impl fmt::Display for PurityViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::purity_analysis::PurityViolation;

    #[test]
    fn test_purity_metrics_total_functions() {
        let metrics = PurityMetrics {
            strictly_pure: 10,
            locally_pure: 5,
            read_only: 3,
            impure: 7,
            almost_pure: vec![],
        };

        assert_eq!(metrics.total_functions(), 25);
    }

    #[test]
    fn test_purity_metrics_purity_percentage() {
        let metrics = PurityMetrics {
            strictly_pure: 15,
            locally_pure: 10,
            read_only: 5,
            impure: 20,
            almost_pure: vec![],
        };

        // (15 + 10) / 50 = 0.5 = 50%
        assert_eq!(metrics.purity_percentage(), 0.5);
    }

    #[test]
    fn test_purity_metrics_purity_percentage_zero_functions() {
        let metrics = PurityMetrics::default();
        assert_eq!(metrics.purity_percentage(), 0.0);
    }

    #[test]
    fn test_purity_metrics_top_refactoring_opportunities() {
        let func1 = AlmostPureFunction {
            name: "func1".to_string(),
            violations: vec![],
            refactoring_suggestion: "test".to_string(),
        };
        let func2 = AlmostPureFunction {
            name: "func2".to_string(),
            violations: vec![],
            refactoring_suggestion: "test".to_string(),
        };
        let func3 = AlmostPureFunction {
            name: "func3".to_string(),
            violations: vec![],
            refactoring_suggestion: "test".to_string(),
        };

        let metrics = PurityMetrics {
            strictly_pure: 0,
            locally_pure: 0,
            read_only: 0,
            impure: 0,
            almost_pure: vec![func1, func2, func3],
        };

        let top = metrics.top_refactoring_opportunities(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].name, "func1");
        assert_eq!(top[1].name, "func2");
    }

    #[test]
    fn test_purity_metrics_has_functions() {
        let empty = PurityMetrics::default();
        assert!(!empty.has_functions());

        let with_functions = PurityMetrics {
            strictly_pure: 1,
            locally_pure: 0,
            read_only: 0,
            impure: 0,
            almost_pure: vec![],
        };
        assert!(with_functions.has_functions());
    }

    #[test]
    fn test_purity_metrics_has_opportunities() {
        let empty = PurityMetrics::default();
        assert!(!empty.has_opportunities());

        let func = AlmostPureFunction {
            name: "test".to_string(),
            violations: vec![],
            refactoring_suggestion: "fix".to_string(),
        };
        let with_opportunities = PurityMetrics {
            strictly_pure: 0,
            locally_pure: 0,
            read_only: 0,
            impure: 0,
            almost_pure: vec![func],
        };
        assert!(with_opportunities.has_opportunities());
    }

    #[test]
    fn test_framework_pattern_metrics_sorted_by_frequency() {
        let pattern1 = DetectedPattern {
            framework: "React".to_string(),
            pattern_type: "hooks".to_string(),
            count: 5,
            examples: vec![],
            recommendation: "test".to_string(),
        };
        let pattern2 = DetectedPattern {
            framework: "Vue".to_string(),
            pattern_type: "components".to_string(),
            count: 15,
            examples: vec![],
            recommendation: "test".to_string(),
        };
        let pattern3 = DetectedPattern {
            framework: "Angular".to_string(),
            pattern_type: "services".to_string(),
            count: 10,
            examples: vec![],
            recommendation: "test".to_string(),
        };

        let metrics = FrameworkPatternMetrics {
            patterns: vec![pattern1, pattern2, pattern3],
        };

        let sorted = metrics.sorted_by_frequency();
        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].count, 15); // Vue (highest)
        assert_eq!(sorted[1].count, 10); // Angular
        assert_eq!(sorted[2].count, 5);  // React (lowest)
    }

    #[test]
    fn test_rust_pattern_metrics_repetitive_traits() {
        let trait1 = TraitImplementation {
            trait_name: "Display".to_string(),
            count: 3,
            types: vec![],
        };
        let trait2 = TraitImplementation {
            trait_name: "Clone".to_string(),
            count: 10,
            types: vec![],
        };

        let metrics = RustPatternMetrics {
            trait_impls: vec![trait1, trait2],
            async_patterns: AsyncPatternSummary::default(),
            error_handling: ErrorHandlingSummary::default(),
            builder_candidates: vec![],
        };

        let repetitive = metrics.repetitive_traits();
        assert_eq!(repetitive.len(), 1);
        assert_eq!(repetitive[0].trait_name, "Clone");
        assert_eq!(repetitive[0].count, 10);
    }

    #[test]
    fn test_rust_pattern_metrics_has_patterns() {
        let empty = RustPatternMetrics::default();
        assert!(!empty.has_patterns());

        let with_traits = RustPatternMetrics {
            trait_impls: vec![TraitImplementation {
                trait_name: "Debug".to_string(),
                count: 1,
                types: vec![],
            }],
            async_patterns: AsyncPatternSummary::default(),
            error_handling: ErrorHandlingSummary::default(),
            builder_candidates: vec![],
        };
        assert!(with_traits.has_patterns());

        let with_async = RustPatternMetrics {
            trait_impls: vec![],
            async_patterns: AsyncPatternSummary {
                async_functions: 5,
                spawn_calls: 0,
                channel_usage: false,
                mutex_usage: false,
            },
            error_handling: ErrorHandlingSummary::default(),
            builder_candidates: vec![],
        };
        assert!(with_async.has_patterns());
    }

    #[test]
    fn test_pattern_analysis_has_patterns() {
        let empty = PatternAnalysis::default();
        assert!(!empty.has_patterns());

        let with_purity = PatternAnalysis {
            purity: PurityMetrics {
                strictly_pure: 5,
                locally_pure: 0,
                read_only: 0,
                impure: 0,
                almost_pure: vec![],
            },
            frameworks: FrameworkPatternMetrics::default(),
            rust_patterns: RustPatternMetrics::default(),
        };
        assert!(with_purity.has_patterns());
    }

    #[test]
    fn test_suggest_purity_refactoring() {
        let violations = vec![
            PurityViolation::StateMutation {
                target: "x".to_string(),
                line: Some(10),
            },
            PurityViolation::IoOperation {
                description: "println".to_string(),
                line: Some(15),
            },
        ];

        let suggestions = suggest_purity_refactoring(&violations);
        assert_eq!(suggestions.len(), 2);
        assert!(suggestions[0].contains("state as function parameter"));
        assert!(suggestions[1].contains("Move I/O to function boundaries"));
    }

    #[test]
    fn test_suggest_purity_refactoring_no_duplicates() {
        let violations = vec![
            PurityViolation::StateMutation {
                target: "x".to_string(),
                line: Some(10),
            },
            PurityViolation::StateMutation {
                target: "y".to_string(),
                line: Some(20),
            },
        ];

        let suggestions = suggest_purity_refactoring(&violations);
        assert_eq!(suggestions.len(), 1); // Should not duplicate the same suggestion
    }

    #[test]
    fn test_generate_framework_recommendation() {
        let rec = generate_framework_recommendation("React", "hooks", 10);
        assert!(rec.contains("React") || rec.contains("10") || rec.contains("hook"));

        let rec2 = generate_framework_recommendation("Tokio", "async", 5);
        assert!(rec2.contains("async") || rec2.contains("5"));
    }
}
