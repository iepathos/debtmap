//! Pure functions for cross-platform path normalization
//!
//! This module provides path normalization utilities for matching coverage data
//! across different platforms and environments. All functions are pure (no I/O)
//! and designed for composition.
//!
//! # Architecture
//!
//! Path normalization follows a functional pipeline:
//! 1. Component extraction - break paths into meaningful segments
//! 2. Separator normalization - convert all separators to forward slash
//! 3. Suffix matching - match paths based on meaningful components
//!
//! # Example
//!
//! ```
//! use std::path::Path;
//! use debtmap::risk::path_normalization::{normalize_path_components, paths_match_by_suffix};
//!
//! let query = Path::new("src/lib.rs");
//! let target = Path::new("/home/user/project/src/lib.rs");
//!
//! let query_components = normalize_path_components(query);
//! let target_components = normalize_path_components(target);
//!
//! assert!(paths_match_by_suffix(&query_components, &target_components));
//! ```

use std::path::{Component, Path, PathBuf};

/// Pure function: Extract meaningful path components
///
/// Filters out current directory (.) and root components,
/// preserving only the meaningful path segments. This enables
/// cross-platform matching by focusing on the actual file path
/// structure rather than platform-specific prefixes.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use debtmap::risk::path_normalization::normalize_path_components;
///
/// let components = normalize_path_components(Path::new("./src/lib.rs"));
/// assert_eq!(components, vec!["src", "lib.rs"]);
///
/// let components = normalize_path_components(Path::new("/abs/path/src/lib.rs"));
/// assert_eq!(components, vec!["abs", "path", "src", "lib.rs"]);
/// ```
///
/// # Platform Differences
///
/// On Windows:
/// ```ignore
/// let components = normalize_path_components(Path::new(r"C:\Users\dev\src\lib.rs"));
/// assert_eq!(components, vec!["Users", "dev", "src", "lib.rs"]);
/// ```
///
/// # Performance
///
/// O(n) where n is the number of path components. Single-pass iteration
/// with no filesystem I/O.
pub fn normalize_path_components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            Component::RootDir | Component::Prefix(_) => None,
            Component::CurDir => None,
            Component::ParentDir => None, // Skip parent directory references
        })
        .collect()
}

/// Pure function: Normalize path separators to forward slash
///
/// Converts all backslashes to forward slashes for consistent
/// cross-platform matching. Also trims trailing slashes.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use debtmap::risk::path_normalization::normalize_path_separators;
///
/// let normalized = normalize_path_separators(Path::new("src/lib.rs"));
/// assert_eq!(normalized, "src/lib.rs");
/// ```
///
/// On Windows:
/// ```ignore
/// let normalized = normalize_path_separators(Path::new(r"src\utils\helper.rs"));
/// assert_eq!(normalized, "src/utils/helper.rs");
/// ```
///
/// # Performance
///
/// O(n) where n is the string length. Single allocation for the result.
pub fn normalize_path_separators(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string()
}

/// Pure function: Check if query path matches target by suffix
///
/// Returns true if query is a suffix of target, enabling matches
/// like `src/lib.rs` matching `/abs/path/src/lib.rs`.
///
/// # Examples
///
/// ```
/// use debtmap::risk::path_normalization::paths_match_by_suffix;
///
/// let query = vec!["src".to_string(), "lib.rs".to_string()];
/// let target = vec!["project".to_string(), "src".to_string(), "lib.rs".to_string()];
/// assert!(paths_match_by_suffix(&query, &target));
///
/// let non_matching = vec!["other".to_string(), "lib.rs".to_string()];
/// assert!(!paths_match_by_suffix(&query, &non_matching));
/// ```
///
/// # Edge Cases
///
/// - Empty query always returns false
/// - Query longer than target always returns false
/// - Comparison is exact string match on each component
///
/// # Performance
///
/// O(min(query.len(), target.len())) for the slice comparison.
/// No allocations.
pub fn paths_match_by_suffix(query: &[String], target: &[String]) -> bool {
    if query.is_empty() {
        return false;
    }
    if query.len() > target.len() {
        return false;
    }

    let target_suffix = &target[target.len() - query.len()..];
    query == target_suffix
}

/// Strategy used to match a path
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchStrategy {
    /// Exact component match
    ExactComponents,
    /// Query is a suffix of candidate
    QuerySuffix,
    /// Candidate is a suffix of query
    CandidateSuffix,
}

/// Pure function: Find best matching path from available paths
///
/// Tries multiple strategies in order of preference:
/// 1. Exact component match - most reliable
/// 2. Suffix match - query is suffix of candidate (e.g., `src/lib.rs` matches `/abs/src/lib.rs`)
/// 3. Reverse suffix match - candidate is suffix of query
///
/// # Examples
///
/// ```
/// use std::path::{Path, PathBuf};
/// use debtmap::risk::path_normalization::{find_matching_path, MatchStrategy};
///
/// let query = Path::new("src/lib.rs");
/// let candidates = vec![
///     PathBuf::from("/abs/path/src/lib.rs"),
///     PathBuf::from("other/file.rs"),
/// ];
///
/// let result = find_matching_path(query, &candidates);
/// assert!(result.is_some());
/// let (matched, strategy) = result.unwrap();
/// assert_eq!(matched, &PathBuf::from("/abs/path/src/lib.rs"));
/// assert_eq!(strategy, MatchStrategy::QuerySuffix);
/// ```
///
/// # Performance
///
/// O(n * m) where n is the number of candidates and m is the average
/// number of path components. Tries exact match first (fast path),
/// then falls back to suffix matching.
pub fn find_matching_path<'a>(
    query_path: &Path,
    available_paths: &'a [PathBuf],
) -> Option<(&'a PathBuf, MatchStrategy)> {
    let query_components = normalize_path_components(query_path);

    // Strategy 1: Exact match
    for candidate in available_paths {
        let candidate_components = normalize_path_components(candidate);
        if query_components == candidate_components {
            return Some((candidate, MatchStrategy::ExactComponents));
        }
    }

    // Strategy 2: Query is suffix of candidate
    for candidate in available_paths {
        let candidate_components = normalize_path_components(candidate);
        if paths_match_by_suffix(&query_components, &candidate_components) {
            return Some((candidate, MatchStrategy::QuerySuffix));
        }
    }

    // Strategy 3: Candidate is suffix of query
    for candidate in available_paths {
        let candidate_components = normalize_path_components(candidate);
        if paths_match_by_suffix(&candidate_components, &query_components) {
            return Some((candidate, MatchStrategy::CandidateSuffix));
        }
    }

    None
}

/// Result of path normalization with diagnostic info
#[derive(Debug, Clone)]
pub struct NormalizedPath {
    /// Original path as provided
    pub original: PathBuf,
    /// Normalized component representation
    pub components: Vec<String>,
    /// Normalized string form (forward slashes)
    pub normalized_str: String,
}

impl NormalizedPath {
    /// Create a normalized path from a Path reference
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use debtmap::risk::path_normalization::NormalizedPath;
    ///
    /// let normalized = NormalizedPath::from_path(Path::new("./src/lib.rs"));
    /// assert_eq!(normalized.components, vec!["src", "lib.rs"]);
    /// assert_eq!(normalized.normalized_str, "src/lib.rs");
    /// ```
    pub fn from_path(path: &Path) -> Self {
        Self {
            original: path.to_path_buf(),
            components: normalize_path_components(path),
            normalized_str: normalize_path_separators(path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_components_unix() {
        assert_eq!(
            normalize_path_components(Path::new("./src/lib.rs")),
            vec!["src", "lib.rs"]
        );
        assert_eq!(
            normalize_path_components(Path::new("/home/user/project/src/lib.rs")),
            vec!["home", "user", "project", "src", "lib.rs"]
        );
    }

    #[test]
    fn test_normalize_path_components_relative() {
        assert_eq!(
            normalize_path_components(Path::new("src/lib.rs")),
            vec!["src", "lib.rs"]
        );
        assert_eq!(
            normalize_path_components(Path::new("./././src/lib.rs")),
            vec!["src", "lib.rs"]
        );
    }

    #[test]
    fn test_normalize_path_components_empty() {
        assert_eq!(
            normalize_path_components(Path::new("")),
            Vec::<String>::new()
        );
        assert_eq!(
            normalize_path_components(Path::new(".")),
            Vec::<String>::new()
        );
        assert_eq!(
            normalize_path_components(Path::new("./")),
            Vec::<String>::new()
        );
    }

    #[test]
    fn test_normalize_path_components_root() {
        assert_eq!(
            normalize_path_components(Path::new("/")),
            Vec::<String>::new()
        );
    }

    #[test]
    fn test_normalize_path_separators_unix() {
        assert_eq!(
            normalize_path_separators(Path::new("src/lib.rs")),
            "src/lib.rs"
        );
        assert_eq!(
            normalize_path_separators(Path::new("/abs/path/src/lib.rs")),
            "/abs/path/src/lib.rs"
        );
    }

    #[test]
    fn test_normalize_path_separators_backslash() {
        // Simulate Windows-style paths
        assert_eq!(
            normalize_path_separators(Path::new("src\\lib.rs")),
            "src/lib.rs"
        );
        assert_eq!(
            normalize_path_separators(Path::new("src\\utils\\helper.rs")),
            "src/utils/helper.rs"
        );
    }

    #[test]
    fn test_normalize_path_separators_mixed() {
        // Mixed separators
        assert_eq!(
            normalize_path_separators(Path::new("src/utils\\helper.rs")),
            "src/utils/helper.rs"
        );
    }

    #[test]
    fn test_normalize_path_separators_trailing_slash() {
        assert_eq!(
            normalize_path_separators(Path::new("src/lib.rs/")),
            "src/lib.rs"
        );
        assert_eq!(
            normalize_path_separators(Path::new("src/lib.rs//")),
            "src/lib.rs"
        );
    }

    #[test]
    fn test_paths_match_by_suffix() {
        let query = vec!["src".to_string(), "lib.rs".to_string()];
        let target = vec![
            "project".to_string(),
            "src".to_string(),
            "lib.rs".to_string(),
        ];
        assert!(paths_match_by_suffix(&query, &target));

        let non_matching = vec!["other".to_string(), "lib.rs".to_string()];
        assert!(!paths_match_by_suffix(&query, &non_matching));
    }

    #[test]
    fn test_paths_match_by_suffix_exact() {
        let query = vec!["src".to_string(), "lib.rs".to_string()];
        let target = vec!["src".to_string(), "lib.rs".to_string()];
        assert!(paths_match_by_suffix(&query, &target));
    }

    #[test]
    fn test_paths_match_by_suffix_empty_query() {
        let query: Vec<String> = vec![];
        let target = vec!["src".to_string(), "lib.rs".to_string()];
        assert!(!paths_match_by_suffix(&query, &target));
    }

    #[test]
    fn test_paths_match_by_suffix_query_longer() {
        let query = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let target = vec!["b".to_string(), "c".to_string()];
        assert!(!paths_match_by_suffix(&query, &target));
    }

    #[test]
    fn test_find_matching_path_exact() {
        let query = PathBuf::from("src/lib.rs");
        let candidates = vec![PathBuf::from("src/lib.rs"), PathBuf::from("other/file.rs")];

        let result = find_matching_path(&query, &candidates);
        assert!(result.is_some());
        let (matched, strategy) = result.unwrap();
        assert_eq!(matched, &PathBuf::from("src/lib.rs"));
        assert_eq!(strategy, MatchStrategy::ExactComponents);
    }

    #[test]
    fn test_find_matching_path_query_suffix() {
        let query = PathBuf::from("src/lib.rs");
        let candidates = vec![
            PathBuf::from("/abs/path/src/lib.rs"),
            PathBuf::from("other/file.rs"),
        ];

        let result = find_matching_path(&query, &candidates);
        assert!(result.is_some());
        let (matched, strategy) = result.unwrap();
        assert_eq!(matched, &PathBuf::from("/abs/path/src/lib.rs"));
        assert_eq!(strategy, MatchStrategy::QuerySuffix);
    }

    #[test]
    fn test_find_matching_path_candidate_suffix() {
        let query = PathBuf::from("/abs/path/src/lib.rs");
        let candidates = vec![PathBuf::from("src/lib.rs"), PathBuf::from("other/file.rs")];

        let result = find_matching_path(&query, &candidates);
        assert!(result.is_some());
        let (matched, strategy) = result.unwrap();
        assert_eq!(matched, &PathBuf::from("src/lib.rs"));
        assert_eq!(strategy, MatchStrategy::CandidateSuffix);
    }

    #[test]
    fn test_find_matching_path_no_match() {
        let query = PathBuf::from("src/lib.rs");
        let candidates = vec![
            PathBuf::from("other/file.rs"),
            PathBuf::from("different/path.rs"),
        ];

        let result = find_matching_path(&query, &candidates);
        assert!(result.is_none());
    }

    #[test]
    fn test_normalized_path_from_path() {
        let normalized = NormalizedPath::from_path(Path::new("./src/lib.rs"));
        assert_eq!(normalized.components, vec!["src", "lib.rs"]);
        assert_eq!(normalized.normalized_str, "./src/lib.rs"); // Preserves original form
        assert_eq!(normalized.original, PathBuf::from("./src/lib.rs"));
    }

    // Cross-platform integration tests
    #[test]
    fn test_windows_unix_cross_platform_match() {
        // Simulate LCOV from Windows (simulated with forward slashes since we're on Unix)
        let lcov_path = PathBuf::from("C:/project/src/lib.rs");
        // Query from Unix
        let query_path = PathBuf::from("/home/dev/project/src/lib.rs");

        let lcov_components = normalize_path_components(&lcov_path);
        let query_components = normalize_path_components(&query_path);

        // Should match on suffix
        let common_suffix = vec!["src".to_string(), "lib.rs".to_string()];
        assert!(paths_match_by_suffix(&common_suffix, &lcov_components));
        assert!(paths_match_by_suffix(&common_suffix, &query_components));
    }

    #[test]
    fn test_docker_container_path_match() {
        // Docker container might have different absolute path but same relative structure
        let container_path = PathBuf::from("/app/src/lib.rs");
        let host_path = PathBuf::from("src/lib.rs");

        let candidates = vec![container_path.clone()];
        let result = find_matching_path(&host_path, &candidates);

        // Host relative path should match as suffix of container absolute path
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, &container_path);
        assert_eq!(result.unwrap().1, MatchStrategy::QuerySuffix);
    }

    #[test]
    fn test_cargo_workspace_relative_paths() {
        let workspace_root = PathBuf::from("crates/parser/src/lib.rs");
        let member_relative = PathBuf::from("src/lib.rs");

        let candidates = vec![workspace_root.clone()];
        let result = find_matching_path(&member_relative, &candidates);

        assert!(result.is_some());
        assert_eq!(result.unwrap().0, &workspace_root);
    }
}
