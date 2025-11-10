---
number: 174
title: Strict Utilities Fallback with Confidence Thresholds
category: optimization
priority: high
status: draft
dependencies: [173, 150]
created: 2025-11-10
---

# Specification 174: Strict Utilities Fallback with Confidence Thresholds

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Spec 173 (Simplify Responsibility Names), Spec 150 (Weight Tuning)

## Context

The current responsibility classification system uses "Utilities" as an unconditional fallback when no prefix matches, leading to over-classification:

**Current Code** (`src/organization/god_object_analysis.rs:983`):
```rust
pub fn infer_responsibility_from_method(method_name: &str) -> String {
    let lower = method_name.to_lowercase();

    RESPONSIBILITY_CATEGORIES
        .iter()
        .find(|cat| cat.matches(&lower))
        .map(|cat| cat.name)
        .unwrap_or("Utilities")  // ← Always fallback to "Utilities"
        .to_string()
}
```

**Problem**: This leads to ~30% of methods being classified as "Utilities" (per Spec 150):
```
- mod_utilities.rs - Utilities (40 methods, ~800 lines)
- semantic_classifier_utilities.rs - Utilities (62 methods, ~1240 lines)
```

Many of these methods **have clear responsibilities** but:
1. Don't match any prefix patterns (e.g., `populate_observer_registry`)
2. Should use multi-signal classification, not just name heuristics
3. Need higher confidence thresholds before accepting classification

## Objective

Replace unconditional "Utilities" fallback with confidence-based classification that:
1. Only assigns "Utilities" when **all signals are weak** (confidence < threshold)
2. Requires **minimum confidence** for any classification
3. Integrates with **multi-signal aggregation** (Spec 145)
4. Reduces "Utilities" prevalence from ~30% to <10%

## Requirements

### Functional Requirements

**FR1: Confidence-Based Classification**

```rust
pub struct ClassificationResult {
    pub category: Option<String>,  // None if confidence too low
    pub confidence: f64,            // 0.0 to 1.0
    pub signals_used: Vec<SignalType>,
}

pub fn classify_with_confidence(
    method_name: &str,
    method_body: Option<&str>,
    signals: &SignalSet,
) -> ClassificationResult {
    // Multi-signal aggregation (Spec 145)
    let aggregated = aggregate_signals(signals);

    // Threshold check
    if aggregated.confidence < MINIMUM_CONFIDENCE {
        return ClassificationResult {
            category: None,  // Refuse to classify
            confidence: aggregated.confidence,
            signals_used: aggregated.signals_used,
        };
    }

    // Only use "Utilities" if explicitly detected (not default)
    if aggregated.category == "utilities" && aggregated.confidence < UTILITIES_CONFIDENCE {
        return ClassificationResult {
            category: None,
            confidence: aggregated.confidence,
            signals_used: aggregated.signals_used,
        };
    }

    ClassificationResult {
        category: Some(aggregated.category),
        confidence: aggregated.confidence,
        signals_used: aggregated.signals_used,
    }
}
```

**FR2: Confidence Thresholds**

| Classification | Minimum Confidence | Rationale |
|---|---|---|
| Any category | 0.50 | Base threshold - require more signal than noise |
| "utilities" | 0.60 | Higher bar - avoid lazy fallback |
| Module split recommendation | 0.65 | High bar - structural changes need confidence |

**FR3: Multi-Signal Integration**

Classification must consider (per Spec 145):
1. **I/O detection** (weight: 0.40) - Detects file/network/console I/O
2. **Call graph** (weight: 0.30) - Who calls this function?
3. **Type signatures** (weight: 0.15) - Return type patterns
4. **Purity/side effects** (weight: 0.10) - Pure vs. impure
5. **Name heuristics** (weight: 0.05) - Prefix matching (current approach)

**FR4: Fallback Strategy**

When confidence < threshold:
1. **Don't create module split** - Keep method in original location
2. **Log low-confidence classifications** - For weight tuning analysis
3. **Suggest manual review** - In verbose output mode
4. **Track unclassified methods** - For corpus improvement

### Non-Functional Requirements

**NFR1: Accuracy**
- Overall classification accuracy ≥85% (on test corpus from Spec 150)
- "Utilities" classification rate ≤10% (down from ~30%)
- False positive rate <5% per category

**NFR2: Observability**
- Log all low-confidence classifications (confidence < 0.60)
- Track signal weights used per classification
- Emit metrics for weight tuning (Spec 150)

**NFR3: Performance**
- Classification overhead <10% of total god object detection time
- Cache signal computations where possible
- Lazy evaluation of expensive signals (call graph, I/O detection)

## Acceptance Criteria

- [ ] Classification returns `Option<String>` instead of always returning a value
- [ ] Minimum confidence threshold enforced (0.50)
- [ ] "Utilities" requires higher confidence (0.60)
- [ ] Multi-signal aggregation integrated (Spec 145)
- [ ] "Utilities" classification rate reduced to <10%
- [ ] Overall accuracy maintained at ≥85%
- [ ] Low-confidence classifications logged for analysis
- [ ] Module splits only created when confidence ≥0.65
- [ ] All existing tests updated to handle `Option<String>`
- [ ] Integration tests validate threshold behavior
- [ ] Performance overhead measured at <10%
- [ ] Documentation updated with confidence thresholds

## Technical Details

### Implementation Approach

**Phase 1: Refactor Return Type**

```rust
// src/organization/god_object_analysis.rs

/// Classify method with confidence scoring
pub fn infer_responsibility_with_confidence(
    method_name: &str,
    method_body: Option<&str>,
    language: Language,
) -> ClassificationResult {
    use crate::analysis::multi_signal_aggregation::*;

    // Build signal set
    let signals = SignalSet {
        name_signal: Some(collect_name_signal(method_name)),
        io_signal: method_body.map(|body| collect_io_signal(body, language)),
        purity_signal: method_body.map(|body| collect_purity_signal(body, language)),
        type_signal: None,  // Requires full signature
        call_graph_signal: None,  // Requires call graph context
        framework_signal: None,  // Requires file context
    };

    // Aggregate signals
    let aggregator = ResponsibilityAggregator::new();
    let aggregated = aggregator.aggregate(&signals);

    // Apply confidence thresholds
    apply_confidence_thresholds(aggregated)
}

fn apply_confidence_thresholds(
    classification: AggregatedClassification
) -> ClassificationResult {
    // Check minimum confidence
    if classification.confidence < MINIMUM_CONFIDENCE {
        return ClassificationResult {
            category: None,
            confidence: classification.confidence,
            signals_used: extract_signal_types(&classification),
        };
    }

    // Special handling for "Utilities"
    if classification.primary == ResponsibilityCategory::Utilities {
        if classification.confidence < UTILITIES_THRESHOLD {
            return ClassificationResult {
                category: None,
                confidence: classification.confidence,
                signals_used: extract_signal_types(&classification),
            };
        }
    }

    ClassificationResult {
        category: Some(classification.primary.as_str().to_string()),
        confidence: classification.confidence,
        signals_used: extract_signal_types(&classification),
    }
}
```

**Phase 2: Update Call Sites**

```rust
// src/organization/god_object_analysis.rs:544
pub fn group_methods_by_responsibility(
    methods: &[MethodInfo],
    language: Language,
) -> HashMap<String, Vec<String>> {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    let mut unclassified: Vec<String> = Vec::new();

    for method in methods {
        let result = infer_responsibility_with_confidence(
            &method.name,
            method.body.as_deref(),
            language,
        );

        match result.category {
            Some(category) => {
                groups.entry(category).or_default().push(method.name.clone());
            }
            None => {
                // Low confidence - don't create module split
                unclassified.push(method.name.clone());

                if log::log_enabled!(log::Level::Debug) {
                    log::debug!(
                        "Low confidence classification: {} (confidence: {:.2})",
                        method.name,
                        result.confidence
                    );
                }
            }
        }
    }

    // Keep unclassified methods in original file
    groups.entry("core".to_string()).or_default().extend(unclassified);

    groups
}
```

**Phase 3: Module Split Filtering**

```rust
// src/organization/god_object_analysis.rs:1067
pub fn recommend_module_splits_with_evidence(
    type_name: &str,
    methods: &[MethodInfo],
    responsibility_groups: &HashMap<String, Vec<ClassifiedMethod>>,
) -> Vec<ModuleSplit> {
    let mut recommendations = Vec::new();

    for (responsibility, classified_methods) in responsibility_groups {
        // Skip if too few methods
        if classified_methods.len() < MIN_METHODS_FOR_SPLIT {
            continue;
        }

        // Calculate aggregate confidence
        let avg_confidence: f64 = classified_methods
            .iter()
            .map(|m| m.classification.confidence)
            .sum::<f64>() / classified_methods.len() as f64;

        // Only recommend split if confidence high enough
        if avg_confidence < MODULE_SPLIT_CONFIDENCE {
            log::info!(
                "Skipping module split for '{}': low confidence ({:.2})",
                responsibility,
                avg_confidence
            );
            continue;
        }

        // Generate recommendation with confidence metadata
        recommendations.push(ModuleSplit {
            suggested_name: format!(
                "{}_{}",
                type_name.to_lowercase(),
                sanitize_module_name(responsibility)
            ),
            methods_to_move: classified_methods.iter().map(|m| m.name.clone()).collect(),
            cohesion_score: Some(avg_confidence),
            warning: generate_confidence_warning(avg_confidence),
            // ... rest of fields
        });
    }

    recommendations
}

fn generate_confidence_warning(confidence: f64) -> Option<String> {
    if confidence < 0.70 {
        Some(format!(
            "Low confidence ({:.0}%) - manual review recommended",
            confidence * 100.0
        ))
    } else {
        None
    }
}
```

### Confidence Threshold Constants

```rust
// src/organization/confidence.rs (new module)

/// Minimum confidence for any classification
pub const MINIMUM_CONFIDENCE: f64 = 0.50;

/// Minimum confidence for "Utilities" classification (higher bar)
pub const UTILITIES_THRESHOLD: f64 = 0.60;

/// Minimum confidence for module split recommendation
pub const MODULE_SPLIT_CONFIDENCE: f64 = 0.65;

/// Number of methods required for module split
pub const MIN_METHODS_FOR_SPLIT: usize = 5;
```

### Logging and Observability

```rust
#[derive(Debug, Serialize)]
pub struct ClassificationMetrics {
    pub total_methods: usize,
    pub classified: usize,
    pub unclassified: usize,
    pub utilities_count: usize,
    pub avg_confidence: f64,
    pub low_confidence_methods: Vec<LowConfidenceMethod>,
}

#[derive(Debug, Serialize)]
pub struct LowConfidenceMethod {
    pub name: String,
    pub confidence: f64,
    pub signals_used: Vec<String>,
    pub alternatives: Vec<(String, f64)>,
}

pub fn emit_classification_metrics(metrics: &ClassificationMetrics) {
    log::info!("Classification metrics:");
    log::info!("  Total methods: {}", metrics.total_methods);
    log::info!("  Classified: {} ({:.1}%)",
        metrics.classified,
        100.0 * metrics.classified as f64 / metrics.total_methods as f64
    );
    log::info!("  Utilities: {} ({:.1}%)",
        metrics.utilities_count,
        100.0 * metrics.utilities_count as f64 / metrics.total_methods as f64
    );
    log::info!("  Avg confidence: {:.2}", metrics.avg_confidence);

    if !metrics.low_confidence_methods.is_empty() {
        log::debug!("Low confidence methods:");
        for method in &metrics.low_confidence_methods {
            log::debug!("  - {} ({:.2})", method.name, method.confidence);
        }
    }
}
```

## Dependencies

**Prerequisites**:
- Spec 173 (Simplify Responsibility Names) - Cleaner category names
- Spec 150 (Weight Tuning) - Signal weights and test corpus

**Affected Components**:
- `src/organization/god_object_analysis.rs` - Core classification logic
- `src/organization/module_function_classifier.rs` - Module splits
- `src/analysis/multi_signal_aggregation.rs` - Signal aggregation (Spec 145)
- `src/priority/formatter.rs` - Display confidence warnings

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_minimum_confidence_threshold() {
    let result = apply_confidence_thresholds(AggregatedClassification {
        primary: ResponsibilityCategory::Validation,
        confidence: 0.45,  // Below minimum
        evidence: vec![],
        alternatives: vec![],
    });

    assert!(result.category.is_none());
    assert_eq!(result.confidence, 0.45);
}

#[test]
fn test_utilities_requires_higher_confidence() {
    let result = apply_confidence_thresholds(AggregatedClassification {
        primary: ResponsibilityCategory::Utilities,
        confidence: 0.55,  // Above minimum but below utilities threshold
        evidence: vec![],
        alternatives: vec![],
    });

    assert!(result.category.is_none());
}

#[test]
fn test_high_confidence_accepted() {
    let result = apply_confidence_thresholds(AggregatedClassification {
        primary: ResponsibilityCategory::Parsing,
        confidence: 0.85,
        evidence: vec![],
        alternatives: vec![],
    });

    assert_eq!(result.category, Some("parsing".to_string()));
}
```

### Integration Tests

**Test: Utilities Reduction**
```rust
#[test]
fn test_utilities_classification_reduced() {
    let methods = load_test_corpus("tests/data/python_type_tracker_methods.json");

    let mut utilities_count = 0;
    let mut total_count = 0;

    for method in methods {
        let result = infer_responsibility_with_confidence(
            &method.name,
            Some(&method.body),
            Language::Rust,
        );

        if let Some(category) = result.category {
            if category == "utilities" {
                utilities_count += 1;
            }
            total_count += 1;
        }
    }

    let utilities_rate = utilities_count as f64 / total_count as f64;
    assert!(
        utilities_rate < 0.10,
        "Utilities rate too high: {:.1}%",
        utilities_rate * 100.0
    );
}
```

**Test: Module Splits Only for High Confidence**
```rust
#[test]
fn test_module_splits_require_high_confidence() {
    let splits = recommend_module_splits_with_evidence(/* ... */);

    for split in splits {
        let confidence = split.cohesion_score.expect("Missing confidence");
        assert!(
            confidence >= MODULE_SPLIT_CONFIDENCE,
            "Split '{}' has low confidence: {:.2}",
            split.suggested_name,
            confidence
        );
    }
}
```

### Regression Tests

```rust
#[test]
fn test_accuracy_maintained() {
    let corpus = load_test_corpus("tests/data/ground_truth.json");

    let mut correct = 0;
    let mut total = 0;

    for sample in corpus {
        let result = infer_responsibility_with_confidence(
            &sample.method_name,
            Some(&sample.body),
            Language::Rust,
        );

        if let Some(category) = result.category {
            if category == sample.expected_category {
                correct += 1;
            }
            total += 1;
        }
    }

    let accuracy = correct as f64 / total as f64;
    assert!(accuracy >= 0.85, "Accuracy too low: {:.1}%", accuracy * 100.0);
}
```

## Documentation Requirements

**Code Documentation**:
- Document confidence threshold constants
- Explain multi-signal aggregation integration
- Add examples of low-confidence scenarios

**User Documentation**:
- Update god object detection guide
- Explain confidence-based classification
- Show how to interpret confidence warnings
- Document flags to adjust thresholds

**Configuration**:
```toml
# debtmap.toml
[god_object]
minimum_confidence = 0.50
utilities_threshold = 0.60
module_split_confidence = 0.65
min_methods_for_split = 5
```

## Implementation Notes

**Migration Strategy**:
1. Add new confidence-based functions alongside old ones
2. Update tests incrementally
3. Switch callers one by one
4. Remove old unconditional fallback
5. Deploy with feature flag initially

**Validation**:
- A/B test with Spec 150 corpus
- Measure "Utilities" rate reduction
- Ensure no accuracy regression
- Monitor performance impact

**Tuning**:
- Start with conservative thresholds
- Tune based on real-world results
- Use weight tuning system (Spec 150)
- Collect feedback from users

## Migration and Compatibility

**Breaking Changes**: Yes - API changes

**Deprecation Path**:
```rust
#[deprecated(since = "0.4.0", note = "Use infer_responsibility_with_confidence")]
pub fn infer_responsibility_from_method(method_name: &str) -> String {
    infer_responsibility_with_confidence(method_name, None, Language::Rust)
        .category
        .unwrap_or_else(|| "utilities".to_string())
}
```

**Configuration Migration**:
- Old configs use default thresholds
- New configs can customize thresholds
- Log warnings for deprecated functions

## Success Metrics

- "Utilities" classification rate reduced from ~30% to <10%
- Overall accuracy maintained at ≥85%
- Zero false positives in high-confidence classifications
- Module split recommendations have avg confidence ≥0.70
- Performance overhead ≤10% of god object detection time
- User satisfaction with recommendation quality improved
