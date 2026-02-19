//! Variable scope tracking for TypeScript/JavaScript purity analysis
//!
//! Tracks variable declarations to distinguish local from external mutations.

#![allow(dead_code)] // Methods reserved for future purity analysis enhancements

use std::collections::HashSet;

/// Tracks variable scopes to determine mutation locality
#[derive(Debug, Default)]
pub struct JsScopeTracker {
    /// Stack of scopes, innermost at the end
    scopes: Vec<Scope>,
}

/// A single scope level
#[derive(Debug, Default)]
struct Scope {
    /// Kind of scope
    kind: ScopeKind,
    /// Variables declared in this scope (let, const, var)
    variables: HashSet<String>,
    /// Function parameters
    params: HashSet<String>,
}

/// Kind of scope
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    /// Function scope (function declaration, expression)
    #[default]
    Function,
    /// Block scope (if, for, while, etc.)
    Block,
    /// Arrow function scope
    Arrow,
    /// Class scope
    Class,
}

impl JsScopeTracker {
    /// Create a new scope tracker
    pub fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    /// Enter a function scope with parameters
    pub fn enter_function(&mut self, params: Vec<String>) {
        let mut scope = Scope {
            kind: ScopeKind::Function,
            variables: HashSet::new(),
            params: HashSet::new(),
        };
        for param in params {
            scope.params.insert(param);
        }
        self.scopes.push(scope);
    }

    /// Enter an arrow function scope with parameters
    pub fn enter_arrow(&mut self, params: Vec<String>) {
        let mut scope = Scope {
            kind: ScopeKind::Arrow,
            variables: HashSet::new(),
            params: HashSet::new(),
        };
        for param in params {
            scope.params.insert(param);
        }
        self.scopes.push(scope);
    }

    /// Enter a block scope
    pub fn enter_block(&mut self) {
        self.scopes.push(Scope {
            kind: ScopeKind::Block,
            variables: HashSet::new(),
            params: HashSet::new(),
        });
    }

    /// Enter a class scope
    pub fn enter_class(&mut self) {
        self.scopes.push(Scope {
            kind: ScopeKind::Class,
            variables: HashSet::new(),
            params: HashSet::new(),
        });
    }

    /// Exit the current scope
    pub fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    /// Declare a variable in the current scope
    pub fn declare_variable(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.variables.insert(name.to_string());
        }
    }

    /// Check if a variable is local (declared in any enclosing scope)
    pub fn is_local(&self, name: &str) -> bool {
        self.scopes
            .iter()
            .any(|scope| scope.variables.contains(name) || scope.params.contains(name))
    }

    /// Check if a variable is a function parameter
    pub fn is_param(&self, name: &str) -> bool {
        self.scopes.iter().any(|scope| scope.params.contains(name))
    }

    /// Check if we're inside any scope
    pub fn in_scope(&self) -> bool {
        !self.scopes.is_empty()
    }

    /// Get the current scope kind
    pub fn current_kind(&self) -> Option<ScopeKind> {
        self.scopes.last().map(|s| s.kind)
    }

    /// Check if we're in a function or arrow function scope
    pub fn in_function_scope(&self) -> bool {
        self.scopes
            .iter()
            .rev()
            .any(|s| matches!(s.kind, ScopeKind::Function | ScopeKind::Arrow))
    }

    /// Get the nesting depth
    pub fn depth(&self) -> usize {
        self.scopes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tracker() {
        let tracker = JsScopeTracker::new();
        assert!(!tracker.in_scope());
        assert_eq!(tracker.depth(), 0);
    }

    #[test]
    fn test_enter_function() {
        let mut tracker = JsScopeTracker::new();
        tracker.enter_function(vec!["a".to_string(), "b".to_string()]);

        assert!(tracker.in_scope());
        assert!(tracker.is_local("a"));
        assert!(tracker.is_local("b"));
        assert!(tracker.is_param("a"));
        assert!(!tracker.is_local("c"));
    }

    #[test]
    fn test_declare_variable() {
        let mut tracker = JsScopeTracker::new();
        tracker.enter_function(vec![]);
        tracker.declare_variable("x");

        assert!(tracker.is_local("x"));
        assert!(!tracker.is_param("x"));
    }

    #[test]
    fn test_nested_scopes() {
        let mut tracker = JsScopeTracker::new();
        tracker.enter_function(vec!["outer".to_string()]);
        tracker.declare_variable("x");
        tracker.enter_block();
        tracker.declare_variable("y");

        assert!(tracker.is_local("outer"));
        assert!(tracker.is_local("x"));
        assert!(tracker.is_local("y"));
        assert_eq!(tracker.depth(), 2);

        tracker.exit_scope();
        assert!(tracker.is_local("x"));
        assert!(!tracker.is_local("y")); // y was in the exited block
        assert_eq!(tracker.depth(), 1);
    }

    #[test]
    fn test_current_kind() {
        let mut tracker = JsScopeTracker::new();
        assert_eq!(tracker.current_kind(), None);

        tracker.enter_function(vec![]);
        assert_eq!(tracker.current_kind(), Some(ScopeKind::Function));

        tracker.enter_block();
        assert_eq!(tracker.current_kind(), Some(ScopeKind::Block));

        tracker.exit_scope();
        assert_eq!(tracker.current_kind(), Some(ScopeKind::Function));
    }

    #[test]
    fn test_in_function_scope() {
        let mut tracker = JsScopeTracker::new();
        assert!(!tracker.in_function_scope());

        tracker.enter_block();
        assert!(!tracker.in_function_scope());

        tracker.exit_scope();
        tracker.enter_function(vec![]);
        assert!(tracker.in_function_scope());

        tracker.enter_block();
        assert!(tracker.in_function_scope()); // Still in a function
    }
}
