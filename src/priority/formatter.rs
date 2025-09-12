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

fn format_file_aggregate_item(
    output: &mut String,
    rank: usize,
    item: &priority::FileAggregateScore,
    config: FormattingConfig,
) {
    let formatter = ColoredFormatter::new(config);
    let severity = if item.aggregate_score >= 300.0 {
        "CRITICAL"
    } else if item.aggregate_score >= 200.0 {
        "HIGH"
    } else if item.aggregate_score >= 100.0 {
        "MEDIUM"
    } else {
        "LOW"
    };

    let severity_color = get_severity_color(item.aggregate_score);

    // Header with rank and score
    writeln!(
        output,
        "#{} {} [{}]",
        rank.to_string().bright_cyan().bold(),
        format!(
            "SCORE: {}",
            score_formatter::format_score(item.aggregate_score)
        )
        .bright_yellow(),
        format!("{} - FILE AGGREGATE", severity)
            .color(severity_color)
            .bold()
    )
    .unwrap();

    // File path and stats
    writeln!(
        output,
        "├─ {} ({} functions, total score: {:.1})",
        item.file_path.display().to_string().bright_green(),
        item.function_count,
        item.total_score
    )
    .unwrap();

    // WHY section - explain what file aggregate means
    writeln!(
        output,
        "├─ {}: File aggregate combines complexity scores from {} individual functions to identify files with widespread technical debt. Unlike single file-level issues (god objects, high line count), this represents accumulated complexity across multiple functions. {} functions exceed complexity thresholds.",
        formatter.emoji("WHY", "WHY").bright_magenta(),
        item.function_count,
        item.problematic_functions
    )
    .unwrap();

    // ACTION section - Be specific about what to fix
    let top_functions = item.top_function_scores.len().min(2);
    let action_msg = if item.problematic_functions > 0 {
        format!(
            "Fix ONLY the top {} functions listed below. DO NOT refactor the entire file. Focus on functions with complexity > 10 or coverage < 60%",
            top_functions
        )
    } else {
        "No immediate action needed - monitor for future degradation".to_string()
    };

    writeln!(
        output,
        "├─ {}: {}",
        formatter.emoji("ACTION", "ACTION").bright_cyan(),
        action_msg
    )
    .unwrap();

    // Specific, prescriptive actions with concrete refactoring patterns
    if item.problematic_functions > 0 {
        writeln!(
            output,
            "│  ├─ {}. Fix ONLY these {} functions (listed in DEPENDENCIES below)",
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
            "│  └─ {}. DO NOT: Create new files, add abstraction layers, or refactor working code",
            4
        )
        .unwrap();
    }

    // IMPACT section
    writeln!(
        output,
        "├─ {}: Reduce overall file complexity by {}%, improve test coverage, enable safer refactoring",
        formatter.emoji("IMPACT", "IMPACT").bright_yellow(),
        ((item.problematic_functions as f64 / item.function_count as f64) * 100.0).round() as u32
    )
    .unwrap();

    // METRICS section
    writeln!(
        output,
        "├─ {}: Functions: {}, Problematic: {}, Avg complexity: {:.1}",
        formatter.emoji("METRICS", "METRICS").bright_blue(),
        item.function_count,
        item.problematic_functions,
        item.total_score / item.function_count as f64
    )
    .unwrap();

    // SCORING breakdown
    writeln!(
        output,
        "├─ {}: Aggregate: {} | Avg per function: {:.1} | Max: {:.1}",
        formatter.emoji("SCORING", "SCORING").bright_red(),
        severity,
        item.aggregate_score / item.function_count as f64,
        item.top_function_scores
            .first()
            .map(|(_, s)| *s)
            .unwrap_or(0.0)
    )
    .unwrap();

    // DEPENDENCIES - Top problematic functions
    writeln!(
        output,
        "└─ {}: {} high-complexity functions identified",
        formatter.emoji("DEPENDENCIES", "DEPS").bright_white(),
        item.problematic_functions
    )
    .unwrap();

    let issues_count = item.top_function_scores.len().min(5);
    for (i, (func_name, score)) in item.top_function_scores.iter().take(5).enumerate() {
        let prefix = if i == issues_count - 1 {
            "   └─"
        } else {
            "   ├─"
        };
        writeln!(output, "{} {}: {:.1}", prefix, func_name, score).unwrap();
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

    // Determine file type label based on characteristics
    let type_label = if item.metrics.god_object_indicators.is_god_object {
        // Distinguish between god objects (classes) and god modules (procedural files)
        if item.metrics.god_object_indicators.fields_count > 5 {
            "FILE - GOD OBJECT"  // Actual class with many fields
        } else {
            "FILE - GOD MODULE"  // Procedural file with many functions
        }
    } else if item.metrics.total_lines > 500 {
        "FILE - HIGH COMPLEXITY"
    } else {
        "FILE"
    };

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

    // Add WHY section
    let why_message = if item.metrics.god_object_indicators.is_god_object {
        if item.metrics.god_object_indicators.fields_count > 5 {
            format!(
                "This class violates single responsibility principle with {} methods, {} fields, and {} distinct responsibilities. High coupling and low cohesion make it difficult to maintain and test.",
                item.metrics.god_object_indicators.methods_count,
                item.metrics.god_object_indicators.fields_count,
                item.metrics.god_object_indicators.responsibilities
            )
        } else {
            format!(
                "This module contains {} functions in a single file, violating module cohesion principles. Large procedural modules are difficult to navigate, understand, and maintain.",
                item.metrics.function_count
            )
        }
    } else if item.metrics.total_lines > 500 {
        format!(
            "File exceeds recommended size with {} lines. Large files are harder to navigate, understand, and maintain. Consider breaking into smaller, focused modules.",
            item.metrics.total_lines
        )
    } else {
        "File exhibits high complexity that impacts maintainability and testability.".to_string()
    };

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

    // Add specific implementation steps based on file type
    if item.metrics.god_object_indicators.is_god_object {
        let file_name = item
            .metrics
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("module");

        // Provide specific steps based on whether it's a god object or god module
        if item.metrics.god_object_indicators.fields_count > 5 {
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
                item.metrics.path.display()
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

    // Add IMPACT section
    let impact = if item.metrics.god_object_indicators.is_god_object {
        format!(
            "Reduce complexity by {}%, improve testability, enable parallel development",
            ((item.metrics.total_lines as f64 / 200.0 - 1.0) * 100.0).min(80.0) as i32
        )
    } else {
        format!(
            "Improve maintainability, reduce file size by {}%",
            ((item.metrics.total_lines as f64 / 500.0 - 1.0) * 100.0).min(50.0) as i32
        )
    };

    writeln!(
        output,
        "{} {}",
        formatter.emoji("├─ IMPACT:", "└─ IMPACT:").bright_blue(),
        impact.bright_cyan()
    )
    .unwrap();

    // Add detailed metrics
    if item.metrics.god_object_indicators.is_god_object {
        writeln!(
            output,
            "{} Methods: {}, Fields: {}, Responsibilities: {}",
            formatter.emoji("├─ METRICS:", "└─ METRICS:").bright_blue(),
            item.metrics
                .god_object_indicators
                .methods_count
                .to_string()
                .yellow(),
            item.metrics
                .god_object_indicators
                .fields_count
                .to_string()
                .yellow(),
            item.metrics
                .god_object_indicators
                .responsibilities
                .to_string()
                .yellow()
        )
        .unwrap();
    }

    // Add SCORING breakdown
    writeln!(
        output,
        "{} File size: {} | Functions: {} | Complexity: HIGH",
        formatter.emoji("├─ SCORING:", "└─ SCORING:").bright_blue(),
        if item.metrics.total_lines > 1000 {
            "CRITICAL"
        } else if item.metrics.total_lines > 500 {
            "HIGH"
        } else {
            "MEDIUM"
        },
        if item.metrics.function_count > 50 {
            "EXCESSIVE"
        } else if item.metrics.function_count > 20 {
            "HIGH"
        } else {
            "MODERATE"
        }
    )
    .unwrap();

    // Add DEPENDENCIES if we have high function count
    if item.metrics.function_count > 10 {
        writeln!(
            output,
            "{} {} functions may have complex interdependencies",
            formatter
                .emoji("└─ DEPENDENCIES:", "└─ DEPENDENCIES:")
                .bright_blue(),
            item.metrics.function_count.to_string().cyan()
        )
        .unwrap();
    }
}

pub fn format_priority_item(output: &mut String, rank: usize, item: &UnifiedDebtItem) {
    let severity = get_severity_label(item.unified_score.final_score);
    let severity_color = get_severity_color(item.unified_score.final_score);

    writeln!(
        output,
        "#{} {} [{}]",
        rank.to_string().bright_cyan().bold(),
        format!(
            "SCORE: {}",
            score_formatter::format_score(item.unified_score.final_score)
        )
        .bright_yellow(),
        severity.color(severity_color).bold()
    )
    .unwrap();

    writeln!(
        output,
        "{} {}:{} {}()",
        "├─ LOCATION:".bright_blue(),
        item.location.file.display(),
        item.location.line,
        item.location.function.bright_green()
    )
    .unwrap();

    writeln!(
        output,
        "{} {}",
        "├─ ACTION:".bright_blue(),
        item.recommendation.primary_action.bright_green().bold()
    )
    .unwrap();

    writeln!(
        output,
        "{} {}",
        "├─ IMPACT:".bright_blue(),
        format_impact(&item.expected_impact).bright_cyan()
    )
    .unwrap();

    // Add complexity details with branch information
    let (cyclomatic, cognitive, branch_count, nesting, _length) = extract_complexity_info(item);
    if cyclomatic > 0 || cognitive > 0 {
        writeln!(
            output,
            "{} cyclomatic={}, branches={}, cognitive={}, nesting={}",
            "├─ COMPLEXITY:".bright_blue(),
            cyclomatic.to_string().yellow(),
            branch_count.to_string().yellow(),
            cognitive.to_string().yellow(),
            nesting.to_string().yellow()
        )
        .unwrap();
    }

    // Add dependency information with caller/callee names
    let (upstream, downstream) = extract_dependency_info(item);
    if upstream > 0 || downstream > 0 {
        writeln!(
            output,
            "{} {} upstream, {} downstream",
            "├─ DEPENDENCIES:".bright_blue(),
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
            writeln!(output, "│  ├─ CALLERS: {}", callers_display.cyan()).unwrap();
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
            writeln!(output, "│  └─ CALLS: {}", callees_display.bright_magenta()).unwrap();
        }
    }

    // Add dead code specific information
    if let DebtType::DeadCode {
        visibility,
        usage_hints,
        ..
    } = &item.debt_type
    {
        writeln!(
            output,
            "├─ VISIBILITY: {} function with no callers",
            format_visibility(visibility).yellow()
        )
        .unwrap();

        for hint in usage_hints {
            writeln!(output, "│  • {}", hint.bright_white()).unwrap();
        }
    }

    let formatter = ColoredFormatter::new(FormattingConfig::default());
    writeln!(
        output,
        "{} {}",
        formatter.emoji("└─ WHY:", "- WHY:").bright_blue(),
        item.recommendation.rationale
    )
    .unwrap();
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
    use crate::priority::call_graph::CallGraph;
    use crate::priority::unified_scorer::Location;
    use crate::priority::{ActionableRecommendation, ImpactMetrics, UnifiedScore};
    use std::path::PathBuf;

    fn strip_ansi_codes(s: &str) -> String {
        // Simple regex to strip ANSI escape codes
        let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
        re.replace_all(s, "").to_string()
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
}
