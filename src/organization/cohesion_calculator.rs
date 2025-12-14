/// Cohesion score calculation for module split recommendations
///
/// This module calculates how tightly related the methods within a proposed module are
/// by analyzing function call patterns. High cohesion indicates methods work together frequently.
use crate::organization::god_object::types::ModuleSplit;
use crate::organization::struct_ownership::StructOwnershipAnalyzer;
use crate::priority::call_graph::CallGraph;
use std::collections::HashSet;
use std::path::Path;

/// Result of file-level cohesion calculation (spec 198)
#[derive(Debug, Clone)]
pub struct FileCohesionResult {
    /// Cohesion score between 0.0 and 1.0
    pub score: f64,
    /// Number of internal function calls (within the same file)
    pub internal_calls: usize,
    /// Number of external function calls (to other files)
    pub external_calls: usize,
    /// Number of functions in the file that were analyzed
    pub functions_analyzed: usize,
}

/// Minimum number of functions required for cohesion calculation (spec 198)
const MIN_FUNCTIONS_FOR_COHESION: usize = 3;

/// Calculate cohesion for a file based on function call patterns (spec 198)
///
/// File-level cohesion measures how tightly related the functions within a file are.
/// High cohesion (>0.7) indicates functions work together frequently.
/// Low cohesion (<0.4) suggests the file contains unrelated functionality.
///
/// Formula: cohesion = internal_calls / (internal_calls + external_calls)
///
/// # Arguments
/// * `file_path` - Path to the file being analyzed
/// * `call_graph` - The function call graph for the codebase
///
/// # Returns
/// `Some(FileCohesionResult)` if the file has 3+ functions, `None` otherwise
pub fn calculate_file_cohesion(
    file_path: &Path,
    call_graph: &CallGraph,
) -> Option<FileCohesionResult> {
    // Find all functions in this file
    let file_functions: HashSet<String> = call_graph
        .get_all_functions()
        .filter(|func| func.file == file_path)
        .map(|func| func.name.clone())
        .collect();

    let functions_count = file_functions.len();

    // Only calculate cohesion if file has enough functions
    if functions_count < MIN_FUNCTIONS_FOR_COHESION {
        return None;
    }

    let mut internal_calls = 0;
    let mut external_calls = 0;

    // Analyze all function calls
    for call in call_graph.get_all_calls() {
        // Only consider calls FROM functions in this file
        if call.caller.file != file_path {
            continue;
        }

        // Check if the callee is in the same file (internal) or different file (external)
        if call.callee.file == file_path {
            internal_calls += 1;
        } else {
            external_calls += 1;
        }
    }

    // Calculate cohesion score
    let total_calls = internal_calls + external_calls;
    let score = if total_calls == 0 {
        // No calls - assume perfect cohesion (self-contained functions)
        1.0
    } else {
        internal_calls as f64 / total_calls as f64
    };

    Some(FileCohesionResult {
        score,
        internal_calls,
        external_calls,
        functions_analyzed: functions_count,
    })
}

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
    use crate::organization::god_object::types::ModuleSplit;
    use crate::organization::god_object::types::Priority;
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

    // Tests for file-level cohesion calculation (spec 198)

    fn create_test_file_call_graph(calls: Vec<(&str, &str, &str, &str)>) -> CallGraph {
        let mut graph = CallGraph::new();
        let file_a = PathBuf::from("src/file_a.rs");
        let file_b = PathBuf::from("src/file_b.rs");

        // First, add all functions as nodes
        for (caller_file, caller_func, callee_file, callee_func) in &calls {
            let caller_path = if *caller_file == "A" {
                &file_a
            } else {
                &file_b
            };
            let callee_path = if *callee_file == "A" {
                &file_a
            } else {
                &file_b
            };

            graph.add_function(
                FunctionId::new(caller_path.clone(), (*caller_func).to_string(), 1),
                false,
                false,
                5,
                20,
            );
            graph.add_function(
                FunctionId::new(callee_path.clone(), (*callee_func).to_string(), 10),
                false,
                false,
                5,
                20,
            );
        }

        // Then add calls
        for (caller_file, caller_func, callee_file, callee_func) in calls {
            let caller_path = if caller_file == "A" { &file_a } else { &file_b };
            let callee_path = if callee_file == "A" { &file_a } else { &file_b };

            let call = FunctionCall {
                caller: FunctionId::new(caller_path.clone(), caller_func.to_string(), 1),
                callee: FunctionId::new(callee_path.clone(), callee_func.to_string(), 10),
                call_type: CallType::Direct,
            };
            graph.add_call(call);
        }
        graph
    }

    #[test]
    fn test_file_cohesion_perfect() {
        // All calls are internal (within file A)
        let calls = vec![
            ("A", "func1", "A", "func2"),
            ("A", "func2", "A", "func3"),
            ("A", "func3", "A", "func1"),
        ];
        let graph = create_test_file_call_graph(calls);
        let result = calculate_file_cohesion(Path::new("src/file_a.rs"), &graph);

        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.score, 1.0);
        assert_eq!(r.internal_calls, 3);
        assert_eq!(r.external_calls, 0);
        assert_eq!(r.functions_analyzed, 3);
    }

    #[test]
    fn test_file_cohesion_low() {
        // All calls from A go to B (external)
        let calls = vec![
            ("A", "func1", "B", "helper1"),
            ("A", "func2", "B", "helper2"),
            ("A", "func3", "B", "helper3"),
        ];
        let graph = create_test_file_call_graph(calls);
        let result = calculate_file_cohesion(Path::new("src/file_a.rs"), &graph);

        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.score, 0.0);
        assert_eq!(r.internal_calls, 0);
        assert_eq!(r.external_calls, 3);
        assert_eq!(r.functions_analyzed, 3);
    }

    #[test]
    fn test_file_cohesion_mixed() {
        // Mix of internal and external calls
        let calls = vec![
            ("A", "func1", "A", "func2"),  // internal
            ("A", "func2", "A", "func3"),  // internal
            ("A", "func3", "B", "helper"), // external
        ];
        let graph = create_test_file_call_graph(calls);
        let result = calculate_file_cohesion(Path::new("src/file_a.rs"), &graph);

        assert!(result.is_some());
        let r = result.unwrap();
        // 2 internal / 3 total = 0.666...
        assert!((r.score - 0.6666).abs() < 0.01);
        assert_eq!(r.internal_calls, 2);
        assert_eq!(r.external_calls, 1);
    }

    #[test]
    fn test_file_cohesion_too_few_functions() {
        // Only 2 functions - below minimum threshold of 3
        let mut graph = CallGraph::new();
        let file = PathBuf::from("src/small.rs");
        graph.add_function(
            FunctionId::new(file.clone(), "func1".to_string(), 1),
            false,
            false,
            5,
            20,
        );
        graph.add_function(
            FunctionId::new(file.clone(), "func2".to_string(), 10),
            false,
            false,
            5,
            20,
        );

        let result = calculate_file_cohesion(Path::new("src/small.rs"), &graph);
        assert!(
            result.is_none(),
            "Files with <3 functions should return None"
        );
    }

    #[test]
    fn test_file_cohesion_no_calls() {
        // 3 functions but no calls between them
        let mut graph = CallGraph::new();
        let file = PathBuf::from("src/isolated.rs");
        graph.add_function(
            FunctionId::new(file.clone(), "func1".to_string(), 1),
            false,
            false,
            5,
            20,
        );
        graph.add_function(
            FunctionId::new(file.clone(), "func2".to_string(), 10),
            false,
            false,
            5,
            20,
        );
        graph.add_function(
            FunctionId::new(file.clone(), "func3".to_string(), 20),
            false,
            false,
            5,
            20,
        );

        let result = calculate_file_cohesion(Path::new("src/isolated.rs"), &graph);
        assert!(result.is_some());
        let r = result.unwrap();
        // No calls = perfect cohesion (self-contained functions)
        assert_eq!(r.score, 1.0);
        assert_eq!(r.internal_calls, 0);
        assert_eq!(r.external_calls, 0);
    }
}
