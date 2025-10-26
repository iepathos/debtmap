# Error Handling Analysis

Debtmap analyzes error handling patterns to identify potential bugs, silent failures, and error handling anti-patterns. This chapter covers error swallowing detection, panic patterns, and best practices across Rust, Python, and JavaScript/TypeScript.

## Overview

Error handling analysis detects:
- Error swallowing (silent failures)
- Poor error propagation
- Panic/exception patterns
- Missing error context
- Language-specific anti-patterns

## Error Swallowing Detection

### Rust Patterns

#### if let Ok(...) without else

```rust
// Detected: error swallowing
if let Ok(value) = risky_operation() {
    process(value);
}
// Error case silently ignored!

// Better: handle or propagate
if let Ok(value) = risky_operation() {
    process(value);
} else {
    log::warn!("Operation failed, using default");
}
```

#### let _ = result

```rust
// Detected: error swallowing
let _ = write_to_file(data);
// Errors completely ignored

// Better: explicitly handle
if let Err(e) = write_to_file(data) {
    log::error!("Failed to write: {}", e);
}
```

#### .ok() discard

```rust
// Detected: error swallowing
config.get("key").ok();
// Result discarded, errors lost

// Better: provide default or propagate
config.get("key").unwrap_or(&default_value)
```

#### Empty error match arms

```rust
// Detected: error swallowing
match operation() {
    Ok(v) => process(v),
    Err(_) => {} // Silent failure!
}

// Better: log or handle
match operation() {
    Ok(v) => process(v),
    Err(e) => log::error!("Operation failed: {}", e),
}
```

#### unwrap_or without logging

```rust
// Detected: potential error swallowing
let value = parse_config().unwrap_or_default();
// Errors lost without trace

// Better: log failure
let value = parse_config().unwrap_or_else(|e| {
    log::warn!("Config parse failed: {}, using default", e);
    Default::default()
});
```

### Python Patterns

#### Bare except clauses

```python
# Detected: overly broad error handling
try:
    risky_operation()
except:  # Catches everything, even KeyboardInterrupt!
    pass

# Better: specific exceptions
try:
    risky_operation()
except ValueError as e:
    logger.error(f"Invalid value: {e}")
```

#### Generic Exception catching

```python
# Detected: too broad
try:
    process_data()
except Exception:  # Catches too much
    pass

# Better: specific types
try:
    process_data()
except (ValueError, KeyError) as e:
    logger.error(f"Data processing failed: {e}")
    raise
```

#### Silent exception handling

```python
# Detected: error swallowing
try:
    save_to_database()
except Exception:
    pass  # Silently fails!

# Better: log and decide
try:
    save_to_database()
except DatabaseError as e:
    logger.error(f"Database save failed: {e}")
    # Re-raise or use fallback
    raise
```

## Error Propagation Patterns

### Type Erasure Issues

```rust
// Detected: information loss
fn process() -> Result<(), Box<dyn Error>> {
    let data = load_data()?;  // Original error type lost
    Ok(())
}

// Better: preserve error types
fn process() -> Result<(), ProcessError> {
    let data = load_data()
        .map_err(ProcessError::LoadFailed)?;
    Ok(())
}
```

### String as Error Type

```rust
// Detected: anti-pattern
fn parse_config() -> Result<Config, String> {
    Err("Failed to parse".to_string())  // No context!
}

// Better: structured error type
#[derive(Debug)]
enum ConfigError {
    ParseFailed { line: usize, reason: String },
    InvalidFormat,
}

fn parse_config() -> Result<Config, ConfigError> {
    Err(ConfigError::ParseFailed {
        line: 42,
        reason: "Missing required field".into(),
    })
}
```

### Missing Error Context

```rust
// Detected: lacks context
fn load_data() -> Result<Data> {
    let file = File::open(path)?;
    Ok(data)
}

// Better: add context
use anyhow::{Context, Result};

fn load_data() -> Result<Data> {
    let file = File::open(path)
        .context("Failed to open data file")?;
    Ok(data)
}
```

## Panic Patterns

### Rust Panic Detection

```rust
// Detected: panic in production code
fn get_value(key: &str) -> String {
    config.get(key).unwrap()  // Will panic if missing!
}

// Better: return Result
fn get_value(key: &str) -> Result<String, ConfigError> {
    config.get(key)
        .ok_or(ConfigError::MissingKey(key.to_string()))
}
```

#### .expect() with generic message

```rust
// Detected: poor error message
let value = parse_int(s).expect("failed");

// Better: descriptive message
let value = parse_int(s)
    .expect("Failed to parse user input as integer");
```

#### panic!() macro

```rust
// Detected: explicit panic
if data.is_empty() {
    panic!("Data cannot be empty");
}

// Better: return error
if data.is_empty() {
    return Err(ValidationError::EmptyData);
}
```

#### unreachable!()

```rust
// Detected: potential panic
match status {
    Status::Active => handle_active(),
    Status::Inactive => handle_inactive(),
    _ => unreachable!(),  // What if new variant added?
}

// Better: handle explicitly
match status {
    Status::Active => handle_active(),
    Status::Inactive => handle_inactive(),
    Status::Unknown => Err(Error::UnknownStatus),
}
```

#### todo!/unimplemented!()

```rust
// Detected: incomplete code
fn critical_function() {
    todo!()  // Don't ship this!
}

// Better: return NotImplemented error or complete implementation
```

### Severity Context

Panic patterns are evaluated based on context:
- **High-Critical severity**: Production code paths
- **Low severity**: Test code, examples, prototypes

## Configuration

### Enable Error Analysis

```bash
# Include error handling patterns in analysis
debtmap analyze . --filter-categories ErrorHandling
```

### Adjust Severity Thresholds

Configure in `.debtmap.toml`:

```toml
[debt_detection.error_handling]
error_swallowing_severity = "high"
panic_in_production = "critical"
panic_in_tests = "low"
type_erasure_severity = "medium"
```

### Suppression

Suppress known acceptable cases:

```rust
// debtmap:ignore-next-line error-swallowing
let _ = best_effort_cleanup();  // Intentionally ignore errors
```

## Best Practices

**Rust:**
- Use `?` operator for error propagation
- Return `Result<T, E>` instead of panicking
- Add context with `anyhow::Context`
- Use structured error types with `thiserror`
- Reserve panics for truly unrecoverable situations

**Python:**
- Catch specific exception types
- Always log exceptions before swallowing
- Re-raise after logging unless recovery is possible
- Use custom exception classes for domain errors

**JavaScript/TypeScript:**
- Don't silently catch and ignore errors
- Provide error context in catch blocks
- Use structured error classes
- Handle Promise rejections

## Recommendations by Pattern

| Pattern | Severity | Recommendation |
|---------|----------|----------------|
| Error swallowing | Medium-High | Add error handling or logging |
| Type erasure | Low-Medium | Use specific error types with context |
| Panic in production | High-Critical | Return Result or handle gracefully |
| Bare except | Medium-High | Catch specific exceptions |
| Missing context | Low | Add error context with .context() |

## Use Cases

### Audit Error Handling

```bash
# Find all error handling issues
debtmap analyze . --filter-categories ErrorHandling --format markdown
```

### Focus on Critical Issues

```bash
# Show only high-severity error patterns
debtmap analyze . --filter-categories ErrorHandling --min-priority high
```

### Pre-Production Review

```bash
# Ensure no panics in production code
debtmap analyze src/ --filter-categories ErrorHandling | grep -i panic
```

## Troubleshooting

### Too Many False Positives

**Issue:** Test code flagged for acceptable panics

**Solution:**
- Configure separate severity for test paths
- Use suppression comments for intentional patterns
- Exclude test directories if needed

### Missing Detections

**Issue:** Known error swallowing not detected

**Solution:**
- Verify pattern matches detection criteria
- Check if code uses uncommon error handling patterns
- Report pattern to debtmap for inclusion

## See Also

- [Suppression Patterns](suppression-patterns.md) - Suppress false positives
- [Configuration](configuration.md) - Configure error analysis
- [Best Practices](#) - Error handling best practices guide
