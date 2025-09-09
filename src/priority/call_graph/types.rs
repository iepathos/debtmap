//! Core types and data structures for call graph representation

use im::{HashMap, HashSet, Vector};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Unique identifier for a function in the codebase
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionId {
    pub file: PathBuf,
    pub name: String,
    pub line: usize,
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
}

/// Internal node representation for a function
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
pub(crate) struct FunctionNode {
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
                let func_id = FunctionId {
                    file: parts[2].into(),
                    name: parts[1].to_string(),
                    line: parts[0].parse().unwrap_or(0),
                };
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
        }
    }
}
