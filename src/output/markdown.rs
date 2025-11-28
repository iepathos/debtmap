use crate::formatting::FormattingConfig;
use crate::priority;
use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn output_markdown(
    analysis: &priority::UnifiedAnalysis,
    top: Option<usize>,
    tail: Option<usize>,
    verbosity: u8,
    output_file: Option<PathBuf>,
    formatting_config: FormattingConfig,
) -> Result<()> {
    // Filter the analysis based on top/tail parameters
    let filtered_analysis = apply_filters(analysis, top, tail);

    // Check if tiered display is enabled
    let display_config = crate::config::get_display_config();

    // Use a large limit since we've already filtered the analysis
    let limit = filtered_analysis
        .items
        .len()
        .max(filtered_analysis.file_items.len());

    let output = if display_config.tiered {
        priority::format_priorities_tiered_markdown(&filtered_analysis, limit, verbosity)
    } else {
        priority::format_priorities_markdown(&filtered_analysis, limit, verbosity, formatting_config)
    };

    if let Some(path) = output_file {
        if let Some(parent) = path.parent() {
            crate::io::ensure_dir(parent)?;
        }
        let mut file = fs::File::create(path)?;
        file.write_all(output.as_bytes())?;
    } else {
        println!("{output}");
    }
    Ok(())
}

fn apply_filters(
    analysis: &priority::UnifiedAnalysis,
    top: Option<usize>,
    tail: Option<usize>,
) -> priority::UnifiedAnalysis {
    // If both top and tail are None, use default of 10 items (legacy behavior)
    let (top, tail) = match (top, tail) {
        (None, None) => (Some(10), None),
        (t, tl) => (t, tl),
    };

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
        Location, UnifiedAnalysisUtils, UnifiedDebtItem, UnifiedScore,
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
                adjusted_cyclomatic: None,
            },
            unified_score: UnifiedScore {
                complexity_factor: 50.0,
                coverage_factor: 80.0,
                dependency_factor: 50.0,
                role_multiplier: 2.0,
                final_score: score,
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
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
    fn test_output_markdown_with_head_parameter() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.md");

        let analysis = create_test_analysis_with_items(10);

        // Test with head=3
        let result = output_markdown(&analysis, Some(3), None, 0, Some(output_path.clone()), FormattingConfig::default());
        assert!(
            result.is_ok(),
            "Failed to write markdown: {:?}",
            result.err()
        );

        let content = fs::read_to_string(&output_path).unwrap();

        // Verify the summary shows 3 items
        assert!(
            content.contains("**Total Debt Items:** 3"),
            "Expected 3 items in summary"
        );

        // The markdown formatter might group items, so just verify we have 3 items
        assert!(
            content.contains("3 items") || content.contains("Count: 3"),
            "Expected to find reference to 3 items in content"
        );
    }

    #[test]
    fn test_output_markdown_with_tail_parameter() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.md");

        let analysis = create_test_analysis_with_items(10);

        // Test with tail=3
        let result = output_markdown(&analysis, None, Some(3), 0, Some(output_path.clone()), FormattingConfig::default());
        assert!(
            result.is_ok(),
            "Failed to write markdown: {:?}",
            result.err()
        );

        let content = fs::read_to_string(&output_path).unwrap();

        // Verify the summary shows 3 items
        assert!(
            content.contains("**Total Debt Items:** 3"),
            "Expected 3 items in summary"
        );

        // The markdown formatter might group items, so just verify we have 3 items
        // and the count matches what we expect
        assert!(
            content.contains("3 items") || content.contains("Count: 3"),
            "Expected to find reference to 3 items in content"
        );
    }

    #[test]
    fn test_output_markdown_default_limit() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.md");

        let analysis = create_test_analysis_with_items(20);

        // Test without head/tail (should default to 10)
        let result = output_markdown(&analysis, None, None, 0, Some(output_path.clone()), FormattingConfig::default());
        assert!(
            result.is_ok(),
            "Failed to write markdown: {:?}",
            result.err()
        );

        let content = fs::read_to_string(&output_path).unwrap();

        // Verify the summary shows 10 items (default)
        assert!(
            content.contains("**Total Debt Items:** 10"),
            "Expected 10 items (default limit)"
        );
    }

    #[test]
    fn test_output_markdown_head_larger_than_items() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.md");

        let analysis = create_test_analysis_with_items(5);

        // Test with head=10 when only 5 items exist
        let result = output_markdown(&analysis, Some(10), None, 0, Some(output_path.clone()), FormattingConfig::default());
        assert!(
            result.is_ok(),
            "Failed to write markdown: {:?}",
            result.err()
        );

        let content = fs::read_to_string(&output_path).unwrap();

        // Verify the summary shows 5 items (all available)
        assert!(
            content.contains("**Total Debt Items:** 5"),
            "Expected 5 items (all available)"
        );
    }

    #[test]
    fn test_output_markdown_tail_larger_than_items() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.md");

        let analysis = create_test_analysis_with_items(5);

        // Test with tail=10 when only 5 items exist
        let result = output_markdown(&analysis, None, Some(10), 0, Some(output_path.clone()), FormattingConfig::default());
        assert!(
            result.is_ok(),
            "Failed to write markdown: {:?}",
            result.err()
        );

        let content = fs::read_to_string(&output_path).unwrap();

        // Verify the summary shows 5 items (all available)
        assert!(
            content.contains("**Total Debt Items:** 5"),
            "Expected 5 items (all available)"
        );
    }

    #[test]
    fn test_apply_filters_with_head() {
        let analysis = create_test_analysis_with_items(10);
        let filtered = apply_filters(&analysis, Some(3), None);

        assert_eq!(filtered.items.len(), 3, "Expected 3 items with head=3");
        assert_eq!(filtered.items[0].location.function, "func_0");
        assert_eq!(filtered.items[2].location.function, "func_2");
    }

    #[test]
    fn test_apply_filters_with_tail() {
        let analysis = create_test_analysis_with_items(10);
        let filtered = apply_filters(&analysis, None, Some(3));

        assert_eq!(filtered.items.len(), 3, "Expected 3 items with tail=3");
        assert_eq!(filtered.items[0].location.function, "func_7");
        assert_eq!(filtered.items[2].location.function, "func_9");
    }

    #[test]
    fn test_apply_filters_default_to_ten() {
        let analysis = create_test_analysis_with_items(20);
        let filtered = apply_filters(&analysis, None, None);

        assert_eq!(filtered.items.len(), 10, "Expected 10 items (default)");
    }
}
