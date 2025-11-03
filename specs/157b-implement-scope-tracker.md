---
number: 157b
title: Implement ScopeTracker Module
category: foundation
priority: critical
status: draft
dependencies: [157a]
created: 2025-11-03
parent_spec: 157
---

# Specification 157b: Implement ScopeTracker Module

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: 157a (PurityLevel enum must exist)
**Parent Spec**: 157 - Local vs External Mutation Distinction

## Context

This is **Stage 2** of implementing local vs external mutation distinction (Spec 157). This stage creates a standalone `ScopeTracker` module for tracking local variables, parameters, and self variants.

This module will be used by PurityDetector in Stage 3 but is implemented independently here for easier testing and review.

## Objective

Create `src/analyzers/scope_tracker.rs` module with comprehensive scope tracking capabilities.

## Requirements

### Functional Requirements

1. **Create scope_tracker.rs** with:
   - `ScopeTracker` struct for tracking variables in scope
   - `ParameterInfo` struct for function parameters
   - `SelfKind` enum for different self variants
   - Methods to add/query variables and parameters

2. **ScopeTracker API**:
   ```rust
   pub struct ScopeTracker {
       local_vars: HashSet<String>,
       params: HashMap<String, ParameterInfo>,
       scope_depth: usize,
   }

   impl ScopeTracker {
       pub fn new() -> Self;
       pub fn add_parameter(&mut self, param: &syn::FnArg);
       pub fn add_local_var(&mut self, name: String);
       pub fn is_local(&self, name: &str) -> bool;
       pub fn is_self(&self, name: &str) -> bool;
       pub fn get_self_kind(&self) -> Option<SelfKind>;
       pub fn enter_scope(&mut self);
       pub fn exit_scope(&mut self);
   }
   ```

3. **Supporting Types**:
   ```rust
   #[derive(Debug, Clone)]
   pub struct ParameterInfo {
       pub name: String,
       pub is_mut: bool,
       pub is_self: bool,
       pub self_kind: Option<SelfKind>,
   }

   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum SelfKind {
       Ref,        // &self
       MutRef,     // &mut self
       Owned,      // self
       MutOwned,   // mut self
   }
   ```

4. **Module Registration**:
   - Add `pub mod scope_tracker;` to `src/analyzers/mod.rs`

### Non-Functional Requirements

- **Standalone Module**: Can be tested independently
- **Comprehensive Unit Tests**: Cover all methods and edge cases
- **Clean API**: Simple, focused interface
- **Performance**: O(1) lookups using HashSet/HashMap

## Acceptance Criteria

- [x] `src/analyzers/scope_tracker.rs` created with complete implementation
- [x] Module added to `src/analyzers/mod.rs`
- [x] Unit tests cover:
  - Parameter tracking (regular params and self variants)
  - Local variable tracking
  - Scope depth tracking
  - Query methods (`is_local`, `is_self`, `get_self_kind`)
- [x] `cargo build` succeeds
- [x] `cargo test` passes (including new scope_tracker tests)
- [x] `cargo clippy` passes
- [x] `cargo fmt` applied
- [x] Documentation comments on all public types and methods

## Implementation Details

### Core Implementation

```rust
use std::collections::{HashMap, HashSet};
use syn::FnArg;

#[derive(Debug, Clone)]
pub struct ScopeTracker {
    /// Set of local variable identifiers
    local_vars: HashSet<String>,

    /// Parameters (including mutability and self info)
    params: HashMap<String, ParameterInfo>,

    /// Current scope depth (for nested blocks)
    scope_depth: usize,
}

#[derive(Debug, Clone)]
pub struct ParameterInfo {
    pub name: String,
    pub is_mut: bool,
    pub is_self: bool,
    pub self_kind: Option<SelfKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfKind {
    Ref,           // &self
    MutRef,        // &mut self
    Owned,         // self
    MutOwned,      // mut self
}

impl ScopeTracker {
    pub fn new() -> Self {
        Self {
            local_vars: HashSet::new(),
            params: HashMap::new(),
            scope_depth: 0,
        }
    }

    /// Add parameter to scope
    pub fn add_parameter(&mut self, param: &FnArg) {
        match param {
            FnArg::Receiver(receiver) => {
                let kind = match (&receiver.reference, &receiver.mutability) {
                    (Some(_), Some(_)) => SelfKind::MutRef,  // &mut self
                    (Some(_), None) => SelfKind::Ref,         // &self
                    (None, Some(_)) => SelfKind::MutOwned,    // mut self
                    (None, None) => SelfKind::Owned,          // self
                };

                self.params.insert("self".to_string(), ParameterInfo {
                    name: "self".to_string(),
                    is_mut: receiver.mutability.is_some(),
                    is_self: true,
                    self_kind: Some(kind),
                });
            }
            FnArg::Typed(pat_type) => {
                if let syn::Pat::Ident(ident) = &*pat_type.pat {
                    self.params.insert(ident.ident.to_string(), ParameterInfo {
                        name: ident.ident.to_string(),
                        is_mut: ident.mutability.is_some(),
                        is_self: false,
                        self_kind: None,
                    });
                }
            }
        }
    }

    /// Add local variable to current scope
    pub fn add_local_var(&mut self, name: String) {
        self.local_vars.insert(name);
    }

    /// Check if identifier is a local variable or owned parameter
    pub fn is_local(&self, name: &str) -> bool {
        self.local_vars.contains(name) ||
        self.params.get(name).map_or(false, |p| !p.is_self)
    }

    /// Check if identifier is self (any kind)
    pub fn is_self(&self, name: &str) -> bool {
        self.params.get(name).map_or(false, |p| p.is_self)
    }

    /// Get the kind of self parameter, if present
    pub fn get_self_kind(&self) -> Option<SelfKind> {
        self.params.get("self").and_then(|p| p.self_kind)
    }

    /// Enter nested scope (e.g., if block, loop)
    pub fn enter_scope(&mut self) {
        self.scope_depth += 1;
    }

    /// Exit scope
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
```

### Unit Tests

```rust
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
```

## Testing Strategy

- **Unit Tests**: Cover all public methods and edge cases
- **Self Variants**: Test all four self kinds
- **Parameters**: Test regular and mut parameters
- **Local Variables**: Test variable tracking
- **Scope Depth**: Test enter/exit scope

## Documentation Requirements

Add module-level documentation:

```rust
//! Scope tracking for purity analysis.
//!
//! This module provides utilities for tracking variables in scope during AST traversal.
//! It distinguishes between:
//! - Function parameters (including self variants)
//! - Local variables
//! - Different kinds of self references
//!
//! Used by `PurityDetector` to classify mutations as local vs external.
```

## Estimated Effort

**Time**: 1-2 hours
**Complexity**: Low-Medium
**Risk**: Low (standalone module, comprehensive tests)

## Next Steps

After this spec is implemented:
- **Spec 157c**: Integrate scope tracking into PurityDetector
