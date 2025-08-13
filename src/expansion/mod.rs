//! Macro expansion module for perfect function call detection
//!
//! This module provides cargo-expand integration to analyze fully expanded
//! Rust code, enabling accurate detection of function calls within macros.

use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

mod cache;
mod expander;
mod source_map;

pub use cache::{CacheEntry, ExpansionCache};
pub use expander::{ExpandedFile, MacroExpander};
pub use source_map::{SourceMap, SourceMapping};

/// Configuration for macro expansion
#[derive(Debug, Clone)]
pub struct ExpansionConfig {
    /// Enable macro expansion
    pub enabled: bool,
    /// Cache directory for expanded files
    pub cache_dir: PathBuf,
    /// Fall back to standard analysis on expansion failure
    pub fallback_on_error: bool,
    /// Use parallel expansion for workspaces
    pub parallel: bool,
    /// Timeout for expansion operations (in seconds)
    pub timeout_secs: u64,
}

impl Default for ExpansionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cache_dir: PathBuf::from(".debtmap/cache/expanded"),
            fallback_on_error: true,
            parallel: true,
            timeout_secs: 60,
        }
    }
}

/// Trait for macro expansion functionality
pub trait MacroExpansion {
    /// Expand macros in a single file
    fn expand_file(&mut self, path: &Path) -> Result<ExpandedFile>;

    /// Expand macros in all workspace files
    fn expand_workspace(&mut self) -> Result<HashMap<PathBuf, ExpandedFile>>;

    /// Clear the expansion cache
    fn clear_cache(&mut self) -> Result<()>;

    /// Check if cache is valid for a file
    fn is_cache_valid(&self, path: &Path) -> bool;
}

/// Result of expanding a file's macros
pub struct ExpansionResult {
    /// Original file path
    pub original_path: PathBuf,
    /// Expanded content (if successful)
    pub expanded: Option<ExpandedFile>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Create a new macro expander with the given configuration
pub fn create_expander(config: ExpansionConfig) -> Result<Box<dyn MacroExpansion>> {
    Ok(Box::new(MacroExpander::new(config)?))
}
