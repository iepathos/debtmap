---
number: 249
title: Closure Capture Analysis for CFG Data Flow
category: foundation
priority: high
status: draft
dependencies: [248]
created: 2025-12-12
---

# Specification 249: Closure Capture Analysis for CFG Data Flow

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 248 (Enhanced Expression Variable Extraction)

## Context

### Current Problem

The CFG-based data flow analysis in `src/analysis/data_flow.rs` has a TODO for closure capture tracking:

```rust
// data_flow.rs:877-878
#[derive(Debug, Clone)]
pub struct EscapeAnalysis {
    /// Variables that escape through returns or method calls
    pub escaping_vars: HashSet<VarId>,
    /// Variables captured by closures (TODO: closure detection not yet implemented)
    pub captured_vars: HashSet<VarId>,  // <-- Always empty
    /// Variables that (directly or indirectly) contribute to the return value
    pub return_dependencies: HashSet<VarId>,
}
```

The `captured_vars` field exists but is never populated because `CfgBuilder` doesn't process closure expressions.

### Why This Matters

Rust code heavily uses closures, especially in iterator chains:

```rust
fn process_items(items: &[Item], threshold: f64) -> Vec<String> {
    items.iter()
        .filter(|item| item.value > threshold)  // Captures: threshold
        .map(|item| item.name.clone())          // No captures
        .collect()
}
```

Without closure capture analysis:
1. **Escape analysis is incomplete**: Can't determine if `threshold` escapes through the closure
2. **Taint analysis misses propagation**: Mutations to captured variables aren't tracked
3. **Purity detection inaccurate**: Closures that mutate captures appear pure

### Existing Infrastructure

A `ClosureAnalyzer` already exists in `src/analyzers/closure_analyzer.rs` that:
- Detects captured variables via `CaptureDetector` visitor
- Infers capture modes (ByValue, ByRef, ByMutRef)
- Determines if captures are mutated
- Classifies mutation scope (Local vs External)

However, this analyzer operates at the **purity detection level**, not the **CFG level**. We need to bridge these two systems.

## Objective

Integrate closure capture detection into CFG construction so that:
1. `captured_vars` in `EscapeAnalysis` is populated
2. Escape analysis considers captured variables
3. Taint analysis tracks mutations through closures
4. Iterator chains are properly analyzed for data flow

## Requirements

### Functional Requirements

1. **Closure Detection in CFG Builder**
   - Detect `Expr::Closure` during CFG construction
   - Extract closure body for analysis
   - Track closure parameters separately from outer scope

2. **Capture Variable Identification**
   - Identify free variables in closure body
   - Distinguish captures from closure parameters
   - Handle nested closures recursively

3. **Capture Mode Inference**
   - Detect `move` keyword for by-value captures
   - Infer `&` vs `&mut` captures from usage
   - Track if captured variable is mutated in closure body

4. **Integration with EscapeAnalysis**
   - Populate `captured_vars` field
   - Mark captured variables as escaping (lifetime extended)
   - Track captured variables in `return_dependencies` if closure is returned

5. **Integration with TaintAnalysis**
   - Propagate taint through captures
   - If captured variable is mutated, mark as tainted
   - If tainted variable is captured, propagate to closure body

### Non-Functional Requirements

- **Performance**: Add <2ms overhead per function with closures
- **Accuracy**: Detect >90% of captures in typical Rust code
- **Memory**: Minimal additional memory for capture tracking
- **Backward Compatibility**: Existing tests must pass

## Acceptance Criteria

- [ ] Closures in CFG produce capture information
- [ ] `EscapeAnalysis.captured_vars` contains captured variables
- [ ] `move` closures mark captures as ByValue
- [ ] Mutable captures (`&mut`) detected correctly
- [ ] Nested closures handled (captures propagate up)
- [ ] Iterator chain closures analyzed (map, filter, fold, etc.)
- [ ] Taint propagates through captured variables
- [ ] Returned closures mark captures in return_dependencies
- [ ] Performance stays under 10ms per function total
- [ ] All existing data_flow tests pass

## Technical Details

### Implementation Approach

#### Phase 1: Add Closure Processing to CfgBuilder

```rust
impl CfgBuilder {
    /// Process a closure expression, extracting captures and body information.
    fn process_closure(&mut self, closure: &ExprClosure) {
        // Step 1: Record outer scope variables before entering closure
        let outer_scope_vars = self.current_scope_vars();

        // Step 2: Create closure parameter scope
        let mut closure_params: HashSet<String> = HashSet::new();
        for input in &closure.inputs {
            if let syn::Pat::Ident(pat_ident) = input {
                let param_name = pat_ident.ident.to_string();
                closure_params.insert(param_name.clone());
                // Don't add to main var_names - these are closure-local
            }
        }

        // Step 3: Visit closure body to find captures
        let mut capture_visitor = ClosureCaptureVisitor::new(
            &outer_scope_vars,
            &closure_params,
        );
        capture_visitor.visit_expr(&closure.body);

        // Step 4: Record captured variables
        for capture in capture_visitor.captures {
            let var_id = self.get_or_create_var(&capture.var_name);
            self.captured_vars.insert(CapturedVar {
                var_id,
                capture_mode: capture.mode,
                is_mutated: capture.is_mutated,
            });
        }

        // Step 5: Emit closure expression statement
        let args: Vec<VarId> = capture_visitor.captures
            .iter()
            .map(|c| self.get_or_create_var(&c.var_name))
            .collect();

        self.current_block.push(Statement::Expr {
            expr: ExprKind::Closure {
                captures: args,
                is_move: closure.capture.is_some(),
            },
            line: self.get_span_line_expr(closure),
        });
    }

    /// Get current scope variables for capture detection.
    fn current_scope_vars(&self) -> HashSet<String> {
        self.var_names.keys().cloned().collect()
    }
}
```

#### Phase 2: Add ExprKind::Closure Variant

```rust
/// Expression kinds for side effect tracking
#[derive(Debug, Clone)]
pub enum ExprKind {
    MethodCall {
        receiver: VarId,
        method: String,
        args: Vec<VarId>,
    },
    MacroCall {
        macro_name: String,
        args: Vec<VarId>,
    },
    /// Closure expression with captured variables
    Closure {
        /// Variables captured from outer scope
        captures: Vec<VarId>,
        /// Whether this is a `move` closure
        is_move: bool,
    },
    Other,
}
```

#### Phase 3: ClosureCaptureVisitor Implementation

```rust
/// Visitor to detect captured variables in closure body.
struct ClosureCaptureVisitor<'a> {
    /// Variables available in outer scope (potential captures)
    outer_scope: &'a HashSet<String>,
    /// Closure parameters (not captures)
    closure_params: &'a HashSet<String>,
    /// Detected captures
    captures: Vec<CaptureInfo>,
    /// Variables mutated in closure body
    mutated_vars: HashSet<String>,
}

#[derive(Debug, Clone)]
struct CaptureInfo {
    var_name: String,
    mode: CaptureMode,
    is_mutated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    ByValue,  // move closure
    ByRef,    // &T capture
    ByMutRef, // &mut T capture
}

impl<'a> ClosureCaptureVisitor<'a> {
    fn new(outer_scope: &'a HashSet<String>, closure_params: &'a HashSet<String>) -> Self {
        Self {
            outer_scope,
            closure_params,
            captures: Vec::new(),
            mutated_vars: HashSet::new(),
        }
    }
}

impl<'ast, 'a> Visit<'ast> for ClosureCaptureVisitor<'a> {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            // Variable reference - potential capture
            Expr::Path(path) => {
                if let Some(ident) = path.path.get_ident() {
                    let name = ident.to_string();

                    // Is it in outer scope but not a closure param?
                    if self.outer_scope.contains(&name)
                        && !self.closure_params.contains(&name)
                        && name != "self"
                        && name != "Self"
                    {
                        // Add as capture if not already captured
                        if !self.captures.iter().any(|c| c.var_name == name) {
                            self.captures.push(CaptureInfo {
                                var_name: name,
                                mode: CaptureMode::ByRef, // Default, refined later
                                is_mutated: false,
                            });
                        }
                    }
                }
            }

            // Assignment - track mutations
            Expr::Assign(assign) => {
                if let Expr::Path(path) = &*assign.left {
                    if let Some(ident) = path.path.get_ident() {
                        self.mutated_vars.insert(ident.to_string());
                    }
                }
                // Continue visiting both sides
                syn::visit::visit_expr(self, expr);
                return;
            }

            // Method call on captured var - may mutate
            Expr::MethodCall(method) => {
                if let Expr::Path(path) = &*method.receiver {
                    if let Some(ident) = path.path.get_ident() {
                        let name = ident.to_string();
                        // Check if it's a mutation method
                        if is_mutation_method(&method.method.to_string()) {
                            self.mutated_vars.insert(name);
                        }
                    }
                }
            }

            // Nested closure - recurse
            Expr::Closure(nested_closure) => {
                // Create nested scope
                let mut nested_params: HashSet<String> = self.closure_params.clone();
                for input in &nested_closure.inputs {
                    if let syn::Pat::Ident(pat_ident) = input {
                        nested_params.insert(pat_ident.ident.to_string());
                    }
                }

                let mut nested_visitor = ClosureCaptureVisitor::new(
                    self.outer_scope,
                    &nested_params,
                );
                nested_visitor.visit_expr(&nested_closure.body);

                // Propagate captures from nested closure
                self.captures.extend(nested_visitor.captures);
                self.mutated_vars.extend(nested_visitor.mutated_vars);

                return; // Don't visit nested closure body again
            }

            _ => {}
        }

        syn::visit::visit_expr(self, expr);
    }
}

fn is_mutation_method(method: &str) -> bool {
    matches!(
        method,
        "push" | "pop" | "insert" | "remove" | "clear" | "append"
            | "extend" | "retain" | "truncate" | "swap" | "reverse"
            | "sort" | "sort_by" | "dedup" | "drain" | "split_off"
    )
}
```

#### Phase 4: Integrate with EscapeAnalysis

```rust
impl EscapeAnalysis {
    pub fn analyze(cfg: &ControlFlowGraph) -> Self {
        let mut escaping_vars = HashSet::new();
        let mut captured_vars = HashSet::new();
        let mut return_dependencies = HashSet::new();

        // --- Existing: Collect return dependencies ---
        for block in &cfg.blocks {
            if let Terminator::Return { value: Some(var) } = &block.terminator {
                return_dependencies.insert(*var);
                escaping_vars.insert(*var);
            }
        }

        // --- NEW: Collect captured variables ---
        for block in &cfg.blocks {
            for stmt in &block.statements {
                if let Statement::Expr {
                    expr: ExprKind::Closure { captures, is_move },
                    ..
                } = stmt
                {
                    for &captured_var in captures {
                        captured_vars.insert(captured_var);
                        // Captured variables escape their original scope
                        escaping_vars.insert(captured_var);
                    }

                    // If closure is moved, captured vars have extended lifetime
                    if *is_move {
                        // Mark as strongly escaping
                        for &captured_var in captures {
                            escaping_vars.insert(captured_var);
                        }
                    }
                }
            }
        }

        // --- Existing: Trace return dependencies ---
        let mut worklist: Vec<VarId> = return_dependencies.iter().copied().collect();
        let mut visited = HashSet::new();

        while let Some(var) = worklist.pop() {
            if visited.contains(&var) {
                continue;
            }
            visited.insert(var);

            for block in &cfg.blocks {
                for stmt in &block.statements {
                    match stmt {
                        Statement::Assign { target, source, .. } if target == &var => {
                            Self::add_source_dependencies(
                                source,
                                &mut return_dependencies,
                                &mut worklist,
                            );
                        }
                        Statement::Declare {
                            var: target,
                            init: Some(init),
                            ..
                        } if target == &var => {
                            Self::add_source_dependencies(
                                init,
                                &mut return_dependencies,
                                &mut worklist,
                            );
                        }
                        // NEW: Handle closure captures in return path
                        Statement::Expr {
                            expr: ExprKind::Closure { captures, .. },
                            ..
                        } => {
                            // If this closure is in a return path, its captures
                            // are return dependencies
                            for &captured_var in captures {
                                if !visited.contains(&captured_var) {
                                    return_dependencies.insert(captured_var);
                                    worklist.push(captured_var);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // --- Existing: Method call arguments escape ---
        for block in &cfg.blocks {
            for stmt in &block.statements {
                if let Statement::Expr {
                    expr: ExprKind::MethodCall { args, .. },
                    ..
                } = stmt
                {
                    for arg in args {
                        escaping_vars.insert(*arg);
                    }
                }
            }
        }

        EscapeAnalysis {
            escaping_vars,
            captured_vars,
            return_dependencies,
        }
    }
}
```

#### Phase 5: Integrate with TaintAnalysis

```rust
impl TaintAnalysis {
    pub fn analyze(
        cfg: &ControlFlowGraph,
        liveness: &LivenessInfo,
        escape: &EscapeAnalysis,
    ) -> Self {
        let mut tainted_vars = HashSet::new();
        let taint_sources = HashMap::new();

        // --- Existing: Fixed-point iteration for taint propagation ---
        let mut changed = true;
        while changed {
            changed = false;

            for block in &cfg.blocks {
                for stmt in &block.statements {
                    match stmt {
                        Statement::Assign { target, source, .. } => {
                            if Self::is_source_tainted(source, &tainted_vars)
                                && tainted_vars.insert(*target)
                            {
                                changed = true;
                            }
                        }
                        Statement::Declare {
                            var,
                            init: Some(init),
                            ..
                        } => {
                            if Self::is_source_tainted(init, &tainted_vars)
                                && tainted_vars.insert(*var)
                            {
                                changed = true;
                            }
                        }
                        // NEW: Taint propagation through closures
                        Statement::Expr {
                            expr: ExprKind::Closure { captures, .. },
                            ..
                        } => {
                            // If any captured var is tainted, consider closure tainted
                            // and all captured vars as potentially affected
                            let any_tainted = captures.iter()
                                .any(|c| tainted_vars.contains(c));

                            if any_tainted {
                                for &captured_var in captures {
                                    if tainted_vars.insert(captured_var) {
                                        changed = true;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Remove dead stores from tainted vars
        tainted_vars.retain(|var| !liveness.dead_stores.contains(var));

        // Check if captured vars contribute to return (via escape.captured_vars)
        let captured_tainted = tainted_vars.iter()
            .any(|var| escape.captured_vars.contains(var));

        let return_tainted = tainted_vars
            .iter()
            .any(|var| escape.return_dependencies.contains(var))
            || captured_tainted;

        TaintAnalysis {
            tainted_vars,
            taint_sources,
            return_tainted,
        }
    }
}
```

### Architecture Changes

1. **New data structures**:
   - `CapturedVar` struct in CfgBuilder
   - `CaptureInfo` and `CaptureMode` for visitor
   - `ExprKind::Closure` variant

2. **Modified components**:
   - `CfgBuilder::process_expr` - handle Expr::Closure
   - `EscapeAnalysis::analyze` - populate captured_vars
   - `TaintAnalysis::analyze` - propagate through captures

### Data Structures

```rust
/// Captured variable in a closure
#[derive(Debug, Clone)]
pub struct CapturedVar {
    pub var_id: VarId,
    pub capture_mode: CaptureMode,
    pub is_mutated: bool,
}

/// Capture mode for closure variables
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    ByValue,  // move closure
    ByRef,    // &T capture
    ByMutRef, // &mut T capture
}
```

### APIs and Interfaces

No changes to public API. Internal methods added to CfgBuilder.

## Dependencies

- **Prerequisites**: Spec 248 (Enhanced Expression Variable Extraction) - needed for proper var extraction
- **Affected Components**:
  - `src/analysis/data_flow.rs` - Main implementation
  - Tests for data_flow.rs
- **External Dependencies**: None (uses existing syn crate)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod closure_capture_tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_simple_closure_capture() {
        let block: Block = parse_quote!({
            let x = 1;
            let f = |y| x + y;
            f(2)
        });

        let cfg = ControlFlowGraph::from_block(&block);
        let escape = EscapeAnalysis::analyze(&cfg);

        // x should be in captured_vars
        let x_var = cfg.var_names.iter()
            .position(|n| n == "x")
            .map(|i| VarId { name_id: i as u32, version: 0 });

        assert!(x_var.is_some());
        assert!(escape.captured_vars.contains(&x_var.unwrap()));
    }

    #[test]
    fn test_move_closure_capture() {
        let block: Block = parse_quote!({
            let data = vec![1, 2, 3];
            let f = move || data.len();
            f()
        });

        let cfg = ControlFlowGraph::from_block(&block);

        // Should detect data as captured with ByValue mode
        // (Implementation would need to track capture mode)
        assert!(!cfg.blocks.is_empty());
    }

    #[test]
    fn test_mutable_capture() {
        let block: Block = parse_quote!({
            let mut counter = 0;
            let mut inc = || counter += 1;
            inc();
            inc();
        });

        let cfg = ControlFlowGraph::from_block(&block);
        let escape = EscapeAnalysis::analyze(&cfg);

        // counter should be captured and marked as escaping
        assert!(!escape.captured_vars.is_empty());
    }

    #[test]
    fn test_iterator_chain_captures() {
        let block: Block = parse_quote!({
            let threshold = 5;
            let items = vec![1, 2, 3, 4, 5, 6];
            items.iter()
                .filter(|x| **x > threshold)
                .map(|x| x * 2)
                .collect::<Vec<_>>()
        });

        let cfg = ControlFlowGraph::from_block(&block);
        let escape = EscapeAnalysis::analyze(&cfg);

        // threshold should be captured by filter closure
        let threshold_var = cfg.var_names.iter()
            .position(|n| n == "threshold")
            .map(|i| VarId { name_id: i as u32, version: 0 });

        if let Some(var) = threshold_var {
            assert!(escape.captured_vars.contains(&var));
        }
    }

    #[test]
    fn test_nested_closure_captures() {
        let block: Block = parse_quote!({
            let x = 1;
            let outer = || {
                let y = 2;
                let inner = || x + y;
                inner()
            };
            outer()
        });

        let cfg = ControlFlowGraph::from_block(&block);
        let escape = EscapeAnalysis::analyze(&cfg);

        // x should be captured (propagated from nested closure)
        assert!(!escape.captured_vars.is_empty());
    }

    #[test]
    fn test_taint_through_capture() {
        let block: Block = parse_quote!({
            let mut tainted = get_user_input();
            let f = || tainted.clone();
            f()
        });

        let cfg = ControlFlowGraph::from_block(&block);
        let liveness = LivenessInfo::analyze(&cfg);
        let escape = EscapeAnalysis::analyze(&cfg);
        let taint = TaintAnalysis::analyze(&cfg, &liveness, &escape);

        // Return should be tainted because tainted var is captured
        // (depends on how taint sources are initialized)
    }

    #[test]
    fn test_closure_no_capture() {
        let block: Block = parse_quote!({
            let f = |x, y| x + y;
            f(1, 2)
        });

        let cfg = ControlFlowGraph::from_block(&block);
        let escape = EscapeAnalysis::analyze(&cfg);

        // No captures expected
        assert!(escape.captured_vars.is_empty());
    }
}
```

### Integration Tests

```rust
#[test]
fn test_purity_with_closure_capture() {
    // Test that purity detection uses CFG closure capture info
    let func: ItemFn = parse_quote!(
        fn process(data: &[i32], threshold: i32) -> Vec<i32> {
            data.iter()
                .filter(|x| **x > threshold)
                .copied()
                .collect()
        }
    );

    let mut detector = PurityDetector::new();
    let analysis = detector.is_pure_function(&func);

    // Should be strictly pure (no mutations, threshold captured by ref)
    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
}
```

### Performance Tests

```rust
#[test]
fn test_closure_analysis_performance() {
    use std::time::Instant;

    // Many closures with captures
    let block: Block = parse_quote!({
        let a = 1;
        let b = 2;
        let c = 3;
        let f1 = || a + 1;
        let f2 = || a + b;
        let f3 = || a + b + c;
        let f4 = move || a * b * c;
        f1() + f2() + f3() + f4()
    });

    let start = Instant::now();
    for _ in 0..100 {
        let cfg = ControlFlowGraph::from_block(&block);
        let _ = EscapeAnalysis::analyze(&cfg);
    }
    let elapsed = start.elapsed();

    // 100 iterations should complete in <200ms (2ms per iteration)
    assert!(elapsed.as_millis() < 200, "Performance regression: {:?}", elapsed);
}
```

## Documentation Requirements

- **Code Documentation**: Add rustdoc to all new structs and functions
- **User Documentation**: No changes (internal implementation)
- **Architecture Updates**: Update data_flow.rs module documentation

## Implementation Notes

### Order of Operations

1. First implement Spec 248 (Expression Variable Extraction)
2. Then implement closure processing in CfgBuilder
3. Then integrate with EscapeAnalysis
4. Finally integrate with TaintAnalysis

### Edge Cases

1. **Async closures**: Currently treat same as regular closures
2. **Generator closures**: Not yet in stable Rust, can ignore
3. **Closure as function argument**: Track captures regardless of position
4. **Returned closures**: Mark captures as return dependencies

### Integration with Existing ClosureAnalyzer

The existing `src/analyzers/closure_analyzer.rs` can be reused:
- Use `ClosurePurity` results to inform CFG capture modes
- Share `CaptureMode` enum between modules
- Consider consolidating capture detection logic

## Migration and Compatibility

- **No migration needed**: New functionality, existing API unchanged
- **Backward compatible**: `captured_vars` was always empty, now populated
- **Gradual benefit**: Downstream analyses automatically improve
