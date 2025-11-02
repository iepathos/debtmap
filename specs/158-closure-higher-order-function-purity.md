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

### Implementation Approach

```rust
// src/analyzers/closure_analyzer.rs

#[derive(Debug, Clone)]
pub struct ClosureAnalyzer {
    /// Purity of analyzed closures
    closure_purity: HashMap<Span, ClosurePurity>,

    /// Captured variables
    captures: Vec<Capture>,
}

#[derive(Debug, Clone)]
pub struct ClosurePurity {
    pub level: PurityLevel,
    pub confidence: f64,
    pub captures: Vec<Capture>,
    pub capture_mode: CaptureMode,
}

#[derive(Debug, Clone)]
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

impl<'ast> Visit<'ast> for PurityDetector {
    fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
        // Create nested analyzer for closure body
        let mut closure_analyzer = PurityDetector::new();

        // Analyze closure body
        closure_analyzer.visit_expr(&closure.body);

        // Determine captures
        let captures = self.analyze_captures(closure);

        // Determine capture mode
        let capture_mode = if closure.capture.is_some() {
            CaptureMode::ByValue
        } else {
            // Analyze actual captures
            if captures.iter().any(|c| matches!(c.mode, CaptureMode::ByMutRef)) {
                CaptureMode::ByMutRef
            } else if captures.iter().any(|c| c.is_mutated) {
                CaptureMode::ByMutRef
            } else {
                CaptureMode::ByRef
            }
        };

        // Determine closure purity
        let closure_purity = ClosurePurity {
            level: self.determine_closure_purity_level(
                &closure_analyzer,
                &captures,
                capture_mode
            ),
            confidence: closure_analyzer.calculate_confidence(),
            captures: captures.clone(),
            capture_mode,
        };

        // Store for use by parent function
        self.closure_analyzer.closure_purity.insert(
            closure.span(),
            closure_purity
        );

        // If closure is impure, parent function is impure
        if closure_analyzer.modifies_external_state ||
           !closure_analyzer.side_effects.is_empty() {
            self.side_effects.push(SideEffect::ImpureClosure);
        }
    }

    fn determine_closure_purity_level(
        &self,
        analyzer: &PurityDetector,
        captures: &[Capture],
        mode: CaptureMode,
    ) -> PurityLevel {
        // Has external side effects?
        if analyzer.modifies_external_state || !analyzer.side_effects.is_empty() {
            return PurityLevel::Impure;
        }

        // Mutates captured variables?
        let mutates_captures = captures.iter().any(|c| c.is_mutated);

        match (mode, mutates_captures) {
            // FnMut with local captures = locally pure
            (CaptureMode::ByMutRef, true) if captures.iter()
                .all(|c| c.scope == MutationScope::Local) => {
                PurityLevel::LocallyPure
            }

            // Mutates external captures = impure
            (CaptureMode::ByMutRef, true) => PurityLevel::Impure,

            // No mutations = strictly pure
            _ => PurityLevel::StrictlyPure,
        }
    }

    fn analyze_captures(&self, closure: &syn::ExprClosure) -> Vec<Capture> {
        // Walk closure body and identify free variables
        let mut capture_analyzer = CaptureAnalyzer::new(&self.scope);
        capture_analyzer.visit_expr(&closure.body);
        capture_analyzer.captures
    }
}

// Iterator method handling
impl<'ast> Visit<'ast> for PurityDetector {
    fn visit_expr_method_call(&mut self, method: &'ast syn::ExprMethodCall) {
        let method_name = method.method.to_string();

        // Check if this is an iterator method
        if matches!(method_name.as_str(), "map" | "filter" | "fold" | "for_each") {
            // Check closure argument
            for arg in &method.args {
                if let Expr::Closure(closure) = arg {
                    // Closure purity already analyzed in visit_expr_closure
                    // Check stored purity
                    if let Some(closure_purity) = self.closure_analyzer
                        .closure_purity.get(&closure.span()) {

                        if closure_purity.level == PurityLevel::Impure {
                            self.side_effects.push(SideEffect::ImpureClosure);
                        }
                    }
                }
            }
        }

        visit::visit_expr_method_call(self, method);
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
```

## Documentation

Add to `docs/purity-analysis.md`:

```markdown
## Closures and Iterators

Debtmap analyzes closures used in iterator chains:

- **Pure closures**: `.map(|x| x * 2)` - maintains function purity
- **Impure closures**: `.for_each(|x| println!("{}", x))` - function becomes impure
- **FnMut with local state**: Captures that mutate local variables = locally pure

```rust
// Pure: 0.70x multiplier
fn calculate(nums: &[i32]) -> Vec<i32> {
    nums.iter().map(|x| x * 2).collect()
}

// Locally Pure: 0.75x multiplier
fn sum(nums: &[i32]) -> i32 {
    let mut total = 0;
    nums.iter().for_each(|x| total += x);
    total
}
```
```

## Migration

- Add `closure_purity` field to `PurityDetector`
- Backward compatible: closures previously marked as unknown function calls
- Scores improve for iterator-heavy code
