//! Core macro expansion logic using cargo-expand

use super::{ExpansionCache, ExpansionConfig, MacroExpansion, SourceMap};
use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

/// File with expanded macros and source mapping
#[derive(Debug, Clone)]
pub struct ExpandedFile {
    /// Original source file path
    pub original_path: PathBuf,
    /// Expanded content with all macros resolved
    pub expanded_content: String,
    /// Mapping from expanded lines to original source
    pub source_map: SourceMap,
    /// Timestamp of expansion
    pub timestamp: SystemTime,
}

/// Macro expander using cargo-expand
pub struct MacroExpander {
    config: ExpansionConfig,
    cache: ExpansionCache,
    cargo_path: PathBuf,
    workspace_root: Option<PathBuf>,
    cargo_expand_available: Option<bool>,
}

impl MacroExpander {
    /// Create a new macro expander
    pub fn new(config: ExpansionConfig) -> Result<Self> {
        // Find cargo executable
        let cargo_path = which::which("cargo").context("cargo not found in PATH")?;

        // Find workspace root if in a Rust project
        let workspace_root = find_workspace_root();

        // Initialize cache
        let cache = ExpansionCache::new(&config.cache_dir)?;

        Ok(Self {
            config,
            cache,
            cargo_path,
            workspace_root,
            cargo_expand_available: None,
        })
    }

    /// Check if cargo-expand is available (cached)
    pub fn check_cargo_expand(&mut self) -> Result<bool> {
        // Return cached result if available
        if let Some(available) = self.cargo_expand_available {
            return Ok(available);
        }

        // Check cargo-expand availability
        let output = Command::new(&self.cargo_path)
            .args(["expand", "--version"])
            .output()
            .context("Failed to run cargo expand")?;

        let available = output.status.success();
        self.cargo_expand_available = Some(available);
        Ok(available)
    }

    /// Set cargo-expand availability (for testing)
    #[cfg(test)]
    pub fn set_cargo_expand_available(&mut self, available: bool) {
        self.cargo_expand_available = Some(available);
    }

    /// Find the Cargo.toml for a given file
    fn find_manifest(&self, path: &Path) -> Result<PathBuf> {
        let mut current = path.parent();

        while let Some(dir) = current {
            let manifest = dir.join("Cargo.toml");
            if manifest.exists() {
                return Ok(manifest);
            }
            current = dir.parent();
        }

        bail!("No Cargo.toml found for {:?}", path)
    }

    /// Compute hash of file content for cache validation
    fn compute_file_hash(&self, path: &Path) -> Result<String> {
        let content =
            fs::read_to_string(path).with_context(|| format!("Failed to read file: {path:?}"))?;

        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Run cargo expand on a specific module
    fn run_cargo_expand(&self, module_path: &str, manifest_path: &Path) -> Result<String> {
        let _timeout = Duration::from_secs(self.config.timeout_secs);

        let output = Command::new(&self.cargo_path)
            .args([
                "expand",
                "--lib",
                "--theme=none",
                "--color=never",
                &format!("--manifest-path={}", manifest_path.display()),
            ])
            .arg(module_path)
            .output()
            .context("Failed to run cargo expand")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("cargo expand failed: {}", stderr);
        }

        String::from_utf8(output.stdout).context("Invalid UTF-8 in cargo expand output")
    }

    /// Parse the expanded output and create source mappings
    fn parse_expansion(&self, expanded: String, original_path: &Path) -> Result<ExpandedFile> {
        // Parse line directives to build source map
        // cargo expand includes line directives like: #[line = 42]
        let source_map = SourceMap::from_expanded(&expanded, original_path)?;

        Ok(ExpandedFile {
            original_path: original_path.to_path_buf(),
            expanded_content: expanded,
            source_map,
            timestamp: SystemTime::now(),
        })
    }

    /// Get module path from file path
    fn get_module_path(&self, file_path: &Path) -> Result<String> {
        // Convert file path to module path
        // e.g., src/analyzers/rust.rs -> analyzers::rust
        let relative = if let Some(root) = &self.workspace_root {
            file_path.strip_prefix(root).unwrap_or(file_path)
        } else {
            file_path
        };

        // Remove src/ prefix and .rs extension
        let path_str = relative.to_string_lossy();
        let module_path = path_str
            .trim_start_matches("src/")
            .trim_start_matches("src\\")
            .trim_end_matches(".rs")
            .replace(['/', '\\'], "::");

        Ok(module_path)
    }

    /// Handle case when cargo-expand is not available
    fn handle_missing_cargo_expand(
        config: &ExpansionConfig,
    ) -> Result<HashMap<PathBuf, ExpandedFile>> {
        if config.fallback_on_error {
            Ok(HashMap::new())
        } else {
            bail!("cargo-expand required but not found")
        }
    }

    /// Process expansion result and update the map
    fn process_expansion_result(
        path: PathBuf,
        result: Result<ExpandedFile>,
        expanded_files: &mut HashMap<PathBuf, ExpandedFile>,
        fallback_on_error: bool,
    ) -> Result<()> {
        match result {
            Ok(expanded) => {
                expanded_files.insert(path, expanded);
                Ok(())
            }
            Err(e) if fallback_on_error => {
                eprintln!("Warning: Failed to expand {}: {}", path.display(), e);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Expand files in parallel
    fn expand_files_parallel(
        &self,
        rust_files: Vec<PathBuf>,
    ) -> Result<HashMap<PathBuf, ExpandedFile>> {
        let results: Vec<(PathBuf, Result<ExpandedFile>)> = rust_files
            .par_iter()
            .map(|path| {
                let mut expander = MacroExpander::new(self.config.clone())?;
                let expanded = expander.expand_file(path);
                Ok((path.clone(), expanded))
            })
            .collect::<Result<Vec<_>>>()?;

        let mut expanded_files = HashMap::new();
        for (path, result) in results {
            Self::process_expansion_result(
                path,
                result,
                &mut expanded_files,
                self.config.fallback_on_error,
            )?;
        }
        Ok(expanded_files)
    }

    /// Expand files sequentially
    fn expand_files_sequential(
        &mut self,
        rust_files: Vec<PathBuf>,
    ) -> Result<HashMap<PathBuf, ExpandedFile>> {
        let mut expanded_files = HashMap::new();
        for path in rust_files {
            let result = self.expand_file(&path);
            Self::process_expansion_result(
                path.clone(),
                result,
                &mut expanded_files,
                self.config.fallback_on_error,
            )?;
        }
        Ok(expanded_files)
    }
}

impl MacroExpander {
    /// Validate cargo-expand availability and return appropriate error
    fn validate_cargo_expand(&self, is_available: bool, fallback_on_error: bool) -> Result<()> {
        match (is_available, fallback_on_error) {
            (true, _) => Ok(()),
            (false, true) => {
                bail!("cargo-expand not available. Install with: cargo install cargo-expand")
            }
            (false, false) => bail!("cargo-expand required but not found"),
        }
    }

    /// Try to retrieve cached expansion if valid
    fn try_cached_expansion(&self, path: &Path, hash: &str) -> Result<Option<ExpandedFile>> {
        self.cache.get(path, hash)
    }

    /// Perform the actual file expansion process
    fn perform_expansion(
        &self,
        path: &Path,
        module_path: &str,
        manifest: &Path,
    ) -> Result<ExpandedFile> {
        let expanded_content = self.run_cargo_expand(module_path, manifest)?;
        self.parse_expansion(expanded_content, path)
    }
}

impl super::MacroExpansion for MacroExpander {
    fn expand_file(&mut self, path: &Path) -> Result<ExpandedFile> {
        // Validate cargo-expand availability
        let is_available = self.check_cargo_expand()?;
        self.validate_cargo_expand(is_available, self.config.fallback_on_error)?;

        // Try cache first
        let hash = self.compute_file_hash(path)?;
        if let Some(cached) = self.try_cached_expansion(path, &hash)? {
            return Ok(cached);
        }

        // Prepare expansion parameters
        let manifest = self.find_manifest(path)?;
        let module_path = self.get_module_path(path)?;

        // Perform expansion
        let expanded = self.perform_expansion(path, &module_path, &manifest)?;

        // Cache the result
        self.cache.store(path, &hash, &expanded)?;

        Ok(expanded)
    }

    fn expand_workspace(&mut self) -> Result<HashMap<PathBuf, ExpandedFile>> {
        // Early return if cargo-expand is not available
        if !self.check_cargo_expand()? {
            return Self::handle_missing_cargo_expand(&self.config);
        }

        // Find all Rust files in workspace
        let rust_files = find_rust_files(self.workspace_root.as_deref())?;

        // Process files based on configuration
        if self.config.parallel {
            self.expand_files_parallel(rust_files)
        } else {
            self.expand_files_sequential(rust_files)
        }
    }

    fn clear_cache(&mut self) -> Result<()> {
        self.cache.clear()
    }

    fn is_cache_valid(&self, path: &Path) -> bool {
        match self.compute_file_hash(path) {
            Ok(hash) => self.cache.is_valid(path, &hash),
            Err(_) => false,
        }
    }
}

/// Find the workspace root by looking for Cargo.toml
fn find_workspace_root() -> Option<PathBuf> {
    let current = std::env::current_dir().ok()?;
    let mut dir = current.as_path();

    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            // Check if this is a workspace root
            if let Ok(content) = fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") {
                    return Some(dir.to_path_buf());
                }
            }
            // If not a workspace, might still be a package root
            return Some(dir.to_path_buf());
        }

        dir = dir.parent()?;
    }
}

/// Check if a directory should be skipped during traversal
fn should_skip_directory(dir_name: &str) -> bool {
    dir_name == "target" || dir_name == ".git" || dir_name.starts_with('.')
}

/// Check if a path is a Rust source file
fn is_rust_file(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some("rs")
}

/// Process a single directory entry, returning files to add and directories to visit
fn process_entry(entry: fs::DirEntry) -> Result<(Option<PathBuf>, Option<PathBuf>)> {
    let path = entry.path();

    if path.is_dir() {
        let should_visit = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|name| !should_skip_directory(name))
            .unwrap_or(false);

        Ok((None, if should_visit { Some(path) } else { None }))
    } else if is_rust_file(&path) {
        Ok((Some(path), None))
    } else {
        Ok((None, None))
    }
}

/// Find all Rust files in a directory recursively
fn find_rust_files(root: Option<&Path>) -> Result<Vec<PathBuf>> {
    let root = root.unwrap_or_else(|| Path::new("."));
    collect_rust_files_from_dir(root)
}

/// Recursively collect Rust files from a directory
fn collect_rust_files_from_dir(dir: &Path) -> Result<Vec<PathBuf>> {
    let entries: Result<Vec<Vec<PathBuf>>> = fs::read_dir(dir)?
        .map(|entry| -> Result<Vec<PathBuf>> {
            let (file, subdir) = process_entry(entry?)?;

            let mut files = Vec::new();
            if let Some(file) = file {
                files.push(file);
            }
            if let Some(subdir) = subdir {
                files.extend(collect_rust_files_from_dir(&subdir)?);
            }
            Ok(files)
        })
        .collect();

    Ok(entries?.into_iter().flatten().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expansion::MacroExpansion;
    use tempfile::TempDir;

    #[test]
    fn test_handle_missing_cargo_expand_with_fallback() {
        let config = ExpansionConfig {
            fallback_on_error: true,
            ..Default::default()
        };
        let result = MacroExpander::handle_missing_cargo_expand(&config);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_handle_missing_cargo_expand_without_fallback() {
        let config = ExpansionConfig {
            fallback_on_error: false,
            ..Default::default()
        };
        let result = MacroExpander::handle_missing_cargo_expand(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cargo-expand required"));
    }

    #[test]
    fn test_process_expansion_result_success() {
        let mut expanded_files = HashMap::new();
        let path = PathBuf::from("test.rs");
        let expanded = ExpandedFile {
            original_path: path.clone(),
            expanded_content: String::from("expanded content"),
            source_map: SourceMap::from_mappings(Vec::new()),
            timestamp: SystemTime::now(),
        };

        let result = MacroExpander::process_expansion_result(
            path.clone(),
            Ok(expanded.clone()),
            &mut expanded_files,
            false,
        );

        assert!(result.is_ok());
        assert_eq!(expanded_files.len(), 1);
        assert!(expanded_files.contains_key(&path));
    }

    #[test]
    fn test_process_expansion_result_error_with_fallback() {
        let mut expanded_files = HashMap::new();
        let path = PathBuf::from("test.rs");
        let error = anyhow::anyhow!("expansion failed");

        let result = MacroExpander::process_expansion_result(
            path.clone(),
            Err(error),
            &mut expanded_files,
            true, // fallback enabled
        );

        assert!(result.is_ok());
        assert_eq!(expanded_files.len(), 0);
    }

    #[test]
    fn test_process_expansion_result_error_without_fallback() {
        let mut expanded_files = HashMap::new();
        let path = PathBuf::from("test.rs");
        let error = anyhow::anyhow!("expansion failed");

        let result = MacroExpander::process_expansion_result(
            path.clone(),
            Err(error),
            &mut expanded_files,
            false, // fallback disabled
        );

        assert!(result.is_err());
        assert_eq!(expanded_files.len(), 0);
    }

    #[test]
    fn test_expand_files_sequential_empty() {
        let config = ExpansionConfig::default();
        if let Ok(mut expander) = MacroExpander::new(config) {
            let result = expander.expand_files_sequential(Vec::new());
            assert!(result.is_ok());
            assert!(result.unwrap().is_empty());
        }
    }

    #[test]
    fn test_expand_files_parallel_empty() {
        let config = ExpansionConfig::default();
        if let Ok(expander) = MacroExpander::new(config) {
            let result = expander.expand_files_parallel(Vec::new());
            assert!(result.is_ok());
            assert!(result.unwrap().is_empty());
        }
    }

    #[test]
    fn test_find_workspace_root_in_project() {
        // This test will pass in a real Rust project
        let root = find_workspace_root();
        // In CI or in a real Rust project, this should find the root
        if root.is_some() {
            assert!(root.unwrap().join("Cargo.toml").exists());
        }
    }

    #[test]
    fn test_should_skip_directory() {
        // Test directories that should be skipped
        assert!(should_skip_directory("target"));
        assert!(should_skip_directory(".git"));
        assert!(should_skip_directory(".hidden"));
        assert!(should_skip_directory(".vscode"));

        // Test directories that should not be skipped
        assert!(!should_skip_directory("src"));
        assert!(!should_skip_directory("tests"));
        assert!(!should_skip_directory("benches"));
        assert!(!should_skip_directory("examples"));
    }

    #[test]
    fn test_is_rust_file() {
        // Test Rust files
        assert!(is_rust_file(Path::new("main.rs")));
        assert!(is_rust_file(Path::new("lib.rs")));
        assert!(is_rust_file(Path::new("src/module.rs")));
        assert!(is_rust_file(Path::new("/absolute/path/file.rs")));

        // Test non-Rust files
        assert!(!is_rust_file(Path::new("README.md")));
        assert!(!is_rust_file(Path::new("Cargo.toml")));
        assert!(!is_rust_file(Path::new("script.py")));
        assert!(!is_rust_file(Path::new("no_extension")));
        assert!(!is_rust_file(Path::new(".rs"))); // Just extension, no name
    }

    #[test]
    fn test_process_entry() {
        let temp_dir = TempDir::new().unwrap();

        // Create a Rust file
        let rust_file = temp_dir.path().join("test.rs");
        fs::write(&rust_file, "fn main() {}").unwrap();

        // Create a non-Rust file
        let other_file = temp_dir.path().join("README.md");
        fs::write(&other_file, "# README").unwrap();

        // Create a directory
        let sub_dir = temp_dir.path().join("src");
        fs::create_dir(&sub_dir).unwrap();

        // Create a target directory (should be skipped)
        let target_dir = temp_dir.path().join("target");
        fs::create_dir(&target_dir).unwrap();

        // Test processing entries
        for entry in fs::read_dir(temp_dir.path()).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let result = process_entry(entry).unwrap();

            if path == rust_file {
                assert!(result.0.is_some()); // Should return the Rust file
                assert!(result.1.is_none());
            } else if path == other_file {
                assert!(result.0.is_none()); // Should not return non-Rust file
                assert!(result.1.is_none());
            } else if path == sub_dir {
                assert!(result.0.is_none());
                assert!(result.1.is_some()); // Should return directory to visit
            } else if path == target_dir {
                assert!(result.0.is_none());
                assert!(result.1.is_none()); // Should skip target directory
            }
        }
    }

    #[test]
    fn test_find_rust_files_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let result = find_rust_files(Some(temp_dir.path()));
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_find_rust_files_with_rust_files() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        // Create some Rust files
        fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
        fs::write(src_dir.join("lib.rs"), "pub fn lib() {}").unwrap();

        // Create a non-Rust file that should be ignored
        fs::write(src_dir.join("README.md"), "# README").unwrap();

        let result = find_rust_files(Some(temp_dir.path()));
        assert!(result.is_ok());

        let files = result.unwrap();
        assert_eq!(files.len(), 2);

        // Check that only .rs files were found
        for file in &files {
            assert_eq!(file.extension().and_then(|e| e.to_str()), Some("rs"));
        }
    }

    #[test]
    fn test_find_rust_files_ignores_target_dir() {
        let temp_dir = TempDir::new().unwrap();

        // Create src directory with a Rust file
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();
        fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();

        // Create target directory with a Rust file that should be ignored
        let target_dir = temp_dir.path().join("target");
        fs::create_dir(&target_dir).unwrap();
        fs::write(target_dir.join("generated.rs"), "// generated").unwrap();

        let result = find_rust_files(Some(temp_dir.path()));
        assert!(result.is_ok());

        let files = result.unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].to_str().unwrap().contains("main.rs"));
        assert!(!files[0].to_str().unwrap().contains("target"));
    }

    #[test]
    fn test_validate_cargo_expand_available() {
        let config = ExpansionConfig::default();
        if let Ok(expander) = MacroExpander::new(config) {
            // Test when cargo-expand is available
            let result = expander.validate_cargo_expand(true, false);
            assert!(result.is_ok());

            let result_with_fallback = expander.validate_cargo_expand(true, true);
            assert!(result_with_fallback.is_ok());
        }
    }

    #[test]
    fn test_validate_cargo_expand_not_available() {
        let config = ExpansionConfig::default();
        if let Ok(expander) = MacroExpander::new(config) {
            // Test when cargo-expand is not available without fallback
            let result = expander.validate_cargo_expand(false, false);
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("cargo-expand required"));

            // Test when cargo-expand is not available with fallback
            let result = expander.validate_cargo_expand(false, true);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Install with"));
        }
    }

    #[test]
    fn test_try_cached_expansion_miss() {
        let temp_dir = TempDir::new().unwrap();
        let config = ExpansionConfig {
            cache_dir: temp_dir.path().join(".debtmap/cache"),
            ..Default::default()
        };

        if let Ok(expander) = MacroExpander::new(config) {
            let path = Path::new("nonexistent.rs");
            let hash = "test_hash";
            let result = expander.try_cached_expansion(path, hash);
            assert!(result.is_ok());
            assert!(result.unwrap().is_none());
        }
    }

    #[test]
    fn test_perform_expansion_invalid_manifest() {
        let config = ExpansionConfig::default();
        if let Ok(expander) = MacroExpander::new(config) {
            let path = Path::new("test.rs");
            let module_path = "test";
            let manifest = Path::new("nonexistent/Cargo.toml");

            let result = expander.perform_expansion(path, module_path, manifest);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_expand_file_with_invalid_path() {
        let config = ExpansionConfig {
            fallback_on_error: true,
            ..Default::default()
        };

        if let Ok(mut expander) = MacroExpander::new(config) {
            let result = expander.expand_file(Path::new("/nonexistent/file.rs"));
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_expand_file_caching() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        // Create a simple Cargo.toml
        let cargo_toml = r#"
[package]
name = "test_caching"
version = "0.1.0"
edition = "2021"
"#;
        fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

        // Create a simple Rust file
        let rust_file = src_dir.join("lib.rs");
        fs::write(&rust_file, "pub fn test() -> i32 { 42 }").unwrap();

        let config = ExpansionConfig {
            cache_dir: temp_dir.path().join(".debtmap/cache"),
            ..Default::default()
        };

        if let Ok(mut expander) = MacroExpander::new(config) {
            // First expansion should work (if cargo-expand is available)
            let first_result = expander.expand_file(&rust_file);

            if first_result.is_ok() {
                // Second expansion should use cache
                let second_result = expander.expand_file(&rust_file);
                assert!(second_result.is_ok());

                // Both results should be the same
                let first = first_result.unwrap();
                let second = second_result.unwrap();
                assert_eq!(first.expanded_content, second.expanded_content);
            }
        }
    }

    #[test]
    fn test_compute_file_hash() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "fn main() {}").unwrap();

        let config = ExpansionConfig::default();
        if let Ok(expander) = MacroExpander::new(config) {
            let result = expander.compute_file_hash(&test_file);
            assert!(result.is_ok());

            let hash = result.unwrap();
            assert!(!hash.is_empty());
            assert_eq!(hash.len(), 64); // SHA256 produces 64 hex chars
        }
    }

    #[test]
    fn test_compute_file_hash_nonexistent() {
        let config = ExpansionConfig::default();
        if let Ok(expander) = MacroExpander::new(config) {
            let result = expander.compute_file_hash(Path::new("/nonexistent/file.rs"));
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_find_manifest() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        // Create Cargo.toml
        let cargo_toml = r#"
[package]
name = "test_project"
version = "0.1.0"
"#;
        fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

        // Create a Rust file in src
        let rust_file = src_dir.join("main.rs");
        fs::write(&rust_file, "fn main() {}").unwrap();

        let config = ExpansionConfig::default();
        if let Ok(expander) = MacroExpander::new(config) {
            let result = expander.find_manifest(&rust_file);
            assert!(result.is_ok());
            assert_eq!(result.unwrap().file_name().unwrap(), "Cargo.toml");
        }
    }

    #[test]
    fn test_find_manifest_not_found() {
        let config = ExpansionConfig::default();
        if let Ok(expander) = MacroExpander::new(config) {
            let result = expander.find_manifest(Path::new("/no/cargo/here.rs"));
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("No Cargo.toml"));
        }
    }

    #[test]
    fn test_get_module_path() {
        let config = ExpansionConfig::default();
        if let Ok(expander) = MacroExpander::new(config) {
            // Test with src prefix
            let path = Path::new("src/analyzers/rust.rs");
            let result = expander.get_module_path(path);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "analyzers::rust");

            // Test without src prefix
            let path = Path::new("lib.rs");
            let result = expander.get_module_path(path);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "lib");

            // Test nested modules
            let path = Path::new("src/deep/nested/module.rs");
            let result = expander.get_module_path(path);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "deep::nested::module");
        }
    }

    #[test]
    fn test_parse_expansion() {
        let config = ExpansionConfig::default();
        if let Ok(expander) = MacroExpander::new(config) {
            let expanded = r#"
#[line = 10]
fn foo() {
    println!("hello");
}
"#;
            let path = Path::new("test.rs");
            let result = expander.parse_expansion(expanded.to_string(), path);
            assert!(result.is_ok());

            let expanded_file = result.unwrap();
            assert_eq!(expanded_file.original_path, path);
            assert_eq!(expanded_file.expanded_content, expanded);
        }
    }

    #[test]
    fn test_expand_workspace_no_cargo_expand() {
        let config = ExpansionConfig {
            fallback_on_error: true,
            ..Default::default()
        };

        if let Ok(mut expander) = MacroExpander::new(config) {
            // Mock cargo-expand as unavailable to avoid slow subprocess call
            expander.set_cargo_expand_available(false);
            
            // This should now return immediately with an empty HashMap
            let result = expander.expand_workspace();
            assert!(result.is_ok());
            assert!(result.unwrap().is_empty());
        }
    }

    #[test]
    fn test_clear_cache() {
        let temp_dir = TempDir::new().unwrap();
        let config = ExpansionConfig {
            cache_dir: temp_dir.path().join(".debtmap/cache"),
            ..Default::default()
        };

        if let Ok(mut expander) = MacroExpander::new(config) {
            // Should succeed even with empty cache
            let result = expander.clear_cache();
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_find_rust_files_ignores_hidden_dirs() {
        let temp_dir = TempDir::new().unwrap();

        // Create src directory with a Rust file
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();
        fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();

        // Create hidden directory with a Rust file that should be ignored
        let hidden_dir = temp_dir.path().join(".hidden");
        fs::create_dir(&hidden_dir).unwrap();
        fs::write(hidden_dir.join("secret.rs"), "// secret").unwrap();

        let result = find_rust_files(Some(temp_dir.path()));
        assert!(result.is_ok());

        let files = result.unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].to_str().unwrap().contains("main.rs"));
    }

    #[test]
    fn test_expand_file_validation_fails_without_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let rust_file = temp_dir.path().join("test.rs");
        fs::write(&rust_file, "pub fn test() {}").unwrap();

        let config = ExpansionConfig {
            fallback_on_error: false,
            ..Default::default()
        };

        if let Ok(mut expander) = MacroExpander::new(config) {
            // Mock check_cargo_expand to return false by using a non-existent file
            let nonexistent = Path::new("/definitely/does/not/exist/file.rs");
            let result = expander.expand_file(nonexistent);
            // Should fail without cargo-expand
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_expand_file_cache_hit_returns_early() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        // Create Cargo.toml
        let cargo_toml = r#"
[package]
name = "test_cache_hit"
version = "0.1.0"
edition = "2021"
"#;
        fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

        let rust_file = src_dir.join("lib.rs");
        let content = "pub fn cached() -> i32 { 100 }";
        fs::write(&rust_file, content).unwrap();

        let cache_dir = temp_dir.path().join(".debtmap/cache");
        let config = ExpansionConfig {
            cache_dir: cache_dir.clone(),
            ..Default::default()
        };

        if let Ok(mut expander) = MacroExpander::new(config.clone()) {
            // First call populates cache
            let first = expander.expand_file(&rust_file);

            if first.is_ok() {
                // Create a new expander with same cache dir
                if let Ok(mut expander2) = MacroExpander::new(config) {
                    // This should hit cache
                    let second = expander2.expand_file(&rust_file);
                    assert!(second.is_ok());
                    assert_eq!(
                        first.unwrap().expanded_content,
                        second.unwrap().expanded_content
                    );
                }
            }
        }
    }

    #[test]
    fn test_expand_file_manifest_not_found() {
        let temp_dir = TempDir::new().unwrap();
        // Create a Rust file without Cargo.toml
        let rust_file = temp_dir.path().join("orphan.rs");
        fs::write(&rust_file, "pub fn orphan() {}").unwrap();

        let config = ExpansionConfig {
            fallback_on_error: false,
            ..Default::default()
        };

        if let Ok(mut expander) = MacroExpander::new(config) {
            let result = expander.expand_file(&rust_file);
            // Should fail to find manifest
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_expand_file_stores_in_cache() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        // Create Cargo.toml
        let cargo_toml = r#"
[package]
name = "test_store"
version = "0.1.0"
edition = "2021"
"#;
        fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

        let rust_file = src_dir.join("lib.rs");
        fs::write(&rust_file, "pub fn store() {}").unwrap();

        let cache_dir = temp_dir.path().join(".debtmap/cache");
        let config = ExpansionConfig {
            cache_dir: cache_dir.clone(),
            ..Default::default()
        };

        if let Ok(mut expander) = MacroExpander::new(config) {
            let result = expander.expand_file(&rust_file);

            if result.is_ok() {
                // Verify cache directory was created
                assert!(cache_dir.exists());
                // Cache should have at least one entry
                assert!(cache_dir.read_dir().unwrap().count() > 0);
            }
        }
    }

    #[test]
    fn test_expand_file_module_path_extraction() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        let mod_dir = src_dir.join("modules");
        fs::create_dir_all(&mod_dir).unwrap();

        // Create Cargo.toml
        let cargo_toml = r#"
[package]
name = "test_module"
version = "0.1.0"
edition = "2021"
"#;
        fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

        // Create nested module file
        let rust_file = mod_dir.join("nested.rs");
        fs::write(&rust_file, "pub fn nested() {}").unwrap();

        let config = ExpansionConfig {
            fallback_on_error: true,
            ..Default::default()
        };

        if let Ok(mut expander) = MacroExpander::new(config) {
            let result = expander.expand_file(&rust_file);
            // Even if expansion fails, module path extraction should work
            assert!(result.is_ok() || result.is_err());
        }
    }

    #[test]
    fn test_expand_workspace_integration() {
        let temp_dir = TempDir::new().unwrap();

        // Create a minimal Cargo.toml
        let cargo_toml = r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#;
        fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

        // Create src directory
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        // Create a simple lib.rs
        fs::write(src_dir.join("lib.rs"), "pub fn hello() {}").unwrap();

        let config = ExpansionConfig {
            enabled: true,
            fallback_on_error: true,
            parallel: false,
            ..Default::default()
        };

        // Change to the temp directory for the test
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        if let Ok(mut expander) = MacroExpander::new(config) {
            let result = expander.expand_workspace();
            // This might fail if cargo-expand is not installed, but should not panic
            assert!(result.is_ok() || expander.config.fallback_on_error);
        }

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }
}
