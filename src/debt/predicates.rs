//! Composable predicate system for debt detection rules.
//!
//! This module provides declarative predicates for detecting technical debt patterns.
//! Predicates are built using stillwater's predicate combinators, enabling:
//!
//! - **Composition**: Combine predicates with `and()`, `or()`, `not()`
//! - **Testability**: Each predicate is independently testable
//! - **Configurability**: Thresholds are parameterized via config
//! - **Self-documentation**: Each predicate describes what it detects
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::debt::predicates::*;
//!
//! // Create predicates with thresholds
//! let high_complexity = HighCyclomatic::new(21);
//! let low_coverage = LowCoverage::new(0.5);
//!
//! // Compose for high-risk detection
//! let high_risk = high_complexity.and(low_coverage);
//!
//! // Check against function metrics
//! if high_risk.check(&metrics) {
//!     println!("High risk function detected!");
//! }
//! ```
//!
//! # Available Predicates
//!
//! ## Complexity Predicates
//! - [`HighCyclomatic`]: Cyclomatic complexity exceeds threshold
//! - [`HighCognitive`]: Cognitive complexity exceeds threshold
//! - [`CriticalComplexity`]: Both cyclomatic AND cognitive exceed thresholds
//!
//! ## Structure Predicates
//! - [`DeepNesting`]: Nesting depth exceeds threshold
//! - [`LongMethod`]: Line count exceeds threshold
//! - [`TooManyParameters`]: Parameter count exceeds threshold (placeholder)
//!
//! ## Risk Predicates (Composed)
//! - [`HighRisk`]: High complexity AND low coverage
//! - [`CriticalRisk`]: Critical complexity AND no coverage
//! - [`ModerateRisk`]: High complexity OR low coverage
//!
//! ## Coverage Predicates
//! - [`LowCoverage`]: Coverage below threshold
//! - [`NoCoverage`]: Coverage is zero or absent
//! - [`PartialCoverage`]: Coverage between 0% and threshold

use crate::core::FunctionMetrics;
use crate::effects::validation::ValidationRuleSet;
use stillwater::predicate::Predicate;

// =============================================================================
// Complexity Predicates
// =============================================================================

/// Predicate for high cyclomatic complexity.
///
/// Cyclomatic complexity measures the number of linearly independent paths
/// through a function. High values indicate functions that are difficult to test.
#[derive(Debug, Clone, Copy)]
pub struct HighCyclomatic {
    threshold: u32,
}

impl HighCyclomatic {
    /// Create a new predicate with the given threshold.
    pub fn new(threshold: u32) -> Self {
        Self { threshold }
    }

    /// Create from a ValidationRuleSet using the warning threshold.
    pub fn from_rules(rules: &ValidationRuleSet) -> Self {
        Self::new(rules.complexity_warning)
    }

    /// Get the threshold value.
    pub fn threshold(&self) -> u32 {
        self.threshold
    }

    /// Get a description of what this predicate checks.
    pub fn description(&self) -> String {
        format!("cyclomatic complexity > {}", self.threshold)
    }
}

impl Predicate<FunctionMetrics> for HighCyclomatic {
    fn check(&self, metrics: &FunctionMetrics) -> bool {
        metrics.cyclomatic > self.threshold
    }
}

impl Predicate<u32> for HighCyclomatic {
    fn check(&self, value: &u32) -> bool {
        *value > self.threshold
    }
}

/// Predicate for high cognitive complexity.
///
/// Cognitive complexity measures how difficult code is to understand.
/// High values suggest the code should be refactored for readability.
#[derive(Debug, Clone, Copy)]
pub struct HighCognitive {
    threshold: u32,
}

impl HighCognitive {
    /// Create a new predicate with the given threshold.
    pub fn new(threshold: u32) -> Self {
        Self { threshold }
    }

    /// Create from a ValidationRuleSet using the warning threshold.
    pub fn from_rules(rules: &ValidationRuleSet) -> Self {
        Self::new(rules.complexity_warning)
    }

    /// Get the threshold value.
    pub fn threshold(&self) -> u32 {
        self.threshold
    }

    /// Get a description of what this predicate checks.
    pub fn description(&self) -> String {
        format!("cognitive complexity > {}", self.threshold)
    }
}

impl Predicate<FunctionMetrics> for HighCognitive {
    fn check(&self, metrics: &FunctionMetrics) -> bool {
        metrics.cognitive > self.threshold
    }
}

impl Predicate<u32> for HighCognitive {
    fn check(&self, value: &u32) -> bool {
        *value > self.threshold
    }
}

/// Predicate for critical complexity (both cyclomatic AND cognitive exceed thresholds).
///
/// This represents functions that are both hard to test AND hard to understand,
/// indicating the highest priority for refactoring.
#[derive(Debug, Clone, Copy)]
pub struct CriticalComplexity {
    cyclomatic_threshold: u32,
    cognitive_threshold: u32,
}

impl CriticalComplexity {
    /// Create a new predicate with the given thresholds.
    pub fn new(cyclomatic_threshold: u32, cognitive_threshold: u32) -> Self {
        Self {
            cyclomatic_threshold,
            cognitive_threshold,
        }
    }

    /// Create from a ValidationRuleSet using the critical threshold.
    pub fn from_rules(rules: &ValidationRuleSet) -> Self {
        Self::new(rules.complexity_critical, rules.complexity_critical)
    }

    /// Get a description of what this predicate checks.
    pub fn description(&self) -> String {
        format!(
            "cyclomatic > {} AND cognitive > {}",
            self.cyclomatic_threshold, self.cognitive_threshold
        )
    }
}

impl Predicate<FunctionMetrics> for CriticalComplexity {
    fn check(&self, metrics: &FunctionMetrics) -> bool {
        metrics.cyclomatic > self.cyclomatic_threshold
            && metrics.cognitive > self.cognitive_threshold
    }
}

// =============================================================================
// Structure Predicates
// =============================================================================

/// Predicate for deep nesting depth.
///
/// Deeply nested code is harder to understand and maintain.
/// High nesting often indicates opportunities for extraction or early returns.
#[derive(Debug, Clone, Copy)]
pub struct DeepNesting {
    threshold: u32,
}

impl DeepNesting {
    /// Create a new predicate with the given threshold.
    pub fn new(threshold: u32) -> Self {
        Self { threshold }
    }

    /// Create from a ValidationRuleSet.
    pub fn from_rules(rules: &ValidationRuleSet) -> Self {
        Self::new(rules.max_nesting_depth)
    }

    /// Get the threshold value.
    pub fn threshold(&self) -> u32 {
        self.threshold
    }

    /// Get a description of what this predicate checks.
    pub fn description(&self) -> String {
        format!("nesting depth > {}", self.threshold)
    }
}

impl Predicate<FunctionMetrics> for DeepNesting {
    fn check(&self, metrics: &FunctionMetrics) -> bool {
        metrics.nesting > self.threshold
    }
}

impl Predicate<u32> for DeepNesting {
    fn check(&self, value: &u32) -> bool {
        *value > self.threshold
    }
}

/// Predicate for long methods.
///
/// Long functions are harder to understand and test.
/// They often indicate multiple responsibilities that should be split.
#[derive(Debug, Clone, Copy)]
pub struct LongMethod {
    threshold: usize,
}

impl LongMethod {
    /// Create a new predicate with the given threshold.
    pub fn new(threshold: usize) -> Self {
        Self { threshold }
    }

    /// Create from a ValidationRuleSet.
    pub fn from_rules(rules: &ValidationRuleSet) -> Self {
        Self::new(rules.max_function_length)
    }

    /// Get the threshold value.
    pub fn threshold(&self) -> usize {
        self.threshold
    }

    /// Get a description of what this predicate checks.
    pub fn description(&self) -> String {
        format!("line count > {}", self.threshold)
    }
}

impl Predicate<FunctionMetrics> for LongMethod {
    fn check(&self, metrics: &FunctionMetrics) -> bool {
        metrics.length > self.threshold
    }
}

impl Predicate<usize> for LongMethod {
    fn check(&self, value: &usize) -> bool {
        *value > self.threshold
    }
}

/// Predicate for too many parameters.
///
/// Functions with many parameters are harder to call correctly and test.
/// Note: This is a placeholder as FunctionMetrics doesn't currently track parameter count.
#[derive(Debug, Clone, Copy)]
pub struct TooManyParameters {
    threshold: usize,
}

impl TooManyParameters {
    /// Create a new predicate with the given threshold.
    pub fn new(threshold: usize) -> Self {
        Self { threshold }
    }

    /// Get the threshold value.
    pub fn threshold(&self) -> usize {
        self.threshold
    }

    /// Get a description of what this predicate checks.
    pub fn description(&self) -> String {
        format!("parameter count > {}", self.threshold)
    }
}

impl Predicate<usize> for TooManyParameters {
    fn check(&self, value: &usize) -> bool {
        *value > self.threshold
    }
}

// =============================================================================
// Coverage Predicates
// =============================================================================

/// Predicate for low test coverage.
///
/// Functions with low coverage are riskier to modify because bugs may not be caught.
#[derive(Debug, Clone, Copy)]
pub struct LowCoverage {
    /// Threshold as a fraction (0.0 to 1.0)
    threshold: f64,
}

impl LowCoverage {
    /// Create a new predicate with the given threshold (0.0 to 1.0).
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold: threshold.clamp(0.0, 1.0),
        }
    }

    /// Create with a percentage threshold (0 to 100).
    pub fn from_percentage(percentage: u32) -> Self {
        Self::new(f64::from(percentage) / 100.0)
    }

    /// Get the threshold value.
    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Get a description of what this predicate checks.
    pub fn description(&self) -> String {
        format!("coverage < {}%", (self.threshold * 100.0) as u32)
    }
}

impl Predicate<f64> for LowCoverage {
    fn check(&self, coverage: &f64) -> bool {
        *coverage < self.threshold
    }
}

impl Predicate<Option<f64>> for LowCoverage {
    fn check(&self, coverage: &Option<f64>) -> bool {
        coverage.is_none_or(|c| c < self.threshold)
    }
}

/// Predicate for zero (or absent) test coverage.
///
/// Functions with no coverage are highest risk for untested changes.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoCoverage;

impl NoCoverage {
    /// Create a new NoCoverage predicate.
    pub fn new() -> Self {
        Self
    }

    /// Get a description of what this predicate checks.
    pub fn description(&self) -> String {
        "coverage = 0% or absent".to_string()
    }
}

impl Predicate<f64> for NoCoverage {
    fn check(&self, coverage: &f64) -> bool {
        *coverage == 0.0
    }
}

impl Predicate<Option<f64>> for NoCoverage {
    fn check(&self, coverage: &Option<f64>) -> bool {
        coverage.is_none_or(|c| c == 0.0)
    }
}

/// Predicate for partial coverage (between 0% and threshold).
///
/// These functions have some tests but not enough to be confident.
#[derive(Debug, Clone, Copy)]
pub struct PartialCoverage {
    threshold: f64,
}

impl PartialCoverage {
    /// Create a new predicate with the given threshold (0.0 to 1.0).
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold: threshold.clamp(0.0, 1.0),
        }
    }

    /// Get the threshold value.
    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Get a description of what this predicate checks.
    pub fn description(&self) -> String {
        format!("0% < coverage < {}%", (self.threshold * 100.0) as u32)
    }
}

impl Predicate<f64> for PartialCoverage {
    fn check(&self, coverage: &f64) -> bool {
        *coverage > 0.0 && *coverage < self.threshold
    }
}

impl Predicate<Option<f64>> for PartialCoverage {
    fn check(&self, coverage: &Option<f64>) -> bool {
        coverage.is_some_and(|c| c > 0.0 && c < self.threshold)
    }
}

// =============================================================================
// Composed Predicates (Logical Combinations)
// =============================================================================

/// High-risk predicate: High complexity AND low coverage.
///
/// Functions that are both complex and poorly tested represent the highest
/// maintenance risk.
#[derive(Debug, Clone)]
pub struct HighRisk {
    high_complexity: HighCyclomatic,
    low_coverage: LowCoverage,
}

impl HighRisk {
    /// Create a new high-risk predicate.
    pub fn new(complexity_threshold: u32, coverage_threshold: f64) -> Self {
        Self {
            high_complexity: HighCyclomatic::new(complexity_threshold),
            low_coverage: LowCoverage::new(coverage_threshold),
        }
    }

    /// Create from a ValidationRuleSet.
    pub fn from_rules(rules: &ValidationRuleSet) -> Self {
        Self::new(rules.complexity_warning, 0.5) // 50% default coverage threshold
    }

    /// Get a description of what this predicate checks.
    pub fn description(&self) -> String {
        format!(
            "{} AND {}",
            self.high_complexity.description(),
            self.low_coverage.description()
        )
    }
}

/// Metrics with coverage for risk evaluation.
pub struct FunctionWithCoverage<'a> {
    pub metrics: &'a FunctionMetrics,
    pub coverage: Option<f64>,
}

impl Predicate<FunctionWithCoverage<'_>> for HighRisk {
    fn check(&self, value: &FunctionWithCoverage<'_>) -> bool {
        self.high_complexity.check(value.metrics) && self.low_coverage.check(&value.coverage)
    }
}

/// Critical-risk predicate: Critical complexity AND no coverage.
///
/// The most dangerous functions: extremely complex with zero tests.
#[derive(Debug, Clone)]
pub struct CriticalRisk {
    critical_complexity: CriticalComplexity,
    no_coverage: NoCoverage,
}

impl CriticalRisk {
    /// Create a new critical-risk predicate.
    pub fn new(cyclomatic_threshold: u32, cognitive_threshold: u32) -> Self {
        Self {
            critical_complexity: CriticalComplexity::new(cyclomatic_threshold, cognitive_threshold),
            no_coverage: NoCoverage::new(),
        }
    }

    /// Create from a ValidationRuleSet.
    pub fn from_rules(rules: &ValidationRuleSet) -> Self {
        Self::new(rules.complexity_critical, rules.complexity_critical)
    }

    /// Get a description of what this predicate checks.
    pub fn description(&self) -> String {
        format!(
            "{} AND {}",
            self.critical_complexity.description(),
            self.no_coverage.description()
        )
    }
}

impl Predicate<FunctionWithCoverage<'_>> for CriticalRisk {
    fn check(&self, value: &FunctionWithCoverage<'_>) -> bool {
        self.critical_complexity.check(value.metrics) && self.no_coverage.check(&value.coverage)
    }
}

/// Moderate-risk predicate: High complexity OR low coverage.
///
/// Functions that have at least one risk factor present.
#[derive(Debug, Clone)]
pub struct ModerateRisk {
    high_complexity: HighCyclomatic,
    low_coverage: LowCoverage,
}

impl ModerateRisk {
    /// Create a new moderate-risk predicate.
    pub fn new(complexity_threshold: u32, coverage_threshold: f64) -> Self {
        Self {
            high_complexity: HighCyclomatic::new(complexity_threshold),
            low_coverage: LowCoverage::new(coverage_threshold),
        }
    }

    /// Create from a ValidationRuleSet.
    pub fn from_rules(rules: &ValidationRuleSet) -> Self {
        Self::new(rules.complexity_warning, 0.5)
    }

    /// Get a description of what this predicate checks.
    pub fn description(&self) -> String {
        format!(
            "{} OR {}",
            self.high_complexity.description(),
            self.low_coverage.description()
        )
    }
}

impl Predicate<FunctionWithCoverage<'_>> for ModerateRisk {
    fn check(&self, value: &FunctionWithCoverage<'_>) -> bool {
        self.high_complexity.check(value.metrics) || self.low_coverage.check(&value.coverage)
    }
}

// =============================================================================
// Predicate Factory
// =============================================================================

/// Factory for creating configured debt detection predicates.
///
/// This struct provides convenient access to all standard predicates,
/// configured from a single validation rule set.
#[derive(Debug, Clone)]
pub struct DebtPredicates {
    /// High cyclomatic complexity predicate.
    pub high_cyclomatic: HighCyclomatic,
    /// High cognitive complexity predicate.
    pub high_cognitive: HighCognitive,
    /// Critical complexity predicate (both thresholds exceeded).
    pub critical_complexity: CriticalComplexity,
    /// Deep nesting predicate.
    pub deep_nesting: DeepNesting,
    /// Long method predicate.
    pub long_method: LongMethod,
    /// Low coverage predicate.
    pub low_coverage: LowCoverage,
    /// No coverage predicate.
    pub no_coverage: NoCoverage,
}

impl DebtPredicates {
    /// Create predicates from a ValidationRuleSet.
    pub fn from_rules(rules: &ValidationRuleSet) -> Self {
        Self {
            high_cyclomatic: HighCyclomatic::from_rules(rules),
            high_cognitive: HighCognitive::from_rules(rules),
            critical_complexity: CriticalComplexity::from_rules(rules),
            deep_nesting: DeepNesting::from_rules(rules),
            long_method: LongMethod::from_rules(rules),
            low_coverage: LowCoverage::new(0.5), // 50% default
            no_coverage: NoCoverage::new(),
        }
    }

    /// Create with custom coverage threshold.
    pub fn with_coverage_threshold(mut self, threshold: f64) -> Self {
        self.low_coverage = LowCoverage::new(threshold);
        self
    }

    /// Create a high-risk predicate from these settings.
    pub fn high_risk(&self) -> HighRisk {
        HighRisk {
            high_complexity: self.high_cyclomatic,
            low_coverage: self.low_coverage,
        }
    }

    /// Create a critical-risk predicate from these settings.
    pub fn critical_risk(&self) -> CriticalRisk {
        CriticalRisk {
            critical_complexity: self.critical_complexity,
            no_coverage: self.no_coverage,
        }
    }

    /// Create a moderate-risk predicate from these settings.
    pub fn moderate_risk(&self) -> ModerateRisk {
        ModerateRisk {
            high_complexity: self.high_cyclomatic,
            low_coverage: self.low_coverage,
        }
    }

    /// Check if a function has any complexity issues.
    pub fn has_complexity_issues(&self, metrics: &FunctionMetrics) -> bool {
        self.high_cyclomatic.check(metrics)
            || self.high_cognitive.check(metrics)
            || self.critical_complexity.check(metrics)
    }

    /// Check if a function has any structural issues.
    pub fn has_structural_issues(&self, metrics: &FunctionMetrics) -> bool {
        self.deep_nesting.check(metrics) || self.long_method.check(metrics)
    }

    /// Check if a function has any issues at all.
    pub fn has_any_issues(&self, metrics: &FunctionMetrics) -> bool {
        self.has_complexity_issues(metrics) || self.has_structural_issues(metrics)
    }
}

impl Default for DebtPredicates {
    fn default() -> Self {
        Self::from_rules(&ValidationRuleSet::default())
    }
}

// =============================================================================
// Predicate Result Types
// =============================================================================

/// Result of predicate evaluation with context.
#[derive(Debug, Clone)]
pub struct PredicateResult {
    /// Whether the predicate matched.
    pub matched: bool,
    /// Name of the predicate that was evaluated.
    pub predicate_name: String,
    /// Human-readable description of the predicate.
    pub description: String,
    /// Optional additional details about the match.
    pub details: Option<String>,
}

impl PredicateResult {
    /// Create a new matched result.
    pub fn matched(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            matched: true,
            predicate_name: name.into(),
            description: description.into(),
            details: None,
        }
    }

    /// Create a new unmatched result.
    pub fn not_matched(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            matched: false,
            predicate_name: name.into(),
            description: description.into(),
            details: None,
        }
    }

    /// Add details to this result.
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

/// Collection of predicate evaluation results for a function.
#[derive(Debug, Clone)]
pub struct DebtFindings {
    /// The function that was evaluated.
    pub function_name: String,
    /// All predicate results (both matched and not).
    pub results: Vec<PredicateResult>,
}

impl DebtFindings {
    /// Create new findings for a function.
    pub fn new(function_name: impl Into<String>) -> Self {
        Self {
            function_name: function_name.into(),
            results: Vec::new(),
        }
    }

    /// Add a predicate result.
    pub fn add_result(&mut self, result: PredicateResult) {
        self.results.push(result);
    }

    /// Get only the matched predicates.
    pub fn matched_predicates(&self) -> impl Iterator<Item = &PredicateResult> {
        self.results.iter().filter(|r| r.matched)
    }

    /// Check if any predicates matched.
    pub fn has_issues(&self) -> bool {
        self.results.iter().any(|r| r.matched)
    }

    /// Count of matched predicates.
    pub fn issue_count(&self) -> usize {
        self.results.iter().filter(|r| r.matched).count()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_metrics(
        name: &str,
        cyclomatic: u32,
        cognitive: u32,
        nesting: u32,
        length: usize,
    ) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic,
            cognitive,
            nesting,
            length,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        }
    }

    // =========================================================================
    // Complexity Predicate Tests
    // =========================================================================

    mod complexity_predicates {
        use super::*;

        #[test]
        fn high_cyclomatic_above_threshold() {
            let pred = HighCyclomatic::new(20);
            let metrics = create_test_metrics("complex_fn", 25, 10, 2, 30);
            assert!(pred.check(&metrics));
        }

        #[test]
        fn high_cyclomatic_at_threshold() {
            let pred = HighCyclomatic::new(20);
            let metrics = create_test_metrics("boundary_fn", 20, 10, 2, 30);
            assert!(!pred.check(&metrics)); // At threshold, not above
        }

        #[test]
        fn high_cyclomatic_below_threshold() {
            let pred = HighCyclomatic::new(20);
            let metrics = create_test_metrics("simple_fn", 10, 5, 1, 15);
            assert!(!pred.check(&metrics));
        }

        #[test]
        fn high_cyclomatic_direct_value() {
            let pred = HighCyclomatic::new(20);
            assert!(pred.check(&25_u32));
            assert!(!pred.check(&20_u32));
            assert!(!pred.check(&15_u32));
        }

        #[test]
        fn high_cognitive_above_threshold() {
            let pred = HighCognitive::new(15);
            let metrics = create_test_metrics("confusing_fn", 10, 20, 2, 30);
            assert!(pred.check(&metrics));
        }

        #[test]
        fn high_cognitive_below_threshold() {
            let pred = HighCognitive::new(15);
            let metrics = create_test_metrics("clear_fn", 10, 10, 2, 30);
            assert!(!pred.check(&metrics));
        }

        #[test]
        fn critical_complexity_both_exceeded() {
            let pred = CriticalComplexity::new(50, 40);
            let metrics = create_test_metrics("nightmare_fn", 60, 50, 5, 100);
            assert!(pred.check(&metrics));
        }

        #[test]
        fn critical_complexity_only_one_exceeded() {
            let pred = CriticalComplexity::new(50, 40);

            // Only cyclomatic exceeded
            let metrics1 = create_test_metrics("high_cyclomatic", 60, 30, 2, 50);
            assert!(!pred.check(&metrics1));

            // Only cognitive exceeded
            let metrics2 = create_test_metrics("high_cognitive", 30, 50, 2, 50);
            assert!(!pred.check(&metrics2));
        }

        #[test]
        fn critical_complexity_neither_exceeded() {
            let pred = CriticalComplexity::new(50, 40);
            let metrics = create_test_metrics("good_fn", 30, 25, 2, 30);
            assert!(!pred.check(&metrics));
        }
    }

    // =========================================================================
    // Structure Predicate Tests
    // =========================================================================

    mod structure_predicates {
        use super::*;

        #[test]
        fn deep_nesting_above_threshold() {
            let pred = DeepNesting::new(4);
            let metrics = create_test_metrics("nested_fn", 10, 8, 6, 30);
            assert!(pred.check(&metrics));
        }

        #[test]
        fn deep_nesting_at_threshold() {
            let pred = DeepNesting::new(4);
            let metrics = create_test_metrics("boundary_fn", 10, 8, 4, 30);
            assert!(!pred.check(&metrics));
        }

        #[test]
        fn deep_nesting_below_threshold() {
            let pred = DeepNesting::new(4);
            let metrics = create_test_metrics("flat_fn", 10, 8, 2, 30);
            assert!(!pred.check(&metrics));
        }

        #[test]
        fn long_method_above_threshold() {
            let pred = LongMethod::new(50);
            let metrics = create_test_metrics("huge_fn", 10, 8, 2, 100);
            assert!(pred.check(&metrics));
        }

        #[test]
        fn long_method_at_threshold() {
            let pred = LongMethod::new(50);
            let metrics = create_test_metrics("boundary_fn", 10, 8, 2, 50);
            assert!(!pred.check(&metrics));
        }

        #[test]
        fn long_method_below_threshold() {
            let pred = LongMethod::new(50);
            let metrics = create_test_metrics("short_fn", 10, 8, 2, 25);
            assert!(!pred.check(&metrics));
        }

        #[test]
        fn too_many_parameters_direct_value() {
            let pred = TooManyParameters::new(4);
            assert!(pred.check(&6_usize));
            assert!(!pred.check(&4_usize));
            assert!(!pred.check(&2_usize));
        }
    }

    // =========================================================================
    // Coverage Predicate Tests
    // =========================================================================

    mod coverage_predicates {
        use super::*;

        #[test]
        fn low_coverage_below_threshold() {
            let pred = LowCoverage::new(0.5);
            assert!(pred.check(&0.3_f64));
            assert!(pred.check(&Some(0.3_f64)));
        }

        #[test]
        fn low_coverage_at_threshold() {
            let pred = LowCoverage::new(0.5);
            assert!(!pred.check(&0.5_f64));
            assert!(!pred.check(&Some(0.5_f64)));
        }

        #[test]
        fn low_coverage_above_threshold() {
            let pred = LowCoverage::new(0.5);
            assert!(!pred.check(&0.7_f64));
            assert!(!pred.check(&Some(0.7_f64)));
        }

        #[test]
        fn low_coverage_missing() {
            let pred = LowCoverage::new(0.5);
            assert!(pred.check(&None::<f64>)); // Missing coverage treated as low
        }

        #[test]
        fn low_coverage_from_percentage() {
            let pred = LowCoverage::from_percentage(50);
            assert!(pred.check(&0.3_f64));
            assert!(!pred.check(&0.5_f64));
            assert!(!pred.check(&0.7_f64));
        }

        #[test]
        fn no_coverage_zero() {
            let pred = NoCoverage::new();
            assert!(pred.check(&0.0_f64));
            assert!(pred.check(&Some(0.0_f64)));
        }

        #[test]
        fn no_coverage_missing() {
            let pred = NoCoverage::new();
            assert!(pred.check(&None::<f64>));
        }

        #[test]
        fn no_coverage_present() {
            let pred = NoCoverage::new();
            assert!(!pred.check(&0.5_f64));
            assert!(!pred.check(&Some(0.5_f64)));
        }

        #[test]
        fn partial_coverage_in_range() {
            let pred = PartialCoverage::new(0.5);
            assert!(pred.check(&0.25_f64));
            assert!(pred.check(&Some(0.25_f64)));
        }

        #[test]
        fn partial_coverage_at_zero() {
            let pred = PartialCoverage::new(0.5);
            assert!(!pred.check(&0.0_f64)); // Zero is excluded
            assert!(!pred.check(&Some(0.0_f64)));
        }

        #[test]
        fn partial_coverage_at_threshold() {
            let pred = PartialCoverage::new(0.5);
            assert!(!pred.check(&0.5_f64)); // At threshold is excluded
            assert!(!pred.check(&Some(0.5_f64)));
        }

        #[test]
        fn partial_coverage_missing() {
            let pred = PartialCoverage::new(0.5);
            assert!(!pred.check(&None::<f64>)); // Missing is not partial
        }
    }

    // =========================================================================
    // Composed Predicate Tests
    // =========================================================================

    mod composed_predicates {
        use super::*;

        #[test]
        fn high_risk_both_conditions() {
            let pred = HighRisk::new(20, 0.5);
            let metrics = create_test_metrics("risky_fn", 30, 10, 2, 30);
            let with_coverage = FunctionWithCoverage {
                metrics: &metrics,
                coverage: Some(0.3),
            };
            assert!(pred.check(&with_coverage));
        }

        #[test]
        fn high_risk_only_complexity() {
            let pred = HighRisk::new(20, 0.5);
            let metrics = create_test_metrics("complex_but_tested", 30, 10, 2, 30);
            let with_coverage = FunctionWithCoverage {
                metrics: &metrics,
                coverage: Some(0.8),
            };
            assert!(!pred.check(&with_coverage));
        }

        #[test]
        fn high_risk_only_low_coverage() {
            let pred = HighRisk::new(20, 0.5);
            let metrics = create_test_metrics("simple_untested", 10, 5, 1, 15);
            let with_coverage = FunctionWithCoverage {
                metrics: &metrics,
                coverage: Some(0.2),
            };
            assert!(!pred.check(&with_coverage));
        }

        #[test]
        fn critical_risk_critical_and_zero() {
            let pred = CriticalRisk::new(50, 50);
            let metrics = create_test_metrics("disaster_fn", 100, 80, 8, 200);
            let with_coverage = FunctionWithCoverage {
                metrics: &metrics,
                coverage: Some(0.0),
            };
            assert!(pred.check(&with_coverage));
        }

        #[test]
        fn critical_risk_critical_but_some_coverage() {
            let pred = CriticalRisk::new(50, 50);
            let metrics = create_test_metrics("complex_tested", 100, 80, 8, 200);
            let with_coverage = FunctionWithCoverage {
                metrics: &metrics,
                coverage: Some(0.5),
            };
            assert!(!pred.check(&with_coverage));
        }

        #[test]
        fn moderate_risk_either_condition() {
            let pred = ModerateRisk::new(20, 0.5);
            let metrics = create_test_metrics("complex_fn", 30, 10, 2, 30);

            // High complexity only
            let with_good_coverage = FunctionWithCoverage {
                metrics: &metrics,
                coverage: Some(0.8),
            };
            assert!(pred.check(&with_good_coverage));

            // Low coverage only
            let simple_metrics = create_test_metrics("simple_fn", 10, 5, 1, 15);
            let with_low_coverage = FunctionWithCoverage {
                metrics: &simple_metrics,
                coverage: Some(0.2),
            };
            assert!(pred.check(&with_low_coverage));
        }

        #[test]
        fn moderate_risk_neither_condition() {
            let pred = ModerateRisk::new(20, 0.5);
            let metrics = create_test_metrics("good_fn", 10, 5, 1, 15);
            let with_coverage = FunctionWithCoverage {
                metrics: &metrics,
                coverage: Some(0.8),
            };
            assert!(!pred.check(&with_coverage));
        }
    }

    // =========================================================================
    // Factory Tests
    // =========================================================================

    mod factory_tests {
        use super::*;

        #[test]
        fn debt_predicates_default() {
            let predicates = DebtPredicates::default();

            // Check default thresholds from ValidationRuleSet::default()
            assert_eq!(predicates.high_cyclomatic.threshold(), 21);
            assert_eq!(predicates.deep_nesting.threshold(), 4);
            assert_eq!(predicates.long_method.threshold(), 50);
        }

        #[test]
        fn debt_predicates_from_strict_rules() {
            let rules = ValidationRuleSet::strict();
            let predicates = DebtPredicates::from_rules(&rules);

            assert_eq!(predicates.high_cyclomatic.threshold(), 10);
            assert_eq!(predicates.deep_nesting.threshold(), 2);
            assert_eq!(predicates.long_method.threshold(), 20);
        }

        #[test]
        fn debt_predicates_from_lenient_rules() {
            let rules = ValidationRuleSet::lenient();
            let predicates = DebtPredicates::from_rules(&rules);

            assert_eq!(predicates.high_cyclomatic.threshold(), 30);
            assert_eq!(predicates.deep_nesting.threshold(), 6);
            assert_eq!(predicates.long_method.threshold(), 100);
        }

        #[test]
        fn debt_predicates_with_custom_coverage() {
            let predicates = DebtPredicates::default().with_coverage_threshold(0.8);
            assert!((predicates.low_coverage.threshold() - 0.8).abs() < 0.001);
        }

        #[test]
        fn debt_predicates_has_complexity_issues() {
            let predicates = DebtPredicates::default();

            let complex = create_test_metrics("complex_fn", 50, 10, 2, 30);
            assert!(predicates.has_complexity_issues(&complex));

            let simple = create_test_metrics("simple_fn", 5, 3, 1, 10);
            assert!(!predicates.has_complexity_issues(&simple));
        }

        #[test]
        fn debt_predicates_has_structural_issues() {
            let predicates = DebtPredicates::default();

            let deeply_nested = create_test_metrics("nested_fn", 10, 8, 10, 30);
            assert!(predicates.has_structural_issues(&deeply_nested));

            let long_fn = create_test_metrics("long_fn", 10, 8, 2, 100);
            assert!(predicates.has_structural_issues(&long_fn));

            let good_fn = create_test_metrics("good_fn", 10, 8, 2, 30);
            assert!(!predicates.has_structural_issues(&good_fn));
        }

        #[test]
        fn debt_predicates_has_any_issues() {
            let predicates = DebtPredicates::default();

            let problematic = create_test_metrics("bad_fn", 50, 40, 8, 100);
            assert!(predicates.has_any_issues(&problematic));

            let good = create_test_metrics("good_fn", 5, 3, 1, 10);
            assert!(!predicates.has_any_issues(&good));
        }
    }

    // =========================================================================
    // Predicate Description Tests
    // =========================================================================

    mod description_tests {
        use super::*;

        #[test]
        fn high_cyclomatic_description() {
            let pred = HighCyclomatic::new(25);
            assert_eq!(pred.description(), "cyclomatic complexity > 25");
        }

        #[test]
        fn high_cognitive_description() {
            let pred = HighCognitive::new(15);
            assert_eq!(pred.description(), "cognitive complexity > 15");
        }

        #[test]
        fn critical_complexity_description() {
            let pred = CriticalComplexity::new(50, 40);
            assert_eq!(pred.description(), "cyclomatic > 50 AND cognitive > 40");
        }

        #[test]
        fn deep_nesting_description() {
            let pred = DeepNesting::new(4);
            assert_eq!(pred.description(), "nesting depth > 4");
        }

        #[test]
        fn long_method_description() {
            let pred = LongMethod::new(50);
            assert_eq!(pred.description(), "line count > 50");
        }

        #[test]
        fn low_coverage_description() {
            let pred = LowCoverage::new(0.5);
            assert_eq!(pred.description(), "coverage < 50%");
        }

        #[test]
        fn no_coverage_description() {
            let pred = NoCoverage::new();
            assert_eq!(pred.description(), "coverage = 0% or absent");
        }

        #[test]
        fn high_risk_description() {
            let pred = HighRisk::new(20, 0.5);
            assert_eq!(
                pred.description(),
                "cyclomatic complexity > 20 AND coverage < 50%"
            );
        }
    }

    // =========================================================================
    // DebtFindings Tests
    // =========================================================================

    mod findings_tests {
        use super::*;

        #[test]
        fn debt_findings_creation() {
            let findings = DebtFindings::new("test_function");
            assert_eq!(findings.function_name, "test_function");
            assert!(findings.results.is_empty());
            assert!(!findings.has_issues());
            assert_eq!(findings.issue_count(), 0);
        }

        #[test]
        fn debt_findings_with_results() {
            let mut findings = DebtFindings::new("problematic_fn");
            findings.add_result(PredicateResult::matched(
                "high_complexity",
                "cyclomatic > 20",
            ));
            findings.add_result(PredicateResult::not_matched("deep_nesting", "nesting > 4"));
            findings.add_result(PredicateResult::matched("long_method", "lines > 50"));

            assert!(findings.has_issues());
            assert_eq!(findings.issue_count(), 2);
            assert_eq!(findings.matched_predicates().count(), 2);
        }

        #[test]
        fn predicate_result_with_details() {
            let result = PredicateResult::matched("high_complexity", "cyclomatic > 20")
                .with_details("Actual value: 45");

            assert!(result.matched);
            assert_eq!(result.details, Some("Actual value: 45".to_string()));
        }
    }
}
