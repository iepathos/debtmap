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

pub(crate) fn format_main_factors_with_coverage(
    unified_score: &crate::priority::UnifiedScore,
    debt_type: &crate::priority::DebtType,
    transitive_coverage: Option<&crate::priority::coverage_propagation::TransitiveCoverage>,
) -> String {
    let weights = crate::config::get_scoring_weights();
    let mut factors = vec![];

    // Show coverage info - both good and bad coverage are important factors
    if let Some(trans_cov) = transitive_coverage {
        let coverage_pct = trans_cov.direct * 100.0;
        if coverage_pct >= 95.0 {
            factors.push(format!("Excellent coverage {:.1}%", coverage_pct));
        } else if coverage_pct >= 80.0 {
            factors.push(format!("Good coverage {:.1}%", coverage_pct));
        } else if unified_score.coverage_factor > 3.0 {
            factors.push(format!(
                "Line coverage {:.1}% (weight: {:.0}%)",
                coverage_pct,
                weights.coverage * 100.0
            ));
        }
    } else if unified_score.coverage_factor > 3.0 {
        factors.push(format!(
            "No coverage data (weight: {:.0}%)",
            weights.coverage * 100.0
        ));
    }
    if unified_score.complexity_factor > 5.0 {
        factors.push(format!(
            "Complexity (weight: {:.0}%)",
            weights.complexity * 100.0
        ));
    } else if unified_score.complexity_factor > 3.0 {
        factors.push("Moderate complexity".to_string());
    }

    if unified_score.dependency_factor > 5.0 {
        factors.push(format!(
            "Critical path (weight: {:.0}%)",
            weights.dependency * 100.0
        ));
    }
    // Organization factor removed per spec 58 - redundant with complexity factor

    // Add specific factors for various debt types
    match debt_type {
        crate::priority::DebtType::NestedLoops { depth, .. } => {
            factors.push("Complexity impact (High)".to_string());
            factors.push(format!("{} level nested loops", depth));
        }
        crate::priority::DebtType::BlockingIO { operation, .. } => {
            factors.push("Resource management issue".to_string());
            factors.push(format!("Blocking {}", operation));
        }
        crate::priority::DebtType::AllocationInefficiency { pattern, .. } => {
            factors.push("Resource management issue".to_string());
            factors.push(format!("Allocation: {}", pattern));
        }
        _ => {} // No additional factors for other debt types
    }

    if !factors.is_empty() {
        format!("*Main factors: {}*\n", factors.join(", "))
    } else {
        String::new()
    }
}
