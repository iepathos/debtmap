//! State Machine Pattern Detector
//!
//! Detects state machine and coordinator patterns (spec 179, 192, 202) by analyzing:
//! - Match expressions on enum variants (state transitions)
//! - Conditional logic based on state comparisons
//! - Action accumulation and dispatching patterns
//!
//! Spec 192 adds state-aware coordinator detection to reduce false positives
//! on validation code by distinguishing:
//! - State comparisons vs value checks
//! - Action accumulation vs error accumulation
//!
//! Spec 202 adds enhanced state field detection with multi-strategy approach:
//! - Extended keyword dictionary (30+ terms)
//! - Type-based heuristics (enum analysis)
//! - Semantic pattern recognition (prefix/suffix)
//! - Usage frequency analysis
//! - Multi-factor confidence scoring

use crate::analyzers::state_field_detector::{
    ConfidenceClass, StateDetectionConfig, StateFieldDetector,
};
use crate::priority::complexity_patterns::{CoordinatorSignals, StateMachineSignals};
use syn::{
    visit::Visit, Arm, Block, Expr, ExprBinary, ExprField, ExprMatch, ExprMethodCall, Pat,
    PatTupleStruct, Stmt,
};

/// Detector for state machine and coordinator patterns
pub struct StateMachinePatternDetector {
    /// Enhanced state field detector (spec 202)
    state_detector: StateFieldDetector,
}

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
        Self {
            state_detector: StateFieldDetector::new(StateDetectionConfig::default()),
        }
    }

    /// Create detector with custom configuration
    pub fn with_config(config: StateDetectionConfig) -> Self {
        Self {
            state_detector: StateFieldDetector::new(config),
        }
    }

    /// Detect state machine pattern from AST block (enhanced in spec 202, spec 203)
    pub fn detect_state_machine(&self, block: &Block) -> Option<StateMachineSignals> {
        let mut visitor = StateMachineVisitor::new();
        visitor.visit_block(block);

        // NEW (spec 202): Enhanced state field detection
        let state_fields: Vec<_> = visitor
            .field_accesses
            .iter()
            .map(|field| self.state_detector.detect_state_field(field))
            .filter(|detection| detection.classification != ConfidenceClass::Low)
            .collect();

        // Enhanced evidence check: accept enum matches OR high-confidence state fields
        if !visitor.has_enum_match && state_fields.is_empty() {
            return None;
        }

        // Calculate field confidence for boosting
        let field_confidence: f64 = state_fields
            .iter()
            .map(|d| d.confidence)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        // Enhanced confidence calculation with state field detection
        let confidence = calculate_enhanced_state_machine_confidence(
            visitor.enum_match_count,
            visitor.tuple_match_count,
            state_fields.len() as u32,
            visitor.action_dispatch_count,
            field_confidence,
        );

        // Lowered threshold from 0.6 to 0.5 (spec 202)
        if confidence < 0.5 {
            return None;
        }

        // NEW (spec 203): Classify arms
        let mut primary = 0;
        let mut nested = 0;
        let mut delegated = 0;
        let mut trivial = 0;
        let mut complex = 0;
        let mut total_lines = 0;

        const TRIVIAL_THRESHOLD: u32 = 10;

        for arm in &visitor.arm_metrics {
            if arm.is_primary_match {
                primary += 1;
            } else {
                nested += 1;
            }

            if arm.is_delegated {
                delegated += 1;
            } else if arm.inline_lines < TRIVIAL_THRESHOLD {
                trivial += 1;
            } else {
                complex += 1;
                total_lines += arm.inline_lines;
            }
        }

        let avg_complexity = if !visitor.arm_metrics.is_empty() {
            visitor
                .arm_metrics
                .iter()
                .map(|a| a.inline_lines)
                .sum::<u32>() as f32
                / visitor.arm_metrics.len() as f32
        } else {
            0.0
        };

        Some(StateMachineSignals {
            transition_count: visitor.enum_match_count + visitor.tuple_match_count,
            match_expression_count: visitor.match_expression_count,
            has_enum_match: visitor.has_enum_match,
            has_state_comparison: !state_fields.is_empty(),
            action_dispatch_count: visitor.action_dispatch_count,
            confidence,
            primary_match_arms: primary,
            nested_match_arms: nested,
            delegated_arms: delegated,
            trivial_arms: trivial,
            complex_inline_arms: complex,
            total_inline_lines: total_lines,
            avg_arm_complexity: avg_complexity,
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

/// Calculate confidence for state machine pattern (legacy)
#[allow(dead_code)]
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

/// Enhanced confidence calculation with state field detection (spec 202)
fn calculate_enhanced_state_machine_confidence(
    enum_match_count: u32,
    tuple_match_count: u32,
    state_field_count: u32,
    action_dispatch_count: u32,
    max_field_confidence: f64,
) -> f64 {
    let mut confidence = 0.0;

    // Enum/tuple matching (original logic)
    if enum_match_count > 0 || tuple_match_count >= 2 {
        confidence += 0.5;
    }

    // NEW (spec 202): State field detection confidence
    if state_field_count > 0 {
        confidence += max_field_confidence * 0.4; // Weight: 40% of field confidence
    }

    // Action dispatch (original logic)
    if action_dispatch_count >= 2 {
        confidence += 0.2;
    }

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

/// Metrics for a single match arm (spec 203)
#[derive(Debug, Clone)]
#[allow(dead_code)] // has_nested_match and arm_index reserved for future use
struct ArmMetrics {
    is_delegated: bool,     // Calls single function
    inline_lines: u32,      // Estimated LOC
    has_nested_match: bool, // Contains nested match
    arm_index: usize,       // Position in match
    is_primary_match: bool, // In primary vs nested match
}

/// Visitor to detect state machine patterns
struct StateMachineVisitor {
    enum_match_count: u32,
    tuple_match_count: u32,
    state_comparison_count: u32,
    action_dispatch_count: u32,
    match_expression_count: u32,
    has_enum_match: bool,
    /// NEW (spec 202): Collect field accesses for enhanced detection
    field_accesses: Vec<ExprField>,
    /// NEW (spec 203): Arm-level metrics
    arm_metrics: Vec<ArmMetrics>,
    match_nesting_depth: u32,
    in_primary_match: bool,
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
            field_accesses: Vec::new(),
            arm_metrics: Vec::new(),
            match_nesting_depth: 0,
            in_primary_match: false,
        }
    }

    /// Analyze a match arm and collect metrics (spec 203)
    fn analyze_arm(&self, arm: &Arm, index: usize) -> ArmMetrics {
        let is_delegated = is_delegated_to_handler(&arm.body);
        let inline_lines = estimate_arm_lines(&arm.body);
        let has_nested_match = contains_nested_match(&arm.body);

        ArmMetrics {
            is_delegated,
            inline_lines,
            has_nested_match,
            arm_index: index,
            is_primary_match: self.in_primary_match,
        }
    }

    /// Check if a pattern is an enum variant or tuple pattern
    fn is_enum_or_tuple_pattern(&self, pat: &Pat) -> bool {
        matches!(
            pat,
            Pat::TupleStruct(_) | Pat::Struct(_) | Pat::Ident(_) | Pat::Tuple(_) | Pat::Path(_)
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
        // Track match nesting depth (spec 203)
        let was_in_primary = self.in_primary_match;
        if self.match_nesting_depth == 0 {
            self.in_primary_match = true;
        }
        self.match_nesting_depth += 1;

        // Count this match expression
        self.match_expression_count += 1;

        // NEW (spec 202): Collect field access from match expression
        if let Expr::Field(field_expr) = match_expr.expr.as_ref() {
            self.field_accesses.push(field_expr.clone());
        }

        // Check if matching on state-related expression
        if self.has_state_field_access(&match_expr.expr) {
            self.has_enum_match = true;
            self.state_comparison_count += 1;
        }

        // Count enum/tuple patterns in arms and collect arm metrics (spec 203)
        for (idx, arm) in match_expr.arms.iter().enumerate() {
            // NEW (spec 203): Collect arm metrics
            let metrics = self.analyze_arm(arm, idx);
            self.arm_metrics.push(metrics);

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

        // Restore nesting state (spec 203)
        self.match_nesting_depth -= 1;
        self.in_primary_match = was_in_primary;
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        // Count state comparisons in if expressions
        if let Expr::If(if_expr) = expr {
            if self.has_state_field_access(&if_expr.cond) {
                self.state_comparison_count += 1;
            }
        }

        // NEW (spec 202): Collect field accesses for enhanced detection
        if let Expr::Field(field_expr) = expr {
            self.field_accesses.push(field_expr.clone());
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

/// Check if an arm body delegates to a handler function (spec 203)
fn is_delegated_to_handler(body: &Expr) -> bool {
    match body {
        // Direct call: handle_foo()?
        Expr::Try(try_expr) => matches!(*try_expr.expr, Expr::Call(_) | Expr::MethodCall(_)),
        // Direct call without ?: handle_foo()
        Expr::Call(_) | Expr::MethodCall(_) => true,
        // Block with single statement that's a call
        Expr::Block(block) if block.block.stmts.len() == 1 => {
            matches!(
                &block.block.stmts[0],
                Stmt::Expr(Expr::Call(_), _)
                    | Stmt::Expr(Expr::Try(_), _)
                    | Stmt::Expr(Expr::MethodCall(_), _)
            )
        }
        _ => false,
    }
}

/// Estimate lines of code in an arm body (spec 203)
fn estimate_arm_lines(body: &Expr) -> u32 {
    let mut counter = LineCounter::new();
    counter.visit_expr(body);
    counter.estimated_lines
}

/// Visitor to count lines in an expression
struct LineCounter {
    estimated_lines: u32,
}

impl LineCounter {
    fn new() -> Self {
        Self { estimated_lines: 0 }
    }
}

impl<'ast> Visit<'ast> for LineCounter {
    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        self.estimated_lines += 1;
        syn::visit::visit_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            Expr::Match(_) | Expr::If(_) | Expr::ForLoop(_) | Expr::While(_) => {
                self.estimated_lines += 1;
            }
            Expr::Struct(struct_expr) => {
                // Count field assignments in struct initialization
                self.estimated_lines += struct_expr.fields.len() as u32;
            }
            _ => {}
        }
        syn::visit::visit_expr(self, expr);
    }
}

/// Check if an expression contains nested match expressions (spec 203)
fn contains_nested_match(body: &Expr) -> bool {
    struct MatchFinder {
        found: bool,
    }
    impl<'ast> Visit<'ast> for MatchFinder {
        fn visit_expr_match(&mut self, _: &'ast ExprMatch) {
            self.found = true;
        }
    }
    let mut finder = MatchFinder { found: false };
    finder.visit_expr(body);
    finder.found
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

    #[test]
    fn enhanced_detection_with_fsm_state_field() {
        let block: Block = parse_quote! {
            {
                match self.fsm_state {
                    FsmState::Idle => self.handle_idle(),
                    FsmState::Processing => self.handle_processing(),
                    FsmState::Complete => self.handle_complete(),
                }
            }
        };

        let detector = StateMachinePatternDetector::new();
        let signals = detector.detect_state_machine(&block);

        assert!(
            signals.is_some(),
            "Should detect state machine with fsm_state field"
        );
        let signals = signals.unwrap();
        assert!(signals.confidence >= 0.5, "Confidence should be >= 0.5");
    }

    #[test]
    fn enhanced_detection_with_prefix_pattern() {
        let block: Block = parse_quote! {
            {
                match self.current_operation {
                    Operation::Read => { /* ... */ }
                    Operation::Write => { /* ... */ }
                    Operation::Delete => { /* ... */ }
                }
            }
        };

        let detector = StateMachinePatternDetector::new();
        let signals = detector.detect_state_machine(&block);

        assert!(
            signals.is_some(),
            "Should detect state machine with current_* prefix"
        );
    }

    #[test]
    fn enhanced_detection_with_suffix_pattern() {
        let block: Block = parse_quote! {
            {
                match self.connection_state {
                    ConnectionState::Idle => { /* ... */ }
                    ConnectionState::Connecting => { /* ... */ }
                    ConnectionState::Connected => { /* ... */ }
                }
            }
        };

        let detector = StateMachinePatternDetector::new();
        let signals = detector.detect_state_machine(&block);

        assert!(
            signals.is_some(),
            "Should detect state machine with *_state suffix"
        );
    }

    // Spec 203 tests - Arm-level analysis

    #[test]
    fn test_delegation_detection() {
        use syn::parse_quote;

        // Delegated: direct call with ?
        let arm1: Expr = parse_quote! { handle_a()? };
        assert!(is_delegated_to_handler(&arm1));

        // Delegated: direct call
        let arm2: Expr = parse_quote! { handle_b(x, y)? };
        assert!(is_delegated_to_handler(&arm2));

        // Delegated: method call
        let arm3: Expr = parse_quote! { self.handle_c()? };
        assert!(is_delegated_to_handler(&arm3));

        // Not delegated: block with single call (but has wrapper)
        let arm4: Expr = parse_quote! {
            {
                handle_d()
            }
        };
        assert!(is_delegated_to_handler(&arm4));

        // Not delegated: multi-statement block
        let arm5: Expr = parse_quote! {
            {
                let cfg = build_config();
                process(cfg)?;
                Ok(())
            }
        };
        assert!(!is_delegated_to_handler(&arm5));
    }

    #[test]
    fn test_arm_complexity_estimation() {
        use syn::parse_quote;

        // Trivial arm (1 line)
        let simple: Expr = parse_quote! { do_thing() };
        let lines = estimate_arm_lines(&simple);
        assert!(lines < 10, "Simple arm should be < 10 lines, got {}", lines);

        // Complex arm with struct initialization
        let complex: Expr = parse_quote! {
            {
                let validate_config = ValidateConfig {
                    path: path,
                    config: config,
                    format: match format {
                        Fmt::Json => Format::Json,
                        Fmt::Markdown => Format::Markdown,
                    },
                    output: None,
                    top: 10,
                };
                debtmap::commands::validate::validate_project(validate_config)?;
                Ok(())
            }
        };
        let lines = estimate_arm_lines(&complex);
        assert!(
            lines >= 10,
            "Complex arm should be >= 10 lines, got {}",
            lines
        );
    }

    #[test]
    fn test_primary_vs_nested_match_tracking() {
        let block: Block = parse_quote! {
            {
                match cmd {
                    Cmd::A => handle_a()?,
                    Cmd::B => {
                        let fmt = match format {
                            Fmt::Json => json(),
                            Fmt::Text => text(),
                        };
                        handle_b(fmt)?
                    }
                    Cmd::C => handle_c()?,
                }
            }
        };

        let detector = StateMachinePatternDetector::new();
        let signals = detector.detect_state_machine(&block).unwrap();

        assert_eq!(signals.primary_match_arms, 3, "Should have 3 primary arms");
        assert_eq!(signals.nested_match_arms, 2, "Should have 2 nested arms");
    }

    #[test]
    fn test_arm_classification() {
        let block: Block = parse_quote! {
            {
                match command {
                    // Delegated
                    Commands::Analyze { .. } => handle_analyze_command(command)?,
                    Commands::Compare { .. } => handle_compare_command(before, after)?,

                    // Trivial
                    Commands::Init { force } => {
                        debtmap::commands::init::init_config(force)?;
                        Ok(())
                    }

                    // Complex inline
                    Commands::Validate { path, config } => {
                        let validate_config = ValidateConfig {
                            path: path,
                            config: config,
                            format: match format {
                                Fmt::Json => Format::Json,
                                Fmt::Markdown => Format::Markdown,
                                Fmt::Text => Format::Text,
                            },
                            output: None,
                            top: 10,
                        };
                        debtmap::commands::validate::validate_project(validate_config)?;
                        Ok(())
                    }
                }
            }
        };

        let detector = StateMachinePatternDetector::new();
        let signals = detector.detect_state_machine(&block).unwrap();

        assert_eq!(signals.primary_match_arms, 4);
        assert_eq!(signals.delegated_arms, 2, "Analyze and Compare");
        assert!(signals.trivial_arms >= 1, "Init is trivial");
        assert!(signals.complex_inline_arms >= 1, "Validate is complex");
        assert_eq!(signals.nested_match_arms, 3, "Format conversion match");
    }

    #[test]
    fn test_clean_state_machine_no_complex_arms() {
        let block: Block = parse_quote! {
            {
                match cmd {
                    Command::Start => handle_start()?,
                    Command::Stop => handle_stop()?,
                    Command::Restart => handle_restart()?,
                }
            }
        };

        let detector = StateMachinePatternDetector::new();
        let signals = detector.detect_state_machine(&block).unwrap();

        assert_eq!(signals.complex_inline_arms, 0, "All arms delegated");
        assert_eq!(signals.delegated_arms, 3, "Should have 3 delegated");
    }

    #[test]
    fn test_contains_nested_match() {
        use syn::parse_quote;

        // Has nested match
        let with_match: Expr = parse_quote! {
            {
                let fmt = match format {
                    Fmt::Json => json(),
                    Fmt::Text => text(),
                };
                process(fmt)
            }
        };
        assert!(contains_nested_match(&with_match));

        // No nested match
        let without_match: Expr = parse_quote! {
            {
                let cfg = build_config();
                process(cfg)
            }
        };
        assert!(!contains_nested_match(&without_match));
    }
}
