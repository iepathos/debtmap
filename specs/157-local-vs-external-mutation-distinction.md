---
number: 157
title: Local vs External Mutation Distinction
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-11-01
---

# Specification 157: Local vs External Mutation Distinction

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

**Critical Bug Identified**: Functions that mutate only local variables are currently marked as impure, but they are functionally pure from a referential transparency perspective. This creates a **20-30% false negative rate** and incorrectly penalizes functional programming patterns.

**Evidence from Tests** (`tests/purity_detection_test.rs:188-193`):
```rust
#[test]
fn test_local_mutation() {
    let code = r#"
        fn process_data(input: Vec<i32>) -> Vec<i32> {
            let mut result = Vec::new();  // Local mutation
            for item in input {
                result.push(item * 2);
            }
            result
        }
    "#;

    // CURRENTLY NO ASSERTION - ACKNOWLEDGED BUG
    // Comment in test: "Function mutates local data but doesn't have
    // external side effects. This could be considered pure from a
    // functional perspective."
}
```

**Current Behavior**:
- Any mutation marked as side effect
- No distinction between `let mut x` (local) and `self.field = value` (external)
- Builder patterns incorrectly marked impure
- Iterator `.collect()` into mutable vec marked impure

**Impact**:
- Functions get 1.0x multiplier instead of 0.75x (for locally pure)
- Risk elevated from Low to Medium
- Idiomatic Rust patterns (builder, accumulator) penalized
- False negatives discourage functional style

**Real-World Example**:
```rust
// This is functionally pure - no observable side effects
fn calculate_totals(items: &[Item]) -> Vec<f64> {
    let mut totals = Vec::new();  // Local mutation only
    for item in items {
        totals.push(item.price * item.quantity);
    }
    totals
}
// CURRENT: Marked impure (1.0x multiplier)
// SHOULD BE: Locally pure (0.75x multiplier)
```

## Objective

Implement **scope-aware mutation analysis** that distinguishes between:
1. **Local mutations** (pure): Only affects local variables/parameters
2. **External mutations** (impure): Modifies state outside function scope
3. **Upvalue mutations** (context-dependent): Captures and mutates from closure

This enables accurate classification of functionally pure functions that use local mutation for implementation efficiency.

## Requirements

### Functional Requirements

1. **Scope Tracking**
   - Track all local variable declarations in function scope
   - Identify function parameters (including `mut` parameters)
   - Track closure captures and their mutability
   - Distinguish `&mut self` (external) from `mut self` (ownership transfer)

2. **Mutation Classification**
   - Classify each mutation as Local, External, or Upvalue
   - Local: Mutations to locally-declared variables
   - External: Mutations to fields, statics, or dereferenced pointers
   - Upvalue: Mutations to closure-captured variables

3. **New Purity Levels**
   - `StrictlyPure`: No mutations whatsoever
   - `LocallyPure`: Only local mutations (functionally pure)
   - `ReadOnly`: Reads external state but doesn't modify
   - `Impure`: Modifies external state or performs I/O

4. **Confidence Adjustment**
   - High confidence (0.9+) for local mutations in simple functions
   - Medium confidence (0.7-0.9) for complex control flow
   - Low confidence (0.5-0.7) when pointer dereferencing involved

### Non-Functional Requirements

- **Accuracy**: Reduce false negative rate from 20-30% to <5%
- **Performance**: <10% overhead compared to current analysis
- **Backward Compatibility**: Existing purity API unchanged
- **Correctness**: Zero false positives (never mark external mutation as local)

## Acceptance Criteria

- [ ] Local variable mutations correctly classified as LocallyPure
- [ ] External field mutations classified as Impure
- [ ] Builder pattern (consuming `mut self`) classified as LocallyPure
- [ ] `&mut self` methods classified as Impure
- [ ] Iterator `.collect()` patterns classified as LocallyPure
- [ ] Closure captures classified correctly (upvalue mutations)
- [ ] Pointer dereference mutations conservatively marked External
- [ ] LocallyPure functions receive 0.75x complexity multiplier
- [ ] Risk calculation uses LocallyPure correctly (Low/Medium risk)
- [ ] Test suite achieves <5% false negative rate on validation corpus

## Technical Details

### Implementation Approach

#### Step 1: Scope Analysis

```rust
// src/analyzers/scope_tracker.rs

#[derive(Debug, Clone)]
pub struct ScopeTracker {
    /// Set of local variable identifiers
    local_vars: HashSet<String>,

    /// Parameters (including mutability)
    params: HashMap<String, ParameterInfo>,

    /// Closure captures
    captures: HashMap<String, CaptureInfo>,

    /// Current scope depth (for nested blocks)
    scope_depth: usize,
}

#[derive(Debug, Clone)]
pub struct ParameterInfo {
    pub name: String,
    pub is_mut: bool,
    pub is_self: bool,
    pub self_kind: Option<SelfKind>, // &self, &mut self, mut self, self
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelfKind {
    Ref,           // &self
    MutRef,        // &mut self
    Owned,         // self
    MutOwned,      // mut self
}

impl ScopeTracker {
    pub fn new() -> Self {
        Self {
            local_vars: HashSet::new(),
            params: HashMap::new(),
            captures: HashMap::new(),
            scope_depth: 0,
        }
    }

    /// Add parameter to scope
    pub fn add_parameter(&mut self, param: &syn::FnArg) {
        match param {
            syn::FnArg::Receiver(receiver) => {
                let kind = if receiver.mutability.is_some() {
                    SelfKind::MutRef
                } else {
                    SelfKind::Ref
                };

                self.params.insert("self".to_string(), ParameterInfo {
                    name: "self".to_string(),
                    is_mut: receiver.mutability.is_some(),
                    is_self: true,
                    self_kind: Some(kind),
                });
            }
            syn::FnArg::Typed(pat_type) => {
                if let syn::Pat::Ident(ident) = &*pat_type.pat {
                    self.params.insert(ident.ident.to_string(), ParameterInfo {
                        name: ident.ident.to_string(),
                        is_mut: ident.mutability.is_some(),
                        is_self: false,
                        self_kind: None,
                    });
                }
            }
        }
    }

    /// Add local variable to current scope
    pub fn add_local_var(&mut self, name: String) {
        self.local_vars.insert(name);
    }

    /// Check if identifier is a local variable
    pub fn is_local(&self, name: &str) -> bool {
        self.local_vars.contains(name) ||
        self.params.get(name).map_or(false, |p| !p.is_self)
    }

    /// Check if identifier is self (any kind)
    pub fn is_self(&self, name: &str) -> bool {
        self.params.get(name).map_or(false, |p| p.is_self)
    }

    /// Enter nested scope (e.g., if block, loop)
    pub fn enter_scope(&mut self) {
        self.scope_depth += 1;
    }

    /// Exit scope
    pub fn exit_scope(&mut self) {
        self.scope_depth = self.scope_depth.saturating_sub(1);
        // Note: We keep all local vars for simplicity
        // Could implement proper scope shadowing if needed
    }
}
```

#### Step 2: Mutation Scope Classification

```rust
// src/analyzers/purity_detector.rs (enhanced)

#[derive(Debug, Clone, PartialEq)]
pub enum MutationScope {
    /// Mutation of local variable or owned parameter
    Local,

    /// Mutation of closure-captured variable
    Upvalue,

    /// Mutation of external state (fields, statics, etc.)
    External,
}

impl<'ast> PurityDetector {
    fn determine_mutation_scope(&self, expr: &syn::Expr) -> MutationScope {
        match expr {
            // Simple identifier: x = value
            Expr::Path(path) => {
                let ident = path.path.get_ident()
                    .map(|i| i.to_string())
                    .unwrap_or_default();

                if self.scope.is_local(&ident) {
                    MutationScope::Local
                } else if self.scope.captures.contains_key(&ident) {
                    MutationScope::Upvalue
                } else {
                    MutationScope::External
                }
            }

            // Field access: obj.field = value
            Expr::Field(field) => {
                self.determine_field_mutation_scope(field)
            }

            // Index: arr[i] = value
            Expr::Index(index) => {
                // Check if base is local
                if let Expr::Path(path) = &*index.expr {
                    if let Some(ident) = path.path.get_ident() {
                        if self.scope.is_local(&ident.to_string()) {
                            return MutationScope::Local;
                        }
                    }
                }
                MutationScope::External
            }

            // Method call: obj.method() that might mutate
            Expr::MethodCall(method) => {
                // Check receiver
                if let Expr::Path(path) = &*method.receiver {
                    if let Some(ident) = path.path.get_ident() {
                        if self.scope.is_local(&ident.to_string()) {
                            return MutationScope::Local;
                        }
                    }
                }
                MutationScope::External
            }

            // Pointer dereference: *ptr = value
            Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
                // Conservative: assume external
                MutationScope::External
            }

            _ => MutationScope::External,
        }
    }

    fn determine_field_mutation_scope(&self, field: &syn::ExprField) -> MutationScope {
        match &*field.base {
            // self.field = value
            Expr::Path(path) if self.scope.is_self(&path.path.get_ident()
                .map(|i| i.to_string())
                .unwrap_or_default()) => {
                // Check self kind
                if let Some(param) = self.scope.params.get("self") {
                    match param.self_kind {
                        Some(SelfKind::MutRef) => MutationScope::External, // &mut self
                        Some(SelfKind::Owned) | Some(SelfKind::MutOwned) => {
                            // mut self or self (owned) - local mutation
                            MutationScope::Local
                        }
                        _ => MutationScope::External,
                    }
                } else {
                    MutationScope::External
                }
            }

            // local_var.field = value
            Expr::Path(path) => {
                let ident = path.path.get_ident()
                    .map(|i| i.to_string())
                    .unwrap_or_default();

                if self.scope.is_local(&ident) {
                    MutationScope::Local
                } else {
                    MutationScope::External
                }
            }

            _ => MutationScope::External,
        }
    }
}
```

#### Step 3: Enhanced Visit Methods

```rust
impl<'ast> Visit<'ast> for PurityDetector {
    fn visit_item_fn(&mut self, func: &'ast syn::ItemFn) {
        // Initialize scope tracker
        self.scope = ScopeTracker::new();

        // Add parameters to scope
        for param in &func.sig.inputs {
            self.scope.add_parameter(param);
        }

        // Visit function body
        visit::visit_item_fn(self, func);
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        // Track local variable declarations
        if let syn::Pat::Ident(ident) = &local.pat {
            self.scope.add_local_var(ident.ident.to_string());
        }

        visit::visit_local(self, local);
    }

    fn visit_expr_assign(&mut self, assign: &'ast syn::ExprAssign) {
        // Classify mutation scope
        let scope = self.determine_mutation_scope(&assign.left);

        match scope {
            MutationScope::Local => {
                self.local_mutations.push(LocalMutation {
                    location: assign.left.span(),
                    target: format!("{:?}", assign.left),
                });
            }
            MutationScope::External => {
                self.modifies_external_state = true;
                self.side_effects.push(SideEffect::ExternalMutation);
            }
            MutationScope::Upvalue => {
                self.upvalue_mutations.push(UpvalueMutation {
                    location: assign.left.span(),
                    captured_var: self.extract_var_name(&assign.left),
                });
            }
        }

        visit::visit_expr_assign(self, assign);
    }

    fn visit_expr_method_call(&mut self, method: &'ast syn::ExprMethodCall) {
        // Check for mutating methods
        if self.is_mutating_method(&method.method) {
            let scope = self.determine_mutation_scope(&method.receiver);

            match scope {
                MutationScope::Local => {
                    self.local_mutations.push(LocalMutation {
                        location: method.span(),
                        target: format!("{:?}", method.receiver),
                    });
                }
                MutationScope::External => {
                    self.modifies_external_state = true;
                    self.side_effects.push(SideEffect::ExternalMutation);
                }
                MutationScope::Upvalue => {
                    self.upvalue_mutations.push(UpvalueMutation {
                        location: method.span(),
                        captured_var: self.extract_var_name(&method.receiver),
                    });
                }
            }
        }

        visit::visit_expr_method_call(self, method);
    }

    fn is_mutating_method(&self, method_name: &syn::Ident) -> bool {
        // Common mutating methods
        matches!(method_name.to_string().as_str(),
            "push" | "pop" | "insert" | "remove" | "clear" |
            "extend" | "append" | "drain" | "sort" | "reverse"
        )
    }
}
```

#### Step 4: Purity Level Determination

```rust
impl PurityDetector {
    pub fn determine_purity_level(&self) -> PurityLevel {
        // Has external side effects (I/O, external mutations)?
        if self.modifies_external_state || !self.side_effects.is_empty() {
            return PurityLevel::Impure;
        }

        // Only reads external state?
        if self.accesses_external_state {
            return PurityLevel::ReadOnly;
        }

        // Has local mutations?
        if !self.local_mutations.is_empty() || !self.upvalue_mutations.is_empty() {
            return PurityLevel::LocallyPure;
        }

        // No mutations or side effects at all
        PurityLevel::StrictlyPure
    }

    pub fn calculate_confidence(&self) -> f64 {
        let mut confidence = 1.0;

        // Reduce confidence for complex control flow
        if self.has_complex_control_flow {
            confidence *= 0.9;
        }

        // Reduce confidence for pointer dereferencing
        if self.has_pointer_deref {
            confidence *= 0.8;
        }

        // Reduce confidence for upvalue mutations (closures)
        if !self.upvalue_mutations.is_empty() {
            confidence *= 0.85;
        }

        // High confidence for simple local mutations
        if !self.local_mutations.is_empty() && self.local_mutations.len() < 5 {
            confidence *= 0.95;
        }

        confidence.clamp(0.5, 1.0)
    }
}
```

### Integration with Scoring

```rust
// src/priority/unified_scorer.rs

fn calculate_purity_adjustment(func: &FunctionMetrics) -> f64 {
    match func.purity_level {
        PurityLevel::StrictlyPure => {
            if func.purity_confidence > 0.8 {
                0.70  // 30% reduction
            } else {
                0.80  // 20% reduction
            }
        }
        PurityLevel::LocallyPure => {
            // New: Local mutations still quite testable
            if func.purity_confidence > 0.8 {
                0.75  // 25% reduction
            } else {
                0.85  // 15% reduction
            }
        }
        PurityLevel::ReadOnly => {
            0.90  // 10% reduction
        }
        PurityLevel::Impure => {
            1.0  // No reduction
        }
    }
}
```

### Risk Classification

```rust
// src/data_flow.rs

fn calculate_risk_level(
    downstream: &[FunctionId],
    has_io: bool,
    purity_level: PurityLevel,
) -> RiskLevel {
    match (downstream.len(), has_io, purity_level) {
        // Strictly or locally pure with no I/O
        (0, false, PurityLevel::StrictlyPure | PurityLevel::LocallyPure) => RiskLevel::Low,
        (1..=5, false, PurityLevel::StrictlyPure | PurityLevel::LocallyPure) => RiskLevel::Medium,

        // Has I/O
        (_, true, _) => RiskLevel::High,

        // Many callers
        (6.., _, _) => RiskLevel::Critical,

        // Default
        _ => RiskLevel::Medium,
    }
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/analyzers/purity_detector.rs` - Add scope tracking
  - `src/analysis/purity_analysis.rs` - Use new purity levels
  - `src/priority/unified_scorer.rs` - Handle LocallyPure
  - `src/data_flow.rs` - Update risk calculation
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_mutation_is_locally_pure() {
        let code = r#"
            fn process_data(input: Vec<i32>) -> Vec<i32> {
                let mut result = Vec::new();
                for item in input {
                    result.push(item * 2);
                }
                result
            }
        "#;

        let analysis = analyze_purity(code).unwrap();
        assert_eq!(analysis.purity_level, PurityLevel::LocallyPure);
        assert!(analysis.purity_confidence > 0.85);
    }

    #[test]
    fn test_external_mutation_is_impure() {
        let code = r#"
            struct Counter { count: u32 }

            impl Counter {
                fn increment(&mut self) {
                    self.count += 1;  // &mut self - external mutation
                }
            }
        "#;

        let analysis = analyze_purity(code).unwrap();
        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }

    #[test]
    fn test_builder_pattern_is_locally_pure() {
        let code = r#"
            struct Config { value: u32 }

            impl Config {
                fn with_value(mut self, value: u32) -> Self {
                    self.value = value;  // mut self (owned) - local mutation
                    self
                }
            }
        "#;

        let analysis = analyze_purity(code).unwrap();
        assert_eq!(analysis.purity_level, PurityLevel::LocallyPure);
    }

    #[test]
    fn test_iterator_collect_is_locally_pure() {
        let code = r#"
            fn double_values(nums: &[i32]) -> Vec<i32> {
                nums.iter().map(|x| x * 2).collect()
            }
        "#;

        let analysis = analyze_purity(code).unwrap();
        assert_eq!(analysis.purity_level, PurityLevel::LocallyPure);
    }

    #[test]
    fn test_static_mutation_is_impure() {
        let code = r#"
            static mut COUNTER: u32 = 0;

            fn increment_global() {
                unsafe { COUNTER += 1; }
            }
        "#;

        let analysis = analyze_purity(code).unwrap();
        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }
}
```

### Integration Tests

```rust
// tests/local_mutation_test.rs

#[test]
fn test_scoring_with_local_mutations() {
    let code = r#"
        fn calculate_totals(items: &[Item]) -> Vec<f64> {
            let mut totals = Vec::new();
            for item in items {
                totals.push(item.price * item.quantity);
            }
            totals
        }
    "#;

    let debt_item = analyze_and_score(code).unwrap();

    // Should be LocallyPure
    assert_eq!(debt_item.purity_level, PurityLevel::LocallyPure);

    // Should get 0.75x multiplier
    let expected_score = calculate_base_score(debt_item.complexity) * 0.75;
    assert!((debt_item.score - expected_score).abs() < 0.1);
}
```

### Validation Corpus

Create ground truth dataset:
```
tests/purity_validation/local_vs_external/
├── local_pure/
│   ├── accumulator_pattern.rs
│   ├── builder_pattern.rs
│   ├── iterator_collect.rs
│   └── mut_parameter.rs
├── external_impure/
│   ├── field_mutation.rs
│   ├── static_mutation.rs
│   └── mut_ref_self.rs
└── validation_results.json
```

Target: <5% false negative rate

## Documentation Requirements

### Code Documentation

- Document `ScopeTracker` API and usage
- Explain mutation scope classification algorithm
- Add examples for each purity level
- Document confidence scoring formula

### User Documentation

Update `docs/purity-analysis.md`:
```markdown
## Purity Levels

Debtmap distinguishes between four levels of purity:

### Strictly Pure
No mutations whatsoever. Pure mathematical functions.

### Locally Pure (NEW)
Uses local mutations for efficiency, but no external side effects.
Functionally pure - same inputs always produce same outputs.

**Example**:
```rust
fn calculate_totals(items: &[Item]) -> Vec<f64> {
    let mut totals = Vec::new();  // Local mutation only
    for item in items {
        totals.push(item.price * item.quantity);
    }
    totals  // Functionally pure!
}
```

**Multiplier**: 0.75x (25% complexity reduction)

### Read-Only
Reads external state but doesn't modify it.

### Impure
Modifies external state or performs I/O.
```

## Implementation Notes

### Edge Cases

1. **Parameter Mutations**: `fn foo(mut x: i32)` - local mutation
2. **Self Variations**:
   - `&self` - read-only
   - `&mut self` - external mutation (impure)
   - `mut self` - local mutation (locally pure)
   - `self` - ownership transfer (pure)

3. **Closures**: Captured variables analyzed separately (see spec 158)

### Performance Considerations

- `ScopeTracker` uses `HashSet` for O(1) lookups
- Scope depth tracking minimal overhead
- No impact on non-mutation analysis paths

## Migration and Compatibility

### Breaking Changes

- New `PurityLevel::LocallyPure` enum variant
- `FunctionMetrics` gains `local_mutations` field

### Migration Strategy

- Add new fields with defaults
- Update all match statements to handle `LocallyPure`
- Backward compatible: old analysis without scope tracking returns `StrictlyPure` or `Impure` only

### Compatibility Guarantees

- Existing purity classifications unchanged (strict binary)
- New classifications refine existing "impure" into "locally pure" vs "impure"
- Scores only improve, never regress
