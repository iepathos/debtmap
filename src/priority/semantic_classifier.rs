use crate::core::FunctionMetrics;
use crate::priority::call_graph::{CallGraph, FunctionId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FunctionRole {
    PureLogic,    // Business logic, high test priority
    Orchestrator, // Coordinates other functions
    IOWrapper,    // Thin I/O layer
    EntryPoint,   // Main entry points
    Unknown,      // Cannot classify
}

pub fn classify_function_role(
    func: &FunctionMetrics,
    func_id: &FunctionId,
    call_graph: &CallGraph,
) -> FunctionRole {
    // Entry point detection
    if call_graph.is_entry_point(func_id) {
        return FunctionRole::EntryPoint;
    }

    // Check if function name suggests entry point
    if is_entry_point_by_name(&func_id.name) {
        return FunctionRole::EntryPoint;
    }

    // Check if function name suggests orchestration
    if is_orchestrator_by_name(&func_id.name) && func.cyclomatic <= 3 {
        return FunctionRole::Orchestrator;
    }

    // Simple orchestration: low complexity, mostly delegates
    if func.cyclomatic <= 2
        && func.cognitive <= 3
        && delegates_to_tested_functions(func_id, call_graph, 0.8)
    {
        return FunctionRole::Orchestrator;
    }

    // I/O wrapper: contains I/O patterns, thin logic
    if contains_io_patterns(func) && func.length < 20 {
        return FunctionRole::IOWrapper;
    }

    // Everything else is considered pure logic
    FunctionRole::PureLogic
}

fn is_entry_point_by_name(name: &str) -> bool {
    let entry_patterns = [
        "main", "run", "start", "init", "handle", "process", "execute", "serve", "listen",
    ];

    let name_lower = name.to_lowercase();
    entry_patterns
        .iter()
        .any(|pattern| name_lower.starts_with(pattern) || name_lower.ends_with(pattern))
}

fn is_orchestrator_by_name(name: &str) -> bool {
    let orchestrator_patterns = [
        "orchestrate",
        "coordinate",
        "manage",
        "dispatch",
        "route",
        "if_requested",
        "if_needed",
        "if_enabled",
        "maybe",
        "try_",
        "attempt_",
        "delegate",
        "forward",
    ];

    let name_lower = name.to_lowercase();

    // Check for conditional patterns like generate_report_if_requested
    if name_lower.contains("_if_") || name_lower.contains("_when_") {
        return true;
    }

    orchestrator_patterns
        .iter()
        .any(|pattern| name_lower.contains(pattern))
}

fn delegates_to_tested_functions(
    func_id: &FunctionId,
    call_graph: &CallGraph,
    _threshold: f64,
) -> bool {
    let callees = call_graph.get_callees(func_id);
    if callees.is_empty() {
        return false;
    }

    // For now, assume delegation if the function calls multiple other functions
    // and has low complexity (actual coverage check would require coverage data)
    callees.len() >= 2 && call_graph.detect_delegation_pattern(func_id)
}

fn contains_io_patterns(func: &FunctionMetrics) -> bool {
    // Check for I/O related patterns in function name or content
    let io_keywords = vec![
        "read", "write", "file", "socket", "http", "request", "response", "stream", "buffer",
        "stdin", "stdout", "stderr", "print", "input", "output",
    ];

    let name_lower = func.name.to_lowercase();
    io_keywords
        .iter()
        .any(|keyword| name_lower.contains(keyword))
}

pub fn get_role_multiplier(role: FunctionRole) -> f64 {
    match role {
        FunctionRole::PureLogic => 1.5,    // High priority for business logic
        FunctionRole::Orchestrator => 0.2, // Low priority if delegates to tested code
        FunctionRole::IOWrapper => 0.1,    // Very low priority for thin I/O
        FunctionRole::EntryPoint => 0.8,   // Medium priority (integration test focus)
        FunctionRole::Unknown => 1.0,      // Default multiplier
    }
}

pub fn calculate_semantic_priority(
    _func: &FunctionMetrics,
    role: FunctionRole,
    func_id: &FunctionId,
    call_graph: &CallGraph,
) -> f64 {
    let mut priority = match role {
        FunctionRole::PureLogic => 8.0,
        FunctionRole::Orchestrator => 2.0,
        FunctionRole::IOWrapper => 1.0,
        FunctionRole::EntryPoint => 6.0,
        FunctionRole::Unknown => 5.0,
    };

    // Adjust based on criticality
    let criticality = call_graph.calculate_criticality(func_id);
    priority *= criticality;

    // Cap at 10.0
    priority.min(10.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::call_graph::CallGraph;
    use std::path::PathBuf;

    fn create_test_metrics(
        name: &str,
        cyclomatic: u32,
        cognitive: u32,
        lines: usize,
    ) -> FunctionMetrics {
        FunctionMetrics {
            file: PathBuf::from("test.rs"),
            name: name.to_string(),
            line: 1,
            length: lines,
            cyclomatic,
            cognitive,
            nesting: 0,
            is_test: false,
        }
    }

    #[test]
    fn test_entry_point_classification() {
        let graph = CallGraph::new();
        let func = create_test_metrics("main", 5, 8, 50);
        let func_id = FunctionId {
            file: PathBuf::from("main.rs"),
            name: "main".to_string(),
            line: 1,
        };

        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(role, FunctionRole::EntryPoint);
    }

    #[test]
    fn test_orchestrator_classification() {
        let mut graph = CallGraph::new();
        let func = create_test_metrics("coordinate_tasks", 2, 3, 15);
        let func_id = FunctionId {
            file: PathBuf::from("coord.rs"),
            name: "coordinate_tasks".to_string(),
            line: 10,
        };

        // Add the orchestrator function
        graph.add_function(func_id.clone(), false, false, 2, 15);

        // Add some worker functions it calls
        for i in 0..3 {
            let worker_id = FunctionId {
                file: PathBuf::from("worker.rs"),
                name: format!("worker_{i}"),
                line: i * 10,
            };
            graph.add_function(worker_id.clone(), false, false, 8, 40);
            graph.add_call(crate::priority::call_graph::FunctionCall {
                caller: func_id.clone(),
                callee: worker_id,
                call_type: crate::priority::call_graph::CallType::Delegate,
            });
        }

        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(role, FunctionRole::Orchestrator);
    }

    #[test]
    fn test_io_wrapper_classification() {
        let graph = CallGraph::new();
        let func = create_test_metrics("read_file", 1, 2, 10);
        let func_id = FunctionId {
            file: PathBuf::from("io.rs"),
            name: "read_file".to_string(),
            line: 5,
        };

        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(role, FunctionRole::IOWrapper);
    }

    #[test]
    fn test_pure_logic_classification() {
        let graph = CallGraph::new();
        let func = create_test_metrics("calculate_risk", 8, 12, 60);
        let func_id = FunctionId {
            file: PathBuf::from("calc.rs"),
            name: "calculate_risk".to_string(),
            line: 20,
        };

        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(role, FunctionRole::PureLogic);
    }

    #[test]
    fn test_role_multipliers() {
        assert_eq!(get_role_multiplier(FunctionRole::PureLogic), 1.5);
        assert_eq!(get_role_multiplier(FunctionRole::Orchestrator), 0.2);
        assert_eq!(get_role_multiplier(FunctionRole::IOWrapper), 0.1);
        assert_eq!(get_role_multiplier(FunctionRole::EntryPoint), 0.8);
        assert_eq!(get_role_multiplier(FunctionRole::Unknown), 1.0);
    }
}
