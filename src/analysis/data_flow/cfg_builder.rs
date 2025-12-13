//! Control flow graph construction from Rust AST.
//!
//! This module transforms syn AST into a control flow graph suitable
//! for data flow analysis. Handles all Rust statement and expression types.
//!
//! # Example
//!
//! ```ignore
//! use debtmap::analysis::data_flow::{ControlFlowGraph, CfgBuilder};
//! use syn::parse_quote;
//!
//! let block = parse_quote! {
//!     {
//!         let x = 1;
//!         if x > 0 { 2 } else { 3 }
//!     }
//! };
//!
//! let cfg = ControlFlowGraph::from_block(&block);
//! ```

use std::collections::{HashMap, HashSet};

use syn::visit::Visit;
use syn::{
    Block, Expr, ExprAssign, ExprClosure, ExprIf, ExprMatch, ExprReturn, ExprWhile, Local, Pat,
    Stmt,
};

use super::types::{
    BasicBlock, BinOp, BlockId, CaptureMode, CapturedVar, ControlFlowGraph, Edge, ExprKind,
    MatchArm, Rvalue, Statement, Terminator, UnOp, VarId,
};

impl ControlFlowGraph {
    /// Build CFG from a function's block.
    pub fn from_block(block: &Block) -> Self {
        let mut builder = CfgBuilder::new();
        builder.process_block(block);
        builder.finalize()
    }
}

/// Builder for constructing control flow graphs from AST.
pub(crate) struct CfgBuilder {
    blocks: Vec<BasicBlock>,
    current_block: Vec<Statement>,
    block_counter: usize,
    edges: HashMap<BlockId, Vec<(BlockId, Edge)>>,
    var_names: HashMap<String, u32>,
    var_versions: HashMap<u32, u32>,
    /// Variables captured by closures in this function
    captured_vars: Vec<CapturedVar>,
}

impl CfgBuilder {
    pub(crate) fn new() -> Self {
        Self {
            blocks: Vec::new(),
            current_block: Vec::new(),
            block_counter: 0,
            edges: HashMap::new(),
            var_names: HashMap::new(),
            var_versions: HashMap::new(),
            captured_vars: Vec::new(),
        }
    }

    pub(crate) fn process_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.process_stmt(stmt);
        }
    }

    fn process_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Local(local) => {
                self.process_local(local);
            }
            Stmt::Expr(expr, _) => {
                self.process_expr(expr);
            }
            _ => {}
        }
    }

    fn process_local(&mut self, local: &Local) {
        // Extract all variable bindings from the pattern
        let vars = self.extract_vars_from_pattern(&local.pat);

        // Process any closures in the initializer first (to populate captured_vars)
        if let Some(init) = &local.init {
            self.process_closures_in_expr(&init.expr);
        }

        // Get Rvalue from initializer
        let init_rvalue = local
            .init
            .as_ref()
            .map(|init| self.expr_to_rvalue(&init.expr));

        // Emit declaration for each binding
        for var in vars {
            self.current_block.push(Statement::Declare {
                var,
                init: init_rvalue.clone(),
                line: None,
            });
        }
    }

    fn process_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::If(expr_if) => self.process_if(expr_if),
            Expr::While(expr_while) => self.process_while(expr_while),
            Expr::Return(expr_return) => self.process_return(expr_return),
            Expr::Assign(assign) => self.process_assign(assign),
            Expr::Closure(closure) => self.process_closure(closure),
            // Match expression - multi-way branch
            Expr::Match(expr_match) => self.process_match(expr_match),
            Expr::MethodCall(method_call) => {
                // Check for closures in method call arguments
                self.process_closures_in_expr(expr);
                // Also create a statement for the method call itself
                let receiver = self
                    .extract_primary_var(&method_call.receiver)
                    .unwrap_or_else(|| self.get_or_create_var("_receiver"));
                let args = method_call
                    .args
                    .iter()
                    .filter_map(|arg| self.extract_primary_var(arg))
                    .collect();
                self.current_block.push(Statement::Expr {
                    expr: ExprKind::MethodCall {
                        receiver,
                        method: method_call.method.to_string(),
                        args,
                    },
                    line: None,
                });
            }
            Expr::Call(call) => {
                // Process any closures in function arguments
                for arg in &call.args {
                    self.process_closures_in_expr(arg);
                }
                self.current_block.push(Statement::Expr {
                    expr: ExprKind::Other,
                    line: None,
                });
            }
            _ => {
                // Process any closures that might be nested in this expression
                self.process_closures_in_expr(expr);
                self.current_block.push(Statement::Expr {
                    expr: ExprKind::Other,
                    line: None,
                });
            }
        }
    }

    /// Process any closures found in an expression (for nested closures in method chains)
    fn process_closures_in_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Closure(closure) => self.process_closure(closure),
            Expr::MethodCall(method_call) => {
                // Check receiver for closures
                self.process_closures_in_expr(&method_call.receiver);
                // Check all arguments for closures
                for arg in &method_call.args {
                    self.process_closures_in_expr(arg);
                }
            }
            Expr::Call(call) => {
                // Check function expression
                self.process_closures_in_expr(&call.func);
                // Check all arguments for closures
                for arg in &call.args {
                    self.process_closures_in_expr(arg);
                }
            }
            _ => {}
        }
    }

    fn process_if(&mut self, expr_if: &ExprIf) {
        // Extract actual condition variable(s)
        let condition = self
            .extract_primary_var(&expr_if.cond)
            .unwrap_or_else(|| self.get_or_create_var("_cond"));

        let then_block = BlockId(self.block_counter + 1);
        let else_block = BlockId(self.block_counter + 2);

        self.finalize_current_block(Terminator::Branch {
            condition,
            then_block,
            else_block,
        });
    }

    fn process_while(&mut self, _expr_while: &ExprWhile) {
        let loop_head = BlockId(self.block_counter + 1);
        self.finalize_current_block(Terminator::Goto { target: loop_head });
    }

    /// Process a match expression, creating proper CFG structure.
    ///
    /// This creates:
    /// 1. A block ending with Match terminator that branches to arm blocks
    /// 2. One block per arm for pattern bindings and arm body
    /// 3. A join block where all arms converge
    fn process_match(&mut self, expr_match: &ExprMatch) {
        // Step 1: Process scrutinee expression and get its variable
        let scrutinee_var = self.process_scrutinee(&expr_match.expr);

        // Step 2: Calculate block IDs for the CFG structure
        // Current block will end with Match terminator
        // Then we have: arm blocks + join block
        let arm_count = expr_match.arms.len();
        let arm_start_id = self.block_counter + 1;
        let join_block_id = BlockId(arm_start_id + arm_count);

        // Step 3: Build match arms metadata (blocks IDs determined, but content later)
        let mut cfg_arms = Vec::with_capacity(arm_count);
        for i in 0..arm_count {
            cfg_arms.push(MatchArm {
                block: BlockId(arm_start_id + i),
                guard: None,          // Will be updated during arm processing if present
                bindings: Vec::new(), // Will be filled during arm processing
            });
        }

        // Step 4: Finalize current block with Match terminator
        self.finalize_current_block(Terminator::Match {
            scrutinee: scrutinee_var,
            arms: cfg_arms.clone(),
            join_block: join_block_id,
        });

        // Step 5: Process each arm, creating its block
        for (i, arm) in expr_match.arms.iter().enumerate() {
            self.process_match_arm(arm, scrutinee_var, join_block_id, i);
        }

        // Step 6: Create the join block (empty, will be populated by subsequent code)
        // The join block is implicitly created when we start adding statements
        // after this method returns - the current_block is now the join block
        self.current_block = Vec::new();
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
            line: None,
        });

        temp_var
    }

    /// Process a single match arm, creating its basic block.
    fn process_match_arm(
        &mut self,
        arm: &syn::Arm,
        scrutinee: VarId,
        join_block: BlockId,
        _arm_index: usize,
    ) {
        // Start a new block for this arm
        self.current_block = Vec::new();

        // Step 1: Bind pattern variables from scrutinee
        let bindings = self.bind_pattern_vars(&arm.pat, scrutinee);

        // Step 2: Process guard if present
        let guard_var = if let Some((_, guard_expr)) = &arm.guard {
            Some(self.process_guard(guard_expr))
        } else {
            None
        };

        // Step 3: Process arm body (this may add statements to current_block)
        self.process_expr(&arm.body);

        // Step 4: Record the bindings and guard in a local struct
        // Note: We can't update cfg_arms here since it was moved into the terminator.
        // The bindings are already tracked in the CFG through the Declare statements.
        let _ = (bindings, guard_var);

        // Step 5: Finalize arm block with goto to join block
        self.finalize_current_block(Terminator::Goto { target: join_block });
    }

    /// Bind pattern variables and return their VarIds.
    fn bind_pattern_vars(&mut self, pat: &Pat, scrutinee: VarId) -> Vec<VarId> {
        let binding_names = self.extract_vars_from_pattern(pat);

        for (i, var) in binding_names.iter().enumerate() {
            // For each bound variable, create a declaration statement
            // The initialization represents the field/element access from scrutinee
            let init = if i == 0 {
                // First/only binding gets direct access
                Rvalue::Use(scrutinee)
            } else {
                // Additional bindings get field access (simplified)
                Rvalue::FieldAccess {
                    base: scrutinee,
                    field: i.to_string(),
                }
            };

            self.current_block.push(Statement::Declare {
                var: *var,
                init: Some(init),
                line: None,
            });
        }

        binding_names
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
            line: None,
        });

        guard_var
    }

    fn process_return(&mut self, expr_return: &ExprReturn) {
        // Extract actual returned variable
        let value = expr_return
            .expr
            .as_ref()
            .and_then(|e| self.extract_primary_var(e));

        self.finalize_current_block(Terminator::Return { value });
    }

    fn process_assign(&mut self, assign: &ExprAssign) {
        // Extract actual target variable
        let target = self
            .extract_primary_var(&assign.left)
            .unwrap_or_else(|| self.get_or_create_var("_unknown"));

        // Convert RHS to proper Rvalue
        let source = self.expr_to_rvalue(&assign.right);

        self.current_block.push(Statement::Assign {
            target,
            source,
            line: None,
        });
    }

    fn get_or_create_var(&mut self, name: &str) -> VarId {
        let len = self.var_names.len();
        let name_id = *self
            .var_names
            .entry(name.to_string())
            .or_insert_with(|| len as u32);
        let version = *self.var_versions.entry(name_id).or_insert(0);
        VarId { name_id, version }
    }

    /// Get current scope variables for capture detection.
    fn current_scope_vars(&self) -> HashSet<String> {
        self.var_names.keys().cloned().collect()
    }

    /// Process a closure expression, extracting captures and body information.
    fn process_closure(&mut self, closure: &ExprClosure) {
        // Step 1: Record outer scope variables before entering closure
        let outer_scope_vars = self.current_scope_vars();

        // Step 2: Create closure parameter scope
        let mut closure_params: HashSet<String> = HashSet::new();
        for input in &closure.inputs {
            if let Pat::Ident(pat_ident) = input {
                let param_name = pat_ident.ident.to_string();
                closure_params.insert(param_name);
                // Don't add to main var_names - these are closure-local
            }
        }

        // Step 3: Visit closure body to find captures
        let is_move = closure.capture.is_some();
        let mut capture_visitor =
            ClosureCaptureVisitor::new(&outer_scope_vars, &closure_params, is_move);
        capture_visitor.visit_expr(&closure.body);

        // Step 4: Finalize and record captured variables
        let captures = capture_visitor.finalize_captures();

        let capture_var_ids: Vec<VarId> = captures
            .iter()
            .map(|c| {
                let var_id = self.get_or_create_var(&c.var_name);
                // Also record in captured_vars for later analysis
                self.captured_vars.push(CapturedVar {
                    var_id,
                    capture_mode: c.mode,
                    is_mutated: c.is_mutated,
                });
                var_id
            })
            .collect();

        // Step 5: Emit closure expression statement
        self.current_block.push(Statement::Expr {
            expr: ExprKind::Closure {
                captures: capture_var_ids,
                is_move,
            },
            line: None,
        });
    }

    /// Extract all variables referenced in an expression.
    /// Returns a list of VarIds for variables that appear in the expression.
    fn extract_vars_from_expr(&mut self, expr: &Expr) -> Vec<VarId> {
        match expr {
            // Path: x, foo::bar
            Expr::Path(path) => {
                if let Some(ident) = path.path.get_ident() {
                    vec![self.get_or_create_var(&ident.to_string())]
                } else if let Some(seg) = path.path.segments.last() {
                    vec![self.get_or_create_var(&seg.ident.to_string())]
                } else {
                    vec![]
                }
            }

            // Field access: x.field, x.y.z
            Expr::Field(field) => self.extract_vars_from_expr(&field.base),

            // Method call: receiver.method(args)
            Expr::MethodCall(method) => {
                let mut vars = self.extract_vars_from_expr(&method.receiver);
                for arg in &method.args {
                    vars.extend(self.extract_vars_from_expr(arg));
                }
                vars
            }

            // Binary: a + b, x && y
            Expr::Binary(binary) => {
                let mut vars = self.extract_vars_from_expr(&binary.left);
                vars.extend(self.extract_vars_from_expr(&binary.right));
                vars
            }

            // Unary: !x, *ptr, -n
            Expr::Unary(unary) => self.extract_vars_from_expr(&unary.expr),

            // Index: arr[i]
            Expr::Index(index) => {
                let mut vars = self.extract_vars_from_expr(&index.expr);
                vars.extend(self.extract_vars_from_expr(&index.index));
                vars
            }

            // Call: f(a, b, c)
            Expr::Call(call) => {
                let mut vars = self.extract_vars_from_expr(&call.func);
                for arg in &call.args {
                    vars.extend(self.extract_vars_from_expr(arg));
                }
                vars
            }

            // Reference: &x, &mut x
            Expr::Reference(reference) => self.extract_vars_from_expr(&reference.expr),

            // Paren: (expr)
            Expr::Paren(paren) => self.extract_vars_from_expr(&paren.expr),

            // Block: { expr }
            Expr::Block(block) => block
                .block
                .stmts
                .last()
                .and_then(|stmt| {
                    if let Stmt::Expr(expr, _) = stmt {
                        Some(self.extract_vars_from_expr(expr))
                    } else {
                        None
                    }
                })
                .unwrap_or_default(),

            // Tuple: (a, b, c)
            Expr::Tuple(tuple) => tuple
                .elems
                .iter()
                .flat_map(|e| self.extract_vars_from_expr(e))
                .collect(),

            // Cast: x as T
            Expr::Cast(cast) => self.extract_vars_from_expr(&cast.expr),

            // Array: [a, b, c]
            Expr::Array(array) => array
                .elems
                .iter()
                .flat_map(|e| self.extract_vars_from_expr(e))
                .collect(),

            // Repeat: [x; N]
            Expr::Repeat(repeat) => self.extract_vars_from_expr(&repeat.expr),

            // Struct: Foo { field: value }
            Expr::Struct(expr_struct) => expr_struct
                .fields
                .iter()
                .flat_map(|f| self.extract_vars_from_expr(&f.expr))
                .collect(),

            // Range: a..b, a..=b
            Expr::Range(range) => {
                let mut vars = Vec::new();
                if let Some(start) = &range.start {
                    vars.extend(self.extract_vars_from_expr(start));
                }
                if let Some(end) = &range.end {
                    vars.extend(self.extract_vars_from_expr(end));
                }
                vars
            }

            // Try: expr?
            Expr::Try(try_expr) => self.extract_vars_from_expr(&try_expr.expr),

            // Await: expr.await
            Expr::Await(await_expr) => self.extract_vars_from_expr(&await_expr.base),

            // Literals and other non-variable expressions
            Expr::Lit(_) => vec![],

            // Default: return empty for unsupported expressions
            _ => vec![],
        }
    }

    /// Extract the primary variable from an expression (for assignment targets, returns).
    /// Returns the first/main variable, or None if expression has no variable.
    fn extract_primary_var(&mut self, expr: &Expr) -> Option<VarId> {
        self.extract_vars_from_expr(expr).into_iter().next()
    }

    /// Extract variable bindings from a pattern.
    fn extract_vars_from_pattern(&mut self, pat: &Pat) -> Vec<VarId> {
        match pat {
            // Simple identifier: let x = ...
            Pat::Ident(pat_ident) => {
                vec![self.get_or_create_var(&pat_ident.ident.to_string())]
            }

            // Tuple: let (a, b) = ...
            Pat::Tuple(tuple) => tuple
                .elems
                .iter()
                .flat_map(|p| self.extract_vars_from_pattern(p))
                .collect(),

            // Struct: let Point { x, y } = ...
            Pat::Struct(pat_struct) => pat_struct
                .fields
                .iter()
                .flat_map(|field| self.extract_vars_from_pattern(&field.pat))
                .collect(),

            // TupleStruct: let Some(x) = ...
            Pat::TupleStruct(tuple_struct) => tuple_struct
                .elems
                .iter()
                .flat_map(|p| self.extract_vars_from_pattern(p))
                .collect(),

            // Slice: let [first, rest @ ..] = ...
            Pat::Slice(slice) => slice
                .elems
                .iter()
                .flat_map(|p| self.extract_vars_from_pattern(p))
                .collect(),

            // Reference: let &x = ... or let &mut x = ...
            Pat::Reference(reference) => self.extract_vars_from_pattern(&reference.pat),

            // Or: let A | B = ...
            Pat::Or(or) => or
                .cases
                .first()
                .map(|p| self.extract_vars_from_pattern(p))
                .unwrap_or_default(),

            // Type: let x: T = ...
            Pat::Type(pat_type) => self.extract_vars_from_pattern(&pat_type.pat),

            // Wildcard: let _ = ...
            Pat::Wild(_) => vec![],

            // Literal patterns: match on literal
            Pat::Lit(_) => vec![],

            // Rest: ..
            Pat::Rest(_) => vec![],

            // Range pattern: 1..=10
            Pat::Range(_) => vec![],

            // Path pattern: None, MyEnum::Variant
            Pat::Path(_) => vec![],

            // Const pattern
            Pat::Const(_) => vec![],

            // Paren pattern: (pat)
            Pat::Paren(paren) => self.extract_vars_from_pattern(&paren.pat),

            _ => vec![],
        }
    }

    /// Convert an expression to an Rvalue, extracting actual variables.
    fn expr_to_rvalue(&mut self, expr: &Expr) -> Rvalue {
        match expr {
            // Simple variable use
            Expr::Path(path) => {
                if let Some(var) = self.extract_primary_var(&Expr::Path(path.clone())) {
                    Rvalue::Use(var)
                } else {
                    Rvalue::Constant
                }
            }

            // Binary operation
            Expr::Binary(binary) => {
                let left = self.extract_primary_var(&binary.left);
                let right = self.extract_primary_var(&binary.right);

                if let (Some(l), Some(r)) = (left, right) {
                    Rvalue::BinaryOp {
                        op: Self::convert_bin_op(&binary.op),
                        left: l,
                        right: r,
                    }
                } else if let Some(l) = left {
                    // Right side is constant
                    Rvalue::Use(l)
                } else if let Some(r) = right {
                    // Left side is constant
                    Rvalue::Use(r)
                } else {
                    Rvalue::Constant
                }
            }

            // Unary operation
            Expr::Unary(unary) => {
                if let Some(operand) = self.extract_primary_var(&unary.expr) {
                    Rvalue::UnaryOp {
                        op: Self::convert_un_op(&unary.op),
                        operand,
                    }
                } else {
                    Rvalue::Constant
                }
            }

            // Field access
            Expr::Field(field) => {
                if let Some(base) = self.extract_primary_var(&field.base) {
                    let field_name = match &field.member {
                        syn::Member::Named(ident) => ident.to_string(),
                        syn::Member::Unnamed(index) => index.index.to_string(),
                    };
                    Rvalue::FieldAccess {
                        base,
                        field: field_name,
                    }
                } else {
                    Rvalue::Constant
                }
            }

            // Reference
            Expr::Reference(reference) => {
                if let Some(var) = self.extract_primary_var(&reference.expr) {
                    Rvalue::Ref {
                        var,
                        mutable: reference.mutability.is_some(),
                    }
                } else {
                    Rvalue::Constant
                }
            }

            // Function call
            Expr::Call(call) => {
                let func_name = Self::extract_func_name(&call.func);
                let args = call
                    .args
                    .iter()
                    .filter_map(|arg| self.extract_primary_var(arg))
                    .collect();
                Rvalue::Call {
                    func: func_name,
                    args,
                }
            }

            // Method call
            Expr::MethodCall(method) => {
                let func_name = method.method.to_string();
                let mut args = vec![];
                if let Some(recv) = self.extract_primary_var(&method.receiver) {
                    args.push(recv);
                }
                args.extend(
                    method
                        .args
                        .iter()
                        .filter_map(|a| self.extract_primary_var(a)),
                );
                Rvalue::Call {
                    func: func_name,
                    args,
                }
            }

            // Paren: (expr) - unwrap
            Expr::Paren(paren) => self.expr_to_rvalue(&paren.expr),

            // Cast: x as T - preserve the variable
            Expr::Cast(cast) => self.expr_to_rvalue(&cast.expr),

            // Block: { expr } - use final expression
            Expr::Block(block) => {
                if let Some(Stmt::Expr(expr, _)) = block.block.stmts.last() {
                    self.expr_to_rvalue(expr)
                } else {
                    Rvalue::Constant
                }
            }

            // Index: arr[i]
            Expr::Index(index) => {
                if let Some(base) = self.extract_primary_var(&index.expr) {
                    // Treat as field access with index as field name
                    Rvalue::FieldAccess {
                        base,
                        field: "[index]".to_string(),
                    }
                } else {
                    Rvalue::Constant
                }
            }

            // Literals and other constant expressions
            Expr::Lit(_) => Rvalue::Constant,

            // Default fallback
            _ => Rvalue::Constant,
        }
    }

    fn extract_func_name(func: &Expr) -> String {
        match func {
            Expr::Path(path) => path
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::"),
            _ => "unknown".to_string(),
        }
    }

    fn convert_bin_op(op: &syn::BinOp) -> BinOp {
        match op {
            syn::BinOp::Add(_) | syn::BinOp::AddAssign(_) => BinOp::Add,
            syn::BinOp::Sub(_) | syn::BinOp::SubAssign(_) => BinOp::Sub,
            syn::BinOp::Mul(_) | syn::BinOp::MulAssign(_) => BinOp::Mul,
            syn::BinOp::Div(_) | syn::BinOp::DivAssign(_) => BinOp::Div,
            syn::BinOp::Eq(_) => BinOp::Eq,
            syn::BinOp::Ne(_) => BinOp::Ne,
            syn::BinOp::Lt(_) => BinOp::Lt,
            syn::BinOp::Gt(_) => BinOp::Gt,
            syn::BinOp::Le(_) => BinOp::Le,
            syn::BinOp::Ge(_) => BinOp::Ge,
            syn::BinOp::And(_) => BinOp::And,
            syn::BinOp::Or(_) => BinOp::Or,
            _ => BinOp::Add, // Fallback for bitwise ops, rem, shl, shr
        }
    }

    fn convert_un_op(op: &syn::UnOp) -> UnOp {
        match op {
            syn::UnOp::Neg(_) => UnOp::Neg,
            syn::UnOp::Not(_) => UnOp::Not,
            syn::UnOp::Deref(_) => UnOp::Deref,
            _ => UnOp::Not, // Fallback for unknown ops
        }
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

    pub(crate) fn finalize(mut self) -> ControlFlowGraph {
        if !self.current_block.is_empty() {
            self.finalize_current_block(Terminator::Return { value: None });
        }

        let exit_blocks = self
            .blocks
            .iter()
            .filter(|b| matches!(b.terminator, Terminator::Return { .. }))
            .map(|b| b.id)
            .collect();

        let var_names = {
            let mut names = vec![String::new(); self.var_names.len()];
            for (name, id) in self.var_names {
                names[id as usize] = name;
            }
            names
        };

        ControlFlowGraph {
            blocks: self.blocks,
            entry_block: BlockId(0),
            exit_blocks,
            edges: self.edges,
            var_names,
            captured_vars: self.captured_vars,
        }
    }
}

// ============================================================================
// Closure Capture Visitor
// ============================================================================

/// Information about a capture detected during closure body analysis.
#[derive(Debug, Clone)]
struct CaptureInfo {
    /// Name of the captured variable
    var_name: String,
    /// Inferred capture mode
    mode: CaptureMode,
    /// Whether the variable is mutated in the closure body
    is_mutated: bool,
}

/// Visitor to detect captured variables in closure body.
///
/// Walks the closure body AST and identifies variables that:
/// 1. Are referenced in the closure body
/// 2. Are defined in the outer scope (not closure parameters)
/// 3. Are not special names like `self` or `Self`
struct ClosureCaptureVisitor<'a> {
    /// Variables available in outer scope (potential captures)
    outer_scope: &'a HashSet<String>,
    /// Closure parameters (not captures)
    closure_params: &'a HashSet<String>,
    /// Detected captures
    captures: Vec<CaptureInfo>,
    /// Variables mutated in closure body
    mutated_vars: HashSet<String>,
    /// Whether this is a move closure
    is_move: bool,
}

impl<'a> ClosureCaptureVisitor<'a> {
    fn new(
        outer_scope: &'a HashSet<String>,
        closure_params: &'a HashSet<String>,
        is_move: bool,
    ) -> Self {
        Self {
            outer_scope,
            closure_params,
            captures: Vec::new(),
            mutated_vars: HashSet::new(),
            is_move,
        }
    }

    /// Finalize capture detection by updating capture modes based on mutation info.
    fn finalize_captures(mut self) -> Vec<CaptureInfo> {
        for capture in &mut self.captures {
            if self.mutated_vars.contains(&capture.var_name) {
                capture.is_mutated = true;
                if !self.is_move {
                    capture.mode = CaptureMode::ByMutRef;
                }
            }
        }
        self.captures
    }
}

impl<'ast, 'a> Visit<'ast> for ClosureCaptureVisitor<'a> {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            // Variable reference - potential capture
            Expr::Path(path) => {
                if let Some(ident) = path.path.get_ident() {
                    let name = ident.to_string();
                    // Skip special names
                    if name == "self" || name == "Self" {
                        return;
                    }
                    // Check if it's from outer scope (not a closure param)
                    if self.outer_scope.contains(&name) && !self.closure_params.contains(&name) {
                        // Check if already captured
                        if !self.captures.iter().any(|c| c.var_name == name) {
                            self.captures.push(CaptureInfo {
                                var_name: name,
                                mode: if self.is_move {
                                    CaptureMode::ByValue
                                } else {
                                    CaptureMode::ByRef
                                },
                                is_mutated: false,
                            });
                        }
                    }
                }
            }
            // Method call - check receiver
            Expr::MethodCall(method_call) => {
                // Visit receiver separately to detect captures
                self.visit_expr(&method_call.receiver);
                // Check if method is mutating
                let method_name = method_call.method.to_string();
                if is_mutating_method(&method_name) {
                    if let Expr::Path(path) = &*method_call.receiver {
                        if let Some(ident) = path.path.get_ident() {
                            self.mutated_vars.insert(ident.to_string());
                        }
                    }
                }
                // Visit args
                for arg in &method_call.args {
                    self.visit_expr(arg);
                }
            }
            // Assignment - track mutation
            Expr::Assign(assign) => {
                if let Expr::Path(path) = &*assign.left {
                    if let Some(ident) = path.path.get_ident() {
                        self.mutated_vars.insert(ident.to_string());
                    }
                }
                // Visit RHS
                self.visit_expr(&assign.right);
            }
            // Binary operation that might be compound assignment (+=, -=, etc.)
            Expr::Binary(binary) => {
                // Check if it's a compound assignment
                let is_assignment_op = matches!(
                    binary.op,
                    syn::BinOp::AddAssign(_)
                        | syn::BinOp::SubAssign(_)
                        | syn::BinOp::MulAssign(_)
                        | syn::BinOp::DivAssign(_)
                        | syn::BinOp::RemAssign(_)
                        | syn::BinOp::BitAndAssign(_)
                        | syn::BinOp::BitOrAssign(_)
                        | syn::BinOp::BitXorAssign(_)
                        | syn::BinOp::ShlAssign(_)
                        | syn::BinOp::ShrAssign(_)
                );
                if is_assignment_op {
                    if let Expr::Path(path) = &*binary.left {
                        if let Some(ident) = path.path.get_ident() {
                            self.mutated_vars.insert(ident.to_string());
                        }
                    }
                }
                self.visit_expr(&binary.left);
                self.visit_expr(&binary.right);
            }
            // Nested closure - recurse with combined scope
            Expr::Closure(nested_closure) => {
                // Extract nested closure params
                let nested_params: HashSet<String> = nested_closure
                    .inputs
                    .iter()
                    .filter_map(extract_pattern_name)
                    .collect();

                let nested_is_move = nested_closure.capture.is_some();
                let mut nested_visitor =
                    ClosureCaptureVisitor::new(self.outer_scope, &nested_params, nested_is_move);
                nested_visitor.visit_expr(&nested_closure.body);

                // Propagate captures from nested closure
                for capture in nested_visitor.finalize_captures() {
                    if !self.captures.iter().any(|c| c.var_name == capture.var_name) {
                        self.captures.push(capture);
                    }
                }
            }
            // Default: recurse into children
            _ => {
                syn::visit::visit_expr(self, expr);
            }
        }
    }
}

/// Check if a method name indicates mutation.
fn is_mutating_method(name: &str) -> bool {
    matches!(
        name,
        "push"
            | "pop"
            | "insert"
            | "remove"
            | "clear"
            | "extend"
            | "drain"
            | "append"
            | "truncate"
            | "reserve"
            | "shrink_to_fit"
            | "set"
            | "swap"
            | "sort"
            | "sort_by"
            | "sort_by_key"
            | "dedup"
            | "retain"
            | "resize"
    )
}

/// Extract the variable name from a pattern.
fn extract_pattern_name(pat: &Pat) -> Option<String> {
    match pat {
        Pat::Ident(pat_ident) => Some(pat_ident.ident.to_string()),
        Pat::Type(pat_type) => extract_pattern_name(&pat_type.pat),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_cfg_construction_simple() {
        let block: Block = parse_quote! {
            {
                let x = 1;
                let y = x + 1;
                y
            }
        };

        let graph = ControlFlowGraph::from_block(&block);
        assert!(!graph.blocks.is_empty());
    }

    #[test]
    fn test_extract_simple_path() {
        let block: Block = parse_quote! {
            {
                let x = y;
            }
        };

        let graph = ControlFlowGraph::from_block(&block);
        assert!(graph.var_names.contains(&"x".to_string()));
        assert!(graph.var_names.contains(&"y".to_string()));
    }

    #[test]
    fn test_extract_binary_op() {
        let block: Block = parse_quote! {
            {
                let result = a + b;
            }
        };

        let graph = ControlFlowGraph::from_block(&block);
        assert!(graph.var_names.contains(&"result".to_string()));
        assert!(graph.var_names.contains(&"a".to_string()));
        assert!(graph.var_names.contains(&"b".to_string()));
    }

    #[test]
    fn test_extract_field_access() {
        let block: Block = parse_quote! {
            {
                let x = point.field;
            }
        };

        let graph = ControlFlowGraph::from_block(&block);
        assert!(graph.var_names.contains(&"x".to_string()));
        assert!(graph.var_names.contains(&"point".to_string()));
    }

    #[test]
    fn test_tuple_pattern() {
        let block: Block = parse_quote! {
            {
                let (a, b, c) = tuple;
            }
        };

        let graph = ControlFlowGraph::from_block(&block);
        assert!(graph.var_names.contains(&"a".to_string()));
        assert!(graph.var_names.contains(&"b".to_string()));
        assert!(graph.var_names.contains(&"c".to_string()));
        assert!(graph.var_names.contains(&"tuple".to_string()));
    }

    #[test]
    fn test_struct_pattern() {
        let block: Block = parse_quote! {
            {
                let Point { x, y } = point;
            }
        };

        let graph = ControlFlowGraph::from_block(&block);
        assert!(graph.var_names.contains(&"x".to_string()));
        assert!(graph.var_names.contains(&"y".to_string()));
        assert!(graph.var_names.contains(&"point".to_string()));
    }

    #[test]
    fn test_assignment_tracks_actual_variables() {
        let block: Block = parse_quote! {
            {
                let mut x = 0;
                x = y + z;
            }
        };

        let graph = ControlFlowGraph::from_block(&block);
        assert!(graph.var_names.contains(&"x".to_string()));
        assert!(graph.var_names.contains(&"y".to_string()));
        assert!(graph.var_names.contains(&"z".to_string()));
        assert!(!graph.var_names.contains(&"_temp".to_string()));
    }

    #[test]
    fn test_return_with_variable() {
        let block: Block = parse_quote! {
            {
                let x = 1;
                return x;
            }
        };

        let graph = ControlFlowGraph::from_block(&block);

        let exit_block = graph
            .blocks
            .iter()
            .find(|b| matches!(b.terminator, Terminator::Return { .. }));

        assert!(exit_block.is_some());
        if let Some(block) = exit_block {
            if let Terminator::Return { value } = &block.terminator {
                assert!(value.is_some(), "Return should track actual variable");
            }
        }
    }

    #[test]
    fn test_if_condition_tracks_variable() {
        let block: Block = parse_quote! {
            {
                if flag {
                    1
                } else {
                    2
                }
            }
        };

        let graph = ControlFlowGraph::from_block(&block);
        assert!(graph.var_names.contains(&"flag".to_string()));
        assert!(!graph.var_names.contains(&"_temp".to_string()));
    }

    #[test]
    fn test_method_call_extracts_receiver_and_args() {
        let block: Block = parse_quote! {
            {
                let result = receiver.method(arg1, arg2);
            }
        };

        let graph = ControlFlowGraph::from_block(&block);
        assert!(graph.var_names.contains(&"result".to_string()));
        assert!(graph.var_names.contains(&"receiver".to_string()));
        assert!(graph.var_names.contains(&"arg1".to_string()));
        assert!(graph.var_names.contains(&"arg2".to_string()));
    }

    #[test]
    fn test_nested_field_access() {
        let block: Block = parse_quote! {
            {
                let z = x.y.z;
            }
        };

        let graph = ControlFlowGraph::from_block(&block);
        assert!(graph.var_names.contains(&"z".to_string()));
        assert!(graph.var_names.contains(&"x".to_string()));
    }

    #[test]
    fn test_function_call_extracts_args() {
        let block: Block = parse_quote! {
            {
                let result = compute(a, b, c);
            }
        };

        let graph = ControlFlowGraph::from_block(&block);
        assert!(graph.var_names.contains(&"result".to_string()));
        assert!(graph.var_names.contains(&"a".to_string()));
        assert!(graph.var_names.contains(&"b".to_string()));
        assert!(graph.var_names.contains(&"c".to_string()));
    }

    #[test]
    fn test_rvalue_binary_op_structure() {
        let block: Block = parse_quote! {
            {
                let sum = x + y;
            }
        };

        let graph = ControlFlowGraph::from_block(&block);

        let decl_stmt = graph
            .blocks
            .iter()
            .flat_map(|b| &b.statements)
            .find(|s| matches!(s, Statement::Declare { .. }));

        assert!(decl_stmt.is_some());
        if let Some(Statement::Declare {
            init: Some(rvalue), ..
        }) = decl_stmt
        {
            assert!(
                matches!(rvalue, Rvalue::BinaryOp { .. }),
                "Expected BinaryOp, got {:?}",
                rvalue
            );
        }
    }

    #[test]
    fn test_rvalue_field_access_structure() {
        let block: Block = parse_quote! {
            {
                let val = obj.field;
            }
        };

        let graph = ControlFlowGraph::from_block(&block);

        let decl_stmt = graph
            .blocks
            .iter()
            .flat_map(|b| &b.statements)
            .find(|s| matches!(s, Statement::Declare { .. }));

        assert!(decl_stmt.is_some());
        if let Some(Statement::Declare {
            init: Some(rvalue), ..
        }) = decl_stmt
        {
            assert!(
                matches!(rvalue, Rvalue::FieldAccess { .. }),
                "Expected FieldAccess, got {:?}",
                rvalue
            );
        }
    }

    #[test]
    fn test_slice_pattern() {
        let block: Block = parse_quote! {
            {
                let [first, second] = arr;
            }
        };

        let graph = ControlFlowGraph::from_block(&block);
        assert!(graph.var_names.contains(&"first".to_string()));
        assert!(graph.var_names.contains(&"second".to_string()));
    }

    #[test]
    fn test_tuple_struct_pattern() {
        let block: Block = parse_quote! {
            {
                let Some(value) = option;
            }
        };

        let graph = ControlFlowGraph::from_block(&block);
        assert!(graph.var_names.contains(&"value".to_string()));
    }

    // ==========================================================================
    // Match Expression CFG Tests
    // ==========================================================================

    #[test]
    fn test_simple_match_cfg_structure() {
        let block: Block = parse_quote! {
            {
                let x = 1;
                match x {
                    1 => {},
                    _ => {},
                }
            }
        };

        let graph = ControlFlowGraph::from_block(&block);

        assert!(
            graph.blocks.len() >= 3,
            "Expected at least 3 blocks, got {}",
            graph.blocks.len()
        );

        let match_term = graph
            .blocks
            .iter()
            .find(|b| matches!(b.terminator, Terminator::Match { .. }));
        assert!(match_term.is_some(), "Should have Match terminator");

        if let Terminator::Match { arms, .. } = &match_term.unwrap().terminator {
            assert_eq!(arms.len(), 2, "Should have 2 arms");
        }
    }

    #[test]
    fn test_match_pattern_bindings() {
        let block: Block = parse_quote! {
            {
                let result = some_result();
                match result {
                    Ok(value) => value,
                    Err(e) => 0,
                }
            }
        };

        let graph = ControlFlowGraph::from_block(&block);

        assert!(
            graph.var_names.contains(&"value".to_string()),
            "Should track 'value'"
        );
        assert!(
            graph.var_names.contains(&"e".to_string()),
            "Should track 'e'"
        );
    }

    #[test]
    fn test_match_with_guard() {
        let block: Block = parse_quote! {
            {
                let x = get_number();
                match x {
                    n if n > 0 => n,
                    _ => 0,
                }
            }
        };

        let graph = ControlFlowGraph::from_block(&block);

        assert!(!graph.blocks.is_empty());

        let match_term = graph
            .blocks
            .iter()
            .find(|b| matches!(b.terminator, Terminator::Match { .. }));
        assert!(match_term.is_some(), "Should have Match terminator");
    }

    #[test]
    fn test_match_scrutinee_tracking() {
        let block: Block = parse_quote! {
            {
                let input = get_input();
                match input {
                    Some(x) => x,
                    None => 0,
                }
            }
        };

        let graph = ControlFlowGraph::from_block(&block);

        assert!(
            graph.var_names.contains(&"input".to_string()),
            "Should track scrutinee 'input'"
        );

        if let Some(block) = graph
            .blocks
            .iter()
            .find(|b| matches!(b.terminator, Terminator::Match { .. }))
        {
            if let Terminator::Match { scrutinee, .. } = &block.terminator {
                let name = graph.var_names.get(scrutinee.name_id as usize);
                assert!(name.is_some(), "Scrutinee should have a valid name");
            }
        }
    }

    #[test]
    fn test_match_struct_pattern() {
        let block: Block = parse_quote! {
            {
                let point = get_point();
                match point {
                    Point { x, y } => x + y,
                }
            }
        };

        let graph = ControlFlowGraph::from_block(&block);

        assert!(
            graph.var_names.contains(&"x".to_string()),
            "Should track 'x' from struct pattern"
        );
        assert!(
            graph.var_names.contains(&"y".to_string()),
            "Should track 'y' from struct pattern"
        );
    }

    #[test]
    fn test_match_data_flow() {
        let block: Block = parse_quote! {
            {
                let x = get_value();
                let y = get_other();
                match x {
                    Some(v) => v + y,
                    None => y,
                }
            }
        };

        let _graph = ControlFlowGraph::from_block(&block);
        // Analysis should complete without panicking
    }

    #[test]
    fn test_match_successors() {
        let block: Block = parse_quote! {
            {
                match x {
                    A => 1,
                    B => 2,
                    C => 3,
                }
            }
        };

        let graph = ControlFlowGraph::from_block(&block);

        let match_block = graph
            .blocks
            .iter()
            .find(|b| matches!(b.terminator, Terminator::Match { .. }));

        if let Some(block) = match_block {
            if let Terminator::Match { arms, .. } = &block.terminator {
                assert!(
                    arms.len() >= 3,
                    "Match should have at least 3 arms, got {}",
                    arms.len()
                );
            }
        }
    }

    #[test]
    fn test_nested_match() {
        let block: Block = parse_quote! {
            {
                let outer = get_outer();
                match outer {
                    Some(inner) => match inner {
                        Ok(v) => v,
                        Err(_) => -1,
                    },
                    None => 0,
                }
            }
        };

        let graph = ControlFlowGraph::from_block(&block);

        let match_count = graph
            .blocks
            .iter()
            .filter(|b| matches!(b.terminator, Terminator::Match { .. }))
            .count();
        assert!(
            match_count >= 1,
            "Should have at least one Match terminator"
        );
    }

    #[test]
    fn test_match_data_flow_analysis() {
        let block: Block = parse_quote! {
            {
                let opt = get_option();
                match opt {
                    Some(x) => x + 1,
                    None => 0,
                }
            }
        };

        let graph = ControlFlowGraph::from_block(&block);
        assert!(!graph.blocks.is_empty());
    }

    #[test]
    fn test_match_tuple_pattern() {
        let block: Block = parse_quote! {
            {
                let pair = get_pair();
                match pair {
                    (a, b) => a + b,
                }
            }
        };

        let graph = ControlFlowGraph::from_block(&block);

        assert!(
            graph.var_names.contains(&"a".to_string()),
            "Should track 'a' from tuple pattern"
        );
        assert!(
            graph.var_names.contains(&"b".to_string()),
            "Should track 'b' from tuple pattern"
        );
    }

    #[test]
    fn test_match_cfg_performance() {
        use std::time::Instant;

        let block: Block = parse_quote! {
            {
                match value {
                    A(x) => x,
                    B(y) => y,
                    C(z) => z,
                    D { a, b } => a + b,
                    E(v) if v > 0 => v,
                    _ => 0,
                }
            }
        };

        let start = Instant::now();
        for _ in 0..100 {
            let graph = ControlFlowGraph::from_block(&block);
            let _ = super::super::reaching_definitions::DataFlowAnalysis::analyze(&graph);
        }
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() < 500,
            "Performance test failed: took {:?}",
            elapsed
        );
    }
}
