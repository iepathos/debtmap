---
number: 126
title: AST-based Data Flow Classification
category: foundation
priority: medium
status: draft
dependencies: [117, 122, 124, 125]
created: 2025-10-21
---

# Specification 126: AST-based Data Flow Classification

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: Specs 117 (Constructor Detection), 122 (AST Analysis), 124 (Enum Converters), 125 (Accessor Detection)

## Context

While Specs 124 and 125 address specific false positive patterns (enum converters and accessors), there remains a broader class of functions that are primarily data transformation pipelines without complex business logic. These functions orchestrate data flow but don't contain testable business rules.

**Current Gap**:

After implementing Specs 124-125, these patterns might still be misclassified:

```rust
// Pattern 1: Data pipeline (mostly transformations)
pub fn prepare_response(data: Vec<Item>) -> Response {
    let filtered = data.into_iter()
        .filter(|item| !item.is_deleted)
        .collect();

    let serialized = serde_json::to_string(&filtered)?;

    Response {
        body: serialized,
        status: 200,
    }
}
// Current: Might be PureLogic due to moderate complexity
// Expected: Orchestrator (coordinates transformations)

// Pattern 2: Configuration builder (data assembly)
pub fn build_config(env: &Environment) -> Config {
    let timeout = env.get("TIMEOUT").unwrap_or("30");
    let host = env.get("HOST").unwrap_or("localhost");
    let port = env.get("PORT").unwrap_or("8080");

    Config {
        timeout: timeout.parse().unwrap(),
        host: host.to_string(),
        port: port.parse().unwrap(),
    }
}
// Current: PureLogic (has complexity)
// Expected: IOWrapper or Orchestrator (environment reads + assembly)

// Pattern 3: Pure data transformation (no business rules)
pub fn normalize_path(path: &Path) -> PathBuf {
    path.components()
        .filter(|c| !matches!(c, Component::CurDir))
        .map(|c| c.as_os_str())
        .collect()
}
// Current: PureLogic
// Expected: IOWrapper (pure transformation, no business logic)
```

**Why This is Important**:

1. **Data flow vs business logic**: Functions that orchestrate data flow are less critical to test exhaustively than business logic with decision rules
2. **Testing priority**: Business rules (pricing, validation, permissions) are higher priority than data plumbing
3. **Cognitive load**: Users should focus on testing complex decision logic, not data transformation pipelines

**However, This is Lower Priority Than Specs 124-125**:

- Specs 124-125 address clear, common false positives with high confidence
- This spec addresses a more nuanced, harder-to-detect pattern
- AST-based data flow analysis is more complex and computationally expensive
- Risk of false positives is higher (misclassifying real business logic)

## Objective

Implement AST-based data flow analysis to detect functions that primarily transform or orchestrate data without complex business logic, classifying them as `Orchestrator` or `IOWrapper` instead of `PureLogic` when appropriate.

**Scope**: This is an enhancement for edge cases after Specs 124-125 are implemented. It should be conservative to avoid misclassifying real business logic.

## Requirements

### Functional Requirements

1. **Data Flow Pattern Detection**
   - Identify functions with high ratio of data transformations to business logic
   - Detect iterator chains (`.map()`, `.filter()`, `.fold()`, `.collect()`)
   - Identify builder patterns (struct initialization with field assignments)
   - Recognize serialization/deserialization operations

2. **Business Logic Detection** (to avoid false positives)
   - Detect validation logic (assertions, bounds checks, invariant checks)
   - Identify calculation logic (arithmetic, algorithms)
   - Recognize decision trees (complex if/match with business rules)
   - Flag complex error handling (domain-specific errors)

3. **Classification Rules**
   - If >70% data transformation + low business logic → Orchestrator
   - If simple transformations + no I/O + no business rules → IOWrapper
   - If contains business logic → PureLogic (default, safe choice)

4. **Conservative Approach**
   - Only classify with high confidence (>80%)
   - When in doubt, keep as PureLogic
   - Provide confidence score for debugging

### Non-Functional Requirements

- Analysis must be efficient (< 5ms per function)
- Must not significantly increase memory usage
- Should be opt-in via configuration (default: disabled initially)
- Must have very low false positive rate (< 5%)

## Acceptance Criteria

- [ ] **AST Analysis Module**: Create `src/analyzers/data_flow_analyzer.rs`:
  - `analyze_data_flow(syn_func: &syn::ItemFn) -> DataFlowProfile`
  - `DataFlowProfile` struct with transformation_ratio, business_logic_ratio, confidence

- [ ] **Pattern Detection**:
  - [ ] Detect iterator methods: map, filter, fold, collect, for_each
  - [ ] Detect builder patterns: struct initialization with many fields
  - [ ] Detect serialization: serde_json, bincode, format!, to_string
  - [ ] Detect I/O operations: File, Read, Write, network calls

- [ ] **Business Logic Heuristics**:
  - [ ] Arithmetic operations: +, -, *, /, %
  - [ ] Comparison operations in complex contexts
  - [ ] Validation patterns: assert, return Err, bounds checks
  - [ ] Domain-specific calculations

- [ ] **Classification Integration**:
  - [ ] Add `is_data_flow_orchestrator()` to `semantic_classifier.rs`
  - [ ] Only run if config enabled (default: false)
  - [ ] Require high confidence threshold (>0.8)
  - [ ] Fall back to default classification if low confidence

- [ ] **Configuration**:
  - [ ] Add `DataFlowClassificationConfig` to `config.rs`
  - [ ] Enable/disable flag (default: false - opt-in)
  - [ ] Configurable confidence threshold (default: 0.8)
  - [ ] Configurable transformation ratio threshold (default: 0.7)

- [ ] **Testing**:
  - [ ] Test case: Iterator chain classified as Orchestrator
  - [ ] Test case: Struct builder classified as Orchestrator/IOWrapper
  - [ ] Test case: Business logic NOT misclassified
  - [ ] Test case: Complex validation remains PureLogic
  - [ ] Performance test: < 5ms overhead per function

- [ ] **Safety Validation**:
  - [ ] Zero false positives on known business logic functions
  - [ ] Confidence scores calibrated correctly
  - [ ] Manual review of top 100 classifications

## Technical Details

### Implementation Approach

**Module Structure**:
```rust
// src/analyzers/data_flow_analyzer.rs

#[derive(Debug, Clone)]
pub struct DataFlowProfile {
    /// Ratio of data transformation operations (0.0 - 1.0)
    pub transformation_ratio: f64,

    /// Ratio of business logic operations (0.0 - 1.0)
    pub business_logic_ratio: f64,

    /// Confidence in classification (0.0 - 1.0)
    pub confidence: f64,

    /// Detected patterns
    pub patterns: Vec<DataFlowPattern>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataFlowPattern {
    IteratorChain { length: usize },
    StructBuilder { fields: usize },
    Serialization { format: String },
    IOOperation { kind: String },
    BusinessLogic { kind: String },
}

pub fn analyze_data_flow(syn_func: &syn::ItemFn) -> DataFlowProfile {
    let mut visitor = DataFlowVisitor::new();
    visitor.visit_item_fn(syn_func);

    let total_ops = visitor.transformation_ops + visitor.business_logic_ops;
    if total_ops == 0 {
        return DataFlowProfile::unknown();
    }

    let transformation_ratio =
        visitor.transformation_ops as f64 / total_ops as f64;
    let business_logic_ratio =
        visitor.business_logic_ops as f64 / total_ops as f64;

    // Calculate confidence based on signal strength
    let confidence = calculate_confidence(&visitor);

    DataFlowProfile {
        transformation_ratio,
        business_logic_ratio,
        confidence,
        patterns: visitor.patterns,
    }
}

struct DataFlowVisitor {
    transformation_ops: usize,
    business_logic_ops: usize,
    patterns: Vec<DataFlowPattern>,
    current_chain_length: usize,
}

impl syn::visit::Visit<'_> for DataFlowVisitor {
    fn visit_expr_method_call(&mut self, node: &syn::ExprMethodCall) {
        let method_name = node.method.to_string();

        // Iterator transformations
        if matches!(
            method_name.as_str(),
            "map" | "filter" | "fold" | "collect" | "for_each" |
            "filter_map" | "flat_map" | "zip" | "chain"
        ) {
            self.transformation_ops += 1;
            self.current_chain_length += 1;
        }

        // Serialization
        if matches!(
            method_name.as_str(),
            "to_string" | "serialize" | "deserialize"
        ) {
            self.transformation_ops += 1;
            self.patterns.push(DataFlowPattern::Serialization {
                format: method_name,
            });
        }

        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_binary(&mut self, node: &syn::ExprBinary) {
        // Arithmetic = business logic
        if matches!(
            node.op,
            syn::BinOp::Add(_) | syn::BinOp::Sub(_) |
            syn::BinOp::Mul(_) | syn::BinOp::Div(_)
        ) {
            self.business_logic_ops += 1;
        }

        syn::visit::visit_expr_binary(self, node);
    }

    fn visit_expr_if(&mut self, node: &syn::ExprIf) {
        // Complex conditionals = business logic
        if is_business_logic_condition(&node.cond) {
            self.business_logic_ops += 1;
        }

        syn::visit::visit_expr_if(self, node);
    }

    // ... more visitors for patterns
}

fn calculate_confidence(visitor: &DataFlowVisitor) -> f64 {
    let total_ops = visitor.transformation_ops + visitor.business_logic_ops;

    // Low total operations = low confidence
    if total_ops < 5 {
        return 0.3;
    }

    // Strong signal = high confidence
    let max_ratio = visitor.transformation_ops.max(visitor.business_logic_ops)
        as f64 / total_ops as f64;

    if max_ratio > 0.9 {
        0.95
    } else if max_ratio > 0.8 {
        0.85
    } else if max_ratio > 0.7 {
        0.75
    } else {
        0.5 // Ambiguous - don't classify
    }
}
```

### Classification Integration

```rust
// src/priority/semantic_classifier.rs

fn classify_by_rules(
    func: &FunctionMetrics,
    func_id: &FunctionId,
    call_graph: &CallGraph,
    syn_func: Option<&syn::ItemFn>,
) -> Option<FunctionRole> {
    // ... existing checks (entry point, constructor, enum converter, accessor)

    // Data flow classification (only if enabled and AST available)
    if let Some(syn_func) = syn_func {
        let config = crate::config::get_data_flow_classification_config();

        if config.enabled {
            let profile = analyze_data_flow(syn_func);

            // Only classify if high confidence
            if profile.confidence >= config.min_confidence {
                if profile.transformation_ratio >= config.min_transformation_ratio
                    && profile.business_logic_ratio < 0.3
                {
                    return Some(FunctionRole::Orchestrator);
                }
            }
        }
    }

    // ... rest of classification
}
```

### Data Structures

```rust
// src/config.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFlowClassificationConfig {
    /// Enable data flow classification (default: false - opt-in)
    pub enabled: bool,

    /// Minimum confidence required (0.0 - 1.0)
    pub min_confidence: f64,

    /// Minimum transformation ratio to classify as Orchestrator
    pub min_transformation_ratio: f64,

    /// Maximum business logic ratio for Orchestrator classification
    pub max_business_logic_ratio: f64,
}

impl Default for DataFlowClassificationConfig {
    fn default() -> Self {
        Self {
            enabled: false, // OPT-IN
            min_confidence: 0.8,
            min_transformation_ratio: 0.7,
            max_business_logic_ratio: 0.3,
        }
    }
}
```

## Dependencies

- **Prerequisites**:
  - Spec 117: Constructor Detection (classification framework)
  - Spec 122: AST-based Analysis (AST visitor patterns)
  - Spec 124: Enum Converter Detection (similar approach)
  - Spec 125: Accessor Detection (name-based baseline)

- **Affected Components**:
  - `src/analyzers/mod.rs`: Add data_flow_analyzer module
  - `src/priority/semantic_classifier.rs`: Add data flow classification
  - `src/config.rs`: Add configuration

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_iterator_chain_detected() {
    let code = r#"
        fn transform(items: Vec<i32>) -> Vec<i32> {
            items.into_iter()
                .filter(|x| x > 0)
                .map(|x| x * 2)
                .collect()
        }
    "#;

    let syn_func = parse_function(code);
    let profile = analyze_data_flow(&syn_func);

    assert!(profile.transformation_ratio > 0.7);
    assert!(profile.confidence > 0.8);
}

#[test]
fn test_business_logic_not_misclassified() {
    let code = r#"
        fn calculate_price(quantity: i32, base_price: f64) -> f64 {
            let discount = if quantity > 100 {
                0.2
            } else if quantity > 50 {
                0.1
            } else {
                0.0
            };

            base_price * quantity as f64 * (1.0 - discount)
        }
    "#;

    let syn_func = parse_function(code);
    let profile = analyze_data_flow(&syn_func);

    // Should NOT be classified as data flow
    assert!(profile.business_logic_ratio > 0.5);
}

#[test]
fn test_low_confidence_rejected() {
    let code = r#"
        fn mixed_function(x: i32) -> i32 {
            let y = x * 2;
            y + 1
        }
    "#;

    let syn_func = parse_function(code);
    let profile = analyze_data_flow(&syn_func);

    // Ambiguous - should have low confidence
    assert!(profile.confidence < 0.8);
}
```

### Integration Tests

```rust
#[test]
fn test_no_business_logic_misclassified() {
    let analysis = analyze_project(Path::new("tests/fixtures/business_logic"));

    // Ensure NO business logic functions are misclassified
    for item in &analysis.items {
        if is_known_business_logic(&item.location.function_name) {
            assert_ne!(
                item.role,
                FunctionRole::Orchestrator,
                "Business logic misclassified: {}",
                item.location.function_name
            );
        }
    }
}
```

### Performance Tests

```rust
#[test]
fn test_analysis_performance() {
    let large_function = generate_test_function_with_n_statements(100);

    let start = Instant::now();
    let _ = analyze_data_flow(&large_function);
    let duration = start.elapsed();

    assert!(duration < Duration::from_millis(5));
}
```

## Documentation Requirements

### Code Documentation

- Document `DataFlowProfile` structure and interpretation
- Explain confidence score calculation
- Provide examples of detected vs rejected patterns

### User Documentation

- Explain data flow classification in debtmap book
- Document how to enable opt-in feature
- Provide guidance on interpreting results
- **Clearly state this is experimental and opt-in**

### Architecture Updates

- Document data flow analysis approach
- Explain trade-offs and limitations
- Add examples to decision tree

## Implementation Notes

### This is a Research Feature

This spec is more experimental than Specs 124-125:

1. **Opt-in by default**: Users must explicitly enable
2. **Lower priority**: Implement after 124-125 are proven
3. **Higher risk**: More complex, more potential for false positives
4. **Conservative threshold**: Only classify with >80% confidence

### Edge Cases

1. **Mixed functions**: Both data flow and business logic → classify as PureLogic
2. **Short functions**: < 5 operations → low confidence, don't classify
3. **Domain-specific patterns**: May need customization per domain

### Performance Considerations

- Cache data flow profiles per file
- Skip analysis if disabled in config
- Limit AST traversal depth
- Early exit if business logic detected

### Calibration Required

After initial implementation:
1. Analyze top 1000 functions
2. Manually review classifications
3. Tune confidence thresholds
4. Adjust transformation ratio thresholds
5. Iterate until false positive rate < 5%

## Migration and Compatibility

### Breaking Changes

None - this is opt-in functionality.

### Configuration

```rust
pub struct ClassificationConfig {
    pub constructors: Option<ConstructorDetectionConfig>,
    pub enum_converters: Option<EnumConverterDetectionConfig>,
    pub accessors: Option<AccessorDetectionConfig>,
    pub data_flow: Option<DataFlowClassificationConfig>, // NEW, default: disabled
}
```

### Rollout Strategy

1. **Phase 1**: Implement and test thoroughly (this spec)
2. **Phase 2**: Gather data on classification accuracy
3. **Phase 3**: Tune thresholds based on real-world usage
4. **Phase 4**: Consider enabling by default (future decision)

## Success Metrics

**Phase 1 (Initial Implementation)**:
- False positive rate < 5% on test corpus
- Confidence scores calibrated correctly
- Performance overhead < 5% total analysis time

**Phase 2 (After Tuning)**:
- Identify 20-30% more Orchestrator functions
- Reduce false positives in top 20 recommendations
- User feedback indicates improved accuracy

**Future (If Enabled by Default)**:
- No increase in user-reported false positives
- Improved focus on business logic in recommendations

## Limitations and Future Work

### Known Limitations

1. **Complex business logic**: May miss subtle business rules embedded in data flow
2. **Domain-specific**: Patterns may vary by domain (web, scientific, systems programming)
3. **AST-only**: Cannot detect runtime behavior or dynamic dispatch

### Future Enhancements

1. **Machine learning**: Train classifier on labeled examples
2. **Type information**: Use full type context for better accuracy
3. **Domain profiles**: Configurable patterns per application domain
4. **User feedback**: Allow users to correct classifications and improve model

## References

- Spec 117: Constructor Detection and Classification
- Spec 122: AST-based Constructor Detection
- Spec 124: Enum Converter Detection
- Spec 125: Accessor/Getter Detection
- Research: "Identifying Data Flow Patterns in Code" (future reference)
