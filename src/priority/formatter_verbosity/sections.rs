use crate::priority::UnifiedDebtItem;
use colored::*;
use std::fmt::Write;

/// Pure function to classify dependency contribution
pub fn classify_dependency_contribution(dependency_factor: f64) -> &'static str {
    match dependency_factor {
        d if d > 10.0 => "VERY HIGH",
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

/// Format recorded score arithmetic for verbosity >= 2.
pub fn format_score_calculation_section(
    item: &UnifiedDebtItem,
    _formatter: &crate::formatting::ColoredFormatter,
) -> Vec<String> {
    std::iter::once("- SCORE CALCULATION:".to_string())
        .chain(
            crate::priority::scoring::trace::explanation_lines(&item.unified_score)
                .into_iter()
                .map(|line| format!("  - {line}")),
        )
        .collect()
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

    if let Some(ref pattern_analysis) = item.pattern_analysis
        && pattern_analysis.has_patterns()
    {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_dependency_contribution() {
        assert_eq!(classify_dependency_contribution(15.0), "VERY HIGH");
        assert_eq!(classify_dependency_contribution(10.1), "VERY HIGH");
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
