//! Core types and data structures for call graph representation

use crate::collections::Vector;
use crate::core::Language;
use foldhash::{HashMap, HashMapExt, HashSet, HashSetExt};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering as AtomicOrdering};
use std::sync::{Arc, RwLock};

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionId {
    pub file: PathBuf,
    pub name: String,
    pub line: usize,
    /// Zero-based identifier column; absent for legacy and non-Rust definitions.
    #[serde(default)]
    pub column: Option<usize>,
    #[serde(default)]
    pub module_path: String,
}

impl Ord for FunctionId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let file = if self.file.as_os_str() == other.file.as_os_str() {
            std::cmp::Ordering::Equal
        } else {
            self.file.cmp(&other.file)
        };
        file.then_with(|| self.line.cmp(&other.line))
            .then_with(|| self.column.cmp(&other.column))
            .then_with(|| self.name.cmp(&other.name))
            .then_with(|| self.module_path.cmp(&other.module_path))
    }
}

impl PartialOrd for FunctionId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Hashes only the file name: hashing a whole `Path` parses every component,
/// and equal paths always share their last component.
impl std::hash::Hash for FunctionId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.file.file_name().hash(state);
        self.name.hash(state);
        self.line.hash(state);
        self.column.hash(state);
        self.module_path.hash(state);
    }
}

impl FunctionId {
    /// Create a new FunctionId
    pub fn new(file: PathBuf, name: String, line: usize) -> Self {
        Self {
            file,
            name,
            line,
            column: None,
            module_path: String::new(),
        }
    }

    /// Create a new FunctionId with module path
    pub fn with_module_path(file: PathBuf, name: String, line: usize, module_path: String) -> Self {
        Self {
            file,
            name,
            line,
            column: None,
            module_path,
        }
    }

    /// Attach the zero-based source identifier column to this definition.
    pub fn with_column(self, column: Option<usize>) -> Self {
        Self { column, ..self }
    }

    /// Get exact key (all fields) for exact matching
    pub fn exact_key(&self) -> ExactFunctionKey {
        ExactFunctionKey {
            file: self.file.clone(),
            name: self.name.clone(),
            line: self.line,
            column: self.column,
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
        let language = Language::from_path(&self.file);
        let (owner, name) = split_owner(language, &Self::normalize_name(&self.name));
        CanonicalSymbolKey {
            language,
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

    /// Canonicalize file path for consistent matching.
    ///
    /// Successful lookups are cached for every analyzed language. Failures stay
    /// uncached so a path created later can still resolve.
    pub fn canonicalize_path(path: &Path) -> PathBuf {
        if let Some(canonical) = CANONICAL.get(path) {
            return canonical.clone();
        }
        match path.canonicalize() {
            Ok(canonical) => {
                remember_canonical(path, &canonical);
                canonical
            }
            Err(_) => path.to_path_buf(),
        }
    }
}

/// Megabytes of successfully canonicalized paths retained for every language.
pub const DEFAULT_CANONICAL_CACHE_MB: usize = 64;

static CANONICAL: once_cell::sync::Lazy<dashmap::DashMap<PathBuf, PathBuf>> =
    once_cell::sync::Lazy::new(dashmap::DashMap::new);
static CANONICAL_BUDGET: AtomicUsize = AtomicUsize::new(DEFAULT_CANONICAL_CACHE_MB << 20);
static CANONICAL_USED: AtomicUsize = AtomicUsize::new(0);

/// Cap cached canonical paths. `0` canonicalizes on every lookup.
pub fn set_canonical_cache_mb(megabytes: usize) {
    CANONICAL_BUDGET.store(megabytes.saturating_mul(1 << 20), AtomicOrdering::Relaxed);
    CANONICAL_USED.store(0, AtomicOrdering::Relaxed);
    CANONICAL.clear();
}

fn remember_canonical(path: &Path, canonical: &Path) {
    let cost = path_bytes(path).saturating_add(path_bytes(canonical));
    if reserve_canonical(cost) {
        CANONICAL.insert(path.to_path_buf(), canonical.to_path_buf());
    }
}

fn path_bytes(path: &Path) -> usize {
    path.as_os_str().len()
}

fn reserve_canonical(cost: usize) -> bool {
    let budget = CANONICAL_BUDGET.load(AtomicOrdering::Relaxed);
    budget > 0
        && CANONICAL_USED
            .fetch_update(AtomicOrdering::Relaxed, AtomicOrdering::Relaxed, |used| {
                let next = used.saturating_add(cost);
                (next <= budget).then_some(next)
            })
            .is_ok()
}

fn split_owner(language: Language, qualified_name: &str) -> (Option<String>, String) {
    let separator = match language {
        Language::Python | Language::Go => ".",
        _ => "::",
    };
    qualified_name
        .rsplit_once(separator)
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
    pub column: Option<usize>,
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

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallEdgeProvenance {
    AstDirect,
    ImportResolution,
    TypeResolution,
    NameHeuristic,
    FrameworkRegistration,
    Legacy,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CallSite {
    pub file: PathBuf,
    pub line: usize,
    pub column: Option<usize>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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

/// Why a call cannot justify an ordinary graph edge.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UncertaintyReason {
    UnknownReceiver,
    AmbiguousDeclaration,
    UnsupportedTypeOperation,
    UnavailableDefinition,
    AnalysisLimit,
}

/// A source call whose admissible targets remain possible rather than resolved.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct UncertainCall {
    pub caller: FunctionId,
    pub call_site: CallSite,
    /// Position in a caller's legacy call summary when a source column is unavailable.
    /// This distinguishes repeated occurrences without inventing source coordinates.
    #[serde(default)]
    pub call_ordinal: Option<usize>,
    pub lexical_module: String,
    pub call_type: CallType,
    pub query: String,
    pub receiver: Option<String>,
    pub candidates: Vec<FunctionId>,
    pub reason: UncertaintyReason,
}

/// Main call graph structure containing nodes and edges
#[derive(Debug, Clone, Serialize)]
pub struct CallGraph {
    #[serde(with = "function_id_map")]
    pub(crate) nodes: HashMap<FunctionId, FunctionNode>,
    pub(crate) edges: Vector<FunctionCall>,
    #[serde(default)]
    pub(crate) edge_evidence: Vector<CallEdgeEvidence>,
    #[serde(default)]
    pub(crate) uncertain_calls: BTreeSet<UncertainCall>,
    #[serde(default, with = "function_id_btree_map")]
    pub(crate) effect_assessments:
        BTreeMap<FunctionId, crate::analysis::effect_evidence::EffectAssessment>,
    #[serde(skip)]
    pub(crate) effect_assessments_propagated: bool,
    #[serde(skip)]
    pub(crate) possible_caller_index: HashMap<FunctionId, HashSet<FunctionId>>,
    #[serde(skip)]
    pub(crate) possible_callee_index: HashMap<FunctionId, HashSet<FunctionId>>,
    #[serde(skip)]
    pub(crate) edge_set: HashSet<FunctionCall>,
    #[serde(skip)]
    pub(crate) evidence_set: HashSet<CallEdgeEvidence>,
    #[serde(with = "function_id_map")]
    pub(crate) caller_index: HashMap<FunctionId, HashSet<FunctionId>>,
    #[serde(with = "function_id_map")]
    pub(crate) callee_index: HashMap<FunctionId, HashSet<FunctionId>>,

    // Fuzzy matching indexes (not serialized - rebuilt on load)
    #[serde(skip)]
    pub(crate) fuzzy_index: HashMap<FuzzyFunctionKey, Vec<FunctionId>>,
    #[serde(skip)]
    pub(crate) name_index: HashMap<String, Vec<FunctionId>>,
    /// Sorted function ids. Built on first read and cleared when a new id is inserted.
    #[serde(skip)]
    ordered_ids: OrderedFunctionIds,
    /// Internal and external call counts keyed by caller file.
    #[serde(skip)]
    caller_file_counts: CallerFileCallCounts,
}

#[derive(Clone, Copy, Debug, Default)]
struct FileCallCounts {
    internal: usize,
    external: usize,
}

/// Built on first read. `ready` skips the lock while the cache is empty.
#[derive(Debug)]
struct CallerFileCallCounts {
    ready: AtomicBool,
    counts: RwLock<Option<Arc<HashMap<PathBuf, FileCallCounts>>>>,
}

/// Shared sorted ids. Readers during parallel analysis only hold `&CallGraph`.
#[derive(Debug)]
struct OrderedFunctionIds(std::sync::RwLock<Option<std::sync::Arc<Vec<FunctionId>>>>);

impl Clone for OrderedFunctionIds {
    fn clone(&self) -> Self {
        let ids = self
            .0
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        Self(std::sync::RwLock::new(ids))
    }
}

impl Default for OrderedFunctionIds {
    fn default() -> Self {
        Self(std::sync::RwLock::new(None))
    }
}

impl Clone for CallerFileCallCounts {
    fn clone(&self) -> Self {
        let counts = self
            .counts
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        Self {
            ready: AtomicBool::new(counts.is_some()),
            counts: RwLock::new(counts),
        }
    }
}

impl Default for CallerFileCallCounts {
    fn default() -> Self {
        Self {
            ready: AtomicBool::new(false),
            counts: RwLock::new(None),
        }
    }
}

#[derive(Deserialize)]
struct SerializedCallGraph {
    #[serde(with = "function_id_map")]
    nodes: HashMap<FunctionId, FunctionNode>,
    edges: Vector<FunctionCall>,
    #[serde(default)]
    edge_evidence: Vector<CallEdgeEvidence>,
    #[serde(default)]
    uncertain_calls: Vector<UncertainCall>,
    #[serde(default, with = "function_id_btree_map")]
    effect_assessments: BTreeMap<FunctionId, crate::analysis::effect_evidence::EffectAssessment>,
    // Consume the historical fields in binary formats, then rebuild these indexes.
    #[serde(default, rename = "caller_index", with = "function_id_map")]
    _caller_index: HashMap<FunctionId, HashSet<FunctionId>>,
    #[serde(default, rename = "callee_index", with = "function_id_map")]
    _callee_index: HashMap<FunctionId, HashSet<FunctionId>>,
}

impl<'de> Deserialize<'de> for CallGraph {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let serialized = SerializedCallGraph::deserialize(deserializer)?;
        let mut graph = Self::new();
        graph.nodes = serialized
            .nodes
            .into_iter()
            .map(|(id, node)| (id, node.normalized()))
            .collect();
        graph.rebuild_lookup_indexes();
        for evidence in serialized.edge_evidence {
            graph.add_call_with_evidence(evidence);
        }
        for call in serialized.edges {
            if !graph.edge_set.contains(&call) {
                graph.add_call(call);
            }
        }
        for call in serialized.uncertain_calls {
            graph.record_uncertain_call(call);
        }
        for (id, assessment) in serialized.effect_assessments {
            graph.record_effect_assessment(id, assessment);
        }
        Ok(graph)
    }
}

/// Internal node representation for a function
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
pub struct FunctionNode {
    pub id: FunctionId,
    #[serde(default)]
    pub role_evidence: crate::analysis::role_policy::RoleEvidence,
    #[serde(default)]
    pub roles: crate::analysis::role_policy::CodeRoles,
    pub is_entry_point: bool,
    pub is_test: bool,
    pub complexity: u32,
    pub _lines: usize,
}

impl FunctionNode {
    /// Preserve legacy role flags while keeping explicit evidence unchanged when sufficient.
    pub(crate) fn effective_role_evidence(&self) -> crate::analysis::role_policy::RoleEvidence {
        use crate::analysis::role_policy::{
            CodeRoles, classify_roles, evidence_from_roles, merge_evidence,
        };
        let known = classify_roles(&self.role_evidence);
        let missing = CodeRoles {
            is_entry_point: (self.is_entry_point || self.roles.is_entry_point)
                && !known.is_entry_point,
            is_test: (self.is_test || self.roles.is_test) && !known.is_test,
            is_framework_managed: self.roles.is_framework_managed && !known.is_framework_managed,
            is_public_api: self.roles.is_public_api && !known.is_public_api,
        };
        if missing == CodeRoles::default() {
            return self.role_evidence.clone();
        }
        merge_evidence(&self.role_evidence, &evidence_from_roles(missing))
    }

    fn normalized(self) -> Self {
        let role_evidence = self.effective_role_evidence();
        let roles = crate::analysis::role_policy::classify_roles(&role_evidence);
        Self {
            role_evidence,
            roles,
            is_entry_point: roles.is_entry_point,
            is_test: roles.is_test,
            ..self
        }
    }
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

    pub fn serialize<S, V>(map: &HashMap<FunctionId, V>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: Serialize,
    {
        map.iter().collect::<Vec<_>>().serialize(serializer)
    }

    pub fn deserialize<'de, D, V>(deserializer: D) -> Result<HashMap<FunctionId, V>, D::Error>
    where
        D: Deserializer<'de>,
        V: Deserialize<'de>,
    {
        if !deserializer.is_human_readable() {
            return Vec::<(FunctionId, V)>::deserialize(deserializer)
                .map(|entries| entries.into_iter().collect());
        }
        match MapRepresentation::deserialize(deserializer)? {
            MapRepresentation::Lossless(entries) => Ok(entries.into_iter().collect()),
            MapRepresentation::Legacy(entries) => Ok(deserialize_legacy(entries)),
        }
    }

    fn deserialize_legacy<V>(entries: StdHashMap<String, V>) -> HashMap<FunctionId, V> {
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

// JSON object keys cannot represent structured identities. A sorted sequence
// remains lossless in JSON and deterministic in every supported format.
mod function_id_btree_map {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S, V>(map: &BTreeMap<FunctionId, V>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: Serialize,
    {
        map.iter().collect::<Vec<_>>().serialize(serializer)
    }

    pub fn deserialize<'de, D, V>(deserializer: D) -> Result<BTreeMap<FunctionId, V>, D::Error>
    where
        D: Deserializer<'de>,
        V: Deserialize<'de>,
    {
        Vec::<(FunctionId, V)>::deserialize(deserializer)
            .map(|entries| entries.into_iter().collect())
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
            uncertain_calls: BTreeSet::new(),
            effect_assessments: BTreeMap::new(),
            effect_assessments_propagated: false,
            possible_caller_index: HashMap::new(),
            possible_callee_index: HashMap::new(),
            edge_set: HashSet::new(),
            evidence_set: HashSet::new(),
            caller_index: HashMap::new(),
            callee_index: HashMap::new(),
            fuzzy_index: HashMap::new(),
            name_index: HashMap::new(),
            ordered_ids: OrderedFunctionIds::default(),
            caller_file_counts: CallerFileCallCounts::default(),
        }
    }

    /// Calls whose caller is in `file`: `(internal, external)`.
    pub(crate) fn calls_from_file(&self, file: &Path) -> (usize, usize) {
        self.caller_file_call_counts()
            .get(file)
            .map(|counts| (counts.internal, counts.external))
            .unwrap_or((0, 0))
    }

    fn caller_file_call_counts(&self) -> Arc<HashMap<PathBuf, FileCallCounts>> {
        if let Some(counts) = self.cached_caller_file_counts() {
            return counts;
        }
        self.store_caller_file_counts()
    }

    fn cached_caller_file_counts(&self) -> Option<Arc<HashMap<PathBuf, FileCallCounts>>> {
        self.caller_file_counts
            .ready
            .load(AtomicOrdering::Relaxed)
            .then(|| {
                self.caller_file_counts
                    .counts
                    .read()
                    .unwrap_or_else(|error| error.into_inner())
                    .as_ref()
                    .cloned()
            })
            .flatten()
    }

    fn store_caller_file_counts(&self) -> Arc<HashMap<PathBuf, FileCallCounts>> {
        let counts = Arc::new(count_calls_by_caller_file(&self.edges));
        let mut guard = self
            .caller_file_counts
            .counts
            .write()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(existing) = guard.as_ref() {
            self.caller_file_counts
                .ready
                .store(true, AtomicOrdering::Relaxed);
            return Arc::clone(existing);
        }
        *guard = Some(Arc::clone(&counts));
        self.caller_file_counts
            .ready
            .store(true, AtomicOrdering::Relaxed);
        counts
    }

    pub(crate) fn invalidate_caller_file_counts(&mut self) {
        if self
            .caller_file_counts
            .ready
            .swap(false, AtomicOrdering::Relaxed)
        {
            *self
                .caller_file_counts
                .counts
                .write()
                .unwrap_or_else(|error| error.into_inner()) = None;
        }
    }

    pub(crate) fn ordered_function_ids(&self) -> std::sync::Arc<Vec<FunctionId>> {
        if let Some(ids) = self.cached_function_ids() {
            return ids;
        }
        self.store_ordered_function_ids()
    }

    fn cached_function_ids(&self) -> Option<std::sync::Arc<Vec<FunctionId>>> {
        self.ordered_ids
            .0
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
            .cloned()
    }

    fn store_ordered_function_ids(&self) -> std::sync::Arc<Vec<FunctionId>> {
        let mut guard = self
            .ordered_ids
            .0
            .write()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(ids) = guard.as_ref() {
            return std::sync::Arc::clone(ids);
        }
        let mut ids: Vec<_> = self.nodes.keys().cloned().collect();
        ids.sort();
        let ids = std::sync::Arc::new(ids);
        *guard = Some(std::sync::Arc::clone(&ids));
        ids
    }

    pub(crate) fn invalidate_ordered_function_ids(&mut self) {
        *self
            .ordered_ids
            .0
            .write()
            .unwrap_or_else(|error| error.into_inner()) = None;
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

fn count_calls_by_caller_file(edges: &Vector<FunctionCall>) -> HashMap<PathBuf, FileCallCounts> {
    edges.iter().fold(HashMap::new(), |mut counts, call| {
        record_caller_file(&mut counts, call);
        counts
    })
}

fn record_caller_file(counts: &mut HashMap<PathBuf, FileCallCounts>, call: &FunctionCall) {
    let entry = counts.entry(call.caller.file.clone()).or_default();
    if call.caller.file == call.callee.file {
        entry.internal += 1;
    } else {
        entry.external += 1;
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
    fn canonical_cache_respects_its_byte_budget() {
        let path = std::env::current_dir().expect("working directory");
        let resolved = path.canonicalize().expect("current directory exists");

        set_canonical_cache_mb(0);
        assert_eq!(FunctionId::canonicalize_path(&path), resolved);
        assert!(!CANONICAL.contains_key(&path));

        CANONICAL_BUDGET.store(1, AtomicOrdering::Relaxed);
        CANONICAL_USED.store(0, AtomicOrdering::Relaxed);
        CANONICAL.clear();
        assert_eq!(FunctionId::canonicalize_path(&path), resolved);
        assert!(!CANONICAL.contains_key(&path));

        set_canonical_cache_mb(DEFAULT_CANONICAL_CACHE_MB);
        assert_eq!(FunctionId::canonicalize_path(&path), resolved);
        assert!(CANONICAL.contains_key(&path));
    }

    #[test]
    fn canonical_symbol_extracts_python_and_go_owners() {
        for (path, qualified_name) in [
            ("src/worker.py", "Worker.run"),
            ("pkg/worker.go", "Worker.Run"),
        ] {
            let symbol =
                FunctionId::new(PathBuf::from(path), qualified_name.into(), 10).canonical_symbol();

            assert_eq!(symbol.owner.as_deref(), Some("Worker"));
            assert_eq!(
                symbol.name,
                if path.ends_with(".go") { "Run" } else { "run" }
            );
        }
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
