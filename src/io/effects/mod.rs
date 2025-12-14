//! Effect-wrapped I/O operations for debtmap analysis.
//!
//! This module provides Effect-based wrappers around file system operations,
//! enabling pure functional composition while maintaining testability.
//!
//! # Module Organization
//!
//! The effects system is organized into focused sub-modules:
//!
//! - **file**: Basic file read/write/existence operations
//! - **directory**: Directory walking operations
//! - **cache**: Cache get/set/invalidate/clear operations
//! - **retry**: Retry-wrapped versions of I/O operations
//! - **compose**: Higher-level composed operations
//!
//! # Design Philosophy
//!
//! Following Stillwater's "Pure Core, Imperative Shell" pattern:
//! - All operations are wrapped in Effect types
//! - Effects defer execution until run with an environment
//! - Enables testing with mock environments
//! - Composes naturally with `and_then`, `map`, etc.
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::io::effects::{read_file_effect, walk_dir_effect};
//! use debtmap::effects::run_effect;
//! use debtmap::config::DebtmapConfig;
//!
//! // Read a single file
//! let content = run_effect(
//!     read_file_effect("src/main.rs".into()),
//!     DebtmapConfig::default(),
//! )?;
//!
//! // Walk a directory and filter files
//! let rust_files = run_effect(
//!     walk_dir_effect("src".into())
//!         .map(|files| files.into_iter()
//!             .filter(|p| p.extension().map_or(false, |e| e == "rs"))
//!             .collect::<Vec<_>>()),
//!     DebtmapConfig::default(),
//! )?;
//! ```

mod cache;
mod compose;
mod directory;
mod file;
mod retry;

// File operations
pub use file::{
    file_exists_effect, is_directory_effect, path_exists_effect, read_file_bytes_effect,
    read_file_effect, write_file_effect,
};

// Directory operations
pub use directory::{walk_dir_effect, walk_dir_with_config_effect};

// Cache operations
pub use cache::{cache_clear_effect, cache_get_effect, cache_invalidate_effect, cache_set_effect};

// Retry operations
pub use retry::{
    read_file_bytes_with_retry_effect, read_file_with_retry_effect, walk_dir_with_retry_effect,
    write_file_with_retry_effect,
};

// Composed operations
pub use compose::{
    read_file_if_exists_effect, read_files_effect, walk_and_analyze_effect,
    walk_and_validate_effect,
};
