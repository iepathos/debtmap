//! State Machine Pattern Detector
//!
//! Detects state machine and coordinator patterns (spec 179) by analyzing:
//! - Match expressions on enum variants (state transitions)
//! - Conditional logic based on state comparisons
//! - Action accumulation and dispatching patterns

use crate::priority::complexity_patterns::{CoordinatorSignals, StateMachineSignals};
use syn::{visit::Visit, Block, Expr, ExprMatch, Pat, PatTupleStruct, Stmt};

/// Detector for state machine and coordinator patterns
pub struct StateMachinePatternDetector;

impl StateMachinePatternDetector {
    pub fn new() -> Self {
        Self
    }

    /// Detect state machine pattern from AST block
    pub fn detect_state_machine(&self, block: &Block) -> Option<StateMachineSignals> {
        let mut visitor = StateMachineVisitor::new();
        visitor.visit_block(block);

        // Require evidence of state machine pattern
        if !visitor.has_enum_match && visitor.state_comparison_count == 0 {
            return None;
        }

        // Calculate confidence based on signals
        let confidence = calculate_state_machine_confidence(
            visitor.enum_match_count,
            visitor.tuple_match_count,
            visitor.state_comparison_count,
            visitor.action_dispatch_count,
        );

        // Require minimum confidence to avoid false positives
        if confidence < 0.6 {
            return None;
        }

        Some(StateMachineSignals {
            transition_count: visitor.enum_match_count + visitor.tuple_match_count,
            has_enum_match: visitor.has_enum_match,
            has_state_comparison: visitor.state_comparison_count > 0,
            action_dispatch_count: visitor.action_dispatch_count,
            confidence,
        })
    }

    /// Detect coordinator pattern from AST block
    pub fn detect_coordinator(&self, block: &Block) -> Option<CoordinatorSignals> {
        let mut visitor = CoordinatorVisitor::new();
        visitor.visit_block(block);

        // Require evidence of coordinator pattern (action accumulation + comparisons)
        if visitor.vec_push_count < 2 || visitor.comparison_count < 2 {
            return None;
        }

        // Calculate confidence
        let confidence = calculate_coordinator_confidence(
            visitor.vec_push_count,
            visitor.comparison_count,
            visitor.has_helper_calls,
        );

        if confidence < 0.6 {
            return None;
        }

        Some(CoordinatorSignals {
            actions: visitor.vec_push_count,
            comparisons: visitor.comparison_count,
            has_action_accumulation: visitor.vec_push_count >= 2,
            has_helper_calls: visitor.has_helper_calls,
            confidence,
        })
    }
}

impl Default for StateMachinePatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate confidence for state machine pattern
fn calculate_state_machine_confidence(
    enum_match_count: u32,
    tuple_match_count: u32,
    state_comparison_count: u32,
    action_dispatch_count: u32,
) -> f64 {
    let mut confidence = 0.0;

    // Match on enum/tuple variants is strongest signal (up to 0.5)
    if enum_match_count > 0 || tuple_match_count > 0 {
        confidence += 0.5;
    }

    // Multiple state comparisons (up to 0.3)
    confidence += (state_comparison_count as f64 / 10.0).min(0.3);

    // Action dispatch within branches (up to 0.2)
    confidence += (action_dispatch_count as f64 / 10.0).min(0.2);

    confidence.min(1.0)
}

/// Calculate confidence for coordinator pattern
fn calculate_coordinator_confidence(
    vec_push_count: u32,
    comparison_count: u32,
    has_helper_calls: bool,
) -> f64 {
    let mut confidence = 0.0;

    // Multiple action accumulations (up to 0.4)
    confidence += (vec_push_count as f64 / 10.0).min(0.4);

    // Multiple comparisons (up to 0.3)
    confidence += (comparison_count as f64 / 10.0).min(0.3);

    // Helper function calls (0.2)
    if has_helper_calls {
        confidence += 0.2;
    }

    // Moderate accumulation (3-6 actions) gets bonus (0.1)
    if (3..=6).contains(&vec_push_count) {
        confidence += 0.1;
    }

    confidence.min(1.0)
}

/// Visitor to detect state machine patterns
struct StateMachineVisitor {
    enum_match_count: u32,
    tuple_match_count: u32,
    state_comparison_count: u32,
    action_dispatch_count: u32,
    has_enum_match: bool,
}

impl StateMachineVisitor {
    fn new() -> Self {
        Self {
            enum_match_count: 0,
            tuple_match_count: 0,
            state_comparison_count: 0,
            action_dispatch_count: 0,
            has_enum_match: false,
        }
    }

    /// Check if a pattern is an enum variant or tuple pattern
    fn is_enum_or_tuple_pattern(&self, pat: &Pat) -> bool {
        matches!(
            pat,
            Pat::TupleStruct(_) | Pat::Struct(_) | Pat::Ident(_) | Pat::Tuple(_)
        )
    }

    /// Check if expression contains state-related field access
    fn has_state_field_access(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Field(field_expr) => {
                // Check if field name contains "state", "mode", "status"
                let field_name = match &field_expr.member {
                    syn::Member::Named(ident) => ident.to_string(),
                    syn::Member::Unnamed(_) => String::new(),
                };
                let field_name = field_name.to_lowercase();
                field_name.contains("state")
                    || field_name.contains("mode")
                    || field_name.contains("status")
            }
            Expr::Path(path) => {
                // Check if path contains state-related identifiers
                path.path.segments.iter().any(|seg| {
                    let name = seg.ident.to_string().to_lowercase();
                    name.contains("state") || name.contains("mode") || name.contains("status")
                })
            }
            _ => false,
        }
    }
}

impl<'ast> Visit<'ast> for StateMachineVisitor {
    fn visit_expr_match(&mut self, match_expr: &'ast ExprMatch) {
        // Check if matching on state-related expression
        if self.has_state_field_access(&match_expr.expr) {
            self.has_enum_match = true;
            self.state_comparison_count += 1;
        }

        // Count enum/tuple patterns in arms
        for arm in &match_expr.arms {
            match &arm.pat {
                Pat::TupleStruct(tuple) => {
                    self.tuple_match_count += 1;
                    self.has_enum_match = true;

                    // Check for nested tuple patterns (e.g., (State::A, State::B))
                    if is_nested_tuple_pattern(tuple) {
                        self.enum_match_count += 1;
                    }
                }
                Pat::Tuple(_) => {
                    // Plain tuple patterns like (Mode::Active, Mode::Standby)
                    self.tuple_match_count += 1;
                    self.has_enum_match = true;
                }
                Pat::Struct(_) => {
                    self.enum_match_count += 1;
                    self.has_enum_match = true;
                }
                _ => {
                    if self.is_enum_or_tuple_pattern(&arm.pat) {
                        self.enum_match_count += 1;
                    }
                }
            }

            // Check for action dispatch in arm body
            if has_vec_push_or_call(&arm.body) {
                self.action_dispatch_count += 1;
            }
        }

        syn::visit::visit_expr_match(self, match_expr);
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        // Count state comparisons in if expressions
        if let Expr::If(if_expr) = expr {
            if self.has_state_field_access(&if_expr.cond) {
                self.state_comparison_count += 1;
            }
        }

        syn::visit::visit_expr(self, expr);
    }
}

/// Check if tuple pattern contains nested patterns (state transitions)
fn is_nested_tuple_pattern(tuple: &PatTupleStruct) -> bool {
    // Check if tuple has at least 2 elements (likely state transition pair)
    tuple.elems.len() >= 2
}

/// Check if expression contains vec push or function calls (action dispatch)
fn has_vec_push_or_call(expr: &Expr) -> bool {
    match expr {
        Expr::MethodCall(method) => method.method == "push" || method.method == "extend",
        Expr::Call(_) => true,
        Expr::Block(block) => block.block.stmts.iter().any(|stmt| match stmt {
            Stmt::Expr(expr, _) => has_vec_push_or_call(expr),
            Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    has_vec_push_or_call(&init.expr)
                } else {
                    false
                }
            }
            _ => false,
        }),
        _ => false,
    }
}

/// Visitor to detect coordinator patterns
struct CoordinatorVisitor {
    vec_push_count: u32,
    comparison_count: u32,
    has_helper_calls: bool,
}

impl CoordinatorVisitor {
    fn new() -> Self {
        Self {
            vec_push_count: 0,
            comparison_count: 0,
            has_helper_calls: false,
        }
    }
}

impl<'ast> Visit<'ast> for CoordinatorVisitor {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            // Count vec.push() calls (action accumulation)
            Expr::MethodCall(method) => {
                if method.method == "push" {
                    self.vec_push_count += 1;
                }
            }
            // Count comparisons (state checks)
            Expr::Binary(binary) => {
                use syn::BinOp;
                if matches!(
                    binary.op,
                    BinOp::Eq(_)
                        | BinOp::Ne(_)
                        | BinOp::Lt(_)
                        | BinOp::Le(_)
                        | BinOp::Gt(_)
                        | BinOp::Ge(_)
                ) {
                    self.comparison_count += 1;
                }
            }
            // Detect helper function calls
            Expr::Call(_) => {
                self.has_helper_calls = true;
            }
            _ => {}
        }

        syn::visit::visit_expr(self, expr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn detect_state_machine_with_match() {
        let block: Block = parse_quote! {
            {
                let mut actions = vec![];
                match (current.mode, desired.mode) {
                    (Mode::Active, Mode::Standby) => {
                        if current.has_active_connections() {
                            actions.push(Action::DrainConnections);
                        }
                        actions.push(Action::TransitionToStandby);
                    }
                    (Mode::Standby, Mode::Active) => {
                        actions.push(Action::TransitionToActive);
                    }
                    _ => {}
                }
                actions
            }
        };

        let detector = StateMachinePatternDetector::new();
        let signals = detector.detect_state_machine(&block);

        assert!(signals.is_some());
        let signals = signals.unwrap();
        assert!(signals.has_enum_match);
        assert_eq!(signals.transition_count, 2); // Two tuple patterns matched
        assert!(signals.confidence >= 0.6);
    }

    #[test]
    fn detect_coordinator_with_action_accumulation() {
        let block: Block = parse_quote! {
            {
                let mut actions = vec![];
                if current.status != desired.status {
                    actions.push(Action::UpdateStatus);
                }
                if current.value > desired.value {
                    actions.push(Action::ReduceValue);
                }
                if helper_check(current) {
                    actions.push(Action::Apply);
                }
                actions
            }
        };

        let detector = StateMachinePatternDetector::new();
        let signals = detector.detect_coordinator(&block);

        assert!(signals.is_some());
        let signals = signals.unwrap();
        assert_eq!(signals.actions, 3);
        assert!(signals.comparisons >= 2);
        assert!(signals.has_action_accumulation);
        assert!(signals.has_helper_calls);
        assert!(signals.confidence >= 0.6);
    }

    #[test]
    fn no_detection_for_simple_function() {
        let block: Block = parse_quote! {
            {
                let x = compute_value();
                x + 10
            }
        };

        let detector = StateMachinePatternDetector::new();
        let state_signals = detector.detect_state_machine(&block);
        let coord_signals = detector.detect_coordinator(&block);

        assert!(state_signals.is_none());
        assert!(coord_signals.is_none());
    }

    #[test]
    fn state_machine_requires_enum_match() {
        let block: Block = parse_quote! {
            {
                if x > 10 {
                    do_something();
                }
            }
        };

        let detector = StateMachinePatternDetector::new();
        let signals = detector.detect_state_machine(&block);

        assert!(signals.is_none());
    }
}
