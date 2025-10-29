---
number: 138c
title: Advanced Pattern Detection Library (Optional Enhancement)
category: optimization
priority: medium
status: draft
dependencies: [138a]
created: 2025-10-29
replaces: 138 (split into 138a/b/c)
note: 138b (code examples) deferred, 138c provides more value through specific detection
---

# Specification 138c: Advanced Pattern Detection Library

**Category**: optimization
**Priority**: medium (optional enhancement, validate after 138a)
**Status**: draft
**Dependencies**: Spec 138a (Concise Recommendations)
**Supersedes**: Spec 138 (split into three focused specs)
**Note**: Spec 138b (code examples) deferred - 138c provides more value

## Context

After implementing concise recommendations (138a), there's an opportunity to **improve pattern detection accuracy** using more sophisticated analysis. This provides more value than code examples (138b, now deferred) because it gives specific, targeted recommendations rather than generic patterns.

This is **optional** and should only be pursued if user feedback shows that recommendations from 138a are too generic.

**Current State** (after 138a):
- Pattern detection uses simple heuristics (nesting depth, complexity, length)
- Works well for common cases
- Some edge cases not well-handled (e.g., appropriate vs inappropriate nesting)

**Potential Improvements**:
1. Detect parameter list length (not just function length)
2. Identify complex boolean expressions
3. Detect state mutation patterns
4. Find resource management issues
5. Identify architectural patterns (God objects, feature envy)

**Key Constraint**: Must validate with real users before implementing. This spec may never be implemented.

## Objective

Build a **pattern detection library** that analyzes code using existing AST structures (already parsed for metrics) to provide more accurate refactoring recommendations.

**Scope**: Rust-only initially. Python/JS/TS only if Rust shows clear value.

## Requirements

### Functional Requirements

1. **Extended Pattern Detection**
   - Long parameter lists (>4 parameters)
   - Complex boolean expressions (>3 operators)
   - Multiple responsibilities (mixed I/O, computation, coordination)
   - State mutation hotspots (many mutable variables)
   - Resource management issues (missing cleanup)

2. **Leverage Existing Analysis**
   - Use AST already parsed for cyclomatic complexity
   - No duplicate parsing or analysis
   - Add lightweight visitors for new patterns

3. **Graceful Degradation**
   - If pattern detection fails, fall back to heuristics
   - Never fail entire analysis due to pattern detection
   - Optional feature flag to disable

4. **Integration with Templates**
   - Map detected patterns to templates from 138b
   - Improve template selection accuracy
   - Provide pattern-specific guidance

### Non-Functional Requirements

1. **Performance**: <10ms overhead per function (measured with benchmarks)
2. **Memory**: No significant increase (reuse existing AST)
3. **Maintainability**: Pure functions, no complex state machines
4. **Language-Specific**: Start with Rust, isolate language-specific code

## Acceptance Criteria

**Gate 1: Validation** (before implementation)
- [ ] User survey shows demand for better pattern detection
- [ ] Analysis of false positives shows current heuristics insufficient
- [ ] Product manager approves implementation

**Gate 2: Implementation** (if Gate 1 passes)
- [ ] 5+ new patterns detected accurately
- [ ] <10ms performance overhead per function
- [ ] 90%+ accuracy on validation corpus
- [ ] Graceful fallback to heuristics
- [ ] Integration tests show improved recommendations
- [ ] Feature flag allows disabling
- [ ] Rust-only implementation complete

**Gate 3: Extension** (optional, after Gate 2)
- [ ] Python/JS/TS implementations (if Rust validated)

## Technical Details

### Implementation Approach

#### 1. Pattern Detection Interface

```rust
/// Detected complexity pattern with evidence
#[derive(Debug, Clone)]
pub struct DetectedPattern {
    pub pattern_type: ComplexityPattern,
    pub confidence: f32, // 0.0-1.0
    pub evidence: Vec<String>,
    pub metrics: PatternMetrics,
}

/// Types of complexity patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplexityPattern {
    NestedConditionals { depth: u32 },
    LongParameterList { count: usize },
    ComplexBooleanExpression { operators: usize },
    MultipleResponsibilities { types: Vec<ResponsibilityType> },
    StateManagementComplexity { mutations: usize },
    ResourceLeak { resources: Vec<String> },
}

#[derive(Debug, Clone, Copy)]
pub enum ResponsibilityType {
    IO,
    Computation,
    Coordination,
    Validation,
    ErrorHandling,
}

#[derive(Debug, Clone)]
pub struct PatternMetrics {
    pub severity: f32, // 0.0-1.0
    pub refactoring_difficulty: Difficulty,
}
```

#### 2. Pattern Detection Functions (Pure)

```rust
/// Detect patterns using existing AST (no reparsing)
pub fn detect_patterns(
    func: &syn::ItemFn,
    metrics: &FunctionMetrics,
) -> Vec<DetectedPattern> {
    let mut patterns = Vec::new();

    // Use existing complexity metrics first
    if metrics.nesting > 3 {
        patterns.push(DetectedPattern {
            pattern_type: ComplexityPattern::NestedConditionals {
                depth: metrics.nesting,
            },
            confidence: 0.95, // High confidence from existing metric
            evidence: vec![format!("Nesting depth: {}", metrics.nesting)],
            metrics: PatternMetrics {
                severity: (metrics.nesting as f32 / 5.0).min(1.0),
                refactoring_difficulty: Difficulty::Medium,
            },
        });
    }

    // Additional pattern detection (lightweight)
    if let Some(param_pattern) = detect_long_parameters(func) {
        patterns.push(param_pattern);
    }

    if let Some(bool_pattern) = detect_complex_booleans(func) {
        patterns.push(bool_pattern);
    }

    if let Some(resp_pattern) = detect_multiple_responsibilities(func, metrics) {
        patterns.push(resp_pattern);
    }

    patterns
}

/// Detect long parameter lists (lightweight AST traversal)
fn detect_long_parameters(func: &syn::ItemFn) -> Option<DetectedPattern> {
    let param_count = func.sig.inputs.len();

    if param_count > 4 {
        Some(DetectedPattern {
            pattern_type: ComplexityPattern::LongParameterList { count: param_count },
            confidence: 1.0, // Objective count
            evidence: vec![
                format!("Function has {} parameters", param_count),
                "Consider parameter object or builder pattern".to_string(),
            ],
            metrics: PatternMetrics {
                severity: ((param_count - 4) as f32 / 6.0).min(1.0),
                refactoring_difficulty: if param_count > 8 {
                    Difficulty::Hard
                } else {
                    Difficulty::Medium
                },
            },
        })
    } else {
        None
    }
}

/// Detect complex boolean expressions
fn detect_complex_booleans(func: &syn::ItemFn) -> Option<DetectedPattern> {
    struct BooleanVisitor {
        max_operators: usize,
        current_operators: usize,
    }

    impl<'ast> syn::visit::Visit<'ast> for BooleanVisitor {
        fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
            if matches!(
                node.op,
                syn::BinOp::And(_) | syn::BinOp::Or(_)
            ) {
                self.current_operators += 1;
                self.max_operators = self.max_operators.max(self.current_operators);
            }
            syn::visit::visit_expr_binary(self, node);
            self.current_operators = self.current_operators.saturating_sub(1);
        }
    }

    let mut visitor = BooleanVisitor {
        max_operators: 0,
        current_operators: 0,
    };

    syn::visit::visit_item_fn(&mut visitor, func);

    if visitor.max_operators > 3 {
        Some(DetectedPattern {
            pattern_type: ComplexityPattern::ComplexBooleanExpression {
                operators: visitor.max_operators,
            },
            confidence: 0.9,
            evidence: vec![
                format!("Boolean expression with {} operators", visitor.max_operators),
                "Consider extracting into named predicate function".to_string(),
            ],
            metrics: PatternMetrics {
                severity: (visitor.max_operators as f32 / 10.0).min(1.0),
                refactoring_difficulty: Difficulty::Easy,
            },
        })
    } else {
        None
    }
}

/// Detect multiple responsibilities using heuristics
fn detect_multiple_responsibilities(
    func: &syn::ItemFn,
    metrics: &FunctionMetrics,
) -> Option<DetectedPattern> {
    struct ResponsibilityVisitor {
        has_io: bool,
        has_computation: bool,
        has_coordination: bool,
    }

    impl<'ast> syn::visit::Visit<'ast> for ResponsibilityVisitor {
        fn visit_macro(&mut self, node: &'ast syn::Macro) {
            let name = node.path.segments.last().map(|s| s.ident.to_string());
            if let Some(ref n) = name {
                if n.contains("print") || n.contains("write") {
                    self.has_io = true;
                }
            }
            syn::visit::visit_macro(self, node);
        }

        fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
            let method = node.method.to_string();
            if method.contains("read") || method.contains("write")
                || method.contains("open") || method.contains("close") {
                self.has_io = true;
            }
            syn::visit::visit_expr_method_call(self, node);
        }

        fn visit_expr_for(&mut self, node: &'ast syn::ExprFor) {
            self.has_computation = true;
            syn::visit::visit_expr_for(self, node);
        }

        fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
            self.has_coordination = true;
            syn::visit::visit_expr_call(self, node);
        }
    }

    let mut visitor = ResponsibilityVisitor {
        has_io: false,
        has_computation: false,
        has_coordination: false,
    };

    syn::visit::visit_item_fn(&mut visitor, func);

    let mut responsibilities = Vec::new();
    if visitor.has_io { responsibilities.push(ResponsibilityType::IO); }
    if visitor.has_computation { responsibilities.push(ResponsibilityType::Computation); }
    if visitor.has_coordination { responsibilities.push(ResponsibilityType::Coordination); }

    if responsibilities.len() > 1 {
        Some(DetectedPattern {
            pattern_type: ComplexityPattern::MultipleResponsibilities {
                types: responsibilities.clone(),
            },
            confidence: 0.7, // Heuristic, lower confidence
            evidence: vec![
                format!("Function handles {} different responsibilities", responsibilities.len()),
                "Consider separating I/O, computation, and coordination".to_string(),
            ],
            metrics: PatternMetrics {
                severity: (responsibilities.len() as f32 / 4.0).min(1.0),
                refactoring_difficulty: Difficulty::Hard,
            },
        })
    } else {
        None
    }
}
```

#### 3. Integration with Recommendation System

```rust
/// Enhanced recommendation selection using patterns
pub fn select_template_with_patterns(
    debt_type: &DebtType,
    metrics: &FunctionMetrics,
    patterns: &[DetectedPattern],
) -> Option<RefactoringPattern> {
    // Prioritize detected patterns by confidence
    let best_pattern = patterns.iter()
        .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())?;

    match &best_pattern.pattern_type {
        ComplexityPattern::NestedConditionals { .. } =>
            Some(RefactoringPattern::NestedConditionals),

        ComplexityPattern::LongParameterList { .. } =>
            Some(RefactoringPattern::ParameterObject),

        ComplexityPattern::ComplexBooleanExpression { .. } =>
            Some(RefactoringPattern::ExtractPredicate),

        ComplexityPattern::MultipleResponsibilities { types } => {
            if types.contains(&ResponsibilityType::IO) {
                Some(RefactoringPattern::SeparateIOFromLogic)
            } else {
                Some(RefactoringPattern::ExtractFunction)
            }
        }

        _ => None, // Fall back to heuristics from 138b
    }
}
```

#### 4. Feature Flag Integration

```rust
/// Configuration for pattern detection
#[derive(Debug, Clone)]
pub struct PatternDetectionConfig {
    pub enabled: bool,
    pub rust_ast_analysis: bool,
    pub python_ast_analysis: bool,
    pub max_analysis_time_ms: u64,
}

impl Default for PatternDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rust_ast_analysis: true,
            python_ast_analysis: false,
            max_analysis_time_ms: 10,
        }
    }
}

/// Detect patterns with timeout and graceful fallback
pub fn detect_patterns_safe(
    func: &syn::ItemFn,
    metrics: &FunctionMetrics,
    config: &PatternDetectionConfig,
) -> Vec<DetectedPattern> {
    if !config.enabled || !config.rust_ast_analysis {
        return Vec::new(); // Fall back to heuristics
    }

    // TODO: Add timeout mechanism
    match std::panic::catch_unwind(|| detect_patterns(func, metrics)) {
        Ok(patterns) => patterns,
        Err(_) => {
            eprintln!("Pattern detection failed, falling back to heuristics");
            Vec::new()
        }
    }
}
```

### Performance Considerations

**Optimization Strategy**:
1. Reuse existing AST (don't reparse)
2. Lightweight visitors (minimal traversal)
3. Early exit when pattern found
4. Cache results per function
5. Optional feature (can be disabled)

**Benchmark Targets**:
- Simple function (<10 LOC): <1ms
- Medium function (10-50 LOC): <5ms
- Complex function (>50 LOC): <10ms

## Dependencies

**Prerequisites**:
- Spec 138a (Concise Recommendations) - Must be implemented
- Spec 138b (Template-Based Examples) - Must be implemented and validated

**Affected Components**:
- `src/recommendations/patterns/` - New module for pattern detection
- `src/analyzers/rust/complexity.rs` - Integration point
- `src/priority/scoring/recommendation.rs` - Use detected patterns

**External Dependencies**:
- `syn` (already in use) - AST traversal
- `quote` (optional) - Code generation (future)

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_detect_long_parameters() {
    let code = r#"
        fn process(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) -> i32 {
            a + b + c + d + e + f
        }
    "#;

    let func = parse_function(code);
    let pattern = detect_long_parameters(&func);

    assert!(pattern.is_some());
    let p = pattern.unwrap();
    assert!(matches!(
        p.pattern_type,
        ComplexityPattern::LongParameterList { count: 6 }
    ));
}

#[test]
fn test_detect_complex_boolean() {
    let code = r#"
        fn is_valid(a: bool, b: bool, c: bool, d: bool) -> bool {
            a && b && c && d && !e
        }
    "#;

    let func = parse_function(code);
    let pattern = detect_complex_booleans(&func);

    assert!(pattern.is_some());
}

#[test]
fn test_detect_multiple_responsibilities() {
    let code = r#"
        fn process_file(path: &Path) -> Result<i32> {
            let content = fs::read_to_string(path)?; // I/O
            let lines = content.lines().count(); // Computation
            println!("Processed {} lines", lines); // I/O
            Ok(lines as i32)
        }
    "#;

    let func = parse_function(code);
    let metrics = create_test_metrics();
    let pattern = detect_multiple_responsibilities(&func, &metrics);

    assert!(pattern.is_some());
    let p = pattern.unwrap();
    if let ComplexityPattern::MultipleResponsibilities { types } = p.pattern_type {
        assert!(types.contains(&ResponsibilityType::IO));
        assert!(types.contains(&ResponsibilityType::Computation));
    } else {
        panic!("Expected MultipleResponsibilities pattern");
    }
}

#[test]
fn test_graceful_fallback_on_error() {
    let config = PatternDetectionConfig::default();
    // Test with malformed AST or panic-inducing code
    // Should return empty vec, not crash
}
```

### Performance Benchmarks

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_pattern_detection(c: &mut Criterion) {
    let code = include_str!("../tests/fixtures/complex_function.rs");
    let func = parse_function(code);
    let metrics = calculate_metrics(&func);

    c.bench_function("detect_patterns", |b| {
        b.iter(|| {
            detect_patterns(black_box(&func), black_box(&metrics))
        })
    });
}

fn bench_pattern_detection_simple(c: &mut Criterion) {
    let code = "fn simple(x: i32) -> i32 { x + 1 }";
    let func = parse_function(code);
    let metrics = calculate_metrics(&func);

    c.bench_function("detect_patterns_simple", |b| {
        b.iter(|| {
            detect_patterns(black_box(&func), black_box(&metrics))
        })
    });
}

criterion_group!(benches, bench_pattern_detection, bench_pattern_detection_simple);
criterion_main!(benches);
```

### Validation Corpus

Build test corpus from real codebases:
- 50 functions with known patterns (manually labeled)
- Measure detection accuracy
- Target: 90%+ precision and recall

## Documentation Requirements

### Code Documentation

- Document each pattern detector with examples
- Explain confidence scoring rationale
- Provide guidelines for adding new patterns

### User Documentation

Update README:
```markdown
## Advanced Pattern Detection (Optional)

Enable advanced pattern detection for more accurate recommendations:

```toml
[analysis]
pattern_detection = true
```

Detects:
- Long parameter lists
- Complex boolean expressions
- Multiple responsibilities
- State management complexity

Disable if analysis is too slow for your use case.
```

## Success Metrics

**Validation Phase** (before implementation):
- User survey shows >70% want better pattern detection
- Analysis shows >30% false positives with current heuristics

**Implementation Phase** (if validated):
- 90%+ accuracy on validation corpus
- <10ms overhead per function
- <5% increase in memory usage
- Integration tests show improved recommendations

## Migration and Compatibility

### Backward Compatibility

**Fully Backward Compatible**:
- Feature flag defaults to enabled
- Falls back to heuristics on error
- No changes to JSON output structure
- Optional enhancement, not required

### Gradual Rollout

**Phase 1**: Implement Rust-only pattern detection
**Phase 2**: Validate with real users (beta flag)
**Phase 3**: Enable by default if validated
**Phase 4**: Consider Python/JS/TS (only if Rust successful)

## Implementation Notes

### Why This is Optional

Pattern detection adds complexity and potential performance overhead. Should only be implemented if:

1. **User Demand**: Users explicitly request better pattern detection
2. **Measurable Gap**: Current heuristics have significant false positives
3. **Performance Budget**: Can maintain <10ms overhead
4. **Maintenance Capacity**: Team has bandwidth for AST analysis

**Do NOT implement** if:
- Current heuristics work well enough
- Performance budget exceeded
- No user demand validated

### Alternative: Improve Heuristics First

Before implementing AST analysis, try improving heuristics:

```rust
// Better heuristic: detect long params from function signature length
fn has_long_params_heuristic(metrics: &FunctionMetrics) -> bool {
    // If function name line is very long, likely many params
    metrics.name.len() > 50 // Crude but fast
}

// Better heuristic: detect I/O from function name
fn likely_io_function(name: &str) -> bool {
    name.contains("read") || name.contains("write")
        || name.contains("load") || name.contains("save")
        || name.contains("fetch") || name.contains("send")
}
```

Improved heuristics are **much cheaper** than AST analysis.

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Performance regression | 🟡 Medium | Benchmarks, feature flag, timeout |
| Maintenance burden | 🟡 Medium | Limit to Rust initially, pure functions |
| False positives | 🟡 Medium | Confidence scores, validation corpus |
| Scope creep | 🟡 Medium | Strict acceptance criteria, optional feature |
| Low user value | 🟡 Medium | Validate before implementing |

## Approval Checklist

**Before Implementation**:
- [ ] User validation shows demand (survey, feedback)
- [ ] Analysis shows current heuristics insufficient
- [ ] Product manager approves scope and priority
- [ ] Team has bandwidth for AST analysis
- [ ] Performance budget allocated

**After Implementation**:
- [ ] All acceptance criteria met
- [ ] Performance benchmarks pass (<10ms)
- [ ] Validation corpus shows 90%+ accuracy
- [ ] Integration tests pass
- [ ] Documentation complete
- [ ] Feature flag works correctly

## Related Specifications

- **Spec 138a**: Concise Recommendations (prerequisite)
- **Spec 138b**: Template-Based Examples (prerequisite)
- **Spec 137**: Call Graph Analysis (complementary)

## Conclusion

**This spec should only be implemented if validated by users.** Start with Spec 138a and 138b, measure impact, gather feedback, then decide if 138c is needed.

**Expected Timeline**:
- Validation Phase: 2-4 weeks (user feedback, false positive analysis)
- Implementation Phase: 4-6 weeks (if validated)
- Total: 6-10 weeks (but may never happen)

**Decision Point**: After 138a and 138b are live for 1 month, review:
1. User feedback - do they want more?
2. False positive rate - is it high enough to justify?
3. Performance impact - can we afford 10ms overhead?

If all three are YES, proceed with 138c. Otherwise, mark as "Deferred" or "Won't Implement."
