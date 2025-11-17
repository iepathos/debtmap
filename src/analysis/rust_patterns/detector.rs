use crate::analysis::multi_signal_aggregation::ResponsibilityCategory;
use crate::analysis::rust_patterns::{
    async_detector::{AsyncPattern, RustAsyncDetector},
    builder_detector::{BuilderPattern, RustBuilderDetector},
    context::RustFunctionContext,
    error_detector::{ErrorPattern, RustErrorDetector},
    trait_detector::{RustTraitDetector, TraitImplClassification},
};
use serde::{Deserialize, Serialize};

pub struct RustPatternDetector {
    trait_detector: RustTraitDetector,
    async_detector: RustAsyncDetector,
    error_detector: RustErrorDetector,
    builder_detector: RustBuilderDetector,
}

impl RustPatternDetector {
    pub fn new() -> Self {
        Self {
            trait_detector: RustTraitDetector::new(),
            async_detector: RustAsyncDetector::new(),
            error_detector: RustErrorDetector::new(),
            builder_detector: RustBuilderDetector::new(),
        }
    }

    /// Detect all Rust-specific patterns for a function
    pub fn detect_all_patterns(
        &self,
        context: &RustFunctionContext,
        validation_signals: Option<crate::priority::complexity_patterns::ValidationSignals>,
        state_machine_signals: Option<crate::priority::complexity_patterns::StateMachineSignals>,
        coordinator_signals: Option<crate::priority::complexity_patterns::CoordinatorSignals>,
    ) -> RustPatternResult {
        RustPatternResult {
            trait_impl: self.trait_detector.detect_trait_impl(context),
            async_patterns: self.async_detector.detect_async_patterns(context),
            error_patterns: self.error_detector.detect_error_patterns(context),
            builder_patterns: self.builder_detector.detect_builder_patterns(context),
            validation_signals,
            state_machine_signals,
            coordinator_signals,
        }
    }

    /// Classify function based on detected patterns
    /// Priority order: Trait impls > Async > Builder > Error handling
    pub fn classify_function(
        &self,
        context: &RustFunctionContext,
    ) -> Option<RustSpecificClassification> {
        // 1. Trait implementations (highest confidence)
        if let Some(trait_impl) = self.trait_detector.detect_trait_impl(context) {
            return Some(RustSpecificClassification {
                category: trait_impl.category,
                confidence: trait_impl.confidence,
                evidence: trait_impl.evidence.clone(),
                rust_pattern: RustPattern::TraitImplementation(trait_impl),
            });
        }

        // 2. Async/concurrency patterns
        let async_patterns = self.async_detector.detect_async_patterns(context);
        if let Some(category) = self
            .async_detector
            .classify_from_async_patterns(&async_patterns)
        {
            return Some(RustSpecificClassification {
                category,
                confidence: 0.85,
                evidence: format!(
                    "Async patterns: {:?}",
                    async_patterns
                        .iter()
                        .map(|p| &p.pattern_type)
                        .collect::<Vec<_>>()
                ),
                rust_pattern: RustPattern::AsyncConcurrency(async_patterns),
            });
        }

        // 3. Builder patterns
        let builder_patterns = self.builder_detector.detect_builder_patterns(context);
        if let Some(category) = self
            .builder_detector
            .classify_from_builder_patterns(&builder_patterns)
        {
            return Some(RustSpecificClassification {
                category,
                confidence: 0.80,
                evidence: format!(
                    "Builder patterns: {:?}",
                    builder_patterns
                        .iter()
                        .map(|p| &p.pattern_type)
                        .collect::<Vec<_>>()
                ),
                rust_pattern: RustPattern::BuilderPattern(builder_patterns),
            });
        }

        // 4. Error handling patterns
        let error_patterns = self.error_detector.detect_error_patterns(context);
        if let Some(category) = self
            .error_detector
            .classify_from_error_patterns(&error_patterns)
        {
            return Some(RustSpecificClassification {
                category,
                confidence: 0.75,
                evidence: format!(
                    "Error handling: {:?}",
                    error_patterns
                        .iter()
                        .map(|p| &p.pattern_type)
                        .collect::<Vec<_>>()
                ),
                rust_pattern: RustPattern::ErrorHandling(error_patterns),
            });
        }

        None
    }
}

impl Default for RustPatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RustPatternResult {
    pub trait_impl: Option<TraitImplClassification>,
    pub async_patterns: Vec<AsyncPattern>,
    pub error_patterns: Vec<ErrorPattern>,
    pub builder_patterns: Vec<BuilderPattern>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_signals: Option<crate::priority::complexity_patterns::ValidationSignals>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_machine_signals: Option<crate::priority::complexity_patterns::StateMachineSignals>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinator_signals: Option<crate::priority::complexity_patterns::CoordinatorSignals>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustSpecificClassification {
    pub category: ResponsibilityCategory,
    pub confidence: f64,
    pub evidence: String,
    pub rust_pattern: RustPattern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RustPattern {
    TraitImplementation(TraitImplClassification),
    AsyncConcurrency(Vec<AsyncPattern>),
    BuilderPattern(Vec<BuilderPattern>),
    ErrorHandling(Vec<ErrorPattern>),
}
