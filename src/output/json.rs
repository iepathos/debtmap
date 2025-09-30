use crate::priority;
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
    // Filter the analysis based on top/tail parameters
    let filtered_analysis = apply_filters(analysis, top, tail);

    let json = serde_json::to_string_pretty(&filtered_analysis)?;
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

fn apply_filters(
    analysis: &priority::UnifiedAnalysis,
    top: Option<usize>,
    tail: Option<usize>,
) -> priority::UnifiedAnalysis {
    // If both top and tail are None, return a clone of the original
    if top.is_none() && tail.is_none() {
        return analysis.clone();
    }

    let mut filtered = analysis.clone();

    // Apply filtering to items (UnifiedDebtItem)
    if let Some(n) = top {
        filtered.items = filtered.items.iter().take(n).cloned().collect();
    } else if let Some(n) = tail {
        let total = filtered.items.len();
        let skip = total.saturating_sub(n);
        filtered.items = filtered.items.iter().skip(skip).cloned().collect();
    }

    // Apply filtering to file_items (FileDebtItem)
    if let Some(n) = top {
        filtered.file_items = filtered.file_items.iter().take(n).cloned().collect();
    } else if let Some(n) = tail {
        let total = filtered.file_items.len();
        let skip = total.saturating_sub(n);
        filtered.file_items = filtered.file_items.iter().skip(skip).cloned().collect();
    }

    filtered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::{
        call_graph::CallGraph, ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics,
        Location, UnifiedDebtItem, UnifiedScore,
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
                final_score: score,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Fix issue".to_string(),
                rationale: "Test reason".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
            },
            expected_impact: ImpactMetrics {
                complexity_reduction: 100.0,
                risk_reduction: 10.0,
                coverage_improvement: 100.0,
                lines_reduction: 500,
            },
            transitive_coverage: None,
            upstream_dependencies: 10,
            downstream_dependencies: 20,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 5,
            function_length: 200,
            cyclomatic_complexity: 25,
            cognitive_complexity: 40,
            entropy_details: None,
            is_pure: Some(false),
            purity_confidence: Some(0.8),
            god_object_indicators: None,
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
        let parsed: priority::UnifiedAnalysis = serde_json::from_str(&content).unwrap();

        assert_eq!(
            parsed.items.len(),
            3,
            "Expected 3 items with head=3, got {}",
            parsed.items.len()
        );

        // Verify we got the top 3 items (highest scores)
        assert_eq!(parsed.items[0].location.function, "func_0");
        assert_eq!(parsed.items[1].location.function, "func_1");
        assert_eq!(parsed.items[2].location.function, "func_2");
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
        let parsed: priority::UnifiedAnalysis = serde_json::from_str(&content).unwrap();

        assert_eq!(
            parsed.items.len(),
            3,
            "Expected 3 items with tail=3, got {}",
            parsed.items.len()
        );

        // Verify we got the last 3 items (lowest scores)
        assert_eq!(parsed.items[0].location.function, "func_7");
        assert_eq!(parsed.items[1].location.function, "func_8");
        assert_eq!(parsed.items[2].location.function, "func_9");
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
        let parsed: priority::UnifiedAnalysis = serde_json::from_str(&content).unwrap();

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
        let parsed: priority::UnifiedAnalysis = serde_json::from_str(&content).unwrap();

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
        let parsed: priority::UnifiedAnalysis = serde_json::from_str(&content).unwrap();

        assert_eq!(
            parsed.items.len(),
            5,
            "Expected 5 items (all available), got {}",
            parsed.items.len()
        );
    }
}
