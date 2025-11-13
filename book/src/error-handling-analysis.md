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

> **Note:** Panic patterns in test code are automatically detected and assigned **Low priority**. See [Test Code Detection and Handling](#test-code-detection-and-handling) for details.

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

## Test Code Detection and Handling

**Source:** `src/debt/error_swallowing.rs:12`, `src/debt/panic_patterns.rs:12`

Debtmap automatically detects test code across all supported languages and applies lower severity or exclusion to error handling patterns in test contexts. This prevents false positives from legitimate test code practices like using `.unwrap()` for assertions.

### How Test Detection Works

#### Rust Test Detection

Rust test code is identified by:
- **`#[cfg(test)]` modules**: Entire modules marked for test compilation
- **`#[test]` attributes**: Individual test functions
- **`test_` function prefix**: Functions starting with `test_`
- **`in_test_function()` check**: AST analysis to determine if code is within a test function
- **`in_test_module()` check**: AST analysis to determine if code is within a `#[cfg(test)]` module

```rust
// ✅ Automatically detected as test code - lower priority
#[cfg(test)]
mod tests {
    #[test]
    fn test_parsing() {
        let result = "42".parse::<i32>().unwrap();  // LOW priority in tests
        assert_eq!(result, 42);
    }

    fn test_helper() {
        let data = load_test_data().unwrap();  // LOW priority (in test module)
    }
}

// ❌ HIGH priority - production code
pub fn parse_user_input(input: &str) -> i32 {
    input.parse::<i32>().unwrap()  // HIGH priority panic pattern
}
```

#### Python Test Detection

Python test code is identified by:
- **`test_` function prefix**: Functions whose names start with `test_`
- **Test file patterns**: Files matching `test_*.py` or `*_test.py`
- **Test class patterns**: Classes inheriting from `unittest.TestCase` or `pytest` fixtures

```python
# ✅ Automatically detected as test code
def test_user_creation():
    user = create_user("test@example.com")
    assert user is not None  # No error handling needed in tests

class TestUserService:
    def test_registration(self):
        result = register_user({"email": "test@example.com"})
        # Exceptions can propagate in tests
```

#### JavaScript/TypeScript Test Detection

JavaScript/TypeScript test code is identified by:
- **Test file patterns**: Files matching `*.test.js`, `*.test.ts`, `*.spec.js`, `*.spec.ts`
- **Test directory patterns**: Files in `__tests__/`, `test/`, `tests/` directories
- **Test function patterns**: Functions passed to `test()`, `it()`, `describe()` from test frameworks

```javascript
// File: user.test.ts - automatically detected as test code
describe('User Service', () => {
    it('should create user', async () => {
        const user = await createUser('test@example.com');
        expect(user).toBeDefined();  // No error handling required
    });
});
```

### Priority Adjustments for Test Code

When error handling patterns are detected in test code, severity is automatically reduced:

| Pattern | Production Severity | Test Severity | Rationale |
|---------|---------------------|---------------|-----------|
| Unwrap on Result | HIGH | LOW | Tests should fail fast on unexpected errors |
| Unwrap on Option | HIGH | LOW | Tests verify expected values exist |
| Panic in function | CRITICAL | LOW | Test failures are acceptable |
| Bare except (Python) | CRITICAL | MEDIUM | Tests may need catch-all for framework |
| expect() with message | MEDIUM | LOW | Test error messages are internal |
| TODO macro | MEDIUM | LOW | Tests can be work-in-progress |

**Configuration:** Test detection is automatic and enabled by default. Severity adjustments are controlled by `severity_overrides` configuration:

```toml
# Default behavior (automatic)
[[error_handling.severity_overrides]]
pattern = "unwrap"
context = "test"
severity = "low"

# Stricter test enforcement (optional)
[[error_handling.severity_overrides]]
pattern = "unwrap"
context = "test"
severity = "medium"  # Still flag in tests but don't treat as high priority
```

### When Test Detection May Miss Code

Test detection is based on static analysis and may not catch:
- Dynamic test registration (Python `nose` or custom test runners)
- Unusual test naming conventions (e.g., `check_*`, `verify_*`)
- Test helper utilities in non-test files

**Solutions:**

1. **Use suppression comments** for legitimate test helpers:
```rust
// debtmap: ignore - Test utility function
pub fn create_test_user() -> User {
    User::new("test@example.com").unwrap()
}
```

2. **Follow standard naming conventions:**
- Rust: Use `#[test]` or `#[cfg(test)]`
- Python: Prefix with `test_` or use standard test frameworks
- JS/TS: Use `.test.js` or `.spec.ts` file extensions

3. **Configure custom test patterns** (future feature):
```toml
# Planned feature for custom test detection
[[test_detection]]
pattern = "verify_.*"
language = "rust"
```

### Benchmark and Example Code Detection

Similar to test code, benchmark and example code receives lower priority:

**Rust Benchmarks:**
- Files in `benches/` directory
- Functions with `#[bench]` attribute
- Criterion benchmark functions

**Example Code:**
- Files in `examples/` directory
- Documentation examples in doc comments

```rust
// File: benches/parsing_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn parse_benchmark(c: &mut Criterion) {
    c.bench_function("parse", |b| {
        b.iter(|| {
            let result = black_box("42").parse::<i32>().unwrap();  // LOW priority
        });
    });
}
```

### Troubleshooting Test Detection

**Problem:** Legitimate test code still flagged as HIGH severity

**Solution:**
1. Check test naming follows conventions (`#[test]`, `test_*`, `*.test.js`)
2. Verify file is in recognized test location (`tests/`, `__tests__/`)
3. Use suppression comment if detection fails:
```rust
let value = result.unwrap();  // debtmap: ignore - Test assertion
```

**Problem:** Test helpers in shared files flagged

**Solution:**
Move test helpers to test-specific modules or use suppression comments:
```rust
#[cfg(test)]
pub mod test_helpers {
    pub fn setup_test_db() -> Database {
        Database::connect("test_db").unwrap()  // LOW priority (in #[cfg(test)])
    }
}
```

### Error Swallowing in Rust

Debtmap detects seven distinct patterns of error swallowing in Rust, where errors are silently ignored without logging or propagation:

#### 1. IfLetOkNoElse - Missing else branch

```rust
// ❌ Detected: if let Ok without else branch
fn try_update(value: &str) {
    if let Ok(parsed) = value.parse::<i32>() {
        update_value(parsed);
    }
    // Error case silently ignored - no logging or handling
}

// ✅ GOOD: Handle both cases
fn try_update(value: &str) -> Result<()> {
    if let Ok(parsed) = value.parse::<i32>() {
        update_value(parsed);
        Ok(())
    } else {
        Err(anyhow!("Failed to parse value: {}", value))
    }
}
```

#### 2. IfLetOkEmptyElse - Empty else branch

```rust
// ❌ Detected: if let Ok with empty else
fn process_result(result: Result<Data, Error>) {
    if let Ok(data) = result {
        process(data);
    } else {
        // Empty else - error silently swallowed
    }
}

// ✅ GOOD: Log the error
fn process_result(result: Result<Data, Error>) {
    if let Ok(data) = result {
        process(data);
    } else {
        log::error!("Failed to process: {:?}", result);
    }
}
```

#### 3. LetUnderscoreResult - Discarding Result with let _

```rust
// ❌ Detected: Result discarded with let _
fn save_data(data: &Data) {
    let _ = fs::write("data.json", serde_json::to_string(data).unwrap());
    // Write failure silently ignored
}

// ✅ GOOD: Handle or propagate the error
fn save_data(data: &Data) -> Result<()> {
    fs::write("data.json", serde_json::to_string(data)?)
        .context("Failed to save data")?;
    Ok(())
}
```

#### 4. OkMethodDiscard - Calling .ok() and discarding

```rust
// ❌ Detected: .ok() called but result discarded
fn try_parse(s: &str) -> Option<i32> {
    s.parse::<i32>().ok();  // Result immediately discarded
    None
}

// ✅ GOOD: Use the Ok value or log the error
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

#### 5. MatchIgnoredErr - Match with ignored error variant

```rust
// ❌ Detected: match with _ in Err branch
fn try_load(path: &Path) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(content) => Some(content),
        Err(_) => None,  // Error details ignored
    }
}

// ✅ GOOD: Log the error with context
fn try_load(path: &Path) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(content) => Some(content),
        Err(e) => {
            log::error!("Failed to read {}: {}", path.display(), e);
            None
        }
    }
}
```

#### 6. UnwrapOrNoLog - .unwrap_or() without logging

```rust
// ❌ Detected: unwrap_or without logging
fn get_config_value(key: &str) -> String {
    load_config()
        .and_then(|c| c.get(key))
        .unwrap_or_else(|| "default".to_string())
    // Error silently replaced with default
}

// ✅ GOOD: Log before falling back to default
fn get_config_value(key: &str) -> String {
    match load_config().and_then(|c| c.get(key)) {
        Ok(value) => value,
        Err(e) => {
            log::warn!("Config key '{}' not found: {}. Using default.", key, e);
            "default".to_string()
        }
    }
}
```

#### 7. UnwrapOrDefaultNoLog - .unwrap_or_default() without logging

```rust
// ❌ Detected: unwrap_or_default without logging
fn load_settings() -> Settings {
    read_settings_file().unwrap_or_default()
    // Error silently replaced with default settings
}

// ✅ GOOD: Log the fallback to defaults
fn load_settings() -> Settings {
    match read_settings_file() {
        Ok(settings) => settings,
        Err(e) => {
            log::warn!("Failed to load settings: {}. Using defaults.", e);
            Settings::default()
        }
    }
}
```

**Summary of Error Swallowing Patterns:**

| Pattern | Description | Common Cause |
|---------|-------------|--------------|
| IfLetOkNoElse | `if let Ok(..)` without else | Quick prototyping, forgotten error path |
| IfLetOkEmptyElse | `if let Ok(..)` with empty else | Incomplete implementation |
| LetUnderscoreResult | `let _ = result` | Intentional ignore without thought |
| OkMethodDiscard | `.ok()` result not used | Misunderstanding of .ok() semantics |
| MatchIgnoredErr | `Err(_) => ...` with no logging | Generic error handling |
| UnwrapOrNoLog | `.unwrap_or()` without logging | Convenience over observability |
| UnwrapOrDefaultNoLog | `.unwrap_or_default()` without logging | Default fallback without visibility |

All these patterns are detected at **Medium to High priority** depending on context, as they represent lost error information that makes debugging difficult.

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

### Contextlib Suppress Detection

Python's `contextlib.suppress()` intentionally silences exceptions, which can hide errors:

```python
from contextlib import suppress

# ❌ MEDIUM: contextlib.suppress hides errors
def cleanup_temp_files(paths):
    for path in paths:
        with suppress(FileNotFoundError, PermissionError):
            os.remove(path)  # Detected: ContextlibSuppress
            # Errors silently suppressed - no visibility into failures

# ✅ GOOD: Log suppressed errors
def cleanup_temp_files(paths):
    for path in paths:
        try:
            os.remove(path)
        except FileNotFoundError:
            logger.debug(f"File already deleted: {path}")
        except PermissionError as e:
            logger.warning(f"Permission denied removing {path}: {e}")
        except Exception as e:
            logger.error(f"Unexpected error removing {path}: {e}")

# ✅ ACCEPTABLE: Use suppress only for truly ignorable cases
def best_effort_cleanup(paths):
    """Best-effort cleanup - failures are expected and acceptable."""
    for path in paths:
        with suppress(OSError):  # OK if documented and intentional
            os.remove(path)
```

**When contextlib.suppress is acceptable:**
- Cleanup operations where failures are genuinely unimportant
- Operations explicitly documented as "best effort"
- Code where logging would create noise without value

**When to avoid contextlib.suppress:**
- Production code where error visibility matters
- Operations where partial failure should be noticed
- Any case where debugging might be needed later

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

### JavaScript/TypeScript Async Error Detection Status

> **Implementation Status: Limited**
>
> JavaScript/TypeScript async error handling detection is **NOT fully implemented** in the current release. Only test-specific unresolved promise detection is available.
>
> **What IS Implemented:**
> - Test-specific async pattern detection (unresolved promises in test functions)
> - Basic try/catch without error handling
> - Resource management (timer/event listener leaks)
>
> **What IS NOT Implemented:**
> - UnhandledPromiseRejection detection in production code
> - MissingAwait detection in production code
> - Promise chain error handling analysis
>
> The JavaScript/TypeScript examples below document **planned features** for future releases. For now, use external linting tools (ESLint with promise plugins) for comprehensive async error detection.

**Current Language Support Comparison:**

| Feature | Rust | Python | JavaScript/TypeScript |
|---------|------|--------|----------------------|
| Basic error swallowing | ✅ Full | ✅ Full | ❌ Not implemented |
| Panic/exception patterns | ✅ Full | ✅ Full | ❌ Not implemented |
| Async error detection | ✅ Full | ❌ N/A | ⚠️ Tests only |
| Error propagation analysis | ✅ Full | ✅ Basic | ❌ Not implemented |
| Context loss detection | ✅ Full | ⚠️ Limited | ❌ Not implemented |

**Source References:**
- Rust async error detection: `src/debt/async_errors.rs`
- Python error handling: `src/debt/python_error_handling.rs`
- JS/TS test async detection: `src/analyzers/javascript/detectors/testing/`

### Unhandled Promise Rejections (Planned Feature)

> **Note:** The following pattern is documented for future implementation. It is **not currently detected** by debtmap.

```javascript
// ❌ PLANNED: Unhandled promise rejection detection
async function loadUserData(userId) {
    const response = await fetch(`/api/users/${userId}`);
    // If fetch rejects, promise is unhandled
    return response.json();
}

loadUserData(123);  // Future: Will detect UnhandledPromiseRejection

// ✅ RECOMMENDED: Handle rejections
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

### Missing Await Detection (Planned Feature)

> **Note:** This pattern is documented for future implementation. It is **not currently detected** by debtmap.

```javascript
// ❌ PLANNED: Missing await detection
async function saveAndNotify(data) {
    await saveToDatabase(data);
    sendNotification(data.userId);  // Future: Will detect MissingAwait
    // Function returns before notification completes
}

// ✅ RECOMMENDED: Await all async operations
async function saveAndNotify(data) {
    await saveToDatabase(data);
    await sendNotification(data.userId);
}
```

**Current Workaround:** Use ESLint with these plugins for JavaScript/TypeScript async error detection:
- `eslint-plugin-promise`
- `@typescript-eslint/no-floating-promises`
- `@typescript-eslint/no-misused-promises`

### Async Rust Error Handling

Debtmap detects five async-specific error handling patterns in Rust:

#### 1. DroppedFuture - Future dropped without awaiting

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
```

#### 2. UnhandledJoinHandle - Spawned task without join

```rust
// ❌ HIGH: Task spawned but handle never checked
async fn background_sync() {
    tokio::spawn(async {
        sync_to_database().await  // Detected: UnhandledJoinHandle
    });
    // Handle dropped - can't detect if task panicked or failed
}

// ✅ GOOD: Store and check join handle
async fn background_sync() -> Result<()> {
    let handle = tokio::spawn(async {
        sync_to_database().await
    });
    handle.await?  // Wait for completion and check for panic
}
```

#### 3. SilentTaskPanic - Task panic without monitoring

```rust
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

#### 4. SpawnWithoutJoin - Spawning without storing handle

```rust
// ❌ MEDIUM: Spawn without storing handle
async fn fire_and_forget_tasks(items: Vec<Item>) {
    for item in items {
        tokio::spawn(process_item(item));  // Detected: SpawnWithoutJoin
        // No way to check task completion or errors
    }
}

// ✅ GOOD: Collect handles for later checking
async fn process_tasks_with_monitoring(items: Vec<Item>) -> Result<()> {
    let handles: Vec<_> = items.into_iter()
        .map(|item| tokio::spawn(process_item(item)))
        .collect();

    for handle in handles {
        handle.await??;
    }
    Ok(())
}
```

#### 5. SelectBranchIgnored - Select branch without error handling

```rust
// ❌ MEDIUM: tokio::select! branch error ignored
async fn process_with_timeout(data: Data) {
    tokio::select! {
        result = process_data(data) => {
            // Detected: SelectBranchIgnored
            // result could be Err but not checked
        }
        _ = tokio::time::sleep(Duration::from_secs(5)) => {
            println!("Timeout");
        }
    }
}

// ✅ GOOD: Handle errors in select branches
async fn process_with_timeout(data: Data) -> Result<()> {
    tokio::select! {
        result = process_data(data) => {
            result?;  // Propagate error
            Ok(())
        }
        _ = tokio::time::sleep(Duration::from_secs(5)) => {
            Err(anyhow!("Processing timeout after 5s"))
        }
    }
}
```

**Async Error Pattern Summary:**

| Pattern | Severity | Description | Common in |
|---------|----------|-------------|-----------|
| DroppedFuture | High | Future result ignored | Fire-and-forget spawns |
| UnhandledJoinHandle | High | JoinHandle never checked | Background tasks |
| SilentTaskPanic | High | Task panic not monitored | Unmonitored spawns |
| SpawnWithoutJoin | Medium | Handle not stored | Quick prototypes |
| SelectBranchIgnored | Medium | select! branch error ignored | Concurrent operations |

All async error patterns emphasize the importance of properly handling errors in concurrent Rust code, where failures can easily go unnoticed.

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

**By default, all error handling detection is fully enabled.** The configuration options below are primarily used to selectively disable specific patterns during gradual adoption or for specific project needs.

Configure error handling analysis in `.debtmap.toml`:

```toml
[error_handling]
# All detection patterns are enabled by default (all default to true)
detect_panic_patterns = true     # Rust unwrap/expect/panic detection
detect_swallowing = true         # Silent exception handling
detect_async_errors = true       # Unhandled promises, dropped futures
detect_context_loss = true       # Error propagation without context
detect_propagation = true        # Error propagation analysis

# Disable specific patterns for gradual adoption
# detect_async_errors = false
```

All error handling patterns are detected by default with the `ErrorSwallowing` debt category (weight 4). The configuration is fully implemented and functional - use it primarily to disable specific patterns when needed.

### Advanced Configuration: Custom Patterns and Severity Overrides

**Source:** `src/config/detection.rs:275-374`

Beyond the basic detection toggles, `ErrorHandlingConfig` supports two advanced configuration options for customizing error pattern detection:

#### Custom Error Patterns

Define project-specific error patterns to detect anti-patterns unique to your codebase:

```toml
[[error_handling.custom_patterns]]
name = "log_and_ignore"
pattern = "log::error.*\n.*return None"
pattern_type = "method_call"
severity = "medium"
description = "Error logged but silently converted to None without propagation"
remediation = "Consider propagating the error or using Result<Option<T>>"

[[error_handling.custom_patterns]]
name = "custom_unwrap_macro"
pattern = "must_succeed!"
pattern_type = "macro_name"
severity = "high"
description = "Custom macro that can panic in production"
remediation = "Replace with proper error handling using Result"

[[error_handling.custom_patterns]]
name = "swallow_on_match"
pattern = "match.*\\{\n.*Ok\\(.*\\).*=>.*,\n.*Err\\(_\\).*=>.*\\(\\),?\n.*\\}"
pattern_type = "match_expression"
severity = "medium"
description = "Match expression with empty tuple on error"
remediation = "Log the error or propagate with ?"
```

**Configuration Fields** (`ErrorPatternConfig` struct):
- `name`: Pattern identifier (used in reports)
- `pattern`: Regex or matcher string
- `pattern_type`: Type of pattern to match
  - `function_name` - Match function calls
  - `macro_name` - Match macro invocations
  - `method_call` - Match method calls
  - `match_expression` - Match expression patterns
- `severity`: `low`, `medium`, `high`, or `critical`
- `description`: What this pattern detects
- `remediation`: Optional suggested fix

#### Severity Overrides

Override severity levels for specific contexts (tests, benchmarks, examples):

```toml
[[error_handling.severity_overrides]]
pattern = "unwrap"
context = "test"
severity = "low"

[[error_handling.severity_overrides]]
pattern = "panic"
context = "test"
severity = "low"

[[error_handling.severity_overrides]]
pattern = "expect"
context = "benchmark"
severity = "low"

[[error_handling.severity_overrides]]
pattern = "todo"
context = "example"
severity = "low"
```

**Configuration Fields** (`SeverityOverride` struct):
- `pattern`: Pattern to match (e.g., `unwrap`, `panic`, `expect`, `todo`)
- `context`: Where override applies
  - `test` - Code in test modules/functions
  - `benchmark` - Code in benchmark functions
  - `example` - Code in example files/directories
- `severity`: New severity level (`low`, `medium`, `high`, `critical`)

**Use Cases:**
- **Custom patterns**: Detect project-specific error handling anti-patterns (custom macros, team conventions)
- **Severity overrides**: Reduce noise from panic patterns in test code while keeping strict detection in production code
- **Context-aware analysis**: Apply different standards to test/benchmark/example code vs production

**Example: Stricter Test Error Handling**

```toml
# Still detect test unwraps, but at lower severity
[[error_handling.severity_overrides]]
pattern = "unwrap"
context = "test"
severity = "medium"  # Instead of default high

# Custom pattern for test-specific anti-pattern
[[error_handling.custom_patterns]]
name = "test_panic_without_message"
pattern = "panic!\\(\\)"
pattern_type = "macro_name"
severity = "medium"
description = "Test panic without descriptive message"
remediation = "Add message explaining what failed: panic!(\"Expected condition X\")"
```

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

### JavaScript/TypeScript Errors Not Being Detected

**Problem:** JavaScript/TypeScript async error patterns (unhandled promises, missing await) not appearing in reports

**Honest Answer:** JavaScript/TypeScript async error detection is **not fully implemented** in the current release.

**What IS detected:**
- Test-specific unresolved promises (in `*.test.js`, `*.spec.ts` files)
- Resource management issues (timer leaks, event listener leaks)
- Basic try/catch without error handling

**What IS NOT detected:**
- Unhandled promise rejections in production code
- Missing await in production code
- Promise chain error handling issues

**Current Workarounds:**

1. **Use ESLint for JavaScript/TypeScript async error detection:**
   ```json
   {
     "plugins": ["promise", "@typescript-eslint"],
     "rules": {
       "promise/catch-or-return": "error",
       "promise/no-return-wrap": "error",
       "@typescript-eslint/no-floating-promises": "error",
       "@typescript-eslint/no-misused-promises": "error"
     }
   }
   ```

2. **Combine debtmap with ESLint:**
   ```bash
   # Use debtmap for Rust/Python error handling
   debtmap analyze --languages rust,python

   # Use ESLint for JavaScript/TypeScript
   eslint src/**/*.{js,ts} --config eslint.config.json
   ```

3. **Track progress on JS/TS support:**
   - Check debtmap releases for async error detection updates
   - The feature is planned but not yet available

**Why the documentation shows JS/TS examples:** The examples document the planned behavior for future releases. They serve as design documentation and best practices guidance, even though detection is not yet implemented.

## Related Topics

- [Configuration](configuration.md) - Complete `.debtmap.toml` reference
- [Suppression Patterns](suppression-patterns.md) - Suppress false positives
- [Scoring Strategies](scoring-strategies.md) - How error handling affects risk scores
- [Coverage Integration](coverage-integration.md) - Detect untested error paths
- [CLI Reference](cli-reference.md) - Command-line options for error analysis
- [Troubleshooting](troubleshooting.md) - General debugging guide
