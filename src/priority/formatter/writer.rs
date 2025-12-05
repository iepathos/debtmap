//! I/O layer for rendering formatted priority items to output.
//!
//! This module contains functions that perform I/O operations. All functions
//! take a `Write` trait object and formatted data structures.
//!
//! # Examples
//!
//! ```no_run
//! use debtmap::priority::formatter::writer::write_priority_item;
//! use debtmap::priority::formatted_output::FormattedPriorityItem;
//! use std::io::Write;
//!
//! # let formatted = todo!();
//! let mut output = Vec::new();
//! write_priority_item(&mut output, &formatted)?;
//! # Ok::<(), std::io::Error>(())
//! ```

use crate::priority::classification::CoverageLevel;
use crate::priority::formatted_output::{FormattedPriorityItem, FormattedSection};
use crate::priority::score_formatter;
use colored::*;
use std::io::{self, Write};

/// Renders a formatted priority item to a writer.
///
/// This function performs I/O operations and should be called at system boundaries.
/// All formatting logic has already been done in the pure layer.
///
/// # Arguments
///
/// * `writer` - Any type implementing `Write` (file, string buffer, stdout, etc.)
/// * `item` - The formatted item to render
///
/// # Returns
///
/// `io::Result<()>` - Success or I/O error
///
/// # Examples
///
/// ```no_run
/// use debtmap::priority::formatter::writer::write_priority_item;
/// use std::io::Write;
/// use std::fs::File;
///
/// # let formatted = todo!();
/// // Write to stdout
/// write_priority_item(&mut std::io::stdout(), &formatted)?;
///
/// // Write to string
/// let mut output = Vec::new();
/// write_priority_item(&mut output, &formatted)?;
/// let output_str = String::from_utf8(output)?;
///
/// // Write to file
/// let mut file = File::create("output.txt")?;
/// write_priority_item(&mut file, &formatted)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn write_priority_item(writer: &mut dyn Write, item: &FormattedPriorityItem) -> io::Result<()> {
    for section in &item.sections {
        write_section(writer, section)?;
    }
    writeln!(writer)?;
    Ok(())
}

/// Writes a single section to the writer.
fn write_section(writer: &mut dyn Write, section: &FormattedSection) -> io::Result<()> {
    match section {
        FormattedSection::Header {
            rank,
            score,
            coverage_tag,
            severity,
        } => write_header_section(writer, *rank, *score, coverage_tag.as_ref(), severity)?,

        FormattedSection::Location {
            file,
            line,
            function,
        } => {
            writeln!(
                writer,
                "{} {}:{} {}()",
                "├─ LOCATION:".bright_blue(),
                file.display(),
                line,
                function.bright_green()
            )?;
        }

        FormattedSection::ContextDampening {
            description,
            dampening_percentage,
        } => {
            writeln!(
                writer,
                "{} {} ({}% dampening applied)",
                "├─ CONTEXT:".bright_blue(),
                description.bright_cyan(),
                dampening_percentage
            )?;
        }

        FormattedSection::Action { action } => {
            writeln!(
                writer,
                "{} {}",
                "├─ ACTION:".bright_blue(),
                action.bright_yellow()
            )?;
        }

        FormattedSection::Impact {
            complexity_reduction,
            risk_reduction,
        } => {
            writeln!(
                writer,
                "{} {}",
                "├─ IMPACT:".bright_blue(),
                format!(
                    "-{} complexity, -{:.1} risk",
                    complexity_reduction, risk_reduction
                )
                .bright_cyan()
            )?;
        }

        FormattedSection::Evidence { text } => {
            writeln!(writer, "{} {}", "├─ EVIDENCE:".bright_blue(), text)?;
        }

        FormattedSection::Complexity {
            cyclomatic,
            cognitive,
            nesting,
            entropy,
        } => {
            write!(
                writer,
                "{} cyclomatic: {}, cognitive: {}, nesting: {}",
                "├─ COMPLEXITY:".bright_blue(),
                cyclomatic,
                cognitive,
                nesting
            )?;
            if let Some(entropy_val) = entropy {
                write!(writer, ", entropy: {:.2}", entropy_val)?;
            }
            writeln!(writer)?;
        }

        FormattedSection::Pattern {
            pattern_type,
            icon,
            metrics,
            confidence,
        } => {
            write!(
                writer,
                "{} {} {} (confidence: {:.0}%)",
                "├─ PATTERN:".bright_blue(),
                icon,
                pattern_type.bright_magenta(),
                confidence * 100.0
            )?;
            if !metrics.is_empty() {
                write!(writer, " - ")?;
                for (i, (key, value)) in metrics.iter().enumerate() {
                    if i > 0 {
                        write!(writer, ", ")?;
                    }
                    write!(writer, "{}: {}", key, value)?;
                }
            }
            writeln!(writer)?;
        }

        FormattedSection::Coverage {
            percentage,
            level,
            details: _,
        } => {
            let status_tag = level.status_tag();
            let color = match level {
                CoverageLevel::Untested => Color::BrightRed,
                CoverageLevel::Low | CoverageLevel::Partial => Color::Yellow,
                CoverageLevel::Moderate => Color::Cyan,
                CoverageLevel::Good => Color::Green,
                CoverageLevel::Excellent => Color::BrightGreen,
            };

            writeln!(
                writer,
                "{} {} ({:.1}%)",
                "├─ COVERAGE:".bright_blue(),
                status_tag.color(color).bold(),
                percentage
            )?;
        }

        FormattedSection::Dependencies {
            upstream,
            downstream,
            callers,
            callees,
        } => {
            writeln!(
                writer,
                "{} {} upstream, {} downstream",
                "├─ DEPENDENCIES:".bright_blue(),
                upstream,
                downstream
            )?;

            if !callers.is_empty() {
                let caller_list = callers
                    .iter()
                    .take(3)
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                let more_text = if callers.len() > 3 {
                    format!(" (+{} more)", callers.len() - 3)
                } else {
                    String::new()
                };
                writeln!(
                    writer,
                    "   {} {}{}",
                    "Called by:".dimmed(),
                    caller_list,
                    more_text
                )?;
            }

            if !callees.is_empty() {
                let callee_list = callees
                    .iter()
                    .take(3)
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                let more_text = if callees.len() > 3 {
                    format!(" (+{} more)", callees.len() - 3)
                } else {
                    String::new()
                };
                writeln!(
                    writer,
                    "   {} {}{}",
                    "Calls:".dimmed(),
                    callee_list,
                    more_text
                )?;
            }
        }

        FormattedSection::DebtSpecific { text } => {
            writeln!(writer, "{} {}", "├─ DETAILS:".bright_blue(), text)?;
        }

        FormattedSection::ContextualRisk {
            base_risk,
            contextual_risk,
            multiplier,
            providers,
        } => {
            writeln!(
                writer,
                "{} base: {:.1}, contextual: {:.1} ({:.2}x multiplier)",
                "├─ CONTEXT RISK:".bright_blue(),
                base_risk,
                contextual_risk,
                multiplier
            )?;

            for provider in providers {
                if provider.impact > 0.1 {
                    write!(
                        writer,
                        "   {} {} +{:.1} impact",
                        "└─".dimmed(),
                        provider.name.bright_cyan(),
                        provider.impact
                    )?;

                    if let Some(ref details) = provider.details {
                        writeln!(writer, " ({})", details.dimmed())?;
                    } else {
                        writeln!(writer)?;
                    }
                }
            }
        }

        FormattedSection::Rationale { text } => {
            writeln!(
                writer,
                "{} {}",
                "└─ RATIONALE:".bright_blue(),
                text.dimmed()
            )?;
        }
    }

    Ok(())
}

/// Writes the header section with rank, score, coverage, and severity.
fn write_header_section(
    writer: &mut dyn Write,
    rank: usize,
    score: f64,
    coverage_tag: Option<&crate::priority::formatted_output::CoverageTag>,
    severity: &crate::priority::formatted_output::SeverityInfo,
) -> io::Result<()> {
    let separator = " • ".dimmed();

    // Build coverage tag if available
    let coverage_display = if let Some(tag) = coverage_tag {
        format!("{}{}", separator, tag.text.color(tag.color).bold())
    } else {
        String::new()
    };

    let severity_separator = if coverage_tag.is_some() {
        format!("{}", separator)
    } else {
        " ".to_string()
    };

    writeln!(
        writer,
        "#{} {}{}{}[{}]",
        rank,
        format!("SCORE: {}", score_formatter::format_score(score)).bright_yellow(),
        coverage_display,
        severity_separator,
        severity.label.color(severity.color).bold()
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::classification::Severity;
    use crate::priority::formatted_output::{CoverageTag, FormattedPriorityItem, SeverityInfo};
    use colored::Color;
    use std::path::PathBuf;

    #[test]
    fn write_produces_output() {
        let formatted = FormattedPriorityItem {
            rank: 1,
            score: 8.5,
            severity: Severity::Critical,
            sections: vec![
                FormattedSection::Header {
                    rank: 1,
                    score: 8.5,
                    coverage_tag: None,
                    severity: SeverityInfo {
                        label: "CRITICAL".to_string(),
                        color: Color::Red,
                    },
                },
                FormattedSection::Location {
                    file: PathBuf::from("test.rs"),
                    line: 10,
                    function: "test_fn".to_string(),
                },
                FormattedSection::Action {
                    action: "Refactor this function".to_string(),
                },
            ],
        };

        let mut output = Vec::new();
        write_priority_item(&mut output, &formatted).unwrap();

        let output_str = String::from_utf8(output).unwrap();

        assert!(output_str.contains("#1"));
        assert!(output_str.contains("8.5"));
        assert!(output_str.contains("test.rs:10"));
        assert!(output_str.contains("test_fn"));
        assert!(output_str.contains("Refactor this function"));
    }

    #[test]
    fn write_with_coverage_tag() {
        let formatted = FormattedPriorityItem {
            rank: 2,
            score: 7.0,
            severity: Severity::High,
            sections: vec![FormattedSection::Header {
                rank: 2,
                score: 7.0,
                coverage_tag: Some(CoverageTag {
                    text: "[ERROR UNTESTED]".to_string(),
                    color: Color::BrightRed,
                }),
                severity: SeverityInfo {
                    label: "HIGH".to_string(),
                    color: Color::Yellow,
                },
            }],
        };

        let mut output = Vec::new();
        write_priority_item(&mut output, &formatted).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        // Note: The ANSI color codes make exact matching tricky, but we can check for presence
        assert!(output_str.contains("#2"));
        assert!(output_str.contains("7.0"));
    }

    #[test]
    fn write_complexity_section() {
        let formatted = FormattedPriorityItem {
            rank: 1,
            score: 5.0,
            severity: Severity::Medium,
            sections: vec![FormattedSection::Complexity {
                cyclomatic: 10,
                cognitive: 15,
                nesting: 3,
                entropy: Some(2.5),
            }],
        };

        let mut output = Vec::new();
        write_priority_item(&mut output, &formatted).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("cyclomatic: 10"));
        assert!(output_str.contains("cognitive: 15"));
        assert!(output_str.contains("nesting: 3"));
        assert!(output_str.contains("entropy: 2.50"));
    }
}
