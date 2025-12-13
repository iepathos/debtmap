---
number: 264
title: Data Flow Module Decomposition
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-13
---

# Specification 264: Data Flow Module Decomposition

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

`src/analysis/data_flow.rs` is the largest single file in the codebase at **3,766 lines**. This violates the project guideline of keeping files under 200 lines and modules under 800 lines.

The file contains four distinct algorithms that are conceptually separate:

1. **Call Classification Database** (~770 lines) - Static database of pure/impure functions
2. **CFG Types** (~236 lines) - Control flow graph data structures
3. **Reaching Definitions** (~700 lines) - Data flow analysis algorithm
4. **CFG Builder** (~1,906 lines) - AST-to-CFG transformation

**Current Structure:**
```
src/analysis/data_flow.rs  (3,766 lines - MONOLITH)
```

**Problems:**
- Hard to navigate - finding related code requires scrolling thousands of lines
- Hard to test - can't test algorithms in isolation
- Hard to understand - no clear boundaries between responsibilities
- Violates single responsibility principle
- Contributes to cognitive overload

**Stillwater Philosophy:**
> "Composition Over Complexity" - Build complex behavior from simple, composable pieces. Each does one thing, is easily testable, is reusable.

## Objective

Decompose `data_flow.rs` into focused modules:

1. **Extract** call classification database to standalone module
2. **Extract** CFG types to shared types module
3. **Extract** reaching definitions algorithm
4. **Extract** CFG builder implementation
5. **Create** clean public API in `mod.rs`
6. **Preserve** all existing functionality and tests

Result: Four focused modules (~700-1,000 lines each) with clear responsibilities and testable boundaries.

## Requirements

### Functional Requirements

1. **Module Structure**
   ```
   src/analysis/data_flow/
   ├── mod.rs                    (~50 lines)  - Public API exports
   ├── call_classification.rs    (~770 lines) - Pure/impure call database
   ├── types.rs                  (~236 lines) - CFG, BlockId, BasicBlock
   ├── reaching_definitions.rs   (~700 lines) - Data flow algorithm
   └── cfg_builder.rs            (~1,906 lines) - AST-to-CFG transformation
   ```

2. **Call Classification Module**
   - `CallPurity` enum and variants
   - `UnknownCallBehavior` configuration
   - Static databases: `KNOWN_PURE_FUNCTIONS`, `KNOWN_IMPURE_FUNCTIONS`
   - Pattern sets: `PURE_METHOD_PATTERNS`, `IMPURE_METHOD_PATTERNS`
   - Classification functions: `classify_call()`, `is_known_pure()`, `is_known_impure()`

3. **Types Module**
   - `ControlFlowGraph`, `BlockId`, `BasicBlock`
   - Edge and Statement types
   - `VarId`, `CaptureMode`, `CapturedVar`
   - Visitor patterns for closure capture

4. **Reaching Definitions Module**
   - `ProgramPoint`, `Definition`, `Use` types
   - `Rvalue`, `BinOp`, `UnOp`, `ExprKind`
   - `DataFlowAnalysis` struct
   - `ReachingDefinitions` algorithm

5. **CFG Builder Module**
   - `CfgBuilder` implementation
   - Statement/expression processing
   - Closure handling
   - Rvalue extraction

### Non-Functional Requirements

1. **No Breaking Changes**
   - Public API remains identical
   - All imports via `data_flow::` continue working
   - Re-exports preserve compatibility

2. **Clear Dependencies**
   - `call_classification` → standalone
   - `types` → standalone
   - `reaching_definitions` → imports `types`
   - `cfg_builder` → imports `types`, `call_classification`

3. **Testability**
   - Each module testable in isolation
   - Existing tests pass unchanged
   - New module-specific tests possible

## Acceptance Criteria

- [ ] `data_flow.rs` replaced by `data_flow/` directory
- [ ] `call_classification.rs` < 800 lines
- [ ] `types.rs` < 300 lines
- [ ] `reaching_definitions.rs` < 800 lines
- [ ] `cfg_builder.rs` < 2,000 lines (still large but focused)
- [ ] All existing `use data_flow::` imports work unchanged
- [ ] All existing tests pass
- [ ] No clippy warnings
- [ ] Documentation updated

## Technical Details

### Implementation Approach

**Phase 1: Create Module Structure**

```bash
mkdir -p src/analysis/data_flow
```

**Phase 2: Extract call_classification.rs**

Extract lines ~154-923 to `src/analysis/data_flow/call_classification.rs`:

```rust
// src/analysis/data_flow/call_classification.rs

//! Call purity classification database.
//!
//! This module maintains static databases of known pure and impure functions
//! for common Rust standard library and ecosystem crates. Used to determine
//! whether function calls can propagate taint or affect purity analysis.

use std::collections::HashSet;
use once_cell::sync::Lazy;

/// Classification of a function call's purity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallPurity {
    /// Known to be pure - no side effects, deterministic
    Pure,
    /// Known to have side effects
    Impure,
    /// Purity unknown - treat conservatively
    Unknown,
}

/// How to handle calls to unknown functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnknownCallBehavior {
    /// Assume unknown calls are impure (conservative)
    AssumeImpure,
    /// Assume unknown calls are pure (optimistic)
    AssumePure,
}

impl Default for UnknownCallBehavior {
    fn default() -> Self {
        Self::AssumeImpure
    }
}

/// Database of known pure functions.
pub static KNOWN_PURE_FUNCTIONS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        // Iterator methods
        "iter", "into_iter", "map", "filter", "fold", "collect",
        "any", "all", "find", "position", "count", "sum", "product",
        // ... rest of database
    ].into_iter().collect()
});

/// Database of known impure functions.
pub static KNOWN_IMPURE_FUNCTIONS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        // I/O operations
        "println", "print", "eprintln", "eprint",
        "read", "write", "read_to_string", "write_all",
        // ... rest of database
    ].into_iter().collect()
});

// ... pattern sets and classification functions

/// Classify a function call's purity.
pub fn classify_call(function_name: &str, behavior: UnknownCallBehavior) -> CallPurity {
    if is_known_pure(function_name) {
        CallPurity::Pure
    } else if is_known_impure(function_name) {
        CallPurity::Impure
    } else {
        match behavior {
            UnknownCallBehavior::AssumeImpure => CallPurity::Impure,
            UnknownCallBehavior::AssumePure => CallPurity::Pure,
        }
    }
}

/// Check if function is in the known pure database.
pub fn is_known_pure(function_name: &str) -> bool {
    KNOWN_PURE_FUNCTIONS.contains(function_name)
        || PURE_METHOD_PATTERNS.iter().any(|p| function_name.contains(p))
}

/// Check if function is in the known impure database.
pub fn is_known_impure(function_name: &str) -> bool {
    KNOWN_IMPURE_FUNCTIONS.contains(function_name)
        || IMPURE_METHOD_PATTERNS.iter().any(|p| function_name.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_known_pure() {
        assert_eq!(
            classify_call("iter", UnknownCallBehavior::AssumeImpure),
            CallPurity::Pure
        );
    }

    #[test]
    fn test_classify_known_impure() {
        assert_eq!(
            classify_call("println", UnknownCallBehavior::AssumePure),
            CallPurity::Impure
        );
    }

    #[test]
    fn test_classify_unknown_conservative() {
        assert_eq!(
            classify_call("my_custom_function", UnknownCallBehavior::AssumeImpure),
            CallPurity::Impure
        );
    }
}
```

**Phase 3: Extract types.rs**

Extract lines ~924-1159 to `src/analysis/data_flow/types.rs`:

```rust
// src/analysis/data_flow/types.rs

//! Control flow graph types and data structures.
//!
//! This module defines the core types used for CFG construction and
//! data flow analysis, including basic blocks, edges, and variable tracking.

use std::collections::{HashMap, HashSet};

/// Unique identifier for a basic block in the CFG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);

/// A basic block in the control flow graph.
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockId,
    pub statements: Vec<Statement>,
    pub terminator: Option<Terminator>,
}

/// A statement within a basic block.
#[derive(Debug, Clone)]
pub enum Statement {
    Assign { target: VarId, value: Rvalue },
    Call { target: Option<VarId>, function: String, args: Vec<VarId> },
    // ... other variants
}

/// Block terminator - control flow edge.
#[derive(Debug, Clone)]
pub enum Terminator {
    Goto(BlockId),
    If { condition: VarId, then_block: BlockId, else_block: BlockId },
    Return(Option<VarId>),
    // ... other variants
}

/// The complete control flow graph.
#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    pub blocks: HashMap<BlockId, BasicBlock>,
    pub entry: BlockId,
    pub exit: BlockId,
}

impl ControlFlowGraph {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            entry: BlockId(0),
            exit: BlockId(0),
        }
    }

    pub fn add_block(&mut self, block: BasicBlock) -> BlockId {
        let id = block.id;
        self.blocks.insert(id, block);
        id
    }

    // ... other methods
}

/// Variable identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VarId(pub String);

/// How a variable is captured by a closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    ByValue,
    ByRef,
    ByMutRef,
}

/// A variable captured by a closure.
#[derive(Debug, Clone)]
pub struct CapturedVar {
    pub var: VarId,
    pub mode: CaptureMode,
}

// ... Rvalue, BinOp, UnOp, ExprKind moved from reaching_definitions
// if they're shared types

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cfg_creation() {
        let mut cfg = ControlFlowGraph::new();
        let block = BasicBlock {
            id: BlockId(0),
            statements: vec![],
            terminator: None,
        };
        cfg.add_block(block);
        assert!(cfg.blocks.contains_key(&BlockId(0)));
    }
}
```

**Phase 4: Extract reaching_definitions.rs**

Extract lines ~1160-1859 to `src/analysis/data_flow/reaching_definitions.rs`:

```rust
// src/analysis/data_flow/reaching_definitions.rs

//! Reaching definitions data flow analysis.
//!
//! Implements the reaching definitions algorithm for determining
//! which variable definitions can reach each program point.

use super::types::{BlockId, ControlFlowGraph, VarId, Statement};
use std::collections::{HashMap, HashSet};

/// A point in the program (block, statement index).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProgramPoint {
    pub block: BlockId,
    pub statement: usize,
}

/// A variable definition at a program point.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Definition {
    pub var: VarId,
    pub point: ProgramPoint,
}

/// A use of a variable at a program point.
#[derive(Debug, Clone)]
pub struct Use {
    pub var: VarId,
    pub point: ProgramPoint,
    pub reaching_defs: HashSet<Definition>,
}

/// Data flow analysis state.
pub struct DataFlowAnalysis {
    cfg: ControlFlowGraph,
    /// Definitions reaching each block entry
    in_sets: HashMap<BlockId, HashSet<Definition>>,
    /// Definitions reaching each block exit
    out_sets: HashMap<BlockId, HashSet<Definition>>,
}

impl DataFlowAnalysis {
    pub fn new(cfg: ControlFlowGraph) -> Self {
        Self {
            cfg,
            in_sets: HashMap::new(),
            out_sets: HashMap::new(),
        }
    }

    /// Run reaching definitions analysis.
    pub fn compute_reaching_definitions(&mut self) {
        // Initialize
        for block_id in self.cfg.blocks.keys() {
            self.in_sets.insert(*block_id, HashSet::new());
            self.out_sets.insert(*block_id, HashSet::new());
        }

        // Iterate until fixpoint
        let mut changed = true;
        while changed {
            changed = false;
            for (block_id, block) in &self.cfg.blocks {
                // IN[B] = union of OUT[P] for all predecessors P
                let in_set = self.compute_in_set(*block_id);

                // OUT[B] = GEN[B] union (IN[B] - KILL[B])
                let out_set = self.compute_out_set(&in_set, block);

                if out_set != *self.out_sets.get(block_id).unwrap() {
                    changed = true;
                    self.out_sets.insert(*block_id, out_set);
                }
                self.in_sets.insert(*block_id, in_set);
            }
        }
    }

    fn compute_in_set(&self, block_id: BlockId) -> HashSet<Definition> {
        // ... implementation
        HashSet::new()
    }

    fn compute_out_set(&self, in_set: &HashSet<Definition>, block: &super::types::BasicBlock) -> HashSet<Definition> {
        // ... implementation
        in_set.clone()
    }

    /// Get reaching definitions for a variable at a program point.
    pub fn get_reaching_definitions(&self, var: &VarId, point: &ProgramPoint) -> HashSet<Definition> {
        // ... implementation
        HashSet::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reaching_definitions_simple() {
        // Test basic reaching definitions
    }
}
```

**Phase 5: Extract cfg_builder.rs**

Extract lines ~1860-3766 to `src/analysis/data_flow/cfg_builder.rs`:

```rust
// src/analysis/data_flow/cfg_builder.rs

//! Control flow graph construction from Rust AST.
//!
//! This module transforms syn AST into a control flow graph suitable
//! for data flow analysis. Handles all Rust statement and expression types.

use super::types::{BasicBlock, BlockId, ControlFlowGraph, Statement, Terminator, VarId};
use super::call_classification::{classify_call, CallPurity, UnknownCallBehavior};
use syn::{Expr, Stmt};

/// Builder for constructing control flow graphs from AST.
pub struct CfgBuilder {
    cfg: ControlFlowGraph,
    current_block: BlockId,
    next_block_id: usize,
    /// Variables defined in current scope
    scope_vars: Vec<VarId>,
}

impl CfgBuilder {
    pub fn new() -> Self {
        let mut cfg = ControlFlowGraph::new();
        let entry = BlockId(0);
        cfg.add_block(BasicBlock {
            id: entry,
            statements: vec![],
            terminator: None,
        });
        cfg.entry = entry;

        Self {
            cfg,
            current_block: entry,
            next_block_id: 1,
            scope_vars: vec![],
        }
    }

    /// Build CFG from function body.
    pub fn build_from_function(body: &syn::Block) -> ControlFlowGraph {
        let mut builder = Self::new();
        builder.process_block(body);
        builder.finish()
    }

    fn finish(mut self) -> ControlFlowGraph {
        self.cfg.exit = self.current_block;
        self.cfg
    }

    fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.next_block_id);
        self.next_block_id += 1;
        self.cfg.add_block(BasicBlock {
            id,
            statements: vec![],
            terminator: None,
        });
        id
    }

    fn process_block(&mut self, block: &syn::Block) {
        for stmt in &block.stmts {
            self.process_stmt(stmt);
        }
    }

    fn process_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Local(local) => self.process_local(local),
            Stmt::Expr(expr, _) => { self.process_expr(expr); }
            Stmt::Item(_) => { /* Skip nested items */ }
            Stmt::Macro(_) => { /* Skip macros */ }
        }
    }

    fn process_local(&mut self, local: &syn::Local) {
        // ... implementation
    }

    fn process_expr(&mut self, expr: &Expr) -> Option<VarId> {
        match expr {
            Expr::If(if_expr) => self.process_if(if_expr),
            Expr::Match(match_expr) => self.process_match(match_expr),
            Expr::Loop(loop_expr) => self.process_loop(loop_expr),
            Expr::While(while_expr) => self.process_while(while_expr),
            Expr::For(for_expr) => self.process_for(for_expr),
            Expr::Block(block_expr) => {
                self.process_block(&block_expr.block);
                None
            }
            Expr::Call(call_expr) => self.process_call(call_expr),
            Expr::MethodCall(method_call) => self.process_method_call(method_call),
            Expr::Closure(closure) => self.process_closure(closure),
            // ... other expression types
            _ => None,
        }
    }

    fn process_if(&mut self, if_expr: &syn::ExprIf) -> Option<VarId> {
        // Process condition
        let cond_var = self.process_expr(&if_expr.cond);

        // Create then and else blocks
        let then_block = self.new_block();
        let else_block = self.new_block();
        let join_block = self.new_block();

        // Set terminator
        if let Some(current) = self.cfg.blocks.get_mut(&self.current_block) {
            current.terminator = Some(Terminator::If {
                condition: cond_var.unwrap_or(VarId("_cond".to_string())),
                then_block,
                else_block,
            });
        }

        // Process then branch
        self.current_block = then_block;
        self.process_block(&if_expr.then_branch);
        if let Some(current) = self.cfg.blocks.get_mut(&self.current_block) {
            current.terminator = Some(Terminator::Goto(join_block));
        }

        // Process else branch
        self.current_block = else_block;
        if let Some((_, else_branch)) = &if_expr.else_branch {
            self.process_expr(else_branch);
        }
        if let Some(current) = self.cfg.blocks.get_mut(&self.current_block) {
            current.terminator = Some(Terminator::Goto(join_block));
        }

        self.current_block = join_block;
        None
    }

    // ... many more expression processing methods

    fn process_match(&mut self, match_expr: &syn::ExprMatch) -> Option<VarId> {
        // ... implementation
        None
    }

    fn process_loop(&mut self, loop_expr: &syn::ExprLoop) -> Option<VarId> {
        // ... implementation
        None
    }

    fn process_while(&mut self, while_expr: &syn::ExprWhile) -> Option<VarId> {
        // ... implementation
        None
    }

    fn process_for(&mut self, for_expr: &syn::ExprForLoop) -> Option<VarId> {
        // ... implementation
        None
    }

    fn process_call(&mut self, call_expr: &syn::ExprCall) -> Option<VarId> {
        // ... implementation
        None
    }

    fn process_method_call(&mut self, method_call: &syn::ExprMethodCall) -> Option<VarId> {
        // ... implementation
        None
    }

    fn process_closure(&mut self, closure: &syn::ExprClosure) -> Option<VarId> {
        // ... implementation
        None
    }
}

impl Default for CfgBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_simple_cfg() {
        let code = "{ let x = 1; x + 1 }";
        let block: syn::Block = syn::parse_str(code).unwrap();
        let cfg = CfgBuilder::build_from_function(&block);
        assert!(cfg.blocks.len() >= 1);
    }

    #[test]
    fn test_build_if_cfg() {
        let code = "{ if true { 1 } else { 2 } }";
        let block: syn::Block = syn::parse_str(code).unwrap();
        let cfg = CfgBuilder::build_from_function(&block);
        // Should have entry, then, else, and join blocks
        assert!(cfg.blocks.len() >= 4);
    }
}
```

**Phase 6: Create mod.rs**

```rust
// src/analysis/data_flow/mod.rs

//! Data flow analysis for Rust code.
//!
//! This module provides control flow graph construction and data flow
//! analysis algorithms for analyzing variable definitions, uses, and
//! function purity.
//!
//! # Module Structure
//!
//! - [`call_classification`] - Database of known pure/impure functions
//! - [`types`] - Core CFG types (blocks, edges, variables)
//! - [`reaching_definitions`] - Data flow analysis algorithm
//! - [`cfg_builder`] - AST-to-CFG transformation
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::analysis::data_flow::{CfgBuilder, DataFlowAnalysis};
//!
//! let cfg = CfgBuilder::build_from_function(&function_body);
//! let mut analysis = DataFlowAnalysis::new(cfg);
//! analysis.compute_reaching_definitions();
//! ```

mod call_classification;
mod types;
mod reaching_definitions;
mod cfg_builder;

// Re-export public API
pub use call_classification::{
    CallPurity, UnknownCallBehavior,
    classify_call, is_known_pure, is_known_impure,
    KNOWN_PURE_FUNCTIONS, KNOWN_IMPURE_FUNCTIONS,
};

pub use types::{
    BlockId, BasicBlock, Statement, Terminator,
    ControlFlowGraph, VarId, CaptureMode, CapturedVar,
};

pub use reaching_definitions::{
    ProgramPoint, Definition, Use, DataFlowAnalysis,
};

pub use cfg_builder::CfgBuilder;
```

### Migration Steps

1. **Create directory**: `mkdir -p src/analysis/data_flow`
2. **Extract modules** in order:
   - `call_classification.rs` (no deps)
   - `types.rs` (no deps)
   - `reaching_definitions.rs` (uses types)
   - `cfg_builder.rs` (uses types, call_classification)
3. **Create mod.rs** with re-exports
4. **Delete** old `data_flow.rs`
5. **Update imports** across codebase
6. **Run tests** to verify

### Dependency Graph

```
mod.rs
  ├── call_classification.rs  (standalone)
  ├── types.rs                (standalone)
  ├── reaching_definitions.rs
  │     └── uses: types
  └── cfg_builder.rs
        ├── uses: types
        └── uses: call_classification
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/analysis/data_flow.rs` → `src/analysis/data_flow/`
  - Any file importing from `data_flow`
- **External Dependencies**: None

## Testing Strategy

### Existing Tests

All existing tests in `data_flow.rs` should pass after migration:

```bash
cargo test data_flow
```

### New Module Tests

Each new module should have focused tests:

```rust
// call_classification tests
#[test]
fn test_pure_function_classification() { ... }

// types tests
#[test]
fn test_cfg_structure() { ... }

// reaching_definitions tests
#[test]
fn test_fixpoint_convergence() { ... }

// cfg_builder tests
#[test]
fn test_if_expression_cfg() { ... }
```

## Documentation Requirements

### Module Documentation

Each module has top-level documentation explaining:
- Purpose and responsibility
- Key types and functions
- Usage examples

### Architecture Updates

Update `ARCHITECTURE.md` to reflect new module structure.

## Implementation Notes

### Import Updates

After migration, update imports across codebase:

```rust
// Before
use crate::analysis::data_flow::CfgBuilder;

// After (unchanged - re-exports preserve compatibility)
use crate::analysis::data_flow::CfgBuilder;
```

### Line Count Verification

After migration, verify line counts:

```bash
wc -l src/analysis/data_flow/*.rs
# Expected:
#   ~50 mod.rs
#  ~770 call_classification.rs
#  ~236 types.rs
#  ~700 reaching_definitions.rs
# ~1906 cfg_builder.rs
```

## Migration and Compatibility

### Breaking Changes

None - public API preserved through re-exports.

### Backward Compatibility

All existing `use data_flow::` imports continue working.

## Success Metrics

- `data_flow.rs` replaced by `data_flow/` directory
- No file exceeds 2,000 lines
- All tests pass
- Public API unchanged
- Clear module responsibilities
