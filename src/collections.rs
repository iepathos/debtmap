//! Maps and sets use [`foldhash`]. Ordered results stay in [`Vector`].
//!
//! Content fingerprints stay on xxhash or SHA-2. Foldhash is a table hasher, not a
//! persistent digest. Concurrent maps use the same fast state as the standard maps.

pub use foldhash::fast::RandomState as FoldState;
pub use foldhash::{HashMap, HashMapExt, HashSet, HashSetExt};

/// Ordered, contiguous collection used for result and graph edge lists.
pub type Vector<T> = Vec<T>;

/// Concurrent map keyed with [`FoldState`].
pub type DashMap<K, V> = dashmap::DashMap<K, V, FoldState>;

/// Concurrent set keyed with [`FoldState`].
pub type DashSet<T> = dashmap::DashSet<T, FoldState>;

/// Builds a foldhash map. [`HashMap::from`] only exists for the standard hasher.
pub fn map_from<K, V>(pairs: impl IntoIterator<Item = (K, V)>) -> HashMap<K, V>
where
    K: Eq + std::hash::Hash,
{
    pairs.into_iter().collect()
}

/// Builds a foldhash set. [`HashSet::from`] only exists for the standard hasher.
pub fn set_from<T>(items: impl IntoIterator<Item = T>) -> HashSet<T>
where
    T: Eq + std::hash::Hash,
{
    items.into_iter().collect()
}
