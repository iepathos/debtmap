use crate::priority;
#[cfg(test)]
use crate::priority::UnifiedAnalysisUtils;
use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn output_json(
    analysis: &priority::UnifiedAnalysis,
    output_file: Option<PathBuf>,
) -> Result<()> {
    output_json_with_filters(analysis, None, None, output_file)
}

pub fn output_json_with_filters(
    analysis: &priority::UnifiedAnalysis,
    top: Option<usize>,
    tail: Option<usize>,
    output_file: Option<PathBuf>,
) -> Result<()> {
    output_json_with_format(analysis, top, tail, output_file, false)
}

pub fn output_json_with_format(
    analysis: &priority::UnifiedAnalysis,
    top: Option<usize>,
    tail: Option<usize>,
    output_file: Option<PathBuf>,
    include_scoring_details: bool,
) -> Result<()> {
    // Always use unified format (spec 202 - removed legacy format)
    let unified_output =
        crate::output::unified::convert_to_unified_format(analysis, include_scoring_details);

    // Apply filtering to unified output
    let filtered = apply_filters_to_unified_output(unified_output, top, tail);
    let json = serde_json::to_string_pretty(&filtered)?;

    if let Some(path) = output_file {
        if let Some(parent) = path.parent() {
            crate::io::ensure_dir(parent)?;
        }
        let mut file = fs::File::create(path)?;
        file.write_all(json.as_bytes())?;
    } else {
        println!("{json}");
    }
    Ok(())
}

fn apply_filters_to_unified_output(
    mut output: crate::output::unified::UnifiedOutput,
    top: Option<usize>,
    tail: Option<usize>,
) -> crate::output::unified::UnifiedOutput {
    if let Some(n) = top {
        output.items.truncate(n);
    } else if let Some(n) = tail {
        let total = output.items.len();
        let skip = total.saturating_sub(n);
        output.items = output.items.into_iter().skip(skip).collect();
    }

    // Update summary to reflect filtered items
    output.summary.total_items = output.items.len();
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::unified::{UnifiedDebtItemOutput, UnifiedOutput};
    use crate::priority::{
        call_graph::CallGraph, score_types::Score0To100, ActionableRecommendation, DebtType,
        FunctionRole, ImpactMetrics, Location, UnifiedDebtItem, UnifiedScore,
    };
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_item(name: &str, score: f64) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                line: 10,
                function: name.to_string(),
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 25,
            },
            unified_score: UnifiedScore {
                complexity_factor: 50.0,
                coverage_factor: 80.0,
                dependency_factor: 50.0,
                role_multiplier: 2.0,
                final_score: Score0To100::new(score),
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Fix issue".to_string(),
                rationale: "Test reason".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                complexity_reduction: 100.0,
                risk_reduction: 10.0,
                coverage_improvement: 100.0,
                lines_reduction: 500,
            },
            transitive_coverage: None,
            file_context: None,
            upstream_dependencies: 10,
            downstream_dependencies: 20,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 5,
            function_length: 200,
            cyclomatic_complexity: 25,
            cognitive_complexity: 40,
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
        }
    }

    fn create_test_analysis_with_items(count: usize) -> priority::UnifiedAnalysis {
        let call_graph = CallGraph::new();
        let mut analysis = priority::UnifiedAnalysis::new(call_graph);

        for i in 0..count {
            let mut item = create_test_item(&format!("func_{}", i), 100.0 - i as f64);
            // Give each item a unique line number to avoid duplicate detection
            item.location.line = 10 + i;
            analysis.add_item(item);
        }

        analysis.sort_by_priority();
        analysis
    }

    /// Helper to extract function name from unified output item
    fn get_function_name(item: &UnifiedDebtItemOutput) -> Option<String> {
        match item {
            UnifiedDebtItemOutput::Function(f) => f.location.function.clone(),
            UnifiedDebtItemOutput::File(_) => None,
        }
    }

    #[test]
    fn test_output_json_creates_parent_directories() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir
            .path()
            .join("nested")
            .join("subdirs")
            .join("output.json");

        let call_graph = CallGraph::new();
        let analysis = priority::UnifiedAnalysis::new(call_graph);

        let result = output_json(&analysis, Some(nested_path.clone()));
        assert!(
            result.is_ok(),
            "Failed to write JSON to nested path: {:?}",
            result.err()
        );
        assert!(
            nested_path.exists(),
            "Output file was not created at nested path"
        );

        let content = fs::read_to_string(&nested_path).unwrap();
        assert!(!content.is_empty(), "Output file is empty");
    }

    #[test]
    fn test_output_json_with_head_parameter() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.json");

        let analysis = create_test_analysis_with_items(10);

        // Test with head=3
        let result = output_json_with_filters(&analysis, Some(3), None, Some(output_path.clone()));
        assert!(result.is_ok(), "Failed to write JSON: {:?}", result.err());

        let content = fs::read_to_string(&output_path).unwrap();
        let parsed: UnifiedOutput = serde_json::from_str(&content).unwrap();

        assert_eq!(
            parsed.items.len(),
            3,
            "Expected 3 items with head=3, got {}",
            parsed.items.len()
        );

        // Verify we got the top 3 items (highest scores)
        assert_eq!(
            get_function_name(&parsed.items[0]),
            Some("func_0".to_string())
        );
        assert_eq!(
            get_function_name(&parsed.items[1]),
            Some("func_1".to_string())
        );
        assert_eq!(
            get_function_name(&parsed.items[2]),
            Some("func_2".to_string())
        );
    }

    #[test]
    fn test_output_json_with_tail_parameter() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.json");

        let analysis = create_test_analysis_with_items(10);

        // Test with tail=3
        let result = output_json_with_filters(&analysis, None, Some(3), Some(output_path.clone()));
        assert!(result.is_ok(), "Failed to write JSON: {:?}", result.err());

        let content = fs::read_to_string(&output_path).unwrap();
        let parsed: UnifiedOutput = serde_json::from_str(&content).unwrap();

        assert_eq!(
            parsed.items.len(),
            3,
            "Expected 3 items with tail=3, got {}",
            parsed.items.len()
        );

        // Verify we got the last 3 items (lowest scores)
        assert_eq!(
            get_function_name(&parsed.items[0]),
            Some("func_7".to_string())
        );
        assert_eq!(
            get_function_name(&parsed.items[1]),
            Some("func_8".to_string())
        );
        assert_eq!(
            get_function_name(&parsed.items[2]),
            Some("func_9".to_string())
        );
    }

    #[test]
    fn test_output_json_without_filters() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.json");

        let analysis = create_test_analysis_with_items(10);

        // Test without filters (should return all items)
        let result = output_json_with_filters(&analysis, None, None, Some(output_path.clone()));
        assert!(result.is_ok(), "Failed to write JSON: {:?}", result.err());

        let content = fs::read_to_string(&output_path).unwrap();
        let parsed: UnifiedOutput = serde_json::from_str(&content).unwrap();

        assert_eq!(
            parsed.items.len(),
            10,
            "Expected all 10 items without filters, got {}",
            parsed.items.len()
        );
    }

    #[test]
    fn test_output_json_head_larger_than_items() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.json");

        let analysis = create_test_analysis_with_items(5);

        // Test with head=10 when only 5 items exist
        let result = output_json_with_filters(&analysis, Some(10), None, Some(output_path.clone()));
        assert!(result.is_ok(), "Failed to write JSON: {:?}", result.err());

        let content = fs::read_to_string(&output_path).unwrap();
        let parsed: UnifiedOutput = serde_json::from_str(&content).unwrap();

        assert_eq!(
            parsed.items.len(),
            5,
            "Expected 5 items (all available), got {}",
            parsed.items.len()
        );
    }

    #[test]
    fn test_output_json_tail_larger_than_items() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.json");

        let analysis = create_test_analysis_with_items(5);

        // Test with tail=10 when only 5 items exist
        let result = output_json_with_filters(&analysis, None, Some(10), Some(output_path.clone()));
        assert!(result.is_ok(), "Failed to write JSON: {:?}", result.err());

        let content = fs::read_to_string(&output_path).unwrap();
        let parsed: UnifiedOutput = serde_json::from_str(&content).unwrap();

        assert_eq!(
            parsed.items.len(),
            5,
            "Expected 5 items (all available), got {}",
            parsed.items.len()
        );
    }

    #[test]
    fn test_output_json_includes_file_level_items() {
        use crate::priority::score_types::Score0To100;
        use crate::priority::{FileDebtItem, FileDebtMetrics, FileImpact};

        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.json");

        let call_graph = CallGraph::new();
        let mut analysis = priority::UnifiedAnalysis::new(call_graph);

        // Add function-level items with lower scores
        for i in 0..3 {
            let mut item = create_test_item(&format!("func_{}", i), 50.0 + i as f64);
            item.location.line = 10 + i;
            analysis.add_item(item);
        }

        // Add file-level items with higher scores (should appear first in output)
        let file_item = FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from("god_object.rs"),
                total_lines: 5530,
                function_count: 179,
                class_count: 0,
                avg_complexity: 25.0,
                max_complexity: 85,
                total_complexity: 4500,
                coverage_percent: 0.3,
                uncovered_lines: 3871,
                god_object_analysis: Some(crate::organization::GodObjectAnalysis {
                    method_count: 179,
                    weighted_method_count: None,
                    field_count: 20,
                    responsibility_count: 15,
                    is_god_object: true,
                    god_object_score: Score0To100::new(8500.0),
                    lines_of_code: 5533,
                    complexity_sum: 4500,
                    responsibilities: vec!["Too many responsibilities".to_string()],
                    responsibility_method_counts: Default::default(),
                    recommended_splits: vec![],
                    confidence: crate::organization::GodObjectConfidence::Definite,
                    purity_distribution: None,
                    module_structure: None,
                    detection_type: crate::organization::DetectionType::GodFile,
                    struct_name: None,
                    struct_line: None,
                    struct_location: None,
                    visibility_breakdown: None,
                    domain_count: 0,
                    domain_diversity: 0.0,
                    struct_ratio: 0.0,
                    analysis_method: crate::organization::SplitAnalysisMethod::None,
                    cross_domain_severity: None,
                    domain_diversity_metrics: None,
                    aggregated_entropy: None,
                    aggregated_error_swallowing_count: None,
                    aggregated_error_swallowing_patterns: None,
                    layering_impact: None,
                    anti_pattern_report: None,
                    complexity_metrics: None,   // Spec 211
                    trait_method_summary: None, // Spec 217
                }),
                function_scores: vec![],
                god_object_type: None,
                file_type: None,
                ..Default::default()
            },
            score: 606.0, // Higher than function items
            priority_rank: 1,
            recommendation: "Split this god object".to_string(),
            impact: FileImpact {
                complexity_reduction: 200.0,
                maintainability_improvement: 80.0,
                test_effort: 40.0,
            },
        };

        analysis.add_file_item(file_item);
        analysis.sort_by_priority();

        // Export to JSON
        let result = output_json_with_filters(&analysis, None, None, Some(output_path.clone()));
        assert!(result.is_ok(), "Failed to write JSON: {:?}", result.err());

        // Parse and verify using unified output format
        let content = fs::read_to_string(&output_path).unwrap();
        let parsed: UnifiedOutput = serde_json::from_str(&content).unwrap();

        // Should have 4 total items (3 function + 1 file)
        assert_eq!(
            parsed.items.len(),
            4,
            "Expected 4 items total (3 function + 1 file), got {}",
            parsed.items.len()
        );

        // File item with highest score should be first
        match &parsed.items[0] {
            UnifiedDebtItemOutput::File(file) => {
                assert_eq!(file.score, 606.0);
                assert_eq!(file.location.file, "god_object.rs");
            }
            _ => panic!("Expected first item to be a File debt item with highest score"),
        }

        // Remaining should be function items
        for i in 1..4 {
            match &parsed.items[i] {
                UnifiedDebtItemOutput::Function(_) => {
                    // This is expected
                }
                _ => panic!("Expected item {} to be a Function debt item", i),
            }
        }
    }
}
