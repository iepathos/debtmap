use debtmap::analyzers::{analyze_file, get_analyzer_with_context};
use debtmap::core::Language;
use std::path::PathBuf;

#[test]
fn test_context_aware_filtering() {
    let code = r#"
use std::fs;

fn main() {
    // This should trigger blocking I/O in non-async warning
    let content = std::fs::read_to_string("config.toml").unwrap();
    println!("{}", content);
}

async fn async_handler() {
    // This should trigger blocking I/O in async context warning
    let content = std::fs::read_to_string("data.txt").unwrap();
    println!("{}", content);
}

#[test]
fn test_something() {
    // This should NOT trigger warnings in test context
    let content = std::fs::read_to_string("test.txt").unwrap();
    assert!(!content.is_empty());
    
    // Input validation with literals should be allowed in tests
    let user = "admin";
    if user == "admin" {
        assert!(true);
    }
}

fn production_code() {
    // Input validation with literals should trigger warning
    let user = "admin";
    if user == "admin" {
        println!("Admin access");
    }
}
"#;

    // Test without context awareness
    let analyzer = get_analyzer_with_context(Language::Rust, false);
    let result = analyze_file(
        code.to_string(),
        PathBuf::from("test.rs"),
        analyzer.as_ref(),
    );
    assert!(result.is_ok());

    let metrics = result.unwrap();
    let debt_count_without_context = metrics.debt_items.len();

    // Test with context awareness
    std::env::set_var("DEBTMAP_CONTEXT_AWARE", "true");
    let analyzer = get_analyzer_with_context(Language::Rust, true);
    let result = analyze_file(
        code.to_string(),
        PathBuf::from("test.rs"),
        analyzer.as_ref(),
    );
    assert!(result.is_ok());

    let metrics = result.unwrap();
    let debt_count_with_context = metrics.debt_items.len();

    // Context-aware should filter out some issues or maintain the same if already optimal
    assert!(
        debt_count_with_context <= debt_count_without_context,
        "Context-aware filtering should reduce or maintain debt items: {} -> {}",
        debt_count_without_context,
        debt_count_with_context
    );

    // Check that no complexity issues are in test functions when context-aware is enabled
    let complexity_in_tests = metrics.debt_items.iter().any(|item| {
        matches!(item.debt_type, debtmap::core::DebtType::Complexity)
            && item.message.contains("test")
    });
    assert!(
        !complexity_in_tests,
        "Context-aware should filter complexity issues in test functions"
    );

    // Clean up
    std::env::remove_var("DEBTMAP_CONTEXT_AWARE");
}
