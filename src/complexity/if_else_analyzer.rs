use serde::{Deserialize, Serialize};
use syn::{visit::Visit, Block, Expr, ExprIf, Pat, Stmt};

/// Information about an if-else chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfElseChain {
    pub start_line: usize,
    pub length: usize,
    pub variable_tested: Option<String>,
    pub condition_types: Vec<ConditionType>,
    pub has_final_else: bool,
    pub return_pattern: ReturnPattern,
}

/// Type of condition in if-else chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionType {
    Equality,
    Range,
    Pattern,
    Complex,
}

/// Pattern of returns in if-else chain
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReturnPattern {
    SimpleValues,
    SameTypeConstructors,
    SideEffects,
    Mixed,
}

/// Suggested refactoring pattern
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RefactoringPattern {
    MatchExpression,
    LookupTable,
    StrategyPattern,
    GuardClauses,
    PolymorphicDispatch,
    ExtractMethod,
}

impl RefactoringPattern {
    pub fn name(&self) -> &'static str {
        match self {
            RefactoringPattern::MatchExpression => "Match Expression",
            RefactoringPattern::LookupTable => "Lookup Table",
            RefactoringPattern::StrategyPattern => "Strategy Pattern",
            RefactoringPattern::GuardClauses => "Guard Clauses",
            RefactoringPattern::PolymorphicDispatch => "Polymorphic Dispatch",
            RefactoringPattern::ExtractMethod => "Extract Method",
        }
    }

    pub fn description(&self) -> String {
        match self {
            RefactoringPattern::MatchExpression => {
                "Convert if-else chain to match expression for better exhaustiveness checking and readability".to_string()
            }
            RefactoringPattern::LookupTable => {
                "Replace repeated value mapping with HashMap or static lookup table for O(1) access".to_string()
            }
            RefactoringPattern::StrategyPattern => {
                "Extract different behaviors into strategy objects or function pointers for better extensibility".to_string()
            }
            RefactoringPattern::GuardClauses => {
                "Use early returns to reduce nesting and improve readability".to_string()
            }
            RefactoringPattern::PolymorphicDispatch => {
                "Use trait objects or enums to dispatch behavior polymorphically".to_string()
            }
            RefactoringPattern::ExtractMethod => {
                "Extract complex conditional logic into separate, well-named functions".to_string()
            }
        }
    }

    pub fn estimated_effort(&self) -> super::message_generator::EstimatedEffort {
        use super::message_generator::EstimatedEffort;
        match self {
            RefactoringPattern::GuardClauses => EstimatedEffort::Low,
            RefactoringPattern::MatchExpression | RefactoringPattern::LookupTable => {
                EstimatedEffort::Low
            }
            RefactoringPattern::ExtractMethod => EstimatedEffort::Medium,
            RefactoringPattern::StrategyPattern | RefactoringPattern::PolymorphicDispatch => {
                EstimatedEffort::High
            }
        }
    }
}

impl IfElseChain {
    /// Suggest the best refactoring pattern for this if-else chain
    pub fn suggest_refactoring(&self) -> RefactoringPattern {
        match (&self.return_pattern, &self.condition_types[0]) {
            (ReturnPattern::SimpleValues, ConditionType::Equality) if self.length <= 5 => {
                RefactoringPattern::LookupTable
            }
            (ReturnPattern::SimpleValues, ConditionType::Equality) => {
                RefactoringPattern::MatchExpression
            }
            (ReturnPattern::SameTypeConstructors, _) => RefactoringPattern::MatchExpression,
            (ReturnPattern::SideEffects, _) if self.length > 5 => {
                RefactoringPattern::StrategyPattern
            }
            (_, ConditionType::Range) if !self.has_final_else => RefactoringPattern::GuardClauses,
            (_, ConditionType::Complex) if self.length > 3 => RefactoringPattern::ExtractMethod,
            _ => RefactoringPattern::MatchExpression,
        }
    }
}

/// Analyzer for detecting if-else chains
pub struct IfElseChainAnalyzer {
    chains: Vec<IfElseChain>,
    current_chain: Option<IfElseChainBuilder>,
}

struct IfElseChainBuilder {
    start_line: usize,
    length: usize,
    variable_tested: Option<String>,
    condition_types: Vec<ConditionType>,
    has_final_else: bool,
    return_types: Vec<ReturnType>,
}

#[derive(Debug, Clone)]
enum ReturnType {
    Value,
    Constructor,
    SideEffect,
    None,
}

impl IfElseChainAnalyzer {
    pub fn new() -> Self {
        Self {
            chains: Vec::new(),
            current_chain: None,
        }
    }

    /// Analyze a block to find if-else chains
    pub fn analyze_block(&mut self, block: &Block) -> Vec<IfElseChain> {
        self.visit_block(block);
        self.finalize_current_chain();
        self.chains.clone()
    }

    fn start_chain(&mut self, expr_if: &ExprIf) {
        let condition_type = Self::analyze_condition(&expr_if.cond);
        let variable = Self::extract_tested_variable(&expr_if.cond);

        self.current_chain = Some(IfElseChainBuilder {
            start_line: 1, // Would use span info in real implementation
            length: 1,
            variable_tested: variable,
            condition_types: vec![condition_type],
            has_final_else: false,
            return_types: Vec::new(),
        });

        // Analyze the then branch
        if let Some(builder) = &mut self.current_chain {
            builder
                .return_types
                .push(Self::analyze_block_return(&expr_if.then_branch));
        }
    }

    fn extend_chain(&mut self, expr_if: &ExprIf) {
        if let Some(builder) = &mut self.current_chain {
            builder.length += 1;
            builder
                .condition_types
                .push(Self::analyze_condition(&expr_if.cond));
            builder
                .return_types
                .push(Self::analyze_block_return(&expr_if.then_branch));

            // Check if the variable being tested is consistent
            if let Some(var) = Self::extract_tested_variable(&expr_if.cond) {
                if builder.variable_tested.is_none() {
                    builder.variable_tested = Some(var);
                }
            }
        }
    }

    fn finalize_current_chain(&mut self) {
        if let Some(builder) = self.current_chain.take() {
            if builder.length >= 2 {
                // Only record chains with at least 2 conditions
                let return_pattern = Self::determine_return_pattern(&builder.return_types);

                self.chains.push(IfElseChain {
                    start_line: builder.start_line,
                    length: builder.length,
                    variable_tested: builder.variable_tested,
                    condition_types: builder.condition_types,
                    has_final_else: builder.has_final_else,
                    return_pattern,
                });
            }
        }
    }

    fn analyze_condition(cond: &Expr) -> ConditionType {
        match cond {
            Expr::Binary(binary) => match &binary.op {
                syn::BinOp::Eq(_) | syn::BinOp::Ne(_) => ConditionType::Equality,
                syn::BinOp::Lt(_) | syn::BinOp::Le(_) | syn::BinOp::Gt(_) | syn::BinOp::Ge(_) => {
                    ConditionType::Range
                }
                syn::BinOp::And(_) | syn::BinOp::Or(_) => ConditionType::Complex,
                _ => ConditionType::Complex,
            },
            Expr::Let(_) => ConditionType::Pattern,
            Expr::MethodCall(_) | Expr::Call(_) => ConditionType::Complex,
            _ => ConditionType::Complex,
        }
    }

    fn extract_tested_variable(cond: &Expr) -> Option<String> {
        match cond {
            Expr::Binary(binary) => {
                // Try to extract variable from left side
                match &*binary.left {
                    Expr::Path(path) => path.path.segments.last().map(|seg| seg.ident.to_string()),
                    Expr::Field(field) => {
                        // Extract field access like self.field
                        match &*field.base {
                            Expr::Path(path) if path.path.is_ident("self") => match &field.member {
                                syn::Member::Named(ident) => Some(format!("self.{}", ident)),
                                syn::Member::Unnamed(index) => {
                                    Some(format!("self.{}", index.index))
                                }
                            },
                            _ => None,
                        }
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn analyze_block_return(block: &Block) -> ReturnType {
        // Check if block contains return statement
        for stmt in &block.stmts {
            match stmt {
                Stmt::Expr(Expr::Return(_), _) => return ReturnType::Value,
                Stmt::Expr(expr, _) => {
                    if Self::is_constructor_call(expr) {
                        return ReturnType::Constructor;
                    }
                    if Self::is_side_effect(expr) {
                        return ReturnType::SideEffect;
                    }
                }
                _ => {}
            }
        }
        ReturnType::None
    }

    fn is_constructor_call(expr: &Expr) -> bool {
        matches!(expr, Expr::Call(_) | Expr::Struct(_) | Expr::Path(_))
    }

    fn is_side_effect(expr: &Expr) -> bool {
        matches!(expr, Expr::MethodCall(_) | Expr::Assign(_) | Expr::Macro(_))
    }

    fn determine_return_pattern(return_types: &[ReturnType]) -> ReturnPattern {
        let all_values = return_types
            .iter()
            .all(|rt| matches!(rt, ReturnType::Value));
        let all_constructors = return_types
            .iter()
            .all(|rt| matches!(rt, ReturnType::Constructor));
        let all_side_effects = return_types
            .iter()
            .all(|rt| matches!(rt, ReturnType::SideEffect));

        if all_values {
            ReturnPattern::SimpleValues
        } else if all_constructors {
            ReturnPattern::SameTypeConstructors
        } else if all_side_effects {
            ReturnPattern::SideEffects
        } else {
            ReturnPattern::Mixed
        }
    }
}

impl<'ast> Visit<'ast> for IfElseChainAnalyzer {
    fn visit_expr_if(&mut self, expr_if: &'ast ExprIf) {
        // Check if this is part of an existing chain
        let is_else_if = self.current_chain.is_some();

        if is_else_if {
            self.extend_chain(expr_if);
        } else {
            self.start_chain(expr_if);
        }

        // Visit the then branch
        self.visit_block(&expr_if.then_branch);

        // Handle else branch
        if let Some((_else_token, else_expr)) = &expr_if.else_branch {
            match &**else_expr {
                Expr::If(nested_if) => {
                    // Continue the chain with else-if
                    self.visit_expr_if(nested_if);
                }
                Expr::Block(block) => {
                    // Final else block
                    if let Some(builder) = &mut self.current_chain {
                        builder.has_final_else = true;
                        builder
                            .return_types
                            .push(Self::analyze_block_return(&block.block));
                    }
                    self.visit_block(&block.block);
                    self.finalize_current_chain();
                }
                _ => {
                    self.visit_expr(else_expr);
                    self.finalize_current_chain();
                }
            }
        } else if !is_else_if {
            // No else branch, finalize the chain
            self.finalize_current_chain();
        }
    }

    fn visit_block(&mut self, block: &'ast Block) {
        for stmt in &block.stmts {
            self.visit_stmt(stmt);
        }
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        match stmt {
            Stmt::Expr(expr, _) => self.visit_expr(expr),
            Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    self.visit_expr(&init.expr);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_simple_if_else_chain() {
        let block: Block = parse_quote! {{
            if x == 1 {
                return "one";
            } else if x == 2 {
                return "two";
            } else if x == 3 {
                return "three";
            } else {
                return "other";
            }
        }};

        let mut analyzer = IfElseChainAnalyzer::new();
        let chains = analyzer.analyze_block(&block);

        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].length, 3);
        assert!(chains[0].has_final_else);
        assert_eq!(chains[0].return_pattern, ReturnPattern::SimpleValues);
    }

    #[test]
    fn test_suggest_refactoring() {
        let chain = IfElseChain {
            start_line: 1,
            length: 4,
            variable_tested: Some("value".to_string()),
            condition_types: vec![
                ConditionType::Equality,
                ConditionType::Equality,
                ConditionType::Equality,
                ConditionType::Equality,
            ],
            has_final_else: true,
            return_pattern: ReturnPattern::SimpleValues,
        };

        let pattern = chain.suggest_refactoring();
        assert_eq!(pattern, RefactoringPattern::LookupTable);
    }

    #[test]
    fn test_guard_clause_suggestion() {
        let chain = IfElseChain {
            start_line: 1,
            length: 3,
            variable_tested: Some("value".to_string()),
            condition_types: vec![
                ConditionType::Range,
                ConditionType::Range,
                ConditionType::Range,
            ],
            has_final_else: false,
            return_pattern: ReturnPattern::Mixed,
        };

        let pattern = chain.suggest_refactoring();
        assert_eq!(pattern, RefactoringPattern::GuardClauses);
    }
}
