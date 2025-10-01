// Integration tests for the compare command
// These tests verify the end-to-end workflow of comparing debtmap analyses

use anyhow::Result;
use debtmap::comparison::types::{ComparisonResult, DebtTrend, TargetStatus};
use debtmap::comparison::{Comparator, PlanParser};
use debtmap::priority::UnifiedAnalysis;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_compare_with_improvement() -> Result<()> {
    // Test comparing analyses where target item was improved
    let before_path = PathBuf::from("tests/data/fixtures/before.json");
    let after_path = PathBuf::from("tests/data/fixtures/after_improved.json");
    let plan_path = PathBuf::from("tests/data/fixtures/IMPLEMENTATION_PLAN.md");

    // Load analysis results from JSON files
    let before_content = fs::read_to_string(&before_path)?;
    let before_analysis: UnifiedAnalysis = serde_json::from_str(&before_content)?;

    let after_content = fs::read_to_string(&after_path)?;
    let after_analysis: UnifiedAnalysis = serde_json::from_str(&after_content)?;

    // Extract target location from plan
    let target_location = PlanParser::extract_target_location(&plan_path)?;

    // Perform comparison
    let comparator = Comparator::new(
        before_analysis,
        after_analysis,
        Some(target_location.clone()),
    );
    let result = comparator.compare()?;

    // Verify the comparison result structure
    assert_eq!(result.metadata.target_location, Some(target_location));

    // Verify target item shows improvement (resolved means it no longer exists in after)
    assert!(result.target_item.is_some());
    let target = result.target_item.as_ref().unwrap();
    assert_eq!(target.status, TargetStatus::Resolved);

    // Verify summary shows improvements
    assert!(result.summary.target_improved);
    assert_eq!(result.summary.new_critical_count, 0);
    assert_eq!(result.summary.resolved_count, 1);
    assert_eq!(result.summary.overall_debt_trend, DebtTrend::Improving);

    // Verify project health shows improvement
    assert!(result.project_health.changes.debt_score_change < 0.0); // Debt decreased
    assert!(result.project_health.changes.critical_items_change < 0); // Fewer critical items

    Ok(())
}

#[test]
fn test_compare_with_regression() -> Result<()> {
    // Test comparing analyses where target item regressed
    let before_path = PathBuf::from("tests/data/fixtures/before.json");
    let after_path = PathBuf::from("tests/data/fixtures/after_regression.json");
    let plan_path = PathBuf::from("tests/data/fixtures/IMPLEMENTATION_PLAN.md");

    // Load analysis results from JSON files
    let before_content = fs::read_to_string(&before_path)?;
    let before_analysis: UnifiedAnalysis = serde_json::from_str(&before_content)?;

    let after_content = fs::read_to_string(&after_path)?;
    let after_analysis: UnifiedAnalysis = serde_json::from_str(&after_content)?;

    // Extract target location from plan
    let target_location = PlanParser::extract_target_location(&plan_path)?;

    // Perform comparison
    let comparator = Comparator::new(before_analysis, after_analysis, Some(target_location));
    let result = comparator.compare()?;

    // Verify target status shows regression
    assert!(result.target_item.is_some());
    let target = result.target_item.as_ref().unwrap();
    assert_eq!(target.status, TargetStatus::Regressed);

    // Verify summary shows regressions
    assert!(!result.summary.target_improved);
    assert!(result.summary.new_critical_count > 0);
    assert_eq!(result.summary.overall_debt_trend, DebtTrend::Regressing);

    // Verify project health shows increase in debt
    assert!(result.project_health.changes.debt_score_change > 0.0); // Debt increased
    assert!(result.project_health.changes.critical_items_change > 0); // More critical items

    Ok(())
}

#[test]
fn test_compare_without_plan() -> Result<()> {
    // Test comparing analyses without a plan file
    let before_path = PathBuf::from("tests/data/fixtures/before.json");
    let after_path = PathBuf::from("tests/data/fixtures/after_improved.json");

    // Load analysis results from JSON files
    let before_content = fs::read_to_string(&before_path)?;
    let before_analysis: UnifiedAnalysis = serde_json::from_str(&before_content)?;

    let after_content = fs::read_to_string(&after_path)?;
    let after_analysis: UnifiedAnalysis = serde_json::from_str(&after_content)?;

    // Perform comparison without target location
    let comparator = Comparator::new(before_analysis, after_analysis, None);
    let result = comparator.compare()?;

    // Should still work but without target tracking
    assert_eq!(result.metadata.target_location, None);
    assert!(result.target_item.is_none());

    // Overall project health metrics should still be calculated correctly
    assert!(result.project_health.changes.debt_score_change < 0.0);
    assert_eq!(result.summary.resolved_count, 1);

    Ok(())
}

#[test]
fn test_compare_result_serialization() -> Result<()> {
    // Test that comparison results can be serialized to JSON
    let before_path = PathBuf::from("tests/data/fixtures/before.json");
    let after_path = PathBuf::from("tests/data/fixtures/after_improved.json");
    let plan_path = PathBuf::from("tests/data/fixtures/IMPLEMENTATION_PLAN.md");

    // Load analysis results from JSON files
    let before_content = fs::read_to_string(&before_path)?;
    let before_analysis: UnifiedAnalysis = serde_json::from_str(&before_content)?;

    let after_content = fs::read_to_string(&after_path)?;
    let after_analysis: UnifiedAnalysis = serde_json::from_str(&after_content)?;

    // Extract target location from plan
    let target_location = PlanParser::extract_target_location(&plan_path)?;

    // Perform comparison
    let comparator = Comparator::new(before_analysis, after_analysis, Some(target_location));
    let result = comparator.compare()?;

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&result)?;

    // Verify JSON contains expected fields
    assert!(json.contains("\"metadata\""));
    assert!(json.contains("\"target_item\""));
    assert!(json.contains("\"project_health\""));
    assert!(json.contains("\"summary\""));
    assert!(json.contains("\"regressions\""));
    assert!(json.contains("\"improvements\""));

    // Deserialize back and verify
    let deserialized: ComparisonResult = serde_json::from_str(&json)?;
    assert_eq!(
        deserialized.summary.target_improved,
        result.summary.target_improved
    );
    assert_eq!(
        deserialized.summary.resolved_count,
        result.summary.resolved_count
    );

    Ok(())
}

#[test]
fn test_compare_command_compiles() {
    // This test ensures the compare module exports are correct
    // Just verify types are accessible
    let _status = TargetStatus::Improved;
    let _trend = DebtTrend::Improving;

    // Test passes if compilation succeeds
}

#[test]
fn test_plan_parser_api() -> Result<()> {
    // Test that PlanParser can extract target location from plan file
    let plan_path = PathBuf::from("tests/data/fixtures/IMPLEMENTATION_PLAN.md");

    // Test extraction
    let location = PlanParser::extract_target_location(&plan_path)?;
    assert_eq!(location, "src/example.rs:complex_function:45");

    Ok(())
}

#[test]
fn test_comparison_types_serialization() -> Result<()> {
    // Verify comparison result types can be serialized/deserialized
    use debtmap::comparison::types::{
        ComparisonMetadata, ComparisonSummary, DebtTrend, TargetStatus,
    };
    use serde_json;

    // Test metadata serialization
    let metadata = ComparisonMetadata {
        comparison_date: "2025-10-01T10:00:00Z".to_string(),
        before_file: "before.json".to_string(),
        after_file: "after.json".to_string(),
        target_location: Some("src/test.rs:10".to_string()),
    };

    let json = serde_json::to_string(&metadata)?;
    let _deserialized: ComparisonMetadata = serde_json::from_str(&json)?;

    // Test enum serialization
    let status = TargetStatus::Improved;
    let json = serde_json::to_string(&status)?;
    let _deserialized: TargetStatus = serde_json::from_str(&json)?;

    let trend = DebtTrend::Improving;
    let json = serde_json::to_string(&trend)?;
    let _deserialized: DebtTrend = serde_json::from_str(&json)?;

    // Test summary serialization
    let summary = ComparisonSummary {
        target_improved: true,
        new_critical_count: 0,
        resolved_count: 5,
        overall_debt_trend: DebtTrend::Improving,
    };

    let json = serde_json::to_string(&summary)?;
    let _deserialized: ComparisonSummary = serde_json::from_str(&json)?;

    Ok(())
}
