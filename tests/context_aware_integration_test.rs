use std::fs;
use std::process::Command;

#[test]
fn test_context_aware_filters_test_functions() {
    // Create a test file with the exact pattern from parameter_analyzer.rs
    let test_content = r#"
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
    }

    #[test]
    fn test_another() {
        assert_eq!(7, 7);
    }
}

struct ParameterAnalyzer;
enum MaintainabilityImpact { Low }

impl ParameterAnalyzer {
    fn classify_parameter_list_impact(n: usize) -> MaintainabilityImpact {
        MaintainabilityImpact::Low
    }
}
"#;

    // Write test file
    fs::write("test_context_aware_temp.rs", test_content).unwrap();

    // Run with --no-context-aware
    let output_without = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "analyze",
            "test_context_aware_temp.rs",
            "--no-context-aware",
        ])
        .output()
        .expect("Failed to run debtmap");

    let stdout_without = String::from_utf8_lossy(&output_without.stdout);

    // Run with default (context-aware enabled)
    let output_with = Command::new("cargo")
        .args(&["run", "--", "analyze", "test_context_aware_temp.rs"])
        .output()
        .expect("Failed to run debtmap");

    let stdout_with = String::from_utf8_lossy(&output_with.stdout);

    // Clean up
    fs::remove_file("test_context_aware_temp.rs").ok();

    // Count security issues
    let security_count_without = stdout_without.matches("SECURITY:").count();
    let security_count_with = stdout_with.matches("SECURITY:").count();

    println!(
        "Security issues with --no-context-aware: {}",
        security_count_without
    );
    println!(
        "Security issues with default (context-aware): {}",
        security_count_with
    );

    // Debug output if test fails
    if security_count_with >= security_count_without && security_count_without > 0 {
        println!("\n=== OUTPUT WITH --NO-CONTEXT-AWARE ===");
        println!("{}", stdout_without);
        println!("\n=== OUTPUT WITH DEFAULT (CONTEXT-AWARE) ===");
        println!("{}", stdout_with);
    }

    // Context-aware should filter security issues in test functions
    assert!(
        security_count_with < security_count_without || security_count_without == 0,
        "Context-aware should reduce security issues in test functions: {} -> {}",
        security_count_without,
        security_count_with
    );
}
