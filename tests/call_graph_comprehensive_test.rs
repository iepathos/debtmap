//! Comprehensive call graph tests to validate end-to-end correctness
//!
//! This test suite validates:
//! 1. Functions are added to the call graph with correct FunctionIds
//! 2. Calls are detected and edges are created
//! 3. Caller/callee lookups work correctly
//! 4. FunctionId matching works across different creation methods
//! 5. The integration with UnifiedDebtItem creation

use debtmap::analyzers::rust_call_graph::extract_call_graph;
use debtmap::core::FunctionMetrics;
use debtmap::priority::call_graph::FunctionId;
use debtmap::priority::scoring::construction::create_unified_debt_item_enhanced;
use std::path::PathBuf;

/// Helper to create minimal FunctionMetrics for testing
fn create_test_metrics(name: &str, file: PathBuf, line: usize) -> FunctionMetrics {
    FunctionMetrics {
        name: name.to_string(),
        file,
        line,
        cyclomatic: 1,
        cognitive: 0,
        nesting: 0,
        length: 3,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: Some(false),
        purity_confidence: Some(0.0),
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    }
}

#[test]
fn test_call_graph_basic_caller_callee_tracking() {
    let code = r#"
fn main() {
    helper();
    process();
}

fn helper() {
    println!("Helper");
}

fn process() {
    validate();
}

fn validate() {
    // validation
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    // Verify nodes exist
    assert_eq!(call_graph.node_count(), 4, "Should have 4 functions");

    // Test main → helper edge
    let main_id = FunctionId::new(path.clone(), "main".to_string(), 2);
    let helper_id = FunctionId::new(path.clone(), "helper".to_string(), 7);

    let main_callees = call_graph.get_callees(&main_id);
    assert!(
        main_callees.iter().any(|id| id.name == "helper"),
        "main should call helper"
    );

    let helper_callers = call_graph.get_callers(&helper_id);
    assert_eq!(
        helper_callers.len(),
        1,
        "helper should have exactly 1 caller"
    );
    assert!(
        helper_callers.iter().any(|id| id.name == "main"),
        "helper should be called by main"
    );
}

#[test]
fn test_call_graph_function_id_consistency() {
    // This test validates that FunctionIds created from FunctionMetrics
    // match those created during call graph extraction
    let code = r#"
fn caller() {
    callee();
}

fn callee() {
    println!("Called");
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    // Simulate what happens in unified analysis:
    // FunctionMetrics are created, then we create FunctionId from them
    let caller_metrics = create_test_metrics("caller", path.clone(), 2);
    let callee_metrics = create_test_metrics("callee", path.clone(), 6);

    // Create FunctionIds the same way create_unified_debt_item_enhanced does
    let caller_id = FunctionId::new(
        caller_metrics.file.clone(),
        caller_metrics.name.clone(),
        caller_metrics.line,
    );
    let callee_id = FunctionId::new(
        callee_metrics.file.clone(),
        callee_metrics.name.clone(),
        callee_metrics.line,
    );

    // Verify that lookups work with these FunctionIds
    let caller_callees = call_graph.get_callees(&caller_id);
    assert!(
        !caller_callees.is_empty(),
        "caller FunctionId should find callees in graph"
    );
    assert!(
        caller_callees.iter().any(|id| id.name == "callee"),
        "caller should have callee in its callees list"
    );

    let callee_callers = call_graph.get_callers(&callee_id);
    assert!(
        !callee_callers.is_empty(),
        "callee FunctionId should find callers in graph"
    );
    assert!(
        callee_callers.iter().any(|id| id.name == "caller"),
        "callee should have caller in its callers list"
    );
}

#[test]
fn test_unified_debt_item_has_callers() {
    // End-to-end test: verify that UnifiedDebtItem creation populates callers correctly
    let code = r#"
fn main() {
    helper();
}

fn helper() {
    println!("Helper");
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    // Create FunctionMetrics for helper
    let helper_metrics = create_test_metrics("helper", path.clone(), 6);

    // Create UnifiedDebtItem using the same function as production code
    let debt_item = create_unified_debt_item_enhanced(&helper_metrics, &call_graph, None, None);

    // Verify that the debt item has callers
    assert!(
        !debt_item.upstream_callers.is_empty(),
        "helper should have callers in UnifiedDebtItem"
    );
    assert_eq!(
        debt_item.upstream_dependencies, 1,
        "helper should have 1 upstream dependency"
    );
    assert!(
        debt_item.upstream_callers.contains(&"main".to_string()),
        "helper's callers should include 'main'"
    );
}

#[test]
fn test_multiple_callers_tracked() {
    let code = r#"
fn main() {
    shared();
}

fn process() {
    shared();
}

fn analyze() {
    shared();
}

fn shared() {
    println!("Shared function");
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    let shared_id = FunctionId::new(path.clone(), "shared".to_string(), 14);
    let callers = call_graph.get_callers(&shared_id);

    assert_eq!(callers.len(), 3, "shared should have exactly 3 callers");

    let caller_names: Vec<String> = callers.iter().map(|id| id.name.clone()).collect();
    assert!(caller_names.contains(&"main".to_string()));
    assert!(caller_names.contains(&"process".to_string()));
    assert!(caller_names.contains(&"analyze".to_string()));
}

#[test]
fn test_method_calls_tracked() {
    let code = r#"
struct Processor;

impl Processor {
    fn run(&self) {
        self.process();
    }

    fn process(&self) {
        self.validate();
    }

    fn validate(&self) {
        println!("Valid");
    }
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    // Check that process has run as a caller
    let process_id = FunctionId::new(path.clone(), "Processor::process".to_string(), 9);
    let process_callers = call_graph.get_callers(&process_id);

    assert!(
        !process_callers.is_empty(),
        "Processor::process should have callers"
    );
    assert!(
        process_callers.iter().any(|id| id.name.contains("run")),
        "Processor::process should be called by run"
    );

    // Check that validate has process as a caller
    let validate_id = FunctionId::new(path.clone(), "Processor::validate".to_string(), 13);
    let validate_callers = call_graph.get_callers(&validate_id);

    assert!(
        !validate_callers.is_empty(),
        "Processor::validate should have callers"
    );
    assert!(
        validate_callers
            .iter()
            .any(|id| id.name.contains("process")),
        "Processor::validate should be called by process"
    );
}

#[test]
fn test_chain_of_calls() {
    let code = r#"
fn level_1() {
    level_2();
}

fn level_2() {
    level_3();
}

fn level_3() {
    level_4();
}

fn level_4() {
    println!("End");
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    // Verify each level has exactly one caller
    let level_2_id = FunctionId::new(path.clone(), "level_2".to_string(), 6);
    let level_2_callers = call_graph.get_callers(&level_2_id);
    assert_eq!(level_2_callers.len(), 1);

    let level_3_id = FunctionId::new(path.clone(), "level_3".to_string(), 10);
    let level_3_callers = call_graph.get_callers(&level_3_id);
    assert_eq!(level_3_callers.len(), 1);

    let level_4_id = FunctionId::new(path.clone(), "level_4".to_string(), 14);
    let level_4_callers = call_graph.get_callers(&level_4_id);
    assert_eq!(level_4_callers.len(), 1);

    // Verify level_1 has no callers (it's the entry point)
    let level_1_id = FunctionId::new(path.clone(), "level_1".to_string(), 2);
    let level_1_callers = call_graph.get_callers(&level_1_id);
    assert_eq!(level_1_callers.len(), 0);
}

#[test]
fn test_no_false_positives() {
    // Test that functions that don't call each other aren't linked
    let code = r#"
fn independent_1() {
    println!("One");
}

fn independent_2() {
    println!("Two");
}

fn independent_3() {
    println!("Three");
}
"#;

    let parsed = syn::parse_file(code).unwrap();
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&parsed, &path);

    // Each function should have no callers and no callees
    for name in &["independent_1", "independent_2", "independent_3"] {
        // Find actual function in graph
        let actual_funcs: Vec<_> = call_graph
            .find_all_functions()
            .into_iter()
            .filter(|f| f.name == *name)
            .collect();

        assert_eq!(
            actual_funcs.len(),
            1,
            "Should find exactly one function named {}",
            name
        );
        let actual_id = &actual_funcs[0];

        let callers = call_graph.get_callers(actual_id);
        let callees = call_graph.get_callees(actual_id);

        assert_eq!(callers.len(), 0, "{} should have no callers", name);

        // Note: callees might include println! or other built-ins
        // We just check that it doesn't call the other independent functions
        let callee_names: Vec<String> = callees.iter().map(|id| id.name.clone()).collect();
        for other_name in &["independent_1", "independent_2", "independent_3"] {
            if other_name != name {
                assert!(
                    !callee_names.contains(&other_name.to_string()),
                    "{} should not call {}",
                    name,
                    other_name
                );
            }
        }
    }
}
