---
number: 250
title: Statement-Level Def-Use Chains
category: foundation
priority: medium
status: draft
dependencies: [248]
created: 2025-12-12
---

# Specification 250: Statement-Level Def-Use Chains

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 248 (Enhanced Expression Variable Extraction)

## Context

### Current Problem

The `ReachingDefinitions` analysis in `src/analysis/data_flow.rs` tracks definitions at **block-level** granularity:

```rust
// data_flow.rs:662-670
#[derive(Debug, Clone)]
pub struct ReachingDefinitions {
    /// Definitions that reach the entry of each block
    pub reach_in: HashMap<BlockId, HashSet<VarId>>,
    /// Definitions that reach the exit of each block
    pub reach_out: HashMap<BlockId, HashSet<VarId>>,
    /// Def-use chains: maps each definition to the program points where it's used
    pub def_use_chains: HashMap<VarId, HashSet<BlockId>>,  // <-- Only BlockId, not statement
}
```

The `def_use_chains` only knows **which block** uses a definition, not **which statement** within that block.

### Why Statement-Level Precision Matters

Consider this code:

```rust
fn example() {
    let mut x = 1;     // Statement 0: def of x.0
    x = 2;             // Statement 1: def of x.1, use of x.0 (dead)
    let y = x + 1;     // Statement 2: use of x.1
    x = y;             // Statement 3: def of x.2, use of y
    println!("{}", x); // Statement 4: use of x.2
}
```

With block-level tracking:
- We know x.1 is defined in block 0 and used in block 0
- But we can't precisely say "x.1 is used at statement 2, x.2 is used at statement 4"

With statement-level tracking:
- We can build precise def-use chains: `x.1 -> [(Block 0, Stmt 2)]`
- Dead store detection becomes more accurate
- SSA-style analysis becomes possible

### Current Limitations

1. **Dead Store Imprecision**: Can't distinguish between multiple assignments in same block
2. **Data Flow Path Ambiguity**: Know block but not exact point of use
3. **Optimization Hints Lost**: Can't identify which specific statements are affected
4. **Debugging Difficulty**: Hard to trace specific definition to specific use

## Objective

Extend `ReachingDefinitions` to track **statement-level** def-use chains, enabling precise identification of which statement defines each use.

## Requirements

### Functional Requirements

1. **Statement Index Tracking**
   - Add `StatementIdx` type (usize index within block)
   - Record (BlockId, StatementIdx) for each definition point
   - Record (BlockId, StatementIdx) for each use point

2. **Enhanced Def-Use Chains**
   - Map each definition to exact use locations
   - Map each use to its reaching definitions
   - Support multiple definitions reaching same use (when from different paths)

3. **Use-Def Chains (Inverse)**
   - Map each use to its defining statements
   - Enable backward analysis from use to definition

4. **Dead Store Precision**
   - Identify dead stores within same block (currently missed)
   - Track if assignment is dead because it's immediately overwritten

5. **Integration with Existing Analysis**
   - Preserve backward compatibility with block-level API
   - Enhance LivenessInfo with statement-level precision

### Non-Functional Requirements

- **Performance**: Add <1ms overhead per function
- **Memory**: Linear increase proportional to number of statements
- **Backward Compatibility**: Existing API continues to work

## Acceptance Criteria

- [ ] `StatementIdx` type introduced for statement identification
- [ ] Def-use chains track (BlockId, StatementIdx) pairs
- [ ] Use-def chains (inverse) available
- [ ] Same-block dead stores detected
- [ ] Multiple definitions reaching same use handled
- [ ] Block-level API unchanged (backward compatible)
- [ ] All existing tests pass
- [ ] New tests verify statement-level precision
- [ ] Performance under 10ms per function total

## Technical Details

### Implementation Approach

#### Phase 1: New Data Types

```rust
/// Index of a statement within a basic block.
pub type StatementIdx = usize;

/// A specific program point: block and statement within that block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProgramPoint {
    pub block: BlockId,
    pub stmt: StatementIdx,
}

impl ProgramPoint {
    pub fn new(block: BlockId, stmt: StatementIdx) -> Self {
        Self { block, stmt }
    }

    /// Create a point at the start of a block (before first statement).
    pub fn block_entry(block: BlockId) -> Self {
        Self { block, stmt: 0 }
    }

    /// Create a point at the end of a block (after last statement).
    pub fn block_exit(block: BlockId, stmt_count: usize) -> Self {
        Self { block, stmt: stmt_count }
    }
}

/// A definition occurrence: variable defined at a specific point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Definition {
    pub var: VarId,
    pub point: ProgramPoint,
}

/// A use occurrence: variable used at a specific point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Use {
    pub var: VarId,
    pub point: ProgramPoint,
}
```

#### Phase 2: Enhanced ReachingDefinitions

```rust
/// Reaching definitions analysis with statement-level precision.
#[derive(Debug, Clone)]
pub struct ReachingDefinitions {
    // --- Block-level (existing, preserved for compatibility) ---

    /// Definitions that reach the entry of each block
    pub reach_in: HashMap<BlockId, HashSet<VarId>>,
    /// Definitions that reach the exit of each block
    pub reach_out: HashMap<BlockId, HashSet<VarId>>,
    /// Def-use chains at block level (backward compatibility)
    pub def_use_chains: HashMap<VarId, HashSet<BlockId>>,

    // --- Statement-level (new) ---

    /// Precise def-use chains: definition point → use points
    pub precise_def_use: HashMap<Definition, HashSet<ProgramPoint>>,
    /// Use-def chains (inverse): use point → reaching definitions
    pub use_def_chains: HashMap<Use, HashSet<Definition>>,
    /// All definitions in the program
    pub all_definitions: Vec<Definition>,
    /// All uses in the program
    pub all_uses: Vec<Use>,
}
```

#### Phase 3: Collecting Definitions and Uses

```rust
impl ReachingDefinitions {
    /// Collect all definitions and uses with their program points.
    fn collect_defs_and_uses(cfg: &ControlFlowGraph) -> (Vec<Definition>, Vec<Use>) {
        let mut definitions = Vec::new();
        let mut uses = Vec::new();

        for block in &cfg.blocks {
            for (stmt_idx, stmt) in block.statements.iter().enumerate() {
                let point = ProgramPoint::new(block.id, stmt_idx);

                match stmt {
                    Statement::Declare { var, init, .. } => {
                        // This is a definition
                        definitions.push(Definition { var: *var, point });

                        // Init expression may use variables
                        if let Some(init_rval) = init {
                            for used_var in Self::rvalue_uses(init_rval) {
                                uses.push(Use { var: used_var, point });
                            }
                        }
                    }

                    Statement::Assign { target, source, .. } => {
                        // This is a definition
                        definitions.push(Definition { var: *target, point });

                        // Source uses variables
                        for used_var in Self::rvalue_uses(source) {
                            uses.push(Use { var: used_var, point });
                        }
                    }

                    Statement::Expr { expr, .. } => {
                        // Expression may use variables
                        for used_var in Self::expr_kind_uses(expr) {
                            uses.push(Use { var: used_var, point });
                        }
                    }
                }
            }

            // Terminator may use variables
            let term_point = ProgramPoint::block_exit(block.id, block.statements.len());
            for used_var in Self::terminator_uses(&block.terminator) {
                uses.push(Use { var: used_var, point: term_point });
            }
        }

        (definitions, uses)
    }

    /// Extract variables used in an Rvalue.
    fn rvalue_uses(rval: &Rvalue) -> Vec<VarId> {
        match rval {
            Rvalue::Use(var) => vec![*var],
            Rvalue::BinaryOp { left, right, .. } => vec![*left, *right],
            Rvalue::UnaryOp { operand, .. } => vec![*operand],
            Rvalue::FieldAccess { base, .. } => vec![*base],
            Rvalue::Ref { var, .. } => vec![*var],
            Rvalue::Call { args, .. } => args.clone(),
            Rvalue::Constant => vec![],
        }
    }

    /// Extract variables used in an ExprKind.
    fn expr_kind_uses(expr: &ExprKind) -> Vec<VarId> {
        match expr {
            ExprKind::MethodCall { receiver, args, .. } => {
                let mut vars = vec![*receiver];
                vars.extend(args.iter().cloned());
                vars
            }
            ExprKind::Closure { captures, .. } => captures.clone(),
            ExprKind::MacroCall { args, .. } => args.clone(),
            ExprKind::Other => vec![],
        }
    }

    /// Extract variables used in a terminator.
    fn terminator_uses(term: &Terminator) -> Vec<VarId> {
        match term {
            Terminator::Return { value: Some(var) } => vec![*var],
            Terminator::Branch { condition, .. } => vec![*condition],
            _ => vec![],
        }
    }
}
```

#### Phase 4: Building Statement-Level Chains

```rust
impl ReachingDefinitions {
    /// Compute precise def-use chains at statement level.
    fn compute_precise_chains(
        cfg: &ControlFlowGraph,
        reach_in: &HashMap<BlockId, HashSet<VarId>>,
        definitions: &[Definition],
        uses: &[Use],
    ) -> (HashMap<Definition, HashSet<ProgramPoint>>, HashMap<Use, HashSet<Definition>>) {
        let mut def_use = HashMap::new();
        let mut use_def = HashMap::new();

        // Initialize def_use for all definitions
        for def in definitions {
            def_use.insert(*def, HashSet::new());
        }

        // For each use, find which definitions reach it
        for use_point in uses {
            let reaching_defs = Self::find_reaching_defs_at_point(
                cfg,
                reach_in,
                definitions,
                use_point,
            );

            use_def.insert(*use_point, reaching_defs.clone());

            // Update def_use (inverse)
            for def in reaching_defs {
                def_use.entry(def).or_default().insert(use_point.point);
            }
        }

        (def_use, use_def)
    }

    /// Find definitions of a variable that reach a specific use point.
    fn find_reaching_defs_at_point(
        cfg: &ControlFlowGraph,
        reach_in: &HashMap<BlockId, HashSet<VarId>>,
        definitions: &[Definition],
        use_point: &Use,
    ) -> HashSet<Definition> {
        let var = use_point.var;
        let block = use_point.point.block;
        let stmt_idx = use_point.point.stmt;

        // Get the block
        let block_data = cfg.blocks.iter()
            .find(|b| b.id == block)
            .expect("Block must exist");

        // Look for definition of same var in same block before this statement
        let mut found_in_block = None;
        for (idx, stmt) in block_data.statements.iter().enumerate() {
            if idx >= stmt_idx {
                break; // Only look at statements before use
            }

            let defines_var = match stmt {
                Statement::Declare { var: def_var, .. } => def_var.name_id == var.name_id,
                Statement::Assign { target, .. } => target.name_id == var.name_id,
                _ => false,
            };

            if defines_var {
                // This is the most recent definition before our use
                found_in_block = Some(Definition {
                    var: VarId { name_id: var.name_id, version: idx as u32 },
                    point: ProgramPoint::new(block, idx),
                });
            }
        }

        // If found in block, that's the only reaching definition
        if let Some(def) = found_in_block {
            return [def].into_iter().collect();
        }

        // Otherwise, use reach_in for this block
        let reaching = reach_in.get(&block).cloned().unwrap_or_default();

        // Find actual definition points for reaching vars
        definitions.iter()
            .filter(|def| def.var.name_id == var.name_id && reaching.contains(&def.var))
            .cloned()
            .collect()
    }
}
```

#### Phase 5: Enhanced analyze() Method

```rust
impl ReachingDefinitions {
    pub fn analyze(cfg: &ControlFlowGraph) -> Self {
        // --- Block-level analysis (existing) ---
        let mut reach_in: HashMap<BlockId, HashSet<VarId>> = HashMap::new();
        let mut reach_out: HashMap<BlockId, HashSet<VarId>> = HashMap::new();

        // Initialize all blocks
        for block in &cfg.blocks {
            reach_in.insert(block.id, HashSet::new());
            reach_out.insert(block.id, HashSet::new());
        }

        // Fixed-point iteration (existing algorithm)
        let mut changed = true;
        while changed {
            changed = false;

            for block in &cfg.blocks {
                let mut new_reach_in = HashSet::new();
                for pred_id in Self::get_predecessors(cfg, block.id) {
                    if let Some(pred_out) = reach_out.get(&pred_id) {
                        new_reach_in.extend(pred_out.iter().cloned());
                    }
                }

                let (gen, kill) = Self::compute_gen_kill(block);

                let mut new_reach_out = new_reach_in.clone();
                new_reach_out.retain(|v| !kill.contains(&v.name_id));
                new_reach_out.extend(gen);

                if new_reach_in != *reach_in.get(&block.id).unwrap()
                    || new_reach_out != *reach_out.get(&block.id).unwrap()
                {
                    changed = true;
                    reach_in.insert(block.id, new_reach_in);
                    reach_out.insert(block.id, new_reach_out);
                }
            }
        }

        // Block-level def-use (existing)
        let def_use_chains = Self::build_def_use_chains(cfg, &reach_in);

        // --- Statement-level analysis (new) ---
        let (all_definitions, all_uses) = Self::collect_defs_and_uses(cfg);
        let (precise_def_use, use_def_chains) = Self::compute_precise_chains(
            cfg,
            &reach_in,
            &all_definitions,
            &all_uses,
        );

        ReachingDefinitions {
            // Block-level (backward compatible)
            reach_in,
            reach_out,
            def_use_chains,
            // Statement-level (new)
            precise_def_use,
            use_def_chains,
            all_definitions,
            all_uses,
        }
    }
}
```

#### Phase 6: Helper Methods for Consumers

```rust
impl ReachingDefinitions {
    /// Get all uses of a specific definition (statement-level).
    pub fn get_uses_of(&self, def: &Definition) -> Option<&HashSet<ProgramPoint>> {
        self.precise_def_use.get(def)
    }

    /// Get all definitions that reach a specific use (statement-level).
    pub fn get_defs_of(&self, use_point: &Use) -> Option<&HashSet<Definition>> {
        self.use_def_chains.get(use_point)
    }

    /// Check if a definition is dead (no uses).
    pub fn is_dead_definition(&self, def: &Definition) -> bool {
        self.precise_def_use
            .get(def)
            .map(|uses| uses.is_empty())
            .unwrap_or(true)
    }

    /// Find same-block dead stores: defs immediately overwritten before any use.
    pub fn find_same_block_dead_stores(&self) -> Vec<Definition> {
        self.all_definitions.iter()
            .filter(|def| {
                let uses = self.precise_def_use.get(def);
                // Dead if no uses at all
                if uses.map(|u| u.is_empty()).unwrap_or(true) {
                    return true;
                }
                // Not dead if there are uses
                false
            })
            .cloned()
            .collect()
    }

    /// Get the single reaching definition for a use (if unique).
    pub fn get_unique_def(&self, use_point: &Use) -> Option<Definition> {
        self.use_def_chains.get(use_point).and_then(|defs| {
            if defs.len() == 1 {
                defs.iter().next().cloned()
            } else {
                None
            }
        })
    }
}
```

### Architecture Changes

1. **New types**: `StatementIdx`, `ProgramPoint`, `Definition`, `Use`
2. **Extended struct**: `ReachingDefinitions` gains 4 new fields
3. **New methods**: Statement-level chain building and queries

### Data Structures

```rust
pub type StatementIdx = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProgramPoint {
    pub block: BlockId,
    pub stmt: StatementIdx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Definition {
    pub var: VarId,
    pub point: ProgramPoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Use {
    pub var: VarId,
    pub point: ProgramPoint,
}
```

### APIs and Interfaces

Backward compatible. New methods added:
- `get_uses_of(&Definition) -> Option<&HashSet<ProgramPoint>>`
- `get_defs_of(&Use) -> Option<&HashSet<Definition>>`
- `is_dead_definition(&Definition) -> bool`
- `find_same_block_dead_stores() -> Vec<Definition>`
- `get_unique_def(&Use) -> Option<Definition>`

## Dependencies

- **Prerequisites**: Spec 248 (Enhanced Expression Variable Extraction)
- **Affected Components**:
  - `src/analysis/data_flow.rs` - Main implementation
  - Tests for data_flow.rs
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod statement_level_tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_simple_def_use_chain() {
        let block: Block = parse_quote!({
            let x = 1;
            let y = x + 2;
        });

        let cfg = ControlFlowGraph::from_block(&block);
        let reaching = ReachingDefinitions::analyze(&cfg);

        // x defined at stmt 0
        // x used at stmt 1
        assert!(!reaching.all_definitions.is_empty());
        assert!(!reaching.all_uses.is_empty());

        // Find definition of x
        let x_def = reaching.all_definitions.iter()
            .find(|d| cfg.var_names[d.var.name_id as usize] == "x");

        assert!(x_def.is_some());
        let x_def = x_def.unwrap();

        // Should have uses
        let uses = reaching.get_uses_of(x_def);
        assert!(uses.is_some());
        assert!(!uses.unwrap().is_empty());
    }

    #[test]
    fn test_same_block_dead_store() {
        let block: Block = parse_quote!({
            let mut x = 1;  // stmt 0: dead store
            x = 2;          // stmt 1: kills stmt 0
            let y = x;      // stmt 2: uses x from stmt 1
        });

        let cfg = ControlFlowGraph::from_block(&block);
        let reaching = ReachingDefinitions::analyze(&cfg);

        let dead_stores = reaching.find_same_block_dead_stores();

        // The first assignment (x = 1) should be dead
        // because x = 2 overwrites it before any use
        assert!(!dead_stores.is_empty());
    }

    #[test]
    fn test_multiple_reaching_defs() {
        let block: Block = parse_quote!({
            let x;
            if condition {
                x = 1;
            } else {
                x = 2;
            }
            let y = x; // Both x=1 and x=2 reach here
        });

        let cfg = ControlFlowGraph::from_block(&block);
        let reaching = ReachingDefinitions::analyze(&cfg);

        // Find use of x in "let y = x"
        let x_uses: Vec<_> = reaching.all_uses.iter()
            .filter(|u| cfg.var_names[u.var.name_id as usize] == "x")
            .collect();

        // Should have at least one use
        assert!(!x_uses.is_empty());

        // That use should have multiple reaching defs (from if/else)
        // Note: This depends on proper CFG construction for if/else
    }

    #[test]
    fn test_use_def_inverse() {
        let block: Block = parse_quote!({
            let a = 1;
            let b = a;
            let c = a + b;
        });

        let cfg = ControlFlowGraph::from_block(&block);
        let reaching = ReachingDefinitions::analyze(&cfg);

        // For each use, should be able to find its definition
        for use_point in &reaching.all_uses {
            let defs = reaching.get_defs_of(use_point);
            assert!(defs.is_some(), "Use {:?} should have reaching defs", use_point);
            assert!(!defs.unwrap().is_empty());
        }
    }

    #[test]
    fn test_dead_definition_detection() {
        let block: Block = parse_quote!({
            let x = expensive_computation();  // Never used
            let y = 42;
        });

        let cfg = ControlFlowGraph::from_block(&block);
        let reaching = ReachingDefinitions::analyze(&cfg);

        // x should be a dead definition
        let x_def = reaching.all_definitions.iter()
            .find(|d| cfg.var_names[d.var.name_id as usize] == "x");

        if let Some(def) = x_def {
            assert!(reaching.is_dead_definition(def));
        }
    }

    #[test]
    fn test_unique_def_retrieval() {
        let block: Block = parse_quote!({
            let x = 1;
            let y = x;  // x has unique reaching def
        });

        let cfg = ControlFlowGraph::from_block(&block);
        let reaching = ReachingDefinitions::analyze(&cfg);

        // Find use of x
        let x_use = reaching.all_uses.iter()
            .find(|u| cfg.var_names[u.var.name_id as usize] == "x");

        if let Some(use_point) = x_use {
            let unique_def = reaching.get_unique_def(use_point);
            assert!(unique_def.is_some(), "Should find unique definition");
        }
    }

    #[test]
    fn test_chained_assignments() {
        let block: Block = parse_quote!({
            let mut x = 1;   // def 0
            x = x + 1;       // def 1, uses def 0
            x = x * 2;       // def 2, uses def 1
            let y = x;       // uses def 2
        });

        let cfg = ControlFlowGraph::from_block(&block);
        let reaching = ReachingDefinitions::analyze(&cfg);

        // Should have 3 definitions of x (including initial)
        let x_defs: Vec<_> = reaching.all_definitions.iter()
            .filter(|d| cfg.var_names[d.var.name_id as usize] == "x")
            .collect();

        assert!(x_defs.len() >= 1, "Should have at least 1 x definition");
    }
}
```

### Integration Tests

```rust
#[test]
fn test_liveness_with_statement_precision() {
    let block: Block = parse_quote!({
        let mut x = 1;
        let mut y = 2;
        x = 10;  // Previous x is dead
        y = x;   // y = 2 is dead
        println!("{} {}", x, y);
    });

    let cfg = ControlFlowGraph::from_block(&block);
    let reaching = ReachingDefinitions::analyze(&cfg);
    let liveness = LivenessInfo::analyze(&cfg);

    // Both dead stores should be detected
    let dead = reaching.find_same_block_dead_stores();
    assert!(!dead.is_empty(), "Should detect dead stores");
}
```

### Performance Tests

```rust
#[test]
fn test_statement_level_performance() {
    use std::time::Instant;

    // Function with many statements
    let block: Block = parse_quote!({
        let a = 1;
        let b = 2;
        let c = a + b;
        let d = b + c;
        let e = c + d;
        let f = d + e;
        let g = e + f;
        let h = f + g;
        let i = g + h;
        let j = h + i;
        j
    });

    let start = Instant::now();
    for _ in 0..100 {
        let cfg = ControlFlowGraph::from_block(&block);
        let _ = ReachingDefinitions::analyze(&cfg);
    }
    let elapsed = start.elapsed();

    // 100 iterations should complete in <100ms
    assert!(elapsed.as_millis() < 100, "Performance regression: {:?}", elapsed);
}
```

## Documentation Requirements

- **Code Documentation**: Add rustdoc to all new types and methods
- **User Documentation**: No changes (internal implementation)
- **Architecture Updates**: Document statement-level precision in module docs

## Implementation Notes

### Algorithm Complexity

- **Time**: O(blocks × statements × definitions) for chain building
- **Space**: O(definitions × uses) for storing chains
- For typical functions (<100 statements), this is negligible

### Memory Optimization

If memory becomes a concern:
1. Use `SmallVec` for small use sets
2. Store chains lazily (compute on demand)
3. Use interned strings for variable names

### Edge Cases

1. **Empty blocks**: Handle gracefully (no statements)
2. **Loops**: Fixed-point correctly handles backedges
3. **Phi nodes**: Not explicit, but multiple defs reaching same use
4. **Terminators**: Included in use collection

## Migration and Compatibility

- **No migration needed**: Additive change only
- **Backward compatible**: Existing fields unchanged
- **New features opt-in**: Use new methods only when needed
