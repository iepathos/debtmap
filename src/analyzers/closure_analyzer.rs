//! Closure analysis for purity detection.
//!
//! This module provides dedicated analysis for closure expressions,
//! including capture detection, capture mode inference, and purity
//! propagation for higher-order functions and iterator chains.

use std::collections::HashSet;
use syn::{visit::Visit, Expr, ExprClosure};

use super::purity_detector::{MutationScope, PurityDetector};
use super::scope_tracker::ScopeTracker;
use crate::core::PurityLevel;

/// Result of closure purity analysis
#[derive(Debug, Clone)]
pub struct ClosurePurity {
    pub level: PurityLevel,
    pub confidence: f32,
    pub captures: Vec<Capture>,
    pub has_nested_closures: bool,
}

/// Capture mode for closure variables
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureMode {
    /// By-value capture (move closure)
    ByValue,
    /// By-reference capture (immutable)
    ByRef,
    /// By-mutable-reference capture
    ByMutRef,
}

/// Information about a captured variable
#[derive(Debug, Clone)]
pub struct Capture {
    pub var_name: String,
    pub mode: CaptureMode,
    pub is_mutated: bool,
    pub scope: MutationScope,
}

/// Dedicated analyzer for closure expressions
#[derive(Debug)]
pub struct ClosureAnalyzer<'a> {
    /// Parent scope for determining captures
    parent_scope: &'a ScopeTracker,
    /// Detected captures
    captures: Vec<Capture>,
    /// Confidence reduction factors
    confidence_penalties: Vec<&'static str>,
}

impl<'a> ClosureAnalyzer<'a> {
    /// Create new closure analyzer with parent scope
    pub fn new(parent_scope: &'a ScopeTracker) -> Self {
        Self {
            parent_scope,
            captures: Vec::new(),
            confidence_penalties: Vec::new(),
        }
    }

    /// Main entry point: Analyze a closure expression
    pub fn analyze_closure(&mut self, closure: &ExprClosure) -> ClosurePurity {
        // Step 1: Create isolated detector for closure body
        let mut body_detector = PurityDetector::new();

        // Step 2: Register closure parameters in body scope
        for input in &closure.inputs {
            if let syn::Pat::Ident(pat_ident) = input {
                body_detector
                    .scope_mut()
                    .add_local_var(pat_ident.ident.to_string());
            }
        }

        // Step 3: Analyze closure body
        body_detector.visit_expr(&closure.body);

        // Step 4: Detect captures (free variables)
        self.captures = self.find_captures(closure, &body_detector);

        // Step 5: Infer capture modes from usage
        self.infer_capture_modes(closure, &body_detector);

        // Step 6: Check for nested closures
        let has_nested_closures = self.contains_nested_closures(&closure.body);
        if has_nested_closures {
            self.confidence_penalties.push("nested_closures");
        }

        // Step 7: Determine purity level
        let level = self.determine_purity_level(&body_detector);

        // Step 8: Calculate confidence
        let confidence = self.calculate_confidence(&body_detector);

        ClosurePurity {
            level,
            confidence,
            captures: self.captures.clone(),
            has_nested_closures,
        }
    }

    /// Detect captured variables (free variables in closure body)
    fn find_captures(
        &self,
        closure: &ExprClosure,
        _body_detector: &PurityDetector,
    ) -> Vec<Capture> {
        // Collect parameter names
        let mut params: HashSet<String> = HashSet::new();
        for input in &closure.inputs {
            if let syn::Pat::Ident(pat_ident) = input {
                params.insert(pat_ident.ident.to_string());
            }
        }

        // Walk body and find variable references
        let mut visitor = CaptureDetector {
            params: &params,
            parent_scope: self.parent_scope,
            captures: Vec::new(),
        };
        visitor.visit_expr(&closure.body);

        visitor.captures
    }

    /// Infer capture modes based on usage patterns
    fn infer_capture_modes(&mut self, closure: &ExprClosure, body_detector: &PurityDetector) {
        // Check for 'move' keyword
        let has_move = closure.capture.is_some();

        for capture in &mut self.captures {
            // 'move' forces by-value capture
            if has_move {
                capture.mode = CaptureMode::ByValue;
                continue;
            }

            // Check if captured variable is mutated in closure body
            let is_mutated = body_detector
                .local_mutations()
                .iter()
                .any(|m| m.target == capture.var_name);

            capture.is_mutated = is_mutated;

            // Infer mode: mutated → mut ref, otherwise immut ref
            capture.mode = if is_mutated {
                CaptureMode::ByMutRef
            } else {
                CaptureMode::ByRef
            };

            // Determine scope (local to function vs external)
            capture.scope = if self.parent_scope.is_local(&capture.var_name) {
                MutationScope::Local
            } else {
                MutationScope::External
            };
        }
    }

    /// Check if closure body contains nested closures
    fn contains_nested_closures(&self, expr: &Expr) -> bool {
        let mut visitor = ClosureDetector { found: false };
        visitor.visit_expr(expr);
        visitor.found
    }

    /// Determine purity level based on closure behavior
    fn determine_purity_level(&self, body_detector: &PurityDetector) -> PurityLevel {
        // Has I/O or unsafe operations?
        if body_detector.has_io_operations() || body_detector.has_unsafe_blocks() {
            return PurityLevel::Impure;
        }

        // Modifies external state?
        if body_detector.modifies_external_state() {
            return PurityLevel::Impure;
        }

        // Check captured variable mutations
        let mutates_external = self
            .captures
            .iter()
            .any(|c| c.is_mutated && c.scope == MutationScope::External);

        if mutates_external {
            return PurityLevel::Impure;
        }

        // Mutates local captures only?
        let mutates_local = self
            .captures
            .iter()
            .any(|c| c.is_mutated && c.scope == MutationScope::Local);

        if mutates_local || !body_detector.local_mutations().is_empty() {
            return PurityLevel::LocallyPure;
        }

        // Accesses external state (reads)?
        if body_detector.accesses_external_state() {
            return PurityLevel::ReadOnly;
        }

        // No side effects detected
        PurityLevel::StrictlyPure
    }

    /// Calculate confidence score with penalty factors
    fn calculate_confidence(&self, body_detector: &PurityDetector) -> f32 {
        let mut confidence: f32 = 1.0;

        // Reduce confidence for nested closures
        if self.confidence_penalties.contains(&"nested_closures") {
            confidence *= 0.85;
        }

        // Reduce confidence for external state access
        if body_detector.accesses_external_state() {
            confidence *= 0.80;
        }

        // Reduce confidence for multiple captures
        if self.captures.len() > 3 {
            confidence *= 0.90;
        }

        // Reduce confidence if capture modes were inferred
        if self.captures.iter().any(|c| c.mode != CaptureMode::ByValue) {
            confidence *= 0.95;
        }

        confidence.clamp(0.5, 1.0)
    }
}

/// Helper visitor to detect captured variables
struct CaptureDetector<'a> {
    params: &'a HashSet<String>,
    parent_scope: &'a ScopeTracker,
    captures: Vec<Capture>,
}

impl<'ast, 'a> Visit<'ast> for CaptureDetector<'a> {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        if let Expr::Path(path) = expr {
            if let Some(ident) = path.path.get_ident() {
                let name = ident.to_string();

                // Not a parameter and not a standard construct?
                if !self.params.contains(&name) && name != "self" && name != "Self" {
                    // Check if it's in parent scope (captured)
                    if self.parent_scope.is_local(&name) || self.parent_scope.is_self(&name) {
                        // Add if not already captured
                        if !self.captures.iter().any(|c| c.var_name == name) {
                            self.captures.push(Capture {
                                var_name: name,
                                mode: CaptureMode::ByRef, // Default, refined later
                                is_mutated: false,
                                scope: MutationScope::Local,
                            });
                        }
                    }
                }
            }
        }

        syn::visit::visit_expr(self, expr);
    }
}

/// Helper visitor to detect nested closures
struct ClosureDetector {
    found: bool,
}

impl<'ast> Visit<'ast> for ClosureDetector {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        if matches!(expr, Expr::Closure(_)) {
            self.found = true;
            return;
        }
        syn::visit::visit_expr(self, expr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_simple_closure_no_captures() {
        let closure: ExprClosure = parse_quote!(|x| x * 2);
        let parent_scope = ScopeTracker::new();
        let mut analyzer = ClosureAnalyzer::new(&parent_scope);

        let result = analyzer.analyze_closure(&closure);

        assert_eq!(result.level, PurityLevel::StrictlyPure);
        assert!(result.captures.is_empty());
        assert!(!result.has_nested_closures);
    }

    #[test]
    fn test_closure_with_capture() {
        let closure: ExprClosure = parse_quote!(|x| x + y);
        let mut parent_scope = ScopeTracker::new();
        parent_scope.add_local_var("y".to_string());
        let mut analyzer = ClosureAnalyzer::new(&parent_scope);

        let result = analyzer.analyze_closure(&closure);

        assert_eq!(result.captures.len(), 1);
        assert_eq!(result.captures[0].var_name, "y");
        assert_eq!(result.captures[0].mode, CaptureMode::ByRef);
    }

    #[test]
    fn test_move_closure() {
        let closure: ExprClosure = parse_quote!(move |x| x + y);
        let mut parent_scope = ScopeTracker::new();
        parent_scope.add_local_var("y".to_string());
        let mut analyzer = ClosureAnalyzer::new(&parent_scope);

        let result = analyzer.analyze_closure(&closure);

        assert_eq!(result.captures.len(), 1);
        assert_eq!(result.captures[0].mode, CaptureMode::ByValue);
    }

    #[test]
    fn test_nested_closure_detection() {
        let closure: ExprClosure = parse_quote!(|x| {
            let f = |y| y * 2;
            f(x)
        });
        let parent_scope = ScopeTracker::new();
        let mut analyzer = ClosureAnalyzer::new(&parent_scope);

        let result = analyzer.analyze_closure(&closure);

        assert!(result.has_nested_closures);
        assert!(result.confidence < 0.9); // Confidence reduced
    }
}
