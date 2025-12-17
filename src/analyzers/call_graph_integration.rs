//! Call Graph Integration
//!
//! This module provides functionality to populate call graph data into FunctionMetrics.
//! It bridges the gap between call graph analysis and function metrics to ensure that
//! upstream_callers and downstream_callees fields are properly populated.

use crate::core::FunctionMetrics;
use crate::priority::call_graph::{CallGraph, FunctionId};
use std::collections::HashMap;

/// Populate call graph data into function metrics
///
/// This pure function takes a vector of FunctionMetrics and a CallGraph, then returns
/// a new vector of FunctionMetrics with the call graph fields populated.
pub fn populate_call_graph_data(
    mut function_metrics: Vec<FunctionMetrics>,
    call_graph: &CallGraph,
) -> Vec<FunctionMetrics> {
    // Create a mapping from function metrics to their function IDs for efficient lookup
    let metric_to_id_map: HashMap<usize, FunctionId> = function_metrics
        .iter()
        .enumerate()
        .map(|(idx, metric)| {
            let func_id = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line);
            (idx, func_id)
        })
        .collect();

    // Populate call graph data for each function metric
    for (idx, metric) in function_metrics.iter_mut().enumerate() {
        if let Some(func_id) = metric_to_id_map.get(&idx) {
            // For Python functions, also try with line 0 since the call graph might use that
            let mut upstream_callers: Vec<String> = call_graph
                .get_callers(func_id)
                .into_iter()
                .map(|caller_id| format_function_name(&caller_id))
                .collect();

            let mut downstream_callees: Vec<String> = call_graph
                .get_callees(func_id)
                .into_iter()
                .map(|callee_id| format_function_name(&callee_id))
                .collect();

            // If no results and this is a Python file, try with line 0
            if upstream_callers.is_empty() && downstream_callees.is_empty() {
                let func_id_zero_line =
                    FunctionId::new(func_id.file.clone(), func_id.name.clone(), 0);

                upstream_callers = call_graph
                    .get_callers(&func_id_zero_line)
                    .into_iter()
                    .map(|caller_id| format_function_name(&caller_id))
                    .collect();

                downstream_callees = call_graph
                    .get_callees(&func_id_zero_line)
                    .into_iter()
                    .map(|callee_id| format_function_name(&callee_id))
                    .collect();
            }

            // Update the function metrics with call graph data
            metric.upstream_callers = if upstream_callers.is_empty() {
                None
            } else {
                Some(upstream_callers)
            };

            metric.downstream_callees = if downstream_callees.is_empty() {
                None
            } else {
                Some(downstream_callees)
            };
        }
    }

    function_metrics
}

/// Format a function ID into a human-readable string
///
/// This pure function creates a consistent string representation of a function
/// that includes the file path (just the filename) and the function name.
fn format_function_name(func_id: &FunctionId) -> String {
    let file_name = func_id
        .file
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");

    format!("{}:{}", file_name, func_id.name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_function_metric(name: &str, file: &str, line: usize) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from(file),
            line,
            cyclomatic: 1,
            cognitive: 1,
            nesting: 1,
            length: 10,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        }
    }

    #[test]
    fn test_populate_call_graph_data_empty() {
        let metrics = vec![];
        let call_graph = CallGraph::new();

        let result = populate_call_graph_data(metrics, &call_graph);
        assert!(result.is_empty());
    }

    #[test]
    fn test_populate_call_graph_data_single_function() {
        let metrics = vec![create_test_function_metric("test_func", "test.py", 10)];
        let mut call_graph = CallGraph::new();

        let func_id = FunctionId::new(PathBuf::from("test.py"), "test_func".to_string(), 10);

        call_graph.add_function(func_id.clone(), false, false, 1, 10);

        let result = populate_call_graph_data(metrics, &call_graph);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].upstream_callers, None);
        assert_eq!(result[0].downstream_callees, None);
    }

    #[test]
    fn test_populate_call_graph_data_with_calls() {
        let metrics = vec![
            create_test_function_metric("caller", "test.py", 5),
            create_test_function_metric("callee", "test.py", 15),
        ];

        let mut call_graph = CallGraph::new();

        let caller_id = FunctionId::new(PathBuf::from("test.py"), "caller".to_string(), 5);

        let callee_id = FunctionId::new(PathBuf::from("test.py"), "callee".to_string(), 15);

        call_graph.add_function(caller_id.clone(), false, false, 1, 10);
        call_graph.add_function(callee_id.clone(), false, false, 1, 10);
        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: caller_id.clone(),
            callee: callee_id.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });

        let result = populate_call_graph_data(metrics, &call_graph);

        assert_eq!(result.len(), 2);

        // Check caller function
        let caller_metric = &result[0];
        assert_eq!(caller_metric.name, "caller");
        assert_eq!(caller_metric.upstream_callers, None);
        assert_eq!(
            caller_metric.downstream_callees,
            Some(vec!["test.py:callee".to_string()])
        );

        // Check callee function
        let callee_metric = &result[1];
        assert_eq!(callee_metric.name, "callee");
        assert_eq!(
            callee_metric.upstream_callers,
            Some(vec!["test.py:caller".to_string()])
        );
        assert_eq!(callee_metric.downstream_callees, None);
    }

    #[test]
    fn test_format_function_name() {
        let func_id = FunctionId::new(
            PathBuf::from("/path/to/test.py"),
            "my_function".to_string(),
            10,
        );

        let formatted = format_function_name(&func_id);
        assert_eq!(formatted, "test.py:my_function");
    }

    /// Test cross-file call graph population.
    ///
    /// This reproduces the scenario where:
    /// - diagnose_coverage.rs has function `diagnose_coverage_file` calling `parse_lcov_file`
    /// - lcov.rs has function `parse_lcov_file`
    ///
    /// The call graph should properly populate upstream_callers and downstream_callees.
    #[test]
    fn test_populate_call_graph_data_cross_file() {
        use crate::analyzers::rust_call_graph::extract_call_graph_multi_file;

        // Create function metrics that match the call graph
        let metrics = vec![
            create_test_function_metric(
                "diagnose_coverage_file",
                "src/commands/diagnose_coverage.rs",
                6,
            ),
            create_test_function_metric(
                "generate_suggestions",
                "src/commands/diagnose_coverage.rs",
                12,
            ),
            create_test_function_metric("parse_lcov_file", "src/risk/lcov.rs", 8),
        ];

        // Build call graph from source code
        let file1_code = r#"
            use crate::risk::lcov::parse_lcov_file;
            use anyhow::Result;
            use std::path::Path;

            pub fn diagnose_coverage_file(lcov_path: &Path, format: &str) -> Result<()> {
                let lcov_data = parse_lcov_file(lcov_path)?;
                let total_files = lcov_data.functions.len();
                let suggestions = generate_suggestions(total_files);
                Ok(())
            }

            fn generate_suggestions(total_files: usize) -> Vec<String> {
                vec![]
            }
        "#;

        let file2_code = r#"
            use anyhow::Result;
            use std::path::Path;

            pub struct LcovData {
                pub functions: std::collections::HashMap<String, Vec<()>>,
            }

            pub fn parse_lcov_file(path: &Path) -> Result<LcovData> {
                Ok(LcovData {
                    functions: std::collections::HashMap::new(),
                })
            }
        "#;

        let file1 = syn::parse_str::<syn::File>(file1_code).expect("Failed to parse file1");
        let file2 = syn::parse_str::<syn::File>(file2_code).expect("Failed to parse file2");

        let files = vec![
            (file1, PathBuf::from("src/commands/diagnose_coverage.rs")),
            (file2, PathBuf::from("src/risk/lcov.rs")),
        ];

        let call_graph = extract_call_graph_multi_file(&files);

        // Debug: print call graph state
        eprintln!("Call graph functions:");
        for func in call_graph.get_all_functions() {
            let callers_vec = call_graph.get_callers(func);
            let callers: Vec<_> = callers_vec.iter().map(|f| &f.name).collect();
            let callees_vec = call_graph.get_callees(func);
            let callees: Vec<_> = callees_vec.iter().map(|f| &f.name).collect();
            eprintln!(
                "  {}:{} (line {}) - callers: {:?}, callees: {:?}",
                func.file.display(),
                func.name,
                func.line,
                callers,
                callees
            );
        }

        // Populate the metrics with call graph data
        let result = populate_call_graph_data(metrics, &call_graph);

        // Debug: print populated metrics
        eprintln!("\nPopulated metrics:");
        for metric in &result {
            eprintln!(
                "  {}:{} (line {}) - upstream: {:?}, downstream: {:?}",
                metric.file.display(),
                metric.name,
                metric.line,
                metric.upstream_callers,
                metric.downstream_callees
            );
        }

        // Find diagnose_coverage_file
        let diagnose_metric = result
            .iter()
            .find(|m| m.name == "diagnose_coverage_file")
            .expect("diagnose_coverage_file should exist");

        // Should have downstream callees
        assert!(
            diagnose_metric.downstream_callees.is_some(),
            "diagnose_coverage_file should have downstream_callees. Got: {:?}",
            diagnose_metric.downstream_callees
        );

        let callees = diagnose_metric.downstream_callees.as_ref().unwrap();
        eprintln!("diagnose_coverage_file callees: {:?}", callees);

        // Should include parse_lcov_file
        assert!(
            callees.iter().any(|c| c.contains("parse_lcov_file")),
            "diagnose_coverage_file should call parse_lcov_file. Found: {:?}",
            callees
        );

        // Should include generate_suggestions
        assert!(
            callees.iter().any(|c| c.contains("generate_suggestions")),
            "diagnose_coverage_file should call generate_suggestions. Found: {:?}",
            callees
        );

        // Find parse_lcov_file
        let parse_metric = result
            .iter()
            .find(|m| m.name == "parse_lcov_file")
            .expect("parse_lcov_file should exist");

        // Should have upstream callers
        assert!(
            parse_metric.upstream_callers.is_some(),
            "parse_lcov_file should have upstream_callers. Got: {:?}",
            parse_metric.upstream_callers
        );

        let callers = parse_metric.upstream_callers.as_ref().unwrap();
        assert!(
            callers.iter().any(|c| c.contains("diagnose_coverage_file")),
            "parse_lcov_file should be called by diagnose_coverage_file. Found: {:?}",
            callers
        );
    }

    /// Test that call graph integration works even with mismatched line numbers.
    ///
    /// In the real world, FunctionMetrics may have different line numbers than what
    /// the call graph extractor discovers (due to comments, attributes, etc.).
    /// The fuzzy matching should still find the right functions.
    #[test]
    fn test_populate_call_graph_data_with_mismatched_lines() {
        use crate::analyzers::rust_call_graph::extract_call_graph_multi_file;

        // Function metrics with DIFFERENT line numbers than the call graph will have
        // (simulating the real world where AST line != call graph line)
        let metrics = vec![
            create_test_function_metric(
                "diagnose_coverage_file",
                "src/commands/diagnose_coverage.rs",
                63, // Real line in actual file
            ),
            create_test_function_metric(
                "generate_suggestions",
                "src/commands/diagnose_coverage.rs",
                202, // Real line in actual file
            ),
            create_test_function_metric("parse_lcov_file", "src/risk/lcov.rs", 268),
        ];

        // Build call graph from source code - this will use line 6, 12, 8
        let file1_code = r#"
            use crate::risk::lcov::parse_lcov_file;
            use anyhow::Result;
            use std::path::Path;

            pub fn diagnose_coverage_file(lcov_path: &Path, format: &str) -> Result<()> {
                let lcov_data = parse_lcov_file(lcov_path)?;
                let total_files = lcov_data.functions.len();
                let suggestions = generate_suggestions(total_files);
                Ok(())
            }

            fn generate_suggestions(total_files: usize) -> Vec<String> {
                vec![]
            }
        "#;

        let file2_code = r#"
            use anyhow::Result;
            use std::path::Path;

            pub struct LcovData {
                pub functions: std::collections::HashMap<String, Vec<()>>,
            }

            pub fn parse_lcov_file(path: &Path) -> Result<LcovData> {
                Ok(LcovData {
                    functions: std::collections::HashMap::new(),
                })
            }
        "#;

        let file1 = syn::parse_str::<syn::File>(file1_code).expect("Failed to parse file1");
        let file2 = syn::parse_str::<syn::File>(file2_code).expect("Failed to parse file2");

        let files = vec![
            (file1, PathBuf::from("src/commands/diagnose_coverage.rs")),
            (file2, PathBuf::from("src/risk/lcov.rs")),
        ];

        let call_graph = extract_call_graph_multi_file(&files);

        // Debug: print call graph state
        eprintln!("\nCall graph functions (with their extracted line numbers):");
        for func in call_graph.get_all_functions() {
            eprintln!(
                "  {}:{} (line {})",
                func.file.display(),
                func.name,
                func.line
            );
        }

        // Populate the metrics with call graph data
        let result = populate_call_graph_data(metrics, &call_graph);

        // Debug: print populated metrics
        eprintln!("\nPopulated metrics (with their DIFFERENT line numbers):");
        for metric in &result {
            eprintln!(
                "  {}:{} (line {}) - upstream: {:?}, downstream: {:?}",
                metric.file.display(),
                metric.name,
                metric.line,
                metric.upstream_callers,
                metric.downstream_callees
            );
        }

        // Find diagnose_coverage_file
        let diagnose_metric = result
            .iter()
            .find(|m| m.name == "diagnose_coverage_file")
            .expect("diagnose_coverage_file should exist");

        // This is the KEY assertion - even with mismatched line numbers,
        // fuzzy matching should still work
        assert!(
            diagnose_metric.downstream_callees.is_some(),
            "BUG: diagnose_coverage_file should have downstream_callees even with mismatched line numbers. Got: {:?}",
            diagnose_metric.downstream_callees
        );

        let callees = diagnose_metric.downstream_callees.as_ref().unwrap();
        assert!(
            callees.iter().any(|c| c.contains("parse_lcov_file")),
            "BUG: diagnose_coverage_file should call parse_lcov_file. Found: {:?}",
            callees
        );
    }
}
