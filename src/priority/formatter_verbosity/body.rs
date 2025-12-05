use super::{complexity, coverage, sections};
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

/// Format the main body of the item (location, action, impact, etc.)
fn format_item_body(
    output: &mut String,
    item: &UnifiedDebtItem,
    _formatter: &ColoredFormatter,
    verbosity: u8,
    tree_pipe: &str,
    has_coverage_data: bool,
) {
    // Location section (spec 139: tree formatting, spec 181: clean location line)
    writeln!(
        output,
        "├─ {} {}:{} {}()",
        "LOCATION:".bright_blue(),
        item.location.file.display(),
        item.location.line,
        item.location.function.bright_green()
    )
    .unwrap();

    // CONTEXT DAMPENING section (spec 191: show context-aware score dampening)
    if let (Some(multiplier), Some(file_type)) = (item.context_multiplier, item.context_type) {
        if multiplier < 1.0 {
            use crate::context::FileType;
            let description = match file_type {
                FileType::Example => "Example/demonstration code (pedagogical patterns accepted)",
                FileType::Test => "Test code (test helper complexity accepted)",
                FileType::Benchmark => "Benchmark code (performance test patterns accepted)",
                FileType::BuildScript => "Build script (build-time complexity accepted)",
                FileType::Documentation => "Documentation code (code example patterns accepted)",
                FileType::Production | FileType::Configuration => "Production code",
            };
            let dampening_percentage = ((1.0 - multiplier) * 100.0) as i32;
            writeln!(
                output,
                "├─ {} {} ({}% dampening applied)",
                "CONTEXT:".bright_blue(),
                description.bright_cyan(),
                dampening_percentage
            )
            .unwrap();
        }
    }

    // IMPACT section (before EVIDENCE per spec 139 ordering)
    writeln!(
        output,
        "├─ {} {}",
        "IMPACT:".bright_blue(),
        crate::priority::formatter::format_impact(&item.expected_impact).bright_cyan()
    )
    .unwrap();

    // COMPLEXITY section (acts as EVIDENCE - metrics only)
    complexity::format_complexity_summary(output, item, _formatter);

    // PATTERN section (spec 190: show detected state machine/coordinator patterns)
    complexity::format_pattern_detection(output, item);

    // COVERAGE section (show right after complexity for consistency)
    coverage::format_coverage_section(
        output,
        item,
        _formatter,
        verbosity,
        tree_pipe,
        has_coverage_data,
    );

    // GIT HISTORY section (spec 202: show contextual risk from git history)
    if let Some(ref contextual_risk) = item.contextual_risk {
        // Find the git_history context provider data
        if let Some(git_context) = contextual_risk
            .contexts
            .iter()
            .find(|c| c.provider == "git_history")
        {
            use crate::risk::context::ContextDetails;
            if let ContextDetails::Historical {
                change_frequency,
                bug_density,
                age_days,
                author_count,
            } = &git_context.details
            {
                writeln!(
                    output,
                    "├─ {} {:.1} changes/month, {:.1}% bugs, {} days old, {} authors",
                    "GIT HISTORY:".bright_blue(),
                    change_frequency,
                    bug_density * 100.0,
                    age_days,
                    author_count
                )
                .unwrap();

                // Show risk impact comparison
                let multiplier = contextual_risk.contextual_risk / contextual_risk.base_risk;
                writeln!(
                    output,
                    "│  └─ {} base_risk={:.1} → contextual_risk={:.1} ({:.1}x multiplier)",
                    "Risk Impact:".bright_cyan(),
                    contextual_risk.base_risk,
                    contextual_risk.contextual_risk,
                    multiplier
                )
                .unwrap();
            }
        }

        // CONTEXT PROVIDER CONTRIBUTIONS (verbose mode only - spec 202)
        if verbosity >= 1 && !contextual_risk.contexts.is_empty() {
            writeln!(
                output,
                "├─ {}",
                "Context Provider Contributions:".bright_blue()
            )
            .unwrap();

            for context in &contextual_risk.contexts {
                use crate::risk::context::ContextDetails;
                match &context.details {
                    ContextDetails::Historical {
                        change_frequency,
                        bug_density,
                        ..
                    } => {
                        let stability_desc = if *change_frequency > 5.0 {
                            "highly unstable"
                        } else if *change_frequency > 2.0 {
                            "moderately unstable"
                        } else {
                            "stable"
                        };
                        let bug_desc = if *bug_density > 0.3 {
                            "high"
                        } else if *bug_density > 0.1 {
                            "moderate"
                        } else {
                            "low"
                        };
                        writeln!(
                            output,
                            "│  └─ {}: +{:.1} impact (weight: {:.1})",
                            context.provider.bright_cyan(),
                            context.contribution,
                            context.weight
                        )
                        .unwrap();
                        writeln!(
                            output,
                            "│     - Change frequency: {:.1}/month ({})",
                            change_frequency, stability_desc
                        )
                        .unwrap();
                        writeln!(
                            output,
                            "│     - Bug density: {:.1}% ({})",
                            bug_density * 100.0,
                            bug_desc
                        )
                        .unwrap();
                    }
                    _ => {
                        // For other context types, show basic info
                        writeln!(
                            output,
                            "│  └─ {}: +{:.1} impact (weight: {:.1})",
                            context.provider.bright_cyan(),
                            context.contribution,
                            context.weight
                        )
                        .unwrap();
                    }
                }
            }
        }
    }

    // FILE CONTEXT section (spec 181: show non-production contexts in default mode)
    if verbosity == 0 {
        if let Some(ref context) = item.file_context {
            use crate::analysis::FileContext;
            use crate::priority::scoring::file_context_scoring::{
                context_label, context_reduction_factor,
            };

            // Only show non-production contexts in default mode
            if !matches!(context, FileContext::Production) {
                let reduction_pct = ((1.0 - context_reduction_factor(context)) * 100.0) as u32;
                writeln!(
                    output,
                    "├─ {} {} ({}% score reduction)",
                    "FILE CONTEXT:".bright_blue(),
                    context_label(context).bright_magenta(),
                    reduction_pct
                )
                .unwrap();
            }
        }
    }

    // WHY THIS MATTERS section (spec 139: explains why evidence matters)
    let why_label = "├─ WHY THIS MATTERS:".bright_blue();
    writeln!(output, "{} {}", why_label, item.recommendation.rationale).unwrap();

    // ACTION section (moved after WHY per spec 139)
    writeln!(
        output,
        "├─ {} {}",
        "RECOMMENDED ACTION:".bright_blue(),
        item.recommendation.primary_action.bright_green().bold()
    )
    .unwrap();

    // Implementation steps
    sections::format_implementation_steps(
        output,
        &item.recommendation.implementation_steps,
        _formatter,
    );

    // DEPENDENCIES section
    sections::format_dependencies_summary(output, item, _formatter, tree_pipe);

    // CALL GRAPH section - show basic info even at verbosity 0
    sections::format_basic_call_graph(output, item, _formatter);

    // SCORING breakdown for verbosity >= 1
    if (1..2).contains(&verbosity) {
        sections::format_scoring_breakdown(output, item, _formatter);
    }

    // RELATED items
    sections::format_related_items(output, &item.recommendation.related_items, _formatter);

    // PATTERN ANALYSIS section (spec 151)
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
    let sev = Severity::from_score(item.unified_score.final_score);
    let severity = sev.as_str();
    let severity_color = sev.color();
    let tree_pipe = " ";

    // Format and write the score header
    let coverage_indicator = get_coverage_indicator(item, has_coverage_data);
    let score_header = format_score_header(
        rank,
        item.unified_score.final_score,
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
                adjusted_cyclomatic: None,
            },
            unified_score: UnifiedScore {
                complexity_factor: 5.0,
                coverage_factor: 5.0,
                dependency_factor: 2.0,
                role_multiplier: 1.0,
                final_score: 10.0,
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
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
            contextual_risk: None,
        }
    }
}
