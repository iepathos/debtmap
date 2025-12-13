//! Control flow graph types and data structures.
//!
//! This module defines the core types used for CFG construction and
//! data flow analysis, including basic blocks, edges, and variable tracking.
//!
//! # Key Types
//!
//! - [`ControlFlowGraph`] - The complete control flow graph for a function
//! - [`BasicBlock`] - A basic block containing statements and a terminator
//! - [`BlockId`] - Unique identifier for a basic block
//! - [`Statement`] - Assignment, declaration, or expression statement
//! - [`Terminator`] - How control leaves a block (goto, branch, return, etc.)
//! - [`VarId`] - Variable identifier with SSA-like versioning

use std::collections::HashMap;

/// Control Flow Graph for intra-procedural analysis.
///
/// Represents a function's control flow as a directed graph of basic blocks.
/// Each basic block contains a sequence of statements with no branches except at the end.
///
/// # Example
///
/// ```ignore
/// use debtmap::analysis::data_flow::ControlFlowGraph;
/// use syn::parse_quote;
///
/// let block = parse_quote! {
///     {
///         let x = if cond { 1 } else { 2 };
///         x + 1
///     }
/// };
///
/// let cfg = ControlFlowGraph::from_block(&block);
/// // CFG will have separate blocks for the if-then-else branches
/// assert!(cfg.blocks.len() >= 3);
/// ```
#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    /// All basic blocks in the CFG
    pub blocks: Vec<BasicBlock>,
    /// The entry block (where execution starts)
    pub entry_block: BlockId,
    /// Exit blocks (where execution may end)
    pub exit_blocks: Vec<BlockId>,
    /// Control flow edges between blocks
    pub edges: HashMap<BlockId, Vec<(BlockId, Edge)>>,
    /// Variable names encountered during CFG construction
    pub var_names: Vec<String>,
    /// Variables captured by closures
    pub captured_vars: Vec<CapturedVar>,
}

/// Unique identifier for a basic block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);

/// A basic block in the control flow graph.
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockId,
    pub statements: Vec<Statement>,
    pub terminator: Terminator,
}

/// A statement within a basic block.
#[derive(Debug, Clone)]
pub enum Statement {
    Assign {
        target: VarId,
        source: Rvalue,
        line: Option<usize>,
    },
    Declare {
        var: VarId,
        init: Option<Rvalue>,
        line: Option<usize>,
    },
    Expr {
        expr: ExprKind,
        line: Option<usize>,
    },
}

/// A match arm in the CFG.
///
/// Represents a single arm of a match expression in the control flow graph.
/// Each arm has its own basic block for the arm body, optionally has a guard
/// condition, and tracks the pattern bindings created in that arm.
#[derive(Debug, Clone)]
pub struct MatchArm {
    /// Block that handles this arm's body
    pub block: BlockId,
    /// Optional guard condition variable (for `if` guards)
    pub guard: Option<VarId>,
    /// Pattern bindings created in this arm
    pub bindings: Vec<VarId>,
}

/// Block terminator - how control leaves the block.
#[derive(Debug, Clone)]
pub enum Terminator {
    Goto {
        target: BlockId,
    },
    Branch {
        condition: VarId,
        then_block: BlockId,
        else_block: BlockId,
    },
    /// Multi-way branch for match expressions.
    ///
    /// Models the control flow of a match expression where the scrutinee
    /// is evaluated and control branches to one of multiple arm blocks.
    Match {
        /// The variable being matched on
        scrutinee: VarId,
        /// The arms of the match expression
        arms: Vec<MatchArm>,
        /// Join block where all arm paths converge
        join_block: BlockId,
    },
    Return {
        value: Option<VarId>,
    },
    Unreachable,
}

/// Edge type in the control flow graph.
#[derive(Debug, Clone)]
pub enum Edge {
    Sequential,
    Branch {
        condition: bool,
    },
    LoopBack,
    /// Edge from match expression to an arm block.
    MatchArm(usize),
    /// Edge from a match arm to the join block.
    MatchJoin,
}

/// Variable identifier with SSA-like versioning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarId {
    pub name_id: u32,
    pub version: u32,
}

/// Capture mode for closure variables.
///
/// Determines how a variable is captured by a closure:
/// - `ByValue`: The variable is moved into the closure (via `move` keyword)
/// - `ByRef`: The variable is borrowed immutably (`&T`)
/// - `ByMutRef`: The variable is borrowed mutably (`&mut T`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    /// Variable is moved into the closure (move closure)
    ByValue,
    /// Variable is borrowed immutably (&T)
    ByRef,
    /// Variable is borrowed mutably (&mut T)
    ByMutRef,
}

/// Information about a captured variable in a closure.
#[derive(Debug, Clone)]
pub struct CapturedVar {
    /// The variable ID of the captured variable
    pub var_id: VarId,
    /// How the variable is captured
    pub capture_mode: CaptureMode,
    /// Whether the variable is mutated inside the closure body
    pub is_mutated: bool,
}

/// Index of a statement within a basic block.
pub type StatementIdx = usize;

/// A specific program point: block and statement within that block.
///
/// Program points are used to precisely identify locations in the CFG
/// where definitions and uses occur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProgramPoint {
    /// The block containing this program point.
    pub block: BlockId,
    /// The statement index within the block.
    /// For terminators, this equals the number of statements (past the last statement).
    pub stmt: StatementIdx,
}

impl ProgramPoint {
    /// Create a new program point.
    pub fn new(block: BlockId, stmt: StatementIdx) -> Self {
        Self { block, stmt }
    }

    /// Create a point at the start of a block (before first statement).
    pub fn block_entry(block: BlockId) -> Self {
        Self { block, stmt: 0 }
    }

    /// Create a point at the end of a block (at the terminator).
    pub fn block_exit(block: BlockId, stmt_count: usize) -> Self {
        Self {
            block,
            stmt: stmt_count,
        }
    }
}

/// A definition occurrence: variable defined at a specific point.
///
/// Represents a single definition (assignment or declaration) of a variable
/// at a precise location in the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Definition {
    /// The variable being defined.
    pub var: VarId,
    /// The program point where the definition occurs.
    pub point: ProgramPoint,
}

/// A use occurrence: variable used at a specific point.
///
/// Represents a single use (read) of a variable at a precise location
/// in the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Use {
    /// The variable being used.
    pub var: VarId,
    /// The program point where the use occurs.
    pub point: ProgramPoint,
}

/// Right-hand side of assignment.
#[derive(Debug, Clone)]
pub enum Rvalue {
    Use(VarId),
    BinaryOp {
        op: BinOp,
        left: VarId,
        right: VarId,
    },
    UnaryOp {
        op: UnOp,
        operand: VarId,
    },
    Constant,
    Call {
        func: String,
        args: Vec<VarId>,
    },
    FieldAccess {
        base: VarId,
        field: String,
    },
    Ref {
        var: VarId,
        mutable: bool,
    },
}

/// Binary operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

/// Unary operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
    Deref,
}

/// Expression kinds for side effect tracking.
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
    /// Expression with no tracked variable uses
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_point_creation() {
        let point = ProgramPoint::new(BlockId(0), 5);
        assert_eq!(point.block.0, 0);
        assert_eq!(point.stmt, 5);

        let entry = ProgramPoint::block_entry(BlockId(1));
        assert_eq!(entry.block.0, 1);
        assert_eq!(entry.stmt, 0);

        let exit = ProgramPoint::block_exit(BlockId(2), 10);
        assert_eq!(exit.block.0, 2);
        assert_eq!(exit.stmt, 10);
    }

    #[test]
    fn test_definition_equality() {
        let def1 = Definition {
            var: VarId {
                name_id: 0,
                version: 0,
            },
            point: ProgramPoint::new(BlockId(0), 0),
        };
        let def2 = Definition {
            var: VarId {
                name_id: 0,
                version: 0,
            },
            point: ProgramPoint::new(BlockId(0), 0),
        };
        let def3 = Definition {
            var: VarId {
                name_id: 0,
                version: 0,
            },
            point: ProgramPoint::new(BlockId(0), 1),
        };

        assert_eq!(def1, def2);
        assert_ne!(def1, def3);
    }

    #[test]
    fn test_var_id_equality() {
        let var1 = VarId {
            name_id: 1,
            version: 0,
        };
        let var2 = VarId {
            name_id: 1,
            version: 0,
        };
        let var3 = VarId {
            name_id: 1,
            version: 1,
        };

        assert_eq!(var1, var2);
        assert_ne!(var1, var3);
    }

    #[test]
    fn test_block_id_equality() {
        let block1 = BlockId(0);
        let block2 = BlockId(0);
        let block3 = BlockId(1);

        assert_eq!(block1, block2);
        assert_ne!(block1, block3);
    }
}
