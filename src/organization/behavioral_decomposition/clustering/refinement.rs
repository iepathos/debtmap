//! Cluster refinement and pattern detection.
//!
//! Provides functions for:
//! - Subdividing oversized clusters
//! - Merging tiny clusters
//! - Applying Rust-specific naming patterns
//! - Detecting I/O boundary, query, matching, and lookup patterns

use std::collections::HashMap;

use super::super::types::{capitalize_first, BehaviorCategory, MethodCluster};
use super::cohesion::calculate_standalone_cohesion;

const MIN_CLUSTER_SIZE: usize = 3;
const OVERSIZED_CLUSTER_THRESHOLD: usize = 15;
const PATTERN_THRESHOLD: f64 = 0.6;

/// Subdivide oversized Domain clusters using verb patterns.
pub fn subdivide_oversized_clusters(
    clusters: Vec<MethodCluster>,
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster> {
    clusters
        .into_iter()
        .flat_map(|cluster| subdivide_if_oversized(cluster, adjacency))
        .collect()
}

/// Subdivide a single cluster if it's oversized.
fn subdivide_if_oversized(
    cluster: MethodCluster,
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster> {
    if !should_subdivide(&cluster) {
        return vec![cluster];
    }

    let subclusters = cluster_by_verb_patterns(&cluster.methods);

    if subclusters.len() <= 1 {
        return vec![cluster];
    }

    create_subclusters_from_verbs(subclusters, adjacency)
}

/// Check if a cluster should be subdivided.
fn should_subdivide(cluster: &MethodCluster) -> bool {
    cluster.methods.len() > OVERSIZED_CLUSTER_THRESHOLD
        && matches!(cluster.category, BehaviorCategory::Domain(_))
}

/// Create subclusters from verb pattern groups.
fn create_subclusters_from_verbs(
    verb_groups: HashMap<String, Vec<String>>,
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster> {
    verb_groups
        .into_iter()
        .filter(|(_, methods)| methods.len() >= MIN_CLUSTER_SIZE)
        .map(|(verb, methods)| create_subcluster(verb, methods, adjacency))
        .collect()
}

/// Create a single subcluster from a verb group.
fn create_subcluster(
    verb: String,
    methods: Vec<String>,
    adjacency: &HashMap<(String, String), usize>,
) -> MethodCluster {
    let (internal_calls, external_calls) = calculate_standalone_cohesion(&methods, adjacency);

    let mut cluster = MethodCluster {
        category: BehaviorCategory::Domain(verb),
        methods,
        fields_accessed: vec![],
        internal_calls,
        external_calls,
        cohesion_score: 0.0,
    };
    cluster.calculate_cohesion();
    cluster
}

/// Cluster methods by verb/action patterns for secondary subdivision.
fn cluster_by_verb_patterns(methods: &[String]) -> HashMap<String, Vec<String>> {
    let mut clusters: HashMap<String, Vec<String>> = HashMap::new();

    for method in methods {
        let verb = extract_verb_pattern(method);
        clusters.entry(verb).or_default().push(method.clone());
    }

    clusters
}

/// Extract verb pattern from method name for grouping.
fn extract_verb_pattern(method_name: &str) -> String {
    let prefixes = get_verb_prefixes();

    for prefix in &prefixes {
        if method_name.to_lowercase().starts_with(prefix) {
            return capitalize_first(prefix);
        }
    }

    extract_first_word(method_name)
}

/// Get common verb prefixes in Rust.
fn get_verb_prefixes() -> Vec<&'static str> {
    vec![
        "parse",
        "build",
        "create",
        "make",
        "construct",
        "get",
        "fetch",
        "retrieve",
        "find",
        "search",
        "lookup",
        "query",
        "set",
        "update",
        "modify",
        "change",
        "is",
        "has",
        "can",
        "should",
        "check",
        "apply",
        "execute",
        "run",
        "process",
        "handle",
        "demangle",
        "normalize",
        "sanitize",
        "clean",
        "calculate",
        "compute",
        "derive",
        "match",
        "compare",
        "equals",
    ]
}

/// Extract first word from method name as fallback.
fn extract_first_word(method_name: &str) -> String {
    method_name
        .split('_')
        .next()
        .filter(|s| !s.is_empty())
        .map(capitalize_first)
        .unwrap_or_else(|| "Utilities".to_string())
}

/// Merge tiny clusters (<3 methods) into related larger clusters.
pub fn merge_tiny_clusters(clusters: Vec<MethodCluster>) -> Vec<MethodCluster> {
    if clusters.len() <= 1 {
        return clusters;
    }

    let (normal, tiny) = partition_by_size(clusters);
    let (mut normal, unmerged) = merge_tiny_into_related(normal, tiny);

    handle_unmerged_methods(&mut normal, unmerged);
    normal
}

/// Partition clusters into normal (>=3) and tiny (<3).
fn partition_by_size(clusters: Vec<MethodCluster>) -> (Vec<MethodCluster>, Vec<MethodCluster>) {
    clusters
        .into_iter()
        .partition(|c| c.methods.len() >= MIN_CLUSTER_SIZE)
}

/// Try to merge tiny clusters into related normal clusters.
fn merge_tiny_into_related(
    mut normal: Vec<MethodCluster>,
    tiny: Vec<MethodCluster>,
) -> (Vec<MethodCluster>, Vec<String>) {
    let mut unmerged = Vec::new();

    for tiny_cluster in tiny {
        if !try_merge_into_related(&mut normal, &tiny_cluster) {
            unmerged.extend(tiny_cluster.methods);
        }
    }

    (normal, unmerged)
}

/// Try to merge a tiny cluster into a related normal cluster.
fn try_merge_into_related(normal: &mut [MethodCluster], tiny: &MethodCluster) -> bool {
    for cluster in normal.iter_mut() {
        if categories_are_related(&tiny.category, &cluster.category) {
            cluster.methods.extend(tiny.methods.clone());
            return true;
        }
    }
    false
}

/// Handle unmerged methods by creating or extending utilities cluster.
fn handle_unmerged_methods(normal: &mut Vec<MethodCluster>, unmerged: Vec<String>) {
    if unmerged.is_empty() {
        return;
    }

    if let Some(utilities) = find_utilities_cluster(normal) {
        utilities.methods.extend(unmerged);
    } else if unmerged.len() >= MIN_CLUSTER_SIZE {
        normal.push(create_utilities_cluster(unmerged));
    } else if let Some(largest) = find_largest_cluster(normal) {
        largest.methods.extend(unmerged);
    } else {
        normal.push(create_utilities_cluster(unmerged));
    }
}

/// Find existing utilities cluster.
fn find_utilities_cluster(clusters: &mut [MethodCluster]) -> Option<&mut MethodCluster> {
    clusters
        .iter_mut()
        .find(|c| matches!(&c.category, BehaviorCategory::Domain(name) if name == "Utilities"))
}

/// Find largest cluster by method count.
fn find_largest_cluster(clusters: &mut [MethodCluster]) -> Option<&mut MethodCluster> {
    clusters.iter_mut().max_by_key(|c| c.methods.len())
}

/// Create a new utilities cluster.
fn create_utilities_cluster(methods: Vec<String>) -> MethodCluster {
    MethodCluster {
        category: BehaviorCategory::Domain("Utilities".to_string()),
        methods,
        fields_accessed: vec![],
        internal_calls: 0,
        external_calls: 0,
        cohesion_score: 0.0,
    }
}

/// Check if two behavioral categories are related for merging purposes.
pub fn categories_are_related(cat1: &BehaviorCategory, cat2: &BehaviorCategory) -> bool {
    use BehaviorCategory::*;

    match (cat1, cat2) {
        // Same category type
        (Lifecycle, Lifecycle)
        | (StateManagement, StateManagement)
        | (Persistence, Persistence)
        | (Validation, Validation)
        | (Rendering, Rendering)
        | (EventHandling, EventHandling)
        | (Computation, Computation)
        | (Parsing, Parsing)
        | (Filtering, Filtering)
        | (Transformation, Transformation)
        | (DataAccess, DataAccess)
        | (Construction, Construction)
        | (Processing, Processing)
        | (Communication, Communication) => true,

        // Domain categories with same or similar names
        (Domain(name1), Domain(name2)) => names_are_related(name1, name2),

        // Related categories
        (Persistence, StateManagement) | (StateManagement, Persistence) => true,
        (Validation, Computation) | (Computation, Validation) => true,
        (Parsing, DataAccess) | (DataAccess, Parsing) => true,
        (Filtering, Transformation) | (Transformation, Filtering) => true,

        _ => false,
    }
}

/// Check if two domain names are related.
fn names_are_related(name1: &str, name2: &str) -> bool {
    name1 == name2
        || name1.to_lowercase().contains(&name2.to_lowercase())
        || name2.to_lowercase().contains(&name1.to_lowercase())
}

/// Apply Rust-specific naming patterns to improve categorization.
pub fn apply_rust_patterns(clusters: Vec<MethodCluster>) -> Vec<MethodCluster> {
    clusters.into_iter().map(apply_pattern_to_cluster).collect()
}

/// Apply pattern detection to a single cluster.
fn apply_pattern_to_cluster(mut cluster: MethodCluster) -> MethodCluster {
    if !matches!(cluster.category, BehaviorCategory::Domain(_)) {
        return cluster;
    }

    // Only change category if a specific pattern is detected
    // Otherwise preserve the original category
    if let Some(new_category) = detect_pattern(&cluster.methods) {
        cluster.category = new_category;
    }
    cluster
}

/// Detect the dominant pattern in a cluster's methods.
///
/// Returns Some(category) if a specific pattern is detected, None otherwise.
fn detect_pattern(methods: &[String]) -> Option<BehaviorCategory> {
    if is_io_boundary_cluster(methods) {
        Some(BehaviorCategory::Domain("Parser".to_string()))
    } else if is_query_cluster(methods) {
        Some(BehaviorCategory::Domain("Query".to_string()))
    } else if is_matching_cluster(methods) {
        Some(BehaviorCategory::Domain("Matching".to_string()))
    } else if is_lookup_cluster(methods) {
        Some(BehaviorCategory::Domain("Lookup".to_string()))
    } else {
        None // Preserve original category
    }
}

/// Check if cluster is an I/O boundary (parser, reader, writer).
fn is_io_boundary_cluster(methods: &[String]) -> bool {
    let keywords = [
        "parse",
        "read",
        "write",
        "load",
        "save",
        "deserialize",
        "serialize",
    ];
    pattern_ratio(methods, &keywords) > PATTERN_THRESHOLD
}

/// Check if cluster is query-focused.
fn is_query_cluster(methods: &[String]) -> bool {
    let prefixes = ["get_", "fetch_", "retrieve_", "query_"];
    prefix_ratio(methods, &prefixes) > PATTERN_THRESHOLD
}

/// Check if cluster is matching/strategy focused.
fn is_matching_cluster(methods: &[String]) -> bool {
    let keywords = ["match", "compare", "equals", "strategy"];
    pattern_ratio(methods, &keywords) > PATTERN_THRESHOLD
}

/// Check if cluster is lookup-focused.
fn is_lookup_cluster(methods: &[String]) -> bool {
    let prefixes = ["find_", "search_", "lookup_", "locate_"];
    prefix_ratio(methods, &prefixes) > PATTERN_THRESHOLD
}

/// Calculate ratio of methods containing any keyword.
fn pattern_ratio(methods: &[String], keywords: &[&str]) -> f64 {
    let count = methods
        .iter()
        .filter(|m| {
            let lower = m.to_lowercase();
            keywords.iter().any(|kw| lower.contains(kw))
        })
        .count();

    count as f64 / methods.len() as f64
}

/// Calculate ratio of methods starting with any prefix.
fn prefix_ratio(methods: &[String], prefixes: &[&str]) -> f64 {
    let count = methods
        .iter()
        .filter(|m| prefixes.iter().any(|p| m.starts_with(p)))
        .count();

    count as f64 / methods.len() as f64
}

/// Merge clusters with identical categories to avoid duplicate module names.
pub fn merge_duplicate_categories(clusters: Vec<MethodCluster>) -> Vec<MethodCluster> {
    let mut category_map: HashMap<String, MethodCluster> = HashMap::new();

    for cluster in clusters {
        let key = cluster.category.display_name();

        if let Some(existing) = category_map.get_mut(&key) {
            merge_clusters(existing, cluster);
        } else {
            category_map.insert(key, cluster);
        }
    }

    category_map.into_values().collect()
}

/// Merge one cluster into another.
fn merge_clusters(target: &mut MethodCluster, source: MethodCluster) {
    target.methods.extend(source.methods);
    target.internal_calls += source.internal_calls;
    target.external_calls += source.external_calls;
    target.calculate_cohesion();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_verb_pattern_known() {
        assert_eq!(extract_verb_pattern("parse_json"), "Parse");
        assert_eq!(extract_verb_pattern("get_value"), "Get");
        assert_eq!(extract_verb_pattern("calculate_sum"), "Calculate");
    }

    #[test]
    fn test_extract_verb_pattern_unknown() {
        assert_eq!(extract_verb_pattern("custom_method"), "Custom");
        assert_eq!(extract_verb_pattern("another_thing"), "Another");
    }

    #[test]
    fn test_categories_related_same_type() {
        assert!(categories_are_related(
            &BehaviorCategory::Parsing,
            &BehaviorCategory::Parsing
        ));
    }

    #[test]
    fn test_categories_related_cross_type() {
        assert!(categories_are_related(
            &BehaviorCategory::Persistence,
            &BehaviorCategory::StateManagement
        ));
    }

    #[test]
    fn test_categories_not_related() {
        assert!(!categories_are_related(
            &BehaviorCategory::Rendering,
            &BehaviorCategory::Parsing
        ));
    }

    #[test]
    fn test_domain_names_related() {
        assert!(names_are_related("Parser", "Parser"));
        assert!(names_are_related("FileParser", "Parser"));
        assert!(!names_are_related("Parser", "Query"));
    }

    #[test]
    fn test_is_io_boundary_cluster() {
        let methods = vec![
            "parse_json".into(),
            "parse_xml".into(),
            "read_file".into(),
            "other".into(),
        ];
        assert!(is_io_boundary_cluster(&methods));
    }

    #[test]
    fn test_is_not_io_boundary_cluster() {
        let methods = vec![
            "calculate_sum".into(),
            "validate_input".into(),
            "process_data".into(),
        ];
        assert!(!is_io_boundary_cluster(&methods));
    }

    #[test]
    fn test_merge_duplicate_categories() {
        let clusters = vec![
            MethodCluster {
                category: BehaviorCategory::Parsing,
                methods: vec!["a".into()],
                fields_accessed: vec![],
                internal_calls: 1,
                external_calls: 0,
                cohesion_score: 0.5,
            },
            MethodCluster {
                category: BehaviorCategory::Parsing,
                methods: vec!["b".into()],
                fields_accessed: vec![],
                internal_calls: 2,
                external_calls: 1,
                cohesion_score: 0.6,
            },
        ];

        let merged = merge_duplicate_categories(clusters);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].methods.len(), 2);
    }
}
