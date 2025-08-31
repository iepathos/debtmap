use crate::formatting::{ColoredFormatter, FormattingConfig, OutputFormatter};
use crate::priority::UnifiedDebtItem;
use colored::*;
use std::fmt::Write;

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
        let mut factors = vec![];

        // Show coverage info - both good and bad coverage are important factors
        if let Some(ref trans_cov) = item.transitive_coverage {
            let coverage_pct = trans_cov.direct * 100.0;
            if coverage_pct >= 95.0 {
                // Excellent coverage - show as positive factor
                factors.push(format!("Excellent coverage {:.1}%", coverage_pct));
            } else if coverage_pct >= 80.0 {
                // Good coverage
                factors.push(format!("Good coverage {:.1}%", coverage_pct));
            } else if item.unified_score.coverage_factor > 3.0 {
                // Poor coverage - show as negative factor with weight
                factors.push(format!(
                    "Line coverage {:.1}% (weight: {:.0}%)",
                    coverage_pct,
                    weights.coverage * 100.0
                ));
            }
        } else if item.unified_score.coverage_factor > 3.0 {
            // No coverage data but high coverage factor
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

        writeln!(
            output,
            "#{} {} [{}]",
            rank.to_string().bright_cyan().bold(),
            format!("SCORE: {:.2}", item.unified_score.final_score).bright_yellow(),
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
        writeln!(
            output,
            "#{} {} [{}]",
            rank.to_string().bright_cyan().bold(),
            format!("SCORE: {:.2}", item.unified_score.final_score).bright_yellow(),
            severity.color(severity_color).bold()
        )
        .unwrap();
    }

    // Show detailed calculation for verbosity >= 2
    if verbosity >= 2 {
        let _weights = crate::config::get_scoring_weights();
        let tree_branch = formatter.emoji("├─", "-");
        let tree_sub_branch = formatter.emoji("│  ├─", "  -");
        let _tree_end = formatter.emoji("│  └─", "  -");
        let tree_pipe = formatter.emoji("│", " ");

        writeln!(output, "{} SCORE CALCULATION:", tree_branch).unwrap();
        writeln!(
            output,
            "{} Multiplicative Components (Spec 68):",
            tree_sub_branch
        )
        .unwrap();

        // Calculate multiplicative factors for display
        let coverage_gap = item.unified_score.coverage_factor / 10.0; // Convert from display scale
        let coverage_factor = coverage_gap.powf(1.5);
        let complexity_factor = item.unified_score.complexity_factor.powf(0.8);
        let dependency_factor =
            ((item.unified_score.dependency_factor + 1.0).sqrt() / 2.0).min(1.0);

        // Show coverage gap with exponential scaling
        let coverage_detail = if let Some(ref trans_cov) = item.transitive_coverage {
            format!(
                " (gap: {:.1}%, coverage: {:.1}%)",
                coverage_gap * 100.0,
                trans_cov.direct * 100.0
            )
        } else if coverage_gap < 1.0 {
            format!(" (gap: {:.1}%)", coverage_gap * 100.0)
        } else {
            " (no coverage data)".to_string()
        };
        writeln!(
            output,
            "{}  {} Coverage Gap: {:.3}^1.5 = {:.3}{}",
            tree_pipe,
            formatter.emoji("├─", "-"),
            coverage_gap,
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

        // Calculate multiplicative base score (spec 68)
        // Apply small constants to avoid zero multiplication
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

    // Rest of the item formatting remains the same
    writeln!(
        output,
        "{} {}:{} {}()",
        format!("{} LOCATION:", formatter.emoji("├─", "-")).bright_blue(),
        item.location.file.display(),
        item.location.line,
        item.location.function.bright_green()
    )
    .unwrap();

    writeln!(
        output,
        "{} {}",
        format!("{} ACTION:", formatter.emoji("├─", "-")).bright_blue(),
        item.recommendation.primary_action.bright_green().bold()
    )
    .unwrap();

    writeln!(
        output,
        "{} {}",
        format!("{} IMPACT:", formatter.emoji("├─", "-")).bright_blue(),
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
                "{} cyclomatic={} (adj:{}), branches={}, cognitive={}, nesting={}, entropy={:.2}",
                format!("{} COMPLEXITY:", formatter.emoji("├─", "-")).bright_blue(),
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
                "{} cyclomatic={}, branches={}, cognitive={}, nesting={}",
                format!("{} COMPLEXITY:", formatter.emoji("├─", "-")).bright_blue(),
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
            "{} {} upstream, {} downstream",
            format!("{} DEPENDENCIES:", formatter.emoji("├─", "-")).bright_blue(),
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

    // Add coverage details section when coverage data is available and function has incomplete coverage
    if let Some(ref trans_cov) = item.transitive_coverage {
        let coverage_pct = trans_cov.direct * 100.0;

        // Show coverage details for functions with less than 100% coverage that have uncovered lines
        if coverage_pct < 100.0 && !trans_cov.uncovered_lines.is_empty() {
            writeln!(
                output,
                "{}",
                format!("{} COVERAGE DETAILS:", formatter.emoji("├─", "-")).bright_blue()
            )
            .unwrap();

            // Sort the uncovered lines first
            let mut sorted_lines = trans_cov.uncovered_lines.clone();
            sorted_lines.sort_unstable();

            // Group consecutive lines into ranges for better readability
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

            // Format uncovered lines/ranges
            let formatted_ranges: Vec<String> = ranges
                .iter()
                .map(|&(start, end)| {
                    if start == end {
                        format!("{}", start)
                    } else {
                        format!("{}-{}", start, end)
                    }
                })
                .collect();

            // Display uncovered lines in a compact format
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

            // Provide specific branch coverage recommendations based on pattern
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
    }

    // Add rationale
    writeln!(
        output,
        "{} {}",
        formatter.emoji("└─ WHY:", "- WHY:").bright_blue(),
        item.recommendation.rationale
    )
    .unwrap();
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
