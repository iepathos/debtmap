//! Retry-wrapped effect operations.
//!
//! This module provides Effect-based wrappers with automatic retry logic:
//! - Read file with retry
//! - Read file bytes with retry
//! - Walk directory with retry
//! - Write file with retry
//!
//! # Design Philosophy
//!
//! These operations wrap the basic I/O effects with retry logic, automatically
//! retrying on transient errors like:
//! - File locked by another process
//! - Resource temporarily unavailable
//! - Network filesystem issues
//!
//! Non-retryable errors (file not found, permission denied for non-temp
//! reasons) fail immediately without retry.

use crate::config::RetryConfig;
use crate::effects::{with_retry, AnalysisEffect};
use std::path::PathBuf;

use super::directory::walk_dir_effect;
use super::file::{read_file_bytes_effect, read_file_effect, write_file_effect};

/// Read file with automatic retry for transient failures.
///
/// This wraps `read_file_effect` with retry logic, automatically retrying
/// on transient errors like:
/// - File locked by another process
/// - Resource temporarily unavailable
/// - Network filesystem issues
///
/// Non-retryable errors (file not found, permission denied for non-temp
/// reasons) fail immediately without retry.
///
/// # Arguments
///
/// * `path` - Path to the file to read
/// * `retry_config` - Configuration for retry behavior
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::io::effects::read_file_with_retry_effect;
/// use debtmap::config::RetryConfig;
///
/// let config = RetryConfig::default();
/// let effect = read_file_with_retry_effect("data.json".into(), config);
/// let content = run_effect(effect, DebtmapConfig::default())?;
/// ```
pub fn read_file_with_retry_effect(
    path: PathBuf,
    retry_config: RetryConfig,
) -> AnalysisEffect<String> {
    with_retry(move || read_file_effect(path.clone()), retry_config)
}

/// Read file bytes with automatic retry for transient failures.
///
/// Similar to `read_file_with_retry_effect` but returns raw bytes.
pub fn read_file_bytes_with_retry_effect(
    path: PathBuf,
    retry_config: RetryConfig,
) -> AnalysisEffect<Vec<u8>> {
    with_retry(move || read_file_bytes_effect(path.clone()), retry_config)
}

/// Walk directory with automatic retry for transient failures.
///
/// Retries the directory walk operation on transient filesystem errors.
pub fn walk_dir_with_retry_effect(
    path: PathBuf,
    retry_config: RetryConfig,
) -> AnalysisEffect<Vec<PathBuf>> {
    with_retry(move || walk_dir_effect(path.clone()), retry_config)
}

/// Write file with automatic retry for transient failures.
///
/// Retries write operations on transient errors like disk busy
/// or temporary unavailability.
///
/// # Note
///
/// Be careful with retry on write operations - ensure the operation
/// is idempotent (writing the same content multiple times is safe).
pub fn write_file_with_retry_effect(
    path: PathBuf,
    content: String,
    retry_config: RetryConfig,
) -> AnalysisEffect<()> {
    with_retry(
        move || write_file_effect(path.clone(), content.clone()),
        retry_config,
    )
}
