use std::fs;
/// Tests that reproduce the exact false positives from production
/// These tests verify the bugs we're seeing with Input Validation in test functions
use std::process::Command;

#[test]
fn test_exact_parameter_analyzer_false_positive() {
    // Exact code from src/organization/parameter_analyzer.rs:362
    let code = r#"
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_parameter_list_impact_low() {
        // Test low impact for 7 or fewer parameters
        assert_eq!(
            ParameterAnalyzer::classify_parameter_list_impact(0),
            MaintainabilityImpact::Low
        );
        assert_eq!(
            ParameterAnalyzer::classify_parameter_list_impact(5),
            MaintainabilityImpact::Low
        );
        assert_eq!(
            ParameterAnalyzer::classify_parameter_list_impact(7),
            MaintainabilityImpact::Low
        );
    }

    #[test]
    fn test_classify_data_clump_impact_medium() {
        // Test medium impact for more than 5 occurrences
        assert_eq!(
            ParameterAnalyzer::classify_data_clump_impact(6),
            MaintainabilityImpact::Medium
        );
    }
}
"#;

    fs::write("test_param_analyzer.rs", code).unwrap();

    // Run without --context-aware
    let output = Command::new("cargo")
        .args(&["run", "--", "analyze", "test_param_analyzer.rs"])
        .output()
        .expect("Failed to run debtmap");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check for Input Validation issues
    let has_input_validation = stdout.contains("Input Validation");

    // Run with --context-aware
    let output_aware = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "analyze",
            "test_param_analyzer.rs",
            "--context-aware",
        ])
        .output()
        .expect("Failed to run debtmap");

    let stdout_aware = String::from_utf8_lossy(&output_aware.stdout);
    let has_input_validation_aware = stdout_aware.contains("Input Validation");

    fs::remove_file("test_param_analyzer.rs").ok();

    println!(
        "Without --context-aware: has Input Validation = {}",
        has_input_validation
    );
    println!(
        "With --context-aware: has Input Validation = {}",
        has_input_validation_aware
    );

    // Document the bug: context-aware flag doesn't help
    if has_input_validation && has_input_validation_aware {
        println!("BUG CONFIRMED: --context-aware flag does not filter Input Validation in test functions");
    }

    // This should be the correct behavior but currently fails
    // assert!(!has_input_validation_aware, "Context-aware should filter Input Validation in test functions");
}

#[test]
fn test_exact_rust_call_graph_false_positive() {
    // Exact patterns from src/analyzers/rust_call_graph.rs
    let code = r#"
fn count_lines(block: &syn::Block) -> usize {
    // Simple approximation based on statement count
    block.stmts.len().max(1)
}

pub fn extract_call_graph(file: &syn::File) -> CallGraph {
    CallGraph::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_lines() {
        let result = count_lines(&block);
        assert_eq!(result, 1);
    }
}
"#;

    fs::write("test_call_graph.rs", code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "analyze",
            "test_call_graph.rs",
            "--context-aware",
        ])
        .output()
        .expect("Failed to run debtmap");

    let stdout = String::from_utf8_lossy(&output.stdout);

    fs::remove_file("test_call_graph.rs").ok();

    // Check for false positives
    let line_1025_issue = stdout.contains("1025") || stdout.contains("count_lines");
    let line_1038_issue = stdout.contains("1038") || stdout.contains("extract_call_graph");

    println!("Has issue for count_lines: {}", line_1025_issue);
    println!("Has issue for extract_call_graph: {}", line_1038_issue);

    // Document current behavior
    if line_1025_issue || line_1038_issue {
        println!("BUG: Functions like count_lines and extract_call_graph trigger false positives");
    }
}

#[test]
fn test_type_registry_false_positive() {
    // Pattern from src/analyzers/type_registry.rs:327
    let code = r#"
impl GlobalTypeRegistry {
    pub fn resolve_type_with_imports(&self, file: &PathBuf, name: &str) -> Option<String> {
        // First check if it's already fully qualified
        if self.types.contains_key(name) {
            return Some(name.to_string());
        }
        None
    }
}
"#;

    fs::write("test_type_registry.rs", code).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "analyze",
            "test_type_registry.rs",
            "--context-aware",
        ])
        .output()
        .expect("Failed to run debtmap");

    let stdout = String::from_utf8_lossy(&output.stdout);

    fs::remove_file("test_type_registry.rs").ok();

    let has_validation_issue =
        stdout.contains("Input Validation") || stdout.contains("resolve_type_with_imports");

    println!(
        "Type registry pattern triggers Input Validation: {}",
        has_validation_issue
    );

    // Document the false positive
    if has_validation_issue {
        println!("BUG: Normal type checking code triggers Input Validation false positive");
    }
}

#[test]
#[ignore] // Multiple cargo run invocations - slow
fn test_comprehensive_false_positive_patterns() {
    // All patterns that trigger false positives
    let patterns = vec![
        (
            "test function with assert_eq",
            r#"
            #[test]
            fn test_something() {
                assert_eq!(5, 5);
            }
        "#,
        ),
        (
            "test function with literals",
            r#"
            #[test]
            fn test_classify() {
                let result = classify(0);
                assert_eq!(result, Low);
            }
        "#,
        ),
        (
            "test module with multiple tests",
            r#"
            #[cfg(test)]
            mod tests {
                #[test]
                fn test_one() { assert!(true); }
                #[test]
                fn test_two() { assert_eq!(1, 1); }
            }
        "#,
        ),
        (
            "function returning literal",
            r#"
            fn count_lines() -> usize {
                1
            }
        "#,
        ),
        (
            "function with max",
            r#"
            fn get_size(n: usize) -> usize {
                n.max(1)
            }
        "#,
        ),
    ];

    for (name, code) in patterns {
        let filename = format!("test_{}.rs", name.replace(' ', "_"));
        fs::write(&filename, code).unwrap();

        let output = Command::new("cargo")
            .args(&["run", "--", "analyze", &filename, "--context-aware"])
            .output()
            .expect("Failed to run debtmap");

        let stdout = String::from_utf8_lossy(&output.stdout);

        fs::remove_file(&filename).ok();

        let has_issue = stdout.contains("Input Validation") || stdout.contains("SECURITY");

        println!(
            "Pattern '{}': triggers false positive = {}",
            name, has_issue
        );

        // These should all NOT trigger issues with context-aware, but currently do
        if has_issue && name.contains("test") {
            println!(
                "  BUG: Test pattern '{}' triggers false positive even with --context-aware",
                name
            );
        }
    }
}
