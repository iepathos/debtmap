use std::path::Path;

/// Coverage gap with different precision levels
#[derive(Debug, Clone, PartialEq)]
pub enum CoverageGap {
    /// Precise gap from line-level coverage data
    Precise {
        uncovered_lines: Vec<usize>,
        instrumented_lines: u32,
        percentage: f64,
    },

    /// Estimated gap from function-level percentage
    Estimated {
        percentage: f64,
        total_lines: u32,
        estimated_uncovered: u32,
    },

    /// No coverage data available
    Unknown { total_lines: u32 },
}

impl CoverageGap {
    /// Format for user display
    pub fn format(&self) -> String {
        match self {
            CoverageGap::Precise {
                uncovered_lines,
                instrumented_lines,
                percentage,
            } => {
                let count = uncovered_lines.len();
                if count == 0 {
                    "Fully covered".to_string()
                } else if count == 1 {
                    format!(
                        "1 line uncovered ({:.0}% gap) - line {}",
                        percentage, uncovered_lines[0]
                    )
                } else {
                    format!(
                        "{} lines uncovered ({:.0}% gap of {} instrumented lines) - lines {}",
                        count,
                        percentage,
                        instrumented_lines,
                        format_line_ranges(uncovered_lines)
                    )
                }
            }

            CoverageGap::Estimated {
                percentage,
                estimated_uncovered,
                ..
            } => {
                if *percentage >= 99.0 {
                    format!("~100% gap (estimated, {} lines)", estimated_uncovered)
                } else if *percentage < 5.0 {
                    format!("~{}% gap (mostly covered)", *percentage as u32)
                } else {
                    format!(
                        "~{}% gap (estimated, ~{} lines)",
                        *percentage as u32, estimated_uncovered
                    )
                }
            }

            CoverageGap::Unknown { total_lines } => {
                format!("Coverage data unavailable ({} lines)", total_lines)
            }
        }
    }

    /// Get percentage gap
    pub fn percentage(&self) -> f64 {
        match self {
            CoverageGap::Precise { percentage, .. } => *percentage,
            CoverageGap::Estimated { percentage, .. } => *percentage,
            CoverageGap::Unknown { .. } => 100.0,
        }
    }

    /// Get uncovered line count
    pub fn uncovered_count(&self) -> u32 {
        match self {
            CoverageGap::Precise {
                uncovered_lines, ..
            } => uncovered_lines.len() as u32,
            CoverageGap::Estimated {
                estimated_uncovered,
                ..
            } => *estimated_uncovered,
            CoverageGap::Unknown { total_lines } => *total_lines,
        }
    }

    /// Get specific uncovered line numbers (if available)
    pub fn uncovered_lines(&self) -> Option<&[usize]> {
        match self {
            CoverageGap::Precise {
                uncovered_lines, ..
            } => Some(uncovered_lines),
            _ => None,
        }
    }
}

/// Format line numbers as compact ranges
/// e.g., [10, 11, 12, 15, 20, 21] → "10-12, 15, 20-21"
fn format_line_ranges(lines: &[usize]) -> String {
    if lines.is_empty() {
        return String::new();
    }

    let mut sorted = lines.to_vec();
    sorted.sort_unstable();

    let mut ranges = vec![];
    let mut range_start = sorted[0];
    let mut range_end = sorted[0];

    for &line in sorted.iter().skip(1) {
        if line == range_end + 1 {
            range_end = line;
        } else {
            if range_start == range_end {
                ranges.push(format!("{}", range_start));
            } else {
                ranges.push(format!("{}-{}", range_start, range_end));
            }
            range_start = line;
            range_end = line;
        }
    }

    // Add final range
    if range_start == range_end {
        ranges.push(format!("{}", range_start));
    } else {
        ranges.push(format!("{}-{}", range_start, range_end));
    }

    ranges.join(", ")
}

/// Line-level coverage data for a function
#[derive(Debug, Clone, Default)]
pub struct LineCoverageData {
    /// Lines with >0 hits
    pub covered_lines: u32,

    /// Specific uncovered line numbers
    pub uncovered_lines: Vec<usize>,
}

/// Calculate coverage gap with line-level precision
///
/// This function provides accurate coverage gap reporting by using line-level
/// coverage data when available, falling back to function-level estimates when not.
///
/// # Arguments
/// * `coverage_pct` - Function-level coverage percentage (0.0-1.0)
/// * `function_length` - Total lines in function from AST
/// * `file` - File path for coverage lookup
/// * `function_name` - Function name for coverage lookup
/// * `start_line` - Starting line number of function
/// * `coverage_data` - Optional LCOV coverage data
///
/// # Returns
/// A `CoverageGap` enum indicating the precision level and gap details
pub fn calculate_coverage_gap(
    coverage_pct: f64,
    function_length: u32,
    file: &Path,
    function_name: &str,
    start_line: usize,
    coverage_data: Option<&super::lcov::LcovData>,
) -> CoverageGap {
    // Try to get line-level data first
    if let Some(data) = coverage_data {
        if let Some(line_cov) = data.get_function_uncovered_lines(file, function_name, start_line) {
            // We have precise line-level data
            let uncovered_count = line_cov.len();

            // Calculate instrumented lines (covered + uncovered)
            // We can infer covered lines from the function's coverage data
            if let Some(funcs) = data.functions.get(file) {
                if let Some(func) = funcs
                    .iter()
                    .find(|f| f.name == function_name || f.start_line == start_line)
                {
                    let instrumented_lines = func.uncovered_lines.len() as u32
                        + ((func.coverage_percentage / 100.0
                            * (func.uncovered_lines.len() as f64 + uncovered_count as f64))
                            as u32);

                    let instrumented_lines = instrumented_lines.max(uncovered_count as u32);

                    let gap_percentage = if instrumented_lines > 0 {
                        (uncovered_count as f64 / instrumented_lines as f64) * 100.0
                    } else {
                        0.0
                    };

                    return CoverageGap::Precise {
                        uncovered_lines: line_cov,
                        instrumented_lines,
                        percentage: gap_percentage,
                    };
                }
            }
        }
    }

    // Fallback to percentage-based estimate
    let gap_pct = (1.0 - coverage_pct) * 100.0;
    let estimated_uncovered = (function_length as f64 * (gap_pct / 100.0)) as u32;

    CoverageGap::Estimated {
        percentage: gap_pct,
        total_lines: function_length,
        estimated_uncovered,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_line_ranges_single() {
        let lines = vec![52];
        assert_eq!(format_line_ranges(&lines), "52");
    }

    #[test]
    fn test_format_line_ranges_contiguous() {
        let lines = vec![10, 11, 12];
        assert_eq!(format_line_ranges(&lines), "10-12");
    }

    #[test]
    fn test_format_line_ranges_mixed() {
        let lines = vec![10, 11, 12, 15, 20, 21];
        assert_eq!(format_line_ranges(&lines), "10-12, 15, 20-21");
    }

    #[test]
    fn test_format_line_ranges_non_contiguous() {
        let lines = vec![10, 15, 20];
        assert_eq!(format_line_ranges(&lines), "10, 15, 20");
    }

    #[test]
    fn test_format_line_ranges_empty() {
        let lines: Vec<usize> = vec![];
        assert_eq!(format_line_ranges(&lines), "");
    }

    #[test]
    fn test_format_line_ranges_unsorted() {
        let lines = vec![20, 10, 11, 15, 12];
        assert_eq!(format_line_ranges(&lines), "10-12, 15, 20");
    }

    #[test]
    fn test_coverage_gap_precise_single_line() {
        let gap = CoverageGap::Precise {
            uncovered_lines: vec![52],
            instrumented_lines: 9,
            percentage: 11.1,
        };

        assert_eq!(gap.format(), "1 line uncovered (11% gap) - line 52");
        assert!((gap.percentage() - 11.1).abs() < 0.1);
        assert_eq!(gap.uncovered_count(), 1);
        assert_eq!(gap.uncovered_lines(), Some(&[52][..]));
    }

    #[test]
    fn test_coverage_gap_precise_multiple_lines() {
        let gap = CoverageGap::Precise {
            uncovered_lines: vec![10, 11, 12, 15],
            instrumented_lines: 20,
            percentage: 20.0,
        };

        let formatted = gap.format();
        assert!(formatted.contains("4 lines uncovered"));
        assert!(formatted.contains("20% gap"));
        assert!(formatted.contains("10-12, 15"));
        assert_eq!(gap.uncovered_count(), 4);
    }

    #[test]
    fn test_coverage_gap_precise_fully_covered() {
        let gap = CoverageGap::Precise {
            uncovered_lines: vec![],
            instrumented_lines: 10,
            percentage: 0.0,
        };

        assert_eq!(gap.format(), "Fully covered");
        assert_eq!(gap.percentage(), 0.0);
        assert_eq!(gap.uncovered_count(), 0);
    }

    #[test]
    fn test_coverage_gap_estimated() {
        let gap = CoverageGap::Estimated {
            percentage: 50.0,
            total_lines: 20,
            estimated_uncovered: 10,
        };

        assert!(gap.format().contains("~50% gap"));
        assert!(gap.format().contains("~10 lines"));
        assert_eq!(gap.percentage(), 50.0);
        assert_eq!(gap.uncovered_count(), 10);
        assert_eq!(gap.uncovered_lines(), None);
    }

    #[test]
    fn test_coverage_gap_estimated_high() {
        let gap = CoverageGap::Estimated {
            percentage: 99.5,
            total_lines: 20,
            estimated_uncovered: 20,
        };

        assert!(gap.format().contains("~100% gap"));
        assert_eq!(gap.percentage(), 99.5);
    }

    #[test]
    fn test_coverage_gap_estimated_low() {
        let gap = CoverageGap::Estimated {
            percentage: 3.0,
            total_lines: 100,
            estimated_uncovered: 3,
        };

        assert!(gap.format().contains("~3% gap"));
        assert!(gap.format().contains("mostly covered"));
    }

    #[test]
    fn test_coverage_gap_unknown() {
        let gap = CoverageGap::Unknown { total_lines: 15 };

        assert!(gap.format().contains("Coverage data unavailable"));
        assert!(gap.format().contains("15 lines"));
        assert_eq!(gap.percentage(), 100.0);
        assert_eq!(gap.uncovered_count(), 15);
        assert_eq!(gap.uncovered_lines(), None);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn gap_percentage_always_between_0_and_100(
            uncovered in 0u32..100,
            covered in 0u32..100,
        ) {
            let total = uncovered + covered;
            if total == 0 {
                return Ok(()); // Skip degenerate case
            }

            let percentage = (uncovered as f64 / total as f64) * 100.0;
            prop_assert!((0.0..=100.0).contains(&percentage));
        }

        #[test]
        fn gap_formatting_never_panics(
            uncovered_lines in prop::collection::vec(1usize..1000, 0..50),
            total in 1u32..100,
        ) {
            let gap = CoverageGap::Precise {
                uncovered_lines: uncovered_lines.clone(),
                instrumented_lines: total,
                percentage: (uncovered_lines.len() as f64 / total as f64) * 100.0,
            };

            // Should never panic, regardless of input
            let formatted = gap.format();
            prop_assert!(!formatted.is_empty());
        }

        #[test]
        fn zero_uncovered_lines_reports_full_coverage(
            total in 1u32..100,
        ) {
            let gap = CoverageGap::Precise {
                uncovered_lines: vec![],
                instrumented_lines: total,
                percentage: 0.0,
            };

            prop_assert!(gap.format().contains("Fully covered"));
        }

        #[test]
        fn all_lines_uncovered_reports_100_percent(
            line_count in 1u32..50,
        ) {
            let uncovered: Vec<usize> = (1..=line_count as usize).collect();
            let gap = CoverageGap::Precise {
                uncovered_lines: uncovered.clone(),
                instrumented_lines: line_count,
                percentage: 100.0,
            };

            let formatted = gap.format();
            prop_assert!(formatted.contains("100"));
        }

        #[test]
        fn line_range_formatting_stable(
            mut lines in prop::collection::vec(1usize..1000, 1..30),
        ) {
            // Remove duplicates and sort
            lines.sort_unstable();
            lines.dedup();

            if lines.is_empty() {
                return Ok(());
            }

            // Should format without panicking
            let formatted = format_line_ranges(&lines);

            // Should contain at least the first line number
            prop_assert!(formatted.contains(&lines[0].to_string()));

            // Should not contain invalid characters
            prop_assert!(!formatted.contains(".."));  // No empty ranges
        }
    }
}
