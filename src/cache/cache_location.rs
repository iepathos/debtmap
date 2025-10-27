use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Strategy for cache storage location
#[derive(Debug, Clone, PartialEq)]
pub enum CacheStrategy {
    /// Store cache in XDG-compliant shared directory (default)
    Shared,
    /// Store cache in user-specified location  
    Custom(PathBuf),
}

/// Manages cache location and project identification
#[derive(Debug, Clone)]
pub struct CacheLocation {
    pub strategy: CacheStrategy,
    pub base_path: PathBuf,
    pub project_id: String,
}

impl CacheLocation {
    /// Resolve cache location based on environment and defaults
    pub fn resolve(repo_path: Option<&Path>) -> Result<Self> {
        // Check if user specified a custom cache directory
        let strategy = if let Ok(custom_dir) = std::env::var("DEBTMAP_CACHE_DIR") {
            CacheStrategy::Custom(PathBuf::from(custom_dir))
        } else {
            CacheStrategy::Shared // Default to shared XDG-compliant location
        };

        Self::resolve_with_strategy(repo_path, strategy)
    }

    /// Resolve cache location with an explicit strategy (for testing)
    pub fn resolve_with_strategy(
        repo_path: Option<&Path>,
        strategy: CacheStrategy,
    ) -> Result<Self> {
        let repo = repo_path.unwrap_or_else(|| Path::new("."));
        let project_id = Self::generate_project_id(repo)?;

        let base_path = match &strategy {
            CacheStrategy::Shared => {
                let cache_dir = Self::get_shared_cache_dir()?;
                cache_dir.join("projects").join(&project_id)
            }
            CacheStrategy::Custom(path) => path.join("debtmap").join("projects").join(&project_id),
        };

        Ok(Self {
            strategy,
            base_path,
            project_id,
        })
    }

    /// Get platform-specific shared cache directory
    fn get_shared_cache_dir() -> Result<PathBuf> {
        // Try XDG_CACHE_HOME first
        if let Ok(xdg_cache) = std::env::var("XDG_CACHE_HOME") {
            return Ok(PathBuf::from(xdg_cache).join("debtmap"));
        }

        // Platform-specific defaults
        #[cfg(target_os = "macos")]
        {
            if let Some(home) = dirs::home_dir() {
                return Ok(home.join("Library").join("Caches").join("debtmap"));
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(home) = dirs::home_dir() {
                return Ok(home.join(".cache").join("debtmap"));
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
                return Ok(PathBuf::from(local_app_data).join("debtmap"));
            }
        }

        // Fallback to temp directory
        Ok(std::env::temp_dir().join("debtmap_cache"))
    }

    /// Generate a stable project ID from repository information
    pub fn generate_project_id(repo_path: &Path) -> Result<String> {
        // Try to get git remote URL first
        if let Ok(git_info) = Self::get_git_info(repo_path) {
            let mut hasher = Sha256::new();
            hasher.update(git_info.as_bytes());
            let hash = format!("{:x}", hasher.finalize());
            return Ok(hash[..16].to_string());
        }

        // Fall back to absolute path hash
        let abs_path = repo_path
            .canonicalize()
            .unwrap_or_else(|_| repo_path.to_path_buf());
        let mut hasher = Sha256::new();
        hasher.update(abs_path.to_string_lossy().as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        Ok(hash[..16].to_string())
    }

    /// Get git repository information for project identification
    fn get_git_info(repo_path: &Path) -> Result<String> {
        use std::process::Command;

        let output = Command::new("git")
            .args(["remote", "get-url", "origin"])
            .current_dir(repo_path)
            .output()
            .context("Failed to get git remote URL")?;

        if output.status.success() {
            let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !url.is_empty() {
                return Ok(url);
            }
        }

        // Try to get repository root as fallback
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(repo_path)
            .output()
            .context("Failed to get git repository root")?;

        if output.status.success() {
            let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !root.is_empty() {
                return Ok(format!("local:{}", root));
            }
        }

        anyhow::bail!("Not a git repository")
    }

    /// Get the full cache path for this project
    pub fn get_cache_path(&self) -> &Path {
        &self.base_path
    }

    /// Get the cache path for a specific component
    pub fn get_component_path(&self, component: &str) -> PathBuf {
        self.base_path.join(component)
    }

    /// Check if we can write to the cache location
    pub fn can_write(&self) -> bool {
        if let Some(parent) = self.base_path.parent() {
            // Check if parent exists and is writable
            if parent.exists() {
                // Try to create a test file
                let test_file = parent.join(".debtmap_test_write");
                if std::fs::write(&test_file, b"test").is_ok() {
                    let _ = std::fs::remove_file(test_file);
                    return true;
                }
            }
        }
        false
    }

    /// Get cache scope from environment (for branch-specific caching)
    pub fn get_cache_scope() -> Option<String> {
        std::env::var("DEBTMAP_CACHE_SCOPE").ok()
    }

    /// Create the cache directory structure
    pub fn ensure_directories(&self) -> Result<()> {
        std::fs::create_dir_all(&self.base_path)
            .with_context(|| format!("Failed to create cache directory: {:?}", self.base_path))?;

        // Create subdirectories for different cache types
        let subdirs = [
            "call_graphs",
            "analysis",
            "metadata",
            "temp",
            "file_metrics",
        ];
        for subdir in &subdirs {
            let path = self.base_path.join(subdir);
            std::fs::create_dir_all(&path)
                .with_context(|| format!("Failed to create cache subdirectory: {:?}", path))?;
        }

        Ok(())
    }
}

// Helper module for directory operations
mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::env;
    use std::sync::Mutex;
    use tempfile::TempDir;

    /// Thread-safe environment guard for isolated testing
    struct EnvGuard {
        original_values: HashMap<String, Option<String>>,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl EnvGuard {
        fn new() -> Self {
            static ENV_MUTEX: Mutex<()> = Mutex::new(());
            let lock = ENV_MUTEX.lock().expect("Failed to acquire env mutex");
            Self {
                original_values: HashMap::new(),
                _lock: lock,
            }
        }

        fn set(&mut self, key: &str, value: &str) {
            if !self.original_values.contains_key(key) {
                self.original_values
                    .insert(key.to_string(), env::var(key).ok());
            }
            env::set_var(key, value);
        }

        fn remove(&mut self, key: &str) {
            if !self.original_values.contains_key(key) {
                self.original_values
                    .insert(key.to_string(), env::var(key).ok());
            }
            env::remove_var(key);
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, value) in &self.original_values {
                match value {
                    Some(v) => env::set_var(key, v),
                    None => env::remove_var(key),
                }
            }
        }
    }

    #[test]
    fn test_cache_strategy_from_env() {
        // Test DEBTMAP_CACHE_DIR with proper isolation
        {
            let mut guard = EnvGuard::new();
            let temp_dir = TempDir::new().unwrap();
            guard.set("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());
            let location = CacheLocation::resolve(None).unwrap();
            if let CacheStrategy::Custom(ref path) = location.strategy {
                assert!(path
                    .to_string_lossy()
                    .contains(temp_dir.path().to_str().unwrap()));
            } else {
                panic!("Expected Custom strategy when DEBTMAP_CACHE_DIR is set");
            }
            // Explicitly drop temp_dir before guard to ensure cleanup order
            drop(temp_dir);
        } // Guard drops here, restoring original env

        // Test default (no env vars) = shared
        // Explicitly verify the environment is clean before testing
        {
            let mut guard = EnvGuard::new();
            guard.remove("DEBTMAP_CACHE_DIR");

            // Double-check the env var is actually removed
            assert!(
                std::env::var("DEBTMAP_CACHE_DIR").is_err(),
                "DEBTMAP_CACHE_DIR should not be set in this test block"
            );

            let location = CacheLocation::resolve(None).unwrap();

            // When DEBTMAP_CACHE_DIR is not set, strategy must be Shared
            assert_eq!(
                location.strategy,
                CacheStrategy::Shared,
                "When DEBTMAP_CACHE_DIR is not set, strategy should be Shared. Got: {:?}",
                location.strategy
            );
        }
    }

    #[test]
    fn test_project_id_generation() {
        let temp_dir = TempDir::new().unwrap();
        let id = CacheLocation::generate_project_id(temp_dir.path()).unwrap();
        assert!(!id.is_empty());
        assert_eq!(id.len(), 16); // Should be first 16 chars of hash
    }

    #[test]
    fn test_cache_scope() {
        // Test with scope set
        {
            let mut guard = EnvGuard::new();
            guard.set("DEBTMAP_CACHE_SCOPE", "feature-branch");
            assert_eq!(
                CacheLocation::get_cache_scope(),
                Some("feature-branch".to_string())
            );
        }

        // Test with no scope
        {
            let mut guard = EnvGuard::new();
            guard.remove("DEBTMAP_CACHE_SCOPE");
            assert_eq!(CacheLocation::get_cache_scope(), None);
        }
    }

    #[test]
    fn test_ensure_directories() {
        let temp_dir = TempDir::new().unwrap();
        let mut guard = EnvGuard::new();
        guard.set("DEBTMAP_CACHE_DIR", temp_dir.path().to_str().unwrap());

        let location = CacheLocation::resolve(None).unwrap();
        location.ensure_directories().unwrap();

        // Check that subdirectories were created
        assert!(location.get_component_path("call_graphs").exists());
        assert!(location.get_component_path("analysis").exists());
        assert!(location.get_component_path("metadata").exists());
        assert!(location.get_component_path("temp").exists());
    }
}
