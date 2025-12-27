# Semantic Classification

Debtmap performs semantic analysis to classify functions by their architectural role, enabling more accurate complexity scoring and prioritization.

## Overview

Semantic classification identifies the purpose of each function based on AST patterns, helping debtmap:
- Apply appropriate complexity expectations
- Adjust scoring based on function role
- Provide role-specific recommendations

## Function Roles

### Pure Logic

Functions that compute without side effects:

```rust
fn calculate_total(items: &[Item]) -> u32 {
    items.iter().map(|i| i.price).sum()
}
```

### Orchestrator

Functions that coordinate other functions:

```rust
fn process_order(order: Order) -> Result<Receipt> {
    let validated = validate_order(&order)?;
    let priced = calculate_prices(&validated)?;
    finalize_order(&priced)
}
```

### I/O Wrapper

Functions that wrap I/O operations:

```rust
fn read_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path)?;
    toml::from_str(&content)
}
```

### Entry Point

Main functions and public API endpoints:

```rust
fn main() {
    let args = Args::parse();
    run(args).unwrap();
}
```

### Pattern Match

Functions dominated by pattern matching:

```rust
fn handle_event(event: Event) -> Action {
    match event {
        Event::Click(pos) => Action::Select(pos),
        Event::Drag(from, to) => Action::Move(from, to),
        Event::Release => Action::Confirm,
    }
}
```

## AST-Based Detection

Semantic classification uses AST analysis to detect:
- Function signatures and return types
- Control flow patterns
- Call relationships
- Side effect indicators

## Role-Specific Expectations

Different roles have different coverage and complexity expectations:

| Role | Coverage Expectation | Complexity Tolerance |
|------|---------------------|---------------------|
| Pure Logic | High | Low |
| Orchestrator | Medium | Medium |
| I/O Wrapper | Low | Low |
| Entry Point | Low | Medium |
| Pattern Match | Medium | Variable |

## Scoring Adjustments

Semantic classification affects scoring through role multipliers:

```toml
[scoring.role_multipliers]
pure_logic = 1.0
orchestrator = 0.8
io_wrapper = 0.5
entry_point = 0.3
pattern_match = 0.7
```

## Configuration

```toml
[semantic]
enabled = true
role_detection = true
adjust_coverage_expectations = true
```

## See Also

- [Role-Based Adjustments](scoring-strategies/role-based.md)
- [Functional Composition Analysis](functional-analysis.md)
