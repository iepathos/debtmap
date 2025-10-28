---
number: 145
title: Multi-Signal Responsibility Aggregation
category: foundation
priority: high
status: draft
dependencies: [141, 142, 143, 144, 147]
created: 2025-10-27
---

# Specification 145: Multi-Signal Responsibility Aggregation

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Specs 141 (I/O Detection), 142 (Call Graph), 143 (Purity), 144 (Frameworks), 147 (Type Signatures)

## Context

Specifications 141-144 and 147 each provide individual **signals** for responsibility classification:

- **Spec 141 (I/O Detection)**: What I/O operations does this function perform?
- **Spec 142 (Call Graph)**: What is this function's role in the call structure?
- **Spec 143 (Purity)**: Is this function pure or impure?
- **Spec 144 (Frameworks)**: Does this match framework patterns?
- **Spec 147 (Type Signatures)**: What do the input/output types suggest?

Each signal provides valuable information, but **no single signal is perfect**. For example:

- I/O detection might classify a function as "File I/O", but miss that it's orchestrating multiple I/O operations
- Call graph might identify an orchestrator, but not realize it's specifically for HTTP request handling
- Framework detection might identify an Axum handler, but not realize it's also doing database operations

**Multi-signal aggregation** combines these signals using weighted voting to achieve higher accuracy than any single signal alone. This is the **integration specification** that ties together the foundation work.

Current accuracy (name-based): ~50%
Target accuracy (multi-signal): **~88%**

## Objective

Implement a weighted multi-signal aggregation system that combines I/O detection, call graph analysis, purity analysis, framework patterns, and type signatures to produce high-accuracy responsibility classifications with confidence scores.

## Requirements

### Functional Requirements

**Signal Collection**:
- Collect all available signals for each function
- Handle missing signals gracefully (not all signals available for all functions)
- Track signal confidence scores
- Preserve evidence from each signal

**Weighted Aggregation**:
- Apply configurable weights to each signal
- Combine signals using weighted voting
- Calculate overall confidence score
- Resolve conflicts between signals

**Default Weight Configuration**:
- I/O Detection: 40% (strongest signal for most code)
- Call Graph Analysis: 30% (structural patterns)
- Type Signatures: 15% (input/output patterns)
- Side Effects/Purity: 10% (complementary to I/O)
- Framework Patterns: 5% (override when present, otherwise low weight)
- Name Heuristics: 5% (fallback only)

**Conflict Resolution**:
- Framework patterns override generic classifications (if confidence > 0.7)
- I/O signals override call graph when contradictory
- Purity signals strengthen computation classifications
- Multiple weak signals can override single strong signal

**Output Format**:
- Primary responsibility (highest-weighted category)
- Overall confidence score (0.0 to 1.0)
- Contributing signals with individual confidences
- Evidence from each signal
- Alternative classifications (if close)

### Non-Functional Requirements

- **Accuracy**: Achieve >85% classification accuracy on test corpus
- **Performance**: Aggregation adds <3% overhead
- **Explainability**: Users can understand why a classification was chosen
- **Configurability**: Weights can be adjusted via configuration file

## Acceptance Criteria

- [ ] Aggregation system collects signals from Specs 141, 142, 143, 144, 147
- [ ] Weighted voting produces primary responsibility classification
- [ ] Confidence scores are computed correctly (0.0-1.0 range)
- [ ] Framework patterns override generic classifications when high confidence
- [ ] Conflicting signals are resolved using weight hierarchy
- [ ] Output includes evidence from each contributing signal
- [ ] Performance overhead <3% compared to single-signal classification
- [ ] Configuration file allows weight adjustments
- [ ] Test corpus achieves >85% accuracy (vs manual ground truth)
- [ ] Explainability output shows signal contributions

## Technical Details

### Implementation Approach

**Phase 1: Signal Collection**

```rust
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SignalSet {
    pub io_signal: Option<IoClassification>,           // Spec 141
    pub call_graph_signal: Option<CallGraphClassification>,  // Spec 142
    pub purity_signal: Option<PurityClassification>,    // Spec 143
    pub framework_signal: Option<FrameworkClassification>,   // Spec 144
    pub type_signal: Option<TypeSignatureClassification>,    // Spec 147
    pub name_signal: Option<NameBasedClassification>,   // Legacy fallback
}

#[derive(Debug, Clone)]
pub struct IoClassification {
    pub category: ResponsibilityCategory,
    pub confidence: f64,
    pub evidence: String,
    pub io_operations: Vec<IoOperation>,
}

#[derive(Debug, Clone)]
pub struct CallGraphClassification {
    pub category: ResponsibilityCategory,
    pub confidence: f64,
    pub evidence: String,
    pub pattern: CallGraphPattern,
}

// Similar for other classification types...

impl SignalSet {
    pub fn collect_for_function(
        function: &FunctionAst,
        context: &AnalysisContext,
    ) -> Self {
        SignalSet {
            io_signal: context.io_analyzer.as_ref().map(|analyzer| {
                analyzer.classify_from_io(function)
            }),
            call_graph_signal: context.call_graph.as_ref().map(|graph| {
                graph.classify_from_structure(function.id)
            }),
            purity_signal: context.purity_analyzer.as_ref().map(|analyzer| {
                analyzer.classify_from_purity(function)
            }),
            framework_signal: context.framework_detector.as_ref().map(|detector| {
                detector.classify_from_framework(function, &context.file_context)
            }),
            type_signal: context.type_analyzer.as_ref().map(|analyzer| {
                analyzer.classify_from_types(function)
            }),
            name_signal: Some(classify_from_name(&function.name)),
        }
    }
}
```

**Phase 2: Weight Configuration**

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AggregationConfig {
    pub weights: SignalWeights,
    pub conflict_resolution: ConflictResolutionStrategy,
    pub minimum_confidence: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SignalWeights {
    pub io_detection: f64,          // Default: 0.40
    pub call_graph: f64,            // Default: 0.30
    pub type_signatures: f64,       // Default: 0.15
    pub purity_side_effects: f64,   // Default: 0.10
    pub framework_patterns: f64,    // Default: 0.05
    pub name_heuristics: f64,       // Default: 0.05
}

impl Default for SignalWeights {
    fn default() -> Self {
        SignalWeights {
            io_detection: 0.40,
            call_graph: 0.30,
            type_signatures: 0.15,
            purity_side_effects: 0.10,
            framework_patterns: 0.05,
            name_heuristics: 0.05,
        }
    }
}

impl SignalWeights {
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
```

**Phase 3: Weighted Aggregation**

```rust
#[derive(Debug, Clone)]
pub struct AggregatedClassification {
    pub primary: ResponsibilityCategory,
    pub confidence: f64,
    pub evidence: Vec<SignalEvidence>,
    pub alternatives: Vec<(ResponsibilityCategory, f64)>,
}

#[derive(Debug, Clone)]
pub struct SignalEvidence {
    pub signal_type: SignalType,
    pub category: ResponsibilityCategory,
    pub confidence: f64,
    pub weight: f64,
    pub contribution: f64,  // confidence * weight
    pub description: String,
}

pub struct ResponsibilityAggregator {
    config: AggregationConfig,
}

impl ResponsibilityAggregator {
    pub fn aggregate(&self, signals: &SignalSet) -> AggregatedClassification {
        // Step 1: Check for high-confidence framework override
        if let Some(ref framework) = signals.framework_signal {
            if framework.confidence >= 0.7 {
                return self.framework_override(framework, signals);
            }
        }

        // Step 2: Collect weighted votes for each category
        let mut category_scores: HashMap<ResponsibilityCategory, f64> = HashMap::new();
        let mut evidence: Vec<SignalEvidence> = Vec::new();

        // I/O Detection (40%)
        if let Some(ref io) = signals.io_signal {
            let contribution = io.confidence * self.config.weights.io_detection;
            *category_scores.entry(io.category).or_insert(0.0) += contribution;

            evidence.push(SignalEvidence {
                signal_type: SignalType::IoDetection,
                category: io.category,
                confidence: io.confidence,
                weight: self.config.weights.io_detection,
                contribution,
                description: io.evidence.clone(),
            });
        }

        // Call Graph (30%)
        if let Some(ref cg) = signals.call_graph_signal {
            let contribution = cg.confidence * self.config.weights.call_graph;
            *category_scores.entry(cg.category).or_insert(0.0) += contribution;

            evidence.push(SignalEvidence {
                signal_type: SignalType::CallGraph,
                category: cg.category,
                confidence: cg.confidence,
                weight: self.config.weights.call_graph,
                contribution,
                description: cg.evidence.clone(),
            });
        }

        // Type Signatures (15%)
        if let Some(ref ts) = signals.type_signal {
            let contribution = ts.confidence * self.config.weights.type_signatures;
            *category_scores.entry(ts.category).or_insert(0.0) += contribution;

            evidence.push(SignalEvidence {
                signal_type: SignalType::TypeSignatures,
                category: ts.category,
                confidence: ts.confidence,
                weight: self.config.weights.type_signatures,
                contribution,
                description: ts.evidence.clone(),
            });
        }

        // Purity/Side Effects (10%)
        if let Some(ref purity) = signals.purity_signal {
            let contribution = purity.confidence * self.config.weights.purity_side_effects;
            *category_scores.entry(purity.category).or_insert(0.0) += contribution;

            evidence.push(SignalEvidence {
                signal_type: SignalType::Purity,
                category: purity.category,
                confidence: purity.confidence,
                weight: self.config.weights.purity_side_effects,
                contribution,
                description: purity.evidence.clone(),
            });
        }

        // Framework Patterns (5%, if present but low confidence)
        if let Some(ref framework) = signals.framework_signal {
            if framework.confidence < 0.7 {  // Already handled high-confidence above
                let contribution = framework.confidence * self.config.weights.framework_patterns;
                *category_scores.entry(framework.category).or_insert(0.0) += contribution;

                evidence.push(SignalEvidence {
                    signal_type: SignalType::Framework,
                    category: framework.category,
                    confidence: framework.confidence,
                    weight: self.config.weights.framework_patterns,
                    contribution,
                    description: framework.evidence.clone(),
                });
            }
        }

        // Name Heuristics (5%, fallback)
        if let Some(ref name) = signals.name_signal {
            let contribution = name.confidence * self.config.weights.name_heuristics;
            *category_scores.entry(name.category).or_insert(0.0) += contribution;

            evidence.push(SignalEvidence {
                signal_type: SignalType::Name,
                category: name.category,
                confidence: name.confidence,
                weight: self.config.weights.name_heuristics,
                contribution,
                description: name.evidence.clone(),
            });
        }

        // Step 3: Select primary and alternatives
        let mut sorted_categories: Vec<_> = category_scores.into_iter().collect();
        sorted_categories.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let (primary_category, primary_score) = sorted_categories[0];
        let alternatives: Vec<_> = sorted_categories.into_iter()
            .skip(1)
            .take(2)  // Top 2 alternatives
            .collect();

        AggregatedClassification {
            primary: primary_category,
            confidence: primary_score,
            evidence,
            alternatives,
        }
    }

    fn framework_override(
        &self,
        framework: &FrameworkClassification,
        signals: &SignalSet,
    ) -> AggregatedClassification {
        // High-confidence framework pattern overrides other signals
        let mut evidence = vec![
            SignalEvidence {
                signal_type: SignalType::Framework,
                category: framework.category,
                confidence: framework.confidence,
                weight: 1.0,  // Full weight for override
                contribution: framework.confidence,
                description: framework.evidence.clone(),
            }
        ];

        // Include other signals as supporting evidence
        if let Some(ref io) = signals.io_signal {
            evidence.push(SignalEvidence {
                signal_type: SignalType::IoDetection,
                category: io.category,
                confidence: io.confidence,
                weight: 0.0,  // Not used in override, but shown for context
                contribution: 0.0,
                description: format!("Also detected: {}", io.evidence),
            });
        }

        AggregatedClassification {
            primary: framework.category,
            confidence: framework.confidence,
            evidence,
            alternatives: vec![],
        }
    }
}
```

**Phase 4: Conflict Resolution**

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ConflictResolutionStrategy {
    /// Use weighted voting (default)
    WeightedVoting,
    /// Framework always wins if present
    FrameworkFirst,
    /// I/O signals override structural signals
    IoFirst,
    /// Highest individual confidence wins
    HighestConfidence,
}

impl ResponsibilityAggregator {
    pub fn resolve_conflict(
        &self,
        signals: &SignalSet,
        strategy: &ConflictResolutionStrategy,
    ) -> ResponsibilityCategory {
        match strategy {
            ConflictResolutionStrategy::WeightedVoting => {
                // Default aggregation approach
                self.aggregate(signals).primary
            }
            ConflictResolutionStrategy::FrameworkFirst => {
                signals.framework_signal
                    .as_ref()
                    .map(|f| f.category)
                    .or_else(|| signals.io_signal.as_ref().map(|i| i.category))
                    .unwrap_or(ResponsibilityCategory::Unknown)
            }
            ConflictResolutionStrategy::IoFirst => {
                signals.io_signal
                    .as_ref()
                    .map(|i| i.category)
                    .or_else(|| signals.call_graph_signal.as_ref().map(|c| c.category))
                    .unwrap_or(ResponsibilityCategory::Unknown)
            }
            ConflictResolutionStrategy::HighestConfidence => {
                let all_signals = vec![
                    signals.io_signal.as_ref().map(|s| (s.category, s.confidence)),
                    signals.call_graph_signal.as_ref().map(|s| (s.category, s.confidence)),
                    signals.framework_signal.as_ref().map(|s| (s.category, s.confidence)),
                    signals.type_signal.as_ref().map(|s| (s.category, s.confidence)),
                ];

                all_signals.into_iter()
                    .flatten()
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                    .map(|(cat, _)| cat)
                    .unwrap_or(ResponsibilityCategory::Unknown)
            }
        }
    }
}
```

### Architecture Changes

**New Module**: `src/analysis/multi_signal_aggregation.rs`
- Signal collection and aggregation
- Weighted voting logic
- Conflict resolution strategies
- Configuration management

**Integration Point**: `src/organization/god_object_analysis.rs`
- Replace `infer_responsibility_from_method()` with multi-signal aggregation
- Preserve backward compatibility during transition
- Add explainability output

**Configuration File**: `aggregation_config.toml`
```toml
[weights]
io_detection = 0.40
call_graph = 0.30
type_signatures = 0.15
purity_side_effects = 0.10
framework_patterns = 0.05
name_heuristics = 0.05

[conflict_resolution]
strategy = "WeightedVoting"
minimum_confidence = 0.30

[framework_override]
enabled = true
min_confidence = 0.70
```

## Dependencies

- **Prerequisites**: All of Specs 141, 142, 143, 144, 147
- **Affected Components**:
  - `src/organization/god_object_analysis.rs` - primary integration point
  - `src/analysis/` - new multi_signal_aggregation module
  - `src/io/formatter.rs` - output formatting for explainability

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_aggregation() {
        let signals = SignalSet {
            io_signal: Some(IoClassification {
                category: ResponsibilityCategory::FileIO,
                confidence: 0.9,
                evidence: "Reads config file".into(),
                io_operations: vec![],
            }),
            call_graph_signal: Some(CallGraphClassification {
                category: ResponsibilityCategory::Orchestration,
                confidence: 0.6,
                evidence: "Calls 5 functions".into(),
                pattern: CallGraphPattern::Orchestrator,
            }),
            purity_signal: Some(PurityClassification {
                category: ResponsibilityCategory::ImpureOperation,
                confidence: 0.8,
                evidence: "Has side effects".into(),
            }),
            framework_signal: None,
            type_signal: None,
            name_signal: None,
        };

        let config = AggregationConfig::default();
        let aggregator = ResponsibilityAggregator::new(config);

        let result = aggregator.aggregate(&signals);

        // I/O has highest weight (0.4) and confidence (0.9)
        assert_eq!(result.primary, ResponsibilityCategory::FileIO);
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn framework_override() {
        let signals = SignalSet {
            io_signal: Some(IoClassification {
                category: ResponsibilityCategory::NetworkIO,
                confidence: 0.8,
                evidence: "HTTP request".into(),
                io_operations: vec![],
            }),
            framework_signal: Some(FrameworkClassification {
                category: ResponsibilityCategory::HttpRequestHandler,
                confidence: 0.95,
                evidence: "Axum handler pattern".into(),
                framework: "Axum".into(),
            }),
            call_graph_signal: None,
            purity_signal: None,
            type_signal: None,
            name_signal: None,
        };

        let config = AggregationConfig::default();
        let aggregator = ResponsibilityAggregator::new(config);

        let result = aggregator.aggregate(&signals);

        // Framework should override with high confidence
        assert_eq!(result.primary, ResponsibilityCategory::HttpRequestHandler);
        assert!(result.confidence >= 0.95);
    }

    #[test]
    fn multiple_weak_signals_override_single() {
        let signals = SignalSet {
            io_signal: Some(IoClassification {
                category: ResponsibilityCategory::PureComputation,
                confidence: 0.5,
                evidence: "No I/O detected".into(),
                io_operations: vec![],
            }),
            call_graph_signal: Some(CallGraphClassification {
                category: ResponsibilityCategory::PureComputation,
                confidence: 0.6,
                evidence: "Leaf node".into(),
                pattern: CallGraphPattern::LeafNode,
            }),
            purity_signal: Some(PurityClassification {
                category: ResponsibilityCategory::PureComputation,
                confidence: 0.7,
                evidence: "Strictly pure".into(),
            }),
            framework_signal: None,
            type_signal: None,
            name_signal: Some(NameBasedClassification {
                category: ResponsibilityCategory::Validation,
                confidence: 0.4,
                evidence: "Name starts with validate_".into(),
            }),
        };

        let config = AggregationConfig::default();
        let aggregator = ResponsibilityAggregator::new(config);

        let result = aggregator.aggregate(&signals);

        // Three medium signals should override one weak signal
        assert_eq!(result.primary, ResponsibilityCategory::PureComputation);
    }
}
```

### Integration Tests

```rust
#[test]
fn end_to_end_classification() {
    let code = r#"
    use axum::{extract::Path, response::Json};

    async fn get_user(Path(user_id): Path<u32>) -> Json<User> {
        let user = database::find_user(user_id).await;
        Json(user)
    }
    "#;

    let ast = parse_rust(code);
    let context = AnalysisContext::new();

    // Collect all signals
    let signals = SignalSet::collect_for_function(&ast.functions[0], &context);

    // Aggregate
    let aggregator = ResponsibilityAggregator::default();
    let result = aggregator.aggregate(&signals);

    // Should be classified as HTTP handler (framework pattern)
    assert_eq!(result.primary, ResponsibilityCategory::HttpRequestHandler);
    assert!(result.confidence > 0.7);

    // Should also note database I/O
    assert!(result.evidence.iter().any(|e| {
        e.signal_type == SignalType::IoDetection &&
        e.description.contains("database")
    }));
}
```

### Accuracy Tests

```rust
#[test]
fn accuracy_on_test_corpus() {
    let test_corpus = load_test_corpus("tests/responsibility_ground_truth.json");

    let aggregator = ResponsibilityAggregator::default();
    let mut correct = 0;
    let mut total = 0;

    for (function, expected_category) in test_corpus {
        let signals = collect_signals(&function);
        let result = aggregator.aggregate(&signals);

        if result.primary == expected_category {
            correct += 1;
        }
        total += 1;
    }

    let accuracy = correct as f64 / total as f64;

    // Target: >85% accuracy
    assert!(accuracy > 0.85, "Accuracy: {:.2}%", accuracy * 100.0);
}
```

## Documentation Requirements

### User Documentation

Update README.md:
```markdown
## Multi-Signal Responsibility Classification

Debtmap uses 6 signals to classify function responsibilities:

1. **I/O Detection (40%)**: Actual I/O operations performed
2. **Call Graph (30%)**: Structural role (orchestrator, leaf, hub)
3. **Type Signatures (15%)**: Input/output type patterns
4. **Purity (10%)**: Side effects and determinism
5. **Framework Patterns (5%)**: Framework-specific idioms
6. **Name Heuristics (5%)**: Fallback signal

**Classification Accuracy**: ~88% (vs 50% with name-based alone)

**Explainability**: Each classification shows:
- Primary responsibility with confidence score
- Contributing signals and their weights
- Evidence from each signal
- Alternative classifications
```

### Architecture Updates

Update ARCHITECTURE.md:
```markdown
## Multi-Signal Classification Pipeline (Spec 145)

1. **Signal Collection**: Gather I/O, call graph, purity, framework, type, name signals
2. **Weighted Aggregation**: Combine using configured weights
3. **Conflict Resolution**: Handle disagreements between signals
4. **Output**: Primary classification + confidence + evidence
```

## Implementation Notes

### Tuning Weights

Weights can be tuned based on empirical accuracy:

```bash
# Run weight optimization
cargo run --bin tune-weights -- \
  --corpus tests/responsibility_ground_truth.json \
  --output tuned_weights.toml
```

### Explainability Format

```
Classification: HTTP Request Handler
Confidence: 0.87

Contributing Signals:
  ✓ Framework Pattern (95% confidence, 0.05 weight) = 0.048
    Evidence: Matches Axum handler pattern with async fn and Path extractor

  ✓ I/O Detection (80% confidence, 0.40 weight) = 0.320
    Evidence: Performs database query operation

  ✓ Call Graph (60% confidence, 0.30 weight) = 0.180
    Evidence: Calls 3 functions, orchestration pattern

  • Purity (N/A) - Not computed
  • Type Signatures (N/A) - Not computed
  • Name Heuristics (40% confidence, 0.05 weight) = 0.020

Total Score: 0.568 (rounded to 0.87 confidence)

Alternative Classifications:
  - Database Operation (0.45)
  - Orchestration (0.32)
```

## Migration and Compatibility

### Gradual Rollout

1. **Phase 1**: Implement aggregation without changing output
   - Run multi-signal in parallel with existing classification
   - Log differences for analysis

2. **Phase 2**: Add confidence scores to output
   - Show both old and new classifications
   - Flag low-confidence classifications for review

3. **Phase 3**: Switch to multi-signal as primary
   - Use aggregation for all classifications
   - Preserve backward compatibility flag

## Expected Impact

### Accuracy Improvement

| Approach | Accuracy | Improvement |
|----------|----------|-------------|
| Name-based (current) | ~50% | Baseline |
| + I/O Detection (Spec 141) | ~70% | +20% |
| + Call Graph (Spec 142) | ~80% | +30% |
| + Purity (Spec 143) | ~82% | +32% |
| + Framework (Spec 144) | ~85% | +35% |
| **+ Multi-Signal (Spec 145)** | **~88%** | **+38%** |

### Real-World Example

```rust
// Function from debtmap codebase
fn analyze_file_complexity(path: &Path) -> Result<FileMetrics> {
    let content = std::fs::read_to_string(path)?;
    let ast = parse_file(&content, path)?;
    let metrics = calculate_metrics(&ast);
    Ok(metrics)
}

// Old classification (name-based): "Analysis"
// New classification (multi-signal):
//   Primary: File I/O & Parsing (0.82 confidence)
//   Signals:
//     - I/O: File read operation (0.9)
//     - Call Graph: Orchestrates 3 functions (0.7)
//     - Purity: Impure due to I/O (0.8)
//     - Name: Contains "analyze" (0.5)
```

## Success Metrics

- **Accuracy**: >85% on test corpus
- **Confidence**: Average confidence >0.70
- **Explainability**: Users can understand classifications
- **Performance**: <3% overhead vs single-signal
- **Adoption**: Used for all responsibility classifications by default
