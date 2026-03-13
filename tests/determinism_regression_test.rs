use debtmap::risk::lcov::parse_lcov_file;
use debtmap::priority::call_graph::{CallGraph, FunctionId, CallType};
use debtmap::risk::context::{Context, ContextDetails};
use std::path::PathBuf;
use std::io::Write;
use std::collections::HashMap;
use tempfile::NamedTempFile;

#[test]
fn test_lcov_parsing_determinism() {
    // Adversarial LCOV content: multiple SF records for same file, different orders,
    // overlapping lines, and duplicate functions (same name/line).
    let lcov_content = r#"SF:src/lib.rs
FN:10,func1
FN:10,func2
FNDA:5,func1
FNDA:10,func2
DA:10,1
end_of_record
SF:src/lib.rs
FN:20,func3
FNDA:0,func3
DA:20,0
end_of_record
SF:src/other.rs
FN:5,other_func
FNDA:1,other_func
DA:5,1
end_of_record
"#;

    let mut results = Vec::new();

    for _ in 0..5 {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();
        
        let data = parse_lcov_file(temp_file.path()).unwrap();
        
        // Extract function data for comparison
        let mut file_data = Vec::new();
        let mut sorted_files: Vec<_> = data.functions.keys().collect();
        sorted_files.sort();
        
        for file in sorted_files {
            let funcs = &data.functions[file];
            // Verify internal order of funcs is stable
            let func_names: Vec<_> = funcs.iter()
                .map(|f| (f.name.clone(), f.start_line, f.coverage_percentage))
                .collect();
            file_data.push((file.clone(), func_names));
        }
        results.push(file_data);
    }

    let first = &results[0];
    for (i, other) in results.iter().enumerate().skip(1) {
        assert_eq!(first, other, "LCOV parsing non-deterministic at iteration {}", i);
    }
}

#[test]
fn test_call_graph_determinism() {
    let mut results = Vec::new();

    for _ in 0..5 {
        let mut graph = CallGraph::new();
        // Functions with same name in different files
        let f1 = FunctionId::new(PathBuf::from("a.rs"), "common".to_string(), 1);
        let f2 = FunctionId::new(PathBuf::from("b.rs"), "common".to_string(), 1);
        let f3 = FunctionId::new(PathBuf::from("c.rs"), "other".to_string(), 1);

        graph.add_function(f1.clone(), true, false, 10, 100);
        graph.add_function(f2.clone(), false, false, 10, 100);
        graph.add_function(f3.clone(), false, false, 10, 100);
        
        graph.add_call_parts(f1.clone(), f2.clone(), CallType::Direct);
        graph.add_call_parts(f2.clone(), f3.clone(), CallType::Direct);

        let topo = graph.topological_sort().unwrap();
        let all: Vec<_> = graph.get_all_functions().cloned().collect();
        let callers = graph.get_callers_by_name("common");
        let callees = graph.get_callees_by_name("common");
        results.push((topo, all, callers, callees));
    }

    let (first_topo, first_all, first_callers, first_callees) = &results[0];
    for (i, (other_topo, other_all, other_callers, other_callees)) in results.iter().enumerate().skip(1) {
        assert_eq!(first_topo, other_topo, "Topo sort non-deterministic at iteration {}", i);
        assert_eq!(first_all, other_all, "get_all_functions non-deterministic at iteration {}", i);
        assert_eq!(first_callers, other_callers, "get_callers_by_name non-deterministic at iteration {}", i);
        assert_eq!(first_callees, other_callees, "get_callees_by_name non-deterministic at iteration {}", i);
    }
}

#[test]
fn test_context_summation_determinism() {
    let mut context_map = HashMap::new();
    
    // Add many small floating point values to a map
    for i in 0..100 {
        context_map.insert(
            format!("provider_{}", i),
            Context {
                provider: format!("provider_{}", i),
                weight: 0.123456789,
                contribution: (i as f64) * 0.0000001,
                details: ContextDetails::Historical {
                    change_frequency: 0.0,
                    bug_density: 0.0,
                    age_days: 0,
                    author_count: 0,
                    total_commits: 0,
                    bug_fix_count: 0,
                },
            },
        );
    }

    let mut results = Vec::new();
    for _ in 0..10 {
        // Simulate total_contribution logic (sorting by provider name)
        let mut values: Vec<_> = context_map.values().collect();
        values.sort_by(|a, b| a.provider.cmp(&b.provider));
        
        let sum: f64 = values.iter().map(|c| c.contribution * c.weight).sum();
        results.push(sum);
    }

    let first = results[0];
    for (i, &other) in results.iter().enumerate().skip(1) {
        // Bit-level comparison for f64
        assert_eq!(first.to_bits(), other.to_bits(), "Float summation non-deterministic at iteration {}", i);
    }
}

#[test]
fn test_typescript_extraction_determinism() {
    use debtmap::analyzers::typescript::parser::parse_source;
    use debtmap::analyzers::typescript::visitor::function_analysis::extract_functions;
    use debtmap::analyzers::typescript::call_graph::extract_call_graph;
    use debtmap::core::ast::JsLanguageVariant;
    use std::path::PathBuf;

    let source = r#"
function common() { return 1; }
class Test {
    common() { return 2; }
}
const arrow = () => common();
"#;
    let path = PathBuf::from("test.ts");

    let mut results = Vec::new();
    for _ in 0..5 {
        let ast = parse_source(source, &path, JsLanguageVariant::TypeScript).unwrap();
        let funcs = extract_functions(&ast, false);
        let graph = extract_call_graph(&ast);
        
        let func_data: Vec<_> = funcs.iter().map(|f| (f.name.clone(), f.line)).collect();
        let graph_nodes: Vec<_> = graph.get_all_functions().cloned().collect();
        results.push((func_data, graph_nodes));
    }

    let (first_funcs, first_nodes) = &results[0];
    for (i, (other_funcs, other_nodes)) in results.iter().enumerate().skip(1) {
        assert_eq!(first_funcs, other_funcs, "TS function extraction non-deterministic at iteration {}", i);
        assert_eq!(first_nodes, other_nodes, "TS call graph nodes non-deterministic at iteration {}", i);
    }
}
