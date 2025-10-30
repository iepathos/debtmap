# Error Handling Analysis

Debtmap provides comprehensive error handling analysis across all supported languages (Rust, Python, JavaScript, TypeScript), detecting anti-patterns that lead to silent failures, production panics, and difficult-to-debug issues.

## Overview

Error handling issues are classified as **ErrorSwallowing** debt with **Major severity** (weight 4), reflecting their significant impact on code reliability and debuggability. Debtmap detects:

- **Error swallowing**: Exception handlers that silently catch errors without logging or re-raising
- **Panic patterns**: Rust code that can panic in production (unwrap, expect, panic!)
- **Error propagation issues**: Missing error context in Result chains
- **Async error handling**: Unhandled promise rejections, dropped futures, missing await
- **Python-specific patterns**: Bare except clauses, silent exception handling

All error handling patterns are filtered intelligently - code detected in test modules (e.g., `#[cfg(test)]`, `test_` prefixes) receives lower priority or is excluded entirely.

## Rust Error Handling Analysis

### Panic Pattern Detection

Debtmap identifies Rust code that can panic at runtime instead of returning `Result`:

**Detected patterns:**

```rust
// ❌ CRITICAL: Direct panic in production code
fn process_data(value: Option<i32>) -> i32 {
    panic!("not implemented");  // Detected: PanicInNonTest
}

// ❌ HIGH: Unwrap on Result
fn read_config(path: &Path) -> Config {
    let content = fs::read_to_string(path).unwrap();  // Detected: UnwrapOnResult
    parse_config(&content)
}

// ❌ HIGH: Unwrap on Option
fn get_user(id: u32) -> User {
    users.get(&id).unwrap()  // Detected: UnwrapOnOption
}

// ❌ MEDIUM: Expect with generic message
fn parse_value(s: &str) -> i32 {
    s.parse().expect("parse failed")  // Detected: ExpectWithGenericMessage
}

// ❌ MEDIUM: TODO in production
fn calculate_tax(amount: f64) -> f64 {
    todo!("implement tax calculation")  // Detected: TodoInProduction
}
```

**Recommended alternatives:**

```rust
// ✅ GOOD: Propagate errors with ?
fn read_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path)?;
    parse_config(&content)
}

// ✅ GOOD: Handle Option explicitly
fn get_user(id: u32) -> Result<User> {
    users.get(&id)
        .ok_or_else(|| anyhow!("User {} not found", id))
}

// ✅ GOOD: Add meaningful context
fn parse_value(s: &str) -> Result<i32> {
    s.parse()
        .with_context(|| format!("Failed to parse '{}' as integer", s))
}
```

**Test code exceptions:**

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_parsing() {
        let result = "42".parse::<i32>().unwrap();  // ✅ OK in tests (LOW priority)
        assert_eq!(result, 42);
    }
}
```

Debtmap detects `#[cfg(test)]` attributes and test function contexts, automatically assigning **Low priority** to panic patterns in test code.

### Error Propagation Analysis

Debtmap detects missing error context in Result chains:

```rust
// ❌ Missing context - which file failed? What was the error?
fn load_multiple_configs(paths: &[PathBuf]) -> Result<Vec<Config>> {
    paths.iter()
        .map(|p| fs::read_to_string(p))  // Error loses file path information
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .map(|c| parse_config(&c))  // Error loses which config failed
        .collect()
}

// ✅ GOOD: Preserve context through the chain
fn load_multiple_configs(paths: &[PathBuf]) -> Result<Vec<Config>> {
    paths.iter()
        .map(|p| {
            fs::read_to_string(p)
                .with_context(|| format!("Failed to read config from {}", p.display()))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .enumerate()
        .map(|(i, content)| {
            parse_config(&content)
                .with_context(|| format!("Failed to parse config #{}", i))
        })
        .collect()
}
```

**Best practices:**
- Use `.context()` or `.with_context()` from `anyhow` or `thiserror`
- Include relevant values in error messages (file paths, indices, input values)
- Maintain error context at each transformation in the chain

### Error Swallowing in Rust

```rust
// ❌ Silent error swallowing
fn try_parse(s: &str) -> Option<i32> {
    match s.parse::<i32>() {
        Ok(v) => Some(v),
        Err(_) => None,  // Detected: Error swallowed without logging
    }
}

// ✅ GOOD: Log the error
fn try_parse(s: &str) -> Option<i32> {
    match s.parse::<i32>() {
        Ok(v) => Some(v),
        Err(e) => {
            log::warn!("Failed to parse '{}': {}", s, e);
            None
        }
    }
}
```

## Python Error Handling Analysis

### Bare Except Clause Detection

Python's bare `except:` catches all exceptions, including system exits and keyboard interrupts:

```python
# ❌ CRITICAL: Bare except catches everything
def process_file(path):
    try:
        with open(path) as f:
            return f.read()
    except:  # Detected: BareExceptClause
        return None  # Catches SystemExit, KeyboardInterrupt, etc.

# ❌ HIGH: Catching Exception is too broad
def load_config(path):
    try:
        return yaml.load(open(path))
    except Exception:  # Detected: OverlyBroadException
        return {}  # Silent failure loses error information

# ✅ GOOD: Specific exception types
def process_file(path):
    try:
        with open(path) as f:
            return f.read()
    except FileNotFoundError:
        log.error(f"File not found: {path}")
        return None
    except PermissionError:
        log.error(f"Permission denied: {path}")
        return None
```

**Why bare except is dangerous:**
- Catches `SystemExit` (prevents clean shutdown)
- Catches `KeyboardInterrupt` (prevents Ctrl+C)
- Catches `GeneratorExit` (breaks generator protocol)
- Masks programming errors like `NameError`, `AttributeError`

**Best practices:**
- Always specify exception types: `except ValueError`, `except (TypeError, KeyError)`
- Use `except Exception` only when truly catching all application errors
- Never use bare `except:` in production code
- Log exceptions with full context before suppressing

### Silent Exception Handling

```python
# ❌ Silent exception handling
def get_user_age(user_id):
    try:
        user = db.get_user(user_id)
        return user.age
    except:  # Detected: SilentException (no logging, no re-raise)
        pass

# ✅ GOOD: Log and provide meaningful default
def get_user_age(user_id):
    try:
        user = db.get_user(user_id)
        return user.age
    except UserNotFound:
        logger.warning(f"User {user_id} not found")
        return None
    except DatabaseError as e:
        logger.error(f"Database error fetching user {user_id}: {e}")
        raise  # Re-raise for caller to handle
```

### Exception Flow Analysis

Debtmap tracks exception propagation through Python codebases to identify functions that can raise exceptions without proper handling. This analysis helps ensure that exceptions are either caught at appropriate levels or documented in the function's interface.

```python
# Potential issue: Exceptions may propagate unhandled
def process_batch(items):
    for item in items:
        validate_item(item)  # Can raise ValueError
        transform_item(item)  # Can raise TransformError
        save_item(item)  # Can raise DatabaseError

# ✅ GOOD: Handle exceptions appropriately
def process_batch(items):
    results = {"success": 0, "failed": 0}
    for item in items:
        try:
            validate_item(item)
            transform_item(item)
            save_item(item)
            results["success"] += 1
        except ValueError as e:
            logger.warning(f"Invalid item {item.id}: {e}")
            results["failed"] += 1
        except (TransformError, DatabaseError) as e:
            logger.error(f"Failed to process item {item.id}: {e}")
            results["failed"] += 1
            # Optionally re-raise critical errors
            if isinstance(e, DatabaseError):
                raise
    return results
```

## Async Error Handling

### Unhandled Promise Rejections (JavaScript/TypeScript)

**Note:** JavaScript and TypeScript support in debtmap currently focuses on complexity analysis and basic error patterns. Advanced async error handling detection (unhandled promise rejections, missing await) is primarily implemented for Rust async code. Enhanced JavaScript/TypeScript async error detection is planned for future releases.

```javascript
// ❌ CRITICAL: Unhandled promise rejection
async function loadUserData(userId) {
    const response = await fetch(`/api/users/${userId}`);
    // If fetch rejects, promise is unhandled
    return response.json();
}

loadUserData(123);  // Detected: UnhandledPromiseRejection

// ✅ GOOD: Handle rejections
async function loadUserData(userId) {
    try {
        const response = await fetch(`/api/users/${userId}`);
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        return await response.json();
    } catch (error) {
        console.error(`Failed to load user ${userId}:`, error);
        throw error;  // Re-throw or return default
    }
}

loadUserData(123).catch(err => {
    console.error("Top-level error handler:", err);
});
```

### Missing Await Detection

```javascript
// ❌ HIGH: Missing await - promise dropped
async function saveAndNotify(data) {
    await saveToDatabase(data);
    sendNotification(data.userId);  // Detected: MissingAwait
    // Function returns before notification completes
}

// ✅ GOOD: Await all async operations
async function saveAndNotify(data) {
    await saveToDatabase(data);
    await sendNotification(data.userId);
}
```

### Async Rust Error Handling

```rust
// ❌ HIGH: Dropped future without error handling
async fn process_requests(requests: Vec<Request>) {
    for req in requests {
        tokio::spawn(async move {
            handle_request(req).await  // Detected: DroppedFuture
            // Errors silently dropped
        });
    }
}

// ✅ GOOD: Join handles and propagate errors
async fn process_requests(requests: Vec<Request>) -> Result<()> {
    let handles: Vec<_> = requests.into_iter()
        .map(|req| {
            tokio::spawn(async move {
                handle_request(req).await
            })
        })
        .collect();

    for handle in handles {
        handle.await??;  // Propagate both JoinError and handler errors
    }
    Ok(())
}

// ❌ HIGH: Task panic silently ignored
tokio::spawn(async {
    panic!("task failed");  // Detected: SilentTaskPanic
});

// ✅ GOOD: Handle task panics
let handle = tokio::spawn(async {
    critical_operation().await
});

match handle.await {
    Ok(Ok(result)) => println!("Success: {:?}", result),
    Ok(Err(e)) => eprintln!("Task failed: {}", e),
    Err(e) => eprintln!("Task panicked: {}", e),
}
```

## Severity Levels and Prioritization

Error handling issues are assigned severity based on their impact:

| Pattern | Severity | Weight | Priority | Rationale |
|---------|----------|--------|----------|-----------|
| Panic in production | CRITICAL | 4 | Critical | Crashes the process |
| Bare except clause | CRITICAL | 4 | Critical | Masks system signals |
| Silent task panic | CRITICAL | 4 | Critical | Hidden failures |
| Unwrap on Result/Option | HIGH | 4 | High | Likely to panic |
| Dropped future | HIGH | 4 | High | Lost error information |
| Unhandled promise rejection | HIGH | 4 | High | Silently fails |
| Error swallowing | MEDIUM | 4 | Medium | Loses debugging context |
| Missing error context | MEDIUM | 4 | Medium | Hard to debug |
| Expect with generic message | MEDIUM | 4 | Medium | Uninformative errors |
| TODO in production | MEDIUM | 4 | Medium | Incomplete implementation |

All ErrorSwallowing debt has **weight 4** (Major severity), but individual patterns receive different priorities based on production impact.

### Integration with Risk Scoring

Error handling issues contribute to the `debt_factor` in Debtmap's risk scoring formula:

```
risk_score = (complexity_factor * 0.4) + (debt_factor * 0.3) + (coverage_factor * 0.3)

where debt_factor includes:
- ErrorSwallowing count * weight (4)
- Combined with other debt types
```

**Compound risk example:**

```rust
// HIGH RISK: High complexity + error swallowing + low coverage
fn process_transaction(tx: Transaction) -> bool {  // Cyclomatic: 12, Cognitive: 18
    if tx.amount > 1000 {
        if tx.verified {
            if validate_funds(&tx).unwrap() {  // ❌ Panic pattern
                if tx.user_type == "premium" {
                    match apply_premium_discount(&tx) {
                        Ok(_) => {},
                        Err(_) => return false,  // ❌ Error swallowed
                    }
                }
                charge_account(&tx).unwrap();  // ❌ Another panic
                return true;
            }
        }
    }
    false
}
// Coverage: 45% (untested error paths)
// Risk Score: Very High (complexity + error handling + coverage gaps)
```

This function would be flagged as **Priority 1** in Debtmap's output due to:
- High cyclomatic complexity (12)
- Multiple panic patterns (unwrap calls)
- Error swallowing (ignored Result)
- Coverage gaps in error handling paths

## Configuration

### Error Handling Configuration Options

Configure error handling analysis in `.debtmap.toml`:

```toml
[error_handling]
# Enable/disable specific detection patterns (all enabled by default)
detect_panic_patterns = true     # Rust unwrap/expect/panic detection
detect_swallowing = true         # Silent exception handling
detect_async_errors = true       # Unhandled promises, dropped futures
detect_context_loss = true       # Error propagation without context
detect_propagation = true        # Error propagation analysis

# Disable specific patterns for gradual adoption
# detect_async_errors = false
```

**Note:** The `[error_handling]` configuration is currently in development. Most error handling patterns are detected by default with `ErrorSwallowing` debt category (weight 4). Per-pattern severity customization is planned for future releases.

## Detection Examples

### What Gets Detected vs. Not Detected

**Rust examples:**

```rust
// ❌ Detected: unwrap() in production code
pub fn get_config() -> Config {
    load_config().unwrap()
}

// ✅ Not detected: ? operator (proper error propagation)
pub fn get_config() -> Result<Config> {
    load_config()?
}

// ✅ Not detected: unwrap() in test
#[test]
fn test_config() {
    let config = load_config().unwrap();  // OK in tests
    assert_eq!(config.port, 8080);
}

// ❌ Detected: expect() with generic message
let value = map.get("key").expect("missing");

// ✅ Not detected: expect() with descriptive context
let value = map.get("key")
    .expect("Configuration must contain 'key' field");
```

**Python examples:**

```python
# ❌ Detected: bare except
try:
    risky_operation()
except:
    pass

# ✅ Not detected: specific exception
try:
    risky_operation()
except ValueError:
    handle_value_error()

# ❌ Detected: silent exception (no logging/re-raise)
try:
    db.save(record)
except DatabaseError:
    pass  # Silent failure

# ✅ Not detected: logged exception
try:
    db.save(record)
except DatabaseError as e:
    logger.error(f"Failed to save record: {e}")
    raise
```

## Suppression Patterns

For cases where error handling patterns are intentional, use suppression comments:

**Rust:**

```rust
// debtmap: ignore - Unwrap is safe here due to prior validation
let value = validated_map.get("key").unwrap();
```

**Python:**

```python
try:
    experimental_feature()
except:  # debtmap: ignore - Intentional catch-all during migration
    use_fallback()
```

See [Suppression Patterns](suppression-patterns.md) for complete syntax and usage.

## Best Practices

### Rust Error Handling

1. **Prefer `?` operator over unwrap/expect**
   ```rust
   // Instead of: fs::read_to_string(path).unwrap()
   // Use: fs::read_to_string(path)?
   ```

2. **Use anyhow for application errors, thiserror for libraries**
   ```rust
   use anyhow::{Context, Result};

   fn load_data(path: &Path) -> Result<Data> {
       let content = fs::read_to_string(path)
           .with_context(|| format!("Failed to read {}", path.display()))?;
       parse_data(&content)
           .context("Invalid data format")
   }
   ```

3. **Add context at each error boundary**
   ```rust
   .with_context(|| format!("meaningful message with {}", value))
   ```

4. **Handle Option explicitly**
   ```rust
   map.get(key).ok_or_else(|| anyhow!("Missing key: {}", key))?
   ```

### Python Error Handling

1. **Always use specific exception types**
   ```python
   except (ValueError, KeyError) as e:
   ```

2. **Log before suppressing**
   ```python
   except DatabaseError as e:
       logger.error(f"Database operation failed: {e}", exc_info=True)
       # Then decide: re-raise, return default, or handle
   ```

3. **Avoid bare except completely**
   ```python
   # If you must catch everything:
   except Exception as e:  # Not bare except:
       logger.exception("Unexpected error")
       raise
   ```

4. **Use context managers for resource cleanup**
   ```python
   with open(path) as f:  # Ensures cleanup even on exception
       process(f)
   ```

### JavaScript/TypeScript Error Handling

1. **Always handle promise rejections**
   ```javascript
   fetchData().catch(err => console.error(err));
   // Or use try/catch with async/await
   ```

2. **Use async/await consistently**
   ```javascript
   async function process() {
       try {
           const data = await fetchData();
           await saveData(data);
       } catch (error) {
           console.error("Failed:", error);
           throw error;
       }
   }
   ```

3. **Don't forget await**
   ```javascript
   await asyncOperation();  // Don't drop promises
   ```

## Improving Error Handling Based on Debtmap Reports

### Workflow

1. **Run analysis with error focus**
   ```bash
   debtmap analyze --filter-categories ErrorSwallowing
   ```

2. **Review priority issues first**
   - Address CRITICAL (panic in production, bare except) immediately
   - Schedule HIGH (unwrap, dropped futures) for next sprint
   - Plan MEDIUM (missing context) for gradual improvement

3. **Fix systematically**
   - One file or module at a time
   - Add tests as you improve error handling
   - Run debtmap after each fix to verify

4. **Validate improvements**
   ```bash
   # Before fixes
   debtmap analyze --output before.json

   # After fixes
   debtmap analyze --output after.json

   # Compare
   debtmap compare before.json after.json
   ```

### Migration Strategy for Legacy Code

```toml
# .debtmap.toml - Gradual adoption
[error_handling]
# Start with just critical panic patterns
detect_panic_patterns = true
detect_swallowing = false      # Add later
detect_async_errors = false    # Add later
detect_context_loss = false    # Add later

# After fixing panic patterns, enable error swallowing detection
# detect_swallowing = true

# Eventually enable all patterns
# detect_swallowing = true
# detect_async_errors = true
# detect_context_loss = true
# detect_propagation = true
```

Track progress over time:
```bash
# Weekly error handling health check
debtmap analyze --filter-categories ErrorSwallowing | tee weekly-error-health.txt
```

## Troubleshooting

### Too Many False Positives in Test Code

**Problem:** Debtmap flagging `unwrap()` in test functions

**Solution:** Debtmap should automatically detect test code via:
- `#[cfg(test)]` modules in Rust
- `#[test]` attributes
- `test_` function name prefix in Python
- `*.test.ts`, `*.spec.js` file patterns

If false positives persist:
```rust
// Use suppression comment
let value = result.unwrap();  // debtmap: ignore - Test assertion
```

### Error Patterns Not Being Detected

**Problem:** Known error patterns not appearing in report

**Causes and solutions:**

1. **Language support not enabled**
   ```bash
   debtmap analyze --languages rust,python,javascript
   ```

2. **Pattern disabled in config**
   ```toml
   [error_handling]
   detect_panic_patterns = true
   detect_swallowing = true
   detect_async_errors = true  # Ensure relevant detectors are enabled
   ```

3. **Suppression comment present**
   - Check for `debtmap: ignore` comments
   - Review `.debtmap.toml` ignore patterns

### Disagreement with Severity Levels

**Problem:** Severity feels too high/low for your codebase

**Solution:** Customize in `.debtmap.toml`:

```toml
[debt_categories.ErrorSwallowing]
weight = 2  # Reduce from default 4 to Warning level
severity = "Warning"

# Or increase for stricter enforcement
# weight = 5
# severity = "Critical"
```

### Can't Find Which Line Has the Issue

**Problem:** Debtmap reports error at wrong line number

**Causes:**
- Source code changed since analysis
- Parser approximation for line numbers

**Solutions:**
1. Re-run analysis: `debtmap analyze`
2. Search for pattern: `rg "\.unwrap\(\)" src/`
3. Enable debug logging: `debtmap analyze --log-level debug`

### Validating Error Handling Improvements

**Problem:** Unsure if fixes actually improved code quality

**Solution:** Use compare workflow:

```bash
# Baseline before fixes
git checkout main
debtmap analyze --output baseline.json

# After fixes
git checkout feature/improve-errors
debtmap analyze --output improved.json

# Compare reports
debtmap compare baseline.json improved.json
```

Look for:
- Reduced ErrorSwallowing debt count
- Lower risk scores for affected functions
- Improved coverage of error paths (if running with coverage)

## Related Topics

- [Configuration](configuration.md) - Complete `.debtmap.toml` reference
- [Suppression Patterns](suppression-patterns.md) - Suppress false positives
- [Scoring Strategies](scoring-strategies.md) - How error handling affects risk scores
- [Coverage Integration](coverage-integration.md) - Detect untested error paths
- [CLI Reference](cli-reference.md) - Command-line options for error analysis
- [Troubleshooting](troubleshooting.md) - General debugging guide
