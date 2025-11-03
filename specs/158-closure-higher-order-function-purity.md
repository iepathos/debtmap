---
number: 158
title: Closure and Higher-Order Function Purity Analysis
category: foundation
priority: high
status: draft
dependencies: [156, 157]
created: 2025-11-01
---

# Specification 158: Closure and Higher-Order Function Purity Analysis

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Specs 156 (Inter-Procedural), 157 (Local vs External Mutation)

## Context

**Critical Gap**: Closures are completely ignored in current purity analysis. The `visit_expr` method in `purity_detector.rs:204-280` has no handler for `Expr::Closure` or `Expr::Async`.

**Evidence from Tests** (`tests/purity_detection_test.rs:196-223`):
```rust
#[test]
fn test_higher_order_functions() {
    let code = r#"
        fn process_items<F>(items: &[i32], f: F) -> Vec<i32>
        where F: Fn(i32) -> i32
        {
            items.iter().map(|x| f(*x)).collect()
        }
    "#;
    // NO ASSERTIONS - closures not analyzed
}
```

**Impact on Functional Rust**:
Iterator chains (`.map()`, `.filter()`, `.fold()`) are ubiquitous in idiomatic Rust. Without closure analysis:
- Functions using iterators incorrectly marked impure
- Higher-order functions can't be analyzed
- Functional programming patterns penalized

**Real-World Example**:
```rust
// Should be pure, but currently marked impure
fn calculate_discounts(prices: &[f64]) -> Vec<f64> {
    prices.iter()
        .map(|&p| p * 0.9)  // Closure not analyzed
        .filter(|&p| p > 10.0)
        .collect()
}
```

## Objective

Implement **closure capture and purity analysis** for closures and higher-order functions, enabling accurate purity classification for functional programming patterns.

## Requirements

### Functional Requirements

1. **Closure Detection and Analysis**
   - Detect closure expressions (`|x| x + 1`)
   - Analyze closure body for purity
   - Track closure captures (by-value vs by-reference)
   - Distinguish `Fn`, `FnMut`, and `FnOnce` traits

2. **Capture Analysis**
   - Identify which variables are captured
   - Determine capture mode (move, ref, mut ref)
   - Classify captured mutations as local vs external
   - Track closure side effects separately

3. **Higher-Order Function Analysis**
   - Analyze functions accepting closures as parameters
   - Propagate closure purity to calling function
   - Handle iterator methods (map, filter, fold, etc.)
   - Support nested closures

4. **Purity Propagation Rules**
   - Pure closure + pure context = pure function
   - Impure closure = impure function
   - FnMut closures with local captures = locally pure

### Non-Functional Requirements

- **Performance**: <15% overhead for closure analysis
- **Accuracy**: Handle 95% of common iterator patterns correctly
- **Coverage**: Support all std::iter methods

## Acceptance Criteria

- [ ] Closure expressions detected and analyzed
- [ ] Closure captures classified (value, ref, mut ref)
- [ ] Pure closures in `.map()` don't affect function purity
- [ ] Impure closures (I/O, mutations) correctly propagate impurity
- [ ] `FnMut` closures with local state classified as locally pure
- [ ] Nested closures analyzed recursively
- [ ] Iterator chain purity correctly determined
- [ ] Higher-order functions receive correct purity based on closure args

## Technical Details

### Architecture: Separate Closure Analyzer Module

To maintain separation of concerns, closure analysis is extracted into its own module that can be used by `PurityDetector`.

```rust
// src/analyzers/closure_analyzer.rs (NEW FILE)

use syn::{visit::Visit, Expr, ExprClosure, Ident};
use crate::analyzers::purity_detector::{PurityDetector, MutationScope};
use crate::core::PurityLevel;
use std::collections::HashSet;

/// Dedicated analyzer for closure expressions
#[derive(Debug, Clone)]
pub struct ClosureAnalyzer<'a> {
    /// Parent scope for determining captures
    parent_scope: &'a ScopeTracker,

    /// Detected captures
    captures: Vec<Capture>,

    /// Confidence reduction factors
    confidence_penalties: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct ClosurePurity {
    pub level: PurityLevel,
    pub confidence: f32,
    pub captures: Vec<Capture>,
    pub has_nested_closures: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureMode {
    ByValue,      // move closure
    ByRef,        // immutable capture
    ByMutRef,     // mutable capture
}

#[derive(Debug, Clone)]
pub struct Capture {
    pub var_name: String,
    pub mode: CaptureMode,
    pub is_mutated: bool,
    pub scope: MutationScope,  // From spec 157
}

impl<'a> ClosureAnalyzer<'a> {
    pub fn new(parent_scope: &'a ScopeTracker) -> Self {
        Self {
            parent_scope,
            captures: Vec::new(),
            confidence_penalties: Vec::new(),
        }
    }

    /// Main entry point: Analyze a closure expression
    pub fn analyze_closure(&mut self, closure: &ExprClosure) -> ClosurePurity {
        // Step 1: Create isolated detector for closure body
        let mut body_detector = PurityDetector::new();

        // Step 2: Register closure parameters in body scope
        for input in &closure.inputs {
            if let syn::Pat::Ident(pat_ident) = input {
                body_detector.scope.add_local_var(pat_ident.ident.to_string());
            }
        }

        // Step 3: Analyze closure body
        body_detector.visit_expr(&closure.body);

        // Step 4: Detect captures (free variables)
        self.captures = self.find_captures(closure, &body_detector);

        // Step 5: Infer capture modes from usage
        self.infer_capture_modes(closure, &body_detector);

        // Step 6: Check for nested closures
        let has_nested_closures = self.contains_nested_closures(&closure.body);
        if has_nested_closures {
            self.confidence_penalties.push("nested_closures");
        }

        // Step 7: Determine purity level
        let level = self.determine_purity_level(&body_detector);

        // Step 8: Calculate confidence
        let confidence = self.calculate_confidence(&body_detector);

        ClosurePurity {
            level,
            confidence,
            captures: self.captures.clone(),
            has_nested_closures,
        }
    }

    /// Detect captured variables (free variables in closure body)
    fn find_captures(
        &self,
        closure: &ExprClosure,
        body_detector: &PurityDetector,
    ) -> Vec<Capture> {
        // Collect parameter names
        let mut params: HashSet<String> = HashSet::new();
        for input in &closure.inputs {
            if let syn::Pat::Ident(pat_ident) = input {
                params.insert(pat_ident.ident.to_string());
            }
        }

        // Walk body and find variable references
        let mut visitor = CaptureDetector {
            params: &params,
            parent_scope: self.parent_scope,
            captures: Vec::new(),
        };
        visitor.visit_expr(&closure.body);

        visitor.captures
    }

    /// Infer capture modes based on usage patterns
    fn infer_capture_modes(
        &mut self,
        closure: &ExprClosure,
        body_detector: &PurityDetector,
    ) {
        // Check for 'move' keyword
        let has_move = matches!(closure.capture, Some(_));

        for capture in &mut self.captures {
            // 'move' forces by-value capture
            if has_move {
                capture.mode = CaptureMode::ByValue;
                continue;
            }

            // Check if captured variable is mutated in closure body
            let is_mutated = body_detector.local_mutations.iter()
                .any(|m| m.target == capture.var_name);

            capture.is_mutated = is_mutated;

            // Infer mode: mutated → mut ref, otherwise immut ref
            capture.mode = if is_mutated {
                CaptureMode::ByMutRef
            } else {
                CaptureMode::ByRef
            };

            // Determine scope (local to function vs external)
            capture.scope = if self.parent_scope.is_local(&capture.var_name) {
                MutationScope::Local
            } else {
                MutationScope::External
            };
        }
    }

    /// Check if closure body contains nested closures
    fn contains_nested_closures(&self, expr: &Expr) -> bool {
        let mut visitor = ClosureDetector { found: false };
        visitor.visit_expr(expr);
        visitor.found
    }

    /// Determine purity level based on closure behavior
    fn determine_purity_level(&self, body_detector: &PurityDetector) -> PurityLevel {
        // Has I/O or unsafe operations?
        if body_detector.has_io_operations || body_detector.has_unsafe_blocks {
            return PurityLevel::Impure;
        }

        // Modifies external state?
        if body_detector.modifies_external_state {
            return PurityLevel::Impure;
        }

        // Check captured variable mutations
        let mutates_external = self.captures.iter()
            .any(|c| c.is_mutated && c.scope == MutationScope::External);

        if mutates_external {
            return PurityLevel::Impure;
        }

        // Mutates local captures only?
        let mutates_local = self.captures.iter()
            .any(|c| c.is_mutated && c.scope == MutationScope::Local);

        if mutates_local || !body_detector.local_mutations.is_empty() {
            return PurityLevel::LocallyPure;
        }

        // Accesses external state (reads)?
        if body_detector.accesses_external_state {
            return PurityLevel::ReadOnly;
        }

        // No side effects detected
        PurityLevel::StrictlyPure
    }

    /// Calculate confidence score with penalty factors
    fn calculate_confidence(&self, body_detector: &PurityDetector) -> f32 {
        let mut confidence = 1.0;

        // Reduce confidence for nested closures
        if self.confidence_penalties.contains(&"nested_closures") {
            confidence *= 0.85;
        }

        // Reduce confidence for external state access
        if body_detector.accesses_external_state {
            confidence *= 0.80;
        }

        // Reduce confidence for multiple captures
        if self.captures.len() > 3 {
            confidence *= 0.90;
        }

        // Reduce confidence if capture modes were inferred
        if self.captures.iter().any(|c| c.mode != CaptureMode::ByValue) {
            confidence *= 0.95;
        }

        confidence.clamp(0.5, 1.0)
    }
}

/// Helper visitor to detect captured variables
struct CaptureDetector<'a> {
    params: &'a HashSet<String>,
    parent_scope: &'a ScopeTracker,
    captures: Vec<Capture>,
}

impl<'ast, 'a> Visit<'ast> for CaptureDetector<'a> {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        if let Expr::Path(path) = expr {
            if let Some(ident) = path.path.get_ident() {
                let name = ident.to_string();

                // Not a parameter and not a standard construct?
                if !self.params.contains(&name)
                    && name != "self"
                    && name != "Self" {

                    // Check if it's in parent scope (captured)
                    if self.parent_scope.is_local(&name)
                        || self.parent_scope.is_self(&name) {

                        // Add if not already captured
                        if !self.captures.iter().any(|c| c.var_name == name) {
                            self.captures.push(Capture {
                                var_name: name,
                                mode: CaptureMode::ByRef,  // Default, refined later
                                is_mutated: false,
                                scope: MutationScope::Local,
                            });
                        }
                    }
                }
            }
        }

        syn::visit::visit_expr(self, expr);
    }
}

/// Helper visitor to detect nested closures
struct ClosureDetector {
    found: bool,
}

impl<'ast> Visit<'ast> for ClosureDetector {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        if matches!(expr, Expr::Closure(_)) {
            self.found = true;
            return;
        }
        syn::visit::visit_expr(self, expr);
    }
}
```

### Integration with PurityDetector

```rust
// src/analyzers/purity_detector.rs (MODIFICATIONS)

use crate::analyzers::closure_analyzer::{ClosureAnalyzer, ClosurePurity};
use std::collections::HashMap;

pub struct PurityDetector {
    // ... existing fields ...

    /// Cache of analyzed closures (by span)
    closure_results: HashMap<proc_macro2::Span, ClosurePurity>,
}

impl<'ast> Visit<'ast> for PurityDetector {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            // NEW: Handle closure expressions
            Expr::Closure(closure) => {
                self.visit_expr_closure(closure);
                return;  // Don't continue visiting (closure analyzer handles body)
            }

            // Existing handlers...
            Expr::Call(ExprCall { func, .. }) => { /* ... */ }
            Expr::MethodCall(method_call) => {
                self.visit_expr_method_call_with_closures(method_call);
                return;
            }
            // ... other cases ...
            _ => {}
        }

        // Continue visiting nested expressions
        syn::visit::visit_expr(self, expr);
    }

    fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
        // Use dedicated analyzer
        let mut analyzer = ClosureAnalyzer::new(&self.scope);
        let closure_purity = analyzer.analyze_closure(closure);

        // Cache result for later lookup
        self.closure_results.insert(closure.span(), closure_purity.clone());

        // Propagate impurity to parent function
        match closure_purity.level {
            PurityLevel::Impure => {
                self.modifies_external_state = true;
                self.has_side_effects = true;
            }
            PurityLevel::LocallyPure => {
                // Local mutations in closure count toward function's local mutations
                self.local_mutations.extend(
                    closure_purity.captures.iter()
                        .filter(|c| c.is_mutated && c.scope == MutationScope::Local)
                        .map(|c| LocalMutation { target: c.var_name.clone() })
                );
            }
            _ => {}
        }
    }

    fn visit_expr_method_call_with_closures(&mut self, method: &'ast syn::ExprMethodCall) {
        let method_name = method.method.to_string();

        // Comprehensive iterator method list
        const ITERATOR_METHODS: &[&str] = &[
            // Consuming methods
            "map", "filter", "filter_map", "flat_map", "flatten",
            "fold", "reduce", "for_each", "try_fold", "try_for_each",
            "scan", "partition", "find", "find_map", "position",
            "any", "all", "collect", "inspect",

            // Result/Option adapters
            "and_then", "or_else", "map_or", "map_or_else",
        ];

        if ITERATOR_METHODS.contains(&method_name.as_str()) {
            // Analyze closure arguments inline
            for arg in &method.args {
                if let Expr::Closure(closure) = arg {
                    // Analyze closure immediately if not already cached
                    if !self.closure_results.contains_key(&closure.span()) {
                        self.visit_expr_closure(closure);
                    }

                    // Check cached purity
                    if let Some(purity) = self.closure_results.get(&closure.span()) {
                        if purity.level == PurityLevel::Impure {
                            self.has_side_effects = true;
                            self.modifies_external_state = true;
                        }
                    }
                }
            }
        }

        // Continue with normal method call analysis
        syn::visit::visit_expr_method_call(self, method);
    }
}
```

### Example Analysis

```rust
// Pure closure
fn example1() {
    let nums = vec![1, 2, 3];
    let doubled: Vec<_> = nums.iter()
        .map(|x| x * 2)  // Pure closure
        .collect();
}
// Result: StrictlyPure (0.70x multiplier)

// Impure closure (I/O)
fn example2() {
    let nums = vec![1, 2, 3];
    nums.iter().for_each(|x| {
        println!("{}", x);  // Impure closure
    });
}
// Result: Impure (1.0x multiplier)

// FnMut with local state (locally pure)
fn example3() {
    let nums = vec![1, 2, 3];
    let mut sum = 0;
    nums.iter().for_each(|x| {
        sum += x;  // Mutable capture, but local
    });
}
// Result: LocallyPure (0.75x multiplier)
```

## Testing Strategy

### Core Functionality Tests

```rust
#[test]
fn test_pure_closure_in_map() {
    let code = r#"
        fn double_values(nums: &[i32]) -> Vec<i32> {
            nums.iter().map(|x| x * 2).collect()
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    assert!(analysis.confidence > 0.90);
}

#[test]
fn test_impure_closure_propagates() {
    let code = r#"
        fn print_values(nums: &[i32]) {
            nums.iter().for_each(|x| println!("{}", x));
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    assert_eq!(analysis.purity_level, PurityLevel::Impure);
    assert!(analysis.reasons.contains(&ImpurityReason::IOOperations));
}

#[test]
fn test_fnmut_local_capture() {
    let code = r#"
        fn sum_values(nums: &[i32]) -> i32 {
            let mut sum = 0;
            nums.iter().for_each(|x| sum += x);
            sum
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    assert_eq!(analysis.purity_level, PurityLevel::LocallyPure);
}

#[test]
fn test_move_closure() {
    let code = r#"
        fn create_adder(x: i32) -> impl Fn(i32) -> i32 {
            move |y| x + y
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
}
```

### Edge Case Tests

```rust
#[test]
fn test_nested_closures() {
    let code = r#"
        fn nested_map(data: &[Vec<i32>]) -> Vec<Vec<i32>> {
            data.iter()
                .map(|inner| inner.iter().map(|x| x * 2).collect())
                .collect()
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    // Confidence should be lower due to nesting
    assert!(analysis.confidence < 0.90);
}

#[test]
fn test_closure_captures_mut_ref() {
    let code = r#"
        fn mutate_through_closure(data: &mut Vec<i32>) {
            data.iter().for_each(|x| data.push(*x));
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    assert_eq!(analysis.purity_level, PurityLevel::Impure);
    assert!(analysis.reasons.contains(&ImpurityReason::ModifiesExternalState));
}

#[test]
fn test_closure_conditional_io() {
    let code = r#"
        fn maybe_print(nums: &[i32], debug: bool) {
            nums.iter().for_each(|x| {
                if debug {
                    println!("{}", x);
                }
            });
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    // Even conditional I/O makes function impure
    assert_eq!(analysis.purity_level, PurityLevel::Impure);
}

#[test]
fn test_mixed_purity_chain() {
    let code = r#"
        fn process(nums: &[i32]) -> Vec<i32> {
            nums.iter()
                .map(|x| x * 2)
                .inspect(|x| println!("{}", x))
                .collect()
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    // Chain contains impure operation
    assert_eq!(analysis.purity_level, PurityLevel::Impure);
}

#[test]
fn test_closure_with_external_fn_call() {
    let code = r#"
        fn external_func(x: i32) -> i32 { x * 2 }

        fn use_closure(nums: &[i32]) -> Vec<i32> {
            nums.iter().map(|x| external_func(*x)).collect()
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    // Purity depends on external_func (requires inter-procedural analysis)
    // Without that, should be conservative
    assert!(matches!(
        analysis.purity_level,
        PurityLevel::StrictlyPure | PurityLevel::ReadOnly
    ));
}

#[test]
fn test_async_closure() {
    let code = r#"
        async fn process_async(nums: Vec<i32>) -> Vec<i32> {
            nums.into_iter()
                .map(|x| async move { x * 2 })
                .collect()
        }
    "#;

    // Should handle async closures gracefully
    let analysis = analyze_purity(code);
    assert!(analysis.is_ok());
}
```

### Iterator Method Coverage Tests

```rust
#[test]
fn test_filter_map_purity() {
    let code = r#"
        fn extract_evens(nums: &[i32]) -> Vec<i32> {
            nums.iter()
                .filter_map(|&x| if x % 2 == 0 { Some(x) } else { None })
                .collect()
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_flat_map_purity() {
    let code = r#"
        fn flatten_data(data: &[Vec<i32>]) -> Vec<i32> {
            data.iter().flat_map(|v| v.iter().copied()).collect()
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_scan_with_accumulator() {
    let code = r#"
        fn running_sum(nums: &[i32]) -> Vec<i32> {
            nums.iter()
                .scan(0, |acc, &x| {
                    *acc += x;
                    Some(*acc)
                })
                .collect()
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    // scan mutates accumulator but it's local to the closure
    assert_eq!(analysis.purity_level, PurityLevel::LocallyPure);
}

#[test]
fn test_try_fold_error_handling() {
    let code = r#"
        fn safe_sum(nums: &[i32]) -> Result<i32, String> {
            nums.iter().try_fold(0, |acc, &x| {
                if x < 0 {
                    Err("Negative number".to_string())
                } else {
                    Ok(acc + x)
                }
            })
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_partition_purity() {
    let code = r#"
        fn partition_evens(nums: &[i32]) -> (Vec<i32>, Vec<i32>) {
            nums.iter().partition(|&&x| x % 2 == 0)
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
}
```

### Integration Tests

```rust
#[test]
fn test_complex_iterator_chain() {
    let code = r#"
        fn complex_processing(data: &[i32]) -> Vec<String> {
            data.iter()
                .filter(|&&x| x > 0)
                .map(|&x| x * 2)
                .filter(|&x| x < 100)
                .map(|x| format!("Value: {}", x))
                .collect()
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_closure_in_option_combinator() {
    let code = r#"
        fn transform_option(opt: Option<i32>) -> Option<i32> {
            opt.and_then(|x| Some(x * 2))
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_closure_in_result_combinator() {
    let code = r#"
        fn transform_result(res: Result<i32, String>) -> Result<i32, String> {
            res.map(|x| x * 2)
        }
    "#;

    let analysis = analyze_purity(code).unwrap();
    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
}
```

### Performance Tests

```rust
#[test]
fn test_closure_analysis_performance() {
    let code = r#"
        fn heavy_processing(data: &[Vec<Vec<i32>>]) -> i32 {
            data.iter()
                .flat_map(|v1| v1.iter())
                .flat_map(|v2| v2.iter())
                .filter(|&&x| x > 0)
                .map(|&x| x * 2)
                .sum()
        }
    "#;

    let start = std::time::Instant::now();
    let analysis = analyze_purity(code).unwrap();
    let duration = start.elapsed();

    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    // Should complete within reasonable time (<15% overhead)
    assert!(duration < std::time::Duration::from_millis(100));
}
```

## Documentation

Add to `docs/purity-analysis.md`:

```markdown
## Closures and Iterators

Debtmap analyzes closures used in iterator chains and higher-order functions to accurately determine purity.

### Purity Classification

- **Pure closures**: `.map(|x| x * 2)` - Maintains function purity (0.70x multiplier)
- **Impure closures**: `.for_each(|x| println!("{}", x))` - Function becomes impure (1.0x multiplier)
- **Locally pure closures**: Captures that mutate local variables (0.75x multiplier)

### Examples

```rust
// StrictlyPure: 0.70x multiplier
fn calculate(nums: &[i32]) -> Vec<i32> {
    nums.iter().map(|x| x * 2).collect()
}

// LocallyPure: 0.75x multiplier
fn sum(nums: &[i32]) -> i32 {
    let mut total = 0;
    nums.iter().for_each(|x| total += x);
    total
}

// Impure: 1.0x multiplier
fn debug_print(nums: &[i32]) {
    nums.iter().for_each(|x| println!("{}", x));
}
```

### Capture Analysis

Debtmap detects and classifies closure captures:

- **By-value captures** (`move` closures): Variables moved into closure
- **By-reference captures**: Immutable borrows
- **By-mutable-reference captures**: Mutable borrows (requires `FnMut`)

```rust
// By-value capture (move)
fn create_adder(x: i32) -> impl Fn(i32) -> i32 {
    move |y| x + y  // x is moved into closure
}

// By-mutable-reference capture
fn accumulate(nums: &[i32]) -> i32 {
    let mut sum = 0;
    nums.iter().for_each(|x| sum += x);  // sum captured mutably
    sum
}
```

### Nested Closures

Debtmap recursively analyzes nested closures:

```rust
fn nested(data: &[Vec<i32>]) -> Vec<Vec<i32>> {
    data.iter()
        .map(|inner| inner.iter().map(|x| x * 2).collect())
        .collect()
}
// Result: StrictlyPure (both closures are pure)
// Note: Lower confidence due to nesting complexity
```

### Supported Iterator Methods

Debtmap recognizes closure arguments in:

- **Transformation**: `map`, `filter`, `filter_map`, `flat_map`, `flatten`
- **Accumulation**: `fold`, `reduce`, `scan`, `try_fold`, `try_for_each`
- **Iteration**: `for_each`, `inspect`, `partition`
- **Search**: `find`, `find_map`, `position`, `any`, `all`
- **Option/Result**: `and_then`, `or_else`, `map_or`, `map_or_else`

### Limitations

1. **No type inference**: Cannot distinguish `Fn` vs `FnMut` vs `FnOnce` without type information
   - Uses conservative heuristics based on usage patterns
   - May be less accurate for generic closures

2. **Macro-generated closures**: Limited analysis of closures created by macros
   - Works for simple cases like `vec!` with closures
   - Complex macro expansions may not be fully analyzed

3. **Async closures**: Basic support for `async move` closures
   - Treats async operations conservatively
   - Future improvements will add async-specific analysis

4. **External function calls**: Purity of external functions called within closures
   - Requires inter-procedural analysis (Spec 156)
   - Conservative assumption without call graph

### Performance

- **Overhead**: <15% additional analysis time for closure-heavy code
- **Caching**: Analyzed closures are cached to avoid redundant work
- **Parallelization**: Independent closures can be analyzed in parallel

### Confidence Scoring

Confidence is reduced for:
- Nested closures: 0.85x multiplier
- External state access: 0.80x multiplier
- Multiple captures (>3): 0.90x multiplier
- Inferred capture modes: 0.95x multiplier

Lower confidence indicates higher uncertainty in purity classification.
```

## Known Limitations and Future Work

### Current Limitations

1. **Type Inference Gap**: Cannot access Rust type information
   - Cannot definitively determine `Fn`/`FnMut`/`FnOnce` trait
   - Relies on usage pattern heuristics
   - Future: Integrate with `rustc` type query system

2. **Closure-Returning Functions**: Limited support for higher-order returns
   - Functions returning closures analyzed conservatively
   - `impl Fn` return types handled, but with lower confidence
   - Future: Track closure provenance through returns

3. **Generic Closure Parameters**: Conservative analysis of generic closures
   - Cannot determine generic type bounds without type system
   - Assumes worst-case for unknown generics
   - Future: Add generic constraint tracking

4. **Macro Expansion**: Closures in macro expansions
   - Limited to syntactically visible closures
   - Macro-generated closures may be missed
   - Future: Pre-expansion analysis with `proc_macro2`

5. **Async/Await**: Basic async closure support
   - Async blocks treated as closures
   - No future-specific purity analysis
   - Future: Async-aware purity model (Spec 162)

### Performance Characteristics

- **Baseline**: Analysis without closure support
- **With Closures**: <15% overhead (target)
- **Best Case**: Simple closures, no captures (2-5% overhead)
- **Worst Case**: Deeply nested, multiple captures (10-20% overhead)

**Optimization Strategies**:
- Early bailout for trivial closures
- Caching by span to avoid re-analysis
- Lazy evaluation of capture details

### Migration Path

**Phase 1**: Add `ClosureAnalyzer` module (this spec)
- Separate module for clarity
- Basic capture detection
- Integration with existing `PurityDetector`

**Phase 2**: Enhance with inter-procedural analysis (Spec 156)
- Closure arguments flow through call graph
- External function purity affects closure purity
- Better handling of closure-returning functions

**Phase 3**: Type system integration (future)
- Query `rustc` for trait information
- Definitive `Fn`/`FnMut`/`FnOnce` detection
- Improved generic handling

## Migration

### Backward Compatibility

- **Existing Code**: No breaking changes to `PurityDetector` public API
- **Default Behavior**: Closures previously ignored, now analyzed
- **Scores**: Functions with iterator chains will see improved purity scores
- **Confidence**: New confidence penalties for closure complexity

### Implementation Steps

1. Create `src/analyzers/closure_analyzer.rs` with new module
2. Add `closure_results` field to `PurityDetector`
3. Implement `visit_expr_closure` handler
4. Update `visit_expr_method_call` to handle closure arguments
5. Add comprehensive test suite
6. Update documentation
7. Run benchmarks to validate performance targets

### Rollout Strategy

- **Stage 1**: Merge closure analyzer (no behavior change)
- **Stage 2**: Enable for test suite validation
- **Stage 3**: Enable for production with monitoring
- **Stage 4**: Tune confidence thresholds based on real-world data

---

## Summary of Improvements (Revision 2)

This specification has been revised to address critical implementation issues and gaps identified during evaluation:

### Critical Fixes

1. **Visit Order Problem**: Changed from post-hoc lookup to inline analysis
   - Closures analyzed immediately when encountered in method calls
   - Removed chicken-and-egg dependency on visitor traversal order
   - Added caching mechanism for already-analyzed closures

2. **Capture Detection Algorithm**: Added concrete implementation
   - `CaptureDetector` visitor walks closure body to find free variables
   - Distinguishes parameters from captures using HashSet
   - Integrates with parent scope to classify local vs external

3. **Capture Mode Inference**: Improved from oversimplified to pattern-based
   - Checks for `move` keyword
   - Analyzes mutation patterns in closure body
   - Determines scope (local vs external) for each capture

4. **Nested Closure Support**: Added recursive analysis
   - `ClosureDetector` visitor identifies nested closures
   - Confidence penalties for complexity
   - Recursive purity propagation

### Enhancements

5. **Iterator Method Coverage**: Expanded from 4 to 22 methods
   - Comprehensive list of std::iter methods
   - Option/Result combinators
   - Try variants (try_fold, try_for_each)

6. **Architectural Separation**: Extracted into dedicated module
   - `ClosureAnalyzer` handles all closure-specific logic
   - Clear separation from `PurityDetector`
   - Easier to test and maintain

7. **Comprehensive Testing**: Added 20+ test cases
   - Edge cases (nested, conditional, captures)
   - Iterator method coverage
   - Integration tests
   - Performance benchmarks

8. **Documentation**: Added limitations and performance details
   - Known limitations clearly stated
   - Performance characteristics documented
   - Confidence scoring explained
   - Migration path defined

### Implementation Readiness

This revised specification is now **ready for implementation** with:
- ✅ Clear architectural design
- ✅ Concrete algorithms for all components
- ✅ Comprehensive test strategy
- ✅ Performance targets and optimization strategies
- ✅ Documentation of limitations
- ✅ Backward-compatible migration path

**Estimated Complexity**: High (3-5 days)
**Risk Level**: Medium (well-defined but complex)
**Dependencies**: Specs 156, 157 (can be implemented independently)
