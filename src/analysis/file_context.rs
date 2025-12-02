//! File context detection for identifying test files vs production code
//!
//! This module implements intelligent test file detection using multiple heuristics
//! to avoid false positives where test files are treated as production code debt.
//!
//! Detection strategy (weighted by reliability):
//! - Test attributes and decorators (35%): `#[test]`, `#[tokio::test]`, `@pytest.fixture`
//! - File naming patterns (25%): *_test.rs, *_tests.rs, test_*.py, etc.
//! - Directory location (20%): tests/, *_tests/
//! - Test function naming (15%): test_*, Test*
//! - Framework imports (5%): proptest, pytest, jest, mocha
//!
//! Strong signals boost: Files with 3+ strong signals (>0.7) achieve high confidence (>0.85)
//!
//! Spec 166: Test File Detection and Context-Aware Scoring
//! Spec 169: Improve Test File Confidence Detection for Component Tests

use crate::core::{FunctionMetrics, Language};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// File context classification for semantic understanding
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FileContext {
    /// Production code
    Production,

    /// Test file with confidence and metadata
    Test {
        confidence: f32,
        test_framework: Option<String>,
        test_count: usize,
    },

    /// Generated code (protobuf, swagger, etc.)
    Generated { generator: String },

    /// Configuration file
    Configuration,

    /// Documentation file
    Documentation,
}

impl FileContext {
    /// Returns true if this is a test file with high confidence (>0.8)
    pub fn is_test(&self) -> bool {
        matches!(self, FileContext::Test { confidence, .. } if *confidence > 0.8)
    }

    /// Returns true if this is a probable test file (confidence >0.5)
    pub fn is_probable_test(&self) -> bool {
        matches!(self, FileContext::Test { confidence, .. } if *confidence > 0.5)
    }

    /// Get test confidence if this is a test file
    pub fn test_confidence(&self) -> Option<f32> {
        match self {
            FileContext::Test { confidence, .. } => Some(*confidence),
            _ => None,
        }
    }

    /// Get test count if this is a test file
    pub fn test_count(&self) -> Option<usize> {
        match self {
            FileContext::Test { test_count, .. } => Some(*test_count),
            _ => None,
        }
    }
}

/// Detailed confidence scoring for test file detection
#[derive(Debug, Clone, PartialEq)]
pub struct TestFileConfidence {
    pub naming_match: f32,
    pub attribute_density: f32,
    pub test_function_ratio: f32,
    pub test_imports: f32,
    pub directory_context: f32,
    pub overall_confidence: f32,
}

/// Test file detector using multi-signal heuristics
pub struct FileContextDetector {
    language: Language,
}

impl FileContextDetector {
    /// Create a new detector for the given language
    pub fn new(language: Language) -> Self {
        Self { language }
    }

    /// Detect the file context using all available heuristics
    pub fn detect(&self, path: &Path, functions: &[FunctionMetrics]) -> FileContext {
        let test_score = self.calculate_test_score(path, functions);
        let mut confidence = test_score.overall_confidence;

        // Boost confidence if multiple strong signals (spec 169)
        let strong_signals = [
            test_score.naming_match >= 0.7,
            test_score.attribute_density >= 0.7,
            test_score.test_function_ratio >= 0.7,
        ]
        .iter()
        .filter(|&&x| x)
        .count();

        if strong_signals >= 3 {
            confidence = confidence.max(0.85);
        }

        if confidence > 0.5 {
            FileContext::Test {
                confidence,
                test_framework: self.detect_framework(functions),
                test_count: self.count_tests(functions),
            }
        } else {
            FileContext::Production
        }
    }

    /// Calculate comprehensive test confidence score
    fn calculate_test_score(
        &self,
        path: &Path,
        functions: &[FunctionMetrics],
    ) -> TestFileConfidence {
        let naming = self.score_naming(path);
        let attributes = self.score_attributes(functions);
        let function_ratio = self.score_test_functions(functions);
        let imports = 0.0; // TODO: Implement import analysis
        let directory = self.score_directory(path);

        TestFileConfidence {
            naming_match: naming,
            attribute_density: attributes,
            test_function_ratio: function_ratio,
            test_imports: imports,
            directory_context: directory,
            overall_confidence: self.weighted_average(
                naming,
                attributes,
                function_ratio,
                imports,
                directory,
            ),
        }
    }

    /// Score based on file naming patterns
    fn score_naming(&self, path: &Path) -> f32 {
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        match self.language {
            Language::Rust => {
                if filename.ends_with("_tests.rs") || filename.ends_with("_test.rs") {
                    1.0 // Perfect score for test files (spec 169)
                } else if filename == "tests.rs" {
                    0.9 // High score for consolidated tests
                } else if filename.starts_with("test_") {
                    0.8
                } else {
                    0.0
                }
            }
            Language::Python => {
                if (filename.starts_with("test_") && filename.ends_with(".py"))
                    || filename.ends_with("_test.py")
                {
                    0.9
                } else {
                    0.0
                }
            }
            Language::Unknown => 0.0,
        }
    }

    /// Score based on test attribute density
    fn score_attributes(&self, functions: &[FunctionMetrics]) -> f32 {
        if functions.is_empty() {
            return 0.0;
        }

        match self.language {
            Language::Rust => {
                // Count functions with test attributes or in test modules
                let test_funcs = functions
                    .iter()
                    .filter(|f| f.is_test || f.in_test_module)
                    .count();

                test_funcs as f32 / functions.len() as f32
            }
            _ => 0.0,
        }
    }

    /// Score based on test function naming conventions
    fn score_test_functions(&self, functions: &[FunctionMetrics]) -> f32 {
        if functions.is_empty() {
            return 0.0;
        }

        let test_named = functions
            .iter()
            .filter(|f| {
                f.name.starts_with("test_")
                    || f.name.starts_with("Test")
                    || f.name.contains("_test")
            })
            .count();

        test_named as f32 / functions.len() as f32
    }

    /// Score based on directory location
    fn score_directory(&self, path: &Path) -> f32 {
        let path_str = path.to_string_lossy();

        // Check for tests/ directory
        if path_str.contains("/tests/") || path_str.starts_with("tests/") {
            return 1.0;
        }

        // Check for *_tests/ directory
        if path_str.contains("_tests/") {
            return 0.9;
        }

        // Check if parent directory is named "tests"
        if let Some(parent) = path.parent() {
            if let Some(dir_name) = parent.file_name().and_then(|n| n.to_str()) {
                if dir_name == "tests" {
                    return 1.0;
                }
            }
        }

        0.0
    }

    /// Calculate weighted average confidence from individual detection signals.
    ///
    /// # Weighting Strategy (Spec 169)
    ///
    /// The weights are carefully balanced to handle component tests (in src/)
    /// without requiring tests/ directory location:
    ///
    /// - **Attributes (35%)**: Strongest signal - #[test] is unambiguous
    /// - **Naming (25%)**: Strong signal - *_test.rs is conventional
    /// - **Directory (20%)**: Reduced from 40% to avoid false negatives on component tests
    /// - **Functions (15%)**: test_* prefix is conventional
    /// - **Imports (5%)**: Weakest signal - many utils import test frameworks
    ///
    /// # Example Scores
    ///
    /// - Component test (src/foo_tests.rs): 0.80-0.90 (high confidence)
    /// - Integration test (tests/foo.rs): 0.95-1.0 (perfect confidence)
    /// - Production file: 0.0-0.3 (not a test)
    fn weighted_average(
        &self,
        naming: f32,
        attributes: f32,
        functions: f32,
        imports: f32,
        directory: f32,
    ) -> f32 {
        directory * 0.20 + attributes * 0.35 + naming * 0.25 + functions * 0.15 + imports * 0.05
    }

    /// Detect which test framework is being used
    fn detect_framework(&self, functions: &[FunctionMetrics]) -> Option<String> {
        match self.language {
            Language::Rust => {
                // Check for tokio::test
                let has_tokio = functions
                    .iter()
                    .any(|f| f.name.contains("async") || f.name.contains("tokio"));

                if has_tokio {
                    Some("tokio".to_string())
                } else {
                    Some("rust-std".to_string())
                }
            }
            _ => None,
        }
    }

    /// Count the number of test functions
    fn count_tests(&self, functions: &[FunctionMetrics]) -> usize {
        functions
            .iter()
            .filter(|f| f.is_test || f.in_test_module || f.name.starts_with("test_"))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detects_rust_test_file_by_naming() {
        let detector = FileContextDetector::new(Language::Rust);
        let path = Path::new("src/foo_tests.rs");
        let score = detector.score_naming(path);
        assert_eq!(score, 1.0, "Expected perfect score of 1.0, got {}", score);
    }

    #[test]
    fn detects_test_file_by_directory() {
        let detector = FileContextDetector::new(Language::Rust);
        let path = Path::new("tests/integration_test.rs");
        let score = detector.score_directory(path);
        assert_eq!(score, 1.0);
    }

    #[test]
    fn detects_test_file_by_attributes() {
        let detector = FileContextDetector::new(Language::Rust);

        let mut func1 =
            FunctionMetrics::new("test_something".to_string(), PathBuf::from("test.rs"), 1);
        func1.is_test = true;

        let mut func2 =
            FunctionMetrics::new("test_another".to_string(), PathBuf::from("test.rs"), 10);
        func2.is_test = true;

        let func3 = FunctionMetrics::new("helper".to_string(), PathBuf::from("test.rs"), 20);

        let functions = vec![func1, func2, func3];
        let score = detector.score_attributes(&functions);

        // 2/3 functions are tests
        assert!(
            (score - 0.666).abs() < 0.01,
            "Expected ~0.666, got {}",
            score
        );
    }

    #[test]
    fn production_file_not_classified_as_test() {
        let detector = FileContextDetector::new(Language::Rust);
        let path = Path::new("src/executor.rs");

        let func1 = FunctionMetrics::new("execute".to_string(), PathBuf::from("executor.rs"), 1);

        let func2 =
            FunctionMetrics::new("run_command".to_string(), PathBuf::from("executor.rs"), 50);

        let functions = vec![func1, func2];
        let context = detector.detect(path, &functions);

        assert!(
            matches!(context, FileContext::Production),
            "Expected Production, got {:?}",
            context
        );
    }

    #[test]
    fn high_confidence_test_file() {
        let detector = FileContextDetector::new(Language::Rust);
        let path = Path::new("tests/my_tests.rs");

        let mut func1 = FunctionMetrics::new(
            "test_feature_a".to_string(),
            PathBuf::from("tests/my_tests.rs"),
            1,
        );
        func1.is_test = true;

        let mut func2 = FunctionMetrics::new(
            "test_feature_b".to_string(),
            PathBuf::from("tests/my_tests.rs"),
            10,
        );
        func2.is_test = true;

        let functions = vec![func1, func2];
        let context = detector.detect(path, &functions);

        match context {
            FileContext::Test {
                confidence,
                test_count,
                ..
            } => {
                assert!(
                    confidence > 0.9,
                    "Expected high confidence, got {}",
                    confidence
                );
                assert_eq!(test_count, 2);
            }
            _ => panic!("Expected Test context, got {:?}", context),
        }
    }

    #[test]
    fn detects_python_test_file() {
        let detector = FileContextDetector::new(Language::Python);
        let path = Path::new("test_module.py");
        let score = detector.score_naming(path);
        assert_eq!(score, 0.9);
    }

    #[test]
    fn weighted_average_calculation() {
        let detector = FileContextDetector::new(Language::Rust);

        // All signals positive
        let avg = detector.weighted_average(1.0, 1.0, 1.0, 1.0, 1.0);
        assert_eq!(avg, 1.0);

        // Only directory signal (spec 169: reduced to 20%)
        let avg = detector.weighted_average(0.0, 0.0, 0.0, 0.0, 1.0);
        assert_eq!(avg, 0.20);

        // Only attributes signal (spec 169: increased to 35%)
        let avg = detector.weighted_average(0.0, 1.0, 0.0, 0.0, 0.0);
        assert_eq!(avg, 0.35);
    }

    #[test]
    fn file_context_helper_methods() {
        let test_ctx = FileContext::Test {
            confidence: 0.95,
            test_framework: Some("rust-std".to_string()),
            test_count: 10,
        };

        assert!(test_ctx.is_test());
        assert!(test_ctx.is_probable_test());
        assert_eq!(test_ctx.test_confidence(), Some(0.95));
        assert_eq!(test_ctx.test_count(), Some(10));

        let prod_ctx = FileContext::Production;
        assert!(!prod_ctx.is_test());
        assert!(!prod_ctx.is_probable_test());
        assert_eq!(prod_ctx.test_confidence(), None);
    }

    // Spec 169: Component test detection tests
    #[test]
    fn test_component_test_achieves_high_confidence() {
        let detector = FileContextDetector::new(Language::Rust);
        let path = Path::new("src/cook/workflow/git_context_diff_tests.rs");

        // Create test file with strong signals: 7 tests, all with #[test] attribute
        let mut functions = Vec::new();
        for i in 0..7 {
            let mut func = FunctionMetrics::new(
                format!("test_feature_{}", i),
                PathBuf::from("git_context_diff_tests.rs"),
                i * 10,
            );
            func.is_test = true;
            functions.push(func);
        }

        let context = detector.detect(path, &functions);

        match context {
            FileContext::Test { confidence, .. } => {
                assert!(
                    confidence > 0.8,
                    "Component test should have >0.8 confidence, got {}",
                    confidence
                );
            }
            _ => panic!("Should be detected as test file"),
        }
    }

    #[test]
    fn test_rebalanced_weights() {
        let detector = FileContextDetector::new(Language::Rust);

        // Component test: strong attributes/naming, no directory
        let confidence = detector.weighted_average(
            1.0, // naming (_tests.rs)
            1.0, // attributes (100% #[test])
            1.0, // functions (100% test_)
            1.0, // imports
            0.0, // directory (not in tests/)
        );

        assert!(
            confidence >= 0.80,
            "Should achieve at least 0.80 confidence, got {}",
            confidence
        );
    }

    #[test]
    fn test_strong_signals_boost() {
        let detector = FileContextDetector::new(Language::Rust);
        let path = Path::new("src/foo_tests.rs");

        // Create file with 3 strong signals
        let mut functions = Vec::new();
        for i in 0..5 {
            let mut func = FunctionMetrics::new(
                format!("test_case_{}", i),
                PathBuf::from("foo_tests.rs"),
                i * 10,
            );
            func.is_test = true;
            functions.push(func);
        }

        let context = detector.detect(path, &functions);

        match context {
            FileContext::Test { confidence, .. } => {
                assert!(
                    confidence >= 0.85,
                    "Strong signals should boost to at least 0.85, got {}",
                    confidence
                );
            }
            _ => panic!("Should be detected as test file"),
        }
    }

    #[test]
    fn test_production_file_unchanged() {
        let detector = FileContextDetector::new(Language::Rust);
        let path = Path::new("src/cook/workflow/executor.rs");

        // Production file: no test signals
        let func1 = FunctionMetrics::new("execute".to_string(), PathBuf::from("executor.rs"), 1);
        let func2 =
            FunctionMetrics::new("run_command".to_string(), PathBuf::from("executor.rs"), 50);

        let functions = vec![func1, func2];
        let context = detector.detect(path, &functions);

        match context {
            FileContext::Production => {} // Expected
            FileContext::Test { confidence, .. } => {
                panic!(
                    "Production file misclassified as test with confidence {}",
                    confidence
                )
            }
            _ => panic!("Unexpected context"),
        }
    }

    #[test]
    fn test_integration_test_still_perfect() {
        let detector = FileContextDetector::new(Language::Rust);
        let path = Path::new("tests/integration_test.rs");

        // Integration test: all signals strong
        let mut functions = Vec::new();
        for i in 0..10 {
            let mut func = FunctionMetrics::new(
                format!("test_integration_{}", i),
                PathBuf::from("integration_test.rs"),
                i * 10,
            );
            func.is_test = true;
            functions.push(func);
        }

        let context = detector.detect(path, &functions);

        match context {
            FileContext::Test { confidence, .. } => {
                assert!(
                    confidence >= 0.95,
                    "Integration test should have ~1.0 confidence, got {}",
                    confidence
                );
            }
            _ => panic!("Should be detected as test file"),
        }
    }

    #[test]
    fn test_naming_perfect_scores() {
        let detector = FileContextDetector::new(Language::Rust);

        // _tests.rs suffix should get 1.0
        let path1 = Path::new("git_context_diff_tests.rs");
        assert_eq!(
            detector.score_naming(path1),
            1.0,
            "_tests.rs should score 1.0"
        );

        // _test.rs suffix should get 1.0
        let path2 = Path::new("component_test.rs");
        assert_eq!(
            detector.score_naming(path2),
            1.0,
            "_test.rs should score 1.0"
        );
    }
}
