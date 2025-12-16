---
number: 213
title: Pure Function Method Weighting in God Object Detection
category: optimization
priority: high
status: draft
dependencies: [209, 211]
created: 2025-12-15
---

# Specification 213: Pure Function Method Weighting in God Object Detection

**Category**: optimization
**Priority**: high (P0)
**Status**: draft
**Dependencies**: Spec 209 (Accessor Detection), Spec 211 (Method Complexity Weighting)

## Context

The current God Object detection counts all methods equally, treating associated functions (that don't reference `self`) the same as instance methods. This causes false positives for modules that follow functional programming principles by decomposing logic into many small pure helper functions.

### Current Problem

```rust
pub struct CallResolver<'a> {
    call_graph: &'a CallGraph,
    current_file: &'a PathBuf,
    function_index: HashMap<String, Vec<FunctionId>>,
}

impl<'a> CallResolver<'a> {
    // Instance method using self (1.0 weight - correct)
    pub fn resolve_call(&self, call: &UnresolvedCall) -> Option<FunctionId> {
        // Uses self.function_index, self.current_file
    }

    // Associated function NOT using self (should have reduced weight)
    pub fn normalize_path_prefix(name: &str) -> String {
        Self::strip_generic_params(name)
    }

    // Pure helper NOT using self (should have reduced weight)
    fn is_exact_match(func_name: &str, search_name: &str) -> bool {
        func_name == search_name
    }

    // Pure helper NOT using self (should have reduced weight)
    fn is_qualified_match(func_name: &str, search_name: &str) -> bool {
        func_name.ends_with(&format!("::{}", search_name))
    }

    // ... 15+ more pure helper functions
}

// Current scoring: 24 methods = score of ~100 (CRITICAL)
// Desired scoring: 3 instance methods + 21 pure helpers = ~8 effective methods (LOW)
```

### Why This Matters

Functional programming best practices encourage:
1. **Decomposing logic into small pure functions** - easier to test, reason about
2. **Grouping related pure functions together** - colocation for discoverability
3. **Instance methods that compose pure functions** - clear separation of concerns

The current heuristics penalize this pattern, encouraging developers to either:
- Keep logic in fewer, larger methods (worse)
- Move pure functions to module level (sometimes artificial separation)
- Ignore god object warnings (alert fatigue)

### Real-World False Positive

The `CallResolver` struct in debtmap's own codebase:
- 24 methods flagged as god object with score 100
- 3 fields only (minimal state)
- 21 of 24 methods are pure functions that don't use `self`
- Single responsibility: resolving function calls
- Well-tested with 250+ lines of tests

This is **exemplary functional design** being flagged as critical debt.

## Objective

Implement detection and reduced weighting of pure/associated functions to:
1. Identify methods that don't reference `self` (associated functions)
2. Apply reduced weight to pure methods in God Object scoring
3. Provide visibility into "instance vs pure" method breakdown

## Requirements

### Functional Requirements

1. **Self-Reference Detection**: Determine if a method references `self`:
   - `&self` or `&mut self` as first parameter
   - `self.field` access in method body
   - `self.method()` calls in method body

2. **Method Classification Extension**: Add to existing classification (Spec 209):
   - `PureAssociated`: No `self` parameter, no `self` references
   - `StaticHelper`: Same as `PureAssociated` but callable without instance
   - `InstanceMethod`: Has `self` parameter and uses instance state

3. **Weighted Scoring Integration**:
   - PureAssociated/StaticHelper: 0.2 weight (significant reduction)
   - InstanceMethod: 1.0 weight (full count)
   - Combine with Spec 209 accessor weighting

4. **Reporting Enhancement**:
   - Show breakdown: "24 methods (3 instance, 21 pure)"
   - Display effective weighted count vs raw count
   - Flag when >50% of methods are pure (may indicate cohesive functional design)

### Non-Functional Requirements

- Detection must be fast (single AST pass)
- Must produce deterministic results
- Should integrate with existing method classification infrastructure

## Acceptance Criteria

- [ ] Methods without `self` parameter are detected as `PureAssociated`
- [ ] Methods with `self` parameter but no body references are detected as `PureAssociated`
- [ ] Pure associated methods receive 0.2 weight in god object scoring
- [ ] `CallResolver` with 24 methods (21 pure) scores as ~6.2 effective methods, not 24
- [ ] Output shows "24 methods (3 instance, 21 pure helpers)"
- [ ] Structs with >50% pure methods get cohesion bonus indicator
- [ ] Existing tests continue to pass
- [ ] New tests validate pure function detection accuracy

## Technical Details

### Implementation Approach

#### 1. Extend Method Classification

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum MethodSelfUsage {
    /// No self parameter, stateless function
    PureAssociated,
    /// Has self parameter but doesn't actually use it
    UnusedSelf,
    /// Has self parameter and uses instance state
    InstanceMethod,
}

impl MethodSelfUsage {
    pub fn weight(&self) -> f64 {
        match self {
            Self::PureAssociated => 0.2,  // Pure helpers barely count
            Self::UnusedSelf => 0.3,      // Slight reduction
            Self::InstanceMethod => 1.0,   // Full weight
        }
    }
}
```

#### 2. Detection in AST Visitor

```rust
// In ast_visitor.rs, enhance method analysis
pub fn classify_self_usage(method: &syn::ImplItemFn) -> MethodSelfUsage {
    // Check if method has self parameter
    let has_self_param = method.sig.inputs.iter().any(|arg| {
        matches!(arg, syn::FnArg::Receiver(_))
    });

    if !has_self_param {
        return MethodSelfUsage::PureAssociated;
    }

    // Check if self is actually used in the body
    let self_used = SelfUsageVisitor::visit_body(&method.block);

    if self_used {
        MethodSelfUsage::InstanceMethod
    } else {
        MethodSelfUsage::UnusedSelf
    }
}

struct SelfUsageVisitor {
    uses_self: bool,
}

impl<'ast> Visit<'ast> for SelfUsageVisitor {
    fn visit_expr_field(&mut self, field: &'ast syn::ExprField) {
        if let syn::Expr::Path(path) = &*field.base {
            if path.path.is_ident("self") {
                self.uses_self = true;
            }
        }
        syn::visit::visit_expr_field(self, field);
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        if let syn::Expr::Path(path) = &*call.receiver {
            if path.path.is_ident("self") {
                self.uses_self = true;
            }
        }
        syn::visit::visit_expr_method_call(self, call);
    }
}
```

#### 3. Combined Weighting Formula

```rust
/// Calculate combined weight for a method
///
/// Combines weights from:
/// - Spec 209: Accessor/boilerplate classification
/// - Spec 213: Self-usage classification (this spec)
/// - Spec 211: Complexity weighting
pub fn calculate_method_weight(
    accessor_class: MethodComplexityClass,
    self_usage: MethodSelfUsage,
    cyclomatic_complexity: u32,
) -> f64 {
    // Use minimum of accessor and self-usage weights
    // (a pure accessor should have very low weight)
    let base_weight = accessor_class.weight().min(self_usage.weight());

    // Apply complexity modifier (Spec 211)
    let complexity_factor = if cyclomatic_complexity <= 2 {
        0.8  // Simple functions get bonus
    } else if cyclomatic_complexity >= 10 {
        1.5  // Complex functions get penalty
    } else {
        1.0
    };

    base_weight * complexity_factor
}
```

#### 4. Output Format Enhancement

```
god object structure
  methods                   24 (3 instance, 21 pure helpers)
  effective methods         6.2
  fields                    3
  responsibilities          1 (Call Resolution)

recommendation
  action                    None - cohesive functional design detected
  rationale                 High pure method ratio (87.5%) indicates
                            intentional functional decomposition
```

### Detection Heuristics

| Pattern | Classification | Weight | Example |
|---------|---------------|--------|---------|
| No `self` param | PureAssociated | 0.2 | `fn helper(x: &str) -> bool` |
| `&self` + uses fields | InstanceMethod | 1.0 | `fn resolve(&self) -> T { self.field }` |
| `&self` + no usage | UnusedSelf | 0.3 | `fn debug(&self) { println!(...) }` |
| `&mut self` + mutates | InstanceMethod | 1.0 | `fn add(&mut self, x: T) { self.items.push(x) }` |

## Integration with Existing Specs

### Spec 209 (Accessor Detection)
- Combined weight calculation uses minimum of both weights
- Example: Pure accessor gets `min(0.1, 0.2) = 0.1` weight

### Spec 211 (Complexity Weighting)
- Complexity factor applied as multiplier after base weight
- Example: Pure simple function gets `0.2 * 0.8 = 0.16` weight

### Spec 208 (Domain-Aware Grouping)
- Pure methods still count toward responsibility grouping
- But reduced weight in god object scoring

## Dependencies

- **Prerequisites**:
  - Spec 209: Accessor detection infrastructure
  - AST visitor method analysis
- **Affected Components**:
  - `ast_visitor.rs`: Add self-usage detection
  - `scoring.rs`: Integrate pure function weight
  - `detector.rs`: Report instance/pure breakdown
  - `types.rs`: Add MethodSelfUsage enum

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_classify_pure_associated() {
    let code = r#"
        impl Foo {
            fn helper(x: &str) -> bool { x.is_empty() }
        }
    "#;
    let method = parse_method(code);
    assert_eq!(classify_self_usage(&method), MethodSelfUsage::PureAssociated);
}

#[test]
fn test_classify_instance_method() {
    let code = r#"
        impl Foo {
            fn get_data(&self) -> &Data { &self.data }
        }
    "#;
    let method = parse_method(code);
    assert_eq!(classify_self_usage(&method), MethodSelfUsage::InstanceMethod);
}

#[test]
fn test_classify_unused_self() {
    let code = r#"
        impl Foo {
            fn debug(&self) { println!("debug"); }
        }
    "#;
    let method = parse_method(code);
    assert_eq!(classify_self_usage(&method), MethodSelfUsage::UnusedSelf);
}
```

### Integration Tests

```rust
#[test]
fn test_call_resolver_not_flagged_as_god_object() {
    // Real-world test case
    let content = include_str!("../../src/analyzers/call_graph/call_resolution.rs");
    let analysis = analyze_file(content);

    let call_resolver = analysis.types.get("CallResolver").unwrap();

    // Should have 24 total methods
    assert_eq!(call_resolver.method_count, 24);

    // Most should be pure
    assert!(call_resolver.pure_method_count >= 20);

    // Effective weighted count should be low
    assert!(call_resolver.weighted_method_count < 10.0);

    // Should NOT be flagged as god object (or very low score)
    assert!(call_resolver.god_object_score < 30.0);
}
```

### Property Tests

```rust
proptest! {
    #[test]
    fn prop_pure_methods_always_lower_weight(method_count in 1..50usize) {
        let pure_weight: f64 = (0..method_count)
            .map(|_| MethodSelfUsage::PureAssociated.weight())
            .sum();
        let instance_weight: f64 = (0..method_count)
            .map(|_| MethodSelfUsage::InstanceMethod.weight())
            .sum();

        prop_assert!(pure_weight < instance_weight);
    }

    #[test]
    fn prop_combined_weight_bounded(
        accessor_weight in 0.0..=1.0f64,
        self_usage in prop::sample::select(vec![
            MethodSelfUsage::PureAssociated,
            MethodSelfUsage::UnusedSelf,
            MethodSelfUsage::InstanceMethod,
        ])
    ) {
        let weight = calculate_method_weight(accessor_weight, self_usage, 5);
        prop_assert!(weight >= 0.0 && weight <= 1.5);
    }
}
```

## Documentation Requirements

- **Code Documentation**: Document self-usage detection in ast_visitor.rs
- **User Documentation**: Explain how pure method weighting affects detection
- **Output Documentation**: Document new "instance/pure" breakdown in output

## Implementation Notes

1. **AST Traversal**: Self-usage detection requires visiting method body, not just signature
2. **Self via `Self::`**: Don't count `Self::associated_fn()` as self usage
3. **Nested Functions**: Don't count self in nested closures/functions
4. **Generic Self**: Handle `self: Box<Self>` patterns
5. **Trait Methods**: Pure trait default implementations should still be detected

## Migration and Compatibility

- Existing god object scores will change (generally lower for functional code)
- Add `--include-pure-methods` flag for backwards compatibility
- Show both raw and weighted counts in verbose output
- Consider gradual rollout via configuration

## Success Metrics

- `CallResolver` (24 methods, 21 pure) should score < 30 (not critical)
- Files with >70% pure methods should rarely exceed "medium" severity
- No regression in detecting actual god objects (low pure method ratio)
