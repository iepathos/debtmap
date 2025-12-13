---
number: 263
title: Critical Unwrap Elimination - Lock Safety
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-12-13
---

# Specification 263: Critical Unwrap Elimination - Lock Safety

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

Debtmap contains 1,530 `unwrap()` calls across the codebase. Of these, 33 are **critical lock unwraps** that can cause cascading production failures:

1. **Lock poisoning** - If a thread panics while holding a lock, subsequent `.lock().unwrap()` calls will panic
2. **Double unwraps** - Patterns like `.lock().unwrap().take().unwrap()` multiply failure modes
3. **No context** - Panics provide no actionable error messages

**Critical Locations:**

| File | Lines | Pattern | Risk |
|------|-------|---------|------|
| `parallel_unified_analysis.rs` | 504-507 | `.lock().unwrap().take().unwrap()` | Double unwrap on parallel results |
| `main.rs` | 77, 82 | `RwLock<HashMap>` access | Config lock poisoning |
| `progress.rs` | 117-311 | Global state locks | Progress system crash |
| `tui/mod.rs` | 148 | App state lock | UI deadlock |
| `io/progress.rs` | 82, 98 | Global unified progress | Progress crash |

**Example of Current Problem:**

```rust
// parallel_unified_analysis.rs:504-507
let data_flow = data_flow_result.lock().unwrap().take().unwrap();
let purity = purity_result.lock().unwrap().take().unwrap();
let test_funcs = test_funcs_result.lock().unwrap().take().unwrap();
let debt_agg = debt_agg_result.lock().unwrap().take().unwrap();
```

If any parallel task panics or produces no result:
- Lock may be poisoned → next `lock().unwrap()` panics
- `take()` returns `None` → `unwrap()` panics
- No error context → difficult to debug

**Stillwater Philosophy:**

> "Errors Should Tell Stories" - Deep call stacks lose context. Every error should explain what was happening and why.

## Objective

Eliminate all critical lock unwraps by replacing them with proper error handling:

1. **Replace `.lock().unwrap()`** with `.lock().map_err()` providing context
2. **Replace `.take().unwrap()`** with `.ok_or_else()` explaining the failure
3. **Consider `parking_lot::Mutex`** for non-poisoning mutexes where appropriate
4. **Add lock timeouts** where deadlock is possible

Result: Production-safe lock handling with contextual errors instead of panics.

## Requirements

### Functional Requirements

1. **Lock Unwrap Replacement**
   - All `Mutex::lock().unwrap()` replaced with error handling
   - All `RwLock::read().unwrap()` replaced with error handling
   - All `RwLock::write().unwrap()` replaced with error handling
   - Contextual error messages explain what operation failed

2. **Double Unwrap Elimination**
   - Replace `.lock().unwrap().take().unwrap()` with Result chain
   - Each failure point has distinct error message
   - Parallel task failures propagate cleanly

3. **Timeout Handling**
   - TUI locks use `try_lock()` or timeout
   - Long-running locks don't block UI
   - Deadlock prevention in critical paths

4. **Error Context**
   - Lock poisoning errors include what lock was affected
   - Missing data errors include what data was expected
   - Parallel task errors include which task failed

### Non-Functional Requirements

1. **No Runtime Panics**
   - All lock operations return `Result`
   - Poisoned locks produce errors, not panics
   - Missing data produces errors, not panics

2. **Debuggability**
   - Error messages identify the specific lock
   - Stack traces preserved through error chain
   - Production errors are actionable

3. **Performance**
   - No performance regression in happy path
   - Lock contention handled gracefully
   - Timeout values chosen appropriately

## Acceptance Criteria

- [ ] Zero `.lock().unwrap()` calls in production code paths
- [ ] Zero `.read().unwrap()` calls in production code paths
- [ ] Zero `.write().unwrap()` calls in production code paths
- [ ] All parallel result extraction uses proper `Result` chains
- [ ] TUI locks have timeout or try_lock fallback
- [ ] All error messages include context (which lock, which data)
- [ ] Existing tests pass
- [ ] No clippy warnings
- [ ] Lock-related panics impossible in normal operation

## Technical Details

### Implementation Approach

**Phase 1: Parallel Result Extraction (parallel_unified_analysis.rs)**

```rust
// Before (lines 504-507)
let data_flow = data_flow_result.lock().unwrap().take().unwrap();
let purity = purity_result.lock().unwrap().take().unwrap();
let test_funcs = test_funcs_result.lock().unwrap().take().unwrap();
let debt_agg = debt_agg_result.lock().unwrap().take().unwrap();

// After
fn extract_parallel_result<T>(
    result: &Mutex<Option<T>>,
    name: &str,
) -> Result<T, AnalysisError> {
    result
        .lock()
        .map_err(|_| AnalysisError::other(format!(
            "Lock poisoned while extracting {} - a parallel task may have panicked", name
        )))?
        .take()
        .ok_or_else(|| AnalysisError::other(format!(
            "{} produced no result - parallel task may have failed silently", name
        )))
}

let data_flow = extract_parallel_result(&data_flow_result, "data flow analysis")?;
let purity = extract_parallel_result(&purity_result, "purity analysis")?;
let test_funcs = extract_parallel_result(&test_funcs_result, "test function detection")?;
let debt_agg = extract_parallel_result(&debt_agg_result, "debt aggregation")?;
```

**Phase 2: Config Provider (main.rs)**

```rust
// Before (lines 77, 82)
impl ConfigProvider for DefaultConfigProvider {
    fn get(&self, key: &str) -> Option<String> {
        let config = self.config.read().unwrap();
        config.get(key).cloned()
    }

    fn set(&self, key: &str, value: String) {
        let mut config = self.config.write().unwrap();
        config.insert(key.to_string(), value);
    }
}

// After
impl ConfigProvider for DefaultConfigProvider {
    fn get(&self, key: &str) -> Option<String> {
        self.config
            .read()
            .ok()  // Ignore poisoned lock, return None
            .and_then(|config| config.get(key).cloned())
    }

    fn set(&self, key: &str, value: String) {
        if let Ok(mut config) = self.config.write() {
            config.insert(key.to_string(), value);
        }
        // Silent failure on poisoned lock - config is best-effort
    }
}
```

**Phase 3: Progress State (progress.rs)**

```rust
// Before (lines 117, 122, etc.)
*GLOBAL_PROGRESS.lock().unwrap() = Some(manager);

// Option 1: parking_lot (non-poisoning)
use parking_lot::Mutex;
static GLOBAL_PROGRESS: Lazy<Mutex<Option<ProgressManager>>> =
    Lazy::new(|| Mutex::new(None));

*GLOBAL_PROGRESS.lock() = Some(manager);  // Never panics

// Option 2: Ignore poisoned lock
if let Ok(mut guard) = GLOBAL_PROGRESS.lock() {
    *guard = Some(manager);
}
```

**Phase 4: TUI State (tui/mod.rs)**

```rust
// Before (line 148)
let app = self.app.lock().unwrap();

// After - with timeout
use std::time::Duration;

fn with_app<T>(&self, f: impl FnOnce(&App) -> T) -> Option<T> {
    // Try to acquire lock with timeout
    let start = std::time::Instant::now();
    let timeout = Duration::from_millis(16);  // ~60 FPS frame budget

    loop {
        match self.app.try_lock() {
            Ok(guard) => return Some(f(&guard)),
            Err(_) if start.elapsed() < timeout => {
                std::thread::sleep(Duration::from_micros(100));
            }
            Err(_) => {
                log::warn!("TUI lock timeout - skipping frame");
                return None;
            }
        }
    }
}

// Or use parking_lot::Mutex which has try_lock_for
use parking_lot::Mutex;
let app = self.app.try_lock_for(Duration::from_millis(16))?;
```

**Phase 5: I/O Progress (io/progress.rs)**

```rust
// Before (lines 82, 98)
*GLOBAL_UNIFIED_PROGRESS.lock().unwrap() = Some(Self::new());

// After
fn set_global_progress(progress: UnifiedProgress) {
    match GLOBAL_UNIFIED_PROGRESS.lock() {
        Ok(mut guard) => *guard = Some(progress),
        Err(poisoned) => {
            // Clear the poisoned state and set new value
            let mut guard = poisoned.into_inner();
            *guard = Some(progress);
            log::warn!("Cleared poisoned progress lock");
        }
    }
}
```

### Recommended Patterns

**Pattern 1: Context-Aware Lock Helper**

```rust
/// Helper for lock operations with context
trait LockExt<T> {
    fn lock_with_context(&self, context: &str) -> Result<MutexGuard<T>, AnalysisError>;
}

impl<T> LockExt<T> for Mutex<T> {
    fn lock_with_context(&self, context: &str) -> Result<MutexGuard<T>, AnalysisError> {
        self.lock()
            .map_err(|_| AnalysisError::other(format!(
                "Lock poisoned: {} - a thread may have panicked", context
            )))
    }
}

// Usage
let guard = data_result.lock_with_context("data flow analysis")?;
```

**Pattern 2: Optional Lock (Best-Effort)**

```rust
/// For operations that can gracefully degrade
fn with_lock_optional<T, R>(
    lock: &Mutex<T>,
    f: impl FnOnce(&mut T) -> R,
) -> Option<R> {
    lock.lock().ok().map(|mut guard| f(&mut guard))
}

// Usage - progress update is best-effort
with_lock_optional(&GLOBAL_PROGRESS, |progress| {
    progress.update(50);
});
```

**Pattern 3: Parking Lot Migration**

```rust
// For critical locks, use parking_lot which doesn't have poisoning
use parking_lot::Mutex;

// Before
static STATE: Lazy<std::sync::Mutex<State>> = ...;
STATE.lock().unwrap()  // Can panic if poisoned

// After
static STATE: Lazy<parking_lot::Mutex<State>> = ...;
STATE.lock()  // Never panics (no poisoning)
```

### Files to Modify

1. **`src/builders/parallel_unified_analysis.rs`**
   - Lines 504-507: Extract parallel results with context
   - Add `extract_parallel_result` helper

2. **`src/main.rs`**
   - Lines 77, 82: ConfigProvider RwLock
   - Make config access best-effort

3. **`src/progress.rs`**
   - Lines 117, 122, 130, 149, 168, 311
   - Consider `parking_lot::Mutex`
   - Or add graceful degradation

4. **`src/tui/mod.rs`**
   - Line 148: App state access
   - Add timeout with `try_lock_for`

5. **`src/io/progress.rs`**
   - Lines 82, 98
   - Handle poisoned lock recovery

6. **`src/resources.rs`**
   - Lines 598-652 (test utilities)
   - Less critical but should be consistent

7. **`src/utils/analysis_helpers.rs`**
   - Lines 229-312
   - Progress tracking locks

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/builders/parallel_unified_analysis.rs`
  - `src/main.rs`
  - `src/progress.rs`
  - `src/tui/mod.rs`
  - `src/io/progress.rs`
  - `src/resources.rs`
  - `src/utils/analysis_helpers.rs`
- **External Dependencies**:
  - Optional: `parking_lot` crate for non-poisoning mutexes

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_extract_parallel_result_success() {
    let result = Mutex::new(Some(42));
    let extracted = extract_parallel_result(&result, "test");
    assert_eq!(extracted.unwrap(), 42);
}

#[test]
fn test_extract_parallel_result_none() {
    let result: Mutex<Option<i32>> = Mutex::new(None);
    let extracted = extract_parallel_result(&result, "test");
    assert!(extracted.is_err());
    assert!(extracted.unwrap_err().to_string().contains("produced no result"));
}

#[test]
fn test_extract_parallel_result_poisoned() {
    let result = Arc::new(Mutex::new(Some(42)));
    let result_clone = result.clone();

    // Poison the lock
    let _ = std::panic::catch_unwind(|| {
        let _guard = result_clone.lock().unwrap();
        panic!("intentional panic");
    });

    let extracted = extract_parallel_result(&result, "test");
    assert!(extracted.is_err());
    assert!(extracted.unwrap_err().to_string().contains("poisoned"));
}
```

### Integration Tests

```rust
#[test]
fn test_parallel_analysis_handles_task_failure() {
    // Verify that parallel analysis produces error, not panic
    // when a task fails
}

#[test]
fn test_tui_handles_lock_contention() {
    // Verify TUI remains responsive under lock contention
}
```

## Documentation Requirements

### Code Documentation

Each replacement includes:
- Why the lock might fail
- What context is provided
- Recovery strategy (if any)

### Architecture Updates

Add to `ARCHITECTURE.md`:
- Lock handling patterns
- When to use parking_lot vs std::sync
- Error context guidelines

## Implementation Notes

### Cargo.toml Update (if using parking_lot)

```toml
[dependencies]
parking_lot = "0.12"
```

### Migration Order

1. `parallel_unified_analysis.rs` - Highest impact
2. `progress.rs` - Consider parking_lot
3. `tui/mod.rs` - Add timeout
4. `main.rs` - Best-effort pattern
5. `io/progress.rs` - Consistency
6. Others - Lower priority

### Pitfalls to Avoid

1. **Silent failures** - Don't swallow errors completely; log warnings
2. **Timeout too short** - TUI timeout should be reasonable (~16ms)
3. **Timeout too long** - Don't block UI for seconds
4. **Inconsistent patterns** - Use same approach throughout

## Migration and Compatibility

### Breaking Changes

None - internal error handling improvement.

### Backward Compatibility

All public APIs unchanged. Only internal panic behavior converted to errors.

## Success Metrics

- Zero lock-related panics in production
- All parallel task failures produce contextual errors
- TUI remains responsive under any condition
- Test suite passes including new error path tests
