//! State Machine Pattern Detector
//!
//! Detects state machine and coordinator patterns (spec 179, 192) by analyzing:
//! - Match expressions on enum variants (state transitions)
//! - Conditional logic based on state comparisons
//! - Action accumulation and dispatching patterns
//!
//! Spec 192 adds state-aware coordinator detection to reduce false positives
//! on validation code by distinguishing:
//! - State comparisons vs value checks
//! - Action accumulation vs error accumulation

use crate::priority::complexity_patterns::{CoordinatorSignals, StateMachineSignals};
use syn::{
    visit::Visit, Block, Expr, ExprBinary, ExprMatch, ExprMethodCall, Pat, PatTupleStruct, Stmt,
};

/// Detector for state machine and coordinator patterns
pub struct StateMachinePatternDetector;

/// Keywords that indicate state-related fields
const STATE_FIELD_KEYWORDS: &[&str] = &[
    "state", "mode", "status", "phase", "stage", "desired", "current", "target", "actual",
];

/// Keywords for state-related paths/variables
const STATE_PATH_KEYWORDS: &[&str] = &["state", "mode", "status", "phase"];

/// Keywords indicating error accumulation (not action accumulation)
const ERROR_ACCUMULATION_KEYWORDS: &[&str] = &[
    "error",
    "err",
    "issue",
    "warning",
    "warn",
    "validation",
    "invalid",
    "problem",
];

/// Patterns indicating explicit action types
const ACTION_TYPE_PATTERNS: &[&str] = &[
    "Action::",
    "Command::",
    "Operation::",
    "Task::",
    "Event::",
    "Message::",
];

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
            match_expression_count: visitor.match_expression_count,
            has_enum_match: visitor.has_enum_match,
            has_state_comparison: visitor.state_comparison_count > 0,
            action_dispatch_count: visitor.action_dispatch_count,
            confidence,
        })
    }

    /// Detect coordinator pattern from AST block with state-awareness (spec 192).
    ///
    /// A true coordinator pattern has:
    /// - State-related comparisons (e.g., `current.state != desired.state`)
    /// - Action accumulation in state-conditional blocks
    /// - Explicit action types (e.g., `Action::DoSomething`)
    ///
    /// This distinguishes coordinators from:
    /// - Validation code (error accumulation)
    /// - Simple conditional logic (non-state comparisons)
    ///
    /// # False Positive Avoidance
    ///
    /// The detector rejects:
    /// - Error accumulation patterns (`errors.push(...)`)
    /// - Validation checks (`if value.is_empty()`)
    /// - Non-state comparisons
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // TRUE COORDINATOR (detected)
    /// let mut actions = vec![];
    /// if current.state != desired.state {
    ///     actions.push(Action::Transition);
    /// }
    ///
    /// // VALIDATION CODE (rejected)
    /// let mut errors = vec![];
    /// if email.is_empty() {
    ///     errors.push("required");
    /// }
    /// ```
    pub fn detect_coordinator(&self, block: &Block) -> Option<CoordinatorSignals> {
        let mut visitor = CoordinatorVisitor::new();
        visitor.visit_block(block);

        // NEW: Require evidence of coordinator pattern with state awareness
        // Use action_push_count if state comparisons exist (coordinator pattern)
        let action_count = if visitor.state_comparison_count >= 2 {
            visitor.action_push_count // All non-error pushes when state comparisons exist
        } else {
            visitor.state_aware_push_count // Must be in state-conditional blocks
        };

        if action_count < 3 || visitor.state_comparison_count < 2 {
            return None;
        }

        // NEW: Penalty for error accumulation patterns
        let total_pushes = action_count + visitor.error_accumulation_count;
        if total_pushes > 0 {
            let error_ratio = visitor.error_accumulation_count as f64 / total_pushes as f64;
            if error_ratio > 0.5 {
                return None; // Likely validation code
            }
        }

        // NEW: Calculate confidence with enhanced signals
        let has_action_types = visitor.explicit_action_type_count > 0;
        let confidence = calculate_enhanced_coordinator_confidence(
            action_count,
            visitor.state_comparison_count,
            visitor.has_helper_calls,
            has_action_types,
            visitor.has_final_dispatch,
        );

        if confidence < 0.7 {
            // Raised from 0.6
            return None;
        }

        Some(CoordinatorSignals {
            actions: action_count,
            comparisons: visitor.state_comparison_count,
            has_action_accumulation: true,
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

/// Calculate enhanced confidence for coordinator pattern (spec 192)
fn calculate_enhanced_coordinator_confidence(
    state_aware_pushes: u32,
    state_comparisons: u32,
    has_helper_calls: bool,
    has_action_types: bool,
    has_final_dispatch: bool,
) -> f64 {
    let mut confidence = 0.0;

    // State-aware action accumulation (up to 0.4)
    confidence += (state_aware_pushes as f64 / 10.0).min(0.4);

    // State-related comparisons (up to 0.3)
    confidence += (state_comparisons as f64 / 10.0).min(0.3);

    // Helper function calls (0.1)
    if has_helper_calls {
        confidence += 0.1;
    }

    // NEW: Explicit action types (0.15 bonus)
    if has_action_types {
        confidence += 0.15;
    }

    // NEW: Final dispatch pattern (0.1 bonus)
    if has_final_dispatch {
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
    match_expression_count: u32,
    has_enum_match: bool,
}

impl StateMachineVisitor {
    fn new() -> Self {
        Self {
            enum_match_count: 0,
            tuple_match_count: 0,
            state_comparison_count: 0,
            action_dispatch_count: 0,
            match_expression_count: 0,
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
        // Count this match expression
        self.match_expression_count += 1;

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

/// Check if a binary expression is a state-related comparison (spec 192)
fn is_state_comparison(binary: &ExprBinary) -> bool {
    contains_state_identifier(&binary.left) || contains_state_identifier(&binary.right)
}

/// Check if an expression contains state-related identifiers (spec 192)
fn contains_state_identifier(expr: &Expr) -> bool {
    match expr {
        Expr::Field(field) => {
            let field_name = match &field.member {
                syn::Member::Named(ident) => ident.to_string(),
                syn::Member::Unnamed(_) => String::new(),
            };
            let field_name = field_name.to_lowercase();
            STATE_FIELD_KEYWORDS
                .iter()
                .any(|kw| field_name.contains(kw))
        }
        Expr::Path(path) => path.path.segments.iter().any(|seg| {
            let name = seg.ident.to_string().to_lowercase();
            STATE_PATH_KEYWORDS.iter().any(|kw| name.contains(kw))
        }),
        _ => false,
    }
}

/// Check if a method call is error accumulation (not action accumulation) (spec 192)
fn is_error_accumulation(method: &ExprMethodCall) -> bool {
    if method.method != "push" {
        return false;
    }

    let receiver_name = get_receiver_name(&method.receiver).to_lowercase();
    ERROR_ACCUMULATION_KEYWORDS
        .iter()
        .any(|kw| receiver_name.contains(kw))
}

/// Check if push arguments are action types (spec 192)
fn is_action_type(args: &syn::punctuated::Punctuated<Expr, syn::token::Comma>) -> bool {
    args.iter().any(|arg| {
        if let Expr::Path(path) = arg {
            let path_str = path_to_string(&path.path);
            ACTION_TYPE_PATTERNS
                .iter()
                .any(|pat| path_str.contains(pat))
        } else {
            false
        }
    })
}

/// Extract receiver name from expression
fn get_receiver_name(receiver: &Expr) -> String {
    match receiver {
        Expr::Path(path) => path_to_string(&path.path),
        Expr::Field(field) => match &field.member {
            syn::Member::Named(ident) => ident.to_string(),
            syn::Member::Unnamed(_) => String::new(),
        },
        _ => String::new(),
    }
}

/// Convert path to string
fn path_to_string(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|seg| seg.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

/// Visitor to detect coordinator patterns (enhanced in spec 192)
struct CoordinatorVisitor {
    // Original fields
    vec_push_count: u32,
    comparison_count: u32,
    has_helper_calls: bool,

    // NEW: State-aware fields (spec 192)
    action_push_count: u32,          // Non-error pushes (actions)
    state_aware_push_count: u32,     // Pushes in state-conditional blocks
    state_comparison_count: u32,     // Comparisons on state-related fields
    error_accumulation_count: u32,   // Pushes to error/issue vectors
    explicit_action_type_count: u32, // Pushes of Action:: variants
    has_final_dispatch: bool,        // Returns action vector

    // NEW: Context tracking
    current_conditional_is_state_related: bool,
}

impl CoordinatorVisitor {
    fn new() -> Self {
        Self {
            vec_push_count: 0,
            comparison_count: 0,
            has_helper_calls: false,
            action_push_count: 0,
            state_aware_push_count: 0,
            state_comparison_count: 0,
            error_accumulation_count: 0,
            explicit_action_type_count: 0,
            has_final_dispatch: false,
            current_conditional_is_state_related: false,
        }
    }
}

impl<'ast> Visit<'ast> for CoordinatorVisitor {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            // Enhanced method call analysis (spec 192)
            Expr::MethodCall(method) if method.method == "push" => {
                self.vec_push_count += 1;

                // NEW: Classify push type
                if is_error_accumulation(method) {
                    self.error_accumulation_count += 1;
                } else {
                    // Count all non-error pushes as action pushes
                    self.action_push_count += 1;

                    // Check if in state-conditional context
                    if self.current_conditional_is_state_related {
                        self.state_aware_push_count += 1;
                    }

                    // Check if explicit action type
                    if is_action_type(&method.args) {
                        self.explicit_action_type_count += 1;
                    }
                }
            }
            // Enhanced comparison analysis (spec 192)
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

                    // NEW: Check if state-related
                    if is_state_comparison(binary) {
                        self.state_comparison_count += 1;
                        self.current_conditional_is_state_related = true;
                    }
                }
            }
            // Detect helper function calls
            Expr::Call(_) => {
                self.has_helper_calls = true;
            }
            _ => {}
        }

        // Visit children
        syn::visit::visit_expr(self, expr);

        // Reset context after visiting conditional
        if matches!(expr, Expr::If(_)) {
            self.current_conditional_is_state_related = false;
        }
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        // Check for final dispatch pattern (return actions)
        if let Stmt::Expr(Expr::Path(path), None) = stmt {
            let path_str = path_to_string(&path.path);
            if path_str.contains("action") || path_str.contains("command") {
                self.has_final_dispatch = true;
            }
        }

        syn::visit::visit_stmt(self, stmt);
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
        assert_eq!(signals.match_expression_count, 1); // One match expression
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
                if current.phase != desired.phase {
                    actions.push(Action::UpdatePhase);
                }
                if current.mode != desired.mode {
                    actions.push(Action::Apply);
                }
                actions
            }
        };

        let detector = StateMachinePatternDetector::new();
        let signals = detector.detect_coordinator(&block);

        assert!(signals.is_some(), "Should detect coordinator pattern");
        let signals = signals.unwrap();
        assert_eq!(signals.actions, 3);
        assert_eq!(signals.comparisons, 3);
        assert!(signals.has_action_accumulation);
        assert!(signals.confidence >= 0.7);
    }

    #[test]
    fn rejects_validation_code() {
        let block: Block = parse_quote! {
            {
                let mut errors = vec![];
                if email.is_empty() {
                    errors.push("Email is required");
                }
                if !email.contains('@') {
                    errors.push("Invalid email format");
                }
                if errors.is_empty() {
                    Ok(())
                } else {
                    Err(errors)
                }
            }
        };

        let detector = StateMachinePatternDetector::new();
        let signals = detector.detect_coordinator(&block);

        assert!(
            signals.is_none(),
            "Validation code should not trigger coordinator pattern"
        );
    }

    #[test]
    fn distinguishes_error_from_action_accumulation() {
        // Error accumulation
        let error_push: ExprMethodCall = parse_quote! {
            errors.push("validation failed")
        };
        assert!(is_error_accumulation(&error_push));

        // Action accumulation (not error)
        let action_push: ExprMethodCall = parse_quote! {
            actions.push(Action::DoSomething)
        };
        assert!(!is_error_accumulation(&action_push));

        // Warning accumulation
        let warning_push: ExprMethodCall = parse_quote! {
            warnings.push("deprecated")
        };
        assert!(is_error_accumulation(&warning_push));
    }

    #[test]
    fn recognizes_state_comparisons() {
        // State comparison (should count)
        let state_comp: ExprBinary = parse_quote! {
            current.state != desired.state
        };
        assert!(is_state_comparison(&state_comp));

        // Mode comparison (should count)
        let mode_comp: ExprBinary = parse_quote! {
            current.mode == Mode::Active
        };
        assert!(is_state_comparison(&mode_comp));

        // Status comparison (should count)
        let status_comp: ExprBinary = parse_quote! {
            obj.status != target.status
        };
        assert!(is_state_comparison(&status_comp));

        // Non-state comparison (should not count)
        let value_comp: ExprBinary = parse_quote! {
            email.is_empty() == true
        };
        assert!(!is_state_comparison(&value_comp));
    }

    #[test]
    fn confidence_scoring_works() {
        // High confidence: clear coordinator
        let high = calculate_enhanced_coordinator_confidence(
            5,    // state-aware pushes
            4,    // state comparisons
            true, // has helper calls
            true, // has action types
            true, // has final dispatch
        );
        assert!(
            high >= 0.8,
            "High confidence should be >= 0.8, got {}",
            high
        );

        // Medium confidence
        let medium = calculate_enhanced_coordinator_confidence(
            3,     // state-aware pushes
            2,     // state comparisons
            false, // no helpers
            true,  // has action types
            false, // no dispatch
        );
        assert!(
            (0.6..0.8).contains(&medium),
            "Medium confidence should be 0.6-0.8, got {}",
            medium
        );

        // Low confidence: ambiguous pattern (should be rejected)
        let low = calculate_enhanced_coordinator_confidence(
            2,     // few pushes
            1,     // few comparisons
            false, // no helpers
            false, // no action types
            false, // no dispatch
        );
        assert!(low < 0.7, "Low confidence should be < 0.7, got {}", low);
    }

    #[test]
    fn detects_true_coordinator_pattern() {
        let block: Block = parse_quote! {
            {
                let mut actions = vec![];
                if current.state != desired.state {
                    actions.push(Action::TransitionState);
                }
                if current.mode != desired.mode {
                    actions.push(Action::ChangeMode);
                }
                if current.status != desired.status {
                    actions.push(Action::UpdateStatus);
                }
                actions
            }
        };

        let detector = StateMachinePatternDetector::new();
        let signals = detector.detect_coordinator(&block);

        assert!(signals.is_some(), "Should detect true coordinator");
        let signals = signals.unwrap();
        assert_eq!(signals.actions, 3);
        assert_eq!(signals.comparisons, 3);
        assert!(signals.confidence >= 0.7);
    }

    #[test]
    fn action_type_detection_works() {
        use syn::punctuated::Punctuated;
        use syn::token::Comma;

        // Explicit Action:: type
        let action_arg: Punctuated<Expr, Comma> = parse_quote! {
            Action::DoSomething
        };
        assert!(is_action_type(&action_arg));

        // Command:: type
        let command_arg: Punctuated<Expr, Comma> = parse_quote! {
            Command::Execute
        };
        assert!(is_action_type(&command_arg));

        // String literal (not action type)
        let string_arg: Punctuated<Expr, Comma> = parse_quote! {
            "error message"
        };
        assert!(!is_action_type(&string_arg));
    }

    #[test]
    fn rejects_mixed_validation_and_action_code() {
        let block: Block = parse_quote! {
            {
                let mut errors = vec![];
                let mut actions = vec![];

                if email.is_empty() {
                    errors.push("required");
                }
                if !email.contains('@') {
                    errors.push("invalid");
                }

                if current.state != desired.state {
                    actions.push(Action::Sync);
                }

                (errors, actions)
            }
        };

        let detector = StateMachinePatternDetector::new();
        let signals = detector.detect_coordinator(&block);

        // Should reject due to high error accumulation ratio
        assert!(
            signals.is_none(),
            "Mixed code with dominant error accumulation should be rejected"
        );
    }

    #[test]
    fn final_dispatch_detection() {
        let block_with_dispatch: Block = parse_quote! {
            {
                let mut actions = vec![];
                if current.state != desired.state {
                    actions.push(Action::Transition);
                }
                if current.mode != desired.mode {
                    actions.push(Action::Change);
                }
                if current.status != desired.status {
                    actions.push(Action::Update);
                }
                actions
            }
        };

        let detector = StateMachinePatternDetector::new();
        let signals = detector.detect_coordinator(&block_with_dispatch);

        assert!(signals.is_some());
        let signals = signals.unwrap();
        // Final dispatch should contribute to confidence
        assert!(signals.confidence >= 0.7);
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
