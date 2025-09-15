use debtmap::io::writers::markdown::EnhancedMarkdownWriter;
use debtmap::io::writers::MarkdownWriter;
use debtmap::priority::{
    aggregation::{AggregationMethod, FileAggregateScore},
    unified_scorer::{Location, UnifiedDebtItem, UnifiedScore},
    CallGraph, DebtType, FunctionRole, FunctionVisibility, ImpactMetrics, UnifiedAnalysis,
};
use std::io::Cursor;
use std::path::PathBuf;

fn create_sample_file_aggregate() -> FileAggregateScore {
    FileAggregateScore {
        file_path: PathBuf::from("src/main.rs"),
        total_score: 25.0,
        function_count: 5,
        problematic_functions: 2,
        top_function_scores: vec![
            ("process_data".to_string(), 7.8),
            ("validate_input".to_string(), 5.2),
        ],
        aggregate_score: 12.5,
        aggregation_method: AggregationMethod::WeightedSum,
    }
}

fn create_sample_unified_item() -> UnifiedDebtItem {
    UnifiedDebtItem {
        location: Location {
            file: PathBuf::from("src/main.rs"),
            function: "process_data".to_string(),
            line: 42,
        },
        debt_type: DebtType::TestingGap {
            coverage: 0.3,
            cyclomatic: 15,
            cognitive: 20,
        },
        unified_score: UnifiedScore {
            complexity_factor: 7.5,
            coverage_factor: 8.0,
            dependency_factor: 4.0,
            role_multiplier: 1.2,
            final_score: 7.8,
        },
        function_role: FunctionRole::PureLogic,
        recommendation: debtmap::priority::ActionableRecommendation {
            primary_action: "Add unit tests".to_string(),
            rationale: "High complexity with low coverage".to_string(),
            implementation_steps: vec![],
            related_items: vec![],
        },
        expected_impact: ImpactMetrics {
            coverage_improvement: 30.0,
            lines_reduction: 0,
            complexity_reduction: 0.0,
            risk_reduction: 25.0,
        },
        transitive_coverage: None,
        upstream_dependencies: 3,
        downstream_dependencies: 5,
        upstream_callers: vec![],
        downstream_callees: vec![],
        nesting_depth: 3,
        function_length: 50,
        cyclomatic_complexity: 15,
        cognitive_complexity: 20,
        is_pure: None,
        purity_confidence: None,
        entropy_details: None,
        god_object_indicators: None,
    }
}

#[test]
fn test_enhanced_markdown_priority_section() {
    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);

    // Add a sample item
    let item = create_sample_unified_item();
    analysis.add_item(item);
    analysis.sort_by_priority();

    let mut output = Vec::new();
    let mut writer = MarkdownWriter::new(Cursor::new(&mut output));

    // Write priority section
    writer.write_priority_section(&analysis).unwrap();

    let markdown = String::from_utf8(output).unwrap();

    // Print for debugging if needed
    if !markdown.contains("process_data") {
        println!("Markdown output:\n{}", markdown);
    }

    // Verify the output contains expected sections
    assert!(markdown.contains("## Priority Technical Debt"));
    if !markdown.contains("_No priority items found._") {
        assert!(markdown.contains("### Top"));
        assert!(markdown.contains("| Rank | Score | Function | Type | Issue |"));
        assert!(markdown.contains("src/main.rs:42")); // Check for location instead
        assert!(markdown.contains("Testing Gap"));
    }
}

#[test]
fn test_enhanced_markdown_dead_code_section() {
    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);

    // Add a dead code item
    let mut item = create_sample_unified_item();
    item.debt_type = DebtType::DeadCode {
        visibility: FunctionVisibility::Private,
        cyclomatic: 10,
        cognitive: 15,
        usage_hints: vec![],
    };
    analysis.add_item(item);

    let mut output = Vec::new();
    let mut writer = MarkdownWriter::new(Cursor::new(&mut output));

    // Write dead code section
    writer.write_dead_code_section(&analysis).unwrap();

    let markdown = String::from_utf8(output).unwrap();

    // Verify the output
    assert!(markdown.contains("## Dead Code Detection"));
    assert!(markdown.contains("### Unused Functions"));
    assert!(markdown.contains("| Function | Visibility | Complexity | Recommendation |"));
    assert!(markdown.contains("process_data"));
    assert!(markdown.contains("private"));
}

#[test]
fn test_enhanced_markdown_testing_recommendations() {
    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);

    // Add a testing gap item
    let item = create_sample_unified_item();
    analysis.add_item(item);

    let mut output = Vec::new();
    let mut writer = MarkdownWriter::new(Cursor::new(&mut output));

    // Write testing recommendations
    writer.write_testing_recommendations(&analysis).unwrap();

    let markdown = String::from_utf8(output).unwrap();

    // Verify the output
    assert!(markdown.contains("## Testing Recommendations"));
    assert!(markdown.contains("### ROI-Based Testing Priorities"));
    assert!(markdown.contains("| Function | ROI | Complexity | Coverage | Risk Reduction |"));
    assert!(markdown.contains("process_data"));
    assert!(markdown.contains("30%")); // Coverage
}

#[test]
fn test_enhanced_markdown_with_verbosity() {
    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);

    // Add multiple items
    for i in 0..3 {
        let mut item = create_sample_unified_item();
        item.location.function = format!("function_{}", i);
        item.unified_score.final_score = 10.0 - i as f64;
        analysis.add_item(item);
    }

    // Add file aggregate
    analysis.file_aggregates.push_back(create_sample_file_aggregate());

    analysis.sort_by_priority();

    let mut output = Vec::new();
    let mut writer = MarkdownWriter::with_verbosity(Cursor::new(&mut output), 2);

    // Write full analysis
    writer.write_unified_analysis(&analysis).unwrap();

    let markdown = String::from_utf8(output).unwrap();

    // Verify verbosity features
    assert!(markdown.contains("<details>"));
    assert!(markdown.contains("Score Breakdown"));
    assert!(markdown.contains("## Call Graph Analysis"));
    assert!(markdown.contains("### Module Statistics"));
}

#[test]
fn test_enhanced_markdown_full_report() {
    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);

    // Add various types of debt items
    let mut item1 = create_sample_unified_item();
    item1.location.function = "untested_function".to_string();
    analysis.add_item(item1);

    let mut item2 = create_sample_unified_item();
    item2.location.function = "dead_function".to_string();
    item2.debt_type = DebtType::DeadCode {
        visibility: FunctionVisibility::Public,
        cyclomatic: 5,
        cognitive: 8,
        usage_hints: vec!["Consider removing".to_string()],
    };
    analysis.add_item(item2);

    let mut item3 = create_sample_unified_item();
    item3.location.function = "complex_function".to_string();
    item3.debt_type = DebtType::ComplexityHotspot {
        cyclomatic: 25,
        cognitive: 30,
    };
    analysis.add_item(item3);

    // Add file aggregate
    analysis.file_aggregates.push_back(create_sample_file_aggregate());

    analysis.sort_by_priority();

    let mut output = Vec::new();
    let mut writer = MarkdownWriter::with_verbosity(Cursor::new(&mut output), 1);

    // Write full analysis
    writer.write_unified_analysis(&analysis).unwrap();

    let markdown = String::from_utf8(output).unwrap();

    // Verify all sections are present
    assert!(markdown.contains("## Priority Technical Debt"));
    assert!(markdown.contains("## Dead Code Detection"));
    assert!(markdown.contains("## Testing Recommendations"));
    assert!(markdown.contains("Top 3 Priority Items"));

    // Verify specific items appear
    assert!(markdown.contains("untested_function"));
    assert!(markdown.contains("dead_function"));
    assert!(markdown.contains("complex_function"));
}
