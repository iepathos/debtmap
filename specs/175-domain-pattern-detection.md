---
number: 175
title: Domain Pattern Detection for Semantic Clustering
category: foundation
priority: high
status: draft
dependencies: [174]
created: 2025-11-10
---

# Specification 175: Domain Pattern Detection for Semantic Clustering

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 174 (Strict Utilities Fallback)

## Context

Current classification relies primarily on **syntactic prefix matching** (e.g., `parse_*` → "parsing"), which fails to identify **semantic domain patterns** that define true cohesion.

**Real-world failure case** (`python_type_tracker/mod.rs`):

| Method | Current Classification | Actual Domain |
|---|---|---|
| `register_observer_interfaces()` | "Utilities" | **Observer Pattern** |
| `detect_observer_dispatch()` | "Utilities" | **Observer Pattern** |
| `populate_observer_registry()` | "Utilities" | **Observer Pattern** |
| `check_for_callback_patterns()` | "Validation" | **Callback Pattern** |
| `extract_callback_expr()` | "Parsing" | **Callback Pattern** |
| `check_for_event_bindings()` | "Validation" | **Callback Pattern** |

These methods form **cohesive clusters** by shared domain logic, not by name prefixes:
- **17 observer methods** (~600 lines) all manipulate `ObserverRegistry`
- **3 callback methods** (~100 lines) all manipulate `CallbackTracker`

**Current recommendation**:
```
- mod_utilities.rs - Utilities (25 methods, ~500 lines)  ← Mixed bag!
```

**Better recommendation** (with domain detection):
```
- observer_pattern.rs - Observer Pattern Detection (17 methods, ~600 lines)
- callback_pattern.rs - Callback Pattern Detection (3 methods, ~100 lines)
```

## Objective

Implement domain pattern detection system that identifies **semantic clusters** based on:
1. **Shared data structures** (e.g., `ObserverRegistry`, `CallbackTracker`)
2. **Design pattern keywords** (e.g., "observer", "callback", "registry", "builder")
3. **Field access patterns** (methods accessing same struct fields)
4. **Call graph cohesion** (methods calling each other frequently)

This provides **domain-specific module names** instead of generic "Utilities".

## Requirements

### Functional Requirements

**FR1: Domain Pattern Definitions**

```rust
pub enum DomainPattern {
    /// Observer/Listener/Subject pattern
    ObserverPattern {
        keywords: &'static [&'static str],
        structures: &'static [&'static str],
    },

    /// Callback/Handler/Event binding pattern
    CallbackPattern {
        keywords: &'static [&'static str],
        structures: &'static [&'static str],
    },

    /// Registry/Catalog/Index/Lookup pattern
    RegistryPattern {
        keywords: &'static [&'static str],
        structures: &'static [&'static str],
    },

    /// Builder/Fluent API pattern
    BuilderPattern {
        keywords: &'static [&'static str],
        structures: &'static [&'static str],
    },

    /// Type inference/checking pattern
    TypeInferencePattern {
        keywords: &'static [&'static str],
        structures: &'static [&'static str],
    },

    /// AST traversal/visitor pattern
    AstTraversalPattern {
        keywords: &'static [&'static str],
        structures: &'static [&'static str],
    },
}

impl DomainPattern {
    fn observer() -> Self {
        DomainPattern::ObserverPattern {
            keywords: &[
                "observer", "subject", "notify", "subscribe",
                "listener", "event", "dispatch", "interface",
                "registry", "register", "unregister"
            ],
            structures: &[
                "ObserverRegistry", "ObserverPattern", "Subject",
                "Listener", "EventDispatcher", "ObserverInterface"
            ],
        }
    }

    fn callback() -> Self {
        DomainPattern::CallbackPattern {
            keywords: &[
                "callback", "handler", "event", "binding",
                "hook", "trigger", "invoke", "deferred"
            ],
            structures: &[
                "CallbackTracker", "EventHandler", "CallbackRegistry",
                "DeferredCallback", "EventBinding"
            ],
        }
    }

    // ... more patterns
}
```

**FR2: Pattern Detection Algorithm**

```rust
pub struct DomainPatternDetector {
    patterns: Vec<DomainPattern>,
}

impl DomainPatternDetector {
    pub fn detect_method_domain(
        &self,
        method: &MethodInfo,
        context: &FileContext,
    ) -> Option<DomainPatternMatch> {
        // Score each pattern based on multiple signals
        let mut scores: Vec<(DomainPattern, f64)> = Vec::new();

        for pattern in &self.patterns {
            let score = self.score_pattern(method, context, pattern);
            if score > 0.0 {
                scores.push((pattern.clone(), score));
            }
        }

        // Return highest scoring pattern if above threshold
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        if let Some((pattern, score)) = scores.first() {
            if *score >= DOMAIN_PATTERN_THRESHOLD {
                return Some(DomainPatternMatch {
                    pattern: pattern.clone(),
                    confidence: *score,
                    evidence: self.collect_evidence(method, context, pattern),
                });
            }
        }

        None
    }

    fn score_pattern(
        &self,
        method: &MethodInfo,
        context: &FileContext,
        pattern: &DomainPattern,
    ) -> f64 {
        let mut score = 0.0;

        // Signal 1: Method name contains pattern keywords (weight: 0.30)
        score += 0.30 * self.name_keyword_match(method, pattern);

        // Signal 2: Accesses pattern-related structures (weight: 0.40)
        score += 0.40 * self.structure_access_match(method, context, pattern);

        // Signal 3: Called by other pattern methods (weight: 0.20)
        score += 0.20 * self.call_graph_cohesion(method, context, pattern);

        // Signal 4: Comment/doc contains pattern keywords (weight: 0.10)
        score += 0.10 * self.documentation_match(method, pattern);

        score
    }

    fn name_keyword_match(&self, method: &MethodInfo, pattern: &DomainPattern) -> f64 {
        let name_lower = method.name.to_lowercase();
        let keywords = pattern.keywords();

        let matches = keywords.iter()
            .filter(|kw| name_lower.contains(*kw))
            .count();

        (matches as f64 / keywords.len() as f64).min(1.0)
    }

    fn structure_access_match(
        &self,
        method: &MethodInfo,
        context: &FileContext,
        pattern: &DomainPattern,
    ) -> f64 {
        // Analyze AST to find struct field accesses
        let accessed_structures = extract_structure_accesses(method, context);
        let pattern_structures = pattern.structures();

        let matches = accessed_structures.iter()
            .filter(|s| pattern_structures.contains(&s.as_str()))
            .count();

        if pattern_structures.is_empty() {
            0.0
        } else {
            (matches as f64 / pattern_structures.len() as f64).min(1.0)
        }
    }

    fn call_graph_cohesion(
        &self,
        method: &MethodInfo,
        context: &FileContext,
        pattern: &DomainPattern,
    ) -> f64 {
        // Find other methods matching this pattern
        let pattern_methods: Vec<_> = context.methods.iter()
            .filter(|m| {
                let score = self.name_keyword_match(m, pattern);
                score > 0.5
            })
            .collect();

        if pattern_methods.is_empty() {
            return 0.0;
        }

        // Calculate what % of calls are to/from pattern methods
        let total_calls = context.call_graph.incoming_edges(method)
            .chain(context.call_graph.outgoing_edges(method))
            .count();

        if total_calls == 0 {
            return 0.0;
        }

        let pattern_calls = context.call_graph.incoming_edges(method)
            .chain(context.call_graph.outgoing_edges(method))
            .filter(|edge| {
                pattern_methods.iter().any(|m| m.name == edge.caller || m.name == edge.callee)
            })
            .count();

        pattern_calls as f64 / total_calls as f64
    }
}
```

**FR3: Cluster Methods by Domain**

```rust
pub fn cluster_methods_by_domain(
    methods: &[MethodInfo],
    context: &FileContext,
    detector: &DomainPatternDetector,
) -> HashMap<DomainPattern, Vec<MethodInfo>> {
    let mut clusters: HashMap<DomainPattern, Vec<MethodInfo>> = HashMap::new();

    for method in methods {
        if let Some(domain_match) = detector.detect_method_domain(method, context) {
            clusters
                .entry(domain_match.pattern)
                .or_default()
                .push(method.clone());
        }
    }

    clusters
}
```

**FR4: Generate Domain-Specific Module Names**

```rust
impl DomainPattern {
    pub fn module_name(&self) -> String {
        match self {
            DomainPattern::ObserverPattern { .. } => "observer_pattern".to_string(),
            DomainPattern::CallbackPattern { .. } => "callback_pattern".to_string(),
            DomainPattern::RegistryPattern { .. } => "registry".to_string(),
            DomainPattern::BuilderPattern { .. } => "builder".to_string(),
            DomainPattern::TypeInferencePattern { .. } => "type_inference".to_string(),
            DomainPattern::AstTraversalPattern { .. } => "ast_traversal".to_string(),
        }
    }

    pub fn description(&self) -> String {
        match self {
            DomainPattern::ObserverPattern { .. } => {
                "Observer Pattern Detection".to_string()
            }
            DomainPattern::CallbackPattern { .. } => {
                "Callback Pattern Detection".to_string()
            }
            // ... more descriptions
        }
    }
}
```

### Non-Functional Requirements

**NFR1: Accuracy**
- Domain pattern detection precision ≥80% (correct patterns / detected patterns)
- Domain pattern detection recall ≥70% (detected patterns / actual patterns)
- False positive rate <10% (incorrect pattern assignments)

**NFR2: Performance**
- Pattern detection overhead <15% of god object detection time
- Lazy evaluation of expensive signals (call graph analysis)
- Cache pattern detection results per file

**NFR3: Extensibility**
- Easy to add new domain patterns
- Pluggable scoring strategies
- Configurable pattern definitions

## Acceptance Criteria

- [ ] 6+ domain patterns defined (observer, callback, registry, builder, type_inference, ast_traversal)
- [ ] Multi-signal scoring algorithm implemented (name, structure, call graph, docs)
- [ ] Pattern detection integrated into classification pipeline
- [ ] Domain-specific module names generated
- [ ] Observer pattern detected in `python_type_tracker/mod.rs` (17 methods)
- [ ] Callback pattern detected in `python_type_tracker/mod.rs` (3 methods)
- [ ] Pattern detection precision ≥80%
- [ ] Pattern detection recall ≥70%
- [ ] Performance overhead <15%
- [ ] All existing tests pass
- [ ] Integration tests validate pattern detection
- [ ] Documentation explains pattern detection system

## Technical Details

### Implementation Approach

**Phase 1: Define Patterns**

Create `src/organization/domain_patterns.rs`:
```rust
pub mod domain_patterns {
    // Pattern definitions
    // Scoring algorithms
    // Evidence collection
}
```

**Phase 2: AST Analysis for Structure Access**

```rust
fn extract_structure_accesses(
    method: &MethodInfo,
    context: &FileContext,
) -> HashSet<String> {
    use syn::visit::Visit;

    struct StructureVisitor {
        accessed: HashSet<String>,
    }

    impl<'ast> Visit<'ast> for StructureVisitor {
        fn visit_field_expr(&mut self, node: &'ast syn::FieldExpr) {
            // self.observer_registry → "ObserverRegistry"
            if let syn::Expr::Path(path) = &*node.base {
                if let Some(ty) = context.type_of_expr(&path) {
                    self.accessed.insert(ty.to_string());
                }
            }
        }
    }

    let mut visitor = StructureVisitor {
        accessed: HashSet::new(),
    };

    visitor.visit_item_fn(&method.ast_node);
    visitor.accessed
}
```

**Phase 3: Integration with Classification**

```rust
// src/organization/god_object_analysis.rs

pub fn group_methods_by_responsibility_and_domain(
    methods: &[MethodInfo],
    context: &FileContext,
) -> HashMap<String, Vec<MethodInfo>> {
    let detector = DomainPatternDetector::new();

    // First pass: Detect domain patterns
    let domain_clusters = cluster_methods_by_domain(methods, context, &detector);

    let mut groups: HashMap<String, Vec<MethodInfo>> = HashMap::new();

    // Add domain-specific clusters
    for (pattern, cluster_methods) in domain_clusters {
        if cluster_methods.len() >= MIN_DOMAIN_CLUSTER_SIZE {
            groups.insert(
                pattern.module_name(),
                cluster_methods
            );
        }
    }

    // Second pass: Classify remaining methods by responsibility
    let unclustered: Vec<_> = methods.iter()
        .filter(|m| !groups.values().any(|v| v.contains(m)))
        .collect();

    for method in unclustered {
        let result = infer_responsibility_with_confidence(
            &method.name,
            Some(&method.body),
            context.language,
        );

        if let Some(category) = result.category {
            groups.entry(category).or_default().push(method.clone());
        }
    }

    groups
}
```

### Pattern Detection Constants

```rust
/// Minimum confidence for domain pattern match
pub const DOMAIN_PATTERN_THRESHOLD: f64 = 0.60;

/// Minimum methods required to form domain cluster
pub const MIN_DOMAIN_CLUSTER_SIZE: usize = 3;

/// Signal weights for pattern scoring
pub const WEIGHT_NAME_KEYWORDS: f64 = 0.30;
pub const WEIGHT_STRUCTURE_ACCESS: f64 = 0.40;
pub const WEIGHT_CALL_GRAPH: f64 = 0.20;
pub const WEIGHT_DOCUMENTATION: f64 = 0.10;
```

### Evidence Collection

```rust
#[derive(Debug, Clone)]
pub struct PatternEvidence {
    pub matched_keywords: Vec<String>,
    pub accessed_structures: Vec<String>,
    pub cohesive_calls: Vec<(String, String)>,  // (caller, callee)
    pub documentation_matches: Vec<String>,
}

impl DomainPatternDetector {
    fn collect_evidence(
        &self,
        method: &MethodInfo,
        context: &FileContext,
        pattern: &DomainPattern,
    ) -> PatternEvidence {
        let name_lower = method.name.to_lowercase();

        PatternEvidence {
            matched_keywords: pattern.keywords().iter()
                .filter(|kw| name_lower.contains(*kw))
                .map(|s| s.to_string())
                .collect(),

            accessed_structures: extract_structure_accesses(method, context)
                .into_iter()
                .filter(|s| pattern.structures().contains(&s.as_str()))
                .collect(),

            cohesive_calls: context.call_graph
                .edges_for_method(method)
                .filter(|edge| {
                    // Check if other method matches pattern
                    context.methods.iter()
                        .find(|m| m.name == edge.other_method)
                        .map(|m| self.name_keyword_match(m, pattern) > 0.5)
                        .unwrap_or(false)
                })
                .map(|edge| (edge.caller.clone(), edge.callee.clone()))
                .collect(),

            documentation_matches: extract_doc_keywords(method)
                .into_iter()
                .filter(|kw| pattern.keywords().contains(&kw.as_str()))
                .collect(),
        }
    }
}
```

## Dependencies

**Prerequisites**:
- Spec 174 (Strict Utilities Fallback) - Confidence-based classification

**Affected Components**:
- `src/organization/domain_patterns.rs` (new) - Pattern definitions
- `src/organization/god_object_analysis.rs` - Integration
- `src/organization/module_function_classifier.rs` - Module split generation
- `src/priority/formatter.rs` - Display pattern-based recommendations

**External Dependencies**:
- `syn` - AST analysis for structure access detection
- Existing call graph infrastructure

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_observer_pattern_detection() {
    let method = MethodInfo {
        name: "register_observer_interfaces".to_string(),
        body: "self.observer_registry.register(...)",
        // ... more fields
    };

    let context = FileContext {
        structures: vec!["ObserverRegistry".to_string()],
        // ... more fields
    };

    let detector = DomainPatternDetector::new();
    let result = detector.detect_method_domain(&method, &context);

    assert!(result.is_some());
    let matched = result.unwrap();
    assert!(matches!(matched.pattern, DomainPattern::ObserverPattern { .. }));
    assert!(matched.confidence >= 0.60);
}

#[test]
fn test_callback_pattern_detection() {
    let method = MethodInfo {
        name: "check_for_callback_patterns".to_string(),
        body: "self.callback_tracker.track(...)",
        // ... more fields
    };

    let detector = DomainPatternDetector::new();
    let result = detector.detect_method_domain(&method, &context);

    assert!(result.is_some());
    assert!(matches!(result.unwrap().pattern, DomainPattern::CallbackPattern { .. }));
}
```

### Integration Tests

**Test: Real-world Pattern Detection**
```rust
#[test]
fn test_python_type_tracker_patterns() {
    let source = include_str!("../src/analysis/python_type_tracker/mod.rs");
    let ast = syn::parse_file(source).unwrap();
    let context = build_file_context(&ast, source);

    let detector = DomainPatternDetector::new();
    let clusters = cluster_methods_by_domain(&context.methods, &context, &detector);

    // Should detect observer pattern cluster
    assert!(clusters.contains_key(&DomainPattern::observer()));
    let observer_methods = &clusters[&DomainPattern::observer()];
    assert!(observer_methods.len() >= 15);

    // Should detect callback pattern cluster
    assert!(clusters.contains_key(&DomainPattern::callback()));
    let callback_methods = &clusters[&DomainPattern::callback()];
    assert!(callback_methods.len() >= 3);
}
```

### Precision/Recall Tests

```rust
#[test]
fn test_pattern_detection_precision() {
    // Load ground truth with manual pattern labels
    let corpus = load_pattern_corpus("tests/data/pattern_ground_truth.json");

    let detector = DomainPatternDetector::new();
    let mut true_positives = 0;
    let mut false_positives = 0;

    for sample in &corpus {
        let result = detector.detect_method_domain(&sample.method, &sample.context);

        match (result, &sample.expected_pattern) {
            (Some(detected), Some(expected)) if detected.pattern == *expected => {
                true_positives += 1;
            }
            (Some(_), None) => {
                false_positives += 1;
            }
            _ => {}
        }
    }

    let precision = true_positives as f64 / (true_positives + false_positives) as f64;
    assert!(precision >= 0.80, "Precision too low: {:.2}", precision);
}
```

## Documentation Requirements

**Code Documentation**:
- Document each domain pattern with examples
- Explain scoring algorithm and signal weights
- Provide guidance on adding new patterns

**User Documentation**:
- Update god object detection guide
- Explain domain pattern detection
- Show examples of pattern-based recommendations
- Document configuration options

**Pattern Catalog**:
```markdown
## Supported Domain Patterns

### Observer Pattern
**Keywords**: observer, subject, notify, subscribe, listener, event
**Structures**: ObserverRegistry, Subject, Listener
**Example**: Methods managing observer lists and dispatching events

### Callback Pattern
**Keywords**: callback, handler, event, binding, hook
**Structures**: CallbackTracker, EventHandler, CallbackRegistry
**Example**: Methods registering and invoking callbacks

... (document all patterns)
```

## Implementation Notes

**Validation**:
- Test on multiple real-world codebases
- Collect false positive/negative examples
- Tune signal weights based on results

**Extensibility**:
- Allow users to define custom patterns
- Support pattern inheritance (e.g., specialized observer patterns)
- Provide pattern templates

**Future Enhancements**:
- ML-based pattern detection
- Cross-file pattern detection
- Pattern evolution tracking

## Migration and Compatibility

**Breaking Changes**: None - purely additive

**Migration**: Automatic - no user action required

**Backward Compatibility**: Full - existing classification still works

## Success Metrics

- Observer pattern detected in `python_type_tracker/mod.rs`
- Callback pattern detected in `python_type_tracker/mod.rs`
- Pattern detection precision ≥80%
- Pattern detection recall ≥70%
- Domain-specific module recommendations generated
- User feedback: "Recommendations make more semantic sense"
- Reduction in generic "Utilities" classifications by 50%+
