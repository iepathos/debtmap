//! Module hierarchy tracking for path resolution
//!
//! This module builds and maintains a tree of module relationships,
//! enabling resolution of relative paths like `super::function`

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Represents the module hierarchy of a Rust project
#[derive(Debug, Clone)]
pub struct ModuleTree {
    /// Maps module path -> file path
    /// Example: "commands::analyze" -> "src/commands/analyze.rs"
    modules: HashMap<String, PathBuf>,

    /// Maps file path -> module path (inverse of modules)
    files: HashMap<PathBuf, String>,

    /// Parent-child relationships
    /// Example: "commands" -> ["commands::analyze", "commands::validate"]
    children: HashMap<String, Vec<String>>,

    /// Child-parent relationships (inverse of children)
    /// Example: "commands::analyze" -> "commands"
    parents: HashMap<String, String>,
}

impl ModuleTree {
    /// Create a new empty module tree
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            files: HashMap::new(),
            children: HashMap::new(),
            parents: HashMap::new(),
        }
    }

    /// Add a module to the tree
    pub fn add_module(&mut self, module_path: String, file_path: PathBuf) {
        self.modules.insert(module_path.clone(), file_path.clone());
        self.files.insert(file_path, module_path.clone());

        // Update parent-child relationships
        if let Some(parent) = Self::extract_parent_module(&module_path) {
            self.children
                .entry(parent.clone())
                .or_default()
                .push(module_path.clone());

            self.parents.insert(module_path, parent);
        }
    }

    /// Extract parent module from a module path
    /// Example: "commands::analyze" -> Some("commands")
    fn extract_parent_module(module_path: &str) -> Option<String> {
        module_path
            .rfind("::")
            .map(|pos| module_path[..pos].to_string())
    }

    /// Resolve a qualified path starting from a given module
    pub fn resolve_path(&self, current_module: &str, path_segments: &[String]) -> Option<String> {
        if path_segments.is_empty() {
            return None;
        }

        match path_segments[0].as_str() {
            "super" => self.resolve_super(current_module, path_segments),
            "self" => self.resolve_self(current_module, path_segments),
            "crate" => self.resolve_crate(path_segments),
            _ => self.resolve_regular(current_module, path_segments),
        }
    }

    /// Resolve `super::` relative paths
    fn resolve_super(&self, current_module: &str, segments: &[String]) -> Option<String> {
        let mut current = current_module.to_string();
        let mut idx = 0;

        // Walk up the hierarchy for each `super`
        while idx < segments.len() && segments[idx] == "super" {
            current = self.parents.get(&current)?.clone();
            idx += 1;
        }

        // Append remaining segments
        if idx < segments.len() {
            let remaining = &segments[idx..];
            if !current.is_empty() {
                current.push_str("::");
            }
            current.push_str(&remaining.join("::"));
        }

        Some(current)
    }

    /// Resolve `self::` relative paths
    fn resolve_self(&self, current_module: &str, segments: &[String]) -> Option<String> {
        if segments.len() <= 1 {
            return Some(current_module.to_string());
        }

        let remaining = &segments[1..];
        let mut result = current_module.to_string();
        if !result.is_empty() {
            result.push_str("::");
        }
        result.push_str(&remaining.join("::"));

        Some(result)
    }

    /// Resolve `crate::` absolute paths
    fn resolve_crate(&self, segments: &[String]) -> Option<String> {
        if segments.len() <= 1 {
            return Some(String::new());
        }

        Some(segments[1..].join("::"))
    }

    /// Resolve regular qualified paths
    fn resolve_regular(&self, current_module: &str, segments: &[String]) -> Option<String> {
        // First try as an absolute path
        let absolute_path = segments.join("::");
        if self.modules.contains_key(&absolute_path) {
            return Some(absolute_path);
        }

        // Try relative to current module
        let relative_path = if current_module.is_empty() {
            segments.join("::")
        } else {
            format!("{}::{}", current_module, segments.join("::"))
        };

        if self.modules.contains_key(&relative_path) {
            return Some(relative_path);
        }

        // Return absolute path as fallback
        Some(absolute_path)
    }

    /// Get the file path for a module
    pub fn get_file(&self, module_path: &str) -> Option<&PathBuf> {
        self.modules.get(module_path)
    }

    /// Get the module path for a file
    pub fn get_module(&self, file_path: &Path) -> Option<&String> {
        self.files.get(file_path)
    }

    /// Get children of a module
    pub fn get_children(&self, module_path: &str) -> Vec<String> {
        self.children.get(module_path).cloned().unwrap_or_default()
    }

    /// Get parent of a module
    pub fn get_parent(&self, module_path: &str) -> Option<&String> {
        self.parents.get(module_path)
    }

    /// Infer module path from file path
    /// Example: src/commands/analyze.rs -> commands::analyze
    pub fn infer_module_from_file(file_path: &Path) -> String {
        let path_str = file_path.to_string_lossy();

        // Find the src/ directory in the path and extract everything after it
        // This handles both relative paths (src/foo.rs) and absolute paths (/path/to/project/src/foo.rs)
        let without_src =
            if let Some(src_idx) = path_str.find("/src/").or_else(|| path_str.find("\\src\\")) {
                // Found src/ in the path, extract everything after it
                let start_idx = src_idx + 5; // Skip "/src/" or "\src\"
                &path_str[start_idx..]
            } else {
                // No src/ found, try stripping it from the beginning as a fallback
                path_str
                    .strip_prefix("src/")
                    .or_else(|| path_str.strip_prefix("src\\"))
                    .unwrap_or(&path_str)
            };

        // Remove .rs extension
        let without_ext = without_src.strip_suffix(".rs").unwrap_or(without_src);

        // Convert path separators to ::
        let module_path = without_ext.replace(['/', '\\'], "::");

        // Handle mod.rs files
        let module_path = module_path.strip_suffix("::mod").unwrap_or(&module_path);

        module_path.to_string()
    }
}

impl Default for ModuleTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_super_qualified_path() {
        let mut tree = ModuleTree::new();
        tree.add_module("builders".to_string(), PathBuf::from("src/builders/mod.rs"));
        tree.add_module(
            "builders::unified_analysis".to_string(),
            PathBuf::from("src/builders/unified_analysis.rs"),
        );
        tree.add_module(
            "builders::call_graph".to_string(),
            PathBuf::from("src/builders/call_graph.rs"),
        );

        // From builders::unified_analysis, super::call_graph should resolve to builders::call_graph
        let resolved = tree.resolve_path(
            "builders::unified_analysis",
            &["super".to_string(), "call_graph".to_string()],
        );
        println!("Resolved super::call_graph: {:?}", resolved);
        assert_eq!(resolved, Some("builders::call_graph".to_string()));
    }

    #[test]
    fn test_add_module() {
        let mut tree = ModuleTree::new();
        tree.add_module(
            "commands::analyze".to_string(),
            PathBuf::from("src/commands/analyze.rs"),
        );

        assert_eq!(
            tree.get_file("commands::analyze"),
            Some(&PathBuf::from("src/commands/analyze.rs"))
        );

        assert_eq!(
            tree.get_module(&PathBuf::from("src/commands/analyze.rs")),
            Some(&"commands::analyze".to_string())
        );
    }

    #[test]
    fn test_parent_child_relationships() {
        let mut tree = ModuleTree::new();
        tree.add_module("commands".to_string(), PathBuf::from("src/commands/mod.rs"));
        tree.add_module(
            "commands::analyze".to_string(),
            PathBuf::from("src/commands/analyze.rs"),
        );

        let children = tree.get_children("commands");
        assert_eq!(children, vec!["commands::analyze"]);

        let parent = tree.get_parent("commands::analyze");
        assert_eq!(parent, Some(&"commands".to_string()));
    }

    #[test]
    fn test_resolve_super() {
        let mut tree = ModuleTree::new();
        tree.add_module("io".to_string(), PathBuf::from("src/io/mod.rs"));
        tree.add_module(
            "io::writers".to_string(),
            PathBuf::from("src/io/writers/mod.rs"),
        );
        tree.add_module(
            "io::writers::markdown".to_string(),
            PathBuf::from("src/io/writers/markdown.rs"),
        );

        let path = vec!["super".to_string(), "helper".to_string()];
        let resolved = tree.resolve_path("io::writers::markdown", &path);
        assert_eq!(resolved, Some("io::writers::helper".to_string()));
    }

    #[test]
    fn test_resolve_multiple_super() {
        let mut tree = ModuleTree::new();
        tree.add_module("a".to_string(), PathBuf::from("src/a/mod.rs"));
        tree.add_module("a::b".to_string(), PathBuf::from("src/a/b/mod.rs"));
        tree.add_module("a::b::c".to_string(), PathBuf::from("src/a/b/c.rs"));

        let path = vec![
            "super".to_string(),
            "super".to_string(),
            "other".to_string(),
        ];
        let resolved = tree.resolve_path("a::b::c", &path);
        assert_eq!(resolved, Some("a::other".to_string()));
    }

    #[test]
    fn test_resolve_self() {
        let mut tree = ModuleTree::new();
        tree.add_module("commands".to_string(), PathBuf::from("src/commands/mod.rs"));

        let path = vec!["self".to_string(), "helper".to_string()];
        let resolved = tree.resolve_path("commands", &path);
        assert_eq!(resolved, Some("commands::helper".to_string()));
    }

    #[test]
    fn test_resolve_crate() {
        let tree = ModuleTree::new();

        let path = vec![
            "crate".to_string(),
            "commands".to_string(),
            "analyze".to_string(),
        ];
        let resolved = tree.resolve_path("", &path);
        assert_eq!(resolved, Some("commands::analyze".to_string()));
    }

    #[test]
    fn test_infer_module_from_file() {
        // Test relative paths
        assert_eq!(
            ModuleTree::infer_module_from_file(&PathBuf::from("src/commands/analyze.rs")),
            "commands::analyze"
        );

        assert_eq!(
            ModuleTree::infer_module_from_file(&PathBuf::from("src/commands/mod.rs")),
            "commands"
        );

        assert_eq!(
            ModuleTree::infer_module_from_file(&PathBuf::from("src/main.rs")),
            "main"
        );

        // Test absolute paths (like those from temp directories)
        assert_eq!(
            ModuleTree::infer_module_from_file(&PathBuf::from("/var/tmp/project/src/module_a.rs")),
            "module_a"
        );

        assert_eq!(
            ModuleTree::infer_module_from_file(&PathBuf::from(
                "/home/user/code/project/src/builders/call_graph.rs"
            )),
            "builders::call_graph"
        );

        // Test Windows-style paths
        #[cfg(target_os = "windows")]
        {
            assert_eq!(
                ModuleTree::infer_module_from_file(&PathBuf::from(
                    "C:\\Users\\dev\\project\\src\\module_b.rs"
                )),
                "module_b"
            );
        }
    }

    #[test]
    fn test_extract_parent_module() {
        assert_eq!(
            ModuleTree::extract_parent_module("commands::analyze"),
            Some("commands".to_string())
        );

        assert_eq!(
            ModuleTree::extract_parent_module("commands::io::writers"),
            Some("commands::io".to_string())
        );

        assert_eq!(ModuleTree::extract_parent_module("root"), None);
    }
}
