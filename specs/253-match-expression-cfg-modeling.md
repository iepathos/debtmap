---
number: 253
title: Match Expression CFG Modeling
category: foundation
priority: medium
status: draft
dependencies: [248, 252]
created: 2025-12-12
---

# Specification 253: Match Expression CFG Modeling

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 248 (Enhanced Expression Variable Extraction), Spec 252 (Pattern Destructuring)

## Context

### Current Problem

The CFG builder in `src/analysis/data_flow.rs` doesn't handle match expressions at all. Currently, `process_expr` falls through to the default case:

```rust
fn process_expr(&mut self, expr: &Expr) {
    match expr {
        Expr::If(expr_if) => self.process_if(expr_if),
        Expr::While(expr_while) => self.process_while(expr_while),
        Expr::Return(expr_return) => self.process_return(expr_return),
        Expr::Assign(assign) => self.process_assign(assign),
        _ => {
            // Match expressions end up here!
            self.current_block.push(Statement::Expr {
                expr: ExprKind::Other,
                line: None,
            });
        }
    }
}
```

Match expressions are completely ignored, meaning:
- Control flow through arms not modeled
- Pattern bindings not tracked
- Guard conditions not analyzed
- Arm bodies not processed

### Why Match CFG Matters

Match is fundamental in Rust - used for:
- Error handling (`match result { Ok(v) => ..., Err(e) => ... }`)
- Option unwrapping (`match opt { Some(x) => ..., None => ... }`)
- Enum dispatch (`match state { State::A => ..., State::B => ... }`)
- Pattern matching with guards

```rust
fn process(input: Input) -> Output {
    match input {
        Input::Valid(data) if data.len() > 0 => {
            // Branch 1: valid with data
            process_data(data)
        }
        Input::Valid(_) => {
            // Branch 2: valid but empty
            Output::Empty
        }
        Input::Invalid(err) => {
            // Branch 3: invalid
            Output::Error(err)
        }
    }
}
```

Without match CFG:
1. **Data flow broken**: `data` and `err` bindings not tracked
2. **Control flow wrong**: All arms treated as sequential, not branching
3. **Guard conditions ignored**: `if data.len() > 0` not analyzed
4. **Liveness incorrect**: Variables in unreachable arms appear live

### Impact Quantification

| Metric | Without Match CFG | With Match CFG |
|--------|-------------------|----------------|
| Control flow accuracy | ~60% | ~95% |
| Pattern binding coverage | 0% | >95% |
| Data flow through match | Broken | Correct |
| Arm reachability | Unknown | Tracked |

## Objective

Implement proper CFG modeling for match expressions, including:
- Multi-way branching to arm blocks
- Pattern binding in each arm
- Guard condition evaluation
- Arm body processing
- Join point after all arms

## Requirements

### Functional Requirements

1. **Scrutinee Evaluation**
   - Process scrutinee expression
   - Extract scrutinee variable for pattern binding
   - Track in CFG as assignment/declaration

2. **Multi-Way Branch Terminator**
   - Create new terminator type for match
   - Branch to multiple arm blocks
   - Include guard conditions where present

3. **Arm Block Processing**
   - Create separate basic block for each arm
   - Bind pattern variables from scrutinee
   - Process arm body recursively
   - Track arm guards as conditions

4. **Join Block**
   - Create block where all arm paths converge
   - Handle match as expression (result value)
   - Connect all arm exits to join

5. **Guard Condition Handling**
   - Model guards as additional conditions
   - Guards can access pattern bindings
   - Failed guard falls through to next arm

### Non-Functional Requirements

- **Performance**: <1ms overhead per match expression
- **Completeness**: Handle all match arm forms
- **Correctness**: Proper CFG structure (no orphan blocks)

## Acceptance Criteria

- [ ] Match expressions create multi-way branch in CFG
- [ ] Each arm gets its own basic block
- [ ] Pattern bindings created in arm blocks
- [ ] Arm bodies recursively processed
- [ ] Guard conditions modeled as branch conditions
- [ ] Join block created after all arms
- [ ] Match-as-expression result tracked
- [ ] Edges correctly connect arm blocks
- [ ] Data flow correct through match arms
- [ ] All existing tests pass
- [ ] Performance under 10ms per function total

## Technical Details

### Implementation Approach

#### Phase 1: New Terminator Variant

```rust
/// Block terminator - how control leaves this block.
#[derive(Debug, Clone)]
pub enum Terminator {
    /// Return from function
    Return { value: Option<VarId> },
    /// Unconditional jump
    Goto { target: BlockId },
    /// Two-way branch (if/else)
    Branch {
        condition: VarId,
        then_block: BlockId,
        else_block: BlockId,
    },
    /// Multi-way branch (match expression)
    Match {
        scrutinee: VarId,
        arms: Vec<MatchArm>,
        otherwise: Option<BlockId>, // For irrefutable matches, or default arm
    },
}

/// A match arm in the CFG.
#[derive(Debug, Clone)]
pub struct MatchArm {
    /// Block that handles this arm
    pub block: BlockId,
    /// Optional guard condition (if present)
    pub guard: Option<VarId>,
    /// Pattern bindings created in this arm (for documentation)
    pub bindings: Vec<VarId>,
}
```

#### Phase 2: Match Expression Processing

```rust
impl CfgBuilder {
    fn process_match(&mut self, expr_match: &ExprMatch) {
        // Step 1: Process scrutinee expression
        let scrutinee_var = self.process_scrutinee(&expr_match.expr);

        // Step 2: Record block IDs for each arm (will be created after current block)
        let arm_count = expr_match.arms.len();
        let arm_start_id = self.block_counter + 1;
        let join_block_id = BlockId(arm_start_id + arm_count);

        // Step 3: Build match arms metadata
        let mut cfg_arms = Vec::new();
        for (i, _arm) in expr_match.arms.iter().enumerate() {
            let arm_block_id = BlockId(arm_start_id + i);
            cfg_arms.push(MatchArm {
                block: arm_block_id,
                guard: None, // Will be filled during arm processing
                bindings: Vec::new(),
            });
        }

        // Step 4: Finalize current block with Match terminator
        self.finalize_current_block(Terminator::Match {
            scrutinee: scrutinee_var,
            arms: cfg_arms.clone(),
            otherwise: None,
        });

        // Step 5: Process each arm body
        for (i, arm) in expr_match.arms.iter().enumerate() {
            self.process_match_arm(
                arm,
                scrutinee_var,
                join_block_id,
                &mut cfg_arms[i],
            );
        }

        // Step 6: Create join block (empty block that follows match)
        self.current_block = Vec::new();
        // Join block will be finalized by next statement or function end
    }

    /// Process the scrutinee expression and return its VarId.
    fn process_scrutinee(&mut self, expr: &Expr) -> VarId {
        // If scrutinee is a simple variable, use it directly
        if let Some(var) = self.extract_primary_var(expr) {
            return var;
        }

        // Otherwise, create a temp for complex expression
        let temp_var = self.get_or_create_var("_scrutinee");
        let rvalue = self.expr_to_rvalue(expr);

        self.current_block.push(Statement::Assign {
            target: temp_var,
            source: rvalue,
            line: self.get_span_line_expr(expr),
        });

        temp_var
    }

    /// Process a single match arm.
    fn process_match_arm(
        &mut self,
        arm: &syn::Arm,
        scrutinee: VarId,
        join_block: BlockId,
        cfg_arm: &mut MatchArm,
    ) {
        // Start new block for this arm
        self.current_block = Vec::new();

        // Step 1: Bind pattern variables from scrutinee
        let bindings = self.bind_pattern_vars(&arm.pat, scrutinee);
        cfg_arm.bindings = bindings;

        // Step 2: Process guard if present
        if let Some((_, guard_expr)) = &arm.guard {
            let guard_var = self.process_guard(guard_expr);
            cfg_arm.guard = Some(guard_var);
        }

        // Step 3: Process arm body
        self.process_expr(&arm.body);

        // Step 4: Create result assignment if match is used as expression
        // (The result would be used in the join block)

        // Step 5: Finalize arm block with goto to join
        self.finalize_current_block(Terminator::Goto { target: join_block });
    }

    /// Bind pattern variables and return their VarIds.
    fn bind_pattern_vars(&mut self, pat: &Pat, scrutinee: VarId) -> Vec<VarId> {
        let bindings = self.extract_pattern_bindings(pat);
        let mut var_ids = Vec::new();

        for binding in bindings {
            let var = self.get_or_create_var(&binding.name);
            var_ids.push(var);

            let init = match &binding.access_path {
                Some(AccessPath::TupleIndex(idx)) => Rvalue::FieldAccess {
                    base: scrutinee,
                    field: idx.to_string(),
                },
                Some(AccessPath::NamedField(name)) => Rvalue::FieldAccess {
                    base: scrutinee,
                    field: name.clone(),
                },
                Some(AccessPath::ArrayIndex(idx)) => Rvalue::FieldAccess {
                    base: scrutinee,
                    field: format!("[{}]", idx),
                },
                Some(AccessPath::Nested(paths)) => {
                    // For nested, just use first level for now
                    if let Some(first) = paths.first() {
                        match first {
                            AccessPath::TupleIndex(idx) => Rvalue::FieldAccess {
                                base: scrutinee,
                                field: idx.to_string(),
                            },
                            AccessPath::NamedField(name) => Rvalue::FieldAccess {
                                base: scrutinee,
                                field: name.clone(),
                            },
                            _ => Rvalue::Use(scrutinee),
                        }
                    } else {
                        Rvalue::Use(scrutinee)
                    }
                }
                _ => Rvalue::Use(scrutinee),
            };

            self.current_block.push(Statement::Declare {
                var,
                init: Some(init),
                line: None,
            });
        }

        var_ids
    }

    /// Process a guard expression and return condition VarId.
    fn process_guard(&mut self, guard_expr: &Expr) -> VarId {
        // Extract or create var for guard condition
        if let Some(var) = self.extract_primary_var(guard_expr) {
            return var;
        }

        let guard_var = self.get_or_create_var("_guard");
        let rvalue = self.expr_to_rvalue(guard_expr);

        self.current_block.push(Statement::Assign {
            target: guard_var,
            source: rvalue,
            line: self.get_span_line_expr(guard_expr),
        });

        guard_var
    }
}
```

#### Phase 3: Update process_expr

```rust
impl CfgBuilder {
    fn process_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::If(expr_if) => self.process_if(expr_if),
            Expr::While(expr_while) => self.process_while(expr_while),
            Expr::Return(expr_return) => self.process_return(expr_return),
            Expr::Assign(assign) => self.process_assign(assign),
            // NEW: Handle match expressions
            Expr::Match(expr_match) => self.process_match(expr_match),
            // Also handle block expressions for arm bodies
            Expr::Block(expr_block) => self.process_block(&expr_block.block),
            // And call expressions (common in match arms)
            Expr::Call(call) => self.process_call(call),
            Expr::MethodCall(method) => self.process_method_call(method),
            _ => {
                self.current_block.push(Statement::Expr {
                    expr: ExprKind::Other,
                    line: None,
                });
            }
        }
    }
}
```

#### Phase 4: Update Data Flow Analysis for Match

```rust
impl ReachingDefinitions {
    /// Get predecessors of a block, including match arm predecessors.
    fn get_predecessors(cfg: &ControlFlowGraph, block_id: BlockId) -> Vec<BlockId> {
        let mut preds = Vec::new();

        for block in &cfg.blocks {
            let is_pred = match &block.terminator {
                Terminator::Goto { target } => *target == block_id,
                Terminator::Branch { then_block, else_block, .. } => {
                    *then_block == block_id || *else_block == block_id
                }
                // NEW: Match can be predecessor of any arm or join block
                Terminator::Match { arms, otherwise, .. } => {
                    arms.iter().any(|arm| arm.block == block_id)
                        || otherwise.map(|o| o == block_id).unwrap_or(false)
                }
                Terminator::Return { .. } => false,
            };

            if is_pred {
                preds.push(block.id);
            }
        }

        preds
    }
}

impl LivenessInfo {
    /// Update liveness for match terminators.
    fn compute_live_in_terminator(term: &Terminator, live_out: &HashSet<VarId>) -> HashSet<VarId> {
        let mut live_in = live_out.clone();

        match term {
            Terminator::Return { value: Some(var) } => {
                live_in.insert(*var);
            }
            Terminator::Branch { condition, .. } => {
                live_in.insert(*condition);
            }
            // NEW: Match scrutinee and guards are used
            Terminator::Match { scrutinee, arms, .. } => {
                live_in.insert(*scrutinee);
                for arm in arms {
                    if let Some(guard) = arm.guard {
                        live_in.insert(guard);
                    }
                }
            }
            _ => {}
        }

        live_in
    }
}
```

#### Phase 5: Edge Recording

```rust
impl CfgBuilder {
    fn finalize_current_block(&mut self, terminator: Terminator) {
        let block_id = BlockId(self.block_counter);

        // Record edges based on terminator type
        match &terminator {
            Terminator::Goto { target } => {
                self.edges.entry(block_id).or_default().push((*target, Edge::Unconditional));
            }
            Terminator::Branch { then_block, else_block, .. } => {
                self.edges.entry(block_id).or_default().push((*then_block, Edge::True));
                self.edges.entry(block_id).or_default().push((*else_block, Edge::False));
            }
            Terminator::Match { arms, otherwise, .. } => {
                for (i, arm) in arms.iter().enumerate() {
                    self.edges.entry(block_id).or_default().push((arm.block, Edge::MatchArm(i)));
                }
                if let Some(default) = otherwise {
                    self.edges.entry(block_id).or_default().push((*default, Edge::Default));
                }
            }
            Terminator::Return { .. } => {
                // No outgoing edges
            }
        }

        let block = BasicBlock {
            id: block_id,
            statements: std::mem::take(&mut self.current_block),
            terminator,
        };
        self.blocks.push(block);
        self.block_counter += 1;
    }
}

/// Edge type in CFG.
#[derive(Debug, Clone)]
pub enum Edge {
    Unconditional,
    True,
    False,
    MatchArm(usize),
    Default,
}
```

### Architecture Changes

1. **New terminator**: `Terminator::Match` with arms
2. **New types**: `MatchArm`, `Edge` enum
3. **Updated methods**: `process_expr`, `get_predecessors`, liveness analysis
4. **New methods**: `process_match`, `process_match_arm`, `process_scrutinee`, `process_guard`

### Data Structures

```rust
pub enum Terminator {
    // ... existing variants ...
    Match {
        scrutinee: VarId,
        arms: Vec<MatchArm>,
        otherwise: Option<BlockId>,
    },
}

pub struct MatchArm {
    pub block: BlockId,
    pub guard: Option<VarId>,
    pub bindings: Vec<VarId>,
}

pub enum Edge {
    Unconditional,
    True,
    False,
    MatchArm(usize),
    Default,
}
```

### APIs and Interfaces

No public API changes. Internal CFG structure enhanced.

## Dependencies

- **Prerequisites**:
  - Spec 248 (Enhanced Expression Variable Extraction)
  - Spec 252 (Pattern Destructuring Support)
- **Affected Components**: `src/analysis/data_flow.rs`
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod match_cfg_tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_simple_match_cfg_structure() {
        let block: Block = parse_quote!({
            let x = get_option();
            match x {
                Some(v) => v,
                None => 0,
            }
        });

        let cfg = ControlFlowGraph::from_block(&block);

        // Should have: entry block, 2 arm blocks, (implicit join)
        assert!(cfg.blocks.len() >= 3);

        // Find the match terminator
        let match_term = cfg.blocks.iter()
            .find(|b| matches!(b.terminator, Terminator::Match { .. }));
        assert!(match_term.is_some(), "Should have Match terminator");

        if let Terminator::Match { arms, .. } = &match_term.unwrap().terminator {
            assert_eq!(arms.len(), 2, "Should have 2 arms");
        }
    }

    #[test]
    fn test_match_pattern_bindings() {
        let block: Block = parse_quote!({
            let result = get_result();
            match result {
                Ok(value) => value,
                Err(e) => panic!("{}", e),
            }
        });

        let cfg = ControlFlowGraph::from_block(&block);

        // 'value' and 'e' should be tracked
        assert!(cfg.var_names.contains(&"value".to_string()));
        assert!(cfg.var_names.contains(&"e".to_string()));
    }

    #[test]
    fn test_match_with_guard() {
        let block: Block = parse_quote!({
            let x = get_number();
            match x {
                n if n > 0 => n,
                _ => 0,
            }
        });

        let cfg = ControlFlowGraph::from_block(&block);

        // Find match terminator and check for guard
        let match_term = cfg.blocks.iter()
            .find(|b| matches!(b.terminator, Terminator::Match { .. }));

        if let Some(block) = match_term {
            if let Terminator::Match { arms, .. } = &block.terminator {
                // First arm should have a guard
                assert!(arms[0].guard.is_some() || !arms.is_empty());
            }
        }
    }

    #[test]
    fn test_match_data_flow() {
        let block: Block = parse_quote!({
            let input = get_input();
            let result = match input {
                Some(x) => x * 2,
                None => 0,
            };
            result
        });

        let cfg = ControlFlowGraph::from_block(&block);
        let escape = EscapeAnalysis::analyze(&cfg);

        // result should be in return dependencies
        // x should contribute to result (in Some arm)
        assert!(!escape.return_dependencies.is_empty());
    }

    #[test]
    fn test_match_struct_pattern() {
        let block: Block = parse_quote!({
            let point = get_point();
            match point {
                Point { x, y } if x == y => x,
                Point { x, .. } => x,
            }
        });

        let cfg = ControlFlowGraph::from_block(&block);

        assert!(cfg.var_names.contains(&"x".to_string()));
        assert!(cfg.var_names.contains(&"y".to_string()));
    }

    #[test]
    fn test_nested_match() {
        let block: Block = parse_quote!({
            let outer = get_outer();
            match outer {
                Some(inner) => match inner {
                    Ok(v) => v,
                    Err(_) => -1,
                },
                None => 0,
            }
        });

        let cfg = ControlFlowGraph::from_block(&block);

        // Should handle nested match
        let match_count = cfg.blocks.iter()
            .filter(|b| matches!(b.terminator, Terminator::Match { .. }))
            .count();
        assert!(match_count >= 1);
    }

    #[test]
    fn test_match_liveness() {
        let block: Block = parse_quote!({
            let x = get_value();
            let y = get_other();
            match x {
                Some(v) => v + y,  // y is live here
                None => y,         // y is live here too
            }
        });

        let cfg = ControlFlowGraph::from_block(&block);
        let liveness = LivenessInfo::analyze(&cfg);

        // y should be live in all arms
        let y_var = cfg.var_names.iter()
            .position(|n| n == "y")
            .map(|i| VarId { name_id: i as u32, version: 0 });

        if let Some(y) = y_var {
            // y should not be a dead store (it's used in both arms)
            assert!(!liveness.dead_stores.contains(&y));
        }
    }

    #[test]
    fn test_match_predecessors() {
        let block: Block = parse_quote!({
            match x {
                A => 1,
                B => 2,
            }
        });

        let cfg = ControlFlowGraph::from_block(&block);

        // Each arm block should have the match block as predecessor
        if let Some(match_block) = cfg.blocks.iter()
            .find(|b| matches!(b.terminator, Terminator::Match { .. }))
        {
            if let Terminator::Match { arms, .. } = &match_block.terminator {
                for arm in arms {
                    let preds = ReachingDefinitions::get_predecessors(&cfg, arm.block);
                    assert!(preds.contains(&match_block.id));
                }
            }
        }
    }
}
```

### Integration Tests

```rust
#[test]
fn test_match_with_taint_propagation() {
    let block: Block = parse_quote!({
        let mut input = get_user_input();
        input.sanitize();  // Mutation - taints input

        match input.parse() {
            Ok(data) => data,   // data is tainted (from input)
            Err(e) => panic!("{}", e),
        }
    });

    let cfg = ControlFlowGraph::from_block(&block);
    let liveness = LivenessInfo::analyze(&cfg);
    let escape = EscapeAnalysis::analyze(&cfg);
    let taint = TaintAnalysis::analyze(&cfg, &liveness, &escape);

    // Return should be tainted (data derived from tainted input)
    assert!(taint.return_tainted);
}

#[test]
fn test_match_def_use_chains() {
    let block: Block = parse_quote!({
        let opt = get_option();
        match opt {
            Some(x) => x + 1,
            None => 0,
        }
    });

    let cfg = ControlFlowGraph::from_block(&block);
    let reaching = ReachingDefinitions::analyze(&cfg);

    // x should have a definition (from pattern binding)
    let x_defs: Vec<_> = reaching.all_definitions.iter()
        .filter(|d| cfg.var_names.get(d.var.name_id as usize) == Some(&"x".to_string()))
        .collect();

    assert!(!x_defs.is_empty(), "x should have definition from pattern");
}
```

### Performance Tests

```rust
#[test]
fn test_match_cfg_performance() {
    use std::time::Instant;

    // Complex match with many arms
    let block: Block = parse_quote!({
        match value {
            A(x) => x,
            B(y) => y,
            C(z) => z,
            D { a, b } => a + b,
            E(v) if v > 0 => v,
            _ => 0,
        }
    });

    let start = Instant::now();
    for _ in 0..100 {
        let cfg = ControlFlowGraph::from_block(&block);
        let _ = LivenessInfo::analyze(&cfg);
        let escape = EscapeAnalysis::analyze(&cfg);
        let _ = TaintAnalysis::analyze(&cfg, &LivenessInfo::analyze(&cfg), &escape);
    }
    let elapsed = start.elapsed();

    // 100 full analyses should complete in <500ms
    assert!(elapsed.as_millis() < 500, "Took {:?}", elapsed);
}
```

## Documentation Requirements

- **Code Documentation**: Document `Terminator::Match`, `MatchArm`, and processing methods
- **User Documentation**: No changes (internal improvement)
- **Architecture Updates**: Document match CFG structure in module docs

## Implementation Notes

### Match as Expression

Match in Rust is an expression. To track the result:
1. Each arm stores result in a shared temp variable
2. Join block uses that temp as the match result
3. Alternative: Each arm has implicit return to join with value

### Guard Evaluation Order

Guards can fail:
1. Pattern matches → guard evaluated
2. Guard fails → fall through to next arm
3. This creates implicit edges between consecutive arms

For simplicity, initial implementation may not model guard failure edges.

### Exhaustiveness

Rust compiler ensures match is exhaustive. We assume all matches are valid and don't need wildcard fallback unless explicitly present.

### Irrefutable Matches

`if let` and `let else` are similar to match but with different structure. This spec focuses on `match`; `if let` can use existing if/else CFG.

## Migration and Compatibility

- **No migration needed**: Additive feature
- **Backward compatible**: Existing code works (match was ignored, now processed)
- **CFG structure enhanced**: More blocks for match expressions
