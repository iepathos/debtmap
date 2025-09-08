use debtmap::analyzers::rust_call_graph::extract_call_graph;
use std::path::PathBuf;

fn main() {
    let code = r#"
fn outer() {
    process();
}

fn process() {
    
}
"#;

    let file = syn::parse_file(code).expect("Failed to parse code");
    println!("DEBUG: Starting call graph extraction");
    let graph = extract_call_graph(&file, &PathBuf::from("test.rs"));
    println!("DEBUG: Call graph extraction complete");

    println!("All functions in graph:");
    for func_id in graph.get_all_functions() {
        println!("  - {}", func_id.name);
    }

    println!("\nAll calls detected:");
    for call in graph.get_all_calls() {
        println!("  {} -> {}", call.caller.name, call.callee.name);
    }
}