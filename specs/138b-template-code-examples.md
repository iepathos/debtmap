---
number: 138b
title: Template-Based Code Examples for Recommendations
category: optimization
priority: low
status: deferred
dependencies: [138a]
created: 2025-10-29
replaces: 138 (split into 138a/b/c)
deferred_reason: Code examples may add output bloat without clear value. Specific pattern detection (138c) provides more actionable guidance. May revisit if user feedback shows demand for visual examples.
---

# Specification 138b: Template-Based Code Examples for Recommendations

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 138a (Concise Recommendations)
**Supersedes**: Spec 138 (split into three focused specs)

## Context

After implementing concise recommendations (Spec 138a), users still need concrete guidance on **how** to implement refactorings. Current recommendations say "Extract complex logic into smaller functions" but don't show what that looks like.

**Current Issues**:
1. **No Visual Examples**: Users told to refactor but not shown how
2. **Generic Advice**: "Split into core/io/utils" doesn't help with specific code
3. **Learning Curve**: Junior developers need examples to understand patterns
4. **Inconsistent Understanding**: Teams interpret "extract function" differently

**User Feedback** (hypothetical but realistic):
> "The recommendations are helpful, but I'd love to see an example of what the refactored code should look like."

## Objective

Add **template-based code examples** to recommendations that show before/after refactoring patterns, personalized with actual function names from the analyzed code.

**Scope**: Templates only, no AST-based code generation (too complex, deferred to 138c if needed).

## Requirements

### Functional Requirements

1. **Template Library**
   - 10-15 common refactoring patterns with before/after templates
   - Templates use placeholder syntax (e.g., `{function_name}`)
   - Personalize with actual function names from metrics
   - Language-agnostic templates (apply to Rust/Python/JS/TS)

2. **Pattern Matching**
   - Match templates to debt types using existing metrics
   - No AST analysis required (use cyclomatic, cognitive, nesting, length)
   - Simple heuristics to select appropriate template

3. **Example Structure**
   - Title: Short description of pattern
   - Before: Code showing the problem
   - After: Code showing the solution
   - Explanation: Why this helps (1-2 sentences)

4. **Personalization**
   - Insert actual function name from metrics
   - Use detected language syntax (Rust `fn`, Python `def`, JS `function`)
   - Keep examples under 15 lines (readable at a glance)

### Non-Functional Requirements

1. **Performance**: <1ms per example generation
2. **Maintainability**: Templates in separate file (easy to update)
3. **Extensibility**: Easy to add new templates
4. **Language Support**: Start with Rust, extend to Python/JS/TS

## Acceptance Criteria

- [ ] 10-15 refactoring templates implemented
- [ ] Templates personalized with actual function names
- [ ] Language-specific syntax detected and applied
- [ ] Examples shown for ComplexityHotspot and TestingGap debt types
- [ ] All examples under 15 lines
- [ ] <1ms performance overhead per example
- [ ] Templates stored in `src/recommendations/templates/` directory
- [ ] Integration test validates ripgrep shows relevant example
- [ ] Documentation explains how to add new templates

## Technical Details

### Implementation Approach

#### 1. Data Structures

```rust
/// Code example showing before/after refactoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExample {
    /// Short title (e.g., "Extract Guard Clauses")
    pub title: String,
    /// Code before refactoring
    pub before: String,
    /// Code after refactoring
    pub after: String,
    /// Why this helps (1-2 sentences)
    pub explanation: String,
}

/// Template for code examples with placeholders
#[derive(Debug, Clone)]
pub struct ExampleTemplate {
    pub pattern: RefactoringPattern,
    pub title: String,
    pub before_template: &'static str,
    pub after_template: &'static str,
    pub explanation: &'static str,
}

/// Types of refactoring patterns we can demonstrate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefactoringPattern {
    NestedConditionals,      // if-in-if → guard clauses
    ExtractFunction,         // long function → multiple small functions
    EarlyReturn,             // nested → early returns
    ExtractPredicate,        // complex condition → named function
    SeparateIOFromLogic,     // mixed → pure core + IO shell
    ParameterObject,         // long param list → struct
    ReplaceLoopWithIterator, // for loop → map/filter/fold
    ExtractTestHelper,       // duplicated test setup → helper
}
```

#### 2. Template Library

```rust
/// Get all available templates
pub fn get_templates() -> Vec<ExampleTemplate> {
    vec![
        // Template 1: Nested Conditionals → Guard Clauses
        ExampleTemplate {
            pattern: RefactoringPattern::NestedConditionals,
            title: "Replace Nested Conditionals with Guard Clauses".to_string(),
            before_template: r#"
fn {function_name}(config: &Config, data: &Data) -> Result<Output> {
    if config.is_enabled() {
        if data.is_valid() {
            if data.has_content() {
                // Process data
                process(data)
            } else {
                Err("No content")
            }
        } else {
            Err("Invalid data")
        }
    } else {
        Err("Disabled")
    }
}
"#,
            after_template: r#"
fn {function_name}(config: &Config, data: &Data) -> Result<Output> {
    if !config.is_enabled() {
        return Err("Disabled");
    }
    if !data.is_valid() {
        return Err("Invalid data");
    }
    if !data.has_content() {
        return Err("No content");
    }

    process(data)
}
"#,
            explanation: "Guard clauses reduce nesting depth and make the happy path clear. \
                         Each check fails fast, leaving the main logic at the end."
        },

        // Template 2: Extract Predicate Function
        ExampleTemplate {
            pattern: RefactoringPattern::ExtractPredicate,
            title: "Extract Complex Condition into Named Predicate".to_string(),
            before_template: r#"
fn {function_name}(user: &User, item: &Item) -> bool {
    if user.is_active && user.has_permission("write")
        && item.status == Status::Available
        && !item.is_locked() {
        // Process...
        true
    } else {
        false
    }
}
"#,
            after_template: r#"
fn {function_name}(user: &User, item: &Item) -> bool {
    if can_modify_item(user, item) {
        // Process...
        true
    } else {
        false
    }
}

fn can_modify_item(user: &User, item: &Item) -> bool {
    user.is_active
        && user.has_permission("write")
        && item.status == Status::Available
        && !item.is_locked()
}
"#,
            explanation: "Named predicates document intent and can be tested independently. \
                         The complex condition becomes self-documenting."
        },

        // Template 3: Separate I/O from Logic
        ExampleTemplate {
            pattern: RefactoringPattern::SeparateIOFromLogic,
            title: "Separate I/O from Pure Logic".to_string(),
            before_template: r#"
fn {function_name}(path: &Path) -> Result<Stats> {
    let content = fs::read_to_string(path)?;
    let lines: Vec<_> = content.lines().collect();
    let word_count = lines.iter()
        .flat_map(|line| line.split_whitespace())
        .count();
    Ok(Stats { lines: lines.len(), words: word_count })
}
"#,
            after_template: r#"
fn {function_name}(path: &Path) -> Result<Stats> {
    let content = fs::read_to_string(path)?;
    Ok(calculate_stats(&content))
}

fn calculate_stats(content: &str) -> Stats {
    let lines: Vec<_> = content.lines().collect();
    let word_count = lines.iter()
        .flat_map(|line| line.split_whitespace())
        .count();
    Stats { lines: lines.len(), words: word_count }
}
"#,
            explanation: "Pure functions are easier to test and reason about. \
                         Keep I/O at the edges, logic in the pure core."
        },

        // Template 4: Extract Function
        ExampleTemplate {
            pattern: RefactoringPattern::ExtractFunction,
            title: "Extract Focused Function from Long Method".to_string(),
            before_template: r#"
fn {function_name}(data: &[Item]) -> Summary {
    // 50+ lines of processing
    let mut total = 0;
    let mut valid_count = 0;
    for item in data {
        if item.is_valid() {
            total += item.value;
            valid_count += 1;
        }
    }
    let average = if valid_count > 0 { total / valid_count } else { 0 };
    // More processing...
    Summary { total, average, count: valid_count }
}
"#,
            after_template: r#"
fn {function_name}(data: &[Item]) -> Summary {
    let valid_items: Vec<_> = filter_valid_items(data);
    let total = calculate_total(&valid_items);
    let average = calculate_average(&valid_items, total);

    Summary { total, average, count: valid_items.len() }
}

fn filter_valid_items(data: &[Item]) -> Vec<&Item> {
    data.iter().filter(|item| item.is_valid()).collect()
}

fn calculate_total(items: &[&Item]) -> u32 {
    items.iter().map(|item| item.value).sum()
}
"#,
            explanation: "Each function does one thing. Easier to test, understand, and modify. \
                         Names document what each step does."
        },

        // Template 5: Replace Loop with Iterator
        ExampleTemplate {
            pattern: RefactoringPattern::ReplaceLoopWithIterator,
            title: "Replace Imperative Loop with Functional Iterator".to_string(),
            before_template: r#"
fn {function_name}(items: &[Item]) -> Vec<ProcessedItem> {
    let mut results = Vec::new();
    for item in items {
        if item.is_active() {
            let processed = transform(item);
            results.push(processed);
        }
    }
    results
}
"#,
            after_template: r#"
fn {function_name}(items: &[Item]) -> Vec<ProcessedItem> {
    items.iter()
        .filter(|item| item.is_active())
        .map(|item| transform(item))
        .collect()
}
"#,
            explanation: "Iterator chains are declarative and concise. Clear data transformation pipeline. \
                         No manual mutation needed."
        },

        // Template 6: Parameter Object
        ExampleTemplate {
            pattern: RefactoringPattern::ParameterObject,
            title: "Replace Long Parameter List with Config Object".to_string(),
            before_template: r#"
fn {function_name}(
    host: &str,
    port: u16,
    timeout: Duration,
    retries: u32,
    use_tls: bool,
    verify_cert: bool,
) -> Result<Connection> {
    // ...
}
"#,
            after_template: r#"
struct ConnectionConfig {
    host: String,
    port: u16,
    timeout: Duration,
    retries: u32,
    use_tls: bool,
    verify_cert: bool,
}

fn {function_name}(config: &ConnectionConfig) -> Result<Connection> {
    // ...
}
"#,
            explanation: "Parameter objects reduce function signatures and enable builder patterns. \
                         Related configuration stays together."
        },

        // Template 7: Extract Test Helper
        ExampleTemplate {
            pattern: RefactoringPattern::ExtractTestHelper,
            title: "Extract Common Test Setup into Helper".to_string(),
            before_template: r#"
#[test]
fn test_{function_name}_case1() {
    let config = Config::new();
    let db = TestDb::new();
    db.insert_test_data();
    let service = Service::new(config, db);

    let result = service.{function_name}(input1);
    assert!(result.is_ok());
}

#[test]
fn test_{function_name}_case2() {
    let config = Config::new();
    let db = TestDb::new();
    db.insert_test_data();
    let service = Service::new(config, db);

    let result = service.{function_name}(input2);
    assert!(result.is_err());
}
"#,
            after_template: r#"
fn setup_test_service() -> (Service, TestDb) {
    let config = Config::new();
    let db = TestDb::new();
    db.insert_test_data();
    let service = Service::new(config, db);
    (service, db)
}

#[test]
fn test_{function_name}_case1() {
    let (service, _db) = setup_test_service();
    let result = service.{function_name}(input1);
    assert!(result.is_ok());
}

#[test]
fn test_{function_name}_case2() {
    let (service, _db) = setup_test_service();
    let result = service.{function_name}(input2);
    assert!(result.is_err());
}
"#,
            explanation: "Test helpers eliminate duplication and make test intent clearer. \
                         Setup code is maintained in one place."
        },

        // Add 8 more templates for other patterns...
    ]
}
```

#### 3. Template Selection Logic

```rust
/// Select appropriate template based on metrics and debt type
pub fn select_template(
    debt_type: &DebtType,
    metrics: &FunctionMetrics,
) -> Option<RefactoringPattern> {
    match debt_type {
        DebtType::ComplexityHotspot { cyclomatic, cognitive } => {
            // Use existing metrics to detect pattern
            if metrics.nesting > 3 {
                Some(RefactoringPattern::NestedConditionals)
            } else if metrics.length > 80 && *cyclomatic > 15 {
                Some(RefactoringPattern::ExtractFunction)
            } else if *cognitive > *cyclomatic + 5 {
                Some(RefactoringPattern::ExtractPredicate)
            } else {
                Some(RefactoringPattern::EarlyReturn)
            }
        }

        DebtType::TestingGap { .. } if metrics.is_test => {
            // For test files with duplication
            Some(RefactoringPattern::ExtractTestHelper)
        }

        DebtType::TestingGap { .. } => {
            // Suggest separating I/O for better testability
            if metrics.name.contains("read") || metrics.name.contains("write")
                || metrics.name.contains("load") || metrics.name.contains("save") {
                Some(RefactoringPattern::SeparateIOFromLogic)
            } else {
                None // No specific template, use generic guidance
            }
        }

        _ => None,
    }
}

/// Generate personalized code example
pub fn generate_code_example(
    pattern: RefactoringPattern,
    metrics: &FunctionMetrics,
) -> Option<CodeExample> {
    let template = get_templates().into_iter()
        .find(|t| t.pattern == pattern)?;

    // Personalize with actual function name
    let before = template.before_template
        .replace("{function_name}", &metrics.name);
    let after = template.after_template
        .replace("{function_name}", &metrics.name);

    Some(CodeExample {
        title: template.title,
        before,
        after,
        explanation: template.explanation.to_string(),
    })
}
```

#### 4. Integration with Spec 138a

```rust
/// Enhanced recommendation with optional code example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionableRecommendation {
    pub primary_action: String,
    pub rationale: String,
    pub steps: Vec<ActionStep>,
    pub estimated_effort_hours: f32,
    /// Optional code example showing refactoring pattern
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_example: Option<CodeExample>,
}

/// Add code example to recommendation if pattern detected
pub fn enhance_with_example(
    mut rec: ActionableRecommendation,
    debt_type: &DebtType,
    metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    if let Some(pattern) = select_template(debt_type, metrics) {
        rec.code_example = generate_code_example(pattern, metrics);
    }
    rec
}
```

### Output Format

```
ACTION: Reduce complexity from 25 to ~10
RATIONALE: High complexity 25/35 makes function hard to test and maintain
EFFORT: 3.0 hours

STEPS:
  1. [Medium] Add tests before refactoring (if coverage < 80%)
     Impact: +safety net for refactoring
     Run: cargo test process_data::

  2. [Hard] Extract 3 focused functions
     Impact: -15 complexity
     Run: cargo clippy

  3. [Easy] Verify tests still pass
     Impact: Confirmed refactoring safe
     Run: cargo test --all

CODE EXAMPLE: Replace Nested Conditionals with Guard Clauses

  Before:
    fn process_data(config: &Config, data: &Data) -> Result<Output> {
        if config.is_enabled() {
            if data.is_valid() {
                if data.has_content() {
                    process(data)
                } else {
                    Err("No content")
                }
            } else {
                Err("Invalid data")
            }
        } else {
            Err("Disabled")
        }
    }

  After:
    fn process_data(config: &Config, data: &Data) -> Result<Output> {
        if !config.is_enabled() {
            return Err("Disabled");
        }
        if !data.is_valid() {
            return Err("Invalid data");
        }
        if !data.has_content() {
            return Err("No content");
        }

        process(data)
    }

  Why: Guard clauses reduce nesting depth and make the happy path clear.
       Each check fails fast, leaving the main logic at the end.
```

## Dependencies

**Prerequisites**:
- Spec 138a (Concise Recommendations) - Must be implemented first

**Affected Components**:
- `src/recommendations/templates/` - New module for templates
- `src/priority/scoring/recommendation.rs` - Add code example generation
- `src/priority/mod.rs` - Add `CodeExample` struct
- `src/io/writers/enhanced_markdown/recommendation_writer.rs` - Format examples

**External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_template_personalization() {
    let metrics = create_test_metrics("process_data", 25, 30);
    let pattern = RefactoringPattern::NestedConditionals;

    let example = generate_code_example(pattern, &metrics)
        .expect("Should generate example");

    assert!(example.before.contains("process_data"));
    assert!(example.after.contains("process_data"));
    assert!(!example.before.contains("{function_name}"));
}

#[test]
fn test_template_selection_for_nested_code() {
    let metrics = FunctionMetrics {
        name: "complex_func".to_string(),
        nesting: 4, // Deep nesting
        cyclomatic: 15,
        cognitive: 20,
        ..default()
    };

    let debt_type = DebtType::ComplexityHotspot { cyclomatic: 15, cognitive: 20 };
    let pattern = select_template(&debt_type, &metrics);

    assert_eq!(pattern, Some(RefactoringPattern::NestedConditionals));
}

#[test]
fn test_all_templates_valid_syntax() {
    for template in get_templates() {
        // Verify templates have required placeholders
        assert!(template.before_template.contains("{function_name}"));
        assert!(template.after_template.contains("{function_name}"));

        // Verify templates are reasonably sized
        assert!(template.before_template.lines().count() < 20);
        assert!(template.after_template.lines().count() < 20);
    }
}

#[test]
fn test_example_generation_performance() {
    let metrics = create_test_metrics("test_func", 20, 25);
    let pattern = RefactoringPattern::ExtractFunction;

    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let _ = generate_code_example(pattern, &metrics);
    }
    let elapsed = start.elapsed();

    // Should be very fast (string replacement only)
    assert!(elapsed.as_millis() < 100, "1000 examples took {}ms", elapsed.as_millis());
}
```

### Integration Tests

```rust
#[test]
fn test_ripgrep_shows_relevant_example() {
    let results = analyze_file("../ripgrep/crates/core/flags/hiargs.rs")
        .expect("Should analyze ripgrep");

    let complex_item = results.items.iter()
        .find(|item| matches!(item.debt_type, DebtType::ComplexityHotspot { .. }))
        .expect("Should find complexity hotspot");

    let rec = &complex_item.recommendation;

    // Should have code example for complex function
    assert!(rec.code_example.is_some(), "Complex function should have code example");

    let example = rec.code_example.as_ref().unwrap();
    assert!(!example.before.is_empty());
    assert!(!example.after.is_empty());
    assert!(!example.explanation.is_empty());

    // Example should be personalized with actual function name
    let func_name = &complex_item.location.function;
    assert!(example.before.contains(func_name) || example.after.contains(func_name));
}
```

## Documentation Requirements

### Code Documentation

- Document each template with rationale
- Explain template selection heuristics
- Provide guidelines for adding new templates

### User Documentation

Update README:
```markdown
## Code Examples in Recommendations

Debtmap provides code examples showing how to refactor complex code:

- **Nested Conditionals** → Guard clauses
- **Long Functions** → Extracted focused functions
- **Complex Conditions** → Named predicates
- **Mixed I/O and Logic** → Separated concerns

Examples use your actual function names for relevance.
```

### Developer Documentation

Create `TEMPLATES.md`:
```markdown
# Adding Refactoring Templates

## Template Structure

Each template needs:
1. Pattern identifier
2. Before/after code with `{function_name}` placeholder
3. Brief explanation

## Example

```rust
ExampleTemplate {
    pattern: RefactoringPattern::YourPattern,
    title: "Your Pattern Title",
    before_template: "fn {function_name}(...) { ... }",
    after_template: "fn {function_name}(...) { ... }",
    explanation: "Why this helps",
}
```

## Selection Heuristics

Add logic to `select_template()` based on existing metrics.
```

## Success Metrics

- **10-15 templates implemented** (manual count)
- **100% of ComplexityHotspot items get example** (automated test)
- **<1ms per example generation** (benchmark)
- **Examples always personalized with function name** (automated test)
- **No examples exceed 15 lines** (automated test)

## Migration and Compatibility

### Backward Compatibility

**Fully Backward Compatible**:
- `code_example` field is optional
- Existing JSON output unchanged (new field skipped if None)
- Existing formatters work without changes
- No breaking changes

### Gradual Rollout

**Phase 1**: Add templates for Rust only
**Phase 2**: Test with real codebases, gather feedback
**Phase 3**: Add Python/JS/TS templates if validated

## Implementation Notes

### Why Templates, Not AST Generation?

**Templates** (this spec):
- ✅ Simple string replacement
- ✅ Fast (<1ms)
- ✅ Easy to maintain
- ✅ Language-agnostic
- ✅ No risk of generating incorrect code

**AST Generation** (deferred):
- ❌ Complex AST manipulation
- ❌ Slower (parsing + generation)
- ❌ Language-specific implementation
- ❌ Risk of incorrect refactorings
- ❌ Harder to maintain

Templates provide 80% of the value with 20% of the complexity.

### Template Maintenance

Store templates in code (not external files) because:
1. Type safety (compile-time validation)
2. No file I/O overhead
3. Easy to grep/search
4. Versioned with code

Templates are static data, not configuration.

## Related Specifications

- **Spec 138a**: Concise Recommendations (prerequisite)
- **Spec 138c**: Pattern Detection Library (future enhancement)
- **Spec 137**: Call Graph Analysis (could improve template selection)

## Approval Checklist

- [ ] Templates reviewed for correctness
- [ ] All templates compile in their target language
- [ ] Performance benchmarks show <1ms overhead
- [ ] Integration test passes
- [ ] Documentation complete
- [ ] Backward compatibility verified
