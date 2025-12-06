//! Data flow graph population from various analysis sources
//!
//! This module provides functions to populate the DataFlowGraph with data from:
//! - Purity analysis (CFG-based data flow analysis)
//! - I/O operation detection
//! - Variable dependency analysis
//! - Data transformation patterns

use crate::analyzers::purity_detector::PurityAnalysis;
use crate::core::FunctionMetrics;
use crate::data_flow::{DataFlowGraph, IoOperation, MutationInfo};
use crate::priority::call_graph::FunctionId;
use std::collections::HashSet;

/// Populate DataFlowGraph from purity analysis results
///
/// Extracts and stores:
/// - Full CFG-based data flow analysis
/// - Mutation information (live vs dead stores)
/// - Escape analysis results
pub fn populate_from_purity_analysis(
    data_flow: &mut DataFlowGraph,
    func_id: &FunctionId,
    purity: &PurityAnalysis,
) {
    // Store full CFG analysis if available
    if let Some(cfg_analysis) = &purity.data_flow_info {
        data_flow.set_cfg_analysis(func_id.clone(), cfg_analysis.clone());
    }

    // Extract mutation information
    let live_mutations: Vec<String> = purity
        .live_mutations
        .iter()
        .map(|m| m.target.clone())
        .collect();

    let mutation_info = MutationInfo {
        live_mutations: live_mutations.clone(),
        total_mutations: purity.total_mutations,
        dead_stores: extract_dead_stores(purity),
        escaping_mutations: extract_escaping_mutations(purity),
    };

    data_flow.set_mutation_info(func_id.clone(), mutation_info);
}

/// Extract dead stores from purity analysis
fn extract_dead_stores(_purity: &PurityAnalysis) -> HashSet<String> {
    // Note: VarId conversion to String requires the CFG's var_names mapping
    // This is a simplified implementation - full implementation would need CFG access
    // For now, we return an empty set as this information is available via cfg_analysis
    HashSet::new()
}

/// Extract escaping mutations from purity analysis
fn extract_escaping_mutations(_purity: &PurityAnalysis) -> HashSet<String> {
    // Note: VarId conversion to String requires the CFG's var_names mapping
    // This is a simplified implementation - full implementation would need CFG access
    // For now, we return an empty set as this information is available via cfg_analysis
    HashSet::new()
}

/// Populate I/O operations from function metrics
///
/// Detects I/O operations by scanning function metrics for known patterns
pub fn populate_io_operations(data_flow: &mut DataFlowGraph, metrics: &[FunctionMetrics]) -> usize {
    let mut total_ops = 0;

    for metric in metrics {
        let func_id = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line);

        // Check for I/O indicators in the metric
        let io_ops = detect_io_from_metrics(metric);

        for op in io_ops {
            data_flow.add_io_operation(func_id.clone(), op);
            total_ops += 1;
        }
    }

    total_ops
}

/// Detect I/O operations from function metrics
fn detect_io_from_metrics(metric: &FunctionMetrics) -> Vec<IoOperation> {
    let mut ops = Vec::new();

    // Check if function has I/O characteristics based on existing analysis
    // This is a simplified version - more sophisticated detection would require AST access
    if !metric.is_pure.unwrap_or(true) {
        // Check function name for common I/O patterns
        let name_lower = metric.name.to_lowercase();

        if name_lower.contains("read") || name_lower.contains("write") {
            ops.push(IoOperation {
                operation_type: "file_io".to_string(),
                variables: vec![],
                line: metric.line,
            });
        } else if name_lower.contains("print") || name_lower.contains("log") {
            ops.push(IoOperation {
                operation_type: "console".to_string(),
                variables: vec![],
                line: metric.line,
            });
        } else if name_lower.contains("fetch") || name_lower.contains("request") {
            ops.push(IoOperation {
                operation_type: "network".to_string(),
                variables: vec![],
                line: metric.line,
            });
        }
    }

    ops
}

/// Populate variable dependencies from function metrics
///
/// Tracks which variables each function depends on
pub fn populate_variable_dependencies(
    data_flow: &mut DataFlowGraph,
    metrics: &[FunctionMetrics],
) -> usize {
    let mut total_deps = 0;

    for metric in metrics {
        let func_id = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line);

        // Extract variable dependencies from metric
        let deps = extract_variable_deps(metric);

        if !deps.is_empty() {
            data_flow.add_variable_dependencies(func_id, deps.clone());
            total_deps += deps.len();
        }
    }

    total_deps
}

/// Extract variable dependencies from function metrics
fn extract_variable_deps(_metric: &FunctionMetrics) -> HashSet<String> {
    // Note: Variable dependency extraction requires AST parsing
    // This is a placeholder implementation
    // Full implementation would parse function signatures to extract parameter names
    HashSet::new()
}

/// Populate data transformations between functions
///
/// Identifies transformation patterns like map, filter, fold
pub fn populate_data_transformations(
    data_flow: &mut DataFlowGraph,
    _metrics: &[FunctionMetrics],
) -> usize {
    // TODO: Implement data transformation detection
    // This would require call graph traversal and AST analysis
    // For now, return 0 as a placeholder
    let _data_flow = data_flow; // Use the parameter
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::data_flow::{
        DataFlowAnalysis, EscapeAnalysis, LivenessInfo, ReachingDefinitions, TaintAnalysis,
    };
    use crate::core::PurityLevel;
    use std::collections::{HashMap, HashSet};
    use std::path::PathBuf;

    fn create_test_function_id(name: &str) -> FunctionId {
        FunctionId::new(PathBuf::from("test.rs"), name.to_string(), 1)
    }

    fn create_test_purity_analysis() -> PurityAnalysis {
        use crate::analyzers::purity_detector::LocalMutation;

        // Create minimal data flow analysis
        let liveness = LivenessInfo {
            live_in: HashMap::new(),
            live_out: HashMap::new(),
            dead_stores: HashSet::new(),
        };

        let reaching_defs = ReachingDefinitions {
            reach_in: HashMap::new(),
            reach_out: HashMap::new(),
            def_use_chains: HashMap::new(),
        };

        let escape_info = EscapeAnalysis {
            escaping_vars: HashSet::new(),
            captured_vars: HashSet::new(),
            return_dependencies: HashSet::new(),
        };

        let taint_info = TaintAnalysis {
            tainted_vars: HashSet::new(),
            taint_sources: HashMap::new(),
            return_tainted: false,
        };

        let data_flow = DataFlowAnalysis {
            liveness,
            reaching_defs,
            escape_info,
            taint_info,
        };

        PurityAnalysis {
            is_pure: false,
            purity_level: PurityLevel::LocallyPure,
            reasons: vec![],
            confidence: 0.9,
            data_flow_info: Some(data_flow),
            live_mutations: vec![LocalMutation {
                target: "x".to_string(),
            }],
            total_mutations: 2,
        }
    }

    #[test]
    fn test_populate_from_purity_analysis() {
        let mut data_flow = DataFlowGraph::new();
        let func_id = create_test_function_id("test_func");
        let purity = create_test_purity_analysis();

        populate_from_purity_analysis(&mut data_flow, &func_id, &purity);

        // Verify CFG analysis was stored
        assert!(data_flow.get_cfg_analysis(&func_id).is_some());

        // Verify mutation info was stored
        let mutation_info = data_flow.get_mutation_info(&func_id).unwrap();
        assert_eq!(mutation_info.live_mutations.len(), 1);
        assert_eq!(mutation_info.total_mutations, 2);
        // Note: dead_stores is empty as VarId->String conversion needs CFG context
        assert!(mutation_info.dead_stores.is_empty());
    }

    #[test]
    fn test_extract_dead_stores() {
        let purity = create_test_purity_analysis();
        let dead_stores = extract_dead_stores(&purity);

        // Note: Currently returns empty set as VarId->String conversion needs CFG context
        // The actual dead store information is available via cfg_analysis field
        assert!(dead_stores.is_empty());
    }

    #[test]
    fn test_extract_escaping_mutations() {
        let purity = create_test_purity_analysis();
        let escaping = extract_escaping_mutations(&purity);

        // Note: Currently returns empty set as VarId->String conversion needs CFG context
        // The actual escaping information is available via cfg_analysis field
        assert!(escaping.is_empty());
    }

    fn create_test_metric(name: &str, line: usize, is_pure: bool) -> FunctionMetrics {
        let mut metric = FunctionMetrics::new(name.to_string(), PathBuf::from("test.rs"), line);
        metric.length = 10;
        metric.is_pure = Some(is_pure);
        metric
    }

    #[test]
    fn test_populate_io_operations() {
        let mut data_flow = DataFlowGraph::new();
        let metrics = vec![
            create_test_metric("read_file", 10, false),
            create_test_metric("pure_func", 20, true),
        ];

        let count = populate_io_operations(&mut data_flow, &metrics);

        assert!(count > 0);
        let mut func_id = create_test_function_id("read_file");
        func_id.line = 10;
        // Should have detected I/O in read_file
        // Note: This test may need adjustment based on actual detection logic
    }
}
