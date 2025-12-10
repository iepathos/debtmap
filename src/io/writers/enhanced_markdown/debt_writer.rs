use crate::core::AnalysisResults;
use crate::priority::UnifiedAnalysis;
use anyhow::Result;
use std::io::Write;

use super::complexity_analyzer::*;
use super::executive_summary::estimate_effort_hours;
use super::formatters::*;
use super::statistics::*;

/// Write priority matrix for debt items
pub fn write_priority_matrix<W: Write>(writer: &mut W, analysis: &UnifiedAnalysis) -> Result<()> {
    writeln!(writer, "### Priority Matrix\n")?;

    writeln!(writer, "| Priority | Item | Effort | Impact |")?;
    writeln!(writer, "|----------|------|--------|--------|")?;

    for (i, item) in analysis.items.iter().take(10).enumerate() {
        writeln!(
            writer,
            "| {} | {} | {} days | {} |",
            get_priority_label(i),
            item.recommendation.primary_action.clone(),
            estimate_effort_hours(item) / 8,
            item.expected_impact.complexity_reduction
        )?;
    }

    writeln!(writer)?;
    Ok(())
}

/// Write debt categories breakdown
pub fn write_debt_categories<W: Write>(writer: &mut W, results: &AnalysisResults) -> Result<()> {
    writeln!(writer, "### Debt by Category\n")?;

    let categories = categorize_debt(&results.technical_debt.items);

    writeln!(writer, "| Category | Count | Severity |")?;
    writeln!(writer, "|----------|-------|----------|")?;

    for (category, items) in categories {
        let priorities: Vec<_> = items.iter().map(|i| i.priority).collect();
        writeln!(
            writer,
            "| {} | {} | {} |",
            category,
            items.len(),
            calculate_category_severity(&priorities)
        )?;
    }

    writeln!(writer)?;
    Ok(())
}

/// Write actionable items section
pub fn write_actionable_items<W: Write>(writer: &mut W, results: &AnalysisResults) -> Result<()> {
    writeln!(writer, "### Actionable Items\n")?;

    writeln!(writer, "#### Quick Wins (< 1 day effort)\n")?;

    for item in results
        .technical_debt
        .items
        .iter()
        .filter(|i| i.priority == crate::core::Priority::Low)
        .take(5)
    {
        writeln!(writer, "- [ ] {}", item.message)?;
    }

    writeln!(writer, "\n#### High Impact (1-3 days effort)\n")?;

    for item in results
        .technical_debt
        .items
        .iter()
        .filter(|i| i.priority == crate::core::Priority::Medium)
        .take(5)
    {
        writeln!(writer, "- [ ] {}", item.message)?;
    }

    writeln!(writer)?;
    Ok(())
}

/// Write dependency analysis section
pub fn write_dependency_analysis<W: Write>(
    writer: &mut W,
    analysis: &UnifiedAnalysis,
) -> Result<()> {
    writeln!(writer, "## Dependency Analysis\n")?;

    let items: Vec<_> = analysis.items.iter().cloned().collect();
    let deps = extract_module_dependencies(&items);

    writeln!(writer, "### Module Coupling\n")?;
    writeln!(writer, "| Module | Afferent | Efferent | Instability |")?;
    writeln!(writer, "|--------|----------|----------|-------------|")?;

    for (module, dependencies) in deps.iter().take(10) {
        let metrics = calculate_coupling_metrics(0, dependencies.len());
        writeln!(
            writer,
            "| {} | {} | {} | {:.2} |",
            module, metrics.afferent, metrics.efferent, metrics.instability
        )?;
    }

    writeln!(writer)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        ComplexityReport, ComplexitySummary, DebtItem, DebtType, DependencyReport, Priority,
        TechnicalDebtReport,
    };
    use crate::priority::{
        ActionableRecommendation, DebtType as UnifiedDebtType, FunctionRole, ImpactMetrics,
        Location, UnifiedDebtItem, UnifiedScore,
    };
    use chrono::Utc;
    use std::collections::HashMap;
    use std::io::Cursor;
    use std::path::PathBuf;

    fn create_test_analysis() -> UnifiedAnalysis {
        use crate::data_flow::DataFlowGraph;
        use crate::priority::call_graph::CallGraph;
        use crate::priority::score_types::Score0To100;
        use im::Vector;

        let items = Vector::from(vec![UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                function: "test_func".to_string(),
                line: 10,
            },
            debt_type: UnifiedDebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 20,
            },
            recommendation: ActionableRecommendation {
                primary_action: "Refactor complex function".to_string(),
                rationale: "High complexity".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                risk_reduction: 0.5,
                complexity_reduction: 0.3,
                coverage_improvement: 0.1,
                lines_reduction: 20,
            },
            unified_score: UnifiedScore {
                final_score: Score0To100::new(8.5),
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                coverage_factor: 1.0,
                complexity_factor: 1.5,
                dependency_factor: 1.2,
                role_multiplier: 1.0,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
            },
            upstream_dependencies: 2,
            downstream_dependencies: 5,
            cyclomatic_complexity: 15,
            cognitive_complexity: 20,
            nesting_depth: 4,
            function_length: 80,
            function_role: FunctionRole::PureLogic,
            transitive_coverage: None,
            file_context: None,
            upstream_callers: vec![],
            downstream_callees: vec![],
            entropy_details: None,
            entropy_adjusted_cognitive: None,
            entropy_dampening_factor: None,
            is_pure: Some(false),
            purity_confidence: Some(0.8),
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None, // spec 190
            detected_pattern: None,
            contextual_risk: None, // spec 203
            file_line_count: None,
            responsibility_category: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
        }]);

        UnifiedAnalysis {
            items,
            file_items: Vector::new(),
            total_impact: ImpactMetrics {
                risk_reduction: 0.0,
                complexity_reduction: 0.0,
                coverage_improvement: 0.0,
                lines_reduction: 0,
            },
            total_debt_score: 0.0,
            debt_density: 0.0,
            total_lines_of_code: 0,
            call_graph: CallGraph::new(),
            data_flow_graph: DataFlowGraph::new(),
            overall_coverage: None,
            has_coverage_data: false,
            timings: None,
            stats: crate::priority::FilterStatistics::new(),
        }
    }

    fn create_test_results_with_debt() -> AnalysisResults {
        AnalysisResults {
            project_path: PathBuf::from("/test"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                summary: ComplexitySummary {
                    total_functions: 5,
                    average_complexity: 10.0,
                    high_complexity_count: 2,
                    max_complexity: 15,
                },
                metrics: vec![],
            },
            technical_debt: TechnicalDebtReport {
                items: vec![
                    DebtItem {
                        id: "debt_1".to_string(),
                        file: PathBuf::from("test.rs"),
                        line: 10,
                        column: None,
                        message: "High complexity function".to_string(),
                        priority: Priority::High,
                        debt_type: DebtType::Complexity {
                            cyclomatic: 10,
                            cognitive: 8,
                        },
                        context: None,
                    },
                    DebtItem {
                        id: "debt_2".to_string(),
                        file: PathBuf::from("test.rs"),
                        line: 20,
                        column: None,
                        message: "Low test coverage".to_string(),
                        priority: Priority::Medium,
                        debt_type: DebtType::Complexity {
                            cyclomatic: 10,
                            cognitive: 8,
                        },
                        context: None,
                    },
                ],
                by_type: HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
            file_contexts: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_write_priority_matrix() {
        let analysis = create_test_analysis();
        let mut buffer = Cursor::new(Vec::new());

        let result = write_priority_matrix(&mut buffer, &analysis);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("Priority Matrix"));
        assert!(output.contains("Refactor complex function"));
    }

    #[test]
    fn test_write_debt_categories() {
        let results = create_test_results_with_debt();
        let mut buffer = Cursor::new(Vec::new());

        let result = write_debt_categories(&mut buffer, &results);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("Debt by Category"));
    }

    #[test]
    fn test_write_actionable_items() {
        let results = create_test_results_with_debt();
        let mut buffer = Cursor::new(Vec::new());

        let result = write_actionable_items(&mut buffer, &results);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("Actionable Items"));
        assert!(output.contains("Quick Wins"));
        assert!(output.contains("High Impact"));
    }
}
