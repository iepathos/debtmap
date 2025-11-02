/// Cohesion score calculation for module split recommendations
///
/// This module calculates how tightly related the methods within a proposed module are
/// by analyzing function call patterns. High cohesion indicates methods work together frequently.
use crate::organization::god_object_analysis::ModuleSplit;
use crate::organization::struct_ownership::StructOwnershipAnalyzer;
use crate::priority::call_graph::CallGraph;
use std::collections::HashSet;

/// Calculate cohesion score for a module split recommendation
///
/// Cohesion measures how tightly related the methods within a module are.
/// High cohesion (>0.7) indicates methods work together frequently.
/// Low cohesion (<0.5) suggests the module might be poorly grouped.
///
/// Formula: cohesion = internal_calls / (internal_calls + external_calls)
///
/// # Arguments
/// * `split` - The module split recommendation
/// * `call_graph` - The function call graph for the file
/// * `ownership` - Struct ownership information
///
/// # Returns
/// Cohesion score between 0.0 (no cohesion) and 1.0 (perfect cohesion)
pub fn calculate_cohesion_score(
    split: &ModuleSplit,
    call_graph: &CallGraph,
    _ownership: &StructOwnershipAnalyzer,
) -> f64 {
    let structs_in_module: HashSet<&str> =
        split.structs_to_move.iter().map(|s| s.as_str()).collect();

    let mut internal_calls = 0;
    let mut external_calls = 0;

    // Iterate through all function calls in the call graph
    for call in call_graph.get_all_calls() {
        // Extract struct and method name from fully qualified function name
        // Format is typically "StructName::method_name"
        let caller_struct = extract_struct_name(&call.caller.name);
        let callee_struct = extract_struct_name(&call.callee.name);

        // Check if caller is in this module
        if let Some(caller_s) = caller_struct {
            if structs_in_module.contains(caller_s.as_str()) {
                // Caller is in this module
                if let Some(callee_s) = callee_struct {
                    if structs_in_module.contains(callee_s.as_str()) {
                        // Callee also in this module -> internal call
                        internal_calls += 1;
                    } else {
                        // Callee in different module -> external call
                        external_calls += 1;
                    }
                } else {
                    // Callee is standalone function or external
                    external_calls += 1;
                }
            }
        }
    }

    // Handle edge cases
    let total_calls = internal_calls + external_calls;
    if total_calls == 0 {
        // No calls - assume perfect cohesion (single-purpose module)
        return 1.0;
    }

    internal_calls as f64 / total_calls as f64
}

/// Extract struct name from a fully qualified function name
///
/// For example: "StructName::method_name" -> Some("StructName")
/// For example: "standalone_function" -> None
fn extract_struct_name(full_name: &str) -> Option<String> {
    let parts: Vec<&str> = full_name.split("::").collect();
    if parts.len() >= 2 {
        // Assume the second-to-last part is the struct name
        // Format: StructName::method or Module::StructName::method
        Some(parts[parts.len() - 2].to_string())
    } else {
        // Standalone function
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organization::god_object_analysis::ModuleSplit;
    use crate::organization::god_object_analysis::Priority;
    use crate::organization::struct_ownership::StructOwnershipAnalyzer;
    use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
    use std::path::PathBuf;

    fn create_test_split(structs: Vec<&str>) -> ModuleSplit {
        ModuleSplit {
            suggested_name: "test_module".to_string(),
            methods_to_move: vec![],
            structs_to_move: structs.iter().map(|s| s.to_string()).collect(),
            responsibility: "test".to_string(),
            estimated_lines: 100,
            method_count: 5,
            warning: None,
            priority: Priority::Medium,
            cohesion_score: None,
            dependencies_in: vec![],
            dependencies_out: vec![],
            domain: String::new(),
            rationale: None,
            method: crate::organization::SplitAnalysisMethod::None,
            severity: None,
            interface_estimate: None,
            classification_evidence: None,
        }
    }

    fn create_test_call_graph(calls: Vec<(&str, &str, &str, &str)>) -> CallGraph {
        let mut graph = CallGraph::new();
        for (caller_struct, caller_method, callee_struct, callee_method) in calls {
            let caller_name = format!("{}::{}", caller_struct, caller_method);
            let callee_name = format!("{}::{}", callee_struct, callee_method);

            let call = FunctionCall {
                caller: FunctionId::new(PathBuf::from("test.rs"), caller_name, 1),
                callee: FunctionId::new(PathBuf::from("test.rs"), callee_name, 10),
                call_type: CallType::Direct,
            };
            graph.add_call(call);
        }
        graph
    }

    fn create_test_ownership(struct_methods: Vec<(&str, Vec<&str>)>) -> StructOwnershipAnalyzer {
        // Create a minimal parsed file for testing
        let code = struct_methods
            .iter()
            .map(|(struct_name, methods)| {
                let methods_code = methods
                    .iter()
                    .map(|m| format!("    pub fn {}(&self) {{}}", m))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!(
                    "struct {} {{}}\nimpl {} {{\n{}\n}}",
                    struct_name, struct_name, methods_code
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let parsed = syn::parse_file(&code).expect("Failed to parse test code");
        StructOwnershipAnalyzer::analyze_file(&parsed)
    }

    #[test]
    fn test_perfect_cohesion() {
        // Module where all calls are internal
        let split = create_test_split(vec!["StructA", "StructB"]);
        let call_graph = create_test_call_graph(vec![
            ("StructA", "m1", "StructB", "m2"),
            ("StructB", "m2", "StructA", "m3"),
        ]);
        let ownership =
            create_test_ownership(vec![("StructA", vec!["m1", "m3"]), ("StructB", vec!["m2"])]);

        let cohesion = calculate_cohesion_score(&split, &call_graph, &ownership);
        assert_eq!(
            cohesion, 1.0,
            "All internal calls should give cohesion of 1.0"
        );
    }

    #[test]
    fn test_zero_cohesion() {
        // Module where all calls are external
        let split = create_test_split(vec!["StructA"]);
        let call_graph = create_test_call_graph(vec![
            ("StructA", "m1", "StructB", "m2"),
            ("StructA", "m2", "StructC", "m1"),
        ]);
        let ownership = create_test_ownership(vec![
            ("StructA", vec!["m1", "m2"]),
            ("StructB", vec!["m2"]),
            ("StructC", vec!["m1"]),
        ]);

        let cohesion = calculate_cohesion_score(&split, &call_graph, &ownership);
        assert_eq!(
            cohesion, 0.0,
            "All external calls should give cohesion of 0.0"
        );
    }

    #[test]
    fn test_mixed_cohesion() {
        // Module with 2 internal and 3 external calls
        let split = create_test_split(vec!["StructA", "StructB"]);
        let call_graph = create_test_call_graph(vec![
            ("StructA", "m1", "StructB", "m1"), // Internal
            ("StructB", "m1", "StructA", "m2"), // Internal
            ("StructA", "m2", "StructC", "m1"), // External
            ("StructA", "m3", "StructD", "m1"), // External
            ("StructB", "m2", "StructE", "m1"), // External
        ]);
        let ownership = create_test_ownership(vec![
            ("StructA", vec!["m1", "m2", "m3"]),
            ("StructB", vec!["m1", "m2"]),
            ("StructC", vec!["m1"]),
            ("StructD", vec!["m1"]),
            ("StructE", vec!["m1"]),
        ]);

        let cohesion = calculate_cohesion_score(&split, &call_graph, &ownership);
        assert_eq!(cohesion, 0.4, "2 internal / 5 total should give 0.4");
    }

    #[test]
    fn test_no_calls_cohesion() {
        // Module with no function calls
        let split = create_test_split(vec!["StructA"]);
        let call_graph = create_test_call_graph(vec![]);
        let ownership = create_test_ownership(vec![("StructA", vec!["m1"])]);

        let cohesion = calculate_cohesion_score(&split, &call_graph, &ownership);
        assert_eq!(cohesion, 1.0, "No calls should default to perfect cohesion");
    }
}
