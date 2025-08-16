use debtmap::expansion::{ExpansionConfig, MacroExpander, MacroExpansion};
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
#[ignore] // Requires cargo-expand to be installed
fn test_cargo_expand_detects_function_calls_in_vec_macro() {
    // Create a test project with a vec! macro containing struct literals
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();

    // Write Cargo.toml
    let cargo_toml = r#"
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#;
    std::fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    // Write lib.rs with vec! macro containing struct literal
    let lib_content = r#"
pub struct Item {
    pub value: String,
}

pub fn create_items() -> Vec<Item> {
    vec![Item {
        value: helper_function(),
    }]
}

fn helper_function() -> String {
    "test".to_string()
}
"#;
    std::fs::write(src_dir.join("lib.rs"), lib_content).unwrap();

    // Create expander with test config
    let config = ExpansionConfig {
        enabled: true,
        cache_dir: temp_dir.path().join(".cache"),
        fallback_on_error: false,
        parallel: false,
        timeout_secs: 30,
    };

    // Change to the temp directory so cargo expand works correctly
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    let mut expander = MacroExpander::new(config).expect("Failed to create expander");

    // Expand the file - use relative path from project root
    let expanded = expander
        .expand_file(&PathBuf::from("src/lib.rs"))
        .expect("Failed to expand file");

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();

    // Check that the expanded content still contains the function call
    assert!(
        expanded.expanded_content.contains("helper_function"),
        "Expanded content should contain the helper_function call"
    );

    // Parse and analyze the expanded content
    let parsed =
        syn::parse_file(&expanded.expanded_content).expect("Failed to parse expanded content");

    use debtmap::analyzers::rust_call_graph::extract_call_graph;
    let call_graph = extract_call_graph(&parsed, &src_dir.join("lib.rs"));

    // Check that the call is detected
    let create_items_id = debtmap::priority::call_graph::FunctionId {
        file: src_dir.join("lib.rs"),
        name: "create_items".to_string(),
        line: 0, // Line numbers don't matter for this test
    };

    let helper_id = debtmap::priority::call_graph::FunctionId {
        file: src_dir.join("lib.rs"),
        name: "helper_function".to_string(),
        line: 0,
    };

    // Find actual function IDs
    let all_functions = call_graph.find_all_functions();
    let actual_create_items = all_functions
        .iter()
        .find(|f| f.name == "create_items")
        .expect("create_items should be in graph");

    let actual_helper = all_functions
        .iter()
        .find(|f| f.name == "helper_function")
        .expect("helper_function should be in graph");

    let calls = call_graph.get_callees(actual_create_items);
    assert!(
        calls.contains(actual_helper),
        "create_items should call helper_function after expansion"
    );

    // Verify helper_function has callers
    let callers = call_graph.get_callers(actual_helper);
    assert!(
        !callers.is_empty(),
        "helper_function should have callers after expansion"
    );
}

#[test]
fn test_expansion_error_handling() {
    use debtmap::expansion::{ExpansionConfig, MacroExpander, MacroExpansion};

    let config = ExpansionConfig {
        enabled: true,
        cache_dir: PathBuf::from(".test-cache"),
        fallback_on_error: true,
        parallel: false,
        timeout_secs: 5,
    };

    let mut expander = MacroExpander::new(config).expect("Failed to create expander");

    // Try to expand a non-existent file
    let result = expander.expand_file(&PathBuf::from("non-existent.rs"));

    // Should fail gracefully
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Failed to read file") || err.to_string().contains("No such file"),
        "Expected file not found error, got: {}",
        err
    );
}

#[test]
#[ignore] // Requires cargo-expand
fn test_expansion_cache() {
    use std::fs;

    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");

    // Create a simple Rust file
    let test_file = temp_dir.path().join("test.rs");
    fs::write(&test_file, "fn main() { println!(\"hello\"); }").unwrap();

    let config = ExpansionConfig {
        enabled: true,
        cache_dir: cache_dir.clone(),
        fallback_on_error: false,
        parallel: false,
        timeout_secs: 10,
    };

    // First expansion should create cache
    {
        let mut expander = MacroExpander::new(config.clone()).expect("Failed to create expander");
        let _ = expander.expand_file(&test_file);
    }

    // Check cache was created
    assert!(cache_dir.exists(), "Cache directory should be created");
    assert!(
        cache_dir.join("cache.json").exists(),
        "Cache file should be created"
    );

    // Second expansion should use cache
    {
        let mut expander = MacroExpander::new(config).expect("Failed to create expander");

        // This should be fast because it uses cache
        let start = std::time::Instant::now();
        let _ = expander.expand_file(&test_file);
        let duration = start.elapsed();

        // Cache hit should be very fast (< 100ms)
        assert!(
            duration.as_millis() < 100,
            "Cache lookup took too long: {:?}",
            duration
        );
    }
}
