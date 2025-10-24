//! Core types and data structures for call graph representation

use im::{HashMap, HashSet, Vector};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Unique identifier for a function in the codebase
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionId {
    pub file: PathBuf,
    pub name: String,
    pub line: usize,
    #[serde(default)]
    pub module_path: String,
}

impl FunctionId {
    /// Create a new FunctionId
    pub fn new(file: PathBuf, name: String, line: usize) -> Self {
        Self {
            file,
            name,
            line,
            module_path: String::new(),
        }
    }

    /// Create a new FunctionId with module path
    pub fn with_module_path(file: PathBuf, name: String, line: usize, module_path: String) -> Self {
        Self {
            file,
            name,
            line,
            module_path,
        }
    }

    /// Get exact key (all fields) for exact matching
    pub fn exact_key(&self) -> ExactFunctionKey {
        ExactFunctionKey {
            file: self.file.clone(),
            name: self.name.clone(),
            line: self.line,
            module_path: self.module_path.clone(),
        }
    }

    /// Get fuzzy key (name + file only) for fuzzy matching
    pub fn fuzzy_key(&self) -> FuzzyFunctionKey {
        FuzzyFunctionKey {
            canonical_file: Self::canonicalize_path(&self.file),
            normalized_name: Self::normalize_name(&self.name),
        }
    }

    /// Get simple key (name only) for name-only matching
    pub fn simple_key(&self) -> SimpleFunctionKey {
        SimpleFunctionKey {
            normalized_name: Self::normalize_name(&self.name),
        }
    }

    /// Normalize function name (strip generics, whitespace)
    pub fn normalize_name(name: &str) -> String {
        // Find the first '<' character indicating generics
        let base_name = name.split('<').next().unwrap_or(name);
        // Remove extra whitespace
        base_name.trim().to_string()
    }

    /// Canonicalize file path for consistent matching
    pub fn canonicalize_path(path: &Path) -> PathBuf {
        // Try to canonicalize the path, but if it fails (e.g., file doesn't exist),
        // just use the path as-is
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    }
}

/// Different matching strategies for FunctionId
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchStrategy {
    /// All fields must match exactly
    Exact,
    /// Name and normalized file must match (ignores line/module_path)
    Fuzzy,
    /// Only function name must match (returns multiple candidates)
    NameOnly,
}

/// Key for exact lookups (current behavior)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ExactFunctionKey {
    pub file: PathBuf,
    pub name: String,
    pub line: usize,
    pub module_path: String,
}

/// Key for fuzzy lookups (name + file, ignores line/module)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct FuzzyFunctionKey {
    pub canonical_file: PathBuf,
    pub normalized_name: String,
}

/// Key for name-only lookups
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct SimpleFunctionKey {
    pub normalized_name: String,
}

/// Represents a function call relationship between two functions
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionCall {
    pub caller: FunctionId,
    pub callee: FunctionId,
    pub call_type: CallType,
}

/// Type of function call
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallType {
    Direct,
    Delegate,
    Pipeline,
    Async,
    Callback,
    /// Dynamic dispatch through observer pattern
    ObserverDispatch,
}

/// Main call graph structure containing nodes and edges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraph {
    #[serde(with = "function_id_map")]
    pub(crate) nodes: HashMap<FunctionId, FunctionNode>,
    pub(crate) edges: Vector<FunctionCall>,
    #[serde(with = "function_id_map")]
    pub(crate) caller_index: HashMap<FunctionId, HashSet<FunctionId>>,
    #[serde(with = "function_id_map")]
    pub(crate) callee_index: HashMap<FunctionId, HashSet<FunctionId>>,

    // Fuzzy matching indexes (not serialized - rebuilt on load)
    #[serde(skip)]
    pub(crate) fuzzy_index: std::collections::HashMap<FuzzyFunctionKey, Vec<FunctionId>>,
    #[serde(skip)]
    pub(crate) name_index: std::collections::HashMap<String, Vec<FunctionId>>,
}

/// Internal node representation for a function
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
pub struct FunctionNode {
    pub id: FunctionId,
    pub is_entry_point: bool,
    pub is_test: bool,
    pub complexity: u32,
    pub _lines: usize,
}

// Custom serialization for HashMap with FunctionId keys
mod function_id_map {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap as StdHashMap;

    pub fn serialize<S, V>(
        map: &im::HashMap<FunctionId, V>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: Serialize,
    {
        let string_map: StdHashMap<String, &V> = map
            .iter()
            .map(|(k, v)| (format!("{}:{}:{}", k.file.display(), k.name, k.line), v))
            .collect();
        string_map.serialize(serializer)
    }

    pub fn deserialize<'de, D, V>(deserializer: D) -> Result<im::HashMap<FunctionId, V>, D::Error>
    where
        D: Deserializer<'de>,
        V: Deserialize<'de> + Clone,
    {
        let string_map: StdHashMap<String, V> = StdHashMap::deserialize(deserializer)?;
        let mut result = im::HashMap::new();
        for (key, value) in string_map {
            let parts: Vec<&str> = key.rsplitn(3, ':').collect();
            if parts.len() == 3 {
                let func_id = FunctionId::new(
                    parts[2].into(),
                    parts[1].to_string(),
                    parts[0].parse().unwrap_or(0),
                );
                result.insert(func_id, value);
            }
        }
        Ok(result)
    }
}

impl Default for CallGraph {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export core functionality from other modules
impl CallGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vector::new(),
            caller_index: HashMap::new(),
            callee_index: HashMap::new(),
            fuzzy_index: std::collections::HashMap::new(),
            name_index: std::collections::HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_generic_name() {
        assert_eq!(FunctionId::normalize_name("foo<T>"), "foo");
        assert_eq!(FunctionId::normalize_name("bar<A, B>"), "bar");
        assert_eq!(FunctionId::normalize_name("baz"), "baz");
        assert_eq!(FunctionId::normalize_name("map<String>"), "map");
        assert_eq!(FunctionId::normalize_name("process< T , U >"), "process");
    }

    #[test]
    fn test_normalize_name_preserves_namespace() {
        assert_eq!(FunctionId::normalize_name("std::vec::Vec"), "std::vec::Vec");
        assert_eq!(
            FunctionId::normalize_name("crate::module::function"),
            "crate::module::function"
        );
    }

    #[test]
    fn test_fuzzy_key_equality() {
        let id1 = FunctionId::new(PathBuf::from("test.rs"), "foo".to_string(), 100);
        let id2 = FunctionId::new(PathBuf::from("test.rs"), "foo".to_string(), 200);

        // Same name + file, different lines should have equal fuzzy keys
        assert_eq!(id1.fuzzy_key(), id2.fuzzy_key());
    }

    #[test]
    fn test_fuzzy_key_different_files() {
        let id1 = FunctionId::new(PathBuf::from("test1.rs"), "foo".to_string(), 100);
        let id2 = FunctionId::new(PathBuf::from("test2.rs"), "foo".to_string(), 100);

        // Different files should have different fuzzy keys
        assert_ne!(id1.fuzzy_key(), id2.fuzzy_key());
    }

    #[test]
    fn test_simple_key_ignores_file_and_line() {
        let id1 = FunctionId::new(PathBuf::from("test1.rs"), "foo".to_string(), 100);
        let id2 = FunctionId::new(PathBuf::from("test2.rs"), "foo".to_string(), 200);

        // Same name should have equal simple keys regardless of file/line
        assert_eq!(id1.simple_key(), id2.simple_key());
    }

    #[test]
    fn test_generic_functions_have_same_fuzzy_key() {
        let id1 = FunctionId::new(PathBuf::from("test.rs"), "map<T>".to_string(), 100);
        let id2 = FunctionId::new(PathBuf::from("test.rs"), "map<String>".to_string(), 100);

        // Generic instantiations should match via fuzzy key
        assert_eq!(id1.fuzzy_key(), id2.fuzzy_key());
    }
}
