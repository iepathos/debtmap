use super::match_patterns::MatchExpressionRecognizer;
use std::collections::HashMap;
use syn::{
    Block, Expr, ExprAsync, ExprBlock, ExprClosure, ExprForLoop, ExprIf, ExprLoop, ExprMatch,
    ExprWhile, ImplItem, Item, Stmt, visit::Visit,
};

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

/// Maximum recursion depth to prevent stack overflow
const MAX_RECURSION_DEPTH: u32 = 150;

/// Cache key for match detection results
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CacheKey {
    function_name: String,
    file_path: String,
}

/// Recursively detects all match expressions throughout the entire function AST
pub struct RecursiveMatchDetector {
    pub matches_found: Vec<MatchLocation>,
    depth_tracker: u32,
    complexity_context: ComplexityContext,
    in_closure: bool,
    in_async: bool,
    /// Cache for previously analyzed functions
    cache: HashMap<CacheKey, Vec<MatchLocation>>,
    /// Maximum depth reached during traversal
    max_depth_reached: u32,
}

impl Default for RecursiveMatchDetector {
    fn default() -> Self {
        Self::new()
    }
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
            cache: HashMap::new(),
            max_depth_reached: 0,
        }
    }

    /// Create a new detector with pre-populated cache
    pub fn with_cache(cache: HashMap<CacheKey, Vec<MatchLocation>>) -> Self {
        let mut detector = Self::new();
        detector.cache = cache;
        detector
    }

    /// Get the current cache for reuse
    pub fn get_cache(&self) -> &HashMap<CacheKey, Vec<MatchLocation>> {
        &self.cache
    }

    /// Check if recursion depth is within safe limits
    fn check_depth_limit(&mut self) -> bool {
        if self.depth_tracker > MAX_RECURSION_DEPTH {
            eprintln!(
                "Warning: Maximum recursion depth {} reached, stopping traversal",
                MAX_RECURSION_DEPTH
            );
            return false;
        }
        if self.depth_tracker > self.max_depth_reached {
            self.max_depth_reached = self.depth_tracker;
        }
        true
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

    fn visit_with_nested_depth(&mut self, visit_nested: impl FnOnce(&mut Self)) {
        self.depth_tracker += 1;
        if self.check_depth_limit() {
            visit_nested(self);
        }
        self.depth_tracker -= 1;
    }

    fn record_match(&mut self, match_expr: &ExprMatch) {
        self.matches_found.push(MatchLocation {
            line: self.get_line_number(match_expr),
            arms: match_expr.arms.len(),
            complexity: self.calculate_match_complexity(match_expr),
            context: self.complexity_context.clone(),
        });
    }

    fn visit_match_expr(&mut self, match_expr: &ExprMatch) {
        self.record_match(match_expr);
        self.visit_with_nested_depth(|detector| {
            for arm in &match_expr.arms {
                detector.visit_expr(&arm.body);
            }
        });
    }

    fn visit_closure_expr(&mut self, closure: &ExprClosure) {
        let was_in_closure = self.in_closure;
        self.in_closure = true;
        self.complexity_context.in_closure = true;

        self.visit_with_nested_depth(|detector| {
            detector.visit_expr(&closure.body);
        });

        self.complexity_context.in_closure = was_in_closure;
        self.in_closure = was_in_closure;
    }

    fn visit_async_expr(&mut self, async_block: &ExprAsync) {
        let was_in_async = self.in_async;
        self.in_async = true;
        self.complexity_context.in_async = true;

        self.visit_with_nested_depth(|detector| {
            detector.visit_block(&async_block.block);
        });

        self.complexity_context.in_async = was_in_async;
        self.in_async = was_in_async;
    }

    fn visit_block_expr(&mut self, expr_block: &ExprBlock) {
        self.visit_with_nested_depth(|detector| {
            detector.visit_block(&expr_block.block);
        });
    }

    fn visit_if_expr(&mut self, if_expr: &ExprIf) {
        self.visit_with_nested_depth(|detector| {
            detector.visit_expr(&if_expr.cond);
            detector.visit_block(&if_expr.then_branch);
            if let Some((_else_token, else_branch)) = &if_expr.else_branch {
                detector.visit_expr(else_branch);
            }
        });
    }

    fn visit_while_expr(&mut self, while_expr: &ExprWhile) {
        self.visit_with_nested_depth(|detector| {
            detector.visit_expr(&while_expr.cond);
            detector.visit_block(&while_expr.body);
        });
    }

    fn visit_for_loop_expr(&mut self, for_loop: &ExprForLoop) {
        self.visit_with_nested_depth(|detector| {
            detector.visit_expr(&for_loop.expr);
            detector.visit_block(&for_loop.body);
        });
    }

    fn visit_loop_expr(&mut self, loop_expr: &ExprLoop) {
        self.visit_with_nested_depth(|detector| {
            detector.visit_block(&loop_expr.body);
        });
    }

    fn visit_other_expr(&mut self, expr: &Expr) {
        self.visit_with_nested_depth(|detector| {
            detector.visit_other_expr_children(expr);
        });
    }

    fn visit_other_expr_children(&mut self, expr: &Expr) {
        match expr {
            Expr::Binary(e) => {
                self.visit_expr(&e.left);
                self.visit_expr(&e.right);
            }
            Expr::Unary(e) => {
                self.visit_expr(&e.expr);
            }
            Expr::Call(e) => {
                self.visit_expr(&e.func);
                for arg in &e.args {
                    self.visit_expr(arg);
                }
            }
            Expr::MethodCall(e) => {
                self.visit_expr(&e.receiver);
                for arg in &e.args {
                    self.visit_expr(arg);
                }
            }
            Expr::Field(e) => {
                self.visit_expr(&e.base);
            }
            Expr::Index(e) => {
                self.visit_expr(&e.expr);
                self.visit_expr(&e.index);
            }
            Expr::Paren(e) => {
                self.visit_expr(&e.expr);
            }
            Expr::Try(e) => {
                self.visit_expr(&e.expr);
            }
            Expr::Await(e) => {
                self.visit_expr(&e.base);
            }
            Expr::Lit(_) | Expr::Path(_) | Expr::Continue(_) | Expr::Break(_) => {}
            _ => {
                if self.depth_tracker < MAX_RECURSION_DEPTH / 2 {
                    syn::visit::visit_expr(self, expr);
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for RecursiveMatchDetector {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        // Check depth limit before processing
        if !self.check_depth_limit() {
            return;
        }

        match expr {
            Expr::Match(match_expr) => self.visit_match_expr(match_expr),
            Expr::Closure(closure) => self.visit_closure_expr(closure),
            Expr::Async(async_block) => self.visit_async_expr(async_block),
            Expr::Block(expr_block) => self.visit_block_expr(expr_block),
            Expr::If(if_expr) => self.visit_if_expr(if_expr),
            Expr::While(while_expr) => self.visit_while_expr(while_expr),
            Expr::ForLoop(for_loop) => self.visit_for_loop_expr(for_loop),
            Expr::Loop(loop_expr) => self.visit_loop_expr(loop_expr),
            _ => self.visit_other_expr(expr),
        }
    }

    fn visit_block(&mut self, block: &'ast Block) {
        // Check depth before processing block
        if !self.check_depth_limit() {
            return;
        }

        for stmt in &block.stmts {
            if !self.check_depth_limit() {
                break;
            }
            self.visit_stmt(stmt);
        }
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        // Check depth before processing statement
        if !self.check_depth_limit() {
            return;
        }

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
                    if self.check_depth_limit() {
                        self.visit_block(&func.block);
                    }
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
    fn test_context_is_restored_after_closure_and_async() {
        let block: Block = parse_quote! {{
            let from_closure = |item| {
                match item {
                    Item::A => 1,
                    _ => 2,
                }
            };

            let from_async = async {
                match fetch_data().await {
                    Ok(data) => data,
                    Err(_) => 0,
                }
            };

            match current {
                Current::Ready => 1,
                _ => 0,
            }
        }};

        let mut detector = RecursiveMatchDetector::new();
        let matches = detector.find_matches_in_block(&block);

        assert_eq!(matches.len(), 3);
        assert!(matches[0].context.in_closure);
        assert!(!matches[0].context.in_async);
        assert!(!matches[1].context.in_closure);
        assert!(matches[1].context.in_async);
        assert!(!matches[2].context.in_closure);
        assert!(!matches[2].context.in_async);
    }

    #[test]
    fn test_default_fallback_finds_match_in_tuple_expression() {
        let block: Block = parse_quote! {{
            let pair = (
                match left {
                    Side::A => 1,
                    _ => 0,
                },
                match right {
                    Side::B => 2,
                    _ => 0,
                },
            );
        }};

        let mut detector = RecursiveMatchDetector::new();
        let matches = detector.find_matches_in_block(&block);

        assert_eq!(matches.len(), 2);
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
