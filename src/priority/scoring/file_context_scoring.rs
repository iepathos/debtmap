//! File context-aware scoring adjustments
//!
//! This module implements score adjustments based on file context (test vs production).
//! Test files receive reduced scores to avoid false positives in technical debt detection.
//!
//! Spec 166: Test File Detection and Context-Aware Scoring

use crate::analysis::FileContext;

/// Apply context-aware score adjustments
///
/// Adjusts scores based on file context:
/// - Test files (confidence > 0.8): Reduce score by 80%
/// - Probable test files (confidence 0.5-0.8): Reduce score by 40%
/// - Production files: No adjustment
/// - Generated files: Reduce score by 90%
pub fn apply_context_adjustments(base_score: f64, context: &FileContext) -> f64 {
    match context {
        FileContext::Test { confidence, .. } => {
            if *confidence > 0.8 {
                // High confidence test file - reduce score significantly
                base_score * 0.2
            } else if *confidence > 0.5 {
                // Probable test file - reduce score moderately
                base_score * 0.6
            } else {
                // Low confidence - no adjustment
                base_score
            }
        }
        FileContext::Generated { .. } => {
            // Generated files are very low priority
            base_score * 0.1
        }
        FileContext::Production | FileContext::Configuration | FileContext::Documentation => {
            // No adjustment for production code
            base_score
        }
    }
}

/// Calculate the reduction factor for a given file context
///
/// Returns a value between 0.0 and 1.0 representing how much the score should be multiplied by.
/// - 1.0 = no reduction (production code)
/// - 0.2 = 80% reduction (high confidence test file)
/// - 0.1 = 90% reduction (generated file)
pub fn context_reduction_factor(context: &FileContext) -> f64 {
    match context {
        FileContext::Test { confidence, .. } => {
            if *confidence > 0.8 {
                0.2 // 80% reduction
            } else if *confidence > 0.5 {
                0.6 // 40% reduction
            } else {
                1.0 // No reduction
            }
        }
        FileContext::Generated { .. } => 0.1, // 90% reduction
        FileContext::Production | FileContext::Configuration | FileContext::Documentation => 1.0,
    }
}

/// Check if a file context indicates a test file
pub fn is_test_context(context: &FileContext) -> bool {
    matches!(context, FileContext::Test { confidence, .. } if *confidence > 0.5)
}

/// Get a human-readable label for the file context
pub fn context_label(context: &FileContext) -> &'static str {
    match context {
        FileContext::Production => "PRODUCTION",
        FileContext::Test { confidence, .. } => {
            if *confidence > 0.8 {
                "TEST FILE"
            } else {
                "PROBABLE TEST"
            }
        }
        FileContext::Generated { .. } => "GENERATED",
        FileContext::Configuration => "CONFIG",
        FileContext::Documentation => "DOCS",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_high_confidence_test_reduction() {
        let context = FileContext::Test {
            confidence: 0.95,
            test_framework: Some("rust-std".to_string()),
            test_count: 10,
        };

        let base_score = 100.0;
        let adjusted = apply_context_adjustments(base_score, &context);

        // Should be reduced by 80%
        assert_eq!(adjusted, 20.0);
    }

    #[test]
    fn test_probable_test_reduction() {
        let context = FileContext::Test {
            confidence: 0.65,
            test_framework: None,
            test_count: 5,
        };

        let base_score = 100.0;
        let adjusted = apply_context_adjustments(base_score, &context);

        // Should be reduced by 40%
        assert_eq!(adjusted, 60.0);
    }

    #[test]
    fn test_production_no_reduction() {
        let context = FileContext::Production;
        let base_score = 100.0;
        let adjusted = apply_context_adjustments(base_score, &context);

        // Should not be reduced
        assert_eq!(adjusted, 100.0);
    }

    #[test]
    fn test_generated_file_reduction() {
        let context = FileContext::Generated {
            generator: "protobuf".to_string(),
        };

        let base_score = 100.0;
        let adjusted = apply_context_adjustments(base_score, &context);

        // Should be reduced by 90%
        assert_eq!(adjusted, 10.0);
    }

    #[test]
    fn test_context_reduction_factors() {
        let high_conf_test = FileContext::Test {
            confidence: 0.95,
            test_framework: None,
            test_count: 5,
        };
        assert_eq!(context_reduction_factor(&high_conf_test), 0.2);

        let probable_test = FileContext::Test {
            confidence: 0.6,
            test_framework: None,
            test_count: 3,
        };
        assert_eq!(context_reduction_factor(&probable_test), 0.6);

        let production = FileContext::Production;
        assert_eq!(context_reduction_factor(&production), 1.0);

        let generated = FileContext::Generated {
            generator: "swagger".to_string(),
        };
        assert_eq!(context_reduction_factor(&generated), 0.1);
    }

    #[test]
    fn test_is_test_context() {
        let high_conf = FileContext::Test {
            confidence: 0.9,
            test_framework: None,
            test_count: 5,
        };
        assert!(is_test_context(&high_conf));

        let low_conf = FileContext::Test {
            confidence: 0.3,
            test_framework: None,
            test_count: 1,
        };
        assert!(!is_test_context(&low_conf));

        let production = FileContext::Production;
        assert!(!is_test_context(&production));
    }

    #[test]
    fn test_context_labels() {
        let test_ctx = FileContext::Test {
            confidence: 0.95,
            test_framework: None,
            test_count: 5,
        };
        assert_eq!(context_label(&test_ctx), "TEST FILE");

        let probable_test = FileContext::Test {
            confidence: 0.6,
            test_framework: None,
            test_count: 2,
        };
        assert_eq!(context_label(&probable_test), "PROBABLE TEST");

        let production = FileContext::Production;
        assert_eq!(context_label(&production), "PRODUCTION");

        let generated = FileContext::Generated {
            generator: "protobuf".to_string(),
        };
        assert_eq!(context_label(&generated), "GENERATED");
    }
}
