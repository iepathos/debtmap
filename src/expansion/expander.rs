//! Core macro expansion logic using cargo-expand

use super::{ExpansionCache, ExpansionConfig, SourceMap};
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
        })
    }

    /// Check if cargo-expand is available
    pub fn check_cargo_expand(&self) -> Result<bool> {
        let output = Command::new(&self.cargo_path)
            .args(["expand", "--version"])
            .output()
            .context("Failed to run cargo expand")?;

        Ok(output.status.success())
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
}

impl super::MacroExpansion for MacroExpander {
    fn expand_file(&mut self, path: &Path) -> Result<ExpandedFile> {
        // Check if cargo-expand is available
        if !self.check_cargo_expand()? {
            if self.config.fallback_on_error {
                bail!("cargo-expand not available. Install with: cargo install cargo-expand");
            } else {
                bail!("cargo-expand required but not found");
            }
        }

        // Check cache first
        let hash = self.compute_file_hash(path)?;
        if let Some(cached) = self.cache.get(path, &hash)? {
            return Ok(cached);
        }

        // Find manifest
        let manifest = self.find_manifest(path)?;

        // Get module path
        let module_path = self.get_module_path(path)?;

        // Run expansion
        let expanded_content = self.run_cargo_expand(&module_path, &manifest)?;

        // Parse and create expanded file
        let expanded = self.parse_expansion(expanded_content, path)?;

        // Cache the result
        self.cache.store(path, &hash, &expanded)?;

        Ok(expanded)
    }

    fn expand_workspace(&mut self) -> Result<HashMap<PathBuf, ExpandedFile>> {
        if !self.check_cargo_expand()? {
            if self.config.fallback_on_error {
                return Ok(HashMap::new());
            } else {
                bail!("cargo-expand required but not found");
            }
        }

        // Find all Rust files in workspace
        let rust_files = find_rust_files(self.workspace_root.as_deref())?;

        if self.config.parallel {
            // Parallel expansion
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
                match result {
                    Ok(expanded) => {
                        expanded_files.insert(path, expanded);
                    }
                    Err(e) if self.config.fallback_on_error => {
                        eprintln!("Warning: Failed to expand {}: {}", path.display(), e);
                    }
                    Err(e) => return Err(e),
                }
            }

            Ok(expanded_files)
        } else {
            // Sequential expansion
            let mut expanded_files = HashMap::new();
            for path in rust_files {
                match self.expand_file(&path) {
                    Ok(expanded) => {
                        expanded_files.insert(path.clone(), expanded);
                    }
                    Err(e) if self.config.fallback_on_error => {
                        eprintln!("Warning: Failed to expand {}: {}", path.display(), e);
                    }
                    Err(e) => return Err(e),
                }
            }
            Ok(expanded_files)
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

/// Find all Rust files in a directory recursively
fn find_rust_files(root: Option<&Path>) -> Result<Vec<PathBuf>> {
    let root = root.unwrap_or_else(|| Path::new("."));
    let mut rust_files = Vec::new();

    fn visit_dir(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Skip target and other build directories
                let dir_name = path.file_name().and_then(|n| n.to_str());
                if let Some(name) = dir_name {
                    if name == "target" || name == ".git" || name.starts_with('.') {
                        continue;
                    }
                }
                visit_dir(&path, files)?;
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                files.push(path);
            }
        }
        Ok(())
    }

    visit_dir(root, &mut rust_files)?;
    Ok(rust_files)
}
