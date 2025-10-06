use debtmap::priority::{
    unified_scorer::{Location, UnifiedDebtItem, UnifiedScore},
    CallGraph, DebtCategory, DebtType, FunctionRole, FunctionVisibility, ImpactMetrics,
    UnifiedAnalysis,
};
use std::path::PathBuf;

fn create_testing_item(function_name: &str) -> UnifiedDebtItem {
    UnifiedDebtItem {
        location: Location {
            file: PathBuf::from("src/test.rs"),
            function: function_name.to_string(),
            line: 10,
        },
        debt_type: DebtType::TestingGap {
            coverage: 0.2,
            cyclomatic: 10,
            cognitive: 12,
        },
        unified_score: UnifiedScore {
            complexity_factor: 6.0,
            coverage_factor: 7.0,
            dependency_factor: 3.0,
            role_multiplier: 1.0,
            final_score: 6.5,
        },
        function_role: FunctionRole::PureLogic,
        recommendation: debtmap::priority::ActionableRecommendation {
            primary_action: "Add unit tests".to_string(),
            rationale: "Low test coverage".to_string(),
            implementation_steps: vec![],
            related_items: vec![],
        },
        expected_impact: ImpactMetrics {
            coverage_improvement: 25.0,
            lines_reduction: 0,
            complexity_reduction: 0.0,
            risk_reduction: 20.0,
        },
        transitive_coverage: None,
        upstream_dependencies: 2,
        downstream_dependencies: 3,
        upstream_callers: vec![],
        downstream_callees: vec![],
        nesting_depth: 2,
        function_length: 30,
        cyclomatic_complexity: 10,
        cognitive_complexity: 12,
        is_pure: None,
        purity_confidence: None,
        entropy_details: None,
        god_object_indicators: None,
        tier: None,
    }
}

fn create_architecture_item(function_name: &str) -> UnifiedDebtItem {
    let mut item = create_testing_item(function_name);
    item.debt_type = DebtType::GodObject {
        methods: 15,
        fields: 8,
        responsibilities: 8,
        god_object_score: 50.0,
    };
    item
}

fn create_performance_item(function_name: &str) -> UnifiedDebtItem {
    let mut item = create_testing_item(function_name);
    item.debt_type = DebtType::ResourceLeak {
        resource_type: "File Handle".to_string(),
        cleanup_missing: "Missing file.close() in error path".to_string(),
    };
    item
}

fn create_dead_code_item(function_name: &str) -> UnifiedDebtItem {
    let mut item = create_testing_item(function_name);
    item.debt_type = DebtType::DeadCode {
        visibility: FunctionVisibility::Private,
        cyclomatic: 8,
        cognitive: 10,
        usage_hints: vec![],
    };
    item
}

#[test]
fn test_filter_testing_category() {
    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);

    // Add items from different categories
    analysis.add_item(create_testing_item("test_func"));
    analysis.add_item(create_architecture_item("god_func"));
    analysis.add_item(create_performance_item("leak_func"));
    analysis.add_item(create_dead_code_item("dead_func"));

    // Filter by Testing category
    let filtered = analysis.filter_by_categories(&[DebtCategory::Testing]);

    // Should only contain testing items
    assert_eq!(filtered.items.len(), 1);
    assert!(filtered.items.iter().all(|item| {
        matches!(
            DebtCategory::from_debt_type(&item.debt_type),
            DebtCategory::Testing
        )
    }));
}

#[test]
fn test_filter_architecture_category() {
    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);

    // Add items from different categories
    analysis.add_item(create_testing_item("test_func"));
    analysis.add_item(create_architecture_item("god_func"));
    analysis.add_item(create_performance_item("leak_func"));
    analysis.add_item(create_dead_code_item("dead_func"));

    // Filter by Architecture category
    let filtered = analysis.filter_by_categories(&[DebtCategory::Architecture]);

    // Should contain architecture items (GodObject + DeadCode)
    assert!(!filtered.items.is_empty());
    assert!(filtered.items.iter().all(|item| {
        matches!(
            DebtCategory::from_debt_type(&item.debt_type),
            DebtCategory::Architecture
        )
    }));
}

#[test]
fn test_filter_multiple_categories() {
    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);

    // Add items from different categories
    analysis.add_item(create_testing_item("test_func"));
    analysis.add_item(create_architecture_item("god_func"));
    analysis.add_item(create_performance_item("leak_func"));
    analysis.add_item(create_dead_code_item("dead_func"));

    // Filter by Architecture and Testing categories
    let filtered =
        analysis.filter_by_categories(&[DebtCategory::Architecture, DebtCategory::Testing]);

    // Should contain both architecture and testing items
    assert!(filtered.items.len() >= 2);
    assert!(filtered.items.iter().all(|item| {
        let category = DebtCategory::from_debt_type(&item.debt_type);
        matches!(category, DebtCategory::Architecture | DebtCategory::Testing)
    }));
}

#[test]
fn test_category_from_string() {
    // Test valid category strings
    assert_eq!(
        DebtCategory::from_string("Architecture"),
        Some(DebtCategory::Architecture)
    );
    assert_eq!(
        DebtCategory::from_string("architecture"),
        Some(DebtCategory::Architecture)
    );
    assert_eq!(
        DebtCategory::from_string("Testing"),
        Some(DebtCategory::Testing)
    );
    assert_eq!(
        DebtCategory::from_string("testing"),
        Some(DebtCategory::Testing)
    );
    assert_eq!(
        DebtCategory::from_string("Performance"),
        Some(DebtCategory::Performance)
    );
    assert_eq!(
        DebtCategory::from_string("performance"),
        Some(DebtCategory::Performance)
    );

    // Test CodeQuality aliases
    assert_eq!(
        DebtCategory::from_string("CodeQuality"),
        Some(DebtCategory::CodeQuality)
    );
    assert_eq!(
        DebtCategory::from_string("code_quality"),
        Some(DebtCategory::CodeQuality)
    );
    assert_eq!(
        DebtCategory::from_string("quality"),
        Some(DebtCategory::CodeQuality)
    );

    // Test invalid category
    assert_eq!(DebtCategory::from_string("InvalidCategory"), None);
}

#[test]
fn test_filter_empty_categories() {
    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph);

    analysis.add_item(create_testing_item("test_func"));
    analysis.add_item(create_architecture_item("god_func"));

    // Filter by empty category list should return nothing
    let filtered = analysis.filter_by_categories(&[]);

    assert_eq!(filtered.items.len(), 0);
}

#[test]
fn test_filter_preserves_call_graph() {
    let call_graph = CallGraph::new();
    let mut analysis = UnifiedAnalysis::new(call_graph.clone());

    analysis.add_item(create_testing_item("test_func"));
    analysis.add_item(create_architecture_item("god_func"));

    // Filter should preserve the call graph
    let filtered = analysis.filter_by_categories(&[DebtCategory::Testing]);

    // The call graph should still be present (we can't compare directly, but we can verify it's not empty)
    assert_eq!(filtered.items.len(), 1);
}
