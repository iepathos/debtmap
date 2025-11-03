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

### Functional Correctness

- [ ] Local variable mutations correctly classified as LocallyPure
- [ ] External field mutations classified as Impure
- [ ] Builder pattern (consuming `mut self`) classified as LocallyPure
- [ ] `&mut self` methods classified as Impure
- [ ] Iterator `.collect()` patterns classified as LocallyPure
- [ ] Closure captures conservatively marked as Upvalue/External
- [ ] Pointer dereference mutations conservatively marked External
- [ ] Parameter mutations (`mut x: T`) classified as LocallyPure

### Integration Requirements

- [ ] LocallyPure functions receive 0.75x complexity multiplier (high confidence) or 0.85x (medium confidence)
- [ ] Risk calculation treats LocallyPure same as StrictlyPure for risk levels
- [ ] `PurityAnalysis` includes both `is_pure` (deprecated) and `purity_level` fields
- [ ] `calculate_purity_adjustment()` handles both old and new fields with fallback
- [ ] Serialization/deserialization maintains backward compatibility

### Quality Gates

- [ ] **Baseline false negative rate measured** and documented
- [ ] **Target <5% false negative rate** achieved on validation corpus
- [ ] **Zero false positives**: No external mutations marked as local
- [ ] All existing purity tests still pass
- [ ] New test suite covers all four purity levels
- [ ] Performance overhead <10% average, <20% worst-case
- [ ] All clippy warnings resolved
- [ ] Documentation updated with examples

### Validation Corpus

- [ ] Ground truth corpus created (50+ functions)
- [ ] Manual classification by Rust expert completed
- [ ] Baseline measurement test passes
- [ ] Post-implementation validation shows improvement
- [ ] False positive rate verified as 0%

## Technical Details

### Implementation Approach

#### Integration with Existing PurityDetector

**Current Structure** (src/analyzers/purity_detector.rs):
```rust
pub struct PurityDetector {
    has_side_effects: bool,
    has_mutable_params: bool,
    has_io_operations: bool,
    has_unsafe_blocks: bool,
    accesses_external_state: bool,
    modifies_external_state: bool,
}
```

**Enhanced Structure** (adds scope tracking):
```rust
pub struct PurityDetector {
    // Existing fields - keep for backward compatibility
    has_side_effects: bool,
    has_mutable_params: bool,
    has_io_operations: bool,
    has_unsafe_blocks: bool,
    accesses_external_state: bool,
    modifies_external_state: bool,

    // NEW: Scope-aware mutation tracking
    scope: ScopeTracker,
    local_mutations: Vec<LocalMutation>,
    upvalue_mutations: Vec<UpvalueMutation>,
}

#[derive(Debug, Clone)]
pub struct LocalMutation {
    pub location: proc_macro2::Span,
    pub target: String,
}

#[derive(Debug, Clone)]
pub struct UpvalueMutation {
    pub location: proc_macro2::Span,
    pub captured_var: String,
}
```

**Changes to `PurityAnalysis` return type**:
```rust
pub struct PurityAnalysis {
    pub is_pure: bool,  // DEPRECATED: Keep for compatibility
    pub purity_level: PurityLevel,  // NEW: Refined classification
    pub reasons: Vec<ImpurityReason>,
    pub confidence: f32,
}
```

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
- **Related Specifications**:
  - Spec 158 (Closure Analysis): Referenced but NOT a prerequisite. Upvalue mutations will be handled conservatively (marked as External) until spec 158 is implemented.
- **Affected Components**:
  - `src/analyzers/purity_detector.rs` - Add scope tracking and integrate `ScopeTracker`
  - `src/core/mod.rs` - Add `PurityLevel` enum and update `FunctionMetrics`
  - `src/priority/unified_scorer.rs` - Update `calculate_purity_adjustment()` to handle `LocallyPure`
  - (Optional) `src/data_flow.rs` - Update risk calculation if present
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

**CRITICAL: Establish Baseline Before Implementation**

#### Step 1: Create Ground Truth Dataset

```
tests/purity_validation/local_vs_external/
├── local_pure/
│   ├── accumulator_pattern.rs        # let mut vec; vec.push()
│   ├── builder_pattern.rs            # mut self builder chains
│   ├── iterator_collect.rs           # .map().collect()
│   ├── mut_parameter.rs              # fn foo(mut x: T)
│   └── owned_self_mutation.rs        # self.field = x with ownership
├── external_impure/
│   ├── field_mutation.rs             # &mut self methods
│   ├── static_mutation.rs            # static mut access
│   ├── mut_ref_self.rs               # impl methods with &mut self
│   └── global_state.rs               # thread_local!, lazy_static!
├── strictly_pure/
│   ├── pure_arithmetic.rs            # No mutations at all
│   ├── pure_recursion.rs             # Recursive pure functions
│   └── immutable_transforms.rs       # .map() without collect
├── read_only/
│   ├── const_access.rs               # Reading const values
│   └── immutable_field_access.rs     # Reading &self fields
└── ground_truth.json                 # Manual classifications
```

#### Step 2: Establish Baseline (REQUIRED)

Before any implementation, run current purity detector on corpus:

```rust
// tests/baseline_measurement.rs
#[test]
fn measure_current_false_negative_rate() {
    let ground_truth = load_ground_truth("tests/purity_validation/ground_truth.json");
    let mut false_negatives = 0;
    let mut total = 0;

    for case in ground_truth {
        let current_result = analyze_with_current_detector(&case.code);
        let expected = case.expected_purity_level;

        if current_result.is_pure && expected == PurityLevel::Impure {
            false_negatives += 1;
        }
        total += 1;
    }

    let rate = (false_negatives as f64 / total as f64) * 100.0;
    println!("Current false negative rate: {:.1}%", rate);
    // Store baseline for comparison
    std::fs::write("tests/baseline_rate.txt", format!("{:.1}", rate)).unwrap();
}
```

#### Step 3: Ground Truth Collection

Manual review by Rust expert to classify each test case:
- **StrictlyPure**: No mutations, referentially transparent
- **LocallyPure**: Local mutations only, functionally pure
- **ReadOnly**: Reads external state, doesn't modify
- **Impure**: External mutations or I/O

Document reasoning in `ground_truth.json`:
```json
{
  "test_cases": [
    {
      "file": "local_pure/accumulator_pattern.rs",
      "function": "process_data",
      "expected": "LocallyPure",
      "reasoning": "Mutates only local Vec, no external side effects"
    }
  ]
}
```

#### Validation Targets

- **Baseline false negative rate**: Measure current (expected 20-30%)
- **Target false negative rate**: <5%
- **False positive rate**: 0% (never mark external mutation as local)
- **Corpus size**: Minimum 50 functions (10-15 per category)

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

#### Computational Complexity

- `ScopeTracker` uses `HashSet` for O(1) variable lookups
- Scope depth tracking: O(1) increment/decrement per scope entry/exit
- Mutation classification: O(1) per mutation site
- No impact on non-mutation analysis paths (lazy evaluation)

#### Performance Benchmarking (REQUIRED)

**Target**: <10% overhead compared to current purity detection

Create benchmark suite in `benches/purity_detection_bench.rs`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_current_purity_detection(c: &mut Criterion) {
    let code = load_test_corpus("benches/fixtures/large_function.rs");

    c.bench_function("current_purity_detection", |b| {
        b.iter(|| {
            let mut detector = PurityDetector::new();
            let ast = parse_code(black_box(&code));
            detector.is_pure_function(&ast)
        });
    });
}

fn benchmark_scope_aware_purity_detection(c: &mut Criterion) {
    let code = load_test_corpus("benches/fixtures/large_function.rs");

    c.bench_function("scope_aware_purity_detection", |b| {
        b.iter(|| {
            let mut detector = PurityDetector::new_with_scope_tracking();
            let ast = parse_code(black_box(&code));
            detector.is_pure_function(&ast)
        });
    });
}

criterion_group!(
    benches,
    benchmark_current_purity_detection,
    benchmark_scope_aware_purity_detection
);
criterion_main!(benches);
```

**Benchmark Scenarios**:
1. Small pure function (5 lines, no mutations)
2. Medium function with local mutations (20 lines, 5 mutations)
3. Large function with mixed mutations (50 lines, 15 mutations)
4. Deeply nested scopes (5 levels of nesting)
5. Many local variables (50+ locals)

**Acceptance Criteria**:
- Average overhead: <10%
- Worst-case overhead: <20%
- Memory overhead: <5% per function

**Optimization Strategies if Needed**:
- Use `SmallVec` for `local_mutations` (most functions have <8 mutations)
- Pool and reuse `ScopeTracker` instances
- Skip scope tracking for functions with no assignments

## Migration and Compatibility

### Breaking Changes

#### API Changes

1. **New `PurityLevel` enum** in `src/core/mod.rs`:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
   pub enum PurityLevel {
       StrictlyPure,
       LocallyPure,  // NEW
       ReadOnly,
       Impure,
   }
   ```

2. **`FunctionMetrics` field additions** (src/core/mod.rs):
   ```rust
   pub struct FunctionMetrics {
       // Existing fields...
       pub is_pure: Option<bool>,  // DEPRECATED but kept for compatibility
       pub purity_confidence: Option<f32>,

       // NEW fields
       pub purity_level: Option<PurityLevel>,  // Replaces is_pure
       pub purity_reasons: Option<Vec<String>>, // Human-readable explanation
   }
   ```

3. **`PurityDetector` internal changes** (src/analyzers/purity_detector.rs):
   ```rust
   pub struct PurityDetector {
       // Existing fields remain...

       // NEW fields
       scope: ScopeTracker,
       local_mutations: Vec<LocalMutation>,
       upvalue_mutations: Vec<UpvalueMutation>,
   }
   ```

4. **`unified_scorer.rs` function signature change**:
   ```rust
   // OLD
   fn calculate_purity_adjustment(func: &FunctionMetrics) -> f64 {
       if func.is_pure == Some(true) { ... }
   }

   // NEW (supports both during transition)
   fn calculate_purity_adjustment(func: &FunctionMetrics) -> f64 {
       // Try new field first, fall back to old
       if let Some(level) = func.purity_level {
           match level { ... }
       } else if func.is_pure == Some(true) {
           // Legacy path
           0.7
       } else {
           1.0
       }
   }
   ```

### Migration Strategy

#### Phase 1: Additive Changes (Non-Breaking)

**Goal**: Add new functionality without breaking existing code.

1. **Add `PurityLevel` enum** to `src/core/mod.rs`
2. **Add `purity_level: Option<PurityLevel>` field** to `FunctionMetrics`
3. **Keep `is_pure: Option<bool>`** field - mark as deprecated in docs
4. **Extend `PurityDetector`** with scope tracking
5. **Update `calculate_purity_adjustment()`** to check both fields (new first, then old)

**Result**: Existing code continues to work. New code can use `purity_level`.

#### Phase 2: Gradual Adoption

**Goal**: Update analyzers to populate new field.

1. **Update `RustAnalyzer`** to set both `is_pure` and `purity_level`:
   ```rust
   let analysis = detector.is_pure_function(func);
   FunctionMetrics {
       is_pure: Some(analysis.is_pure),  // Legacy
       purity_level: Some(analysis.purity_level),  // New
       purity_confidence: Some(analysis.confidence),
       // ...
   }
   ```

2. **Update all match statements** in codebase to handle `LocallyPure`
3. **Add tests** for new purity levels
4. **Validate** on test corpus

**Result**: Both fields populated. Consumers can migrate at their own pace.

#### Phase 3: Deprecation (Future Breaking Release)

**Goal**: Remove `is_pure` field entirely.

1. **Remove `is_pure` field** from `FunctionMetrics`
2. **Make `purity_level` non-optional**: `pub purity_level: PurityLevel`
3. **Update serialization** to migrate old data
4. **Bump major version** (breaking change)

**Timeline**: Not before 2-3 minor releases after Phase 2.

### Backward Compatibility Guarantees

#### Serialization Compatibility

**Reading old data**:
```rust
// Handle deserialization of old format
impl<'de> Deserialize<'de> for FunctionMetrics {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Custom deserialization logic
        let mut metrics = /* ... */;

        // Migrate old is_pure to purity_level if needed
        if metrics.purity_level.is_none() && metrics.is_pure.is_some() {
            metrics.purity_level = Some(if metrics.is_pure.unwrap() {
                PurityLevel::StrictlyPure  // Conservative
            } else {
                PurityLevel::Impure
            });
        }

        Ok(metrics)
    }
}
```

#### API Stability

- **`is_pure` field**: Continues to work during Phases 1-2
- **Scoring logic**: Fallback ensures old data scores correctly
- **JSON output**: Both fields present for compatibility
- **Classification refinement**: Only "impure" functions get reclassified as "locally pure"
  - Pure functions remain pure
  - Impure functions may upgrade to locally pure (better score)
  - **No regressions**: Scores only improve, never worsen

### Migration Checklist

- [ ] Add `PurityLevel` enum to `src/core/mod.rs`
- [ ] Add `purity_level` field to `FunctionMetrics` (optional)
- [ ] Implement `ScopeTracker` in new module
- [ ] Update `PurityDetector` with scope tracking
- [ ] Update `calculate_purity_adjustment()` with fallback logic
- [ ] Find all `match func.is_pure` statements and update
- [ ] Add deserialization migration logic
- [ ] Run validation corpus to verify no regressions
- [ ] Update documentation with deprecation notice
- [ ] Plan Phase 3 timeline for future release

## Implementation Order

**Recommended sequence to minimize risk and enable incremental progress.**

### Stage 0: Preparation (2-3 hours)

**Goal**: Establish baseline and infrastructure

1. **Create validation corpus structure**:
   ```bash
   mkdir -p tests/purity_validation/{local_pure,external_impure,strictly_pure,read_only}
   ```

2. **Collect test cases** from real Rust projects:
   - debtmap's own codebase
   - Popular crates (itertools, rayon, serde)
   - Target: 50+ functions

3. **Manual classification** by Rust expert → `ground_truth.json`

4. **Write baseline measurement test** (`tests/baseline_measurement.rs`)

5. **Run baseline test** and record current false negative rate

**Deliverable**: Documented baseline (e.g., "Current: 23.5% false negative rate")

### Stage 1: Core Types (1-2 hours)

**Goal**: Add new types without breaking existing code

1. **Add `PurityLevel` enum** to `src/core/mod.rs`:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
   pub enum PurityLevel {
       StrictlyPure,
       LocallyPure,
       ReadOnly,
       Impure,
   }
   ```

2. **Add `purity_level` field** to `FunctionMetrics`:
   ```rust
   pub purity_level: Option<PurityLevel>,
   ```

3. **Update `FunctionMetrics::new()`** to initialize as `None`

4. **Run tests**: `cargo test` - all should pass

**Deliverable**: New types added, backward compatible

### Stage 2: Scope Tracker (2-3 hours)

**Goal**: Implement scope analysis in isolation

1. **Create `src/analyzers/scope_tracker.rs`**

2. **Implement `ScopeTracker` struct** with methods:
   - `new()`
   - `add_parameter()`
   - `add_local_var()`
   - `is_local()`, `is_self()`
   - `enter_scope()`, `exit_scope()`

3. **Add unit tests** for `ScopeTracker`:
   - Parameter tracking
   - Local variable tracking
   - Self detection (all variants)
   - Nested scopes

4. **Run tests**: `cargo test scope_tracker`

**Deliverable**: Fully tested `ScopeTracker` module

### Stage 3: PurityDetector Integration (3-4 hours)

**Goal**: Add scope tracking to purity detection

1. **Add fields to `PurityDetector`**:
   ```rust
   scope: ScopeTracker,
   local_mutations: Vec<LocalMutation>,
   upvalue_mutations: Vec<UpvalueMutation>,
   ```

2. **Update `PurityDetector::new()`** to initialize new fields

3. **Implement `determine_mutation_scope()` method**

4. **Update visitor methods**:
   - `visit_item_fn()` - initialize scope with parameters
   - `visit_local()` - track local declarations
   - `visit_expr_assign()` - classify mutation scope
   - `visit_expr_method_call()` - classify mutating methods

5. **Implement `determine_purity_level()` method**

6. **Update `PurityAnalysis` struct** to include `purity_level`

7. **Maintain backward compatibility**: Set both `is_pure` and `purity_level`

8. **Run existing tests**: `cargo test purity_detector` - all should pass

**Deliverable**: Enhanced `PurityDetector` with scope awareness

### Stage 4: Test Suite (2-3 hours)

**Goal**: Comprehensive test coverage for new classification

1. **Update existing test**: `tests/purity_detection_test.rs:174-193`
   - Currently has no assertions
   - Add assertion for `LocallyPure`

2. **Add new unit tests** (lines 566-647 in spec):
   - `test_local_mutation_is_locally_pure()`
   - `test_external_mutation_is_impure()`
   - `test_builder_pattern_is_locally_pure()`
   - `test_iterator_collect_is_locally_pure()`
   - `test_static_mutation_is_impure()`

3. **Run validation corpus**:
   ```bash
   cargo test --test purity_validation
   ```

4. **Measure new false negative rate** - should be <5%

**Deliverable**: Test suite passing, validation targets met

### Stage 5: Scoring Integration (1-2 hours)

**Goal**: Update scoring to use new purity levels

1. **Update `calculate_purity_adjustment()`** in `src/priority/unified_scorer.rs`:
   ```rust
   fn calculate_purity_adjustment(func: &FunctionMetrics) -> f64 {
       if let Some(level) = func.purity_level {
           match level {
               PurityLevel::StrictlyPure => { /* 0.7 or 0.8 */ }
               PurityLevel::LocallyPure => { /* 0.75 or 0.85 */ }
               PurityLevel::ReadOnly => 0.9,
               PurityLevel::Impure => 1.0,
           }
       } else if func.is_pure == Some(true) {
           0.7  // Fallback to old logic
       } else {
           1.0
       }
   }
   ```

2. **Add integration test** for scoring (lines 651-675 in spec)

3. **Run full test suite**: `cargo test`

**Deliverable**: Scoring updated, all tests passing

### Stage 6: Performance Validation (1-2 hours)

**Goal**: Verify performance targets met

1. **Create `benches/purity_detection_bench.rs`** with criterion benchmarks

2. **Run benchmarks**:
   ```bash
   cargo bench --bench purity_detection_bench
   ```

3. **Verify**: Average overhead <10%, worst-case <20%

4. **If overhead too high**: Apply optimization strategies from spec

**Deliverable**: Performance validated

### Stage 7: Documentation (1 hour)

**Goal**: Update all documentation

1. **Add doc comments** to `ScopeTracker`

2. **Update `docs/purity-analysis.md`** with new levels (lines 708-738)

3. **Add deprecation notice** to `is_pure` field

4. **Update CHANGELOG.md**

**Deliverable**: Documentation complete

### Stage 8: Final Validation (1 hour)

**Goal**: Ensure everything works end-to-end

1. **Run full test suite**: `cargo test --all-features`

2. **Run clippy**: `cargo clippy --all-targets --all-features`

3. **Run fmt**: `cargo fmt --all -- --check`

4. **Build docs**: `cargo doc --no-deps`

5. **Compare validation metrics**:
   - Baseline: X% false negatives
   - Post-implementation: Y% false negatives (should be <5%)
   - False positives: 0%

6. **Update spec status** to `implemented`

**Deliverable**: Production-ready implementation

---

## Estimated Total Effort

- **Stage 0 (Preparation)**: 2-3 hours
- **Stage 1 (Core Types)**: 1-2 hours
- **Stage 2 (Scope Tracker)**: 2-3 hours
- **Stage 3 (Integration)**: 3-4 hours
- **Stage 4 (Tests)**: 2-3 hours
- **Stage 5 (Scoring)**: 1-2 hours
- **Stage 6 (Performance)**: 1-2 hours
- **Stage 7 (Documentation)**: 1 hour
- **Stage 8 (Validation)**: 1 hour

**Total: 14-22 hours** (approximately 2-3 working days)

---

## Risk Mitigation

### High Risk Items

1. **Baseline measurement shows <20% false negatives**: Re-evaluate necessity
   - Mitigation: Still implement, but adjust target and impact claims

2. **Performance overhead >20%**: Optimization needed
   - Mitigation: Apply optimization strategies from spec
   - Fallback: Make scope tracking opt-in via feature flag

3. **False positives detected**: Critical bug
   - Mitigation: Be conservative - when in doubt, mark as External

### Medium Risk Items

4. **Deserialization migration breaks**: Backward compatibility issue
   - Mitigation: Test with real production data before release

5. **Scoring changes cause regressions**: Unexpected behavior
   - Mitigation: A/B test on sample projects, verify scores only improve
