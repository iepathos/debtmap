---
number: 205
title: Retry Pattern for Resilient Operations
category: foundation
priority: low
status: draft
dependencies: [195, 198]
created: 2025-11-27
---

# Specification 205: Retry Pattern for Resilient Operations

**Category**: foundation
**Priority**: low
**Status**: draft
**Dependencies**: Specs 195, 198 (stillwater foundation, effect composition)

## Context

Some debtmap operations can fail transiently due to external factors:

1. **File System Operations**
   - File locked by another process
   - Network file system temporary unavailability
   - Permission issues during concurrent access

2. **Git Operations**
   - Lock file contention
   - Network issues for remote operations
   - Index file corruption recovery

3. **External Tool Invocations**
   - Coverage tool execution
   - Formatter/linter integration
   - External analysis tools

Currently, these failures are fatal and require manual retry. Stillwater provides `RetryPolicy` for declarative retry logic:

- **Exponential backoff** - Prevent thundering herd
- **Jitter support** - Add randomness to delays
- **Configurable limits** - Max retries, timeout
- **Clean composition** - Integrates with Effect system

## Objective

Integrate stillwater's retry pattern for operations that can fail transiently, improving resilience without code complexity.

## Requirements

### Functional Requirements

1. **Configurable Retry Policies**
   - Support different strategies: constant, linear, exponential, fibonacci
   - Configurable max retries and timeouts
   - Optional jitter for distributed scenarios

2. **Targeted Retry**
   - Only retry specific error types (transient)
   - Don't retry permanent failures (syntax errors)
   - Provide retry predicate for custom logic

3. **Observability**
   - Log retry attempts with details
   - Track retry statistics
   - Report final failure with retry history

### Non-Functional Requirements

1. **Performance**
   - Minimal overhead for successful operations
   - Reasonable default timeouts
   - Backoff to prevent resource exhaustion

2. **User Experience**
   - Clear feedback during retries
   - Configurable via debtmap.toml
   - Sensible defaults that work out of the box

## Acceptance Criteria

- [ ] Create `RetryConfig` section in `DebtmapConfig`
- [ ] Implement `with_retry` effect combinator
- [ ] Add retry logic to file read operations for locked files
- [ ] Add retry logic to git operations
- [ ] Add retry predicate for transient vs permanent errors
- [ ] Create logging/metrics for retry attempts
- [ ] Configuration documentation
- [ ] Tests for retry behavior

## Technical Details

### Implementation Approach

#### 1. Retry Configuration

```rust
// In src/config/retry.rs
use std::time::Duration;

/// Retry configuration for resilient operations.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RetryConfig {
    /// Enable automatic retries (default: true)
    pub enabled: bool,

    /// Maximum number of retry attempts (default: 3)
    pub max_retries: u32,

    /// Base delay between retries (default: 100ms)
    pub base_delay_ms: u64,

    /// Retry strategy (default: exponential)
    pub strategy: RetryStrategy,

    /// Maximum total time to spend retrying (default: 30s)
    pub timeout_seconds: u64,

    /// Add jitter to delays (default: 0.1 = 10%)
    pub jitter_factor: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RetryStrategy {
    Constant,
    Linear,
    Exponential,
    Fibonacci,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_retries: 3,
            base_delay_ms: 100,
            strategy: RetryStrategy::Exponential,
            timeout_seconds: 30,
            jitter_factor: 0.1,
        }
    }
}
```

#### 2. Effect Retry Combinator

```rust
// In src/effects/retry.rs
use stillwater::{RetryPolicy, RetryStrategy as StillwaterStrategy};
use stillwater::effect::prelude::*;

/// Convert debtmap retry config to stillwater RetryPolicy.
pub fn to_retry_policy(config: &RetryConfig) -> RetryPolicy {
    let base = Duration::from_millis(config.base_delay_ms);

    let policy = match config.strategy {
        RetryStrategy::Constant => RetryPolicy::constant(base),
        RetryStrategy::Linear => RetryPolicy::linear(base),
        RetryStrategy::Exponential => RetryPolicy::exponential(base),
        RetryStrategy::Fibonacci => RetryPolicy::fibonacci(base, base),
    };

    policy
        .with_max_retries(config.max_retries)
        .with_timeout(Duration::from_secs(config.timeout_seconds))
        .with_jitter(config.jitter_factor)
}

/// Wrap an effect with retry logic.
pub fn with_retry<T, F>(
    effect_factory: F,
    policy: RetryPolicy,
    is_retryable: impl Fn(&AnalysisError) -> bool + Send + Sync + 'static,
) -> AnalysisEffect<T>
where
    T: Send + 'static,
    F: Fn() -> AnalysisEffect<T> + Send + Sync + 'static,
{
    // Implementation using stillwater's retry functionality
    from_async(move |env: &RealEnv| {
        let env = env.clone();
        let policy = policy.clone();
        async move {
            let mut attempts = 0;
            let mut last_error = None;

            loop {
                match effect_factory().run(&env).await {
                    Ok(value) => return Ok(value),
                    Err(e) if is_retryable(&e) && attempts < policy.max_retries() => {
                        attempts += 1;
                        tracing::warn!(
                            "Retrying operation (attempt {}/{}): {}",
                            attempts,
                            policy.max_retries(),
                            e
                        );
                        let delay = policy.delay_for_attempt(attempts);
                        tokio::time::sleep(delay).await;
                        last_error = Some(e);
                    }
                    Err(e) => return Err(e),
                }
            }
        }
    }).boxed()
}
```

#### 3. Retryable Error Classification

```rust
// In src/errors.rs
impl AnalysisError {
    /// Check if this error is potentially transient and retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            // I/O errors that might be transient
            AnalysisError::Io(e) => {
                matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted
                        | std::io::ErrorKind::ResourceBusy
                )
            }
            // Git lock contention
            AnalysisError::Git(msg) => {
                msg.contains("lock") || msg.contains("index.lock")
            }
            // Coverage tool execution issues
            AnalysisError::Coverage(msg) => {
                msg.contains("connection") || msg.contains("timeout")
            }
            // Other errors are not retryable
            _ => false,
        }
    }
}
```

#### 4. Retryable File Operations

```rust
// In src/io/effects.rs

/// Read file with retry for locked files.
pub fn read_file_with_retry_effect(path: PathBuf) -> AnalysisEffect<String> {
    asks_config(move |config| {
        let retry_config = config.retry.clone().unwrap_or_default();
        let policy = to_retry_policy(&retry_config);
        let path = path.clone();

        with_retry(
            move || read_file_effect(path.clone()),
            policy,
            |e| e.is_retryable(),
        )
    }).and_then(|effect| effect).boxed()
}

/// Walk directory with retry for filesystem issues.
pub fn walk_dir_with_retry_effect(path: PathBuf) -> AnalysisEffect<Vec<PathBuf>> {
    asks_config(move |config| {
        let retry_config = config.retry.clone().unwrap_or_default();
        let policy = to_retry_policy(&retry_config);
        let path = path.clone();

        with_retry(
            move || walk_dir_effect(path.clone()),
            policy,
            |e| e.is_retryable(),
        )
    }).and_then(|effect| effect).boxed()
}
```

### Configuration Example

```toml
# debtmap.toml

[retry]
enabled = true
max_retries = 3
base_delay_ms = 100
strategy = "exponential"
timeout_seconds = 30
jitter_factor = 0.1
```

### Architecture Changes

1. **New Module**: `src/config/retry.rs`
   - Retry configuration types
   - Conversion to stillwater policy

2. **New Module**: `src/effects/retry.rs`
   - `with_retry` combinator
   - Retry logging and metrics

3. **Modified Module**: `src/errors.rs`
   - Add `is_retryable` method

4. **Modified Module**: `src/io/effects.rs`
   - Add retry variants of I/O effects

## Dependencies

- **Prerequisites**:
  - Spec 195 (stillwater foundation)
  - Spec 198 (effect composition)

- **Affected Components**:
  - `src/config/mod.rs`
  - `src/errors.rs`
  - `src/io/effects.rs`

- **External Dependencies**:
  - stillwater 0.11.0+ retry module

## Testing Strategy

### Unit Tests

```rust
#[tokio::test]
async fn test_retry_succeeds_after_transient_failure() {
    let attempt = Arc::new(AtomicUsize::new(0));
    let attempt_clone = attempt.clone();

    let effect_factory = move || {
        let attempt = attempt_clone.clone();
        from_fn(move |_env: &RealEnv| {
            let current = attempt.fetch_add(1, Ordering::SeqCst);
            if current < 2 {
                Err(AnalysisError::io("Resource busy"))
            } else {
                Ok("success".to_string())
            }
        }).boxed()
    };

    let policy = RetryPolicy::constant(Duration::from_millis(10))
        .with_max_retries(3);

    let effect = with_retry(effect_factory, policy, |e| e.is_retryable());
    let result = effect.run(&RealEnv::default()).await;

    assert!(result.is_ok());
    assert_eq!(attempt.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_retry_fails_on_permanent_error() {
    let effect_factory = || {
        from_fn(|_env: &RealEnv| {
            Err::<String, _>(AnalysisError::parse("Syntax error"))
        }).boxed()
    };

    let policy = RetryPolicy::constant(Duration::from_millis(10))
        .with_max_retries(3);

    let effect = with_retry(effect_factory, policy, |e| e.is_retryable());
    let result = effect.run(&RealEnv::default()).await;

    assert!(result.is_err());
    // Should fail immediately without retry
}
```

### Integration Tests
- Test file read retry with simulated lock
- Test git operation retry
- Test timeout behavior

## Documentation Requirements

- **Code Documentation**: Document retry configuration options
- **User Documentation**: Add retry section to configuration guide
- **Architecture Updates**: Document retry pattern in DESIGN.md

## Implementation Notes

1. **Default Off for CI**: Consider disabling retries in CI environments where failures should be immediate.

2. **Logging**: Log all retry attempts at WARN level for visibility.

3. **Metrics**: Consider adding retry metrics for observability.

4. **Idempotency**: Only use retry for read operations or idempotent writes.

## Migration and Compatibility

- **No Breaking Changes**: Retry is opt-in via configuration
- **Default Behavior**: Sensible defaults work out of the box
- **Backwards Compatible**: Non-retry variants remain available
