//! Python statement processing module
//!
//! This module contains all functions related to processing Python AST statements
//! and extracting tokens from them. Functions are designed using functional programming
//! principles where possible.

use crate::complexity::entropy_traits::GenericToken;
use rustpython_parser::ast;

use super::core::{normalize_identifier, PythonEntropyAnalyzer};

/// Statement processor that handles all Python statement types
/// Uses functional programming principles with static methods
pub struct StatementProcessor;

impl StatementProcessor {
    /// Main entry point for processing any statement type
    pub fn process_statement(
        analyzer: &PythonEntropyAnalyzer,
        stmt: &ast::Stmt,
        tokens: &mut Vec<GenericToken>,
    ) {
        use ast::Stmt::*;
        match stmt {
            // Control flow statements
            If(if_stmt) => Self::process_if_stmt(analyzer, if_stmt, tokens),
            While(while_stmt) => Self::process_while_stmt(analyzer, while_stmt, tokens),
            For(for_stmt) => Self::process_for_stmt(analyzer, for_stmt, tokens),
            With(with_stmt) => Self::process_with_stmt(analyzer, with_stmt, tokens),
            Match(match_stmt) => Self::process_match_stmt(analyzer, match_stmt, tokens),
            Try(try_stmt) => Self::process_try_stmt(analyzer, try_stmt, tokens),

            // Simple statements
            Return(return_stmt) => Self::process_return_stmt(analyzer, return_stmt, tokens),
            Raise(raise_stmt) => Self::process_raise_stmt(analyzer, raise_stmt, tokens),
            Assert(assert_stmt) => Self::process_assert_stmt(analyzer, assert_stmt, tokens),

            // Definition statements
            FunctionDef(func_def) => Self::process_function_def(analyzer, func_def, tokens),
            AsyncFunctionDef(func_def) => {
                Self::process_async_function_def(analyzer, func_def, tokens)
            }
            ClassDef(class_def) => Self::process_class_def(analyzer, class_def, tokens),

            // Assignment statements
            Assign(assign_stmt) => Self::process_assign_stmt(analyzer, assign_stmt, tokens),
            AugAssign(aug_assign) => Self::process_aug_assign_stmt(analyzer, aug_assign, tokens),
            AnnAssign(ann_assign) => Self::process_ann_assign_stmt(analyzer, ann_assign, tokens),

            // Expression statement
            Expr(expr_stmt) => analyzer.extract_tokens_from_expr(&expr_stmt.value, tokens),

            // Other statements (pass, break, continue, etc.)
            _ => Self::process_other_stmt(stmt, tokens),
        }
    }
}

// Control Flow Statement Processors
impl StatementProcessor {
    /// Process if statement - extracts control flow tokens and processes branches
    fn process_if_stmt(
        analyzer: &PythonEntropyAnalyzer,
        if_stmt: &ast::StmtIf,
        tokens: &mut Vec<GenericToken>,
    ) {
        tokens.push(GenericToken::control_flow("if".to_string()));
        analyzer.extract_tokens_from_expr(&if_stmt.test, tokens);
        analyzer.process_stmt_body(&if_stmt.body, tokens);

        if !if_stmt.orelse.is_empty() {
            tokens.push(GenericToken::keyword("else".to_string()));
            analyzer.process_stmt_body(&if_stmt.orelse, tokens);
        }
    }

    /// Process while statement - extracts loop tokens and body
    fn process_while_stmt(
        analyzer: &PythonEntropyAnalyzer,
        while_stmt: &ast::StmtWhile,
        tokens: &mut Vec<GenericToken>,
    ) {
        tokens.push(GenericToken::control_flow("while".to_string()));
        analyzer.extract_tokens_from_expr(&while_stmt.test, tokens);
        analyzer.process_stmt_body(&while_stmt.body, tokens);

        if !while_stmt.orelse.is_empty() {
            tokens.push(GenericToken::keyword("else".to_string()));
            analyzer.process_stmt_body(&while_stmt.orelse, tokens);
        }
    }

    /// Process for statement - extracts loop tokens and iteration logic
    fn process_for_stmt(
        analyzer: &PythonEntropyAnalyzer,
        for_stmt: &ast::StmtFor,
        tokens: &mut Vec<GenericToken>,
    ) {
        tokens.push(GenericToken::control_flow("for".to_string()));
        analyzer.extract_tokens_from_expr(&for_stmt.target, tokens);
        tokens.push(GenericToken::keyword("in".to_string()));
        analyzer.extract_tokens_from_expr(&for_stmt.iter, tokens);
        analyzer.process_stmt_body(&for_stmt.body, tokens);

        if !for_stmt.orelse.is_empty() {
            tokens.push(GenericToken::keyword("else".to_string()));
            analyzer.process_stmt_body(&for_stmt.orelse, tokens);
        }
    }

    /// Process with statement - extracts context manager tokens
    fn process_with_stmt(
        analyzer: &PythonEntropyAnalyzer,
        with_stmt: &ast::StmtWith,
        tokens: &mut Vec<GenericToken>,
    ) {
        tokens.push(GenericToken::keyword("with".to_string()));

        // Process each context manager
        for (i, item) in with_stmt.items.iter().enumerate() {
            if i > 0 {
                tokens.push(GenericToken::operator(",".to_string()));
            }
            analyzer.extract_tokens_from_expr(&item.context_expr, tokens);

            // Handle 'as' clause
            if let Some(optional_vars) = &item.optional_vars {
                tokens.push(GenericToken::keyword("as".to_string()));
                analyzer.extract_tokens_from_expr(optional_vars, tokens);
            }
        }

        analyzer.process_stmt_body(&with_stmt.body, tokens);
    }

    /// Process match statement - extracts pattern matching tokens
    fn process_match_stmt(
        analyzer: &PythonEntropyAnalyzer,
        match_stmt: &ast::StmtMatch,
        tokens: &mut Vec<GenericToken>,
    ) {
        tokens.push(GenericToken::control_flow("match".to_string()));
        analyzer.extract_tokens_from_expr(&match_stmt.subject, tokens);

        for case in &match_stmt.cases {
            tokens.push(GenericToken::keyword("case".to_string()));

            // Process the case pattern
            // Process the case pattern directly (no longer optional in newer AST)
            tokens.push(GenericToken::identifier("pattern".to_string()));

            // Process guard condition
            if let Some(guard) = &case.guard {
                tokens.push(GenericToken::keyword("if".to_string()));
                analyzer.extract_tokens_from_expr(guard, tokens);
            }

            analyzer.process_stmt_body(&case.body, tokens);
        }
    }

    /// Process try statement - extracts exception handling tokens
    fn process_try_stmt(
        analyzer: &PythonEntropyAnalyzer,
        try_stmt: &ast::StmtTry,
        tokens: &mut Vec<GenericToken>,
    ) {
        tokens.push(GenericToken::keyword("try".to_string()));
        analyzer.process_stmt_body(&try_stmt.body, tokens);

        // Process exception handlers
        for handler in &try_stmt.handlers {
            tokens.push(GenericToken::keyword("except".to_string()));
            match handler {
                ast::ExceptHandler::ExceptHandler(h) => {
                    // Process exception type
                    if let Some(exception_type) = &h.type_ {
                        analyzer.extract_tokens_from_expr(exception_type, tokens);
                    }

                    // Process exception name binding
                    if let Some(name) = &h.name {
                        tokens.push(GenericToken::keyword("as".to_string()));
                        tokens.push(GenericToken::identifier(normalize_identifier(name)));
                    }

                    analyzer.process_stmt_body(&h.body, tokens);
                }
            }
        }

        // Process else clause
        if !try_stmt.orelse.is_empty() {
            tokens.push(GenericToken::keyword("else".to_string()));
            analyzer.process_stmt_body(&try_stmt.orelse, tokens);
        }

        // Process finally clause
        if !try_stmt.finalbody.is_empty() {
            tokens.push(GenericToken::keyword("finally".to_string()));
            analyzer.process_stmt_body(&try_stmt.finalbody, tokens);
        }
    }
}

// Simple Statement Processors
impl StatementProcessor {
    /// Process return statement - extracts return keyword and value
    fn process_return_stmt(
        analyzer: &PythonEntropyAnalyzer,
        return_stmt: &ast::StmtReturn,
        tokens: &mut Vec<GenericToken>,
    ) {
        tokens.push(GenericToken::keyword("return".to_string()));
        if let Some(value) = &return_stmt.value {
            analyzer.extract_tokens_from_expr(value, tokens);
        }
    }

    /// Process raise statement - extracts exception raising tokens
    fn process_raise_stmt(
        analyzer: &PythonEntropyAnalyzer,
        raise_stmt: &ast::StmtRaise,
        tokens: &mut Vec<GenericToken>,
    ) {
        tokens.push(GenericToken::keyword("raise".to_string()));

        if let Some(exc) = &raise_stmt.exc {
            analyzer.extract_tokens_from_expr(exc, tokens);

            // Handle 'from' clause
            if let Some(cause) = &raise_stmt.cause {
                tokens.push(GenericToken::keyword("from".to_string()));
                analyzer.extract_tokens_from_expr(cause, tokens);
            }
        }
    }

    /// Process assert statement - extracts assertion tokens
    fn process_assert_stmt(
        analyzer: &PythonEntropyAnalyzer,
        assert_stmt: &ast::StmtAssert,
        tokens: &mut Vec<GenericToken>,
    ) {
        tokens.push(GenericToken::keyword("assert".to_string()));
        analyzer.extract_tokens_from_expr(&assert_stmt.test, tokens);

        // Handle optional assertion message
        if let Some(msg) = &assert_stmt.msg {
            tokens.push(GenericToken::operator(",".to_string()));
            analyzer.extract_tokens_from_expr(msg, tokens);
        }
    }
}

// Definition Statement Processors
impl StatementProcessor {
    /// Process function definition - extracts function tokens and body
    fn process_function_def(
        analyzer: &PythonEntropyAnalyzer,
        func_def: &ast::StmtFunctionDef,
        tokens: &mut Vec<GenericToken>,
    ) {
        tokens.push(GenericToken::keyword("def".to_string()));
        tokens.push(GenericToken::identifier(normalize_identifier(
            &func_def.name,
        )));

        // Process decorators
        for decorator in &func_def.decorator_list {
            tokens.push(GenericToken::keyword("@".to_string()));
            analyzer.extract_tokens_from_expr(decorator, tokens);
        }

        // Process parameters
        Self::process_function_parameters(&func_def.args, tokens);

        // Process return annotation
        if let Some(returns) = &func_def.returns {
            tokens.push(GenericToken::operator("->".to_string()));
            analyzer.extract_tokens_from_expr(returns, tokens);
        }

        analyzer.process_stmt_body(&func_def.body, tokens);
    }

    /// Process async function definition - similar to regular function but with async
    fn process_async_function_def(
        analyzer: &PythonEntropyAnalyzer,
        func_def: &ast::StmtAsyncFunctionDef,
        tokens: &mut Vec<GenericToken>,
    ) {
        tokens.push(GenericToken::keyword("async".to_string()));
        tokens.push(GenericToken::keyword("def".to_string()));
        tokens.push(GenericToken::identifier(normalize_identifier(
            &func_def.name,
        )));

        // Process decorators
        for decorator in &func_def.decorator_list {
            tokens.push(GenericToken::keyword("@".to_string()));
            analyzer.extract_tokens_from_expr(decorator, tokens);
        }

        // Process parameters
        Self::process_function_parameters(&func_def.args, tokens);

        // Process return annotation
        if let Some(returns) = &func_def.returns {
            tokens.push(GenericToken::operator("->".to_string()));
            analyzer.extract_tokens_from_expr(returns, tokens);
        }

        analyzer.process_stmt_body(&func_def.body, tokens);
    }

    /// Process class definition - extracts class tokens and body
    fn process_class_def(
        analyzer: &PythonEntropyAnalyzer,
        class_def: &ast::StmtClassDef,
        tokens: &mut Vec<GenericToken>,
    ) {
        tokens.push(GenericToken::keyword("class".to_string()));
        tokens.push(GenericToken::identifier(normalize_identifier(
            &class_def.name,
        )));

        // Process decorators
        for decorator in &class_def.decorator_list {
            tokens.push(GenericToken::keyword("@".to_string()));
            analyzer.extract_tokens_from_expr(decorator, tokens);
        }

        // Process base classes
        if !class_def.bases.is_empty() {
            for (i, base) in class_def.bases.iter().enumerate() {
                if i > 0 {
                    tokens.push(GenericToken::operator(",".to_string()));
                }
                analyzer.extract_tokens_from_expr(base, tokens);
            }
        }

        // Process keyword arguments (like metaclass)
        for keyword in &class_def.keywords {
            if let Some(arg) = &keyword.arg {
                tokens.push(GenericToken::identifier(normalize_identifier(arg)));
                tokens.push(GenericToken::operator("=".to_string()));
            }
            analyzer.extract_tokens_from_expr(&keyword.value, tokens);
        }

        analyzer.process_stmt_body(&class_def.body, tokens);
    }

    /// Helper function to process function parameters
    fn process_function_parameters(parameters: &ast::Arguments, tokens: &mut Vec<GenericToken>) {
        // Process positional arguments
        for (i, arg) in parameters.args.iter().enumerate() {
            if i > 0 {
                tokens.push(GenericToken::operator(",".to_string()));
            }
            tokens.push(GenericToken::identifier(normalize_identifier(
                arg.def.arg.as_ref(),
            )));
        }

        // Process *args
        if let Some(vararg) = &parameters.vararg {
            if !parameters.args.is_empty() {
                tokens.push(GenericToken::operator(",".to_string()));
            }
            tokens.push(GenericToken::operator("*".to_string()));
            tokens.push(GenericToken::identifier(normalize_identifier(&vararg.arg)));
        }

        // Process keyword-only arguments
        for (i, arg) in parameters.kwonlyargs.iter().enumerate() {
            if i > 0 || !parameters.args.is_empty() || parameters.vararg.is_some() {
                tokens.push(GenericToken::operator(",".to_string()));
            }
            tokens.push(GenericToken::identifier(normalize_identifier(
                arg.def.arg.as_ref(),
            )));
        }

        // Process **kwargs
        if let Some(kwarg) = &parameters.kwarg {
            if !parameters.args.is_empty()
                || parameters.vararg.is_some()
                || !parameters.kwonlyargs.is_empty()
            {
                tokens.push(GenericToken::operator(",".to_string()));
            }
            tokens.push(GenericToken::operator("**".to_string()));
            tokens.push(GenericToken::identifier(normalize_identifier(&kwarg.arg)));
        }
    }
}

// Assignment Statement Processors
impl StatementProcessor {
    /// Process regular assignment - extracts assignment tokens
    fn process_assign_stmt(
        analyzer: &PythonEntropyAnalyzer,
        assign_stmt: &ast::StmtAssign,
        tokens: &mut Vec<GenericToken>,
    ) {
        // Process all targets
        for (i, target) in assign_stmt.targets.iter().enumerate() {
            if i > 0 {
                tokens.push(GenericToken::operator("=".to_string()));
            }
            analyzer.extract_tokens_from_expr(target, tokens);
        }

        tokens.push(GenericToken::operator("=".to_string()));
        analyzer.extract_tokens_from_expr(&assign_stmt.value, tokens);
    }

    /// Process augmented assignment (+=, -=, etc.) - extracts operator tokens
    fn process_aug_assign_stmt(
        analyzer: &PythonEntropyAnalyzer,
        aug_assign: &ast::StmtAugAssign,
        tokens: &mut Vec<GenericToken>,
    ) {
        analyzer.extract_tokens_from_expr(&aug_assign.target, tokens);

        // Convert operator to string representation
        let op_str = Self::augmented_operator_to_string(&aug_assign.op);
        tokens.push(GenericToken::operator(format!("{}=", op_str)));

        analyzer.extract_tokens_from_expr(&aug_assign.value, tokens);
    }

    /// Process annotated assignment - extracts type annotation and assignment
    fn process_ann_assign_stmt(
        analyzer: &PythonEntropyAnalyzer,
        ann_assign: &ast::StmtAnnAssign,
        tokens: &mut Vec<GenericToken>,
    ) {
        analyzer.extract_tokens_from_expr(&ann_assign.target, tokens);

        // Add type annotation
        tokens.push(GenericToken::operator(":".to_string()));
        analyzer.extract_tokens_from_expr(&ann_assign.annotation, tokens);

        // Add assignment if present
        if let Some(value) = &ann_assign.value {
            tokens.push(GenericToken::operator("=".to_string()));
            analyzer.extract_tokens_from_expr(value, tokens);
        }
    }

    /// Convert augmented assignment operator to string
    fn augmented_operator_to_string(op: &ast::Operator) -> &'static str {
        use ast::Operator::*;
        match op {
            Add => "+",
            Sub => "-",
            Mult => "*",
            MatMult => "@",
            Div => "/",
            Mod => "%",
            Pow => "**",
            LShift => "<<",
            RShift => ">>",
            BitOr => "|",
            BitXor => "^",
            BitAnd => "&",
            FloorDiv => "//",
        }
    }
}

// Other Statement Processors
impl StatementProcessor {
    /// Process statements not covered by specific handlers
    fn process_other_stmt(stmt: &ast::Stmt, tokens: &mut Vec<GenericToken>) {
        use ast::Stmt::*;
        match stmt {
            Pass(_) => tokens.push(GenericToken::keyword("pass".to_string())),
            Break(_) => tokens.push(GenericToken::keyword("break".to_string())),
            Continue(_) => tokens.push(GenericToken::keyword("continue".to_string())),
            Global(global) => {
                tokens.push(GenericToken::keyword("global".to_string()));
                for (i, name) in global.names.iter().enumerate() {
                    if i > 0 {
                        tokens.push(GenericToken::operator(",".to_string()));
                    }
                    tokens.push(GenericToken::identifier(normalize_identifier(name)));
                }
            }
            Nonlocal(nonlocal) => {
                tokens.push(GenericToken::keyword("nonlocal".to_string()));
                for (i, name) in nonlocal.names.iter().enumerate() {
                    if i > 0 {
                        tokens.push(GenericToken::operator(",".to_string()));
                    }
                    tokens.push(GenericToken::identifier(normalize_identifier(name)));
                }
            }
            Delete(delete) => {
                tokens.push(GenericToken::keyword("del".to_string()));
                for (i, _target) in delete.targets.iter().enumerate() {
                    if i > 0 {
                        tokens.push(GenericToken::operator(",".to_string()));
                    }
                    // For delete targets, we'll add a simple identifier token
                    tokens.push(GenericToken::identifier("target".to_string()));
                }
            }
            Import(import) => {
                tokens.push(GenericToken::keyword("import".to_string()));
                for (i, alias) in import.names.iter().enumerate() {
                    if i > 0 {
                        tokens.push(GenericToken::operator(",".to_string()));
                    }
                    tokens.push(GenericToken::identifier(normalize_identifier(&alias.name)));
                    if let Some(asname) = &alias.asname {
                        tokens.push(GenericToken::keyword("as".to_string()));
                        tokens.push(GenericToken::identifier(normalize_identifier(asname)));
                    }
                }
            }
            ImportFrom(import_from) => {
                tokens.push(GenericToken::keyword("from".to_string()));
                if let Some(module) = &import_from.module {
                    tokens.push(GenericToken::identifier(normalize_identifier(module)));
                }
                tokens.push(GenericToken::keyword("import".to_string()));

                for (i, alias) in import_from.names.iter().enumerate() {
                    if i > 0 {
                        tokens.push(GenericToken::operator(",".to_string()));
                    }
                    tokens.push(GenericToken::identifier(normalize_identifier(&alias.name)));
                    if let Some(asname) = &alias.asname {
                        tokens.push(GenericToken::keyword("as".to_string()));
                        tokens.push(GenericToken::identifier(normalize_identifier(asname)));
                    }
                }
            }
            _ => {
                // For any other statement types, add a generic token
                tokens.push(GenericToken::keyword("stmt".to_string()));
            }
        }
    }
}
