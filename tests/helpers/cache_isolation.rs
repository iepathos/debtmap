use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

/// Thread-safe environment variable guard that restores original values on drop
pub struct EnvGuard {
    original_values: HashMap<String, Option<String>>,
    _lock: Option<std::sync::MutexGuard<'static, ()>>,
}

impl EnvGuard {
    /// Create a new environment guard
    pub fn new() -> Self {
        // Use a static mutex to ensure environment modifications are synchronized
        static ENV_MUTEX: Mutex<()> = Mutex::new(());
        let lock = ENV_MUTEX
            .lock()
            .expect("Failed to acquire environment mutex");

        Self {
            original_values: HashMap::new(),
            _lock: Some(lock),
        }
    }

    /// Set an environment variable and track the original value
    pub fn set(&mut self, key: &str, value: &str) {
        if !self.original_values.contains_key(key) {
            self.original_values
                .insert(key.to_string(), env::var(key).ok());
        }
        env::set_var(key, value);
    }

    /// Remove an environment variable and track the original value
    #[allow(dead_code)]
    pub fn remove(&mut self, key: &str) {
        if !self.original_values.contains_key(key) {
            self.original_values
                .insert(key.to_string(), env::var(key).ok());
        }
        env::remove_var(key);
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        // Restore original environment values
        for (key, value) in &self.original_values {
            match value {
                Some(v) => env::set_var(key, v),
                None => env::remove_var(key),
            }
        }
    }
}

/// Isolated test context for cache testing
pub struct IsolatedCacheTest {
    #[allow(dead_code)]
    pub test_id: String,
    pub cache_dir: TempDir,
    pub project_dir: TempDir,
    #[allow(dead_code)]
    pub env_guard: EnvGuard,
}

impl IsolatedCacheTest {
    /// Create a new isolated test context
    #[allow(dead_code)]
    pub fn new(test_name: &str) -> Self {
        // Generate unique test ID using test name, thread ID, and timestamp
        let thread_id = format!("{:?}", thread::current().id())
            .replace("ThreadId(", "")
            .replace(")", "");
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let test_id = format!("{}-{}-{}", test_name, thread_id, timestamp);

        // Create isolated temporary directories
        let cache_dir = TempDir::new().expect("Failed to create cache temp dir");
        let project_dir = TempDir::new().expect("Failed to create project temp dir");

        // Set up environment isolation
        let mut env_guard = EnvGuard::new();

        // Clear any existing cache directory environment variable
        env_guard.remove("DEBTMAP_CACHE_DIR");

        // Set test-specific cache directory
        env_guard.set("DEBTMAP_CACHE_DIR", cache_dir.path().to_str().unwrap());

        Self {
            test_id,
            cache_dir,
            project_dir,
            env_guard,
        }
    }

    /// Get the cache directory path
    #[allow(dead_code)]
    pub fn cache_path(&self) -> &Path {
        self.cache_dir.path()
    }

    /// Get the project directory path
    #[allow(dead_code)]
    pub fn project_path(&self) -> &Path {
        self.project_dir.path()
    }

    /// Create a new isolated cache directory with a specific name
    #[allow(dead_code)]
    pub fn create_cache_dir(&self, name: &str) -> PathBuf {
        let dir = self.cache_dir.path().join(name);
        std::fs::create_dir_all(&dir).expect("Failed to create cache directory");
        dir
    }

    /// Create a new isolated project directory with a specific name
    #[allow(dead_code)]
    pub fn create_project_dir(&self, name: &str) -> PathBuf {
        let dir = self.project_dir.path().join(name);
        std::fs::create_dir_all(&dir).expect("Failed to create project directory");
        dir
    }
}

/// Ensure a project ID is unique by appending a suffix if needed
#[allow(dead_code)]
pub fn ensure_unique_project_id(base: &str) -> String {
    let thread_id = format!("{:?}", thread::current().id())
        .replace("ThreadId(", "")
        .replace(")", "");
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros();
    format!("{}-{}-{}", base, thread_id, timestamp)
}
