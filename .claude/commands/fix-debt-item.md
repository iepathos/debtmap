---
name: fix-debt-item
description: Fix a specific tech debt item identified by debtmap
---

# Fix Specific Tech Debt Item

Fix a single tech debt item with the provided parameters from debtmap analysis.

## Parameters

Required parameters passed from MapReduce workflow:
- `--file`: The file containing the debt item
- `--function`: The function name with the issue
- `--line`: The line number where the issue starts
- `--score`: The unified priority score of this item

## Process

### Step 1: Analyze the Specific Issue
Read the file and locate the exact function:
```bash
# View the function context
sed -n "${line},+50p" ${file}
```

Determine the issue type based on the function's characteristics:
- High cyclomatic complexity (>10)
- Low test coverage
- Both complexity and coverage issues

### Step 2: Apply Functional Programming Refactoring

For high complexity issues, follow these functional patterns:

#### Pattern 1: Extract Pure Classification Functions
If the function contains multiple conditionals for categorization:

```rust
// Before: Multiple if-else chains
if condition_a { TypeA } else if condition_b { TypeB } else { TypeC }

// After: Pure static function with pattern matching
fn classify_type(input: &str) -> Type {
    match () {
        _ if input.starts_with("test_") => Type::Test,
        _ if input.contains("_impl") => Type::Implementation,
        _ => Type::Regular,
    }
}
```

#### Pattern 2: Replace Imperative Loops with Iterator Chains
Transform loops into functional iterator patterns:

```rust
// Before: Imperative loop
let mut results = Vec::new();
for item in items {
    if item.is_valid() {
        results.push(item.transform());
    }
}

// After: Functional chain
let results: Vec<_> = items
    .iter()
    .filter(|item| item.is_valid())
    .map(|item| item.transform())
    .collect();
```

#### Pattern 3: Extract Pure Business Logic from I/O
Separate side effects from pure logic:

```rust
// Before: Mixed I/O and logic
fn process_file(path: &Path) {
    let content = fs::read_to_string(path).unwrap();
    // Complex parsing logic here
    println!("{}", result);
}

// After: Pure function + thin I/O wrapper
fn parse_content(content: &str) -> Result<ParsedData, Error> {
    // Pure parsing logic
}

fn process_file(path: &Path) -> Result<(), Error> {
    let content = fs::read_to_string(path)?;
    let result = parse_content(&content)?;
    println!("{}", result);
    Ok(())
}
```

### Step 3: Add Comprehensive Tests

For functions lacking coverage, create thorough test cases:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_happy_path() {
        // Test normal operation
    }

    #[test]
    fn test_edge_cases() {
        // Test boundary conditions
    }

    #[test]
    fn test_error_conditions() {
        // Test error handling
    }
}
```

Focus on testing:
- Pure functions extracted during refactoring
- Business logic separated from I/O
- Edge cases and error conditions
- All branches of pattern matching

### Step 4: Verify the Fix

Run tests to ensure the fix works:
```bash
cargo test --lib $(echo ${function} | tr '::' ' ')
```

Check that the function still compiles and passes clippy:
```bash
cargo clippy --all-targets -- -D warnings
```

### Step 5: Measure Improvement

Calculate the improvement for this specific function:
- If complexity was reduced, note the before/after cyclomatic complexity
- If tests were added, note the coverage improvement
- Document which functional patterns were applied

## Implementation Guidelines

### Functional Programming Principles

**Always Prefer:**
- **Pure functions** over stateful methods
- **Immutability** - use `&self` instead of `&mut self` where possible
- **Function composition** - build complex behavior from simple functions
- **Pattern matching** over if-else chains
- **Iterator chains** over imperative loops
- **Type-driven design** - use the type system to enforce invariants

### Refactoring Decision Tree

```
Score >= 7 AND Complexity > 10?
├─ YES → Apply functional refactoring
│   ├─ Classification logic? → Extract pure static function
│   ├─ Nested loops? → Convert to iterator chains
│   ├─ Mixed I/O and logic? → Extract pure core
│   └─ Complex conditionals? → Use pattern matching
└─ NO → Add comprehensive tests only
```

### What NOT to Do

**Avoid these anti-patterns:**
- Creating single-use helper methods
- Over-abstracting simple logic
- Breaking apart clear match expressions
- Adding complexity to reduce metrics
- Testing I/O directly instead of extracting logic

### Idiomatic Rust Patterns

**Use these Rust idioms:**
```rust
// Use ? for error propagation
let data = read_file()?;

// Prefer &str over String in parameters
fn process(input: &str) -> Result<String, Error>

// Use #[derive] for common traits
#[derive(Debug, Clone, PartialEq)]
struct Data { ... }

// Use From/Into for type conversions
impl From<String> for MyType { ... }

// Prefer iterators over indexing
items.iter().map(|x| x.process())

// Use Option/Result combinators
value.map(|v| v.transform()).unwrap_or_default()
```

## Success Criteria

The fix is complete when:
- [ ] The specific function has been refactored or tested
- [ ] All tests pass (`cargo test`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Code follows functional programming patterns
- [ ] The fix reduces the debt score for this specific item
- [ ] Implementation uses idiomatic Rust patterns

## Notes

- Focus only on the specific function identified
- Apply functional programming patterns consistently
- Keep changes minimal and focused
- Ensure backward compatibility
- Document any extracted pure functions
- This command is designed for MapReduce workflow integration