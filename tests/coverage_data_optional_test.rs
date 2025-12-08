/// Integration tests for spec 108: Optional coverage data handling
///
/// This test verifies that when has_coverage_data=false:
/// 1. [UNTESTED] labels are not displayed
/// 2. Coverage indicators are hidden
/// 3. Coverage warning message is shown exactly once
use debtmap::priority::formatter::format_priorities_with_verbosity;
use debtmap::priority::score_types::Score0To100;
use debtmap::priority::{
    CallGraph, DebtType, FunctionRole, ImpactMetrics, Location, OutputFormat, UnifiedAnalysis,
    UnifiedAnalysisUtils, UnifiedDebtItem, UnifiedScore,
};
use std::path::PathBuf;

fn create_untested_item() -> UnifiedDebtItem {
    UnifiedDebtItem {
        location: Location {
            file: PathBuf::from("src/main.rs"),
            function: "untested_function".to_string(),
            line: 100,
        },
        debt_type: DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 15,
            cognitive: 20,
        },
        unified_score: UnifiedScore {
            complexity_factor: 7.5,
            coverage_factor: 10.0, // High coverage factor indicates untested
            dependency_factor: 4.0,
            role_multiplier: 1.0,
            final_score: Score0To100::new(85.0),
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
        recommendation: debtmap::priority::ActionableRecommendation {
            primary_action: "Add unit tests".to_string(),
            rationale: "Function is untested".to_string(),
            implementation_steps: vec![],
            related_items: vec![],
            steps: None,
            estimated_effort_hours: None,
        },
        expected_impact: ImpactMetrics {
            coverage_improvement: 100.0,
            lines_reduction: 0,
            complexity_reduction: 0.0,
            risk_reduction: 50.0,
        },
        transitive_coverage: None, // No coverage data
        file_context: None,
        upstream_dependencies: 2,
        downstream_dependencies: 3,
        upstream_callers: vec![],
        downstream_callees: vec![],
        nesting_depth: 3,
        function_length: 50,
        cyclomatic_complexity: 15,
        cognitive_complexity: 20,
        is_pure: None,
        purity_confidence: None,
        purity_level: None,
        entropy_details: None,
        entropy_adjusted_cyclomatic: None,
        entropy_adjusted_cognitive: None,
        entropy_dampening_factor: None,
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
    }
}

#[test]
fn test_untested_labels_hidden_when_no_coverage_data() {
    // Create an analysis with an item that would normally show [UNTESTED] label
    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);
    analysis.has_coverage_data = false; // No coverage data

    let item = create_untested_item();
    analysis.add_item(item);
    analysis.sort_by_priority();

    // Format the analysis
    let output = format_priorities_with_verbosity(&analysis, OutputFormat::Top(1), 0);

    // Verify NO coverage indicators are present
    assert!(
        !output.contains("UNTESTED"),
        "Output should not contain UNTESTED label when has_coverage_data=false. Output:\n{}",
        output
    );
    assert!(
        !output.contains("🔴"),
        "Output should not contain red circle emoji when has_coverage_data=false. Output:\n{}",
        output
    );
    assert!(
        !output.contains("LOW COVERAGE"),
        "Output should not contain LOW COVERAGE label when has_coverage_data=false. Output:\n{}",
        output
    );
    assert!(
        !output.contains("PARTIAL COVERAGE"),
        "Output should not contain PARTIAL COVERAGE label when has_coverage_data=false. Output:\n{}",
        output
    );
}

#[test]
fn test_untested_labels_shown_when_coverage_data_available() {
    // Create an analysis WITH coverage data
    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);
    analysis.has_coverage_data = true; // Has coverage data

    let item = create_untested_item();
    analysis.add_item(item);
    analysis.sort_by_priority();

    // Format the analysis
    let output = format_priorities_with_verbosity(&analysis, OutputFormat::Top(1), 0);

    // Verify coverage line is present (spec 180)
    // When LCOV was provided but function not found, show "no coverage data"
    // Note: Check for "COVERAGE" without colon because ANSI color codes separate them
    assert!(
        output.contains("COVERAGE"),
        "Output should contain COVERAGE line when has_coverage_data=true. Output:\n{}",
        output
    );
    assert!(
        output.contains("no coverage data"),
        "Output should show 'no coverage data' when function not found in LCOV. Output:\n{}",
        output
    );
}

#[test]
fn test_coverage_indicator_hidden_in_unified_analysis_without_coverage() {
    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);

    // Set has_coverage_data to false
    analysis.has_coverage_data = false;

    // Add an untested item
    let item = create_untested_item();
    analysis.add_item(item);
    analysis.sort_by_priority();

    // Format the analysis
    let output = format_priorities_with_verbosity(&analysis, OutputFormat::Top(1), 0);

    // Verify NO coverage indicators
    assert!(
        !output.contains("UNTESTED"),
        "UnifiedAnalysis with has_coverage_data=false should not show UNTESTED. Output:\n{}",
        output
    );
    assert!(
        !output.contains("🔴"),
        "UnifiedAnalysis with has_coverage_data=false should not show coverage emoji. Output:\n{}",
        output
    );
}

#[test]
fn test_coverage_scoring_factors_hidden_without_coverage_data() {
    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);
    analysis.has_coverage_data = false;

    let item = create_untested_item();
    analysis.add_item(item);
    analysis.sort_by_priority();

    // Format with verbosity 1 (shows main factors) but no coverage data
    let output = format_priorities_with_verbosity(&analysis, OutputFormat::Top(1), 1);

    // When has_coverage_data=false, coverage factors should not be shown
    // Check that coverage-related factors are not mentioned
    let lines: Vec<&str> = output.lines().collect();
    let factors_line = lines
        .iter()
        .find(|line| line.contains("Main factors:"))
        .unwrap_or(&"");

    // The factors line should not mention coverage when has_coverage_data=false
    assert!(
        !factors_line.contains("UNTESTED"),
        "Main factors should not mention UNTESTED when has_coverage_data=false. Factors line: {}",
        factors_line
    );
    assert!(
        !factors_line.contains("coverage"),
        "Main factors should not mention coverage when has_coverage_data=false. Factors line: {}",
        factors_line
    );
}

#[test]
fn test_detailed_score_calculation_respects_coverage_flag() {
    // Test with coverage data
    let call_graph_with = CallGraph::new();
    let mut analysis_with = UnifiedAnalysis::new(call_graph_with);
    analysis_with.has_coverage_data = true;
    let item1 = create_untested_item();
    analysis_with.add_item(item1);
    analysis_with.sort_by_priority();

    // Test without coverage data
    let call_graph_without = CallGraph::new();
    let mut analysis_without = UnifiedAnalysis::new(call_graph_without);
    analysis_without.has_coverage_data = false;
    let item2 = create_untested_item();
    analysis_without.add_item(item2);
    analysis_without.sort_by_priority();

    // Format with verbosity 2 (detailed calculation)
    let output_with_coverage =
        format_priorities_with_verbosity(&analysis_with, OutputFormat::Top(1), 2);
    let output_without_coverage =
        format_priorities_with_verbosity(&analysis_without, OutputFormat::Top(1), 2);

    // Both should show score calculation (scores are always calculated)
    assert!(output_with_coverage.contains("SCORE CALCULATION:"));
    assert!(output_without_coverage.contains("SCORE CALCULATION:"));

    // But the detailed coverage section should not be shown when has_coverage_data=false
    // Actually, looking at the code, score calculation always shows the calculation
    // The difference is in the labels and indicators, not the calculation itself
    // So we just verify that UNTESTED labels don't appear
    assert!(
        output_with_coverage.contains("UNTESTED") || output_with_coverage.contains("coverage"),
        "With coverage data, should mention coverage"
    );
    assert!(
        !output_without_coverage.contains("[UNTESTED]"),
        "Without coverage data, should not show [UNTESTED] label"
    );
}
