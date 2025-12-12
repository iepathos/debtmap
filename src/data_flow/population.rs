//! Data flow graph population from various analysis sources
//!
//! This module provides functions to populate the DataFlowGraph with data from:
//! - Purity analysis (CFG-based data flow analysis)
//! - I/O operation detection (AST-based pattern matching)
//! - Variable dependency analysis
//! - Data transformation patterns

use crate::analyzers::io_detector::detect_io_operations;
use crate::analyzers::purity_detector::PurityAnalysis;
use crate::core::FunctionMetrics;
use crate::data_flow::{DataFlowGraph, MutationInfo};
use crate::priority::call_graph::FunctionId;
use std::collections::HashSet;

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
    pub fn find_function_in_ast<'a>(
        ast: &'a syn::File,
        metric_name: &str,
        line: usize,
    ) -> Option<FoundFunction<'a>> {
        for item in &ast.items {
            match item {
                syn::Item::Fn(item_fn) => {
                    if let Some(span_line) = item_fn.sig.ident.span().start().line.checked_sub(1) {
                        // For top-level functions, metric_name should match exactly
                        // or be the simple part of a qualified name
                        let fn_name = item_fn.sig.ident.to_string();
                        if span_line == line && (metric_name == fn_name) {
                            return Some(FoundFunction::TopLevel(item_fn));
                        }
                    }
                }
                syn::Item::Impl(item_impl) => {
                    for impl_item in &item_impl.items {
                        if let syn::ImplItem::Fn(method) = impl_item {
                            if let Some(span_line) =
                                method.sig.ident.span().start().line.checked_sub(1)
                            {
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
/// - Mutation information (live vs dead stores)
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

    // Extract mutation information
    let live_mutations: Vec<String> = purity
        .live_mutations
        .iter()
        .map(|m| m.target.clone())
        .collect();

    let mutation_info = MutationInfo {
        live_mutations: live_mutations.clone(),
        total_mutations: purity.total_mutations,
        dead_stores: extract_dead_stores(purity, data_flow, func_id),
        escaping_mutations: extract_escaping_mutations(purity, data_flow, func_id),
    };

    data_flow.set_mutation_info(func_id.clone(), mutation_info);
}

/// Extract dead stores from purity analysis
///
/// Now uses the stored CfgAnalysisWithContext to translate VarIds to variable names.
fn extract_dead_stores(
    _purity: &PurityAnalysis,
    data_flow: &DataFlowGraph,
    func_id: &FunctionId,
) -> HashSet<String> {
    // Use the translation layer to get dead store names
    data_flow
        .get_dead_store_names(func_id)
        .into_iter()
        .collect()
}

/// Extract escaping mutations from purity analysis
///
/// Now uses the stored CfgAnalysisWithContext to translate VarIds to variable names.
fn extract_escaping_mutations(
    _purity: &PurityAnalysis,
    data_flow: &DataFlowGraph,
    func_id: &FunctionId,
) -> HashSet<String> {
    // Use the translation layer to get escaping variable names
    data_flow
        .get_escaping_var_names(func_id)
        .into_iter()
        .collect()
}

/// Populate I/O operations from function metrics using AST-based detection
///
/// Detects I/O operations by analyzing function ASTs for actual I/O patterns
/// rather than relying on function name heuristics. This provides significantly
/// higher accuracy and coverage (~70-80% vs ~4.3% with name-based detection).
///
/// # Implementation (Spec 245, Spec 202)
///
/// Uses AST visitor pattern to detect:
/// - File I/O (std::fs, File, BufReader/Writer)
/// - Console I/O (println!, eprintln!, stdout/stderr)
/// - Network I/O (TcpStream, HTTP clients)
/// - Database I/O (query, execute, prepare)
/// - Async I/O (tokio::fs, async-std::fs)
///
/// Spec 202: Now handles methods inside impl blocks, not just top-level functions.
pub fn populate_io_operations(data_flow: &mut DataFlowGraph, metrics: &[FunctionMetrics]) -> usize {
    use crate::analyzers::io_detector::detect_io_operations_from_block;
    use std::fs;

    let mut total_ops = 0;

    for metric in metrics {
        let func_id = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line);

        // Read file and parse to get AST
        let content = match fs::read_to_string(&metric.file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "Warning: Failed to read file {} for I/O detection: {}",
                    metric.file.display(),
                    e
                );
                continue;
            }
        };

        let file_ast = match syn::parse_file(&content) {
            Ok(ast) => ast,
            Err(_) => continue,
        };

        // Find the function by name and line number (handles both top-level and impl methods)
        if let Some(found) = find_function_in_ast(&file_ast, &metric.name, metric.line) {
            // Use AST-based I/O detector on the function block
            let io_ops = match found {
                FoundFunction::TopLevel(item_fn) => detect_io_operations(item_fn),
                FoundFunction::ImplMethod(method) => detect_io_operations_from_block(&method.block),
            };

            for op in io_ops {
                data_flow.add_io_operation(func_id.clone(), op);
                total_ops += 1;
            }
        }
    }

    total_ops
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
///
/// Spec 202: Now handles methods inside impl blocks, not just top-level functions.
fn extract_variable_deps(metric: &FunctionMetrics) -> HashSet<String> {
    use std::fs;

    // Read file and parse to get AST
    let content = match fs::read_to_string(&metric.file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "Warning: Failed to read file {}: {}",
                metric.file.display(),
                e
            );
            return HashSet::new();
        }
    };

    let file_ast = match syn::parse_file(&content) {
        Ok(ast) => ast,
        Err(_) => return HashSet::new(),
    };

    // Find the function by name and line number (handles both top-level and impl methods)
    if let Some(found) = find_function_in_ast(&file_ast, &metric.name, metric.line) {
        // Extract parameter names from function signature
        let mut deps = HashSet::new();
        for input in found.inputs() {
            if let syn::FnArg::Typed(pat_type) = input {
                if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                    deps.insert(pat_ident.ident.to_string());
                }
            }
        }
        return deps;
    }

    HashSet::new()
}

/// Populate data transformations between functions
///
/// Identifies transformation patterns like map, filter, fold
///
/// Spec 202: Now handles methods inside impl blocks, not just top-level functions.
pub fn populate_data_transformations(
    data_flow: &mut DataFlowGraph,
    metrics: &[FunctionMetrics],
) -> usize {
    use std::fs;

    let mut transformation_count = 0;

    for metric in metrics {
        // Read file and parse to get AST
        let content = match fs::read_to_string(&metric.file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "Warning: Failed to read file {}: {}",
                    metric.file.display(),
                    e
                );
                continue;
            }
        };

        let file_ast = match syn::parse_file(&content) {
            Ok(ast) => ast,
            Err(_) => continue,
        };

        // Find the function by name and line number (handles both top-level and impl methods)
        if let Some(found) = find_function_in_ast(&file_ast, &metric.name, metric.line) {
            // Detect transformation patterns in the function body
            transformation_count +=
                detect_transformation_patterns(found.block(), data_flow, metric);
        }
    }

    transformation_count
}

/// Detect data transformation patterns (map, filter, fold, etc.) in a code block
fn detect_transformation_patterns(
    block: &syn::Block,
    _data_flow: &mut DataFlowGraph,
    _metric: &FunctionMetrics,
) -> usize {
    let mut count = 0;

    // Visit all statements and expressions looking for iterator method chains
    for stmt in &block.stmts {
        count += count_transformations_in_stmt(stmt);
    }

    count
}

/// Count transformation patterns in a statement
fn count_transformations_in_stmt(stmt: &syn::Stmt) -> usize {
    match stmt {
        syn::Stmt::Expr(expr, _) => count_transformations_in_expr(expr),
        syn::Stmt::Local(local) => {
            if let Some(init) = &local.init {
                count_transformations_in_expr(&init.expr)
            } else {
                0
            }
        }
        _ => 0,
    }
}

/// Count transformation patterns in an expression
fn count_transformations_in_expr(expr: &syn::Expr) -> usize {
    let mut count = 0;

    match expr {
        syn::Expr::MethodCall(method_call) => {
            // Check if this is a transformation method
            let method_name = method_call.method.to_string();
            if is_transformation_method(&method_name) {
                count += 1;
            }
            // Recursively check the receiver
            count += count_transformations_in_expr(&method_call.receiver);
        }
        syn::Expr::Call(call) => {
            // Check arguments for nested transformations
            for arg in &call.args {
                count += count_transformations_in_expr(arg);
            }
        }
        syn::Expr::Block(block) => {
            for stmt in &block.block.stmts {
                count += count_transformations_in_stmt(stmt);
            }
        }
        _ => {}
    }

    count
}

/// Check if a method name represents a data transformation
fn is_transformation_method(name: &str) -> bool {
    matches!(
        name,
        "map"
            | "filter"
            | "fold"
            | "reduce"
            | "filter_map"
            | "flat_map"
            | "scan"
            | "collect"
            | "for_each"
            | "any"
            | "all"
            | "find"
            | "partition"
            | "zip"
            | "chain"
    )
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

        // Verify mutation info was stored
        let mutation_info = data_flow.get_mutation_info(&func_id).unwrap();
        assert_eq!(mutation_info.live_mutations.len(), 1);
        assert_eq!(mutation_info.total_mutations, 2);
        // Note: dead_stores is empty as VarId->String conversion needs CFG context
        assert!(mutation_info.dead_stores.is_empty());
    }

    #[test]
    fn test_extract_dead_stores() {
        let mut data_flow = DataFlowGraph::new();
        let func_id = create_test_function_id("test_func");
        let purity = create_test_purity_analysis();

        // Populate data flow first so we have the context
        populate_from_purity_analysis(&mut data_flow, &func_id, &purity);

        let dead_stores = extract_dead_stores(&purity, &data_flow, &func_id);

        // Should return empty since test analysis has no dead stores
        assert!(dead_stores.is_empty());
    }

    #[test]
    fn test_extract_escaping_mutations() {
        let mut data_flow = DataFlowGraph::new();
        let func_id = create_test_function_id("test_func");
        let purity = create_test_purity_analysis();

        // Populate data flow first so we have the context
        populate_from_purity_analysis(&mut data_flow, &func_id, &purity);

        let escaping = extract_escaping_mutations(&purity, &data_flow, &func_id);

        // Should return empty since test analysis has no escaping vars
        assert!(escaping.is_empty());
    }

    #[test]
    fn test_populate_io_operations() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create temporary file with actual Rust code containing I/O operations
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_code = r#"
use std::fs::File;
use std::io::Read;

fn read_file() -> std::io::Result<String> {
    let mut file = File::open("data.txt")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

fn pure_func(x: i32) -> i32 {
    x * 2
}
"#;
        temp_file.write_all(test_code.as_bytes()).unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let mut data_flow = DataFlowGraph::new();
        let metrics = vec![
            FunctionMetrics::new("read_file".to_string(), temp_path.clone(), 4),
            FunctionMetrics::new("pure_func".to_string(), temp_path.clone(), 11),
        ];

        let count = populate_io_operations(&mut data_flow, &metrics);

        assert!(
            count > 0,
            "Should detect I/O operations in read_file function"
        );
    }

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

        // Should find top-level function at line 1 (0-indexed)
        let found = find_function_in_ast(&ast, "top_level_func", 1);
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

        // Should find method by simple name at line 4 (0-indexed)
        let found = find_function_in_ast(&ast, "method", 4);
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

        // Should find method by qualified name (Type::method)
        let found = find_function_in_ast(&ast, "Bar::do_something", 4);
        assert!(found.is_some(), "Should find impl method by qualified name");

        match found.unwrap() {
            FoundFunction::ImplMethod(_) => { /* expected */ }
            FoundFunction::TopLevel(_) => panic!("Should be ImplMethod, not TopLevel"),
        }
    }

    #[test]
    fn test_populate_io_operations_impl_method() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create temporary file with impl method containing I/O
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_code = r#"
use std::fs::File;
use std::io::Read;

struct FileReader;

impl FileReader {
    fn read_contents(&self) -> std::io::Result<String> {
        let mut file = File::open("data.txt")?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(contents)
    }
}
"#;
        temp_file.write_all(test_code.as_bytes()).unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let mut data_flow = DataFlowGraph::new();
        // Use qualified name format as FunctionMetrics would have
        let metrics = vec![FunctionMetrics::new(
            "FileReader::read_contents".to_string(),
            temp_path.clone(),
            7, // Line of the method
        )];

        let count = populate_io_operations(&mut data_flow, &metrics);

        assert!(
            count > 0,
            "Should detect I/O operations in impl method (spec 202)"
        );
    }

    #[test]
    fn test_extract_variable_deps_impl_method() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create temporary file with impl method containing parameters
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_code = r#"
struct Calculator;

impl Calculator {
    fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }
}
"#;
        temp_file.write_all(test_code.as_bytes()).unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let metric = FunctionMetrics::new("Calculator::add".to_string(), temp_path.clone(), 4);

        let deps = extract_variable_deps(&metric);

        // Should find parameters 'a' and 'b' (not 'self' as it's a receiver)
        assert!(deps.contains("a"), "Should find parameter 'a'");
        assert!(deps.contains("b"), "Should find parameter 'b'");
    }

    #[test]
    fn test_populate_data_transformations_impl_method() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create temporary file with impl method using iterator transformations
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_code = r#"
struct DataProcessor;

impl DataProcessor {
    fn process(&self, items: Vec<i32>) -> Vec<i32> {
        items.iter()
            .map(|x| x * 2)
            .filter(|x| *x > 10)
            .collect()
    }
}
"#;
        temp_file.write_all(test_code.as_bytes()).unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let mut data_flow = DataFlowGraph::new();
        let metrics = vec![FunctionMetrics::new(
            "DataProcessor::process".to_string(),
            temp_path.clone(),
            4,
        )];

        let count = populate_data_transformations(&mut data_flow, &metrics);

        assert!(
            count > 0,
            "Should detect transformation patterns in impl method (spec 202)"
        );
    }

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

        // Check top-level block
        let top_level = find_function_in_ast(&ast, "top_level", 1).unwrap();
        let block = top_level.block();
        assert!(
            !block.stmts.is_empty(),
            "Top-level block should have statements"
        );

        // Check impl method block
        let method = find_function_in_ast(&ast, "method", 8).unwrap();
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

        // Check top-level inputs
        let top_level = find_function_in_ast(&ast, "top_level", 1).unwrap();
        let inputs: Vec<_> = top_level.inputs().collect();
        assert_eq!(inputs.len(), 2, "Top-level should have 2 inputs");

        // Check impl method inputs (includes &self)
        let method = find_function_in_ast(&ast, "method", 6).unwrap();
        let inputs: Vec<_> = method.inputs().collect();
        assert_eq!(
            inputs.len(),
            3,
            "Impl method should have 3 inputs (self + 2 params)"
        );
    }
}
