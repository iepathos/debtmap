---
number: 187
title: Extract Pure Functions from Analyzers
category: optimization
priority: low
status: draft
dependencies: []
created: 2025-11-30
---

# Specification 187: Extract Pure Functions from Analyzers

**Category**: optimization
**Priority**: low
**Status**: draft
**Dependencies**: None

## Context

The analyzer modules (`src/analyzers/*.rs`) contain several long methods (30+ lines) that mix calculation logic with I/O operations. According to STILLWATER_EVALUATION.md (lines 699-702) and the project's guideline of maximum 20 lines per function, these should be refactored into smaller, composable pure functions.

**Current Issues:**

1. **Long methods** - Many analyzer methods exceed 30 lines
2. **Mixed concerns** - Calculation logic mixed with I/O
3. **Poor testability** - Hard to test pure logic in isolation
4. **Code duplication** - Similar patterns across language analyzers
5. **Complexity** - High cyclomatic complexity in some methods

**Examples of Problem Functions:**

```rust
// src/analyzers/rust.rs
pub fn analyze_file(&mut self, path: &Path) -> Result<FileMetrics> {
    // 50+ lines mixing:
    // - File I/O (reading file)
    // - Parsing (AST generation)
    // - Metric calculation (pure)
    // - Error handling
    // - Result aggregation
}

// src/analyzers/python.rs
fn calculate_complexity(&self, node: &Node) -> u32 {
    // 40+ lines of nested conditionals
    // Pure logic but too complex
}
```

According to the Stillwater philosophy and project guidelines, we should:
1. **Extract pure calculation logic** into focused functions (5-20 lines)
2. **Keep I/O in thin wrapper functions**
3. **Make each function testable in isolation**
4. **Reduce cyclomatic complexity** to < 5 per function

## Objective

Systematically refactor analyzer modules to extract pure functions:

1. **Identify** all methods over 30 lines in analyzer modules
2. **Extract** pure calculation logic into separate functions (5-20 lines each)
3. **Separate** I/O operations into thin wrapper functions
4. **Ensure** each function has single responsibility
5. **Test** extracted functions independently
6. **Document** function purposes and compositions

Result: Cleaner, more testable, more maintainable analyzer code following functional programming principles.

## Requirements

### Functional Requirements

1. **Function Size Reduction**
   - All functions under 20 lines (target: 5-10 lines)
   - Break 30+ line methods into multiple functions
   - Each function has single responsibility
   - Clear, descriptive function names

2. **Pure Function Extraction**
   - Extract calculation logic into pure functions
   - No I/O in calculation functions
   - Deterministic results
   - Easy to unit test
   - Examples:
     - `calculate_cyclomatic_complexity(ast: &Ast) -> u32`
     - `count_function_parameters(func: &Function) -> usize`
     - `extract_function_calls(body: &Block) -> Vec<FunctionCall>`

3. **I/O Wrapper Functions**
   - Keep I/O in thin wrapper functions
   - Wrappers orchestrate pure functions
   - Clear separation between I/O and logic
   - Examples:
     - `read_and_parse_file(path: &Path) -> Result<Ast>`
     - `analyze_and_format(ast: &Ast) -> Result<FormattedOutput>`

4. **Preserve Functionality**
   - All existing analyzer behavior preserved
   - Same metrics calculated
   - Same error handling
   - Backward compatible API

### Non-Functional Requirements

1. **Testability**
   - Pure functions unit tested without mocks
   - Fast tests (no I/O)
   - High test coverage (aim for 90%+)
   - Clear test cases

2. **Maintainability**
   - Easy to understand function purposes
   - Clear composition of functions
   - Easy to modify individual functions
   - Consistent patterns across analyzers

3. **Performance**
   - No performance regression
   - Same or better speed
   - Efficient memory usage

## Acceptance Criteria

- [ ] All functions in analyzers/* under 20 lines
- [ ] Pure calculation functions extracted and tested
- [ ] I/O separated into wrapper functions
- [ ] Cyclomatic complexity < 5 for all functions
- [ ] Each function documented with clear purpose
- [ ] Unit tests for all pure functions
- [ ] All existing integration tests pass
- [ ] No clippy warnings
- [ ] Performance benchmarks show no regression
- [ ] Code review confirms improved readability

## Technical Details

### Implementation Approach

**Phase 1: Identify Long Functions**

```bash
# Find functions over 30 lines
rg -A 50 'fn \w+' src/analyzers/ --type rust | \
  awk '/^fn/{start=NR} NR==start+30{print file":"start" "line}' > long_functions.txt

# Manual review to categorize:
# - Pure logic (extract)
# - I/O operations (wrapper)
# - Mixed (needs splitting)
```

**Phase 2: Extract Pure Functions**

**Before (Long, Mixed Function):**

```rust
// src/analyzers/rust.rs (50+ lines)
pub fn analyze_file(&mut self, path: &Path) -> Result<FileMetrics> {
    // I/O: Read file
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    // I/O: Parse to AST
    let syntax = syn::parse_file(&content)
        .with_context(|| format!("Failed to parse Rust file: {}", path.display()))?;

    // Pure: Calculate complexity (but inline, hard to test)
    let mut total_complexity = 0;
    for item in &syntax.items {
        if let syn::Item::Fn(func) = item {
            // Nested complexity calculation (20+ lines)
            let mut complexity = 1;
            for stmt in &func.block.stmts {
                match stmt {
                    syn::Stmt::Expr(expr, _) => {
                        // More nested logic...
                        if let syn::Expr::If(_) = expr {
                            complexity += 1;
                        }
                        // ... many more patterns
                    }
                    _ => {}
                }
            }
            total_complexity += complexity;
        }
    }

    // Pure: Count functions (inline)
    let function_count = syntax.items.iter()
        .filter(|item| matches!(item, syn::Item::Fn(_)))
        .count();

    // Pure: Calculate lines (inline)
    let lines = content.lines().count();

    // Result aggregation
    Ok(FileMetrics {
        path: path.to_path_buf(),
        complexity: total_complexity,
        functions: function_count,
        lines,
    })
}
```

**After (Extracted, Pure Functions):**

```rust
// src/analyzers/rust.rs

// ============================================================================
// PURE FUNCTIONS (Calculation Logic)
// ============================================================================

/// Calculates total cyclomatic complexity for all functions in syntax tree.
///
/// Pure function - deterministic, no side effects.
fn calculate_total_complexity(syntax: &syn::File) -> u32 {
    syntax.items
        .iter()
        .filter_map(extract_function)
        .map(calculate_function_complexity)
        .sum()
}

/// Extracts function item if present.
fn extract_function(item: &syn::Item) -> Option<&syn::ItemFn> {
    match item {
        syn::Item::Fn(func) => Some(func),
        _ => None,
    }
}

/// Calculates cyclomatic complexity for a single function.
///
/// Pure function - operates on parsed AST data only.
fn calculate_function_complexity(func: &syn::ItemFn) -> u32 {
    let base_complexity = 1;
    let statement_complexity = count_complexity_contributors(&func.block);
    base_complexity + statement_complexity
}

/// Counts complexity-contributing statements in a block.
fn count_complexity_contributors(block: &syn::Block) -> u32 {
    block.stmts
        .iter()
        .map(count_statement_complexity)
        .sum()
}

/// Counts complexity contribution of a single statement.
fn count_statement_complexity(stmt: &syn::Stmt) -> u32 {
    match stmt {
        syn::Stmt::Expr(expr, _) => count_expression_complexity(expr),
        _ => 0,
    }
}

/// Counts complexity contribution of an expression.
fn count_expression_complexity(expr: &syn::Expr) -> u32 {
    match expr {
        syn::Expr::If(_) => 1,
        syn::Expr::While(_) => 1,
        syn::Expr::For(_) => 1,
        syn::Expr::Match(_) => 1,
        syn::Expr::Loop(_) => 1,
        _ => 0,
    }
}

/// Counts total number of functions in syntax tree.
fn count_functions(syntax: &syn::File) -> usize {
    syntax.items
        .iter()
        .filter(|item| matches!(item, syn::Item::Fn(_)))
        .count()
}

/// Counts lines in source code.
fn count_lines(content: &str) -> usize {
    content.lines().count()
}

// ============================================================================
// I/O WRAPPER FUNCTIONS
// ============================================================================

/// Reads and parses Rust file (I/O wrapper).
fn read_and_parse_rust_file(path: &Path) -> Result<(String, syn::File)> {
    let content = read_file_content(path)?;
    let syntax = parse_rust_content(&content, path)?;
    Ok((content, syntax))
}

/// Reads file content (I/O).
fn read_file_content(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))
}

/// Parses Rust content to AST (I/O for error context).
fn parse_rust_content(content: &str, path: &Path) -> Result<syn::File> {
    syn::parse_file(content)
        .with_context(|| format!("Failed to parse Rust file: {}", path.display()))
}

/// Builds file metrics from components (pure).
fn build_file_metrics(
    path: &Path,
    content: &str,
    syntax: &syn::File,
) -> FileMetrics {
    FileMetrics {
        path: path.to_path_buf(),
        complexity: calculate_total_complexity(syntax),
        functions: count_functions(syntax),
        lines: count_lines(content),
    }
}

// ============================================================================
// PUBLIC API (Composition)
// ============================================================================

/// Analyzes Rust file and returns metrics.
///
/// This is a thin wrapper that composes I/O and pure functions.
pub fn analyze_file(&mut self, path: &Path) -> Result<FileMetrics> {
    let (content, syntax) = read_and_parse_rust_file(path)?;
    Ok(build_file_metrics(path, &content, &syntax))
}
```

**Benefits:**
- `analyze_file` now just 3 lines (I/O composition)
- Each pure function under 10 lines
- Easy to test each function independently
- Clear separation of concerns
- Cyclomatic complexity < 5 for all functions

**Phase 3: Extract Common Patterns**

Many analyzers share similar patterns. Extract to shared utilities:

```rust
// src/analyzers/common.rs

/// Common pure functions used across language analyzers.

/// Counts lines in content (pure).
pub fn count_lines(content: &str) -> usize {
    content.lines().count()
}

/// Counts non-empty lines (pure).
pub fn count_non_empty_lines(content: &str) -> usize {
    content.lines()
        .filter(|line| !line.trim().is_empty())
        .count()
}

/// Counts comment lines (language-specific pattern).
pub fn count_comment_lines(content: &str, comment_prefix: &str) -> usize {
    content.lines()
        .filter(|line| line.trim_start().starts_with(comment_prefix))
        .count()
}

/// Calculates complexity weight based on nesting level.
pub fn nesting_weight(level: usize) -> u32 {
    match level {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 4,
        _ => 8,  // Deep nesting heavily penalized
    }
}
```

**Phase 4: Consistent Analyzer Pattern**

Establish consistent pattern across all analyzers:

```rust
// Pattern for all analyzers:
//
// 1. Pure calculation functions (top of file)
// 2. I/O wrapper functions (middle)
// 3. Public API (bottom)

impl LanguageAnalyzer {
    // ========================================================================
    // PUBLIC API
    // ========================================================================

    /// Analyzes file and returns metrics.
    pub fn analyze_file(&mut self, path: &Path) -> Result<FileMetrics> {
        let (content, ast) = self.read_and_parse(path)?;
        Ok(self.calculate_metrics(path, &content, &ast))
    }

    // ========================================================================
    // I/O OPERATIONS
    // ========================================================================

    /// Reads and parses file (I/O).
    fn read_and_parse(&self, path: &Path) -> Result<(String, Ast)> {
        let content = read_file(path)?;
        let ast = parse_content(&content, path)?;
        Ok((content, ast))
    }

    // ========================================================================
    // PURE CALCULATION FUNCTIONS
    // ========================================================================

    /// Calculates all metrics from parsed data (pure).
    fn calculate_metrics(&self, path: &Path, content: &str, ast: &Ast) -> FileMetrics {
        FileMetrics {
            path: path.to_path_buf(),
            complexity: self.calculate_complexity(ast),
            functions: self.count_functions(ast),
            lines: count_lines(content),
            // ... other metrics
        }
    }

    /// Calculates complexity (pure).
    fn calculate_complexity(&self, ast: &Ast) -> u32 {
        ast.functions()
            .map(|f| self.function_complexity(f))
            .sum()
    }

    // ... more pure functions
}
```

### Function Extraction Guidelines

**When to Extract:**

1. Function > 20 lines → Extract helper functions
2. Nested conditionals → Extract predicate functions
3. Repeated patterns → Extract common functions
4. Mixed I/O and logic → Separate into pure + wrapper

**How to Extract:**

1. **Identify pure logic block** in long function
2. **Create new function** with descriptive name
3. **Move logic** to new function
4. **Add parameters** for needed data
5. **Return result** from new function
6. **Test** new function independently
7. **Replace** original code with function call

**Naming Conventions:**

```rust
// Calculation functions (pure)
calculate_*     // calculate_complexity, calculate_total_weight
count_*         // count_functions, count_parameters
extract_*       // extract_imports, extract_calls
filter_*        // filter_public_items, filter_complex_functions
group_*         // group_by_type, group_by_complexity

// Predicate functions (pure)
is_*            // is_public, is_complex
has_*           // has_parameters, has_return_type
should_*        // should_include, should_warn

// I/O wrapper functions
read_and_*      // read_and_parse, read_and_validate
parse_*         // parse_content, parse_imports
load_*          // load_file, load_config
```

### Complexity Reduction Strategies

**Strategy 1: Extract Predicates**

```rust
// Before: Complex conditional
if node.kind == NodeKind::Function
    && node.visibility == Visibility::Public
    && node.parameters.len() > 5
    && node.lines > 50 {
    // ...
}

// After: Named predicate
fn is_complex_public_function(node: &Node) -> bool {
    is_public_function(node)
        && has_many_parameters(node)
        && is_long_function(node)
}

fn is_public_function(node: &Node) -> bool {
    node.kind == NodeKind::Function && node.visibility == Visibility::Public
}

fn has_many_parameters(node: &Node) -> bool {
    node.parameters.len() > 5
}

fn is_long_function(node: &Node) -> bool {
    node.lines > 50
}

// Usage: Clear and self-documenting
if is_complex_public_function(&node) {
    // ...
}
```

**Strategy 2: Extract Nested Loops**

```rust
// Before: Nested loops (complex)
for module in &ast.modules {
    for function in &module.functions {
        for statement in &function.body {
            // ... nested logic
        }
    }
}

// After: Extracted functions
fn process_modules(modules: &[Module]) -> Vec<Result> {
    modules.iter()
        .flat_map(process_module_functions)
        .collect()
}

fn process_module_functions(module: &Module) -> Vec<Result> {
    module.functions.iter()
        .flat_map(process_function_statements)
        .collect()
}

fn process_function_statements(function: &Function) -> Vec<Result> {
    function.body.iter()
        .map(process_statement)
        .collect()
}
```

**Strategy 3: Pattern Matching Extraction**

```rust
// Before: Long match statement (30+ lines)
fn analyze_node(node: &Node) -> Metrics {
    match node.kind {
        NodeKind::Function => {
            // 10 lines of function logic
        }
        NodeKind::Class => {
            // 10 lines of class logic
        }
        NodeKind::Module => {
            // 10 lines of module logic
        }
    }
}

// After: Extracted match arms
fn analyze_node(node: &Node) -> Metrics {
    match node.kind {
        NodeKind::Function => analyze_function_node(node),
        NodeKind::Class => analyze_class_node(node),
        NodeKind::Module => analyze_module_node(node),
    }
}

fn analyze_function_node(node: &Node) -> Metrics {
    // Function-specific logic (5-10 lines)
}

fn analyze_class_node(node: &Node) -> Metrics {
    // Class-specific logic (5-10 lines)
}

fn analyze_module_node(node: &Node) -> Metrics {
    // Module-specific logic (5-10 lines)
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/analyzers/rust.rs`
  - `src/analyzers/python.rs`
  - `src/analyzers/javascript.rs`
  - `src/analyzers/typescript.rs`
  - `src/analyzers/common.rs` (new - shared utilities)
- **External Dependencies**: None

## Testing Strategy

### Unit Tests (Pure Functions)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_complexity_empty() {
        let syntax = create_empty_syntax_tree();
        assert_eq!(calculate_total_complexity(&syntax), 0);
    }

    #[test]
    fn test_calculate_complexity_simple() {
        let syntax = create_simple_function();
        assert_eq!(calculate_total_complexity(&syntax), 1);
    }

    #[test]
    fn test_calculate_complexity_with_if() {
        let syntax = create_function_with_if();
        assert_eq!(calculate_total_complexity(&syntax), 2);
    }

    #[test]
    fn test_count_functions() {
        let syntax = create_syntax_with_functions(3);
        assert_eq!(count_functions(&syntax), 3);
    }

    #[test]
    fn test_count_lines() {
        let content = "line1\nline2\nline3";
        assert_eq!(count_lines(content), 3);
    }

    #[test]
    fn test_is_complex_public_function() {
        let complex = Node {
            kind: NodeKind::Function,
            visibility: Visibility::Public,
            parameters: vec![/* 6 params */],
            lines: 100,
        };
        assert!(is_complex_public_function(&complex));

        let simple = Node {
            kind: NodeKind::Function,
            visibility: Visibility::Public,
            parameters: vec![/* 2 params */],
            lines: 10,
        };
        assert!(!is_complex_public_function(&simple));
    }
}
```

### Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_complexity_never_negative(
        functions in prop::collection::vec(any::<SynFunction>(), 0..100)
    ) {
        let syntax = create_syntax_from_functions(functions);
        let complexity = calculate_total_complexity(&syntax);
        prop_assert!(complexity >= 0);
    }

    #[test]
    fn test_count_functions_accurate(
        function_count in 0usize..1000
    ) {
        let syntax = create_syntax_with_functions(function_count);
        prop_assert_eq!(count_functions(&syntax), function_count);
    }
}
```

### Integration Tests

```rust
#[test]
fn test_analyze_file_preserves_behavior() {
    // Ensure refactored code produces same results
    let path = PathBuf::from("tests/fixtures/sample.rs");

    let result = analyze_file(&path);

    assert!(result.is_ok());
    let metrics = result.unwrap();
    assert_eq!(metrics.functions, 5);  // Known value
    assert_eq!(metrics.lines, 100);    // Known value
}
```

### Performance Tests

```rust
#[test]
fn test_no_performance_regression() {
    let large_file = PathBuf::from("tests/fixtures/large.rs");

    let start = Instant::now();
    let _ = analyze_file(&large_file);
    let duration = start.elapsed();

    // Should complete quickly (no regression)
    assert!(duration < Duration::from_millis(100));
}
```

## Documentation Requirements

### Code Documentation

```rust
/// Calculates total cyclomatic complexity for all functions.
///
/// This is a pure function that operates on parsed syntax tree data.
/// No I/O operations are performed.
///
/// # Pure Function Properties
///
/// - Deterministic: Same input always produces same output
/// - No side effects: Doesn't modify inputs or global state
/// - No I/O: Works only with in-memory data
/// - Easily testable: Can unit test without mocks
///
/// # Arguments
///
/// * `syntax` - Parsed Rust syntax tree
///
/// # Returns
///
/// Sum of cyclomatic complexity for all functions in the syntax tree
///
/// # Examples
///
/// ```
/// let syntax = parse_rust_file("src/main.rs")?;
/// let complexity = calculate_total_complexity(&syntax);
/// assert!(complexity > 0);
/// ```
fn calculate_total_complexity(syntax: &syn::File) -> u32 {
    // ...
}
```

### Architecture Updates

Add to `ARCHITECTURE.md`:

```markdown
## Analyzer Function Design

All analyzer modules follow consistent patterns:

### Structure

1. **Pure Calculation Functions** (top)
   - No I/O operations
   - Deterministic results
   - Easy to test
   - Under 20 lines each

2. **I/O Wrapper Functions** (middle)
   - File reading
   - Parsing
   - Error handling
   - Orchestration

3. **Public API** (bottom)
   - Composes pure + I/O functions
   - Clean interfaces

### Example

```rust
// Pure (easily tested)
fn calculate_complexity(ast: &Ast) -> u32 { ... }
fn count_functions(ast: &Ast) -> usize { ... }

// I/O wrapper
fn read_and_parse(path: &Path) -> Result<Ast> { ... }

// Public API (composition)
pub fn analyze_file(path: &Path) -> Result<Metrics> {
    let ast = read_and_parse(path)?;
    Ok(Metrics {
        complexity: calculate_complexity(&ast),
        functions: count_functions(&ast),
    })
}
```

### Guidelines

- Maximum function length: 20 lines (prefer 5-10)
- Maximum cyclomatic complexity: 5
- Extract complex conditionals to predicates
- Share common logic in `analyzers/common.rs`
```

## Implementation Notes

### Refactoring Workflow

1. **Analyze** long function
2. **Identify** pure logic blocks
3. **Extract** to new function
4. **Test** extracted function
5. **Replace** in original function
6. **Verify** tests still pass
7. **Repeat** until function < 20 lines

### Tools

```bash
# Find long functions
tokei --files src/analyzers/

# Check complexity
cargo clippy -- -W clippy::cognitive_complexity

# Run tests
cargo test --package debtmap --lib analyzers::
```

### Common Pitfalls

1. **Over-extraction** - Don't create too many tiny functions
2. **Poor naming** - Function names must be descriptive
3. **Breaking tests** - Update tests after extraction
4. **Lost context** - Ensure extracted functions are self-contained

## Migration and Compatibility

### Breaking Changes

**None** - Internal refactoring only. Public APIs unchanged.

### Migration Steps

No user or developer migration needed. Internal improvement only.

## Success Metrics

- ✅ All analyzer functions under 20 lines
- ✅ Cyclomatic complexity < 5 for all functions
- ✅ 90%+ test coverage for pure functions
- ✅ No performance regression
- ✅ All existing tests pass
- ✅ Clear function responsibilities
- ✅ Improved code readability

## Follow-up Work

After this implementation:
- Apply same patterns to other modules
- Create analyzer refactoring guide for contributors
- Consider property-based testing for all analyzers

## References

- **STILLWATER_EVALUATION.md** - Lines 699-702 (Extract pure functions recommendation)
- **CLAUDE.md** - Function design guidelines (max 20 lines)
- **Spec 183** - Analyzer I/O separation pattern
- **Stillwater Philosophy** - Pure core principles
