use super::context::{DebtSpecificInfo, FormatContext};
use super::dependencies::{filter_dependencies, format_function_reference};
use crate::formatting::{ColoredFormatter, FormattingConfig};
use crate::priority::score_formatter;
use colored::*;
use std::fmt::Write;

pub(crate) struct FormattedSections {
    pub header: String,
    pub location: String,
    pub context_dampening: Option<String>, // spec 191: show context-aware dampening
    pub action: String,
    pub impact: String,
    pub evidence: Option<String>, // New: combines complexity + metrics
    pub complexity: Option<String>,
    pub coverage: Option<String>,
    pub dependencies: Option<String>,
    pub debt_specific: Option<String>,
    pub rationale: String,
}

// Pure function to generate all formatted sections
pub(crate) fn generate_formatted_sections(context: &FormatContext) -> FormattedSections {
    FormattedSections {
        header: format_header_section(context),
        location: format_location_section(context),
        context_dampening: format_context_dampening_section(context), // spec 191
        action: format_action_section(context),
        impact: format_impact_section(context),
        evidence: format_evidence_section(context), // New
        complexity: format_complexity_section(context),
        coverage: format_coverage_section(context),
        dependencies: format_dependencies_section(context),
        debt_specific: format_debt_specific_section(context),
        rationale: format_rationale_section(context),
    }
}

// Pure function to format header section with visual separators
// Tag order: SCORE → COVERAGE → SEVERITY
fn format_header_section(context: &FormatContext) -> String {
    let separator = " • ".dimmed();

    // Build coverage tag if available
    let (coverage_tag, severity_separator) = if let Some(ref coverage_info) = context.coverage_info
    {
        (
            format!(
                "{}{}",
                separator,
                coverage_info.tag.color(coverage_info.color).bold()
            ),
            format!("{}", separator),
        )
    } else {
        (String::new(), " ".to_string())
    };

    format!(
        "#{} {}{}{}[{}]",
        context.rank,
        format!("SCORE: {}", score_formatter::format_score(context.score)).bright_yellow(),
        coverage_tag,
        severity_separator,
        context
            .severity_info
            .label
            .color(context.severity_info.color)
            .bold()
    )
}

// Pure function to format location section
fn format_location_section(context: &FormatContext) -> String {
    format!(
        "{} {}:{} {}()",
        "├─ LOCATION:".bright_blue(),
        context.location_info.file.display(),
        context.location_info.line,
        context.location_info.function.bright_green()
    )
}

// Pure function to format context dampening section (spec 191)
fn format_context_dampening_section(context: &FormatContext) -> Option<String> {
    let dampening_info = context.context_info.as_ref()?;

    let dampening_percentage = ((1.0 - dampening_info.multiplier) * 100.0) as i32;

    Some(format!(
        "{} {} ({}% dampening applied)",
        "├─ CONTEXT:".bright_blue(),
        dampening_info.description.bright_cyan(),
        dampening_percentage
    ))
}

// Pure function to format action section
fn format_action_section(context: &FormatContext) -> String {
    format!(
        "{} {}",
        "├─ RECOMMENDED ACTION:".bright_blue(),
        context.action.bright_green().bold()
    )
}

// Pure function to format impact section
fn format_impact_section(context: &FormatContext) -> String {
    format!(
        "{} {}",
        "├─ IMPACT:".bright_blue(),
        super::format_impact(&context.impact).bright_cyan()
    )
}

// Pure function to format complexity section (spec 183)
fn format_complexity_section(context: &FormatContext) -> Option<String> {
    if !context.complexity_info.has_complexity {
        return None;
    }

    if let Some(ref entropy) = context.complexity_info.entropy_details {
        // Show raw → adjusted for clarity (spec 183)
        Some(format!(
            "{} cyclomatic={} → {} (entropy-adjusted, factor: {:.2}), est_branches={}, cognitive={}, nesting={}, entropy={:.2}",
            "├─ COMPLEXITY:".bright_blue(),
            format!("{}", context.complexity_info.cyclomatic).yellow(),
            format!("{}", entropy.adjusted_complexity).bright_green().bold(),
            entropy.dampening_factor,
            format!("{}", context.complexity_info.branch_count).yellow(),
            format!("{}", context.complexity_info.cognitive).yellow(),
            format!("{}", context.complexity_info.nesting).yellow(),
            entropy.entropy_score
        ))
    } else {
        Some(format!(
            "{} cyclomatic={}, est_branches={}, cognitive={}, nesting={}",
            "├─ COMPLEXITY:".bright_blue(),
            format!("{}", context.complexity_info.cyclomatic).yellow(),
            format!("{}", context.complexity_info.branch_count).yellow(),
            format!("{}", context.complexity_info.cognitive).yellow(),
            format!("{}", context.complexity_info.nesting).yellow()
        ))
    }
}

// Pure function to format coverage section (spec 180)
// Shows coverage status when has_coverage_data=true
fn format_coverage_section(context: &FormatContext) -> Option<String> {
    // Only show coverage section if coverage data is being tracked
    let coverage_info = context.coverage_info.as_ref()?;

    // If we have actual transitive coverage data with a percentage, show it
    if let Some(coverage_pct) = coverage_info.coverage_percentage {
        Some(format!(
            "{} {:.1}%",
            "├─ COVERAGE:".bright_blue(),
            coverage_pct
        ))
    } else {
        // LCOV was provided but this function was not found in it
        Some(format!(
            "{} {}",
            "├─ COVERAGE:".bright_blue(),
            "no coverage data"
        ))
    }
}

// Pure function to format evidence section (metrics only, no rationale)
fn format_evidence_section(context: &FormatContext) -> Option<String> {
    if !context.complexity_info.has_complexity {
        return None;
    }

    let mut section = format!("{}", "├─ EVIDENCE:".bright_blue());

    // Show complexity metrics in priority order
    if context.complexity_info.cyclomatic > 0 {
        section.push_str(&format!(
            "\n│  {} Cyclomatic Complexity: {}",
            "├─",
            format!("{}", context.complexity_info.cyclomatic).yellow()
        ));
    }

    if context.complexity_info.cognitive > 0 {
        section.push_str(&format!(
            "\n│  {} Cognitive Complexity: {}",
            "├─",
            format!("{}", context.complexity_info.cognitive).yellow()
        ));
    }

    if context.complexity_info.branch_count > 0 {
        section.push_str(&format!(
            "\n│  {} Estimated Branches: {}",
            "├─",
            format!("{}", context.complexity_info.branch_count).yellow()
        ));
    }

    if context.complexity_info.nesting > 0 {
        section.push_str(&format!(
            "\n│  {} Nesting Depth: {}",
            "└─",
            format!("{}", context.complexity_info.nesting).yellow()
        ));
    }

    Some(section)
}

// Pure function to format dependencies section with enhanced caller/callee display
pub(crate) fn format_dependencies_section_with_config(
    context: &FormatContext,
    formatting_config: FormattingConfig,
) -> Option<String> {
    let config = &formatting_config.caller_callee;
    let _formatter = ColoredFormatter::new(formatting_config);

    // Filter callers and callees based on configuration
    let filtered_callers = filter_dependencies(&context.dependency_info.upstream_callers, config);
    let filtered_callees = filter_dependencies(&context.dependency_info.downstream_callees, config);

    // Always show dependencies section (per spec 117)
    let mut section = format!("{}", "├─ DEPENDENCIES:".bright_blue());

    // Display callers
    if !filtered_callers.is_empty() {
        let caller_count = filtered_callers.len();
        let display_count = caller_count.min(config.max_callers);

        section.push_str(&format!(
            "\n{}  {} {} ({}):",
            "|", "|-", "Called by", caller_count
        ));

        for caller in filtered_callers.iter().take(display_count) {
            let formatted_caller = format_function_reference(caller);
            section.push_str(&format!(
                "\n{}  {}     {} {}",
                "|",
                "|",
                "*",
                formatted_caller.bright_cyan()
            ));
        }

        if caller_count > display_count {
            section.push_str(&format!(
                "\n{}  {}     {} (showing {} of {})",
                "|", "|", "...", display_count, caller_count
            ));
        }
    } else {
        section.push_str(&format!(
            "\n{}  {} {} No direct callers detected",
            "|", "|-", "Called by"
        ));
    }

    // Display callees
    if !filtered_callees.is_empty() {
        let callee_count = filtered_callees.len();
        let display_count = callee_count.min(config.max_callees);

        section.push_str(&format!(
            "\n{}  {} {} ({}):",
            "|", "+-", "Calls", callee_count
        ));

        for callee in filtered_callees.iter().take(display_count) {
            let formatted_callee = format_function_reference(callee);
            section.push_str(&format!(
                "\n{}       {} {}",
                "|",
                "*",
                formatted_callee.bright_magenta()
            ));
        }

        if callee_count > display_count {
            section.push_str(&format!(
                "\n{}       {} (showing {} of {})",
                "|", "...", display_count, callee_count
            ));
        }
    } else {
        // Always show callees section, even when empty (per spec 117)
        section.push_str(&format!(
            "\n{}  {} {} Calls no other functions",
            "|", "+-", "Calls"
        ));
    }

    Some(section)
}

// Wrapper function that uses default formatting configuration
fn format_dependencies_section(context: &FormatContext) -> Option<String> {
    format_dependencies_section_with_config(context, FormattingConfig::default())
}

// Pure function to format debt-specific section
fn format_debt_specific_section(context: &FormatContext) -> Option<String> {
    match &context.debt_specific_info {
        DebtSpecificInfo::DeadCode {
            visibility,
            usage_hints,
        } => {
            let mut section = format!(
                "├─ VISIBILITY: {} function with no callers",
                visibility.yellow()
            );

            for hint in usage_hints {
                section.push_str(&format!("\n│  • {}", hint.bright_white()));
            }

            Some(section)
        }
        DebtSpecificInfo::Other => None,
    }
}

// Pure function to format rationale section
// This explains WHY the evidence matters (implications, not repeating metrics)
fn format_rationale_section(context: &FormatContext) -> String {
    let _formatter = ColoredFormatter::new(FormattingConfig::default());
    format!(
        "{} {}",
        "├─ WHY THIS MATTERS:".bright_blue(),
        context.rationale
    )
}

// I/O function to apply formatted sections to output
// Following spec 139: Header → Location → Context → Impact → Evidence → WHY → Action
pub(crate) fn apply_formatted_sections(output: &mut String, sections: FormattedSections) {
    writeln!(output, "{}", sections.header).unwrap();
    writeln!(output, "{}", sections.location).unwrap();

    // Context dampening section (spec 191) - show after location
    if let Some(context) = sections.context_dampening {
        writeln!(output, "{}", context).unwrap();
    }

    writeln!(output, "{}", sections.impact).unwrap();

    // Evidence section (new) - metrics only
    if let Some(evidence) = sections.evidence {
        writeln!(output, "{}", evidence).unwrap();
    }

    // Keep legacy complexity for backward compatibility
    if let Some(complexity) = sections.complexity {
        writeln!(output, "{}", complexity).unwrap();
    }

    // Coverage section (spec 180)
    if let Some(coverage) = sections.coverage {
        writeln!(output, "{}", coverage).unwrap();
    }

    // WHY section - rationale explaining why evidence matters
    writeln!(output, "{}", sections.rationale).unwrap();

    // Action comes after WHY (spec 139 ordering)
    writeln!(output, "{}", sections.action).unwrap();

    if let Some(dependencies) = sections.dependencies {
        writeln!(output, "{}", dependencies).unwrap();
    }

    if let Some(debt_specific) = sections.debt_specific {
        writeln!(output, "{}", debt_specific).unwrap();
    }
}
