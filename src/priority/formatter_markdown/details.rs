//! Detailed formatting for markdown output
//!
//! Handles detailed item formatting including score breakdowns,
//! dependency information, and comprehensive item details

use crate::priority::UnifiedDebtItem;
use std::fmt::Write;

use super::utilities::{
    extract_complexity_info, format_debt_type, format_dependency_list, format_impact,
    get_severity_label,
};

pub(crate) fn format_priority_item_markdown(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    verbosity: u8,
) {
    let severity = get_severity_label(item.unified_score.final_score);

    // Header with rank, tier, and score
    let tier_label = item
        .tier
        .as_ref()
        .map(|t| format!("[{}] ", t.short_label()))
        .unwrap_or_default();

    writeln!(
        output,
        "### #{} {}Score: {:.1} [{}]",
        rank, tier_label, item.unified_score.final_score, severity
    )
    .unwrap();

    // Show score breakdown for verbosity >= 2
    if verbosity >= 2 {
        output.push_str(&format_score_breakdown_with_coverage(
            &item.unified_score,
            item.transitive_coverage.as_ref(),
        ));
    } else if verbosity >= 1 {
        // Show main contributing factors for verbosity >= 1
        output.push_str(&format_main_factors_with_coverage(
            &item.unified_score,
            &item.debt_type,
            item.transitive_coverage.as_ref(),
        ));
    }

    // Location and type
    // Location line (spec 181: clean location line without context tag)
    writeln!(
        output,
        "**Type:** {} | **Location:** `{}:{} {}()`",
        format_debt_type(&item.debt_type),
        item.location.file.display(),
        item.location.line,
        item.location.function
    )
    .unwrap();

    // Action and impact
    writeln!(output, "**Action:** {}", item.recommendation.primary_action).unwrap();
    writeln!(
        output,
        "**Impact:** {}",
        format_impact(&item.expected_impact)
    )
    .unwrap();

    // Complexity details
    if let Some(complexity) = extract_complexity_info(&item.debt_type) {
        writeln!(output, "**Complexity:** {}", complexity).unwrap();
    }

    // Dependencies
    if verbosity >= 1 {
        writeln!(output, "\n#### Dependencies").unwrap();
        writeln!(
            output,
            "- **Upstream:** {} | **Downstream:** {}",
            item.upstream_dependencies, item.downstream_dependencies
        )
        .unwrap();

        if !item.upstream_callers.is_empty() && verbosity >= 2 {
            let caller_info = format_dependency_list(&item.upstream_callers, 3, "Called by");
            if !caller_info.is_empty() {
                writeln!(output, "{}", caller_info).unwrap();
            }
        }

        if !item.downstream_callees.is_empty() && verbosity >= 2 {
            let callee_info = format_dependency_list(&item.downstream_callees, 3, "Calls");
            if !callee_info.is_empty() {
                writeln!(output, "{}", callee_info).unwrap();
            }
        }
    }

    // Rationale
    writeln!(output, "\n**Why:** {}", item.recommendation.rationale).unwrap();

    // Context suggestions (spec 263)
    if verbosity >= 2 {
        if let Some(context) = &item.context_suggestion {
            format_context_suggestion(output, context);
        }
    }
}

/// Format context suggestion section for markdown output (spec 263).
fn format_context_suggestion(
    output: &mut String,
    context: &crate::priority::context::ContextSuggestion,
) {
    writeln!(
        output,
        "\n#### Context to Read ({} lines, {:.0}% confidence)",
        context.total_lines,
        context.completeness_confidence * 100.0
    )
    .unwrap();

    writeln!(
        output,
        "**Primary:** {}:{}-{} {}",
        context.primary.file.display(),
        context.primary.start_line,
        context.primary.end_line,
        context
            .primary
            .symbol
            .as_ref()
            .map(|s| format!("({})", s))
            .unwrap_or_default()
    )
    .unwrap();

    if !context.related.is_empty() {
        writeln!(output, "\n**Related:**").unwrap();
        for rel in &context.related {
            writeln!(
                output,
                "- {}:{}-{} ({}) - {}",
                rel.range.file.display(),
                rel.range.start_line,
                rel.range.end_line,
                rel.relationship,
                rel.reason
            )
            .unwrap();
        }
    }
}

pub(crate) fn format_score_breakdown_with_coverage(
    unified_score: &crate::priority::UnifiedScore,
    transitive_coverage: Option<&crate::priority::coverage_propagation::TransitiveCoverage>,
) -> String {
    let weights = crate::config::get_scoring_weights();
    let mut output = String::new();

    writeln!(&mut output, "\n#### Score Calculation\n").unwrap();
    writeln!(
        &mut output,
        "| Component | Value | Weight | Contribution | Details |"
    )
    .unwrap();
    writeln!(
        &mut output,
        "|-----------|-------|--------|--------------|----------|"
    )
    .unwrap();
    writeln!(
        &mut output,
        "| Complexity | {:.1} | {:.0}% | {:.2} | |",
        unified_score.complexity_factor,
        weights.complexity * 100.0,
        unified_score.complexity_factor * weights.complexity
    )
    .unwrap();

    // Add coverage details if available
    let coverage_details = if let Some(trans_cov) = transitive_coverage {
        format!("Line: {:.2}%", trans_cov.direct * 100.0)
    } else {
        "No data".to_string()
    };
    writeln!(
        &mut output,
        "| Coverage | {:.1} | {:.0}% | {:.2} | {} |",
        unified_score.coverage_factor,
        weights.coverage * 100.0,
        unified_score.coverage_factor * weights.coverage,
        coverage_details
    )
    .unwrap();
    // Semantic and ROI factors removed per spec 55 and 58
    writeln!(
        &mut output,
        "| Dependency | {:.1} | {:.0}% | {:.2} | |",
        unified_score.dependency_factor,
        weights.dependency * 100.0,
        unified_score.dependency_factor * weights.dependency
    )
    .unwrap();

    // Organization factor removed per spec 58 - redundant with complexity factor

    // New weights after removing security: complexity, coverage, dependency
    let base_score = unified_score.complexity_factor * weights.complexity
        + unified_score.coverage_factor * weights.coverage
        + unified_score.dependency_factor * weights.dependency;

    writeln!(&mut output).unwrap();
    writeln!(&mut output, "- **Base Score:** {:.2}", base_score).unwrap();
    writeln!(
        &mut output,
        "- **Role Adjustment:** ×{:.2}",
        unified_score.role_multiplier
    )
    .unwrap();
    writeln!(
        &mut output,
        "- **Final Score:** {:.2}",
        unified_score.final_score
    )
    .unwrap();
    writeln!(&mut output).unwrap();

    output
}

/// Extract coverage factor description based on coverage percentage and score.
fn coverage_factor(
    transitive_coverage: Option<&crate::priority::coverage_propagation::TransitiveCoverage>,
    coverage_factor_score: f64,
    coverage_weight: f64,
) -> Option<String> {
    match transitive_coverage {
        Some(trans_cov) => {
            let pct = trans_cov.direct * 100.0;
            if pct >= 95.0 {
                Some(format!("Excellent coverage {:.1}%", pct))
            } else if pct >= 80.0 {
                Some(format!("Good coverage {:.1}%", pct))
            } else if coverage_factor_score > 3.0 {
                Some(format!(
                    "Line coverage {:.1}% (weight: {:.0}%)",
                    pct,
                    coverage_weight * 100.0
                ))
            } else {
                None
            }
        }
        None if coverage_factor_score > 3.0 => Some(format!(
            "No coverage data (weight: {:.0}%)",
            coverage_weight * 100.0
        )),
        None => None,
    }
}

/// Extract complexity factor description based on score threshold.
fn complexity_factor(complexity_score: f64, complexity_weight: f64) -> Option<String> {
    if complexity_score > 5.0 {
        Some(format!(
            "Complexity (weight: {:.0}%)",
            complexity_weight * 100.0
        ))
    } else if complexity_score > 3.0 {
        Some("Moderate complexity".to_string())
    } else {
        None
    }
}

/// Extract dependency factor description based on score threshold.
fn dependency_factor(dependency_score: f64, dependency_weight: f64) -> Option<String> {
    if dependency_score > 5.0 {
        Some(format!(
            "Critical path (weight: {:.0}%)",
            dependency_weight * 100.0
        ))
    } else {
        None
    }
}

/// Extract debt-type-specific factors.
fn debt_type_factors(debt_type: &crate::priority::DebtType) -> Vec<String> {
    match debt_type {
        crate::priority::DebtType::NestedLoops { depth, .. } => {
            vec![
                "Complexity impact (High)".to_string(),
                format!("{} level nested loops", depth),
            ]
        }
        crate::priority::DebtType::BlockingIO { operation, .. } => {
            vec![
                "Resource management issue".to_string(),
                format!("Blocking {}", operation),
            ]
        }
        crate::priority::DebtType::AllocationInefficiency { pattern, .. } => {
            vec![
                "Resource management issue".to_string(),
                format!("Allocation: {}", pattern),
            ]
        }
        _ => vec![],
    }
}

pub(crate) fn format_main_factors_with_coverage(
    unified_score: &crate::priority::UnifiedScore,
    debt_type: &crate::priority::DebtType,
    transitive_coverage: Option<&crate::priority::coverage_propagation::TransitiveCoverage>,
) -> String {
    let weights = crate::config::get_scoring_weights();

    let factors: Vec<String> = [
        coverage_factor(
            transitive_coverage,
            unified_score.coverage_factor,
            weights.coverage,
        ),
        complexity_factor(unified_score.complexity_factor, weights.complexity),
        dependency_factor(unified_score.dependency_factor, weights.dependency),
    ]
    .into_iter()
    .flatten()
    .chain(debt_type_factors(debt_type))
    .collect();

    if factors.is_empty() {
        String::new()
    } else {
        format!("*Main factors: {}*\n", factors.join(", "))
    }
}
