use crate::priority::classification::CoverageLevel;
use crate::priority::{TransitiveCoverage, UnifiedDebtItem};
use colored::*;
use std::fmt::Write;

/// Format coverage section using shared classification (spec 202)
pub fn format_coverage_section(
    output: &mut String,
    item: &UnifiedDebtItem,
    _formatter: &crate::formatting::ColoredFormatter,
    verbosity: u8,
    tree_pipe: &str,
    has_coverage_data: bool,
) {
    // Skip entire coverage section if no LCOV data was provided (spec 180)
    if !has_coverage_data {
        return;
    }

    if let Some(ref trans_cov) = item.transitive_coverage {
        let coverage_pct = trans_cov.direct * 100.0;

        // Always show simple coverage percentage line
        writeln!(
            output,
            "├─ {}: {:.1}% coverage",
            "COVERAGE".bright_blue(),
            coverage_pct
        )
        .unwrap();

        // For verbosity >= 2, show detailed analysis with test recommendations
        if coverage_pct < 100.0 && !trans_cov.uncovered_lines.is_empty() && verbosity >= 2 {
            format_detailed_coverage_analysis(output, trans_cov, item, _formatter, tree_pipe);
        }
    } else if has_coverage_data {
        // Coverage data was provided but this function was not found in LCOV
        // Always show "no coverage data" for consistency (spec 180)
        writeln!(output, "├─ {}: no coverage data", "COVERAGE".bright_blue()).unwrap();
    } else {
        // No coverage data available - show this explicitly
        writeln!(output, "├─ {}: no coverage data", "COVERAGE".bright_blue()).unwrap();
    }
}

/// Format detailed coverage analysis for verbosity >= 2
fn format_detailed_coverage_analysis(
    output: &mut String,
    trans_cov: &TransitiveCoverage,
    item: &UnifiedDebtItem,
    _formatter: &crate::formatting::ColoredFormatter,
    tree_pipe: &str,
) {
    writeln!(output, "- {}", "COVERAGE DETAILS:".bright_blue()).unwrap();

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
        "{}  - Uncovered lines: {}",
        tree_pipe,
        lines_str.bright_red()
    )
    .unwrap();

    let branch_recommendations = analyze_coverage_gaps(&sorted_lines, item);
    if !branch_recommendations.is_empty() {
        writeln!(output, "{}  - Test focus areas:", tree_pipe).unwrap();
        for rec in branch_recommendations.iter().take(3) {
            writeln!(output, "{}      * {}", tree_pipe, rec.yellow()).unwrap();
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

/// Pure function to group consecutive lines into ranges
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

/// Pure function to format ranges as strings
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

/// Pure function to format coverage factor description
pub fn format_coverage_factor_description(
    item: &UnifiedDebtItem,
    _weights: &crate::config::ScoringWeights,
    has_coverage_data: bool,
) -> Option<String> {
    if !has_coverage_data {
        return None; // Don't show coverage info when not available
    }

    if let Some(ref trans_cov) = item.transitive_coverage {
        let coverage_pct = trans_cov.direct * 100.0;
        let level = CoverageLevel::from_percentage(coverage_pct);
        match level {
            CoverageLevel::Untested => Some("[UNTESTED] (0% coverage, weight: 50%)".to_string()),
            CoverageLevel::Low => Some(format!(
                "[WARN LOW COVERAGE] ({:.1}%, weight: 50%)",
                coverage_pct
            )),
            CoverageLevel::Partial => Some(format!(
                "[WARN PARTIAL COVERAGE] ({:.1}%, weight: 50%)",
                coverage_pct
            )),
            CoverageLevel::Excellent => Some(format!("Excellent coverage {:.1}%", coverage_pct)),
            CoverageLevel::Good => Some(format!("Good coverage {:.1}%", coverage_pct)),
            CoverageLevel::Moderate => {
                if item.unified_score.coverage_factor > 3.0 {
                    Some(format!("Line coverage {:.1}% (weight: 50%)", coverage_pct))
                } else {
                    None
                }
            }
        }
    } else if item.unified_score.coverage_factor >= 10.0 {
        Some("[UNTESTED] (no coverage data, weight: 50%)".to_string())
    } else if item.unified_score.coverage_factor > 3.0 {
        Some("No coverage data (weight: 50%)".to_string())
    } else {
        None
    }
}

/// Pure function to classify coverage contribution
pub fn classify_coverage_contribution(item: &UnifiedDebtItem) -> &'static str {
    if let Some(ref trans_cov) = item.transitive_coverage {
        let coverage_pct = trans_cov.direct * 100.0;
        let level = CoverageLevel::from_percentage(coverage_pct);
        match level {
            CoverageLevel::Untested => "CRITICAL (0% coverage)",
            CoverageLevel::Low => "HIGH (low coverage)",
            CoverageLevel::Partial => "MEDIUM (partial coverage)",
            _ => "LOW",
        }
    } else {
        "HIGH (no data)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
