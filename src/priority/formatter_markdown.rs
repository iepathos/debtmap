use crate::priority::{DebtType, UnifiedAnalysis, UnifiedDebtItem};
use std::fmt::Write;

/// Format priorities for markdown output without ANSI color codes
pub fn format_priorities_markdown(
    analysis: &UnifiedAnalysis,
    limit: usize,
    verbosity: u8,
) -> String {
    let mut output = String::new();

    writeln!(output, "# Priority Technical Debt Fixes\n").unwrap();

    let top_items = analysis.get_top_priorities(limit);
    let count = top_items.len().min(limit);

    writeln!(
        output,
        "## Top {} Recommendations (by unified priority)\n",
        count
    )
    .unwrap();

    for (idx, item) in top_items.iter().enumerate() {
        format_priority_item_markdown(&mut output, idx + 1, item, verbosity);
        writeln!(output).unwrap();
    }

    // Add summary
    writeln!(output, "---\n").unwrap();
    writeln!(
        output,
        "**Total Debt Score:** {:.0}",
        analysis.total_debt_score
    )
    .unwrap();

    if let Some(coverage) = analysis.overall_coverage {
        writeln!(output, "**Overall Coverage:** {:.2}%", coverage).unwrap();
    }

    output
}

fn format_priority_item_markdown(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    verbosity: u8,
) {
    let severity = get_severity_label(item.unified_score.final_score);

    // Header with rank and score
    writeln!(
        output,
        "### #{} - Score: {:.1} [{}]",
        rank, item.unified_score.final_score, severity
    )
    .unwrap();

    // Show score breakdown for verbosity >= 2
    if verbosity >= 2 {
        let weights = crate::config::get_scoring_weights();
        writeln!(output, "\n#### Score Calculation\n").unwrap();
        writeln!(output, "| Component | Value | Weight | Contribution |").unwrap();
        writeln!(output, "|-----------|-------|--------|--------------|").unwrap();
        writeln!(
            output,
            "| Complexity | {:.1} | {:.0}% | {:.2} |",
            item.unified_score.complexity_factor,
            weights.complexity * 100.0,
            item.unified_score.complexity_factor * weights.complexity
        )
        .unwrap();
        writeln!(
            output,
            "| Coverage | {:.1} | {:.0}% | {:.2} |",
            item.unified_score.coverage_factor,
            weights.coverage * 100.0,
            item.unified_score.coverage_factor * weights.coverage
        )
        .unwrap();
        writeln!(
            output,
            "| ROI | {:.1} | {:.0}% | {:.2} |",
            item.unified_score.roi_factor,
            weights.roi * 100.0,
            item.unified_score.roi_factor * weights.roi
        )
        .unwrap();
        writeln!(
            output,
            "| Semantic | {:.1} | {:.0}% | {:.2} |",
            item.unified_score.semantic_factor,
            weights.semantic * 100.0,
            item.unified_score.semantic_factor * weights.semantic
        )
        .unwrap();
        writeln!(
            output,
            "| Dependency | {:.1} | {:.0}% | {:.2} |",
            item.unified_score.dependency_factor,
            weights.dependency * 100.0,
            item.unified_score.dependency_factor * weights.dependency
        )
        .unwrap();

        let base_score = item.unified_score.complexity_factor * weights.complexity
            + item.unified_score.coverage_factor * weights.coverage
            + item.unified_score.roi_factor * weights.roi
            + item.unified_score.semantic_factor * weights.semantic
            + item.unified_score.dependency_factor * weights.dependency;

        writeln!(output).unwrap();
        writeln!(output, "- **Base Score:** {:.2}", base_score).unwrap();
        writeln!(
            output,
            "- **Role Adjustment:** ×{:.2}",
            item.unified_score.role_multiplier
        )
        .unwrap();
        writeln!(
            output,
            "- **Final Score:** {:.2}",
            item.unified_score.final_score
        )
        .unwrap();
        writeln!(output).unwrap();
    } else if verbosity >= 1 {
        // Show main contributing factors for verbosity >= 1
        let weights = crate::config::get_scoring_weights();
        let mut factors = vec![];

        if item.unified_score.coverage_factor > 3.0 {
            factors.push(format!("Coverage gap ({:.0}%)", weights.coverage * 100.0));
        }
        if item.unified_score.roi_factor > 7.0 {
            factors.push(format!("High ROI ({:.0}%)", weights.roi * 100.0));
        }
        if item.unified_score.dependency_factor > 5.0 {
            factors.push(format!(
                "Critical path ({:.0}%)",
                weights.dependency * 100.0
            ));
        }
        if item.unified_score.complexity_factor > 5.0 {
            factors.push(format!("Complexity ({:.0}%)", weights.complexity * 100.0));
        }

        if !factors.is_empty() {
            writeln!(output, "*Main factors: {}*\n", factors.join(", ")).unwrap();
        }
    }

    // Location and type
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
            let caller_list = if item.upstream_callers.len() > 3 {
                format!(
                    "{}, ... ({} more)",
                    item.upstream_callers
                        .iter()
                        .take(3)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", "),
                    item.upstream_callers.len() - 3
                )
            } else {
                item.upstream_callers.to_vec().join(", ")
            };
            writeln!(output, "- **Called by:** {}", caller_list).unwrap();
        }

        if !item.downstream_callees.is_empty() && verbosity >= 2 {
            let callee_list = if item.downstream_callees.len() > 3 {
                format!(
                    "{}, ... ({} more)",
                    item.downstream_callees
                        .iter()
                        .take(3)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", "),
                    item.downstream_callees.len() - 3
                )
            } else {
                item.downstream_callees.to_vec().join(", ")
            };
            writeln!(output, "- **Calls:** {}", callee_list).unwrap();
        }
    }

    // Rationale
    writeln!(output, "\n**Why:** {}", item.recommendation.rationale).unwrap();
}

fn get_severity_label(score: f64) -> &'static str {
    match score {
        s if s >= 9.0 => "CRITICAL",
        s if s >= 7.0 => "HIGH",
        s if s >= 5.0 => "MEDIUM",
        s if s >= 3.0 => "LOW",
        _ => "MINIMAL",
    }
}

fn format_debt_type(debt_type: &DebtType) -> &'static str {
    match debt_type {
        DebtType::TestingGap { .. } => "Testing Gap",
        DebtType::ComplexityHotspot { .. } => "Complexity",
        DebtType::DeadCode { .. } => "Dead Code",
        DebtType::Orchestration { .. } => "Orchestration",
        DebtType::Duplication { .. } => "Duplication",
        DebtType::Risk { .. } => "Risk",
        DebtType::TestComplexityHotspot { .. } => "Test Complexity",
        DebtType::TestTodo { .. } => "Test TODO",
        DebtType::TestDuplication { .. } => "Test Duplication",
    }
}

fn format_impact(impact: &crate::priority::ImpactMetrics) -> String {
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

fn extract_complexity_info(debt_type: &DebtType) -> Option<String> {
    match debt_type {
        DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive,
        }
        | DebtType::TestComplexityHotspot {
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
        DebtType::DeadCode { cyclomatic, .. } => Some(format!("cyclomatic={}", cyclomatic)),
        _ => None,
    }
}
