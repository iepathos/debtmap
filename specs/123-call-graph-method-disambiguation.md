---
number: 123
title: Call Graph Method Name Disambiguation
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-10-21
---

# Specification 123: Call Graph Method Name Disambiguation

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

Debtmap's call graph builder currently suffers from a **method name collision bug** that confuses:
- **Static method calls**: `ContextMatcher::any()` (associated function)
- **Instance method calls**: `iterator.any(|x| ...)` (trait method from `std::iter::Iterator`)

This causes severe false positives in caller analysis, reporting **17 callers** when only **1 actual caller** exists.

### Real-World Impact

**Observed Issue** (debtmap v0.2.9):
```bash
$ debtmap analyze . --lcov target/coverage/lcov.info

#2 SCORE: 17.0 [UNTESTED] [CRITICAL]
├─ LOCATION: ./src/context/rules.rs:52 ContextMatcher::any()
├─ CALLERS: has_test_attribute, AsyncioPatternDetector::detect_gather_without_exception_handling,
│           GraphBuilder::add_impl_method, ... (14 more)
└─ COVERAGE: 0% (line 52 uncovered)
```

**Reality**:
```rust
// ONLY 1 ACTUAL CALLER:
fn parse_config_rule(...) {
    let matcher = ContextMatcher::any();  // Line 221 - ONLY call site
}

// 17 BOGUS "CALLERS" (calling Iterator::any()):
fn has_test_attribute(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| ...)  // Different function entirely!
}

fn detect_gather_without_exception_handling(...) {
    calls.iter().any(|call| ...)  // Not ContextMatcher::any()!
}
```

### Root Cause

**File**: `src/analyzers/call_graph/mod.rs:179-201`

```rust
fn construct_method_name(&mut self, receiver: &Expr, method: &syn::Ident) -> (String, bool) {
    let method_name = method.to_string();  // ← Just "any", loses context!

    let receiver_type = if CallResolver::is_self_receiver(receiver) {
        self.current_impl_type.clone()
    } else {
        self.type_tracker.resolve_expr_type(receiver).map(|t| t.type_name)
    };

    // Constructs qualified name (e.g., "ContextMatcher::any")
    let full_name = CallResolver::construct_method_name(
        receiver_type,
        &method_name,
        &self.current_impl_type,
    );

    (full_name, same_file_hint)
}
```

**Problem**: When `receiver_type` resolution fails (common for iterators, generic types, trait methods), it falls back to just the method name `"any"`, which then matches **all functions named `any`** regardless of type.

**File**: `src/analyzers/call_graph/call_resolution.rs:244-268`

```rust
pub fn is_function_match(
    func: &FunctionId,
    normalized_name: &str,
    original_name: &str,
) -> bool {
    // 1. Exact match
    if func_name == normalized_name { return true; }

    // 2. Qualified match
    if func_name.ends_with(&format!("::{}", normalized_name)) { return true; }

    // 3. Base name match  ← THIS CAUSES THE BUG
    Self::is_base_name_match(func_name, normalized_name)
}

fn is_base_name_match(func_name: &str, search_name: &str) -> bool {
    func_name.split("::").last() == Some(search_name)
    // "ContextMatcher::any".split("::").last() == "any" ✅
    // "Iterator::any".split("::").last() == "any" ✅  ← FALSE POSITIVE!
}
```

This "base name match" strategy incorrectly assumes all methods with the same name are the same function.

## Objective

Fix call graph construction to **accurately distinguish** between:
1. **Static/associated function calls**: `Type::method()`
2. **Instance method calls**: `receiver.method()`
3. **Trait method calls**: `trait_object.method()` or methods from `std` traits

Eliminate false positives in caller identification, ensuring reported callers are actual call sites.

## Requirements

### Functional Requirements

**FR1: Call Site Type Detection**
- Distinguish between `Expr::Call` (static) and `Expr::MethodCall` (instance)
- Track whether call uses `::` (associated function) or `.` (method call)
- Store call site type in `UnresolvedCall` for disambiguation

**FR2: Receiver Type Resolution**
- Improve type inference for method call receivers
- Identify trait methods from standard library (e.g., `Iterator::any`, `Option::map`)
- Fall back to conservative matching when type is unknown (prefer false negatives over false positives)

**FR3: Conservative Matching for Ambiguous Cases**
- When receiver type cannot be determined, DO NOT match by base name only
- Require explicit type qualification for matches (e.g., `ContextMatcher::any`)
- Only match unqualified calls to unqualified definitions (same-file heuristic)

**FR4: Standard Library Method Exclusion**
- Identify and exclude calls to `std::iter::Iterator` methods (`.any()`, `.map()`, `.filter()`, etc.)
- Exclude calls to `std::option::Option` methods
- Exclude calls to other common `std` trait methods
- Make exclusion list configurable

**FR5: Call Type Attribution**
- Store call type metadata: `Static`, `Instance`, `TraitMethod`, `StdLibMethod`
- Report call type in output for debugging
- Use call type in resolution to avoid cross-type matches

### Non-Functional Requirements

**NFR1: Accuracy**
- **Target**: <5% false positive rate in caller identification
- **Current**: ~94% false positive rate (17 reported, only 1 real)
- **Validation**: Regression test on `ContextMatcher::any()` must show exactly 1 caller

**NFR2: Performance**
- Type resolution adds <5% overhead to call graph construction
- Maintain O(1) lookup in `CallResolver::function_index`
- Cache resolved receiver types within a file

**NFR3: Backward Compatibility**
- Existing call graph API remains unchanged
- Existing tests continue to pass (may need adjustment for accuracy)
- No breaking changes to `FunctionCall`, `FunctionId`, or `CallGraph` types

**NFR4: Debuggability**
- Log ambiguous call resolutions at DEBUG level
- Include call site info (file, line, expression type) in warnings
- Provide clear error messages when type resolution fails

## Acceptance Criteria

- [ ] `ContextMatcher::any()` reports exactly **1 caller** (`parse_config_rule()`)
- [ ] `Iterator::any()` calls (17 instances) are **not** attributed to `ContextMatcher::any()`
- [ ] Regression test validates no false positives for common method names (`any`, `map`, `filter`, `new`)
- [ ] Call graph distinguishes static calls (`Type::func()`) from instance calls (`obj.method()`)
- [ ] Standard library trait methods excluded from project function call graph
- [ ] Call resolution logs ambiguous matches at DEBUG level
- [ ] Performance overhead <5% on large codebases (>100k LOC)
- [ ] All existing call graph tests pass with updated assertions
- [ ] Integration test with real-world Rust project (e.g., debtmap itself) shows improved caller accuracy
- [ ] Documentation explains call type disambiguation and exclusion rules

## Technical Details

### Implementation Approach

**Phase 1: Call Type Categorization**

Extend `UnresolvedCall` to include call site type:

```rust
// File: src/analyzers/call_graph/call_resolution.rs

#[derive(Debug, Clone, PartialEq)]
pub enum CallSiteType {
    /// Static/associated function: Type::function()
    Static,

    /// Instance method call: receiver.method()
    Instance { receiver_type: Option<String> },

    /// Trait method call (known trait): receiver.trait_method()
    TraitMethod { trait_name: String, receiver_type: Option<String> },

    /// Call through function pointer or closure
    Indirect,
}

#[derive(Debug, Clone)]
pub struct UnresolvedCall {
    pub caller: FunctionId,
    pub callee_name: String,
    pub call_type: CallType,
    pub call_site_type: CallSiteType,  // NEW
    pub same_file_hint: bool,
}
```

**Phase 2: Call Site Detection**

Update `CallGraphExtractor` to detect call site type:

```rust
// File: src/analyzers/call_graph/mod.rs

impl CallGraphExtractor {
    fn visit_expr(&mut self, expr: &Expr) {
        match expr {
            // Static call: function() or Type::function()
            Expr::Call(call_expr) => {
                let call_site_type = self.classify_call_expr(&call_expr.func);
                self.handle_call_expr_with_type(&call_expr.func, &call_expr.args, call_site_type);
            }

            // Instance method: receiver.method()
            Expr::MethodCall(method_call) => {
                let call_site_type = self.classify_method_call(
                    &method_call.receiver,
                    &method_call.method,
                );
                self.handle_method_call_with_type(
                    &method_call.receiver,
                    &method_call.method,
                    &method_call.args,
                    call_site_type,
                );
            }

            // ... other cases
        }
    }

    fn classify_call_expr(&self, func: &Expr) -> CallSiteType {
        match func {
            // Type::function() or module::function()
            Expr::Path(path) => {
                if path.path.segments.len() > 1 {
                    CallSiteType::Static
                } else {
                    // Unqualified call - could be local or imported
                    CallSiteType::Static
                }
            }

            // function pointer call
            _ => CallSiteType::Indirect,
        }
    }

    fn classify_method_call(&mut self, receiver: &Expr, method: &syn::Ident) -> CallSiteType {
        let method_name = method.to_string();

        // Try to resolve receiver type
        let receiver_type = if CallResolver::is_self_receiver(receiver) {
            self.current_impl_type.clone()
        } else {
            self.type_tracker.resolve_expr_type(receiver).map(|t| t.type_name)
        };

        // Check if this is a known std trait method
        if Self::is_std_trait_method(&method_name) {
            return CallSiteType::TraitMethod {
                trait_name: Self::infer_trait_name(&method_name),
                receiver_type,
            };
        }

        CallSiteType::Instance { receiver_type }
    }

    fn is_std_trait_method(method_name: &str) -> bool {
        matches!(
            method_name,
            // Iterator methods
            "any" | "all" | "map" | "filter" | "fold" | "collect" |
            "find" | "position" | "enumerate" | "zip" | "chain" |

            // Option/Result methods
            "unwrap" | "expect" | "unwrap_or" | "unwrap_or_else" |
            "map" | "and_then" | "or_else" |

            // Common trait methods
            "clone" | "to_string" | "into" | "from"
        )
    }

    fn infer_trait_name(method_name: &str) -> String {
        match method_name {
            "any" | "all" | "map" | "filter" | "fold" | "collect" |
            "find" | "position" | "enumerate" | "zip" | "chain" => "Iterator".to_string(),

            "unwrap" | "expect" | "unwrap_or" | "map" | "and_then" => "Option".to_string(),

            "clone" => "Clone".to_string(),
            "to_string" => "ToString".to_string(),

            _ => "Unknown".to_string(),
        }
    }
}
```

**Phase 3: Disambiguation in Resolution**

Update `CallResolver::is_function_match` to consider call site type:

```rust
// File: src/analyzers/call_graph/call_resolution.rs

impl<'a> CallResolver<'a> {
    pub fn resolve_call(&self, call: &UnresolvedCall) -> Option<FunctionId> {
        // Filter candidates based on call site type
        let candidates = match &call.call_site_type {
            CallSiteType::Static => {
                // Static calls: match exactly or by qualified name
                self.resolve_static_call(&call.callee_name)
            }

            CallSiteType::Instance { receiver_type: Some(recv_type) } => {
                // Instance call with known receiver: match Type::method
                self.resolve_instance_call(&call.callee_name, recv_type)
            }

            CallSiteType::Instance { receiver_type: None } => {
                // Instance call with unknown receiver: be conservative
                // Only match if same file or explicit qualification
                if call.same_file_hint {
                    self.resolve_same_file_call(&call.callee_name)
                } else {
                    // Cannot safely resolve - skip to avoid false positives
                    return None;
                }
            }

            CallSiteType::TraitMethod { .. } => {
                // Trait method call - exclude std lib traits
                if Self::is_std_trait_method(&call.callee_name) {
                    return None;  // Don't match std library methods
                }

                // For project-defined traits, resolve like instance methods
                self.resolve_trait_method_call(call)
            }

            CallSiteType::Indirect => {
                // Indirect call - try to resolve but be conservative
                self.resolve_indirect_call(&call.callee_name)
            }
        };

        candidates
    }

    fn resolve_static_call(&self, callee_name: &str) -> Option<FunctionId> {
        // Require exact match or qualified match
        self.function_index.get(callee_name).and_then(|candidates| {
            candidates
                .iter()
                .find(|func| {
                    // Match if function name is exactly the call name
                    // or function is qualified and call is unqualified (same module)
                    Self::is_exact_match(&func.name, callee_name)
                        || Self::is_qualified_match(&func.name, callee_name)
                })
                .cloned()
        })
    }

    fn resolve_instance_call(
        &self,
        method_name: &str,
        receiver_type: &str,
    ) -> Option<FunctionId> {
        // Construct expected function name: Type::method
        let expected_name = format!("{}::{}", receiver_type, method_name);

        // Look up by expected qualified name
        self.function_index
            .get(&expected_name)
            .and_then(|candidates| candidates.first().cloned())
            .or_else(|| {
                // Fallback: search for any Type::method match
                self.function_index.get(method_name).and_then(|candidates| {
                    candidates
                        .iter()
                        .find(|func| func.name.starts_with(&format!("{}::", receiver_type)))
                        .cloned()
                })
            })
    }

    fn resolve_same_file_call(&self, method_name: &str) -> Option<FunctionId> {
        // Only match functions in the same file
        self.function_index.get(method_name).and_then(|candidates| {
            candidates
                .iter()
                .find(|func| func.file == *self.current_file)
                .cloned()
        })
    }

    fn resolve_trait_method_call(&self, call: &UnresolvedCall) -> Option<FunctionId> {
        // For trait methods, require explicit type or same-file hint
        if let CallSiteType::TraitMethod { receiver_type: Some(recv_type), .. } = &call.call_site_type {
            self.resolve_instance_call(&call.callee_name, recv_type)
        } else if call.same_file_hint {
            self.resolve_same_file_call(&call.callee_name)
        } else {
            None
        }
    }

    fn resolve_indirect_call(&self, callee_name: &str) -> Option<FunctionId> {
        // For indirect calls (through pointers), use existing resolution
        // but prefer same-file matches
        self.function_index.get(callee_name).and_then(|candidates| {
            // Prefer same-file match
            candidates
                .iter()
                .find(|func| func.file == *self.current_file)
                .or_else(|| candidates.first())
                .cloned()
        })
    }

    fn is_std_trait_method(method_name: &str) -> bool {
        // Reuse classification logic
        CallGraphExtractor::is_std_trait_method(method_name)
    }
}
```

**Phase 4: Standard Library Exclusion**

Create exclusion list for common std methods:

```rust
// File: src/analyzers/call_graph/stdlib_methods.rs

/// Standard library trait methods that should be excluded from call graph
pub struct StdLibMethodFilter {
    excluded_methods: HashSet<String>,
}

impl StdLibMethodFilter {
    pub fn new() -> Self {
        let mut excluded_methods = HashSet::new();

        // Iterator trait methods
        excluded_methods.extend([
            "any", "all", "map", "filter", "fold", "reduce",
            "collect", "find", "position", "enumerate", "zip",
            "chain", "flat_map", "flatten", "skip", "take",
            "cloned", "copied", "cycle", "rev", "peekable",
        ].iter().map(|s| s.to_string()));

        // Option trait methods
        excluded_methods.extend([
            "unwrap", "expect", "unwrap_or", "unwrap_or_else",
            "map", "and_then", "or_else", "filter", "is_some",
            "is_none", "as_ref", "as_mut",
        ].iter().map(|s| s.to_string()));

        // Result trait methods
        excluded_methods.extend([
            "unwrap", "expect", "unwrap_or", "unwrap_or_else",
            "map", "and_then", "or_else", "is_ok", "is_err",
        ].iter().map(|s| s.to_string()));

        // Common trait methods
        excluded_methods.extend([
            "clone", "to_string", "to_owned", "into", "from",
            "default", "eq", "ne", "cmp", "partial_cmp",
        ].iter().map(|s| s.to_string()));

        Self { excluded_methods }
    }

    pub fn is_excluded(&self, method_name: &str) -> bool {
        self.excluded_methods.contains(method_name)
    }

    pub fn should_exclude_call(&self, call: &UnresolvedCall) -> bool {
        match &call.call_site_type {
            CallSiteType::TraitMethod { trait_name, .. } => {
                // Exclude known std traits
                matches!(
                    trait_name.as_str(),
                    "Iterator" | "Option" | "Result" | "Clone" | "ToString" | "Default"
                )
            }
            CallSiteType::Instance { receiver_type: None } => {
                // If receiver type unknown and method is std-like, exclude
                self.is_excluded(&call.callee_name)
            }
            _ => false,
        }
    }
}
```

### Architecture Changes

**Modified Files**:
- `src/analyzers/call_graph/call_resolution.rs` - Add `CallSiteType` enum, update resolution logic
- `src/analyzers/call_graph/mod.rs` - Add call site classification in expression visiting
- `src/analyzers/call_graph/graph_builder.rs` - Update `add_call` to accept `CallSiteType`
- `src/priority/call_graph/mod.rs` - Potentially store call site metadata (optional)

**New Files**:
- `src/analyzers/call_graph/stdlib_methods.rs` - Standard library method exclusion list
- `tests/call_graph_disambiguation_test.rs` - Regression tests for method disambiguation

**Data Structure Changes**:
- `UnresolvedCall`: Add `call_site_type: CallSiteType` field
- `FunctionCall`: Optionally add metadata for debugging (not required for correctness)

### Algorithms

**Call Site Classification Algorithm**:

1. **Input**: AST expression (`Expr::Call` or `Expr::MethodCall`)
2. **Analysis**:
   - For `Expr::Call`: Check path qualification → `Static`
   - For `Expr::MethodCall`: Resolve receiver type
     - If `self` → Use `current_impl_type` → `Instance`
     - If variable → Look up in type tracker → `Instance { receiver_type }`
     - If iterator/option/result → Detect trait → `TraitMethod`
     - If unknown → `Instance { receiver_type: None }`
3. **Exclusion**: If known std trait method → exclude from graph
4. **Output**: `CallSiteType` for disambiguation

**Disambiguation Algorithm**:

1. **Input**: `UnresolvedCall` with `call_site_type`
2. **Filtering**:
   - `Static`: Match by exact name or qualified name
   - `Instance { Some(type) }`: Match `Type::method`
   - `Instance { None }`: Only match same-file or skip
   - `TraitMethod`: Check exclusion list, then resolve like instance
3. **Selection**: Use existing best-candidate logic on filtered set
4. **Output**: `Option<FunctionId>` (None if ambiguous or excluded)

## Dependencies

**Prerequisites**:
- Existing call graph infrastructure
- Type resolution system (already present in `type_tracker`)
- AST expression visitor

**Affected Components**:
- Call graph extraction pipeline
- Call resolution logic
- Caller/callee relationship reporting
- Coverage analysis (depends on accurate caller counts)

**External Dependencies**: None (uses existing `syn` AST)

## Testing Strategy

### Unit Tests

**Test Call Site Classification**:
```rust
#[test]
fn test_classify_static_call() {
    let code = "ContextMatcher::any()";
    let expr = parse_expr(code);
    let call_site_type = classify_call_expr(&expr);
    assert_eq!(call_site_type, CallSiteType::Static);
}

#[test]
fn test_classify_iterator_any() {
    let code = "items.iter().any(|x| x > 0)";
    let expr = parse_expr(code);
    let call_site_type = classify_method_call(&receiver, &method);
    assert!(matches!(call_site_type, CallSiteType::TraitMethod { trait_name, .. } if trait_name == "Iterator"));
}

#[test]
fn test_std_trait_method_exclusion() {
    let filter = StdLibMethodFilter::new();
    assert!(filter.is_excluded("any"));
    assert!(filter.is_excluded("map"));
    assert!(!filter.is_excluded("parse_config_rule"));
}
```

**Test Disambiguation Logic**:
```rust
#[test]
fn test_resolve_static_call_exact_match() {
    let mut graph = CallGraph::new();
    graph.add_function(func_id("ContextMatcher::any", "rules.rs", 52), ...);

    let call = UnresolvedCall {
        callee_name: "ContextMatcher::any".to_string(),
        call_site_type: CallSiteType::Static,
        ...
    };

    let resolver = CallResolver::new(&graph, &PathBuf::from("rules.rs"));
    let resolved = resolver.resolve_call(&call);

    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().name, "ContextMatcher::any");
}

#[test]
fn test_iterator_any_not_matched_to_context_matcher() {
    let mut graph = CallGraph::new();
    graph.add_function(func_id("ContextMatcher::any", "rules.rs", 52), ...);

    let call = UnresolvedCall {
        callee_name: "any".to_string(),
        call_site_type: CallSiteType::TraitMethod {
            trait_name: "Iterator".to_string(),
            receiver_type: None,
        },
        ...
    };

    let resolver = CallResolver::new(&graph, &PathBuf::from("other.rs"));
    let resolved = resolver.resolve_call(&call);

    assert!(resolved.is_none(), "Iterator::any should not match ContextMatcher::any");
}
```

### Integration Tests

**Regression Test for ContextMatcher::any()**:
```rust
#[test]
fn test_context_matcher_any_caller_accuracy() {
    // Parse debtmap's own codebase
    let project_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let call_graph = build_call_graph(&project_path).unwrap();

    // Find ContextMatcher::any function
    let any_func = call_graph
        .get_all_functions()
        .find(|f| f.name == "ContextMatcher::any" && f.file.ends_with("context/rules.rs"))
        .expect("ContextMatcher::any not found");

    // Get callers
    let callers = call_graph.get_callers(&any_func);

    // Should have exactly 1 caller: parse_config_rule
    assert_eq!(
        callers.len(),
        1,
        "ContextMatcher::any should have exactly 1 caller, found: {:?}",
        callers
    );

    assert_eq!(callers[0].name, "parse_config_rule");
    assert!(callers[0].file.ends_with("context/rules.rs"));
}

#[test]
fn test_iterator_any_excluded() {
    let project_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let call_graph = build_call_graph(&project_path).unwrap();

    // Find a function that uses Iterator::any
    let has_test_attr = call_graph
        .get_all_functions()
        .find(|f| f.name == "has_test_attribute")
        .expect("has_test_attribute not found");

    // Get callees
    let callees = call_graph.get_callees(&has_test_attr);

    // Should NOT include "any" or "ContextMatcher::any"
    assert!(
        !callees.iter().any(|c| c.name == "any" || c.name.ends_with("::any")),
        "has_test_attribute should not call any user function named 'any'"
    );
}
```

**Performance Test**:
```rust
#[test]
fn test_disambiguation_performance() {
    let large_graph = create_large_call_graph(100_000); // 100k functions

    let start = Instant::now();

    // Resolve 10k method calls
    for i in 0..10_000 {
        let call = UnresolvedCall {
            callee_name: format!("method_{}", i % 100),
            call_site_type: CallSiteType::Instance { receiver_type: Some(format!("Type_{}", i % 10)) },
            ...
        };

        let resolver = CallResolver::new(&large_graph, &PathBuf::from("test.rs"));
        let _ = resolver.resolve_call(&call);
    }

    let duration = start.elapsed();

    // Should complete in <500ms (5% overhead on ~10s baseline)
    assert!(
        duration.as_millis() < 500,
        "Disambiguation took too long: {:?}",
        duration
    );
}
```

## Documentation Requirements

### Code Documentation

**Module Documentation**:
```rust
//! # Call Graph Method Disambiguation
//!
//! This module implements accurate method call resolution by distinguishing:
//! - Static calls: `Type::function()`
//! - Instance calls: `receiver.method()`
//! - Trait calls: `iterator.any(|x| ...)`
//!
//! ## Problem
//!
//! Previous implementation used base name matching, causing false positives:
//! - `ContextMatcher::any()` matched with `Iterator::any()`
//! - Reported 17 callers when only 1 actual caller existed
//!
//! ## Solution
//!
//! 1. **Call Site Classification**: Determine whether call is static, instance, or trait method
//! 2. **Type-Aware Resolution**: Match calls to functions using receiver type information
//! 3. **Standard Library Exclusion**: Filter out calls to `std` trait methods
//! 4. **Conservative Fallback**: Prefer false negatives over false positives when type is unknown
//!
//! ## Example
//!
//! ```rust
//! // Static call - matches ContextMatcher::any()
//! let matcher = ContextMatcher::any();
//!
//! // Trait method call - excluded from graph (std::iter::Iterator)
//! let has_any = items.iter().any(|x| x > 0);
//!
//! // Instance call - matches Type::method() if Type is known
//! let result = object.method();
//! ```
```

### User Documentation

**Update**: `book/src/call-graph-analysis.md`

```markdown
## Call Graph Accuracy

### Method Name Disambiguation

Debtmap accurately distinguishes between different functions with the same name:

**Static vs Instance Calls**:
```rust
// Static/associated function call
ContextMatcher::any()  // ✅ Tracked as call to ContextMatcher::any

// Instance method call on iterator
items.iter().any(|x| x > 0)  // ✅ Recognized as std::iter::Iterator::any
                              // ❌ NOT tracked (std library method)
```

**Why This Matters**:
- **Accurate caller counts**: Functions show only real call sites
- **Better prioritization**: Unused code correctly identified
- **Reduced false positives**: Helper functions not confused with std methods

### Standard Library Method Exclusion

Debtmap excludes common standard library trait methods from the call graph:
- **Iterator**: `any`, `map`, `filter`, `fold`, `collect`, etc.
- **Option/Result**: `unwrap`, `map`, `and_then`, etc.
- **Clone/ToString**: `clone`, `to_string`, etc.

**Rationale**: These methods are:
1. Well-tested in the standard library
2. Not part of your project's complexity
3. Called ubiquitously (would pollute the graph)

**Configuration**:
```toml
[call_graph]
exclude_std_methods = true  # Default: true
additional_exclusions = ["custom_trait_method"]
```
```

## Implementation Notes

### Performance Optimization

**Type Resolution Caching**:
```rust
pub struct TypeResolutionCache {
    cache: HashMap<ExprId, Option<String>>,  // Cache resolved types
}

impl TypeResolutionCache {
    pub fn get_or_resolve(
        &mut self,
        expr_id: ExprId,
        resolver: impl FnOnce() -> Option<String>,
    ) -> Option<String> {
        self.cache.entry(expr_id).or_insert_with(resolver).clone()
    }
}
```

**Index Optimization**:
- Maintain separate indices for static vs instance methods
- Pre-filter std methods during index construction (not at query time)
- Use trie structure for qualified name lookups (e.g., `Type::*`)

### Edge Cases

**Ambiguous Receiver Types**:
```rust
// Cannot determine if receiver is Iterator or custom type
let result = get_something().any(|x| x > 0);

// Strategy: Assume std trait method, exclude from graph
// Better to miss a real call than create false positive
```

**Generic Methods**:
```rust
fn process<T: Iterator>(iter: T) {
    iter.any(|x| ...)  // T is Iterator, so this is trait method
}

// Solution: Type bounds analysis to detect trait constraints
```

**Macro-Generated Calls**:
```rust
macro_rules! call_any {
    ($type:ty) => {
        <$type>::any()
    }
}

// Solution: Expand macros before call graph extraction (already done)
```

**Method Call on Type Alias**:
```rust
type MyMatcher = ContextMatcher;
MyMatcher::any()  // Should match ContextMatcher::any

// Solution: Type alias resolution in type_tracker
```

## Migration and Compatibility

### Breaking Changes

**None** - This is a pure bug fix that improves accuracy.

### Behavior Changes

**Before (v0.2.9)**:
```
ContextMatcher::any() - 17 callers (16 false positives)
```

**After (v0.3.0)**:
```
ContextMatcher::any() - 1 caller (accurate)
```

**Impact on Existing Projects**:
- Caller counts will **decrease** (false positives removed)
- Risk scores may **decrease** (fewer phantom dependencies)
- Functions previously flagged as "widely used" may now show as "rarely used"

### Rollback Plan

If accuracy issues arise:
1. Add flag: `--legacy-call-resolution`
2. Fallback to base name matching (old behavior)
3. Log differences between old and new resolution

```rust
if config.use_legacy_call_resolution {
    return self.resolve_call_legacy(call);
}
```

## Success Metrics

### Quantitative Metrics

- **False Positive Reduction**: From 94% (16/17) to <5% (<1/17)
- **Caller Accuracy**: 100% for `ContextMatcher::any()` (1 reported, 1 actual)
- **Performance Overhead**: <5% increase in call graph construction time
- **Test Coverage**: >90% coverage for disambiguation logic

### Qualitative Metrics

- **User Trust**: Accurate caller counts improve confidence in analysis
- **Reduced Noise**: Fewer bogus dependencies in output
- **Better Prioritization**: Functions correctly identified as unused or rarely used

### Validation Methodology

1. **Regression Test**: `ContextMatcher::any()` shows exactly 1 caller
2. **Manual Audit**: Review 100 random method calls, verify caller accuracy
3. **Comparison Test**: Run old vs new call graph on debtmap itself, compare results
4. **Performance Benchmark**: Measure overhead on large projects (>100k LOC)

## Future Enhancements

### Phase 2: Full Type Inference

Implement complete type inference for method receivers:
- Track variable types through assignments
- Resolve generic type parameters
- Handle complex expressions (chained calls, closures)

### Phase 3: Cross-Crate Call Graph

Extend call graph to track calls into dependencies:
- Parse dependency source code or use rustdoc JSON
- Build unified call graph across workspace
- Track public API usage patterns

### Phase 4: Trait Resolution

Improve trait method tracking:
- Identify which trait implementation is called
- Track trait bounds and where clause constraints
- Build trait implementation graph

## Related Issues

- **Issue**: ContextMatcher::any() false positives (#17 callers reported)
- **Root Cause**: Base name matching without type disambiguation
- **Related Specs**:
  - Spec 120: Indirect Coverage Detection (depends on accurate caller counts)
  - Spec 121: Coverage Gap Calculation (uses call graph for prioritization)
