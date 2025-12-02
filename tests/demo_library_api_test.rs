// Demo test showing library API usage instead of subprocess spawning
mod common;

use common::analyze_code_snippet;
use debtmap::core::Language;

#[test]
#[ignore = "Demo test - may hang in CI"]
fn test_library_api_demo() {
    // This test demonstrates using the library API directly
    // instead of spawning `cargo run` subprocesses

    let rust_code = r#"
fn calculate_total(items: Vec<i32>) -> i32 {
    let mut total = 0;
    for item in items {
        total += item;
    }
    total
}

fn main() {
    // TODO: Implement the main logic
    let numbers = vec![1, 2, 3, 4, 5];
    let result = calculate_total(numbers);
    println!("Total: {}", result);
}
"#;

    // Analyze code directly using library API
    let metrics = analyze_code_snippet(rust_code, Language::Rust).expect("Failed to analyze code");

    println!("Analyzed file: {:?}", metrics.path);
    println!("Language: {:?}", metrics.language);
    println!(
        "Cyclomatic complexity: {}",
        metrics.complexity.cyclomatic_complexity
    );
    println!(
        "Cognitive complexity: {}",
        metrics.complexity.cognitive_complexity
    );
    println!("Found {} debt items", metrics.debt_items.len());

    // Check for TODO items
    let todos = metrics
        .debt_items
        .iter()
        .filter(|item| matches!(item.debt_type, debtmap::core::DebtType::Todo))
        .count();

    assert!(todos > 0, "Should have found at least one TODO");

    // This test runs directly without spawning any subprocesses
    // It's fast, reliable, and doesn't hang
    println!("Test completed successfully!");
}

#[test]
#[ignore = "Demo test - may hang in CI"]
fn test_no_subprocess_hanging() {
    // This test shows that we can run multiple analyses quickly
    // without subprocess overhead or hanging issues

    let codes = vec![
        ("fn simple() { }", Language::Rust),
        ("def simple(): pass", Language::Python),
        ("function simple() {}", Language::Rust),
    ];

    for (code, lang) in codes {
        let start = std::time::Instant::now();
        let _metrics = analyze_code_snippet(code, lang).expect("Failed to analyze code");
        let elapsed = start.elapsed();

        println!("Analyzed {} code in {:?}", lang, elapsed);
        assert!(
            elapsed.as_millis() < 100,
            "Analysis should be fast (< 100ms)"
        );
    }
}
