//! Control flow graph construction from Rust AST.
//!
//! This module transforms syn AST into a control flow graph suitable
//! for data flow analysis. Handles all Rust statement and expression types.
//!
//! # Module Structure
//!
//! - [`closure`] - Closure capture analysis
//! - [`control_flow`] - Control flow processing (if, while, match, etc.)
//! - [`extraction`] - Variable extraction from expressions and patterns
//! - [`rvalue`] - Expression to Rvalue conversion
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

mod closure;
mod control_flow;
mod extraction;
mod rvalue;

use std::collections::{HashMap, HashSet};

use syn::{Block, Expr, Local, Stmt};

use super::types::{
    BasicBlock, BlockId, CapturedVar, ControlFlowGraph, ExprKind, Statement, Terminator, VarId,
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
    edges: HashMap<BlockId, Vec<(BlockId, super::types::Edge)>>,
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

    /// Process all statements in a block.
    pub(crate) fn process_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.process_stmt(stmt);
        }
    }

    /// Process a single statement.
    fn process_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Local(local) => self.process_local(local),
            Stmt::Expr(expr, _) => self.process_expr(expr),
            _ => {}
        }
    }

    /// Process a local variable declaration.
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

    /// Process an expression, handling control flow and other constructs.
    pub(super) fn process_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::If(expr_if) => self.process_if(expr_if),
            Expr::While(expr_while) => self.process_while(expr_while),
            Expr::Return(expr_return) => self.process_return(expr_return),
            Expr::Assign(assign) => self.process_assign(assign),
            Expr::Closure(closure) => self.process_closure(closure),
            Expr::Match(expr_match) => self.process_match(expr_match),
            Expr::MethodCall(method_call) => self.process_method_call(method_call),
            Expr::Call(call) => self.process_call(call),
            _ => self.process_other_expr(expr),
        }
    }

    /// Process a method call expression.
    fn process_method_call(&mut self, method_call: &syn::ExprMethodCall) {
        // Check for closures in method call arguments
        self.process_closures_in_expr(&Expr::MethodCall(method_call.clone()));

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

    /// Process a function call expression.
    fn process_call(&mut self, call: &syn::ExprCall) {
        for arg in &call.args {
            self.process_closures_in_expr(arg);
        }
        self.current_block.push(Statement::Expr {
            expr: ExprKind::Other,
            line: None,
        });
    }

    /// Process other expressions that might contain closures.
    fn process_other_expr(&mut self, expr: &Expr) {
        self.process_closures_in_expr(expr);
        self.current_block.push(Statement::Expr {
            expr: ExprKind::Other,
            line: None,
        });
    }

    /// Process any closures found in an expression (for nested closures in method chains).
    fn process_closures_in_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Closure(closure) => self.process_closure(closure),
            Expr::MethodCall(method_call) => {
                self.process_closures_in_expr(&method_call.receiver);
                for arg in &method_call.args {
                    self.process_closures_in_expr(arg);
                }
            }
            Expr::Call(call) => {
                self.process_closures_in_expr(&call.func);
                for arg in &call.args {
                    self.process_closures_in_expr(arg);
                }
            }
            _ => {}
        }
    }

    /// Get or create a variable ID for a name.
    pub(super) fn get_or_create_var(&mut self, name: &str) -> VarId {
        let len = self.var_names.len();
        let name_id = *self
            .var_names
            .entry(name.to_string())
            .or_insert_with(|| len as u32);
        let version = *self.var_versions.entry(name_id).or_insert(0);
        VarId { name_id, version }
    }

    /// Get current scope variables for capture detection.
    pub(super) fn current_scope_vars(&self) -> HashSet<String> {
        self.var_names.keys().cloned().collect()
    }

    /// Finalize the current block with a terminator.
    pub(super) fn finalize_current_block(&mut self, terminator: Terminator) {
        let block = BasicBlock {
            id: BlockId(self.block_counter),
            statements: std::mem::take(&mut self.current_block),
            terminator,
        };
        self.blocks.push(block);
        self.block_counter += 1;
    }

    /// Finalize the CFG construction and return the completed graph.
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

#[cfg(test)]
mod tests {
    use super::super::types::Rvalue;
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
