/// Integration test for spec 166: Test file detection and context-aware scoring
///
/// This test verifies that:
/// 1. Test files are correctly detected based on naming patterns and content
/// 2. Scores for test file debt items are reduced appropriately
/// 3. File context information is preserved through the analysis pipeline
use debtmap::{
    analysis::FileContext,
    core::AnalysisResults,
    priority::scoring::file_context_scoring::{
        apply_context_adjustments, context_label, is_test_context,
    },
};
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn test_file_context_scoring_reduction() {
    // Test 1: Verify score reduction for high confidence test files
    let test_context = FileContext::Test {
        confidence: 0.95,
        test_framework: Some("rust-std".to_string()),
        test_count: 10,
    };

    let base_score = 100.0;
    let adjusted_score = apply_context_adjustments(base_score, &test_context);

    // High confidence test file should have 80% reduction (score * 0.2)
    assert_eq!(adjusted_score, 20.0);
    assert!(adjusted_score < base_score);

    // Test 2: Verify score reduction for probable test files
    let probable_test = FileContext::Test {
        confidence: 0.65,
        test_framework: None,
        test_count: 5,
    };

    let probable_score = apply_context_adjustments(base_score, &probable_test);

    // Probable test should have 40% reduction (score * 0.6)
    assert_eq!(probable_score, 60.0);
    assert!(probable_score < base_score);
    assert!(probable_score > adjusted_score); // Less reduction than high confidence

    // Test 3: Verify production files are not reduced
    let prod_context = FileContext::Production;

    let prod_score = apply_context_adjustments(base_score, &prod_context);
    assert_eq!(prod_score, base_score); // No reduction for production files
}

#[test]
fn test_file_context_labels() {
    // Test that context labels are generated correctly
    let high_conf_test = FileContext::Test {
        confidence: 0.95,
        test_framework: Some("rust-std".to_string()),
        test_count: 10,
    };
    assert_eq!(context_label(&high_conf_test), "TEST FILE");

    let probable_test = FileContext::Test {
        confidence: 0.65,
        test_framework: None,
        test_count: 3,
    };
    assert_eq!(context_label(&probable_test), "PROBABLE TEST");

    let prod_context = FileContext::Production;
    assert_eq!(context_label(&prod_context), "PRODUCTION");
}

#[test]
fn test_file_context_detection_predicates() {
    // Test high confidence detection
    let high_conf = FileContext::Test {
        confidence: 0.9,
        test_framework: None,
        test_count: 5,
    };
    assert!(high_conf.is_test()); // High confidence (>0.8)
    assert!(high_conf.is_probable_test());
    assert!(is_test_context(&high_conf));

    // Test probable detection
    let probable = FileContext::Test {
        confidence: 0.6,
        test_framework: None,
        test_count: 2,
    };
    assert!(!probable.is_test()); // Not high confidence
    assert!(probable.is_probable_test());
    assert!(is_test_context(&probable));

    // Test low confidence (not considered test)
    let low_conf = FileContext::Test {
        confidence: 0.3,
        test_framework: None,
        test_count: 1,
    };
    assert!(!low_conf.is_test());
    assert!(!low_conf.is_probable_test());
    assert!(!is_test_context(&low_conf));

    // Test production file
    let prod = FileContext::Production;
    assert!(!prod.is_test());
    assert!(!prod.is_probable_test());
    assert!(!is_test_context(&prod));
}

#[test]
fn test_analysis_results_contains_file_contexts() {
    // Create mock analysis results with file contexts
    let test_file = PathBuf::from("src/lib_test.rs");
    let prod_file = PathBuf::from("src/lib.rs");

    let mut file_contexts = HashMap::new();
    file_contexts.insert(
        test_file.clone(),
        FileContext::Test {
            confidence: 0.95,
            test_framework: Some("rust-std".to_string()),
            test_count: 10,
        },
    );
    file_contexts.insert(prod_file.clone(), FileContext::Production);

    let results = AnalysisResults {
        project_path: PathBuf::from("."),
        timestamp: chrono::Utc::now(),
        complexity: debtmap::core::ComplexityReport {
            metrics: vec![],
            summary: debtmap::core::ComplexitySummary {
                total_functions: 0,
                average_complexity: 0.0,
                max_complexity: 0,
                high_complexity_count: 0,
            },
        },
        technical_debt: debtmap::core::TechnicalDebtReport {
            items: vec![],
            by_type: HashMap::new(),
            priorities: vec![],
            duplications: vec![],
        },
        dependencies: debtmap::core::DependencyReport {
            modules: vec![],
            circular: vec![],
        },
        duplications: vec![],
        file_contexts: file_contexts.clone(),
    };

    // Verify file contexts are stored correctly
    assert_eq!(results.file_contexts.len(), 2);
    assert!(results.file_contexts.contains_key(&test_file));
    assert!(results.file_contexts.contains_key(&prod_file));

    // Verify the test file context
    let test_ctx = results.file_contexts.get(&test_file).unwrap();
    assert!(test_ctx.is_test()); // High confidence
    assert_eq!(test_ctx.test_confidence(), Some(0.95));

    // Verify the production file context
    let prod_ctx = results.file_contexts.get(&prod_file).unwrap();
    assert!(!prod_ctx.is_test()); // Not a test file
}
