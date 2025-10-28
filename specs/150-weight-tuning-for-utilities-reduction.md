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
use ndarray::Array1;

pub struct WeightOptimizer {
    corpus: GroundTruthCorpus,
    classifier: ResponsibilityAggregator,
}

#[derive(Debug, Clone)]
pub struct WeightConfig {
    pub io_detection: f64,
    pub call_graph: f64,
    pub type_signatures: f64,
    pub purity_side_effects: f64,
    pub framework_patterns: f64,
    pub name_heuristics: f64,
}

impl WeightConfig {
    pub fn to_array(&self) -> Array1<f64> {
        Array1::from_vec(vec![
            self.io_detection,
            self.call_graph,
            self.type_signatures,
            self.purity_side_effects,
            self.framework_patterns,
            self.name_heuristics,
        ])
    }

    pub fn from_array(arr: &Array1<f64>) -> Self {
        WeightConfig {
            io_detection: arr[0],
            call_graph: arr[1],
            type_signatures: arr[2],
            purity_side_effects: arr[3],
            framework_patterns: arr[4],
            name_heuristics: arr[5],
        }
    }

    pub fn validate(&self) -> Result<()> {
        let sum = self.io_detection
            + self.call_graph
            + self.type_signatures
            + self.purity_side_effects
            + self.framework_patterns
            + self.name_heuristics;

        if (sum - 1.0).abs() > 0.01 {
            return Err(anyhow!("Weights must sum to 1.0, got {}", sum));
        }

        Ok(())
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

impl WeightOptimizer {
    pub fn optimize_grid_search(&self) -> Result<OptimizationResult> {
        let mut best_result = None;
        let mut best_score = 0.0;

        // Grid search over weight combinations
        for io in (20..=60).step_by(10) {
            for cg in (10..=40).step_by(10) {
                for ts in (10..=30).step_by(10) {
                    for name in (0..=15).step_by(5) {
                        let remaining = 100 - io - cg - ts - name;
                        let purity = remaining / 2;
                        let framework = remaining - purity;

                        let weights = WeightConfig {
                            io_detection: io as f64 / 100.0,
                            call_graph: cg as f64 / 100.0,
                            type_signatures: ts as f64 / 100.0,
                            purity_side_effects: purity as f64 / 100.0,
                            framework_patterns: framework as f64 / 100.0,
                            name_heuristics: name as f64 / 100.0,
                        };

                        if weights.validate().is_err() {
                            continue;
                        }

                        let result = self.evaluate_weights(&weights)?;
                        let score = self.calculate_score(&result);

                        if score > best_score {
                            best_score = score;
                            best_result = Some(result);
                        }
                    }
                }
            }
        }

        best_result.ok_or_else(|| anyhow!("No valid weight configuration found"))
    }

    fn evaluate_weights(&self, weights: &WeightConfig) -> Result<OptimizationResult> {
        let mut classifier = self.classifier.clone();
        classifier.set_weights(weights.clone());

        let mut correct = 0;
        let mut total = 0;
        let mut utilities_tp = 0;  // True positives
        let mut utilities_fp = 0;  // False positives
        let mut utilities_fn = 0;  // False negatives

        let mut category_correct: HashMap<ResponsibilityCategory, usize> = HashMap::new();
        let mut category_total: HashMap<ResponsibilityCategory, usize> = HashMap::new();

        for example in &self.corpus.examples {
            let function = self.analyze_function(&example.file_path, &example.function_name)?;
            let signals = self.collect_signals(&function)?;
            let classification = classifier.aggregate(&signals);

            total += 1;
            *category_total.entry(example.expected_category).or_insert(0) += 1;

            if classification.primary == example.expected_category {
                correct += 1;
                *category_correct.entry(example.expected_category).or_insert(0) += 1;
            }

            // Track utilities precision/recall
            if example.expected_category == ResponsibilityCategory::Utilities {
                if classification.primary == ResponsibilityCategory::Utilities {
                    utilities_tp += 1;
                } else {
                    utilities_fn += 1;
                }
            } else if classification.primary == ResponsibilityCategory::Utilities {
                utilities_fp += 1;
            }
        }

        let accuracy = correct as f64 / total as f64;
        let utilities_precision = utilities_tp as f64 / (utilities_tp + utilities_fp) as f64;
        let utilities_recall = utilities_tp as f64 / (utilities_tp + utilities_fn) as f64;

        let category_accuracies = category_total.iter()
            .map(|(cat, &total)| {
                let correct = category_correct.get(cat).copied().unwrap_or(0);
                (*cat, correct as f64 / total as f64)
            })
            .collect();

        Ok(OptimizationResult {
            weights: weights.clone(),
            accuracy,
            utilities_precision,
            utilities_recall,
            category_accuracies,
            confusion_matrix: ConfusionMatrix::new(),  // TODO: implement
        })
    }

    fn calculate_score(&self, result: &OptimizationResult) -> f64 {
        // Composite score balancing multiple objectives
        let accuracy_score = result.accuracy * 0.50;  // 50% weight on overall accuracy
        let utilities_score = (1.0 - result.utilities_recall) * 0.30;  // 30% weight on reducing utilities
        let balance_score = self.calculate_balance_score(result) * 0.20;  // 20% weight on category balance

        accuracy_score + utilities_score + balance_score
    }

    fn calculate_balance_score(&self, result: &OptimizationResult) -> f64 {
        // Penalize if some categories have very low accuracy
        let min_accuracy = result.category_accuracies.values()
            .copied()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        min_accuracy
    }
}
```

**Phase 3: Utilities Fallback Threshold**

```rust
impl ResponsibilityAggregator {
    pub fn aggregate_with_utilities_threshold(
        &self,
        signals: &SignalSet,
        config: &AggregationConfig,
    ) -> AggregatedClassification {
        let base_result = self.aggregate(signals);

        // Check if should fallback to Utilities
        if base_result.primary == ResponsibilityCategory::Utilities ||
           base_result.confidence < config.utilities_threshold {

            // Check if ALL specific category signals are weak
            let all_weak = signals.signals.iter()
                .filter(|s| s.category != ResponsibilityCategory::Utilities)
                .all(|s| s.confidence < config.utilities_threshold);

            if all_weak {
                // Genuine utilities case
                return base_result;
            } else {
                // At least one strong signal exists, use it
                let best_non_utility = signals.signals.iter()
                    .filter(|s| s.category != ResponsibilityCategory::Utilities)
                    .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap());

                if let Some(signal) = best_non_utility {
                    return AggregatedClassification {
                        primary: signal.category,
                        confidence: signal.confidence,
                        evidence: vec![signal.clone()],
                        alternatives: vec![],
                    };
                }
            }
        }

        base_result
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
  - `ndarray` (for weight optimization math)
  - `serde_json` (for ground truth corpus)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weight_config_validation() {
        let valid = WeightConfig {
            io_detection: 0.40,
            call_graph: 0.30,
            type_signatures: 0.15,
            purity_side_effects: 0.10,
            framework_patterns: 0.03,
            name_heuristics: 0.02,
        };

        assert!(valid.validate().is_ok());

        let invalid = WeightConfig {
            io_detection: 0.50,
            call_graph: 0.30,
            type_signatures: 0.30,
            purity_side_effects: 0.10,
            framework_patterns: 0.05,
            name_heuristics: 0.05,
        };

        assert!(invalid.validate().is_err());  // Sum > 1.0
    }

    #[test]
    fn utilities_threshold_prevents_overuse() {
        let signals = SignalSet {
            io_signal: Some(IoClassification {
                category: ResponsibilityCategory::FileIO,
                confidence: 0.55,  // Above threshold
                ..Default::default()
            }),
            call_graph_signal: Some(CallGraphClassification {
                category: ResponsibilityCategory::Utilities,
                confidence: 0.45,  // Below threshold
                ..Default::default()
            }),
            ..Default::default()
        };

        let config = AggregationConfig {
            utilities_threshold: 0.50,
            ..Default::default()
        };

        let result = aggregate_with_utilities_threshold(&signals, &config);

        // Should NOT classify as Utilities when FileIO signal is strong enough
        assert_ne!(result.primary, ResponsibilityCategory::Utilities);
        assert_eq!(result.primary, ResponsibilityCategory::FileIO);
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

To optimize weights for your codebase:
```bash
# Create ground truth examples
debtmap ground-truth create --output my_corpus.json

# Optimize weights
debtmap tune-weights --corpus my_corpus.json --output tuned_weights.toml

# Test new weights
debtmap analyze --weights tuned_weights.toml
```
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
