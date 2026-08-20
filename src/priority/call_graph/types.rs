//! Core types and data structures for call graph representation

use crate::collections::{HashMap, HashSet, Vector};
use crate::core::Language;
use serde::{Deserialize, Deserializer, Serialize};
use std::path::{Component, Path, PathBuf};

/// Stable cross-language function identity. Source line is intentionally absent.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CanonicalSymbolKey {
    pub language: Language,
    pub path: PathBuf,
    pub module: String,
    pub owner: Option<String>,
    pub name: String,
    pub signature: Option<String>,
}

/// Unique identifier for a function in the codebase
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionId {
    pub file: PathBuf,
    pub name: String,
    pub line: usize,
    #[serde(default)]
    pub module_path: String,
}

impl Ord for FunctionId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.file
            .cmp(&other.file)
            .then_with(|| self.line.cmp(&other.line))
            .then_with(|| self.name.cmp(&other.name))
            .then_with(|| self.module_path.cmp(&other.module_path))
    }
}

impl PartialOrd for FunctionId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
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

    /// Convert the legacy location-bearing identifier to stable symbol identity.
    pub fn canonical_symbol(&self) -> CanonicalSymbolKey {
        let (owner, name) = split_owner(&Self::normalize_name(&self.name));
        CanonicalSymbolKey {
            language: Language::from_path(&self.file),
            path: normalize_symbol_path(&self.file),
            module: self.module_path.clone(),
            owner,
            name,
            signature: None,
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

fn split_owner(qualified_name: &str) -> (Option<String>, String) {
    qualified_name
        .rsplit_once("::")
        .map(|(owner, name)| (Some(owner.to_string()), name.to_string()))
        .unwrap_or_else(|| (None, qualified_name.to_string()))
}

fn normalize_symbol_path(path: &Path) -> PathBuf {
    path.components()
        .fold(PathBuf::new(), |mut normalized, component| {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    normalized.pop();
                }
                _ => normalized.push(component.as_os_str()),
            }
            normalized
        })
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
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct FunctionCall {
    pub caller: FunctionId,
    pub callee: FunctionId,
    pub call_type: CallType,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallEdgeProvenance {
    AstDirect,
    ImportResolution,
    TypeResolution,
    FrameworkRegistration,
    Legacy,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallSite {
    pub file: PathBuf,
    pub line: usize,
    pub column: Option<usize>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallEdgeEvidence {
    pub call: FunctionCall,
    pub provenance: CallEdgeProvenance,
    /// Integer percentage in the inclusive range 0..=100.
    pub confidence: u8,
    pub call_site: Option<CallSite>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionOutcome {
    Resolved {
        target: FunctionId,
        provenance: CallEdgeProvenance,
        confidence: u8,
        call_site: Option<CallSite>,
    },
    Ambiguous {
        candidates: Vec<FunctionId>,
    },
    Unresolved {
        query: String,
    },
    Ignored {
        reason: String,
    },
}

/// Type of function call
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
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
#[derive(Debug, Clone, Serialize)]
pub struct CallGraph {
    #[serde(with = "function_id_map")]
    pub(crate) nodes: HashMap<FunctionId, FunctionNode>,
    pub(crate) edges: Vector<FunctionCall>,
    #[serde(default)]
    pub(crate) edge_evidence: Vector<CallEdgeEvidence>,
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

#[derive(Deserialize)]
struct SerializedCallGraph {
    #[serde(with = "function_id_map")]
    nodes: HashMap<FunctionId, FunctionNode>,
    edges: Vector<FunctionCall>,
    #[serde(default)]
    edge_evidence: Vector<CallEdgeEvidence>,
    #[serde(with = "function_id_map")]
    caller_index: HashMap<FunctionId, HashSet<FunctionId>>,
    #[serde(with = "function_id_map")]
    callee_index: HashMap<FunctionId, HashSet<FunctionId>>,
}

impl<'de> Deserialize<'de> for CallGraph {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let serialized = SerializedCallGraph::deserialize(deserializer)?;
        let mut graph = Self {
            nodes: serialized.nodes,
            edges: serialized.edges,
            edge_evidence: serialized.edge_evidence,
            caller_index: serialized.caller_index,
            callee_index: serialized.callee_index,
            fuzzy_index: std::collections::HashMap::new(),
            name_index: std::collections::HashMap::new(),
        };
        graph.rebuild_lookup_indexes();
        Ok(graph)
    }
}

/// Internal node representation for a function
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
pub struct FunctionNode {
    pub id: FunctionId,
    #[serde(default)]
    pub roles: crate::analysis::role_policy::CodeRoles,
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

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum MapRepresentation<V> {
        Lossless(Vec<(FunctionId, V)>),
        Legacy(StdHashMap<String, V>),
    }

    pub fn serialize<S, V>(
        map: &crate::collections::HashMap<FunctionId, V>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: Serialize,
    {
        map.iter().collect::<Vec<_>>().serialize(serializer)
    }

    pub fn deserialize<'de, D, V>(
        deserializer: D,
    ) -> Result<crate::collections::HashMap<FunctionId, V>, D::Error>
    where
        D: Deserializer<'de>,
        V: Deserialize<'de>,
    {
        match MapRepresentation::deserialize(deserializer)? {
            MapRepresentation::Lossless(entries) => Ok(entries.into_iter().collect()),
            MapRepresentation::Legacy(entries) => Ok(deserialize_legacy(entries)),
        }
    }

    fn deserialize_legacy<V>(
        entries: StdHashMap<String, V>,
    ) -> crate::collections::HashMap<FunctionId, V> {
        entries
            .into_iter()
            .filter_map(|(key, value)| legacy_function_id(&key).map(|id| (id, value)))
            .collect()
    }

    fn legacy_function_id(key: &str) -> Option<FunctionId> {
        let parts: Vec<&str> = key.rsplitn(3, ':').collect();
        (parts.len() == 3).then(|| {
            FunctionId::new(
                parts[2].into(),
                parts[1].to_string(),
                parts[0].parse().unwrap_or(0),
            )
        })
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
            edge_evidence: Vector::new(),
            caller_index: HashMap::new(),
            callee_index: HashMap::new(),
            fuzzy_index: std::collections::HashMap::new(),
            name_index: std::collections::HashMap::new(),
        }
    }

    fn rebuild_lookup_indexes(&mut self) {
        for id in self.nodes.keys() {
            self.fuzzy_index
                .entry(id.fuzzy_key())
                .or_default()
                .push(id.clone());
            self.name_index
                .entry(FunctionId::normalize_name(&id.name))
                .or_default()
                .push(id.clone());
        }
        for candidates in self.fuzzy_index.values_mut() {
            candidates.sort();
        }
        for candidates in self.name_index.values_mut() {
            candidates.sort();
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

    #[test]
    fn canonical_symbol_is_stable_across_line_changes() {
        let first = FunctionId::with_module_path(
            PathBuf::from("src/./service.rs"),
            "Worker::run".into(),
            10,
            "crate::service".into(),
        );
        let moved = FunctionId::with_module_path(
            PathBuf::from("src/service.rs"),
            "Worker::run".into(),
            900,
            "crate::service".into(),
        );

        assert_eq!(first.canonical_symbol(), moved.canonical_symbol());
        assert_eq!(first.canonical_symbol().owner.as_deref(), Some("Worker"));
        assert_eq!(first.canonical_symbol().name, "run");
    }

    #[test]
    fn call_graph_roundtrip_preserves_qualified_identity_and_indexes() {
        let id = FunctionId::with_module_path(
            PathBuf::from("C:/workspace/service.rs"),
            "Worker::run".into(),
            42,
            "crate::service".into(),
        );
        let mut graph = CallGraph::new();
        graph.add_function(id.clone(), false, false, 3, 10);

        let json = serde_json::to_string(&graph).unwrap();
        let restored: CallGraph = serde_json::from_str(&json).unwrap();
        let fuzzy = FunctionId::new(id.file.clone(), id.name.clone(), 999);

        assert_eq!(restored.find_function(&fuzzy), Some(id));
    }
}
