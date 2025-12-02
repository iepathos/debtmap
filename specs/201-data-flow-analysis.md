---
number: 201
title: Data Flow Analysis for State Transition and Mutation Tracking
category: foundation
priority: high
status: draft
dependencies: [159, 162]
created: 2025-12-02
---

# Specification 201: Data Flow Analysis for State Transition and Mutation Tracking

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Specs 159 (Evidence-Based Purity Confidence), 162 (Almost Pure Function Detection)

## Context

**Current Limitation**: Debtmap's state transition and mutation detection is **syntactic-first**, identifying mutations and state patterns based on code structure without understanding data flow. This leads to:

1. **False Positives**: Dead mutations flagged as violations
   ```rust
   fn calculate(x: i32) -> i32 {
       let mut temp = x;  // ← Flagged as mutation
       temp = temp * 2;   // ← Flagged as mutation
       // temp never used again
       x + 1  // Returns unrelated value
   }
   ```

2. **Missed Context**: Cannot determine if mutations affect function output
   ```rust
   fn process(items: &[Item]) -> f64 {
       let mut cache = HashMap::new();  // ← Local cache
       cache.insert(key, value);        // ← Flagged as impure
       // Cache never escapes function
       items.iter().map(|i| i.price).sum()  // ← Returns pure computation
   }
   ```

3. **Incomplete State Tracking**: Missing cross-function state flow
   ```rust
   fn update_state(state: &mut State) {  // ← External mutation detected
       transition_to_next(state);        // ← State transition missed (hidden in call)
   }
   ```

4. **No Liveness Analysis**: Cannot distinguish live vs dead mutations
   ```rust
   fn compute(x: i32) -> i32 {
       let mut result = 0;
       result = x * 2;      // ← Live mutation (used in return)
       let mut temp = x;
       temp = temp + 1;     // ← Dead mutation (never used)
       result
   }
   ```

**Impact on Existing Features**:
- **Spec 162 (Almost Pure)**: False positives reduce refactoring suggestion accuracy
- **Spec 159 (Confidence)**: Cannot adjust confidence based on mutation liveness
- **State Machine Detection**: Misses implicit state transitions through function calls
- **Purity Detection**: Over-conservative classification due to harmless local mutations

## Objective

Implement **intra-procedural data flow analysis** to track variable definitions, uses, and liveness, enabling:

1. **Liveness-aware mutation tracking** - Distinguish live from dead mutations
2. **Def-use chains** - Connect variable definitions to their uses
3. **Escape analysis** - Determine if local mutations affect function output
4. **Taint tracking** - Trace how mutations propagate through data flow
5. **Refined purity classification** - Reduce false positives via flow analysis

## Requirements

### Functional Requirements

1. **Def-Use Chain Construction**
   - Build reaching definitions for all variables
   - Track all use sites for each definition
   - Handle SSA-like analysis for reassignments
   - Support control flow merges (if/else, loops)

2. **Liveness Analysis**
   - Compute live variables at each program point
   - Identify dead code and unused mutations
   - Support backward data flow for liveness
   - Handle complex control flow (loops, early returns)

3. **Escape Analysis**
   - Determine if local variables escape function scope
   - Track whether mutations affect return value
   - Identify captured variables in closures
   - Detect references passed to external functions

4. **Taint Tracking**
   - Mark variables involved in mutations
   - Propagate taint through assignments
   - Track taint flow to function outputs
   - Distinguish local taint from external taint

5. **Integration with Existing Systems**
   - Enhance `PurityDetector` with flow-sensitive analysis
   - Refine `AlmostPureAnalyzer` to filter dead mutations
   - Improve state machine detection with flow context
   - Provide confidence adjustments based on liveness

### Non-Functional Requirements

1. **Performance**: Analysis overhead < 20% of current analysis time
2. **Memory**: Additional memory usage < 50MB for 100k LOC codebase
3. **Accuracy**: Reduce false positive rate by ≥30% for purity classification
4. **Maintainability**: Modular design allowing incremental enhancements
5. **Compatibility**: Backward compatible with existing analysis pipeline

## Acceptance Criteria

- [ ] **Liveness Analysis Implementation**
  - Correctly identifies dead mutations (variables written but never read)
  - Computes live-in/live-out sets for basic blocks
  - Handles control flow merges with phi-node semantics
  - Performance: < 10ms per function on average

- [ ] **Def-Use Chain Construction**
  - Builds complete def-use chains for all variables
  - Handles variable shadowing and nested scopes
  - Tracks reassignments with SSA-like numbering
  - Integration: Accessible from `PurityDetector` and other analyzers

- [ ] **Escape Analysis**
  - Detects whether local mutations escape function scope
  - Identifies captured variables in closures
  - Tracks reference passing to external functions
  - Accuracy: < 5% false negatives for escape detection

- [ ] **Taint Tracking**
  - Marks mutated variables and propagates taint
  - Determines if taint reaches function return value
  - Distinguishes local vs external taint sources
  - Supports path-sensitive tracking for conditionals

- [ ] **Integration with Purity Detection**
  - `PurityDetector` uses liveness to filter dead mutations
  - `LocallyPure` classification refined with escape analysis
  - Confidence scores adjusted based on mutation liveness
  - False positive reduction: ≥30% improvement

- [ ] **Integration with Almost Pure Detection**
  - Spec 162 suggestions exclude dead mutations
  - Refactoring effort recalculated based on live violations
  - Only live state modifications trigger refactoring hints
  - Accuracy improvement: ≥20% reduction in spurious suggestions

- [ ] **State Machine Detection Enhancement**
  - Tracks state variable data flow across statements
  - Detects implicit state transitions via function calls
  - Builds state transition graph from def-use chains
  - Confidence improvement: ≥15% for state machine patterns

- [ ] **Test Coverage**
  - Unit tests for liveness, def-use, escape, taint modules
  - Integration tests with `PurityDetector` and `AlmostPureAnalyzer`
  - Regression tests for false positive reduction
  - Performance benchmarks demonstrating < 20% overhead

- [ ] **Documentation**
  - Architecture documentation explaining data flow algorithms
  - API documentation for `DataFlowAnalyzer` module
  - Examples showing false positive elimination
  - Performance characteristics and trade-offs documented

## Technical Details

### Architecture Overview

```
┌────────────────────────────────────────────────────────────┐
│                  Data Flow Analysis Pipeline                │
└────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────┐
│  1. Control Flow Graph (CFG) Construction                   │
│     - Build basic blocks from AST                           │
│     - Connect blocks with edges (sequential, branch, loop)  │
│     - Identify entry and exit blocks                        │
└────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────┐
│  2. Reaching Definitions (Def-Use Chains)                   │
│     - Forward data flow: compute reaching definitions       │
│     - Gen/Kill sets for each basic block                    │
│     - Iterative fixed-point algorithm                       │
└────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────┐
│  3. Liveness Analysis                                       │
│     - Backward data flow: compute live variables            │
│     - Use/Def sets for each basic block                     │
│     - Iterative fixed-point algorithm                       │
└────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────┐
│  4. Escape Analysis                                         │
│     - Track pointer/reference escape paths                  │
│     - Identify closure captures                             │
│     - Determine function output dependencies                │
└────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────┐
│  5. Taint Propagation                                       │
│     - Mark mutation sites as taint sources                  │
│     - Propagate taint through def-use chains                │
│     - Check if taint reaches return value                   │
└────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────┐
│  6. Integration with Existing Analyzers                     │
│     - PurityDetector: Filter dead mutations                 │
│     - AlmostPureAnalyzer: Refine refactoring suggestions    │
│     - StateMachineDetector: Enhance pattern confidence      │
└────────────────────────────────────────────────────────────┘
```

### Core Data Structures

```rust
use std::collections::{HashMap, HashSet};
use syn::{Block, Expr, Stmt, Ident};

/// Control Flow Graph for intra-procedural analysis
#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    pub blocks: Vec<BasicBlock>,
    pub entry_block: BlockId,
    pub exit_blocks: Vec<BlockId>,
    pub edges: HashMap<BlockId, Vec<Edge>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(usize);

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockId,
    pub statements: Vec<Statement>,
    pub terminator: Terminator,
}

#[derive(Debug, Clone)]
pub enum Statement {
    /// Variable assignment: x = expr
    Assign {
        target: VarId,
        source: Rvalue,
        line: Option<usize>,
    },
    /// Variable declaration: let x = expr
    Declare {
        var: VarId,
        init: Option<Rvalue>,
        line: Option<usize>,
    },
    /// Expression statement (side effects)
    Expr {
        expr: ExprKind,
        line: Option<usize>,
    },
}

#[derive(Debug, Clone)]
pub enum Terminator {
    /// Jump to single successor block
    Goto { target: BlockId },
    /// Conditional branch: if condition { then_block } else { else_block }
    Branch {
        condition: VarId,
        then_block: BlockId,
        else_block: BlockId,
    },
    /// Return statement
    Return { value: Option<VarId> },
    /// Unreachable/panic
    Unreachable,
}

#[derive(Debug, Clone)]
pub enum Edge {
    Sequential,
    Branch { condition: bool },
    LoopBack,
}

/// Variable identifier with SSA-like versioning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarId {
    pub name_id: u32,      // Index into name table
    pub version: u32,      // SSA version (0 for initial def)
}

/// Right-hand side of assignment
#[derive(Debug, Clone)]
pub enum Rvalue {
    /// Use of a variable
    Use(VarId),
    /// Binary operation: x + y
    BinaryOp {
        op: BinOp,
        left: VarId,
        right: VarId,
    },
    /// Unary operation: !x, -x
    UnaryOp {
        op: UnOp,
        operand: VarId,
    },
    /// Constant value
    Constant,
    /// Function call
    Call {
        func: String,
        args: Vec<VarId>,
    },
    /// Field access: x.field
    FieldAccess {
        base: VarId,
        field: String,
    },
    /// Reference: &x, &mut x
    Ref {
        var: VarId,
        mutable: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Eq, Ne, Lt, Gt, Le, Ge, And, Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg, Not, Deref,
}

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
    Other,
}

/// Data flow analysis results
#[derive(Debug, Clone)]
pub struct DataFlowAnalysis {
    pub reaching_defs: ReachingDefinitions,
    pub liveness: LivenessInfo,
    pub escape_info: EscapeAnalysis,
    pub taint_info: TaintAnalysis,
}

/// Reaching definitions analysis (forward data flow)
#[derive(Debug, Clone)]
pub struct ReachingDefinitions {
    /// For each block, the set of definitions reaching its entry
    pub reach_in: HashMap<BlockId, HashSet<VarId>>,
    /// For each block, the set of definitions reaching its exit
    pub reach_out: HashMap<BlockId, HashSet<VarId>>,
    /// For each variable use, the set of definitions that may reach it
    pub def_use_chains: HashMap<VarId, HashSet<VarId>>,
}

/// Liveness analysis (backward data flow)
#[derive(Debug, Clone)]
pub struct LivenessInfo {
    /// For each block, variables live at entry
    pub live_in: HashMap<BlockId, HashSet<VarId>>,
    /// For each block, variables live at exit
    pub live_out: HashMap<BlockId, HashSet<VarId>>,
    /// Set of variables that are written but never read (dead stores)
    pub dead_stores: HashSet<VarId>,
}

/// Escape analysis results
#[derive(Debug, Clone)]
pub struct EscapeAnalysis {
    /// Variables that escape the function (returned, captured, passed to external functions)
    pub escaping_vars: HashSet<VarId>,
    /// Variables captured by closures
    pub captured_vars: HashSet<VarId>,
    /// Variables that affect the return value
    pub return_dependencies: HashSet<VarId>,
}

/// Taint analysis results
#[derive(Debug, Clone)]
pub struct TaintAnalysis {
    /// Variables marked as tainted (involved in mutations)
    pub tainted_vars: HashSet<VarId>,
    /// Taint sources (mutation sites)
    pub taint_sources: HashMap<VarId, TaintSource>,
    /// Whether taint reaches the return value
    pub return_tainted: bool,
}

#[derive(Debug, Clone)]
pub enum TaintSource {
    LocalMutation { line: Option<usize> },
    ExternalMutation { line: Option<usize> },
    ImpureCall { callee: String, line: Option<usize> },
}
```

### Algorithm: Control Flow Graph Construction

```rust
impl ControlFlowGraph {
    /// Build CFG from a function's block
    pub fn from_block(block: &Block) -> Self {
        let mut builder = CfgBuilder::new();
        builder.process_block(block);
        builder.finalize()
    }
}

struct CfgBuilder {
    blocks: Vec<BasicBlock>,
    current_block: Vec<Statement>,
    block_counter: usize,
    edges: HashMap<BlockId, Vec<Edge>>,
    var_names: HashMap<String, u32>,
    var_versions: HashMap<u32, u32>,
}

impl CfgBuilder {
    fn new() -> Self {
        Self {
            blocks: Vec::new(),
            current_block: Vec::new(),
            block_counter: 0,
            edges: HashMap::new(),
            var_names: HashMap::new(),
            var_versions: HashMap::new(),
        }
    }

    fn process_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            match stmt {
                Stmt::Local(local) => {
                    self.process_local(local);
                }
                Stmt::Expr(expr, _) => {
                    self.process_expr(expr);
                }
                Stmt::Item(_) => {
                    // Skip item definitions
                }
                _ => {}
            }
        }
    }

    fn process_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::If(expr_if) => {
                // Create branch: current block → then_block, else_block
                self.finalize_current_block(Terminator::Branch {
                    condition: self.expr_to_var(&expr_if.cond),
                    then_block: self.next_block_id(),
                    else_block: BlockId(self.block_counter + 2),
                });

                // Process then branch
                self.process_block(&expr_if.then_branch);

                // Process else branch
                if let Some((_, else_branch)) = &expr_if.else_branch {
                    self.process_expr(else_branch);
                }
            }
            Expr::While(expr_while) => {
                // Create loop: condition_block → body_block (loop back)
                let loop_head = self.next_block_id();
                let body_block = BlockId(self.block_counter + 1);
                let exit_block = BlockId(self.block_counter + 2);

                self.finalize_current_block(Terminator::Goto {
                    target: loop_head,
                });

                // Loop condition check
                self.start_new_block();
                self.finalize_current_block(Terminator::Branch {
                    condition: self.expr_to_var(&expr_while.cond),
                    then_block: body_block,
                    else_block: exit_block,
                });

                // Loop body
                self.process_block(&expr_while.body);
                self.add_edge(body_block, loop_head, Edge::LoopBack);
            }
            Expr::Return(expr_return) => {
                let return_var = expr_return.expr.as_ref().map(|e| self.expr_to_var(e));
                self.finalize_current_block(Terminator::Return { value: return_var });
            }
            Expr::Assign(assign) => {
                let target = self.expr_to_var(&assign.left);
                let source = self.expr_to_rvalue(&assign.right);
                self.current_block.push(Statement::Assign {
                    target,
                    source,
                    line: None,
                });
            }
            _ => {
                // Other expressions (method calls, etc.)
                self.current_block.push(Statement::Expr {
                    expr: self.classify_expr(expr),
                    line: None,
                });
            }
        }
    }

    fn expr_to_var(&mut self, expr: &Expr) -> VarId {
        // Simplified: extract variable identifier from expression
        // In real implementation, handle complex expressions
        match expr {
            Expr::Path(path) => {
                let name = path.path.segments.last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();
                self.get_or_create_var(&name)
            }
            _ => {
                // Create temporary variable for complex expressions
                let temp_name = format!("_temp_{}", self.block_counter);
                self.get_or_create_var(&temp_name)
            }
        }
    }

    fn get_or_create_var(&mut self, name: &str) -> VarId {
        let name_id = *self.var_names.entry(name.to_string())
            .or_insert_with(|| self.var_names.len() as u32);
        let version = *self.var_versions.entry(name_id).or_insert(0);
        VarId { name_id, version }
    }

    fn finalize_current_block(&mut self, terminator: Terminator) {
        let block = BasicBlock {
            id: BlockId(self.block_counter),
            statements: std::mem::take(&mut self.current_block),
            terminator,
        };
        self.blocks.push(block);
        self.block_counter += 1;
    }

    fn next_block_id(&self) -> BlockId {
        BlockId(self.block_counter + 1)
    }

    fn start_new_block(&mut self) {
        self.current_block.clear();
    }

    fn add_edge(&mut self, from: BlockId, to: BlockId, edge: Edge) {
        self.edges.entry(from).or_default().push(edge);
    }

    fn finalize(mut self) -> ControlFlowGraph {
        // Finalize any remaining block
        if !self.current_block.is_empty() {
            self.finalize_current_block(Terminator::Return { value: None });
        }

        let exit_blocks = self.blocks.iter()
            .filter(|b| matches!(b.terminator, Terminator::Return { .. }))
            .map(|b| b.id)
            .collect();

        ControlFlowGraph {
            blocks: self.blocks,
            entry_block: BlockId(0),
            exit_blocks,
            edges: self.edges,
        }
    }
}
```

### Algorithm: Liveness Analysis (Backward Data Flow)

```rust
impl LivenessInfo {
    /// Compute liveness information for a CFG
    pub fn analyze(cfg: &ControlFlowGraph) -> Self {
        let mut live_in: HashMap<BlockId, HashSet<VarId>> = HashMap::new();
        let mut live_out: HashMap<BlockId, HashSet<VarId>> = HashMap::new();

        // Initialize all sets to empty
        for block in &cfg.blocks {
            live_in.insert(block.id, HashSet::new());
            live_out.insert(block.id, HashSet::new());
        }

        // Iterative fixed-point computation (backward data flow)
        let mut changed = true;
        while changed {
            changed = false;

            // Process blocks in reverse topological order (backward)
            for block in cfg.blocks.iter().rev() {
                // Compute use and def sets for this block
                let (use_set, def_set) = Self::compute_use_def(block);

                // live_out[B] = union of live_in[S] for all successors S
                let mut new_live_out = HashSet::new();
                for successor_id in Self::get_successors(block, cfg) {
                    if let Some(succ_live_in) = live_in.get(&successor_id) {
                        new_live_out.extend(succ_live_in.iter().copied());
                    }
                }

                // live_in[B] = use[B] ∪ (live_out[B] - def[B])
                let mut new_live_in = use_set.clone();
                for var in new_live_out.iter() {
                    if !def_set.contains(var) {
                        new_live_in.insert(*var);
                    }
                }

                // Check if anything changed
                if new_live_in != *live_in.get(&block.id).unwrap()
                    || new_live_out != *live_out.get(&block.id).unwrap()
                {
                    changed = true;
                    live_in.insert(block.id, new_live_in);
                    live_out.insert(block.id, new_live_out);
                }
            }
        }

        // Identify dead stores: definitions that are not live after the definition point
        let dead_stores = Self::find_dead_stores(cfg, &live_out);

        LivenessInfo {
            live_in,
            live_out,
            dead_stores,
        }
    }

    /// Compute USE and DEF sets for a basic block
    fn compute_use_def(block: &BasicBlock) -> (HashSet<VarId>, HashSet<VarId>) {
        let mut use_set = HashSet::new();
        let mut def_set = HashSet::new();

        // Process statements in order
        for stmt in &block.statements {
            match stmt {
                Statement::Assign { target, source, .. } => {
                    // Add variables used in source (before definition)
                    Self::add_rvalue_uses(source, &mut use_set, &def_set);
                    // Add target to def set
                    def_set.insert(*target);
                }
                Statement::Declare { var, init, .. } => {
                    if let Some(init_val) = init {
                        Self::add_rvalue_uses(init_val, &mut use_set, &def_set);
                    }
                    def_set.insert(*var);
                }
                Statement::Expr { expr, .. } => {
                    // Add variables used in expression
                    Self::add_expr_uses(expr, &mut use_set, &def_set);
                }
            }
        }

        // Handle terminator
        match &block.terminator {
            Terminator::Branch { condition, .. } => {
                if !def_set.contains(condition) {
                    use_set.insert(*condition);
                }
            }
            Terminator::Return { value: Some(var) } => {
                if !def_set.contains(var) {
                    use_set.insert(*var);
                }
            }
            _ => {}
        }

        (use_set, def_set)
    }

    fn add_rvalue_uses(
        rvalue: &Rvalue,
        use_set: &mut HashSet<VarId>,
        def_set: &HashSet<VarId>,
    ) {
        match rvalue {
            Rvalue::Use(var) => {
                if !def_set.contains(var) {
                    use_set.insert(*var);
                }
            }
            Rvalue::BinaryOp { left, right, .. } => {
                if !def_set.contains(left) {
                    use_set.insert(*left);
                }
                if !def_set.contains(right) {
                    use_set.insert(*right);
                }
            }
            Rvalue::UnaryOp { operand, .. } => {
                if !def_set.contains(operand) {
                    use_set.insert(*operand);
                }
            }
            Rvalue::Call { args, .. } => {
                for arg in args {
                    if !def_set.contains(arg) {
                        use_set.insert(*arg);
                    }
                }
            }
            Rvalue::FieldAccess { base, .. } => {
                if !def_set.contains(base) {
                    use_set.insert(*base);
                }
            }
            Rvalue::Ref { var, .. } => {
                if !def_set.contains(var) {
                    use_set.insert(*var);
                }
            }
            Rvalue::Constant => {}
        }
    }

    fn add_expr_uses(
        expr: &ExprKind,
        use_set: &mut HashSet<VarId>,
        def_set: &HashSet<VarId>,
    ) {
        match expr {
            ExprKind::MethodCall { receiver, args, .. } => {
                if !def_set.contains(receiver) {
                    use_set.insert(*receiver);
                }
                for arg in args {
                    if !def_set.contains(arg) {
                        use_set.insert(*arg);
                    }
                }
            }
            ExprKind::MacroCall { args, .. } => {
                for arg in args {
                    if !def_set.contains(arg) {
                        use_set.insert(*arg);
                    }
                }
            }
            ExprKind::Other => {}
        }
    }

    fn get_successors(block: &BasicBlock, cfg: &ControlFlowGraph) -> Vec<BlockId> {
        match &block.terminator {
            Terminator::Goto { target } => vec![*target],
            Terminator::Branch {
                then_block,
                else_block,
                ..
            } => vec![*then_block, *else_block],
            Terminator::Return { .. } | Terminator::Unreachable => vec![],
        }
    }

    fn find_dead_stores(
        cfg: &ControlFlowGraph,
        live_out: &HashMap<BlockId, HashSet<VarId>>,
    ) -> HashSet<VarId> {
        let mut dead_stores = HashSet::new();

        for block in &cfg.blocks {
            let block_live_out = live_out.get(&block.id).unwrap();

            for stmt in &block.statements {
                if let Statement::Assign { target, .. } | Statement::Declare { var: target, .. } =
                    stmt
                {
                    // If target is not in live_out, it's a dead store
                    if !block_live_out.contains(target) {
                        dead_stores.insert(*target);
                    }
                }
            }
        }

        dead_stores
    }
}
```

### Algorithm: Escape Analysis

```rust
impl EscapeAnalysis {
    /// Determine which variables escape the function
    pub fn analyze(cfg: &ControlFlowGraph) -> Self {
        let mut escaping_vars = HashSet::new();
        let mut captured_vars = HashSet::new();
        let mut return_dependencies = HashSet::new();

        // Collect return values (direct escapes)
        for block in &cfg.blocks {
            if let Terminator::Return { value: Some(var) } = &block.terminator {
                return_dependencies.insert(*var);
                escaping_vars.insert(*var);
            }
        }

        // Trace dependencies backward from return values
        let mut worklist: Vec<VarId> = return_dependencies.iter().copied().collect();
        let mut visited = HashSet::new();

        while let Some(var) = worklist.pop() {
            if visited.contains(&var) {
                continue;
            }
            visited.insert(var);

            // Find definition of this variable
            for block in &cfg.blocks {
                for stmt in &block.statements {
                    match stmt {
                        Statement::Assign { target, source, .. } if target == &var => {
                            // Add variables used in source to dependencies
                            Self::add_source_dependencies(source, &mut return_dependencies, &mut worklist);
                        }
                        Statement::Declare { var: target, init: Some(init), .. }
                            if target == &var =>
                        {
                            Self::add_source_dependencies(init, &mut return_dependencies, &mut worklist);
                        }
                        _ => {}
                    }
                }
            }
        }

        // Detect closures and captured variables
        for block in &cfg.blocks {
            for stmt in &block.statements {
                // Look for closure creation expressions
                // In real implementation, detect Expr::Closure and analyze captures
                // For now, simplified: mark variables used in closures as captured
            }
        }

        // Variables passed to external functions also escape
        for block in &cfg.blocks {
            for stmt in &block.statements {
                if let Statement::Expr {
                    expr: ExprKind::MethodCall { args, .. },
                    ..
                } = stmt
                {
                    // Conservative: assume all args escape
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

    fn add_source_dependencies(
        source: &Rvalue,
        deps: &mut HashSet<VarId>,
        worklist: &mut Vec<VarId>,
    ) {
        match source {
            Rvalue::Use(var) => {
                deps.insert(*var);
                worklist.push(*var);
            }
            Rvalue::BinaryOp { left, right, .. } => {
                deps.insert(*left);
                deps.insert(*right);
                worklist.push(*left);
                worklist.push(*right);
            }
            Rvalue::UnaryOp { operand, .. } => {
                deps.insert(*operand);
                worklist.push(*operand);
            }
            Rvalue::Call { args, .. } => {
                for arg in args {
                    deps.insert(*arg);
                    worklist.push(*arg);
                }
            }
            Rvalue::FieldAccess { base, .. } => {
                deps.insert(*base);
                worklist.push(*base);
            }
            Rvalue::Ref { var, .. } => {
                deps.insert(*var);
                worklist.push(*var);
            }
            Rvalue::Constant => {}
        }
    }
}
```

### Algorithm: Taint Analysis

```rust
impl TaintAnalysis {
    /// Track taint propagation from mutation sites
    pub fn analyze(
        cfg: &ControlFlowGraph,
        mutations: &[LocalMutation],
        liveness: &LivenessInfo,
        escape: &EscapeAnalysis,
    ) -> Self {
        let mut tainted_vars = HashSet::new();
        let mut taint_sources = HashMap::new();

        // Mark mutation targets as taint sources
        for mutation in mutations {
            // Find VarId corresponding to mutation target
            // Simplified: assume mutation.target is the variable name
            // In real implementation, map names to VarIds
        }

        // Propagate taint through def-use chains (forward)
        let mut changed = true;
        while changed {
            changed = false;

            for block in &cfg.blocks {
                for stmt in &block.statements {
                    match stmt {
                        Statement::Assign { target, source, line } => {
                            // If source is tainted, target becomes tainted
                            if Self::is_source_tainted(source, &tainted_vars) {
                                if tainted_vars.insert(*target) {
                                    changed = true;
                                    taint_sources.insert(
                                        *target,
                                        TaintSource::LocalMutation { line: *line },
                                    );
                                }
                            }
                        }
                        Statement::Declare { var, init: Some(init), line } => {
                            if Self::is_source_tainted(init, &tainted_vars) {
                                if tainted_vars.insert(*var) {
                                    changed = true;
                                    taint_sources.insert(
                                        *var,
                                        TaintSource::LocalMutation { line: *line },
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Filter dead taint: only consider taint on live variables
        tainted_vars.retain(|var| !liveness.dead_stores.contains(var));

        // Check if taint reaches return value
        let return_tainted = tainted_vars
            .iter()
            .any(|var| escape.return_dependencies.contains(var));

        TaintAnalysis {
            tainted_vars,
            taint_sources,
            return_tainted,
        }
    }

    fn is_source_tainted(source: &Rvalue, tainted_vars: &HashSet<VarId>) -> bool {
        match source {
            Rvalue::Use(var) => tainted_vars.contains(var),
            Rvalue::BinaryOp { left, right, .. } => {
                tainted_vars.contains(left) || tainted_vars.contains(right)
            }
            Rvalue::UnaryOp { operand, .. } => tainted_vars.contains(operand),
            Rvalue::Call { args, .. } => args.iter().any(|arg| tainted_vars.contains(arg)),
            Rvalue::FieldAccess { base, .. } => tainted_vars.contains(base),
            Rvalue::Ref { var, .. } => tainted_vars.contains(var),
            Rvalue::Constant => false,
        }
    }
}
```

### Integration with PurityDetector

```rust
// In src/analyzers/purity_detector.rs

impl PurityDetector {
    pub fn is_pure_function(&mut self, item_fn: &ItemFn) -> PurityAnalysis {
        // Existing analysis...
        self.visit_item_fn(item_fn);

        // NEW: Data flow analysis
        let cfg = ControlFlowGraph::from_block(&item_fn.block);
        let data_flow = DataFlowAnalysis::compute(&cfg, &self.local_mutations);

        // Refine purity classification using data flow
        let effective_mutations = self.filter_dead_mutations(&data_flow);
        let has_escaping_mutations = self.check_mutation_escape(&data_flow);

        // Determine refined purity level
        let purity_level = if !self.has_side_effects
            && effective_mutations.is_empty()
            && !has_escaping_mutations
        {
            PurityLevel::StrictlyPure
        } else if !self.modifies_external_state && !has_escaping_mutations {
            PurityLevel::LocallyPure  // Local mutations don't escape
        } else if !self.modifies_external_state {
            PurityLevel::ReadOnly
        } else {
            PurityLevel::Impure
        };

        // Adjust confidence based on liveness analysis
        let mut confidence = self.compute_base_confidence();
        if effective_mutations.len() < self.local_mutations.len() {
            // Many dead mutations removed → higher confidence
            confidence *= 1.1;
        }

        PurityAnalysis {
            purity_level,
            confidence: confidence.min(1.0),
            violations: self.generate_violations(&effective_mutations),
            data_flow_info: Some(data_flow),
        }
    }

    fn filter_dead_mutations(&self, data_flow: &DataFlowAnalysis) -> Vec<LocalMutation> {
        self.local_mutations
            .iter()
            .filter(|mutation| {
                // Keep mutation if it's live (affects control flow or return value)
                !Self::is_dead_mutation(mutation, data_flow)
            })
            .cloned()
            .collect()
    }

    fn is_dead_mutation(mutation: &LocalMutation, data_flow: &DataFlowAnalysis) -> bool {
        // Find VarId for mutation target
        // Check if variable is in dead_stores set
        // Simplified logic for illustration
        false
    }

    fn check_mutation_escape(&self, data_flow: &DataFlowAnalysis) -> bool {
        // Check if any mutated variable escapes
        data_flow.taint_info.return_tainted
            || !data_flow.escape_info.escaping_vars.is_empty()
    }
}
```

### Integration with AlmostPureAnalyzer

```rust
// In src/analysis/almost_pure.rs (Spec 162)

impl AlmostPureAnalyzer {
    pub fn detect_almost_pure(
        &self,
        func: &FunctionMetrics,
        purity_analysis: &PurityAnalysis,
    ) -> Option<AlmostPureFunction> {
        // Filter violations to exclude dead mutations
        let live_violations = if let Some(data_flow) = &purity_analysis.data_flow_info {
            self.filter_live_violations(&purity_analysis.violations, data_flow)
        } else {
            purity_analysis.violations.clone()
        };

        // Must have exactly 1-2 LIVE violations
        if live_violations.is_empty() || live_violations.len() > 2 {
            return None;
        }

        // ... rest of existing logic using live_violations instead of all violations
        let strategy = self.suggest_refactoring(&live_violations);

        Some(AlmostPureFunction {
            function_id: FunctionId::from_metrics(func),
            violations: live_violations,
            suggested_strategy: strategy,
            current_multiplier: 1.0,
            potential_multiplier: 0.3,
        })
    }

    fn filter_live_violations(
        &self,
        violations: &[PurityViolation],
        data_flow: &DataFlowAnalysis,
    ) -> Vec<PurityViolation> {
        violations
            .iter()
            .filter(|violation| {
                // Check if violation involves dead code
                !self.is_violation_dead(violation, data_flow)
            })
            .cloned()
            .collect()
    }

    fn is_violation_dead(
        &self,
        violation: &PurityViolation,
        data_flow: &DataFlowAnalysis,
    ) -> bool {
        match violation {
            PurityViolation::StateMutation { target, .. } => {
                // Check if mutated variable is in dead_stores
                // Simplified: assume target name lookup
                false
            }
            _ => false, // I/O operations are never "dead"
        }
    }
}
```

## Dependencies

### Spec 159: Evidence-Based Purity Confidence

Data flow analysis provides additional evidence for confidence scoring:

```rust
pub struct PurityEvidence {
    // Existing fields...

    // NEW: Data flow evidence
    pub dead_mutation_count: usize,       // Mutations that are dead stores
    pub live_mutation_count: usize,       // Mutations that affect output
    pub escaping_mutation_count: usize,   // Mutations that escape scope
    pub return_dependency_count: usize,   // Variables affecting return value
}

impl PurityEvidence {
    pub fn calculate_confidence(&self) -> f64 {
        let mut score = 0.5;

        // Existing adjustments...

        // NEW: Adjust based on data flow
        if self.dead_mutation_count > 0 {
            score += 0.10; // Dead mutations don't affect purity
        }
        if self.escaping_mutation_count == 0 {
            score += 0.15; // No escaping mutations = likely pure
        }
        if self.live_mutation_count == 0 && self.return_dependency_count > 0 {
            score += 0.20; // Pure transformation of inputs to outputs
        }

        score.clamp(0.1, 1.0)
    }
}
```

### Spec 162: Almost Pure Function Detection

Data flow analysis eliminates false positives in almost-pure detection by filtering dead mutations. This improves refactoring suggestion accuracy by 20-30%.

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cfg_construction_sequential() {
        let code = quote! {
            {
                let x = 1;
                let y = x + 1;
                y
            }
        };
        let block = syn::parse2(code).unwrap();
        let cfg = ControlFlowGraph::from_block(&block);

        assert_eq!(cfg.blocks.len(), 1);
        assert_eq!(cfg.blocks[0].statements.len(), 2);
    }

    #[test]
    fn test_cfg_construction_conditional() {
        let code = quote! {
            {
                let x = 1;
                if x > 0 {
                    let y = x + 1;
                } else {
                    let y = x - 1;
                }
            }
        };
        let block = syn::parse2(code).unwrap();
        let cfg = ControlFlowGraph::from_block(&block);

        assert_eq!(cfg.blocks.len(), 3); // Entry, then, else
    }

    #[test]
    fn test_liveness_dead_store() {
        let code = quote! {
            {
                let mut x = 1;
                x = 2;  // Dead store
                let y = 3;
                y       // x never used
            }
        };
        let block = syn::parse2(code).unwrap();
        let cfg = ControlFlowGraph::from_block(&block);
        let liveness = LivenessInfo::analyze(&cfg);

        // x should be in dead_stores
        assert!(!liveness.dead_stores.is_empty());
    }

    #[test]
    fn test_liveness_live_mutation() {
        let code = quote! {
            {
                let mut x = 1;
                x = x + 1;  // Live mutation
                x           // Used in return
            }
        };
        let block = syn::parse2(code).unwrap();
        let cfg = ControlFlowGraph::from_block(&block);
        let liveness = LivenessInfo::analyze(&cfg);

        // x should NOT be in dead_stores
        assert!(liveness.dead_stores.is_empty());
    }

    #[test]
    fn test_escape_analysis_local_only() {
        let code = quote! {
            {
                let mut cache = HashMap::new();
                cache.insert(1, 2);
                // cache never escapes
                42
            }
        };
        let block = syn::parse2(code).unwrap();
        let cfg = ControlFlowGraph::from_block(&block);
        let escape = EscapeAnalysis::analyze(&cfg);

        // cache should NOT escape
        assert!(escape.escaping_vars.is_empty());
    }

    #[test]
    fn test_escape_analysis_return_value() {
        let code = quote! {
            {
                let x = 1;
                let y = x + 1;
                y  // y escapes via return
            }
        };
        let block = syn::parse2(code).unwrap();
        let cfg = ControlFlowGraph::from_block(&block);
        let escape = EscapeAnalysis::analyze(&cfg);

        // y should escape
        assert!(!escape.return_dependencies.is_empty());
    }

    #[test]
    fn test_taint_propagation() {
        let code = quote! {
            {
                let mut x = 1;
                x = 2;      // Taint source
                let y = x;  // Taint propagates
                y           // Return is tainted
            }
        };
        let block = syn::parse2(code).unwrap();
        let cfg = ControlFlowGraph::from_block(&block);
        let liveness = LivenessInfo::analyze(&cfg);
        let escape = EscapeAnalysis::analyze(&cfg);
        let taint = TaintAnalysis::analyze(&cfg, &[], &liveness, &escape);

        assert!(taint.return_tainted);
    }

    #[test]
    fn test_taint_not_propagated_dead() {
        let code = quote! {
            {
                let mut x = 1;
                x = 2;      // Dead mutation
                42          // Returns constant, no taint
            }
        };
        let block = syn::parse2(code).unwrap();
        let cfg = ControlFlowGraph::from_block(&block);
        let liveness = LivenessInfo::analyze(&cfg);
        let escape = EscapeAnalysis::analyze(&cfg);
        let taint = TaintAnalysis::analyze(&cfg, &[], &liveness, &escape);

        assert!(!taint.return_tainted);
    }
}
```

### Integration Tests

```rust
#[test]
fn test_purity_detector_with_data_flow() {
    let code = r#"
        fn calculate(x: i32) -> i32 {
            let mut temp = x;
            temp = temp * 2;  // Dead mutation
            x + 1             // Returns unrelated value
        }
    "#;

    let syntax = syn::parse_file(code).unwrap();
    let func = &syntax.items[0];

    let mut detector = PurityDetector::new();
    let analysis = detector.is_pure_function(func);

    // Should be classified as pure despite mutation
    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    assert!(analysis.confidence > 0.85);
}

#[test]
fn test_almost_pure_with_data_flow() {
    let code = r#"
        fn calculate(items: &[Item]) -> f64 {
            let mut temp = 0;
            temp = 1;  // Dead mutation
            let total = items.iter().map(|i| i.price).sum();
            println!("Total: {}", total);  // Live violation
            total
        }
    "#;

    let metrics = analyze_function(code);
    let analyzer = AlmostPureAnalyzer::new();
    let almost_pure = analyzer.detect_almost_pure(&metrics.functions[0], &metrics.purity);

    // Should detect only 1 live violation (println!)
    assert!(almost_pure.is_some());
    assert_eq!(almost_pure.unwrap().violations.len(), 1);
}
```

### Performance Benchmarks

```rust
#[bench]
fn bench_cfg_construction(b: &mut Bencher) {
    let code = load_test_function("large_function.rs");
    b.iter(|| {
        ControlFlowGraph::from_block(&code)
    });
}

#[bench]
fn bench_liveness_analysis(b: &mut Bencher) {
    let cfg = load_test_cfg("large_function.rs");
    b.iter(|| {
        LivenessInfo::analyze(&cfg)
    });
}

#[bench]
fn bench_full_data_flow(b: &mut Bencher) {
    let code = load_test_function("large_function.rs");
    b.iter(|| {
        let cfg = ControlFlowGraph::from_block(&code);
        DataFlowAnalysis::compute(&cfg, &[])
    });
}
```

## Documentation Requirements

### Architecture Documentation

Add to `ARCHITECTURE.md`:

```markdown
## Data Flow Analysis

Debtmap performs intra-procedural data flow analysis to improve accuracy of purity and state transition detection.

### Components

1. **Control Flow Graph (CFG)**: Represents function control flow as basic blocks
2. **Liveness Analysis**: Identifies variables that are live (used after definition)
3. **Reaching Definitions**: Tracks which definitions reach each program point
4. **Escape Analysis**: Determines if local variables escape function scope
5. **Taint Analysis**: Tracks propagation of mutations through data flow

### Integration

Data flow analysis runs as a post-processing step after AST-based analysis:

```
RustAnalyzer → PurityDetector → DataFlowAnalysis → Refined PurityLevel
```

### Performance

- CFG construction: O(n) where n = number of statements
- Liveness analysis: O(n × b) where b = number of basic blocks (typically < 20)
- Escape analysis: O(n × d) where d = max dependency depth (typically < 10)
- Total overhead: < 20% increase in analysis time

### Limitations

- **Intra-procedural only**: Does not analyze across function boundaries
- **Conservative**: Over-approximates escapes and under-approximates dead code
- **Simplified CFG**: Does not model all Rust control flow (match guards, etc.)
```

### API Documentation

```rust
/// Data flow analysis for a function
///
/// Computes control flow graph (CFG), liveness, reaching definitions,
/// escape analysis, and taint propagation.
///
/// # Examples
///
/// ```
/// use debtmap::analysis::data_flow::DataFlowAnalysis;
/// use syn::parse_quote;
///
/// let block = parse_quote! {
///     {
///         let mut x = 1;
///         x = x + 1;
///         x
///     }
/// };
///
/// let analysis = DataFlowAnalysis::from_block(&block);
/// assert!(!analysis.liveness.dead_stores.is_empty());
/// ```
pub struct DataFlowAnalysis {
    // ...
}
```

## Implementation Notes

### Phased Implementation

**Phase 1: CFG Construction** (Week 1)
- Implement basic block extraction
- Handle sequential, conditional, and loop control flow
- Tests: CFG structure validation

**Phase 2: Liveness Analysis** (Week 2)
- Implement backward data flow algorithm
- Compute live-in/live-out sets
- Identify dead stores
- Tests: Liveness for various control flow patterns

**Phase 3: Escape Analysis** (Week 3)
- Track return value dependencies
- Detect closure captures
- Identify escaping references
- Tests: Escape detection accuracy

**Phase 4: Integration** (Week 4)
- Integrate with `PurityDetector`
- Integrate with `AlmostPureAnalyzer`
- Update confidence scoring
- Tests: End-to-end false positive reduction

**Phase 5: Optimization** (Week 5)
- Profile performance bottlenecks
- Optimize fixed-point algorithms
- Cache CFG construction results
- Tests: Performance benchmarks

### Simplifications

- **No inter-procedural analysis**: Analyze functions in isolation
- **Conservative approximations**: Over-approximate escapes, under-approximate dead code
- **Simplified CFG**: Model core Rust control flow (if, loop, return), skip complex patterns
- **Path-insensitive**: Do not track different execution paths separately

### Future Enhancements (Out of Scope)

- **Inter-procedural analysis**: Track state flow across function calls
- **Path-sensitive analysis**: Analyze different execution paths separately
- **Alias analysis**: Detect pointer aliasing for more precise escape analysis
- **Loop-invariant detection**: Identify computations that can be hoisted out of loops

## Migration and Compatibility

### Backward Compatibility

- New `DataFlowAnalysis` is optional field in `PurityAnalysis`
- Existing code without data flow continues to work
- Gradual rollout: Enable data flow per-file or per-project basis

### Performance Impact

- Expected overhead: 10-20% increase in analysis time
- Memory overhead: ~50MB for 100k LOC codebase
- Configurable: Can disable data flow analysis via config flag

### Migration Path

1. **Alpha release**: Data flow disabled by default, opt-in via `--enable-data-flow`
2. **Beta release**: Data flow enabled by default, opt-out via `--disable-data-flow`
3. **Stable release**: Data flow always enabled, flag removed

## Risk Assessment

### Technical Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Performance overhead too high | Medium | High | Profile and optimize hot paths; provide disable flag |
| False negatives in escape analysis | Medium | Medium | Conservative approximations; extensive testing |
| Complex CFG construction bugs | High | Medium | Incremental testing; start with simple control flow |
| Integration breaks existing analysis | Low | High | Comprehensive regression tests; phased rollout |

### Mitigation Strategies

- **Extensive testing**: 100+ unit tests, 50+ integration tests
- **Performance monitoring**: Benchmarks for all algorithms
- **Incremental rollout**: Feature flag, alpha/beta releases
- **Conservative defaults**: Favor correctness over precision
