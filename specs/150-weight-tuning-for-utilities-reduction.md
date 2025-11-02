---
number: 150
title: Weight Tuning for Utilities Reduction
category: optimization
priority: high
status: draft
dependencies: [145]
created: 2025-10-28
---

# Specification 150: Weight Tuning for Utilities Reduction

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Spec 145 (Multi-Signal Responsibility Aggregation)

## Context

In the latest debtmap output, "Utilities" is appearing too frequently as a classification:

```
- [M] python_type_tracker_utilities.rs - Utilities (20 methods, ~400 lines)
- [M] mod_utilities.rs - Utilities (40 methods, ~800 lines)
- [M] semantic_classifier_utilities.rs - Utilities (62 methods, ~1240 lines)
```

The "Utilities" category is meant to be a **catch-all for genuine helper functions**, but it's being overused for functions that could be more specifically classified as:
- **Validation** (input checking, error validation)
- **Transformation** (data conversion, normalization)
- **Pure Computation** (calculations, algorithms)
- **I/O Helpers** (file reading wrappers, path manipulation)

This suggests the **current signal weights in Spec 145 are not optimal**. The default weights are:

```rust
io_detection: 0.40          // 40%
call_graph: 0.30            // 30%
type_signatures: 0.15       // 15%
purity_side_effects: 0.10   // 10%
framework_patterns: 0.05    // 5%
name_heuristics: 0.05       // 5%
```

**Hypothesized Issues**:
1. **Name heuristics weight too low** (0.05) - Even when names clearly indicate purpose (e.g., `validate_*`, `format_*`), the weight is too low to override weak I/O signals
2. **Call graph weight too high** (0.30) - Utility functions often have medium coupling, creating weak call graph signals that dominate
3. **Type signatures weight too low** (0.15) - Type patterns like `T → Result<(), E>` (validators) or `T → U` (transformers) should have more influence
4. **No threshold for "Utilities" fallback** - Should only use "Utilities" when all signals are weak (<0.50 confidence)

## Objective

Implement weight tuning system to optimize signal weights for reducing "Utilities" prevalence while maintaining overall classification accuracy. Provide empirical evidence for optimal weights through:
1. Ground truth corpus evaluation
2. Automated weight optimization
3. A/B testing framework
4. Manual tuning tools

Target: Reduce "Utilities" classifications from current ~30% to <10% while maintaining >85% overall accuracy.

## Requirements

### Functional Requirements

**Weight Optimization Engine**:
- Load ground truth corpus (manually labeled functions)
- Test different weight configurations
- Measure accuracy, precision, recall per category
- Identify optimal weights through grid search or gradient descent
- Generate weight tuning report

**Ground Truth Corpus**:
- Manually label 500+ functions across diverse codebases
- Include examples from debtmap's own codebase
- Cover all responsibility categories
- Include edge cases and ambiguous functions
- Version and maintain corpus for regression testing

**Tuning Metrics**:
- **Overall accuracy**: Correct classifications / total
- **Utilities precision**: True utilities / classified as utilities
- **Utilities recall**: Classified as utilities / actual utilities
- **Category balance**: Distribution across all categories
- **Confidence scores**: Average confidence per category

**Configuration Management**:
- Save/load weight configurations
- Compare configurations side-by-side
- A/B test new weights on real codebases
- Gradual rollout of weight changes

**Utilities Threshold**:
- Only classify as "Utilities" when confidence of all specific categories < threshold
- Default threshold: 0.50 (if best signal is <50% confident, use Utilities)
- Require at least 2 weak signals to classify as Utilities

### Non-Functional Requirements

- **Reproducibility**: Same corpus + weights = same results
- **Performance**: Weight optimization completes in <5 minutes
- **Maintainability**: Easy to add new ground truth examples
- **Transparency**: Clear reporting of why weights were changed

## Acceptance Criteria

- [ ] Ground truth corpus contains 500+ manually labeled functions
- [ ] Weight optimization engine finds optimal weights via grid search
- [ ] Optimal weights reduce "Utilities" to <10% of classifications
- [ ] Overall accuracy remains >85% with optimized weights
- [ ] Configuration file supports custom weights
- [ ] A/B testing framework compares old vs new weights
- [ ] Tuning report shows accuracy per category and weight configuration
- [ ] Utilities fallback requires all signals <0.50 confidence
- [ ] Test suite validates weight changes don't regress accuracy
- [ ] Documentation explains how to retune weights for custom codebases

## Technical Details

### Implementation Approach

**Phase 1: Ground Truth Corpus**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruthExample {
    pub file_path: String,
    pub function_name: String,
    pub language: Language,
    pub expected_category: ResponsibilityCategory,
    pub rationale: String,
    pub source: String,  // "debtmap", "tokio", "serde", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruthCorpus {
    pub examples: Vec<GroundTruthExample>,
    pub version: String,
    pub created: DateTime<Utc>,
}

impl GroundTruthCorpus {
    pub fn from_json(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    pub fn validate(&self) -> Result<()> {
        // Ensure balanced distribution
        let category_counts = self.category_distribution();

        for (category, count) in category_counts {
            if count < 20 {
                log::warn!("Category {:?} has only {} examples", category, count);
            }
        }

        Ok(())
    }

    pub fn category_distribution(&self) -> HashMap<ResponsibilityCategory, usize> {
        let mut counts = HashMap::new();

        for example in &self.examples {
            *counts.entry(example.expected_category).or_insert(0) += 1;
        }

        counts
    }
}
```

**Phase 2: Weight Optimization Engine**

```rust
use rayon::prelude::*;

pub struct WeightOptimizer {
    corpus: GroundTruthCorpus,
    classifier: ResponsibilityAggregator,
}

/// Weight configuration for signal aggregation.
/// Weights must sum to 1.0 (enforced at construction time).
#[derive(Debug, Clone, PartialEq)]
pub struct WeightConfig {
    io_detection: f64,
    call_graph: f64,
    type_signatures: f64,
    purity_side_effects: f64,
    framework_patterns: f64,
    name_heuristics: f64,
}

impl WeightConfig {
    /// Create a new WeightConfig from individual weights.
    /// Returns error if weights don't sum to ~1.0.
    pub fn new(
        io_detection: f64,
        call_graph: f64,
        type_signatures: f64,
        purity_side_effects: f64,
        framework_patterns: f64,
        name_heuristics: f64,
    ) -> Result<Self> {
        let config = WeightConfig {
            io_detection,
            call_graph,
            type_signatures,
            purity_side_effects,
            framework_patterns,
            name_heuristics,
        };

        config.validate()?;
        Ok(config)
    }

    /// Get weight components as tuple for pattern matching
    pub fn as_tuple(&self) -> (f64, f64, f64, f64, f64, f64) {
        (
            self.io_detection,
            self.call_graph,
            self.type_signatures,
            self.purity_side_effects,
            self.framework_patterns,
            self.name_heuristics,
        )
    }

    pub fn io_detection(&self) -> f64 { self.io_detection }
    pub fn call_graph(&self) -> f64 { self.call_graph }
    pub fn type_signatures(&self) -> f64 { self.type_signatures }
    pub fn purity_side_effects(&self) -> f64 { self.purity_side_effects }
    pub fn framework_patterns(&self) -> f64 { self.framework_patterns }
    pub fn name_heuristics(&self) -> f64 { self.name_heuristics }

    fn validate(&self) -> Result<()> {
        let sum = self.io_detection
            + self.call_graph
            + self.type_signatures
            + self.purity_side_effects
            + self.framework_patterns
            + self.name_heuristics;

        if (sum - 1.0).abs() > 0.01 {
            return Err(anyhow!("Weights must sum to 1.0, got {}", sum));
        }

        // Ensure all weights are non-negative
        let weights = [
            self.io_detection,
            self.call_graph,
            self.type_signatures,
            self.purity_side_effects,
            self.framework_patterns,
            self.name_heuristics,
        ];

        if weights.iter().any(|&w| w < 0.0 || w > 1.0) {
            return Err(anyhow!("All weights must be in range [0.0, 1.0]"));
        }

        Ok(())
    }

    pub fn default_weights() -> Self {
        WeightConfig {
            io_detection: 0.40,
            call_graph: 0.30,
            type_signatures: 0.15,
            purity_side_effects: 0.10,
            framework_patterns: 0.05,
            name_heuristics: 0.05,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OptimizationResult {
    pub weights: WeightConfig,
    pub accuracy: f64,
    pub utilities_precision: f64,
    pub utilities_recall: f64,
    pub category_accuracies: HashMap<ResponsibilityCategory, f64>,
    pub confusion_matrix: ConfusionMatrix,
}

/// Single classification result for functional aggregation
#[derive(Debug, Clone)]
struct ClassificationResult {
    expected: ResponsibilityCategory,
    actual: ResponsibilityCategory,
    is_utilities_expected: bool,
    is_utilities_actual: bool,
}

impl ClassificationResult {
    fn is_correct(&self) -> bool {
        self.expected == self.actual
    }
}

/// Aggregated metrics from classification results
#[derive(Debug, Clone, Default)]
struct ClassificationMetrics {
    total: usize,
    correct: usize,
    utilities_tp: usize,
    utilities_fp: usize,
    utilities_fn: usize,
    category_counts: HashMap<ResponsibilityCategory, CategoryCount>,
}

#[derive(Debug, Clone, Default)]
struct CategoryCount {
    total: usize,
    correct: usize,
}

impl ClassificationMetrics {
    fn empty() -> Self {
        Self::default()
    }

    fn from_result(result: ClassificationResult) -> Self {
        let mut metrics = Self::empty();
        metrics.total = 1;
        metrics.correct = if result.is_correct() { 1 } else { 0 };

        // Track utilities precision/recall
        metrics.utilities_tp = if result.is_utilities_expected && result.is_utilities_actual { 1 } else { 0 };
        metrics.utilities_fp = if !result.is_utilities_expected && result.is_utilities_actual { 1 } else { 0 };
        metrics.utilities_fn = if result.is_utilities_expected && !result.is_utilities_actual { 1 } else { 0 };

        // Track per-category accuracy
        let mut category_count = CategoryCount { total: 1, correct: 0 };
        if result.is_correct() {
            category_count.correct = 1;
        }
        metrics.category_counts.insert(result.expected, category_count);

        metrics
    }

    fn merge(self, other: Self) -> Self {
        let mut category_counts = self.category_counts;
        for (cat, other_count) in other.category_counts {
            category_counts
                .entry(cat)
                .and_modify(|count| {
                    count.total += other_count.total;
                    count.correct += other_count.correct;
                })
                .or_insert(other_count);
        }

        ClassificationMetrics {
            total: self.total + other.total,
            correct: self.correct + other.correct,
            utilities_tp: self.utilities_tp + other.utilities_tp,
            utilities_fp: self.utilities_fp + other.utilities_fp,
            utilities_fn: self.utilities_fn + other.utilities_fn,
            category_counts,
        }
    }

    fn into_optimization_result(self, weights: WeightConfig) -> Result<OptimizationResult> {
        let accuracy = if self.total > 0 {
            self.correct as f64 / self.total as f64
        } else {
            0.0
        };

        let utilities_precision = {
            let denom = self.utilities_tp + self.utilities_fp;
            if denom > 0 {
                self.utilities_tp as f64 / denom as f64
            } else {
                1.0  // No false positives = perfect precision
            }
        };

        let utilities_recall = {
            let denom = self.utilities_tp + self.utilities_fn;
            if denom > 0 {
                self.utilities_tp as f64 / denom as f64
            } else {
                1.0  // No actual utilities = perfect recall
            }
        };

        let category_accuracies = self.category_counts
            .into_iter()
            .map(|(cat, count)| {
                let acc = if count.total > 0 {
                    count.correct as f64 / count.total as f64
                } else {
                    0.0
                };
                (cat, acc)
            })
            .collect();

        Ok(OptimizationResult {
            weights,
            accuracy,
            utilities_precision,
            utilities_recall,
            category_accuracies,
            confusion_matrix: ConfusionMatrix::new(),  // TODO: implement
        })
    }
}

impl WeightOptimizer {
    pub fn optimize_grid_search(&self) -> Result<OptimizationResult> {
        // Generate all weight combinations
        let weight_configs = self.generate_weight_grid();

        log::info!("Testing {} weight configurations", weight_configs.len());

        // Parallel evaluation of all weight configurations
        weight_configs
            .into_par_iter()
            .map(|weights| {
                let result = self.evaluate_weights(&weights)?;
                let score = calculate_optimization_score(&result);
                Ok((score, result))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(_, result)| result)
            .ok_or_else(|| anyhow!("No valid weight configuration found"))
    }

    fn generate_weight_grid(&self) -> Vec<WeightConfig> {
        let mut configs = Vec::new();

        // Grid search over weight combinations
        for io in (20..=60).step_by(10) {
            for cg in (10..=40).step_by(10) {
                for ts in (10..=30).step_by(10) {
                    for name in (0..=15).step_by(5) {
                        let remaining = 100 - io - cg - ts - name;
                        let purity = remaining / 2;
                        let framework = remaining - purity;

                        if let Ok(weights) = WeightConfig::new(
                            io as f64 / 100.0,
                            cg as f64 / 100.0,
                            ts as f64 / 100.0,
                            purity as f64 / 100.0,
                            framework as f64 / 100.0,
                            name as f64 / 100.0,
                        ) {
                            configs.push(weights);
                        }
                    }
                }
            }
        }

        configs
    }

    fn evaluate_weights(&self, weights: &WeightConfig) -> Result<OptimizationResult> {
        let mut classifier = self.classifier.clone();
        classifier.set_weights(weights.clone());

        // Parallel classification of all examples
        let metrics = self.corpus.examples
            .par_iter()
            .map(|example| self.classify_example(example, &classifier))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .map(ClassificationMetrics::from_result)
            .fold(ClassificationMetrics::empty(), |acc, m| acc.merge(m));

        metrics.into_optimization_result(weights.clone())
    }

    fn classify_example(
        &self,
        example: &GroundTruthExample,
        classifier: &ResponsibilityAggregator,
    ) -> Result<ClassificationResult> {
        let function = self.analyze_function(&example.file_path, &example.function_name)?;
        let signals = self.collect_signals(&function)?;
        let classification = classifier.aggregate(&signals);

        Ok(ClassificationResult {
            expected: example.expected_category,
            actual: classification.primary,
            is_utilities_expected: example.expected_category == ResponsibilityCategory::Utilities,
            is_utilities_actual: classification.primary == ResponsibilityCategory::Utilities,
        })
    }
}

/// Pure function to calculate optimization score from results.
/// Balances accuracy, utilities reduction, and category balance.
fn calculate_optimization_score(result: &OptimizationResult) -> f64 {
    let accuracy_score = result.accuracy * 0.50;  // 50% weight on overall accuracy
    let utilities_score = (1.0 - result.utilities_recall) * 0.30;  // 30% weight on reducing utilities
    let balance_score = calculate_balance_score(&result.category_accuracies) * 0.20;  // 20% weight on category balance

    accuracy_score + utilities_score + balance_score
}

/// Pure function to calculate balance score from category accuracies.
/// Returns minimum accuracy across all categories (penalizes poor performance on any category).
fn calculate_balance_score(category_accuracies: &HashMap<ResponsibilityCategory, f64>) -> f64 {
    category_accuracies
        .values()
        .copied()
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0.0)
}
```

**Phase 3: Utilities Fallback Threshold**

```rust
/// Configuration for utilities fallback behavior
#[derive(Debug, Clone)]
pub struct AggregationConfig {
    pub utilities_threshold: f64,
    pub min_confidence: f64,
}

impl Default for AggregationConfig {
    fn default() -> Self {
        Self {
            utilities_threshold: 0.50,
            min_confidence: 0.40,
        }
    }
}

impl ResponsibilityAggregator {
    /// Aggregate signals with utilities threshold applied.
    /// Only classifies as Utilities when all specific category signals are weak.
    pub fn aggregate_with_utilities_threshold(
        &self,
        signals: &SignalSet,
        config: &AggregationConfig,
    ) -> AggregatedClassification {
        let base_result = self.aggregate(signals);

        apply_utilities_threshold(base_result, signals, config)
    }
}

/// Pure function to apply utilities threshold logic to classification.
/// Returns the original classification unless it should be overridden.
fn apply_utilities_threshold(
    classification: AggregatedClassification,
    signals: &SignalSet,
    config: &AggregationConfig,
) -> AggregatedClassification {
    // Only intervene if classified as Utilities or confidence is low
    if !should_check_utilities_override(&classification, config) {
        return classification;
    }

    // Check if there's a strong non-utility signal to use instead
    match find_best_non_utility_signal(signals, config.utilities_threshold) {
        Some(signal) => create_classification_from_signal(signal),
        None => classification,  // Genuinely utilities - all signals weak
    }
}

/// Pure predicate: should we check for utilities override?
fn should_check_utilities_override(
    classification: &AggregatedClassification,
    config: &AggregationConfig,
) -> bool {
    classification.primary == ResponsibilityCategory::Utilities
        || classification.confidence < config.utilities_threshold
}

/// Pure function to find the strongest non-utility signal above threshold.
/// Returns None if all non-utility signals are weak.
fn find_best_non_utility_signal(
    signals: &SignalSet,
    threshold: f64,
) -> Option<&Signal> {
    signals
        .signals
        .iter()
        .filter(|s| s.category != ResponsibilityCategory::Utilities)
        .filter(|s| s.confidence >= threshold)
        .max_by(|a, b| {
            a.confidence
                .partial_cmp(&b.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// Pure function to create classification from a signal.
fn create_classification_from_signal(signal: &Signal) -> AggregatedClassification {
    AggregatedClassification {
        primary: signal.category,
        confidence: signal.confidence,
        evidence: vec![signal.clone()],
        alternatives: vec![],
    }
}
```

**Phase 4: A/B Testing Framework**

```rust
pub struct WeightABTest {
    pub control_weights: WeightConfig,
    pub experiment_weights: WeightConfig,
    pub corpus: GroundTruthCorpus,
}

impl WeightABTest {
    pub fn run(&self) -> ABTestResult {
        let control_result = self.evaluate_weights(&self.control_weights);
        let experiment_result = self.evaluate_weights(&self.experiment_weights);

        ABTestResult {
            control: control_result,
            experiment: experiment_result,
            improvement: self.calculate_improvement(&control_result, &experiment_result),
        }
    }

    fn calculate_improvement(&self, control: &OptimizationResult, experiment: &OptimizationResult) -> ImprovementMetrics {
        ImprovementMetrics {
            accuracy_delta: experiment.accuracy - control.accuracy,
            utilities_reduction: control.utilities_recall - experiment.utilities_recall,
            category_improvements: self.compare_category_accuracies(control, experiment),
        }
    }
}
```

### Architecture Changes

**New Module**: `src/tuning/weight_optimizer.rs`
- Weight optimization engine
- Grid search and gradient descent
- Evaluation metrics

**New Module**: `src/tuning/ground_truth.rs`
- Ground truth corpus management
- Example loading and validation
- Category distribution analysis

**New File**: `tests/ground_truth/corpus.json`
- Manually labeled examples (500+ functions)
- Version controlled for regression testing
- Diverse codebase coverage

**Modified Module**: `src/analysis/multi_signal_aggregation.rs`
- Add utilities threshold configuration
- Configurable weights (from file or defaults)
- Enhanced fallback logic

**New Configuration**: `weights.toml`
```toml
[weights]
io_detection = 0.35       # Reduced from 0.40
call_graph = 0.25         # Reduced from 0.30
type_signatures = 0.20    # Increased from 0.15
purity_side_effects = 0.10
framework_patterns = 0.05
name_heuristics = 0.05    # Keep low but present

[thresholds]
utilities_fallback = 0.50  # Only use Utilities if all signals < 0.50
min_confidence = 0.40      # Minimum confidence to use any classification
```

## Dependencies

- **Prerequisites**: Spec 145 (Multi-Signal Aggregation)
- **Affected Components**:
  - `src/analysis/multi_signal_aggregation.rs` - weight configuration
  - `src/config.rs` - weight settings
- **External Dependencies**:
  - `rayon` - parallel processing for grid search and evaluation (already in project)
  - `serde_json` - ground truth corpus serialization (already in project)
  - `proptest` - property-based testing (dev dependency, already in project)
  - `pretty_assertions` - better test output (dev dependency, already in project)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn weight_config_validation() {
        // Valid configuration
        let valid = WeightConfig::new(0.40, 0.30, 0.15, 0.10, 0.03, 0.02);
        assert!(valid.is_ok());

        // Invalid: sum > 1.0
        let invalid = WeightConfig::new(0.50, 0.30, 0.30, 0.10, 0.05, 0.05);
        assert!(invalid.is_err());

        // Invalid: negative weight
        let negative = WeightConfig::new(-0.1, 0.5, 0.3, 0.2, 0.05, 0.05);
        assert!(negative.is_err());

        // Invalid: weight > 1.0
        let too_large = WeightConfig::new(1.5, 0.0, 0.0, 0.0, 0.0, -0.5);
        assert!(too_large.is_err());
    }

    #[test]
    fn utilities_threshold_prevents_overuse() {
        let signals = SignalSet {
            signals: vec![
                Signal {
                    category: ResponsibilityCategory::FileIO,
                    confidence: 0.55,  // Above threshold
                    evidence: "File I/O operations detected".to_string(),
                },
                Signal {
                    category: ResponsibilityCategory::Utilities,
                    confidence: 0.45,  // Below threshold
                    evidence: "Medium coupling".to_string(),
                },
            ],
        };

        let config = AggregationConfig {
            utilities_threshold: 0.50,
            min_confidence: 0.40,
        };

        let aggregator = ResponsibilityAggregator::new();
        let result = aggregator.aggregate_with_utilities_threshold(&signals, &config);

        // Should NOT classify as Utilities when FileIO signal is strong enough
        assert_ne!(result.primary, ResponsibilityCategory::Utilities);
        assert_eq!(result.primary, ResponsibilityCategory::FileIO);
    }

    #[test]
    fn utilities_used_when_all_signals_weak() {
        let signals = SignalSet {
            signals: vec![
                Signal {
                    category: ResponsibilityCategory::FileIO,
                    confidence: 0.35,  // Below threshold
                    evidence: "Weak I/O signal".to_string(),
                },
                Signal {
                    category: ResponsibilityCategory::Validation,
                    confidence: 0.40,  // Below threshold
                    evidence: "Weak validation signal".to_string(),
                },
                Signal {
                    category: ResponsibilityCategory::Utilities,
                    confidence: 0.48,
                    evidence: "Generic helper".to_string(),
                },
            ],
        };

        let config = AggregationConfig::default();

        let aggregator = ResponsibilityAggregator::new();
        let result = aggregator.aggregate_with_utilities_threshold(&signals, &config);

        // Should classify as Utilities when all specific signals are weak
        assert_eq!(result.primary, ResponsibilityCategory::Utilities);
    }

    #[test]
    fn classification_metrics_merge_is_associative() {
        let m1 = ClassificationMetrics {
            total: 10,
            correct: 8,
            utilities_tp: 2,
            utilities_fp: 1,
            utilities_fn: 1,
            category_counts: HashMap::new(),
        };

        let m2 = ClassificationMetrics {
            total: 15,
            correct: 12,
            utilities_tp: 3,
            utilities_fp: 2,
            utilities_fn: 0,
            category_counts: HashMap::new(),
        };

        let m3 = ClassificationMetrics {
            total: 5,
            correct: 4,
            utilities_tp: 1,
            utilities_fp: 0,
            utilities_fn: 1,
            category_counts: HashMap::new(),
        };

        // (m1 + m2) + m3 should equal m1 + (m2 + m3)
        let left = m1.clone().merge(m2.clone()).merge(m3.clone());
        let right = m1.merge(m2.merge(m3));

        assert_eq!(left.total, right.total);
        assert_eq!(left.correct, right.correct);
        assert_eq!(left.utilities_tp, right.utilities_tp);
    }

    // Property-based tests
    proptest! {
        #[test]
        fn weight_config_always_sums_to_one(
            io in 0.0..1.0,
            cg in 0.0..1.0,
            ts in 0.0..1.0,
        ) {
            // Generate remaining weights to sum to 1.0
            let total = io + cg + ts;
            if total <= 0.99 {
                let remaining = 1.0 - total;
                let purity = remaining / 3.0;
                let framework = remaining / 3.0;
                let name = remaining - purity - framework;

                if let Ok(config) = WeightConfig::new(io, cg, ts, purity, framework, name) {
                    let sum = config.io_detection()
                        + config.call_graph()
                        + config.type_signatures()
                        + config.purity_side_effects()
                        + config.framework_patterns()
                        + config.name_heuristics();

                    prop_assert!((sum - 1.0).abs() < 0.01);
                }
            }
        }

        #[test]
        fn accuracy_always_between_zero_and_one(
            correct in 0..1000usize,
            total in 1..1000usize,
        ) {
            let correct = correct.min(total);  // Ensure correct <= total
            let metrics = ClassificationMetrics {
                total,
                correct,
                utilities_tp: 0,
                utilities_fp: 0,
                utilities_fn: 0,
                category_counts: HashMap::new(),
            };

            let result = metrics.into_optimization_result(WeightConfig::default_weights()).unwrap();
            prop_assert!(result.accuracy >= 0.0 && result.accuracy <= 1.0);
        }

        #[test]
        fn metrics_merge_is_commutative(
            t1 in 0..100usize,
            c1 in 0..100usize,
            t2 in 0..100usize,
            c2 in 0..100usize,
        ) {
            let m1 = ClassificationMetrics {
                total: t1,
                correct: c1.min(t1),
                utilities_tp: 0,
                utilities_fp: 0,
                utilities_fn: 0,
                category_counts: HashMap::new(),
            };

            let m2 = ClassificationMetrics {
                total: t2,
                correct: c2.min(t2),
                utilities_tp: 0,
                utilities_fp: 0,
                utilities_fn: 0,
                category_counts: HashMap::new(),
            };

            let left = m1.clone().merge(m2.clone());
            let right = m2.merge(m1);

            prop_assert_eq!(left.total, right.total);
            prop_assert_eq!(left.correct, right.correct);
        }

        #[test]
        fn optimization_score_bounded(
            accuracy in 0.0..1.0,
            utilities_recall in 0.0..1.0,
        ) {
            let result = OptimizationResult {
                weights: WeightConfig::default_weights(),
                accuracy,
                utilities_precision: 0.8,
                utilities_recall,
                category_accuracies: HashMap::new(),
                confusion_matrix: ConfusionMatrix::new(),
            };

            let score = calculate_optimization_score(&result);
            prop_assert!(score >= 0.0 && score <= 1.0);
        }
    }
}
```

### Integration Tests

```rust
#[test]
fn optimized_weights_reduce_utilities() {
    let corpus = GroundTruthCorpus::from_json("tests/ground_truth/corpus.json").unwrap();
    let optimizer = WeightOptimizer::new(corpus);

    let default_weights = WeightConfig::default();
    let optimized_weights = optimizer.optimize_grid_search().unwrap();

    let default_result = optimizer.evaluate_weights(&default_weights).unwrap();
    let optimized_result = optimizer.evaluate_weights(&optimized_weights.weights).unwrap();

    // Optimized should have lower utilities recall
    assert!(optimized_result.utilities_recall < default_result.utilities_recall);

    // But maintain overall accuracy
    assert!(optimized_result.accuracy >= default_result.accuracy * 0.95);
}
```

## Corpus Tooling

### Interactive Labeling CLI

```rust
/// CLI for creating and managing ground truth corpus
pub struct CorpusLabelingTool {
    corpus_path: PathBuf,
    corpus: GroundTruthCorpus,
}

impl CorpusLabelingTool {
    /// Interactive labeling session
    pub fn label_functions(&mut self, project_path: &Path) -> Result<()> {
        let functions = discover_functions(project_path)?;

        for func in functions {
            // Display function code with syntax highlighting
            self.display_function(&func)?;

            // Show current classification from analyzer
            let current = self.classify_function(&func)?;
            println!("Current classification: {:?} ({:.0}% confidence)",
                     current.primary, current.confidence * 100.0);

            // Prompt for correct category
            let category = self.prompt_category()?;
            let rationale = self.prompt_rationale()?;

            self.corpus.add_example(GroundTruthExample {
                file_path: func.file_path.clone(),
                function_name: func.name.clone(),
                language: func.language,
                expected_category: category,
                rationale,
                source: "user_labeled".to_string(),
            });

            // Save incrementally
            self.save()?;
        }

        Ok(())
    }

    /// Validate corpus for balance and quality
    pub fn validate(&self) -> Result<ValidationReport> {
        let distribution = self.corpus.category_distribution();

        let warnings = distribution
            .iter()
            .filter(|(_, &count)| count < 20)
            .map(|(cat, count)| {
                format!("Category {:?} has only {} examples (minimum 20 recommended)", cat, count)
            })
            .collect();

        Ok(ValidationReport {
            total_examples: self.corpus.examples.len(),
            category_distribution: distribution,
            warnings,
        })
    }
}

// CLI commands
pub fn ground_truth_create(output: &Path) -> Result<()> {
    let corpus = GroundTruthCorpus {
        examples: vec![],
        version: "1.0.0".to_string(),
        created: Utc::now(),
    };

    corpus.save_to_json(output)?;
    println!("Created empty corpus at {}", output.display());
    Ok(())
}

pub fn ground_truth_label(corpus_path: &Path, project_path: &Path) -> Result<()> {
    let mut tool = CorpusLabelingTool::load(corpus_path)?;
    tool.label_functions(project_path)?;

    let report = tool.validate()?;
    println!("{}", report);
    Ok(())
}

pub fn ground_truth_validate(corpus_path: &Path) -> Result<()> {
    let corpus = GroundTruthCorpus::from_json(corpus_path)?;
    let validator = CorpusValidator::new(corpus);

    let report = validator.validate()?;

    if report.warnings.is_empty() {
        println!("✓ Corpus is well-balanced ({} examples)", report.total_examples);
    } else {
        println!("⚠ Corpus validation warnings:");
        for warning in report.warnings {
            println!("  - {}", warning);
        }
    }

    println!("\nCategory distribution:");
    for (cat, count) in report.category_distribution {
        let percentage = (count as f64 / report.total_examples as f64) * 100.0;
        println!("  {:?}: {} ({:.1}%)", cat, count, percentage);
    }

    Ok(())
}
```

## Configuration Management

### Configuration File Locations

Debtmap looks for weight configuration in the following order (highest priority first):

1. **CLI argument**: `--weights <path>` or `--weights-preset <name>`
2. **Project config**: `.debtmap/weights.toml` (in project root)
3. **User config**: `~/.config/debtmap/weights.toml`
4. **Built-in presets**: `default`, `optimized`, `aggressive`
5. **Hardcoded defaults**: Spec 145 defaults

### Configuration File Format

**`.debtmap/weights.toml`** or **`~/.config/debtmap/weights.toml`**:

```toml
# Weight configuration for responsibility classification
version = "1.0"

[weights]
io_detection = 0.35       # File I/O and network detection (0.0-1.0)
call_graph = 0.25         # Call graph coupling analysis
type_signatures = 0.20    # Type pattern matching
purity_side_effects = 0.10  # Purity and side effect detection
framework_patterns = 0.05   # Framework-specific patterns
name_heuristics = 0.05      # Function name analysis

[thresholds]
utilities_fallback = 0.50  # Only use Utilities if all signals < threshold
min_confidence = 0.40      # Minimum confidence to report classification

[optimization]
# Optional: track where these weights came from
source = "optimized"       # "default", "optimized", "manual", or "custom"
generated_from_corpus = "my_corpus.json"
optimization_date = "2025-10-28"
accuracy_on_corpus = 0.88
```

### Built-in Presets

```bash
# Use default weights (Spec 145)
debtmap analyze

# Use optimized weights (from this spec)
debtmap analyze --weights-preset optimized

# Use custom weights file
debtmap analyze --weights .debtmap/weights.toml

# Use aggressive utilities reduction
debtmap analyze --weights-preset aggressive
```

**Preset definitions**:

```rust
pub enum WeightPreset {
    Default,     // Spec 145 weights
    Optimized,   // Spec 150 optimized weights
    Aggressive,  // Maximize utilities reduction
}

impl WeightPreset {
    pub fn to_config(&self) -> WeightConfig {
        match self {
            WeightPreset::Default => WeightConfig::new(
                0.40, 0.30, 0.15, 0.10, 0.05, 0.05
            ).unwrap(),
            WeightPreset::Optimized => WeightConfig::new(
                0.35, 0.25, 0.20, 0.10, 0.05, 0.10
            ).unwrap(),
            WeightPreset::Aggressive => WeightConfig::new(
                0.30, 0.20, 0.25, 0.10, 0.05, 0.15
            ).unwrap(),
        }
    }
}
```

## Documentation Requirements

### User Documentation

Update README.md:
```markdown
## Weight Tuning

Debtmap's classification weights can be customized via `weights.toml`:

```toml
[weights]
io_detection = 0.35
call_graph = 0.25
type_signatures = 0.20
purity_side_effects = 0.10
framework_patterns = 0.05
name_heuristics = 0.10
```

### Creating a Custom Ground Truth Corpus

```bash
# 1. Create empty corpus
debtmap ground-truth create --output my_corpus.json

# 2. Interactively label functions from your codebase
debtmap ground-truth label --corpus my_corpus.json --project /path/to/project

# 3. Validate corpus balance
debtmap ground-truth validate --corpus my_corpus.json

# 4. Optimize weights based on corpus
debtmap tune-weights --corpus my_corpus.json --output tuned_weights.toml

# 5. Test new weights
debtmap analyze --weights tuned_weights.toml

# 6. Compare with default weights
debtmap tune-weights --corpus my_corpus.json --compare
```

### Using Weight Presets

```bash
# Use default weights (balanced, conservative)
debtmap analyze

# Use optimized weights (reduced utilities prevalence)
debtmap analyze --weights-preset optimized

# Use aggressive utilities reduction
debtmap analyze --weights-preset aggressive
```

### Configuration File Locations

- **Project-specific**: `.debtmap/weights.toml` (committed to version control)
- **User-specific**: `~/.config/debtmap/weights.toml` (personal preferences)
- **CLI override**: `--weights <file>` (for experiments)
```

### Ground Truth Example Format

`tests/ground_truth/corpus.json`:
```json
{
  "version": "1.0.0",
  "created": "2025-10-28T00:00:00Z",
  "examples": [
    {
      "file_path": "src/io/reader.rs",
      "function_name": "read_config",
      "language": "Rust",
      "expected_category": "FileIO",
      "rationale": "Reads file from disk, returns Result<String, io::Error>",
      "source": "debtmap"
    },
    {
      "file_path": "src/analysis/parser.rs",
      "function_name": "parse_json",
      "language": "Rust",
      "expected_category": "Parsing",
      "rationale": "Parses JSON string to struct, &str → Result<T, E> pattern",
      "source": "debtmap"
    }
  ]
}
```

## Implementation Notes

### Creating Ground Truth Corpus

Prioritize diversity:
- **10% debtmap**: Own codebase examples
- **20% popular crates**: tokio, serde, clap, diesel
- **30% web frameworks**: axum, actix-web, rocket
- **20% data processing**: polars, arrow, datafusion
- **20% utilities**: itertools, rayon, regex

Balance across categories:
- FileIO: 60 examples
- Parsing: 60 examples
- Formatting: 50 examples
- Validation: 50 examples
- Transformation: 50 examples
- Orchestration: 40 examples
- **Utilities: 30 examples** (genuine helpers only)
- Pure Computation: 40 examples
- ... (other categories)

### Suggested Starting Weights

Based on analysis of current "Utilities" overuse:

```toml
# Optimized weights (hypothesis)
[weights]
io_detection = 0.35       # Reduce: Too dominant, causing mixed signals
call_graph = 0.25         # Reduce: Medium coupling common in helpers
type_signatures = 0.20    # Increase: Strong signal for specific patterns
purity_side_effects = 0.10  # Keep: Complementary signal
framework_patterns = 0.05   # Keep: Override when detected
name_heuristics = 0.05      # Keep: Useful fallback

[thresholds]
utilities_fallback = 0.50   # Require all signals weak to use Utilities
```

## Migration and Compatibility

### Backward Compatibility

Default weights remain the same initially. Users can opt-in to optimized weights:

```bash
# Use default weights (Spec 145)
debtmap analyze

# Use optimized weights (this spec)
debtmap analyze --weights optimized
```

### Gradual Rollout

1. **v0.4.0**: Add weight tuning capability, keep defaults
2. **v0.4.1**: Ship optimized weights as optional preset
3. **v0.5.0**: Make optimized weights the new default

## Expected Impact

### Utilities Reduction

**Current distribution** (estimated from output):
- Utilities: ~30%
- Specific categories: ~70%

**Target distribution**:
- Utilities: <10% (genuine helpers only)
- Specific categories: >90%

**Expected improvements**:
```
python_type_tracker_utilities.rs (20 methods)
  Before: Utilities (48% confidence)
  After: Validation (72% confidence) + Transformation (65% confidence)
  → Split into validation.rs and transformation.rs

mod_utilities.rs (40 methods)
  Before: Utilities (45% confidence)
  After: I/O Helpers (68% confidence) + Pure Computation (71% confidence)
  → Split into io_helpers.rs and computation.rs
```

### Accuracy Maintenance

- Overall accuracy: Maintain >85%
- Per-category accuracy: All categories >70%
- Confidence scores: Increase average from ~0.60 to ~0.75

## Success Metrics

- [ ] Ground truth corpus created with 500+ examples
- [ ] Optimized weights reduce Utilities to <10% of classifications
- [ ] Overall accuracy remains >85%
- [ ] Utilities precision >80% (true utilities / classified as utilities)
- [ ] No category drops below 70% accuracy
- [ ] Average confidence increases by >10%
- [ ] User feedback: Fewer complaints about "Utilities" groupings
