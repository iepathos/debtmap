use crate::priority::UnifiedDebtItem;
use colored::*;
use std::fmt::Write;

pub fn format_priority_item_with_verbosity(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    verbosity: u8,
) {
    let severity = crate::priority::formatter::get_severity_label(item.unified_score.final_score);
    let severity_color =
        crate::priority::formatter::get_severity_color(item.unified_score.final_score);

    // Base score line - add score breakdown for verbosity >= 1
    if verbosity >= 1 {
        // Get scoring weights for display
        let weights = crate::config::get_scoring_weights();

        // Calculate main contributing factors
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

        writeln!(
            output,
            "#{} {} [{}]",
            rank.to_string().bright_cyan().bold(),
            format!("SCORE: {:.1}", item.unified_score.final_score).bright_white(),
            severity.color(severity_color).bold()
        )
        .unwrap();

        if !factors.is_empty() {
            writeln!(output, "   ↳ Main factors: {}", factors.join(", ").dimmed()).unwrap();
        }
    } else {
        writeln!(
            output,
            "#{} {} [{}]",
            rank.to_string().bright_cyan().bold(),
            format!("SCORE: {:.1}", item.unified_score.final_score).bright_white(),
            severity.color(severity_color).bold()
        )
        .unwrap();
    }

    // Show detailed calculation for verbosity >= 2
    if verbosity >= 2 {
        let weights = crate::config::get_scoring_weights();
        writeln!(output, "├─ SCORE CALCULATION:").unwrap();
        writeln!(output, "│  ├─ Base Components (Weighted):").unwrap();
        writeln!(
            output,
            "│  │  ├─ Complexity:  {:.1} × {:.0}% = {:.2}",
            item.unified_score.complexity_factor,
            weights.complexity * 100.0,
            item.unified_score.complexity_factor * weights.complexity
        )
        .unwrap();
        writeln!(
            output,
            "│  │  ├─ Coverage:    {:.1} × {:.0}% = {:.2}",
            item.unified_score.coverage_factor,
            weights.coverage * 100.0,
            item.unified_score.coverage_factor * weights.coverage
        )
        .unwrap();
        writeln!(
            output,
            "│  │  ├─ ROI:        {:.1} × {:.0}% = {:.2}",
            item.unified_score.roi_factor,
            weights.roi * 100.0,
            item.unified_score.roi_factor * weights.roi
        )
        .unwrap();
        writeln!(
            output,
            "│  │  ├─ Semantic:    {:.1} × {:.0}% = {:.2}",
            item.unified_score.semantic_factor,
            weights.semantic * 100.0,
            item.unified_score.semantic_factor * weights.semantic
        )
        .unwrap();
        writeln!(
            output,
            "│  │  └─ Dependency:  {:.1} × {:.0}% = {:.2}",
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

        writeln!(output, "│  ├─ Base Score: {:.2}", base_score).unwrap();
        writeln!(
            output,
            "│  ├─ Role Adjustment: ×{:.2}",
            item.unified_score.role_multiplier
        )
        .unwrap();
        writeln!(
            output,
            "│  └─ Final Score: {:.2}",
            item.unified_score.final_score
        )
        .unwrap();
    }

    // Rest of the item formatting remains the same
    writeln!(
        output,
        "├─ {}: {}:{} {}()",
        crate::priority::formatter::format_debt_type(&item.debt_type).bright_yellow(),
        item.location.file.display(),
        item.location.line,
        item.location.function.bright_green()
    )
    .unwrap();

    writeln!(
        output,
        "├─ ACTION: {}",
        item.recommendation.primary_action.bright_white()
    )
    .unwrap();

    writeln!(
        output,
        "├─ IMPACT: {}",
        crate::priority::formatter::format_impact(&item.expected_impact).bright_cyan()
    )
    .unwrap();

    // Add complexity details
    let (cyclomatic, cognitive, branch_count, nesting, length) =
        crate::priority::formatter::extract_complexity_info(item);
    if cyclomatic > 0 || cognitive > 0 {
        writeln!(
            output,
            "├─ COMPLEXITY: cyclomatic={}, branches={}, cognitive={}, nesting={}, lines={}",
            cyclomatic.to_string().dimmed(),
            branch_count.to_string().dimmed(),
            cognitive.to_string().dimmed(),
            nesting.to_string().dimmed(),
            length.to_string().dimmed()
        )
        .unwrap();
    }

    // Add dependency information
    let (upstream, downstream) = crate::priority::formatter::extract_dependency_info(item);
    if upstream > 0 || downstream > 0 {
        writeln!(
            output,
            "├─ DEPENDENCIES: {} upstream, {} downstream",
            upstream.to_string().dimmed(),
            downstream.to_string().dimmed()
        )
        .unwrap();

        // Add upstream callers if present
        if !item.upstream_callers.is_empty() {
            let callers_display = if item.upstream_callers.len() <= 3 {
                item.upstream_callers.join(", ")
            } else {
                format!(
                    "{}, ... ({} more)",
                    item.upstream_callers[..3].join(", "),
                    item.upstream_callers.len() - 3
                )
            };
            writeln!(output, "│  ├─ CALLERS: {}", callers_display.bright_blue()).unwrap();
        }

        // Add downstream callees if present
        if !item.downstream_callees.is_empty() {
            let callees_display = if item.downstream_callees.len() <= 3 {
                item.downstream_callees.join(", ")
            } else {
                format!(
                    "{}, ... ({} more)",
                    item.downstream_callees[..3].join(", "),
                    item.downstream_callees.len() - 3
                )
            };
            writeln!(output, "│  └─ CALLS: {}", callees_display.bright_magenta()).unwrap();
        }
    }

    // Add rationale
    writeln!(output, "└─ WHY: {}", item.recommendation.rationale.dimmed()).unwrap();
}
