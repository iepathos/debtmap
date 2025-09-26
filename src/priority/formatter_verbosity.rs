use crate::formatting::{ColoredFormatter, FormattingConfig, OutputFormatter};
use crate::priority::unified_scorer::EntropyDetails;
use crate::priority::{score_formatter, TransitiveCoverage, UnifiedDebtItem};
use colored::*;
use std::fmt::Write;

// Pure function to classify coverage percentage
fn classify_coverage_percentage(coverage_pct: f64) -> (&'static str, &'static str) {
    match coverage_pct {
        0.0 => (" [🔴 UNTESTED]", "🔴 UNTESTED"),
        c if c < 20.0 => (" [🟠 LOW COVERAGE]", "🟠 LOW COVERAGE"),
        c if c < 50.0 => (" [🟡 PARTIAL COVERAGE]", "🟡 PARTIAL COVERAGE"),
        _ => ("", ""),
    }
}

// Pure function to get coverage indicator
fn get_coverage_indicator(item: &UnifiedDebtItem) -> &'static str {
    if let Some(ref trans_cov) = item.transitive_coverage {
        let coverage_pct = trans_cov.direct * 100.0;
        classify_coverage_percentage(coverage_pct).0
    } else if item.unified_score.coverage_factor >= 10.0 {
        " [🔴 UNTESTED]"
    } else {
        ""
    }
}

// Pure function to format coverage status
fn format_coverage_status(coverage_pct: f64) -> String {
    match coverage_pct {
        0.0 => "🔴 UNTESTED".to_string(),
        c if c < 20.0 => format!("🟠 LOW ({:.1}%)", c),
        c if c < 50.0 => format!("🟡 PARTIAL ({:.1}%)", c),
        c if c < 80.0 => format!("🟨 MODERATE ({:.1}%)", c),
        c if c < 95.0 => format!("🟢 GOOD ({:.1}%)", c),
        _ => format!("✅ EXCELLENT ({:.1}%)", coverage_pct),
    }
}

// Pure function to format coverage factor description
fn format_coverage_factor_description(
    item: &UnifiedDebtItem,
    _weights: &crate::config::ScoringWeights,
) -> Option<String> {
    if let Some(ref trans_cov) = item.transitive_coverage {
        let coverage_pct = trans_cov.direct * 100.0;
        match coverage_pct {
            0.0 => Some("🔴 UNTESTED (0% coverage, weight: 50%)".to_string()),
            c if c < 20.0 => Some(format!("🟠 LOW COVERAGE ({:.1}%, weight: 50%)", c)),
            c if c < 50.0 => Some(format!("🟡 PARTIAL COVERAGE ({:.1}%, weight: 50%)", c)),
            c if c >= 95.0 => Some(format!("Excellent coverage {:.1}%", c)),
            c if c >= 80.0 => Some(format!("Good coverage {:.1}%", c)),
            _ if item.unified_score.coverage_factor > 3.0 => {
                Some(format!("Line coverage {:.1}% (weight: 50%)", coverage_pct))
            }
            _ => None,
        }
    } else if item.unified_score.coverage_factor >= 10.0 {
        Some("🔴 UNTESTED (no coverage data, weight: 50%)".to_string())
    } else if item.unified_score.coverage_factor > 3.0 {
        Some("No coverage data (weight: 50%)".to_string())
    } else {
        None
    }
}

// Pure function to classify coverage contribution
fn classify_coverage_contribution(item: &UnifiedDebtItem) -> &'static str {
    if let Some(ref trans_cov) = item.transitive_coverage {
        let coverage_pct = trans_cov.direct * 100.0;
        match coverage_pct {
            0.0 => "CRITICAL (0% coverage)",
            c if c < 20.0 => "HIGH (low coverage)",
            c if c < 50.0 => "MEDIUM (partial coverage)",
            _ => "LOW",
        }
    } else {
        "HIGH (no data)"
    }
}

// Pure function to classify complexity contribution
fn classify_complexity_contribution(complexity_factor: f64) -> &'static str {
    match complexity_factor {
        c if c > 10.0 => "VERY HIGH",
        c if c > 5.0 => "HIGH",
        c if c > 3.0 => "MEDIUM",
        _ => "LOW",
    }
}

// Pure function to classify dependency contribution
fn classify_dependency_contribution(dependency_factor: f64) -> &'static str {
    match dependency_factor {
        d if d > 10.0 => "CRITICAL PATH",
        d if d > 5.0 => "HIGH",
        d if d > 2.0 => "MEDIUM",
        _ => "LOW",
    }
}

// Pure function to group consecutive lines into ranges
fn group_lines_into_ranges(lines: &[usize]) -> Vec<(usize, usize)> {
    if lines.is_empty() {
        return Vec::new();
    }

    let mut sorted_lines = lines.to_vec();
    sorted_lines.sort_unstable();
    sorted_lines.dedup();

    let mut ranges = Vec::new();
    let mut current_start = sorted_lines[0];
    let mut current_end = sorted_lines[0];

    for &line in &sorted_lines[1..] {
        if line == current_end + 1 {
            current_end = line;
        } else {
            ranges.push((current_start, current_end));
            current_start = line;
            current_end = line;
        }
    }
    ranges.push((current_start, current_end));
    ranges
}

// Pure function to format ranges as strings
fn format_ranges(ranges: &[(usize, usize)]) -> Vec<String> {
    ranges
        .iter()
        .map(|&(start, end)| {
            if start == end {
                format!("{}", start)
            } else {
                format!("{}-{}", start, end)
            }
        })
        .collect()
}

// Pure function to format uncovered lines summary
fn format_uncovered_lines_summary(uncovered_lines: &[usize], max_ranges: usize) -> String {
    if uncovered_lines.is_empty() {
        return String::new();
    }

    let ranges = group_lines_into_ranges(uncovered_lines);
    let formatted_ranges = format_ranges(&ranges);

    let display_ranges: Vec<String> = formatted_ranges.iter().take(max_ranges).cloned().collect();

    let more_indicator = if ranges.len() > max_ranges {
        format!(", ... ({} total gaps)", uncovered_lines.len())
    } else {
        String::new()
    };

    format!(
        " - Missing lines: {}{}",
        display_ranges.join(", "),
        more_indicator
    )
}

// Pure function to collect main scoring factors
fn collect_scoring_factors(
    item: &UnifiedDebtItem,
    weights: &crate::config::ScoringWeights,
) -> Vec<String> {
    let mut factors = vec![];

    // Coverage factor (50% weight in weighted sum model)
    if let Some(desc) = format_coverage_factor_description(item, weights) {
        factors.push(desc);
    }

    // Complexity factor (35% weight in weighted sum model)
    if item.unified_score.complexity_factor > 5.0 {
        factors.push("Complexity (weight: 35%)".to_string());
    } else if item.unified_score.complexity_factor > 3.0 {
        factors.push("Moderate complexity".to_string());
    }

    // Dependency factor (15% weight in weighted sum model)
    if item.unified_score.dependency_factor > 5.0 {
        factors.push("Critical path (weight: 15%)".to_string());
    }

    // Performance specific factors
    match &item.debt_type {
        crate::priority::DebtType::NestedLoops { depth, .. } => {
            factors.push("Performance impact (High)".to_string());
            factors.push(format!("{} level nested loops", depth));
        }
        crate::priority::DebtType::BlockingIO { operation, .. } => {
            factors.push("Performance impact (High)".to_string());
            factors.push(format!("Blocking {}", operation));
        }
        crate::priority::DebtType::AllocationInefficiency { pattern, .. } => {
            factors.push("Performance impact (Medium)".to_string());
            factors.push(format!("Allocation: {}", pattern));
        }
        _ => {}
    }

    factors
}

// Pure function to calculate score factors
struct ScoreFactors {
    coverage_gap: f64,
    coverage_pct: f64,
    coverage_factor: f64,
    complexity_factor: f64,
    dependency_factor: f64,
}

fn calculate_score_factors(item: &UnifiedDebtItem) -> ScoreFactors {
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
        complexity_factor: item.unified_score.complexity_factor.powf(0.8),
        dependency_factor: ((item.unified_score.dependency_factor + 1.0).sqrt() / 2.0).min(1.0),
    }
}

// Pure function to format coverage detail string
fn format_coverage_detail(has_coverage: bool, gap: f64, pct: f64) -> String {
    if has_coverage {
        format!(" (gap: {:.1}%, coverage: {:.1}%)", gap * 100.0, pct * 100.0)
    } else {
        " (no coverage data)".to_string()
    }
}

// Pure function to format complexity detail
fn format_complexity_detail(entropy: &Option<EntropyDetails>) -> String {
    if let Some(ref e) = entropy {
        format!(" (entropy-adjusted from {})", e.original_complexity)
    } else {
        String::new()
    }
}

// Pure function to format callers display
fn format_callers_display(callers: &[String], max_display: usize) -> String {
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

// Pure function to format callees display
fn format_callees_display(callees: &[String], max_display: usize) -> String {
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

/// Format a list of line numbers into readable ranges (e.g., "10-15, 22, 30-35")
fn format_line_ranges(lines: &[usize]) -> String {
    if lines.is_empty() {
        return String::new();
    }

    let mut sorted_lines = lines.to_vec();
    sorted_lines.sort_unstable();
    sorted_lines.dedup();

    let mut ranges = Vec::new();
    let mut start = sorted_lines[0];
    let mut end = start;

    for &line in &sorted_lines[1..] {
        if line == end + 1 {
            end = line;
        } else {
            if start == end {
                ranges.push(start.to_string());
            } else {
                ranges.push(format!("{}-{}", start, end));
            }
            start = line;
            end = line;
        }
    }

    // Add the last range
    if start == end {
        ranges.push(start.to_string());
    } else {
        ranges.push(format!("{}-{}", start, end));
    }

    ranges.join(", ")
}

#[allow(dead_code)]
pub fn format_priority_item_with_verbosity(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    verbosity: u8,
) {
    format_priority_item_with_config(output, rank, item, verbosity, FormattingConfig::default())
}

/// Format the score header line with severity
fn format_score_header(
    rank: usize,
    score: f64,
    coverage_indicator: &str,
    severity: &str,
    severity_color: Color,
) -> String {
    format!(
        "#{} {}{} [{}]",
        rank.to_string().bright_cyan().bold(),
        format!("SCORE: {}", score_formatter::format_score(score)).bright_yellow(),
        coverage_indicator.bright_red().bold(),
        severity.color(severity_color).bold()
    )
}

/// Format the main factors line
fn format_main_factors(factors: &[String], formatter: &ColoredFormatter) -> String {
    if factors.is_empty() {
        String::new()
    } else {
        format!(
            "   {} Main factors: {}",
            formatter.emoji("↳", "  "),
            factors.join(", ").bright_white()
        )
    }
}

/// Format score calculation section for verbosity >= 2
fn format_score_calculation_section(
    item: &UnifiedDebtItem,
    formatter: &ColoredFormatter,
) -> Vec<String> {
    let mut lines = Vec::new();
    let tree_branch = formatter.emoji("├─", "-");
    let tree_sub_branch = formatter.emoji("│  ├─", "  -");
    let tree_pipe = formatter.emoji("│", " ");

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

    lines.push(format!(
        "{}  {} Coverage Score: {:.1} × 50% = {:.2}{}",
        tree_pipe,
        formatter.emoji("├─", "-"),
        factors.coverage_factor * 10.0, // Convert to 0-100 scale
        factors.coverage_factor * 10.0 * 0.5,
        coverage_detail
    ));

    // Show complexity score
    let complexity_detail = format_complexity_detail(&item.entropy_details);
    lines.push(format!(
        "{}  {} Complexity Score: {:.1} × 35% = {:.2}{}",
        tree_pipe,
        formatter.emoji("├─", "-"),
        factors.complexity_factor * 10.0, // Convert to 0-100 scale
        factors.complexity_factor * 10.0 * 0.35,
        complexity_detail
    ));

    // Show dependency score
    lines.push(format!(
        "{}  {} Dependency Score: {:.1} × 15% = {:.2} ({} callers)",
        tree_pipe,
        formatter.emoji("├─", "-"),
        factors.dependency_factor * 10.0, // Convert to 0-100 scale
        factors.dependency_factor * 10.0 * 0.15,
        item.unified_score.dependency_factor as u32
    ));

    // Calculate weighted sum base score
    let coverage_contribution = factors.coverage_factor * 10.0 * 0.5;
    let complexity_contribution = factors.complexity_factor * 10.0 * 0.35;
    let dependency_contribution = factors.dependency_factor * 10.0 * 0.15;
    let base_score = coverage_contribution + complexity_contribution + dependency_contribution;

    lines.push(format!(
        "{}  {} Base Score: {:.2} + {:.2} + {:.2} = {:.2}",
        tree_pipe,
        formatter.emoji("├─", "-"),
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
            formatter.emoji("├─", "-"),
            (1.0 - entropy.dampening_factor) * 100.0,
            entropy.entropy_score,
            entropy.pattern_repetition * 100.0
        ));
    }

    lines.push(format!(
        "{}  {} Role Adjustment: ×{:.2}",
        tree_pipe,
        formatter.emoji("├─", "-"),
        item.unified_score.role_multiplier
    ));

    lines.push(format!(
        "{}  {} Final Score: {:.2}",
        tree_pipe,
        formatter.emoji("└─", "-"),
        item.unified_score.final_score
    ));

    lines
}

/// Format complexity details section
fn format_complexity_details_section(
    item: &UnifiedDebtItem,
    formatter: &ColoredFormatter,
) -> Vec<String> {
    let mut lines = Vec::new();
    let tree_pipe = formatter.emoji("│", " ");

    lines.push(format!(
        "{} {}",
        formatter.emoji("├─", "-"),
        "COMPLEXITY DETAILS:".bright_blue()
    ));

    lines.push(format!(
        "{}  {} Cyclomatic Complexity: {}",
        tree_pipe,
        formatter.emoji("├─", "-"),
        item.cyclomatic_complexity
    ));

    lines.push(format!(
        "{}  {} Cognitive Complexity: {}",
        tree_pipe,
        formatter.emoji("├─", "-"),
        item.cognitive_complexity
    ));

    lines.push(format!(
        "{}  {} Function Length: {} lines",
        tree_pipe,
        formatter.emoji("├─", "-"),
        item.function_length
    ));

    lines.push(format!(
        "{}  {} Nesting Depth: {}",
        tree_pipe,
        formatter.emoji("└─", "-"),
        item.nesting_depth
    ));

    lines
}

/// Format coverage details section for verbosity >= 2
fn format_coverage_details_section(
    trans_cov: &TransitiveCoverage,
    formatter: &ColoredFormatter,
) -> Vec<String> {
    let mut lines = Vec::new();
    let tree_pipe = formatter.emoji("│", " ");

    if !trans_cov.uncovered_lines.is_empty() {
        lines.push(format!(
            "{} {}",
            formatter.emoji("├─", "-"),
            "COVERAGE DETAILS:".bright_blue()
        ));

        lines.push(format!(
            "{}  {} Coverage: {:.1}%",
            tree_pipe,
            formatter.emoji("├─", "-"),
            trans_cov.direct * 100.0
        ));

        let line_ranges = format_line_ranges(&trans_cov.uncovered_lines);
        lines.push(format!(
            "{}  {} Uncovered Lines: {}",
            tree_pipe,
            formatter.emoji("└─", "-"),
            line_ranges
        ));
    }

    lines
}

/// Format call graph section for verbosity >= 2
fn format_call_graph_section(item: &UnifiedDebtItem, formatter: &ColoredFormatter) -> Vec<String> {
    let mut lines = Vec::new();
    let tree_pipe = formatter.emoji("│", " ");

    if !item.upstream_callers.is_empty() || !item.downstream_callees.is_empty() {
        lines.push(format!(
            "{} {}",
            formatter.emoji("├─", "-"),
            "CALL GRAPH:".bright_blue()
        ));

        if !item.upstream_callers.is_empty() {
            let callers = format_callers_display(&item.upstream_callers, 5);
            lines.push(format!(
                "{}  {} Called by: {}",
                tree_pipe,
                formatter.emoji("├─", "-"),
                callers
            ));
        }

        if !item.downstream_callees.is_empty() {
            let callees = format_callees_display(&item.downstream_callees, 5);
            lines.push(format!(
                "{}  {} Calls: {}",
                tree_pipe,
                formatter.emoji("└─", "-"),
                callees
            ));
        } else if !item.upstream_callers.is_empty() {
            // Change the last caller line to use └─ if there are no callees
            lines.push(format!(
                "{}  {} Dependencies: {} upstream, {} downstream",
                tree_pipe,
                formatter.emoji("└─", "-"),
                item.upstream_dependencies,
                item.downstream_dependencies
            ));
        }
    }

    lines
}

/// Format basic call graph info for verbosity level 0
fn format_basic_call_graph(
    output: &mut String,
    item: &UnifiedDebtItem,
    formatter: &ColoredFormatter,
) {
    let caller_count = item.upstream_callers.len();
    let callee_count = item.downstream_callees.len();

    // Only show if there's interesting call graph info
    if caller_count > 0 || callee_count > 0 {
        writeln!(
            output,
            "{} {} {} caller{}, {} callee{}",
            formatter.emoji("├─", "-"),
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
                "{}   {} {}",
                formatter.emoji("│", " "),
                formatter.emoji("⚠", "!"),
                "No callers detected - may be dead code".yellow()
            )
            .unwrap();
        }
    }
}

/// Format the main body of the item (location, action, impact, etc.)
fn format_item_body(
    output: &mut String,
    item: &UnifiedDebtItem,
    formatter: &ColoredFormatter,
    verbosity: u8,
    tree_pipe: &str,
) {
    // Location section
    writeln!(
        output,
        "{} {} {}:{} {}()",
        formatter.emoji("├─", "-"),
        "LOCATION:".bright_blue(),
        item.location.file.display(),
        item.location.line,
        item.location.function.bright_green()
    )
    .unwrap();

    // WHY section (the rationale)
    let why_label = formatter.emoji("└─ WHY:", "- WHY:").bright_blue();
    writeln!(output, "{} {}", why_label, item.recommendation.rationale).unwrap();

    // ACTION section
    writeln!(
        output,
        "{} {} {}",
        formatter.emoji("├─", "-"),
        "ACTION:".bright_blue(),
        item.recommendation.primary_action.bright_green().bold()
    )
    .unwrap();

    // Implementation steps
    format_implementation_steps(output, &item.recommendation.implementation_steps, formatter);

    // IMPACT section
    writeln!(
        output,
        "{} {} {}",
        formatter.emoji("├─", "-"),
        "IMPACT:".bright_blue(),
        crate::priority::formatter::format_impact(&item.expected_impact).bright_cyan()
    )
    .unwrap();

    // COMPLEXITY section
    format_complexity_summary(output, item, formatter);

    // DEPENDENCIES section
    format_dependencies_summary(output, item, formatter, tree_pipe);

    // CALL GRAPH section - show basic info even at verbosity 0
    format_basic_call_graph(output, item, formatter);

    // SCORING breakdown for verbosity >= 1
    if (1..2).contains(&verbosity) {
        format_scoring_breakdown(output, item, formatter);
    }

    // COVERAGE section
    format_coverage_section(output, item, formatter, verbosity, tree_pipe);

    // RELATED items
    format_related_items(output, &item.recommendation.related_items, formatter);
}

/// Format implementation steps
fn format_implementation_steps(
    output: &mut String,
    steps: &[String],
    formatter: &ColoredFormatter,
) {
    if !steps.is_empty() {
        for (i, step) in steps.iter().enumerate() {
            let prefix = if i == steps.len() - 1 {
                formatter.emoji("│  └─", "   -")
            } else {
                formatter.emoji("│  ├─", "   -")
            };
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

/// Format complexity summary line
fn format_complexity_summary(
    output: &mut String,
    item: &UnifiedDebtItem,
    formatter: &ColoredFormatter,
) {
    let (cyclomatic, cognitive, branch_count, nesting, _length) =
        crate::priority::formatter::extract_complexity_info(item);

    if cyclomatic > 0 || cognitive > 0 {
        if let Some(ref entropy) = item.entropy_details {
            writeln!(
                output,
                "{} {} cyclomatic={} (adj:{}), branches={}, cognitive={}, nesting={}, entropy={:.2}",
                formatter.emoji("├─", "-"),
                "COMPLEXITY:".bright_blue(),
                cyclomatic.to_string().yellow(),
                entropy.adjusted_complexity.to_string().yellow(),
                branch_count.to_string().yellow(),
                cognitive.to_string().yellow(),
                nesting.to_string().yellow(),
                entropy.entropy_score
            )
            .unwrap();
        } else {
            writeln!(
                output,
                "{} {} cyclomatic={}, branches={}, cognitive={}, nesting={}",
                formatter.emoji("├─", "-"),
                "COMPLEXITY:".bright_blue(),
                cyclomatic.to_string().yellow(),
                branch_count.to_string().yellow(),
                cognitive.to_string().yellow(),
                nesting.to_string().yellow()
            )
            .unwrap();
        }
    }
}

/// Format dependencies summary
fn format_dependencies_summary(
    output: &mut String,
    item: &UnifiedDebtItem,
    formatter: &ColoredFormatter,
    tree_pipe: &str,
) {
    let (upstream, downstream) = crate::priority::formatter::extract_dependency_info(item);

    if upstream > 0 || downstream > 0 {
        writeln!(
            output,
            "{} {} {} upstream, {} downstream",
            formatter.emoji("├─", "-"),
            "DEPENDENCIES:".bright_blue(),
            upstream.to_string().cyan(),
            downstream.to_string().cyan()
        )
        .unwrap();

        if !item.upstream_callers.is_empty() {
            let callers_display = format_callers_display(&item.upstream_callers, 3);
            writeln!(
                output,
                "{}  {} CALLERS: {}",
                tree_pipe,
                formatter.emoji("├─", "-"),
                callers_display.cyan()
            )
            .unwrap();
        }

        if !item.downstream_callees.is_empty() {
            let callees_display = format_callees_display(&item.downstream_callees, 3);
            writeln!(
                output,
                "{}  {} CALLS: {}",
                tree_pipe,
                formatter.emoji("└─", "-"),
                callees_display.bright_magenta()
            )
            .unwrap();
        }
    }
}

/// Format scoring breakdown for verbosity 1
fn format_scoring_breakdown(
    output: &mut String,
    item: &UnifiedDebtItem,
    formatter: &ColoredFormatter,
) {
    let coverage_contribution = classify_coverage_contribution(item);
    let complexity_contribution =
        classify_complexity_contribution(item.unified_score.complexity_factor);
    let dependency_contribution =
        classify_dependency_contribution(item.unified_score.dependency_factor);

    writeln!(
        output,
        "{} {} Coverage: {} | Complexity: {} | Dependencies: {}",
        formatter.emoji("├─", "-"),
        "SCORING:".bright_blue(),
        coverage_contribution.bright_yellow(),
        complexity_contribution.bright_yellow(),
        dependency_contribution.bright_yellow()
    )
    .unwrap();
}

/// Format coverage section
fn format_coverage_section(
    output: &mut String,
    item: &UnifiedDebtItem,
    formatter: &ColoredFormatter,
    verbosity: u8,
    tree_pipe: &str,
) {
    if let Some(ref trans_cov) = item.transitive_coverage {
        let coverage_pct = trans_cov.direct * 100.0;
        let coverage_status = format_coverage_status(coverage_pct);
        let uncovered_summary = format_uncovered_lines_summary(&trans_cov.uncovered_lines, 5);

        writeln!(
            output,
            "{} {} {}{}",
            formatter.emoji("├─", "-"),
            "COVERAGE:".bright_blue(),
            coverage_status.bright_yellow(),
            uncovered_summary.bright_red()
        )
        .unwrap();

        // Detailed coverage analysis for verbosity >= 2
        if coverage_pct < 100.0 && !trans_cov.uncovered_lines.is_empty() && verbosity >= 2 {
            format_detailed_coverage_analysis(output, trans_cov, item, formatter, tree_pipe);
        }
    }
}

/// Format detailed coverage analysis for verbosity >= 2
fn format_detailed_coverage_analysis(
    output: &mut String,
    trans_cov: &TransitiveCoverage,
    item: &UnifiedDebtItem,
    formatter: &ColoredFormatter,
    tree_pipe: &str,
) {
    writeln!(
        output,
        "{} {}",
        formatter.emoji("├─", "-"),
        "COVERAGE DETAILS:".bright_blue()
    )
    .unwrap();

    let mut sorted_lines = trans_cov.uncovered_lines.clone();
    sorted_lines.sort_unstable();

    let ranges = group_lines_into_ranges(&sorted_lines);
    let formatted_ranges = format_ranges(&ranges);

    let lines_str = if formatted_ranges.len() <= 10 {
        formatted_ranges.join(", ")
    } else {
        format!(
            "{}, ... ({} total uncovered lines)",
            formatted_ranges[..10].join(", "),
            sorted_lines.len()
        )
    };

    writeln!(
        output,
        "{}  {} Uncovered lines: {}",
        tree_pipe,
        formatter.emoji("├─", "-"),
        lines_str.bright_red()
    )
    .unwrap();

    let branch_recommendations = analyze_coverage_gaps(&sorted_lines, item);
    if !branch_recommendations.is_empty() {
        writeln!(
            output,
            "{}  {} Test focus areas:",
            tree_pipe,
            formatter.emoji("└─", "-")
        )
        .unwrap();
        for rec in branch_recommendations.iter().take(3) {
            writeln!(
                output,
                "{}      {} {}",
                tree_pipe,
                formatter.emoji("•", "*"),
                rec.yellow()
            )
            .unwrap();
        }
    }
}

/// Format related items section
fn format_related_items(
    output: &mut String,
    related_items: &[String],
    formatter: &ColoredFormatter,
) {
    if !related_items.is_empty() {
        writeln!(
            output,
            "{} {} {} related items to address:",
            formatter.emoji("├─", "-"),
            "RELATED:".bright_blue(),
            related_items.len().to_string().cyan()
        )
        .unwrap();

        for (i, related) in related_items.iter().enumerate() {
            let prefix = if i == related_items.len() - 1 {
                formatter.emoji("│  └─", "   -")
            } else {
                formatter.emoji("│  ├─", "   -")
            };
            writeln!(output, "{} {}", prefix, related.bright_magenta()).unwrap();
        }
    }
}

pub fn format_priority_item_with_config(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    verbosity: u8,
    config: FormattingConfig,
) {
    let formatter = ColoredFormatter::new(config);
    let severity = crate::priority::formatter::get_severity_label(item.unified_score.final_score);
    let severity_color =
        crate::priority::formatter::get_severity_color(item.unified_score.final_score);
    let tree_pipe = formatter.emoji("│", " ");

    // Format and write the score header
    let coverage_indicator = get_coverage_indicator(item);
    let score_header = format_score_header(
        rank,
        item.unified_score.final_score,
        coverage_indicator,
        severity,
        severity_color,
    );
    writeln!(output, "{}", score_header).unwrap();

    // Add main factors for verbosity >= 1
    if verbosity >= 1 {
        let weights = crate::config::get_scoring_weights();
        let factors = collect_scoring_factors(item, weights);
        let factors_line = format_main_factors(&factors, &formatter);
        if !factors_line.is_empty() {
            writeln!(output, "{}", factors_line).unwrap();
        }
    }

    // Show detailed calculation for verbosity >= 2
    if verbosity >= 2 {
        // Format score calculation section
        let score_calc_lines = format_score_calculation_section(item, &formatter);
        for line in score_calc_lines {
            writeln!(output, "{}", line).unwrap();
        }

        // Format complexity details section
        let complexity_lines = format_complexity_details_section(item, &formatter);
        for line in complexity_lines {
            writeln!(output, "{}", line).unwrap();
        }

        // Format coverage details if available
        if let Some(ref trans_cov) = item.transitive_coverage {
            let coverage_lines = format_coverage_details_section(trans_cov, &formatter);
            for line in coverage_lines {
                writeln!(output, "{}", line).unwrap();
            }
        }

        // Format call graph section
        let call_graph_lines = format_call_graph_section(item, &formatter);
        for line in call_graph_lines {
            writeln!(output, "{}", line).unwrap();
        }
    }

    // Format the rest of the item
    format_item_body(output, item, &formatter, verbosity, &tree_pipe);
}

/// Analyze coverage gaps to provide specific testing recommendations
fn analyze_coverage_gaps(uncovered_lines: &[usize], item: &UnifiedDebtItem) -> Vec<String> {
    let mut recommendations = Vec::new();

    // Check for patterns in uncovered lines
    let line_count = uncovered_lines.len();

    // Large contiguous blocks suggest untested branches
    let mut max_consecutive = 0;
    let mut current_consecutive = 1;
    for i in 1..uncovered_lines.len() {
        if uncovered_lines[i] == uncovered_lines[i - 1] + 1 {
            current_consecutive += 1;
            max_consecutive = max_consecutive.max(current_consecutive);
        } else {
            current_consecutive = 1;
        }
    }

    if max_consecutive >= 5 {
        recommendations.push(format!(
            "Large uncovered block ({} consecutive lines) - likely an entire conditional branch",
            max_consecutive
        ));
    }

    // Many scattered lines suggest missing edge cases
    if line_count > 10 && max_consecutive < 3 {
        recommendations.push(
            "Scattered uncovered lines - consider testing edge cases and error conditions"
                .to_string(),
        );
    }

    // Check complexity vs coverage
    if item.cyclomatic_complexity > 10 && line_count > 0 {
        let branch_coverage_estimate =
            1.0 - (line_count as f32 / (item.cyclomatic_complexity * 2) as f32);
        if branch_coverage_estimate < 0.5 {
            recommendations.push(format!(
                "Low branch coverage (est. <50%) with {} branches - prioritize testing main paths",
                item.cyclomatic_complexity
            ));
        }
    }

    // Specific recommendations based on debt type
    match &item.debt_type {
        crate::priority::DebtType::ComplexityHotspot { .. } => {
            if line_count > 0 {
                recommendations.push(
                    "Complex function - focus tests on boundary conditions and error paths"
                        .to_string(),
                );
            }
        }
        crate::priority::DebtType::Risk { .. } => {
            if line_count > 0 {
                recommendations.push(
                    "High-risk function - ensure all error handling paths are tested".to_string(),
                );
            }
        }
        crate::priority::DebtType::TestingGap { .. } => {
            if line_count > 0 {
                recommendations
                    .push("Testing gap - add tests covering the uncovered branches".to_string());
            }
        }
        _ => {}
    }

    recommendations
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::{DebtType, UnifiedDebtItem, UnifiedScore};
    use std::path::PathBuf;

    #[test]
    fn test_classify_coverage_percentage() {
        assert_eq!(
            classify_coverage_percentage(0.0),
            (" [🔴 UNTESTED]", "🔴 UNTESTED")
        );
        assert_eq!(
            classify_coverage_percentage(10.0),
            (" [🟠 LOW COVERAGE]", "🟠 LOW COVERAGE")
        );
        assert_eq!(
            classify_coverage_percentage(19.9),
            (" [🟠 LOW COVERAGE]", "🟠 LOW COVERAGE")
        );
        assert_eq!(
            classify_coverage_percentage(20.0),
            (" [🟡 PARTIAL COVERAGE]", "🟡 PARTIAL COVERAGE")
        );
        assert_eq!(
            classify_coverage_percentage(49.9),
            (" [🟡 PARTIAL COVERAGE]", "🟡 PARTIAL COVERAGE")
        );
        assert_eq!(classify_coverage_percentage(50.0), ("", ""));
        assert_eq!(classify_coverage_percentage(100.0), ("", ""));
    }

    #[test]
    fn test_format_coverage_status() {
        assert_eq!(format_coverage_status(0.0), "🔴 UNTESTED");
        assert_eq!(format_coverage_status(10.0), "🟠 LOW (10.0%)");
        assert_eq!(format_coverage_status(30.0), "🟡 PARTIAL (30.0%)");
        assert_eq!(format_coverage_status(60.0), "🟨 MODERATE (60.0%)");
        assert_eq!(format_coverage_status(85.0), "🟢 GOOD (85.0%)");
        assert_eq!(format_coverage_status(96.0), "✅ EXCELLENT (96.0%)");
    }

    #[test]
    fn test_format_line_ranges() {
        // Test empty input
        assert_eq!(format_line_ranges(&[]), "");

        // Test single line
        assert_eq!(format_line_ranges(&[42]), "42");

        // Test consecutive lines
        assert_eq!(format_line_ranges(&[1, 2, 3]), "1-3");

        // Test mixed ranges
        assert_eq!(format_line_ranges(&[1, 2, 3, 5, 7, 8, 9]), "1-3, 5, 7-9");

        // Test unsorted input (should be sorted)
        assert_eq!(format_line_ranges(&[9, 1, 3, 2, 5]), "1-3, 5, 9");

        // Test duplicates (should be deduplicated)
        assert_eq!(format_line_ranges(&[1, 1, 2, 2, 3]), "1-3");
    }

    #[test]
    fn test_group_lines_into_ranges() {
        // Test with limit of 3 ranges
        let lines = vec![1, 2, 3, 5, 7, 8, 9, 11, 13, 14, 15];
        let ranges = group_lines_into_ranges(&lines);
        assert_eq!(ranges, vec![(1, 3), (5, 5), (7, 9), (11, 11), (13, 15)]);

        // Test with no input
        let ranges = group_lines_into_ranges(&[]);
        assert!(ranges.is_empty());

        // Test with single line
        let ranges = group_lines_into_ranges(&[5]);
        assert_eq!(ranges, vec![(5, 5)]);
    }

    #[test]
    fn test_classify_complexity_contribution() {
        assert_eq!(classify_complexity_contribution(15.0), "VERY HIGH");
        assert_eq!(classify_complexity_contribution(10.1), "VERY HIGH");
        assert_eq!(classify_complexity_contribution(10.0), "HIGH");
        assert_eq!(classify_complexity_contribution(7.0), "HIGH");
        assert_eq!(classify_complexity_contribution(5.1), "HIGH");
        assert_eq!(classify_complexity_contribution(5.0), "MEDIUM");
        assert_eq!(classify_complexity_contribution(4.0), "MEDIUM");
        assert_eq!(classify_complexity_contribution(3.1), "MEDIUM");
        assert_eq!(classify_complexity_contribution(3.0), "LOW");
        assert_eq!(classify_complexity_contribution(1.0), "LOW");
    }

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

    #[test]
    fn test_format_score_header() {
        use colored::Color;

        let header = format_score_header(1, 85.5, " ⚠", "HIGH", Color::Red);
        assert!(header.contains("#1"));
        assert!(header.contains("SCORE:"));
        assert!(header.contains("[HIGH]"));
    }

    #[test]
    fn test_format_main_factors() {
        let formatter = ColoredFormatter::new(FormattingConfig::plain());

        // Test with empty factors
        let factors: Vec<String> = vec![];
        let result = format_main_factors(&factors, &formatter);
        assert_eq!(result, "");

        // Test with factors
        let factors = vec!["Factor1".to_string(), "Factor2".to_string()];
        let result = format_main_factors(&factors, &formatter);
        assert!(result.contains("Main factors:"));
        assert!(result.contains("Factor1, Factor2"));
    }

    #[test]
    fn test_format_score_calculation_section() {
        let item = create_test_item();
        let formatter = ColoredFormatter::new(FormattingConfig::plain());

        let lines = format_score_calculation_section(&item, &formatter);

        // Check for expected sections
        assert!(lines.iter().any(|l| l.contains("SCORE CALCULATION:")));
        assert!(lines.iter().any(|l| l.contains("Weighted Sum Model:")));
        assert!(lines.iter().any(|l| l.contains("Coverage Score:")));
        assert!(lines.iter().any(|l| l.contains("Complexity Score:")));
        assert!(lines.iter().any(|l| l.contains("Dependency Score:")));
        assert!(lines.iter().any(|l| l.contains("Base Score:")));
        assert!(lines.iter().any(|l| l.contains("Final Score:")));
    }

    #[test]
    fn test_format_complexity_details_section() {
        let mut item = create_test_item();
        item.cyclomatic_complexity = 15;
        item.cognitive_complexity = 25;
        item.function_length = 150;
        item.nesting_depth = 3;

        let formatter = ColoredFormatter::new(FormattingConfig::plain());
        let lines = format_complexity_details_section(&item, &formatter);

        assert!(lines.iter().any(|l| l.contains("COMPLEXITY DETAILS:")));
        assert!(lines
            .iter()
            .any(|l| l.contains("Cyclomatic Complexity: 15")));
        assert!(lines.iter().any(|l| l.contains("Cognitive Complexity: 25")));
        assert!(lines
            .iter()
            .any(|l| l.contains("Function Length: 150 lines")));
        assert!(lines.iter().any(|l| l.contains("Nesting Depth: 3")));
    }

    #[test]
    fn test_format_implementation_steps() {
        let mut output = String::new();
        let formatter = ColoredFormatter::new(FormattingConfig::plain());
        let steps = vec![
            "Step 1: Do this".to_string(),
            "Step 2: Do that".to_string(),
            "Step 3: Finish".to_string(),
        ];

        format_implementation_steps(&mut output, &steps, &formatter);

        assert!(output.contains("1. Step 1: Do this"));
        assert!(output.contains("2. Step 2: Do that"));
        assert!(output.contains("3. Step 3: Finish"));
    }

    #[test]
    fn test_format_complexity_summary() {
        let mut output = String::new();
        let mut item = create_test_item();
        item.cyclomatic_complexity = 20;
        item.cognitive_complexity = 30;

        // Test without entropy
        let formatter = ColoredFormatter::new(FormattingConfig::plain());
        format_complexity_summary(&mut output, &item, &formatter);
        assert!(output.contains("COMPLEXITY:"));
        assert!(output.contains("cyclomatic=20"));
        assert!(output.contains("cognitive=30"));

        // Test with entropy
        output.clear();
        item.entropy_details = Some(EntropyDetails {
            entropy_score: 0.75,
            pattern_repetition: 0.25,
            original_complexity: 20,
            adjusted_complexity: 15,
            dampening_factor: 0.8,
        });
        format_complexity_summary(&mut output, &item, &formatter);
        assert!(output.contains("adj:15"));
        assert!(output.contains("entropy=0.75"));
    }

    #[test]
    fn test_format_dependencies_summary() {
        let mut output = String::new();
        let mut item = create_test_item();
        item.upstream_dependencies = 3;
        item.downstream_dependencies = 5;
        item.upstream_callers = vec!["caller1".to_string(), "caller2".to_string()];
        item.downstream_callees = vec!["callee1".to_string(), "callee2".to_string()];

        let formatter = ColoredFormatter::new(FormattingConfig::plain());
        format_dependencies_summary(&mut output, &item, &formatter, " ");

        assert!(output.contains("DEPENDENCIES:"));
        assert!(output.contains("3 upstream"));
        assert!(output.contains("5 downstream"));
        assert!(output.contains("CALLERS:"));
        assert!(output.contains("CALLS:"));
    }

    #[test]
    fn test_format_scoring_breakdown() {
        let mut output = String::new();
        let item = create_test_item();
        let formatter = ColoredFormatter::new(FormattingConfig::plain());

        format_scoring_breakdown(&mut output, &item, &formatter);

        assert!(output.contains("SCORING:"));
        assert!(output.contains("Coverage:"));
        assert!(output.contains("Complexity:"));
        assert!(output.contains("Dependencies:"));
    }

    #[test]
    fn test_collect_scoring_factors() {
        let weights = crate::config::ScoringWeights {
            complexity: 0.3,
            coverage: 0.5,
            dependency: 0.2,
            semantic: 0.0,
            security: 0.0,
            organization: 0.0,
        };

        // Test with no coverage data and high coverage factor
        let mut item = create_test_item();
        item.unified_score.coverage_factor = 10.0;
        item.unified_score.complexity_factor = 3.5;
        item.unified_score.dependency_factor = 1.0;

        let factors = collect_scoring_factors(&item, &weights);
        assert!(factors.iter().any(|f| f.contains("UNTESTED")));
        assert!(factors.iter().any(|f| f.contains("Moderate complexity")));

        // Test with nested loops debt type
        item.debt_type = DebtType::NestedLoops {
            depth: 3,
            complexity_estimate: "O(n^3)".to_string(),
        };
        let factors = collect_scoring_factors(&item, &weights);
        assert!(factors
            .iter()
            .any(|f| f.contains("Performance impact (High)")));
        assert!(factors.iter().any(|f| f.contains("3 level nested loops")));
    }

    #[test]
    fn test_analyze_coverage_gaps() {
        let mut item = create_test_item();

        // Test large consecutive block detection
        let lines = vec![10, 11, 12, 13, 14, 15, 20];
        let recommendations = analyze_coverage_gaps(&lines, &item);
        assert!(recommendations
            .iter()
            .any(|r| r.contains("6 consecutive lines")));

        // Test scattered lines detection
        let scattered = vec![1, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 55];
        let recommendations = analyze_coverage_gaps(&scattered, &item);
        assert!(recommendations
            .iter()
            .any(|r| r.contains("Scattered uncovered lines")));

        // Test high complexity with coverage gaps
        item.cyclomatic_complexity = 11;
        // Need more than (complexity * 2) / 2 lines to trigger low branch coverage
        let many_lines = vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
        ];
        let recommendations = analyze_coverage_gaps(&many_lines, &item);
        // Now (1.0 - 23/22) = negative, which triggers the condition
        assert!(recommendations.iter().any(|r| r.contains("branches")));

        // Test scattered lines detection - needs > 10 lines with max_consecutive < 3
        let scattered = vec![1, 5, 9, 13, 17, 21, 25, 29, 33, 37, 41]; // 11 lines, all non-consecutive
        let recommendations = analyze_coverage_gaps(&scattered, &item);
        assert!(recommendations.iter().any(|r| r.contains("Scattered")));

        // Test with ComplexityHotspot debt type
        item.debt_type = DebtType::ComplexityHotspot {
            cyclomatic: 20,
            cognitive: 30,
        };
        let recommendations = analyze_coverage_gaps(&lines, &item);
        assert!(recommendations
            .iter()
            .any(|r| r.contains("Complex function")));
    }

    // Helper function to create a test UnifiedDebtItem
    fn create_test_item() -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: crate::priority::Location {
                file: PathBuf::from("test.rs"),
                function: "test_function".to_string(),
                line: 100,
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 10,
                cognitive: 20,
            },
            unified_score: UnifiedScore {
                complexity_factor: 5.0,
                coverage_factor: 5.0,
                dependency_factor: 2.0,
                role_multiplier: 1.0,
                final_score: 10.0,
            },
            function_role: crate::priority::FunctionRole::Unknown,
            recommendation: crate::priority::ActionableRecommendation {
                primary_action: "Test action".to_string(),
                rationale: "Test rationale".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
            },
            expected_impact: crate::priority::ImpactMetrics {
                coverage_improvement: 0.5,
                lines_reduction: 10,
                complexity_reduction: 5.0,
                risk_reduction: 2.0,
            },
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 2,
            function_length: 50,
            cyclomatic_complexity: 10,
            cognitive_complexity: 20,
            entropy_details: None,
            is_pure: Some(false),
            purity_confidence: Some(0.5),
            god_object_indicators: None,
        }
    }
}
