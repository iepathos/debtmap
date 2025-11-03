---
number: 157c
title: Integrate Scope Tracking into PurityDetector
category: foundation
priority: critical
status: draft
dependencies: [157a, 157b]
created: 2025-11-03
parent_spec: 157
---

# Specification 157c: Integrate Scope Tracking into PurityDetector

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: 157a (PurityLevel enum), 157b (ScopeTracker module)
**Parent Spec**: 157 - Local vs External Mutation Distinction

## Context

This is **Stage 3** of implementing local vs external mutation distinction (Spec 157). This stage integrates the `ScopeTracker` module into `PurityDetector` to classify mutations and determine purity levels.

## Objective

Enhance `PurityDetector` to use scope-aware mutation tracking and populate the `purity_level` field in `PurityAnalysis`.

## Requirements

### Functional Requirements

1. **Add Fields to PurityDetector**:
   ```rust
   pub struct PurityDetector {
       // Existing fields...
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
   ```

2. **Add Supporting Types**:
   ```rust
   #[derive(Debug, Clone)]
   pub struct LocalMutation {
       pub target: String,
   }

   #[derive(Debug, Clone)]
   pub struct UpvalueMutation {
       pub captured_var: String,
   }

   #[derive(Debug, Clone, PartialEq, Eq)]
   pub enum MutationScope {
       Local,    // Mutation of local variable or owned parameter
       Upvalue,  // Mutation of closure-captured variable
       External, // Mutation of external state (fields, statics, etc.)
   }
   ```

3. **Update PurityAnalysis**:
   ```rust
   pub struct PurityAnalysis {
       pub is_pure: bool,  // Keep for compatibility
       pub purity_level: PurityLevel,  // NEW
       pub reasons: Vec<ImpurityReason>,
       pub confidence: f32,
   }
   ```

4. **Implement Mutation Classification**:
   - `determine_mutation_scope(&self, expr: &Expr) -> MutationScope`
   - `determine_field_mutation_scope(&self, field: &ExprField) -> MutationScope`
   - `determine_purity_level(&self) -> PurityLevel`

5. **Update Visitor Methods**:
   - Initialize scope with parameters in `is_pure_function()`
   - Track local variables in `visit_local()`
   - Classify mutations in `visit_expr_assign()` and `visit_expr_method_call()`

6. **Update RustAnalyzer**:
   - Populate `purity_level` field when creating `FunctionMetrics`

### Non-Functional Requirements

- **Backward Compatible**: Keep existing `is_pure` field populated
- **Conservative Classification**: When in doubt, mark as External
- **Performance**: <10% overhead compared to current analysis
- **Confidence Scoring**: Adjust confidence based on mutation complexity

## Acceptance Criteria

- [x] PurityDetector enhanced with scope tracking fields
- [x] Mutation classification methods implemented
- [x] PurityAnalysis includes purity_level field
- [x] Both is_pure and purity_level fields populated
- [x] Tests added for:
  - Local mutation classified as LocallyPure
  - External mutation classified as Impure
  - Builder pattern (mut self) classified as LocallyPure
  - &mut self methods classified as Impure
- [x] Existing purity tests still pass
- [x] RustAnalyzer populates purity_level
- [x] `cargo build` succeeds
- [x] `cargo test` passes
- [x] `cargo clippy` passes
- [x] `cargo fmt` applied

## Implementation Details

### Mutation Scope Classification

```rust
impl PurityDetector {
    fn determine_mutation_scope(&self, expr: &Expr) -> MutationScope {
        match expr {
            // Simple identifier: x = value
            Expr::Path(path) => {
                let ident = path
                    .path
                    .get_ident()
                    .map(|i| i.to_string())
                    .unwrap_or_default();

                if self.scope.is_local(&ident) {
                    MutationScope::Local
                } else {
                    // Conservative: assume external
                    MutationScope::External
                }
            }

            // Field access: obj.field = value
            Expr::Field(field) => self.determine_field_mutation_scope(field),

            // Index: arr[i] = value
            Expr::Index(index) => {
                if let Expr::Path(path) = &*index.expr {
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

    fn determine_field_mutation_scope(&self, field: &ExprField) -> MutationScope {
        match &*field.base {
            // self.field = value
            Expr::Path(path) if self.scope.is_self(
                &path.path.get_ident()
                    .map(|i| i.to_string())
                    .unwrap_or_default()
            ) => {
                // Check self kind
                if let Some(self_kind) = self.scope.get_self_kind() {
                    match self_kind {
                        SelfKind::MutRef => MutationScope::External, // &mut self
                        SelfKind::Owned | SelfKind::MutOwned => {
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
                let ident = path
                    .path
                    .get_ident()
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

    fn determine_purity_level(&self) -> PurityLevel {
        // Has external side effects (I/O, external mutations)?
        if self.modifies_external_state || self.has_io_operations || self.has_unsafe_blocks {
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

    fn calculate_confidence_score(&self) -> f32 {
        let mut confidence = 1.0;

        // Reduce confidence if we only access external state
        if self.accesses_external_state && !self.modifies_external_state {
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

        // If no impurities detected, high confidence
        if !self.has_side_effects
            && !self.has_io_operations
            && !self.has_unsafe_blocks
            && !self.modifies_external_state
        {
            confidence = 0.95;
        }

        confidence.clamp(0.5, 1.0)
    }
}
```

### Updated is_pure_function Method

```rust
pub fn is_pure_function(&mut self, item_fn: &ItemFn) -> PurityAnalysis {
    // Reset state
    self.has_side_effects = false;
    self.has_mutable_params = false;
    self.has_io_operations = false;
    self.has_unsafe_blocks = false;
    self.accesses_external_state = false;
    self.modifies_external_state = false;
    self.scope = ScopeTracker::new();
    self.local_mutations.clear();
    self.upvalue_mutations.clear();

    // Initialize scope with parameters
    for arg in &item_fn.sig.inputs {
        self.scope.add_parameter(arg);

        // Check function signature for mutable parameters
        if let syn::FnArg::Typed(pat_type) = arg {
            if self.type_has_mutable_reference(&pat_type.ty) {
                self.has_mutable_params = true;
            }
            if self.has_mutable_reference(&pat_type.pat) {
                self.has_mutable_params = true;
            }
        }
    }

    // Visit the function body
    self.visit_block(&item_fn.block);

    let purity_level = self.determine_purity_level();

    PurityAnalysis {
        is_pure: !self.has_side_effects
            && !self.has_mutable_params
            && !self.has_io_operations
            && !self.has_unsafe_blocks
            && !self.modifies_external_state,
        purity_level,
        reasons: self.collect_impurity_reasons(),
        confidence: self.calculate_confidence_score(),
    }
}
```

## Testing Strategy

### New Unit Tests

```rust
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
    assert!(analysis.confidence > 0.85);
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
fn test_strictly_pure_function() {
    let code = r#"
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_read_only_function() {
    let code = r#"
        const MAX: i32 = 100;

        fn is_valid(x: i32) -> bool {
            x < MAX  // Reads external constant
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    assert_eq!(analysis.purity_level, PurityLevel::ReadOnly);
}
```

## RustAnalyzer Integration

Update `src/analyzers/rust.rs` to populate purity_level:

```rust
// In analyze_function method
let analysis = purity_detector.is_pure_function(func);

FunctionMetrics {
    // ... existing fields ...
    is_pure: Some(analysis.is_pure),  // Keep for compatibility
    purity_level: Some(analysis.purity_level),  // NEW
    purity_confidence: Some(analysis.confidence),
    // ... rest of fields ...
}
```

## Estimated Effort

**Time**: 2-3 hours
**Complexity**: Medium
**Risk**: Medium (core logic changes, but backward compatible)

## Next Steps

After this spec is implemented:
- **Spec 157d**: Update scoring to use LocallyPure levels
