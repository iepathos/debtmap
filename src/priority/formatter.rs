use crate::priority::{DebtType, FunctionRole, UnifiedAnalysis, UnifiedDebtItem};
use colored::*;
use std::fmt::Write;

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Default,        // Top 10 with clean formatting
    PrioritiesOnly, // Minimal list
    Detailed,       // Full analysis with priority overlay
    Top(usize),     // Top N items
}

pub fn format_priorities(analysis: &UnifiedAnalysis, format: OutputFormat) -> String {
    match format {
        OutputFormat::Default => format_default(analysis, 10),
        OutputFormat::PrioritiesOnly => format_priorities_only(analysis, 10),
        OutputFormat::Detailed => format_detailed(analysis),
        OutputFormat::Top(n) => format_default(analysis, n),
    }
}

fn format_default(analysis: &UnifiedAnalysis, limit: usize) -> String {
    let mut output = String::new();

    writeln!(output, "{}", "═".repeat(44).bright_blue()).unwrap();
    writeln!(
        output,
        "    {}",
        "PRIORITY TECHNICAL DEBT FIXES".bright_white().bold()
    )
    .unwrap();
    writeln!(output, "{}", "═".repeat(44).bright_blue()).unwrap();
    writeln!(output).unwrap();

    let top_items = analysis.get_top_priorities(limit);
    let count = top_items.len().min(limit);

    writeln!(
        output,
        "🎯 {} (by unified priority)",
        format!("TOP {count} RECOMMENDATIONS")
            .bright_yellow()
            .bold()
    )
    .unwrap();
    writeln!(output).unwrap();

    for (idx, item) in top_items.iter().enumerate() {
        if idx >= limit {
            break;
        }
        format_priority_item(&mut output, idx + 1, item);
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

    // Add high-impact low-complexity fixes section
    // _format_quick_wins(&mut output, analysis);

    // Add total impact summary
    // _format_total_impact(&mut output, analysis);

    output
}

fn format_priorities_only(analysis: &UnifiedAnalysis, limit: usize) -> String {
    let mut output = String::new();

    writeln!(output, "{}", "TOP PRIORITIES:".bright_white().bold()).unwrap();

    let top_items = analysis.get_top_priorities(limit);
    for (idx, item) in top_items.iter().enumerate() {
        if idx >= limit {
            break;
        }
        writeln!(
            output,
            "{}. {}: {}:{} {}()",
            idx + 1,
            get_action_verb(&item.debt_type),
            item.location.file.display(),
            item.location.line,
            item.location.function
        )
        .unwrap();
    }

    writeln!(output).unwrap();

    let critical_count = top_items
        .iter()
        .filter(|i| i.unified_score.final_score >= 8.0)
        .count();
    let high_count = top_items
        .iter()
        .filter(|i| i.unified_score.final_score >= 6.0 && i.unified_score.final_score < 8.0)
        .count();

    writeln!(
        output,
        "High-impact items: {critical_count} critical, {high_count} high priority"
    )
    .unwrap();
    writeln!(output, "Focus on measurable code quality improvements").unwrap();

    output
}

fn format_detailed(analysis: &UnifiedAnalysis) -> String {
    let mut output = String::new();

    writeln!(output, "{}", "═".repeat(44).bright_blue()).unwrap();
    writeln!(
        output,
        "    {}",
        "UNIFIED PRIORITY ANALYSIS".bright_white().bold()
    )
    .unwrap();
    writeln!(output, "{}", "═".repeat(44).bright_blue()).unwrap();
    writeln!(output).unwrap();

    for (idx, item) in analysis.items.iter().enumerate() {
        format_detailed_item(&mut output, idx + 1, item);
        writeln!(output).unwrap();
    }

    output
}

fn format_priority_item(output: &mut String, rank: usize, item: &UnifiedDebtItem) {
    let severity = get_severity_label(item.unified_score.final_score);
    let severity_color = get_severity_color(item.unified_score.final_score);

    writeln!(
        output,
        "#{} {} [{}]",
        rank.to_string().bright_cyan().bold(),
        format!("SCORE: {:.1}", item.unified_score.final_score).bright_white(),
        severity.color(severity_color).bold()
    )
    .unwrap();

    writeln!(
        output,
        "├─ {}: {}:{} {}()",
        format_debt_type(&item.debt_type).bright_yellow(),
        item.location.file.display(),
        item.location.line,
        item.location.function.bright_green()
    )
    .unwrap();

    writeln!(
        output,
        "├─ ACTION: {}",
        item.recommendation.primary_action.bright_white()
    )
    .unwrap();

    writeln!(
        output,
        "├─ IMPACT: {}",
        format_impact(&item.expected_impact).bright_cyan()
    )
    .unwrap();

    // Add complexity details with branch information
    let (cyclomatic, cognitive, branch_count, nesting, length) = extract_complexity_info(item);
    if cyclomatic > 0 || cognitive > 0 {
        writeln!(
            output,
            "├─ COMPLEXITY: {}, branches={}, cognitive={}, nesting={}, lines={}",
            format!("cyclomatic={cyclomatic}").dimmed(),
            branch_count.to_string().dimmed(),
            cognitive.to_string().dimmed(),
            nesting.to_string().dimmed(),
            length.to_string().dimmed()
        )
        .unwrap();
    }

    // Add dependency information if available
    let (upstream, downstream) = extract_dependency_info(item);
    if upstream > 0 || downstream > 0 {
        writeln!(
            output,
            "├─ DEPENDENCIES: {} upstream, {} downstream",
            upstream.to_string().dimmed(),
            downstream.to_string().dimmed()
        )
        .unwrap();
    }

    writeln!(output, "└─ WHY: {}", item.recommendation.rationale.dimmed()).unwrap();
}

fn format_detailed_item(output: &mut String, rank: usize, item: &UnifiedDebtItem) {
    writeln!(
        output,
        "#{} {}() - UNIFIED SCORE: {:.1}",
        rank,
        item.location.function.bright_green(),
        item.unified_score.final_score
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
        "│  ├─ ROI Factor: {:.1}",
        item.unified_score.roi_factor
    )
    .unwrap();
    writeln!(
        output,
        "│  └─ Semantic Factor: {:.1}",
        item.unified_score.semantic_factor
    )
    .unwrap();

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

fn _format_quick_wins(output: &mut String, analysis: &UnifiedAnalysis) {
    writeln!(output).unwrap();
    writeln!(
        output,
        "💡 {}",
        "HIGH-IMPACT LOW-COMPLEXITY FIXES".bright_yellow().bold()
    )
    .unwrap();

    // Find items with high impact but low complexity
    let quick_wins: Vec<_> = analysis
        .items
        .iter()
        .filter(|item| {
            item.unified_score.final_score >= 5.0 && item.unified_score.complexity_factor < 3.0
        })
        .take(3)
        .collect();

    for item in quick_wins {
        writeln!(
            output,
            "• {}: {}:{}",
            get_action_verb(&item.debt_type),
            item.location.file.display(),
            item.location.line
        )
        .unwrap();
    }
}

fn _format_total_impact(output: &mut String, analysis: &UnifiedAnalysis) {
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

fn format_impact(impact: &crate::priority::ImpactMetrics) -> String {
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

fn format_debt_type(debt_type: &DebtType) -> &'static str {
    match debt_type {
        DebtType::TestingGap { .. } => "TEST GAP",
        DebtType::ComplexityHotspot { .. } => "COMPLEXITY",
        DebtType::Orchestration { .. } => "ORCHESTRATION",
        DebtType::Duplication { .. } => "DUPLICATION",
        DebtType::Risk { .. } => "RISK",
        DebtType::TestComplexityHotspot { .. } => "TEST COMPLEXITY",
        DebtType::TestTodo { .. } => "TEST TODO",
        DebtType::TestDuplication { .. } => "TEST DUPLICATION",
    }
}

fn format_role(role: FunctionRole) -> &'static str {
    match role {
        FunctionRole::PureLogic => "PureLogic",
        FunctionRole::Orchestrator => "Orchestrator",
        FunctionRole::IOWrapper => "IOWrapper",
        FunctionRole::EntryPoint => "EntryPoint",
        FunctionRole::Unknown => "Unknown",
    }
}

fn get_action_verb(debt_type: &DebtType) -> &'static str {
    match debt_type {
        DebtType::TestingGap { .. } => "Add tests",
        DebtType::ComplexityHotspot { .. } => "Reduce complexity",
        DebtType::Orchestration { .. } => "Add integration test",
        DebtType::Duplication { .. } => "Extract duplication",
        DebtType::Risk { .. } => "Fix debt",
        DebtType::TestComplexityHotspot { .. } => "Simplify test",
        DebtType::TestTodo { .. } => "Complete TODO",
        DebtType::TestDuplication { .. } => "Remove test duplication",
    }
}

fn get_severity_label(score: f64) -> &'static str {
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

fn get_severity_color(score: f64) -> colored::Color {
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

fn extract_complexity_info(item: &UnifiedDebtItem) -> (u32, u32, u32, u32, usize) {
    let (cyclomatic, cognitive, branch_count) = match &item.debt_type {
        DebtType::TestingGap {
            cyclomatic,
            cognitive,
            ..
        } => {
            // For testing gaps, use both cyclomatic and cognitive
            (*cyclomatic, *cognitive, *cyclomatic)
        }
        DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive,
        } => (*cyclomatic, *cognitive, *cyclomatic),
        DebtType::TestComplexityHotspot {
            cyclomatic,
            cognitive,
            ..
        } => (*cyclomatic, *cognitive, *cyclomatic),
        _ => (0, 0, 0),
    };

    (
        cyclomatic,
        cognitive,
        branch_count,
        item.nesting_depth,
        item.function_length,
    )
}

fn extract_dependency_info(item: &UnifiedDebtItem) -> (usize, usize) {
    (item.upstream_dependencies, item.downstream_dependencies)
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
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                function: "test_func".to_string(),
                line: 10,
            },
            debt_type: DebtType::TestingGap {
                coverage: 0.1,
                cyclomatic: 5,
                cognitive: 7,
            },
            unified_score: UnifiedScore {
                complexity_factor: 5.0,
                coverage_factor: 8.0,
                roi_factor: 6.0,
                semantic_factor: 7.0,
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
            nesting_depth: 1,
            function_length: 15,
        }
    }

    #[test]
    fn test_format_priorities_only() {
        let mut analysis = UnifiedAnalysis::new(CallGraph::new());
        analysis.add_item(create_test_item(9.0));
        analysis.add_item(create_test_item(7.0));
        analysis.add_item(create_test_item(5.0));
        analysis.sort_by_priority();

        let output = format_priorities(&analysis, OutputFormat::PrioritiesOnly);
        let output_plain = strip_ansi_codes(&output);

        assert!(output_plain.contains("TOP PRIORITIES:"));
        assert!(output_plain.contains("1. Add tests"));
        assert!(output_plain.contains("High-impact items"));
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

        assert!(output_plain.contains("PRIORITY TECHNICAL DEBT FIXES"));
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
}
