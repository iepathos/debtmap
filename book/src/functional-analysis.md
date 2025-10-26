# Functional Composition Analysis

AST-based functional composition analysis identifies pure functions, detects side effects, and recognizes functional programming patterns in your code. This helps teams adopt functional programming practices and identify imperative anti-patterns.

## Overview

Functional analysis detects:
- Pure functions (no side effects)
- Side effect patterns
- Functional composition patterns
- Purity distribution across codebase

## Purity Profiles

### Strict Profile

**Purpose:** Enforce functional programming standards

**Criteria:**
- No mutable state access
- No I/O operations
- No external function calls with side effects
- Deterministic behavior only

```bash
debtmap analyze . --ast-functional-analysis strict
```

### Balanced Profile

**Purpose:** Pragmatic functional style (default)

**Criteria:**
- Allow some controlled mutations (e.g., builders)
- Permit logging
- Accept common patterns like caching

```bash
debtmap analyze . --ast-functional-analysis balanced
```

### Lenient Profile

**Purpose:** Track functional patterns without strict enforcement

**Criteria:**
- Identify clear pure functions
- Flag obvious side effects
- Allow most common patterns

```bash
debtmap analyze . --ast-functional-analysis lenient
```

## Pure Function Detection

A function is considered pure when it:
1. Returns same output for same input (deterministic)
2. Has no observable side effects
3. Doesn't mutate external state
4. Doesn't perform I/O

### Examples

```rust
// Pure function
fn add(a: i32, b: i32) -> i32 {
    a + b
}

// Pure function with internal mutation
fn factorial(n: u32) -> u32 {
    (1..=n).product()  // Pure despite internal iteration
}

// Not pure: I/O side effect
fn log_and_add(a: i32, b: i32) -> i32 {
    println!("Adding {} and {}", a, b);  // Side effect!
    a + b
}

// Not pure: mutates external state
fn increment_counter(counter: &mut i32) -> i32 {
    *counter += 1;  // Side effect!
    *counter
}
```

## Side Effect Detection

### Detected Side Effects

**I/O Operations:**
- File reading/writing
- Network calls
- Console output
- Database queries

**State Mutation:**
- Mutable global variables
- Shared mutable state
- Reference mutations

**Randomness:**
- Random number generation
- Time-dependent behavior

**System Interaction:**
- Environment variable access
- System calls
- Thread spawning

### Rust-Specific Detection

```rust
// Interior mutability detection
use std::cell::RefCell;

fn has_side_effect() {
    let data = RefCell::new(vec![]);
    data.borrow_mut().push(1);  // Detected as mutation
}

// Unsafe code detection
fn unsafe_side_effect() {
    unsafe {
        // Automatically flagged as potentially impure
    }
}
```

## Composition Pattern Recognition

### Function Composition

```rust
// Detected composition pattern
fn process_data(input: String) -> Result<Output> {
    input
        .parse()
        .map(validate)
        .and_then(transform)
        .map(normalize)
}
```

### Higher-Order Functions

```rust
// Detected HOF pattern
fn apply_twice<F>(f: F, x: i32) -> i32
where
    F: Fn(i32) -> i32,
{
    f(f(x))
}
```

### Map/Filter/Fold Chains

```rust
// Detected functional pipeline
let result = items
    .iter()
    .filter(|x| x.is_valid())
    .map(|x| x.transform())
    .fold(0, |acc, x| acc + x);
```

## Purity Scoring

### Distribution Analysis

Debtmap calculates purity distribution:
- **Pure functions**: 0 side effects detected
- **Mostly pure**: Minor side effects (e.g., logging)
- **Impure**: Multiple side effects
- **Highly impure**: Extensive state mutation and I/O

### Scoring Formula

```
Purity Score = (pure_functions / total_functions) × 100
Side Effect Density = total_side_effects / total_functions
```

### Codebase Health Metrics

```
Target Purity Levels:
- Core business logic: 80%+ pure
- Utilities: 70%+ pure
- I/O layer: 20-30% pure (expected)
- Overall: 50%+ pure
```

## Configuration

### Enable Functional Analysis

```bash
debtmap analyze . --ast-functional-analysis
```

### Select Profile

```bash
# Strict functional requirements
debtmap analyze . --ast-functional-analysis strict

# Balanced approach
debtmap analyze . --ast-functional-analysis balanced

# Lenient tracking
debtmap analyze . --ast-functional-analysis lenient
```

### Filter Results

```bash
# Show only impure functions
debtmap analyze . --ast-functional-analysis --filter-categories SideEffects
```

## Best Practices

**Separate pure from impure:**
- Keep pure logic in core modules
- Isolate I/O at boundaries
- Use dependency injection for testability

**Maximize purity in:**
- Business logic
- Calculations and transformations
- Validation functions
- Data structure operations

**Accept impurity in:**
- I/O layers
- Logging and monitoring
- External system integration
- Application boundaries

**Refactoring strategy:**
1. Identify impure functions
2. Extract pure logic
3. Push side effects to boundaries
4. Test pure functions exhaustively

## Use Cases

### Code Quality Audit

```bash
# Assess functional purity
debtmap analyze . --ast-functional-analysis --format markdown
```

### Refactoring Targets

```bash
# Find impure functions in core logic
debtmap analyze src/core/ --ast-functional-analysis strict
```

### Onboarding Guide

```bash
# Show functional patterns in codebase
debtmap analyze . --ast-functional-analysis balanced --summary
```

## Troubleshooting

### Too Many False Positives

**Issue:** Pure functions flagged as impure

**Solution:**
- Use lenient profile
- Suppress known patterns
- Review detection criteria
- Report false positives

### Missing Side Effects

**Issue:** Known impure functions not detected

**Solution:**
- Use strict profile
- Check for exotic side effect patterns
- Enable comprehensive analysis

## See Also

- [Analysis Guide](analysis-guide.md) - Understanding analysis types
- [Best Practices](#) - Functional programming in Rust
- [Refactoring](refactoring-guide.md) - Extracting pure functions
