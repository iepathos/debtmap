use debtmap::analyzers::rust_call_graph::extract_call_graph;
use std::path::PathBuf;
use syn;

#[test]
fn test_real_complexity_refactoring_struct_literal() {
    // This test mirrors the exact structure from complexity_refactoring.rs
    let code = r#"
use crate::refactoring::{ExtractionStrategy, RefactoringOpportunity};

struct ComplexityRefactoring;

impl RefactoringDetector for ComplexityRefactoring {
    fn detect(&self, function: &FunctionMetrics) -> Vec<RefactoringOpportunity> {
        let functions_to_extract = 3;
        
        vec![RefactoringOpportunity {
            extraction_strategy: ExtractionStrategy::DirectFunctionalTransformation {
                patterns_to_apply: vec![
                    FunctionalPattern::MapOverLoop,
                    FunctionalPattern::FilterPredicate,
                ],
                functions_to_extract,
            },
            suggested_functions: generate_suggested_functions(
                &function.name,
                functions_to_extract,
            ),
            functional_patterns: vec![
                FunctionalPattern::MapOverLoop,
                FunctionalPattern::FilterPredicate,
                FunctionalPattern::ComposeFunctions,
            ],
        }]
    }
}

fn generate_suggested_functions(base_name: &str, count: u32) -> Vec<PureFunctionSpec> {
    let mut functions = Vec::new();
    
    if count > 0 {
        functions.push(PureFunctionSpec {
            name: format!("{}_validate", base_name),
        });
    }
    
    functions
}
    "#;

    let syntax = syn::parse_file(code).expect("Failed to parse code");
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&syntax, &path);
    
    // Find the actual function IDs from the graph
    let all_functions = call_graph.find_all_functions();
    
    println!("All functions found:");
    for func in &all_functions {
        println!("  {:?}", func);
    }
    
    // Find generate_suggested_functions
    let generate_id = all_functions
        .iter()
        .find(|f| f.name == "generate_suggested_functions")
        .expect("generate_suggested_functions should be in the graph");
    
    // Check that it has callers
    let callers = call_graph.get_callers(&generate_id);
    println!("Callers of generate_suggested_functions: {:?}", callers);
    
    assert!(
        !callers.is_empty(),
        "generate_suggested_functions should have callers, but has none"
    );
}

#[test]
fn test_vec_macro_with_struct_literal() {
    // Test with vec! macro containing struct literal
    let code = r#"
struct RefactoringOpportunity {
    suggested_functions: Vec<String>,
}

fn detect() -> Vec<RefactoringOpportunity> {
    vec![RefactoringOpportunity {
        suggested_functions: generate_functions("base", 3),
    }]
}

fn generate_functions(base_name: &str, count: u32) -> Vec<String> {
    vec![format!("{}_validate", base_name)]
}
    "#;

    let syntax = syn::parse_file(code).expect("Failed to parse code");
    let path = PathBuf::from("test.rs");
    let call_graph = extract_call_graph(&syntax, &path);
    
    let all_functions = call_graph.find_all_functions();
    
    println!("Functions in vec! macro test:");
    for func in &all_functions {
        println!("  {:?}", func);
    }
    
    let detect_id = all_functions
        .iter()
        .find(|f| f.name == "detect")
        .expect("detect should be in the graph")
        .clone();
    
    let generate_id = all_functions
        .iter()
        .find(|f| f.name == "generate_functions")
        .expect("generate_functions should be in the graph")
        .clone();
    
    // Check the calls
    let calls_from_detect = call_graph.get_callees(&detect_id);
    println!("Calls from detect: {:?}", calls_from_detect);
    
    assert!(
        calls_from_detect.contains(&generate_id),
        "detect should call generate_functions"
    );
    
    // Check that generate_functions has callers
    let callers = call_graph.get_callers(&generate_id);
    assert!(
        !callers.is_empty(),
        "generate_functions should have callers"
    );
}