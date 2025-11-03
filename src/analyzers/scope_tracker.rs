//! Scope tracking for purity analysis.
//!
//! This module provides utilities for tracking variables in scope during AST traversal.
//! It distinguishes between:
//! - Function parameters (including self variants)
//! - Local variables
//! - Different kinds of self references
//!
//! Used by `PurityDetector` to classify mutations as local vs external.

use std::collections::{HashMap, HashSet};
use syn::FnArg;

/// Tracks variables in scope during AST traversal
#[derive(Debug, Clone)]
pub struct ScopeTracker {
    /// Set of local variable identifiers
    local_vars: HashSet<String>,

    /// Parameters (including mutability and self info)
    params: HashMap<String, ParameterInfo>,

    /// Current scope depth (for nested blocks)
    scope_depth: usize,
}

/// Information about a function parameter
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    /// Parameter name
    pub name: String,
    /// Whether the parameter is mutable
    pub is_mut: bool,
    /// Whether this is a self parameter
    pub is_self: bool,
    /// Kind of self parameter (if applicable)
    pub self_kind: Option<SelfKind>,
}

/// Kind of self parameter in a method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfKind {
    /// &self - immutable reference
    Ref,
    /// &mut self - mutable reference
    MutRef,
    /// self - owned value
    Owned,
    /// mut self - mutable owned value
    MutOwned,
}

impl ScopeTracker {
    /// Create a new empty scope tracker
    pub fn new() -> Self {
        Self {
            local_vars: HashSet::new(),
            params: HashMap::new(),
            scope_depth: 0,
        }
    }

    /// Add a parameter to the scope
    ///
    /// Parses the parameter to extract name, mutability, and self kind information.
    pub fn add_parameter(&mut self, param: &FnArg) {
        match param {
            FnArg::Receiver(receiver) => {
                let kind = match (&receiver.reference, &receiver.mutability) {
                    (Some(_), Some(_)) => SelfKind::MutRef, // &mut self
                    (Some(_), None) => SelfKind::Ref,       // &self
                    (None, Some(_)) => SelfKind::MutOwned,  // mut self
                    (None, None) => SelfKind::Owned,        // self
                };

                self.params.insert(
                    "self".to_string(),
                    ParameterInfo {
                        name: "self".to_string(),
                        is_mut: receiver.mutability.is_some(),
                        is_self: true,
                        self_kind: Some(kind),
                    },
                );
            }
            FnArg::Typed(pat_type) => {
                if let syn::Pat::Ident(ident) = &*pat_type.pat {
                    self.params.insert(
                        ident.ident.to_string(),
                        ParameterInfo {
                            name: ident.ident.to_string(),
                            is_mut: ident.mutability.is_some(),
                            is_self: false,
                            self_kind: None,
                        },
                    );
                }
            }
        }
    }

    /// Add a local variable to the current scope
    pub fn add_local_var(&mut self, name: String) {
        self.local_vars.insert(name);
    }

    /// Check if an identifier is a local variable or owned parameter
    ///
    /// Returns true if the name is either:
    /// - A local variable, or
    /// - A non-self parameter
    pub fn is_local(&self, name: &str) -> bool {
        self.local_vars.contains(name) || self.params.get(name).is_some_and(|p| !p.is_self)
    }

    /// Check if an identifier is self (any kind)
    pub fn is_self(&self, name: &str) -> bool {
        self.params.get(name).is_some_and(|p| p.is_self)
    }

    /// Get the kind of self parameter, if present
    pub fn get_self_kind(&self) -> Option<SelfKind> {
        self.params.get("self").and_then(|p| p.self_kind)
    }

    /// Enter a nested scope (e.g., if block, loop)
    pub fn enter_scope(&mut self) {
        self.scope_depth += 1;
    }

    /// Exit the current scope
    pub fn exit_scope(&mut self) {
        self.scope_depth = self.scope_depth.saturating_sub(1);
        // Note: We keep all local vars for simplicity
        // Could implement proper scope shadowing if needed
    }
}

impl Default for ScopeTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_new_tracker_is_empty() {
        let tracker = ScopeTracker::new();
        assert!(!tracker.is_local("x"));
        assert!(!tracker.is_self("self"));
        assert_eq!(tracker.get_self_kind(), None);
    }

    #[test]
    fn test_add_regular_parameter() {
        let mut tracker = ScopeTracker::new();
        let param: FnArg = parse_quote!(x: i32);
        tracker.add_parameter(&param);

        assert!(tracker.is_local("x"));
        assert!(!tracker.is_self("x"));
    }

    #[test]
    fn test_add_mut_parameter() {
        let mut tracker = ScopeTracker::new();
        let param: FnArg = parse_quote!(mut x: i32);
        tracker.add_parameter(&param);

        assert!(tracker.is_local("x"));
    }

    #[test]
    fn test_self_ref() {
        let mut tracker = ScopeTracker::new();
        let param: FnArg = parse_quote!(&self);
        tracker.add_parameter(&param);

        assert!(tracker.is_self("self"));
        assert_eq!(tracker.get_self_kind(), Some(SelfKind::Ref));
    }

    #[test]
    fn test_self_mut_ref() {
        let mut tracker = ScopeTracker::new();
        let param: FnArg = parse_quote!(&mut self);
        tracker.add_parameter(&param);

        assert!(tracker.is_self("self"));
        assert_eq!(tracker.get_self_kind(), Some(SelfKind::MutRef));
    }

    #[test]
    fn test_self_owned() {
        let mut tracker = ScopeTracker::new();
        let param: FnArg = parse_quote!(self);
        tracker.add_parameter(&param);

        assert!(tracker.is_self("self"));
        assert_eq!(tracker.get_self_kind(), Some(SelfKind::Owned));
    }

    #[test]
    fn test_self_mut_owned() {
        let mut tracker = ScopeTracker::new();
        let param: FnArg = parse_quote!(mut self);
        tracker.add_parameter(&param);

        assert!(tracker.is_self("self"));
        assert_eq!(tracker.get_self_kind(), Some(SelfKind::MutOwned));
    }

    #[test]
    fn test_add_local_variable() {
        let mut tracker = ScopeTracker::new();
        tracker.add_local_var("result".to_string());

        assert!(tracker.is_local("result"));
        assert!(!tracker.is_self("result"));
    }

    #[test]
    fn test_scope_depth() {
        let mut tracker = ScopeTracker::new();
        assert_eq!(tracker.scope_depth, 0);

        tracker.enter_scope();
        assert_eq!(tracker.scope_depth, 1);

        tracker.enter_scope();
        assert_eq!(tracker.scope_depth, 2);

        tracker.exit_scope();
        assert_eq!(tracker.scope_depth, 1);

        tracker.exit_scope();
        assert_eq!(tracker.scope_depth, 0);
    }
}
