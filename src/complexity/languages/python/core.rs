use crate::complexity::entropy_core::{LanguageEntropyAnalyzer, PatternMetrics};
use crate::complexity::entropy_traits::GenericToken;
use rustpython_parser::ast;
use std::collections::HashSet;

use super::expressions::ExpressionProcessor;
use super::statements::StatementProcessor;

/// Categories for expression types to simplify pattern matching
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprCategory {
    Operator,
    ControlFlow,
    Comprehension,
    Literal,
    Collection,
    Access,
    Special,
}

/// Python-specific entropy analyzer implementation
pub struct PythonEntropyAnalyzer<'a> {
    _source: &'a str,
}

impl<'a> PythonEntropyAnalyzer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { _source: source }
    }

    /// Extract tokens from Python AST statements - pure function
    pub fn extract_python_tokens(&self, stmts: &[ast::Stmt]) -> Vec<GenericToken> {
        stmts
            .iter()
            .flat_map(|stmt| self.extract_tokens_from_stmt(stmt))
            .collect()
    }

    /// Extract tokens from a single statement - pure function
    pub fn extract_tokens_from_stmt(&self, stmt: &ast::Stmt) -> Vec<GenericToken> {
        let mut tokens = Vec::new();
        StatementProcessor::process_statement(self, stmt, &mut tokens);
        tokens
    }

    /// Extract tokens from an expression - delegates to expression processor
    pub fn extract_tokens_from_expr(&self, expr: &ast::Expr, tokens: &mut Vec<GenericToken>) {
        ExpressionProcessor::process_expression(self, expr, tokens);
    }

    /// Process a statement body
    pub fn process_stmt_body(&self, body: &[ast::Stmt], tokens: &mut Vec<GenericToken>) {
        for stmt in body {
            StatementProcessor::process_statement(self, stmt, tokens);
        }
    }

    /// Detect patterns in Python code - pure function
    pub fn detect_python_patterns(&self, stmts: &[ast::Stmt]) -> Vec<String> {
        stmts
            .iter()
            .flat_map(|stmt| self.detect_patterns_in_stmt(stmt))
            .collect()
    }

    fn detect_patterns_in_stmt(&self, stmt: &ast::Stmt) -> Vec<String> {
        use ast::Stmt::*;
        match stmt {
            If(if_stmt) => {
                let mut patterns = vec!["if".to_string()];
                patterns.extend(self.detect_patterns_in_expr(&if_stmt.test));
                patterns.extend(if_stmt.body.iter().flat_map(|s| self.detect_patterns_in_stmt(s)));
                patterns.extend(if_stmt.orelse.iter().flat_map(|s| self.detect_patterns_in_stmt(s)));
                patterns
            }
            While(while_stmt) => {
                let mut patterns = vec!["while".to_string()];
                patterns.extend(self.detect_patterns_in_expr(&while_stmt.test));
                patterns.extend(while_stmt.body.iter().flat_map(|s| self.detect_patterns_in_stmt(s)));
                patterns
            }
            For(for_stmt) => {
                let mut patterns = vec!["for".to_string()];
                patterns.extend(self.detect_patterns_in_expr(&for_stmt.iter));
                patterns.extend(for_stmt.body.iter().flat_map(|s| self.detect_patterns_in_stmt(s)));
                patterns
            }
            FunctionDef(func_def) => {
                let mut patterns = vec![format!("def:{}", func_def.name)];
                patterns.extend(func_def.body.iter().flat_map(|s| self.detect_patterns_in_stmt(s)));
                patterns
            }
            ClassDef(class_def) => {
                let mut patterns = vec![format!("class:{}", class_def.name)];
                patterns.extend(class_def.body.iter().flat_map(|s| self.detect_patterns_in_stmt(s)));
                patterns
            }
            Expr(expr_stmt) => self.detect_patterns_in_expr(&expr_stmt.value),
            _ => vec![],
        }
    }

    fn detect_patterns_in_expr(&self, expr: &ast::Expr) -> Vec<String> {
        ExpressionProcessor::detect_expression_patterns(expr)
    }

    /// Calculate branch similarity for Python code - pure function
    pub fn calculate_python_branch_similarity(&self, stmts: &[ast::Stmt]) -> f64 {
        let branch_groups = self.collect_branch_groups(stmts);

        if branch_groups.is_empty() {
            return 0.0;
        }

        let similarities: Vec<f64> = branch_groups
            .iter()
            .map(|group| group.calculate_similarity())
            .collect();

        similarities.iter().sum::<f64>() / similarities.len() as f64
    }

    fn collect_branch_groups(&self, stmts: &[ast::Stmt]) -> Vec<BranchGroup> {
        stmts
            .iter()
            .filter_map(|stmt| self.extract_branch_group(stmt))
            .collect()
    }

    fn extract_branch_group(&self, stmt: &ast::Stmt) -> Option<BranchGroup> {
        use ast::Stmt::*;
        match stmt {
            If(if_stmt) => {
                let mut group = BranchGroup::new();

                // Add if branch
                let if_tokens = if_stmt.body.iter()
                    .flat_map(|s| self.extract_tokens_from_stmt(s))
                    .collect();
                group.add_branch(if_tokens);

                // Add else branch if present
                if !if_stmt.orelse.is_empty() {
                    let else_tokens = if_stmt.orelse.iter()
                        .flat_map(|s| self.extract_tokens_from_stmt(s))
                        .collect();
                    group.add_branch(else_tokens);
                }

                Some(group)
            }
            Match(match_stmt) => {
                let mut group = BranchGroup::new();
                for case in &match_stmt.cases {
                    let case_tokens = case.body.iter()
                        .flat_map(|s| self.extract_tokens_from_stmt(s))
                        .collect();
                    group.add_branch(case_tokens);
                }
                Some(group)
            }
            _ => None,
        }
    }

    /// Count unique variables in the code - pure function
    pub fn count_unique_variables(&self, stmts: &[ast::Stmt]) -> usize {
        let variables = self.collect_variables(stmts);
        variables.len()
    }

    fn collect_variables(&self, stmts: &[ast::Stmt]) -> HashSet<String> {
        stmts
            .iter()
            .flat_map(|stmt| self.collect_variables_from_stmt(stmt))
            .collect()
    }

    fn collect_variables_from_stmt(&self, stmt: &ast::Stmt) -> HashSet<String> {
        use ast::Stmt::*;
        match stmt {
            Assign(assign) => {
                let mut vars = HashSet::new();
                for target in &assign.targets {
                    vars.extend(self.collect_variables_from_expr(target));
                }
                vars
            }
            AnnAssign(ann_assign) => self.collect_variables_from_expr(&ann_assign.target),
            For(for_stmt) => {
                let mut vars = self.collect_variables_from_expr(&for_stmt.target);
                vars.extend(for_stmt.body.iter().flat_map(|s| self.collect_variables_from_stmt(s)));
                vars
            }
            FunctionDef(func_def) => {
                let mut vars = HashSet::new();
                vars.insert(func_def.name.to_string());
                for param in &func_def.args.args {
                    vars.insert(param.def.arg.to_string());
                }
                for stmt in &func_def.body {
                    vars.extend(self.collect_variables_from_stmt(stmt));
                }
                vars
            }
            _ => HashSet::new(),
        }
    }

    fn collect_variables_from_expr(&self, expr: &ast::Expr) -> HashSet<String> {
        ExpressionProcessor::collect_variables_from_expression(expr)
    }

    /// Calculate maximum nesting depth - pure function
    pub fn calculate_max_nesting(&self, stmts: &[ast::Stmt]) -> u32 {
        stmts
            .iter()
            .map(|stmt| self.calculate_stmt_nesting(stmt, 0))
            .max()
            .unwrap_or(0)
    }

    fn calculate_stmt_nesting(&self, stmt: &ast::Stmt, current_depth: u32) -> u32 {
        use ast::Stmt::*;
        match stmt {
            If(if_stmt) => {
                let if_depth = if_stmt.body.iter()
                    .map(|s| self.calculate_stmt_nesting(s, current_depth + 1))
                    .max()
                    .unwrap_or(current_depth + 1);

                let else_depth = if_stmt.orelse.iter()
                    .map(|s| self.calculate_stmt_nesting(s, current_depth + 1))
                    .max()
                    .unwrap_or(current_depth);

                if_depth.max(else_depth)
            }
            While(while_stmt) => {
                while_stmt.body.iter()
                    .map(|s| self.calculate_stmt_nesting(s, current_depth + 1))
                    .max()
                    .unwrap_or(current_depth + 1)
            }
            For(for_stmt) => {
                for_stmt.body.iter()
                    .map(|s| self.calculate_stmt_nesting(s, current_depth + 1))
                    .max()
                    .unwrap_or(current_depth + 1)

            }
            FunctionDef(func_def) => {
                func_def.body.iter()
                    .map(|s| self.calculate_stmt_nesting(s, current_depth + 1))
                    .max()
                    .unwrap_or(current_depth + 1)
            }
            AsyncFunctionDef(func_def) => {
                func_def.body.iter()
                    .map(|s| self.calculate_stmt_nesting(s, current_depth + 1))
                    .max()
                    .unwrap_or(current_depth + 1)
            }
            ClassDef(class_def) => {
                class_def.body.iter()
                    .map(|s| self.calculate_stmt_nesting(s, current_depth + 1))
                    .max()
                    .unwrap_or(current_depth + 1)
            }
            _ => current_depth,
        }
    }
}

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
pub struct BranchGroup {
    branches: Vec<Vec<GenericToken>>,
}

impl BranchGroup {
    pub fn new() -> Self {
        Self {
            branches: Vec::new(),
        }
    }

    pub fn add_branch(&mut self, tokens: Vec<GenericToken>) {
        self.branches.push(tokens);
    }

    pub fn calculate_similarity(&self) -> f64 {
        if self.branches.len() < 2 {
            return 0.0;
        }

        let mut total_similarity = 0.0;
        let mut comparisons = 0;

        for i in 0..self.branches.len() {
            for j in i + 1..self.branches.len() {
                total_similarity += Self::token_similarity(&self.branches[i], &self.branches[j]);
                comparisons += 1;
            }
        }

        if comparisons > 0 {
            total_similarity / comparisons as f64
        } else {
            0.0
        }
    }

    fn token_similarity(tokens1: &[GenericToken], tokens2: &[GenericToken]) -> f64 {
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

/// Normalize identifier names to reduce noise - pure function
pub fn normalize_identifier(name: &str) -> String {
    if name.len() > 3 {
        "VAR".to_string()
    } else {
        name.to_uppercase()
    }
}