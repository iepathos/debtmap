---
number: 122
title: AST-Based Constructor Detection
category: foundation
priority: medium
status: draft
dependencies: []
created: 2025-10-21
updated: 2025-10-21
---

# Specification 122: AST-Based Constructor Detection

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: None (builds on existing constructor detection in `src/priority/semantic_classifier.rs`)

## Context

The existing name-based constructor detection (implemented in `src/priority/semantic_classifier.rs:90-109`) catches 80-90% of constructors using pattern matching (e.g., `new`, `from_*`, `with_*`), but misses:

1. **Non-standard names**: `create_default_client()`, `init_empty()`, `make_server()`
2. **Builder methods**: `set_timeout()`, `add_header()` (return `Self`)
3. **Language-specific patterns**: Python `@classmethod`, TypeScript `static` factories
4. **Macro-generated constructors**: Rust `derive(Default)` implementations

**Example Missed Constructors**:

```rust
// Not caught by name patterns (doesn't start with "new", "from_", etc.)
pub fn create_default_client() -> Self {
    Self {
        timeout: Duration::from_secs(30),
        retries: 3,
    }
}

// Builder method returning Self
pub fn set_timeout(mut self, timeout: Duration) -> Self {
    self.timeout = timeout;
    self
}
```

**Current Implementation** (`src/priority/semantic_classifier.rs:90-109`):
```rust
fn is_simple_constructor(func: &FunctionMetrics) -> bool {
    let config = crate::config::get_constructor_detection_config();
    let name_lower = func.name.to_lowercase();
    let matches_constructor_name = config.patterns.iter().any(|pattern| {
        name_lower == *pattern || name_lower.starts_with(pattern) || name_lower.ends_with(pattern)
    });
    let is_simple = func.cyclomatic <= config.max_cyclomatic
        && func.length < config.max_length
        && func.nesting <= config.max_nesting;
    let is_initialization = func.cognitive <= config.max_cognitive;
    matches_constructor_name && is_simple && is_initialization
}
```

**Current Impact**:
- ~10-20% of constructors still misclassified as `PureLogic`
- False positives for builder pattern methods
- No detection of macro-generated constructors
- Constructors currently map to `FunctionRole::IOWrapper` (0.7x multiplier)

**Why AST-Based Detection**:
- Analyze function body structure, not just name
- Detect `-> Self` return type via `syn` AST
- Recognize struct initialization patterns
- Identify builder pattern characteristics
- Language-specific detection using existing parsers (`syn` for Rust, `rustpython_parser` for Python)

## Objective

Enhance constructor detection using AST analysis to catch non-standard constructor patterns, reducing false positive rate to <5%.

**Phased Approach**:
- **Phase 1** (This Spec): Rust-only AST detection using `syn` (integrates with existing infrastructure)
- **Phase 2** (Future): Python/TypeScript/JavaScript support
- **Phase 3** (Future): Builder pattern detection as separate `FunctionRole`

## Requirements

### Functional Requirements

**FR1: Return Type Analysis**
- Parse function signature to extract return type
- Detect `-> Self`, `-> Result<Self>`, `-> Option<Self>` patterns
- Identify constructors by return type even with non-standard names

**FR2: Body Pattern Recognition**
- Detect struct initialization: `Self { field1, field2 }`
- Identify builder patterns: `self.field = value; self`
- Recognize delegation to other constructors: `Self::new()`
- Detect minimal control flow (≤2 branches)

**FR3: Language-Specific Detection (Phase 1: Rust Only)**

**Rust** (using `syn` AST):
- `-> Self` return type via `syn::ReturnType`
- Struct expression `Self { ... }` via `syn::ExprStruct`
- Tuple struct: `Self(value)` via `syn::ExprCall`
- Enum variant: `Self::Variant(value)` via `syn::ExprPath`

**Future Phases** (Python/TypeScript/JavaScript):
- Python: `@classmethod` detection via existing `rustpython_parser`
- TypeScript/JavaScript: Static factory methods via existing parser
- Will be specified in separate specs when Phase 1 is validated

**FR4: Complexity Thresholds**
- Cyclomatic complexity ≤ 5 (allow some validation logic)
- Nesting depth ≤ 2
- Function length < 30 lines (more lenient than Spec 117's 15)
- No loops (constructors shouldn't iterate)

**FR5: Builder Pattern Detection (Phase 3 - Future)**
- Detects methods that:
  1. Take `self` or `mut self` as receiver
  2. Return `Self` or `Self` type
  3. Modify fields and return `self`
- Classify as `FunctionRole::Builder` (new role, 0.6x multiplier)
- **Not included in Phase 1** - requires separate enum variant addition

### Non-Functional Requirements

**NFR1: Performance**
- `syn` AST already parsed in `src/analyzers/rust.rs` for complexity analysis
- Return type extraction: traverse existing AST (minimal overhead)
- Body pattern detection: requires additional AST walk
- **Target**: < 5% overhead vs current name-only detection (measured via benchmarks)
- Cache AST analysis results in `FunctionMetrics` (optional field)

**NFR2: Accuracy**
- Reduce false positives to < 5% (from ~20% with name-only)
- No false negatives for standard constructors
- Conservative classification (when unsure, don't classify as constructor)

**NFR3: Maintainability**
- Language-specific patterns in separate modules
- Extensible to new languages
- Clear documentation of detection heuristics

**NFR4: Compatibility**
- Falls back to existing `is_simple_constructor()` if AST analysis fails
- Phase 1: Rust-only (Python/TypeScript/Go in future phases)
- Graceful degradation for syntax errors
- **Non-breaking**: AST detection is additive enhancement of existing behavior
- **Enabled by default** - provides better accuracy out-of-box
- Opt-out available via configuration if needed

## Acceptance Criteria

**Phase 1 (Rust-only)**:
- [ ] AST-based return type analysis implemented for Rust using `syn`
- [ ] Struct initialization pattern detection working (`Self { ... }`)
- [ ] False positive rate reduced to < 10% for constructor classification (vs ~20% currently)
- [ ] `create_default_client()` example correctly classified as constructor
- [ ] Test suite includes AST-based detection test cases for Rust
- [ ] Performance overhead < 5% compared to name-only detection (measured via benchmarks)
- [ ] Configuration flag `classification.constructors.ast_detection` added (default: `true`)
- [ ] Fallback to `is_simple_constructor()` works when AST analysis fails
- [ ] Opt-out mechanism works when users set `ast_detection = false`
- [ ] Documentation updated with AST detection logic
- [ ] No breaking changes to existing JSON output schema (AST data is optional)

**Future Phases** (not in this spec):
- [ ] Builder pattern methods detected and classified separately (`FunctionRole::Builder`)
- [ ] Python `@classmethod` constructor patterns recognized
- [ ] TypeScript/JavaScript static factory methods recognized
- [ ] False positive rate reduced to < 5% (ultimate goal)

## Technical Details

### Implementation Approach

**Phase 1: AST Return Type Extraction**

**File**: `src/analyzers/rust_constructor_detector.rs` (new module)

```rust
use syn::{ItemFn, ReturnType as SynReturnType, Type, TypePath};

/// Extract return type from function signature using syn
pub fn extract_return_type(func: &ItemFn) -> Option<ConstructorReturnType> {
    match &func.sig.output {
        SynReturnType::Default => None, // No return type
        SynReturnType::Type(_, ty) => classify_return_type(ty),
    }
}

/// Classify return type for constructor detection
fn classify_return_type(ty: &Type) -> Option<ConstructorReturnType> {
    match ty {
        Type::Path(type_path) => {
            let path_str = quote::quote!(#type_path).to_string();

            if path_str == "Self" {
                Some(ConstructorReturnType::OwnedSelf)
            } else if path_str.starts_with("Result < Self") {
                Some(ConstructorReturnType::ResultSelf)
            } else if path_str.starts_with("Option < Self") {
                Some(ConstructorReturnType::OptionSelf)
            } else {
                Some(ConstructorReturnType::Other)
            }
        }
        Type::Reference(type_ref) => {
            if let Type::Path(path) = &*type_ref.elem {
                let path_str = quote::quote!(#path).to_string();
                if path_str == "Self" {
                    return Some(ConstructorReturnType::RefSelf);
                }
            }
            Some(ConstructorReturnType::Other)
        }
        _ => Some(ConstructorReturnType::Other),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructorReturnType {
    OwnedSelf,          // -> Self
    ResultSelf,         // -> Result<Self, E>
    OptionSelf,         // -> Option<Self>
    RefSelf,            // -> &Self or &mut Self (builder pattern)
    Other,              // Other types
}
```

**Phase 2: Body Pattern Detection**

```rust
use syn::{visit::Visit, Expr, ExprStruct, ExprPath, Stmt};

/// Visitor to detect constructor patterns in function body
pub struct ConstructorPatternVisitor {
    pattern: BodyPattern,
}

impl ConstructorPatternVisitor {
    pub fn new() -> Self {
        Self {
            pattern: BodyPattern::default(),
        }
    }

    pub fn into_pattern(self) -> BodyPattern {
        self.pattern
    }
}

impl<'ast> Visit<'ast> for ConstructorPatternVisitor {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            Expr::Struct(_) => {
                self.pattern.struct_init_count += 1;
            }
            Expr::Path(path) => {
                // Check for Self references
                let path_str = quote::quote!(#path).to_string();
                if path_str.starts_with("Self") {
                    self.pattern.self_refs += 1;
                }
            }
            Expr::If(_) => self.pattern.has_if = true,
            Expr::Match(_) => self.pattern.has_match = true,
            Expr::Loop(_) | Expr::While(_) | Expr::ForLoop(_) => {
                self.pattern.has_loop = true;
            }
            Expr::Return(_) => self.pattern.early_returns += 1,
            Expr::Field(_) | Expr::Assign(_) => {
                self.pattern.field_assignments += 1;
            }
            _ => {}
        }
        syn::visit::visit_expr(self, expr);
    }
}

/// Analyze function body for constructor patterns
pub fn analyze_function_body(func: &ItemFn) -> BodyPattern {
    let mut visitor = ConstructorPatternVisitor::new();
    visitor.visit_block(&func.block);
    visitor.into_pattern()
}

#[derive(Debug, Clone, Default)]
pub struct BodyPattern {
    pub struct_init_count: usize,
    pub self_refs: usize,
    pub field_assignments: usize,
    pub has_if: bool,
    pub has_match: bool,
    pub has_loop: bool,
    pub early_returns: usize,
}

impl BodyPattern {
    /// Does this look like a constructor body?
    pub fn is_constructor_like(&self) -> bool {
        // Has struct initialization and no loops
        (self.struct_init_count > 0 && !self.has_loop)
        ||
        // Or minimal logic (≤1 if/match) with Self refs
        (self.self_refs > 0 && !self.has_loop && !self.has_match && self.field_assignments == 0)
    }

    /// Does this look like a builder method? (Phase 3 - not implemented yet)
    pub fn is_builder_like(&self) -> bool {
        // Modifies fields and returns self
        self.field_assignments > 0
            && self.early_returns <= 1
            && !self.has_loop
    }
}
```

**Phase 3: Combined Detection Logic**

**File**: `src/priority/semantic_classifier.rs` (modify existing `is_simple_constructor()`)

```rust
/// Enhanced constructor detection using AST (enabled by default)
fn is_constructor_enhanced(
    func: &FunctionMetrics,
    syn_func: Option<&syn::ItemFn>,
) -> bool {
    // Check configuration
    let config = crate::config::get_constructor_detection_config();

    // If AST detection disabled or unavailable, use name-based detection
    if !config.ast_detection || syn_func.is_none() {
        return is_simple_constructor(func);
    }

    let syn_func = syn_func.unwrap();

    // Extract AST information
    let return_type = extract_return_type(syn_func);
    let body_pattern = analyze_function_body(syn_func);

    // Check return type (must return Self)
    let returns_self = matches!(
        return_type,
        Some(ConstructorReturnType::OwnedSelf
            | ConstructorReturnType::ResultSelf
            | ConstructorReturnType::OptionSelf)
    );

    if !returns_self {
        // Fallback to name-based detection if not returning Self
        return is_simple_constructor(func);
    }

    // Check body pattern
    if !body_pattern.is_constructor_like() {
        return false;
    }

    // Check complexity thresholds (more lenient for AST-detected constructors)
    let is_simple_enough = func.cyclomatic <= 5
        && func.nesting <= 2
        && func.length < 30
        && !body_pattern.has_loop;

    returns_self && is_simple_enough
}

// Update classify_by_rules to use enhanced detection
fn classify_by_rules(
    func: &FunctionMetrics,
    func_id: &FunctionId,
    call_graph: &CallGraph,
    syn_func: Option<&syn::ItemFn>,  // NEW parameter
) -> Option<FunctionRole> {
    // Entry point has highest precedence
    if is_entry_point(func_id, call_graph) {
        return Some(FunctionRole::EntryPoint);
    }

    // Check for constructors with AST enhancement
    if is_constructor_enhanced(func, syn_func) {
        return Some(FunctionRole::IOWrapper);  // Phase 1: Still maps to IOWrapper
    }

    // ... rest of classification logic unchanged
}
```

**Note**: Phase 1 keeps constructors as `FunctionRole::IOWrapper`. Adding a dedicated `Constructor` variant is deferred to Phase 3 to avoid breaking changes.

**Phase 4: Threading AST Through Analysis**

**File**: `src/analyzers/rust.rs` (modify existing analysis)

```rust
// In analyze_rust_file(), extract functions and preserve syn::ItemFn references
fn analyze_rust_file(ast: &RustAst, ...) -> FileMetrics {
    let source_content = read_source_content(&ast.path);

    // Visit each function in the AST
    let mut functions = Vec::new();
    for item in &ast.file.items {
        if let syn::Item::Fn(func) = item {
            let metrics = extract_function_metrics(func, &ast.path);

            // Store reference to syn::ItemFn for later AST analysis
            // (OR: extract constructor info here and store in metrics)

            functions.push(metrics);
        }
    }

    // ... rest of analysis
}
```

**Future: Adding `FunctionRole::Constructor` and `Builder`** (Phase 3):
- Requires enum variant addition to `src/priority/semantic_classifier.rs`
- Breaking change to pattern matching across codebase
- Requires migration guide for users
- Separate spec recommended for this change

**Future: Language-Specific Patterns** (Phase 2):

Python constructor detection (using `rustpython_parser`):
```rust
// To be added in Phase 2
mod python_constructor_detector {
    use rustpython_parser::ast;

    pub fn is_python_constructor(func: &ast::StmtFunctionDef) -> bool {
        // Check for @classmethod decorator
        func.decorator_list.iter().any(|dec| {
            // Check if decorator is "classmethod"
            matches!(dec.node, ast::ExprName { id, .. } if id == "classmethod")
        })
    }
}
```

TypeScript/JavaScript constructor detection:
```rust
// To be specified in Phase 2 spec
// Will use existing JavaScript/TypeScript parser infrastructure
```

### Architecture Changes

**Modified Files** (Phase 1):
- `src/analyzers/rust.rs` - Thread `syn::ItemFn` through to classification
- `src/priority/semantic_classifier.rs` - Add `is_constructor_enhanced()` function
- `src/config.rs` - Add `ast_detection` flag to `ConstructorDetectionConfig`

**New Files** (Phase 1):
- `src/analyzers/rust_constructor_detector.rs` - Rust constructor pattern detection using `syn`
  - `extract_return_type()` - Parse return type from `syn::ItemFn`
  - `analyze_function_body()` - Detect constructor body patterns
  - `ConstructorPatternVisitor` - `syn::visit::Visit` implementation

**Future Files** (Phase 2+):
- `src/analyzers/python_constructor_detector.rs` - Python patterns
- `src/analyzers/javascript_constructor_detector.rs` - JS/TS patterns

**Data Flow** (Phase 1):
```
Source Code (Rust)
    ↓
syn::parse_str() [existing in rust.rs]
    ↓
syn::File AST
    ↓
Extract functions → Vec<syn::ItemFn>
    ↓
For each function:
    ├─ FunctionMetrics extraction (existing)
    ├─ Return Type Analysis (NEW: extract_return_type)
    ├─ Body Pattern Analysis (NEW: analyze_function_body)
    └─ Complexity Metrics (existing)
    ↓
Classification (semantic_classifier.rs)
    ├─ Name-based detection (existing: is_simple_constructor)
    ├─ AST-based detection (NEW: is_constructor_enhanced)
    └─ Combined Decision → FunctionRole::IOWrapper
    ↓
Risk Scoring (uses role multipliers)
```

### Data Structures

**Phase 1**: No changes to `FunctionMetrics` schema (avoid breaking changes)

Constructor detection is performed on-the-fly during classification using `syn::ItemFn` AST.

**Future** (Phase 3): If caching AST analysis results is needed:
```rust
// Add optional field to FunctionMetrics (requires schema versioning)
impl FunctionMetrics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constructor_info: Option<ConstructorInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructorInfo {
    pub detected_by_ast: bool,
    pub return_type: String,  // "Self", "Result<Self>", etc.
    pub struct_init_count: usize,
}
```

**Rationale**: Phase 1 keeps schema stable. AST analysis adds ~5% overhead, which is acceptable for on-the-fly detection.

### APIs and Interfaces

**Internal API** (Phase 1 - not exposed publicly):
```rust
// In src/analyzers/rust_constructor_detector.rs

/// Extract return type from syn::ItemFn
pub(crate) fn extract_return_type(func: &syn::ItemFn) -> Option<ConstructorReturnType>;

/// Analyze function body for constructor patterns
pub(crate) fn analyze_function_body(func: &syn::ItemFn) -> BodyPattern;

// In src/priority/semantic_classifier.rs

/// Enhanced constructor detection (internal)
fn is_constructor_enhanced(
    func: &FunctionMetrics,
    syn_func: Option<&syn::ItemFn>,
) -> bool;
```

**Public API** (no changes): Constructor detection is internal to classification logic.

## Dependencies

**Prerequisites** (already implemented):
- Name-based constructor detection in `src/priority/semantic_classifier.rs:90-109`
- `syn` AST parsing in `src/analyzers/rust.rs`
- `FunctionMetrics` data structure in `src/core/mod.rs`
- `ConstructorDetectionConfig` in `src/config.rs`

**Affected Components**:
- `src/analyzers/rust.rs` - Need to pass `syn::ItemFn` to classification
- `src/priority/semantic_classifier.rs` - Add AST-based detection
- Risk scoring (no changes - uses existing role multipliers)

**External Dependencies** (no new dependencies):
- `syn = "2.0"` (already in `Cargo.toml`)
- `quote = "1.0"` (already in `Cargo.toml`)

## Testing Strategy

### Unit Tests

**Test AST-Based Detection**:
```rust
#[test]
fn test_ast_detects_non_standard_constructor_name() {
    let source = r#"
        pub fn create_default_client() -> Self {
            Self {
                timeout: Duration::from_secs(30),
                retries: 3,
            }
        }
    "#;

    let ast_analysis = analyze_ast(source);
    assert_eq!(ast_analysis.return_type, Some(ReturnType::OwnedSelf));
    assert!(ast_analysis.body_pattern.is_constructor_like());

    let func = create_function_from_source(source);
    assert!(is_constructor_ast(&func, &Some(ast_analysis)));
}

#[test]
fn test_builder_method_detection() {
    let source = r#"
        pub fn set_timeout(mut self, timeout: Duration) -> Self {
            self.timeout = timeout;
            self
        }
    "#;

    let ast_analysis = analyze_ast(source);
    assert_eq!(ast_analysis.return_type, Some(ReturnType::OwnedSelf));
    assert!(ast_analysis.body_pattern.is_builder_like());

    let func = create_function_from_source(source);
    assert!(is_builder_method_ast(&func, &Some(ast_analysis)));
}

#[test]
fn test_not_constructor_when_has_loops() {
    let source = r#"
        pub fn process_items() -> Self {
            let mut result = Self::new();
            for item in items {
                result.add(item);
            }
            result
        }
    "#;

    let ast_analysis = analyze_ast(source);
    assert!(ast_analysis.body_pattern.has_loop);

    let func = create_function_from_source(source);
    assert!(!is_constructor_ast(&func, &Some(ast_analysis)));
}
```

**Language-Specific Tests**:
```rust
#[test]
fn test_python_classmethod_constructor() {
    let source = r#"
        @classmethod
        def from_config(cls, config):
            return cls(config.timeout, config.retries)
    "#;

    let ast_analysis = analyze_python_ast(source);
    assert!(python_patterns::is_python_constructor(&ast_analysis));
}

#[test]
fn test_typescript_static_factory() {
    let source = r#"
        static createDefault(): Client {
            return new Client(30, 3);
        }
    "#;

    let ast_analysis = analyze_typescript_ast(source);
    assert!(typescript_patterns::is_typescript_constructor(&ast_analysis));
}
```

### Integration Tests

**Regression Test**:
```rust
#[test]
fn test_reduced_false_positive_rate() {
    let test_functions = load_test_suite(); // 1000 labeled functions

    let with_name_only = classify_with_name_detection(&test_functions);
    let with_ast = classify_with_ast_detection(&test_functions);

    let fp_rate_name = calculate_false_positive_rate(&with_name_only);
    let fp_rate_ast = calculate_false_positive_rate(&with_ast);

    // AST should reduce false positives significantly
    assert!(fp_rate_ast < fp_rate_name * 0.5, "AST should halve false positive rate");
    assert!(fp_rate_ast < 0.05, "Target <5% false positive rate");
}
```

### Performance Tests

```rust
#[test]
fn test_ast_detection_performance_overhead() {
    let large_codebase = load_large_codebase();

    let baseline = benchmark(|| classify_with_name_detection(&large_codebase));
    let with_ast = benchmark(|| classify_with_ast_detection(&large_codebase));

    let overhead = (with_ast.as_millis() - baseline.as_millis()) as f64
        / baseline.as_millis() as f64;

    // Should add < 2% overhead (AST already parsed for complexity)
    assert!(overhead < 0.02, "AST detection adds {}% overhead, target <2%", overhead * 100.0);
}
```

## Documentation Requirements

### Code Documentation

**Module Documentation**:
```rust
//! AST-based constructor detection
//!
//! This module enhances name-based constructor detection (Spec 117)
//! with AST analysis to catch non-standard patterns.
//!
//! # Detection Strategy
//!
//! 1. **Return Type**: Function returns `Self` (or `Result<Self>`)
//! 2. **Body Pattern**: Struct initialization or simple field assignments
//! 3. **Complexity**: Low cyclomatic (≤5), no loops, minimal branching
//!
//! # Examples Caught
//!
//! ```rust
//! // Non-standard name (missed by name-based)
//! pub fn create_default_client() -> Self {
//!     Self { timeout: Duration::from_secs(30) }
//! }
//!
//! // Builder method (different role)
//! pub fn set_timeout(mut self, timeout: Duration) -> Self {
//!     self.timeout = timeout;
//!     self
//! }
//! ```
//!
//! # Fallback
//!
//! If AST unavailable (syntax errors, unsupported language):
//! - Falls back to name-based detection (Spec 117)
//! - Graceful degradation, no failures
```

### User Documentation

**Update**: `book/src/classification-system.md`

```markdown
## Constructor Detection

Debtmap uses a two-tier approach to detect constructors:

### Tier 1: Name-Based Detection (Spec 117)

Fast heuristic matching common patterns:
- `new`, `default`, `from_*`, `with_*`, `create_*`
- Catches ~80-90% of constructors
- Low overhead, language-agnostic

### Tier 2: AST-Based Detection (Spec 122)

Deep analysis of function structure:
- Return type analysis (`-> Self`)
- Body pattern recognition (struct initialization)
- Builder pattern detection
- Catches remaining ~10-20% of constructors

### Examples

**Standard Constructor** (caught by both):
```rust
pub fn new() -> Self { /* ... */ }
```

**Non-Standard Constructor** (AST-based only):
```rust
pub fn create_default_client() -> Self {
    Self { timeout: Duration::from_secs(30) }
}
```

**Builder Method** (AST-based, different classification):
```rust
pub fn set_timeout(mut self, timeout: Duration) -> Self {
    self.timeout = timeout;
    self
}
```

### Configuration

AST-based detection is **enabled by default** for Rust code.

```toml
[classification.constructors]
# AST detection is on by default (Phase 1: Rust only)
# No configuration needed - just works!

# To disable and use only name-based detection:
# ast_detection = false
```
```

## Implementation Notes

### syn AST Patterns (Phase 1: Rust)

**Return Type Detection**:
```rust
// Match on syn::ReturnType to find -> Self patterns
match &func.sig.output {
    syn::ReturnType::Type(_, ty) => {
        // Check if type is Self, Result<Self>, or Option<Self>
    }
    _ => None,
}
```

**Body Pattern Detection**:
```rust
// Use syn::visit::Visit to traverse function body
impl<'ast> Visit<'ast> for ConstructorPatternVisitor {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            Expr::Struct(_) => /* Found struct initialization */,
            Expr::Path(path) if is_self_path(path) => /* Found Self reference */,
            // ... detect loops, if statements, etc.
        }
    }
}
```

### Edge Cases

**Macro-Generated Constructors**:
```rust
#[derive(Default)]
struct Client { /* ... */ }

// Default::default() is a constructor but not in source
// Solution: Detect in macro expansion analysis
```

**Complex Validation**:
```rust
pub fn new(config: Config) -> Result<Self, Error> {
    validate_config(&config)?;  // 1 early return
    complex_parsing()?;          // 2 early returns
    Ok(Self { /* ... */ })       // 3 early returns
}

// Still a constructor despite complexity
// Solution: Allow higher cyclomatic (≤5) for Result-returning
```

## Migration and Compatibility

### Breaking Changes

**Phase 1**: None - Pure enhancement

- AST detection is **enabled by default** for better accuracy
- No schema changes (no new fields in `FunctionMetrics`)
- Constructors still map to `FunctionRole::IOWrapper` (existing behavior)
- Falls back to name-based detection when AST analysis fails
- **User-visible change**: Some functions may be reclassified (this is the goal - better accuracy)

### Backward Compatibility

**Backward compatible with caveats**:
- Existing configurations work without modification
- **Default behavior**: AST detection **enabled** for Rust code
- Graceful fallback to `is_simple_constructor()` if AST fails
- No changes to JSON output format (schema-compatible)
- No changes to scoring multipliers (IOWrapper still 0.7x)
- **Classification changes expected**: Better constructor detection means some functions reclassified (improved accuracy)

### Configuration Migration

**Existing config** (`.debtmap.toml`):
```toml
[classification.constructors]
patterns = ["new", "default", "from_", "with_", "create_"]
max_cyclomatic = 2
max_cognitive = 3
max_length = 15
max_nesting = 1
```

**New config** (default behavior):
```toml
[classification.constructors]
# Existing fields (unchanged)
patterns = ["new", "default", "from_", "with_", "create_"]
max_cyclomatic = 2
max_cognitive = 3
max_length = 15
max_nesting = 1

# NEW: AST-based detection (enabled by default)
# ast_detection = true  # Default: true (Phase 1: Rust only)
```

**To disable AST detection** (use only name-based):
```toml
[classification.constructors]
ast_detection = false  # Revert to name-only detection
```

**Implementation in `src/config.rs`**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructorDetectionConfig {
    // Existing fields...
    pub patterns: Vec<String>,
    pub max_cyclomatic: u32,
    pub max_cognitive: u32,
    pub max_length: usize,
    pub max_nesting: u32,

    // NEW: AST detection flag (enabled by default)
    #[serde(default = "default_ast_detection")]
    pub ast_detection: bool,
}

fn default_ast_detection() -> bool {
    true  // Enabled by default
}
```

## Success Metrics

### Quantitative Metrics (Phase 1: Rust only)

- **False Positive Reduction**: 30-50% reduction vs name-only (from ~20% to <10-14%)
- **Coverage**: Catches 90-95% of Rust constructors (vs ~80-85% name-only)
- **Performance**: < 5% overhead (measured via benchmarks)
- **Language Support**: Rust only (Python/TypeScript in Phase 2)

### Qualitative Metrics

- **User Satisfaction**: Fewer false positives for non-standard constructor names
- **Accuracy**: More precise classification for Rust code
- **Maintenance**: Clean separation of concerns (dedicated module)

### Validation Approach

**Baseline Measurement**:
1. Run debtmap on Rust codebase (e.g., debtmap itself)
2. Manually label 100 functions as constructor/non-constructor
3. Measure precision/recall with name-based detection

**Post-Implementation**:
1. Enable AST detection
2. Measure precision/recall on same labeled dataset
3. Compare false positive rates

**Performance Benchmarking**:
```bash
# Baseline: Name-only detection
cargo bench --bench constructor_detection -- --baseline name_only

# With AST: AST-based detection
cargo bench --bench constructor_detection -- --baseline ast_enabled
```

## Error Handling

### Graceful Degradation

**AST Parsing Failures**:
```rust
fn is_constructor_enhanced(
    func: &FunctionMetrics,
    syn_func: Option<&syn::ItemFn>,
) -> bool {
    // If syn_func is None (parsing failed), fall back to name-based
    let Some(syn_func) = syn_func else {
        return is_simple_constructor(func);
    };

    // If AST analysis fails for any reason, fall back
    let Ok(return_type) = extract_return_type(syn_func) else {
        return is_simple_constructor(func);
    };

    // Continue with AST analysis...
}
```

**Configuration Errors**:
- Invalid `ast_detection` value: Default to `true` (enabled by default)
- Missing configuration: Use existing `ConstructorDetectionConfig::default()` (AST enabled)

**Edge Cases**:
- Macro-expanded code: AST may not represent source accurately → fallback
- Generic return types: `-> T` where `T` may resolve to `Self` → not detected (acceptable)
- Complex type aliases: `type Ret = Self; -> Ret` → not detected (Phase 2 enhancement)

## Future Enhancements

### Phase 2: Multi-Language Support (Separate Specs)
- Python `@classmethod` detection using `rustpython_parser`
- TypeScript/JavaScript static factories
- Go struct initialization patterns
- Target: <5% false positive rate across all languages

### Phase 3: Builder Pattern Detection (Separate Spec)
- Add `FunctionRole::Builder` enum variant (breaking change)
- Detect builder pattern methods (`set_*()`, `with_*()` returning `Self`)
- Different multiplier (0.6x vs constructor 0.5x)
- Requires schema versioning and migration path

### Phase 4: Advanced Patterns
- Macro-generated constructors (via macro expansion analysis)
- Generic constructor detection (`-> impl Trait`)
- Builder pattern chains (`builder().field(x).field(y).build()`)

### Phase 5: IDE Integration
- LSP extension for real-time constructor classification
- Inline hints showing detected role
- Quick-fix suggestions for complex constructors
