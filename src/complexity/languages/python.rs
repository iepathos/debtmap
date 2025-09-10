use crate::complexity::entropy_core::{LanguageEntropyAnalyzer, PatternMetrics, TokenCategory};
use crate::complexity::entropy_traits::{AnalyzerHelpers, GenericToken};
use rustpython_parser::ast;
use std::collections::HashSet;

/// Python-specific entropy analyzer implementation
pub struct PythonEntropyAnalyzer<'a> {
    _source: &'a str,
}

impl<'a> PythonEntropyAnalyzer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { _source: source }
    }

    /// Extract tokens from Python AST statements
    fn extract_python_tokens(&self, stmts: &[ast::Stmt]) -> Vec<GenericToken> {
        let mut tokens = Vec::new();
        for stmt in stmts {
            self.extract_tokens_from_stmt(stmt, &mut tokens);
        }
        tokens
    }

    /// Extract tokens from a single statement
    fn extract_tokens_from_stmt(&self, stmt: &ast::Stmt, tokens: &mut Vec<GenericToken>) {
        match stmt {
            // Control flow statements
            ast::Stmt::If(if_stmt) => {
                tokens.push(GenericToken::control_flow("if".to_string()));
                self.extract_tokens_from_expr(&if_stmt.test, tokens);
                for s in &if_stmt.body {
                    self.extract_tokens_from_stmt(s, tokens);
                }
                for s in &if_stmt.orelse {
                    self.extract_tokens_from_stmt(s, tokens);
                }
            }
            ast::Stmt::While(while_stmt) => {
                tokens.push(GenericToken::control_flow("while".to_string()));
                self.extract_tokens_from_expr(&while_stmt.test, tokens);
                for s in &while_stmt.body {
                    self.extract_tokens_from_stmt(s, tokens);
                }
            }
            ast::Stmt::For(for_stmt) => {
                tokens.push(GenericToken::control_flow("for".to_string()));
                self.extract_tokens_from_expr(&for_stmt.target, tokens);
                self.extract_tokens_from_expr(&for_stmt.iter, tokens);
                for s in &for_stmt.body {
                    self.extract_tokens_from_stmt(s, tokens);
                }
            }
            ast::Stmt::With(with_stmt) => {
                tokens.push(GenericToken::keyword("with".to_string()));
                for item in &with_stmt.items {
                    self.extract_tokens_from_expr(&item.context_expr, tokens);
                }
                for s in &with_stmt.body {
                    self.extract_tokens_from_stmt(s, tokens);
                }
            }
            ast::Stmt::Match(match_stmt) => {
                tokens.push(GenericToken::control_flow("match".to_string()));
                self.extract_tokens_from_expr(&match_stmt.subject, tokens);
                for case in &match_stmt.cases {
                    // Pattern matching case
                    tokens.push(GenericToken::keyword("case".to_string()));
                    for s in &case.body {
                        self.extract_tokens_from_stmt(s, tokens);
                    }
                }
            }
            ast::Stmt::Try(try_stmt) => {
                tokens.push(GenericToken::keyword("try".to_string()));
                for s in &try_stmt.body {
                    self.extract_tokens_from_stmt(s, tokens);
                }
                for handler in &try_stmt.handlers {
                    tokens.push(GenericToken::keyword("except".to_string()));
                    match handler {
                        ast::ExceptHandler::ExceptHandler(h) => {
                            for s in &h.body {
                                self.extract_tokens_from_stmt(s, tokens);
                            }
                        }
                    }
                }
                if !try_stmt.orelse.is_empty() {
                    tokens.push(GenericToken::keyword("else".to_string()));
                }
                if !try_stmt.finalbody.is_empty() {
                    tokens.push(GenericToken::keyword("finally".to_string()));
                }
            }
            ast::Stmt::Return(return_stmt) => {
                tokens.push(GenericToken::keyword("return".to_string()));
                if let Some(value) = &return_stmt.value {
                    self.extract_tokens_from_expr(value, tokens);
                }
            }
            ast::Stmt::Raise(raise_stmt) => {
                tokens.push(GenericToken::keyword("raise".to_string()));
                if let Some(exc) = &raise_stmt.exc {
                    self.extract_tokens_from_expr(exc, tokens);
                }
            }
            ast::Stmt::Break(_) => tokens.push(GenericToken::keyword("break".to_string())),
            ast::Stmt::Continue(_) => tokens.push(GenericToken::keyword("continue".to_string())),
            ast::Stmt::Pass(_) => tokens.push(GenericToken::keyword("pass".to_string())),
            ast::Stmt::Assert(assert_stmt) => {
                tokens.push(GenericToken::keyword("assert".to_string()));
                self.extract_tokens_from_expr(&assert_stmt.test, tokens);
            }
            ast::Stmt::Global(_) => tokens.push(GenericToken::keyword("global".to_string())),
            ast::Stmt::Nonlocal(_) => tokens.push(GenericToken::keyword("nonlocal".to_string())),
            ast::Stmt::FunctionDef(func_def) => {
                tokens.push(GenericToken::keyword("def".to_string()));
                tokens.push(GenericToken::identifier(normalize_identifier(
                    &func_def.name,
                )));
                for s in &func_def.body {
                    self.extract_tokens_from_stmt(s, tokens);
                }
            }
            ast::Stmt::AsyncFunctionDef(func_def) => {
                tokens.push(GenericToken::keyword("async".to_string()));
                tokens.push(GenericToken::keyword("def".to_string()));
                tokens.push(GenericToken::identifier(normalize_identifier(
                    &func_def.name,
                )));
                for s in &func_def.body {
                    self.extract_tokens_from_stmt(s, tokens);
                }
            }
            ast::Stmt::ClassDef(class_def) => {
                tokens.push(GenericToken::keyword("class".to_string()));
                tokens.push(GenericToken::identifier(normalize_identifier(
                    &class_def.name,
                )));
                for s in &class_def.body {
                    self.extract_tokens_from_stmt(s, tokens);
                }
            }
            ast::Stmt::Expr(expr_stmt) => {
                self.extract_tokens_from_expr(&expr_stmt.value, tokens);
            }
            ast::Stmt::Assign(assign_stmt) => {
                tokens.push(GenericToken::operator("=".to_string()));
                self.extract_tokens_from_expr(&assign_stmt.value, tokens);
                for target in &assign_stmt.targets {
                    self.extract_tokens_from_expr(target, tokens);
                }
            }
            ast::Stmt::AugAssign(aug_assign) => {
                tokens.push(GenericToken::operator(format!("{:?}=", aug_assign.op)));
                self.extract_tokens_from_expr(&aug_assign.target, tokens);
                self.extract_tokens_from_expr(&aug_assign.value, tokens);
            }
            ast::Stmt::AnnAssign(ann_assign) => {
                if let Some(value) = &ann_assign.value {
                    tokens.push(GenericToken::operator("=".to_string()));
                    self.extract_tokens_from_expr(value, tokens);
                }
                self.extract_tokens_from_expr(&ann_assign.target, tokens);
            }
            _ => {}
        }
    }

    /// Extract tokens from expressions
    fn extract_tokens_from_expr(&self, expr: &ast::Expr, tokens: &mut Vec<GenericToken>) {
        match expr {
            ast::Expr::BoolOp(bool_op) => self.extract_bool_op_tokens(bool_op, tokens),
            ast::Expr::BinOp(bin_op) => self.extract_bin_op_tokens(bin_op, tokens),
            ast::Expr::UnaryOp(unary_op) => self.extract_unary_op_tokens(unary_op, tokens),
            ast::Expr::Lambda(lambda) => self.extract_lambda_tokens(lambda, tokens),
            ast::Expr::IfExp(if_exp) => self.extract_if_exp_tokens(if_exp, tokens),
            ast::Expr::ListComp(list_comp) => self.extract_list_comp_tokens(list_comp, tokens),
            ast::Expr::SetComp(set_comp) => self.extract_set_comp_tokens(set_comp, tokens),
            ast::Expr::DictComp(dict_comp) => self.extract_dict_comp_tokens(dict_comp, tokens),
            ast::Expr::GeneratorExp(gen_exp) => self.extract_generator_tokens(gen_exp, tokens),
            ast::Expr::Await(await_expr) => self.extract_await_tokens(await_expr, tokens),
            ast::Expr::Yield(yield_expr) => self.extract_yield_tokens(yield_expr, tokens),
            ast::Expr::YieldFrom(yield_from) => self.extract_yield_from_tokens(yield_from, tokens),
            ast::Expr::Compare(compare) => self.extract_compare_tokens(compare, tokens),
            ast::Expr::Call(call) => self.extract_call_tokens(call, tokens),
            ast::Expr::Name(name) => self.extract_name_tokens(name, tokens),
            ast::Expr::Constant(constant) => self.extract_constant_tokens(constant, tokens),
            ast::Expr::NamedExpr(named) => self.extract_named_expr_tokens(named, tokens),
            _ => {}
        }
    }

    /// Extract tokens from boolean operations (and, or)
    fn extract_bool_op_tokens(&self, bool_op: &ast::ExprBoolOp, tokens: &mut Vec<GenericToken>) {
        let op = match bool_op.op {
            ast::BoolOp::And => "and",
            ast::BoolOp::Or => "or",
        };
        tokens.push(GenericToken::operator(op.to_string()));
        for value in &bool_op.values {
            self.extract_tokens_from_expr(value, tokens);
        }
    }

    /// Extract tokens from binary operations
    fn extract_bin_op_tokens(&self, bin_op: &ast::ExprBinOp, tokens: &mut Vec<GenericToken>) {
        let op = format!("{:?}", bin_op.op);
        tokens.push(GenericToken::operator(op));
        self.extract_tokens_from_expr(&bin_op.left, tokens);
        self.extract_tokens_from_expr(&bin_op.right, tokens);
    }

    /// Extract tokens from unary operations
    fn extract_unary_op_tokens(&self, unary_op: &ast::ExprUnaryOp, tokens: &mut Vec<GenericToken>) {
        let op = match unary_op.op {
            ast::UnaryOp::Not => "not",
            ast::UnaryOp::Invert => "~",
            ast::UnaryOp::UAdd => "+",
            ast::UnaryOp::USub => "-",
        };
        tokens.push(GenericToken::operator(op.to_string()));
        self.extract_tokens_from_expr(&unary_op.operand, tokens);
    }

    /// Extract tokens from lambda expressions
    fn extract_lambda_tokens(&self, lambda: &ast::ExprLambda, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::keyword("lambda".to_string()));
        self.extract_tokens_from_expr(&lambda.body, tokens);
    }

    /// Extract tokens from conditional expressions (ternary operator)
    fn extract_if_exp_tokens(&self, if_exp: &ast::ExprIfExp, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::control_flow("if".to_string()));
        self.extract_tokens_from_expr(&if_exp.test, tokens);
        self.extract_tokens_from_expr(&if_exp.body, tokens);
        self.extract_tokens_from_expr(&if_exp.orelse, tokens);
    }

    /// Helper function for comprehension patterns
    fn extract_comprehension_tokens(
        &self,
        comp_type: &str,
        generators: &[ast::Comprehension],
        tokens: &mut Vec<GenericToken>,
    ) {
        tokens.push(GenericToken::new(
            TokenCategory::Custom(comp_type.to_string()),
            1.1,
            comp_type.to_string(),
        ));
        for gen in generators {
            self.extract_tokens_from_comprehension(gen, tokens);
        }
    }

    /// Extract tokens from list comprehensions
    fn extract_list_comp_tokens(
        &self,
        list_comp: &ast::ExprListComp,
        tokens: &mut Vec<GenericToken>,
    ) {
        self.extract_comprehension_tokens("list_comp", &list_comp.generators, tokens);
        self.extract_tokens_from_expr(&list_comp.elt, tokens);
    }

    /// Extract tokens from set comprehensions
    fn extract_set_comp_tokens(&self, set_comp: &ast::ExprSetComp, tokens: &mut Vec<GenericToken>) {
        self.extract_comprehension_tokens("set_comp", &set_comp.generators, tokens);
        self.extract_tokens_from_expr(&set_comp.elt, tokens);
    }

    /// Extract tokens from dict comprehensions
    fn extract_dict_comp_tokens(
        &self,
        dict_comp: &ast::ExprDictComp,
        tokens: &mut Vec<GenericToken>,
    ) {
        self.extract_comprehension_tokens("dict_comp", &dict_comp.generators, tokens);
        self.extract_tokens_from_expr(&dict_comp.key, tokens);
        self.extract_tokens_from_expr(&dict_comp.value, tokens);
    }

    /// Extract tokens from generator expressions
    fn extract_generator_tokens(
        &self,
        gen_exp: &ast::ExprGeneratorExp,
        tokens: &mut Vec<GenericToken>,
    ) {
        self.extract_comprehension_tokens("generator", &gen_exp.generators, tokens);
        self.extract_tokens_from_expr(&gen_exp.elt, tokens);
    }

    /// Extract tokens from await expressions
    fn extract_await_tokens(&self, await_expr: &ast::ExprAwait, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::keyword("await".to_string()));
        self.extract_tokens_from_expr(&await_expr.value, tokens);
    }

    /// Extract tokens from yield expressions
    fn extract_yield_tokens(&self, yield_expr: &ast::ExprYield, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::keyword("yield".to_string()));
        if let Some(value) = &yield_expr.value {
            self.extract_tokens_from_expr(value, tokens);
        }
    }

    /// Extract tokens from yield from expressions
    fn extract_yield_from_tokens(
        &self,
        yield_from: &ast::ExprYieldFrom,
        tokens: &mut Vec<GenericToken>,
    ) {
        tokens.push(GenericToken::keyword("yield".to_string()));
        tokens.push(GenericToken::keyword("from".to_string()));
        self.extract_tokens_from_expr(&yield_from.value, tokens);
    }

    /// Extract comparison operator string from AST
    fn get_comparison_op_str(op: &ast::CmpOp) -> &'static str {
        match op {
            ast::CmpOp::Eq => "==",
            ast::CmpOp::NotEq => "!=",
            ast::CmpOp::Lt => "<",
            ast::CmpOp::LtE => "<=",
            ast::CmpOp::Gt => ">",
            ast::CmpOp::GtE => ">=",
            ast::CmpOp::Is => "is",
            ast::CmpOp::IsNot => "is not",
            ast::CmpOp::In => "in",
            ast::CmpOp::NotIn => "not in",
        }
    }

    /// Extract tokens from comparison expressions
    fn extract_compare_tokens(&self, compare: &ast::ExprCompare, tokens: &mut Vec<GenericToken>) {
        self.extract_tokens_from_expr(&compare.left, tokens);
        for op in &compare.ops {
            let op_str = Self::get_comparison_op_str(op);
            tokens.push(GenericToken::operator(op_str.to_string()));
        }
        for comp in &compare.comparators {
            self.extract_tokens_from_expr(comp, tokens);
        }
    }

    /// Extract tokens from function call expressions
    fn extract_call_tokens(&self, call: &ast::ExprCall, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::function_call("call".to_string()));
        self.extract_tokens_from_expr(&call.func, tokens);
        for arg in &call.args {
            self.extract_tokens_from_expr(arg, tokens);
        }
    }

    /// Extract tokens from name expressions
    fn extract_name_tokens(&self, name: &ast::ExprName, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::identifier(normalize_identifier(&name.id)));
    }

    /// Get constant type string from AST constant
    fn get_constant_type_str(constant: &rustpython_parser::ast::Constant) -> &'static str {
        match constant {
            rustpython_parser::ast::Constant::None => "None",
            rustpython_parser::ast::Constant::Bool(_) => "bool",
            rustpython_parser::ast::Constant::Str(_) => "string",
            rustpython_parser::ast::Constant::Bytes(_) => "bytes",
            rustpython_parser::ast::Constant::Int(_) => "int",
            rustpython_parser::ast::Constant::Float(_) => "float",
            rustpython_parser::ast::Constant::Complex { .. } => "complex",
            rustpython_parser::ast::Constant::Ellipsis => "...",
            rustpython_parser::ast::Constant::Tuple(_) => "tuple",
        }
    }

    /// Extract tokens from constant expressions
    fn extract_constant_tokens(
        &self,
        constant: &ast::ExprConstant,
        tokens: &mut Vec<GenericToken>,
    ) {
        let const_type = Self::get_constant_type_str(&constant.value);
        tokens.push(GenericToken::literal(const_type.to_string()));
    }

    /// Extract tokens from named expressions (walrus operator)
    fn extract_named_expr_tokens(
        &self,
        named: &ast::ExprNamedExpr,
        tokens: &mut Vec<GenericToken>,
    ) {
        tokens.push(GenericToken::operator(":=".to_string()));
        self.extract_tokens_from_expr(&named.target, tokens);
        self.extract_tokens_from_expr(&named.value, tokens);
    }

    /// Extract tokens from comprehension generators
    fn extract_tokens_from_comprehension(
        &self,
        gen: &ast::Comprehension,
        tokens: &mut Vec<GenericToken>,
    ) {
        tokens.push(GenericToken::control_flow("for".to_string()));
        self.extract_tokens_from_expr(&gen.target, tokens);
        tokens.push(GenericToken::operator("in".to_string()));
        self.extract_tokens_from_expr(&gen.iter, tokens);
        for if_clause in &gen.ifs {
            tokens.push(GenericToken::control_flow("if".to_string()));
            self.extract_tokens_from_expr(if_clause, tokens);
        }
    }

    /// Detect patterns in Python code
    fn detect_python_patterns(&self, stmts: &[ast::Stmt]) -> Vec<String> {
        let mut patterns = Vec::new();
        for stmt in stmts {
            self.collect_patterns_from_stmt(stmt, &mut patterns);
        }
        patterns
    }

    /// Collect patterns from a statement
    fn collect_patterns_from_stmt(&self, stmt: &ast::Stmt, patterns: &mut Vec<String>) {
        match stmt {
            ast::Stmt::If(if_stmt) => {
                patterns.push("if-stmt".to_string());
                for s in &if_stmt.body {
                    self.collect_patterns_from_stmt(s, patterns);
                }
                if !if_stmt.orelse.is_empty() {
                    patterns.push("else-stmt".to_string());
                    for s in &if_stmt.orelse {
                        self.collect_patterns_from_stmt(s, patterns);
                    }
                }
            }
            ast::Stmt::While(_) => patterns.push("while-loop".to_string()),
            ast::Stmt::For(_) => patterns.push("for-loop".to_string()),
            ast::Stmt::With(_) => patterns.push("with-context".to_string()),
            ast::Stmt::Match(match_stmt) => {
                patterns.push(format!("match-{}", match_stmt.cases.len()));
            }
            ast::Stmt::Try(try_stmt) => {
                patterns.push(format!("try-except-{}", try_stmt.handlers.len()));
            }
            ast::Stmt::FunctionDef(_) => patterns.push("function-def".to_string()),
            ast::Stmt::AsyncFunctionDef(_) => patterns.push("async-function".to_string()),
            ast::Stmt::ClassDef(_) => patterns.push("class-def".to_string()),
            ast::Stmt::Return(_) => patterns.push("return".to_string()),
            ast::Stmt::Raise(_) => patterns.push("raise".to_string()),
            ast::Stmt::Assert(_) => patterns.push("assert".to_string()),
            ast::Stmt::Expr(expr_stmt) => {
                self.collect_patterns_from_expr(&expr_stmt.value, patterns);
            }
            ast::Stmt::Assign(_) => patterns.push("assign".to_string()),
            _ => {}
        }
    }

    /// Collect patterns from expressions
    fn collect_patterns_from_expr(&self, expr: &ast::Expr, patterns: &mut Vec<String>) {
        match expr {
            ast::Expr::ListComp(_) => patterns.push("list-comp".to_string()),
            ast::Expr::SetComp(_) => patterns.push("set-comp".to_string()),
            ast::Expr::DictComp(_) => patterns.push("dict-comp".to_string()),
            ast::Expr::GeneratorExp(_) => patterns.push("generator".to_string()),
            ast::Expr::Lambda(_) => patterns.push("lambda".to_string()),
            ast::Expr::Call(_) => patterns.push("call".to_string()),
            ast::Expr::BinOp(_) => patterns.push("binary".to_string()),
            ast::Expr::BoolOp(_) => patterns.push("bool-op".to_string()),
            ast::Expr::Compare(_) => patterns.push("compare".to_string()),
            ast::Expr::NamedExpr(_) => patterns.push("walrus".to_string()),
            _ => {}
        }
    }

    /// Calculate branch similarity for Python
    fn calculate_python_branch_similarity(&self, stmts: &[ast::Stmt]) -> f64 {
        let mut branch_groups = Vec::new();
        for stmt in stmts {
            self.collect_branches_from_stmt(stmt, &mut branch_groups);
        }

        if branch_groups.is_empty() {
            return 0.0;
        }

        let total_similarity: f64 = branch_groups
            .iter()
            .map(|group| group.calculate_similarity())
            .sum();

        (total_similarity / branch_groups.len() as f64).min(1.0)
    }

    /// Collect branches from a statement
    #[allow(clippy::only_used_in_recursion)]
    fn collect_branches_from_stmt(&self, stmt: &ast::Stmt, groups: &mut Vec<BranchGroup>) {
        match stmt {
            ast::Stmt::If(if_stmt) => {
                let mut group = BranchGroup::new();

                // Main if branch
                let if_tokens = self.extract_python_tokens(&if_stmt.body);
                group.add_branch(if_tokens);

                // Else/elif branches
                if !if_stmt.orelse.is_empty() {
                    let else_tokens = self.extract_python_tokens(&if_stmt.orelse);
                    group.add_branch(else_tokens);
                }

                if group.branches.len() > 1 {
                    groups.push(group);
                }

                // Recursively check nested statements
                for s in &if_stmt.body {
                    self.collect_branches_from_stmt(s, groups);
                }
                for s in &if_stmt.orelse {
                    self.collect_branches_from_stmt(s, groups);
                }
            }
            ast::Stmt::Match(match_stmt) => {
                let mut group = BranchGroup::new();

                for case in &match_stmt.cases {
                    let case_tokens = self.extract_python_tokens(&case.body);
                    group.add_branch(case_tokens);
                }

                if group.branches.len() > 1 {
                    groups.push(group);
                }

                // Recursively check case bodies
                for case in &match_stmt.cases {
                    for s in &case.body {
                        self.collect_branches_from_stmt(s, groups);
                    }
                }
            }
            ast::Stmt::Try(try_stmt) => {
                let mut group = BranchGroup::new();

                // Try body
                let try_tokens = self.extract_python_tokens(&try_stmt.body);
                group.add_branch(try_tokens);

                // Exception handlers
                for handler in &try_stmt.handlers {
                    match handler {
                        ast::ExceptHandler::ExceptHandler(h) => {
                            let handler_tokens = self.extract_python_tokens(&h.body);
                            group.add_branch(handler_tokens);
                        }
                    }
                }

                if group.branches.len() > 1 {
                    groups.push(group);
                }

                // Recursively check nested statements
                for s in &try_stmt.body {
                    self.collect_branches_from_stmt(s, groups);
                }
                for handler in &try_stmt.handlers {
                    match handler {
                        ast::ExceptHandler::ExceptHandler(h) => {
                            for s in &h.body {
                                self.collect_branches_from_stmt(s, groups);
                            }
                        }
                    }
                }
            }
            ast::Stmt::For(for_stmt) => {
                for s in &for_stmt.body {
                    self.collect_branches_from_stmt(s, groups);
                }
            }
            ast::Stmt::While(while_stmt) => {
                for s in &while_stmt.body {
                    self.collect_branches_from_stmt(s, groups);
                }
            }
            ast::Stmt::With(with_stmt) => {
                for s in &with_stmt.body {
                    self.collect_branches_from_stmt(s, groups);
                }
            }
            ast::Stmt::FunctionDef(func_def) => {
                for s in &func_def.body {
                    self.collect_branches_from_stmt(s, groups);
                }
            }
            ast::Stmt::AsyncFunctionDef(func_def) => {
                for s in &func_def.body {
                    self.collect_branches_from_stmt(s, groups);
                }
            }
            ast::Stmt::ClassDef(class_def) => {
                for s in &class_def.body {
                    self.collect_branches_from_stmt(s, groups);
                }
            }
            _ => {}
        }
    }

    /// Count unique variables in Python code
    fn count_unique_variables(&self, stmts: &[ast::Stmt]) -> usize {
        let mut variables = HashSet::new();
        for stmt in stmts {
            self.collect_variables_from_stmt(stmt, &mut variables);
        }
        variables.len()
    }

    /// Collect variables from a statement
    #[allow(clippy::only_used_in_recursion)]
    fn collect_variables_from_stmt(&self, stmt: &ast::Stmt, vars: &mut HashSet<String>) {
        match stmt {
            ast::Stmt::Assign(assign) => {
                for target in &assign.targets {
                    self.collect_variables_from_expr(target, vars);
                }
            }
            ast::Stmt::AugAssign(aug) => {
                self.collect_variables_from_expr(&aug.target, vars);
            }
            ast::Stmt::AnnAssign(ann) => {
                self.collect_variables_from_expr(&ann.target, vars);
            }
            ast::Stmt::For(for_stmt) => {
                self.collect_variables_from_expr(&for_stmt.target, vars);
                for s in &for_stmt.body {
                    self.collect_variables_from_stmt(s, vars);
                }
            }
            ast::Stmt::FunctionDef(func) => {
                for arg in &func.args.args {
                    vars.insert(arg.def.arg.to_string());
                }
                for s in &func.body {
                    self.collect_variables_from_stmt(s, vars);
                }
            }
            ast::Stmt::AsyncFunctionDef(func) => {
                for arg in &func.args.args {
                    vars.insert(arg.def.arg.to_string());
                }
                for s in &func.body {
                    self.collect_variables_from_stmt(s, vars);
                }
            }
            ast::Stmt::If(if_stmt) => {
                for s in &if_stmt.body {
                    self.collect_variables_from_stmt(s, vars);
                }
                for s in &if_stmt.orelse {
                    self.collect_variables_from_stmt(s, vars);
                }
            }
            ast::Stmt::While(while_stmt) => {
                for s in &while_stmt.body {
                    self.collect_variables_from_stmt(s, vars);
                }
            }
            ast::Stmt::With(with_stmt) => {
                for item in &with_stmt.items {
                    if let Some(optional_vars) = &item.optional_vars {
                        self.collect_variables_from_expr(optional_vars, vars);
                    }
                }
                for s in &with_stmt.body {
                    self.collect_variables_from_stmt(s, vars);
                }
            }
            ast::Stmt::Try(try_stmt) => {
                for s in &try_stmt.body {
                    self.collect_variables_from_stmt(s, vars);
                }
                for handler in &try_stmt.handlers {
                    match handler {
                        ast::ExceptHandler::ExceptHandler(h) => {
                            if let Some(name) = &h.name {
                                vars.insert(name.to_string());
                            }
                            for s in &h.body {
                                self.collect_variables_from_stmt(s, vars);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Collect variables from expressions
    #[allow(clippy::only_used_in_recursion)]
    fn collect_variables_from_expr(&self, expr: &ast::Expr, vars: &mut HashSet<String>) {
        match expr {
            ast::Expr::Name(name) => {
                vars.insert(name.id.to_string());
            }
            ast::Expr::Tuple(tuple) => {
                for elt in &tuple.elts {
                    self.collect_variables_from_expr(elt, vars);
                }
            }
            ast::Expr::List(list) => {
                for elt in &list.elts {
                    self.collect_variables_from_expr(elt, vars);
                }
            }
            _ => {}
        }
    }

    /// Calculate maximum nesting depth
    fn calculate_max_nesting(&self, stmts: &[ast::Stmt]) -> u32 {
        let mut max_depth = 0;
        for stmt in stmts {
            max_depth = max_depth.max(self.calculate_stmt_nesting(stmt, 0));
        }
        max_depth
    }

    /// Calculate nesting depth for a statement
    #[allow(clippy::only_used_in_recursion)]
    fn calculate_stmt_nesting(&self, stmt: &ast::Stmt, current_depth: u32) -> u32 {
        let nested_depth = current_depth + 1;
        match stmt {
            ast::Stmt::If(if_stmt) => {
                let mut max = nested_depth;
                for s in &if_stmt.body {
                    max = max.max(self.calculate_stmt_nesting(s, nested_depth));
                }
                for s in &if_stmt.orelse {
                    max = max.max(self.calculate_stmt_nesting(s, nested_depth));
                }
                max
            }
            ast::Stmt::While(while_stmt) => {
                let mut max = nested_depth;
                for s in &while_stmt.body {
                    max = max.max(self.calculate_stmt_nesting(s, nested_depth));
                }
                max
            }
            ast::Stmt::For(for_stmt) => {
                let mut max = nested_depth;
                for s in &for_stmt.body {
                    max = max.max(self.calculate_stmt_nesting(s, nested_depth));
                }
                max
            }
            ast::Stmt::With(with_stmt) => {
                let mut max = nested_depth;
                for s in &with_stmt.body {
                    max = max.max(self.calculate_stmt_nesting(s, nested_depth));
                }
                max
            }
            ast::Stmt::Try(try_stmt) => {
                let mut max = nested_depth;
                for s in &try_stmt.body {
                    max = max.max(self.calculate_stmt_nesting(s, nested_depth));
                }
                for handler in &try_stmt.handlers {
                    match handler {
                        ast::ExceptHandler::ExceptHandler(h) => {
                            for s in &h.body {
                                max = max.max(self.calculate_stmt_nesting(s, nested_depth));
                            }
                        }
                    }
                }
                for s in &try_stmt.finalbody {
                    max = max.max(self.calculate_stmt_nesting(s, nested_depth));
                }
                max
            }
            ast::Stmt::Match(match_stmt) => {
                let mut max = nested_depth;
                for case in &match_stmt.cases {
                    for s in &case.body {
                        max = max.max(self.calculate_stmt_nesting(s, nested_depth));
                    }
                }
                max
            }
            ast::Stmt::FunctionDef(func) => {
                let mut max = nested_depth;
                for s in &func.body {
                    max = max.max(self.calculate_stmt_nesting(s, nested_depth));
                }
                max
            }
            ast::Stmt::AsyncFunctionDef(func) => {
                let mut max = nested_depth;
                for s in &func.body {
                    max = max.max(self.calculate_stmt_nesting(s, nested_depth));
                }
                max
            }
            ast::Stmt::ClassDef(class) => {
                let mut max = nested_depth;
                for s in &class.body {
                    max = max.max(self.calculate_stmt_nesting(s, nested_depth));
                }
                max
            }
            _ => current_depth,
        }
    }
}

impl<'a> AnalyzerHelpers for PythonEntropyAnalyzer<'a> {}

impl<'a> LanguageEntropyAnalyzer for PythonEntropyAnalyzer<'a> {
    type AstNode = Vec<ast::Stmt>;
    type Token = GenericToken;

    fn extract_tokens(&self, node: &Self::AstNode) -> Vec<Self::Token> {
        self.extract_python_tokens(node)
    }

    fn detect_patterns(&self, node: &Self::AstNode) -> PatternMetrics {
        let patterns = self.detect_python_patterns(node);
        let unique_patterns: HashSet<_> = patterns.iter().cloned().collect();

        let mut metrics = PatternMetrics::new();
        metrics.total_patterns = patterns.len();
        metrics.unique_patterns = unique_patterns.len();
        metrics.calculate_repetition();

        metrics
    }

    fn calculate_branch_similarity(&self, node: &Self::AstNode) -> f64 {
        self.calculate_python_branch_similarity(node)
    }

    fn analyze_structure(&self, node: &Self::AstNode) -> (usize, u32) {
        let unique_vars = self.count_unique_variables(node);
        let max_nesting = self.calculate_max_nesting(node);
        (unique_vars, max_nesting)
    }

    fn generate_cache_key(&self, node: &Self::AstNode) -> String {
        // Generate a unique key based on the statements hash
        use sha2::{Digest, Sha256};
        let stmt_repr = format!("{:?}", node);
        let mut hasher = Sha256::new();
        hasher.update(stmt_repr.as_bytes());
        let hash = hasher.finalize();
        format!("python_{:x}", hash)
    }
}

/// Helper struct for branch similarity calculation
struct BranchGroup {
    branches: Vec<Vec<GenericToken>>,
}

impl BranchGroup {
    fn new() -> Self {
        Self {
            branches: Vec::new(),
        }
    }

    fn add_branch(&mut self, tokens: Vec<GenericToken>) {
        self.branches.push(tokens);
    }

    fn calculate_similarity(&self) -> f64 {
        if self.branches.len() < 2 {
            return 0.0;
        }

        let mut total_similarity = 0.0;
        let mut comparisons = 0;

        for i in 0..self.branches.len() {
            for j in i + 1..self.branches.len() {
                total_similarity += self.token_similarity(&self.branches[i], &self.branches[j]);
                comparisons += 1;
            }
        }

        if comparisons > 0 {
            total_similarity / comparisons as f64
        } else {
            0.0
        }
    }

    fn token_similarity(&self, tokens1: &[GenericToken], tokens2: &[GenericToken]) -> f64 {
        if tokens1.is_empty() || tokens2.is_empty() {
            return 0.0;
        }

        let set1: HashSet<_> = tokens1.iter().collect();
        let set2: HashSet<_> = tokens2.iter().collect();

        let intersection = set1.intersection(&set2).count();
        let union = set1.union(&set2).count();

        if union > 0 {
            intersection as f64 / union as f64
        } else {
            0.0
        }
    }
}

/// Normalize identifier names to reduce noise
fn normalize_identifier(name: &str) -> String {
    if name.len() > 3 {
        "VAR".to_string()
    } else {
        name.to_uppercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::complexity::entropy_core::EntropyToken;
    use rustpython_parser::{ast, Parse};

    fn create_analyzer() -> PythonEntropyAnalyzer<'static> {
        PythonEntropyAnalyzer::new("")
    }

    fn parse_expr(code: &str) -> ast::Expr {
        let full_code = format!("x = {}", code);
        let parsed = ast::Suite::parse(&full_code, "<test>").unwrap();
        if let ast::Stmt::Assign(assign) = &parsed[0] {
            *assign.value.clone()
        } else {
            panic!("Failed to parse expression")
        }
    }

    #[test]
    fn test_extract_bool_op_tokens() {
        let analyzer = create_analyzer();
        let mut tokens = Vec::new();
        let expr = parse_expr("True and False or True");

        analyzer.extract_tokens_from_expr(&expr, &mut tokens);

        // Should have boolean operators
        assert!(tokens.iter().any(|t| t.value() == "and"));
        assert!(tokens.iter().any(|t| t.value() == "or"));
    }

    #[test]
    fn test_extract_bin_op_tokens() {
        let analyzer = create_analyzer();
        let mut tokens = Vec::new();
        let expr = parse_expr("5 + 3 * 2");

        analyzer.extract_tokens_from_expr(&expr, &mut tokens);

        // Should have binary operators
        let op_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(t.to_category(), TokenCategory::Operator))
            .collect();
        assert_eq!(op_tokens.len(), 2);
    }

    #[test]
    fn test_extract_unary_op_tokens() {
        let analyzer = create_analyzer();
        let mut tokens = Vec::new();
        let expr = parse_expr("not True");

        analyzer.extract_tokens_from_expr(&expr, &mut tokens);

        // Should have unary operator
        assert!(tokens.iter().any(|t| t.value() == "not"));
    }

    #[test]
    fn test_extract_compare_tokens() {
        let analyzer = create_analyzer();
        let mut tokens = Vec::new();
        let expr = parse_expr("x > 5 and y <= 10");

        analyzer.extract_tokens_from_expr(&expr, &mut tokens);

        // Should have comparison operators
        assert!(tokens.iter().any(|t| t.value() == ">"));
        assert!(tokens.iter().any(|t| t.value() == "<="));
    }

    #[test]
    fn test_get_comparison_op_str() {
        assert_eq!(
            PythonEntropyAnalyzer::get_comparison_op_str(&ast::CmpOp::Eq),
            "=="
        );
        assert_eq!(
            PythonEntropyAnalyzer::get_comparison_op_str(&ast::CmpOp::NotEq),
            "!="
        );
        assert_eq!(
            PythonEntropyAnalyzer::get_comparison_op_str(&ast::CmpOp::Lt),
            "<"
        );
        assert_eq!(
            PythonEntropyAnalyzer::get_comparison_op_str(&ast::CmpOp::LtE),
            "<="
        );
        assert_eq!(
            PythonEntropyAnalyzer::get_comparison_op_str(&ast::CmpOp::Gt),
            ">"
        );
        assert_eq!(
            PythonEntropyAnalyzer::get_comparison_op_str(&ast::CmpOp::GtE),
            ">="
        );
        assert_eq!(
            PythonEntropyAnalyzer::get_comparison_op_str(&ast::CmpOp::Is),
            "is"
        );
        assert_eq!(
            PythonEntropyAnalyzer::get_comparison_op_str(&ast::CmpOp::IsNot),
            "is not"
        );
        assert_eq!(
            PythonEntropyAnalyzer::get_comparison_op_str(&ast::CmpOp::In),
            "in"
        );
        assert_eq!(
            PythonEntropyAnalyzer::get_comparison_op_str(&ast::CmpOp::NotIn),
            "not in"
        );
    }

    #[test]
    fn test_extract_list_comp_tokens() {
        let analyzer = create_analyzer();
        let mut tokens = Vec::new();
        let expr = parse_expr("[x * 2 for x in range(10) if x > 5]");

        analyzer.extract_tokens_from_expr(&expr, &mut tokens);

        // Should have list comprehension marker
        assert!(tokens.iter().any(|t| t.value() == "list_comp"));
        // Should have control flow tokens
        assert!(tokens.iter().any(|t| t.value() == "for"));
        assert!(tokens.iter().any(|t| t.value() == "if"));
    }

    #[test]
    fn test_extract_lambda_tokens() {
        let analyzer = create_analyzer();
        let mut tokens = Vec::new();
        let expr = parse_expr("lambda x: x * 2");

        analyzer.extract_tokens_from_expr(&expr, &mut tokens);

        // Should have lambda keyword
        assert!(tokens.iter().any(|t| t.value() == "lambda"));
    }

    #[test]
    fn test_extract_if_exp_tokens() {
        let analyzer = create_analyzer();
        let mut tokens = Vec::new();
        let expr = parse_expr("5 if x > 0 else 10");

        analyzer.extract_tokens_from_expr(&expr, &mut tokens);

        // Should have if control flow
        assert!(tokens.iter().any(|t| t.value() == "if"));
    }

    #[test]
    fn test_extract_call_tokens() {
        let analyzer = create_analyzer();
        let mut tokens = Vec::new();
        let expr = parse_expr("func(1, 2, 3)");

        analyzer.extract_tokens_from_expr(&expr, &mut tokens);

        // Should have function call
        assert!(tokens.iter().any(|t| t.value() == "call"));
    }

    #[test]
    fn test_get_constant_type_str() {
        assert_eq!(
            PythonEntropyAnalyzer::get_constant_type_str(&rustpython_parser::ast::Constant::None),
            "None"
        );
        assert_eq!(
            PythonEntropyAnalyzer::get_constant_type_str(&rustpython_parser::ast::Constant::Bool(
                true
            )),
            "bool"
        );
        assert_eq!(
            PythonEntropyAnalyzer::get_constant_type_str(&rustpython_parser::ast::Constant::Str(
                "test".to_string()
            )),
            "string"
        );
        assert_eq!(
            PythonEntropyAnalyzer::get_constant_type_str(&rustpython_parser::ast::Constant::Int(
                42.into()
            )),
            "int"
        );
        assert_eq!(
            PythonEntropyAnalyzer::get_constant_type_str(&rustpython_parser::ast::Constant::Float(
                3.5
            )),
            "float"
        );
        assert_eq!(
            PythonEntropyAnalyzer::get_constant_type_str(
                &rustpython_parser::ast::Constant::Ellipsis
            ),
            "..."
        );
    }

    #[test]
    fn test_extract_constant_tokens() {
        let analyzer = create_analyzer();
        let mut tokens = Vec::new();
        let expr = parse_expr("42");

        analyzer.extract_tokens_from_expr(&expr, &mut tokens);

        // Should have literal token
        assert!(tokens.iter().any(|t| t.value() == "int"));
    }

    #[test]
    fn test_extract_name_tokens() {
        let analyzer = create_analyzer();
        let mut tokens = Vec::new();
        let expr = parse_expr("variable_name");

        analyzer.extract_tokens_from_expr(&expr, &mut tokens);

        // Should have normalized identifier
        assert!(tokens
            .iter()
            .any(|t| matches!(t.to_category(), TokenCategory::Identifier)));
    }

    #[test]
    fn test_extract_await_tokens() {
        let analyzer = create_analyzer();
        let mut tokens = Vec::new();
        let expr = parse_expr("await async_func()");

        analyzer.extract_tokens_from_expr(&expr, &mut tokens);

        // Should have await keyword
        assert!(tokens.iter().any(|t| t.value() == "await"));
    }

    #[test]
    fn test_extract_yield_tokens() {
        // Parse as statement since yield is a statement-level expression
        let code = "def f():\n    yield 42";
        let parsed = ast::Suite::parse(code, "<test>").unwrap();

        if let ast::Stmt::FunctionDef(func) = &parsed[0] {
            if let ast::Stmt::Expr(expr_stmt) = &func.body[0] {
                let analyzer = create_analyzer();
                let mut tokens = Vec::new();

                analyzer.extract_tokens_from_expr(&expr_stmt.value, &mut tokens);

                // Should have yield keyword
                assert!(tokens.iter().any(|t| t.value() == "yield"));
            }
        }
    }

    #[test]
    fn test_extract_dict_comp_tokens() {
        let analyzer = create_analyzer();
        let mut tokens = Vec::new();
        let expr = parse_expr("{k: v * 2 for k, v in items.items()}");

        analyzer.extract_tokens_from_expr(&expr, &mut tokens);

        // Should have dict comprehension marker
        assert!(tokens.iter().any(|t| t.value() == "dict_comp"));
        // Should have control flow tokens
        assert!(tokens.iter().any(|t| t.value() == "for"));
    }

    #[test]
    fn test_extract_set_comp_tokens() {
        let analyzer = create_analyzer();
        let mut tokens = Vec::new();
        let expr = parse_expr("{x * 2 for x in range(10)}");

        analyzer.extract_tokens_from_expr(&expr, &mut tokens);

        // Should have set comprehension marker
        assert!(tokens.iter().any(|t| t.value() == "set_comp"));
    }

    #[test]
    fn test_normalize_identifier() {
        assert_eq!(normalize_identifier("x"), "X");
        assert_eq!(normalize_identifier("foo"), "FOO");
        assert_eq!(normalize_identifier("long_variable_name"), "VAR");
        assert_eq!(normalize_identifier("test"), "VAR");
    }

    #[test]
    fn test_complex_expression() {
        let analyzer = create_analyzer();
        let mut tokens = Vec::new();
        let expr = parse_expr("[x * 2 if x > 0 else -x for x in range(10) if x != 5]");

        analyzer.extract_tokens_from_expr(&expr, &mut tokens);

        // Should handle complex nested expressions
        assert!(tokens.iter().any(|t| t.value() == "list_comp"));
        assert!(tokens.iter().any(|t| t.value() == "for"));
        assert!(tokens.iter().any(|t| t.value() == "if"));
        assert!(tokens.iter().any(|t| t.value() == "in"));
        assert!(tokens.iter().any(|t| t.value() == "!="));
    }

    #[test]
    fn test_named_expr_tokens() {
        // Walrus operator (Python 3.8+)
        let code = "if (n := len(items)) > 10:\n    pass";
        let parsed = ast::Suite::parse(code, "<test>").unwrap();

        if let ast::Stmt::If(if_stmt) = &parsed[0] {
            let analyzer = create_analyzer();
            let mut tokens = Vec::new();

            analyzer.extract_tokens_from_expr(&if_stmt.test, &mut tokens);

            // Should have walrus operator and comparison
            assert!(tokens.iter().any(|t| t.value() == ":="));
            assert!(tokens.iter().any(|t| t.value() == ">"));
        }
    }
}
