mod common;

use common::subprocess_converter::analyze_as_text;
use std::fs;
use std::path::Path;

fn create_test_content() -> &'static str {
    r#"
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
"#
}

fn run_context_aware_analysis(file_path: &Path) -> (String, String) {
    let stdout_without = analyze_as_text(file_path, false)
        .expect("Failed to analyze without context-aware");
    let stdout_with = analyze_as_text(file_path, true)
        .expect("Failed to analyze with context-aware");
    (stdout_without, stdout_with)
}

fn count_security_issues(output: &str) -> usize {
    output.matches("SECURITY:").count()
}

fn print_debug_output_if_needed(stdout_without: &str, stdout_with: &str, security_count_without: usize, security_count_with: usize) {
    if security_count_with >= security_count_without && security_count_without > 0 {
        println!("\n=== OUTPUT WITH --NO-CONTEXT-AWARE ===");
        println!("{}", stdout_without);
        println!("\n=== OUTPUT WITH DEFAULT (CONTEXT-AWARE) ===");
        println!("{}", stdout_with);
    }
}

fn assert_context_aware_effectiveness(security_count_without: usize, security_count_with: usize) {
    assert!(
        security_count_with < security_count_without || security_count_without == 0,
        "Context-aware should reduce security issues in test functions: {} -> {}",
        security_count_without,
        security_count_with
    );
}

#[test]
fn test_context_aware_filters_test_functions() {
    let test_content = create_test_content();
    let temp_file = "test_context_aware_temp.rs";
    
    fs::write(temp_file, test_content).unwrap();
    
    let (stdout_without, stdout_with) = run_context_aware_analysis(Path::new(temp_file));
    
    fs::remove_file(temp_file).ok();
    
    let security_count_without = count_security_issues(&stdout_without);
    let security_count_with = count_security_issues(&stdout_with);
    
    println!("Security issues with --no-context-aware: {}", security_count_without);
    println!("Security issues with default (context-aware): {}", security_count_with);
    
    print_debug_output_if_needed(&stdout_without, &stdout_with, security_count_without, security_count_with);
    assert_context_aware_effectiveness(security_count_without, security_count_with);
}
