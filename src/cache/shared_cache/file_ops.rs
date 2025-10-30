//! File operations and cleanup logic for SharedCache
//!
//! This module contains all file management operations:
//! - File and directory classification
//! - Path operations
//! - File copying and deletion
//! - Cache cleanup logic

use crate::cache::cache_location::CacheLocation;
use crate::cache::index_manager::{CacheMetadata, IndexManager};
use anyhow::{Context, Result};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Type of directory entry for classification
#[derive(Debug, PartialEq)]
pub(crate) enum EntryType {
    File,
    Directory,
    Other,
}

// Pure functions for file operations

/// Classify a path as file, directory, or other
pub(crate) fn classify_entry(path: &Path) -> EntryType {
    if path.is_file() {
        EntryType::File
    } else if path.is_dir() {
        EntryType::Directory
    } else {
        EntryType::Other
    }
}

/// Build destination path from base and entry name
pub(crate) fn build_dest_path(dest: &Path, entry_name: &OsStr) -> PathBuf {
    dest.join(entry_name)
}

/// Copy a single file with error context
pub(crate) fn copy_file_entry(src: &Path, dest: &Path) -> Result<()> {
    fs::copy(src, dest)
        .with_context(|| format!("Failed to copy file from {:?} to {:?}", src, dest))?;
    Ok(())
}

/// Create a directory with error context
pub(crate) fn copy_dir_entry(dest: &Path) -> Result<()> {
    fs::create_dir_all(dest).with_context(|| format!("Failed to create directory {:?}", dest))?;
    Ok(())
}

/// Get the file path for a cache entry
pub(super) fn get_cache_file_path(location: &CacheLocation, key: &str, component: &str) -> PathBuf {
    let component_path = location.get_component_path(component);

    // Use first 2 chars of key for directory sharding
    let shard = if key.len() >= 2 { &key[..2] } else { "00" };

    component_path.join(shard).join(format!("{}.cache", key))
}

// File deletion operations

/// Delete cache files for the given keys
pub(super) fn delete_cache_files(location: &CacheLocation, removed_keys: &[String]) -> Result<()> {
    const CACHE_COMPONENTS: &[&str] = &[
        "call_graphs",
        "analysis",
        "metadata",
        "temp",
        "file_metrics",
        "test",
    ];

    for key in removed_keys {
        for component in CACHE_COMPONENTS {
            delete_component_file(location, key, component);
        }
    }
    Ok(())
}

/// Delete a single cache component file with error handling
pub(super) fn delete_component_file(location: &CacheLocation, key: &str, component: &str) {
    let cache_path = get_cache_file_path(location, key, component);
    if cache_path.exists() {
        if let Err(e) = fs::remove_file(&cache_path) {
            log::debug!(
                "Failed to delete cache file {:?}: {}. This may be due to concurrent access.",
                cache_path,
                e
            );
        }
    }
}

// Cleanup operations

/// Select keys for removal until target size is reached
pub(crate) fn select_keys_for_removal(
    entries: Vec<(String, CacheMetadata)>,
    target_size: u64,
    current_size: u64,
) -> Vec<String> {
    let mut removed_keys = Vec::new();
    let mut remaining_size = current_size;

    for (key, metadata) in entries {
        if remaining_size <= target_size {
            break;
        }
        removed_keys.push(key);
        remaining_size -= metadata.size_bytes;
    }
    removed_keys
}

/// Determine which cache keys should be removed based on size and age
pub(super) fn determine_keys_to_remove(
    index_manager: &Arc<IndexManager>,
    max_cache_size: u64,
) -> Result<Vec<String>> {
    let (sorted_entries, total_size) = index_manager.get_sorted_entries_and_stats();
    let target_size = max_cache_size / 2;
    let keys_to_remove = select_keys_for_removal(sorted_entries, target_size, total_size);

    index_manager.remove_entries(&keys_to_remove)?;
    Ok(keys_to_remove)
}

/// Clear all files in a component directory
pub(super) fn clear_component_files(location: &CacheLocation, component: &str) -> Result<()> {
    let component_path = location.get_component_path(component);
    if !component_path.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(&component_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            fs::remove_file(&path)?;
            continue;
        }

        if path.is_dir() {
            // Remove files in sharded subdirectories
            for subentry in fs::read_dir(&path)? {
                let subentry = subentry?;
                if subentry.path().is_file() {
                    fs::remove_file(subentry.path())?;
                }
            }
            // Try to remove the now-empty shard directory
            let _ = fs::remove_dir(&path);
        }
    }

    Ok(())
}

// Migration operations

/// Recursively copy directory contents
pub(super) fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    for entry in fs::read_dir(src).with_context(|| format!("Failed to read directory {:?}", src))? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = build_dest_path(dest, &entry.file_name());

        match classify_entry(&path) {
            EntryType::File => copy_file_entry(&path, &dest_path)?,
            EntryType::Directory => {
                copy_dir_entry(&dest_path)?;
                copy_dir_recursive(&path, &dest_path)?;
            }
            EntryType::Other => {
                // Skip other entry types (symlinks, etc.)
            }
        }
    }
    Ok(())
}

/// Migrate cache from local to shared location
pub(super) fn migrate_from_local(location: &CacheLocation, local_cache_path: &Path) -> Result<()> {
    if !local_cache_path.exists() {
        return Ok(()); // Nothing to migrate
    }

    log::info!(
        "Migrating cache from {:?} to {:?}",
        local_cache_path,
        location.get_cache_path()
    );

    // Copy all cache files
    for entry in fs::read_dir(local_cache_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(file_name) = path.file_name() {
                let dest = location.get_cache_path().join(file_name);
                fs::copy(&path, &dest).with_context(|| {
                    format!("Failed to copy cache file from {:?} to {:?}", path, dest)
                })?;
            }
        } else if path.is_dir() {
            // Recursively copy subdirectories
            if let Some(dir_name) = path.file_name() {
                let dest_dir = location.get_cache_path().join(dir_name);
                fs::create_dir_all(&dest_dir)?;
                copy_dir_recursive(&path, &dest_dir)?;
            }
        }
    }

    log::info!("Cache migration completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_classify_entry() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("file.txt");
        let dir_path = temp.path().join("dir");

        std::fs::write(&file_path, "test").unwrap();
        std::fs::create_dir(&dir_path).unwrap();

        assert_eq!(classify_entry(&file_path), EntryType::File);
        assert_eq!(classify_entry(&dir_path), EntryType::Directory);
    }

    #[test]
    fn test_build_dest_path() {
        let base = Path::new("/tmp/cache");
        let name = OsStr::new("file.txt");
        let result = build_dest_path(base, name);

        assert_eq!(result, PathBuf::from("/tmp/cache/file.txt"));
    }

    #[test]
    fn test_select_keys_for_removal() {
        let entries = vec![
            ("key1".to_string(), create_test_metadata(100)),
            ("key2".to_string(), create_test_metadata(200)),
            ("key3".to_string(), create_test_metadata(300)),
        ];

        let removed = select_keys_for_removal(entries, 300, 600);
        assert_eq!(removed.len(), 2); // Should remove until size <= 300
    }

    #[test]
    fn test_select_keys_for_removal_target_reached() {
        let entries = vec![
            ("key1".to_string(), create_test_metadata(100)),
            ("key2".to_string(), create_test_metadata(200)),
        ];

        let removed = select_keys_for_removal(entries, 500, 300);
        assert_eq!(removed.len(), 0); // Already below target
    }

    fn create_test_metadata(size: u64) -> CacheMetadata {
        CacheMetadata {
            version: "1.0".to_string(),
            size_bytes: size,
            last_accessed: std::time::SystemTime::now(),
            created_at: std::time::SystemTime::now(),
            access_count: 1,
            debtmap_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}
