---
number: 215
title: Functional Decomposition Recognition in God Object Detection
category: optimization
priority: high
status: draft
dependencies: [208, 213]
created: 2025-12-15
---

# Specification 215: Functional Decomposition Recognition in God Object Detection

**Category**: optimization
**Priority**: high (P0)
**Status**: draft
**Dependencies**: Spec 208 (Domain-Aware Grouping), Spec 213 (Pure Function Weighting)

## Context

The current God Object detection counts method count as a primary signal, but doesn't recognize that **functional decomposition** (breaking logic into many small, composable functions) is a best practice, not a code smell.

### Current Problem

The detection heuristics assume:
- More methods = more responsibilities = more likely god object
- This holds for **imperative OOP** patterns where methods accumulate capabilities

But this fails for **functional programming** patterns:
- Many small pure functions = well-decomposed code
- Functions compose via pipelines, not inheritance
- Single responsibility achieved through composition

```rust
// Functional decomposition pattern - GOOD CODE
pub struct CallResolver<'a> {
    call_graph: &'a CallGraph,
    current_file: &'a PathBuf,
    function_index: HashMap<String, Vec<FunctionId>>,
}

impl<'a> CallResolver<'a> {
    // 3 methods that coordinate (orchestrators)
    pub fn new(...) -> Self { ... }
    pub fn resolve_call(&self, call: &UnresolvedCall) -> Option<FunctionId> {
        // Composes pure functions via pipeline
        let candidates = self.find_candidates(call)?;
        candidates
            .pipe(|c| Self::apply_same_file_preference(c, self.current_file))
            .pipe(Self::apply_qualification_preference)
            .pipe(Self::apply_generic_preference)
            .into_iter().next()
    }

    // 21 pure helper functions (functional building blocks)
    fn normalize_path_prefix(name: &str) -> String { ... }
    fn strip_generic_params(name: &str) -> String { ... }
    fn is_exact_match(a: &str, b: &str) -> bool { ... }
    fn is_qualified_match(a: &str, b: &str) -> bool { ... }
    fn apply_same_file_preference(candidates: Vec<T>, file: &Path) -> Vec<T> { ... }
    fn apply_qualification_preference(candidates: Vec<T>) -> Vec<T> { ... }
    fn apply_generic_preference(candidates: Vec<T>) -> Vec<T> { ... }
    // ... etc
}
```

This is **exemplary functional design**:
- Single responsibility: resolve function calls
- Small, testable pure functions
- Composition via `.pipe()` chains
- Clear separation of orchestration vs computation

But current metrics flag it as critical debt due to method count alone.

### Recognition Problem

The recommendation "Extract 5 sub-orchestrators to reduce coordination complexity" is wrong because:
1. There IS only one orchestrator (`resolve_call`)
2. The pure helpers aren't orchestrators - they're composable building blocks
3. Splitting would reduce cohesion without benefit

## Objective

Implement recognition of functional decomposition patterns to:
1. Identify structs following functional programming principles
2. Apply significant score reduction for well-decomposed functional code
3. Generate appropriate recommendations (or no recommendation if well-designed)

## Requirements

### Functional Requirements

1. **Functional Pattern Detection**: Identify structs with functional decomposition:
   - High ratio of pure/associated methods to instance methods (>70%)
   - Methods compose via explicit function calls (not deep inheritance)
   - Single orchestrator pattern (1-3 coordinating methods)
   - Many small helper functions (<10 lines each)

2. **Composition Detection**: Identify functional composition patterns:
   - `.pipe()` chains
   - Iterator method chains (`.map().filter().collect()`)
   - Direct function composition (`f(g(x))`)
   - Builder patterns that accumulate transformations

3. **Scoring Adjustment**:
   - Functional decomposition bonus: 0.3x multiplier (70% reduction)
   - Applied when functional pattern score > 0.7
   - Stacks with pure method weighting (Spec 213)

4. **Recommendation Override**:
   - When functional decomposition detected, suppress "extract sub-orchestrators"
   - Instead show: "Well-structured functional design - no action needed"
   - Or provide functional-appropriate suggestions if issues found

### Non-Functional Requirements

- Detection must be based on structural patterns, not just naming
- Should work for multiple functional styles (pure FP, Rust patterns, etc.)
- Must produce deterministic results

## Acceptance Criteria

- [ ] Structs with >70% pure methods and <3 orchestrators are detected
- [ ] Functional decomposition bonus (0.3x) is applied
- [ ] `CallResolver` gets bonus, scoring < 20 instead of 100
- [ ] "Extract sub-orchestrators" recommendation is suppressed
- [ ] Appropriate message shown: "Functional decomposition detected"
- [ ] Actual god objects (low pure %, high orchestrator count) still flagged
- [ ] Tests validate functional pattern detection accuracy

## Technical Details

### Implementation Approach

#### 1. Functional Pattern Metrics

```rust
/// Metrics for detecting functional decomposition pattern
#[derive(Debug, Clone)]
pub struct FunctionalDecompositionMetrics {
    /// Ratio of pure methods to total methods
    pub pure_method_ratio: f64,
    /// Number of methods that coordinate/orchestrate
    pub orchestrator_count: usize,
    /// Number of pure helper methods
    pub pure_helper_count: usize,
    /// Average lines per pure method
    pub avg_pure_method_loc: f64,
    /// Detected composition patterns
    pub composition_patterns: Vec<CompositionPattern>,
    /// Overall functional pattern score (0.0 - 1.0)
    pub functional_score: f64,
}

#[derive(Debug, Clone)]
pub enum CompositionPattern {
    /// .pipe() method chains
    PipeChain { length: usize },
    /// Iterator method chains
    IteratorChain { length: usize },
    /// Direct function composition f(g(x))
    DirectComposition { depth: usize },
    /// Builder pattern accumulation
    BuilderPattern,
}
```

#### 2. Detection Logic

```rust
/// Detect functional decomposition in a struct
pub fn detect_functional_decomposition(
    methods: &[MethodAnalysis],
    call_graph: &MethodCallGraph,
) -> FunctionalDecompositionMetrics {
    // Count pure vs instance methods
    let pure_count = methods.iter()
        .filter(|m| m.self_usage == MethodSelfUsage::PureAssociated)
        .count();
    let instance_count = methods.len() - pure_count;

    let pure_method_ratio = if methods.is_empty() {
        0.0
    } else {
        pure_count as f64 / methods.len() as f64
    };

    // Identify orchestrators (instance methods that call multiple other methods)
    let orchestrator_count = methods.iter()
        .filter(|m| {
            m.self_usage == MethodSelfUsage::InstanceMethod &&
            call_graph.outgoing_calls(&m.name).len() >= 2
        })
        .count();

    // Calculate average LOC for pure methods
    let pure_methods: Vec<_> = methods.iter()
        .filter(|m| m.self_usage == MethodSelfUsage::PureAssociated)
        .collect();
    let avg_pure_method_loc = if pure_methods.is_empty() {
        0.0
    } else {
        pure_methods.iter().map(|m| m.loc as f64).sum::<f64>()
            / pure_methods.len() as f64
    };

    // Detect composition patterns
    let composition_patterns = detect_composition_patterns(methods);

    // Calculate functional score
    let functional_score = calculate_functional_score(
        pure_method_ratio,
        orchestrator_count,
        avg_pure_method_loc,
        &composition_patterns,
    );

    FunctionalDecompositionMetrics {
        pure_method_ratio,
        orchestrator_count,
        pure_helper_count: pure_count,
        avg_pure_method_loc,
        composition_patterns,
        functional_score,
    }
}

/// Calculate functional decomposition score
fn calculate_functional_score(
    pure_ratio: f64,
    orchestrator_count: usize,
    avg_pure_loc: f64,
    patterns: &[CompositionPattern],
) -> f64 {
    let mut score = 0.0;

    // High pure method ratio is strong signal (0.0 - 0.4)
    score += pure_ratio * 0.4;

    // Few orchestrators is good (0.0 - 0.2)
    let orchestrator_score = match orchestrator_count {
        0..=2 => 0.2,
        3..=5 => 0.1,
        _ => 0.0,
    };
    score += orchestrator_score;

    // Small average pure method size is good (0.0 - 0.2)
    let size_score = if avg_pure_loc <= 5.0 {
        0.2
    } else if avg_pure_loc <= 10.0 {
        0.15
    } else if avg_pure_loc <= 15.0 {
        0.1
    } else {
        0.0
    };
    score += size_score;

    // Composition patterns detected (0.0 - 0.2)
    let pattern_score = if !patterns.is_empty() {
        0.2_f64.min(patterns.len() as f64 * 0.05)
    } else {
        0.0
    };
    score += pattern_score;

    score.min(1.0)
}
```

#### 3. Composition Pattern Detection

```rust
/// Detect composition patterns in method bodies
fn detect_composition_patterns(methods: &[MethodAnalysis]) -> Vec<CompositionPattern> {
    let mut patterns = Vec::new();

    for method in methods {
        // Check for .pipe() chains
        if let Some(chain_length) = detect_pipe_chain(&method.body_ast) {
            if chain_length >= 2 {
                patterns.push(CompositionPattern::PipeChain { length: chain_length });
            }
        }

        // Check for iterator chains
        if let Some(chain_length) = detect_iterator_chain(&method.body_ast) {
            if chain_length >= 3 {
                patterns.push(CompositionPattern::IteratorChain { length: chain_length });
            }
        }

        // Check for direct composition
        if let Some(depth) = detect_direct_composition(&method.body_ast) {
            if depth >= 2 {
                patterns.push(CompositionPattern::DirectComposition { depth });
            }
        }
    }

    patterns
}

/// Detect .pipe() or similar functional chain patterns
fn detect_pipe_chain(body: &syn::Block) -> Option<usize> {
    struct PipeChainVisitor {
        max_chain: usize,
        current_chain: usize,
    }

    impl<'ast> Visit<'ast> for PipeChainVisitor {
        fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
            // Check for .pipe() method name
            if call.method == "pipe" {
                self.current_chain += 1;
                self.max_chain = self.max_chain.max(self.current_chain);
            } else {
                self.current_chain = 0;
            }
            syn::visit::visit_expr_method_call(self, call);
        }
    }

    let mut visitor = PipeChainVisitor { max_chain: 0, current_chain: 0 };
    visitor.visit_block(body);

    if visitor.max_chain > 0 {
        Some(visitor.max_chain)
    } else {
        None
    }
}
```

#### 4. Scoring Integration

```rust
/// Calculate final god object score with functional bonus
pub fn calculate_god_object_score_with_functional_bonus(
    base_score: f64,
    functional_metrics: &FunctionalDecompositionMetrics,
) -> f64 {
    // Apply functional decomposition bonus if score is high enough
    if functional_metrics.functional_score >= 0.7 {
        // Strong functional pattern: 70% reduction
        base_score * 0.3
    } else if functional_metrics.functional_score >= 0.5 {
        // Moderate functional pattern: 50% reduction
        base_score * 0.5
    } else if functional_metrics.functional_score >= 0.3 {
        // Weak functional pattern: 25% reduction
        base_score * 0.75
    } else {
        // No functional pattern: no bonus
        base_score
    }
}
```

#### 5. Recommendation Override

```rust
/// Generate recommendation considering functional decomposition
pub fn generate_recommendation_with_functional_awareness(
    analysis: &GodObjectAnalysis,
    functional_metrics: &FunctionalDecompositionMetrics,
) -> GodObjectRecommendation {
    // If strong functional pattern, override default recommendation
    if functional_metrics.functional_score >= 0.7 {
        return GodObjectRecommendation {
            action: RecommendationAction::NoActionNeeded,
            rationale: format!(
                "Well-structured functional design detected: {} pure helpers composing into {} orchestrator(s). \
                 This pattern is intentional decomposition, not a god object.",
                functional_metrics.pure_helper_count,
                functional_metrics.orchestrator_count,
            ),
            suggested_extractions: vec![],
        };
    }

    // If moderate functional pattern but some issues, provide tailored advice
    if functional_metrics.functional_score >= 0.5 {
        // Check if there are actual issues worth addressing
        if analysis.responsibility_count > 3 {
            return GodObjectRecommendation {
                action: RecommendationAction::ConsiderRefactoring,
                rationale: format!(
                    "Partial functional decomposition detected ({}% pure methods), \
                     but {} distinct responsibilities suggest some grouping could help.",
                    (functional_metrics.pure_method_ratio * 100.0) as usize,
                    analysis.responsibility_count,
                ),
                suggested_extractions: suggest_responsibility_based_grouping(analysis),
            };
        } else {
            return GodObjectRecommendation {
                action: RecommendationAction::NoActionNeeded,
                rationale: "Functional decomposition with focused responsibilities".to_string(),
                suggested_extractions: vec![],
            };
        }
    }

    // Default to standard recommendation generation
    generate_standard_recommendation(analysis)
}
```

#### 6. Output Format

```
evaluation location
  file                      ./src/analyzers/call_graph/call_resolution.rs
  function                  CallResolver
  line                      53

score
  total                     18.5 [low]
  base                      92.0 (before functional bonus)
  functional bonus          0.3x applied

functional decomposition
  pattern detected          Yes (score: 0.82)
  pure methods              21 (87.5%)
  orchestrators             3
  avg helper size           8.2 lines
  composition patterns      PipeChain(3), IteratorChain(5)

recommendation
  action                    No action needed
  rationale                 Well-structured functional design: 21 pure helpers
                            composing into 3 orchestrators. This pattern is
                            intentional decomposition, not a god object.
```

### Detection Thresholds

| Metric | Threshold | Weight |
|--------|-----------|--------|
| Pure method ratio | >= 70% | 0.4 |
| Orchestrator count | <= 3 | 0.2 |
| Avg pure method LOC | <= 10 | 0.2 |
| Composition patterns | >= 1 | 0.2 |

| Functional Score | Interpretation | Multiplier |
|-----------------|----------------|------------|
| >= 0.7 | Strong functional design | 0.3x |
| >= 0.5 | Moderate functional style | 0.5x |
| >= 0.3 | Some functional elements | 0.75x |
| < 0.3 | Traditional OOP style | 1.0x |

## Dependencies

- **Prerequisites**:
  - Spec 213: Pure function method weighting (provides pure method detection)
  - Spec 208: Domain-aware grouping (provides responsibility analysis)
- **Affected Components**:
  - `detector.rs`: Add functional pattern detection
  - `recommendation_generator.rs`: Override recommendations
  - `scoring.rs`: Apply functional bonus
  - `types.rs`: Add FunctionalDecompositionMetrics

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_detect_high_functional_score() {
    let methods = vec![
        MethodAnalysis { name: "new".into(), self_usage: MethodSelfUsage::InstanceMethod, loc: 5, .. },
        MethodAnalysis { name: "resolve".into(), self_usage: MethodSelfUsage::InstanceMethod, loc: 20, .. },
        // 18 pure helpers
        MethodAnalysis { name: "is_match".into(), self_usage: MethodSelfUsage::PureAssociated, loc: 3, .. },
        MethodAnalysis { name: "normalize".into(), self_usage: MethodSelfUsage::PureAssociated, loc: 5, .. },
        // ... etc
    ];

    let metrics = detect_functional_decomposition(&methods, &call_graph);

    assert!(metrics.pure_method_ratio >= 0.7);
    assert!(metrics.orchestrator_count <= 3);
    assert!(metrics.functional_score >= 0.7);
}

#[test]
fn test_functional_bonus_application() {
    let base_score = 100.0;
    let high_functional = FunctionalDecompositionMetrics {
        functional_score: 0.8,
        ..Default::default()
    };

    let adjusted = calculate_god_object_score_with_functional_bonus(base_score, &high_functional);
    assert_eq!(adjusted, 30.0);  // 100 * 0.3
}

#[test]
fn test_recommendation_override() {
    let analysis = GodObjectAnalysis {
        method_count: 24,
        responsibility_count: 7,
        ..Default::default()
    };

    let functional_metrics = FunctionalDecompositionMetrics {
        functional_score: 0.8,
        pure_helper_count: 21,
        orchestrator_count: 3,
        ..Default::default()
    };

    let recommendation = generate_recommendation_with_functional_awareness(&analysis, &functional_metrics);

    assert_eq!(recommendation.action, RecommendationAction::NoActionNeeded);
    assert!(recommendation.rationale.contains("functional design"));
}
```

### Integration Tests

```rust
#[test]
fn test_call_resolver_gets_functional_bonus() {
    let content = include_str!("../../src/analyzers/call_graph/call_resolution.rs");
    let analysis = analyze_file_for_god_objects(content);

    let call_resolver = analysis.get("CallResolver").unwrap();

    // Should detect functional decomposition
    assert!(call_resolver.functional_metrics.functional_score >= 0.7,
        "Expected functional score >= 0.7, got {}",
        call_resolver.functional_metrics.functional_score);

    // Score should be low after bonus
    assert!(call_resolver.adjusted_score < 30.0,
        "Expected adjusted score < 30, got {}",
        call_resolver.adjusted_score);

    // Recommendation should acknowledge good design
    assert_eq!(call_resolver.recommendation.action, RecommendationAction::NoActionNeeded);
}
```

## Documentation Requirements

- **Code Documentation**: Document functional pattern detection heuristics
- **User Documentation**: Explain functional decomposition bonus
- **Output Documentation**: Document new functional metrics in output

## Implementation Notes

1. **Pattern Detection**: Focus on structural patterns, not naming conventions
2. **Composition Detection**: May miss custom composition utilities
3. **Edge Cases**: Handle hybrid OOP/FP styles gracefully
4. **Performance**: Composition detection requires AST traversal

## Migration and Compatibility

- New bonus reduces scores for functional code
- Add `--no-functional-bonus` flag for old behavior
- Document behavior change in release notes

## Success Metrics

- `CallResolver` (24 methods): Score drops from 100 to <20
- Recommendation changes to "No action needed"
- Actual god objects (low pure ratio) still flagged correctly
- False positive rate on functional code reduced by >70%
