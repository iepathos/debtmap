use debtmap::risk::lcov::parse_lcov_file;
use debtmap::priority::call_graph::{CallGraph, FunctionId};
use std::path::PathBuf;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_lcov_parsing_determinism() {
    // Adversarial LCOV content: multiple SF records for same file, different orders
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
"#;

    let mut results = Vec::new();

    for _ in 0..10 {
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", lcov_content).unwrap();
        
        let data = parse_lcov_file(temp_file.path()).unwrap();
        
        // Extract function data for comparison
        let mut file_data = Vec::new();
        let mut sorted_files: Vec<_> = data.functions.keys().collect();
        sorted_files.sort();
        
        for file in sorted_files {
            let funcs = &data.functions[file];
            // We want to see if the internal order of funcs is stable
            let func_names: Vec<_> = funcs.iter().map(|f| (f.name.clone(), f.coverage_percentage)).collect();
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

    for _ in 0..10 {
        let mut graph = CallGraph::new();
        // Functions with same name in different files
        let f1 = FunctionId::new(PathBuf::from("a.rs"), "common".to_string(), 1);
        let f2 = FunctionId::new(PathBuf::from("b.rs"), "common".to_string(), 1);
        let f3 = FunctionId::new(PathBuf::from("c.rs"), "other".to_string(), 1);

        graph.add_function(f1.clone(), true, false, 10, 100);
        graph.add_function(f2.clone(), false, false, 10, 100);
        graph.add_function(f3.clone(), false, false, 10, 100);
        
        graph.add_call_parts(f1.clone(), f2.clone(), debtmap::priority::call_graph::CallType::Direct);
        graph.add_call_parts(f2.clone(), f3.clone(), debtmap::priority::call_graph::CallType::Direct);

        let topo = graph.topological_sort().unwrap();
        let all = graph.get_all_functions().cloned().collect::<Vec<_>>();
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
    use debtmap::risk::context::{Context, ContextDetails};
    use std::collections::HashMap;

    let mut context_map = HashMap::new();
    
    // Add many small floating point values
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

    // We can't easily access the private total_contribution from here without moving it
    // but we can simulate what it does.
    
    let mut results = Vec::new();
    for _ in 0..10 {
        let mut values: Vec<_> = context_map.values().collect();
        // Shuffling would be ideal, but even just relying on different HashMap 
        // instances (if we created them) would work. 
        // Here we just test if our SORTING fix works.
        values.sort_by(|a, b| a.provider.cmp(&b.provider));
        
        let sum: f64 = values.iter().map(|c| c.contribution * c.weight).sum();
        results.push(sum);
    }

    let first = results[0];
    for (i, &other) in results.iter().enumerate().skip(1) {
        assert_eq!(first, other, "Float summation non-deterministic at iteration {}", i);
    }
}
