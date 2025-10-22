---
number: 125
title: Accessor and Getter Method Detection
category: foundation
priority: high
status: draft
dependencies: [117, 124]
created: 2025-10-21
---

# Specification 125: Accessor and Getter Method Detection

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Specs 117 (Constructor Detection), 124 (Enum Converter Detection)

## Context

Debtmap currently flags simple accessor and getter methods as business logic requiring extensive testing. These methods are thin wrappers around field access or simple data transformations, not complex business logic deserving CRITICAL priority.

**Common False Positive Patterns**:

```rust
// Pattern 1: Field accessor
impl User {
    pub fn id(&self) -> UserId {
        self.id  // Just returns a field
    }

    pub fn email(&self) -> &str {
        &self.email
    }
}

// Pattern 2: Data transformer
impl Status {
    pub fn is_active(&self) -> bool {
        matches!(self, Status::Active)
    }

    pub fn kind(&self) -> &str {
        match self {
            Status::Active => "active",
            Status::Inactive => "inactive",
        }
    }
}

// Pattern 3: Standard trait implementations
impl Config {
    pub fn default() -> Self {
        Self { timeout: 30 }
    }

    pub fn clone(&self) -> Self {
        Self { timeout: self.timeout }
    }
}
```

**Why These are False Positives**:
- Accessors are infrastructure code, not business logic
- They have minimal complexity (typically cyclomatic ≤ 2, cognitive ≤ 1)
- Testing is low-value compared to actual business logic
- Similar to getters/setters in OOP - infrastructure for encapsulation
- Often auto-generated or trivial implementations

**Current Impact**:
- Priority reports cluttered with trivial accessors
- Users must manually filter out simple getters
- Actual complex business logic harder to find
- Coverage metrics skewed by untested accessors

**Examples from Real Codebases**:
```rust
// Common in Rust: getter methods for private fields
pub struct Point {
    x: f64,
    y: f64,
}

impl Point {
    pub fn x(&self) -> f64 { self.x }  // Should be IOWrapper
    pub fn y(&self) -> f64 { self.y }  // Should be IOWrapper

    // This IS business logic (should stay PureLogic)
    pub fn distance_to(&self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}
```

## Objective

Implement name-based and complexity-based detection to identify simple accessor and getter methods, classifying them as `IOWrapper` instead of `PureLogic` to reduce their priority score by 30%.

## Requirements

### Functional Requirements

1. **Name Pattern Detection**
   - Recognize common accessor names: `id()`, `name()`, `value()`, `kind()`, `type()`, `status()`
   - Detect getter prefixes: `get_*`, `is_*`, `has_*`, `can_*`
   - Identify converter methods: `as_*`, `to_*`, `into_*`
   - Handle ownership patterns: `&self` methods only (no `&mut self`)

2. **Complexity-based Filtering**
   - Require cyclomatic complexity ≤ 2 (simple linear code or single branch)
   - Require cognitive complexity ≤ 1 (minimal mental overhead)
   - Require function length < 10 lines
   - Require nesting depth ≤ 1

3. **Return Type Analysis** (name-based heuristics)
   - Functions returning references likely accessors: `-> &T`, `-> &str`, `-> &[T]`
   - Functions returning primitives likely accessors: `-> bool`, `-> i32`, `-> usize`
   - Functions returning owned simple types: `-> String`, `-> Vec<T>` (if simple enough)

4. **Body Pattern Analysis** (when AST available)
   - Single field access: `self.field`
   - Simple reference: `&self.field`
   - Simple match/if returning literals or fields
   - Method chaining: `self.field.clone()`, `self.field.to_string()`

5. **Exclusions** (NOT accessors)
   - Functions with `&mut self` (modifiers, not accessors)
   - Functions with multiple statements and side effects
   - Functions calling external APIs or I/O
   - Functions with loops or complex control flow

### Non-Functional Requirements

- Detection must be extremely fast (< 0.1ms per function) since name-based
- Should work without AST (fallback to name + complexity only)
- Zero false positives for complex business logic
- Acceptable false negative rate (better to miss an accessor than misclassify business logic)

## Acceptance Criteria

- [ ] **Detection Module**: Update `src/priority/semantic_classifier.rs` with:
  - `is_accessor_method(func: &FunctionMetrics) -> bool`
  - `matches_accessor_name(name: &str) -> bool`
  - `is_simple_accessor_body(syn_func: &syn::ItemFn) -> bool` (optional AST check)

- [ ] **Name Pattern Matching**:
  - [ ] Detects single-word accessors: `id`, `name`, `value`, `kind`, `status`, `type`
  - [ ] Detects getter prefixes: `get_*`, `is_*`, `has_*`, `can_*`, `should_*`
  - [ ] Detects converter methods: `as_*`, `to_*`, `into_*`
  - [ ] Case-insensitive matching (handle `ID()`, `getName()`, etc.)

- [ ] **Complexity Filtering**:
  - [ ] Rejects if cyclomatic > 2
  - [ ] Rejects if cognitive > 1
  - [ ] Rejects if length ≥ 10
  - [ ] Rejects if nesting > 1

- [ ] **AST Body Analysis** (when available):
  - [ ] Detects direct field access: `self.field`
  - [ ] Detects reference to field: `&self.field`
  - [ ] Detects simple method call: `self.field.clone()`
  - [ ] Rejects if multiple statements with side effects

- [ ] **Integration**:
  - [ ] Add accessor check after enum converter detection
  - [ ] Before pattern matching detection
  - [ ] Return `FunctionRole::IOWrapper` for detected accessors

- [ ] **Configuration**:
  - [ ] Add `AccessorDetectionConfig` to `config.rs`
  - [ ] Configurable name patterns (default: id, name, value, get_*, is_*, etc.)
  - [ ] Configurable complexity thresholds
  - [ ] Enable/disable flag (default: true)

- [ ] **Testing**:
  - [ ] Test case: `User::id()` returning field classified as IOWrapper
  - [ ] Test case: `Status::is_active()` classified as IOWrapper
  - [ ] Test case: `Config::get_timeout()` classified as IOWrapper
  - [ ] Test case: Complex method with accessor name NOT detected
  - [ ] Test case: Method with side effects NOT detected
  - [ ] Regression: Business logic methods still classified correctly

- [ ] **Impact Validation**:
  - [ ] Simple accessors no longer in top 20 CRITICAL items
  - [ ] Top recommendations focus on actual business logic
  - [ ] No complex business logic misclassified as accessor

## Technical Details

### Implementation Approach

**Module Structure**:
```rust
// src/priority/semantic_classifier.rs

/// Detect simple accessor/getter methods
fn is_accessor_method(func: &FunctionMetrics, syn_func: Option<&syn::ItemFn>) -> bool {
    let config = crate::config::get_accessor_detection_config();

    // Check name matches accessor pattern
    if !matches_accessor_name(&func.name, &config) {
        return false;
    }

    // Check complexity is minimal
    if func.cyclomatic > config.max_cyclomatic
        || func.cognitive > config.max_cognitive
        || func.length >= config.max_length
        || func.nesting > config.max_nesting
    {
        return false;
    }

    // If AST available, verify body is simple
    if let Some(syn_func) = syn_func {
        if !is_simple_accessor_body(syn_func) {
            return false;
        }
    }

    true
}

/// Check if name matches accessor patterns
fn matches_accessor_name(name: &str, config: &AccessorDetectionConfig) -> bool {
    let name_lower = name.to_lowercase();

    // Single-word accessors
    if config.single_word_patterns.iter().any(|p| name_lower == *p) {
        return true;
    }

    // Prefix patterns
    if config.prefix_patterns.iter().any(|p| name_lower.starts_with(p)) {
        return true;
    }

    false
}

/// Check if function body is simple accessor pattern (AST analysis)
fn is_simple_accessor_body(syn_func: &syn::ItemFn) -> bool {
    // Function should take &self (not &mut self)
    if !has_immutable_self_receiver(syn_func) {
        return false;
    }

    // Single statement or expression
    let stmts = &syn_func.block.stmts;
    if stmts.is_empty() {
        return false;
    }

    // Check for simple patterns
    match stmts.len() {
        1 => {
            // Single expression: self.field, &self.field, self.field.clone()
            match &stmts[0] {
                syn::Stmt::Expr(expr, _) => is_simple_accessor_expr(expr),
                _ => false,
            }
        }
        2 => {
            // Let binding + return: let x = self.field; x
            // This is acceptable for accessors
            is_simple_binding_pattern(stmts)
        }
        _ => false, // Multiple statements - too complex
    }
}

/// Check if expression is simple accessor pattern
fn is_simple_accessor_expr(expr: &syn::Expr) -> bool {
    match expr {
        // Direct field access: self.field
        syn::Expr::Field(field_expr) => {
            matches!(&*field_expr.base, syn::Expr::Path(path)
                if path.path.is_ident("self"))
        }

        // Reference to field: &self.field
        syn::Expr::Reference(ref_expr) => {
            is_simple_accessor_expr(&ref_expr.expr)
        }

        // Method call on field: self.field.clone()
        syn::Expr::MethodCall(method_call) => {
            // Must be called on self.field
            is_simple_accessor_expr(&method_call.receiver)
                // Common accessor methods
                && is_simple_accessor_method(&method_call.method)
        }

        // Simple match or if (for bool accessors)
        syn::Expr::Match(_) | syn::Expr::If(_) => {
            // Already validated by complexity metrics
            // If cognitive ≤ 1, it's simple enough
            true
        }

        _ => false,
    }
}

fn is_simple_accessor_method(method: &syn::Ident) -> bool {
    matches!(
        method.to_string().as_str(),
        "clone" | "to_string" | "as_ref" | "as_str" | "as_bytes" | "copied"
    )
}
```

### Architecture Changes

**Classification Pipeline Update**:
```rust
fn classify_by_rules(
    func: &FunctionMetrics,
    func_id: &FunctionId,
    call_graph: &CallGraph,
    syn_func: Option<&syn::ItemFn>,
) -> Option<FunctionRole> {
    if is_entry_point(func_id, call_graph) {
        return Some(FunctionRole::EntryPoint);
    }

    if is_constructor_enhanced(func, syn_func) {
        return Some(FunctionRole::IOWrapper);
    }

    if let Some(syn_func) = syn_func {
        if is_enum_converter_enhanced(func, syn_func) {
            return Some(FunctionRole::IOWrapper);
        }
    }

    // NEW: Check for accessor methods (Spec 125)
    if is_accessor_method(func, syn_func) {
        return Some(FunctionRole::IOWrapper);
    }

    if is_pattern_matching_function(func, func_id) {
        return Some(FunctionRole::PatternMatch);
    }

    // ... rest of classification
}
```

### Data Structures

```rust
// src/config.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessorDetectionConfig {
    /// Enable accessor method detection
    pub enabled: bool,

    /// Single-word accessor names
    pub single_word_patterns: Vec<String>,

    /// Prefix patterns for accessors
    pub prefix_patterns: Vec<String>,

    /// Maximum cyclomatic complexity
    pub max_cyclomatic: u32,

    /// Maximum cognitive complexity
    pub max_cognitive: u32,

    /// Maximum function length
    pub max_length: usize,

    /// Maximum nesting depth
    pub max_nesting: u32,
}

impl Default for AccessorDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            single_word_patterns: vec![
                "id".to_string(),
                "name".to_string(),
                "value".to_string(),
                "kind".to_string(),
                "type".to_string(),
                "status".to_string(),
                "code".to_string(),
                "key".to_string(),
                "index".to_string(),
            ],
            prefix_patterns: vec![
                "get_".to_string(),
                "is_".to_string(),
                "has_".to_string(),
                "can_".to_string(),
                "should_".to_string(),
                "as_".to_string(),
                "to_".to_string(),
                "into_".to_string(),
            ],
            max_cyclomatic: 2,
            max_cognitive: 1,
            max_length: 10,
            max_nesting: 1,
        }
    }
}
```

## Dependencies

- **Prerequisites**:
  - Spec 117: Constructor Detection (classification framework)
  - Spec 124: Enum Converter Detection (similar pattern, different use case)

- **Affected Components**:
  - `src/priority/semantic_classifier.rs`: Add accessor detection
  - `src/config.rs`: Add configuration struct

- **External Dependencies**:
  - `syn` crate (optional, for AST analysis)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_field_accessor_detected() {
        let metrics = create_test_metrics("id", 1, 0, 3);
        assert!(is_accessor_method(&metrics, None));

        let metrics = create_test_metrics("get_name", 1, 0, 5);
        assert!(is_accessor_method(&metrics, None));
    }

    #[test]
    fn test_bool_accessor_detected() {
        let metrics = create_test_metrics("is_active", 2, 1, 8);
        assert!(is_accessor_method(&metrics, None));

        let metrics = create_test_metrics("has_permission", 2, 0, 5);
        assert!(is_accessor_method(&metrics, None));
    }

    #[test]
    fn test_converter_method_detected() {
        let metrics = create_test_metrics("as_str", 1, 0, 3);
        assert!(is_accessor_method(&metrics, None));

        let metrics = create_test_metrics("to_string", 1, 0, 4);
        assert!(is_accessor_method(&metrics, None));
    }

    #[test]
    fn test_complex_method_not_detected() {
        // High complexity despite accessor name
        let metrics = create_test_metrics("get_value", 5, 3, 20);
        assert!(!is_accessor_method(&metrics, None));
    }

    #[test]
    fn test_business_logic_not_misclassified() {
        // Business logic method
        let metrics = create_test_metrics("calculate_total", 4, 2, 15);
        assert!(!is_accessor_method(&metrics, None));
    }

    #[test]
    fn test_ast_body_validation() {
        let code = r#"
            pub fn id(&self) -> u32 {
                self.id
            }
        "#;

        let syn_func = parse_function(code);
        assert!(is_simple_accessor_body(&syn_func));
    }

    #[test]
    fn test_side_effect_rejected() {
        let code = r#"
            pub fn get_value(&self) -> i32 {
                self.counter.fetch_add(1, Ordering::SeqCst);
                self.value
            }
        "#;

        let syn_func = parse_function(code);
        assert!(!is_simple_accessor_body(&syn_func));
    }
}
```

### Integration Tests

```rust
#[test]
fn test_accessor_classification_integration() {
    let analysis = analyze_project(Path::new("tests/fixtures/accessor_examples"));

    // Simple accessors should be IOWrapper
    let id_method = find_function(&analysis, "User::id");
    assert_eq!(id_method.role, FunctionRole::IOWrapper);

    let is_active = find_function(&analysis, "Status::is_active");
    assert_eq!(is_active.role, FunctionRole::IOWrapper);

    // Complex methods should still be PureLogic
    let calculate = find_function(&analysis, "Calculator::calculate_total");
    assert_eq!(calculate.role, FunctionRole::PureLogic);
}
```

## Documentation Requirements

### Code Documentation

- Document `is_accessor_method()` with examples
- Explain name pattern matching logic
- Document AST body analysis patterns

### User Documentation

- Update debtmap book with accessor detection examples
- Explain why accessors are classified as IOWrapper
- Provide guidance on configuring accessor patterns

### Architecture Updates

- Update classification pipeline diagram
- Document decision criteria for accessor vs business logic
- Add examples to `ARCHITECTURE.md`

## Implementation Notes

### Conservative Approach

This implementation is intentionally conservative to avoid false positives:

1. **Strict complexity limits**: Cyclomatic ≤ 2, Cognitive ≤ 1
2. **Name pattern required**: Must match known accessor patterns
3. **AST validation**: When available, verify body is simple
4. **Better false negative than false positive**: OK to miss some accessors

### Edge Cases

1. **Accessors with side effects**: Reject (e.g., lazy initialization)
2. **Accessors with logging**: Reject (multiple statements)
3. **Accessors with validation**: Depends on complexity (might be PureLogic)
4. **Computed properties**: If simple (≤2 cyclomatic), accept; otherwise reject

### Performance

- Name matching is extremely fast (string prefix check)
- Complexity check already computed
- AST analysis is optional enhancement
- Total overhead: < 0.1ms per function

## Migration and Compatibility

### Breaking Changes

None - additive functionality only.

### Configuration Migration

Add to existing classification config:

```rust
pub struct ClassificationConfig {
    pub constructors: Option<ConstructorDetectionConfig>,
    pub enum_converters: Option<EnumConverterDetectionConfig>,
    pub accessors: Option<AccessorDetectionConfig>, // NEW
}
```

### Backward Compatibility

- Works without AST (name + complexity only)
- Falls back gracefully if AST unavailable
- Existing classifications unchanged for non-accessors

## Success Metrics

**Before Implementation**:
- Simple accessors flagged as business logic
- Users manually filter accessor methods
- Coverage metrics skewed by untested getters

**After Implementation**:
- Accessors classified as IOWrapper (0.7x multiplier)
- Top 20 recommendations focus on business logic
- Users can opt-in to testing accessors if desired
- Zero complex business logic misclassified

## Examples

### Detected as Accessors (IOWrapper)

```rust
pub fn id(&self) -> UserId { self.id }
pub fn name(&self) -> &str { &self.name }
pub fn is_active(&self) -> bool { matches!(self.status, Status::Active) }
pub fn get_timeout(&self) -> u32 { self.timeout }
pub fn as_str(&self) -> &str { &self.value }
pub fn to_string(&self) -> String { self.value.clone() }
```

### NOT Detected (PureLogic)

```rust
pub fn calculate_total(&self) -> f64 {
    self.items.iter().map(|i| i.price).sum()
}

pub fn validate(&self) -> Result<()> {
    if self.value < 0 { Err(...) } else { Ok(()) }
}

pub fn process(&mut self) -> Result<Output> {
    self.state.update();
    self.generate_output()
}
```

## References

- Spec 117: Constructor Detection and Classification
- Spec 124: Enum Converter Detection
- Related: Rust API guidelines on getters and setters
