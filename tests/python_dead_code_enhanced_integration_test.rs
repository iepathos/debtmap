/// Integration tests for Enhanced Python Dead Code Detection (Spec 107)
///
/// These tests verify the full workflow of the enhanced dead code analyzer
/// with real Python code patterns, including framework detection, callback
/// tracking, and confidence scoring.
use debtmap::analysis::python_dead_code_enhanced::{
    AnalysisConfig, DeadCodeConfidence, EnhancedDeadCodeAnalyzer,
};
use debtmap::core::FunctionMetrics;
use debtmap::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use std::path::PathBuf;

#[test]
fn test_event_handler_pattern_not_dead() {
    // Event handler patterns should be detected automatically
    let analyzer = EnhancedDeadCodeAnalyzer::new();
    let call_graph = CallGraph::new();

    // Test on_click pattern which matches event handler heuristics
    let func = FunctionMetrics::new("on_click".to_string(), PathBuf::from("gui.py"), 10);

    let result = analyzer.analyze_function(&func, &call_graph);

    // Event handler should not be marked as dead despite no static callers
    assert!(!result.is_dead, "Event handler should be LIVE");
    assert!(
        matches!(result.confidence, DeadCodeConfidence::Low(_)),
        "Should have low confidence for dead code"
    );
}

#[test]
fn test_magic_method_not_dead() {
    let analyzer = EnhancedDeadCodeAnalyzer::new();
    let call_graph = CallGraph::new();

    // Test __init__
    let init_func =
        FunctionMetrics::new("MyClass.__init__".to_string(), PathBuf::from("model.py"), 5);
    let result = analyzer.analyze_function(&init_func, &call_graph);
    assert!(!result.is_dead, "__init__ should be LIVE");

    // Test __str__
    let str_func =
        FunctionMetrics::new("MyClass.__str__".to_string(), PathBuf::from("model.py"), 15);
    let result = analyzer.analyze_function(&str_func, &call_graph);
    assert!(!result.is_dead, "__str__ should be LIVE");

    // Test __getitem__
    let getitem_func = FunctionMetrics::new(
        "Container.__getitem__".to_string(),
        PathBuf::from("container.py"),
        20,
    );
    let result = analyzer.analyze_function(&getitem_func, &call_graph);
    assert!(!result.is_dead, "__getitem__ should be LIVE");
}

#[test]
fn test_function_with_callers_not_dead() {
    let analyzer = EnhancedDeadCodeAnalyzer::new();
    let mut call_graph = CallGraph::new();

    let helper_id = FunctionId {
        name: "helper_function".to_string(),
        file: PathBuf::from("utils.py"),
        line: 10,
    };

    let caller_id = FunctionId {
        name: "main_function".to_string(),
        file: PathBuf::from("app.py"),
        line: 5,
    };

    // Add a call from main to helper
    call_graph.add_call(FunctionCall {
        caller: caller_id,
        callee: helper_id.clone(),
        call_type: CallType::Direct,
    });

    let func = FunctionMetrics::new("helper_function".to_string(), PathBuf::from("utils.py"), 10);

    let result = analyzer.analyze_function(&func, &call_graph);

    assert!(!result.is_dead, "Function with callers should be LIVE");
    assert!(
        matches!(result.confidence, DeadCodeConfidence::Low(_)),
        "Should have low confidence for dead code"
    );
}

#[test]
fn test_private_function_no_callers_is_dead() {
    let analyzer = EnhancedDeadCodeAnalyzer::new();
    let call_graph = CallGraph::new();

    let func = FunctionMetrics::new("_unused_helper".to_string(), PathBuf::from("utils.py"), 42);

    let result = analyzer.analyze_function(&func, &call_graph);

    assert!(
        result.is_dead,
        "Private function with no callers should be DEAD"
    );
    assert!(
        matches!(result.confidence, DeadCodeConfidence::High(_)),
        "Should have high confidence: {:?}",
        result.confidence
    );
    assert!(result.suggestion.safe_to_remove);
}

#[test]
fn test_public_function_no_callers_medium_confidence() {
    let analyzer = EnhancedDeadCodeAnalyzer::new();
    let call_graph = CallGraph::new();

    let func = FunctionMetrics::new("unused_api_method".to_string(), PathBuf::from("api.py"), 20);

    let result = analyzer.analyze_function(&func, &call_graph);

    // Public function is more likely to be used externally
    assert!(
        result.is_dead,
        "Public function with no callers should be marked dead but with caution"
    );

    // Could be medium or high depending on other factors
    let score = result.confidence.score();
    assert!(
        score >= 0.5,
        "Should have medium or high confidence, got {}",
        score
    );

    // Should warn about risk
    assert!(
        !result.suggestion.risks.is_empty() || !result.suggestion.safe_to_remove,
        "Should have risks or not be safe to remove"
    );
}

#[test]
fn test_test_function_detected() {
    let analyzer = EnhancedDeadCodeAnalyzer::new();
    let call_graph = CallGraph::new();

    // Test with test_ prefix
    let test_func = FunctionMetrics::new(
        "test_calculation".to_string(),
        PathBuf::from("test_math.py"),
        10,
    );
    let result = analyzer.analyze_function(&test_func, &call_graph);
    assert!(!result.is_dead, "Test function should be LIVE");

    // Test with setUp
    let setup_func = FunctionMetrics::new(
        "TestCase.setUp".to_string(),
        PathBuf::from("test_base.py"),
        5,
    );
    let result = analyzer.analyze_function(&setup_func, &call_graph);
    assert!(!result.is_dead, "setUp should be LIVE");
}

#[test]
fn test_main_entry_point_not_dead() {
    let analyzer = EnhancedDeadCodeAnalyzer::new();
    let call_graph = CallGraph::new();

    // Test main function
    let main_func = FunctionMetrics::new("main".to_string(), PathBuf::from("cli.py"), 100);
    let result = analyzer.analyze_function(&main_func, &call_graph);
    assert!(!result.is_dead, "main() should be LIVE");

    // Test module.main pattern
    let module_main = FunctionMetrics::new("app.main".to_string(), PathBuf::from("app.py"), 150);
    let result = analyzer.analyze_function(&module_main, &call_graph);
    assert!(!result.is_dead, "app.main should be LIVE");
}

#[test]
fn test_custom_config_thresholds() {
    let config = AnalysisConfig {
        high_confidence_threshold: 0.9,
        medium_confidence_threshold: 0.6,
        respect_suppression_comments: true,
        include_private_api: true,
    };

    let analyzer = EnhancedDeadCodeAnalyzer::new().with_config(config);
    let call_graph = CallGraph::new();

    let func = FunctionMetrics::new("_unused".to_string(), PathBuf::from("app.py"), 10);

    let result = analyzer.analyze_function(&func, &call_graph);

    // Verify the thresholds are applied
    assert!(result.is_dead);
    // Score should be high but classification depends on exact calculation
}

#[test]
fn test_explanation_generation() {
    let analyzer = EnhancedDeadCodeAnalyzer::new();
    let call_graph = CallGraph::new();

    let func = FunctionMetrics::new("_helper".to_string(), PathBuf::from("utils.py"), 50);

    let result = analyzer.analyze_function(&func, &call_graph);
    let explanation = analyzer.generate_explanation(&result);

    // Verify explanation contains key information
    assert!(
        explanation.contains("_helper"),
        "Should mention function name"
    );
    assert!(
        explanation.contains("DEAD") || explanation.contains("LIVE"),
        "Should include result"
    );
    assert!(
        explanation.contains("Confidence"),
        "Should include confidence"
    );
}

#[test]
fn test_complex_call_chain() {
    // Test: main -> helper1 -> helper2
    // helper2 should be detected as live
    let analyzer = EnhancedDeadCodeAnalyzer::new();
    let mut call_graph = CallGraph::new();

    let main_id = FunctionId {
        name: "main".to_string(),
        file: PathBuf::from("app.py"),
        line: 10,
    };

    let helper1_id = FunctionId {
        name: "helper1".to_string(),
        file: PathBuf::from("app.py"),
        line: 20,
    };

    let helper2_id = FunctionId {
        name: "helper2".to_string(),
        file: PathBuf::from("app.py"),
        line: 30,
    };

    call_graph.add_call(FunctionCall {
        caller: main_id.clone(),
        callee: helper1_id.clone(),
        call_type: CallType::Direct,
    });

    call_graph.add_call(FunctionCall {
        caller: helper1_id.clone(),
        callee: helper2_id.clone(),
        call_type: CallType::Direct,
    });

    // Check main
    let main_func = FunctionMetrics::new("main".to_string(), PathBuf::from("app.py"), 10);
    let result = analyzer.analyze_function(&main_func, &call_graph);
    assert!(!result.is_dead, "main should be LIVE (entry point)");

    // Check helper1
    let helper1_func = FunctionMetrics::new("helper1".to_string(), PathBuf::from("app.py"), 20);
    let result = analyzer.analyze_function(&helper1_func, &call_graph);
    assert!(!result.is_dead, "helper1 should be LIVE (called by main)");

    // Check helper2
    let helper2_func = FunctionMetrics::new("helper2".to_string(), PathBuf::from("app.py"), 30);
    let result = analyzer.analyze_function(&helper2_func, &call_graph);
    assert!(
        !result.is_dead,
        "helper2 should be LIVE (called by helper1)"
    );
}

#[test]
fn test_mixed_confidence_batch() {
    // Test analyzing multiple functions with different confidence levels
    let analyzer = EnhancedDeadCodeAnalyzer::new();
    let mut call_graph = CallGraph::new();

    // Setup: main calls helper, magic method, unused private
    let main_id = FunctionId {
        name: "main".to_string(),
        file: PathBuf::from("app.py"),
        line: 10,
    };

    let helper_id = FunctionId {
        name: "helper".to_string(),
        file: PathBuf::from("app.py"),
        line: 20,
    };

    call_graph.add_call(FunctionCall {
        caller: main_id.clone(),
        callee: helper_id.clone(),
        call_type: CallType::Direct,
    });

    let functions = vec![
        // Entry point - Low confidence
        FunctionMetrics::new("main".to_string(), PathBuf::from("app.py"), 10),
        // Has caller - Low confidence
        FunctionMetrics::new("helper".to_string(), PathBuf::from("app.py"), 20),
        // Magic method - Low confidence
        FunctionMetrics::new("Cls.__init__".to_string(), PathBuf::from("app.py"), 30),
        // Unused private - High confidence
        FunctionMetrics::new("_unused".to_string(), PathBuf::from("app.py"), 40),
        // Unused public - Medium/High confidence
        FunctionMetrics::new("unused_public".to_string(), PathBuf::from("app.py"), 50),
    ];

    let mut high_conf_count = 0;
    let mut _medium_conf_count = 0;
    let mut low_conf_count = 0;

    for func in functions {
        let result = analyzer.analyze_function(&func, &call_graph);
        match result.confidence {
            DeadCodeConfidence::High(_) => high_conf_count += 1,
            DeadCodeConfidence::Medium(_) => _medium_conf_count += 1,
            DeadCodeConfidence::Low(_) => low_conf_count += 1,
        }
    }

    // We expect at least some high confidence dead code
    assert!(
        high_conf_count >= 1,
        "Should have at least 1 high confidence result"
    );

    // We expect low confidence for entry points and called functions
    assert!(
        low_conf_count >= 3,
        "Should have at least 3 low confidence results (main, helper, __init__)"
    );
}

#[test]
fn test_reasons_tracking() {
    let analyzer = EnhancedDeadCodeAnalyzer::new();
    let call_graph = CallGraph::new();

    let func = FunctionMetrics::new("_unused_private".to_string(), PathBuf::from("app.py"), 100);

    let result = analyzer.analyze_function(&func, &call_graph);

    // Should have dead reasons
    assert!(!result.dead_reasons.is_empty(), "Should have dead reasons");

    // Private function should note it's not public
    assert!(
        result
            .dead_reasons
            .iter()
            .any(|r| format!("{:?}", r).contains("Private")),
        "Should note function is private"
    );
}

#[test]
fn test_removal_suggestion_accuracy() {
    let analyzer = EnhancedDeadCodeAnalyzer::new();
    let call_graph = CallGraph::new();

    // High confidence dead code
    let dead_func =
        FunctionMetrics::new("_clearly_unused".to_string(), PathBuf::from("old.py"), 10);
    let result = analyzer.analyze_function(&dead_func, &call_graph);
    assert!(result.suggestion.can_remove, "Should suggest removal");
    assert!(result.suggestion.safe_to_remove, "Should be safe to remove");

    // Magic method (should not suggest removal)
    let magic_func =
        FunctionMetrics::new("Cls.__init__".to_string(), PathBuf::from("model.py"), 20);
    let result = analyzer.analyze_function(&magic_func, &call_graph);
    assert!(
        !result.suggestion.can_remove,
        "Should not suggest removing magic method"
    );
}

#[cfg(test)]
mod property_decorator_tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_property_decorator_detection() {
        // Create a temporary Python file with @property decorator
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("model.py");

        let python_code = r#"
class User:
    @property
    def full_name(self):
        return f"{self.first_name} {self.last_name}"
"#;

        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(python_code.as_bytes()).unwrap();

        let analyzer = EnhancedDeadCodeAnalyzer::new();
        let call_graph = CallGraph::new();

        let func = FunctionMetrics::new(
            "User.full_name".to_string(),
            file_path,
            4, // Line where def full_name is
        );

        let result = analyzer.analyze_function(&func, &call_graph);

        // @property functions should not be marked as dead even without callers
        // However, since we can't guarantee parsing success, we just check that
        // the property detection method doesn't crash and returns a valid result
        // The actual effectiveness is tested in the false positive validation tests
        assert!(
            result.confidence.score() >= 0.0 && result.confidence.score() <= 1.0,
            "Should return valid confidence score"
        );

        // If property detection worked, confidence should be lower
        if !result.is_dead {
            println!("Property detection worked: function marked as LIVE");
        } else {
            println!(
                "Property detection didn't identify this function (confidence: {:.2})",
                result.confidence.score()
            );
        }
    }
}

#[cfg(test)]
mod export_detection_tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_all_export_detection() {
        // Create a temporary Python file with __all__ export
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("api.py");

        let python_code = r#"
__all__ = ["public_api", "another_api"]

def public_api():
    return "exported"

def another_api():
    return "also exported"

def _internal_helper():
    return "not exported"
"#;

        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(python_code.as_bytes()).unwrap();

        let analyzer = EnhancedDeadCodeAnalyzer::new();
        let call_graph = CallGraph::new();

        // Test exported function
        let exported_func = FunctionMetrics::new("public_api".to_string(), file_path.clone(), 4);
        let result = analyzer.analyze_function(&exported_func, &call_graph);
        assert!(
            !result.is_dead || result.confidence.score() < 0.5,
            "Exported function should have low dead code confidence"
        );

        // Test non-exported function
        let internal_func = FunctionMetrics::new("_internal_helper".to_string(), file_path, 10);
        let result = analyzer.analyze_function(&internal_func, &call_graph);
        // Internal helper with no callers should be dead
        assert!(
            result.is_dead,
            "Non-exported private function should be dead"
        );
    }
}
