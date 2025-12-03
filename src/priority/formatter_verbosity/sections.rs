use crate::priority::unified_scorer::EntropyDetails;
use crate::priority::UnifiedDebtItem;
use colored::*;
use std::fmt::Write;

/// Pure function to classify dependency contribution
pub fn classify_dependency_contribution(dependency_factor: f64) -> &'static str {
    match dependency_factor {
        d if d > 10.0 => "CRITICAL PATH",
        d if d > 5.0 => "HIGH",
        d if d > 2.0 => "MEDIUM",
        _ => "LOW",
    }
}

/// Pure function to format callers display
pub fn format_callers_display(callers: &[String], max_display: usize) -> String {
    if callers.len() <= max_display {
        callers.join(", ")
    } else {
        format!(
            "{}, ... ({} more)",
            callers[..max_display].join(", "),
            callers.len() - max_display
        )
    }
}

/// Pure function to format callees display
pub fn format_callees_display(callees: &[String], max_display: usize) -> String {
    if callees.len() <= max_display {
        callees.join(", ")
    } else {
        format!(
            "{}, ... ({} more)",
            callees[..max_display].join(", "),
            callees.len() - max_display
        )
    }
}

/// Pure function to calculate score factors
pub struct ScoreFactors {
    pub coverage_gap: f64,
    pub coverage_pct: f64,
    pub coverage_factor: f64,
    pub complexity_factor: f64,
    pub dependency_factor: f64,
}

pub fn calculate_score_factors(item: &UnifiedDebtItem) -> ScoreFactors {
    let (coverage_gap, coverage_pct) = if let Some(ref trans_cov) = item.transitive_coverage {
        let pct = trans_cov.direct;
        (1.0 - pct, pct)
    } else {
        (1.0, 0.0)
    };

    ScoreFactors {
        coverage_gap,
        coverage_pct,
        coverage_factor: (coverage_gap.powf(1.5) + 0.1).max(0.1),
        complexity_factor: item.unified_score.complexity_factor,
        dependency_factor: ((item.unified_score.dependency_factor + 1.0).sqrt() / 2.0).min(1.0),
    }
}

/// Pure function to format coverage detail string
pub fn format_coverage_detail(has_coverage: bool, gap: f64, pct: f64) -> String {
    if has_coverage {
        format!(" (gap: {:.1}%, coverage: {:.1}%)", gap * 100.0, pct * 100.0)
    } else {
        " (no coverage data)".to_string()
    }
}

/// Pure function to format complexity detail
pub fn format_complexity_detail(entropy: &Option<EntropyDetails>) -> String {
    if let Some(ref e) = entropy {
        format!(" (entropy-adjusted from {})", e.original_complexity)
    } else {
        String::new()
    }
}

/// Format score calculation section for verbosity >= 2
pub fn format_score_calculation_section(
    item: &UnifiedDebtItem,
    _formatter: &crate::formatting::ColoredFormatter,
) -> Vec<String> {
    let mut lines = Vec::new();
    let tree_branch = "-";
    let tree_sub_branch = "  -";
    let tree_pipe = " ";

    lines.push(format!(
        "{} {}",
        tree_branch,
        "SCORE CALCULATION:".bright_blue()
    ));
    lines.push(format!("{} Weighted Sum Model:", tree_sub_branch));

    // Calculate multiplicative factors for display
    let factors = calculate_score_factors(item);
    let coverage_detail = format_coverage_detail(
        item.transitive_coverage.is_some(),
        factors.coverage_gap,
        factors.coverage_pct,
    );

    // Add role-based coverage adjustment indicator for entry points
    let role_coverage_indicator = if matches!(
        item.function_role,
        crate::priority::FunctionRole::EntryPoint
    ) {
        " (entry point - integration tested, lower unit coverage expected)"
    } else {
        ""
    };

    lines.push(format!(
        "{}  {} Coverage Score: {:.1} × 40% = {:.2}{}{}",
        tree_pipe,
        "-",
        factors.coverage_factor * 10.0, // Convert to 0-100 scale
        factors.coverage_factor * 10.0 * 0.4,
        coverage_detail,
        role_coverage_indicator
    ));

    // Show complexity score
    let complexity_detail = format_complexity_detail(&item.entropy_details);
    lines.push(format!(
        "{}  {} Complexity Score: {:.1} × 40% = {:.2}{}",
        tree_pipe,
        "-",
        factors.complexity_factor * 10.0, // Convert to 0-100 scale
        factors.complexity_factor * 10.0 * 0.40,
        complexity_detail
    ));

    // Show dependency score
    lines.push(format!(
        "{}  {} Dependency Score: {:.1} × 20% = {:.2} ({} callers)",
        tree_pipe,
        "-",
        factors.dependency_factor * 10.0, // Convert to 0-100 scale
        factors.dependency_factor * 10.0 * 0.20,
        item.upstream_callers.len() // Display actual caller count, not normalized score
    ));

    // Calculate weighted sum base score
    let coverage_contribution = factors.coverage_factor * 10.0 * 0.4;
    let complexity_contribution = factors.complexity_factor * 10.0 * 0.4;
    let dependency_contribution = factors.dependency_factor * 10.0 * 0.2;
    let base_score = coverage_contribution + complexity_contribution + dependency_contribution;

    lines.push(format!(
        "{}  {} Base Score: {:.2} + {:.2} + {:.2} = {:.2}",
        tree_pipe,
        "-",
        coverage_contribution,
        complexity_contribution,
        dependency_contribution,
        base_score
    ));

    // Show entropy impact if present
    if let Some(ref entropy) = item.entropy_details {
        lines.push(format!(
            "{}  {} Entropy Impact: {:.0}% dampening (entropy: {:.2}, repetition: {:.0}%)",
            tree_pipe,
            "-",
            (1.0 - entropy.dampening_factor) * 100.0,
            entropy.entropy_score,
            entropy.pattern_repetition * 100.0
        ));
    }

    lines.push(format!(
        "{}  {} Role Adjustment: ×{:.2}",
        tree_pipe, "-", item.unified_score.role_multiplier
    ));

    lines.push(format!(
        "{}  {} Final Score: {:.2}",
        tree_pipe, "-", item.unified_score.final_score
    ));

    lines
}

/// Format call graph section for verbosity >= 2
pub fn format_call_graph_section(
    item: &UnifiedDebtItem,
    _formatter: &crate::formatting::ColoredFormatter,
) -> Vec<String> {
    let mut lines = Vec::new();
    let tree_pipe = " ";

    if !item.upstream_callers.is_empty() || !item.downstream_callees.is_empty() {
        lines.push(format!("{} {}", "-", "CALL GRAPH:".bright_blue()));

        if !item.upstream_callers.is_empty() {
            let callers = format_callers_display(&item.upstream_callers, 5);
            lines.push(format!("{}  {} Called by: {}", tree_pipe, "-", callers));
        }

        if !item.downstream_callees.is_empty() {
            let callees = format_callees_display(&item.downstream_callees, 5);
            lines.push(format!("{}  {} Calls: {}", tree_pipe, "-", callees));
        } else if !item.upstream_callers.is_empty() {
            // Change the last caller line to use └─ if there are no callees
            lines.push(format!(
                "{}  {} Dependencies: {} upstream, {} downstream",
                tree_pipe, "-", item.upstream_dependencies, item.downstream_dependencies
            ));
        }
    }

    lines
}

/// Format basic call graph info for verbosity level 0
pub fn format_basic_call_graph(
    output: &mut String,
    item: &UnifiedDebtItem,
    _formatter: &crate::formatting::ColoredFormatter,
) {
    let caller_count = item.upstream_callers.len();
    let callee_count = item.downstream_callees.len();

    // Only show if there's interesting call graph info
    if caller_count > 0 || callee_count > 0 {
        writeln!(
            output,
            "- {} {} caller{}, {} callee{}",
            "CALLS:".bright_blue(),
            caller_count,
            if caller_count == 1 { "" } else { "s" },
            callee_count,
            if callee_count == 1 { "" } else { "s" }
        )
        .unwrap();

        // Show if function is potentially dead code (no callers)
        if caller_count == 0 && callee_count > 0 {
            writeln!(
                output,
                "    ! {}",
                "No callers detected - may be dead code".yellow()
            )
            .unwrap();
        }
    }
}

/// Format implementation steps
pub fn format_implementation_steps(
    output: &mut String,
    steps: &[String],
    _formatter: &crate::formatting::ColoredFormatter,
) {
    if !steps.is_empty() {
        for (i, step) in steps.iter().enumerate() {
            let prefix = "   -";
            writeln!(
                output,
                "{} {}. {}",
                prefix,
                (i + 1).to_string().cyan(),
                step.bright_white()
            )
            .unwrap();
        }
    }
}

/// Format dependencies summary
pub fn format_dependencies_summary(
    output: &mut String,
    item: &UnifiedDebtItem,
    _formatter: &crate::formatting::ColoredFormatter,
    tree_pipe: &str,
) {
    let (upstream, downstream) = crate::priority::formatter::extract_dependency_info(item);

    if upstream > 0 || downstream > 0 {
        writeln!(
            output,
            "- {} {} upstream, {} downstream",
            "DEPENDENCIES:".bright_blue(),
            upstream.to_string().cyan(),
            downstream.to_string().cyan()
        )
        .unwrap();

        if !item.upstream_callers.is_empty() {
            let callers_display = format_callers_display(&item.upstream_callers, 3);
            writeln!(
                output,
                "{}  - CALLERS: {}",
                tree_pipe,
                callers_display.cyan()
            )
            .unwrap();
        }

        if !item.downstream_callees.is_empty() {
            let callees_display = format_callees_display(&item.downstream_callees, 3);
            writeln!(
                output,
                "{}  - CALLS: {}",
                tree_pipe,
                callees_display.bright_magenta()
            )
            .unwrap();
        }
    }
}

/// Format scoring breakdown for verbosity 1
pub fn format_scoring_breakdown(
    output: &mut String,
    item: &UnifiedDebtItem,
    _formatter: &crate::formatting::ColoredFormatter,
) {
    use super::complexity::classify_complexity_contribution;
    use super::coverage::classify_coverage_contribution;

    let coverage_contribution = classify_coverage_contribution(item);
    let complexity_contribution =
        classify_complexity_contribution(item.unified_score.complexity_factor);
    let dependency_contribution =
        classify_dependency_contribution(item.unified_score.dependency_factor);

    writeln!(
        output,
        "- {} Coverage: {} | Complexity: {} | Dependencies: {}",
        "SCORING:".bright_blue(),
        coverage_contribution.bright_yellow(),
        complexity_contribution.bright_yellow(),
        dependency_contribution.bright_yellow()
    )
    .unwrap();

    // Add file context information (spec 181: show context in verbose mode)
    if let Some(ref context) = item.file_context {
        use crate::priority::scoring::file_context_scoring::{
            context_label, context_reduction_factor,
        };

        let factor = context_reduction_factor(context);
        let label = context_label(context);

        // Determine explanation based on context type
        let explanation = if factor >= 1.0 {
            "no score adjustment"
        } else if factor >= 0.6 {
            "40% score reduction"
        } else if factor >= 0.2 {
            "80% score reduction"
        } else {
            "90% score reduction"
        };

        writeln!(
            output,
            "  - {} {} ({})",
            "File Context:".bright_blue(),
            label.bright_magenta(),
            explanation
        )
        .unwrap();

        writeln!(
            output,
            "  - {} {:.2}",
            "Context Factor:".bright_blue(),
            factor
        )
        .unwrap();
    }
}

/// Format related items section
pub fn format_related_items(
    output: &mut String,
    related_items: &[String],
    _formatter: &crate::formatting::ColoredFormatter,
) {
    if !related_items.is_empty() {
        writeln!(
            output,
            "- {} {} related items to address:",
            "RELATED:".bright_blue(),
            related_items.len().to_string().cyan()
        )
        .unwrap();

        for related in related_items.iter() {
            let prefix = "   -";
            writeln!(output, "{} {}", prefix, related.bright_magenta()).unwrap();
        }
    }
}

/// Format pattern analysis section if available (spec 151)
pub fn format_pattern_analysis(output: &mut String, item: &UnifiedDebtItem, verbosity: u8) {
    // Only show pattern analysis if verbosity >= 1 and patterns are available
    if verbosity < 1 {
        return;
    }

    if let Some(ref pattern_analysis) = item.pattern_analysis {
        if pattern_analysis.has_patterns() {
            writeln!(output, "├─ {}", "PATTERN ANALYSIS:".bright_blue()).unwrap();

            // Use PatternFormatter to format the analysis
            let formatted =
                crate::output::pattern_formatter::PatternFormatter::format(pattern_analysis);

            // Indent each line for proper tree formatting
            for line in formatted.lines() {
                if !line.is_empty() {
                    writeln!(output, "│  {}", line).unwrap();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_dependency_contribution() {
        assert_eq!(classify_dependency_contribution(15.0), "CRITICAL PATH");
        assert_eq!(classify_dependency_contribution(10.1), "CRITICAL PATH");
        assert_eq!(classify_dependency_contribution(10.0), "HIGH");
        assert_eq!(classify_dependency_contribution(7.0), "HIGH");
        assert_eq!(classify_dependency_contribution(5.1), "HIGH");
        assert_eq!(classify_dependency_contribution(5.0), "MEDIUM");
        assert_eq!(classify_dependency_contribution(3.0), "MEDIUM");
        assert_eq!(classify_dependency_contribution(2.1), "MEDIUM");
        assert_eq!(classify_dependency_contribution(2.0), "LOW");
        assert_eq!(classify_dependency_contribution(0.0), "LOW");
    }
}
