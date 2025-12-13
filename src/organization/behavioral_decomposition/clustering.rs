/// Clustering algorithms for behavioral method grouping.
///
/// This module implements various clustering strategies for grouping methods
/// based on their behavioral characteristics and call patterns:
///
/// - **Call graph analysis**: Builds adjacency matrix from method calls
/// - **Community detection**: Groups methods by call graph connectivity
/// - **Hybrid clustering**: Combines name-based categorization with call graph analysis
/// - **Production-ready clustering**: Comprehensive pipeline with test filtering and size balancing
use std::collections::{HashMap, HashSet};
use syn::visit::Visit;

use super::categorization::{cluster_methods_by_behavior, infer_cluster_category, is_test_method};
use super::types::{capitalize_first, BehaviorCategory, MethodCluster};

/// Build method call adjacency matrix from impl blocks and standalone functions
///
/// This function analyzes method call patterns within impl blocks and standalone
/// functions to build an adjacency matrix showing which methods call which other methods.
///
/// Returns: HashMap<(method_a, method_b), call_count>
pub fn build_method_call_adjacency_matrix(
    impl_blocks: &[&syn::ItemImpl],
) -> HashMap<(String, String), usize> {
    build_method_call_adjacency_matrix_with_functions(impl_blocks, &[])
}

/// Build method call adjacency matrix with support for standalone functions
///
/// This enhanced version also tracks calls between standalone functions in the same file,
/// providing better clustering for modules with utility functions.
///
/// Returns: HashMap<(method_a, method_b), call_count>
pub fn build_method_call_adjacency_matrix_with_functions(
    impl_blocks: &[&syn::ItemImpl],
    standalone_functions: &[&syn::ItemFn],
) -> HashMap<(String, String), usize> {
    use syn::visit::Visit;

    let mut matrix = HashMap::new();

    // Collect all function names for validation
    let mut all_function_names = HashSet::new();

    // Add impl method names
    for impl_block in impl_blocks {
        for item in &impl_block.items {
            if let syn::ImplItem::Fn(method) = item {
                all_function_names.insert(method.sig.ident.to_string());
            }
        }
    }

    // Add standalone function names
    for func in standalone_functions {
        all_function_names.insert(func.sig.ident.to_string());
    }

    // Process impl methods
    for impl_block in impl_blocks {
        for item in &impl_block.items {
            if let syn::ImplItem::Fn(method) = item {
                let method_name = method.sig.ident.to_string();

                // Visit method body to find method calls
                let mut call_visitor = MethodCallVisitor {
                    current_method: method_name.clone(),
                    calls: Vec::new(),
                    all_function_names: &all_function_names,
                };
                call_visitor.visit_impl_item_fn(method);

                // Record calls in adjacency matrix
                for called_method in call_visitor.calls {
                    let key = (method_name.clone(), called_method);
                    *matrix.entry(key).or_insert(0) += 1;
                }
            }
        }
    }

    // Process standalone functions
    for func in standalone_functions {
        let func_name = func.sig.ident.to_string();

        let mut call_visitor = MethodCallVisitor {
            current_method: func_name.clone(),
            calls: Vec::new(),
            all_function_names: &all_function_names,
        };
        call_visitor.visit_item_fn(func);

        // Record calls in adjacency matrix
        for called_function in call_visitor.calls {
            let key = (func_name.clone(), called_function);
            *matrix.entry(key).or_insert(0) += 1;
        }
    }

    matrix
}

/// Visitor to extract method calls from a method body
struct MethodCallVisitor<'a> {
    current_method: String,
    calls: Vec<String>,
    all_function_names: &'a HashSet<String>,
}

impl<'ast, 'a> Visit<'ast> for MethodCallVisitor<'a> {
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        // Check if this is a self.method_name() call
        if let syn::Expr::Path(ref path) = *node.receiver {
            if path
                .path
                .segments
                .first()
                .map(|seg| seg.ident == "self")
                .unwrap_or(false)
            {
                let method_name = node.method.to_string();
                if method_name != self.current_method {
                    self.calls.push(method_name);
                }
            }
        }

        // Continue visiting nested expressions
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        // Check for self::method_name() or Self::method_name() calls
        if let syn::Expr::Path(ref path) = *node.func {
            if path.path.segments.len() >= 2 {
                let first = &path.path.segments[0].ident;
                if first == "self" || first == "Self" {
                    let method_name = path.path.segments[1].ident.to_string();
                    if method_name != self.current_method {
                        self.calls.push(method_name);
                    }
                }
            } else if path.path.segments.len() == 1 {
                // NEW: Track standalone function calls within the same module
                let func_name = path.path.segments[0].ident.to_string();

                // Only track if this is a function defined in the same file
                // and not the current method (avoid self-recursion)
                if func_name != self.current_method && self.all_function_names.contains(&func_name)
                {
                    self.calls.push(func_name);
                }
            }
        }

        syn::visit::visit_expr_call(self, node);
    }
}

/// Apply community detection algorithm to cluster methods
///
/// Uses a simplified Louvain-style algorithm to identify communities
/// of methods with high internal cohesion.
///
/// Algorithm:
/// 1. Start with each method in its own cluster
/// 2. For each method, try moving it to neighbor clusters
/// 3. Accept moves that increase modularity (cohesion)
/// 4. Repeat until no more improvements
pub fn apply_community_detection(
    methods: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster> {
    // Performance optimization: If no method calls exist, skip expensive clustering
    if adjacency.is_empty() {
        return Vec::new();
    }

    // Performance optimization: Limit to reasonable method count for clustering
    // Files with >200 methods should use responsibility-based grouping instead
    const MAX_METHODS_FOR_CLUSTERING: usize = 200;
    if methods.len() > MAX_METHODS_FOR_CLUSTERING {
        return Vec::new();
    }

    // Build initial clusters - one per method
    let mut clusters: HashMap<usize, Vec<String>> = methods
        .iter()
        .enumerate()
        .map(|(i, m)| (i, vec![m.clone()]))
        .collect();

    let mut method_to_cluster: HashMap<String, usize> = methods
        .iter()
        .enumerate()
        .map(|(i, m)| (m.clone(), i))
        .collect();

    let mut improved = true;
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 10;

    // Iteratively improve clustering
    while improved && iterations < MAX_ITERATIONS {
        improved = false;
        iterations += 1;

        for method in methods {
            // Safe: method_to_cluster is built from the same methods slice
            let Some(&current_cluster) = method_to_cluster.get(method) else {
                continue;
            };

            // Find best cluster for this method
            let mut best_cluster = current_cluster;
            let mut best_modularity = calculate_method_modularity(
                method,
                &clusters[&current_cluster],
                adjacency,
                methods,
            );

            // Try all other clusters
            for (cluster_id, cluster_methods) in &clusters {
                if *cluster_id == current_cluster {
                    continue;
                }

                let modularity =
                    calculate_method_modularity(method, cluster_methods, adjacency, methods);

                if modularity > best_modularity {
                    best_modularity = modularity;
                    best_cluster = *cluster_id;
                }
            }

            // Move method if better cluster found
            if best_cluster != current_cluster {
                // Remove from current cluster
                if let Some(cluster) = clusters.get_mut(&current_cluster) {
                    cluster.retain(|m| m != method);
                }

                // Add to best cluster
                clusters
                    .entry(best_cluster)
                    .or_default()
                    .push(method.clone());
                method_to_cluster.insert(method.clone(), best_cluster);
                improved = true;
            }
        }

        // Merge empty clusters
        clusters.retain(|_, methods| !methods.is_empty());
    }

    // Convert to MethodCluster structs
    let clusters_result: Vec<MethodCluster> = clusters
        .into_values()
        .filter(|methods| methods.len() >= 3) // Only clusters with 3+ methods (more granular)
        .map(|methods| {
            let (internal_calls, external_calls) =
                calculate_cluster_cohesion(&methods, adjacency, &method_to_cluster);

            let mut cluster = MethodCluster {
                category: infer_cluster_category(&methods),
                methods: methods.clone(),
                fields_accessed: vec![],
                internal_calls,
                external_calls,
                cohesion_score: 0.0,
            };

            cluster.calculate_cohesion();
            cluster
        })
        .filter(|cluster| cluster.cohesion_score > 0.2) // Filter low-cohesion clusters (relaxed threshold)
        .collect();

    // Return all clusters found, even if only one
    // A single large cohesive cluster can still be a useful signal for splitting
    // (e.g., by responsibility or behavioral patterns)
    clusters_result
}

/// Apply hybrid clustering: name-based categorization refined by community detection
///
/// This improved approach combines the best of both strategies:
/// 1. Initial clustering by behavioral categories (name-based)
/// 2. Refinement using call-graph community detection for large clusters
///
/// Benefits:
/// - Works for files with sparse call graphs (utility modules)
/// - Finds natural cohesion boundaries within behavioral categories
/// - More accurate than either approach alone
pub fn apply_hybrid_clustering(
    methods: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster> {
    // Step 1: Initial clustering by name-based behavioral categories
    let name_clusters = cluster_methods_by_behavior(methods);

    if name_clusters.is_empty() {
        // No meaningful categorization found, fall back to pure community detection
        return apply_community_detection(methods, adjacency);
    }

    let mut refined_clusters = Vec::new();

    // Step 2: Refine each behavioral category
    for (category, cluster_methods) in name_clusters {
        if cluster_methods.len() <= 5 {
            // Small clusters: keep as-is, no need for further splitting
            let mut cluster = MethodCluster {
                category: category.clone(),
                methods: cluster_methods.clone(),
                fields_accessed: vec![],
                internal_calls: 0,
                external_calls: 0,
                cohesion_score: 0.0,
            };

            // Calculate cohesion for this cluster
            let method_to_cluster: HashMap<String, usize> =
                cluster_methods.iter().map(|m| (m.clone(), 0)).collect();

            let (internal_calls, external_calls) =
                calculate_cluster_cohesion(&cluster_methods, adjacency, &method_to_cluster);

            cluster.internal_calls = internal_calls;
            cluster.external_calls = external_calls;
            cluster.calculate_cohesion();

            refined_clusters.push(cluster);
        } else {
            // Large clusters: try to split further using community detection
            let sub_clusters = apply_community_detection(&cluster_methods, adjacency);

            if sub_clusters.is_empty() {
                // Community detection found no useful splits, keep original behavioral cluster
                let mut cluster = MethodCluster {
                    category: category.clone(),
                    methods: cluster_methods.clone(),
                    fields_accessed: vec![],
                    internal_calls: 0,
                    external_calls: 0,
                    cohesion_score: 0.0,
                };

                let method_to_cluster: HashMap<String, usize> =
                    cluster_methods.iter().map(|m| (m.clone(), 0)).collect();

                let (internal_calls, external_calls) =
                    calculate_cluster_cohesion(&cluster_methods, adjacency, &method_to_cluster);

                cluster.internal_calls = internal_calls;
                cluster.external_calls = external_calls;
                cluster.calculate_cohesion();

                refined_clusters.push(cluster);
            } else {
                // Found meaningful subclusters, use those instead
                // But preserve the original behavioral category as a hint
                for subcluster in &sub_clusters {
                    let mut refined_subcluster = subcluster.clone();
                    // If the subcluster's inferred category is generic (Domain),
                    // prefer the original behavioral category
                    if matches!(refined_subcluster.category, BehaviorCategory::Domain(_)) {
                        refined_subcluster.category = category.clone();
                    }
                    refined_clusters.push(refined_subcluster);
                }

                // CRITICAL: Check for lost methods during community detection
                // Community detection may filter out low-cohesion or small clusters,
                // losing methods. We must recover them and keep them in the behavioral category.
                let methods_in_subclusters: std::collections::HashSet<String> = sub_clusters
                    .iter()
                    .flat_map(|c| &c.methods)
                    .cloned()
                    .collect();

                let lost_methods: Vec<String> = cluster_methods
                    .iter()
                    .filter(|m| !methods_in_subclusters.contains(*m))
                    .cloned()
                    .collect();

                if !lost_methods.is_empty() {
                    // Recover lost methods by keeping them in the original behavioral category
                    let mut recovery_cluster = MethodCluster {
                        category: category.clone(),
                        methods: lost_methods.clone(),
                        fields_accessed: vec![],
                        internal_calls: 0,
                        external_calls: 0,
                        cohesion_score: 0.0,
                    };

                    let method_to_cluster: HashMap<String, usize> =
                        lost_methods.iter().map(|m| (m.clone(), 0)).collect();

                    let (internal_calls, external_calls) =
                        calculate_cluster_cohesion(&lost_methods, adjacency, &method_to_cluster);

                    recovery_cluster.internal_calls = internal_calls;
                    recovery_cluster.external_calls = external_calls;
                    recovery_cluster.calculate_cohesion();

                    refined_clusters.push(recovery_cluster);
                }
            }
        }
    }

    // Note: We don't filter by cohesion score here because name-based clusters
    // may have low/zero cohesion (no internal calls) but are still valid behavioral groups.
    // Community detection already filters by cohesion (>0.2), so we trust the categorization.

    refined_clusters
}

/// Apply production-ready clustering with test filtering and size balancing
///
/// This is a refinement pipeline on top of hybrid clustering that:
/// 1. Filters out test methods (should stay in #[cfg(test)])
/// 2. Subdivides oversized Domain clusters using secondary heuristics
/// 3. Merges tiny clusters (<3 methods) into related ones
/// 4. Applies Rust-specific patterns (I/O vs Pure vs Query)
///
/// Use this for generating split recommendations for Rust projects.
pub fn apply_production_ready_clustering(
    methods: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster> {
    // Step 1: Filter out test methods (they should stay in #[cfg(test)] modules)
    let production_methods: Vec<String> = methods
        .iter()
        .filter(|m| !is_test_method(m))
        .cloned()
        .collect();

    if production_methods.is_empty() {
        return Vec::new();
    }

    // Step 2: Apply hybrid clustering on production methods only
    let mut clusters = apply_hybrid_clustering(&production_methods, adjacency);

    // Step 3: Subdivide oversized Domain clusters (>15 methods)
    clusters = subdivide_oversized_clusters(clusters, adjacency);

    // Step 4: Merge tiny clusters (<3 methods) into related ones
    clusters = merge_tiny_clusters(clusters);

    // Step 5: Apply Rust-specific naming patterns
    clusters = apply_rust_patterns(clusters);

    // Step 6: Merge clusters with identical categories to avoid duplicate module names
    clusters = merge_duplicate_categories(clusters);

    // Step 7: CRITICAL - Ensure all methods are accounted for (no method loss)
    clusters = ensure_all_methods_clustered(clusters, &production_methods, adjacency);

    clusters
}

/// Merge clusters that have the same category name to avoid duplicate module names
fn merge_duplicate_categories(clusters: Vec<MethodCluster>) -> Vec<MethodCluster> {
    use std::collections::HashMap;

    let mut category_map: HashMap<String, MethodCluster> = HashMap::new();

    for cluster in clusters {
        let category_key = cluster.category.display_name();

        if let Some(existing) = category_map.get_mut(&category_key) {
            // Merge into existing cluster with same category
            existing.methods.extend(cluster.methods);
            existing.internal_calls += cluster.internal_calls;
            existing.external_calls += cluster.external_calls;
            // Recalculate cohesion after merge
            existing.calculate_cohesion();
        } else {
            // First cluster with this category
            category_map.insert(category_key, cluster);
        }
    }

    category_map.into_values().collect()
}

/// Ensure all production methods are accounted for in clusters
///
/// This is a safety check to prevent method loss during clustering.
/// If any methods are missing (filtered out by cohesion or size thresholds),
/// we create a Utilities cluster to hold them.
fn ensure_all_methods_clustered(
    mut clusters: Vec<MethodCluster>,
    all_methods: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster> {
    use std::collections::HashSet;

    // Collect all methods currently in clusters
    let clustered_methods: HashSet<String> =
        clusters.iter().flat_map(|c| &c.methods).cloned().collect();

    // Find missing methods
    let missing_methods: Vec<String> = all_methods
        .iter()
        .filter(|m| !clustered_methods.contains(*m))
        .cloned()
        .collect();

    if !missing_methods.is_empty() {
        // Log unclustered methods warning (only at verbosity >= 2)
        if crate::progress::ProgressManager::global()
            .map(|pm| pm.verbosity() >= 2)
            .unwrap_or(false)
        {
            eprintln!(
                "WARNING: {} methods were not clustered, merging into existing clusters: {:?}",
                missing_methods.len(),
                &missing_methods[..missing_methods.len().min(5)]
            );
        }

        // Try to find an existing Utilities cluster first
        let utilities_cluster = clusters.iter_mut().find(
            |c| matches!(c.category, BehaviorCategory::Domain(ref name) if name == "Utilities"),
        );

        if let Some(utilities) = utilities_cluster {
            // Merge into existing Utilities cluster
            utilities.methods.extend(missing_methods);
        } else if missing_methods.len() >= 3 {
            // Create new Utilities cluster only if we have enough methods
            let mut utilities_cluster = MethodCluster {
                category: BehaviorCategory::Domain("Utilities".to_string()),
                methods: missing_methods.clone(),
                fields_accessed: vec![],
                internal_calls: 0,
                external_calls: 0,
                cohesion_score: 0.0,
            };

            let method_to_cluster: HashMap<String, usize> =
                missing_methods.iter().map(|m| (m.clone(), 0)).collect();

            let (internal_calls, external_calls) =
                calculate_cluster_cohesion(&missing_methods, adjacency, &method_to_cluster);

            utilities_cluster.internal_calls = internal_calls;
            utilities_cluster.external_calls = external_calls;
            utilities_cluster.calculate_cohesion();

            clusters.push(utilities_cluster);
        } else {
            // <3 missing methods - merge into largest existing cluster to avoid tiny clusters
            if let Some(largest) = clusters.iter_mut().max_by_key(|c| c.methods.len()) {
                largest.methods.extend(missing_methods);
            } else {
                // Edge case: no clusters exist at all
                // Create one anyway (shouldn't happen in practice)
                clusters.push(MethodCluster {
                    category: BehaviorCategory::Domain("Utilities".to_string()),
                    methods: missing_methods,
                    fields_accessed: vec![],
                    internal_calls: 0,
                    external_calls: 0,
                    cohesion_score: 0.0,
                });
            }
        }
    }

    clusters
}

/// Subdivide oversized Domain clusters using secondary heuristics
fn subdivide_oversized_clusters(
    clusters: Vec<MethodCluster>,
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster> {
    let mut result = Vec::new();

    for cluster in clusters {
        // Only subdivide large Domain clusters (>15 methods)
        if cluster.methods.len() > 15 && matches!(cluster.category, BehaviorCategory::Domain(_)) {
            // Try secondary clustering by prefix/verb patterns
            let subclusters = cluster_by_verb_patterns(&cluster.methods);

            if subclusters.len() > 1 {
                // Found meaningful subclusters based on naming
                for (verb, methods) in subclusters {
                    if methods.len() >= 3 {
                        let mut subcluster = MethodCluster {
                            category: BehaviorCategory::Domain(verb),
                            methods: methods.clone(),
                            fields_accessed: vec![],
                            internal_calls: 0,
                            external_calls: 0,
                            cohesion_score: 0.0,
                        };

                        // Calculate cohesion
                        let method_to_cluster: HashMap<String, usize> =
                            methods.iter().map(|m| (m.clone(), 0)).collect();

                        let (internal_calls, external_calls) =
                            calculate_cluster_cohesion(&methods, adjacency, &method_to_cluster);

                        subcluster.internal_calls = internal_calls;
                        subcluster.external_calls = external_calls;
                        subcluster.calculate_cohesion();

                        result.push(subcluster);
                    }
                }
            } else {
                // No subdivision possible, keep as-is
                result.push(cluster);
            }
        } else {
            // Small enough or non-Domain cluster, keep as-is
            result.push(cluster);
        }
    }

    result
}

/// Cluster methods by verb/action patterns for secondary subdivision
fn cluster_by_verb_patterns(methods: &[String]) -> HashMap<String, Vec<String>> {
    let mut clusters: HashMap<String, Vec<String>> = HashMap::new();

    for method in methods {
        let verb = extract_verb_pattern(method);
        clusters.entry(verb).or_default().push(method.clone());
    }

    clusters
}

/// Extract verb pattern from method name for grouping
fn extract_verb_pattern(method_name: &str) -> String {
    // Common verb patterns in Rust
    let prefixes = [
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
    ];

    for prefix in &prefixes {
        if method_name.to_lowercase().starts_with(prefix) {
            return capitalize_first(prefix);
        }
    }

    // Fallback: use first word
    method_name
        .split('_')
        .next()
        .filter(|s| !s.is_empty())
        .map(capitalize_first)
        .unwrap_or_else(|| "Utilities".to_string())
}

/// Merge tiny clusters (<3 methods) into related larger clusters
///
/// Enforces minimum cluster size of 3 methods by:
/// 1. Merging tiny clusters into related normal clusters
/// 2. Combining all unmerged tiny clusters into a single Utilities cluster
/// 3. NEVER dropping methods - all methods must be accounted for
fn merge_tiny_clusters(clusters: Vec<MethodCluster>) -> Vec<MethodCluster> {
    if clusters.len() <= 1 {
        return clusters;
    }

    // Separate tiny clusters from normal ones
    let mut normal_clusters = Vec::new();
    let mut tiny_clusters = Vec::new();

    for cluster in clusters {
        if cluster.methods.len() < 3 {
            tiny_clusters.push(cluster);
        } else {
            normal_clusters.push(cluster);
        }
    }

    // Try to merge each tiny cluster into a related normal cluster
    let mut unmerged_methods = Vec::new();

    for tiny in tiny_clusters {
        let mut merged = false;

        // Find a normal cluster with same category type
        for normal in &mut normal_clusters {
            if categories_are_related(&tiny.category, &normal.category) {
                // Merge tiny into normal
                normal.methods.extend(tiny.methods.clone());
                merged = true;
                break;
            }
        }

        if !merged {
            // No related cluster found - collect methods for Utilities cluster
            unmerged_methods.extend(tiny.methods);
        }
    }

    // If we have unmerged methods, combine them intelligently
    // This ensures:
    // 1. No methods are lost
    // 2. We enforce minimum cluster size
    // 3. Small clusters don't survive
    if !unmerged_methods.is_empty() {
        // Check if we should create a Utilities cluster or merge into existing
        let utilities_exists = normal_clusters.iter_mut().find(
            |c| matches!(c.category, BehaviorCategory::Domain(ref name) if name == "Utilities"),
        );

        if let Some(utilities) = utilities_exists {
            // Merge into existing Utilities cluster
            utilities.methods.extend(unmerged_methods);
        } else if unmerged_methods.len() >= 3 {
            // Only create new Utilities cluster if we have enough methods
            normal_clusters.push(MethodCluster {
                category: BehaviorCategory::Domain("Utilities".to_string()),
                methods: unmerged_methods,
                fields_accessed: vec![],
                internal_calls: 0,
                external_calls: 0,
                cohesion_score: 0.0,
            });
        } else if let Some(largest_cluster) =
            normal_clusters.iter_mut().max_by_key(|c| c.methods.len())
        {
            // If we have <3 unmerged methods and no Utilities cluster exists,
            // merge them into the largest existing cluster to avoid creating tiny clusters
            largest_cluster.methods.extend(unmerged_methods);
        } else {
            // Edge case: no normal clusters exist, and <3 unmerged methods
            // Create cluster anyway (will be handled by ensure_all_methods_clustered)
            normal_clusters.push(MethodCluster {
                category: BehaviorCategory::Domain("Utilities".to_string()),
                methods: unmerged_methods,
                fields_accessed: vec![],
                internal_calls: 0,
                external_calls: 0,
                cohesion_score: 0.0,
            });
        }
    }

    normal_clusters
}

/// Check if two behavioral categories are related for merging purposes
fn categories_are_related(cat1: &BehaviorCategory, cat2: &BehaviorCategory) -> bool {
    use BehaviorCategory::*;

    match (cat1, cat2) {
        // Same category type
        (Lifecycle, Lifecycle) => true,
        (StateManagement, StateManagement) => true,
        (Persistence, Persistence) => true,
        (Validation, Validation) => true,
        (Rendering, Rendering) => true,
        (EventHandling, EventHandling) => true,
        (Computation, Computation) => true,
        (Parsing, Parsing) => true,
        (Filtering, Filtering) => true,
        (Transformation, Transformation) => true,
        (DataAccess, DataAccess) => true,
        (Construction, Construction) => true,
        (Processing, Processing) => true,
        (Communication, Communication) => true,

        // Domain categories with same or similar names
        (Domain(name1), Domain(name2)) => {
            name1 == name2
                || name1.to_lowercase().contains(&name2.to_lowercase())
                || name2.to_lowercase().contains(&name1.to_lowercase())
        }

        // Related categories
        (Persistence, StateManagement) | (StateManagement, Persistence) => true,
        (Validation, Computation) | (Computation, Validation) => true,
        (Parsing, DataAccess) | (DataAccess, Parsing) => true,
        (Filtering, Transformation) | (Transformation, Filtering) => true,

        _ => false,
    }
}

/// Apply Rust-specific naming patterns to improve categorization
///
/// Only refines Domain categories - preserves named behavioral categories
/// (Rendering, StateManagement, etc.) to avoid over-relabeling
fn apply_rust_patterns(clusters: Vec<MethodCluster>) -> Vec<MethodCluster> {
    clusters
        .into_iter()
        .map(|mut cluster| {
            // Only refine generic Domain categories, preserve specific behavioral categories
            if !matches!(cluster.category, BehaviorCategory::Domain(_)) {
                return cluster;
            }

            // Detect I/O boundary patterns (parser, reader, writer)
            if cluster_is_io_boundary(&cluster.methods) {
                cluster.category = BehaviorCategory::Domain("Parser".to_string());
            }
            // Detect query patterns (get_*, fetch_*, find_*)
            else if cluster_is_query(&cluster.methods) {
                cluster.category = BehaviorCategory::Domain("Query".to_string());
            }
            // Detect matching/strategy patterns
            else if cluster_is_matching(&cluster.methods) {
                cluster.category = BehaviorCategory::Domain("Matching".to_string());
            }
            // Detect lookup patterns
            else if cluster_is_lookup(&cluster.methods) {
                cluster.category = BehaviorCategory::Domain("Lookup".to_string());
            }

            cluster
        })
        .collect()
}

fn cluster_is_io_boundary(methods: &[String]) -> bool {
    let io_keywords = [
        "parse",
        "read",
        "write",
        "load",
        "save",
        "deserialize",
        "serialize",
    ];
    let io_count = methods
        .iter()
        .filter(|m| {
            let lower = m.to_lowercase();
            io_keywords.iter().any(|kw| lower.contains(kw))
        })
        .count();

    io_count as f64 / methods.len() as f64 > 0.6 // >60% I/O methods
}

fn cluster_is_query(methods: &[String]) -> bool {
    let query_keywords = ["get_", "fetch_", "retrieve_", "query_"];
    methods
        .iter()
        .filter(|m| query_keywords.iter().any(|kw| m.starts_with(kw)))
        .count() as f64
        / methods.len() as f64
        > 0.6
}

fn cluster_is_matching(methods: &[String]) -> bool {
    let match_keywords = ["match", "compare", "equals", "strategy"];
    methods
        .iter()
        .filter(|m| {
            let lower = m.to_lowercase();
            match_keywords.iter().any(|kw| lower.contains(kw))
        })
        .count() as f64
        / methods.len() as f64
        > 0.6
}

fn cluster_is_lookup(methods: &[String]) -> bool {
    let lookup_keywords = ["find_", "search_", "lookup_", "locate_"];
    methods
        .iter()
        .filter(|m| lookup_keywords.iter().any(|kw| m.starts_with(kw)))
        .count() as f64
        / methods.len() as f64
        > 0.6
}

/// Calculate modularity score for a method in a cluster
fn calculate_method_modularity(
    method: &str,
    cluster: &[String],
    adjacency: &HashMap<(String, String), usize>,
    all_methods: &[String],
) -> f64 {
    if cluster.is_empty() {
        return 0.0;
    }

    // Count connections to methods in this cluster
    let mut internal_connections = 0;
    for cluster_method in cluster {
        if cluster_method == method {
            continue;
        }

        // Check both directions
        internal_connections += adjacency
            .get(&(method.to_string(), cluster_method.clone()))
            .unwrap_or(&0);
        internal_connections += adjacency
            .get(&(cluster_method.clone(), method.to_string()))
            .unwrap_or(&0);
    }

    // Count connections to methods outside this cluster
    let mut external_connections = 0;
    for other_method in all_methods {
        if cluster.contains(other_method) || other_method == method {
            continue;
        }

        external_connections += adjacency
            .get(&(method.to_string(), other_method.clone()))
            .unwrap_or(&0);
        external_connections += adjacency
            .get(&(other_method.clone(), method.to_string()))
            .unwrap_or(&0);
    }

    let total = internal_connections + external_connections;
    if total == 0 {
        return 0.0;
    }

    internal_connections as f64 / total as f64
}

/// Calculate cohesion metrics for a cluster
fn calculate_cluster_cohesion(
    cluster: &[String],
    adjacency: &HashMap<(String, String), usize>,
    method_to_cluster: &HashMap<String, usize>,
) -> (usize, usize) {
    let cluster_id = method_to_cluster.get(&cluster[0]).copied();

    let mut internal = 0;
    let mut external = 0;

    for method in cluster {
        for (key, &count) in adjacency {
            let (from, to) = key;

            if from == method {
                let to_cluster = method_to_cluster.get(to);
                if to_cluster == cluster_id.as_ref() {
                    internal += count;
                } else {
                    external += count;
                }
            }
        }
    }

    (internal, external)
}
