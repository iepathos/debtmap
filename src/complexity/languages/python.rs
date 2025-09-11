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
        use ast::Stmt::*;
        match stmt {
            // Control flow statements
            If(if_stmt) => self.process_if_stmt(if_stmt, tokens),
            While(while_stmt) => self.process_while_stmt(while_stmt, tokens),
            For(for_stmt) => self.process_for_stmt(for_stmt, tokens),
            With(with_stmt) => self.process_with_stmt(with_stmt, tokens),
            Match(match_stmt) => self.process_match_stmt(match_stmt, tokens),
            Try(try_stmt) => self.process_try_stmt(try_stmt, tokens),

            // Simple statements
            Return(return_stmt) => self.process_return_stmt(return_stmt, tokens),
            Raise(raise_stmt) => self.process_raise_stmt(raise_stmt, tokens),
            Break(_) => tokens.push(GenericToken::keyword("break".to_string())),
            Continue(_) => tokens.push(GenericToken::keyword("continue".to_string())),
            Pass(_) => tokens.push(GenericToken::keyword("pass".to_string())),
            Assert(assert_stmt) => self.process_assert_stmt(assert_stmt, tokens),
            Global(_) => tokens.push(GenericToken::keyword("global".to_string())),
            Nonlocal(_) => tokens.push(GenericToken::keyword("nonlocal".to_string())),

            // Definition statements
            FunctionDef(func_def) => self.process_function_def(func_def, tokens),
            AsyncFunctionDef(func_def) => self.process_async_function_def(func_def, tokens),
            ClassDef(class_def) => self.process_class_def(class_def, tokens),

            // Assignment statements
            Expr(expr_stmt) => self.extract_tokens_from_expr(&expr_stmt.value, tokens),
            Assign(assign_stmt) => self.process_assign_stmt(assign_stmt, tokens),
            AugAssign(aug_assign) => self.process_aug_assign_stmt(aug_assign, tokens),
            AnnAssign(ann_assign) => self.process_ann_assign_stmt(ann_assign, tokens),
            _ => {}
        }
    }

    // Process if statement
    fn process_if_stmt(&self, if_stmt: &ast::StmtIf, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::control_flow("if".to_string()));
        self.extract_tokens_from_expr(&if_stmt.test, tokens);
        self.process_stmt_body(&if_stmt.body, tokens);
        self.process_stmt_body(&if_stmt.orelse, tokens);
    }

    // Process while statement
    fn process_while_stmt(&self, while_stmt: &ast::StmtWhile, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::control_flow("while".to_string()));
        self.extract_tokens_from_expr(&while_stmt.test, tokens);
        self.process_stmt_body(&while_stmt.body, tokens);
    }

    // Process for statement
    fn process_for_stmt(&self, for_stmt: &ast::StmtFor, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::control_flow("for".to_string()));
        self.extract_tokens_from_expr(&for_stmt.target, tokens);
        self.extract_tokens_from_expr(&for_stmt.iter, tokens);
        self.process_stmt_body(&for_stmt.body, tokens);
    }

    // Process with statement
    fn process_with_stmt(&self, with_stmt: &ast::StmtWith, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::keyword("with".to_string()));
        for item in &with_stmt.items {
            self.extract_tokens_from_expr(&item.context_expr, tokens);
        }
        self.process_stmt_body(&with_stmt.body, tokens);
    }

    // Process match statement
    fn process_match_stmt(&self, match_stmt: &ast::StmtMatch, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::control_flow("match".to_string()));
        self.extract_tokens_from_expr(&match_stmt.subject, tokens);
        for case in &match_stmt.cases {
            tokens.push(GenericToken::keyword("case".to_string()));
            self.process_stmt_body(&case.body, tokens);
        }
    }

    // Process try statement
    fn process_try_stmt(&self, try_stmt: &ast::StmtTry, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::keyword("try".to_string()));
        self.process_stmt_body(&try_stmt.body, tokens);

        for handler in &try_stmt.handlers {
            tokens.push(GenericToken::keyword("except".to_string()));
            match handler {
                ast::ExceptHandler::ExceptHandler(h) => {
                    self.process_stmt_body(&h.body, tokens);
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

    // Process return statement
    fn process_return_stmt(&self, return_stmt: &ast::StmtReturn, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::keyword("return".to_string()));
        if let Some(value) = &return_stmt.value {
            self.extract_tokens_from_expr(value, tokens);
        }
    }

    // Process raise statement
    fn process_raise_stmt(&self, raise_stmt: &ast::StmtRaise, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::keyword("raise".to_string()));
        if let Some(exc) = &raise_stmt.exc {
            self.extract_tokens_from_expr(exc, tokens);
        }
    }

    // Process assert statement
    fn process_assert_stmt(&self, assert_stmt: &ast::StmtAssert, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::keyword("assert".to_string()));
        self.extract_tokens_from_expr(&assert_stmt.test, tokens);
    }

    // Process function definition
    fn process_function_def(
        &self,
        func_def: &ast::StmtFunctionDef,
        tokens: &mut Vec<GenericToken>,
    ) {
        tokens.push(GenericToken::keyword("def".to_string()));
        tokens.push(GenericToken::identifier(normalize_identifier(
            &func_def.name,
        )));
        self.process_stmt_body(&func_def.body, tokens);
    }

    // Process async function definition
    fn process_async_function_def(
        &self,
        func_def: &ast::StmtAsyncFunctionDef,
        tokens: &mut Vec<GenericToken>,
    ) {
        tokens.push(GenericToken::keyword("async".to_string()));
        tokens.push(GenericToken::keyword("def".to_string()));
        tokens.push(GenericToken::identifier(normalize_identifier(
            &func_def.name,
        )));
        self.process_stmt_body(&func_def.body, tokens);
    }

    // Process class definition
    fn process_class_def(&self, class_def: &ast::StmtClassDef, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::keyword("class".to_string()));
        tokens.push(GenericToken::identifier(normalize_identifier(
            &class_def.name,
        )));
        self.process_stmt_body(&class_def.body, tokens);
    }

    // Process assignment statement
    fn process_assign_stmt(&self, assign_stmt: &ast::StmtAssign, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::operator("=".to_string()));
        self.extract_tokens_from_expr(&assign_stmt.value, tokens);
        for target in &assign_stmt.targets {
            self.extract_tokens_from_expr(target, tokens);
        }
    }

    // Process augmented assignment
    fn process_aug_assign_stmt(
        &self,
        aug_assign: &ast::StmtAugAssign,
        tokens: &mut Vec<GenericToken>,
    ) {
        tokens.push(GenericToken::operator(format!("{:?}=", aug_assign.op)));
        self.extract_tokens_from_expr(&aug_assign.target, tokens);
        self.extract_tokens_from_expr(&aug_assign.value, tokens);
    }

    // Process annotated assignment
    fn process_ann_assign_stmt(
        &self,
        ann_assign: &ast::StmtAnnAssign,
        tokens: &mut Vec<GenericToken>,
    ) {
        if let Some(value) = &ann_assign.value {
            tokens.push(GenericToken::operator("=".to_string()));
            self.extract_tokens_from_expr(value, tokens);
        }
        self.extract_tokens_from_expr(&ann_assign.target, tokens);
    }

    // Helper to process statement bodies
    fn process_stmt_body(&self, body: &[ast::Stmt], tokens: &mut Vec<GenericToken>) {
        for s in body {
            self.extract_tokens_from_stmt(s, tokens);
        }
    }

    /// Extract tokens from expressions
    fn extract_tokens_from_expr(&self, expr: &ast::Expr, tokens: &mut Vec<GenericToken>) {
        use ast::Expr::*;
        match expr {
            BoolOp(bool_op) => self.process_bool_op(bool_op, tokens),
            BinOp(bin_op) => self.process_bin_op(bin_op, tokens),
            UnaryOp(unary_op) => self.process_unary_op(unary_op, tokens),
            Lambda(lambda) => self.process_lambda(lambda, tokens),
            IfExp(if_exp) => self.process_if_exp(if_exp, tokens),
            ListComp(comp) => self.process_list_comp(comp, tokens),
            SetComp(comp) => self.process_set_comp(comp, tokens),
            DictComp(comp) => self.process_dict_comp(comp, tokens),
            GeneratorExp(gen) => self.process_generator(gen, tokens),
            Await(await_expr) => self.process_await(await_expr, tokens),
            Yield(yield_expr) => self.process_yield(yield_expr, tokens),
            YieldFrom(yield_from) => self.process_yield_from(yield_from, tokens),
            Compare(compare) => self.process_compare(compare, tokens),
            Call(call) => self.process_call(call, tokens),
            Name(name) => Self::process_name(name, tokens),
            Constant(constant) => Self::process_constant(constant, tokens),
            NamedExpr(named) => self.process_named_expr(named, tokens),
            _ => {}
        }
    }

    // Process boolean operations
    fn process_bool_op(&self, bool_op: &ast::ExprBoolOp, tokens: &mut Vec<GenericToken>) {
        let op = Self::classify_bool_op(bool_op.op);
        tokens.push(GenericToken::operator(op.to_string()));
        for value in &bool_op.values {
            self.extract_tokens_from_expr(value, tokens);
        }
    }

    // Process binary operations
    fn process_bin_op(&self, bin_op: &ast::ExprBinOp, tokens: &mut Vec<GenericToken>) {
        let op = format!("{:?}", bin_op.op);
        tokens.push(GenericToken::operator(op));
        self.extract_tokens_from_expr(&bin_op.left, tokens);
        self.extract_tokens_from_expr(&bin_op.right, tokens);
    }

    // Process unary operations
    fn process_unary_op(&self, unary_op: &ast::ExprUnaryOp, tokens: &mut Vec<GenericToken>) {
        let op = Self::classify_unary_op(unary_op.op);
        tokens.push(GenericToken::operator(op.to_string()));
        self.extract_tokens_from_expr(&unary_op.operand, tokens);
    }

    // Process lambda expressions
    fn process_lambda(&self, lambda: &ast::ExprLambda, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::keyword("lambda".to_string()));
        self.extract_tokens_from_expr(&lambda.body, tokens);
    }

    // Process if expressions (ternary)
    fn process_if_exp(&self, if_exp: &ast::ExprIfExp, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::control_flow("if".to_string()));
        self.extract_tokens_from_expr(&if_exp.test, tokens);
        self.extract_tokens_from_expr(&if_exp.body, tokens);
        self.extract_tokens_from_expr(&if_exp.orelse, tokens);
    }

    // Process list comprehensions
    fn process_list_comp(&self, comp: &ast::ExprListComp, tokens: &mut Vec<GenericToken>) {
        self.extract_comprehension_tokens("list_comp", &comp.elt, &comp.generators, tokens);
    }

    // Process set comprehensions
    fn process_set_comp(&self, comp: &ast::ExprSetComp, tokens: &mut Vec<GenericToken>) {
        self.extract_comprehension_tokens("set_comp", &comp.elt, &comp.generators, tokens);
    }

    // Process dictionary comprehensions
    fn process_dict_comp(&self, comp: &ast::ExprDictComp, tokens: &mut Vec<GenericToken>) {
        tokens.push(Self::create_comprehension_token("dict_comp"));
        self.extract_tokens_from_expr(&comp.key, tokens);
        self.extract_tokens_from_expr(&comp.value, tokens);
        for gen in &comp.generators {
            self.extract_tokens_from_comprehension(gen, tokens);
        }
    }

    // Process generator expressions
    fn process_generator(&self, gen: &ast::ExprGeneratorExp, tokens: &mut Vec<GenericToken>) {
        self.extract_comprehension_tokens("generator", &gen.elt, &gen.generators, tokens);
    }

    // Process await expressions
    fn process_await(&self, await_expr: &ast::ExprAwait, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::keyword("await".to_string()));
        self.extract_tokens_from_expr(&await_expr.value, tokens);
    }

    // Process yield expressions
    fn process_yield(&self, yield_expr: &ast::ExprYield, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::keyword("yield".to_string()));
        if let Some(value) = &yield_expr.value {
            self.extract_tokens_from_expr(value, tokens);
        }
    }

    // Process yield from expressions
    fn process_yield_from(&self, yield_from: &ast::ExprYieldFrom, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::keyword("yield".to_string()));
        tokens.push(GenericToken::keyword("from".to_string()));
        self.extract_tokens_from_expr(&yield_from.value, tokens);
    }

    // Process comparison expressions
    fn process_compare(&self, compare: &ast::ExprCompare, tokens: &mut Vec<GenericToken>) {
        self.extract_tokens_from_expr(&compare.left, tokens);
        for op in &compare.ops {
            let op_str = Self::classify_compare_op(op);
            tokens.push(GenericToken::operator(op_str.to_string()));
        }
        for comp in &compare.comparators {
            self.extract_tokens_from_expr(comp, tokens);
        }
    }

    // Process function calls
    fn process_call(&self, call: &ast::ExprCall, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::function_call("call".to_string()));
        self.extract_tokens_from_expr(&call.func, tokens);
        for arg in &call.args {
            self.extract_tokens_from_expr(arg, tokens);
        }
    }

    // Process name references
    fn process_name(name: &ast::ExprName, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::identifier(normalize_identifier(&name.id)));
    }

    // Process constant values
    fn process_constant(constant: &ast::ExprConstant, tokens: &mut Vec<GenericToken>) {
        let const_type = Self::classify_constant(&constant.value);
        tokens.push(GenericToken::literal(const_type.to_string()));
    }

    // Process named expressions (walrus operator)
    fn process_named_expr(&self, named: &ast::ExprNamedExpr, tokens: &mut Vec<GenericToken>) {
        tokens.push(GenericToken::operator(":=".to_string()));
        self.extract_tokens_from_expr(&named.target, tokens);
        self.extract_tokens_from_expr(&named.value, tokens);
    }

    // Pure function to classify boolean operators
    fn classify_bool_op(op: ast::BoolOp) -> &'static str {
        match op {
            ast::BoolOp::And => "and",
            ast::BoolOp::Or => "or",
        }
    }

    // Pure function to classify unary operators
    fn classify_unary_op(op: ast::UnaryOp) -> &'static str {
        match op {
            ast::UnaryOp::Not => "not",
            ast::UnaryOp::Invert => "~",
            ast::UnaryOp::UAdd => "+",
            ast::UnaryOp::USub => "-",
        }
    }

    // Pure function to classify comparison operators
    fn classify_compare_op(op: &ast::CmpOp) -> &'static str {
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

    // Pure function to classify constant types
    fn classify_constant(value: &rustpython_parser::ast::Constant) -> &'static str {
        match value {
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

    // Pure function to create comprehension token
    fn create_comprehension_token(comp_type: &str) -> GenericToken {
        GenericToken::new(
            TokenCategory::Custom(comp_type.to_string()),
            1.1,
            comp_type.to_string(),
        )
    }

    // Extract tokens from comprehensions (list, set, generator)
    fn extract_comprehension_tokens(
        &self,
        comp_type: &str,
        elt: &ast::Expr,
        generators: &[ast::Comprehension],
        tokens: &mut Vec<GenericToken>,
    ) {
        tokens.push(Self::create_comprehension_token(comp_type));
        self.extract_tokens_from_expr(elt, tokens);
        for gen in generators {
            self.extract_tokens_from_comprehension(gen, tokens);
        }
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

    #[test]
    fn test_process_bool_op() {
        let source = "x and y";
        let analyzer = PythonEntropyAnalyzer::new(source);
        let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        let ast::Mod::Module(module) = module else {
            panic!("Expected Module");
        };

        let mut tokens = Vec::new();
        if let ast::Stmt::Expr(expr_stmt) = &module.body[0] {
            analyzer.extract_tokens_from_expr(&expr_stmt.value, &mut tokens);
        }

        assert!(!tokens.is_empty());
        assert!(tokens.iter().any(|t| t.value() == "and"));
    }

    #[test]
    fn test_process_bin_op() {
        let source = "x + y";
        let analyzer = PythonEntropyAnalyzer::new(source);
        let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        let ast::Mod::Module(module) = module else {
            panic!("Expected Module");
        };

        let mut tokens = Vec::new();
        if let ast::Stmt::Expr(expr_stmt) = &module.body[0] {
            analyzer.extract_tokens_from_expr(&expr_stmt.value, &mut tokens);
        }

        assert!(!tokens.is_empty());
        assert!(tokens.iter().any(|t| t.value().contains("Add")));
    }

    #[test]
    fn test_process_unary_op() {
        let source = "not x";
        let analyzer = PythonEntropyAnalyzer::new(source);
        let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        let ast::Mod::Module(module) = module else {
            panic!("Expected Module");
        };

        let mut tokens = Vec::new();
        if let ast::Stmt::Expr(expr_stmt) = &module.body[0] {
            analyzer.extract_tokens_from_expr(&expr_stmt.value, &mut tokens);
        }

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].value(), "not");
    }

    #[test]
    fn test_process_lambda() {
        let source = "lambda x: x + 1";
        let analyzer = PythonEntropyAnalyzer::new(source);
        let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        let ast::Mod::Module(module) = module else {
            panic!("Expected Module");
        };

        let mut tokens = Vec::new();
        if let ast::Stmt::Expr(expr_stmt) = &module.body[0] {
            analyzer.extract_tokens_from_expr(&expr_stmt.value, &mut tokens);
        }

        assert!(!tokens.is_empty());
        assert_eq!(tokens[0].value(), "lambda");
    }

    #[test]
    fn test_process_if_exp() {
        let source = "x if condition else y";
        let analyzer = PythonEntropyAnalyzer::new(source);
        let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        let ast::Mod::Module(module) = module else {
            panic!("Expected Module");
        };

        let mut tokens = Vec::new();
        if let ast::Stmt::Expr(expr_stmt) = &module.body[0] {
            analyzer.extract_tokens_from_expr(&expr_stmt.value, &mut tokens);
        }

        assert!(tokens.len() >= 4);
        assert!(tokens.iter().any(|t| t.value() == "if"));
    }

    #[test]
    fn test_process_list_comp() {
        let source = "[x for x in range(10)]";
        let analyzer = PythonEntropyAnalyzer::new(source);
        let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        let ast::Mod::Module(module) = module else {
            panic!("Expected Module");
        };

        let mut tokens = Vec::new();
        if let ast::Stmt::Expr(expr_stmt) = &module.body[0] {
            analyzer.extract_tokens_from_expr(&expr_stmt.value, &mut tokens);
        }

        assert!(!tokens.is_empty());
        assert!(tokens.iter().any(|t| t.value().contains("list_comp")));
    }

    #[test]
    fn test_process_call() {
        let source = "func(x, y)";
        let analyzer = PythonEntropyAnalyzer::new(source);
        let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        let ast::Mod::Module(module) = module else {
            panic!("Expected Module");
        };

        let mut tokens = Vec::new();
        if let ast::Stmt::Expr(expr_stmt) = &module.body[0] {
            analyzer.extract_tokens_from_expr(&expr_stmt.value, &mut tokens);
        }

        assert!(tokens.len() >= 3);
        assert!(tokens.iter().any(|t| t.value() == "call"));
    }

    #[test]
    fn test_process_compare() {
        let source = "x > 0";
        let analyzer = PythonEntropyAnalyzer::new(source);
        let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        let ast::Mod::Module(module) = module else {
            panic!("Expected Module");
        };

        let mut tokens = Vec::new();
        if let ast::Stmt::Expr(expr_stmt) = &module.body[0] {
            analyzer.extract_tokens_from_expr(&expr_stmt.value, &mut tokens);
        }

        assert_eq!(tokens.len(), 3);
        assert!(tokens.iter().any(|t| t.value() == ">"));
    }

    #[test]
    fn test_process_yield() {
        let source = "def f():\n    yield 42";
        let analyzer = PythonEntropyAnalyzer::new(source);
        let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        let ast::Mod::Module(module) = module else {
            panic!("Expected Module");
        };

        let mut tokens = Vec::new();
        if let ast::Stmt::FunctionDef(func) = &module.body[0] {
            for stmt in &func.body {
                analyzer.extract_tokens_from_stmt(stmt, &mut tokens);
            }
        }

        assert!(tokens.iter().any(|t| t.value() == "yield"));
    }

    #[test]
    fn test_classify_bool_op() {
        assert_eq!(
            PythonEntropyAnalyzer::classify_bool_op(ast::BoolOp::And),
            "and"
        );
        assert_eq!(
            PythonEntropyAnalyzer::classify_bool_op(ast::BoolOp::Or),
            "or"
        );
    }

    #[test]
    fn test_classify_unary_op() {
        assert_eq!(
            PythonEntropyAnalyzer::classify_unary_op(ast::UnaryOp::Not),
            "not"
        );
        assert_eq!(
            PythonEntropyAnalyzer::classify_unary_op(ast::UnaryOp::Invert),
            "~"
        );
        assert_eq!(
            PythonEntropyAnalyzer::classify_unary_op(ast::UnaryOp::UAdd),
            "+"
        );
        assert_eq!(
            PythonEntropyAnalyzer::classify_unary_op(ast::UnaryOp::USub),
            "-"
        );
    }

    #[test]
    fn test_classify_compare_op() {
        assert_eq!(
            PythonEntropyAnalyzer::classify_compare_op(&ast::CmpOp::Eq),
            "=="
        );
        assert_eq!(
            PythonEntropyAnalyzer::classify_compare_op(&ast::CmpOp::NotEq),
            "!="
        );
        assert_eq!(
            PythonEntropyAnalyzer::classify_compare_op(&ast::CmpOp::Lt),
            "<"
        );
        assert_eq!(
            PythonEntropyAnalyzer::classify_compare_op(&ast::CmpOp::LtE),
            "<="
        );
        assert_eq!(
            PythonEntropyAnalyzer::classify_compare_op(&ast::CmpOp::Gt),
            ">"
        );
        assert_eq!(
            PythonEntropyAnalyzer::classify_compare_op(&ast::CmpOp::GtE),
            ">="
        );
    }

    #[test]
    fn test_normalize_identifier() {
        assert_eq!(normalize_identifier("x"), "X");
        assert_eq!(normalize_identifier("abc"), "ABC");
        assert_eq!(normalize_identifier("long_name"), "VAR");
        assert_eq!(normalize_identifier("function"), "VAR");
    }

    #[test]
    fn test_complex_expression() {
        let source = "(x + y) * z if condition else default_value";
        let analyzer = PythonEntropyAnalyzer::new(source);
        let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        let ast::Mod::Module(module) = module else {
            panic!("Expected Module");
        };

        let mut tokens = Vec::new();
        if let ast::Stmt::Expr(expr_stmt) = &module.body[0] {
            analyzer.extract_tokens_from_expr(&expr_stmt.value, &mut tokens);
        }

        // Should have multiple operators and identifiers
        assert!(tokens.len() > 5);
        assert!(tokens.iter().any(|t| t.value() == "if"));
        assert!(tokens.iter().any(|t| t.value().contains("Add")));
        assert!(tokens.iter().any(|t| t.value().contains("Mult")));
    }

    #[test]
    fn test_process_if_stmt() {
        let source = "if x > 0:\n    print('positive')\nelse:\n    print('negative')";
        let analyzer = PythonEntropyAnalyzer::new(source);
        let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        let ast::Mod::Module(module) = module else {
            panic!("Expected Module");
        };

        let mut tokens = Vec::new();
        analyzer.extract_tokens_from_stmt(&module.body[0], &mut tokens);

        assert!(tokens.iter().any(|t| t.value() == "if"));
        assert!(tokens.iter().any(|t| t.value() == "call"));
        assert!(tokens.iter().any(|t| t.value() == ">"));
    }

    #[test]
    fn test_process_for_stmt() {
        let source = "for i in range(10):\n    x = i * 2";
        let analyzer = PythonEntropyAnalyzer::new(source);
        let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        let ast::Mod::Module(module) = module else {
            panic!("Expected Module");
        };

        let mut tokens = Vec::new();
        analyzer.extract_tokens_from_stmt(&module.body[0], &mut tokens);

        assert!(tokens.iter().any(|t| t.value() == "for"));
        assert!(tokens.iter().any(|t| t.value() == "call"));
        assert!(tokens.iter().any(|t| t.value() == "="));
    }

    #[test]
    fn test_process_while_stmt() {
        let source = "while x > 0:\n    x = x - 1";
        let analyzer = PythonEntropyAnalyzer::new(source);
        let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        let ast::Mod::Module(module) = module else {
            panic!("Expected Module");
        };

        let mut tokens = Vec::new();
        analyzer.extract_tokens_from_stmt(&module.body[0], &mut tokens);

        assert!(tokens.iter().any(|t| t.value() == "while"));
        assert!(tokens.iter().any(|t| t.value() == ">"));
        assert!(tokens.iter().any(|t| t.value() == "="));
    }

    #[test]
    fn test_process_try_stmt() {
        let source = "try:\n    risky_operation()\nexcept Exception:\n    handle_error()\nfinally:\n    cleanup()";
        let analyzer = PythonEntropyAnalyzer::new(source);
        let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        let ast::Mod::Module(module) = module else {
            panic!("Expected Module");
        };

        let mut tokens = Vec::new();
        analyzer.extract_tokens_from_stmt(&module.body[0], &mut tokens);

        assert!(tokens.iter().any(|t| t.value() == "try"));
        assert!(tokens.iter().any(|t| t.value() == "except"));
        assert!(tokens.iter().any(|t| t.value() == "finally"));
        assert!(tokens.iter().any(|t| t.value() == "call"));
    }

    #[test]
    fn test_process_function_def() {
        let source = "def my_function(x, y):\n    return x + y";
        let analyzer = PythonEntropyAnalyzer::new(source);
        let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        let ast::Mod::Module(module) = module else {
            panic!("Expected Module");
        };

        let mut tokens = Vec::new();
        analyzer.extract_tokens_from_stmt(&module.body[0], &mut tokens);

        assert!(tokens.iter().any(|t| t.value() == "def"));
        assert!(tokens.iter().any(|t| t.value() == "VAR"));
        assert!(tokens.iter().any(|t| t.value() == "return"));
    }

    #[test]
    fn test_process_class_def() {
        let source = "class MyClass:\n    def __init__(self):\n        self.value = 0";
        let analyzer = PythonEntropyAnalyzer::new(source);
        let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        let ast::Mod::Module(module) = module else {
            panic!("Expected Module");
        };

        let mut tokens = Vec::new();
        analyzer.extract_tokens_from_stmt(&module.body[0], &mut tokens);

        assert!(tokens.iter().any(|t| t.value() == "class"));
        assert!(tokens.iter().any(|t| t.value() == "VAR"));
        assert!(tokens.iter().any(|t| t.value() == "def"));
    }

    #[test]
    fn test_process_match_stmt() {
        let source = "match status:\n    case 200:\n        return 'OK'\n    case _:\n        return 'Error'";
        let analyzer = PythonEntropyAnalyzer::new(source);
        let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        let ast::Mod::Module(module) = module else {
            panic!("Expected Module");
        };

        let mut tokens = Vec::new();
        analyzer.extract_tokens_from_stmt(&module.body[0], &mut tokens);

        assert!(tokens.iter().any(|t| t.value() == "match"));
        assert!(tokens.iter().any(|t| t.value() == "case"));
        assert!(tokens.iter().any(|t| t.value() == "return"));
    }

    #[test]
    fn test_process_with_stmt() {
        let source = "with open('file.txt') as f:\n    content = f.read()";
        let analyzer = PythonEntropyAnalyzer::new(source);
        let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        let ast::Mod::Module(module) = module else {
            panic!("Expected Module");
        };

        let mut tokens = Vec::new();
        analyzer.extract_tokens_from_stmt(&module.body[0], &mut tokens);

        assert!(tokens.iter().any(|t| t.value() == "with"));
        assert!(tokens.iter().any(|t| t.value() == "call"));
        assert!(tokens.iter().any(|t| t.value() == "="));
    }

    #[test]
    fn test_process_async_function_def() {
        let source = "async def async_func():\n    await some_operation()";
        let analyzer = PythonEntropyAnalyzer::new(source);
        let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        let ast::Mod::Module(module) = module else {
            panic!("Expected Module");
        };

        let mut tokens = Vec::new();
        analyzer.extract_tokens_from_stmt(&module.body[0], &mut tokens);

        assert!(tokens.iter().any(|t| t.value() == "async"));
        assert!(tokens.iter().any(|t| t.value() == "def"));
        assert!(tokens.iter().any(|t| t.value() == "await"));
    }

    #[test]
    fn test_process_assign_variations() {
        let source = "x = 5\ny += 3\nz: int = 10";
        let analyzer = PythonEntropyAnalyzer::new(source);
        let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        let ast::Mod::Module(module) = module else {
            panic!("Expected Module");
        };

        let mut tokens = Vec::new();
        for stmt in &module.body {
            analyzer.extract_tokens_from_stmt(stmt, &mut tokens);
        }

        assert!(tokens.iter().any(|t| t.value() == "="));
        assert!(tokens.iter().any(|t| t.value().contains("Add=")));
    }

    #[test]
    fn test_process_simple_statements() {
        let source = "break\ncontinue\npass\nglobal x\nnonlocal y\nassert x > 0\nraise ValueError('error')\nreturn result";
        let analyzer = PythonEntropyAnalyzer::new(source);
        let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        let ast::Mod::Module(module) = module else {
            panic!("Expected Module");
        };

        let mut tokens = Vec::new();
        for stmt in &module.body {
            analyzer.extract_tokens_from_stmt(stmt, &mut tokens);
        }

        assert!(tokens.iter().any(|t| t.value() == "break"));
        assert!(tokens.iter().any(|t| t.value() == "continue"));
        assert!(tokens.iter().any(|t| t.value() == "pass"));
        assert!(tokens.iter().any(|t| t.value() == "global"));
        assert!(tokens.iter().any(|t| t.value() == "nonlocal"));
        assert!(tokens.iter().any(|t| t.value() == "assert"));
        assert!(tokens.iter().any(|t| t.value() == "raise"));
        assert!(tokens.iter().any(|t| t.value() == "return"));
    }
}
