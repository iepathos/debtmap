/// Test for detecting self method calls within impl blocks
use debtmap::analyzers::rust_call_graph::extract_call_graph_multi_file;
use std::path::PathBuf;

#[test]
fn test_self_method_call_detection() {
    // Create a simple test case that mirrors the UnifiedAnalysis pattern
    let code = r#"
struct TestStruct {
    data: Vec<String>,
}

impl TestStruct {
    pub fn caller_method(&self) -> String {
        let result = self.helper_method();
        result
    }

    fn helper_method(&self) -> String {
        self.data.join(",")
    }
}
"#;

    let ast = syn::parse_str(code).expect("Failed to parse code");
    let file_path = PathBuf::from("src/test.rs");

    let call_graph = extract_call_graph_multi_file(&[(ast, file_path.clone())]);

    // Check that helper_method exists in the graph
    let helper_func = call_graph
        .get_all_functions()
        .find(|f| f.name.contains("helper_method"))
        .expect("helper_method should exist in call graph");

    println!("Helper function: {:?}", helper_func);

    // Check that helper_method has at least one caller
    let callers = call_graph.get_callers(helper_func);
    println!("Callers of helper_method: {}", callers.len());
    for caller in &callers {
        println!(
            "  Caller: {} at {}:{}",
            caller.name,
            caller.file.display(),
            caller.line
        );
    }

    assert!(
        !callers.is_empty(),
        "helper_method should have at least one caller (caller_method), but has 0 callers"
    );

    // Verify the caller is caller_method
    let has_caller_method = callers.iter().any(|c| c.name.contains("caller_method"));
    assert!(
        has_caller_method,
        "caller_method should be in the list of callers"
    );
}

#[test]
fn test_unified_analysis_get_debt_type_key_pattern() {
    // Simplified version of the UnifiedAnalysis pattern
    let code = r#"
enum DebtItem {
    Function(FunctionDebt),
    File(FileDebt),
}

struct FunctionDebt {
    name: String,
}

struct FileDebt {
    path: String,
}

struct UnifiedAnalysis {
    items: Vec<DebtItem>,
}

impl UnifiedAnalysis {
    pub fn get_tiered_display(&self) -> Vec<String> {
        let mut results = Vec::new();

        for item in &self.items {
            let debt_type = self.get_debt_type_key(&item);
            results.push(debt_type);
        }

        results
    }

    fn get_debt_type_key(&self, item: &DebtItem) -> String {
        match item {
            DebtItem::Function(func) => format!("Function: {}", func.name),
            DebtItem::File(file) => format!("File: {}", file.path),
        }
    }
}
"#;

    let ast = syn::parse_str(code).expect("Failed to parse code");
    let file_path = PathBuf::from("src/priority/mod.rs");

    let call_graph = extract_call_graph_multi_file(&[(ast, file_path.clone())]);

    println!("\n=== ALL FUNCTIONS IN GRAPH ===");
    for func in call_graph.get_all_functions() {
        println!(
            "Function: {} at {}:{}",
            func.name,
            func.file.display(),
            func.line
        );
    }

    // Find get_debt_type_key
    let debt_type_key_func = call_graph
        .get_all_functions()
        .find(|f| f.name.contains("get_debt_type_key"))
        .expect("get_debt_type_key should exist in call graph");

    println!("\n=== TARGET FUNCTION ===");
    println!(
        "Function: {} at {}:{}",
        debt_type_key_func.name,
        debt_type_key_func.file.display(),
        debt_type_key_func.line
    );

    // Check callers
    let callers = call_graph.get_callers(debt_type_key_func);
    println!("\n=== CALLERS ===");
    println!("Number of callers: {}", callers.len());
    for caller in &callers {
        println!(
            "  Caller: {} at {}:{}",
            caller.name,
            caller.file.display(),
            caller.line
        );
    }

    assert!(
        !callers.is_empty(),
        "get_debt_type_key should have at least one caller (get_tiered_display), but has {} callers",
        callers.len()
    );

    // Verify get_tiered_display is the caller
    let has_tiered_display = callers
        .iter()
        .any(|c| c.name.contains("get_tiered_display"));
    assert!(
        has_tiered_display,
        "get_tiered_display should be in the list of callers for get_debt_type_key"
    );
}
