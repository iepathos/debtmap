---
number: 209
title: Accessor and Boilerplate Method Detection
category: optimization
priority: medium
status: draft
dependencies: [206]
created: 2025-12-15
---

# Specification 209: Accessor and Boilerplate Method Detection

**Category**: optimization
**Priority**: medium (P1)
**Status**: draft
**Dependencies**: Spec 206 (Cohesion Gate)

## Context

The current God Object detection counts all methods equally when determining if a struct is a God Object. Simple accessor methods (getters/setters) and boilerplate methods (new, default, clone) inflate the method count, causing false positives.

### Current Problem

```rust
pub struct ModuleTracker {
    modules: Vec<Module>,
    calls: Vec<Call>,
    imports: Vec<Import>,
    exports: Vec<Export>,
}

impl ModuleTracker {
    // These 8 accessors should NOT contribute to God Object score
    pub fn get_modules(&self) -> &[Module] { &self.modules }
    pub fn get_calls(&self) -> &[Call] { &self.calls }
    pub fn get_imports(&self) -> &[Import] { &self.imports }
    pub fn get_exports(&self) -> &[Export] { &self.exports }
    pub fn modules_mut(&mut self) -> &mut Vec<Module> { &mut self.modules }
    pub fn calls_mut(&mut self) -> &mut Vec<Call> { &mut self.calls }
    pub fn imports_mut(&mut self) -> &mut Vec<Import> { &mut self.imports }
    pub fn exports_mut(&mut self) -> &mut Vec<Export> { &mut self.exports }

    // These 4 business methods SHOULD contribute
    pub fn analyze_workspace(&mut self, files: &[File]) -> Result<()> { ... }
    pub fn resolve_call(&self, path: &str) -> Option<Function> { ... }
    pub fn build_dependency_graph(&self) -> Graph { ... }
    pub fn detect_cycles(&self) -> Vec<Cycle> { ... }
}

// Current: method_count = 12 (exceeds threshold of 15 easily with a few more)
// Desired: substantive_method_count = 4 (much more accurate)
```

## Objective

Implement detection and weighting of accessor/boilerplate methods to:
1. Identify trivial methods that don't contribute to complexity
2. Apply reduced weight to these methods in God Object scoring
3. Provide a more accurate "substantive method count" metric

## Requirements

### Functional Requirements

1. **Method Classification**: Classify methods into complexity categories:
   - `TrivialAccessor`: Single-line getter/setter returning field directly
   - `SimpleAccessor`: Getter/setter with minor transformation
   - `Boilerplate`: Constructor (new), Default impl, Clone, From/Into
   - `Delegating`: Method that simply calls another method
   - `Substantive`: Method with actual business logic

2. **Weighted Counting**: Apply weights to method counts:
   - TrivialAccessor: 0.1 weight
   - SimpleAccessor: 0.3 weight
   - Boilerplate: 0.0 weight (don't count at all)
   - Delegating: 0.5 weight
   - Substantive: 1.0 weight

3. **Scoring Integration**: Use weighted method count in God Object scoring formula

### Non-Functional Requirements

- Classification must be fast (minimal AST traversal overhead)
- Must produce deterministic results
- Should handle edge cases gracefully (default to Substantive if uncertain)

## Acceptance Criteria

- [ ] Methods named `get_*`, `set_*`, `*_mut` returning field references are classified as accessors
- [ ] Methods named `new`, `default`, `clone`, `from`, `into` are classified as boilerplate
- [ ] One-line methods with single return statement are detected as trivial
- [ ] Weighted method count is used in God Object score calculation
- [ ] A struct with 10 accessors + 5 business methods scores like 5-6 methods, not 15
- [ ] Existing tests continue to pass
- [ ] New tests validate classification accuracy

## Technical Details

### Implementation Approach

#### 1. Method Complexity Classification

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum MethodComplexityClass {
    TrivialAccessor,   // Single-line field return: fn get_x(&self) -> &X { &self.x }
    SimpleAccessor,    // Minor transformation: fn get_x(&self) -> X { self.x.clone() }
    Boilerplate,       // new, default, clone, from, into
    Delegating,        // fn foo(&self) { self.inner.foo() }
    Substantive,       // Actual business logic
}

impl MethodComplexityClass {
    pub fn weight(&self) -> f64 {
        match self {
            Self::TrivialAccessor => 0.1,
            Self::SimpleAccessor => 0.3,
            Self::Boilerplate => 0.0,
            Self::Delegating => 0.5,
            Self::Substantive => 1.0,
        }
    }
}
```

#### 2. Classification Function

```rust
pub fn classify_method_complexity(
    method_name: &str,
    body_line_count: usize,
    has_control_flow: bool,
    call_count: usize,
    return_expr_type: Option<ReturnExprType>,
) -> MethodComplexityClass {
    let name_lower = method_name.to_lowercase();

    // Boilerplate detection by name
    if matches!(name_lower.as_str(), "new" | "default" | "clone" | "from" | "into") {
        return MethodComplexityClass::Boilerplate;
    }

    // Accessor patterns
    let is_accessor_name = name_lower.starts_with("get_")
        || name_lower.starts_with("set_")
        || name_lower.ends_with("_mut")
        || name_lower.starts_with("is_")
        || name_lower.starts_with("has_");

    // Trivial accessor: single line, no control flow, returns field reference
    if is_accessor_name && body_line_count <= 1 && !has_control_flow {
        if let Some(ReturnExprType::FieldAccess) = return_expr_type {
            return MethodComplexityClass::TrivialAccessor;
        }
    }

    // Simple accessor: short accessor with minor work
    if is_accessor_name && body_line_count <= 3 && !has_control_flow {
        return MethodComplexityClass::SimpleAccessor;
    }

    // Delegating: short method that calls one other method
    if body_line_count <= 2 && call_count == 1 && !has_control_flow {
        return MethodComplexityClass::Delegating;
    }

    // Default to substantive
    MethodComplexityClass::Substantive
}

#[derive(Debug, Clone)]
pub enum ReturnExprType {
    FieldAccess,    // &self.field or self.field
    MethodCall,     // self.other_method() or self.field.method()
    Literal,        // true, false, 0, etc.
    Complex,        // Anything else
}
```

#### 3. AST Analysis for Classification

```rust
// In ast_visitor.rs, enhance method analysis
pub struct MethodAnalysis {
    pub name: String,
    pub body_line_count: usize,
    pub has_control_flow: bool,  // if, match, loop, while, for
    pub call_count: usize,
    pub return_expr_type: Option<ReturnExprType>,
    pub complexity_class: MethodComplexityClass,
}

impl<'ast> Visit<'ast> for TypeVisitor {
    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        let analysis = analyze_method_body(&item.block);
        let complexity = classify_method_complexity(
            &item.sig.ident.to_string(),
            analysis.line_count,
            analysis.has_control_flow,
            analysis.call_count,
            analysis.return_expr_type,
        );

        // Store analysis...
    }
}

fn analyze_method_body(block: &syn::Block) -> BodyAnalysis {
    let mut analysis = BodyAnalysis::default();

    // Count lines (excluding empty and comment-only)
    analysis.line_count = count_substantive_lines(block);

    // Check for control flow
    analysis.has_control_flow = has_control_flow_statements(block);

    // Count method calls
    analysis.call_count = count_method_calls(block);

    // Analyze return expression type
    analysis.return_expr_type = classify_return_expr(block);

    analysis
}
```

#### 4. Integration with Scoring

```rust
// In detector.rs
fn analyze_single_struct(...) -> Option<GodObjectAnalysis> {
    // ... existing code ...

    // NEW: Calculate weighted method count
    let method_analyses: Vec<MethodAnalysis> = type_analysis
        .methods
        .iter()
        .map(|m| get_method_analysis(m))
        .collect();

    let weighted_method_count: f64 = method_analyses
        .iter()
        .map(|m| m.complexity_class.weight())
        .sum();

    // Use weighted count in scoring
    let god_object_score = calculate_god_object_score_weighted(
        weighted_method_count,  // Changed from method_count
        field_count,
        responsibility_count,
        lines_of_code,
        avg_complexity,
        thresholds,
    );

    // ... rest of analysis ...
}
```

### Detection Heuristics

| Pattern | Classification | Example |
|---------|---------------|---------|
| `get_*` returning `&self.field` | TrivialAccessor | `fn get_name(&self) -> &str { &self.name }` |
| `set_*` assigning to field | TrivialAccessor | `fn set_name(&mut self, n: String) { self.name = n; }` |
| `is_*` returning boolean field | TrivialAccessor | `fn is_enabled(&self) -> bool { self.enabled }` |
| `*_mut` returning `&mut self.field` | TrivialAccessor | `fn data_mut(&mut self) -> &mut Data { &mut self.data }` |
| Accessor with `.clone()` | SimpleAccessor | `fn get_name(&self) -> String { self.name.clone() }` |
| `new()` constructor | Boilerplate | `fn new() -> Self { Self::default() }` |
| `Default::default()` impl | Boilerplate | `fn default() -> Self { Self { ... } }` |
| Single method call | Delegating | `fn process(&self) { self.inner.process() }` |
| Complex logic | Substantive | Multi-line with control flow |

## Dependencies

- **Prerequisites**:
  - Spec 206: Cohesion Gate (shares method analysis infrastructure)
- **Affected Components**:
  - `ast_visitor.rs`: Add method body analysis
  - `detector.rs`: Use weighted method count
  - `scoring.rs`: Accept weighted count parameter
  - `classifier.rs`: New classification types and functions

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_classify_trivial_accessor() {
    let class = classify_method_complexity(
        "get_name",
        1,     // single line
        false, // no control flow
        0,     // no calls
        Some(ReturnExprType::FieldAccess),
    );
    assert_eq!(class, MethodComplexityClass::TrivialAccessor);
}

#[test]
fn test_classify_boilerplate() {
    assert_eq!(
        classify_method_complexity("new", 3, false, 1, None),
        MethodComplexityClass::Boilerplate
    );
    assert_eq!(
        classify_method_complexity("default", 5, false, 0, None),
        MethodComplexityClass::Boilerplate
    );
}

#[test]
fn test_classify_substantive() {
    let class = classify_method_complexity(
        "analyze_workspace",
        25,    // multi-line
        true,  // has control flow
        5,     // multiple calls
        None,
    );
    assert_eq!(class, MethodComplexityClass::Substantive);
}

#[test]
fn test_weighted_count() {
    let methods = vec![
        MethodAnalysis { complexity_class: MethodComplexityClass::TrivialAccessor, .. },
        MethodAnalysis { complexity_class: MethodComplexityClass::TrivialAccessor, .. },
        MethodAnalysis { complexity_class: MethodComplexityClass::Boilerplate, .. },
        MethodAnalysis { complexity_class: MethodComplexityClass::Substantive, .. },
    ];

    let weighted: f64 = methods.iter().map(|m| m.complexity_class.weight()).sum();
    // 0.1 + 0.1 + 0.0 + 1.0 = 1.2
    assert!((weighted - 1.2).abs() < 0.01);
}
```

### Integration Tests

```rust
#[test]
fn test_struct_with_many_accessors_not_flagged() {
    let content = r#"
        pub struct DataContainer {
            a: i32, b: i32, c: i32, d: i32, e: i32,
        }

        impl DataContainer {
            pub fn new() -> Self { Self { a: 0, b: 0, c: 0, d: 0, e: 0 } }
            pub fn get_a(&self) -> i32 { self.a }
            pub fn get_b(&self) -> i32 { self.b }
            pub fn get_c(&self) -> i32 { self.c }
            pub fn get_d(&self) -> i32 { self.d }
            pub fn get_e(&self) -> i32 { self.e }
            pub fn set_a(&mut self, v: i32) { self.a = v; }
            pub fn set_b(&mut self, v: i32) { self.b = v; }
            pub fn set_c(&mut self, v: i32) { self.c = v; }
            pub fn set_d(&mut self, v: i32) { self.d = v; }
            pub fn set_e(&mut self, v: i32) { self.e = v; }
            // 1 new + 10 accessors = 11 methods raw, but ~2.0 weighted
        }
    "#;

    let analyses = analyze_content(content);
    assert!(analyses.is_empty(), "11 accessors should not trigger God Object");
}
```

## Documentation Requirements

- **Code Documentation**: Document classification heuristics in classifier.rs
- **User Documentation**: Explain how method weighting affects detection

## Implementation Notes

1. **Conservative Classification**: Default to Substantive if uncertain
2. **Return Type Analysis**: Detecting `&self.field` requires AST inspection
3. **Performance**: Cache classification results per method
4. **Edge Cases**: Handle async methods, generic methods, trait implementations

## Migration and Compatibility

- Existing God Object scores will change (generally lower due to accessor weighting)
- Add `--raw-method-count` flag for backwards compatibility
- Consider gradual weight adjustment via configuration

## Estimated Effort

- Implementation: ~3 hours
- Testing: ~2 hours
- Documentation: ~0.5 hours
- Total: ~5.5 hours
