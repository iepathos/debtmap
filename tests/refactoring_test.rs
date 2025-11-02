use debtmap::core::{FileMetrics, FunctionMetrics};
use debtmap::refactoring::{ComplexityLevel, PatternRecognitionEngine, RefactoringOpportunity};
use std::path::PathBuf;

fn create_test_function(name: &str, cyclomatic: u32, cognitive: u32) -> FunctionMetrics {
    FunctionMetrics {
        name: name.to_string(),
        file: PathBuf::from("test.rs"),
        line: 10,
        cyclomatic,
        cognitive,
        length: 50,
        nesting: 2,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
        language_specific: None,
        purity_reason: None,
        call_dependencies: None,
    }
}

fn create_test_file() -> FileMetrics {
    FileMetrics {
        path: PathBuf::from("test.rs"),
        language: debtmap::core::Language::Rust,
        complexity: debtmap::core::ComplexityMetrics::default(),
        debt_items: vec![],
        dependencies: vec![],
        duplications: vec![],
        module_scope: None,
        classes: None,
    }
}

#[test]
fn test_moderate_complexity_refactoring_guidance() {
    let engine = PatternRecognitionEngine::new();
    let func = create_test_function("calculate_total", 8, 6);
    let file = create_test_file();

    let analysis = engine.analyze_function(&func, &file);

    // Should suggest extracting pure functions
    assert!(!analysis.refactoring_opportunities.is_empty());

    if let Some(RefactoringOpportunity::ExtractPureFunctions {
        complexity_level,
        suggested_functions,
        functional_patterns,
        ..
    }) = analysis.refactoring_opportunities.first()
    {
        assert!(matches!(complexity_level, ComplexityLevel::Moderate));
        assert!(suggested_functions.len() >= 2);
        assert!(!functional_patterns.is_empty());

        // Should always suggest pure functions
        for func_spec in suggested_functions {
            assert!(func_spec.no_side_effects);
        }
    } else {
        panic!("Expected ExtractPureFunctions recommendation");
    }
}

#[test]
fn test_high_complexity_refactoring_guidance() {
    let engine = PatternRecognitionEngine::new();
    let func = create_test_function("process_data", 12, 15);
    let file = create_test_file();

    let analysis = engine.analyze_function(&func, &file);

    assert!(!analysis.refactoring_opportunities.is_empty());

    if let Some(RefactoringOpportunity::ExtractPureFunctions {
        complexity_level,
        suggested_functions,
        ..
    }) = analysis.refactoring_opportunities.first()
    {
        assert!(matches!(complexity_level, ComplexityLevel::High));
        assert!(suggested_functions.len() >= 3);

        // All suggested functions should be pure
        for func_spec in suggested_functions {
            assert!(func_spec.no_side_effects);
        }
    } else {
        panic!("Expected ExtractPureFunctions recommendation");
    }
}

#[test]
fn test_severe_complexity_refactoring_guidance() {
    let engine = PatternRecognitionEngine::new();
    let func = create_test_function("mega_function", 20, 25);
    let file = create_test_file();

    let analysis = engine.analyze_function(&func, &file);

    assert!(!analysis.refactoring_opportunities.is_empty());

    if let Some(RefactoringOpportunity::ExtractPureFunctions {
        complexity_level,
        extraction_strategy,
        ..
    }) = analysis.refactoring_opportunities.first()
    {
        assert!(matches!(complexity_level, ComplexityLevel::Severe));

        // Should suggest architectural refactoring
        if let debtmap::refactoring::ExtractionStrategy::ArchitecturalRefactoring {
            pure_core_functions,
            ..
        } = extraction_strategy
        {
            assert!(!pure_core_functions.is_empty());

            // All core functions should be pure
            for func_spec in pure_core_functions {
                assert!(func_spec.no_side_effects);
            }
        } else {
            panic!("Expected ArchitecturalRefactoring strategy");
        }
    } else {
        panic!("Expected ExtractPureFunctions recommendation");
    }
}

#[test]
fn test_low_complexity_no_refactoring() {
    let engine = PatternRecognitionEngine::new();
    let func = create_test_function("simple_function", 3, 2);
    let file = create_test_file();

    let analysis = engine.analyze_function(&func, &file);

    // Low complexity functions should not have refactoring recommendations
    let has_complexity_refactoring = analysis
        .refactoring_opportunities
        .iter()
        .any(|opp| matches!(opp, RefactoringOpportunity::ExtractPureFunctions { .. }));

    assert!(!has_complexity_refactoring);
}

#[test]
fn test_io_function_pattern_detection() {
    let engine = PatternRecognitionEngine::new();
    let func = create_test_function("save_file", 7, 5);
    let file = create_test_file();

    let analysis = engine.analyze_function(&func, &file);

    // Should detect I/O pattern
    assert!(matches!(
        analysis.function_role,
        debtmap::refactoring::FunctionRole::IOOrchestrator { .. }
    ));
}

#[test]
fn test_formatting_function_pattern_detection() {
    let engine = PatternRecognitionEngine::new();
    let func = create_test_function("format_output", 4, 3);
    let file = create_test_file();

    let analysis = engine.analyze_function(&func, &file);

    // Should detect formatting pattern
    assert!(matches!(
        analysis.function_role,
        debtmap::refactoring::FunctionRole::FormattingFunction { .. }
    ));
}

#[test]
fn test_trait_implementation_detection() {
    let engine = PatternRecognitionEngine::new();
    let func = create_test_function("fmt", 2, 1);
    let file = create_test_file();

    let analysis = engine.analyze_function(&func, &file);

    // Should detect trait implementation
    assert!(matches!(
        analysis.function_role,
        debtmap::refactoring::FunctionRole::TraitImplementation { .. }
    ));
}

#[test]
fn test_functional_patterns_suggested() {
    let engine = PatternRecognitionEngine::new();
    let func = create_test_function("process_items", 9, 8);
    let file = create_test_file();

    let analysis = engine.analyze_function(&func, &file);

    if let Some(RefactoringOpportunity::ExtractPureFunctions {
        functional_patterns,
        ..
    }) = analysis.refactoring_opportunities.first()
    {
        // Should suggest functional patterns
        assert!(!functional_patterns.is_empty());

        // Should include common patterns
        let pattern_names: Vec<String> = functional_patterns
            .iter()
            .map(|p| format!("{:?}", p))
            .collect();

        assert!(
            pattern_names.iter().any(|p| p.contains("Map"))
                || pattern_names.iter().any(|p| p.contains("Filter"))
                || pattern_names.iter().any(|p| p.contains("Fold"))
        );
    }
}

#[test]
fn test_consistent_pure_function_terminology() {
    let engine = PatternRecognitionEngine::new();

    // Test moderate complexity
    let func1 = create_test_function("func1", 7, 6);
    let file = create_test_file();
    let analysis1 = engine.analyze_function(&func1, &file);

    // Test high complexity
    let func2 = create_test_function("func2", 13, 12);
    let analysis2 = engine.analyze_function(&func2, &file);

    // Both should recommend extracting PURE functions
    for analysis in [analysis1, analysis2] {
        if let Some(RefactoringOpportunity::ExtractPureFunctions {
            suggested_functions,
            ..
        }) = analysis.refactoring_opportunities.first()
        {
            // All functions should be marked as pure (no side effects)
            for func_spec in suggested_functions {
                assert!(
                    func_spec.no_side_effects,
                    "All extracted functions should be pure"
                );
            }
        }
    }
}

#[test]
fn test_complexity_determines_strategy_not_purity() {
    let engine = PatternRecognitionEngine::new();
    let file = create_test_file();

    // Different complexity levels
    let moderate = create_test_function("moderate", 8, 7);
    let high = create_test_function("high", 12, 11);
    let severe = create_test_function("severe", 18, 20);

    let moderate_analysis = engine.analyze_function(&moderate, &file);
    let high_analysis = engine.analyze_function(&high, &file);
    let severe_analysis = engine.analyze_function(&severe, &file);

    // All should extract pure functions
    for analysis in [moderate_analysis, high_analysis, severe_analysis] {
        if let Some(RefactoringOpportunity::ExtractPureFunctions {
            suggested_functions,
            extraction_strategy,
            ..
        }) = analysis.refactoring_opportunities.first()
        {
            // Functions are always pure
            if !suggested_functions.is_empty() {
                for func in suggested_functions {
                    assert!(func.no_side_effects);
                }
            }

            // But strategy differs based on complexity
            match extraction_strategy {
                debtmap::refactoring::ExtractionStrategy::DirectFunctionalTransformation {
                    ..
                } => {
                    // Moderate complexity
                }
                debtmap::refactoring::ExtractionStrategy::DecomposeAndTransform { .. } => {
                    // High complexity
                }
                debtmap::refactoring::ExtractionStrategy::ArchitecturalRefactoring {
                    pure_core_functions,
                    ..
                } => {
                    // Severe complexity - even architectural refactoring uses pure functions
                    for func in pure_core_functions {
                        assert!(func.no_side_effects);
                    }
                }
            }
        }
    }
}
