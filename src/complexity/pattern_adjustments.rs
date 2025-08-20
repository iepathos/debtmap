use quote::ToTokens;
use syn::{visit::Visit, Block, Expr, ExprIf, Stmt};

/// Information about detected pattern matching
#[derive(Debug, Clone)]
pub struct PatternMatchInfo {
    pub variable_name: String,
    pub condition_count: usize,
    pub has_default: bool,
    pub pattern_type: PatternType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternType {
    StringMatching,        // ends_with, starts_with patterns
    EnumMatching,          // Matching against enum variants
    RangeMatching,         // Numeric range checks
    TypeChecking,          // instanceof or type checks
    SimpleComparison,      // Simple equality/inequality checks
    TraitDelegation,       // Trait method delegation patterns
    SerializationDispatch, // Serialization/encoding dispatch patterns
}

/// Trait for pattern recognition
pub trait PatternRecognizer {
    fn detect(&self, block: &Block) -> Option<PatternMatchInfo>;
    fn adjust_complexity(&self, info: &PatternMatchInfo, base: u32) -> u32;
}

/// Recognizes pattern matching structures
pub struct PatternMatchRecognizer;

impl Default for PatternMatchRecognizer {
    fn default() -> Self {
        Self
    }
}

impl PatternMatchRecognizer {
    pub fn new() -> Self {
        Self
    }

    /// Check if an expression has an immediate return
    fn has_immediate_return(&self, block: &Block) -> bool {
        // Check if the block has only one or two statements and includes a return
        if block.stmts.is_empty() || block.stmts.len() > 2 {
            return false;
        }

        // Check if first or only statement is a return
        block
            .stmts
            .iter()
            .any(|stmt| matches!(stmt, Stmt::Expr(Expr::Return(_), _)))
    }

    /// Extract the variable being tested in a condition
    fn extract_tested_variable(&self, expr: &Expr) -> Option<String> {
        match expr {
            // path.ends_with(), path.starts_with(), etc.
            Expr::MethodCall(method) => {
                if let Expr::Path(path) = &*method.receiver {
                    if let Some(ident) = path.path.get_ident() {
                        return Some(ident.to_string());
                    }
                }
                // Also handle field access like self.field.method()
                if let Expr::Field(field) = &*method.receiver {
                    return self.extract_field_path(&field.base);
                }
                None
            }
            // var == value, var != value
            Expr::Binary(binary) => {
                if let Expr::Path(path) = &*binary.left {
                    if let Some(ident) = path.path.get_ident() {
                        return Some(ident.to_string());
                    }
                }
                // Also check right side for commutative operations
                if let Expr::Path(path) = &*binary.right {
                    if let Some(ident) = path.path.get_ident() {
                        return Some(ident.to_string());
                    }
                }
                None
            }
            // !var or other unary expressions
            Expr::Unary(unary) => self.extract_tested_variable(&unary.expr),
            // Grouped expressions (var)
            Expr::Paren(paren) => self.extract_tested_variable(&paren.expr),
            // Direct path reference
            Expr::Path(path) => path.path.get_ident().map(|ident| ident.to_string()),
            _ => None,
        }
    }

    /// Extract field path like self.field or obj.field
    fn extract_field_path(&self, expr: &Expr) -> Option<String> {
        let mut path_parts = Vec::new();
        let mut current = expr;

        loop {
            match current {
                Expr::Field(field) => {
                    path_parts.push(field.member.to_token_stream().to_string());
                    current = &field.base;
                }
                Expr::Path(path) => {
                    if let Some(ident) = path.path.get_ident() {
                        path_parts.push(ident.to_string());
                    }
                    break;
                }
                _ => break,
            }
        }

        if !path_parts.is_empty() {
            path_parts.reverse();
            Some(path_parts.join("."))
        } else {
            None
        }
    }

    /// Detect the type of pattern being matched
    fn detect_pattern_type(&self, expr: &Expr) -> PatternType {
        match expr {
            Expr::MethodCall(method) => {
                let method_name = method.method.to_string();
                match method_name.as_str() {
                    "ends_with" | "starts_with" | "contains" => PatternType::StringMatching,
                    _ => PatternType::SimpleComparison,
                }
            }
            Expr::Binary(binary) => {
                use syn::BinOp;
                match binary.op {
                    BinOp::Eq(_) | BinOp::Ne(_) => PatternType::SimpleComparison,
                    BinOp::Lt(_) | BinOp::Le(_) | BinOp::Gt(_) | BinOp::Ge(_) => {
                        PatternType::RangeMatching
                    }
                    _ => PatternType::SimpleComparison,
                }
            }
            _ => PatternType::SimpleComparison,
        }
    }

    /// Check if a block contains a pattern matching structure
    fn detect_pattern_matching(&self, block: &Block) -> Option<PatternMatchInfo> {
        let mut conditions = Vec::new();
        let mut variable_name: Option<String> = None;
        let mut pattern_types = Vec::new();
        let mut has_else = false;

        // Analyze each statement in the block
        for stmt in &block.stmts {
            match stmt {
                Stmt::Expr(Expr::If(if_expr), _) => {
                    if let Some(info) = self.analyze_if_chain(if_expr, &mut variable_name) {
                        conditions.extend(info.0);
                        pattern_types.extend(info.1);
                        has_else = info.2;
                    } else {
                        // If we encounter an if statement that doesn't match the pattern, bail
                        if !conditions.is_empty() {
                            break;
                        }
                    }
                }
                _ => {
                    // Non-if statements break the pattern matching sequence
                    if !conditions.is_empty() {
                        break;
                    }
                }
            }
        }

        // Require at least 3 conditions to consider it pattern matching
        if conditions.len() >= 3 {
            // Determine the dominant pattern type
            let pattern_type = pattern_types
                .iter()
                .max_by_key(|t| pattern_types.iter().filter(|pt| pt == t).count())
                .cloned()
                .unwrap_or(PatternType::SimpleComparison);

            Some(PatternMatchInfo {
                variable_name: variable_name.unwrap_or_else(|| String::from("unknown")),
                condition_count: conditions.len(),
                has_default: has_else,
                pattern_type,
            })
        } else {
            None
        }
    }

    /// Analyze an if-else chain to see if it's pattern matching
    fn analyze_if_chain(
        &self,
        if_expr: &ExprIf,
        tracked_var: &mut Option<String>,
    ) -> Option<(Vec<()>, Vec<PatternType>, bool)> {
        let mut conditions = Vec::new();
        let mut pattern_types = Vec::new();
        let mut has_else = false;
        let mut current_if = if_expr;

        loop {
            // Extract the variable being tested
            if let Some(var) = self.extract_tested_variable(&current_if.cond) {
                if tracked_var.is_none() {
                    *tracked_var = Some(var.clone());
                } else if tracked_var.as_ref() != Some(&var) {
                    // Different variable, not pattern matching
                    return None;
                }

                // Check for immediate return in then branch
                if !self.has_immediate_return(&current_if.then_branch) {
                    // Allow simple single expressions too (common in pattern matching)
                    if current_if.then_branch.stmts.len() != 1 {
                        return None;
                    }
                }

                conditions.push(());
                pattern_types.push(self.detect_pattern_type(&current_if.cond));

                // Check else branch
                match &current_if.else_branch {
                    Some((_, else_expr)) => match &**else_expr {
                        Expr::If(next_if) => {
                            current_if = next_if;
                            continue;
                        }
                        Expr::Block(_block) => {
                            // This is the final else block
                            has_else = true;
                            break;
                        }
                        _ => {
                            has_else = true;
                            break;
                        }
                    },
                    None => break,
                }
            } else {
                // Couldn't extract variable, not pattern matching
                return None;
            }
        }

        if !conditions.is_empty() {
            Some((conditions, pattern_types, has_else))
        } else {
            None
        }
    }
}

impl PatternRecognizer for PatternMatchRecognizer {
    fn detect(&self, block: &Block) -> Option<PatternMatchInfo> {
        self.detect_pattern_matching(block)
    }

    fn adjust_complexity(&self, info: &PatternMatchInfo, _base: u32) -> u32 {
        // Use logarithmic scaling for pattern matching
        let adjusted = (info.condition_count as f32).log2().ceil() as u32;

        // Small penalty for missing default case
        let default_penalty = if !info.has_default { 1 } else { 0 };

        // The adjusted complexity replaces the base complexity
        adjusted + default_penalty
    }
}

/// Recognizes simple delegation patterns
pub struct SimpleDelegationRecognizer;

impl Default for SimpleDelegationRecognizer {
    fn default() -> Self {
        Self
    }
}

impl SimpleDelegationRecognizer {
    pub fn new() -> Self {
        Self
    }

    /// Check if a block is a simple delegation (cyclomatic complexity 1)
    fn is_simple_delegation(&self, block: &Block) -> bool {
        // Must have at least one statement
        if block.stmts.is_empty() {
            return false;
        }

        // Count control flow statements
        let mut control_flow_count = 0;
        let mut visitor = ControlFlowCounter {
            count: &mut control_flow_count,
        };
        visitor.visit_block(block);

        // Simple delegation has no control flow (cyclomatic complexity = 1)
        // AND must have at least 2 statements (transformation + return)
        control_flow_count == 0 && block.stmts.len() >= 2
    }
}

impl PatternRecognizer for SimpleDelegationRecognizer {
    fn detect(&self, block: &Block) -> Option<PatternMatchInfo> {
        if self.is_simple_delegation(block) {
            Some(PatternMatchInfo {
                variable_name: String::from("delegation"),
                condition_count: 0,
                has_default: true,
                pattern_type: PatternType::SimpleComparison,
            })
        } else {
            None
        }
    }

    fn adjust_complexity(&self, _info: &PatternMatchInfo, _base: u32) -> u32 {
        // Simple delegation has minimal cognitive complexity
        1
    }
}

/// Helper visitor to count control flow statements
struct ControlFlowCounter<'a> {
    count: &'a mut u32,
}

impl<'ast> Visit<'ast> for ControlFlowCounter<'_> {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            Expr::If(_) | Expr::While(_) | Expr::ForLoop(_) | Expr::Loop(_) | Expr::Match(_) => {
                *self.count += 1;
            }
            Expr::Binary(binary) => {
                use syn::BinOp;
                if matches!(binary.op, BinOp::And(_) | BinOp::Or(_)) {
                    *self.count += 1;
                }
            }
            _ => {}
        }
        syn::visit::visit_expr(self, expr);
    }
}

/// Calculate adjusted cognitive complexity using pattern recognition
pub fn calculate_cognitive_adjusted(block: &Block, base_complexity: u32) -> u32 {
    use super::match_patterns::MatchExpressionRecognizer;

    // First check for match expressions in the block
    for stmt in &block.stmts {
        if let Stmt::Expr(Expr::Match(_match_expr), _) = stmt {
            let recognizer = MatchExpressionRecognizer::new();
            // Create a temporary block with just the match expression
            let temp_block = syn::Block {
                brace_token: block.brace_token,
                stmts: vec![stmt.clone()],
            };
            if let Some(info) = recognizer.detect(&temp_block) {
                return recognizer.adjust_complexity(&info, base_complexity);
            }
        }
    }

    let recognizers: Vec<Box<dyn PatternRecognizer>> = vec![
        Box::new(MatchExpressionRecognizer::new()),
        Box::new(PatternMatchRecognizer::new()),
        Box::new(SimpleDelegationRecognizer::new()),
    ];

    for recognizer in recognizers {
        if let Some(info) = recognizer.detect(block) {
            return recognizer.adjust_complexity(&info, base_complexity);
        }
    }

    // Fall back to standard calculation
    base_complexity
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_pattern_matching_detection() {
        let block: Block = parse_quote! {{
            if path.ends_with(".rs") {
                return FileType::Rust;
            }
            if path.ends_with(".py") {
                return FileType::Python;
            }
            if path.ends_with(".js") {
                return FileType::JavaScript;
            }
            if path.ends_with(".ts") {
                return FileType::TypeScript;
            }
        }};

        let recognizer = PatternMatchRecognizer::new();
        let info = recognizer.detect(&block);

        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.condition_count, 4);
        assert_eq!(info.pattern_type, PatternType::StringMatching);
        assert!(!info.has_default);
    }

    #[test]
    fn test_simple_delegation_detection() {
        let block: Block = parse_quote! {{
            let result = calculate_something(x, y, z);
            transform_result(result)
        }};

        let recognizer = SimpleDelegationRecognizer::new();
        let info = recognizer.detect(&block);

        assert!(info.is_some());
    }

    #[test]
    fn test_logarithmic_scaling() {
        let info = PatternMatchInfo {
            variable_name: "test".to_string(),
            condition_count: 8,
            has_default: true,
            pattern_type: PatternType::StringMatching,
        };

        let recognizer = PatternMatchRecognizer::new();
        let adjusted = recognizer.adjust_complexity(&info, 8);

        // log2(8) = 3, so adjusted should be 3
        assert_eq!(adjusted, 3);
    }

    #[test]
    fn test_non_pattern_matching() {
        let block: Block = parse_quote! {{
            if x > 0 {
                if y > 0 {
                    return true;
                }
            }
            return false;
        }};

        let recognizer = PatternMatchRecognizer::new();
        let info = recognizer.detect(&block);

        assert!(info.is_none());
    }
}
