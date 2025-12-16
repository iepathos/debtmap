//! Call graph adapter for building CallGraph from extracted data.
//!
//! This module provides pure conversion functions that transform `ExtractedFileData`
//! into a `CallGraph` suitable for the existing analysis pipeline.
//!
//! # Design
//!
//! All functions in this module are pure (no I/O, no parsing). The call graph
//! construction is O(n*m) where n is the number of functions and m is the average
//! number of calls per function.
//!
//! # Pipeline Usage
//!
//! The main analysis pipeline uses `crate::builders::parallel_call_graph::build_call_graph_from_extracted`
//! which returns additional metadata (framework_exclusions, function_pointer_used).
//! This adapter provides a simpler pure implementation useful for testing and
//! standalone call graph analysis.

use crate::extraction::types::{CallSite, CallType as ExtractedCallType, ExtractedFileData};
use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use std::collections::HashMap;
use std::path::PathBuf;

/// Build a CallGraph from extracted file data.
///
/// This is a pure function with no file I/O.
///
/// # Arguments
///
/// * `extracted` - Map of file paths to extracted file data
///
/// # Returns
///
/// A fully constructed `CallGraph` with function nodes and call edges.
pub fn build_call_graph(extracted: &HashMap<PathBuf, ExtractedFileData>) -> CallGraph {
    let mut graph = CallGraph::new();

    // First pass: add all function nodes
    for (path, file_data) in extracted {
        for func in &file_data.functions {
            // Use qualified_name for method disambiguation (e.g., "Type::method")
            let func_id = FunctionId::new(path.clone(), func.qualified_name.clone(), func.line);

            graph.add_function(
                func_id,
                is_entry_point(&func.name),
                func.is_test,
                func.cyclomatic,
                func.length,
            );
        }
    }

    // Second pass: add call edges
    for (path, file_data) in extracted {
        for func in &file_data.functions {
            let caller_id = FunctionId::new(path.clone(), func.qualified_name.clone(), func.line);

            for call in &func.calls {
                if let Some(callee_id) = resolve_call(path, call, extracted) {
                    let call_type = convert_call_type(call.call_type);
                    graph.add_call(FunctionCall {
                        caller: caller_id.clone(),
                        callee: callee_id,
                        call_type,
                    });
                }
            }
        }
    }

    graph
}

/// Check if a function name indicates an entry point.
///
/// Entry points include main functions and common handler patterns.
fn is_entry_point(name: &str) -> bool {
    name == "main"
        || name.starts_with("handle_")
        || name.starts_with("run_")
        || name.starts_with("start_")
        || name.starts_with("process_")
}

/// Convert extracted CallType to call graph CallType.
fn convert_call_type(call_type: ExtractedCallType) -> CallType {
    match call_type {
        ExtractedCallType::Direct => CallType::Direct,
        ExtractedCallType::Method => CallType::Direct,
        ExtractedCallType::StaticMethod => CallType::Direct,
        ExtractedCallType::TraitMethod => CallType::Delegate,
        ExtractedCallType::Closure => CallType::Callback,
        ExtractedCallType::FunctionPointer => CallType::Callback,
    }
}

/// Resolve a call site to a FunctionId.
///
/// Attempts to find the callee in the same file first, then searches
/// other files for cross-file calls.
fn resolve_call(
    caller_file: &PathBuf,
    call: &CallSite,
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> Option<FunctionId> {
    // Try same file first (most common case)
    if let Some(file_data) = extracted.get(caller_file) {
        if let Some(func) = find_function_by_name(file_data, &call.callee_name) {
            return Some(FunctionId::new(
                caller_file.clone(),
                func.name.clone(),
                func.line,
            ));
        }
    }

    // Try cross-file resolution for direct and static method calls
    match call.call_type {
        ExtractedCallType::Direct | ExtractedCallType::StaticMethod => {
            // Search all files for matching function
            for (path, file_data) in extracted {
                if path == caller_file {
                    continue;
                }
                if let Some(func) = find_function_by_name(file_data, &call.callee_name) {
                    return Some(FunctionId::new(path.clone(), func.name.clone(), func.line));
                }
            }
        }
        ExtractedCallType::Method | ExtractedCallType::TraitMethod => {
            // Method calls are harder to resolve without type info
            // Return None and let downstream analysis handle it
        }
        _ => {}
    }

    None
}

/// Find a function in extracted file data by name.
///
/// Supports matching by:
/// - Exact name
/// - Qualified name
/// - Suffix match (for qualified names like `Type::method`)
fn find_function_by_name<'a>(
    file_data: &'a ExtractedFileData,
    name: &str,
) -> Option<&'a crate::extraction::types::ExtractedFunctionData> {
    file_data.functions.iter().find(|f| {
        f.name == name
            || f.qualified_name == name
            || f.qualified_name.ends_with(&format!("::{}", name))
    })
}

/// Merge call graph data from extracted files into an existing graph.
///
/// # Arguments
///
/// * `graph` - Existing call graph to merge into
/// * `extracted` - New extracted file data to add
pub fn merge_into_call_graph(
    graph: &mut CallGraph,
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) {
    let new_graph = build_call_graph(extracted);
    graph.merge(new_graph);
}

/// Build a call graph from a single file's extracted data.
///
/// Useful for incremental updates or single-file analysis.
///
/// # Arguments
///
/// * `file_data` - Extracted data from a single file
///
/// # Returns
///
/// A `CallGraph` containing only the functions and internal calls from this file.
pub fn build_single_file_graph(file_data: &ExtractedFileData) -> CallGraph {
    let mut extracted = HashMap::new();
    extracted.insert(file_data.path.clone(), file_data.clone());
    build_call_graph(&extracted)
}

/// Count the number of call edges that can be resolved.
///
/// Useful for validation and debugging.
pub fn count_resolvable_calls(extracted: &HashMap<PathBuf, ExtractedFileData>) -> (usize, usize) {
    let mut resolved = 0;
    let mut unresolved = 0;

    for (path, file_data) in extracted {
        for func in &file_data.functions {
            for call in &func.calls {
                if resolve_call(path, call, extracted).is_some() {
                    resolved += 1;
                } else {
                    unresolved += 1;
                }
            }
        }
    }

    (resolved, unresolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extraction::types::{
        CallSite, CallType as ExtCallType, ExtractedFileData, ExtractedFunctionData,
        PurityAnalysisData,
    };

    fn create_test_function(name: &str, line: usize) -> ExtractedFunctionData {
        ExtractedFunctionData {
            name: name.to_string(),
            qualified_name: name.to_string(),
            line,
            end_line: line + 10,
            length: 10,
            cyclomatic: 5,
            cognitive: 3,
            nesting: 2,
            purity_analysis: PurityAnalysisData::pure(),
            io_operations: vec![],
            parameter_names: vec![],
            transformation_patterns: vec![],
            calls: vec![],
            is_test: false,
            is_async: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
        }
    }

    fn create_test_file() -> ExtractedFileData {
        ExtractedFileData {
            path: PathBuf::from("src/main.rs"),
            functions: vec![
                create_test_function("foo", 1),
                create_test_function("bar", 20),
            ],
            structs: vec![],
            impls: vec![],
            imports: vec![],
            total_lines: 50,
            detected_patterns: vec![],
            test_lines: 0, // Spec 214
        }
    }

    #[test]
    fn test_build_call_graph_nodes() {
        let mut extracted = HashMap::new();
        extracted.insert(PathBuf::from("src/main.rs"), create_test_file());

        let graph = build_call_graph(&extracted);

        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn test_build_call_graph_edges() {
        let mut extracted = HashMap::new();
        let mut file_data = create_test_file();

        // Add a call from foo to bar
        file_data.functions[0].calls.push(CallSite {
            callee_name: "bar".to_string(),
            call_type: ExtCallType::Direct,
            line: 5,
        });

        extracted.insert(PathBuf::from("src/main.rs"), file_data);

        let graph = build_call_graph(&extracted);

        let foo_id = FunctionId::new(PathBuf::from("src/main.rs"), "foo".to_string(), 1);
        let callees = graph.get_callees(&foo_id);

        assert_eq!(callees.len(), 1);
        assert_eq!(callees[0].name, "bar");
    }

    #[test]
    fn test_is_entry_point() {
        assert!(is_entry_point("main"));
        assert!(is_entry_point("handle_request"));
        assert!(is_entry_point("run_server"));
        assert!(is_entry_point("start_worker"));
        assert!(is_entry_point("process_item"));
        assert!(!is_entry_point("helper"));
        assert!(!is_entry_point("calculate"));
    }

    #[test]
    fn test_cross_file_call_resolution() {
        let mut extracted = HashMap::new();

        let mut file1 = create_test_file();
        file1.path = PathBuf::from("src/caller.rs");
        file1.functions[0].calls.push(CallSite {
            callee_name: "helper".to_string(),
            call_type: ExtCallType::Direct,
            line: 5,
        });

        let mut file2 = ExtractedFileData::empty(PathBuf::from("src/utils.rs"));
        file2.functions.push(create_test_function("helper", 1));

        extracted.insert(PathBuf::from("src/caller.rs"), file1);
        extracted.insert(PathBuf::from("src/utils.rs"), file2);

        let graph = build_call_graph(&extracted);

        let caller_id = FunctionId::new(PathBuf::from("src/caller.rs"), "foo".to_string(), 1);
        let callees = graph.get_callees(&caller_id);

        assert_eq!(callees.len(), 1);
        assert_eq!(callees[0].name, "helper");
        assert_eq!(callees[0].file, PathBuf::from("src/utils.rs"));
    }

    #[test]
    fn test_count_resolvable_calls() {
        let mut extracted = HashMap::new();
        let mut file_data = create_test_file();

        // Add one resolvable and one unresolvable call
        file_data.functions[0].calls.push(CallSite {
            callee_name: "bar".to_string(),
            call_type: ExtCallType::Direct,
            line: 5,
        });
        file_data.functions[0].calls.push(CallSite {
            callee_name: "external_fn".to_string(),
            call_type: ExtCallType::Direct,
            line: 6,
        });

        extracted.insert(PathBuf::from("src/main.rs"), file_data);

        let (resolved, unresolved) = count_resolvable_calls(&extracted);

        assert_eq!(resolved, 1);
        assert_eq!(unresolved, 1);
    }

    #[test]
    fn test_single_file_graph() {
        let file_data = create_test_file();
        let graph = build_single_file_graph(&file_data);

        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn test_convert_call_types() {
        assert!(matches!(
            convert_call_type(ExtCallType::Direct),
            CallType::Direct
        ));
        assert!(matches!(
            convert_call_type(ExtCallType::Method),
            CallType::Direct
        ));
        assert!(matches!(
            convert_call_type(ExtCallType::TraitMethod),
            CallType::Delegate
        ));
        assert!(matches!(
            convert_call_type(ExtCallType::Closure),
            CallType::Callback
        ));
    }

    #[test]
    fn test_qualified_name_resolution() {
        let mut extracted = HashMap::new();
        let mut file_data = create_test_file();

        // Set qualified name for bar
        file_data.functions[1].qualified_name = "MyStruct::bar".to_string();

        // Add call using qualified name
        file_data.functions[0].calls.push(CallSite {
            callee_name: "MyStruct::bar".to_string(),
            call_type: ExtCallType::StaticMethod,
            line: 5,
        });

        extracted.insert(PathBuf::from("src/main.rs"), file_data);

        let graph = build_call_graph(&extracted);

        let foo_id = FunctionId::new(PathBuf::from("src/main.rs"), "foo".to_string(), 1);
        let callees = graph.get_callees(&foo_id);

        assert_eq!(callees.len(), 1);
    }

    #[test]
    fn test_merge_into_call_graph() {
        let mut graph = CallGraph::new();

        let func_id = FunctionId::new(PathBuf::from("existing.rs"), "existing".to_string(), 1);
        graph.add_function(func_id, false, false, 1, 5);

        let mut extracted = HashMap::new();
        extracted.insert(PathBuf::from("src/main.rs"), create_test_file());

        merge_into_call_graph(&mut graph, &extracted);

        assert_eq!(graph.node_count(), 3); // 1 existing + 2 new
    }

    #[test]
    fn test_empty_extracted() {
        let extracted: HashMap<PathBuf, ExtractedFileData> = HashMap::new();
        let graph = build_call_graph(&extracted);

        assert!(graph.is_empty());
    }

    #[test]
    fn test_test_function_marking() {
        let mut extracted = HashMap::new();
        let mut file_data = create_test_file();
        file_data.functions[0].is_test = true;

        extracted.insert(PathBuf::from("src/main.rs"), file_data);

        let graph = build_call_graph(&extracted);

        let test_func_id = FunctionId::new(PathBuf::from("src/main.rs"), "foo".to_string(), 1);
        assert!(graph.is_test_function(&test_func_id));
    }
}
