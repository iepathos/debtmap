---
number: 206
title: Bracket Pattern for Resource Management
category: foundation
priority: low
status: draft
dependencies: [195, 198]
created: 2025-11-27
---

# Specification 206: Bracket Pattern for Resource Management

**Category**: foundation
**Priority**: low
**Status**: draft
**Dependencies**: Specs 195, 198 (stillwater foundation, effect composition)

## Context

Debtmap manages several types of resources that require proper cleanup:

1. **Lock Files**
   - Analysis lock files to prevent concurrent runs
   - Git index locks during operations

2. **Temporary Files**
   - Intermediate analysis results
   - Coverage data processing
   - Report generation staging

3. **Progress State**
   - Progress bars and spinners
   - Status updates that need cleanup on error

4. **File Handles**
   - Large file streaming
   - Log file handles

Currently, cleanup relies on `Drop` implementations or manual try-finally patterns. This is error-prone and doesn't compose well with the Effect system.

Stillwater provides the `bracket` pattern:
```rust
bracket(acquire, release, use_resource)
```

This guarantees cleanup even on error, composing naturally with other effects.

## Objective

Implement bracket-based resource management for debtmap operations, ensuring proper cleanup regardless of success or failure.

## Requirements

### Functional Requirements

1. **Guaranteed Cleanup**
   - Resources released even on error or panic
   - Cleanup runs in correct order for nested resources
   - Support both sync and async cleanup

2. **Common Resource Patterns**
   - Lock file acquisition and release
   - Temporary directory management
   - Progress indicator lifecycle

3. **Composable Resources**
   - Nest multiple resources safely
   - Combine with other effects naturally
   - Support resource transformation

### Non-Functional Requirements

1. **Reliability**
   - Never leak resources
   - Handle cleanup errors gracefully
   - Work correctly in async context

2. **Ergonomics**
   - Simple API for common cases
   - Helper functions for typical resources
   - Clear error messages on failures

## Acceptance Criteria

- [ ] Create `with_lock_file` effect for exclusive access
- [ ] Create `with_temp_dir` effect for temporary directories
- [ ] Create `with_progress` effect for progress indicators
- [ ] Add bracket-based file streaming
- [ ] Tests for cleanup on success, error, and panic
- [ ] Documentation with examples

## Technical Details

### Implementation Approach

#### 1. Lock File Management

```rust
// In src/effects/resources.rs
use stillwater::bracket;
use stillwater::effect::prelude::*;
use std::path::PathBuf;
use std::fs::{File, OpenOptions};

/// Lock file resource for exclusive access.
pub struct LockFile {
    path: PathBuf,
    _file: File,
}

/// Acquire a lock file, run the effect, then release.
pub fn with_lock_file<T, F, Eff>(
    lock_path: PathBuf,
    effect: F,
) -> AnalysisEffect<T>
where
    T: Send + 'static,
    F: FnOnce() -> Eff + Send + 'static,
    Eff: Effect<Output = T, Error = AnalysisError, Env = RealEnv> + Send + 'static,
{
    bracket(
        // Acquire
        {
            let path = lock_path.clone();
            from_fn(move |_env: &RealEnv| {
                let file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
                    .map_err(|e| {
                        if e.kind() == std::io::ErrorKind::AlreadyExists {
                            AnalysisError::other(format!(
                                "Lock file exists: {}. Another process may be running.",
                                path.display()
                            ))
                        } else {
                            AnalysisError::io_with_path(
                                format!("Failed to create lock: {}", e),
                                &path,
                            )
                        }
                    })?;

                Ok(LockFile { path, _file: file })
            })
        },
        // Release
        |lock: LockFile| {
            from_fn(move |_env: &RealEnv| {
                std::fs::remove_file(&lock.path).ok(); // Ignore errors on cleanup
                Ok(())
            })
        },
        // Use
        |_lock| effect(),
    ).boxed()
}
```

#### 2. Temporary Directory Management

```rust
/// Temporary directory that cleans up automatically.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Create a temporary directory, run the effect, then cleanup.
pub fn with_temp_dir<T, F, Eff>(
    prefix: &str,
    effect: F,
) -> AnalysisEffect<T>
where
    T: Send + 'static,
    F: FnOnce(PathBuf) -> Eff + Send + 'static,
    Eff: Effect<Output = T, Error = AnalysisError, Env = RealEnv> + Send + 'static,
{
    let prefix = prefix.to_string();

    bracket(
        // Acquire
        from_fn(move |_env: &RealEnv| {
            let temp_path = std::env::temp_dir()
                .join(format!("debtmap-{}-{}", prefix, uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&temp_path)
                .map_err(|e| AnalysisError::io_with_path(
                    format!("Failed to create temp dir: {}", e),
                    &temp_path,
                ))?;
            Ok(TempDir { path: temp_path })
        }),
        // Release
        |temp: TempDir| {
            from_fn(move |_env: &RealEnv| {
                std::fs::remove_dir_all(&temp.path).ok();
                Ok(())
            })
        },
        // Use
        |temp| effect(temp.path),
    ).boxed()
}
```

#### 3. Progress Indicator Management

```rust
/// Progress indicator resource.
pub struct ProgressHandle {
    bar: indicatif::ProgressBar,
}

/// Show a progress bar during an effect, cleanup on completion.
pub fn with_progress<T, F, Eff>(
    message: &str,
    total: u64,
    effect: F,
) -> AnalysisEffect<T>
where
    T: Send + 'static,
    F: FnOnce(ProgressHandle) -> Eff + Send + 'static,
    Eff: Effect<Output = T, Error = AnalysisError, Env = RealEnv> + Send + 'static,
{
    let message = message.to_string();

    bracket(
        // Acquire
        from_fn(move |_env: &RealEnv| {
            let bar = indicatif::ProgressBar::new(total);
            bar.set_style(
                indicatif::ProgressStyle::default_bar()
                    .template("{msg} [{bar:40}] {pos}/{len}")
                    .unwrap()
            );
            bar.set_message(message);
            Ok(ProgressHandle { bar })
        }),
        // Release
        |progress: ProgressHandle| {
            from_fn(move |_env: &RealEnv| {
                progress.bar.finish_and_clear();
                Ok(())
            })
        },
        // Use
        effect,
    ).boxed()
}
```

#### 4. Simple Bracket Helper

```rust
/// Simplified bracket for common use cases.
pub fn bracket_simple<T, Acquire, Release, Use, AcqEff, RelEff, UseEff>(
    acquire: Acquire,
    release: Release,
    use_fn: Use,
) -> AnalysisEffect<T>
where
    T: Send + 'static,
    Acquire: FnOnce() -> AcqEff + Send + 'static,
    Release: FnOnce() -> RelEff + Send + 'static,
    Use: FnOnce() -> UseEff + Send + 'static,
    AcqEff: Effect<Output = (), Error = AnalysisError, Env = RealEnv> + Send + 'static,
    RelEff: Effect<Output = (), Error = AnalysisError, Env = RealEnv> + Send + 'static,
    UseEff: Effect<Output = T, Error = AnalysisError, Env = RealEnv> + Send + 'static,
{
    bracket(
        acquire(),
        |_| release(),
        |_| use_fn(),
    ).boxed()
}
```

### Usage Examples

```rust
// Lock-protected analysis
pub fn analyze_with_lock(project: PathBuf) -> AnalysisEffect<AnalysisResults> {
    let lock_path = project.join(".debtmap.lock");

    with_lock_file(lock_path, move || {
        analyze_project_effect(project)
    })
}

// Analysis with temporary staging
pub fn analyze_with_staging(project: PathBuf) -> AnalysisEffect<AnalysisResults> {
    with_temp_dir("analysis", move |staging_dir| {
        // Copy files to staging, analyze, cleanup automatically
        copy_project_to_staging(&project, &staging_dir)
            .and_then(|_| analyze_project_effect(staging_dir))
    })
}

// Nested resources
pub fn analyze_with_progress_and_lock(
    project: PathBuf,
    file_count: u64,
) -> AnalysisEffect<AnalysisResults> {
    let lock_path = project.join(".debtmap.lock");

    with_lock_file(lock_path, move || {
        with_progress("Analyzing", file_count, move |progress| {
            analyze_project_with_progress(project, progress)
        })
    })
}
```

### Architecture Changes

1. **New Module**: `src/effects/resources.rs`
   - Lock file management
   - Temporary directory management
   - Progress indicator management

2. **Modified Module**: `src/effects.rs`
   - Re-export resource helpers

3. **Modified Module**: `src/analyzers/mod.rs`
   - Use bracket for analysis lock

## Dependencies

- **Prerequisites**:
  - Spec 195 (stillwater foundation)
  - Spec 198 (effect composition)

- **Affected Components**:
  - `src/effects.rs`
  - `src/analyzers/mod.rs`
  - `src/main.rs` (for lock file usage)

- **External Dependencies**:
  - stillwater 0.11.0+ bracket module
  - indicatif (already a dependency)
  - uuid (for temp dir names)

## Testing Strategy

### Unit Tests

```rust
#[tokio::test]
async fn test_with_lock_file_cleanup_on_success() {
    let temp = TempDir::new().unwrap();
    let lock_path = temp.path().join("test.lock");

    let effect = with_lock_file(lock_path.clone(), || pure(42));
    let result = effect.run(&RealEnv::default()).await;

    assert_eq!(result.unwrap(), 42);
    assert!(!lock_path.exists(), "Lock file should be removed");
}

#[tokio::test]
async fn test_with_lock_file_cleanup_on_error() {
    let temp = TempDir::new().unwrap();
    let lock_path = temp.path().join("test.lock");

    let effect: AnalysisEffect<i32> = with_lock_file(lock_path.clone(), || {
        fail(AnalysisError::other("intentional failure"))
    });
    let result = effect.run(&RealEnv::default()).await;

    assert!(result.is_err());
    assert!(!lock_path.exists(), "Lock file should be removed on error");
}

#[tokio::test]
async fn test_with_temp_dir_cleanup() {
    let temp_path = Arc::new(Mutex::new(None));
    let temp_path_clone = temp_path.clone();

    let effect = with_temp_dir("test", move |path| {
        *temp_path_clone.lock().unwrap() = Some(path.clone());
        pure(42)
    });
    let result = effect.run(&RealEnv::default()).await;

    assert_eq!(result.unwrap(), 42);

    let captured_path = temp_path.lock().unwrap().clone().unwrap();
    assert!(!captured_path.exists(), "Temp dir should be removed");
}
```

### Integration Tests
- Test lock file prevents concurrent access
- Test nested bracket cleanup order
- Test cleanup on async cancellation

## Documentation Requirements

- **Code Documentation**: Document all resource helpers
- **User Documentation**: Explain resource management patterns
- **Architecture Updates**: Document bracket pattern in DESIGN.md

## Implementation Notes

1. **Cleanup Errors**: Log but don't fail on cleanup errors.

2. **Panic Safety**: Ensure cleanup runs even on panic (may need catch_unwind).

3. **Async Cleanup**: Support async release functions for network resources.

4. **Nested Order**: Inner resources released before outer.

## Migration and Compatibility

- **No Breaking Changes**: New functionality, additive only
- **Optional Usage**: Existing code continues to work
- **Gradual Adoption**: Migrate resource management incrementally
