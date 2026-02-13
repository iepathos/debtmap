/// Dependency analysis for module split recommendations
///
/// This module extracts dependency information to identify which external modules
/// a proposed module depends on and which modules depend on it.
use crate::organization::god_object::types::ModuleSplit;
use crate::organization::struct_ownership::StructOwnershipAnalyzer;
use crate::priority::call_graph::CallGraph;
use std::collections::HashSet;

/// Extract dependencies for a module split
///
/// Identifies which external modules/structs this module depends on
/// and which external modules depend on this module.
///
/// # Arguments
/// * `split` - The module split recommendation
/// * `call_graph` - The function call graph
/// * `ownership` - Struct ownership information
/// * `all_structs` - All struct names in the file (for filtering)
///
/// # Returns
/// Tuple of (dependencies_in, dependencies_out)
pub fn extract_dependencies(
    split: &ModuleSplit,
    call_graph: &CallGraph,
    _ownership: &StructOwnershipAnalyzer,
    all_structs: &[String],
) -> (Vec<String>, Vec<String>) {
    let structs_in_module: HashSet<&str> =
        split.structs_to_move.iter().map(|s| s.as_str()).collect();

    let (deps_in, deps_out) = call_graph
        .get_all_calls()
        .into_iter()
        .filter_map(|call| {
            let caller_s = extract_struct_name(&call.caller.name)?;
            let callee_s = extract_struct_name(&call.callee.name)?;
            Some((caller_s, callee_s))
        })
        .fold(
            (HashSet::new(), HashSet::new()),
            |(mut deps_in, mut deps_out), (caller_s, callee_s)| {
                let caller_in = structs_in_module.contains(caller_s.as_str());
                let callee_in = structs_in_module.contains(callee_s.as_str());

                match (caller_in, callee_in) {
                    (true, false) if all_structs.contains(&callee_s) => {
                        deps_in.insert(callee_s);
                    }
                    (false, true) if all_structs.contains(&caller_s) => {
                        deps_out.insert(caller_s);
                    }
                    _ => {}
                }
                (deps_in, deps_out)
            },
        );

    let mut deps_in: Vec<String> = deps_in.into_iter().collect();
    let mut deps_out: Vec<String> = deps_out.into_iter().collect();
    deps_in.sort();
    deps_out.sort();

    (deps_in, deps_out)
}

/// Estimate the interface size between modules.
///
/// Calculates how many public functions and types would need to be exposed
/// at the module boundary for a proposed split.
///
/// # Arguments
/// * `split` - The module split recommendation
/// * `call_graph` - The function call graph
/// * `ownership` - Struct ownership information
/// * `all_structs` - All struct names in the file
///
/// # Returns
/// InterfaceEstimate with counts of public functions, shared types, and estimated LOC
pub fn estimate_interface_size(
    split: &ModuleSplit,
    call_graph: &CallGraph,
    _ownership: &StructOwnershipAnalyzer,
    all_structs: &[String],
) -> crate::organization::InterfaceEstimate {
    use crate::organization::InterfaceEstimate;

    let structs_in_module: HashSet<&str> =
        split.structs_to_move.iter().map(|s| s.as_str()).collect();

    let (public_functions, shared_types) = call_graph
        .get_all_calls()
        .into_iter()
        .filter_map(|call| {
            let caller_s = extract_struct_name(&call.caller.name)?;
            let callee_s = extract_struct_name(&call.callee.name)?;
            Some((call, caller_s, callee_s))
        })
        .filter(|(_, caller_s, callee_s)| {
            let caller_in = structs_in_module.contains(caller_s.as_str());
            let callee_in = structs_in_module.contains(callee_s.as_str());
            caller_in != callee_in && all_structs.contains(callee_s)
        })
        .fold(
            (HashSet::new(), HashSet::new()),
            |(mut funcs, mut types), (call, _, callee_s)| {
                funcs.insert(call.callee.name.clone());
                types.insert(callee_s);
                (funcs, types)
            },
        );

    // Estimate LOC: ~5 lines per public function signature + ~10 lines per shared type
    let estimated_loc = (public_functions.len() * 5) + (shared_types.len() * 10);

    InterfaceEstimate {
        public_functions_needed: public_functions.len(),
        shared_types: shared_types.len(),
        estimated_loc,
    }
}

/// Extract struct name from a fully qualified function name
///
/// For example: "StructName::method_name" -> Some("StructName")
fn extract_struct_name(full_name: &str) -> Option<String> {
    let parts: Vec<&str> = full_name.split("::").collect();
    if parts.len() >= 2 {
        Some(parts[parts.len() - 2].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organization::god_object::types::{ModuleSplit, Priority};
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
            representative_methods: vec![],
            fields_needed: vec![],
            trait_suggestion: None,
            behavior_category: None,
            ..Default::default()
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
    fn test_dependency_extraction() {
        let split = create_test_split(vec!["StructA", "StructB"]);
        let call_graph = create_test_call_graph(vec![
            ("StructA", "m1", "StructC", "m1"), // We depend on StructC
            ("StructD", "m1", "StructB", "m1"), // StructD depends on us
        ]);
        let ownership = create_test_ownership(vec![
            ("StructA", vec!["m1"]),
            ("StructB", vec!["m1"]),
            ("StructC", vec!["m1"]),
            ("StructD", vec!["m1"]),
        ]);
        let all_structs = vec![
            "StructA".to_string(),
            "StructB".to_string(),
            "StructC".to_string(),
            "StructD".to_string(),
        ];

        let (deps_in, deps_out) =
            extract_dependencies(&split, &call_graph, &ownership, &all_structs);

        assert_eq!(deps_in, vec!["StructC"]);
        assert_eq!(deps_out, vec!["StructD"]);
    }

    #[test]
    fn test_no_dependencies() {
        let split = create_test_split(vec!["StructA"]);
        let call_graph = create_test_call_graph(vec![]);
        let ownership = create_test_ownership(vec![("StructA", vec!["m1"])]);
        let all_structs = vec!["StructA".to_string()];

        let (deps_in, deps_out) =
            extract_dependencies(&split, &call_graph, &ownership, &all_structs);

        assert!(deps_in.is_empty());
        assert!(deps_out.is_empty());
    }

    #[test]
    fn test_multiple_dependencies() {
        let split = create_test_split(vec!["StructA"]);
        let call_graph = create_test_call_graph(vec![
            ("StructA", "m1", "StructB", "m1"),
            ("StructA", "m2", "StructC", "m1"),
            ("StructD", "m1", "StructA", "m1"),
            ("StructE", "m1", "StructA", "m2"),
        ]);
        let ownership = create_test_ownership(vec![
            ("StructA", vec!["m1", "m2"]),
            ("StructB", vec!["m1"]),
            ("StructC", vec!["m1"]),
            ("StructD", vec!["m1"]),
            ("StructE", vec!["m1"]),
        ]);
        let all_structs = vec![
            "StructA".to_string(),
            "StructB".to_string(),
            "StructC".to_string(),
            "StructD".to_string(),
            "StructE".to_string(),
        ];

        let (deps_in, deps_out) =
            extract_dependencies(&split, &call_graph, &ownership, &all_structs);

        assert_eq!(deps_in, vec!["StructB", "StructC"]);
        assert_eq!(deps_out, vec!["StructD", "StructE"]);
    }

    #[test]
    fn test_interface_size_estimation() {
        let split = create_test_split(vec!["StructA", "StructB"]);
        let call_graph = create_test_call_graph(vec![
            ("StructA", "m1", "StructC", "m1"), // A -> C (boundary crossing)
            ("StructB", "m1", "StructC", "m2"), // B -> C (boundary crossing)
            ("StructA", "m2", "StructB", "m1"), // A -> B (internal)
        ]);
        let ownership = create_test_ownership(vec![
            ("StructA", vec!["m1", "m2"]),
            ("StructB", vec!["m1"]),
            ("StructC", vec!["m1", "m2"]),
        ]);
        let all_structs = vec![
            "StructA".to_string(),
            "StructB".to_string(),
            "StructC".to_string(),
        ];

        let estimate = estimate_interface_size(&split, &call_graph, &ownership, &all_structs);

        // Should need 2 public functions (StructC::m1 and StructC::m2)
        assert_eq!(estimate.public_functions_needed, 2);
        // Should have 1 shared type (StructC)
        assert_eq!(estimate.shared_types, 1);
        // Estimated LOC: 2 * 5 + 1 * 10 = 20
        assert_eq!(estimate.estimated_loc, 20);
    }

    #[test]
    fn test_interface_size_no_crossing() {
        let split = create_test_split(vec!["StructA"]);
        let call_graph = create_test_call_graph(vec![
            ("StructA", "m1", "StructA", "m2"), // Internal call only
        ]);
        let ownership = create_test_ownership(vec![("StructA", vec!["m1", "m2"])]);
        let all_structs = vec!["StructA".to_string()];

        let estimate = estimate_interface_size(&split, &call_graph, &ownership, &all_structs);

        assert_eq!(estimate.public_functions_needed, 0);
        assert_eq!(estimate.shared_types, 0);
        assert_eq!(estimate.estimated_loc, 0);
    }
}
