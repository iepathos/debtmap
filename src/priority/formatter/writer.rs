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
        } => write_complexity_section(writer, *cyclomatic, *cognitive, *nesting, *entropy)?,

        FormattedSection::Pattern {
            pattern_type,
            icon,
            metrics,
            confidence,
        } => write_pattern_section(writer, pattern_type, icon, metrics, *confidence)?,

        FormattedSection::Coverage {
            percentage,
            level,
            details: _,
        } => write_coverage_section(writer, *percentage, level)?,

        FormattedSection::Dependencies {
            upstream,
            downstream,
            callers,
            callees,
        } => write_dependencies_section(writer, *upstream, *downstream, callers, callees)?,

        FormattedSection::DebtSpecific { text } => {
            writeln!(writer, "{} {}", "├─ DETAILS:".bright_blue(), text)?;
        }

        FormattedSection::ContextualRisk {
            base_risk,
            contextual_risk,
            multiplier,
            providers,
        } => write_contextual_risk_section(
            writer,
            *base_risk,
            *contextual_risk,
            *multiplier,
            providers,
        )?,

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

/// Writes the complexity section with cyclomatic, cognitive, nesting, and optional entropy.
fn write_complexity_section(
    writer: &mut dyn Write,
    cyclomatic: u32,
    cognitive: u32,
    nesting: u32,
    entropy: Option<f64>,
) -> io::Result<()> {
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
    Ok(())
}

/// Writes the pattern section with pattern type, icon, metrics, and confidence.
fn write_pattern_section(
    writer: &mut dyn Write,
    pattern_type: &str,
    icon: &str,
    metrics: &[(String, String)],
    confidence: f64,
) -> io::Result<()> {
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
    Ok(())
}

/// Writes the coverage section with percentage, level, and status tag.
fn write_coverage_section(
    writer: &mut dyn Write,
    percentage: f64,
    level: &CoverageLevel,
) -> io::Result<()> {
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
    Ok(())
}

/// Writes the dependencies section with upstream/downstream counts and caller/callee lists.
fn write_dependencies_section(
    writer: &mut dyn Write,
    upstream: usize,
    downstream: usize,
    callers: &[String],
    callees: &[String],
) -> io::Result<()> {
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

    Ok(())
}

/// Writes the contextual risk section with base risk, contextual risk, multiplier, and providers.
fn write_contextual_risk_section(
    writer: &mut dyn Write,
    base_risk: f64,
    contextual_risk: f64,
    multiplier: f64,
    providers: &[crate::priority::formatted_output::ContextProviderInfo],
) -> io::Result<()> {
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
    fn write_complexity_section_integration() {
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

    // Unit tests for extracted helper functions

    #[test]
    fn test_write_complexity_section_with_entropy() {
        let mut output = Vec::new();
        super::write_complexity_section(&mut output, 10, 15, 3, Some(2.5)).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("cyclomatic: 10"));
        assert!(output_str.contains("cognitive: 15"));
        assert!(output_str.contains("nesting: 3"));
        assert!(output_str.contains("entropy: 2.50"));
    }

    #[test]
    fn test_write_complexity_section_without_entropy() {
        let mut output = Vec::new();
        super::write_complexity_section(&mut output, 5, 8, 2, None).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("cyclomatic: 5"));
        assert!(output_str.contains("cognitive: 8"));
        assert!(output_str.contains("nesting: 2"));
        assert!(!output_str.contains("entropy"));
    }

    #[test]
    fn test_write_pattern_section_with_metrics() {
        let mut output = Vec::new();
        let metrics = vec![
            ("size".to_string(), "large".to_string()),
            ("methods".to_string(), "42".to_string()),
        ];
        super::write_pattern_section(&mut output, "God Object", "👿", &metrics, 0.85).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("God Object"));
        assert!(output_str.contains("👿"));
        assert!(output_str.contains("85%"));
        assert!(output_str.contains("size: large"));
        assert!(output_str.contains("methods: 42"));
    }

    #[test]
    fn test_write_pattern_section_without_metrics() {
        let mut output = Vec::new();
        let metrics = vec![];
        super::write_pattern_section(&mut output, "Simple Pattern", "📝", &metrics, 0.95).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("Simple Pattern"));
        assert!(output_str.contains("95%"));
        // Should not have the " - " separator that precedes metrics
        assert!(!output_str.contains(" - "));
    }

    #[test]
    fn test_write_coverage_section_untested() {
        let mut output = Vec::new();
        super::write_coverage_section(&mut output, 0.0, &CoverageLevel::Untested).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("COVERAGE"));
        assert!(output_str.contains("0.0%"));
    }

    #[test]
    fn test_write_coverage_section_good() {
        let mut output = Vec::new();
        super::write_coverage_section(&mut output, 87.5, &CoverageLevel::Good).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("COVERAGE"));
        assert!(output_str.contains("87.5%"));
    }

    #[test]
    fn test_write_dependencies_section_with_callers_and_callees() {
        let mut output = Vec::new();
        let callers = vec!["main".to_string(), "init".to_string()];
        let callees = vec!["helper1".to_string(), "helper2".to_string(), "helper3".to_string()];
        super::write_dependencies_section(&mut output, 5, 3, &callers, &callees).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("5 upstream"));
        assert!(output_str.contains("3 downstream"));
        assert!(output_str.contains("Called by:"));
        assert!(output_str.contains("main"));
        assert!(output_str.contains("init"));
        assert!(output_str.contains("Calls:"));
        assert!(output_str.contains("helper1"));
        assert!(output_str.contains("helper2"));
        assert!(output_str.contains("helper3"));
    }

    #[test]
    fn test_write_dependencies_section_with_many_callers() {
        let mut output = Vec::new();
        let callers = vec![
            "func1".to_string(),
            "func2".to_string(),
            "func3".to_string(),
            "func4".to_string(),
            "func5".to_string(),
        ];
        let callees = vec![];
        super::write_dependencies_section(&mut output, 2, 0, &callers, &callees).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("func1"));
        assert!(output_str.contains("func2"));
        assert!(output_str.contains("func3"));
        assert!(output_str.contains("(+2 more)"));
    }

    #[test]
    fn test_write_dependencies_section_empty() {
        let mut output = Vec::new();
        let callers = vec![];
        let callees = vec![];
        super::write_dependencies_section(&mut output, 0, 0, &callers, &callees).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("0 upstream"));
        assert!(output_str.contains("0 downstream"));
        assert!(!output_str.contains("Called by:"));
        assert!(!output_str.contains("Calls:"));
    }

    #[test]
    fn test_write_contextual_risk_section_with_providers() {
        use crate::priority::formatted_output::ContextProviderInfo;

        let mut output = Vec::new();
        let providers = vec![
            ContextProviderInfo {
                name: "High Complexity".to_string(),
                contribution: 0.5,
                weight: 1.0,
                impact: 2.5,
                details: Some("Too many branches".to_string()),
            },
            ContextProviderInfo {
                name: "Low Coverage".to_string(),
                contribution: 0.3,
                weight: 0.8,
                impact: 1.8,
                details: None,
            },
        ];
        super::write_contextual_risk_section(&mut output, 5.0, 8.5, 1.7, &providers).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("base: 5.0"));
        assert!(output_str.contains("contextual: 8.5"));
        assert!(output_str.contains("1.70x multiplier"));
        assert!(output_str.contains("High Complexity"));
        assert!(output_str.contains("+2.5 impact"));
        assert!(output_str.contains("Too many branches"));
        assert!(output_str.contains("Low Coverage"));
        assert!(output_str.contains("+1.8 impact"));
    }

    #[test]
    fn test_write_contextual_risk_section_filters_low_impact() {
        use crate::priority::formatted_output::ContextProviderInfo;

        let mut output = Vec::new();
        let providers = vec![
            ContextProviderInfo {
                name: "High Impact".to_string(),
                contribution: 0.5,
                weight: 1.0,
                impact: 2.0,
                details: None,
            },
            ContextProviderInfo {
                name: "Low Impact".to_string(),
                contribution: 0.01,
                weight: 0.1,
                impact: 0.05,
                details: None,
            },
        ];
        super::write_contextual_risk_section(&mut output, 3.0, 5.0, 1.5, &providers).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("High Impact"));
        assert!(!output_str.contains("Low Impact"));
    }
}
