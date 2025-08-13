//! Cache management for expanded Rust code

use super::{ExpandedFile, SourceMap};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Cache for expanded files
pub struct ExpansionCache {
    cache_dir: PathBuf,
    entries: HashMap<PathBuf, CacheEntry>,
}

/// A cached expansion entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Version of debtmap that created this cache
    pub version: String,
    /// Rust compiler version used for expansion
    pub rust_version: String,
    /// SHA-256 hash of original file
    pub original_hash: String,
    /// Expanded content
    pub expanded_content: String,
    /// Source mappings
    pub source_mappings: Vec<SourceMappingData>,
    /// Unix timestamp of cache creation
    pub timestamp: u64,
}

/// Serializable source mapping data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMappingData {
    pub expanded_line: usize,
    pub original_file: PathBuf,
    pub original_line: usize,
    pub is_macro_generated: bool,
}

impl ExpansionCache {
    /// Create a new cache instance
    pub fn new(cache_dir: &Path) -> Result<Self> {
        // Create cache directory if it doesn't exist
        fs::create_dir_all(cache_dir)
            .with_context(|| format!("Failed to create cache directory: {:?}", cache_dir))?;

        let mut cache = Self {
            cache_dir: cache_dir.to_path_buf(),
            entries: HashMap::new(),
        };

        // Load existing cache entries
        cache.load_cache()?;

        Ok(cache)
    }

    /// Load cache entries from disk
    fn load_cache(&mut self) -> Result<()> {
        let cache_file = self.cache_dir.join("cache.json");

        if !cache_file.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&cache_file).context("Failed to read cache file")?;

        self.entries = serde_json::from_str(&content).context("Failed to parse cache file")?;

        Ok(())
    }

    /// Save cache entries to disk
    fn save_cache(&self) -> Result<()> {
        let cache_file = self.cache_dir.join("cache.json");

        let json =
            serde_json::to_string_pretty(&self.entries).context("Failed to serialize cache")?;

        fs::write(&cache_file, json).context("Failed to write cache file")?;

        Ok(())
    }

    /// Get a cached expansion if valid
    pub fn get(&self, path: &Path, hash: &str) -> Result<Option<ExpandedFile>> {
        if let Some(entry) = self.entries.get(path) {
            // Check if hash matches
            if entry.original_hash == hash {
                // Check version compatibility
                if entry.version == env!("CARGO_PKG_VERSION") {
                    // Reconstruct ExpandedFile from cache entry
                    let source_map = SourceMap::from_mappings(
                        entry
                            .source_mappings
                            .iter()
                            .map(|m| super::SourceMapping {
                                expanded_line: m.expanded_line,
                                original_file: m.original_file.clone(),
                                original_line: m.original_line,
                                is_macro_generated: m.is_macro_generated,
                            })
                            .collect(),
                    );

                    let expanded = ExpandedFile {
                        original_path: path.to_path_buf(),
                        expanded_content: entry.expanded_content.clone(),
                        source_map,
                        timestamp: SystemTime::UNIX_EPOCH
                            + std::time::Duration::from_secs(entry.timestamp),
                    };

                    return Ok(Some(expanded));
                }
            }
        }

        Ok(None)
    }

    /// Store an expansion in the cache
    pub fn store(&mut self, path: &Path, hash: &str, expanded: &ExpandedFile) -> Result<()> {
        let rust_version = get_rust_version()?;

        let mappings: Vec<SourceMappingData> = expanded
            .source_map
            .mappings()
            .iter()
            .map(|m| SourceMappingData {
                expanded_line: m.expanded_line,
                original_file: m.original_file.clone(),
                original_line: m.original_line,
                is_macro_generated: m.is_macro_generated,
            })
            .collect();

        let timestamp = expanded
            .timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let entry = CacheEntry {
            version: env!("CARGO_PKG_VERSION").to_string(),
            rust_version,
            original_hash: hash.to_string(),
            expanded_content: expanded.expanded_content.clone(),
            source_mappings: mappings,
            timestamp,
        };

        self.entries.insert(path.to_path_buf(), entry);
        self.save_cache()?;

        Ok(())
    }

    /// Clear the cache
    pub fn clear(&mut self) -> Result<()> {
        self.entries.clear();

        // Remove cache file
        let cache_file = self.cache_dir.join("cache.json");
        if cache_file.exists() {
            fs::remove_file(&cache_file).context("Failed to remove cache file")?;
        }

        Ok(())
    }

    /// Check if a cache entry is valid
    pub fn is_valid(&self, path: &Path, hash: &str) -> bool {
        if let Some(entry) = self.entries.get(path) {
            entry.original_hash == hash && entry.version == env!("CARGO_PKG_VERSION")
        } else {
            false
        }
    }
}

/// Get the Rust compiler version
fn get_rust_version() -> Result<String> {
    use std::process::Command;

    let output = Command::new("rustc")
        .arg("--version")
        .output()
        .context("Failed to get rustc version")?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
