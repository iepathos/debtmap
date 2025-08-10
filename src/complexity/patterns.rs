use syn::{visit::Visit, Expr, Item};

/// Modern pattern complexity detection
#[derive(Debug, Clone, Default)]
pub struct PatternComplexity {
    pub async_await_count: u32,
    pub callback_depth: u32,
    pub promise_chains: u32,
    pub higher_order_functions: u32,
    pub functional_composition: u32,
    pub nested_ternaries: u32,
    pub method_chain_length: u32,
    pub recursive_calls: u32,
    pub error_handling_blocks: u32,
    pub generic_complexity: u32,
    pub unsafe_blocks: u32,
}

impl PatternComplexity {
    pub fn total_complexity(&self) -> u32 {
        self.async_await_count * 2
            + self.callback_depth * 3
            + self.promise_chains * 2
            + self.higher_order_functions
            + self.functional_composition
            + self.nested_ternaries * 2
            + (self.method_chain_length / 3)
            + self.recursive_calls * 3
            + self.error_handling_blocks
            + self.generic_complexity
            + self.unsafe_blocks * 2
    }

    pub fn merge(&mut self, other: &PatternComplexity) {
        self.async_await_count += other.async_await_count;
        self.callback_depth = self.callback_depth.max(other.callback_depth);
        self.promise_chains += other.promise_chains;
        self.higher_order_functions += other.higher_order_functions;
        self.functional_composition += other.functional_composition;
        self.nested_ternaries += other.nested_ternaries;
        self.method_chain_length = self.method_chain_length.max(other.method_chain_length);
        self.recursive_calls += other.recursive_calls;
        self.error_handling_blocks += other.error_handling_blocks;
        self.generic_complexity += other.generic_complexity;
        self.unsafe_blocks += other.unsafe_blocks;
    }
}

pub struct PatternDetector {
    pub patterns: PatternComplexity,
    current_function_name: Option<String>,
    closure_depth: u32,
    ternary_depth: u32,
}

impl Default for PatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternDetector {
    pub fn new() -> Self {
        Self {
            patterns: PatternComplexity::default(),
            current_function_name: None,
            closure_depth: 0,
            ternary_depth: 0,
        }
    }

    pub fn analyze_item(&mut self, item: &Item) {
        self.visit_item(item);
    }

    pub fn analyze_expr(&mut self, expr: &Expr) {
        self.visit_expr(expr);
    }

    fn detect_async_pattern(&mut self, sig: &syn::Signature) {
        if sig.asyncness.is_some() {
            self.patterns.async_await_count += 1;
        }

        // Count generic type parameters as complexity
        let generic_count = sig.generics.params.len() as u32;
        if generic_count > 0 {
            self.patterns.generic_complexity += generic_count;
        }
    }

    fn detect_method_chain(&mut self, expr: &Expr) -> u32 {
        let mut chain_length = 0;
        let mut current = expr;

        loop {
            match current {
                Expr::MethodCall(method) => {
                    chain_length += 1;
                    current = &method.receiver;
                }
                Expr::Field(field) => {
                    chain_length += 1;
                    current = &field.base;
                }
                _ => break,
            }
        }

        chain_length
    }

    fn is_higher_order_function(&self, expr: &Expr) -> bool {
        match expr {
            Expr::MethodCall(method) => {
                let method_name = method.method.to_string();
                matches!(
                    method_name.as_str(),
                    "map"
                        | "filter"
                        | "fold"
                        | "reduce"
                        | "flat_map"
                        | "filter_map"
                        | "and_then"
                        | "or_else"
                        | "map_or"
                        | "map_or_else"
                )
            }
            _ => false,
        }
    }

    fn detect_functional_composition(&mut self, expr: &Expr) {
        if let Expr::MethodCall(method) = expr {
            if self.is_higher_order_function(expr) {
                self.patterns.higher_order_functions += 1;

                // Check if it's part of a chain
                if let Expr::MethodCall(_) = &*method.receiver {
                    self.patterns.functional_composition += 1;
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for PatternDetector {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let old_name = self.current_function_name.clone();
        self.current_function_name = Some(node.sig.ident.to_string());

        self.detect_async_pattern(&node.sig);
        syn::visit::visit_item_fn(self, node);

        self.current_function_name = old_name;
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let old_name = self.current_function_name.clone();
        self.current_function_name = Some(node.sig.ident.to_string());

        self.detect_async_pattern(&node.sig);
        syn::visit::visit_impl_item_fn(self, node);

        self.current_function_name = old_name;
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            // Async/await detection
            Expr::Await(_) => {
                self.patterns.async_await_count += 1;
            }

            // Closure detection - only count async closures for patterns
            // Regular closures are already handled in cognitive complexity
            Expr::Closure(closure) => {
                self.closure_depth += 1;

                // Only count async closures in patterns (they add extra complexity)
                // Regular closures are already counted in cognitive complexity
                if closure.asyncness.is_some() {
                    self.patterns.async_await_count += 1;
                }

                syn::visit::visit_expr_closure(self, closure);
                self.closure_depth -= 1;
            }

            // Method chain detection
            Expr::MethodCall(_) => {
                let chain_length = self.detect_method_chain(expr);
                if chain_length > 2 {
                    self.patterns.method_chain_length =
                        self.patterns.method_chain_length.max(chain_length);
                }

                // Functional composition detection
                self.detect_functional_composition(expr);

                // Check for recursive calls
                if let Some(ref func_name) = self.current_function_name {
                    if let Expr::MethodCall(method) = expr {
                        if method.method.to_string() == *func_name {
                            self.patterns.recursive_calls += 1;
                        }
                    }
                }

                syn::visit::visit_expr(self, expr);
            }

            // Ternary/conditional expression detection
            // Only count as ternary if it's an if-expression with else branch
            // and single expressions (not statements) in both branches
            Expr::If(if_expr)
                if if_expr.else_branch.is_some()
                    && if_expr.then_branch.stmts.len() == 1
                    && matches!(&if_expr.then_branch.stmts[0], syn::Stmt::Expr(_, None)) =>
            {
                self.ternary_depth += 1;
                if self.ternary_depth > 1 {
                    self.patterns.nested_ternaries += 1;
                }
                syn::visit::visit_expr_if(self, if_expr);
                self.ternary_depth -= 1;
            }

            // Try expressions are already counted in cognitive complexity,
            // so we don't add them to patterns to avoid double-counting
            Expr::Try(_) => {
                syn::visit::visit_expr(self, expr);
            }

            // Unsafe block detection
            Expr::Unsafe(_) => {
                self.patterns.unsafe_blocks += 1;
                syn::visit::visit_expr(self, expr);
            }

            // Recursive call detection for direct function calls
            Expr::Call(call) => {
                if let Some(ref func_name) = self.current_function_name {
                    if let Expr::Path(path) = &*call.func {
                        if let Some(segment) = path.path.segments.last() {
                            if segment.ident.to_string() == *func_name {
                                self.patterns.recursive_calls += 1;
                            }
                        }
                    }
                }
                syn::visit::visit_expr(self, expr);
            }

            _ => syn::visit::visit_expr(self, expr),
        }
    }
}

/// Calculate pattern-based complexity for a function
pub fn analyze_patterns(block: &syn::Block) -> PatternComplexity {
    let mut detector = PatternDetector::new();
    detector.visit_block(block);
    detector.patterns
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_complexity_total() {
        let mut patterns = PatternComplexity::default();
        patterns.async_await_count = 2;
        patterns.callback_depth = 1;
        patterns.method_chain_length = 6;

        assert_eq!(patterns.total_complexity(), 2 * 2 + 3 + 6 / 3);
    }

    #[test]
    fn test_pattern_merge() {
        let mut p1 = PatternComplexity::default();
        p1.async_await_count = 1;
        p1.callback_depth = 2;

        let mut p2 = PatternComplexity::default();
        p2.async_await_count = 2;
        p2.callback_depth = 3;

        p1.merge(&p2);
        assert_eq!(p1.async_await_count, 3);
        assert_eq!(p1.callback_depth, 3);
    }
}
