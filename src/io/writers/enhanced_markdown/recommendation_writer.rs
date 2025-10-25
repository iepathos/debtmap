use crate::core::AnalysisResults;
use crate::priority::UnifiedAnalysis;
use anyhow::Result;
use std::io::Write;

use super::executive_summary::estimate_effort_hours;
use super::statistics::calculate_average_complexity;

/// Write priority actions section
pub fn write_priority_actions<W: Write>(
    writer: &mut W,
    unified_analysis: Option<&UnifiedAnalysis>,
) -> Result<()> {
    writeln!(writer, "### 🚨 Priority Actions\n")?;

    if let Some(analysis) = unified_analysis {
        let priority_items: Vec<_> = analysis
            .items
            .iter()
            .take(3)
            .enumerate()
            .flat_map(|(i, item)| {
                vec![
                    format!("{}. **{}**", i + 1, item.recommendation.primary_action),
                    format!("   - Location: `{}`", item.location.file.display()),
                    format!(
                        "   - Estimated Effort: {} hours",
                        estimate_effort_hours(item)
                    ),
                ]
            })
            .collect();

        for line in priority_items {
            writeln!(writer, "{}", line)?;
        }

        if !analysis.items.is_empty() {
            writeln!(writer)?;
        }
    }

    Ok(())
}

/// Generate strategic recommendations based on metrics
fn generate_strategic_recommendations(avg_complexity: f64, debt_count: usize) -> Vec<String> {
    let mut recommendations = Vec::new();
    let mut index = 1;

    if avg_complexity > 10.0 {
        recommendations.push(format!(
            "{}. **Reduce Complexity**: Implement code review process focusing on cyclomatic complexity",
            index
        ));
        index += 1;
    }

    if debt_count > 50 {
        recommendations.push(format!(
            "{}. **Debt Reduction Sprint**: Allocate 20% of sprint capacity to debt reduction",
            index
        ));
    }

    recommendations
}

/// Write strategic recommendations section
pub fn write_strategic_recommendations<W: Write>(
    writer: &mut W,
    results: &AnalysisResults,
) -> Result<()> {
    writeln!(writer, "### 📋 Strategic Recommendations\n")?;

    let avg_complexity = calculate_average_complexity(results);
    let debt_count = results.technical_debt.items.len();

    let recommendations = generate_strategic_recommendations(avg_complexity, debt_count);

    for recommendation in recommendations {
        writeln!(writer, "{}", recommendation)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ComplexityReport, ComplexitySummary, DependencyReport, TechnicalDebtReport};
    use crate::priority::{
        ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics, Location, UnifiedAnalysis,
        UnifiedDebtItem, UnifiedScore,
    };
    use chrono::Utc;
    use std::collections::HashMap;
    use std::io::Cursor;
    use std::path::PathBuf;

    fn create_test_analysis() -> UnifiedAnalysis {
        use crate::data_flow::DataFlowGraph;
        use crate::priority::call_graph::CallGraph;
        use im::Vector;

        let items = Vector::from(vec![UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                function: "test_func".to_string(),
                line: 10,
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 20,
            },
            recommendation: ActionableRecommendation {
                primary_action: "Refactor complex function".to_string(),
                rationale: "High complexity".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
            },
            expected_impact: ImpactMetrics {
                risk_reduction: 0.5,
                complexity_reduction: 0.3,
                coverage_improvement: 0.1,
                lines_reduction: 20,
            },
            unified_score: UnifiedScore {
                final_score: 8.5,
                pre_adjustment_score: None,
                adjustment_applied: None,
                coverage_factor: 1.0,
                complexity_factor: 1.5,
                dependency_factor: 1.2,
                role_multiplier: 1.0,
            },
            upstream_dependencies: 2,
            downstream_dependencies: 5,
            cyclomatic_complexity: 15,
            cognitive_complexity: 20,
            nesting_depth: 4,
            function_length: 80,
            function_role: FunctionRole::PureLogic,
            transitive_coverage: None,
            upstream_callers: vec![],
            downstream_callees: vec![],
            entropy_details: None,
            is_pure: Some(false),
            purity_confidence: Some(0.8),
            god_object_indicators: None,
            tier: None,
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
        }
    }

    fn create_test_results(avg_complexity: f64, debt_count: usize) -> AnalysisResults {
        use crate::core::{DebtItem, DebtType, FunctionMetrics, Priority};

        let metrics: Vec<FunctionMetrics> = (0..10)
            .map(|i| FunctionMetrics {
                file: PathBuf::from(format!("test_{}.rs", i)),
                name: format!("test_function_{}", i),
                line: i * 10,
                cyclomatic: avg_complexity as u32,
                cognitive: (avg_complexity * 1.5) as u32,
                nesting: (i % 4) as u32,
                length: 20 + i * 10,
                is_test: false,
                is_pure: Some(i % 2 == 0),
                purity_confidence: Some(0.8),
                visibility: Some("pub".to_string()),
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            })
            .collect();

        AnalysisResults {
            project_path: PathBuf::from("/test"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                summary: ComplexitySummary {
                    total_functions: 10,
                    average_complexity: avg_complexity,
                    high_complexity_count: 3,
                    max_complexity: 20,
                },
                metrics,
            },
            technical_debt: TechnicalDebtReport {
                items: (0..debt_count)
                    .map(|i| DebtItem {
                        id: format!("debt_{}", i),
                        file: PathBuf::from("test.rs"),
                        line: i * 10,
                        column: None,
                        message: "Test debt".to_string(),
                        priority: Priority::Medium,
                        debt_type: DebtType::Complexity,
                        context: None,
                    })
                    .collect(),
                by_type: HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        }
    }

    #[test]
    fn test_write_priority_actions() {
        let analysis = create_test_analysis();
        let mut buffer = Cursor::new(Vec::new());

        let result = write_priority_actions(&mut buffer, Some(&analysis));
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("Priority Actions"));
        assert!(output.contains("Refactor complex function"));
    }

    #[test]
    fn test_generate_strategic_recommendations_high_complexity() {
        let recommendations = generate_strategic_recommendations(15.0, 10);
        assert_eq!(recommendations.len(), 1);
        assert!(recommendations[0].contains("Reduce Complexity"));
    }

    #[test]
    fn test_generate_strategic_recommendations_high_debt() {
        let recommendations = generate_strategic_recommendations(5.0, 60);
        assert_eq!(recommendations.len(), 1);
        assert!(recommendations[0].contains("Debt Reduction Sprint"));
    }

    #[test]
    fn test_generate_strategic_recommendations_both() {
        let recommendations = generate_strategic_recommendations(15.0, 60);
        assert_eq!(recommendations.len(), 2);
        assert!(recommendations[0].contains("Reduce Complexity"));
        assert!(recommendations[1].contains("Debt Reduction Sprint"));
    }

    #[test]
    fn test_generate_strategic_recommendations_none() {
        let recommendations = generate_strategic_recommendations(5.0, 10);
        assert_eq!(recommendations.len(), 0);
    }

    #[test]
    fn test_write_strategic_recommendations() {
        let results = create_test_results(15.0, 60);
        let mut buffer = Cursor::new(Vec::new());

        let result = write_strategic_recommendations(&mut buffer, &results);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("Strategic Recommendations"));
        assert!(output.contains("Reduce Complexity"));
        assert!(output.contains("Debt Reduction Sprint"));
    }
}
