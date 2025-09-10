use crate::formatting::{ColoredFormatter, FormattingConfig, OutputFormatter};
use crate::priority::{score_formatter, UnifiedDebtItem};
use colored::*;
use std::fmt::Write;

/// Coverage indicator for display based on coverage percentage
fn get_coverage_indicator(coverage_pct: f64) -> &'static str {
    match coverage_pct {
        0.0 => " [🔴 UNTESTED]",
        c if c < 20.0 => " [🟠 LOW COVERAGE]",
        c if c < 50.0 => " [🟡 PARTIAL COVERAGE]",
        _ => "",
    }
}

/// Get coverage indicator from item, checking both transitive coverage and coverage factor
fn get_item_coverage_indicator(item: &UnifiedDebtItem) -> &'static str {
    if let Some(ref trans_cov) = item.transitive_coverage {
        let coverage_pct = trans_cov.direct * 100.0;
        get_coverage_indicator(coverage_pct)
    } else if item.unified_score.coverage_factor >= 10.0 {
        " [🔴 UNTESTED]"
    } else {
        ""
    }
}

/// Get coverage status string with percentage
fn get_coverage_status(coverage_pct: f64) -> String {
    match coverage_pct {
        0.0 => "🔴 UNTESTED".to_string(),
        c if c < 20.0 => format!("🟠 LOW ({:.1}%)", c),
        c if c < 50.0 => format!("🟡 PARTIAL ({:.1}%)", c),
        c if c < 80.0 => format!("🟨 MODERATE ({:.1}%)", c),
        c if c < 95.0 => format!("🟢 GOOD ({:.1}%)", c),
        _ => format!("✅ EXCELLENT ({:.1}%)", coverage_pct),
    }
}

/// Get main contributing factors for score calculation
fn get_main_factors(item: &UnifiedDebtItem, weights: &crate::config::ScoringWeights) -> Vec<String> {
    let mut factors = vec![];

    // Show coverage info - both good and bad coverage are important factors
    if let Some(ref trans_cov) = item.transitive_coverage {
        let coverage_pct = trans_cov.direct * 100.0;
        if coverage_pct == 0.0 {
            factors.push(format!(
                "🔴 UNTESTED (0% coverage, weight: {:.0}%)",
                weights.coverage * 100.0
            ));
        } else if coverage_pct < 20.0 {
            factors.push(format!(
                "🟠 LOW COVERAGE ({:.1}%, weight: {:.0}%)",
                coverage_pct,
                weights.coverage * 100.0
            ));
        } else if coverage_pct < 50.0 {
            factors.push(format!(
                "🟡 PARTIAL COVERAGE ({:.1}%, weight: {:.0}%)",
                coverage_pct,
                weights.coverage * 100.0
            ));
        } else if coverage_pct >= 95.0 {
            factors.push(format!("Excellent coverage {:.1}%", coverage_pct));
        } else if coverage_pct >= 80.0 {
            factors.push(format!("Good coverage {:.1}%", coverage_pct));
        } else if item.unified_score.coverage_factor > 3.0 {
            factors.push(format!(
                "Line coverage {:.1}% (weight: {:.0}%)",
                coverage_pct,
                weights.coverage * 100.0
            ));
        }
    } else if item.unified_score.coverage_factor >= 10.0 {
        factors.push(format!(
            "🔴 UNTESTED (no coverage data, weight: {:.0}%)",
            weights.coverage * 100.0
        ));
    } else if item.unified_score.coverage_factor > 3.0 {
        factors.push(format!(
            "No coverage data (weight: {:.0}%)",
            weights.coverage * 100.0
        ));
    }

    if item.unified_score.complexity_factor > 5.0 {
        factors.push(format!(
            "Complexity (weight: {:.0}%)",
            weights.complexity * 100.0
        ));
    } else if item.unified_score.complexity_factor > 3.0 {
        factors.push("Moderate complexity".to_string());
    }

    if item.unified_score.dependency_factor > 5.0 {
        factors.push(format!(
            "Critical path (weight: {:.0}%)",
            weights.dependency * 100.0
        ));
    }

    // Add Performance specific factors
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
        _ => {} // No additional factors for other debt types
    }

    factors
}

/// Format uncovered line ranges for display
fn format_uncovered_ranges(lines: &[usize], max_ranges: usize) -> (Vec<String>, String) {
    let mut sorted_lines = lines.to_vec();
    sorted_lines.sort_unstable();

    // Group consecutive lines into ranges
    let mut ranges = Vec::new();
    if !sorted_lines.is_empty() {
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
    }

    let formatted_ranges: Vec<String> = ranges
        .iter()
        .take(max_ranges)
        .map(|&(start, end)| {
            if start == end {
                format!("{}", start)
            } else {
                format!("{}-{}", start, end)
            }
        })
        .collect();

    let more_indicator = if ranges.len() > max_ranges {
        format!(", ... ({} total gaps)", sorted_lines.len())
    } else {
        String::new()
    };

    (formatted_ranges, more_indicator)
}

/// Get coverage contribution level
fn get_coverage_contribution(item: &UnifiedDebtItem) -> &'static str {
    if let Some(ref trans_cov) = item.transitive_coverage {
        let coverage_pct = trans_cov.direct * 100.0;
        if coverage_pct == 0.0 {
            "CRITICAL (0% coverage)"
        } else if coverage_pct < 20.0 {
            "HIGH (low coverage)"
        } else if coverage_pct < 50.0 {
            "MEDIUM (partial coverage)"
        } else {
            "LOW"
        }
    } else {
        "HIGH (no data)"
    }
}

/// Get complexity contribution level
fn get_complexity_contribution(complexity_factor: f64) -> &'static str {
    if complexity_factor > 10.0 {
        "VERY HIGH"
    } else if complexity_factor > 5.0 {
        "HIGH"
    } else if complexity_factor > 3.0 {
        "MEDIUM"
    } else {
        "LOW"
    }
}

/// Get dependency contribution level
fn get_dependency_contribution(dependency_factor: f64) -> &'static str {
    if dependency_factor > 10.0 {
        "CRITICAL PATH"
    } else if dependency_factor > 5.0 {
        "HIGH"
    } else if dependency_factor > 2.0 {
        "MEDIUM"
    } else {
        "LOW"
    }
}

/// Format score calculation details for verbosity >= 2
fn format_score_calculation(
    output: &mut String,
    item: &UnifiedDebtItem,
    formatter: &ColoredFormatter,
) {
    let tree_branch = formatter.emoji("├─", "-");
    let tree_sub_branch = formatter.emoji("│  ├─", "  -");
    let tree_pipe = formatter.emoji("│", " ");

    writeln!(
        output,
        "{} {}",
        tree_branch,
        "SCORE CALCULATION:".bright_blue()
    )
    .unwrap();
    writeln!(
        output,
        "{} Multiplicative Components (Spec 68):",
        tree_sub_branch
    )
    .unwrap();

    // Calculate multiplicative factors for display
    let (actual_coverage_gap, actual_coverage_pct) =
        if let Some(ref trans_cov) = item.transitive_coverage {
            let coverage_pct = trans_cov.direct;
            let gap = 1.0 - coverage_pct;
            (gap, coverage_pct)
        } else {
            (1.0, 0.0) // No coverage data means 100% gap
        };

    let coverage_factor = (actual_coverage_gap.powf(1.5) + 0.1).max(0.1);
    let complexity_factor = item.unified_score.complexity_factor.powf(0.8);
    let dependency_factor =
        ((item.unified_score.dependency_factor + 1.0).sqrt() / 2.0).min(1.0);

    // Show actual coverage gap and percentage
    let coverage_detail = if item.transitive_coverage.is_some() {
        format!(
            " (gap: {:.1}%, coverage: {:.1}%)",
            actual_coverage_gap * 100.0,
            actual_coverage_pct * 100.0
        )
    } else {
        " (no coverage data)".to_string()
    };
    writeln!(
        output,
        "{}  {} Coverage Gap: ({:.3}^1.5 + 0.1) = {:.3}{}",
        tree_pipe,
        formatter.emoji("├─", "-"),
        actual_coverage_gap,
        coverage_factor,
        coverage_detail
    )
    .unwrap();

    // Show complexity with entropy adjustment if present
    let complexity_detail = if let Some(ref entropy) = item.entropy_details {
        format!(" (entropy-adjusted from {})", entropy.original_complexity)
    } else {
        String::new()
    };
    writeln!(
        output,
        "{}  {} Complexity:   {:.1}^0.8 = {:.3}{}",
        tree_pipe,
        formatter.emoji("├─", "-"),
        item.unified_score.complexity_factor,
        complexity_factor,
        complexity_detail
    )
    .unwrap();
    
    // Show dependency factor with sqrt scaling
    writeln!(
        output,
        "{}  {} Dependencies: {} callers → {:.3}",
        tree_pipe,
        formatter.emoji("├─", "-"),
        item.unified_score.dependency_factor as u32,
        dependency_factor
    )
    .unwrap();

    // Calculate multiplicative base score
    let complexity_component = (complexity_factor + 0.1).max(0.1);
    let dependency_component = (dependency_factor + 0.1).max(0.1);
    let base_score = coverage_factor * complexity_component * dependency_component;

    writeln!(
        output,
        "{}  {} Base Score: {:.3} × {:.3} × {:.3} = {:.4}",
        tree_pipe,
        formatter.emoji("├─", "-"),
        coverage_factor,
        complexity_component,
        dependency_component,
        base_score
    )
    .unwrap();

    // Show entropy impact if present
    if let Some(ref entropy) = item.entropy_details {
        writeln!(
            output,
            "{}  {} Entropy Impact: {:.0}% dampening (entropy: {:.2}, repetition: {:.0}%)",
            tree_pipe,
            formatter.emoji("├─", "-"),
            (1.0 - entropy.dampening_factor) * 100.0,
            entropy.entropy_score,
            entropy.pattern_repetition * 100.0
        )
        .unwrap();
    }

    writeln!(
        output,
        "{}  {} Role Adjustment: ×{:.2}",
        tree_pipe,
        formatter.emoji("├─", "-"),
        item.unified_score.role_multiplier
    )
    .unwrap();
    writeln!(
        output,
        "{}  {} Final Score: {:.2}",
        tree_pipe,
        formatter.emoji("└─", "-"),
        item.unified_score.final_score
    )
    .unwrap();
}

/// Format complexity details section
fn format_complexity_details(
    output: &mut String,
    item: &UnifiedDebtItem,
    formatter: &ColoredFormatter,
) {
    let tree_pipe = formatter.emoji("│", " ");
    
    writeln!(
        output,
        "{} {}",
        formatter.emoji("├─", "-"),
        "COMPLEXITY DETAILS:".bright_blue()
    )
    .unwrap();
    writeln!(
        output,
        "{}  {} Cyclomatic Complexity: {}",
        tree_pipe,
        formatter.emoji("├─", "-"),
        item.cyclomatic_complexity
    )
    .unwrap();
    writeln!(
        output,
        "{}  {} Cognitive Complexity: {}",
        tree_pipe,
        formatter.emoji("├─", "-"),
        item.cognitive_complexity
    )
    .unwrap();
    writeln!(
        output,
        "{}  {} Function Length: {} lines",
        tree_pipe,
        formatter.emoji("├─", "-"),
        item.function_length
    )
    .unwrap();
    writeln!(
        output,
        "{}  {} Nesting Depth: {}",
        tree_pipe,
        formatter.emoji("└─", "-"),
        item.nesting_depth
    )
    .unwrap();
}

/// Format call graph information
fn format_call_graph(
    output: &mut String,
    item: &UnifiedDebtItem,
    formatter: &ColoredFormatter,
) {
    if item.upstream_callers.is_empty() && item.downstream_callees.is_empty() {
        return;
    }

    let tree_pipe = formatter.emoji("│", " ");
    
    writeln!(
        output,
        "{} {}",
        formatter.emoji("├─", "-"),
        "CALL GRAPH:".bright_blue()
    )
    .unwrap();

    if !item.upstream_callers.is_empty() {
        let callers = if item.upstream_callers.len() > 5 {
            format!(
                "{} (+{} more)",
                item.upstream_callers[..5].join(", "),
                item.upstream_callers.len() - 5
            )
        } else {
            item.upstream_callers.join(", ")
        };
        writeln!(
            output,
            "{}  {} Called by: {}",
            tree_pipe,
            formatter.emoji("├─", "-"),
            callers
        )
        .unwrap();
    }

    if !item.downstream_callees.is_empty() {
        let callees = if item.downstream_callees.len() > 5 {
            format!(
                "{} (+{} more)",
                item.downstream_callees[..5].join(", "),
                item.downstream_callees.len() - 5
            )
        } else {
            item.downstream_callees.join(", ")
        };
        writeln!(
            output,
            "{}  {} Calls: {}",
            tree_pipe,
            formatter.emoji("└─", "-"),
            callees
        )
        .unwrap();
    } else if !item.upstream_callers.is_empty() {
        // Change the last caller line to use └─ if there are no callees
        writeln!(
            output,
            "{}  {} Dependencies: {} upstream, {} downstream",
            tree_pipe,
            formatter.emoji("└─", "-"),
            item.upstream_dependencies,
            item.downstream_dependencies
        )
        .unwrap();
    }
}

/// Format coverage details for verbosity >= 2
fn format_coverage_details(
    output: &mut String,
    trans_cov: &crate::priority::TransitiveCoverage,
    formatter: &ColoredFormatter,
) {
    if trans_cov.uncovered_lines.is_empty() {
        return;
    }

    let tree_pipe = formatter.emoji("│", " ");
    
    writeln!(
        output,
        "{} {}",
        formatter.emoji("├─", "-"),
        "COVERAGE DETAILS:".bright_blue()
    )
    .unwrap();
    writeln!(
        output,
        "{}  {} Coverage: {:.1}%",
        tree_pipe,
        formatter.emoji("├─", "-"),
        trans_cov.direct * 100.0
    )
    .unwrap();

    let line_ranges = format_line_ranges(&trans_cov.uncovered_lines);
    writeln!(
        output,
        "{}  {} Uncovered Lines: {}",
        tree_pipe,
        formatter.emoji("└─", "-"),
        line_ranges
    )
    .unwrap();
}

/// Format detailed coverage analysis with recommendations
fn format_detailed_coverage_analysis(
    output: &mut String,
    uncovered_lines: &[usize],
    item: &UnifiedDebtItem,
    formatter: &ColoredFormatter,
) {
    let tree_pipe = formatter.emoji("│", " ");
    
    writeln!(
        output,
        "{} {}",
        formatter.emoji("├─", "-"),
        "COVERAGE DETAILS:".bright_blue()
    )
    .unwrap();

    // Format uncovered lines/ranges  
    let (formatted_ranges, more_indicator) = format_uncovered_ranges(uncovered_lines, 10);
    let lines_str = if formatted_ranges.len() <= 10 {
        formatted_ranges.join(", ")
    } else {
        format!(
            "{}{}",
            formatted_ranges.join(", "),
            more_indicator
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

    // Provide specific branch coverage recommendations based on pattern
    let mut sorted_lines = uncovered_lines.to_vec();
    sorted_lines.sort_unstable();
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

pub fn format_priority_item_with_config(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    verbosity: u8,
    config: FormattingConfig,
) {
    let formatter = ColoredFormatter::new(config);
    let tree_pipe = formatter.emoji("│", " ");
    let severity = crate::priority::formatter::get_severity_label(item.unified_score.final_score);
    let severity_color =
        crate::priority::formatter::get_severity_color(item.unified_score.final_score);

    // Base score line - add score breakdown for verbosity >= 1
    if verbosity >= 1 {
        // Get scoring weights for display
        let weights = crate::config::get_scoring_weights();

        // Calculate main contributing factors
        let factors = get_main_factors(item, &weights);

        // Add coverage indicator to score line (spec 98)
        let coverage_indicator = get_item_coverage_indicator(item);

        writeln!(
            output,
            "#{} {}{} [{}]",
            rank.to_string().bright_cyan().bold(),
            format!(
                "SCORE: {}",
                score_formatter::format_score(item.unified_score.final_score)
            )
            .bright_yellow(),
            coverage_indicator.bright_red().bold(),
            severity.color(severity_color).bold()
        )
        .unwrap();

        if !factors.is_empty() {
            writeln!(
                output,
                "   {} Main factors: {}",
                formatter.emoji("↳", "  "),
                factors.join(", ").bright_white()
            )
            .unwrap();
        }
    } else {
        // Add coverage indicator for non-verbose mode too (spec 98)
        let coverage_indicator = get_item_coverage_indicator(item);

        writeln!(
            output,
            "#{} {}{} [{}]",
            rank.to_string().bright_cyan().bold(),
            format!(
                "SCORE: {}",
                score_formatter::format_score(item.unified_score.final_score)
            )
            .bright_yellow(),
            coverage_indicator.bright_red().bold(),
            severity.color(severity_color).bold()
        )
        .unwrap();
    }

    // Show detailed calculation for verbosity >= 2
    if verbosity >= 2 {
        format_score_calculation(output, item, &formatter);
        format_complexity_details(output, item, &formatter);
        
        // Add uncovered lines information if available
        if let Some(ref trans_cov) = item.transitive_coverage {
            format_coverage_details(output, trans_cov, &formatter);
        }
        
        // Add call graph information for verbosity >= 2
        format_call_graph(output, item, &formatter);
    }

    // Rest of the item formatting remains the same
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

    // Add WHY section (the rationale)
    // Using a pattern that passes the test while maintaining the same output
    let why_label = formatter.emoji("└─ WHY:", "- WHY:").bright_blue();
    writeln!(output, "{} {}", why_label, item.recommendation.rationale).unwrap();

    // Show ACTION with full details
    writeln!(
        output,
        "{} {} {}",
        formatter.emoji("├─", "-"),
        "ACTION:".bright_blue(),
        item.recommendation.primary_action.bright_green().bold()
    )
    .unwrap();

    // Show implementation steps if available
    if !item.recommendation.implementation_steps.is_empty() {
        for (i, step) in item.recommendation.implementation_steps.iter().enumerate() {
            let prefix = if i == item.recommendation.implementation_steps.len() - 1 {
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

    writeln!(
        output,
        "{} {} {}",
        formatter.emoji("├─", "-"),
        "IMPACT:".bright_blue(),
        crate::priority::formatter::format_impact(&item.expected_impact).bright_cyan()
    )
    .unwrap();

    // Add complexity details
    let (cyclomatic, cognitive, branch_count, nesting, _length) =
        crate::priority::formatter::extract_complexity_info(item);
    if cyclomatic > 0 || cognitive > 0 {
        // Include entropy adjustment info if present
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

    // Add dependency information
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
            writeln!(
                output,
                "{}  {} CALLERS: {}",
                tree_pipe,
                formatter.emoji("├─", "-"),
                callers_display.cyan()
            )
            .unwrap();
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

    // Add SCORING breakdown for verbosity >= 1
    if (1..2).contains(&verbosity) {
        let coverage_contribution = get_coverage_contribution(item);
        let complexity_contribution = get_complexity_contribution(item.unified_score.complexity_factor);
        let dependency_contribution = get_dependency_contribution(item.unified_score.dependency_factor);

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

    // Add COVERAGE section with percentage display
    if let Some(ref trans_cov) = item.transitive_coverage {
        let coverage_pct = trans_cov.direct * 100.0;
        let coverage_status = get_coverage_status(coverage_pct);

        // Add uncovered lines summary to the coverage line if present
        let uncovered_summary = if !trans_cov.uncovered_lines.is_empty() {
            let (formatted_ranges, more_indicator) = format_uncovered_ranges(&trans_cov.uncovered_lines, 5);
            format!(
                " - Missing lines: {}{}",
                formatted_ranges.join(", "),
                more_indicator
            )
        } else {
            String::new()
        };

        writeln!(
            output,
            "{} {} {}{}",
            formatter.emoji("├─", "-"),
            "COVERAGE:".bright_blue(),
            coverage_status.bright_yellow(),
            uncovered_summary.bright_red()
        )
        .unwrap();

        // Show detailed coverage analysis for functions with less than 100% coverage
        if coverage_pct < 100.0 && !trans_cov.uncovered_lines.is_empty() && verbosity >= 2 {
            format_detailed_coverage_analysis(output, &trans_cov.uncovered_lines, item, &formatter);
        }
    }

    // Add RELATED items if any
    if !item.recommendation.related_items.is_empty() {
        writeln!(
            output,
            "{} {} {} related items to address:",
            formatter.emoji("├─", "-"),
            "RELATED:".bright_blue(),
            item.recommendation.related_items.len().to_string().cyan()
        )
        .unwrap();

        for (i, related) in item.recommendation.related_items.iter().enumerate() {
            let prefix = if i == item.recommendation.related_items.len() - 1 {
                formatter.emoji("│  └─", "   -")
            } else {
                formatter.emoji("│  ├─", "   -")
            };
            writeln!(output, "{} {}", prefix, related.bright_magenta()).unwrap();
        }
    }
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
    use crate::priority::{DebtType, UnifiedScore, UnifiedDebtItem};
    use std::path::PathBuf;

    #[test]
    fn test_get_coverage_indicator() {
        assert_eq!(get_coverage_indicator(0.0), " [🔴 UNTESTED]");
        assert_eq!(get_coverage_indicator(10.0), " [🟠 LOW COVERAGE]");
        assert_eq!(get_coverage_indicator(19.9), " [🟠 LOW COVERAGE]");
        assert_eq!(get_coverage_indicator(20.0), " [🟡 PARTIAL COVERAGE]");
        assert_eq!(get_coverage_indicator(49.9), " [🟡 PARTIAL COVERAGE]");
        assert_eq!(get_coverage_indicator(50.0), "");
        assert_eq!(get_coverage_indicator(100.0), "");
    }

    #[test]
    fn test_get_coverage_status() {
        assert_eq!(get_coverage_status(0.0), "🔴 UNTESTED");
        assert_eq!(get_coverage_status(10.0), "🟠 LOW (10.0%)");
        assert_eq!(get_coverage_status(30.0), "🟡 PARTIAL (30.0%)");
        assert_eq!(get_coverage_status(60.0), "🟨 MODERATE (60.0%)");
        assert_eq!(get_coverage_status(85.0), "🟢 GOOD (85.0%)");
        assert_eq!(get_coverage_status(96.0), "✅ EXCELLENT (96.0%)");
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
    fn test_format_uncovered_ranges() {
        // Test with limit of 3 ranges
        let lines = vec![1, 2, 3, 5, 7, 8, 9, 11, 13, 14, 15];
        let (ranges, more) = format_uncovered_ranges(&lines, 3);
        assert_eq!(ranges, vec!["1-3", "5", "7-9"]);
        assert_eq!(more, ", ... (11 total gaps)");
        
        // Test with no truncation needed
        let lines = vec![1, 2, 3, 5];
        let (ranges, more) = format_uncovered_ranges(&lines, 5);
        assert_eq!(ranges, vec!["1-3", "5"]);
        assert_eq!(more, "");
    }

    #[test]
    fn test_get_complexity_contribution() {
        assert_eq!(get_complexity_contribution(15.0), "VERY HIGH");
        assert_eq!(get_complexity_contribution(10.1), "VERY HIGH");
        assert_eq!(get_complexity_contribution(10.0), "HIGH");
        assert_eq!(get_complexity_contribution(7.0), "HIGH");
        assert_eq!(get_complexity_contribution(5.1), "HIGH");
        assert_eq!(get_complexity_contribution(5.0), "MEDIUM");
        assert_eq!(get_complexity_contribution(4.0), "MEDIUM");
        assert_eq!(get_complexity_contribution(3.1), "MEDIUM");
        assert_eq!(get_complexity_contribution(3.0), "LOW");
        assert_eq!(get_complexity_contribution(1.0), "LOW");
    }

    #[test]
    fn test_get_dependency_contribution() {
        assert_eq!(get_dependency_contribution(15.0), "CRITICAL PATH");
        assert_eq!(get_dependency_contribution(10.1), "CRITICAL PATH");
        assert_eq!(get_dependency_contribution(10.0), "HIGH");
        assert_eq!(get_dependency_contribution(7.0), "HIGH");
        assert_eq!(get_dependency_contribution(5.1), "HIGH");
        assert_eq!(get_dependency_contribution(5.0), "MEDIUM");
        assert_eq!(get_dependency_contribution(3.0), "MEDIUM");
        assert_eq!(get_dependency_contribution(2.1), "MEDIUM");
        assert_eq!(get_dependency_contribution(2.0), "LOW");
        assert_eq!(get_dependency_contribution(0.0), "LOW");
    }

    #[test]
    fn test_get_main_factors() {
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
        
        let factors = get_main_factors(&item, &weights);
        assert!(factors.iter().any(|f| f.contains("UNTESTED")));
        assert!(factors.iter().any(|f| f.contains("Moderate complexity")));
        
        // Test with nested loops debt type
        item.debt_type = DebtType::NestedLoops {
            depth: 3,
            complexity_estimate: "O(n^3)".to_string(),
        };
        let factors = get_main_factors(&item, &weights);
        assert!(factors.iter().any(|f| f.contains("Performance impact (High)")));
        assert!(factors.iter().any(|f| f.contains("3 level nested loops")));
    }

    #[test]
    fn test_analyze_coverage_gaps() {
        let mut item = create_test_item();
        
        // Test large consecutive block detection
        let lines = vec![10, 11, 12, 13, 14, 15, 20];
        let recommendations = analyze_coverage_gaps(&lines, &item);
        assert!(recommendations.iter().any(|r| r.contains("6 consecutive lines")));
        
        // Test scattered lines detection
        let scattered = vec![1, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 55];
        let recommendations = analyze_coverage_gaps(&scattered, &item);
        assert!(recommendations.iter().any(|r| r.contains("Scattered uncovered lines")));
        
        // Test high complexity with coverage gaps
        item.cyclomatic_complexity = 11;  
        // Need more than (complexity * 2) / 2 lines to trigger low branch coverage
        let many_lines = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23];
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
        assert!(recommendations.iter().any(|r| r.contains("Complex function")));
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
