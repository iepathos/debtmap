use super::match_patterns::MatchExpressionRecognizer;
use syn::{visit::Visit, Block, Expr, ExprMatch, ImplItem, Item, ItemImpl, Stmt};

/// Location information for a match expression found in the AST
#[derive(Debug, Clone)]
pub struct MatchLocation {
    pub line: usize,
    pub arms: usize,
    pub complexity: u32,
    pub context: ComplexityContext,
}

/// Context information about where a match expression was found
#[derive(Debug, Clone)]
pub struct ComplexityContext {
    pub in_closure: bool,
    pub in_async: bool,
    pub nesting_depth: u32,
    pub function_role: FunctionRole,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionRole {
    EntryPoint,
    CoreLogic,
    Utility,
    Test,
    Unknown,
}

/// Recursively detects all match expressions throughout the entire function AST
pub struct RecursiveMatchDetector {
    pub matches_found: Vec<MatchLocation>,
    depth_tracker: u32,
    complexity_context: ComplexityContext,
    in_closure: bool,
    in_async: bool,
}

impl RecursiveMatchDetector {
    pub fn new() -> Self {
        Self {
            matches_found: Vec::new(),
            depth_tracker: 0,
            complexity_context: ComplexityContext {
                in_closure: false,
                in_async: false,
                nesting_depth: 0,
                function_role: FunctionRole::Unknown,
            },
            in_closure: false,
            in_async: false,
        }
    }

    /// Find all match expressions in a function item
    pub fn find_all_matches(&mut self, item: &Item) -> Vec<MatchLocation> {
        self.traverse_item_recursively(item);
        self.matches_found.clone()
    }

    /// Find all match expressions in a block
    pub fn find_matches_in_block(&mut self, block: &Block) -> Vec<MatchLocation> {
        self.visit_block(block);
        self.matches_found.clone()
    }

    fn traverse_item_recursively(&mut self, item: &Item) {
        match item {
            Item::Fn(func) => {
                self.determine_function_role(&func.sig.ident.to_string());
                self.visit_block(&func.block);
            }
            Item::Impl(impl_block) => {
                for item in &impl_block.items {
                    if let ImplItem::Fn(method) = item {
                        self.determine_function_role(&method.sig.ident.to_string());
                        self.visit_block(&method.block);
                    }
                }
            }
            _ => {}
        }
    }

    fn determine_function_role(&mut self, name: &str) {
        self.complexity_context.function_role = if name == "main" {
            FunctionRole::EntryPoint
        } else if name.starts_with("test_") || name.ends_with("_test") {
            FunctionRole::Test
        } else if name.starts_with("get_") || name.starts_with("set_") || name.starts_with("is_") {
            FunctionRole::Utility
        } else {
            FunctionRole::CoreLogic
        };
    }

    fn calculate_match_complexity(&self, match_expr: &ExprMatch) -> u32 {
        let recognizer = MatchExpressionRecognizer::new();
        let mut complexity = match_expr.arms.len() as u32;

        // Check if arms are simple (reduces complexity)
        let simple_arms = match_expr
            .arms
            .iter()
            .all(|arm| recognizer.is_simple_arm(&arm.body));

        if simple_arms {
            // Apply logarithmic scaling for simple match patterns
            complexity = (complexity as f32).log2().ceil() as u32;
        }

        // Add depth penalty
        complexity += self.depth_tracker.min(3);

        complexity
    }

    fn get_line_number(&self, _expr: &ExprMatch) -> usize {
        // In a real implementation, we'd use span information
        // For now, return a placeholder
        1
    }
}

impl<'ast> Visit<'ast> for RecursiveMatchDetector {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            Expr::Match(match_expr) => {
                // Record this match expression
                self.matches_found.push(MatchLocation {
                    line: self.get_line_number(match_expr),
                    arms: match_expr.arms.len(),
                    complexity: self.calculate_match_complexity(match_expr),
                    context: self.complexity_context.clone(),
                });

                // Continue traversing match arms
                self.depth_tracker += 1;
                for arm in &match_expr.arms {
                    self.visit_expr(&arm.body);
                }
                self.depth_tracker -= 1;
            }
            Expr::Closure(closure) => {
                let was_in_closure = self.in_closure;
                self.in_closure = true;
                self.complexity_context.in_closure = true;
                self.depth_tracker += 1;

                self.visit_expr(&closure.body);

                self.depth_tracker -= 1;
                self.complexity_context.in_closure = was_in_closure;
                self.in_closure = was_in_closure;
            }
            Expr::Async(async_block) => {
                let was_in_async = self.in_async;
                self.in_async = true;
                self.complexity_context.in_async = true;
                self.depth_tracker += 1;

                self.visit_block(&async_block.block);

                self.depth_tracker -= 1;
                self.complexity_context.in_async = was_in_async;
                self.in_async = was_in_async;
            }
            Expr::Block(expr_block) => {
                self.visit_block(&expr_block.block);
            }
            Expr::If(if_expr) => {
                self.depth_tracker += 1;
                self.visit_expr(&if_expr.cond);
                self.visit_block(&if_expr.then_branch);
                if let Some((_else_token, else_branch)) = &if_expr.else_branch {
                    self.visit_expr(else_branch);
                }
                self.depth_tracker -= 1;
            }
            Expr::While(while_expr) => {
                self.depth_tracker += 1;
                self.visit_expr(&while_expr.cond);
                self.visit_block(&while_expr.body);
                self.depth_tracker -= 1;
            }
            Expr::ForLoop(for_loop) => {
                self.depth_tracker += 1;
                self.visit_expr(&for_loop.expr);
                self.visit_block(&for_loop.body);
                self.depth_tracker -= 1;
            }
            Expr::Loop(loop_expr) => {
                self.depth_tracker += 1;
                self.visit_block(&loop_expr.body);
                self.depth_tracker -= 1;
            }
            _ => {
                // For all other expressions, use the default visitor
                syn::visit::visit_expr(self, expr);
            }
        }
    }

    fn visit_block(&mut self, block: &'ast Block) {
        for stmt in &block.stmts {
            self.visit_stmt(stmt);
        }
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        match stmt {
            Stmt::Expr(expr, _) => {
                self.visit_expr(expr);
            }
            Stmt::Macro(_) => {
                // Macros can't be analyzed without expansion
            }
            Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    self.visit_expr(&init.expr);
                }
            }
            Stmt::Item(item) => {
                // Handle nested items (like inner functions)
                if let Item::Fn(func) = item {
                    self.depth_tracker += 1;
                    self.visit_block(&func.block);
                    self.depth_tracker -= 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_recursive_match_detection() {
        let item: Item = parse_quote! {
            fn process_value(val: Value) -> Result<String> {
                let result = match val {
                    Value::String(s) => s,
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Nested(inner) => {
                        // Nested match expression
                        match inner.kind {
                            Kind::A => "type_a",
                            Kind::B => "type_b",
                            Kind::C => "type_c",
                        }
                    }
                    _ => "unknown",
                };

                Ok(result)
            }
        };

        let mut detector = RecursiveMatchDetector::new();
        let matches = detector.find_all_matches(&item);

        // Should find 2 match expressions (outer and nested)
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].arms, 5); // Outer match
        assert_eq!(matches[1].arms, 3); // Inner match
    }

    #[test]
    fn test_match_in_closure() {
        let block: Block = parse_quote! {{
            let processor = |item| {
                match item {
                    Item::A => 1,
                    Item::B => 2,
                    Item::C => 3,
                }
            };
        }};

        let mut detector = RecursiveMatchDetector::new();
        let matches = detector.find_matches_in_block(&block);

        assert_eq!(matches.len(), 1);
        assert!(matches[0].context.in_closure);
    }

    #[test]
    fn test_match_in_async_block() {
        let block: Block = parse_quote! {{
            let result = async {
                match fetch_data().await {
                    Ok(data) => process(data),
                    Err(e) => handle_error(e),
                }
            };
        }};

        let mut detector = RecursiveMatchDetector::new();
        let matches = detector.find_matches_in_block(&block);

        assert_eq!(matches.len(), 1);
        assert!(matches[0].context.in_async);
    }

    #[test]
    fn test_deeply_nested_matches() {
        let block: Block = parse_quote! {{
            if condition {
                for item in items {
                    while processing {
                        match item.state {
                            State::Init => {
                                match item.sub_state {
                                    SubState::Ready => "ready",
                                    SubState::Waiting => "waiting",
                                }
                            }
                            State::Done => "done",
                        }
                    }
                }
            }
        }};

        let mut detector = RecursiveMatchDetector::new();
        let matches = detector.find_matches_in_block(&block);

        assert_eq!(matches.len(), 2);
        // The deeper match should have higher nesting depth
        assert!(matches[1].complexity > matches[0].arms as u32);
    }
}
