//! Data flow graph population from various analysis sources
//!
//! This module provides functions to populate the DataFlowGraph with data from:
//! - Purity analysis (CFG-based data flow analysis)
//! - I/O operation detection (AST-based pattern matching)
//! - Variable dependency analysis
//! - Data transformation patterns
//!
//! # Spec 213: Extracted Data Functions
//!
//! This module now includes `*_from_extracted` variants that populate the
//! DataFlowGraph from pre-extracted data, avoiding per-function file parsing.
//! These functions should be preferred when extracted data is available.

use crate::analyzers::purity_detector::PurityAnalysis;
use crate::data_flow::{DataFlowGraph, MutationInfo, PurityInfo};
use crate::extraction::ExtractedFileData;
use crate::priority::call_graph::FunctionId;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Helper module for finding functions in AST (spec 202)
///
/// Handles both top-level `fn` items and methods inside `impl` blocks.
mod ast_helpers {
    use syn::{ImplItemFn, ItemFn};

    /// Result of finding a function in the AST
    pub enum FoundFunction<'a> {
        /// Top-level function
        TopLevel(&'a ItemFn),
        /// Method in an impl block
        ImplMethod(&'a ImplItemFn),
    }

    impl<'a> FoundFunction<'a> {
        /// Get the function block
        pub fn block(&self) -> &syn::Block {
            match self {
                FoundFunction::TopLevel(f) => &f.block,
                FoundFunction::ImplMethod(m) => &m.block,
            }
        }

        /// Get the function signature inputs for parameter extraction
        pub fn inputs(&self) -> impl Iterator<Item = &syn::FnArg> {
            match self {
                FoundFunction::TopLevel(f) => f.sig.inputs.iter(),
                FoundFunction::ImplMethod(m) => m.sig.inputs.iter(),
            }
        }
    }

    /// Find a function in the AST by name and line number.
    ///
    /// Searches both top-level functions (`syn::Item::Fn`) and methods
    /// inside impl blocks (`syn::Item::Impl` -> `syn::ImplItem::Fn`).
    ///
    /// # Name Matching
    ///
    /// The `metric_name` parameter may be in one of two formats:
    /// - Simple name: `"method_name"` (for top-level functions)
    /// - Qualified name: `"TypeName::method_name"` (for impl methods)
    ///
    /// For impl methods, we match if either:
    /// - The metric name equals the simple method name
    /// - The metric name ends with `::method_name`
    ///
    /// # Spec 213
    ///
    /// This function is used by the fallback purity analysis path when pre-extracted
    /// data is not available. When `with_extracted_data()` is called on the builder,
    /// this function is not used. It's kept for backward compatibility with non-parallel
    /// analysis paths and testing.
    pub fn find_function_in_ast<'a>(
        ast: &'a syn::File,
        metric_name: &str,
        line: usize,
    ) -> Option<FoundFunction<'a>> {
        for item in &ast.items {
            match item {
                syn::Item::Fn(item_fn) => {
                    // FunctionMetrics.line uses 1-indexed span.start().line,
                    // so compare directly without converting to 0-indexed
                    let span_line = item_fn.sig.ident.span().start().line;
                    // For top-level functions, metric_name should match exactly
                    // or be the simple part of a qualified name
                    let fn_name = item_fn.sig.ident.to_string();
                    if span_line == line && (metric_name == fn_name) {
                        return Some(FoundFunction::TopLevel(item_fn));
                    }
                }
                syn::Item::Impl(item_impl) => {
                    for impl_item in &item_impl.items {
                        if let syn::ImplItem::Fn(method) = impl_item {
                            // FunctionMetrics.line uses 1-indexed span.start().line,
                            // so compare directly without converting to 0-indexed
                            let span_line = method.sig.ident.span().start().line;
                            let method_name = method.sig.ident.to_string();
                            // Match if:
                            // 1. Exact match with simple method name
                            // 2. Metric name ends with ::method_name (qualified format)
                            let matches_name = metric_name == method_name
                                || metric_name.ends_with(&format!("::{}", method_name));

                            if span_line == line && matches_name {
                                return Some(FoundFunction::ImplMethod(method));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }
}

pub use ast_helpers::{find_function_in_ast, FoundFunction};

/// Populate DataFlowGraph from purity analysis results
///
/// Extracts and stores:
/// - Full CFG-based data flow analysis with variable name context
/// - Mutation information
/// - Escape analysis results
pub fn populate_from_purity_analysis(
    data_flow: &mut DataFlowGraph,
    func_id: &FunctionId,
    purity: &PurityAnalysis,
) {
    // Store full CFG analysis with context if available
    if let Some(cfg_analysis) = &purity.data_flow_info {
        // Store raw analysis (for backward compatibility)
        data_flow.set_cfg_analysis(func_id.clone(), cfg_analysis.clone());

        // Store analysis with var_names context for translation
        use crate::data_flow::CfgAnalysisWithContext;
        let context = CfgAnalysisWithContext::new(purity.var_names.clone(), cfg_analysis.clone());
        data_flow.set_cfg_analysis_with_context(func_id.clone(), context);
    }

    // Extract mutation information using binary signals (spec 257)
    // Escape analysis removed - not providing actionable debt signals
    let detected_mutations: Vec<String> = purity
        .live_mutations
        .iter()
        .map(|m| m.target.clone())
        .collect();

    let mutation_info = MutationInfo {
        has_mutations: !detected_mutations.is_empty() || purity.total_mutations > 0,
        detected_mutations,
    };

    data_flow.set_mutation_info(func_id.clone(), mutation_info);
}

// Note: Old per-function parsing functions (populate_io_operations, populate_variable_dependencies,
// populate_data_transformations) were removed in Spec 213. Use the *_from_extracted variants instead
// which work with pre-extracted data from the unified extraction pipeline.

// ============================================================================
// Spec 213: Extracted Data Functions
// ============================================================================

/// Populate purity analysis from extracted file data (pure).
///
/// # Spec 213
///
/// Populates the DataFlowGraph with purity information from pre-extracted data.
/// Avoids per-function file parsing by using the extraction results directly.
pub fn populate_purity_from_extracted(
    data_flow: &mut DataFlowGraph,
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> usize {
    let mut count = 0;

    for (path, file_data) in extracted {
        for func in &file_data.functions {
            let func_id = FunctionId::new(path.clone(), func.qualified_name.clone(), func.line);

            // Convert extracted purity to PurityInfo
            let purity_info = PurityInfo {
                is_pure: func.purity_analysis.is_pure,
                confidence: func.purity_analysis.confidence,
                impurity_reasons: Vec::new(), // Not stored in extracted data
            };
            data_flow.set_purity_info(func_id.clone(), purity_info);

            // Convert to MutationInfo
            let mutation_info = MutationInfo {
                has_mutations: func.purity_analysis.has_mutations,
                detected_mutations: func.purity_analysis.local_mutations.clone(),
            };
            data_flow.set_mutation_info(func_id, mutation_info);

            count += 1;
        }
    }

    count
}

/// Populate I/O operations from extracted file data (pure).
///
/// # Spec 213
///
/// Populates the DataFlowGraph with I/O operations from pre-extracted data.
/// Avoids per-function file parsing by using the extraction results directly.
pub fn populate_io_from_extracted(
    data_flow: &mut DataFlowGraph,
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> usize {
    let mut count = 0;

    for (path, file_data) in extracted {
        for func in &file_data.functions {
            if func.io_operations.is_empty() {
                continue;
            }

            let func_id = FunctionId::new(path.clone(), func.qualified_name.clone(), func.line);

            for io_op in &func.io_operations {
                // Convert extraction IoOperation to data_flow IoOperation
                let op = crate::data_flow::IoOperation {
                    operation_type: io_op.description.clone(),
                    variables: Vec::new(), // Not stored in extracted data
                    line: io_op.line,
                };
                data_flow.add_io_operation(func_id.clone(), op);
                count += 1;
            }
        }
    }

    count
}

/// Populate variable dependencies from extracted file data (pure).
///
/// # Spec 213
///
/// Populates the DataFlowGraph with variable dependencies from pre-extracted data.
/// Uses parameter names from the extraction as dependencies.
pub fn populate_variable_deps_from_extracted(
    data_flow: &mut DataFlowGraph,
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> usize {
    let mut count = 0;

    for (path, file_data) in extracted {
        for func in &file_data.functions {
            if func.parameter_names.is_empty() {
                continue;
            }

            let func_id = FunctionId::new(path.clone(), func.qualified_name.clone(), func.line);

            let deps: HashSet<String> = func.parameter_names.iter().cloned().collect();
            count += deps.len();
            data_flow.add_variable_dependencies(func_id, deps);
        }
    }

    count
}

/// Populate data transformations from extracted file data (pure).
///
/// # Spec 213
///
/// Populates the DataFlowGraph with transformation patterns from pre-extracted data.
/// The extraction already identifies map/filter/fold/etc patterns.
pub fn populate_transformations_from_extracted(
    _data_flow: &mut DataFlowGraph,
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> usize {
    // Count transformation patterns across all functions
    // Note: We don't currently store these in DataFlowGraph,
    // but we count them for metrics purposes
    extracted
        .values()
        .flat_map(|file_data| &file_data.functions)
        .map(|func| func.transformation_patterns.len())
        .sum()
}

/// Populate all data flow information from extracted data (pure).
///
/// # Spec 213
///
/// Convenience function that populates all data flow information from extracted
/// file data in a single call. This is the preferred entry point when using
/// the unified extraction pipeline.
pub fn populate_all_from_extracted(
    data_flow: &mut DataFlowGraph,
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> PopulationStats {
    let purity_count = populate_purity_from_extracted(data_flow, extracted);
    let io_count = populate_io_from_extracted(data_flow, extracted);
    let dep_count = populate_variable_deps_from_extracted(data_flow, extracted);
    let transform_count = populate_transformations_from_extracted(data_flow, extracted);

    PopulationStats {
        purity_entries: purity_count,
        io_operations: io_count,
        variable_dependencies: dep_count,
        transformation_patterns: transform_count,
    }
}

/// Statistics from data flow population.
#[derive(Debug, Clone, Default)]
pub struct PopulationStats {
    /// Number of functions with purity analysis populated
    pub purity_entries: usize,
    /// Number of I/O operations populated
    pub io_operations: usize,
    /// Number of variable dependencies populated
    pub variable_dependencies: usize,
    /// Number of transformation patterns detected
    pub transformation_patterns: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::data_flow::{DataFlowAnalysis, ReachingDefinitions};
    use crate::core::PurityLevel;
    use std::path::PathBuf;

    fn create_test_function_id(name: &str) -> FunctionId {
        FunctionId::new(PathBuf::from("test.rs"), name.to_string(), 1)
    }

    fn create_test_purity_analysis() -> PurityAnalysis {
        use crate::analyzers::purity_detector::LocalMutation;

        // Create minimal data flow analysis (escape/taint removed)
        let data_flow = DataFlowAnalysis {
            reaching_defs: ReachingDefinitions::default(),
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
            var_names: vec!["x".to_string(), "y".to_string()],
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

        // Verify mutation info was stored with binary signals (spec 257)
        let mutation_info = data_flow.get_mutation_info(&func_id).unwrap();
        assert!(mutation_info.has_mutations);
        assert_eq!(mutation_info.detected_mutations.len(), 1);
    }

    // Escape vars test removed - escape analysis no longer provides actionable signals
    // Old populate_io_operations test removed - spec 213 removed per-function parsing functions

    // ============================================================
    // Spec 202: Tests for impl block method handling
    // ============================================================

    #[test]
    fn test_find_function_in_ast_top_level() {
        let code = r#"
fn top_level_func(x: i32) -> i32 {
    x + 1
}
"#;
        let ast = syn::parse_file(code).unwrap();

        // Should find top-level function at line 2 (1-indexed, line 1 is empty after r#")
        let found = find_function_in_ast(&ast, "top_level_func", 2);
        assert!(found.is_some(), "Should find top-level function");

        // Should not find at wrong line
        let not_found = find_function_in_ast(&ast, "top_level_func", 5);
        assert!(not_found.is_none(), "Should not find at wrong line");
    }

    #[test]
    fn test_find_function_in_ast_impl_method_simple_name() {
        let code = r#"
struct Foo;

impl Foo {
    fn method(&self) -> i32 {
        42
    }
}
"#;
        let ast = syn::parse_file(code).unwrap();

        // Should find method by simple name at line 5 (1-indexed)
        let found = find_function_in_ast(&ast, "method", 5);
        assert!(found.is_some(), "Should find impl method by simple name");

        match found.unwrap() {
            FoundFunction::ImplMethod(_) => { /* expected */ }
            FoundFunction::TopLevel(_) => panic!("Should be ImplMethod, not TopLevel"),
        }
    }

    #[test]
    fn test_find_function_in_ast_impl_method_qualified_name() {
        let code = r#"
struct Bar;

impl Bar {
    fn do_something(&self, x: i32) -> i32 {
        x * 2
    }
}
"#;
        let ast = syn::parse_file(code).unwrap();

        // Should find method by qualified name (Type::method) at line 5 (1-indexed)
        let found = find_function_in_ast(&ast, "Bar::do_something", 5);
        assert!(found.is_some(), "Should find impl method by qualified name");

        match found.unwrap() {
            FoundFunction::ImplMethod(_) => { /* expected */ }
            FoundFunction::TopLevel(_) => panic!("Should be ImplMethod, not TopLevel"),
        }
    }

    // Tests for old per-function parsing functions (populate_io_operations_impl_method,
    // extract_variable_deps_impl_method, populate_data_transformations_impl_method)
    // removed - spec 213 deleted per-function parsing in favor of extraction pipeline

    #[test]
    fn test_found_function_block() {
        let code = r#"
fn top_level() {
    let x = 1;
}

struct Foo;

impl Foo {
    fn method(&self) {
        let y = 2;
    }
}
"#;
        let ast = syn::parse_file(code).unwrap();

        // Check top-level block (line 2, 1-indexed)
        let top_level = find_function_in_ast(&ast, "top_level", 2).unwrap();
        let block = top_level.block();
        assert!(
            !block.stmts.is_empty(),
            "Top-level block should have statements"
        );

        // Check impl method block (line 9, 1-indexed)
        let method = find_function_in_ast(&ast, "method", 9).unwrap();
        let block = method.block();
        assert!(
            !block.stmts.is_empty(),
            "Impl method block should have statements"
        );
    }

    #[test]
    fn test_found_function_inputs() {
        let code = r#"
fn top_level(a: i32, b: String) {}

struct Foo;

impl Foo {
    fn method(&self, x: i32, y: bool) {}
}
"#;
        let ast = syn::parse_file(code).unwrap();

        // Check top-level inputs (line 2, 1-indexed)
        let top_level = find_function_in_ast(&ast, "top_level", 2).unwrap();
        let inputs: Vec<_> = top_level.inputs().collect();
        assert_eq!(inputs.len(), 2, "Top-level should have 2 inputs");

        // Check impl method inputs (includes &self) (line 7, 1-indexed)
        let method = find_function_in_ast(&ast, "method", 7).unwrap();
        let inputs: Vec<_> = method.inputs().collect();
        assert_eq!(
            inputs.len(),
            3,
            "Impl method should have 3 inputs (self + 2 params)"
        );
    }
}
