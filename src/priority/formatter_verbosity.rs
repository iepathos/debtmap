use crate::formatting::{ColoredFormatter, FormattingConfig, OutputFormatter};
use crate::priority::{score_formatter, UnifiedDebtItem};
use colored::*;
use std::fmt::Write;

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
        let mut factors = vec![];

        // Show coverage info - both good and bad coverage are important factors
        if let Some(ref trans_cov) = item.transitive_coverage {
            let coverage_pct = trans_cov.direct * 100.0;
            if coverage_pct == 0.0 {
                // ZERO COVERAGE - Critical priority (spec 98)
                factors.push(format!(
                    "🔴 UNTESTED (0% coverage, weight: {:.0}%)",
                    weights.coverage * 100.0
                ));
            } else if coverage_pct < 20.0 {
                // Very low coverage - high priority (spec 98)
                factors.push(format!(
                    "🟠 LOW COVERAGE ({:.1}%, weight: {:.0}%)",
                    coverage_pct,
                    weights.coverage * 100.0
                ));
            } else if coverage_pct < 50.0 {
                // Partial coverage (spec 98)
                factors.push(format!(
                    "🟡 PARTIAL COVERAGE ({:.1}%, weight: {:.0}%)",
                    coverage_pct,
                    weights.coverage * 100.0
                ));
            } else if coverage_pct >= 95.0 {
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
        } else if item.unified_score.coverage_factor >= 10.0 {
            // No coverage data and max coverage factor - likely 0% coverage (spec 98)
            factors.push(format!(
                "🔴 UNTESTED (no coverage data, weight: {:.0}%)",
                weights.coverage * 100.0
            ));
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

        // Add coverage indicator to score line (spec 98)
        let coverage_indicator = if let Some(ref trans_cov) = item.transitive_coverage {
            let coverage_pct = trans_cov.direct * 100.0;
            match coverage_pct {
                0.0 => " [🔴 UNTESTED]",
                c if c < 20.0 => " [🟠 LOW COVERAGE]",
                c if c < 50.0 => " [🟡 PARTIAL COVERAGE]",
                _ => "",
            }
        } else if item.unified_score.coverage_factor >= 10.0 {
            " [🔴 UNTESTED]"
        } else {
            ""
        };

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
        let coverage_indicator = if let Some(ref trans_cov) = item.transitive_coverage {
            let coverage_pct = trans_cov.direct * 100.0;
            match coverage_pct {
                0.0 => " [🔴 UNTESTED]",
                c if c < 20.0 => " [🟠 LOW COVERAGE]",
                c if c < 50.0 => " [🟡 PARTIAL COVERAGE]",
                _ => "",
            }
        } else if item.unified_score.coverage_factor >= 10.0 {
            " [🔴 UNTESTED]"
        } else {
            ""
        };

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
        // Get actual coverage percentage and gap from the data
        let (actual_coverage_gap, actual_coverage_pct) =
            if let Some(ref trans_cov) = item.transitive_coverage {
                let coverage_pct = trans_cov.direct;
                let gap = 1.0 - coverage_pct;
                (gap, coverage_pct)
            } else {
                (1.0, 0.0) // No coverage data means 100% gap
            };

        // Calculate the coverage factor using the same formula as in scoring
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

        // Add detailed complexity metrics for verbosity >= 2
        writeln!(output, "{} COMPLEXITY DETAILS:", formatter.emoji("├─", "-")).unwrap();
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

        // Add uncovered lines information if available
        if let Some(ref trans_cov) = item.transitive_coverage {
            if !trans_cov.uncovered_lines.is_empty() {
                writeln!(output, "{} COVERAGE DETAILS:", formatter.emoji("├─", "-")).unwrap();
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
        }

        // Add call graph information for verbosity >= 2
        if !item.upstream_callers.is_empty() || !item.downstream_callees.is_empty() {
            writeln!(output, "{} CALL GRAPH:", formatter.emoji("├─", "-")).unwrap();

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

    // Add WHY section (the rationale)
    writeln!(
        output,
        "{} {}",
        format!("{} WHY:", formatter.emoji("├─", "-")).bright_blue(),
        item.recommendation.rationale.bright_white()
    )
    .unwrap();

    // Show ACTION with full details
    writeln!(
        output,
        "{} {}",
        format!("{} ACTION:", formatter.emoji("├─", "-")).bright_blue(),
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

    // Add SCORING breakdown for verbosity >= 1
    if (1..2).contains(&verbosity) {
        let _weights = crate::config::get_scoring_weights();

        // Calculate the individual factor contributions
        let coverage_contribution = if let Some(ref trans_cov) = item.transitive_coverage {
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
        };

        let complexity_contribution = if item.unified_score.complexity_factor > 10.0 {
            "VERY HIGH"
        } else if item.unified_score.complexity_factor > 5.0 {
            "HIGH"
        } else if item.unified_score.complexity_factor > 3.0 {
            "MEDIUM"
        } else {
            "LOW"
        };

        let dependency_contribution = if item.unified_score.dependency_factor > 10.0 {
            "CRITICAL PATH"
        } else if item.unified_score.dependency_factor > 5.0 {
            "HIGH"
        } else if item.unified_score.dependency_factor > 2.0 {
            "MEDIUM"
        } else {
            "LOW"
        };

        writeln!(
            output,
            "{} Coverage: {} | Complexity: {} | Dependencies: {}",
            format!("{} SCORING:", formatter.emoji("├─", "-")).bright_blue(),
            coverage_contribution.bright_yellow(),
            complexity_contribution.bright_yellow(),
            dependency_contribution.bright_yellow()
        )
        .unwrap();
    }

    // Add COVERAGE section with percentage display
    if let Some(ref trans_cov) = item.transitive_coverage {
        let coverage_pct = trans_cov.direct * 100.0;
        let coverage_status = match coverage_pct {
            0.0 => "🔴 UNTESTED".to_string(),
            c if c < 20.0 => format!("🟠 LOW ({:.1}%)", c),
            c if c < 50.0 => format!("🟡 PARTIAL ({:.1}%)", c),
            c if c < 80.0 => format!("🟨 MODERATE ({:.1}%)", c),
            c if c < 95.0 => format!("🟢 GOOD ({:.1}%)", c),
            _ => format!("✅ EXCELLENT ({:.1}%)", coverage_pct),
        };

        // Add uncovered lines summary to the coverage line if present
        let uncovered_summary = if !trans_cov.uncovered_lines.is_empty() {
            let mut sorted_lines = trans_cov.uncovered_lines.clone();
            sorted_lines.sort_unstable();

            // Group consecutive lines for display
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

            let formatted_ranges: Vec<String> = ranges
                .iter()
                .take(5) // Show first 5 ranges
                .map(|&(start, end)| {
                    if start == end {
                        format!("{}", start)
                    } else {
                        format!("{}-{}", start, end)
                    }
                })
                .collect();

            let more_indicator = if ranges.len() > 5 {
                format!(", ... ({} total gaps)", trans_cov.uncovered_lines.len())
            } else {
                String::new()
            };

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
            "{} {}{}",
            format!("{} COVERAGE:", formatter.emoji("├─", "-")).bright_blue(),
            coverage_status.bright_yellow(),
            uncovered_summary.bright_red()
        )
        .unwrap();

        // Show detailed coverage analysis for functions with less than 100% coverage
        if coverage_pct < 100.0 && !trans_cov.uncovered_lines.is_empty() && verbosity >= 2 {
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

    // Add RELATED items if any
    if !item.recommendation.related_items.is_empty() {
        writeln!(
            output,
            "{} {} related items to address:",
            format!("{} RELATED:", formatter.emoji("├─", "-")).bright_blue(),
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
