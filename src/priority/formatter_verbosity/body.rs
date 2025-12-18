use super::{complexity, context, coverage, git_history, sections};
use crate::formatting::{ColoredFormatter, FormattingConfig};
use crate::priority::classification::Severity;
use crate::priority::{score_formatter, UnifiedDebtItem};
use colored::*;
use std::fmt::Write;

/// Format the score header line with severity
fn format_score_header(
    rank: usize,
    score: f64,
    coverage_indicator: &str,
    severity: &str,
    severity_color: Color,
) -> String {
    format!(
        "#{} {}{} [{}]",
        rank.to_string().bright_cyan().bold(),
        format!("SCORE: {}", score_formatter::format_score(score)).bright_yellow(),
        coverage_indicator.bright_red().bold(),
        severity.color(severity_color).bold()
    )
}

/// Format the main factors line
fn format_main_factors(factors: &[String], _formatter: &ColoredFormatter) -> String {
    if factors.is_empty() {
        String::new()
    } else {
        format!(
            "   {} Main factors: {}",
            "  ",
            factors.join(", ").bright_white()
        )
    }
}

/// Pure function to collect main scoring factors
fn collect_scoring_factors(
    item: &UnifiedDebtItem,
    weights: &crate::config::ScoringWeights,
    has_coverage_data: bool,
) -> Vec<String> {
    let mut factors = vec![];

    // Coverage factor (50% weight in weighted sum model) - only if coverage data available
    if let Some(desc) =
        coverage::format_coverage_factor_description(item, weights, has_coverage_data)
    {
        factors.push(desc);
    }

    // Complexity factor (35% weight in weighted sum model)
    if item.unified_score.complexity_factor > 5.0 {
        factors.push("Complexity (weight: 35%)".to_string());
    } else if item.unified_score.complexity_factor > 3.0 {
        factors.push("Moderate complexity".to_string());
    }

    // Dependency factor (15% weight in weighted sum model)
    if item.unified_score.dependency_factor > 5.0 {
        factors.push("Critical path (weight: 15%)".to_string());
    }

    // Performance specific factors
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
        _ => {}
    }

    factors
}

/// Pure function to get coverage indicator
/// Note: Coverage is now shown in dedicated COVERAGE line, not in header
fn get_coverage_indicator(_item: &UnifiedDebtItem, _has_coverage_data: bool) -> &'static str {
    "" // Coverage shown in dedicated line below, not in header
}

/// Format the location section
fn format_location_section(output: &mut String, item: &UnifiedDebtItem) {
    writeln!(
        output,
        "├─ {} {}:{} {}()",
        "LOCATION:".bright_blue(),
        item.location.file.display(),
        item.location.line,
        item.location.function.bright_green()
    )
    .unwrap();
}

/// Format the impact section
fn format_impact_section(output: &mut String, item: &UnifiedDebtItem) {
    writeln!(
        output,
        "├─ {} {}",
        "IMPACT:".bright_blue(),
        crate::priority::formatter::format_impact(&item.expected_impact).bright_cyan()
    )
    .unwrap();
}

/// Format the why this matters section
fn format_why_section(output: &mut String, item: &UnifiedDebtItem) {
    writeln!(
        output,
        "{} {}",
        "├─ WHY THIS MATTERS:".bright_blue(),
        item.recommendation.rationale
    )
    .unwrap();
}

/// Format the recommended action section
fn format_action_section(output: &mut String, item: &UnifiedDebtItem) {
    writeln!(
        output,
        "├─ {} {}",
        "RECOMMENDED ACTION:".bright_blue(),
        item.recommendation.primary_action.bright_green().bold()
    )
    .unwrap();
}

/// Format the main body of the item (location, action, impact, etc.)
///
/// Composes focused section formatters following Stillwater philosophy:
/// each section is a single-responsibility function.
fn format_item_body(
    output: &mut String,
    item: &UnifiedDebtItem,
    formatter: &ColoredFormatter,
    verbosity: u8,
    tree_pipe: &str,
    has_coverage_data: bool,
) {
    // Location section (spec 139, 181)
    format_location_section(output, item);

    // Context dampening section (spec 191)
    context::format_context_dampening_section(output, item);

    // Impact section (before EVIDENCE per spec 139)
    format_impact_section(output, item);

    // Complexity section (acts as EVIDENCE)
    complexity::format_complexity_summary(output, item, formatter);

    // Pattern section (spec 190)
    complexity::format_pattern_detection(output, item);

    // Coverage section
    coverage::format_coverage_section(
        output,
        item,
        formatter,
        verbosity,
        tree_pipe,
        has_coverage_data,
    );

    // Git history section (spec 202)
    git_history::format_git_history_section(output, item);

    // Context provider contributions (verbose mode - spec 202)
    git_history::format_context_provider_contributions(output, item, verbosity);

    // File context section (spec 181)
    context::format_file_context_section(output, item, verbosity);

    // Why this matters section (spec 139)
    format_why_section(output, item);

    // Recommended action section (spec 139)
    format_action_section(output, item);

    // Implementation steps
    sections::format_implementation_steps(
        output,
        &item.recommendation.implementation_steps,
        formatter,
    );

    // Dependencies section
    sections::format_dependencies_summary(output, item, formatter, tree_pipe);

    // Call graph section
    sections::format_basic_call_graph(output, item, formatter);

    // Scoring breakdown (verbosity 1)
    if verbosity == 1 {
        sections::format_scoring_breakdown(output, item, formatter);
    }

    // Related items
    sections::format_related_items(output, &item.recommendation.related_items, formatter);

    // Pattern analysis (spec 151)
    sections::format_pattern_analysis(output, item, verbosity);
}

pub fn format_priority_item_with_config(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    verbosity: u8,
    config: FormattingConfig,
    has_coverage_data: bool,
) {
    let formatter = ColoredFormatter::new(config);
    let sev = Severity::from_score_100(item.unified_score.final_score.value());
    let severity = sev.as_str();
    let severity_color = sev.color();
    let tree_pipe = " ";

    // Format and write the score header
    let coverage_indicator = get_coverage_indicator(item, has_coverage_data);
    let score_header = format_score_header(
        rank,
        item.unified_score.final_score.value(),
        coverage_indicator,
        severity,
        severity_color,
    );
    writeln!(output, "{}", score_header).unwrap();

    // Add main factors for verbosity >= 1
    if verbosity >= 1 {
        let weights = crate::config::get_scoring_weights();
        let factors = collect_scoring_factors(item, weights, has_coverage_data);
        let factors_line = format_main_factors(&factors, &formatter);
        if !factors_line.is_empty() {
            writeln!(output, "{}", factors_line).unwrap();
        }
    }

    // Show detailed calculation for verbosity >= 2
    if verbosity >= 2 {
        // Format score calculation section
        let score_calc_lines = sections::format_score_calculation_section(item, &formatter);
        for line in score_calc_lines {
            writeln!(output, "{}", line).unwrap();
        }

        // Format complexity details section
        let complexity_lines = complexity::format_complexity_details_section(item, &formatter);
        for line in complexity_lines {
            writeln!(output, "{}", line).unwrap();
        }

        // Coverage details are shown in the coverage summary section below
        // (see format_coverage_summary -> format_detailed_coverage_analysis)

        // Format call graph section
        let call_graph_lines = sections::format_call_graph_section(item, &formatter);
        for line in call_graph_lines {
            writeln!(output, "{}", line).unwrap();
        }
    }

    // Format the rest of the item
    format_item_body(
        output,
        item,
        &formatter,
        verbosity,
        tree_pipe,
        has_coverage_data,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::{DebtType, UnifiedDebtItem, UnifiedScore};
    use std::path::PathBuf;

    #[test]
    fn test_format_score_header() {
        use colored::Color;

        // Disable colors for test to check plain text output
        colored::control::set_override(false);

        let header = format_score_header(1, 85.5, " [WARN]", "HIGH", Color::Red);
        assert!(header.contains("#1"));
        assert!(header.contains("SCORE:"));
        assert!(header.contains("[HIGH]"));

        // Re-enable colors
        colored::control::unset_override();
    }

    #[test]
    fn test_format_main_factors() {
        let formatter = ColoredFormatter::new(FormattingConfig::plain());

        // Test with empty factors
        let factors: Vec<String> = vec![];
        let result = format_main_factors(&factors, &formatter);
        assert_eq!(result, "");

        // Test with factors
        let factors = vec!["Factor1".to_string(), "Factor2".to_string()];
        let result = format_main_factors(&factors, &formatter);
        assert!(result.contains("Main factors:"));
        assert!(result.contains("Factor1, Factor2"));
    }

    #[test]
    fn test_collect_scoring_factors() {
        let weights = crate::config::ScoringWeights {
            complexity: 0.3,
            coverage: 0.5,
            dependency: 0.2,
            semantic: 0.0,
            security: 0.0,
            organization: 0.0,
        };

        // Test with no coverage data and high coverage factor
        let mut item = create_test_item();
        item.unified_score.coverage_factor = 10.0;
        item.unified_score.complexity_factor = 3.5;
        item.unified_score.dependency_factor = 1.0;

        let factors = collect_scoring_factors(&item, &weights, true); // has_coverage_data = true
        assert!(factors.iter().any(|f| f.contains("UNTESTED")));
        assert!(factors.iter().any(|f| f.contains("Moderate complexity")));

        // Test with nested loops debt type
        item.debt_type = DebtType::NestedLoops {
            depth: 3,
            complexity_estimate: "O(n^3)".to_string(),
        };
        let factors = collect_scoring_factors(&item, &weights, true);
        assert!(factors
            .iter()
            .any(|f| f.contains("Performance impact (High)")));
        assert!(factors.iter().any(|f| f.contains("3 level nested loops")));
    }

    // Helper function to create a test UnifiedDebtItem
    fn create_test_item() -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: crate::priority::Location {
                file: PathBuf::from("test.rs"),
                function: "test_function".to_string(),
                line: 100,
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 10,
                cognitive: 20,
            },
            unified_score: UnifiedScore {
                complexity_factor: 5.0,
                coverage_factor: 5.0,
                dependency_factor: 2.0,
                role_multiplier: 1.0,
                final_score: crate::priority::score_types::Score0To100::new(10.0),
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
                // Spec 260: Score transparency fields
                debt_adjustment: None,
                pre_normalization_score: None,
                structural_multiplier: Some(1.0),
                has_coverage_data: false,
                contextual_risk_multiplier: None,
                pre_contextual_score: None,
            },
            function_role: crate::priority::FunctionRole::Unknown,
            recommendation: crate::priority::ActionableRecommendation {
                primary_action: "Test action".to_string(),
                rationale: "Test rationale".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: crate::priority::ImpactMetrics {
                coverage_improvement: 0.5,
                lines_reduction: 10,
                complexity_reduction: 5.0,
                risk_reduction: 2.0,
            },
            transitive_coverage: None,
            file_context: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 2,
            function_length: 50,
            cyclomatic_complexity: 10,
            cognitive_complexity: 20,
            entropy_details: None,
            entropy_adjusted_cognitive: None,
            entropy_dampening_factor: None,
            is_pure: Some(false),
            purity_confidence: Some(0.5),
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None,
            detected_pattern: None,
            contextual_risk: None, // spec 203
            file_line_count: None,
            responsibility_category: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        }
    }
}
