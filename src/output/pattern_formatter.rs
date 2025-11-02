//! Pattern Formatter - Pure Display Logic
//!
//! This module contains all formatting logic for pattern analysis, completely separated
//! from data structures. All formatting functions are pure (data in, string out).
//!
//! # Architecture
//!
//! - **Stateless formatter** - all methods are pure static functions
//! - **No business logic** - only string formatting and display
//! - **Empty metrics produce no output** - not "No patterns detected" messages
//! - **Handles edge cases** - Unicode, empty lists, zero functions, etc.
//!
//! # Example
//!
//! ```ignore
//! use debtmap::output::pattern_formatter::PatternFormatter;
//! use debtmap::output::pattern_analysis::PatternAnalysis;
//!
//! let pattern_analysis = PatternAnalysis::from_functions(&functions);
//! let formatted = PatternFormatter::format(&pattern_analysis);
//! println!("{}", formatted);
//! ```

use crate::output::pattern_analysis::{
    AlmostPureFunction, AsyncPatternSummary, DetectedPattern, ErrorHandlingSummary,
    FrameworkPatternMetrics, PatternAnalysis, PurityMetrics, RustPatternMetrics,
};

/// Formatter for pattern analysis - pure display logic only
pub struct PatternFormatter;

impl PatternFormatter {
    /// Formats complete pattern analysis for output
    pub fn format(analysis: &PatternAnalysis) -> String {
        if !analysis.has_patterns() {
            return String::new();
        }

        [
            Self::format_purity(&analysis.purity),
            Self::format_frameworks(&analysis.frameworks),
            Self::format_rust_patterns(&analysis.rust_patterns),
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
    }

    fn format_purity(metrics: &PurityMetrics) -> String {
        if !metrics.has_functions() {
            return String::new();
        }

        let mut sections = vec![format!(
            "PURITY ANALYSIS:\n\
             - Strictly Pure: {} functions ({:.0}%)\n\
             - Locally Pure: {} functions\n\
             - Read-Only: {} functions\n\
             - Impure: {} functions",
            metrics.strictly_pure,
            metrics.purity_percentage() * 100.0,
            metrics.locally_pure,
            metrics.read_only,
            metrics.impure
        )];

        let opportunities = Self::format_refactoring_opportunities(metrics);
        if !opportunities.is_empty() {
            sections.push(opportunities);
        }

        sections.join("\n\n")
    }

    fn format_refactoring_opportunities(metrics: &PurityMetrics) -> String {
        let opportunities = metrics.top_refactoring_opportunities(5);
        if opportunities.is_empty() {
            return String::new();
        }

        let mut output = String::from("REFACTORING OPPORTUNITIES:");
        for func in opportunities {
            output.push_str(&Self::format_almost_pure_function(func));
        }
        output
    }

    fn format_almost_pure_function(func: &AlmostPureFunction) -> String {
        let violation_count = func.violations.len();
        let plural = if violation_count == 1 { "" } else { "s" };

        let violation_desc = if !func.violations.is_empty() {
            func.violations[0].description()
        } else {
            "Unknown violation".to_string()
        };

        format!(
            "\n  - {} ({} violation{}): {}\n    → Suggestion: {}",
            func.name, violation_count, plural, violation_desc, func.refactoring_suggestion
        )
    }

    fn format_frameworks(metrics: &FrameworkPatternMetrics) -> String {
        if !metrics.has_patterns() {
            return String::new();
        }

        let mut output = String::from("FRAMEWORK PATTERNS DETECTED:");
        for pattern in metrics.sorted_by_frequency() {
            output.push_str(&Self::format_detected_pattern(pattern));
        }
        output
    }

    fn format_detected_pattern(pattern: &DetectedPattern) -> String {
        let examples = if pattern.examples.is_empty() {
            "(no examples)".to_string()
        } else {
            pattern.examples.join(", ")
        };

        format!(
            "\n  - {} {} ({}x detected)\n    Examples: {}\n    Recommendation: {}",
            pattern.framework,
            pattern.pattern_type,
            pattern.count,
            examples,
            pattern.recommendation
        )
    }

    fn format_rust_patterns(metrics: &RustPatternMetrics) -> String {
        if !metrics.has_patterns() {
            return String::new();
        }

        let sections = [
            Self::format_trait_implementations(metrics),
            Self::format_async_patterns(metrics),
            Self::format_error_handling(metrics),
            Self::format_builder_candidates(metrics),
        ];

        let non_empty: Vec<_> = sections.into_iter().filter(|s| !s.is_empty()).collect();

        if non_empty.is_empty() {
            return String::new();
        }

        format!("RUST-SPECIFIC PATTERNS:\n{}", non_empty.join("\n"))
    }

    fn format_trait_implementations(metrics: &RustPatternMetrics) -> String {
        if metrics.trait_impls.is_empty() {
            return String::new();
        }

        let mut output = String::from("  Trait Implementations:");
        for trait_impl in &metrics.trait_impls {
            let types_display = if trait_impl.types.is_empty() {
                "(no examples)".to_string()
            } else {
                trait_impl.types.join(", ")
            };

            output.push_str(&format!(
                "\n    - {}: {} implementations ({})",
                trait_impl.trait_name, trait_impl.count, types_display
            ));
        }

        let repetitive = metrics.repetitive_traits();
        if !repetitive.is_empty() {
            output.push_str("\n    → Consider using macros for repetitive implementations");
        }

        output
    }

    fn format_async_patterns(metrics: &RustPatternMetrics) -> String {
        if !metrics.has_async_patterns() {
            return String::new();
        }

        Self::format_async_summary(&metrics.async_patterns)
    }

    fn format_async_summary(async_pat: &AsyncPatternSummary) -> String {
        let mut output = format!(
            "  Async/Concurrency:\n\
               - Async functions: {}\n\
               - Spawn calls: {}\n\
               - Channels: {}\n\
               - Mutex usage: {}",
            async_pat.async_functions,
            async_pat.spawn_calls,
            if async_pat.channel_usage { "Yes" } else { "No" },
            if async_pat.mutex_usage { "Yes" } else { "No" }
        );

        if async_pat.spawn_calls > 0 {
            output.push_str("\n    → Concurrency management detected - group spawn logic");
        }

        output
    }

    fn format_error_handling(metrics: &RustPatternMetrics) -> String {
        if !metrics.has_error_handling() {
            return String::new();
        }

        Self::format_error_summary(&metrics.error_handling)
    }

    fn format_error_summary(error_handling: &ErrorHandlingSummary) -> String {
        let mut output = format!(
            "  Error Handling:\n    - Average ? operators per function: {:.1}",
            error_handling.question_mark_density
        );

        if error_handling.unwrap_count > 0 {
            output.push_str(&format!(
                "\n    ⚠ {} unwrap() calls detected - replace with proper error handling",
                error_handling.unwrap_count
            ));
        }

        output
    }

    fn format_builder_candidates(metrics: &RustPatternMetrics) -> String {
        if !metrics.has_builder_candidates() {
            return String::new();
        }

        format!(
            "  Builder Patterns:\n\
               - Candidates: {}\n\
               → Extract builder logic into separate module",
            metrics.builder_candidates.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_empty_analysis_produces_empty_string() {
        let analysis = PatternAnalysis::default();
        let output = PatternFormatter::format(&analysis);
        assert!(output.is_empty());
    }

    #[test]
    fn format_purity_with_functions() {
        let metrics = PurityMetrics {
            strictly_pure: 15,
            locally_pure: 10,
            read_only: 5,
            impure: 20,
            almost_pure: vec![],
        };

        let output = PatternFormatter::format_purity(&metrics);

        assert!(output.contains("PURITY ANALYSIS"));
        assert!(output.contains("Strictly Pure: 15"));
        assert!(output.contains("50%")); // (15 + 10) / 50 * 100
    }

    #[test]
    fn format_purity_empty_metrics() {
        let metrics = PurityMetrics::default();
        let output = PatternFormatter::format_purity(&metrics);
        assert!(output.is_empty());
    }

    #[test]
    fn format_handles_empty_examples() {
        let metrics = FrameworkPatternMetrics {
            patterns: vec![DetectedPattern {
                framework: "Test".into(),
                pattern_type: "Pattern".into(),
                count: 5,
                examples: vec![],
                recommendation: "Do something".into(),
            }],
        };

        let output = PatternFormatter::format_frameworks(&metrics);
        assert!(output.contains("(no examples)"));
        assert!(!output.contains("Examples: ,"));
    }
}
