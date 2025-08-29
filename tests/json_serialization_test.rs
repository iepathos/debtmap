use debtmap::data_flow::DataFlowGraph;
use debtmap::priority::{CallGraph, ImpactMetrics, UnifiedAnalysis};
use im::Vector;

#[test]
fn test_unified_analysis_json_serialization() {
    // Create a minimal UnifiedAnalysis instance
    let analysis = UnifiedAnalysis {
        items: Vector::new(),
        total_impact: ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: 0,
            complexity_reduction: 0.0,
            risk_reduction: 0.0,
        },
        total_debt_score: 0.0,
        call_graph: CallGraph::new(),
        data_flow_graph: DataFlowGraph::new(),
        overall_coverage: None,
    };

    // Attempt to serialize to JSON
    let result = serde_json::to_string_pretty(&analysis);

    // This should succeed
    assert!(
        result.is_ok(),
        "Failed to serialize UnifiedAnalysis to JSON: {:?}",
        result.err()
    );

    // Verify the JSON is valid
    if let Ok(json_str) = result {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
        assert!(parsed.is_ok(), "Generated invalid JSON");
    }
}

#[test]
fn test_data_flow_graph_serialization() {
    let data_flow_graph = DataFlowGraph::new();

    // This test specifically checks if DataFlowGraph can be serialized
    let result = serde_json::to_string(&data_flow_graph);
    assert!(
        result.is_ok(),
        "DataFlowGraph serialization failed: {:?}",
        result.err()
    );
}
