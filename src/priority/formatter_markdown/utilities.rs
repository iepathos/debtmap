//! Pure utility functions for markdown formatting
//!
//! This module contains pure logic helpers with no I/O - these are
//! easily testable functions that perform categorization, extraction,
//! and data transformation.

use crate::priority::classification::Severity;
use crate::priority::DebtType;

/// Helper to get file extension from path
pub(crate) fn get_file_extension(path: &std::path::Path) -> &str {
    path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("unknown")
}

pub(crate) fn score_category(lines: usize) -> &'static str {
    match lines {
        0..=200 => "LOW",
        201..=500 => "MODERATE",
        501..=1000 => "HIGH",
        _ => "CRITICAL",
    }
}

pub(crate) fn function_category(count: usize) -> &'static str {
    match count {
        0..=10 => "LOW",
        11..=20 => "MODERATE",
        21..=50 => "HIGH",
        _ => "EXCESSIVE",
    }
}

pub(crate) fn complexity_category(avg: f64) -> &'static str {
    match avg as usize {
        0..=5 => "LOW",
        6..=10 => "MODERATE",
        11..=20 => "HIGH",
        _ => "VERY HIGH",
    }
}

pub(crate) fn format_file_impact(impact: &crate::priority::FileImpact) -> String {
    let mut parts = vec![];

    if impact.complexity_reduction > 0.0 {
        parts.push(format!(
            "Reduce complexity by {:.0}%",
            impact.complexity_reduction
        ));
    }
    if impact.test_effort > 0.0 {
        parts.push(format!("Test effort: {:.1}", impact.test_effort));
    }
    if impact.maintainability_improvement > 0.0 {
        parts.push("Enable parallel development".to_string());
    }

    if parts.is_empty() {
        "No measurable impact".to_string()
    } else {
        parts.join(", ")
    }
}

/// Get severity label using shared classification (Spec 202)
pub(crate) fn get_severity_label(score: f64) -> &'static str {
    Severity::from_score(score).as_str()
}

pub(crate) fn format_debt_type(debt_type: &DebtType) -> &'static str {
    match debt_type {
        DebtType::TestingGap { .. } => "Testing Gap",
        DebtType::ComplexityHotspot { .. } => "Complexity",
        DebtType::DeadCode { .. } => "Dead Code",
        DebtType::Duplication { .. } => "Duplication",
        DebtType::Risk { .. } => "Risk",
        DebtType::TestComplexityHotspot { .. } => "Test Complexity",
        DebtType::TestTodo { .. } => "Test TODO",
        DebtType::TestDuplication { .. } => "Test Duplication",
        DebtType::ErrorSwallowing { .. } => "Error Swallowing",
        // Resource Management debt types
        DebtType::AllocationInefficiency { .. } => "Allocation Inefficiency",
        DebtType::StringConcatenation { .. } => "String Concatenation",
        DebtType::NestedLoops { .. } => "Nested Loops",
        DebtType::BlockingIO { .. } => "Blocking I/O",
        DebtType::SuboptimalDataStructure { .. } => "Suboptimal Data Structure",
        // Organization debt types
        DebtType::GodObject { .. } => "God Object",
        DebtType::GodModule { .. } => "God Module",
        DebtType::FeatureEnvy { .. } => "Feature Envy",
        DebtType::PrimitiveObsession { .. } => "Primitive Obsession",
        DebtType::MagicValues { .. } => "Magic Values",
        // Testing quality debt types
        DebtType::AssertionComplexity { .. } => "Assertion Complexity",
        DebtType::FlakyTestPattern { .. } => "Flaky Test Pattern",
        // Resource management debt types
        DebtType::AsyncMisuse { .. } => "Async Misuse",
        DebtType::ResourceLeak { .. } => "Resource Leak",
        DebtType::CollectionInefficiency { .. } => "Collection Inefficiency",
        // Type organization (Spec 187)
        DebtType::ScatteredType { .. } => "Scattered Type",
        DebtType::OrphanedFunctions { .. } => "Orphaned Functions",
        DebtType::UtilitiesSprawl { .. } => "Utilities Sprawl",
        // Default for legacy variants
        _ => "Other",
    }
}

pub(crate) fn format_impact(impact: &crate::priority::ImpactMetrics) -> String {
    let mut parts = vec![];

    if impact.complexity_reduction > 0.0 {
        parts.push(format!("-{:.1} complexity", impact.complexity_reduction));
    }
    if impact.risk_reduction > 0.1 {
        parts.push(format!("-{:.1} risk", impact.risk_reduction));
    }
    if impact.coverage_improvement > 0.01 {
        parts.push(format!("+{:.0}% coverage", impact.coverage_improvement));
    }
    if impact.lines_reduction > 0 {
        parts.push(format!("-{} lines", impact.lines_reduction));
    }

    if parts.is_empty() {
        "No measurable impact".to_string()
    } else {
        parts.join(", ")
    }
}

pub(crate) fn extract_complexity_info(debt_type: &DebtType) -> Option<String> {
    match debt_type {
        DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive,
            adjusted_cyclomatic,
        } => {
            // Show adjusted complexity if available (spec 182)
            if let Some(adjusted) = adjusted_cyclomatic {
                Some(format!(
                    "cyclomatic={} (adj={}), cognitive={}",
                    cyclomatic, adjusted, cognitive
                ))
            } else {
                Some(format!(
                    "cyclomatic={}, cognitive={}",
                    cyclomatic, cognitive
                ))
            }
        }
        DebtType::TestComplexityHotspot {
            cyclomatic,
            cognitive,
            ..
        } => Some(format!(
            "cyclomatic={}, cognitive={}",
            cyclomatic, cognitive
        )),
        DebtType::TestingGap {
            cyclomatic,
            cognitive,
            ..
        } => Some(format!(
            "cyclomatic={}, cognitive={}",
            cyclomatic, cognitive
        )),
        DebtType::Risk { .. } => None,
        DebtType::DeadCode {
            cyclomatic,
            cognitive,
            ..
        } => Some(format!(
            "cyclomatic={}, cognitive={}",
            cyclomatic, cognitive
        )),
        _ => None,
    }
}

pub(crate) fn format_dependency_list(
    items: &[String],
    max_shown: usize,
    list_type: &str,
) -> String {
    if items.is_empty() {
        return String::new();
    }

    let list = if items.len() > max_shown {
        format!(
            "{}, ... ({} more)",
            items
                .iter()
                .take(max_shown)
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
            items.len() - max_shown
        )
    } else {
        items.to_vec().join(", ")
    };

    format!("- **{}:** {}", list_type, list)
}
