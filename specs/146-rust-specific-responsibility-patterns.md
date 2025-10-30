---
number: 146
title: Rust-Specific Responsibility Patterns
category: optimization
priority: medium
status: ready-for-implementation
dependencies: [141, 142]
created: 2025-10-27
updated: 2025-10-30
---

# Specification 146: Rust-Specific Responsibility Patterns

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 141 (I/O Detection), Spec 142 (Call Graph)

## Context

Rust has unique language patterns that indicate specific responsibilities, which generic multi-signal analysis may miss:

- **Trait Implementations**: Functions implementing standard traits (Display, From, Into, etc.) have specific responsibilities
- **Error Handling Patterns**: Functions using `?` operator, `Result`, custom error types
- **Async/Concurrency**: `async fn`, `tokio::spawn`, channels, mutexes
- **Memory Management**: `Drop` impls, `Box::leak`, unsafe blocks, raw pointers
- **Builder Patterns**: Chainable methods returning `Self`, `new()` constructors
- **Type Conversions**: `From`/`Into` impls, `as` casts, `try_from`
- **Iterator Implementations**: Custom iterators, `IntoIterator`, adapters
- **Macro Expansion**: Declarative and procedural macro patterns

While Spec 144 (Framework Patterns) handles framework-specific idioms, this specification addresses **language-level Rust patterns** that are universally applicable regardless of framework.

## Objective

Detect Rust-specific language patterns to enhance responsibility classification for Rust code. Recognize trait implementations, async patterns, error handling, and memory management to provide more accurate and Rust-idiomatic classifications.

## Prerequisites and Architecture Alignment

### AST Audit Findings (2025-10-29)

The following capabilities are **verified to exist** in the current codebase:

✅ **Async Detection**: Available via `syn::ItemFn::sig.asyncness` field
- Location: `src/analyzers/signature_extractor.rs:53,91`
- Usage: `item_fn.sig.asyncness.is_some()`

✅ **Call Graph Extraction**: Fully implemented module
- Location: `src/analyzers/rust_call_graph.rs` + `src/analyzers/call_graph/`
- API: `extract_call_graph(file: &syn::File, path: &Path) -> CallGraph`
- Features: Multi-file support, trait resolution, import-aware resolution

✅ **Function Body Access**: Via `quote::ToTokens`
- Location: `src/analyzers/rust.rs:567,985`
- Usage: `quote::quote!(#block).to_string()`

✅ **Impl Block Tracking**: Context maintained during AST traversal
- Location: `src/analyzers/rust.rs:824-851`
- Fields: `current_impl_type`, `current_impl_is_trait`, tracked via `syn::visit::Visit`

⚠️ **Trait Implementation Tracker**: Exists but needs integration
- Location: `src/analyzers/trait_implementation_tracker.rs`
- Status: Module present, needs hook into pattern detection

### Data Structures

**Primary Analysis Context** (to be created):
```rust
pub struct RustFunctionContext<'a> {
    pub item_fn: &'a syn::ItemFn,              // Parsed function AST
    pub metrics: Option<&'a FunctionMetrics>,  // Computed metrics (if available)
    pub impl_context: Option<ImplContext>,     // Parent impl block info
    pub file_path: &'a Path,                   // For error reporting
}

pub struct ImplContext {
    pub impl_type: String,           // Type being implemented (e.g., "MyStruct")
    pub is_trait_impl: bool,         // Whether this is a trait impl
    pub trait_name: Option<String>,  // Trait name if trait impl (e.g., "Display")
}
```

**Existing Structure** (to be extended):
- `FunctionMetrics` at `src/core/mod.rs:42-64` stores computed metrics
- `syn::ItemFn` provides full AST access
- `CallGraph` at `src/priority/call_graph/types.rs` provides call relationships

**Memory Footprint Optimization**:
```rust
// Extension to FunctionMetrics (src/core/mod.rs)
// Use language-specific extension to avoid memory overhead for non-Rust files

#[derive(Debug, Clone)]
pub struct FunctionMetrics {
    // ... existing fields ...
    pub language_specific: Option<LanguageSpecificData>,  // NEW
}

#[derive(Debug, Clone)]
pub enum LanguageSpecificData {
    Rust(RustPatternResult),
    // Future: Python(PythonPatternResult), JavaScript(JSPatternResult)
}
```

This approach:
- Avoids allocating Rust-specific data for Python/JavaScript files
- Provides extensibility for future language-specific analyzers
- Maintains memory efficiency (Option wrapper + enum discriminant = ~24 bytes overhead)

## Requirements

### Functional Requirements

**Trait Implementation Detection**:
- Detect standard trait implementations (Display, Debug, From, Into, Default, etc.)
- Classify functions by trait purpose (formatting, conversion, construction)
- Track custom trait implementations
- Integrate with existing `TraitImplementationTracker` module

**Async/Concurrency Pattern Detection**:
- Detect `async fn` via `sig.asyncness.is_some()`
- Identify tokio patterns (spawn, channels, select!, join!) via AST traversal
- Detect mutex/rwlock usage via AST path analysis (not string matching)
- Track thread spawning and synchronization primitives

**Error Handling Pattern Detection**:
- Count `?` operator usage via AST traversal (not string counting)
- Detect custom error type definitions via type analysis
- Track error conversion patterns (`From<E>` for error types)
- Identify comprehensive anti-patterns via AST expression matching:
  - `unwrap()` - panic on None/Err
  - `expect()` - panic with message
  - `panic!()` - explicit panic macro
  - `unreachable!()` - assertion that code is unreachable
  - `unwrap_or_default()` - silent error suppression
  - `ok().unwrap()` - chained anti-pattern
  - `expect_err()` - inverse unwrap for Err values

**Memory Management Pattern Detection**:
- Detect unsafe blocks via `syn::Block` analysis
- Identify custom `Drop` implementations via trait tracker
- Track reference counting patterns (Rc, Arc) via type path analysis
- Detect memory leaks (`Box::leak`, `mem::forget`) via function call detection

**Builder Pattern Detection** (Phase 1 - High ROI):
- Identify builder structs (methods returning `Self`)
- Detect constructor patterns (`new`, `with_*`, `from_*`)
- Track builder finalization methods (`build`, `finalize`)
- Use existing signature extraction infrastructure
- Rationale for Phase 1 inclusion: Low implementation complexity, high value for debtmap's own codebase

**Type Conversion Detection**:
- Detect `From`/`Into` trait implementations
- Identify `as` cast expressions via AST
- Track `TryFrom`/`TryInto` for fallible conversions
- Detect newtype wrappers

### Non-Functional Requirements

- **Accuracy**: Correctly identify >90% of Rust-specific patterns
- **Performance**: Pattern detection adds <5% overhead (empirical validation required)
  - Baseline: Measure current analysis speed on 10k+ function corpus
  - With patterns: Re-measure with all detectors enabled
  - Memory: Measure heap allocation increase (target <10% increase)
  - Per-function: Pattern detection should complete in <100μs per function
  - Validation: Use criterion benchmarks with statistical significance testing
- **Memory Footprint**: Language-specific data adds <10% memory overhead
  - Measured on debtmap's own codebase (~500 functions)
  - Use `LanguageSpecificData` enum to avoid overhead for non-Rust code
  - Benchmark with `dhat` or similar memory profiling tools
- **Rust Version Support**: Support stable Rust patterns (no nightly-only)
- **Idiomatic**: Classifications align with Rust community terminology
- **AST-Based**: Use `syn::visit::Visit` patterns, not string matching

## Acceptance Criteria

- [ ] Standard trait implementations are correctly classified (Display, From, Into, etc.)
- [ ] Async functions detected via `sig.asyncness.is_some()`
- [ ] Comprehensive error anti-patterns detected (unwrap, expect, panic, unreachable, ok().unwrap())
- [ ] Error handling patterns detected via AST traversal (not string matching)
- [ ] Unsafe blocks and memory management patterns flagged via AST analysis
- [ ] Builder patterns identified via signature analysis (Phase 1 implementation)
- [ ] Type conversion implementations detected via trait tracker integration
- [ ] Custom Drop implementations classified as "Resource Cleanup"
- [ ] Iterator implementations classified as "Iteration Logic"
- [ ] Performance overhead <5% validated with criterion benchmarks (baseline vs with-patterns)
- [ ] Memory overhead <10% validated with memory profiler (dhat or similar)
- [ ] Per-function detection completes in <100μs (99th percentile)
- [ ] Test suite includes debtmap's own Rust code examples
- [ ] No false positives from comments or string literals (AST-based detection)
- [ ] `LanguageSpecificData` enum prevents memory waste on non-Rust files

## Technical Details

### Implementation Approach

**Phase 1: Foundation and Context**

Create analysis context structure that unifies existing data:

```rust
// File: src/analysis/rust_patterns/context.rs

use std::path::Path;
use syn::ItemFn;
use crate::core::FunctionMetrics;

/// Context for Rust pattern detection combining AST and metadata
pub struct RustFunctionContext<'a> {
    /// Parsed function AST from syn
    pub item_fn: &'a ItemFn,

    /// Computed metrics (may be None during initial analysis)
    pub metrics: Option<&'a FunctionMetrics>,

    /// Parent impl block context if this is a method
    pub impl_context: Option<ImplContext>,

    /// File path for error reporting
    pub file_path: &'a Path,
}

#[derive(Clone, Debug)]
pub struct ImplContext {
    pub impl_type: String,
    pub is_trait_impl: bool,
    pub trait_name: Option<String>,
}

impl<'a> RustFunctionContext<'a> {
    pub fn from_item_fn(item_fn: &'a ItemFn, file_path: &'a Path) -> Self {
        Self {
            item_fn,
            metrics: None,
            impl_context: None,
            file_path,
        }
    }

    pub fn with_impl_context(mut self, ctx: ImplContext) -> Self {
        self.impl_context = Some(ctx);
        self
    }

    /// Check if function is async (leverages existing capability)
    pub fn is_async(&self) -> bool {
        self.item_fn.sig.asyncness.is_some()
    }

    /// Get function body for AST traversal
    pub fn body(&self) -> &syn::Block {
        &self.item_fn.block
    }

    /// Get function body as string for reporting (NOT for pattern matching)
    pub fn body_text(&self) -> String {
        use quote::ToTokens;
        quote::quote!(#(self.item_fn.block)).to_string()
    }

    /// Check if this is a trait method implementation
    pub fn is_trait_impl(&self) -> bool {
        self.impl_context.as_ref()
            .map(|ctx| ctx.is_trait_impl)
            .unwrap_or(false)
    }

    /// Get trait name if this is a trait implementation
    pub fn trait_name(&self) -> Option<&str> {
        self.impl_context.as_ref()
            .and_then(|ctx| ctx.trait_name.as_deref())
    }
}
```

**Phase 2: Trait Implementation Detection (AST-Based)**

Leverage existing `TraitImplementationTracker` module:

```rust
// File: src/analysis/rust_patterns/trait_detector.rs

use syn::{ItemImpl, Path};
use crate::analyzers::trait_implementation_tracker::TraitImplementationTracker;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StandardTrait {
    // Formatting
    Display,
    Debug,

    // Conversions
    From,
    Into,
    TryFrom,
    TryInto,
    AsRef,
    AsMut,

    // Construction
    Default,
    Clone,

    // Resource Management
    Drop,

    // Iteration
    Iterator,
    IntoIterator,

    // Operators
    Add, Sub, Mul, Div,
    Deref, DerefMut,

    // Comparison
    PartialEq, Eq,
    PartialOrd, Ord,

    // Hashing
    Hash,

    // Serialization (common crates)
    Serialize,
    Deserialize,
}

#[derive(Debug, Clone)]
pub struct TraitImplClassification {
    pub trait_name: String,
    pub standard_trait: Option<StandardTrait>,
    pub category: ResponsibilityCategory,
    pub confidence: f64,
    pub evidence: String,
}

pub struct RustTraitDetector {
    trait_patterns: HashMap<StandardTrait, ResponsibilityCategory>,
}

impl RustTraitDetector {
    pub fn new() -> Self {
        let mut trait_patterns = HashMap::new();

        trait_patterns.insert(StandardTrait::Display, ResponsibilityCategory::Formatting);
        trait_patterns.insert(StandardTrait::Debug, ResponsibilityCategory::Formatting);

        trait_patterns.insert(StandardTrait::From, ResponsibilityCategory::TypeConversion);
        trait_patterns.insert(StandardTrait::Into, ResponsibilityCategory::TypeConversion);
        trait_patterns.insert(StandardTrait::TryFrom, ResponsibilityCategory::TypeConversion);
        trait_patterns.insert(StandardTrait::TryInto, ResponsibilityCategory::TypeConversion);

        trait_patterns.insert(StandardTrait::Default, ResponsibilityCategory::Construction);
        trait_patterns.insert(StandardTrait::Clone, ResponsibilityCategory::Construction);

        trait_patterns.insert(StandardTrait::Drop, ResponsibilityCategory::ResourceCleanup);

        trait_patterns.insert(StandardTrait::Iterator, ResponsibilityCategory::Iteration);
        trait_patterns.insert(StandardTrait::IntoIterator, ResponsibilityCategory::Iteration);

        // Operators are "Computation"
        for op_trait in [StandardTrait::Add, StandardTrait::Sub,
                         StandardTrait::Mul, StandardTrait::Div] {
            trait_patterns.insert(op_trait, ResponsibilityCategory::Computation);
        }

        RustTraitDetector { trait_patterns }
    }

    /// Detect trait implementation from context
    pub fn detect_trait_impl(
        &self,
        context: &RustFunctionContext,
    ) -> Option<TraitImplClassification> {
        // Check if this function is a trait method
        if !context.is_trait_impl() {
            return None;
        }

        let trait_name = context.trait_name()?;

        // Match against standard traits
        let standard_trait = self.match_standard_trait(trait_name)?;
        let category = self.trait_patterns.get(&standard_trait)?;

        Some(TraitImplClassification {
            trait_name: trait_name.to_string(),
            standard_trait: Some(standard_trait),
            category: *category,
            confidence: 0.95,  // High confidence for trait impls
            evidence: format!("Implements {} trait", trait_name),
        })
    }

    /// Match trait name to standard trait enum
    /// Handles both simple names and qualified paths
    fn match_standard_trait(&self, trait_name: &str) -> Option<StandardTrait> {
        // Extract final segment for matching
        let simple_name = trait_name.split("::").last()?;

        match simple_name {
            "Display" => Some(StandardTrait::Display),
            "Debug" => Some(StandardTrait::Debug),
            "From" => Some(StandardTrait::From),
            "Into" => Some(StandardTrait::Into),
            "TryFrom" => Some(StandardTrait::TryFrom),
            "TryInto" => Some(StandardTrait::TryInto),
            "Default" => Some(StandardTrait::Default),
            "Clone" => Some(StandardTrait::Clone),
            "Drop" => Some(StandardTrait::Drop),
            "Iterator" => Some(StandardTrait::Iterator),
            "IntoIterator" => Some(StandardTrait::IntoIterator),
            "Add" | "Sub" | "Mul" | "Div" => Some(StandardTrait::Add),
            "Serialize" => Some(StandardTrait::Serialize),
            "Deserialize" => Some(StandardTrait::Deserialize),
            _ => None,
        }
    }
}
```

**Phase 3: Async/Concurrency Pattern Detection (AST-Based)**

Use `syn::visit::Visit` for accurate detection:

```rust
// File: src/analysis/rust_patterns/async_detector.rs

use syn::{visit::Visit, Expr, ExprAwait, ExprCall, ExprPath, Path};

#[derive(Debug, Clone)]
pub struct AsyncPattern {
    pub pattern_type: AsyncPatternType,
    pub confidence: f64,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsyncPatternType {
    AsyncFunction,
    TaskSpawning,
    ChannelCommunication,
    MutexUsage,
    SelectMacro,
    JoinMacro,
}

/// AST visitor for detecting concurrency patterns
pub struct ConcurrencyPatternVisitor {
    pub has_mutex: bool,
    pub has_rwlock: bool,
    pub has_channel_send: bool,
    pub has_channel_recv: bool,
    pub spawn_calls: Vec<String>,
    pub await_points: usize,
}

impl ConcurrencyPatternVisitor {
    pub fn new() -> Self {
        Self {
            has_mutex: false,
            has_rwlock: false,
            has_channel_send: false,
            has_channel_recv: false,
            spawn_calls: Vec::new(),
            await_points: 0,
        }
    }
}

impl<'ast> Visit<'ast> for ConcurrencyPatternVisitor {
    fn visit_expr_await(&mut self, await_expr: &'ast ExprAwait) {
        self.await_points += 1;
        syn::visit::visit_expr_await(self, await_expr);
    }

    fn visit_path(&mut self, path: &'ast Path) {
        // Build path string for analysis
        let path_segments: Vec<_> = path.segments.iter()
            .map(|s| s.ident.to_string())
            .collect();
        let path_str = path_segments.join("::");

        // Detect synchronization primitives
        if path_segments.iter().any(|s| s == "Mutex") {
            self.has_mutex = true;
        }
        if path_segments.iter().any(|s| s == "RwLock") {
            self.has_rwlock = true;
        }

        syn::visit::visit_path(self, path);
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        // Detect spawn calls (tokio::spawn, async_std::spawn, etc.)
        if let Expr::Path(ExprPath { path, .. }) = &*call.func {
            let path_str = path.segments.iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");

            if path_str.contains("spawn") {
                self.spawn_calls.push(path_str);
            }
        }

        syn::visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, method: &'ast syn::ExprMethodCall) {
        let method_name = method.method.to_string();

        // Detect channel operations
        if method_name == "send" || method_name == "try_send" {
            self.has_channel_send = true;
        }
        if method_name == "recv" || method_name == "try_recv" {
            self.has_channel_recv = true;
        }

        syn::visit::visit_expr_method_call(self, method);
    }
}

pub struct RustAsyncDetector;

impl RustAsyncDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detect_async_patterns(
        &self,
        context: &RustFunctionContext,
    ) -> Vec<AsyncPattern> {
        let mut patterns = Vec::new();

        // Check if function is async (using verified capability)
        if context.is_async() {
            patterns.push(AsyncPattern {
                pattern_type: AsyncPatternType::AsyncFunction,
                confidence: 1.0,
                evidence: "Function is declared as async".into(),
            });
        }

        // Traverse AST to find concurrency patterns
        let mut visitor = ConcurrencyPatternVisitor::new();
        visitor.visit_block(context.body());

        // Task spawning detected
        if !visitor.spawn_calls.is_empty() {
            patterns.push(AsyncPattern {
                pattern_type: AsyncPatternType::TaskSpawning,
                confidence: 0.9,
                evidence: format!("Spawns async tasks: {}", visitor.spawn_calls.join(", ")),
            });
        }

        // Channel communication
        if visitor.has_channel_send || visitor.has_channel_recv {
            patterns.push(AsyncPattern {
                pattern_type: AsyncPatternType::ChannelCommunication,
                confidence: 0.85,
                evidence: "Uses channel communication".into(),
            });
        }

        // Mutex usage
        if visitor.has_mutex || visitor.has_rwlock {
            patterns.push(AsyncPattern {
                pattern_type: AsyncPatternType::MutexUsage,
                confidence: 0.85,
                evidence: format!(
                    "Uses synchronization: Mutex={}, RwLock={}",
                    visitor.has_mutex, visitor.has_rwlock
                ),
            });
        }

        patterns
    }

    pub fn classify_from_async_patterns(
        &self,
        patterns: &[AsyncPattern],
    ) -> Option<ResponsibilityCategory> {
        if patterns.is_empty() {
            return None;
        }

        // Task spawning = Concurrency Management
        if patterns.iter().any(|p| p.pattern_type == AsyncPatternType::TaskSpawning) {
            return Some(ResponsibilityCategory::ConcurrencyManagement);
        }

        // Async function with channel = Communication
        if patterns.iter().any(|p| p.pattern_type == AsyncPatternType::ChannelCommunication) {
            return Some(ResponsibilityCategory::CommunicationOrchestration);
        }

        // Just async = Asynchronous Operation
        if patterns.iter().any(|p| p.pattern_type == AsyncPatternType::AsyncFunction) {
            return Some(ResponsibilityCategory::AsynchronousOperation);
        }

        None
    }
}
```

**Phase 4: Error Handling Pattern Detection (AST-Based)**

Count `?` operator and detect error types via AST:

```rust
// File: src/analysis/rust_patterns/error_detector.rs

use syn::{visit::Visit, Expr, ExprTry, ReturnType, Type};

#[derive(Debug, Clone)]
pub struct ErrorPattern {
    pub pattern_type: ErrorPatternType,
    pub count: usize,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorPatternType {
    QuestionMarkOperator,
    CustomErrorType,
    ErrorConversion,
    UnwrapUsage,
    PanicUsage,
    ExpectUsage,
    UnreachableUsage,        // NEW: unreachable!() macro
    UnwrapOrDefaultUsage,    // NEW: silent error suppression
    OkUnwrapChain,           // NEW: .ok().unwrap() anti-pattern
    ExpectErrUsage,          // NEW: .expect_err() for inverting Result
}

/// AST visitor for error handling patterns
pub struct ErrorPatternVisitor {
    pub question_mark_count: usize,
    pub unwrap_count: usize,
    pub expect_count: usize,
    pub panic_count: usize,
    pub unreachable_count: usize,           // NEW
    pub unwrap_or_default_count: usize,     // NEW
    pub ok_unwrap_chain_count: usize,       // NEW
    pub expect_err_count: usize,            // NEW
}

impl ErrorPatternVisitor {
    pub fn new() -> Self {
        Self {
            question_mark_count: 0,
            unwrap_count: 0,
            expect_count: 0,
            panic_count: 0,
            unreachable_count: 0,
            unwrap_or_default_count: 0,
            ok_unwrap_chain_count: 0,
            expect_err_count: 0,
        }
    }
}

impl<'ast> Visit<'ast> for ErrorPatternVisitor {
    fn visit_expr_try(&mut self, try_expr: &'ast ExprTry) {
        // The `?` operator
        self.question_mark_count += 1;
        syn::visit::visit_expr_try(self, try_expr);
    }

    fn visit_expr_method_call(&mut self, method: &'ast syn::ExprMethodCall) {
        let method_name = method.method.to_string();

        match method_name.as_str() {
            "unwrap" => {
                // Check if this is part of .ok().unwrap() chain
                if let Expr::MethodCall(inner) = &*method.receiver {
                    if inner.method.to_string() == "ok" {
                        self.ok_unwrap_chain_count += 1;
                    }
                }
                self.unwrap_count += 1;
            }
            "expect" => self.expect_count += 1,
            "expect_err" => self.expect_err_count += 1,
            "unwrap_or_default" => self.unwrap_or_default_count += 1,
            _ => {}
        }

        syn::visit::visit_expr_method_call(self, method);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let macro_name = mac.path.segments.last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();

        match macro_name.as_str() {
            "panic" => self.panic_count += 1,
            "unreachable" => self.unreachable_count += 1,
            _ => {}
        }

        syn::visit::visit_macro(self, mac);
    }
}

pub struct RustErrorDetector;

impl RustErrorDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detect_error_patterns(
        &self,
        context: &RustFunctionContext,
    ) -> Vec<ErrorPattern> {
        let mut patterns = Vec::new();

        // Traverse AST for error handling
        let mut visitor = ErrorPatternVisitor::new();
        visitor.visit_block(context.body());

        // Question mark operator usage
        if visitor.question_mark_count > 0 {
            patterns.push(ErrorPattern {
                pattern_type: ErrorPatternType::QuestionMarkOperator,
                count: visitor.question_mark_count,
                evidence: format!("Uses ? operator {} times", visitor.question_mark_count),
            });
        }

        // Unwrap usage (anti-pattern)
        if visitor.unwrap_count > 0 {
            patterns.push(ErrorPattern {
                pattern_type: ErrorPatternType::UnwrapUsage,
                count: visitor.unwrap_count,
                evidence: format!("Uses unwrap() {} times (anti-pattern)", visitor.unwrap_count),
            });
        }

        // Expect usage (better than unwrap)
        if visitor.expect_count > 0 {
            patterns.push(ErrorPattern {
                pattern_type: ErrorPatternType::ExpectUsage,
                count: visitor.expect_count,
                evidence: format!("Uses expect() {} times", visitor.expect_count),
            });
        }

        // Panic usage (anti-pattern)
        if visitor.panic_count > 0 {
            patterns.push(ErrorPattern {
                pattern_type: ErrorPatternType::PanicUsage,
                count: visitor.panic_count,
                evidence: format!("Uses panic!() {} times (anti-pattern)", visitor.panic_count),
            });
        }

        // Unreachable usage (anti-pattern in production code)
        if visitor.unreachable_count > 0 {
            patterns.push(ErrorPattern {
                pattern_type: ErrorPatternType::UnreachableUsage,
                count: visitor.unreachable_count,
                evidence: format!("Uses unreachable!() {} times", visitor.unreachable_count),
            });
        }

        // Unwrap or default (silent error suppression)
        if visitor.unwrap_or_default_count > 0 {
            patterns.push(ErrorPattern {
                pattern_type: ErrorPatternType::UnwrapOrDefaultUsage,
                count: visitor.unwrap_or_default_count,
                evidence: format!("Uses unwrap_or_default() {} times (may hide errors)", visitor.unwrap_or_default_count),
            });
        }

        // Ok().unwrap() chain (particularly bad anti-pattern)
        if visitor.ok_unwrap_chain_count > 0 {
            patterns.push(ErrorPattern {
                pattern_type: ErrorPatternType::OkUnwrapChain,
                count: visitor.ok_unwrap_chain_count,
                evidence: format!("Uses .ok().unwrap() {} times (severe anti-pattern)", visitor.ok_unwrap_chain_count),
            });
        }

        // Expect_err usage (uncommon, may indicate test code)
        if visitor.expect_err_count > 0 {
            patterns.push(ErrorPattern {
                pattern_type: ErrorPatternType::ExpectErrUsage,
                count: visitor.expect_err_count,
                evidence: format!("Uses expect_err() {} times", visitor.expect_err_count),
            });
        }

        // Check return type for Result
        if let ReturnType::Type(_, ty) = &context.item_fn.sig.output {
            if Self::is_result_type(ty) {
                patterns.push(ErrorPattern {
                    pattern_type: ErrorPatternType::QuestionMarkOperator,
                    count: 1,
                    evidence: "Returns Result type".into(),
                });
            }
        }

        patterns
    }

    fn is_result_type(ty: &Type) -> bool {
        if let Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                return segment.ident == "Result";
            }
        }
        false
    }

    pub fn classify_from_error_patterns(
        &self,
        patterns: &[ErrorPattern],
    ) -> Option<ResponsibilityCategory> {
        // High ? usage = Error Propagation & Handling
        let question_mark_count: usize = patterns.iter()
            .filter(|p| p.pattern_type == ErrorPatternType::QuestionMarkOperator)
            .map(|p| p.count)
            .sum();

        if question_mark_count >= 3 {
            return Some(ResponsibilityCategory::ErrorHandling);
        }

        // Custom error type = Error Handling
        if patterns.iter().any(|p| p.pattern_type == ErrorPatternType::CustomErrorType) {
            return Some(ResponsibilityCategory::ErrorHandling);
        }

        None
    }
}
```

**Phase 5: Builder Pattern Detection (AST-Based)**

Detect builder patterns common in Rust codebases:

```rust
// File: src/analysis/rust_patterns/builder_detector.rs

use syn::{FnArg, ItemFn, ReturnType, Type};

#[derive(Debug, Clone)]
pub struct BuilderPattern {
    pub pattern_type: BuilderPatternType,
    pub confidence: f64,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuilderPatternType {
    Constructor,           // new(), default()
    BuilderMethod,         // Methods returning Self
    WithMethod,            // with_* pattern
    SetterMethod,          // set_* pattern
    BuildFinalization,     // build(), finalize()
}

pub struct RustBuilderDetector;

impl RustBuilderDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detect_builder_patterns(
        &self,
        context: &RustFunctionContext,
    ) -> Vec<BuilderPattern> {
        let mut patterns = Vec::new();
        let fn_name = context.item_fn.sig.ident.to_string();

        // Constructor pattern
        if matches!(fn_name.as_str(), "new" | "default" | "create") {
            patterns.push(BuilderPattern {
                pattern_type: BuilderPatternType::Constructor,
                confidence: 0.9,
                evidence: format!("Constructor method: {}", fn_name),
            });
        }

        // Check return type for Self
        if let ReturnType::Type(_, ty) = &context.item_fn.sig.output {
            if Self::returns_self(ty) {
                // with_* pattern
                if fn_name.starts_with("with_") {
                    patterns.push(BuilderPattern {
                        pattern_type: BuilderPatternType::WithMethod,
                        confidence: 0.95,
                        evidence: format!("Builder with_* method: {}", fn_name),
                    });
                    patterns.push(BuilderPattern {
                        pattern_type: BuilderPatternType::BuilderMethod,
                        confidence: 0.9,
                        evidence: "Returns Self for chaining".into(),
                    });
                }
                // set_* pattern
                else if fn_name.starts_with("set_") {
                    patterns.push(BuilderPattern {
                        pattern_type: BuilderPatternType::SetterMethod,
                        confidence: 0.85,
                        evidence: format!("Builder set_* method: {}", fn_name),
                    });
                    patterns.push(BuilderPattern {
                        pattern_type: BuilderPatternType::BuilderMethod,
                        confidence: 0.85,
                        evidence: "Returns Self for chaining".into(),
                    });
                }
                // Generic builder method returning Self
                else if Self::takes_self_param(&context.item_fn) {
                    patterns.push(BuilderPattern {
                        pattern_type: BuilderPatternType::BuilderMethod,
                        confidence: 0.75,
                        evidence: "Method returns Self for chaining".into(),
                    });
                }
            }
        }

        // Build finalization pattern
        if matches!(fn_name.as_str(), "build" | "finalize" | "finish") {
            patterns.push(BuilderPattern {
                pattern_type: BuilderPatternType::BuildFinalization,
                confidence: 0.9,
                evidence: format!("Builder finalization method: {}", fn_name),
            });
        }

        patterns
    }

    /// Check if return type is Self
    fn returns_self(ty: &Type) -> bool {
        if let Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                return segment.ident == "Self";
            }
        }
        false
    }

    /// Check if function takes &self or &mut self
    fn takes_self_param(item_fn: &ItemFn) -> bool {
        item_fn.sig.inputs.iter().any(|arg| {
            matches!(arg, FnArg::Receiver(_))
        })
    }

    pub fn classify_from_builder_patterns(
        &self,
        patterns: &[BuilderPattern],
    ) -> Option<ResponsibilityCategory> {
        if patterns.is_empty() {
            return None;
        }

        // Constructor = Construction
        if patterns.iter().any(|p| p.pattern_type == BuilderPatternType::Constructor) {
            return Some(ResponsibilityCategory::Construction);
        }

        // Builder methods = Configuration
        if patterns.iter().any(|p| matches!(
            p.pattern_type,
            BuilderPatternType::WithMethod | BuilderPatternType::SetterMethod
        )) {
            return Some(ResponsibilityCategory::ConfigurationBuilder);
        }

        // Build finalization = Finalization
        if patterns.iter().any(|p| p.pattern_type == BuilderPatternType::BuildFinalization) {
            return Some(ResponsibilityCategory::Finalization);
        }

        None
    }
}
```

**Phase 6: Main Pattern Detector and Integration**

```rust
// File: src/analysis/rust_patterns/detector.rs

pub struct RustPatternDetector {
    trait_detector: RustTraitDetector,
    async_detector: RustAsyncDetector,
    error_detector: RustErrorDetector,
    builder_detector: RustBuilderDetector,  // NEW
}

impl RustPatternDetector {
    pub fn new() -> Self {
        Self {
            trait_detector: RustTraitDetector::new(),
            async_detector: RustAsyncDetector::new(),
            error_detector: RustErrorDetector::new(),
            builder_detector: RustBuilderDetector::new(),  // NEW
        }
    }

    /// Detect all Rust-specific patterns for a function
    pub fn detect_all_patterns(
        &self,
        context: &RustFunctionContext,
    ) -> RustPatternResult {
        RustPatternResult {
            trait_impl: self.trait_detector.detect_trait_impl(context),
            async_patterns: self.async_detector.detect_async_patterns(context),
            error_patterns: self.error_detector.detect_error_patterns(context),
            builder_patterns: self.builder_detector.detect_builder_patterns(context),  // NEW
        }
    }

    /// Classify function based on detected patterns
    /// Priority order: Trait impls > Async > Builder > Error handling
    pub fn classify_function(
        &self,
        context: &RustFunctionContext,
    ) -> Option<RustSpecificClassification> {
        // 1. Trait implementations (highest confidence)
        if let Some(trait_impl) = self.trait_detector.detect_trait_impl(context) {
            return Some(RustSpecificClassification {
                category: trait_impl.category,
                confidence: trait_impl.confidence,
                evidence: trait_impl.evidence,
                rust_pattern: RustPattern::TraitImplementation(trait_impl),
            });
        }

        // 2. Async/concurrency patterns
        let async_patterns = self.async_detector.detect_async_patterns(context);
        if let Some(category) = self.async_detector.classify_from_async_patterns(&async_patterns) {
            return Some(RustSpecificClassification {
                category,
                confidence: 0.85,
                evidence: format!("Async patterns: {:?}", async_patterns),
                rust_pattern: RustPattern::AsyncConcurrency(async_patterns),
            });
        }

        // 3. Builder patterns (NEW)
        let builder_patterns = self.builder_detector.detect_builder_patterns(context);
        if let Some(category) = self.builder_detector.classify_from_builder_patterns(&builder_patterns) {
            return Some(RustSpecificClassification {
                category,
                confidence: 0.80,
                evidence: format!("Builder patterns: {:?}", builder_patterns),
                rust_pattern: RustPattern::BuilderPattern(builder_patterns),
            });
        }

        // 4. Error handling patterns
        let error_patterns = self.error_detector.detect_error_patterns(context);
        if let Some(category) = self.error_detector.classify_from_error_patterns(&error_patterns) {
            return Some(RustSpecificClassification {
                category,
                confidence: 0.75,
                evidence: format!("Error handling: {:?}", error_patterns),
                rust_pattern: RustPattern::ErrorHandling(error_patterns),
            });
        }

        None
    }
}

#[derive(Debug, Clone)]
pub struct RustPatternResult {
    pub trait_impl: Option<TraitImplClassification>,
    pub async_patterns: Vec<AsyncPattern>,
    pub error_patterns: Vec<ErrorPattern>,
    pub builder_patterns: Vec<BuilderPattern>,  // NEW
}

#[derive(Debug, Clone)]
pub struct RustSpecificClassification {
    pub category: ResponsibilityCategory,
    pub confidence: f64,
    pub evidence: String,
    pub rust_pattern: RustPattern,
}

#[derive(Debug, Clone)]
pub enum RustPattern {
    TraitImplementation(TraitImplClassification),
    AsyncConcurrency(Vec<AsyncPattern>),
    BuilderPattern(Vec<BuilderPattern>),  // NEW
    ErrorHandling(Vec<ErrorPattern>),
}
```

**Phase 7: Integration into RustAnalyzer**

```rust
// File: src/analyzers/rust.rs (modifications)

use crate::analysis::rust_patterns::{
    RustPatternDetector, RustFunctionContext, ImplContext,
};

struct FunctionVisitor {
    // ... existing fields ...
    rust_pattern_detector: RustPatternDetector,
    current_impl_type: Option<String>,
    current_impl_is_trait: bool,
    current_trait_name: Option<String>,  // NEW: track trait name
}

impl FunctionVisitor {
    fn new(path: PathBuf, /* ... */) -> Self {
        Self {
            // ... existing initialization ...
            rust_pattern_detector: RustPatternDetector::new(),
            current_impl_type: None,
            current_impl_is_trait: false,
            current_trait_name: None,
        }
    }

    fn analyze_function(
        &mut self,
        name: String,
        item_fn: &syn::ItemFn,
        line: usize,
        is_trait_method: bool,
    ) {
        // ... existing analysis ...

        // NEW: Add Rust-specific pattern detection
        let impl_context = self.current_impl_type.as_ref().map(|impl_type| {
            ImplContext {
                impl_type: impl_type.clone(),
                is_trait_impl: self.current_impl_is_trait,
                trait_name: self.current_trait_name.clone(),
            }
        });

        let mut context = RustFunctionContext::from_item_fn(item_fn, &self.path);
        if let Some(ctx) = impl_context {
            context = context.with_impl_context(ctx);
        }

        let rust_patterns = self.rust_pattern_detector.detect_all_patterns(&context);

        // Store patterns in metrics using language-specific extension
        let mut metrics = FunctionMetrics::new(name.clone(), self.path.clone(), line);
        // ... set other metrics ...
        metrics.language_specific = Some(LanguageSpecificData::Rust(rust_patterns));  // NEW

        self.functions.push(metrics);
    }
}

impl<'ast> Visit<'ast> for FunctionVisitor {
    fn visit_item_impl(&mut self, item_impl: &'ast syn::ItemImpl) {
        // ... existing impl type extraction ...

        // NEW: Extract trait name if this is a trait impl
        let trait_name = item_impl.trait_.as_ref()
            .map(|(_, path, _)| {
                path.segments.last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default()
            });

        self.current_trait_name = trait_name;

        // ... continue visiting ...
    }
}
```

### Architecture Changes

**New Module**: `src/analysis/rust_patterns/`
- `mod.rs` - Module exports and public API
- `context.rs` - `RustFunctionContext` and `ImplContext`
- `detector.rs` - Main `RustPatternDetector`
- `trait_detector.rs` - Trait implementation detection (Phase 2)
- `async_detector.rs` - Async/concurrency patterns (Phase 3)
- `error_detector.rs` - Comprehensive error handling patterns (Phase 4)
- `builder_detector.rs` - Builder pattern detection (Phase 5)
- `conversion_detector.rs` - Type conversion patterns (future Phase 8)

**Modified Files**:
- `src/analyzers/rust.rs` - Integration point for pattern detection
- `src/core/mod.rs` - Add `LanguageSpecificData` enum and optional field to `FunctionMetrics`
  - Avoids memory overhead for non-Rust files
  - Provides extensibility for future Python/JavaScript pattern detectors

**No Integration with Spec 145**: This module operates standalone. If multi-signal aggregation is implemented later, it can consume `RustPatternResult` as one signal.

## Dependencies

- **Prerequisites**:
  - ✅ Spec 141 (I/O Detection) - Independent
  - ✅ Spec 142 (Call Graph) - Already implemented at `src/analyzers/rust_call_graph.rs`
- **Optional Integration**: Spec 145 (Multi-Signal Aggregation) - Not required
- **Affected Components**:
  - `src/analysis/` - new rust_patterns module (new)
  - `src/analyzers/rust.rs` - integration point (modified)
  - `src/core/mod.rs` - extend FunctionMetrics (modified)
- **External Dependencies**:
  - `syn` (already in use)
  - `quote` (already in use)

## Testing Strategy

### Unit Tests

All pattern detectors have unit tests using AST-based verification:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn detect_display_trait_via_context() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                write!(f, "MyType")
            }
        };

        let mut context = RustFunctionContext::from_item_fn(
            &item_fn,
            Path::new("test.rs"),
        );
        context = context.with_impl_context(ImplContext {
            impl_type: "MyType".into(),
            is_trait_impl: true,
            trait_name: Some("Display".into()),
        });

        let detector = RustTraitDetector::new();
        let classification = detector.detect_trait_impl(&context).unwrap();

        assert_eq!(classification.category, ResponsibilityCategory::Formatting);
        assert_eq!(classification.standard_trait, Some(StandardTrait::Display));
        assert!(classification.confidence > 0.9);
    }

    #[test]
    fn detect_async_spawn_via_ast() {
        let item_fn: syn::ItemFn = parse_quote! {
            async fn process_tasks() {
                tokio::spawn(async {
                    // Task logic
                });
            }
        };

        let context = RustFunctionContext::from_item_fn(
            &item_fn,
            Path::new("test.rs"),
        );

        let detector = RustAsyncDetector::new();
        let patterns = detector.detect_async_patterns(&context);

        assert!(patterns.iter().any(|p|
            p.pattern_type == AsyncPatternType::AsyncFunction
        ));
        assert!(patterns.iter().any(|p|
            p.pattern_type == AsyncPatternType::TaskSpawning
        ));
    }

    #[test]
    fn detect_error_propagation_via_ast() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn read_config() -> Result<Config, Error> {
                let file = File::open("config.toml")?;
                let content = read_to_string(file)?;
                let config = parse_toml(&content)?;
                Ok(config)
            }
        };

        let context = RustFunctionContext::from_item_fn(
            &item_fn,
            Path::new("test.rs"),
        );

        let detector = RustErrorDetector::new();
        let patterns = detector.detect_error_patterns(&context);

        let question_marks: usize = patterns.iter()
            .filter(|p| p.pattern_type == ErrorPatternType::QuestionMarkOperator)
            .map(|p| p.count)
            .sum();

        assert!(question_marks >= 3);
    }

    #[test]
    fn no_false_positives_from_comments() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn example() {
                // This comment mentions Mutex but shouldn't trigger detection
                let x = 42;
            }
        };

        let context = RustFunctionContext::from_item_fn(
            &item_fn,
            Path::new("test.rs"),
        );

        let detector = RustAsyncDetector::new();
        let patterns = detector.detect_async_patterns(&context);

        // Should NOT detect mutex usage from comment
        assert!(!patterns.iter().any(|p|
            p.pattern_type == AsyncPatternType::MutexUsage
        ));
    }
}
```

### Integration Tests

Test on debtmap's own codebase:

```rust
#[test]
fn rust_patterns_on_debtmap_code() {
    let files = vec![
        "src/analyzers/rust.rs",
        "src/core/mod.rs",
        "src/analysis/call_graph/graph_builder.rs",
    ];

    let detector = RustPatternDetector::new();

    for file_path in files {
        let content = std::fs::read_to_string(file_path).unwrap();
        let file: syn::File = syn::parse_file(&content).unwrap();

        for item in &file.items {
            if let syn::Item::Fn(item_fn) = item {
                let context = RustFunctionContext::from_item_fn(
                    item_fn,
                    Path::new(file_path),
                );

                if let Some(classification) = detector.classify_function(&context) {
                    println!(
                        "{}: {} ({:.2}) - {}",
                        item_fn.sig.ident,
                        classification.category,
                        classification.confidence,
                        classification.evidence
                    );
                }
            }
        }
    }
}
```

### Performance Benchmarks

Use `criterion` to validate <5% overhead claim:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_pattern_detection(c: &mut Criterion) {
    let item_fn: syn::ItemFn = parse_quote! {
        async fn complex_function() -> Result<(), Error> {
            let mutex = Mutex::new(0);
            tokio::spawn(async move {
                let _ = mutex.lock()?;
            });
            Ok(())
        }
    };

    let context = RustFunctionContext::from_item_fn(
        &item_fn,
        Path::new("bench.rs"),
    );

    let detector = RustPatternDetector::new();

    c.bench_function("detect_all_patterns", |b| {
        b.iter(|| detector.detect_all_patterns(black_box(&context)))
    });
}

criterion_group!(benches, benchmark_pattern_detection);
criterion_main!(benches);
```

### Performance Validation Strategy

**Baseline Establishment (Phase 1)**:
```bash
# Measure current analysis speed without pattern detection
cargo bench --bench analysis_baseline -- --save-baseline before-patterns

# Measure memory usage without pattern detection
cargo run --release --features dhat-heap -- analyze src/
```

**Per-Detector Validation (Phases 2-5)**:
After each detector implementation:
1. Run benchmarks: `cargo bench --bench analysis_baseline -- --baseline before-patterns`
2. Verify <5% regression from baseline
3. If overhead exceeds threshold, profile and optimize before proceeding

**Memory Profiling**:
```rust
// benches/memory_footprint.rs
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[bench]
fn measure_memory_with_patterns() {
    let _profiler = dhat::Profiler::new_heap();

    // Analyze debtmap's own codebase (~500 functions)
    let analyzer = RustAnalyzer::new();
    let results = analyzer.analyze_directory("src/");

    // dhat will report total allocations and peak memory
}
```

**Performance Acceptance Criteria**:
- Total overhead: <5% increase in wall-clock time
- Per-function latency: 99th percentile <100μs
- Memory overhead: <10% increase in heap allocations
- Zero performance impact on non-Rust files

## Documentation Requirements

### User Documentation

Update README.md:

```markdown
## Rust-Specific Pattern Detection

For Rust code, debtmap recognizes language-specific patterns:

**Trait Implementations**:
- Display/Debug → Formatting
- From/Into → Type Conversion
- Default/Clone → Construction
- Drop → Resource Cleanup
- Iterator → Iteration Logic

**Async Patterns**:
- async fn → Asynchronous Operation
- tokio::spawn → Concurrency Management
- Channels → Communication Orchestration

**Error Handling**:
- High ? usage → Error Propagation
- Custom error types → Error Handling
- unwrap() usage → Flagged as anti-pattern

**Builder Patterns**:
- Chainable methods → Configuration Builder
- build() method → Finalization

All pattern detection uses AST traversal for accuracy - no false positives from comments or strings.
```

### Developer Documentation

Create `docs/rust-patterns.md` explaining:
- How to add new pattern detectors
- AST visitor patterns used
- Integration with existing analyzers
- Performance considerations

## Implementation Notes

### AST-Based Detection Principles

1. **Always use `syn::visit::Visit`** for traversing function bodies
2. **Never use string matching** on function body text (only for reporting)
3. **Leverage existing infrastructure**:
   - `sig.asyncness` for async detection
   - `TraitImplementationTracker` for trait analysis
   - `CallGraph` for call relationships
4. **Test for false positives** from comments/strings

### Performance Optimization

- Pattern detection runs during single-pass AST traversal
- Visitors are lightweight (no heap allocations in hot paths)
- Results cached in `FunctionMetrics` (computed once)
- Parallel analysis of files maintains performance

## Migration and Compatibility

### Backward Compatibility

- New optional field in `FunctionMetrics`: `rust_pattern_result: Option<RustPatternResult>`
- No breaking changes to existing APIs
- Pattern detection opt-in via configuration flag

### Rollout Strategy

1. **Phase 1**: Implement standalone module with tests
2. **Phase 2**: Integrate into `RustAnalyzer` behind feature flag
3. **Phase 3**: Enable by default after validation
4. **Phase 4**: Use patterns for debt prioritization

## Expected Impact

### Accuracy Improvement for Rust

- **Generic analysis**: ~85% accuracy on Rust code
- **+ Rust patterns**: ~92% accuracy on Rust code
- **Improvement**: +7 percentage points for Rust

### Better Rust-Specific Classifications

```rust
// Before (generic analysis)
impl Display for User {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { ... }
}
// Classification: "General Logic" (low confidence)

// After (Rust-specific)
impl Display for User {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { ... }
}
// Classification: "Formatting (Display trait)" (0.95 confidence)
```

### Foundation for Future Enhancements

This pattern establishes:
- AST-based pattern detection architecture
- Language-specific plugin system
- Path for Python/JavaScript pattern detectors

## Integration with Spec 145 (Multi-Signal Aggregation)

### Overview

Spec 145 is **implemented** at `src/analysis/multi_signal_aggregation.rs`. Rust-specific patterns should integrate as a new signal type alongside existing signals.

### Current Signal Types (Spec 145)

```rust
pub enum SignalType {
    IoDetection,
    CallGraph,
    Purity,
    Framework,
    TypeSignatures,
    Name,
    // Add: RustPatterns (Spec 146)
}

pub struct SignalSet {
    pub io_signal: Option<IoClassification>,
    pub call_graph_signal: Option<CallGraphClassification>,
    pub purity_signal: Option<PurityClassification>,
    pub framework_signal: Option<FrameworkClassification>,
    pub type_signal: Option<TypeSignatureClassification>,
    pub name_signal: Option<NameBasedClassification>,
    // Add: rust_pattern_signal (Spec 146)
}
```

### Integration Changes Required

**1. Extend `SignalType` enum**:

```rust
// File: src/analysis/multi_signal_aggregation.rs

pub enum SignalType {
    IoDetection,
    CallGraph,
    Purity,
    Framework,
    TypeSignatures,
    Name,
    RustPatterns,  // NEW
}
```

**2. Add Rust pattern classification type**:

```rust
// File: src/analysis/multi_signal_aggregation.rs

/// Classification from Rust-specific pattern detection (Spec 146)
#[derive(Debug, Clone)]
pub struct RustPatternClassification {
    pub category: ResponsibilityCategory,
    pub confidence: f64,
    pub evidence: String,
    pub pattern_type: String,  // e.g., "Display trait", "Async spawn", "Error handling"
}
```

**3. Extend `SignalSet` structure**:

```rust
// File: src/analysis/multi_signal_aggregation.rs

#[derive(Debug, Clone, Default)]
pub struct SignalSet {
    pub io_signal: Option<IoClassification>,
    pub call_graph_signal: Option<CallGraphClassification>,
    pub purity_signal: Option<PurityClassification>,
    pub framework_signal: Option<FrameworkClassification>,
    pub type_signal: Option<TypeSignatureClassification>,
    pub name_signal: Option<NameBasedClassification>,
    pub rust_pattern_signal: Option<RustPatternClassification>,  // NEW
}
```

**4. Update `SignalWeights` configuration**:

```rust
// File: src/analysis/multi_signal_aggregation.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalWeights {
    pub io_detection: f64,
    pub call_graph: f64,
    pub type_signatures: f64,
    pub purity_side_effects: f64,
    pub framework_patterns: f64,
    pub rust_patterns: f64,      // NEW
    pub name_heuristics: f64,
}

impl Default for SignalWeights {
    fn default() -> Self {
        SignalWeights {
            io_detection: 0.30,      // Reduced from 0.35
            call_graph: 0.25,
            type_signatures: 0.15,
            purity_side_effects: 0.05,
            framework_patterns: 0.05,
            rust_patterns: 0.05,     // NEW weight for Rust patterns
            name_heuristics: 0.15,
            // Total: 1.00
        }
    }
}
```

**5. Add to ResponsibilityCategory enum** (expand existing categories):

```rust
// File: src/analysis/multi_signal_aggregation.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResponsibilityCategory {
    // Existing categories...
    FileIO,
    NetworkIO,
    DatabaseIO,
    // ... etc ...

    // NEW: Rust-specific categories
    TypeConversion,             // From/Into traits
    Construction,               // Default/Clone/new()
    ConfigurationBuilder,       // Builder with_*/set_* methods
    Finalization,               // build()/finalize() methods
    ResourceCleanup,            // Drop trait
    Iteration,                  // Iterator trait
    ConcurrencyManagement,      // tokio::spawn, threads
    CommunicationOrchestration, // Channels
    AsynchronousOperation,      // async fn

    // Existing...
    Unknown,
}
```

**6. Extend ResponsibilityAggregator**:

```rust
// File: src/analysis/multi_signal_aggregation.rs

use crate::analysis::rust_patterns::RustPatternDetector;

pub struct ResponsibilityAggregator {
    config: AggregationConfig,
    io_detector: IoDetector,
    purity_analyzer: PurityAnalyzer,
    framework_detector: Option<FrameworkDetector>,
    call_graph: Option<RustCallGraph>,
    type_tracker: Option<TypeFlowTracker>,
    rust_pattern_detector: Option<RustPatternDetector>,  // NEW
}

impl ResponsibilityAggregator {
    /// Set Rust pattern detector (only active for Rust code)
    pub fn with_rust_pattern_detector(mut self, detector: RustPatternDetector) -> Self {
        self.rust_pattern_detector = Some(detector);
        self
    }

    /// Collect Rust pattern signal from function context
    pub fn collect_rust_pattern_signal(
        &self,
        context: &RustFunctionContext,
    ) -> Option<RustPatternClassification> {
        let detector = self.rust_pattern_detector.as_ref()?;
        let classification = detector.classify_function(context)?;

        Some(RustPatternClassification {
            category: classification.category,
            confidence: classification.confidence,
            evidence: classification.evidence,
            pattern_type: format!("{:?}", classification.rust_pattern),
        })
    }
}
```

**7. Update aggregation logic**:

```rust
// File: src/analysis/multi_signal_aggregation.rs

impl ResponsibilityAggregator {
    pub fn aggregate(&self, signals: SignalSet) -> AggregatedClassification {
        let mut scores: HashMap<ResponsibilityCategory, f64> = HashMap::new();
        let mut evidence: Vec<SignalEvidence> = Vec::new();

        // ... existing signal processing ...

        // NEW: Process Rust pattern signal
        if let Some(rust_signal) = signals.rust_pattern_signal {
            let weight = self.config.weights.rust_patterns;
            let contribution = rust_signal.confidence * weight;

            *scores.entry(rust_signal.category).or_insert(0.0) += contribution;

            evidence.push(SignalEvidence {
                signal_type: SignalType::RustPatterns,
                category: rust_signal.category,
                confidence: rust_signal.confidence,
                weight,
                contribution,
                description: rust_signal.evidence,
            });
        }

        // ... rest of aggregation logic ...
    }
}
```

### Integration Priority

Rust patterns have **high confidence** (0.85-0.95) for specific cases:
- Trait implementations: 0.95 confidence → Should influence final classification strongly
- Async patterns: 0.85 confidence
- Error handling: 0.75 confidence

**Override Strategy**: When Rust patterns detect a trait implementation with >0.9 confidence, it should override weaker signals unless `framework_patterns` has higher confidence.

### Language-Specific Activation

Rust patterns should only activate for Rust code:

```rust
// In RustAnalyzer integration
fn analyze_function(&mut self, item_fn: &syn::ItemFn, ...) {
    // ... existing analysis ...

    // Only collect Rust pattern signal for Rust files
    if self.language == Language::Rust {
        let rust_signal = self.aggregator.collect_rust_pattern_signal(&context);
        signal_set.rust_pattern_signal = rust_signal;
    }

    let classification = self.aggregator.aggregate(signal_set);
}
```

### Weight Rationale

**5% weight** for Rust patterns is conservative but appropriate because:
- High precision (>90%) but limited scope (Rust only)
- Complements rather than replaces other signals
- Most valuable for trait impls and async patterns
- Framework patterns (5%) handle similar concerns for other languages

### Expected Accuracy Impact

With Rust patterns integrated:
- **Rust code**: ~92% accuracy (up from ~85%)
- **Other languages**: No change (signal inactive)
- **Overall project**: Depends on Rust percentage

## Open Questions

1. ✅ **Multi-Signal Integration**: **RESOLVED** - Spec 145 is implemented, integrate as new signal type

2. ✅ **Builder Pattern Detection**: **RESOLVED** - Included in Phase 5 (high ROI, low complexity)

3. ✅ **Memory Footprint**: **RESOLVED** - Use `LanguageSpecificData` enum to avoid overhead for non-Rust files

4. ✅ **Comprehensive Error Patterns**: **RESOLVED** - Detect unwrap, expect, panic, unreachable, ok().unwrap(), etc.

5. **Configuration**: Should pattern detection be always-on or opt-in?
   - **Recommendation**: Always-on for Rust files (minimal overhead validated via benchmarks)

6. **Signal Weight**: Is 5% appropriate or should it be higher?
   - **Recommendation**: Start with 5%, monitor accuracy metrics, adjust if needed

7. **Performance Validation**: When should benchmarks be established?
   - **Recommendation**: Baseline benchmarks in Phase 1, re-validate after each detector addition

## Revision History

- 2025-10-27: Initial draft
- 2025-10-29: Major revision based on AST audit
  - Corrected data structures (FunctionAst → RustFunctionContext)
  - Changed from string matching to AST-based detection
  - Added verified capability references
  - Defined standalone integration strategy
  - Added performance benchmarking requirements
- 2025-10-29: Integration with Spec 145
  - Added multi-signal aggregation integration section
  - Defined RustPatternClassification structure
  - Specified SignalSet and SignalWeights modifications
  - Defined ResponsibilityCategory extensions
  - Clarified language-specific activation strategy
  - Set 5% weight for rust_patterns signal
- 2025-10-30: Performance and feature enhancements
  - Added `LanguageSpecificData` enum for memory footprint optimization
  - Expanded error pattern detection (unreachable!, unwrap_or_default(), ok().unwrap(), expect_err())
  - Moved builder pattern detection from "future" to Phase 5 (high ROI)
  - Added comprehensive performance validation requirements (baseline, memory, per-function latency)
  - Added memory footprint benchmarking with dhat
  - Added builder-specific ResponsibilityCategory values (ConfigurationBuilder, Finalization)
  - Defined empirical performance targets (<5% overhead, <100μs per function)
  - Updated acceptance criteria with performance and memory validation
  - Resolved open questions (builder patterns, memory footprint, error patterns)
