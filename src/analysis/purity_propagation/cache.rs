//! Persistent Caching for Purity Propagation Results
//!
//! This module provides persistent caching of purity propagation results to avoid
//! re-analysis on subsequent runs. It uses deterministic dependency fingerprints
//! and postcard for efficient serialization.

use crate::priority::call_graph::FunctionId;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use super::PurityResult;

const CACHE_MAGIC: &[u8; 8] = b"DMPURTY\0";
const CACHE_VERSION: u32 = 2;
const CACHE_FILE: &str = ".debtmap/purity_cache.postcard";

/// Version of the semantic model represented by dependency assessment hashes.
///
/// This is intentionally independent from the binary cache format version. A
/// model change must invalidate callers even when their dependency identities
/// and the cache encoding itself are unchanged.
pub const PURITY_MODEL_VERSION: u32 = 1;
const PURITY_MODEL_NAMESPACE: &str = "rust-effect-evidence";

/// Persistent cache for purity propagation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurityCache {
    /// Schema version for migration compatibility
    version: u32,

    /// Semantic analysis version for invalidating otherwise compatible entries.
    model_version: u32,

    /// Cached purity results indexed by function ID
    entries: HashMap<FunctionId, CachedPurity>,
}

/// Cached purity entry with validation information
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedPurity {
    /// Purity propagation result
    result: PurityResult,

    /// xxHash64 of function source code
    source_hash: u64,

    /// Deterministic hash of dependency identities and assessment summaries
    deps_hash: u64,

    /// File modification time (seconds since epoch)
    file_mtime: u64,
}

impl PurityCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            version: CACHE_VERSION,
            model_version: PURITY_MODEL_VERSION,
            entries: HashMap::new(),
        }
    }

    /// Load cache from disk, creating new if doesn't exist
    pub fn load(project_root: &Path) -> Result<Self> {
        let cache_path = project_root.join(CACHE_FILE);

        if !cache_path.exists() {
            return Ok(Self::new());
        }

        let bytes = std::fs::read(&cache_path)?;
        Ok(Self::decode_current(&bytes).unwrap_or_else(|| {
            eprintln!("Purity cache is obsolete or corrupt; rebuilding cache");
            Self::new()
        }))
    }

    /// Save cache to disk
    pub fn save(&self, project_root: &Path) -> Result<()> {
        let cache_path = project_root.join(CACHE_FILE);
        std::fs::create_dir_all(cache_path.parent().unwrap())?;

        let bytes = self.encode_current()?;
        std::fs::write(&cache_path, bytes)?;

        Ok(())
    }

    /// Check if cached entry is still valid
    pub fn is_valid(
        &self,
        func_id: &FunctionId,
        current_mtime: u64,
        current_source_hash: u64,
        current_deps_hash: u64,
    ) -> bool {
        if let Some(cached) = self.entries.get(func_id) {
            cached.file_mtime == current_mtime
                && cached.source_hash == current_source_hash
                && cached.deps_hash == current_deps_hash
        } else {
            false
        }
    }

    /// Insert a new cache entry
    pub fn insert(
        &mut self,
        func_id: FunctionId,
        result: PurityResult,
        source_hash: u64,
        deps_hash: u64,
        file_mtime: u64,
    ) {
        self.entries.insert(
            func_id,
            CachedPurity {
                result,
                source_hash,
                deps_hash,
                file_mtime,
            },
        );
    }

    /// Get a cached result if valid
    pub fn get(&self, func_id: &FunctionId) -> Option<&PurityResult> {
        self.entries.get(func_id).map(|cached| &cached.result)
    }

    /// Invalidate entries for a specific file
    pub fn invalidate_file(&mut self, file_path: &Path) {
        self.entries.retain(|id, _| id.file != file_path);
    }

    /// Invalidate all entries that depend on a changed function
    pub fn invalidate_dependents(&mut self, changed_func_ids: &[FunctionId]) {
        // For now, we use a simple approach: invalidate all entries
        // A more sophisticated approach would track the call graph
        // and only invalidate transitive callers
        if !changed_func_ids.is_empty() {
            // Simple heuristic: clear cache when dependencies change
            // This ensures correctness at the cost of some redundant analysis
            self.entries.clear();
        }
    }

    /// Get cache size in number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn encode_current(&self) -> std::result::Result<Vec<u8>, postcard::Error> {
        let payload = postcard::to_allocvec(self)?;
        let mut bytes = Vec::with_capacity(CACHE_MAGIC.len() + 4 + payload.len());
        bytes.extend_from_slice(CACHE_MAGIC);
        bytes.extend_from_slice(&CACHE_VERSION.to_le_bytes());
        bytes.extend(payload);
        Ok(bytes)
    }

    fn decode_current(bytes: &[u8]) -> Option<Self> {
        let payload = current_payload(bytes)?;
        let cache: Self = postcard::from_bytes(payload).ok()?;
        (cache.version == CACHE_VERSION && cache.model_version == PURITY_MODEL_VERSION)
            .then_some(cache)
    }
}

fn current_payload(bytes: &[u8]) -> Option<&[u8]> {
    let version_start = CACHE_MAGIC.len();
    let payload_start = version_start + std::mem::size_of::<u32>();
    (bytes.get(..version_start)? == CACHE_MAGIC).then_some(())?;
    let version = u32::from_le_bytes(bytes.get(version_start..payload_start)?.try_into().ok()?);
    (version == CACHE_VERSION).then(|| &bytes[payload_start..])
}

impl Default for PurityCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Hash a string for cache invalidation.
#[allow(dead_code)]
pub fn hash_string(s: &str) -> u64 {
    // For now, use a simple hash
    // TODO: Replace with xxhash-rust once dependency is added
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Hash a list of function IDs for dependency tracking
#[allow(dead_code)]
pub fn hash_deps(deps: &[FunctionId]) -> u64 {
    let dependencies = deps.iter().cloned().map(|id| (id, 0)).collect::<Vec<_>>();
    hash_deps_with_assessments(&dependencies, PURITY_MODEL_VERSION)
}

/// Hash exact dependency identities and their normalized assessment summaries.
///
/// The assessment hash is supplied by the evidence model so a change in a
/// callee's effects or uncertainty invalidates its callers. Ordering does not
/// affect the result, while columns, module paths, assessment changes, and model
/// version changes do.
pub fn hash_deps_with_assessments(dependencies: &[(FunctionId, u64)], model_version: u32) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut sorted = dependencies.to_vec();
    sorted.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    let mut hasher = DefaultHasher::new();
    PURITY_MODEL_NAMESPACE.hash(&mut hasher);
    model_version.hash(&mut hasher);
    sorted.hash(&mut hasher);
    hasher.finish()
}

/// Get file modification time in seconds since epoch
#[allow(dead_code)]
pub fn get_mtime(file_path: &Path) -> Result<u64> {
    let metadata = std::fs::metadata(file_path)?;
    let modified = metadata.modified()?;
    let duration = modified.duration_since(std::time::UNIX_EPOCH)?;
    Ok(duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::purity_analysis::PurityLevel;
    use std::path::PathBuf;

    fn id(name: &str, column: usize, module: &str) -> FunctionId {
        FunctionId::with_module_path(
            PathBuf::from("src/lib.rs"),
            name.to_string(),
            10,
            module.to_string(),
        )
        .with_column(Some(column))
    }

    fn result() -> PurityResult {
        PurityResult {
            level: PurityLevel::ReadOnly,
            confidence: 0.8,
            reason: super::super::PurityReason::Intrinsic,
            assessment: crate::analysis::effect_evidence::EffectAssessment::unknown(
                crate::analysis::effect_evidence::UnresolvedReason::LegacyEvidence,
                "cache test",
            ),
        }
    }

    fn write_cache_bytes(root: &Path, bytes: &[u8]) {
        let path = root.join(CACHE_FILE);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn test_cache_new() {
        let cache = PurityCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.version, CACHE_VERSION);
        assert_eq!(cache.model_version, PURITY_MODEL_VERSION);
    }

    #[test]
    fn test_hash_deps_deterministic() {
        let func1 = FunctionId::new(PathBuf::from("test.rs"), "foo".to_string(), 100);
        let func2 = FunctionId::new(PathBuf::from("test.rs"), "bar".to_string(), 200);

        let deps1 = vec![func1.clone(), func2.clone()];
        let deps2 = vec![func2.clone(), func1.clone()];

        // Hash should be same regardless of order
        assert_eq!(hash_deps(&deps1), hash_deps(&deps2));
    }

    #[test]
    fn dependency_assessment_hash_is_order_independent() {
        let left = id("left", 3, "one");
        let right = id("right", 8, "two");
        let first = vec![(left.clone(), 11), (right.clone(), 22)];
        let second = vec![(right, 22), (left, 11)];
        assert_eq!(
            hash_deps_with_assessments(&first, PURITY_MODEL_VERSION),
            hash_deps_with_assessments(&second, PURITY_MODEL_VERSION)
        );
    }

    #[test]
    fn dependency_hash_includes_exact_identity_assessment_and_model() {
        let base = vec![(id("work", 3, "one"), 11)];
        for changed in [
            vec![(id("work", 4, "one"), 11)],
            vec![(id("work", 3, "two"), 11)],
            vec![(id("work", 3, "one"), 12)],
        ] {
            assert_ne!(
                hash_deps_with_assessments(&base, PURITY_MODEL_VERSION),
                hash_deps_with_assessments(&changed, PURITY_MODEL_VERSION)
            );
        }
        assert_ne!(
            hash_deps_with_assessments(&base, PURITY_MODEL_VERSION),
            hash_deps_with_assessments(&base, PURITY_MODEL_VERSION + 1)
        );
    }

    #[test]
    fn current_cache_round_trips_through_postcard_envelope() {
        let root = tempfile::tempdir().unwrap();
        let function = id("work", 3, "module");
        let mut cache = PurityCache::new();
        cache.insert(function.clone(), result(), 1, 2, 3);
        cache.save(root.path()).unwrap();

        let restored = PurityCache::load(root.path()).unwrap();
        let restored_result = restored.get(&function).unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored_result.level, PurityLevel::ReadOnly);
        assert_eq!(restored_result.confidence, 0.8);
    }

    #[test]
    fn obsolete_header_rebuilds_without_decoding_payload() {
        let root = tempfile::tempdir().unwrap();
        let mut bytes = CACHE_MAGIC.to_vec();
        bytes.extend_from_slice(&(CACHE_VERSION - 1).to_le_bytes());
        bytes.extend_from_slice(b"not postcard");
        write_cache_bytes(root.path(), &bytes);
        assert!(PurityCache::load(root.path()).unwrap().is_empty());
    }

    #[test]
    fn obsolete_model_rebuilds_even_with_current_binary_format() {
        let root = tempfile::tempdir().unwrap();
        let mut cache = PurityCache::new();
        cache.model_version += 1;
        write_cache_bytes(root.path(), &cache.encode_current().unwrap());
        assert!(PurityCache::load(root.path()).unwrap().is_empty());
    }

    #[test]
    fn legacy_unframed_payload_rebuilds_without_new_format_decode() {
        let root = tempfile::tempdir().unwrap();
        let bytes = postcard::to_allocvec(&PurityCache::new()).unwrap();
        write_cache_bytes(root.path(), &bytes);
        assert!(PurityCache::load(root.path()).unwrap().is_empty());
    }

    #[test]
    fn corrupt_current_payload_rebuilds_safely() {
        let root = tempfile::tempdir().unwrap();
        let mut bytes = CACHE_MAGIC.to_vec();
        bytes.extend_from_slice(&CACHE_VERSION.to_le_bytes());
        bytes.extend_from_slice(b"not postcard");
        write_cache_bytes(root.path(), &bytes);
        assert!(PurityCache::load(root.path()).unwrap().is_empty());
    }
}
