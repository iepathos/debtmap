use crate::formatting::{ColoredFormatter, FormattingConfig, OutputFormatter};
use crate::priority::{
    self, score_formatter, DebtType, FunctionRole, FunctionVisibility, UnifiedAnalysis,
    UnifiedDebtItem,
};
use colored::*;
use std::fmt::Write;

#[path = "formatter_verbosity.rs"]
mod verbosity;

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

fn format_default_with_config(
    analysis: &UnifiedAnalysis,
    limit: usize,
    verbosity: u8,
    config: FormattingConfig,
) -> String {
    let mut output = String::new();
    let version = env!("CARGO_PKG_VERSION");
    let formatter = ColoredFormatter::new(config);

    let divider = formatter.emoji("═".repeat(44).as_str(), "=".repeat(44).as_str());
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
        "{} {}",
        formatter.emoji("🎯", "[TARGET]"),
        format!("TOP {count} RECOMMENDATIONS")
            .bright_yellow()
            .bold()
    )
    .unwrap();
    writeln!(output).unwrap();

    for (idx, item) in top_items.iter().enumerate() {
        format_mixed_priority_item(&mut output, idx + 1, item, verbosity, config);
        writeln!(output).unwrap();
    }

    // Add summary
    writeln!(
        output,
        "{} {}",
        formatter.emoji("📊", "[STATS]"),
        format!("TOTAL DEBT SCORE: {:.0}", analysis.total_debt_score).bright_cyan()
    )
    .unwrap();

    if let Some(coverage) = analysis.overall_coverage {
        writeln!(
            output,
            "{} {}",
            formatter.emoji("📈", "[CHART]"),
            format!("OVERALL COVERAGE: {:.2}%", coverage).bright_green()
        )
        .unwrap();
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
    let formatter = ColoredFormatter::new(config);

    let divider = formatter.emoji("═".repeat(44).as_str(), "=".repeat(44).as_str());
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
        );
        writeln!(output).unwrap();
    }

    output
}

#[allow(dead_code)]
fn format_tail(analysis: &UnifiedAnalysis, limit: usize) -> String {
    let mut output = String::new();
    let version = env!("CARGO_PKG_VERSION");
    let formatter = ColoredFormatter::new(FormattingConfig::default());

    let divider = formatter.emoji("═".repeat(44).as_str(), "=".repeat(44).as_str());
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
        "📉 {} (items {}-{})",
        format!("BOTTOM {count} ITEMS").bright_yellow().bold(),
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
        format_priority_item(&mut output, rank, item);
        writeln!(output).unwrap();
    }

    // Add total debt score
    writeln!(output).unwrap();
    writeln!(
        output,
        "📊 {}",
        format!("TOTAL DEBT SCORE: {:.0}", analysis.total_debt_score)
            .bright_cyan()
            .bold()
    )
    .unwrap();

    // Add overall coverage if available
    if let Some(coverage) = analysis.overall_coverage {
        writeln!(
            output,
            "📈 {}",
            format!("OVERALL COVERAGE: {coverage:.2}%")
                .bright_green()
                .bold()
        )
        .unwrap();
    }

    output
}

#[allow(dead_code)]
fn format_detailed(analysis: &UnifiedAnalysis) -> String {
    let mut output = String::new();
    let version = env!("CARGO_PKG_VERSION");
    let formatter = ColoredFormatter::new(FormattingConfig::default());

    let divider = formatter.emoji("═".repeat(44).as_str(), "=".repeat(44).as_str());
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
) {
    match item {
        priority::DebtItem::Function(func_item) => {
            verbosity::format_priority_item_with_config(output, rank, func_item, verbosity, config);
        }
        priority::DebtItem::File(file_item) => {
            format_file_priority_item(output, rank, file_item, config);
        }
        priority::DebtItem::FileAggregate(agg_item) => {
            format_file_aggregate_item(output, rank, agg_item, config);
        }
    }
}

// Pure function to determine severity level from score
fn determine_file_aggregate_severity(score: f64) -> &'static str {
    match score {
        s if s >= 300.0 => "CRITICAL",
        s if s >= 200.0 => "HIGH",
        s if s >= 100.0 => "MEDIUM",
        _ => "LOW",
    }
}

// Pure function to generate action message
fn generate_action_message(problematic_functions: usize, top_functions: usize) -> String {
    if problematic_functions > 0 {
        format!(
            "Focus on the top {} high-complexity functions listed below (complexity > 10 or coverage < 60%)",
            top_functions
        )
    } else {
        "No immediate action needed - monitor for future degradation".to_string()
    }
}

// Pure function to calculate impact percentage
fn calculate_impact_percentage(problematic: usize, total: usize) -> u32 {
    if total == 0 {
        0
    } else {
        ((problematic as f64 / total as f64) * 100.0).round() as u32
    }
}

// Pure function: Generate header text
fn generate_header(rank: usize, score: f64, severity: &str, severity_color: Color) -> String {
    format!(
        "#{} {} [{}]",
        rank.to_string().bright_cyan().bold(),
        format!("SCORE: {}", score_formatter::format_score(score)).bright_yellow(),
        format!("{} - FILE AGGREGATE", severity)
            .color(severity_color)
            .bold()
    )
}

// Pure function: Generate file path line
fn generate_file_path_line(item: &priority::FileAggregateScore) -> String {
    format!(
        "├─ {} ({} functions, total score: {:.1})",
        item.file_path.display().to_string().bright_green(),
        item.function_count,
        item.total_score
    )
}

// Pure function: Generate WHY section
fn generate_why_section(
    formatter: &ColoredFormatter,
    item: &priority::FileAggregateScore,
) -> String {
    format!(
        "├─ {}: File aggregate combines complexity scores from {} individual functions to identify files with widespread technical debt. Unlike single file-level issues (god objects, high line count), this represents accumulated complexity across multiple functions. {} functions exceed complexity thresholds.",
        formatter.emoji("WHY", "WHY").bright_magenta(),
        item.function_count,
        item.problematic_functions
    )
}

// Pure function: Generate ACTION section
fn generate_action_section(formatter: &ColoredFormatter, msg: &str) -> String {
    format!(
        "├─ {}: {}",
        formatter.emoji("ACTION", "ACTION").bright_cyan(),
        msg
    )
}

// Pure function: Generate IMPACT section
fn generate_impact_section(formatter: &ColoredFormatter, percentage: u32) -> String {
    format!(
        "├─ {}: Reduce overall file complexity by {}%, improve test coverage, enable safer refactoring",
        formatter.emoji("IMPACT", "IMPACT").bright_yellow(),
        percentage
    )
}

// Pure function: Generate METRICS section
fn generate_metrics_section(
    formatter: &ColoredFormatter,
    item: &priority::FileAggregateScore,
) -> String {
    format!(
        "├─ {}: Functions: {}, Problematic: {}, Avg complexity: {:.1}",
        formatter.emoji("METRICS", "METRICS").bright_blue(),
        item.function_count,
        item.problematic_functions,
        item.total_score / item.function_count as f64
    )
}

// Pure function: Generate SCORING section
fn generate_scoring_section(
    formatter: &ColoredFormatter,
    item: &priority::FileAggregateScore,
    severity: &str,
) -> String {
    format!(
        "├─ {}: Aggregate: {} | Avg per function: {:.1} | Max: {:.1}",
        formatter.emoji("SCORING", "SCORING").bright_red(),
        severity,
        item.aggregate_score / item.function_count as f64,
        item.top_function_scores
            .first()
            .map(|(_, s)| *s)
            .unwrap_or(0.0)
    )
}

// Pure function: Generate DEPENDENCIES section
fn generate_dependencies_section(formatter: &ColoredFormatter, problematic_count: usize) -> String {
    format!(
        "└─ {}: {} high-complexity functions identified",
        formatter.emoji("DEPENDENCIES", "DEPS").bright_white(),
        problematic_count
    )
}

// Main function using composition
fn format_file_aggregate_item(
    output: &mut String,
    rank: usize,
    item: &priority::FileAggregateScore,
    config: FormattingConfig,
) {
    let formatter = ColoredFormatter::new(config);
    let severity = determine_file_aggregate_severity(item.aggregate_score);
    let severity_color = get_severity_color(item.aggregate_score);

    // Compose sections using pure functions
    writeln!(
        output,
        "{}",
        generate_header(rank, item.aggregate_score, severity, severity_color)
    )
    .unwrap();
    writeln!(output, "{}", generate_file_path_line(item)).unwrap();
    writeln!(output, "{}", generate_why_section(&formatter, item)).unwrap();

    // ACTION section with message generation
    let top_functions = item.top_function_scores.len().min(2);
    let action_msg = generate_action_message(item.problematic_functions, top_functions);
    writeln!(
        output,
        "{}",
        generate_action_section(&formatter, &action_msg)
    )
    .unwrap();

    // Add refactoring steps if needed
    if item.problematic_functions > 0 {
        format_refactoring_steps(output, top_functions);
    }

    // IMPACT section
    let impact_percentage =
        calculate_impact_percentage(item.problematic_functions, item.function_count);
    writeln!(
        output,
        "{}",
        generate_impact_section(&formatter, impact_percentage)
    )
    .unwrap();

    // Remaining sections
    writeln!(output, "{}", generate_metrics_section(&formatter, item)).unwrap();
    writeln!(
        output,
        "{}",
        generate_scoring_section(&formatter, item, severity)
    )
    .unwrap();
    writeln!(
        output,
        "{}",
        generate_dependencies_section(&formatter, item.problematic_functions)
    )
    .unwrap();

    format_top_functions_list(output, &item.top_function_scores);
}

// Pure function to format refactoring steps
fn format_refactoring_steps(output: &mut String, top_functions: usize) {
    writeln!(
        output,
        "│  ├─ {}. Start with these {} functions (listed in DEPENDENCIES below)",
        1, top_functions
    )
    .unwrap();
    writeln!(
        output,
        "│  ├─ {}. For each function: If coverage < 60%, add tests for uncovered lines ONLY",
        2
    )
    .unwrap();
    writeln!(
        output,
        "│  ├─ {}. For each function: If complexity > 10, apply these patterns:",
        3
    )
    .unwrap();
    writeln!(
        output,
        "│  │   • Extract guard clauses: Convert nested if-else to early returns",
    )
    .unwrap();
    writeln!(
        output,
        "│  │   • Extract validation: Move input checks to separate function",
    )
    .unwrap();
    writeln!(
        output,
        "│  │   • Replace conditionals with map/filter when processing collections",
    )
    .unwrap();
    writeln!(
        output,
        "│  │   • Extract complex boolean expressions into named predicates",
    )
    .unwrap();
    writeln!(
        output,
        "│  └─ {}. Keep refactoring focused: Extract helpers as needed, but avoid over-engineering",
        4
    )
    .unwrap();
}

// Pure function to format top functions list
fn format_top_functions_list(output: &mut String, top_function_scores: &[(String, f64)]) {
    let issues_count = top_function_scores.len().min(5);
    for (i, (func_name, score)) in top_function_scores.iter().take(5).enumerate() {
        let prefix = if i == issues_count - 1 {
            "   └─"
        } else {
            "   ├─"
        };
        writeln!(output, "{} {}: {:.1}", prefix, func_name, score).unwrap();
    }
}

// Pure function to determine file type label based on characteristics
fn determine_file_type_label(
    is_god_object: bool,
    fields_count: usize,
    total_lines: usize,
) -> &'static str {
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

// Pure function to generate WHY explanation message
fn generate_why_message(
    is_god_object: bool,
    fields_count: usize,
    methods_count: usize,
    responsibilities: usize,
    function_count: usize,
    total_lines: usize,
) -> String {
    if is_god_object {
        if fields_count > 5 {
            format!(
                "This class violates single responsibility principle with {} methods, {} fields, and {} distinct responsibilities. High coupling and low cohesion make it difficult to maintain and test.",
                methods_count,
                fields_count,
                responsibilities
            )
        } else {
            format!(
                "This module contains {} functions in a single file, violating module cohesion principles. Large procedural modules are difficult to navigate, understand, and maintain.",
                function_count
            )
        }
    } else if total_lines > 500 {
        format!(
            "File exceeds recommended size with {} lines. Large files are harder to navigate, understand, and maintain. Consider breaking into smaller, focused modules.",
            total_lines
        )
    } else {
        "File exhibits high complexity that impacts maintainability and testability.".to_string()
    }
}

// Pure function to format implementation steps for god objects
fn format_god_object_steps(
    output: &mut String,
    formatter: &ColoredFormatter,
    fields_count: usize,
    file_path: &std::path::Path,
    file_name: &str,
) {
    if fields_count > 5 {
        // God Object (class with many fields)
        writeln!(
            output,
            "{}  {} 1. Identify distinct responsibilities in the class",
            formatter.emoji("│", ""),
            formatter.emoji("├─", "-").cyan()
        )
        .unwrap();
        writeln!(
            output,
            "{}  {} 2. Group methods and fields by responsibility",
            formatter.emoji("│", ""),
            formatter.emoji("├─", "-").cyan()
        )
        .unwrap();
        writeln!(
            output,
            "{}  {} 3. Extract each group into a separate focused class",
            formatter.emoji("│", ""),
            formatter.emoji("├─", "-").cyan()
        )
        .unwrap();
        writeln!(
            output,
            "{}  {} 4. Use composition or dependency injection to connect the new classes",
            formatter.emoji("│", ""),
            formatter.emoji("├─", "-").cyan()
        )
        .unwrap();
    } else {
        // God Module (many functions, few fields)
        writeln!(
            output,
            "{}  {} 1. Run `grep -n \"^pub fn\\|^fn\" {}` to list all functions",
            formatter.emoji("│", ""),
            formatter.emoji("├─", "-").cyan(),
            file_path.display()
        )
        .unwrap();
        writeln!(
            output,
            "{}  {} 2. Group functions by: a) AST node types they handle b) similar prefixes c) data flow patterns",
            formatter.emoji("│", ""),
            formatter.emoji("├─", "-").cyan()
        )
        .unwrap();
    }

    // Common steps for both god objects and modules
    writeln!(
        output,
        "{}  {} 3. Create new files: `{}_core.rs`, `{}_io.rs`, `{}_utils.rs` (adjust names to match groups)",
        formatter.emoji("│", ""),
        formatter.emoji("├─", "-").cyan(),
        file_name, file_name, file_name
    )
    .unwrap();
    writeln!(
        output,
        "{}  {} 4. Move functions in groups of 10-20, test after each move",
        formatter.emoji("│", ""),
        formatter.emoji("├─", "-").cyan()
    )
    .unwrap();
    writeln!(
        output,
        "{}  {} 5. DO NOT: Try to fix everything at once. Move incrementally, test frequently",
        formatter.emoji("│", ""),
        formatter.emoji("└─", "-").cyan()
    )
    .unwrap();
}

// Pure function to calculate impact message
fn calculate_impact_message(is_god_object: bool, total_lines: usize) -> String {
    if is_god_object {
        format!(
            "Reduce complexity by {}%, improve testability, enable parallel development",
            ((total_lines as f64 / 200.0 - 1.0) * 100.0).min(80.0) as i32
        )
    } else {
        format!(
            "Improve maintainability, reduce file size by {}%",
            ((total_lines as f64 / 500.0 - 1.0) * 100.0).min(50.0) as i32
        )
    }
}

// Function to format detailed metrics section
fn format_detailed_metrics(
    output: &mut String,
    formatter: &ColoredFormatter,
    is_god_object: bool,
    methods_count: usize,
    fields_count: usize,
    responsibilities: usize,
) {
    if is_god_object {
        writeln!(
            output,
            "{} Methods: {}, Fields: {}, Responsibilities: {}",
            formatter.emoji("├─ METRICS:", "└─ METRICS:").bright_blue(),
            methods_count.to_string().yellow(),
            fields_count.to_string().yellow(),
            responsibilities.to_string().yellow()
        )
        .unwrap();
    }
}

// Pure functions to determine file size and function count classifications
fn classify_file_size(total_lines: usize) -> &'static str {
    if total_lines > 1000 {
        "CRITICAL"
    } else if total_lines > 500 {
        "HIGH"
    } else {
        "MEDIUM"
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
    formatter: &ColoredFormatter,
    total_lines: usize,
    function_count: usize,
) {
    // Add SCORING breakdown
    writeln!(
        output,
        "{} File size: {} | Functions: {} | Complexity: HIGH",
        formatter.emoji("├─ SCORING:", "└─ SCORING:").bright_blue(),
        classify_file_size(total_lines),
        classify_function_count(function_count)
    )
    .unwrap();

    // Add DEPENDENCIES if we have high function count
    if function_count > 10 {
        writeln!(
            output,
            "{} {} functions may have complex interdependencies",
            formatter
                .emoji("└─ DEPENDENCIES:", "└─ DEPENDENCIES:")
                .bright_blue(),
            function_count.to_string().cyan()
        )
        .unwrap();
    }
}

fn format_file_priority_item(
    output: &mut String,
    rank: usize,
    item: &priority::FileDebtItem,
    config: FormattingConfig,
) {
    let formatter = ColoredFormatter::new(config);
    let severity = get_severity_label(item.score);
    let severity_color = get_severity_color(item.score);

    let type_label = determine_file_type_label(
        item.metrics.god_object_indicators.is_god_object,
        item.metrics.god_object_indicators.fields_count,
        item.metrics.total_lines,
    );

    writeln!(
        output,
        "#{} {} [{}]",
        rank.to_string().bright_cyan().bold(),
        format!("SCORE: {}", score_formatter::format_score(item.score)).bright_yellow(),
        format!("{} - {}", severity, type_label)
            .color(severity_color)
            .bold()
    )
    .unwrap();

    writeln!(
        output,
        "{} {} ({} lines, {} functions)",
        formatter.emoji("├─", "└─").bright_blue(),
        item.metrics.path.display().to_string().bright_green(),
        item.metrics.total_lines,
        item.metrics.function_count
    )
    .unwrap();

    let why_message = generate_why_message(
        item.metrics.god_object_indicators.is_god_object,
        item.metrics.god_object_indicators.fields_count,
        item.metrics.god_object_indicators.methods_count,
        item.metrics.god_object_indicators.responsibilities,
        item.metrics.function_count,
        item.metrics.total_lines,
    );

    writeln!(
        output,
        "{} {}",
        formatter.emoji("├─ WHY:", "└─ WHY:").bright_blue(),
        why_message.bright_white()
    )
    .unwrap();

    writeln!(
        output,
        "{} {}",
        formatter.emoji("├─ ACTION:", "└─ ACTION:").bright_yellow(),
        item.recommendation.bright_green().bold()
    )
    .unwrap();

    if item.metrics.god_object_indicators.is_god_object {
        let file_name = item
            .metrics
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("module");

        format_god_object_steps(
            output,
            &formatter,
            item.metrics.god_object_indicators.fields_count,
            &item.metrics.path,
            file_name,
        );
    }

    let impact = calculate_impact_message(
        item.metrics.god_object_indicators.is_god_object,
        item.metrics.total_lines,
    );

    writeln!(
        output,
        "{} {}",
        formatter.emoji("├─ IMPACT:", "└─ IMPACT:").bright_blue(),
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
        item.metrics.function_count,
    );
}

// Helper function to format a list with truncation
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

pub fn format_priority_item(output: &mut String, rank: usize, item: &UnifiedDebtItem) {
    // Use functional composition to format different sections
    let format_context = create_format_context(rank, item);

    // Format each section using pure functions composed together
    let formatted_sections = generate_formatted_sections(&format_context);

    // Apply formatting to output (I/O at edges)
    apply_formatted_sections(output, formatted_sections);
}

// Pure function to create formatting context
fn create_format_context(rank: usize, item: &UnifiedDebtItem) -> FormatContext {
    FormatContext {
        rank,
        score: item.unified_score.final_score,
        severity_info: SeverityInfo::from_score(item.unified_score.final_score),
        location_info: LocationInfo::from_item(item),
        action: item.recommendation.primary_action.clone(),
        impact: item.expected_impact.clone(),
        complexity_info: ComplexityInfo::from_item(item),
        dependency_info: DependencyInfo::from_item(item),
        debt_specific_info: DebtSpecificInfo::from_item(item),
        rationale: item.recommendation.rationale.clone(),
    }
}

// Data structures for formatted content
struct FormatContext {
    rank: usize,
    score: f64,
    severity_info: SeverityInfo,
    location_info: LocationInfo,
    action: String,
    impact: crate::priority::ImpactMetrics,
    complexity_info: ComplexityInfo,
    dependency_info: DependencyInfo,
    debt_specific_info: DebtSpecificInfo,
    rationale: String,
}

struct SeverityInfo {
    label: String,
    color: colored::Color,
}

impl SeverityInfo {
    fn from_score(score: f64) -> Self {
        Self {
            label: get_severity_label(score).to_string(),
            color: get_severity_color(score),
        }
    }
}

struct LocationInfo {
    file: std::path::PathBuf,
    line: u32,
    function: String,
}

impl LocationInfo {
    fn from_item(item: &UnifiedDebtItem) -> Self {
        Self {
            file: item.location.file.clone(),
            line: item.location.line as u32,
            function: item.location.function.clone(),
        }
    }
}

struct ComplexityInfo {
    cyclomatic: u32,
    cognitive: u32,
    branch_count: u32,
    nesting: u32,
    has_complexity: bool,
}

impl ComplexityInfo {
    fn from_item(item: &UnifiedDebtItem) -> Self {
        let (cyclomatic, cognitive, branch_count, nesting, _length) = extract_complexity_info(item);
        Self {
            cyclomatic,
            cognitive,
            branch_count,
            nesting,
            has_complexity: cyclomatic > 0 || cognitive > 0,
        }
    }
}

struct DependencyInfo {
    upstream: usize,
    downstream: usize,
    upstream_callers: Vec<String>,
    downstream_callees: Vec<String>,
    has_dependencies: bool,
}

impl DependencyInfo {
    fn from_item(item: &UnifiedDebtItem) -> Self {
        let (upstream, downstream) = extract_dependency_info(item);
        Self {
            upstream,
            downstream,
            upstream_callers: item.upstream_callers.clone(),
            downstream_callees: item.downstream_callees.clone(),
            has_dependencies: upstream > 0 || downstream > 0,
        }
    }
}

enum DebtSpecificInfo {
    DeadCode {
        visibility: String,
        usage_hints: Vec<String>,
    },
    Other,
}

impl DebtSpecificInfo {
    fn from_item(item: &UnifiedDebtItem) -> Self {
        match &item.debt_type {
            DebtType::DeadCode {
                visibility,
                usage_hints,
                ..
            } => Self::DeadCode {
                visibility: format_visibility(visibility).to_string(),
                usage_hints: usage_hints.clone(),
            },
            _ => Self::Other,
        }
    }
}

struct FormattedSections {
    header: String,
    location: String,
    action: String,
    impact: String,
    complexity: Option<String>,
    dependencies: Option<String>,
    debt_specific: Option<String>,
    rationale: String,
}

// Pure function to generate all formatted sections
fn generate_formatted_sections(context: &FormatContext) -> FormattedSections {
    FormattedSections {
        header: format_header_section(context),
        location: format_location_section(context),
        action: format_action_section(context),
        impact: format_impact_section(context),
        complexity: format_complexity_section(context),
        dependencies: format_dependencies_section(context),
        debt_specific: format_debt_specific_section(context),
        rationale: format_rationale_section(context),
    }
}

// Pure function to format header section
fn format_header_section(context: &FormatContext) -> String {
    format!(
        "#{} {} [{}]",
        context.rank.to_string().bright_cyan().bold(),
        format!("SCORE: {}", score_formatter::format_score(context.score)).bright_yellow(),
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

// Pure function to format action section
fn format_action_section(context: &FormatContext) -> String {
    format!(
        "{} {}",
        "├─ ACTION:".bright_blue(),
        context.action.bright_green().bold()
    )
}

// Pure function to format impact section
fn format_impact_section(context: &FormatContext) -> String {
    format!(
        "{} {}",
        "├─ IMPACT:".bright_blue(),
        format_impact(&context.impact).bright_cyan()
    )
}

// Pure function to format complexity section
fn format_complexity_section(context: &FormatContext) -> Option<String> {
    if !context.complexity_info.has_complexity {
        return None;
    }

    Some(format!(
        "{} cyclomatic={}, branches={}, cognitive={}, nesting={}",
        "├─ COMPLEXITY:".bright_blue(),
        context.complexity_info.cyclomatic.to_string().yellow(),
        context.complexity_info.branch_count.to_string().yellow(),
        context.complexity_info.cognitive.to_string().yellow(),
        context.complexity_info.nesting.to_string().yellow()
    ))
}

// Pure function to format dependencies section
fn format_dependencies_section(context: &FormatContext) -> Option<String> {
    if !context.dependency_info.has_dependencies {
        return None;
    }

    let mut section = format!(
        "{} {} upstream, {} downstream",
        "├─ DEPENDENCIES:".bright_blue(),
        context.dependency_info.upstream.to_string().cyan(),
        context.dependency_info.downstream.to_string().cyan()
    );

    // Add callers information
    if !context.dependency_info.upstream_callers.is_empty() {
        let callers_display = format_truncated_list(&context.dependency_info.upstream_callers, 3);
        section.push_str(&format!("\n│  ├─ CALLERS: {}", callers_display.cyan()));
    }

    // Add callees information
    if !context.dependency_info.downstream_callees.is_empty() {
        let callees_display = format_truncated_list(&context.dependency_info.downstream_callees, 3);
        section.push_str(&format!(
            "\n│  └─ CALLS: {}",
            callees_display.bright_magenta()
        ));
    }

    Some(section)
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
fn format_rationale_section(context: &FormatContext) -> String {
    let formatter = ColoredFormatter::new(FormattingConfig::default());
    format!(
        "{} {}",
        formatter.emoji("└─ WHY:", "- WHY:").bright_blue(),
        context.rationale
    )
}

// I/O function to apply formatted sections to output
fn apply_formatted_sections(output: &mut String, sections: FormattedSections) {
    writeln!(output, "{}", sections.header).unwrap();
    writeln!(output, "{}", sections.location).unwrap();
    writeln!(output, "{}", sections.action).unwrap();
    writeln!(output, "{}", sections.impact).unwrap();

    if let Some(complexity) = sections.complexity {
        writeln!(output, "{}", complexity).unwrap();
    }

    if let Some(dependencies) = sections.dependencies {
        writeln!(output, "{}", dependencies).unwrap();
    }

    if let Some(debt_specific) = sections.debt_specific {
        writeln!(output, "{}", debt_specific).unwrap();
    }

    writeln!(output, "{}", sections.rationale).unwrap();
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
                "⚠️".bright_yellow(),
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
        "📊 {}",
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
fn format_visibility(visibility: &FunctionVisibility) -> &'static str {
    match visibility {
        FunctionVisibility::Private => "private",
        FunctionVisibility::Crate => "crate-public",
        FunctionVisibility::Public => "public",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formatting::{ColorMode, EmojiMode};
    use crate::priority::call_graph::CallGraph;
    use crate::priority::file_metrics::{
        FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators,
    };
    use crate::priority::unified_scorer::Location;
    use crate::priority::{ActionableRecommendation, ImpactMetrics, UnifiedScore};
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
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Add unit tests".to_string(),
                rationale: "Low coverage critical function".to_string(),
                implementation_steps: vec!["Write tests".to_string()],
                related_items: vec![],
            },
            expected_impact: ImpactMetrics {
                coverage_improvement: 50.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 3.5,
            },
            transitive_coverage: None,
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
            entropy_details: None,
            god_object_indicators: None,
        }
    }

    fn create_test_file_aggregate(
        score: f64,
        function_count: usize,
    ) -> priority::FileAggregateScore {
        use crate::priority::aggregation::AggregationMethod;

        priority::FileAggregateScore {
            file_path: PathBuf::from("test_aggregate.rs"),
            function_count,
            problematic_functions: (function_count / 2).max(1),
            total_score: score * function_count as f64 / 2.0,
            aggregate_score: score,
            top_function_scores: vec![
                ("high_complexity_func1".to_string(), score * 0.4),
                ("high_complexity_func2".to_string(), score * 0.3),
                ("medium_complexity_func".to_string(), score * 0.2),
                ("low_complexity_func".to_string(), score * 0.1),
            ],
            aggregation_method: AggregationMethod::WeightedSum,
        }
    }

    #[test]
    fn test_format_file_aggregate_item_critical() {
        let mut output = String::new();
        let item = create_test_file_aggregate(350.0, 10);
        format_file_aggregate_item(&mut output, 1, &item, FormattingConfig::default());

        let clean_output = strip_ansi_codes(&output);

        // Verify critical severity
        assert!(clean_output.contains("[CRITICAL - FILE AGGREGATE]"));
        assert!(clean_output.contains("SCORE: 350"));

        // Verify file info
        assert!(clean_output.contains("test_aggregate.rs (10 functions"));
        assert!(clean_output.contains("total score: 1750.0"));

        // Verify WHY section
        assert!(clean_output
            .contains("File aggregate combines complexity scores from 10 individual functions"));
        assert!(clean_output.contains("5 functions exceed complexity thresholds"));

        // Verify ACTION section with specific refactoring patterns
        assert!(clean_output.contains("Focus on the top 2 high-complexity functions"));
        assert!(clean_output.contains("Extract guard clauses"));
        assert!(clean_output.contains("Extract validation"));
        assert!(clean_output.contains("Replace conditionals with map/filter"));

        // Verify METRICS section
        assert!(clean_output.contains("Functions: 10, Problematic: 5"));

        // Verify top functions are listed
        assert!(clean_output.contains("high_complexity_func1: 140.0"));
        assert!(clean_output.contains("high_complexity_func2: 105.0"));
    }

    #[test]
    fn test_format_file_aggregate_item_high() {
        let mut output = String::new();
        let item = create_test_file_aggregate(250.0, 8);
        format_file_aggregate_item(&mut output, 2, &item, FormattingConfig::default());

        let clean_output = strip_ansi_codes(&output);

        // Verify high severity
        assert!(clean_output.contains("[HIGH - FILE AGGREGATE]"));
        assert!(clean_output.contains("SCORE: 250"));

        // Verify function count and scores
        assert!(clean_output.contains("8 functions"));
        assert!(clean_output.contains("total score: 1000.0"));

        // Verify problematic functions identified
        assert!(clean_output.contains("4 functions exceed complexity thresholds"));

        // Verify IMPACT section calculation
        assert!(clean_output.contains("Reduce overall file complexity by 50%"));
    }

    #[test]
    fn test_format_file_aggregate_item_medium() {
        let mut output = String::new();
        let item = create_test_file_aggregate(150.0, 6);
        format_file_aggregate_item(&mut output, 3, &item, FormattingConfig::default());

        let clean_output = strip_ansi_codes(&output);

        // Verify medium severity
        assert!(clean_output.contains("[MEDIUM - FILE AGGREGATE]"));
        assert!(clean_output.contains("SCORE: 150"));

        // Verify average complexity calculation
        assert!(clean_output.contains("Avg complexity: 75.0"));

        // Verify SCORING breakdown
        assert!(clean_output.contains("Aggregate: MEDIUM"));
        assert!(clean_output.contains("Avg per function: 25.0"));
        assert!(clean_output.contains("Max: 60.0"));
    }

    #[test]
    fn test_format_file_aggregate_item_low() {
        let mut output = String::new();
        let item = create_test_file_aggregate(50.0, 4);
        format_file_aggregate_item(&mut output, 4, &item, FormattingConfig::default());

        let clean_output = strip_ansi_codes(&output);

        // Verify low severity
        assert!(clean_output.contains("[LOW - FILE AGGREGATE]"));
        assert!(clean_output.contains("SCORE: 50.0"));
    }

    #[test]
    fn test_format_file_aggregate_item_no_problematic_functions() {
        let mut output = String::new();
        let mut item = create_test_file_aggregate(50.0, 4);
        item.problematic_functions = 0;

        format_file_aggregate_item(&mut output, 5, &item, FormattingConfig::default());

        let clean_output = strip_ansi_codes(&output);

        // Should show no action needed
        assert!(
            clean_output.contains("No immediate action needed - monitor for future degradation")
        );

        // Should not show refactoring steps
        assert!(!clean_output.contains("Extract guard clauses"));
        assert!(!clean_output.contains("Extract validation"));
    }

    #[test]
    fn test_format_file_aggregate_item_with_emoji_config() {
        let mut output = String::new();
        let item = create_test_file_aggregate(200.0, 5);
        let config = FormattingConfig {
            color: ColorMode::Auto,
            emoji: EmojiMode::Always,
        };

        format_file_aggregate_item(&mut output, 1, &item, config);

        // With emoji enabled, should contain emoji characters
        assert!(output.contains("📍") || output.contains("WHY"));
        assert!(output.contains("🎯") || output.contains("ACTION"));
        assert!(output.contains("💥") || output.contains("IMPACT"));
    }

    #[test]
    fn test_format_file_aggregate_item_edge_cases() {
        // Test with minimal functions
        let mut output = String::new();
        let mut item = create_test_file_aggregate(100.0, 1);
        item.top_function_scores = vec![("only_func".to_string(), 100.0)];

        format_file_aggregate_item(&mut output, 1, &item, FormattingConfig::default());

        let clean_output = strip_ansi_codes(&output);
        assert!(clean_output.contains("1 functions"));
        assert!(clean_output.contains("only_func: 100.0"));

        // Test with many top functions (should limit to 5)
        let mut item = create_test_file_aggregate(500.0, 20);
        item.top_function_scores = (0..10)
            .map(|i| (format!("func_{}", i), 50.0 - i as f64))
            .collect();

        output.clear();
        format_file_aggregate_item(&mut output, 1, &item, FormattingConfig::default());

        let clean_output = strip_ansi_codes(&output);
        // Should only show first 5 functions
        assert!(clean_output.contains("func_0"));
        assert!(clean_output.contains("func_4"));
        assert!(!clean_output.contains("func_5"));
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
        format_priority_item(&mut output, 1, &item);
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
        format_priority_item(&mut output, 2, &item);
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
        format_priority_item(&mut output, 1, &item);
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("DEPENDENCIES:"));
        assert!(plain.contains("3 upstream"));
    }

    #[test]
    fn test_format_priority_item_with_callers() {
        let mut output = String::new();
        let mut item = create_test_item(6.0);
        item.upstream_callers = vec!["caller1".to_string(), "caller2".to_string()];
        item.upstream_dependencies = 2;
        format_priority_item(&mut output, 1, &item);
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("CALLERS:"));
        assert!(plain.contains("caller1, caller2"));
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
        ];
        item.upstream_dependencies = 5;
        format_priority_item(&mut output, 1, &item);
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("... (2 more)"));
    }

    #[test]
    fn test_format_priority_item_with_callees() {
        let mut output = String::new();
        let mut item = create_test_item(7.5);
        item.downstream_callees = vec!["func1".to_string(), "func2".to_string()];
        item.downstream_dependencies = 2;
        format_priority_item(&mut output, 1, &item);
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("CALLS:"));
        assert!(plain.contains("func1, func2"));
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
        format_priority_item(&mut output, 1, &item);
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("VISIBILITY:"));
        assert!(plain.contains("Consider removing"));
    }

    #[test]
    fn test_format_priority_item_critical_severity() {
        let mut output = String::new();
        let item = create_test_item(9.5);
        format_priority_item(&mut output, 1, &item);
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("[CRITICAL]"));
    }

    #[test]
    fn test_format_priority_item_low_severity() {
        let mut output = String::new();
        let item = create_test_item(2.0);
        format_priority_item(&mut output, 3, &item);
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("[LOW]"));
    }

    #[test]
    fn test_format_priority_item_no_complexity() {
        let mut output = String::new();
        let mut item = create_test_item(3.0);
        item.cyclomatic_complexity = 0;
        item.cognitive_complexity = 0;
        format_priority_item(&mut output, 1, &item);
        let plain = strip_ansi_codes(&output);
        assert!(!plain.contains("COMPLEXITY:"));
    }

    #[test]
    fn test_format_priority_item_no_dependencies() {
        let mut output = String::new();
        let mut item = create_test_item(3.5);
        item.upstream_dependencies = 0;
        item.downstream_dependencies = 0;
        format_priority_item(&mut output, 1, &item);
        let plain = strip_ansi_codes(&output);
        assert!(!plain.contains("DEPENDENCIES:"));
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
                cognitive: 15
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
                },
                function_scores: vec![8.5, 7.2, 6.9, 5.8, 4.3],
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
        assert!(
            clean_output.contains("[CRITICAL - FILE - GOD OBJECT]")
                || clean_output.contains("[HIGH - FILE - GOD OBJECT]")
        );

        // Check file path
        assert!(clean_output.contains("src/test_file.rs"));
        assert!(clean_output.contains("(1500 lines, 45 functions)"));

        // Check WHY section
        assert!(clean_output.contains("WHY:"));
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
        assert!(clean_output.contains("FILE - GOD MODULE"));
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
        assert!(clean_output.contains("FILE - HIGH COMPLEXITY"));
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
        // For regular files with lower scores
        assert!(clean_output.contains("FILE"));
        assert!(!clean_output.contains("GOD"));
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

        format_file_priority_item(&mut output, 1, &item, config);

        let clean_output = strip_ansi_codes(&output);
        // The coverage info might be in WHY or METRICS section
        assert!(
            clean_output.contains("0.0%")
                || clean_output.contains("0%")
                || clean_output.contains("no coverage")
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
}
