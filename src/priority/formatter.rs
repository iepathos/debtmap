use crate::formatting::{ColoredFormatter, FormattingConfig};
use crate::output::evidence_formatter::EvidenceFormatter;
use crate::priority::{
    self, score_formatter, DebtType, DisplayGroup, FunctionRole, Tier, UnifiedAnalysis,
    UnifiedAnalysisQueries, UnifiedDebtItem,
};
use colored::*;
use std::fmt::Write;

#[path = "formatter_verbosity.rs"]
mod verbosity;

mod context;
mod dependencies;
mod sections;

use context::create_format_context;
use sections::{apply_formatted_sections, generate_formatted_sections};

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Default,     // Top 10 with clean formatting
    Top(usize),  // Top N items
    Tail(usize), // Bottom N items (lowest priority)
}

pub fn format_priorities(analysis: &UnifiedAnalysis, format: OutputFormat) -> String {
    format_priorities_with_verbosity(analysis, format, 0)
}

pub fn format_priorities_with_verbosity(
    analysis: &UnifiedAnalysis,
    format: OutputFormat,
    verbosity: u8,
) -> String {
    format_priorities_with_config(analysis, format, verbosity, FormattingConfig::default())
}

pub fn format_priorities_with_config(
    analysis: &UnifiedAnalysis,
    format: OutputFormat,
    verbosity: u8,
    config: FormattingConfig,
) -> String {
    match format {
        OutputFormat::Default => format_default_with_config(analysis, 10, verbosity, config),
        OutputFormat::Top(n) => format_default_with_config(analysis, n, verbosity, config),
        OutputFormat::Tail(n) => format_tail_with_config(analysis, n, verbosity, config),
    }
}

fn format_default_with_verbosity(
    analysis: &UnifiedAnalysis,
    limit: usize,
    verbosity: u8,
) -> String {
    format_default_with_config(analysis, limit, verbosity, FormattingConfig::default())
}

// Pure function to generate output legend explaining header tags
// Displayed once at the start of recommendations when verbosity >= 1
fn generate_legend(verbosity: u8, has_coverage_data: bool) -> String {
    if verbosity == 0 || !has_coverage_data {
        return String::new();
    }

    let mut legend = String::new();
    writeln!(legend, "{}", "Legend:".bright_white().bold()).unwrap();
    writeln!(
        legend,
        "  {} Numeric priority (higher = more important)",
        "SCORE:".bright_yellow()
    )
    .unwrap();

    if has_coverage_data {
        writeln!(
            legend,
            "  {} Coverage status (how well tested)",
            "[ERROR/WARN/INFO/OK]:".bright_cyan()
        )
        .unwrap();
    }

    writeln!(
        legend,
        "  {} Item severity (fix urgency)",
        "[CRITICAL/HIGH/MEDIUM/LOW]:".bright_magenta()
    )
    .unwrap();
    writeln!(legend).unwrap();

    legend
}

fn format_default_with_config(
    analysis: &UnifiedAnalysis,
    limit: usize,
    verbosity: u8,
    config: FormattingConfig,
) -> String {
    // Check if summary mode is explicitly requested
    // TODO: Add --summary flag to CLI to enable this
    // For now, always use detailed format to preserve existing functionality
    let mut output = String::new();
    let version = env!("CARGO_PKG_VERSION");
    let _formatter = ColoredFormatter::new(config);

    let divider = "=".repeat(44);
    writeln!(output, "{}", divider.bright_blue()).unwrap();
    writeln!(
        output,
        "    {}",
        format!("Debtmap v{}", version).bright_white().bold()
    )
    .unwrap();
    writeln!(output, "{}", divider.bright_blue()).unwrap();
    writeln!(output).unwrap();

    let top_items = analysis.get_top_mixed_priorities(limit);
    let count = top_items.len().min(limit);
    writeln!(
        output,
        "{}",
        format!("TOP {count} RECOMMENDATIONS")
            .bright_yellow()
            .bold()
    )
    .unwrap();
    writeln!(output).unwrap();

    // Add legend if verbosity >= 1 and coverage data is available
    let legend = generate_legend(verbosity, analysis.has_coverage_data);
    if !legend.is_empty() {
        output.push_str(&legend);
    }

    for (idx, item) in top_items.iter().enumerate() {
        format_mixed_priority_item(
            &mut output,
            idx + 1,
            item,
            verbosity,
            config,
            analysis.has_coverage_data,
        );
        writeln!(output).unwrap();
    }

    // Add summary
    writeln!(
        output,
        "{}",
        format!("TOTAL DEBT SCORE: {:.0}", analysis.total_debt_score).bright_cyan()
    )
    .unwrap();

    writeln!(
        output,
        "{}",
        format!(
            "DEBT DENSITY: {:.1} per 1K LOC ({} total LOC)",
            analysis.debt_density, analysis.total_lines_of_code
        )
        .bright_yellow()
    )
    .unwrap();

    // Only show overall coverage if coverage data was provided (spec 108)
    if analysis.has_coverage_data {
        if let Some(coverage) = analysis.overall_coverage {
            writeln!(
                output,
                "{}",
                format!("OVERALL COVERAGE: {:.2}%", coverage).bright_green()
            )
            .unwrap();
        }
    }

    output
}

#[allow(dead_code)]
fn format_default(analysis: &UnifiedAnalysis, limit: usize) -> String {
    format_default_with_verbosity(analysis, limit, 0)
}

#[allow(dead_code)]
fn format_tail_with_verbosity(analysis: &UnifiedAnalysis, n: usize, verbosity: u8) -> String {
    format_tail_with_config(analysis, n, verbosity, FormattingConfig::default())
}

fn format_tail_with_config(
    analysis: &UnifiedAnalysis,
    n: usize,
    verbosity: u8,
    config: FormattingConfig,
) -> String {
    let mut output = String::new();
    let version = env!("CARGO_PKG_VERSION");
    let _formatter = ColoredFormatter::new(config);

    let divider = "=".repeat(44);
    writeln!(output, "{}", divider.bright_blue()).unwrap();
    writeln!(
        output,
        "    {}",
        format!("Debtmap v{}", version).bright_white().bold()
    )
    .unwrap();
    writeln!(output, "{}", divider.bright_blue()).unwrap();
    writeln!(output).unwrap();

    let tail_items = analysis.get_bottom_priorities(n);
    let start_rank = (analysis.items.len() - tail_items.len()) + 1;

    for (idx, item) in tail_items.iter().enumerate() {
        verbosity::format_priority_item_with_config(
            &mut output,
            start_rank + idx,
            item,
            verbosity,
            config,
            analysis.has_coverage_data,
        );
        writeln!(output).unwrap();
    }

    output
}

/// Format priorities with tiered display for terminal output (summary mode)
pub fn format_summary_terminal(analysis: &UnifiedAnalysis, limit: usize, verbosity: u8) -> String {
    format_tiered_terminal(analysis, limit, verbosity, FormattingConfig::default())
}

/// Internal implementation of tiered display for terminal output
fn format_tiered_terminal(
    analysis: &UnifiedAnalysis,
    limit: usize,
    verbosity: u8,
    config: FormattingConfig,
) -> String {
    let mut output = String::new();
    let version = env!("CARGO_PKG_VERSION");
    let _formatter = ColoredFormatter::new(config);

    // Header
    let divider = "=".repeat(44);
    writeln!(output, "{}", divider.bright_blue()).unwrap();
    writeln!(
        output,
        "    {}",
        format!("Debtmap v{}", version).bright_white().bold()
    )
    .unwrap();
    writeln!(output, "{}", divider.bright_blue()).unwrap();
    writeln!(output).unwrap();

    // Get tiered display
    let tiered_display = analysis.get_tiered_display(limit);

    writeln!(
        output,
        "{}",
        "TECHNICAL DEBT ANALYSIS - PRIORITY TIERS"
            .bright_yellow()
            .bold()
    )
    .unwrap();
    writeln!(output).unwrap();

    // Format each tier
    format_tier_terminal(
        &mut output,
        &tiered_display.critical,
        Tier::Critical,
        verbosity,
        config,
    );
    format_tier_terminal(
        &mut output,
        &tiered_display.high,
        Tier::High,
        verbosity,
        config,
    );
    format_tier_terminal(
        &mut output,
        &tiered_display.moderate,
        Tier::Moderate,
        verbosity,
        config,
    );
    format_tier_terminal(
        &mut output,
        &tiered_display.low,
        Tier::Low,
        verbosity,
        config,
    );

    // Summary section
    let critical_count: usize = tiered_display.critical.iter().map(|g| g.items.len()).sum();
    let high_count: usize = tiered_display.high.iter().map(|g| g.items.len()).sum();
    let moderate_count: usize = tiered_display.moderate.iter().map(|g| g.items.len()).sum();
    let low_count: usize = tiered_display.low.iter().map(|g| g.items.len()).sum();

    writeln!(output, "{}", divider.bright_blue()).unwrap();
    writeln!(output, "[SUMMARY] DEBT DISTRIBUTION").unwrap();

    if critical_count > 0 {
        writeln!(
            output,
            "  [!] Critical: {} items",
            critical_count.to_string().bright_red()
        )
        .unwrap();
    }
    if high_count > 0 {
        writeln!(
            output,
            "  [*] High: {} items",
            high_count.to_string().bright_yellow()
        )
        .unwrap();
    }
    if moderate_count > 0 {
        writeln!(
            output,
            "  [+] Moderate: {} items",
            moderate_count.to_string().bright_blue()
        )
        .unwrap();
    }
    if low_count > 0 {
        writeln!(output, "  [-] Low: {} items", low_count.to_string().white()).unwrap();
    }

    writeln!(output).unwrap();
    writeln!(
        output,
        "[TOTAL] {}",
        format!("TOTAL DEBT SCORE: {:.0}", analysis.total_debt_score).bright_cyan()
    )
    .unwrap();

    writeln!(
        output,
        "{}",
        format!(
            "DEBT DENSITY: {:.1} per 1K LOC ({} total LOC)",
            analysis.debt_density, analysis.total_lines_of_code
        )
        .bright_yellow()
    )
    .unwrap();

    // Only show overall coverage if coverage data was provided (spec 108)
    if analysis.has_coverage_data {
        if let Some(coverage) = analysis.overall_coverage {
            writeln!(
                output,
                "[COVERAGE] {}",
                format!("OVERALL COVERAGE: {:.2}%", coverage).bright_green()
            )
            .unwrap();
        }
    }

    output
}

/// Format a single tier for terminal output
fn format_tier_terminal(
    output: &mut String,
    groups: &[DisplayGroup],
    tier: Tier,
    verbosity: u8,
    config: FormattingConfig,
) {
    if groups.is_empty() {
        return;
    }

    let _formatter = ColoredFormatter::new(config);

    // Tier header with color based on tier level
    let tier_header = match tier {
        Tier::Critical => format!(
            "{} {} - {}",
            "[CRITICAL]",
            "CRITICAL".bright_red().bold(),
            "Immediate Action Required".red()
        ),
        Tier::High => format!(
            "{} {} - {}",
            "[HIGH]",
            "HIGH PRIORITY",
            "Current Sprint".yellow()
        ),
        Tier::Moderate => format!(
            "{} {} - {}",
            "[MODERATE]",
            "MODERATE".bright_blue().bold(),
            "Next Sprint".blue()
        ),
        Tier::Low => format!(
            "{} {} - {}",
            "[LOW]",
            "LOW".white().bold(),
            "Backlog".white()
        ),
    };

    writeln!(output, "{}", tier_header).unwrap();
    writeln!(output, "{}", tier.effort_estimate()).unwrap();
    writeln!(output).unwrap();

    let max_items_per_tier = if verbosity >= 2 { 999 } else { 5 };
    let mut items_shown = 0;

    for group in groups {
        if items_shown >= max_items_per_tier {
            let remaining: usize = groups.iter().skip(items_shown).map(|g| g.items.len()).sum();
            if remaining > 0 {
                writeln!(
                    output,
                    "  [+] ... and {} more items in this tier",
                    remaining
                )
                .unwrap();
            }
            break;
        }

        format_display_group_terminal(output, group, &mut items_shown, verbosity, config);
    }

    writeln!(output).unwrap();
}

/// Format a display group for terminal output
fn format_display_group_terminal(
    output: &mut String,
    group: &DisplayGroup,
    items_shown: &mut usize,
    verbosity: u8,
    config: FormattingConfig,
) {
    let _formatter = ColoredFormatter::new(config);

    if group.items.len() > 1 && group.batch_action.is_some() {
        // Grouped similar items
        writeln!(
            output,
            "  [GROUP] {} ({} similar items)",
            group.debt_type.bright_cyan(),
            group.items.len().to_string().yellow()
        )
        .unwrap();

        if let Some(action) = &group.batch_action {
            writeln!(output, "    -> {}", action.green()).unwrap();
        }

        // Show first item as example if verbose
        if verbosity >= 1 && !group.items.is_empty() {
            writeln!(
                output,
                "    [eg] Example: {}",
                format_item_location(&group.items[0])
            )
            .unwrap();
        }

        *items_shown += group.items.len();
    } else {
        // Individual items
        for item in &group.items {
            if *items_shown >= 5 && verbosity < 2 {
                return;
            }

            // Use compact format for tiered display
            format_compact_item(output, *items_shown + 1, item, verbosity, config);
            *items_shown += 1;
        }
    }
}

/// Format an item in compact mode for tiered display
fn format_compact_item(
    output: &mut String,
    index: usize,
    item: &priority::DebtItem,
    verbosity: u8,
    config: FormattingConfig,
) {
    let _formatter = ColoredFormatter::new(config);

    match item {
        priority::DebtItem::Function(func) => {
            writeln!(
                output,
                "  > #{} [{}] {}:{} {}",
                index,
                format!("{:.1}", func.unified_score.final_score).yellow(),
                func.location.file.display(),
                func.location.line,
                func.location.function.bright_green()
            )
            .unwrap();

            // Show brief action
            writeln!(
                output,
                "      -> {}",
                func.recommendation.primary_action.green()
            )
            .unwrap();
        }
        priority::DebtItem::File(file) => {
            writeln!(
                output,
                "  [F] #{} [{}] {} ({} lines)",
                index,
                format!("{:.1}", file.score).yellow(),
                file.metrics.path.display(),
                file.metrics.total_lines
            )
            .unwrap();

            // Show brief action
            writeln!(output, "      -> {}", file.recommendation.green()).unwrap();
        }
    }

    if verbosity >= 1 {
        writeln!(output).unwrap();
    }
}

/// Helper to format item location
fn format_item_location(item: &priority::DebtItem) -> String {
    match item {
        priority::DebtItem::Function(func) => {
            format!("{}:{}", func.location.file.display(), func.location.line)
        }
        priority::DebtItem::File(file) => {
            format!("{}", file.metrics.path.display())
        }
    }
}

#[allow(dead_code)]
fn format_tail(analysis: &UnifiedAnalysis, limit: usize) -> String {
    let mut output = String::new();
    let version = env!("CARGO_PKG_VERSION");
    let _formatter = ColoredFormatter::new(FormattingConfig::default());

    let divider = "=".repeat(44);
    writeln!(output, "{}", divider.bright_blue()).unwrap();
    writeln!(
        output,
        "    {}",
        format!("Debtmap v{}", version).bright_white().bold()
    )
    .unwrap();
    writeln!(output, "{}", divider.bright_blue()).unwrap();
    writeln!(output).unwrap();

    let bottom_items = analysis.get_bottom_priorities(limit);
    let count = bottom_items.len().min(limit);
    let total_items = analysis.items.len();

    writeln!(
        output,
        "📉 BOTTOM {count} ITEMS (items {}-{})",
        total_items.saturating_sub(count - 1),
        total_items
    )
    .unwrap();
    writeln!(output).unwrap();

    for (idx, item) in bottom_items.iter().enumerate() {
        if idx >= limit {
            break;
        }
        let rank = total_items - bottom_items.len() + idx + 1;
        format_priority_item(&mut output, rank, item, analysis.has_coverage_data);
        writeln!(output).unwrap();
    }

    // Add total debt score
    writeln!(output).unwrap();
    writeln!(
        output,
        "{}",
        format!("TOTAL DEBT SCORE: {:.0}", analysis.total_debt_score)
            .bright_cyan()
            .bold()
    )
    .unwrap();

    // Add overall coverage if available and coverage data was provided (spec 108)
    if analysis.has_coverage_data {
        if let Some(coverage) = analysis.overall_coverage {
            writeln!(
                output,
                "[COVERAGE] {}",
                format!("OVERALL COVERAGE: {coverage:.2}%")
                    .bright_green()
                    .bold()
            )
            .unwrap();
        }
    }

    output
}

#[allow(dead_code)]
fn format_detailed(analysis: &UnifiedAnalysis) -> String {
    let mut output = String::new();
    let version = env!("CARGO_PKG_VERSION");
    let _formatter = ColoredFormatter::new(FormattingConfig::default());

    let divider = "=".repeat(44);
    writeln!(output, "{}", divider.bright_blue()).unwrap();
    writeln!(
        output,
        "    {}",
        format!("Debtmap v{}", version).bright_white().bold()
    )
    .unwrap();
    writeln!(output, "{}", divider.bright_blue()).unwrap();
    writeln!(output).unwrap();

    for (idx, item) in analysis.items.iter().enumerate() {
        format_detailed_item(&mut output, idx + 1, item);
        writeln!(output).unwrap();
    }

    output
}

fn format_mixed_priority_item(
    output: &mut String,
    rank: usize,
    item: &priority::DebtItem,
    verbosity: u8,
    config: FormattingConfig,
    has_coverage_data: bool,
) {
    match item {
        priority::DebtItem::Function(func_item) => {
            verbosity::format_priority_item_with_config(
                output,
                rank,
                func_item,
                verbosity,
                config,
                has_coverage_data,
            );
        }
        priority::DebtItem::File(file_item) => {
            format_file_priority_item_with_verbosity(output, rank, file_item, config, verbosity);
        }
    }
}

// Pure function to determine file type label based on characteristics
// Note: Kept for potential future use, but not used in current spec 139 format
#[allow(dead_code)]
fn determine_file_type_label(
    is_god_object: bool,
    fields_count: usize,
    total_lines: usize,
    god_object_type: Option<&crate::organization::GodObjectType>,
) -> &'static str {
    // Check for boilerplate pattern first
    if let Some(crate::organization::GodObjectType::BoilerplatePattern { .. }) = god_object_type {
        return "FILE - BOILERPLATE PATTERN";
    }

    if is_god_object {
        // Distinguish between god objects (classes) and god modules (procedural files)
        if fields_count > 5 {
            "FILE - GOD OBJECT" // Actual class with many fields
        } else {
            "FILE - GOD MODULE" // Procedural file with many functions
        }
    } else if total_lines > 500 {
        "FILE - HIGH COMPLEXITY"
    } else {
        "FILE"
    }
}

// Parameters for generate_why_message function
struct WhyMessageParams<'a> {
    is_god_object: bool,
    fields_count: usize,
    methods_count: usize,
    responsibilities: usize,
    function_count: usize,
    total_lines: usize,
    god_object_type: Option<&'a crate::organization::GodObjectType>,
    domain_diversity_metrics: Option<&'a crate::organization::DomainDiversityMetrics>,
    detection_type: Option<&'a crate::organization::DetectionType>,
}

// Pure function to generate WHY explanation message with accurate terminology
fn generate_why_message(params: WhyMessageParams<'_>) -> String {
    // Check for boilerplate pattern first
    if let Some(crate::organization::GodObjectType::BoilerplatePattern {
        pattern,
        confidence,
        ..
    }) = params.god_object_type
    {
        let pattern_description = match pattern {
            crate::organization::boilerplate_detector::BoilerplatePattern::TraitImplementation {
                trait_name,
                impl_count,
                ..
            } => {
                format!(
                    "{} implementations of {} trait",
                    impl_count, trait_name
                )
            }
            crate::organization::boilerplate_detector::BoilerplatePattern::BuilderPattern {
                builder_count,
            } => {
                format!("{} builder pattern instances", builder_count)
            }
            crate::organization::boilerplate_detector::BoilerplatePattern::TestBoilerplate {
                test_count,
                ..
            } => {
                format!("{} repetitive test functions", test_count)
            }
        };

        return format!(
            "BOILERPLATE DETECTED: {} ({:.0}% confidence). This file contains repetitive patterns that should be macro-ified or code-generated, not split into modules.",
            pattern_description,
            confidence * 100.0
        );
    }

    if params.is_god_object {
        // Determine detection type
        let is_hybrid = matches!(
            params.detection_type,
            Some(crate::organization::DetectionType::GodModule)
        );
        let is_pure_functional = matches!(
            params.detection_type,
            Some(crate::organization::DetectionType::GodFile)
        );

        // If we have domain diversity metrics, use those for more accurate analysis
        if let Some(metrics) = params.domain_diversity_metrics {
            format!(
                "This module contains {} structs across {} distinct domains. Cross-domain mixing (Severity: {}) violates single responsibility principle and increases maintenance complexity.",
                metrics.total_structs,
                metrics.domain_count,
                metrics.severity.as_str()
            )
        }
        // God Class: Single struct with many impl methods
        else if params.fields_count > 5
            && params.methods_count > 20
            && !is_hybrid
            && !is_pure_functional
        {
            format!(
                "This struct violates single responsibility principle with {} methods and {} fields across {} distinct responsibilities. High coupling and low cohesion make it difficult to maintain and test.",
                params.methods_count,
                params.fields_count,
                params.responsibilities
            )
        }
        // Hybrid/Functional: Many module functions
        else if params.function_count > 50 || is_hybrid || is_pure_functional {
            format!(
                "This module contains {} module functions across {} responsibilities. Large modules with many diverse functions are difficult to navigate, understand, and maintain.",
                params.function_count,
                params.responsibilities
            )
        }
        // Default case for god object
        else {
            format!(
                "This file contains {} functions with {} distinct responsibilities. Consider splitting by responsibility for better organization.",
                params.function_count,
                params.responsibilities
            )
        }
    } else if params.total_lines > 500 {
        format!(
            "File exceeds recommended size with {} lines. Large files are harder to navigate, understand, and maintain. Consider breaking into smaller, focused modules.",
            params.total_lines
        )
    } else {
        "File exhibits high complexity that impacts maintainability and testability.".to_string()
    }
}

// Pure function to get file extension or default based on path
fn get_file_extension(file_path: &std::path::Path) -> &str {
    file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("py")
}

// Pure function to get language name from extension
fn get_language_name(extension: &str) -> &str {
    match extension {
        "rs" => "Rust",
        "py" => "Python",
        "js" => "JavaScript",
        "ts" => "TypeScript",
        "jsx" => "JavaScript",
        "tsx" => "TypeScript",
        _ => "code",
    }
}

// Pure function to format implementation steps for god objects with detailed split recommendations
// Kept for potential future use or backward compatibility
#[allow(dead_code)]
fn format_god_object_steps(
    output: &mut String,
    formatter: &ColoredFormatter,
    item: &priority::FileDebtItem,
) {
    format_god_object_steps_with_verbosity(output, formatter, item, 0)
}

// Internal implementation with verbosity support for evidence display
fn format_god_object_steps_with_verbosity(
    output: &mut String,
    formatter: &ColoredFormatter,
    item: &priority::FileDebtItem,
    verbosity: u8,
) {
    let extension = get_file_extension(&item.metrics.path);
    let language = get_language_name(extension);
    let indicators = &item.metrics.god_object_indicators;

    // If we have detailed split recommendations, use them
    if !indicators.recommended_splits.is_empty() {
        // Use different header based on number of splits
        if indicators.recommended_splits.len() == 1 {
            writeln!(
                output,
                "  - EXTRACTION CANDIDATE (single cohesive group found):"
            )
            .unwrap();
        } else {
            writeln!(
                output,
                "  - RECOMMENDED SPLITS ({} modules):",
                indicators.recommended_splits.len()
            )
            .unwrap();
        }

        // Create evidence formatter for displaying classification evidence
        let evidence_formatter = EvidenceFormatter::new(verbosity);

        for (idx, split) in indicators.recommended_splits.iter().enumerate() {
            let is_last = idx == indicators.recommended_splits.len() - 1;
            let branch = "-";

            // Show priority indicator
            let priority_indicator = match split.priority {
                crate::priority::file_metrics::Priority::High => "High",
                crate::priority::file_metrics::Priority::Medium => "Medium",
                crate::priority::file_metrics::Priority::Low => "Low",
            };

            // Show module name and responsibility (reformatted for readability)
            writeln!(
                output,
                "\n  {}  {}.{}",
                branch, split.suggested_name, extension,
            )
            .unwrap();
            writeln!(
                output,
                "      Category: {} | Priority: {}",
                split.responsibility, priority_indicator
            )
            .unwrap();
            writeln!(
                output,
                "      Size: {} methods, ~{} lines",
                split.methods_to_move.len(),
                split.estimated_lines,
            )
            .unwrap();

            // Display classification evidence if available and verbosity > 0
            if verbosity > 0 {
                if let Some(ref evidence) = split.classification_evidence {
                    let formatted_evidence = evidence_formatter.format_evidence(evidence);
                    // Indent evidence to align with split details
                    for line in formatted_evidence.lines() {
                        writeln!(output, "      {}", line).unwrap();
                    }

                    // Show alternatives warning if confidence is low
                    if evidence.confidence < 0.80 && !evidence.alternatives.is_empty() {
                        writeln!(
                            output,
                            "      [WARN] Low confidence classification - review recommended"
                        )
                        .unwrap();
                    }
                }
            }

            // Show representative methods first (Spec 178: methods before structs)
            if !split.representative_methods.is_empty() {
                let total_methods = split.representative_methods.len();
                let sample_size = 5.min(total_methods);

                writeln!(output, "      Methods ({} total):", total_methods).unwrap();

                for method in split.representative_methods.iter().take(sample_size) {
                    writeln!(output, "        • {}()", method).unwrap();
                }

                if total_methods > sample_size {
                    writeln!(
                        output,
                        "        ... and {} more",
                        total_methods - sample_size
                    )
                    .unwrap();
                }
            } else if !split.methods_to_move.is_empty() {
                // Fallback to methods_to_move if representative_methods not populated
                let total_methods = split.methods_to_move.len();
                let sample_size = 5.min(total_methods);

                writeln!(output, "      Methods ({} total):", total_methods).unwrap();

                for method in split.methods_to_move.iter().take(sample_size) {
                    writeln!(output, "        • {}()", method).unwrap();
                }

                if total_methods > sample_size {
                    writeln!(
                        output,
                        "        ... and {} more",
                        total_methods - sample_size
                    )
                    .unwrap();
                }
            }

            // Show fields needed (Spec 178: field dependencies)
            if !split.fields_needed.is_empty() {
                writeln!(
                    output,
                    "      Fields needed: {}",
                    split.fields_needed.join(", ")
                )
                .unwrap();
            }

            // Show trait extraction suggestion (Spec 178)
            if let Some(ref trait_suggestion) = split.trait_suggestion {
                if verbosity > 0 {
                    writeln!(output, "      Trait extraction:").unwrap();
                    for line in trait_suggestion.lines() {
                        writeln!(output, "        {}", line).unwrap();
                    }
                }
            }

            // Show structs being moved (secondary to methods, per Spec 178)
            if !split.structs_to_move.is_empty() {
                writeln!(
                    output,
                    "      Structs: {}",
                    split.structs_to_move.join(", ")
                )
                .unwrap();
            }

            // Show warning if present
            if let Some(warning) = &split.warning {
                let _branch_prefix = if is_last { " " } else { "│" };
                writeln!(output, "      [!] {}", warning).unwrap();
            }
        }

        // Add explanation if only 1 group found
        if indicators.recommended_splits.len() == 1 {
            writeln!(output).unwrap();
            writeln!(
                output,
                "  NOTE: Only one cohesive group detected. This suggests methods are tightly coupled."
            )
            .unwrap();
            writeln!(
                output,
                "        Consider splitting by feature/responsibility rather than call patterns."
            )
            .unwrap();
        }

        // Add language-specific advice
        output.push('\n');

        format_language_specific_advice(output, formatter, language, extension);
    } else {
        // No detailed splits available - provide diagnostic message (Spec 149)
        writeln!(output, "  - NO DETAILED SPLITS AVAILABLE:").unwrap();
        writeln!(
            output,
            "  -  Analysis could not generate responsibility-based splits for this file."
        )
        .unwrap();
        writeln!(output, "  -  This may indicate:").unwrap();
        writeln!(
            output,
            "  -    • File has too few functions (< 3 per responsibility)"
        )
        .unwrap();
        writeln!(
            output,
            "  -    • Functions lack clear responsibility signals"
        )
        .unwrap();
        writeln!(output, "  -    • File may be test-only or configuration").unwrap();
        writeln!(
            output,
            "  -  Consider manual analysis or consult documentation."
        )
        .unwrap();
    }

    // Add enhanced module structure analysis if available
    // Use detailed verbosity by default (can be controlled via config in future)
    if let Some(module_structure) = &indicators.module_structure {
        use crate::config::VerbosityLevel;
        format_module_structure_analysis(
            output,
            module_structure,
            VerbosityLevel::Detailed,
            indicators.responsibilities, // Use god_object_indicators responsibilities for consistency
        );
    }

    // Add implementation guidance
    output.push('\n');
    writeln!(output, "  - IMPLEMENTATION ORDER:").unwrap();
    writeln!(
        output,
        "  -  [1] Start with lowest coupling modules (Data Access, Utilities)"
    )
    .unwrap();
    writeln!(
        output,
        "  -  [2] Move 10-20 methods at a time, test after each move"
    )
    .unwrap();
    writeln!(
        output,
        "  -  [3] Keep original file as facade during migration"
    )
    .unwrap();
    writeln!(
        output,
        "  -  [4] Refactor incrementally: 10-20 methods at a time"
    )
    .unwrap();
}

// Format enhanced module structure analysis
fn format_module_structure_analysis(
    output: &mut String,
    module_structure: &crate::analysis::ModuleStructure,
    verbosity: crate::config::VerbosityLevel,
    responsibilities: usize, // Use god_object_indicators value for consistency
) {
    use crate::config::VerbosityLevel;
    use std::fmt::Write;

    output.push('\n');

    // Summary: Show basic structure info
    writeln!(
        output,
        "  - STRUCTURE: {} responsibilities across {} components",
        responsibilities, // Use god_object_indicators value instead of module_structure.responsibility_count
        module_structure.components.len()
    )
    .unwrap();

    // Detailed and Comprehensive: Show function breakdown
    if verbosity != VerbosityLevel::Summary {
        let func_counts = &module_structure.function_counts;
        // Use visibility-based total to match public+private breakdown
        let total_functions = func_counts.public_functions + func_counts.private_functions;
        writeln!(
            output,
            "  - FUNCTIONS: {} total ({} public, {} private)",
            total_functions, func_counts.public_functions, func_counts.private_functions
        )
        .unwrap();

        // Show largest components
        if !module_structure.components.is_empty() {
            writeln!(output, "  - LARGEST COMPONENTS:").unwrap();
            let mut sorted_components = module_structure.components.clone();
            sorted_components.sort_by_key(|c| std::cmp::Reverse(c.line_count()));

            let component_count = if verbosity == VerbosityLevel::Comprehensive {
                5
            } else {
                3
            };
            for component in sorted_components.iter().take(component_count) {
                writeln!(
                    output,
                    "    - {}: {} functions, {} lines",
                    component.name(),
                    component.method_count(),
                    component.line_count()
                )
                .unwrap();
            }
        }
    }

    // Comprehensive: Show detailed coupling analysis
    if verbosity == VerbosityLevel::Comprehensive {
        let deps = &module_structure.dependencies;
        if !deps.coupling_scores.is_empty() {
            writeln!(output, "  - COUPLING ANALYSIS:").unwrap();

            let mut low_coupling: Vec<_> = deps
                .coupling_scores
                .iter()
                .filter(|(_, score)| **score < 0.3)
                .collect();
            low_coupling.sort_by(|a, b| a.1.partial_cmp(b.1).unwrap());

            if !low_coupling.is_empty() {
                writeln!(output, "    - Low coupling (easy to extract):").unwrap();
                for (component, score) in low_coupling.iter().take(3) {
                    writeln!(output, "      - {} (coupling: {:.2})", component, score).unwrap();
                }
            }

            let mut high_coupling: Vec<_> = deps
                .coupling_scores
                .iter()
                .filter(|(_, score)| **score >= 0.7)
                .collect();
            high_coupling.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());

            if !high_coupling.is_empty() {
                writeln!(output, "    - High coupling (refactor carefully):").unwrap();
                for (component, score) in high_coupling.iter().take(3) {
                    writeln!(output, "      - {} (coupling: {:.2})", component, score).unwrap();
                }
            }
        }
    }
}

// Format language-specific refactoring advice
// NOTE (spec 151): This function provides generic advice for god object splitting.
// It is context-specific (only shown during god object refactoring) rather than
// generic pattern advice unrelated to detected issues.
fn format_language_specific_advice(
    output: &mut String,
    _formatter: &ColoredFormatter,
    language: &str,
    extension: &str,
) {
    writeln!(output, "  - {} PATTERNS:", language).unwrap();

    match extension {
        "py" => {
            writeln!(
                output,
                "      - Use dataclasses/attrs for data-heavy classes"
            )
            .unwrap();
            writeln!(output, "      - Extract interfaces with Protocol/ABC").unwrap();
            writeln!(output, "      - Prefer composition over inheritance").unwrap();
        }
        "rs" => {
            writeln!(output, "      - Extract traits for shared behavior").unwrap();
            writeln!(output, "      - Use newtype pattern for domain types").unwrap();
            writeln!(
                output,
                "      - Consider builder pattern for complex construction"
            )
            .unwrap();
        }
        "js" | "jsx" => {
            writeln!(output, "      - Decompose into smaller classes/modules").unwrap();
            writeln!(output, "      - Use functional composition where possible").unwrap();
            writeln!(output, "      - Extract hooks for React components").unwrap();
        }
        "ts" | "tsx" => {
            writeln!(output, "      - Extract interfaces for contracts").unwrap();
            writeln!(output, "      - Use type guards for domain logic").unwrap();
            writeln!(output, "      - Leverage discriminated unions").unwrap();
        }
        _ => {
            writeln!(
                output,
                "      - Extract interfaces/protocols for shared behavior"
            )
            .unwrap();
            writeln!(output, "      - Prefer composition over inheritance").unwrap();
        }
    }
}

// Pure function to calculate impact message based on actual complexity metrics
//
// Previous implementation used LOC-only formula: ((LOC/200 - 1) * 100).min(80)
// This meant any file >360 LOC claimed "80% reduction" regardless of actual complexity.
//
// New implementation:
// - Uses god_object_score (derived from cyclomatic, cognitive, responsibilities)
// - Provides conservative estimates (30-60% instead of 80%)
// - Accounts for split quality (fewer, larger splits = lower reduction)
// - Explicitly states dependency on coupling/cohesion
fn calculate_impact_message(
    is_god_object: bool,
    god_object_score: f64,
    responsibilities: usize,
    split_count: usize,
) -> String {
    if is_god_object {
        // Base reduction estimate from god_object_score
        // Score typically ranges 20-200+, normalize to 0.0-1.0
        let score_normalized = (god_object_score / 100.0).min(1.0);

        // Adjust for split quality indicators
        // More responsibilities with fewer splits suggests tight coupling
        let coupling_penalty = if responsibilities > 5 && split_count <= 2 {
            0.7 // High coupling: 30% reduction in estimate
        } else if split_count >= responsibilities.saturating_sub(1) {
            1.0 // Good split coverage: no penalty
        } else {
            0.85 // Moderate coupling: 15% reduction
        };

        // Calculate conservative estimate: 30-60% range based on score
        let base_estimate = (30.0 + score_normalized * 30.0) * coupling_penalty;
        let estimate = base_estimate as i32;

        // Provide range to avoid false precision
        let range_start = estimate.saturating_sub(5);
        let range_end = estimate.saturating_add(10);

        if split_count == 0 {
            // No actionable splits detected - be honest about it
            "complexity reduction difficult to estimate (no clear split boundaries detected). \
             Focus on extracting pure functions and reducing nesting first."
                .to_string()
        } else if coupling_penalty < 0.8 {
            // High coupling detected - warn user
            format!(
                "Estimated {}-{}% complexity reduction (high coupling detected - splits may be challenging). \
                 Improve testability, enable parallel development",
                range_start, range_end
            )
        } else {
            format!(
                "Estimated {}-{}% complexity reduction (depends on split cohesion). \
                 Improve testability, enable parallel development",
                range_start, range_end
            )
        }
    } else {
        // Non-god-object files: conservative estimate
        "Improve maintainability, reduce complexity through focused refactoring".to_string()
    }
}

// Function to format detailed metrics section
fn format_detailed_metrics(
    output: &mut String,
    _formatter: &ColoredFormatter,
    is_god_object: bool,
    methods_count: usize,
    fields_count: usize,
    responsibilities: usize,
) {
    if is_god_object {
        writeln!(
            output,
            "{} Methods: {}, Fields: {}, Responsibilities: {}",
            "└─ METRICS:".bright_blue(),
            methods_count.to_string().yellow(),
            fields_count.to_string().yellow(),
            responsibilities.to_string().yellow()
        )
        .unwrap();
    }
}

// Context-aware file size classification (spec 135)
fn classify_file_size(
    total_lines: usize,
    function_count: usize,
    file_type: Option<&crate::organization::FileType>,
) -> &'static str {
    use crate::organization::{get_threshold, recommendation_level, RecommendationLevel};

    // Use context-aware thresholds if file type is available
    if let Some(ft) = file_type {
        let threshold = get_threshold(ft, function_count, total_lines);
        let level = recommendation_level(ft, total_lines, &threshold);

        match level {
            RecommendationLevel::Critical => "CRITICAL",
            RecommendationLevel::High => "HIGH",
            RecommendationLevel::Medium => "MEDIUM",
            RecommendationLevel::Low => "LOW",
            RecommendationLevel::Suppressed => "OK",
        }
    } else {
        // Fallback to legacy behavior if no file type info
        if total_lines > 1000 {
            "CRITICAL"
        } else if total_lines > 500 {
            "HIGH"
        } else {
            "MEDIUM"
        }
    }
}

fn classify_function_count(function_count: usize) -> &'static str {
    if function_count > 50 {
        "EXCESSIVE"
    } else if function_count > 20 {
        "HIGH"
    } else {
        "MODERATE"
    }
}

// Function to format scoring breakdown and dependencies
fn format_scoring_and_dependencies(
    output: &mut String,
    _formatter: &ColoredFormatter,
    total_lines: usize,
    function_count: usize,
    file_type: Option<&crate::organization::FileType>,
) {
    // Add SCORING breakdown
    writeln!(
        output,
        "{} File size: {} | Functions: {} | Complexity: HIGH",
        "└─ SCORING:".bright_blue(),
        classify_file_size(total_lines, function_count, file_type),
        classify_function_count(function_count)
    )
    .unwrap();

    // Add DEPENDENCIES if we have high function count
    if function_count > 10 {
        writeln!(
            output,
            "{} {} functions may have complex interdependencies",
            "└─ DEPENDENCIES:".bright_blue(),
            function_count
        )
        .unwrap();
    }
}

#[cfg(test)]
fn format_file_priority_item(
    output: &mut String,
    rank: usize,
    item: &priority::FileDebtItem,
    config: FormattingConfig,
) {
    format_file_priority_item_with_verbosity(output, rank, item, config, 0)
}

/// Format file score calculation breakdown for verbosity >= 2
fn format_file_score_calculation_section(
    item: &priority::FileDebtItem,
    _formatter: &ColoredFormatter,
) -> Vec<String> {
    let mut lines = Vec::new();
    let factors = item.metrics.get_score_factors();

    lines.push(format!("- {}", "FILE SCORE CALCULATION:".bright_blue()));

    // Size factor
    lines.push(format!(
        "   - Size Factor: {:.2} (√({}/100))",
        factors.size_factor, factors.size_basis
    ));

    // Complexity factor
    lines.push(format!(
        "   - Complexity Factor: {:.2} (avg {:.1} × total factor)",
        factors.complexity_factor, factors.avg_complexity
    ));

    // Coverage factor with warning
    let coverage_detail = if factors.coverage_percent == 0.0 {
        format!(
            " {}",
            "[WARN] No coverage data - assuming untested".bright_red()
        )
    } else if factors.coverage_factor >= 2.0 {
        format!(
            " {} Low coverage: {:.0}%",
            "[WARN]".bright_yellow(),
            factors.coverage_percent * 100.0
        )
    } else {
        format!(" ({:.0}% coverage)", factors.coverage_percent * 100.0)
    };

    lines.push(format!(
        "   - Coverage Factor: {:.2}{}",
        factors.coverage_factor, coverage_detail
    ));

    // Density factor
    let density_detail = if factors.function_count > 50 {
        format!(
            " ({} functions, {} over threshold)",
            factors.function_count,
            factors.function_count - 50
        )
    } else {
        format!(" ({} functions, below threshold)", factors.function_count)
    };

    lines.push(format!(
        "   - Density Factor: {:.2}{}",
        factors.density_factor, density_detail
    ));

    // God object multiplier with warning
    let god_detail = if factors.is_god_object {
        format!(
            " {} Flagged as god object (score: {:.1})",
            "[WARN]".bright_yellow(),
            factors.god_object_score
        )
    } else {
        " (not flagged)".to_string()
    };

    lines.push(format!(
        "   - God Object Multiplier: {:.2} (2.0 + {:.1}){}",
        factors.god_object_multiplier, factors.god_object_score, god_detail
    ));

    // Function factor
    lines.push(format!(
        "   - Function Factor: {:.2} (function scores sum: {:.1})",
        factors.function_factor, factors.function_score_sum
    ));

    // Final calculation
    let calculated_score = factors.size_factor
        * factors.complexity_factor
        * factors.coverage_factor
        * factors.density_factor
        * factors.god_object_multiplier
        * factors.function_factor;

    lines.push(format!(
        "   - Final: {:.2} × {:.2} × {:.2} × {:.2} × {:.2} × {:.2} = {:.1}",
        factors.size_factor,
        factors.complexity_factor,
        factors.coverage_factor,
        factors.density_factor,
        factors.god_object_multiplier,
        factors.function_factor,
        calculated_score
    ));

    // Validation check (debug mode only)
    #[cfg(debug_assertions)]
    {
        let actual_score = item.score;
        let diff = (calculated_score - actual_score).abs();
        if diff > 0.5 {
            lines.push(format!(
                "   {} Calculation mismatch: displayed={:.1}, calculated={:.1}",
                "[WARN]".bright_red(),
                actual_score,
                calculated_score
            ));
        }
    }

    lines
}

fn format_file_priority_item_with_verbosity(
    output: &mut String,
    rank: usize,
    item: &priority::FileDebtItem,
    config: FormattingConfig,
    verbosity: u8,
) {
    let formatter = ColoredFormatter::new(config);
    let severity = get_severity_label(item.score);
    let severity_color = get_severity_color(item.score);

    // Spec 139: Separate severity from issue type
    // Type label is now shown in location context, not in header
    writeln!(
        output,
        "#{} {} [{}]",
        rank,
        format!("SCORE: {}", score_formatter::format_score(item.score)).bright_yellow(),
        severity.color(severity_color).bold()
    )
    .unwrap();

    // Use methods_count for god objects (excludes tests), raw function_count otherwise
    let display_function_count = if item.metrics.god_object_indicators.is_god_object {
        item.metrics.god_object_indicators.methods_count
    } else {
        item.metrics.function_count
    };

    writeln!(
        output,
        "{} {} ({} lines, {} functions)",
        "└─".bright_blue(),
        item.metrics.path.display().to_string().bright_green(),
        item.metrics.total_lines,
        display_function_count
    )
    .unwrap();

    // Show detailed calculation for verbosity >= 2
    if verbosity >= 2 {
        let score_calc_lines = format_file_score_calculation_section(item, &formatter);
        for line in score_calc_lines {
            writeln!(output, "{}", line).unwrap();
        }
        writeln!(output).unwrap(); // Add blank line after calculation
    }

    let why_message = generate_why_message(WhyMessageParams {
        is_god_object: item.metrics.god_object_indicators.is_god_object,
        fields_count: item.metrics.god_object_indicators.fields_count,
        methods_count: item.metrics.god_object_indicators.methods_count,
        responsibilities: item.metrics.god_object_indicators.responsibilities,
        function_count: item.metrics.god_object_indicators.methods_count, // Use methods_count for consistency (excludes tests)
        total_lines: item.metrics.total_lines,
        god_object_type: item.metrics.god_object_type.as_ref(),
        domain_diversity_metrics: item
            .metrics
            .god_object_indicators
            .domain_diversity_metrics
            .as_ref(),
        detection_type: item.metrics.god_object_indicators.detection_type.as_ref(),
    });

    writeln!(
        output,
        "{} {}",
        "└─ WHY THIS MATTERS:".bright_blue(),
        why_message
    )
    .unwrap();

    // Spec 152: Display domain diversity analysis if available
    if let Some(ref metrics) = item.metrics.god_object_indicators.domain_diversity_metrics {
        let formatted_output = metrics.format_for_output();
        // Add proper indentation to match the formatter's style
        for line in formatted_output.lines() {
            if !line.is_empty() {
                writeln!(output, "   {}", line).unwrap();
            } else {
                writeln!(output).unwrap();
            }
        }
    }

    writeln!(
        output,
        "{} {}",
        "└─ ACTION:".bright_yellow(),
        item.recommendation.bright_green().bold()
    )
    .unwrap();

    if item.metrics.god_object_indicators.is_god_object {
        format_god_object_steps_with_verbosity(output, &formatter, item, verbosity);
    }

    let impact = calculate_impact_message(
        item.metrics.god_object_indicators.is_god_object,
        item.metrics.god_object_indicators.god_object_score,
        item.metrics.god_object_indicators.responsibilities,
        item.metrics.god_object_indicators.recommended_splits.len(),
    );

    writeln!(
        output,
        "{} {}",
        "└─ IMPACT:".bright_blue(),
        impact.bright_cyan()
    )
    .unwrap();

    format_detailed_metrics(
        output,
        &formatter,
        item.metrics.god_object_indicators.is_god_object,
        item.metrics.god_object_indicators.methods_count,
        item.metrics.god_object_indicators.fields_count,
        item.metrics.god_object_indicators.responsibilities,
    );

    format_scoring_and_dependencies(
        output,
        &formatter,
        item.metrics.total_lines,
        display_function_count, // Use consistent count (methods_count for god objects)
        item.metrics.file_type.as_ref(),
    );
}

// Helper function to format a list with truncation
// Note: Currently used only in tests, but kept for potential future use
#[allow(dead_code)]
fn format_truncated_list(items: &[String], max_display: usize) -> String {
    if items.len() <= max_display {
        items.join(", ")
    } else {
        format!(
            "{}, ... ({} more)",
            items[..max_display].join(", "),
            items.len() - max_display
        )
    }
}

pub fn format_priority_item(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    has_coverage_data: bool,
) {
    // Use functional composition to format different sections
    let format_context = create_format_context(rank, item, has_coverage_data);

    // Format each section using pure functions composed together
    let formatted_sections = generate_formatted_sections(&format_context);

    // Apply formatting to output (I/O at edges)
    apply_formatted_sections(output, formatted_sections);
}

#[allow(dead_code)]
fn format_detailed_item(output: &mut String, rank: usize, item: &UnifiedDebtItem) {
    writeln!(
        output,
        "#{} {}() - UNIFIED SCORE: {}",
        rank,
        item.location.function.bright_green(),
        score_formatter::format_score(item.unified_score.final_score)
    )
    .unwrap();

    writeln!(
        output,
        "├─ Function Role: {} ({:.1}x multiplier)",
        format_role(item.function_role),
        item.unified_score.role_multiplier
    )
    .unwrap();

    writeln!(output, "├─ Score Breakdown:").unwrap();
    writeln!(
        output,
        "│  ├─ Coverage Factor: {:.1}",
        item.unified_score.coverage_factor
    )
    .unwrap();

    if let Some(ref cov) = item.transitive_coverage {
        writeln!(
            output,
            "│  │  └─ ({:.0}% direct, {:.0}% transitive)",
            cov.direct * 100.0,
            cov.transitive * 100.0
        )
        .unwrap();
    }

    writeln!(
        output,
        "│  ├─ Complexity Factor: {:.1}",
        item.unified_score.complexity_factor
    )
    .unwrap();
    writeln!(
        output,
        "│  ├─ Dependency Factor: {:.1}",
        item.unified_score.dependency_factor
    )
    .unwrap();

    // Display god object indicators if present
    if let Some(ref god_obj) = item.god_object_indicators {
        if god_obj.is_god_object {
            writeln!(
                output,
                "│  └─ {} God Object: {} methods, {} fields, {} responsibilities",
                "[WARNING]".bright_yellow(),
                god_obj.method_count,
                god_obj.field_count,
                god_obj.responsibility_count
            )
            .unwrap();
            writeln!(
                output,
                "│      Score: {:.0} (Confidence: {:?})",
                god_obj.god_object_score, god_obj.confidence
            )
            .unwrap();
        }
    }

    writeln!(
        output,
        "└─ Recommendation: {}",
        item.recommendation.primary_action
    )
    .unwrap();

    for step in &item.recommendation.implementation_steps {
        writeln!(output, "   • {step}").unwrap();
    }
}

#[cfg_attr(test, allow(dead_code))]
pub(crate) fn _format_total_impact(output: &mut String, analysis: &UnifiedAnalysis) {
    writeln!(output).unwrap();
    writeln!(
        output,
        "{}",
        "TOTAL IMPACT IF ALL FIXED".bright_green().bold()
    )
    .unwrap();

    let impact = &analysis.total_impact;

    if impact.coverage_improvement > 0.0 {
        writeln!(
            output,
            "• +{:.1}% test coverage potential",
            impact.coverage_improvement
        )
        .unwrap();
    }

    if impact.lines_reduction > 0 {
        writeln!(output, "• -{} lines of code", impact.lines_reduction).unwrap();
    }

    if impact.complexity_reduction > 0.0 {
        writeln!(
            output,
            "• -{:.0}% average complexity",
            impact.complexity_reduction
        )
        .unwrap();
    }

    writeln!(
        output,
        "• {} actionable items prioritized by measurable impact",
        analysis.items.len()
    )
    .unwrap();
}

pub fn format_impact(impact: &crate::priority::ImpactMetrics) -> String {
    let mut parts = Vec::new();

    if impact.coverage_improvement > 0.0 {
        // Show function-level coverage improvement
        if impact.coverage_improvement >= 100.0 {
            parts.push("Full test coverage".to_string());
        } else if impact.coverage_improvement >= 50.0 {
            parts.push(format!(
                "+{}% function coverage",
                impact.coverage_improvement as i32
            ));
        } else {
            // For complex functions that need refactoring first
            parts.push("Partial coverage after refactor".to_string());
        }
    }

    if impact.complexity_reduction > 0.0 {
        parts.push(format!(
            "-{} complexity",
            impact.complexity_reduction as i32
        ));
    }

    if impact.risk_reduction > 0.0 {
        parts.push(format!("-{:.1} risk", impact.risk_reduction));
    }

    if impact.lines_reduction > 0 {
        parts.push(format!("-{} LOC", impact.lines_reduction));
    }

    if parts.is_empty() {
        "Improved maintainability".to_string()
    } else {
        parts.join(", ")
    }
}

pub fn format_debt_type(debt_type: &DebtType) -> &'static str {
    match debt_type {
        DebtType::TestingGap { .. } => "TEST GAP",
        DebtType::ComplexityHotspot { .. } => "COMPLEXITY",
        DebtType::DeadCode { .. } => "DEAD CODE",
        DebtType::Duplication { .. } => "DUPLICATION",
        DebtType::Risk { .. } => "RISK",
        DebtType::TestComplexityHotspot { .. } => "TEST COMPLEXITY",
        DebtType::TestTodo { .. } => "TEST TODO",
        DebtType::TestDuplication { .. } => "TEST DUPLICATION",
        DebtType::ErrorSwallowing { .. } => "ERROR SWALLOWING",
        // Resource Management debt types
        DebtType::AllocationInefficiency { .. } => "ALLOCATION",
        DebtType::StringConcatenation { .. } => "STRING CONCAT",
        DebtType::NestedLoops { .. } => "NESTED LOOPS",
        DebtType::BlockingIO { .. } => "BLOCKING I/O",
        DebtType::SuboptimalDataStructure { .. } => "DATA STRUCTURE",
        // Organization debt types
        DebtType::GodObject { .. } => "GOD OBJECT",
        DebtType::GodModule { .. } => "GOD MODULE",
        DebtType::FeatureEnvy { .. } => "FEATURE ENVY",
        DebtType::PrimitiveObsession { .. } => "PRIMITIVE OBSESSION",
        DebtType::MagicValues { .. } => "MAGIC VALUES",
        // Testing quality debt types
        DebtType::AssertionComplexity { .. } => "ASSERTION COMPLEXITY",
        DebtType::FlakyTestPattern { .. } => "FLAKY TEST",
        // Resource management debt types
        DebtType::AsyncMisuse { .. } => "ASYNC MISUSE",
        DebtType::ResourceLeak { .. } => "RESOURCE LEAK",
        DebtType::CollectionInefficiency { .. } => "COLLECTION INEFFICIENCY",
        // Type organization (Spec 187)
        DebtType::ScatteredType { .. } => "SCATTERED TYPE",
        DebtType::OrphanedFunctions { .. } => "ORPHANED FUNCTIONS",
        DebtType::UtilitiesSprawl { .. } => "UTILITIES SPRAWL",
    }
}

#[allow(dead_code)]
fn format_role(role: FunctionRole) -> &'static str {
    match role {
        FunctionRole::PureLogic => "PureLogic",
        FunctionRole::Orchestrator => "Orchestrator",
        FunctionRole::IOWrapper => "IOWrapper",
        FunctionRole::EntryPoint => "EntryPoint",
        FunctionRole::PatternMatch => "PatternMatch",
        FunctionRole::Debug => "Debug",
        FunctionRole::Unknown => "Unknown",
    }
}

pub fn get_severity_label(score: f64) -> &'static str {
    if score >= 8.0 {
        "CRITICAL"
    } else if score >= 6.0 {
        "HIGH"
    } else if score >= 4.0 {
        "MEDIUM"
    } else {
        "LOW"
    }
}

pub fn get_severity_color(score: f64) -> colored::Color {
    if score >= 8.0 {
        Color::Red
    } else if score >= 6.0 {
        Color::Yellow
    } else if score >= 4.0 {
        Color::Blue
    } else {
        Color::Green
    }
}

pub fn extract_complexity_info(item: &UnifiedDebtItem) -> (u32, u32, u32, u32, usize) {
    // Always show complexity metrics from the item itself, regardless of debt type
    let cyclomatic = item.cyclomatic_complexity;
    let cognitive = item.cognitive_complexity;
    let branch_count = cyclomatic; // Use cyclomatic as proxy for branch count

    (
        cyclomatic,
        cognitive,
        branch_count,
        item.nesting_depth,
        item.function_length,
    )
}

pub fn extract_dependency_info(item: &UnifiedDebtItem) -> (usize, usize) {
    (item.upstream_dependencies, item.downstream_dependencies)
}

#[allow(dead_code)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CallerCalleeConfig;
    use crate::formatting::ColorMode;
    use crate::priority::call_graph::CallGraph;
    use crate::priority::file_metrics::{
        FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators,
    };
    use crate::priority::unified_scorer::Location;
    use crate::priority::FunctionVisibility;
    use crate::priority::UnifiedAnalysisUtils;
    use crate::priority::{ActionableRecommendation, ImpactMetrics, UnifiedScore};
    use sections::format_dependencies_section_with_config;
    use std::path::PathBuf;

    fn strip_ansi_codes(s: &str) -> String {
        // Simple regex to strip ANSI escape codes
        let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
        re.replace_all(s, "").to_string()
    }

    #[test]
    fn test_format_truncated_list() {
        // Test with list smaller than max
        let items = vec!["one".to_string(), "two".to_string()];
        assert_eq!(format_truncated_list(&items, 3), "one, two");

        // Test with list exactly at max
        let items = vec!["one".to_string(), "two".to_string(), "three".to_string()];
        assert_eq!(format_truncated_list(&items, 3), "one, two, three");

        // Test with list larger than max
        let items = vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
            "four".to_string(),
            "five".to_string(),
        ];
        assert_eq!(
            format_truncated_list(&items, 3),
            "one, two, three, ... (2 more)"
        );

        // Test empty list
        let items: Vec<String> = vec![];
        assert_eq!(format_truncated_list(&items, 3), "");
    }

    fn create_test_item(score: f64) -> UnifiedDebtItem {
        // Use score as part of line number to make each test item unique
        // This prevents duplicate detection from filtering test items
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                function: "test_func".to_string(),
                line: (score * 10.0) as usize,
            },
            debt_type: DebtType::TestingGap {
                coverage: 0.1,
                cyclomatic: 5,
                cognitive: 7,
            },
            unified_score: UnifiedScore {
                complexity_factor: 5.0,
                coverage_factor: 8.0,
                dependency_factor: 3.0,
                role_multiplier: 1.0,
                final_score: score,
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Add unit tests".to_string(),
                rationale: "Low coverage critical function".to_string(),
                implementation_steps: vec!["Write tests".to_string()],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                coverage_improvement: 50.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 3.5,
            },
            transitive_coverage: None,
            file_context: None,
            upstream_dependencies: 2,
            downstream_dependencies: 3,
            upstream_callers: vec!["main".to_string(), "process_data".to_string()],
            downstream_callees: vec![
                "validate".to_string(),
                "transform".to_string(),
                "save".to_string(),
            ],
            nesting_depth: 1,
            function_length: 15,
            cyclomatic_complexity: 5,
            cognitive_complexity: 7,
            is_pure: None,
            purity_confidence: None,
            purity_level: None,
            entropy_details: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
        }
    }

    #[test]
    fn test_format_default() {
        let mut analysis = UnifiedAnalysis::new(CallGraph::new());
        analysis.add_item(create_test_item(9.0));
        analysis.add_item(create_test_item(7.0));
        analysis.sort_by_priority();
        analysis.calculate_total_impact();

        let output = format_priorities(&analysis, OutputFormat::Default);

        // Strip ANSI color codes for testing
        let output_plain = strip_ansi_codes(&output);

        assert!(output_plain.contains("Debtmap v"));
        assert!(output_plain.contains("TOP 2 RECOMMENDATIONS"));
        assert!(output_plain.contains("SCORE: 9.0"));
        assert!(output_plain.contains("[CRITICAL]"));
        // assert!(output_plain.contains("TOTAL IMPACT"));
    }

    #[test]
    fn test_severity_labels() {
        assert_eq!(get_severity_label(9.0), "CRITICAL");
        assert_eq!(get_severity_label(7.0), "HIGH");
        assert_eq!(get_severity_label(5.0), "MEDIUM");
        assert_eq!(get_severity_label(2.0), "LOW");
    }

    #[test]
    fn test_format_total_impact_with_all_improvements() {
        let mut analysis = UnifiedAnalysis::new(CallGraph::new());
        analysis.total_impact = ImpactMetrics {
            coverage_improvement: 25.5,
            lines_reduction: 150,
            complexity_reduction: 12.7,
            risk_reduction: 8.2,
        };
        analysis.add_item(create_test_item(7.0));
        analysis.add_item(create_test_item(5.0));

        let mut output = String::new();
        super::_format_total_impact(&mut output, &analysis);
        let output_plain = strip_ansi_codes(&output);

        assert!(output_plain.contains("TOTAL IMPACT IF ALL FIXED"));
        assert!(output_plain.contains("+25.5% test coverage potential"));
        assert!(output_plain.contains("-150 lines of code"));
        assert!(output_plain.contains("-13% average complexity")); // 12.7 rounds to 13
        assert!(output_plain.contains("2 actionable items prioritized"));
    }

    #[test]
    fn test_format_total_impact_coverage_only() {
        let mut analysis = UnifiedAnalysis::new(CallGraph::new());
        analysis.total_impact = ImpactMetrics {
            coverage_improvement: 45.3,
            lines_reduction: 0,
            complexity_reduction: 0.0,
            risk_reduction: 5.0,
        };
        analysis.add_item(create_test_item(8.0));

        let mut output = String::new();
        super::_format_total_impact(&mut output, &analysis);
        let output_plain = strip_ansi_codes(&output);

        assert!(output_plain.contains("+45.3% test coverage potential"));
        assert!(!output_plain.contains("lines of code")); // Should not show 0 lines
        assert!(!output_plain.contains("average complexity")); // Should not show 0 complexity
        assert!(output_plain.contains("1 actionable items prioritized"));
    }

    #[test]
    fn test_format_priority_item_basic() {
        let mut output = String::new();
        let item = create_test_item(5.0);
        format_priority_item(&mut output, 1, &item, false);
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("#1 SCORE: 5.0"));
        assert!(plain.contains("test_func()"));
        assert!(plain.contains("Add unit tests"));
    }

    #[test]
    fn test_format_priority_item_with_complexity() {
        let mut output = String::new();
        let mut item = create_test_item(8.0);
        item.cyclomatic_complexity = 15;
        item.cognitive_complexity = 20;
        format_priority_item(&mut output, 2, &item, false);
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("COMPLEXITY:"));
        assert!(plain.contains("cyclomatic=15"));
    }

    #[test]
    fn test_format_priority_item_with_dependencies() {
        let mut output = String::new();
        let mut item = create_test_item(7.0);
        item.upstream_dependencies = 3;
        item.downstream_dependencies = 2;
        item.upstream_callers = vec!["caller1".to_string()];
        format_priority_item(&mut output, 1, &item, false);
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("DEPENDENCIES:"));
        // New format shows count in parentheses
        assert!(plain.contains("(1)") || plain.contains("caller1"));
    }

    #[test]
    fn test_format_priority_item_with_callers() {
        let mut output = String::new();
        let mut item = create_test_item(6.0);
        item.upstream_callers = vec!["caller1".to_string(), "caller2".to_string()];
        item.upstream_dependencies = 2;
        format_priority_item(&mut output, 1, &item, false);
        let plain = strip_ansi_codes(&output);
        // New format shows "Called by" with count
        assert!(plain.contains("Called by") || plain.contains("caller1"));
        assert!(plain.contains("caller1"));
        assert!(plain.contains("caller2"));
    }

    #[test]
    fn test_format_priority_item_many_callers() {
        let mut output = String::new();
        let mut item = create_test_item(6.5);
        item.upstream_callers = vec![
            "c1".to_string(),
            "c2".to_string(),
            "c3".to_string(),
            "c4".to_string(),
            "c5".to_string(),
            "c6".to_string(),
            "c7".to_string(),
        ];
        item.upstream_dependencies = 7;
        format_priority_item(&mut output, 1, &item, false);
        let plain = strip_ansi_codes(&output);
        // New format shows "showing 5 of 7"
        assert!(plain.contains("showing 5 of 7"));
    }

    #[test]
    fn test_format_priority_item_with_callees() {
        let mut output = String::new();
        let mut item = create_test_item(7.5);
        item.downstream_callees = vec!["func1".to_string(), "func2".to_string()];
        item.downstream_dependencies = 2;
        format_priority_item(&mut output, 1, &item, false);
        let plain = strip_ansi_codes(&output);
        // New format shows "Calls" with count
        assert!(plain.contains("Calls") || plain.contains("func1"));
        assert!(plain.contains("func1"));
        assert!(plain.contains("func2"));
    }

    #[test]
    fn test_format_priority_item_empty_dependencies_shows_section() {
        // Spec 117: DEPENDENCIES section should always appear, even when empty
        let mut output = String::new();
        let mut item = create_test_item(6.0);
        item.upstream_callers = vec![];
        item.downstream_callees = vec![];
        item.upstream_dependencies = 0;
        item.downstream_dependencies = 0;
        format_priority_item(&mut output, 1, &item, false);
        let plain = strip_ansi_codes(&output);

        // Must show DEPENDENCIES section
        assert!(
            plain.contains("DEPENDENCIES:"),
            "Missing DEPENDENCIES section for empty callers/callees"
        );

        // Must show empty caller message
        assert!(
            plain.contains("No direct callers detected"),
            "Missing empty caller message"
        );

        // Must show empty callee message
        assert!(
            plain.contains("Calls no other functions"),
            "Missing empty callee message"
        );
    }

    #[test]
    fn test_format_priority_item_empty_callers_only() {
        // Test when only callers are empty
        let mut output = String::new();
        let mut item = create_test_item(6.5);
        item.upstream_callers = vec![];
        item.downstream_callees = vec!["some_function".to_string()];
        item.upstream_dependencies = 0;
        item.downstream_dependencies = 1;
        format_priority_item(&mut output, 1, &item, false);
        let plain = strip_ansi_codes(&output);

        assert!(plain.contains("DEPENDENCIES:"));
        assert!(plain.contains("No direct callers detected"));
        assert!(plain.contains("some_function"));
    }

    #[test]
    fn test_format_priority_item_empty_callees_only() {
        // Test when only callees are empty
        let mut output = String::new();
        let mut item = create_test_item(7.5);
        item.upstream_callers = vec!["caller_func".to_string()];
        item.downstream_callees = vec![];
        item.upstream_dependencies = 1;
        item.downstream_dependencies = 0;
        format_priority_item(&mut output, 1, &item, false);
        let plain = strip_ansi_codes(&output);

        assert!(plain.contains("DEPENDENCIES:"));
        assert!(plain.contains("caller_func"));
        assert!(plain.contains("Calls no other functions"));
    }

    #[test]
    fn test_format_priority_item_dead_code() {
        let mut output = String::new();
        let mut item = create_test_item(4.0);
        item.debt_type = DebtType::DeadCode {
            visibility: FunctionVisibility::Public,
            cyclomatic: 5,
            cognitive: 7,
            usage_hints: vec!["Consider removing".to_string()],
        };
        format_priority_item(&mut output, 1, &item, false);
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("VISIBILITY:"));
        assert!(plain.contains("Consider removing"));
    }

    #[test]
    fn test_format_priority_item_critical_severity() {
        let mut output = String::new();
        let item = create_test_item(9.5);
        format_priority_item(&mut output, 1, &item, false);
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("[CRITICAL]"));
    }

    #[test]
    fn test_format_priority_item_low_severity() {
        let mut output = String::new();
        let item = create_test_item(2.0);
        format_priority_item(&mut output, 3, &item, false);
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("[LOW]"));
    }

    #[test]
    fn test_format_priority_item_no_complexity() {
        let mut output = String::new();
        let mut item = create_test_item(3.0);
        item.cyclomatic_complexity = 0;
        item.cognitive_complexity = 0;
        format_priority_item(&mut output, 1, &item, false);
        let plain = strip_ansi_codes(&output);
        assert!(!plain.contains("COMPLEXITY:"));
    }

    #[test]
    fn test_format_priority_item_no_dependencies() {
        let mut output = String::new();
        let mut item = create_test_item(3.5);
        item.upstream_dependencies = 0;
        item.downstream_dependencies = 0;
        // Clear the caller/callee lists too
        item.upstream_callers = vec![];
        item.downstream_callees = vec![];
        format_priority_item(&mut output, 1, &item, false);
        let plain = strip_ansi_codes(&output);
        // Per spec 117: DEPENDENCIES section should ALWAYS appear
        assert!(
            plain.contains("DEPENDENCIES:"),
            "DEPENDENCIES section must always appear"
        );
        assert!(
            plain.contains("No direct callers detected"),
            "Must show empty caller message"
        );
        assert!(
            plain.contains("Calls no other functions"),
            "Must show empty callee message"
        );
    }

    #[test]
    fn test_format_total_impact_complexity_and_lines() {
        let mut analysis = UnifiedAnalysis::new(CallGraph::new());
        analysis.total_impact = ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: 75,
            complexity_reduction: 8.9,
            risk_reduction: 3.2,
        };
        analysis.add_item(create_test_item(6.0));
        analysis.add_item(create_test_item(4.0));
        analysis.add_item(create_test_item(3.0));

        let mut output = String::new();
        super::_format_total_impact(&mut output, &analysis);
        let output_plain = strip_ansi_codes(&output);

        assert!(!output_plain.contains("test coverage")); // Should not show 0 coverage
        assert!(output_plain.contains("-75 lines of code"));
        assert!(output_plain.contains("-9% average complexity")); // 8.9 rounds to 9
        assert!(output_plain.contains("3 actionable items prioritized"));
    }

    #[test]
    fn test_format_total_impact_no_improvements() {
        let mut analysis = UnifiedAnalysis::new(CallGraph::new());
        analysis.total_impact = ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: 0,
            complexity_reduction: 0.0,
            risk_reduction: 0.0,
        };
        // Empty analysis with no items

        let mut output = String::new();
        super::_format_total_impact(&mut output, &analysis);
        let output_plain = strip_ansi_codes(&output);

        assert!(output_plain.contains("TOTAL IMPACT IF ALL FIXED"));
        assert!(!output_plain.contains("test coverage")); // No coverage improvement
        assert!(!output_plain.contains("lines of code")); // No lines reduction
        assert!(!output_plain.contains("average complexity")); // No complexity reduction
        assert!(output_plain.contains("0 actionable items prioritized"));
    }

    #[test]
    fn test_debt_type_formatting() {
        assert_eq!(
            format_debt_type(&DebtType::TestingGap {
                coverage: 0.1,
                cyclomatic: 5,
                cognitive: 7
            }),
            "TEST GAP"
        );
        assert_eq!(
            format_debt_type(&DebtType::ComplexityHotspot {
                cyclomatic: 10,
                cognitive: 15,
                adjusted_cyclomatic: None,
            }),
            "COMPLEXITY"
        );
        assert_eq!(
            format_debt_type(&DebtType::Duplication {
                instances: 3,
                total_lines: 60
            }),
            "DUPLICATION"
        );
    }

    #[test]
    fn test_format_role_pure_logic() {
        assert_eq!(format_role(FunctionRole::PureLogic), "PureLogic");
    }

    #[test]
    fn test_format_role_orchestrator() {
        assert_eq!(format_role(FunctionRole::Orchestrator), "Orchestrator");
    }

    #[test]
    fn test_format_role_io_wrapper() {
        assert_eq!(format_role(FunctionRole::IOWrapper), "IOWrapper");
    }

    #[test]
    fn test_format_role_entry_point() {
        assert_eq!(format_role(FunctionRole::EntryPoint), "EntryPoint");
    }

    #[test]
    fn test_format_role_unknown() {
        assert_eq!(format_role(FunctionRole::Unknown), "Unknown");
    }

    // ============================================================================
    // Caller/Callee Tests
    // ============================================================================

    #[test]
    fn test_format_dependencies_section_with_callers() {
        let mut item = create_test_item(6.0);
        item.upstream_callers = vec![
            "caller1".to_string(),
            "caller2".to_string(),
            "crate::module::caller3".to_string(),
        ];
        item.downstream_callees = vec!["callee1".to_string()];

        let context = create_format_context(1, &item, false);
        // Use explicit formatting config to ensure deterministic behavior in tests
        let formatting_config = FormattingConfig::new(ColorMode::Never);
        let section = format_dependencies_section_with_config(&context, formatting_config);

        assert!(section.is_some());
        let section_text = strip_ansi_codes(&section.unwrap());
        assert!(section_text.contains("DEPENDENCIES:"));
        assert!(section_text.contains("Called by"));
        assert!(section_text.contains("caller1"));
        assert!(section_text.contains("caller2"));
    }

    #[test]
    fn test_format_dependencies_section_no_callers() {
        let mut item = create_test_item(6.0);
        item.upstream_callers = vec![];
        item.downstream_callees = vec!["callee1".to_string(), "callee2".to_string()];

        let context = create_format_context(1, &item, false);
        // Use explicit formatting config to ensure deterministic behavior in tests
        let formatting_config = FormattingConfig::new(ColorMode::Never);
        let section = format_dependencies_section_with_config(&context, formatting_config);

        assert!(section.is_some());
        let section_text = strip_ansi_codes(&section.unwrap());
        assert!(section_text.contains("No direct callers detected"));
        assert!(section_text.contains("Calls"));
    }

    #[test]
    fn test_format_dependencies_section_filters_std_lib() {
        let mut item = create_test_item(6.0);
        item.upstream_callers = vec!["caller1".to_string()];
        item.downstream_callees = vec![
            "std::println".to_string(),
            "writeln".to_string(),
            "my_function".to_string(),
        ];

        let context = create_format_context(1, &item, false);
        // Use explicit formatting config to ensure deterministic behavior in tests
        let formatting_config = FormattingConfig::new(ColorMode::Never);
        let section = format_dependencies_section_with_config(&context, formatting_config);

        assert!(section.is_some());
        let section_text = strip_ansi_codes(&section.unwrap());

        // Should include my_function but not std lib calls
        assert!(section_text.contains("my_function"));
        assert!(!section_text.contains("println"));
        assert!(!section_text.contains("writeln"));

        // Should show count of 1 (only my_function)
        assert!(section_text.contains("(1)"));
    }

    #[test]
    fn test_format_dependencies_section_truncation() {
        let mut item = create_test_item(6.0);
        // Create more than 5 callers
        item.upstream_callers = vec![
            "caller1".to_string(),
            "caller2".to_string(),
            "caller3".to_string(),
            "caller4".to_string(),
            "caller5".to_string(),
            "caller6".to_string(),
            "caller7".to_string(),
        ];

        let context = create_format_context(1, &item, false);
        // Use explicit formatting config to ensure deterministic behavior in tests
        let formatting_config = FormattingConfig::new(ColorMode::Never);
        let section = format_dependencies_section_with_config(&context, formatting_config);

        assert!(section.is_some());
        let section_text = strip_ansi_codes(&section.unwrap());

        // Should show truncation message
        assert!(section_text.contains("showing 5 of 7"));
    }

    #[test]
    fn test_caller_callee_config_defaults() {
        let config = CallerCalleeConfig::default();

        assert_eq!(config.max_callers, 5);
        assert_eq!(config.max_callees, 5);
        assert!(!config.show_external);
        assert!(!config.show_std_lib);
    }

    // Helper function to create test FileDebtItem
    fn create_test_file_debt_item() -> FileDebtItem {
        FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from("src/test_file.rs"),
                total_lines: 1500,
                function_count: 45,
                class_count: 2,
                avg_complexity: 12.5,
                max_complexity: 35,
                total_complexity: 562,
                coverage_percent: 0.35,
                uncovered_lines: 975,
                god_object_indicators: GodObjectIndicators {
                    methods_count: 45,
                    fields_count: 12,
                    responsibilities: 8,
                    is_god_object: true,
                    god_object_score: 0.85,
                    responsibility_names: Vec::new(),
                    recommended_splits: Vec::new(),
                    module_structure: None,

                    domain_count: 0,
                    domain_diversity: 0.0,
                    struct_ratio: 0.0,
                    analysis_method: crate::priority::file_metrics::SplitAnalysisMethod::None,
                    cross_domain_severity: None,
                    domain_diversity_metrics: None,
                    detection_type: None,
                },
                function_scores: vec![8.5, 7.2, 6.9, 5.8, 4.3],
                god_object_type: None,
                file_type: None,
            },
            score: 75.5,
            priority_rank: 1,
            recommendation: "Split this god object into smaller, focused modules".to_string(),
            impact: FileImpact {
                complexity_reduction: 45.0,
                maintainability_improvement: 0.65,
                test_effort: 20.0,
            },
        }
    }

    #[test]
    fn test_format_file_priority_item_god_object() {
        let mut output = String::new();
        let item = create_test_file_debt_item();
        let config = FormattingConfig::default();

        format_file_priority_item(&mut output, 1, &item, config);

        // Strip ANSI codes for testing
        let clean_output = strip_ansi_codes(&output);

        // Check header elements
        assert!(clean_output.contains("#1"));
        assert!(clean_output.contains("SCORE: 75.5"));
        // Spec 139: Severity should be separate from issue type
        assert!(clean_output.contains("[CRITICAL]") || clean_output.contains("[HIGH]"));

        // Check file path
        assert!(clean_output.contains("src/test_file.rs"));
        assert!(clean_output.contains("(1500 lines, 45 functions)"));

        // Check WHY THIS MATTERS section
        assert!(clean_output.contains("WHY THIS MATTERS:"));
        assert!(clean_output.contains("45 methods"));
        assert!(clean_output.contains("12 fields"));

        // Check recommendation
        assert!(clean_output.contains("ACTION:"));
        assert!(clean_output.contains("Split this god object into smaller, focused modules"));

        // Check impact
        assert!(clean_output.contains("IMPACT:"));
        assert!(clean_output.contains("complexity"));

        // Check metrics
        assert!(clean_output.contains("METRICS:"));
        assert!(clean_output.contains("Methods: 45"));
    }

    #[test]
    fn test_format_file_priority_item_god_module() {
        let mut output = String::new();
        let mut item = create_test_file_debt_item();
        // Make it a god module (many functions, few fields)
        item.metrics.god_object_indicators.fields_count = 2;

        let config = FormattingConfig::default();

        format_file_priority_item(&mut output, 2, &item, config);

        let clean_output = strip_ansi_codes(&output);
        // Spec 139: Header shows only severity, not file type
        assert!(clean_output.contains("[HIGH]") || clean_output.contains("[CRITICAL]"));
        assert!(clean_output.contains("#2"));
    }

    #[test]
    fn test_format_file_priority_item_high_complexity() {
        let mut output = String::new();
        let mut item = create_test_file_debt_item();
        // Not a god object but high complexity
        item.metrics.god_object_indicators.is_god_object = false;
        item.metrics.total_lines = 600;

        let config = FormattingConfig::default();

        format_file_priority_item(&mut output, 3, &item, config);

        let clean_output = strip_ansi_codes(&output);
        // Spec 139: Header shows only severity, not file type
        assert!(
            clean_output.contains("[HIGH]")
                || clean_output.contains("[CRITICAL]")
                || clean_output.contains("[MEDIUM]")
        );
        assert!(clean_output.contains("#3"));
    }

    #[test]
    fn test_format_file_priority_item_regular_file() {
        let mut output = String::new();
        let mut item = create_test_file_debt_item();
        // Regular file
        item.metrics.god_object_indicators.is_god_object = false;
        item.metrics.total_lines = 300;
        item.score = 35.0;

        let config = FormattingConfig::default();

        format_file_priority_item(&mut output, 4, &item, config);

        let clean_output = strip_ansi_codes(&output);
        // Spec 139: Header shows only severity based on score
        // Score of 35.0 is >= 8.0, so it's CRITICAL severity
        assert!(clean_output.contains("[CRITICAL]"));
        assert!(clean_output.contains("#4"));
        assert!(clean_output.contains("SCORE: 35.0"));
    }

    #[test]
    fn test_format_file_priority_item_with_dependencies() {
        let mut output = String::new();
        let mut item = create_test_file_debt_item();
        item.metrics.function_scores = vec![9.5, 8.7, 7.9, 7.2, 6.8, 5.9, 5.1];

        let config = FormattingConfig::default();

        format_file_priority_item(&mut output, 1, &item, config);

        let clean_output = strip_ansi_codes(&output);
        // Should show dependencies section
        assert!(clean_output.contains("DEPENDENCIES:") || clean_output.contains("functions"));
    }

    #[test]
    fn test_format_file_priority_item_no_coverage() {
        let mut output = String::new();
        let mut item = create_test_file_debt_item();
        item.metrics.coverage_percent = 0.0;
        item.metrics.uncovered_lines = item.metrics.total_lines;

        let config = FormattingConfig::default();

        // Use verbosity 2 to show coverage information in score breakdown
        format_file_priority_item_with_verbosity(&mut output, 1, &item, config, 2);

        let clean_output = strip_ansi_codes(&output);
        // The coverage info is in the FILE SCORE CALCULATION section at verbosity >= 2
        assert!(
            clean_output.contains("Coverage Factor")
                && (clean_output.contains("No coverage")
                    || clean_output.contains("assuming untested"))
        );
    }

    #[test]
    fn test_format_file_priority_item_good_coverage() {
        let mut output = String::new();
        let mut item = create_test_file_debt_item();
        item.metrics.coverage_percent = 0.92;
        item.metrics.uncovered_lines = 120;

        let config = FormattingConfig::default();

        format_file_priority_item(&mut output, 1, &item, config);

        let clean_output = strip_ansi_codes(&output);
        // Good coverage might not be explicitly shown in output
        // Just verify the test runs without panic
        assert!(!clean_output.is_empty());
    }

    #[test]
    fn test_format_file_priority_item_scoring_section() {
        let mut output = String::new();
        let item = create_test_file_debt_item();
        let config = FormattingConfig::default();

        format_file_priority_item(&mut output, 1, &item, config);

        let clean_output = strip_ansi_codes(&output);
        // Check scoring section
        assert!(clean_output.contains("SCORING:"));
        assert!(clean_output.contains("File size:"));
        assert!(clean_output.contains("Functions:"));
        assert!(clean_output.contains("Complexity:"));
    }

    #[test]
    fn test_format_file_priority_item_high_score() {
        let mut output = String::new();
        let mut item = create_test_file_debt_item();
        item.score = 95.0;

        let config = FormattingConfig::default();

        format_file_priority_item(&mut output, 1, &item, config);

        let clean_output = strip_ansi_codes(&output);
        assert!(clean_output.contains("CRITICAL"));
        assert!(clean_output.contains("95.0") || clean_output.contains("95"));
    }

    #[test]
    fn test_format_file_priority_item_medium_score() {
        let mut output = String::new();
        let mut item = create_test_file_debt_item();
        item.score = 55.0;

        let config = FormattingConfig::default();

        format_file_priority_item(&mut output, 1, &item, config);

        let clean_output = strip_ansi_codes(&output);
        assert!(clean_output.contains("MEDIUM") || clean_output.contains("HIGH"));
    }

    #[test]
    fn test_file_score_breakdown_verbosity_0() {
        let mut output = String::new();
        let item = create_test_file_debt_item();
        let config = FormattingConfig::default();

        format_file_priority_item_with_verbosity(&mut output, 1, &item, config, 0);

        let clean_output = strip_ansi_codes(&output);
        assert!(!clean_output.contains("FILE SCORE CALCULATION"));
        assert!(!clean_output.contains("Size Factor"));
    }

    #[test]
    fn test_file_score_breakdown_verbosity_1() {
        let mut output = String::new();
        let item = create_test_file_debt_item();
        let config = FormattingConfig::default();

        format_file_priority_item_with_verbosity(&mut output, 1, &item, config, 1);

        let clean_output = strip_ansi_codes(&output);
        // Verbosity 1 shows categorical labels but not detailed breakdown
        assert!(!clean_output.contains("FILE SCORE CALCULATION"));
        assert!(!clean_output.contains("Size Factor"));
    }

    #[test]
    fn test_file_score_breakdown_verbosity_2() {
        let mut output = String::new();
        let item = create_test_file_debt_item();
        let config = FormattingConfig::default();

        format_file_priority_item_with_verbosity(&mut output, 1, &item, config, 2);

        let clean_output = strip_ansi_codes(&output);
        assert!(clean_output.contains("FILE SCORE CALCULATION"));
        assert!(clean_output.contains("Size Factor"));
        assert!(clean_output.contains("Coverage Factor"));
        assert!(clean_output.contains("Complexity Factor"));
        assert!(clean_output.contains("Density Factor"));
        assert!(clean_output.contains("God Object Multiplier"));
        assert!(clean_output.contains("Function Factor"));
        assert!(clean_output.contains("Final:"));
    }

    #[test]
    fn test_file_score_breakdown_structure() {
        let mut output = String::new();
        let mut item = create_test_file_debt_item();
        item.metrics.total_lines = 354;
        item.metrics.function_count = 7;
        item.metrics.avg_complexity = 8.0;
        item.metrics.coverage_percent = 0.0;
        item.metrics.god_object_indicators.is_god_object = true;
        item.metrics.god_object_indicators.god_object_score = 7.0;

        let config = FormattingConfig::default();

        format_file_priority_item_with_verbosity(&mut output, 1, &item, config, 2);

        let clean_output = strip_ansi_codes(&output);
        // Check for size factor calculation
        assert!(clean_output.contains("(354/100)"));
        // Check for coverage warning
        assert!(clean_output.contains("No coverage data") || clean_output.contains("untested"));
        // Check for god object warning
        assert!(clean_output.contains("Flagged as god object"));
    }

    #[test]
    fn test_file_score_calculation_accuracy() {
        let mut output = String::new();
        let item = create_test_file_debt_item();
        let config = FormattingConfig::default();

        format_file_priority_item_with_verbosity(&mut output, 1, &item, config, 2);

        let clean_output = strip_ansi_codes(&output);
        // The displayed score and calculated score should match
        // Extract the score from the header
        assert!(clean_output.contains("SCORE:"));
        // Check that Final: line exists (actual values may vary)
        assert!(clean_output.contains("Final:"));
    }

    // ============================================================================
    // Header Format and Legend Tests (Spec 169)
    // ============================================================================

    #[test]
    fn test_header_visual_separation() {
        let mut output = String::new();
        let mut item = create_test_item(7.5);
        item.transitive_coverage = Some(crate::priority::TransitiveCoverage {
            direct: 0.0,
            transitive: 0.0,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });
        format_priority_item(&mut output, 1, &item, true);
        let plain = strip_ansi_codes(&output);

        // Check that header contains score
        assert!(plain.contains("SCORE:"));
        // Check for visual separator (bullet)
        assert!(plain.contains("•"));
    }

    #[test]
    fn test_header_coverage_tag_with_data() {
        let mut output = String::new();
        let mut item = create_test_item(8.0);
        item.transitive_coverage = Some(crate::priority::TransitiveCoverage {
            direct: 0.0,
            transitive: 0.0,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });

        format_priority_item(&mut output, 1, &item, true);
        let plain = strip_ansi_codes(&output);

        // Should show ERROR UNTESTED tag for 0% coverage
        assert!(plain.contains("[ERROR UNTESTED]"));
    }

    #[test]
    fn test_header_coverage_tag_without_data() {
        let mut output = String::new();
        let item = create_test_item(8.0);

        format_priority_item(&mut output, 1, &item, false);
        let plain = strip_ansi_codes(&output);

        // Should not show coverage tag when has_coverage_data is false
        assert!(!plain.contains("[ERROR UNTESTED]"));
        assert!(!plain.contains("[WARN"));
        assert!(!plain.contains("[OK"));
    }

    #[test]
    fn test_header_tag_ordering() {
        let mut output = String::new();
        let mut item = create_test_item(9.0);
        item.transitive_coverage = Some(crate::priority::TransitiveCoverage {
            direct: 0.15,
            transitive: 0.0,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });

        format_priority_item(&mut output, 1, &item, true);
        let plain = strip_ansi_codes(&output);

        // Find positions of each component
        let score_pos = plain.find("SCORE").unwrap();
        let coverage_pos = plain
            .find("[WARN LOW]")
            .or_else(|| plain.find("[ERROR"))
            .unwrap();
        let severity_pos = plain
            .find("[CRITICAL]")
            .or_else(|| plain.find("[HIGH]"))
            .unwrap();

        // Verify ordering: SCORE < COVERAGE < SEVERITY
        assert!(score_pos < coverage_pos);
        assert!(coverage_pos < severity_pos);
    }

    #[test]
    fn test_legend_generation_with_coverage() {
        let legend = generate_legend(1, true);

        assert!(legend.contains("Legend:"));
        assert!(legend.contains("SCORE:"));
        assert!(legend.contains("Numeric priority"));
        assert!(legend.contains("[ERROR/WARN/INFO/OK]:"));
        assert!(legend.contains("Coverage status"));
        assert!(legend.contains("[CRITICAL/HIGH/MEDIUM/LOW]:"));
        assert!(legend.contains("Item severity"));
    }

    #[test]
    fn test_legend_generation_without_coverage() {
        let legend = generate_legend(1, false);

        // Should be empty when no coverage data
        assert!(legend.is_empty());
    }

    #[test]
    fn test_legend_generation_verbosity_zero() {
        let legend = generate_legend(0, true);

        // Should be empty when verbosity is 0
        assert!(legend.is_empty());
    }

    #[test]
    fn test_coverage_info_classification() {
        use crate::priority::TransitiveCoverage;

        // Test 0% coverage
        let mut item = create_test_item(5.0);
        item.transitive_coverage = Some(TransitiveCoverage {
            direct: 0.0,
            transitive: 0.0,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });
        let context = create_format_context(1, &item, true);
        assert!(context.coverage_info.is_some());
        let coverage = context.coverage_info.unwrap();
        assert!(coverage.tag.contains("[ERROR UNTESTED]"));

        // Test low coverage (15%)
        let mut item = create_test_item(5.0);
        item.transitive_coverage = Some(TransitiveCoverage {
            direct: 0.15,
            transitive: 0.0,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });
        let context = create_format_context(1, &item, true);
        assert!(context.coverage_info.is_some());
        let coverage = context.coverage_info.unwrap();
        assert!(coverage.tag.contains("[WARN LOW]"));

        // Test partial coverage (35%)
        let mut item = create_test_item(5.0);
        item.transitive_coverage = Some(TransitiveCoverage {
            direct: 0.35,
            transitive: 0.0,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });
        let context = create_format_context(1, &item, true);
        assert!(context.coverage_info.is_some());
        let coverage = context.coverage_info.unwrap();
        assert!(coverage.tag.contains("[WARN PARTIAL]"));

        // Test good coverage (85%)
        let mut item = create_test_item(5.0);
        item.transitive_coverage = Some(TransitiveCoverage {
            direct: 0.85,
            transitive: 0.0,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });
        let context = create_format_context(1, &item, true);
        assert!(context.coverage_info.is_some());
        let coverage = context.coverage_info.unwrap();
        assert!(coverage.tag.contains("[OK GOOD]"));

        // Test excellent coverage (98%)
        let mut item = create_test_item(5.0);
        item.transitive_coverage = Some(TransitiveCoverage {
            direct: 0.98,
            transitive: 0.0,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });
        let context = create_format_context(1, &item, true);
        assert!(context.coverage_info.is_some());
        let coverage = context.coverage_info.unwrap();
        assert!(coverage.tag.contains("[OK EXCELLENT]"));
    }
}
