//! Terminal formatting for priority analysis results
//!
//! This module provides formatted output for technical debt priorities,
//! including detailed recommendations and summary tables.

use crate::formatting::FormattingConfig;
use crate::output::unified::{classify_coupling, CouplingClassification};
use crate::priority::{UnifiedAnalysis, UnifiedDebtItem};

use crate::priority::formatter_verbosity as verbosity;

// Submodules (spec 205: organized by responsibility)
mod context;
mod dependencies;
mod helpers;
mod orchestrators;
pub mod pure;
mod recommendations;
mod sections;
pub mod summary;
pub mod writer;

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Default,     // Top 10 with clean formatting
    Top(usize),  // Top N items
    Tail(usize), // Bottom N items (lowest priority)
}

pub fn format_priorities(analysis: &UnifiedAnalysis, format: OutputFormat) -> String {
    format_priorities_with_verbosity(analysis, format, 0)
}

pub fn format_priorities_with_verbosity(
    analysis: &UnifiedAnalysis,
    format: OutputFormat,
    verbosity: u8,
) -> String {
    format_priorities_with_config(analysis, format, verbosity, FormattingConfig::default())
}

pub fn format_priorities_with_config(
    analysis: &UnifiedAnalysis,
    format: OutputFormat,
    verbosity: u8,
    config: FormattingConfig,
) -> String {
    match format {
        OutputFormat::Default => {
            orchestrators::format_default_with_config(analysis, 10, verbosity, config)
        }
        OutputFormat::Top(n) => {
            orchestrators::format_default_with_config(analysis, n, verbosity, config)
        }
        OutputFormat::Tail(n) => {
            orchestrators::format_tail_with_config(analysis, n, verbosity, config)
        }
    }
}

/// Format priorities with tiered display for terminal output (summary mode)
pub fn format_summary_terminal(analysis: &UnifiedAnalysis, limit: usize, verbosity: u8) -> String {
    summary::format_summary_terminal(analysis, limit, verbosity)
}

// Terminal formatting functions moved to summary.rs

// Unused formatting functions removed (format_tail, format_detailed)

pub fn format_priority_item(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    has_coverage_data: bool,
) {
    // Use pure functional formatting with writer pattern
    let formatted = pure::format_priority_item(
        rank,
        item,
        0, // default verbosity
        FormattingConfig::default(),
        has_coverage_data,
    );

    // Write to output buffer (I/O at edges)
    let mut buffer = Vec::new();
    let _ = writer::write_priority_item(&mut buffer, &formatted);
    if let Ok(result) = String::from_utf8(buffer) {
        output.push_str(&result);
    }
}

// Re-export helper functions from helpers module (spec 205)
pub use helpers::{
    extract_complexity_info, extract_dependency_info, format_debt_type, format_impact, format_role,
};

// === Coupling display helpers (spec 202) ===

/// Format truncated list with (+N more) suffix for display
fn format_truncated_list(items: &[String], max: usize) -> String {
    if items.is_empty() {
        return String::new();
    }
    if items.len() <= max {
        items.join(", ")
    } else {
        let shown: Vec<_> = items.iter().take(max).collect();
        format!(
            "{} (+{} more)",
            shown
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            items.len() - max
        )
    }
}

/// Get the display label for a coupling classification
fn coupling_classification_label(classification: &CouplingClassification) -> &'static str {
    match classification {
        // Spec 269: Architecture-aware classifications
        CouplingClassification::WellTestedCore => "well-tested core",
        CouplingClassification::StableFoundation => "stable foundation",
        CouplingClassification::UnstableHighCoupling => "unstable high coupling",
        CouplingClassification::ArchitecturalHub => "architectural hub",
        // Existing classifications
        CouplingClassification::StableCore => "stable core",
        CouplingClassification::UtilityModule => "utility",
        CouplingClassification::LeafModule => "leaf",
        CouplingClassification::Isolated => "isolated",
        CouplingClassification::HighlyCoupled => "highly coupled",
    }
}

// Format file-level priority items with detailed information
pub fn format_file_priority_item_with_verbosity(
    output: &mut String,
    rank: usize,
    item: &crate::priority::FileDebtItem,
    _config: FormattingConfig,
    _verbosity: u8,
) {
    use colored::*;
    use std::fmt::Write;

    // Determine severity based on score
    let severity = crate::priority::classification::Severity::from_score_100(item.score);
    let (severity_label, severity_color) = (severity.as_str(), severity.color());

    // Header section
    writeln!(
        output,
        "#{} {} [{}]",
        rank,
        format!("SCORE: {:.1}", item.score).bright_yellow(),
        severity_label.color(severity_color).bold()
    )
    .unwrap();

    // Location section (file-level)
    writeln!(
        output,
        "{} {}",
        "├─ LOCATION:".bright_blue(),
        item.metrics.path.display()
    )
    .unwrap();

    // Impact section
    writeln!(
        output,
        "{} {}",
        "├─ IMPACT:".bright_blue(),
        format!(
            "-{:.0} complexity, -{:.1} maintainability improvement",
            item.impact.complexity_reduction, item.impact.maintainability_improvement
        )
        .bright_cyan()
    )
    .unwrap();

    // File metrics section
    writeln!(
        output,
        "{} {} lines, {} functions, avg complexity: {:.1}",
        "├─ METRICS:".bright_blue(),
        item.metrics.total_lines,
        item.metrics.function_count,
        item.metrics.avg_complexity
    )
    .unwrap();

    // Coupling section (spec 202)
    let total_coupling = item.metrics.afferent_coupling + item.metrics.efferent_coupling;
    if total_coupling >= 2 {
        let classification = classify_coupling(
            item.metrics.afferent_coupling,
            item.metrics.efferent_coupling,
        );
        let label = coupling_classification_label(&classification);

        writeln!(
            output,
            "{} Ca={} ({}), Ce={}, I={:.2}",
            "├─ COUPLING:".bright_blue(),
            item.metrics.afferent_coupling,
            label,
            item.metrics.efferent_coupling,
            item.metrics.instability
        )
        .unwrap();

        // Dependents list (incoming)
        if !item.metrics.dependents.is_empty() {
            let display = format_truncated_list(&item.metrics.dependents, 3);
            writeln!(output, "   {} {}", "←".dimmed(), display).unwrap();
        }

        // Dependencies list (outgoing)
        if !item.metrics.dependencies_list.is_empty() {
            let display = format_truncated_list(&item.metrics.dependencies_list, 3);
            writeln!(output, "   {} {}", "→".dimmed(), display).unwrap();
        }
    }

    // God object details (if applicable)
    if let Some(ref god_analysis) = item.metrics.god_object_analysis {
        if god_analysis.is_god_object {
            writeln!(
                output,
                "{} {} methods, {} fields, {} responsibilities (score: {:.1})",
                "├─ GOD OBJECT:".bright_blue(),
                god_analysis.method_count,
                god_analysis.field_count,
                god_analysis.responsibility_count,
                god_analysis.god_object_score
            )
            .unwrap();

            // Show recommended splits if available
            if !god_analysis.recommended_splits.is_empty() {
                writeln!(
                    output,
                    "   {} {} recommended module splits",
                    "Suggested:".dimmed(),
                    god_analysis.recommended_splits.len()
                )
                .unwrap();
            }
        }
    }

    // Action section
    writeln!(
        output,
        "{} {}",
        "├─ ACTION:".bright_blue(),
        item.recommendation.bright_yellow()
    )
    .unwrap();

    // Rationale section
    let rationale = format_file_rationale(item);
    writeln!(
        output,
        "{} {}",
        "└─ WHY THIS MATTERS:".bright_blue(),
        rationale
    )
    .unwrap();
}

/// Generate rationale explaining why this file-level debt matters
fn format_file_rationale(item: &crate::priority::FileDebtItem) -> String {
    if let Some(ref god_analysis) = item.metrics.god_object_analysis {
        if god_analysis.is_god_object {
            let responsibilities = god_analysis.responsibility_count;
            let methods = god_analysis.method_count;

            if responsibilities > 5 {
                return format!(
                    "File has {} distinct responsibilities across {} methods. High coupling makes changes risky and testing difficult. Splitting by responsibility will improve maintainability and reduce change impact.",
                    responsibilities, methods
                );
            } else if methods > 50 {
                return format!(
                    "File contains {} methods with {} responsibilities. Large interface makes it difficult to understand and maintain. Extracting cohesive modules will improve clarity.",
                    methods, responsibilities
                );
            } else {
                return format!(
                    "File exhibits god object characteristics (score: {:.1}). Refactoring will improve separation of concerns and testability.",
                    god_analysis.god_object_score
                );
            }
        }
    }

    if item.metrics.total_complexity > 500 {
        format!(
            "High total complexity ({}) across {} functions (avg: {:.1}). Breaking into smaller modules will reduce cognitive load and improve maintainability.",
            item.metrics.total_complexity,
            item.metrics.function_count,
            item.metrics.avg_complexity
        )
    } else if item.metrics.total_lines > 1000 {
        format!(
            "Large file ({} lines) with {} functions. Size alone increases maintenance burden and makes navigation difficult.",
            item.metrics.total_lines,
            item.metrics.function_count
        )
    } else {
        "File-level refactoring will improve overall code organization and maintainability."
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Tests for format_truncated_list (spec 202) ===

    #[test]
    fn test_format_truncated_list_empty() {
        let items: Vec<String> = vec![];
        assert_eq!(format_truncated_list(&items, 3), "");
    }

    #[test]
    fn test_format_truncated_list_under_limit() {
        let items = vec!["a.rs".to_string(), "b.rs".to_string()];
        assert_eq!(format_truncated_list(&items, 3), "a.rs, b.rs");
    }

    #[test]
    fn test_format_truncated_list_at_limit() {
        let items = vec!["a.rs".to_string(), "b.rs".to_string(), "c.rs".to_string()];
        assert_eq!(format_truncated_list(&items, 3), "a.rs, b.rs, c.rs");
    }

    #[test]
    fn test_format_truncated_list_over_limit() {
        let items = vec![
            "a.rs".to_string(),
            "b.rs".to_string(),
            "c.rs".to_string(),
            "d.rs".to_string(),
            "e.rs".to_string(),
        ];
        assert_eq!(
            format_truncated_list(&items, 3),
            "a.rs, b.rs, c.rs (+2 more)"
        );
    }

    #[test]
    fn test_format_truncated_list_single_item() {
        let items = vec!["only.rs".to_string()];
        assert_eq!(format_truncated_list(&items, 3), "only.rs");
    }

    // === Tests for coupling_classification_label (spec 202) ===

    #[test]
    fn test_coupling_classification_labels() {
        assert_eq!(
            coupling_classification_label(&CouplingClassification::StableCore),
            "stable core"
        );
        assert_eq!(
            coupling_classification_label(&CouplingClassification::UtilityModule),
            "utility"
        );
        assert_eq!(
            coupling_classification_label(&CouplingClassification::LeafModule),
            "leaf"
        );
        assert_eq!(
            coupling_classification_label(&CouplingClassification::Isolated),
            "isolated"
        );
        assert_eq!(
            coupling_classification_label(&CouplingClassification::HighlyCoupled),
            "highly coupled"
        );
    }

    // === Tests for format_file_rationale (spec 205) ===

    fn create_test_file_debt_item(
        is_god_object: bool,
        responsibilities: usize,
        methods: usize,
        god_score: f64,
        total_complexity: u32,
        total_lines: usize,
    ) -> crate::priority::FileDebtItem {
        use crate::organization::{DetectionType, GodObjectAnalysis, GodObjectConfidence};
        use crate::priority::{FileDebtItem, FileDebtMetrics, FileImpact};
        use std::path::PathBuf;

        let god_object_analysis = if is_god_object {
            Some(GodObjectAnalysis {
                is_god_object: true,
                method_count: methods,
                field_count: 10,
                responsibility_count: responsibilities,
                lines_of_code: total_lines,
                complexity_sum: total_complexity,
                god_object_score: god_score,
                confidence: GodObjectConfidence::Definite,
                detection_type: DetectionType::GodClass,
                struct_name: None,
                struct_line: None,
                struct_location: None,
                responsibilities: Vec::new(),
                responsibility_method_counts: Default::default(),
                recommended_splits: Vec::new(),
                purity_distribution: None,
                module_structure: None,
                visibility_breakdown: None,
                domain_count: 0,
                domain_diversity: 0.0,
                struct_ratio: 0.0,
                analysis_method: Default::default(),
                cross_domain_severity: None,
                domain_diversity_metrics: None,
                aggregated_entropy: None,
                aggregated_error_swallowing_count: None,
                aggregated_error_swallowing_patterns: None,
                layering_impact: None,
                anti_pattern_report: None,
                complexity_metrics: None,
                trait_method_summary: None,
                weighted_method_count: None,
            })
        } else {
            None
        };

        FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from("test.rs"),
                total_lines,
                function_count: methods,
                class_count: 0,
                avg_complexity: if methods > 0 {
                    total_complexity as f64 / methods as f64
                } else {
                    0.0
                },
                max_complexity: 20,
                total_complexity,
                coverage_percent: 0.5,
                uncovered_lines: total_lines / 2,
                god_object_analysis,
                function_scores: Vec::new(),
                god_object_type: None,
                file_type: None,
                afferent_coupling: 0,
                efferent_coupling: 0,
                instability: 0.0,
                dependents: Vec::new(),
                dependencies_list: Vec::new(),
            },
            score: 50.0,
            priority_rank: 1,
            recommendation: "Test recommendation".to_string(),
            impact: FileImpact::default(),
        }
    }

    #[test]
    fn test_format_file_rationale_god_object_many_responsibilities() {
        let item = create_test_file_debt_item(true, 8, 40, 75.0, 200, 500);
        let rationale = format_file_rationale(&item);

        assert!(rationale.contains("8 distinct responsibilities"));
        assert!(rationale.contains("40 methods"));
        assert!(rationale.contains("Splitting by responsibility"));
    }

    #[test]
    fn test_format_file_rationale_god_object_many_methods() {
        let item = create_test_file_debt_item(true, 3, 60, 65.0, 300, 800);
        let rationale = format_file_rationale(&item);

        assert!(rationale.contains("60 methods"));
        assert!(rationale.contains("3 responsibilities"));
        assert!(rationale.contains("Extracting cohesive modules"));
    }

    #[test]
    fn test_format_file_rationale_god_object_default() {
        let item = create_test_file_debt_item(true, 3, 30, 55.0, 150, 400);
        let rationale = format_file_rationale(&item);

        assert!(rationale.contains("god object characteristics"));
        assert!(rationale.contains("score: 55.0"));
    }

    #[test]
    fn test_format_file_rationale_high_complexity() {
        let item = create_test_file_debt_item(false, 0, 25, 0.0, 550, 600);
        let rationale = format_file_rationale(&item);

        assert!(rationale.contains("High total complexity"));
        assert!(rationale.contains("550"));
        assert!(rationale.contains("25 functions"));
    }

    #[test]
    fn test_format_file_rationale_large_file() {
        let item = create_test_file_debt_item(false, 0, 20, 0.0, 100, 1200);
        let rationale = format_file_rationale(&item);

        assert!(rationale.contains("Large file"));
        assert!(rationale.contains("1200 lines"));
        assert!(rationale.contains("20 functions"));
    }

    #[test]
    fn test_format_file_rationale_default() {
        let item = create_test_file_debt_item(false, 0, 10, 0.0, 50, 200);
        let rationale = format_file_rationale(&item);

        assert!(rationale.contains("File-level refactoring"));
        assert!(rationale.contains("maintainability"));
    }
}
