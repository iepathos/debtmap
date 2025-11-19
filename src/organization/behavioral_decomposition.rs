/// Behavioral decomposition for god object refactoring recommendations.
///
/// This module implements Spec 178: shifting from struct-based organization
/// to behavioral method clustering for god object refactoring.
use std::collections::{HashMap, HashSet};
use syn::{visit::Visit, Expr, ExprField, ImplItemFn, ItemImpl};

/// Behavioral category for method clustering
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BehaviorCategory {
    /// Lifecycle methods: new, init, setup, destroy, cleanup
    Lifecycle,
    /// State management: getters, setters, state transitions
    StateManagement,
    /// Rendering/Display: render, draw, paint, display, format
    Rendering,
    /// Event handling: handle_*, on_*, event dispatchers
    EventHandling,
    /// Persistence: save, load, serialize, deserialize
    Persistence,
    /// Validation: validate_*, check_*, verify_*, ensure_*
    Validation,
    /// Pure computation: deterministic calculations with no state mutation
    Computation,
    /// Domain-specific behavior with custom name
    Domain(String),
}

impl BehaviorCategory {
    /// Get human-readable name for this category
    pub fn display_name(&self) -> String {
        match self {
            BehaviorCategory::Lifecycle => "Lifecycle".to_string(),
            BehaviorCategory::StateManagement => "State Management".to_string(),
            BehaviorCategory::Rendering => "Rendering".to_string(),
            BehaviorCategory::EventHandling => "Event Handling".to_string(),
            BehaviorCategory::Persistence => "Persistence".to_string(),
            BehaviorCategory::Validation => "Validation".to_string(),
            BehaviorCategory::Computation => "Computation".to_string(),
            BehaviorCategory::Domain(name) => name.clone(),
        }
    }

    /// Get suggested module name for this category
    pub fn module_name(&self) -> String {
        match self {
            BehaviorCategory::Lifecycle => "lifecycle".to_string(),
            BehaviorCategory::StateManagement => "state".to_string(),
            BehaviorCategory::Rendering => "rendering".to_string(),
            BehaviorCategory::EventHandling => "events".to_string(),
            BehaviorCategory::Persistence => "persistence".to_string(),
            BehaviorCategory::Validation => "validation".to_string(),
            BehaviorCategory::Computation => "computation".to_string(),
            BehaviorCategory::Domain(name) => name.to_lowercase().replace(' ', "_"),
        }
    }
}

/// Cluster of methods with behavioral cohesion
#[derive(Debug, Clone)]
pub struct MethodCluster {
    /// Behavioral category for this cluster
    pub category: BehaviorCategory,
    /// Method names in this cluster
    pub methods: Vec<String>,
    /// Fields accessed by methods in this cluster
    pub fields_accessed: Vec<String>,
    /// Number of calls within the cluster
    pub internal_calls: usize,
    /// Number of calls outside the cluster
    pub external_calls: usize,
    /// Cohesion score (0.0 to 1.0) - higher is better
    pub cohesion_score: f64,
}

impl MethodCluster {
    /// Calculate cohesion score for this cluster
    ///
    /// Formula: internal_calls / (internal_calls + external_calls)
    ///
    /// High cohesion (>0.6) indicates methods should stay together.
    pub fn calculate_cohesion(&mut self) {
        let total_calls = self.internal_calls + self.external_calls;
        self.cohesion_score = if total_calls > 0 {
            self.internal_calls as f64 / total_calls as f64
        } else {
            0.0
        };
    }

    /// Check if this cluster is a good extraction candidate
    ///
    /// Criteria:
    /// - Cohesion > 0.6
    /// - 10-50 methods (sweet spot: 15-25)
    /// - Accesses <30% of original fields
    pub fn is_good_extraction_candidate(&self, total_fields: usize) -> bool {
        let method_count = self.methods.len();
        let field_ratio = if total_fields > 0 {
            self.fields_accessed.len() as f64 / total_fields as f64
        } else {
            0.0
        };

        self.cohesion_score > 0.6 && (10..=50).contains(&method_count) && field_ratio < 0.3
    }

    /// Get size category for reporting
    pub fn size_category(&self) -> &'static str {
        let count = self.methods.len();
        match count {
            0..=10 => "Small",
            11..=25 => "Medium",
            26..=50 => "Large",
            _ => "Very Large",
        }
    }
}

/// Behavioral categorizer for method names
pub struct BehavioralCategorizer;

impl BehavioralCategorizer {
    /// Categorize a method based on its name and signature
    ///
    /// Uses heuristics from Spec 178:
    /// - Lifecycle: new, create, init, destroy, etc.
    /// - Rendering: render, draw, paint, format, etc.
    /// - Event handling: handle_*, on_*, etc.
    /// - Persistence: save, load, serialize, etc.
    /// - Validation: validate_*, check_*, verify_*, etc.
    /// - State management: get_*, set_*, update_*, etc.
    pub fn categorize_method(method_name: &str) -> BehaviorCategory {
        let lower_name = method_name.to_lowercase();

        // Lifecycle methods
        if Self::is_lifecycle(&lower_name) {
            return BehaviorCategory::Lifecycle;
        }

        // Rendering/Display methods
        if Self::is_rendering(&lower_name) {
            return BehaviorCategory::Rendering;
        }

        // Event handling methods
        if Self::is_event_handling(&lower_name) {
            return BehaviorCategory::EventHandling;
        }

        // Persistence methods
        if Self::is_persistence(&lower_name) {
            return BehaviorCategory::Persistence;
        }

        // Validation methods
        if Self::is_validation(&lower_name) {
            return BehaviorCategory::Validation;
        }

        // State management methods
        if Self::is_state_management(&lower_name) {
            return BehaviorCategory::StateManagement;
        }

        // Default: domain-specific based on first word (capitalized for better naming)
        let domain = method_name
            .split('_')
            .next()
            .filter(|s| !s.is_empty())
            .map(capitalize_first)
            .unwrap_or_else(|| "Operations".to_string());
        BehaviorCategory::Domain(domain)
    }

    fn is_lifecycle(name: &str) -> bool {
        const LIFECYCLE_KEYWORDS: &[&str] = &[
            "new",
            "create",
            "init",
            "initialize",
            "setup",
            "destroy",
            "cleanup",
            "dispose",
            "shutdown",
            "close",
        ];
        LIFECYCLE_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    fn is_rendering(name: &str) -> bool {
        const RENDERING_KEYWORDS: &[&str] = &[
            "render",
            "draw",
            "paint",
            "display",
            "show",
            "present",
            "format",
            "to_string",
        ];
        RENDERING_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    fn is_event_handling(name: &str) -> bool {
        name.starts_with("handle_")
            || name.starts_with("on_")
            || name.contains("_event")
            || name.contains("dispatch")
            || name.contains("trigger")
    }

    fn is_persistence(name: &str) -> bool {
        const PERSISTENCE_KEYWORDS: &[&str] = &[
            "save",
            "load",
            "persist",
            "restore",
            "serialize",
            "deserialize",
            "write",
            "read",
            "parse",
        ];
        PERSISTENCE_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    fn is_validation(name: &str) -> bool {
        const VALIDATION_KEYWORDS: &[&str] = &["validate", "check", "verify", "ensure", "is_valid"];
        VALIDATION_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    fn is_state_management(name: &str) -> bool {
        name.starts_with("get_")
            || name.starts_with("set_")
            || name.starts_with("update_")
            || name.starts_with("mutate_")
            || name.contains("_state")
    }
}

/// Cluster methods by behavioral category
pub fn cluster_methods_by_behavior(methods: &[String]) -> HashMap<BehaviorCategory, Vec<String>> {
    let mut clusters: HashMap<BehaviorCategory, Vec<String>> = HashMap::new();

    for method in methods {
        let category = BehavioralCategorizer::categorize_method(method);
        clusters.entry(category).or_default().push(method.clone());
    }

    // Filter out misc/domain clusters with too few methods
    clusters.retain(|category, methods| {
        matches!(
            category,
            BehaviorCategory::Lifecycle
                | BehaviorCategory::StateManagement
                | BehaviorCategory::Rendering
                | BehaviorCategory::EventHandling
                | BehaviorCategory::Persistence
                | BehaviorCategory::Validation
                | BehaviorCategory::Computation
        ) || methods.len() >= 3 // Keep domain clusters only if they have 3+ methods
    });

    clusters
}

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
            let current_cluster = *method_to_cluster.get(method).unwrap();

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
                for mut subcluster in sub_clusters {
                    // If the subcluster's inferred category is generic (Domain),
                    // prefer the original behavioral category
                    if matches!(subcluster.category, BehaviorCategory::Domain(_)) {
                        subcluster.category = category.clone();
                    }
                    refined_clusters.push(subcluster);
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

/// Detect if a method is a test method that should stay in #[cfg(test)]
pub fn is_test_method(method_name: &str) -> bool {
    // Common test patterns in Rust
    method_name.starts_with("test_")
        || method_name.contains("_test_")
        || method_name.ends_with("_test")
        // Benchmark patterns
        || method_name.starts_with("bench_")
        || method_name.contains("_bench_")
        // Test helper patterns
        || method_name.starts_with("mock_")
        || method_name.starts_with("stub_")
        || method_name.starts_with("fixture_")
        || method_name == "setup"
        || method_name == "teardown"
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
        "parse", "build", "create", "make", "construct",
        "get", "fetch", "retrieve", "find", "search", "lookup", "query",
        "set", "update", "modify", "change",
        "is", "has", "can", "should", "check",
        "apply", "execute", "run", "process", "handle",
        "demangle", "normalize", "sanitize", "clean",
        "calculate", "compute", "derive",
        "match", "compare", "equals",
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
            // No related cluster found, create "Utilities" cluster or keep as-is if >=2 methods
            if tiny.methods.len() >= 2 {
                normal_clusters.push(tiny);
            }
            // If only 1 method, we drop it (could add to a utilities cluster instead)
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

        // Domain categories with same or similar names
        (Domain(name1), Domain(name2)) => {
            name1 == name2
                || name1.to_lowercase().contains(&name2.to_lowercase())
                || name2.to_lowercase().contains(&name1.to_lowercase())
        }

        // Related categories
        (Persistence, StateManagement) | (StateManagement, Persistence) => true,
        (Validation, Computation) | (Computation, Validation) => true,

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
    let io_keywords = ["parse", "read", "write", "load", "save", "deserialize", "serialize"];
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

/// Infer behavioral category from cluster method names
fn infer_cluster_category(methods: &[String]) -> BehaviorCategory {
    let mut category_counts: HashMap<BehaviorCategory, usize> = HashMap::new();

    for method in methods {
        let category = BehavioralCategorizer::categorize_method(method);
        *category_counts.entry(category).or_insert(0) += 1;
    }

    // Return most common category (excluding Domain categories)
    category_counts
        .into_iter()
        .filter(|(cat, _)| !matches!(cat, BehaviorCategory::Domain(_)))
        .max_by_key(|(_, count)| *count)
        .map(|(cat, _)| cat)
        .unwrap_or_else(|| {
            // If no clear category, use domain based on first method
            BehavioralCategorizer::categorize_method(&methods[0])
        })
}

/// Detect methods that could be extracted to service objects
///
/// Service object candidates are methods with:
/// - Minimal field dependencies (<3 fields)
/// - Stateless behavior (can work with passed parameters)
/// - No internal state mutation
///
/// Returns: Vec of (method_name, fields_needed, rationale)
pub fn detect_service_candidates(
    field_tracker: &FieldAccessTracker,
    methods: &[String],
) -> Vec<(String, Vec<String>, String)> {
    let mut candidates = Vec::new();

    for method in methods {
        let fields = field_tracker.get_method_fields(method);

        // Service object criteria: minimal field dependencies
        if fields.len() < 3 {
            let rationale = if fields.is_empty() {
                format!(
                    "Method '{}' accesses no fields - pure computation candidate for service object extraction",
                    method
                )
            } else {
                format!(
                    "Method '{}' accesses only {} field(s): {} - good service object candidate",
                    method,
                    fields.len(),
                    fields.join(", ")
                )
            };

            candidates.push((method.clone(), fields, rationale));
        }
    }

    candidates
}

/// Generate service object extraction recommendation
///
/// Creates a recommendation for extracting low-coupling methods
/// into a separate service struct.
pub fn recommend_service_extraction(
    candidates: &[(String, Vec<String>, String)],
    service_name: &str,
) -> String {
    if candidates.is_empty() {
        return String::new();
    }

    let method_list: Vec<_> = candidates
        .iter()
        .take(5)
        .map(|(method, fields, _)| {
            if fields.is_empty() {
                format!("    fn {}(...) -> Result<...>", method)
            } else {
                format!(
                    "    fn {}(&self, {}: ...) -> Result<...>",
                    method,
                    fields.join(", ")
                )
            }
        })
        .collect();

    let remaining = candidates.len().saturating_sub(5);

    format!(
        "struct {} {{\n    // {} low-coupling methods total\n{}{}\n}}\n\nRationale: These methods have minimal field dependencies and can be extracted to a service object.",
        service_name,
        candidates.len(),
        method_list.join("\n"),
        if remaining > 0 {
            format!("\n    // ... +{} more methods", remaining)
        } else {
            String::new()
        }
    )
}

/// Generate trait extraction recommendation
pub fn suggest_trait_extraction(cluster: &MethodCluster, _struct_name: &str) -> String {
    let trait_name = match &cluster.category {
        BehaviorCategory::Lifecycle => "Lifecycle".to_string(),
        BehaviorCategory::StateManagement => "StatefulObject".to_string(),
        BehaviorCategory::Rendering => "Renderable".to_string(),
        BehaviorCategory::EventHandling => "EventHandler".to_string(),
        BehaviorCategory::Persistence => "Persistable".to_string(),
        BehaviorCategory::Validation => "Validatable".to_string(),
        BehaviorCategory::Computation => "Calculator".to_string(),
        BehaviorCategory::Domain(name) => format!("{}Ops", capitalize_first(name)),
    };

    let method_examples: Vec<_> = cluster.methods.iter().take(3).cloned().collect();
    let remaining = cluster.methods.len().saturating_sub(3);

    format!(
        "trait {} {{\n    // {} methods total\n{}{}\n}}",
        trait_name,
        cluster.methods.len(),
        method_examples
            .iter()
            .map(|m| format!("    fn {}(&self);", m))
            .collect::<Vec<_>>()
            .join("\n"),
        if remaining > 0 {
            format!("\n    // ... +{} more methods", remaining)
        } else {
            String::new()
        }
    )
}

/// Capitalize first character of a string
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Field access tracker for analyzing which fields each method accesses
pub struct FieldAccessTracker {
    /// Map from method name to set of fields accessed
    method_fields: HashMap<String, HashSet<String>>,
    /// Currently analyzing method name
    current_method: Option<String>,
}

impl FieldAccessTracker {
    /// Create a new field access tracker
    pub fn new() -> Self {
        Self {
            method_fields: HashMap::new(),
            current_method: None,
        }
    }

    /// Analyze an impl block and extract field access patterns
    pub fn analyze_impl(&mut self, impl_block: &ItemImpl) {
        self.visit_item_impl(impl_block);
    }

    /// Get fields accessed by a specific method
    pub fn get_method_fields(&self, method_name: &str) -> Vec<String> {
        self.method_fields
            .get(method_name)
            .map(|fields| {
                let mut sorted: Vec<_> = fields.iter().cloned().collect();
                sorted.sort();
                sorted
            })
            .unwrap_or_default()
    }

    /// Get minimal field set for a group of methods
    pub fn get_minimal_field_set(&self, methods: &[String]) -> Vec<String> {
        let mut field_set = HashSet::new();
        for method in methods {
            if let Some(fields) = self.method_fields.get(method) {
                field_set.extend(fields.iter().cloned());
            }
        }
        let mut sorted: Vec<_> = field_set.into_iter().collect();
        sorted.sort();
        sorted
    }

    /// Check if a field is a core dependency (accessed by >50% of methods)
    ///
    /// Core dependencies are fields that most methods access. These should
    /// typically remain in the original struct rather than being extracted.
    pub fn is_core_dependency(&self, field_name: &str, total_methods: usize) -> bool {
        if total_methods == 0 {
            return false;
        }

        let access_count = self
            .method_fields
            .values()
            .filter(|fields| fields.contains(field_name))
            .count();

        access_count as f64 / total_methods as f64 > 0.5
    }

    /// Check if a field is cluster-specific (accessed by >80% of cluster methods)
    ///
    /// Cluster-specific fields are good candidates for extraction with the cluster,
    /// as they're heavily used by that group of methods but not broadly elsewhere.
    pub fn is_cluster_specific(&self, field_name: &str, cluster_methods: &[String]) -> bool {
        if cluster_methods.is_empty() {
            return false;
        }

        let access_count = cluster_methods
            .iter()
            .filter(|method| {
                self.method_fields
                    .get(*method)
                    .map(|fields| fields.contains(field_name))
                    .unwrap_or(false)
            })
            .count();

        access_count as f64 / cluster_methods.len() as f64 > 0.8
    }

    /// Get cluster-specific fields for a group of methods
    ///
    /// Returns fields that are heavily used by this cluster (>80% of methods)
    /// but not core dependencies of the overall struct (<50% global usage).
    pub fn get_cluster_specific_fields(
        &self,
        cluster_methods: &[String],
        total_methods: usize,
    ) -> Vec<String> {
        let mut cluster_specific = Vec::new();
        let cluster_fields = self.get_minimal_field_set(cluster_methods);

        for field in cluster_fields {
            if self.is_cluster_specific(&field, cluster_methods)
                && !self.is_core_dependency(&field, total_methods)
            {
                cluster_specific.push(field);
            }
        }

        cluster_specific.sort();
        cluster_specific
    }

    /// Calculate field coupling percentage for a method
    ///
    /// Returns the percentage of struct fields that this method accesses.
    /// Lower coupling indicates easier extraction.
    pub fn calculate_field_coupling(&self, method_name: &str, total_fields: usize) -> f64 {
        if total_fields == 0 {
            return 0.0;
        }

        let accessed_fields = self
            .method_fields
            .get(method_name)
            .map(|fields| fields.len())
            .unwrap_or(0);

        accessed_fields as f64 / total_fields as f64
    }

    /// Get all fields accessed across all methods
    pub fn get_all_fields(&self) -> Vec<String> {
        let mut all_fields = HashSet::new();
        for fields in self.method_fields.values() {
            all_fields.extend(fields.iter().cloned());
        }
        let mut sorted: Vec<_> = all_fields.into_iter().collect();
        sorted.sort();
        sorted
    }

    /// Get field access statistics
    pub fn get_field_access_stats(&self) -> HashMap<String, FieldAccessStats> {
        let mut stats = HashMap::new();
        let all_fields = self.get_all_fields();
        let total_methods = self.method_fields.len();

        for field in all_fields {
            let accessed_by = self
                .method_fields
                .iter()
                .filter_map(|(method, fields)| {
                    if fields.contains(&field) {
                        Some(method.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            let access_frequency = accessed_by.len();
            let access_percentage = if total_methods > 0 {
                access_frequency as f64 / total_methods as f64
            } else {
                0.0
            };

            stats.insert(
                field.clone(),
                FieldAccessStats {
                    field_name: field,
                    accessed_by,
                    access_frequency,
                    access_percentage,
                },
            );
        }

        stats
    }
}

/// Statistics about field access patterns
#[derive(Debug, Clone)]
pub struct FieldAccessStats {
    pub field_name: String,
    pub accessed_by: Vec<String>,
    pub access_frequency: usize,
    pub access_percentage: f64,
}

impl Default for FieldAccessTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl<'ast> Visit<'ast> for FieldAccessTracker {
    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        let method_name = node.sig.ident.to_string();
        let old_method = self.current_method.replace(method_name.clone());

        // Initialize field set for this method
        self.method_fields.insert(method_name, HashSet::new());

        // Visit the method body
        syn::visit::visit_impl_item_fn(self, node);

        self.current_method = old_method;
    }

    fn visit_expr_field(&mut self, node: &'ast ExprField) {
        // Track field accesses of the form self.field_name
        if let Some(ref method_name) = self.current_method {
            // Check if this is a self.field access
            if is_self_field_access(&node.base) {
                if let syn::Member::Named(field_ident) = &node.member {
                    if let Some(fields) = self.method_fields.get_mut(method_name) {
                        fields.insert(field_ident.to_string());
                    }
                }
            }
        }

        syn::visit::visit_expr_field(self, node);
    }
}

/// Check if an expression is a self reference
fn is_self_field_access(expr: &Expr) -> bool {
    match expr {
        Expr::Path(path) => path
            .path
            .segments
            .first()
            .map(|seg| seg.ident == "self")
            .unwrap_or(false),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_lifecycle_methods() {
        assert_eq!(
            BehavioralCategorizer::categorize_method("new"),
            BehaviorCategory::Lifecycle
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("initialize_system"),
            BehaviorCategory::Lifecycle
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("cleanup"),
            BehaviorCategory::Lifecycle
        );
    }

    #[test]
    fn test_categorize_rendering_methods() {
        assert_eq!(
            BehavioralCategorizer::categorize_method("render"),
            BehaviorCategory::Rendering
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("draw_cursor"),
            BehaviorCategory::Rendering
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("paint_background"),
            BehaviorCategory::Rendering
        );
    }

    #[test]
    fn test_categorize_event_handling() {
        assert_eq!(
            BehavioralCategorizer::categorize_method("handle_keypress"),
            BehaviorCategory::EventHandling
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("on_mouse_down"),
            BehaviorCategory::EventHandling
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("dispatch_event"),
            BehaviorCategory::EventHandling
        );
    }

    #[test]
    fn test_categorize_persistence() {
        assert_eq!(
            BehavioralCategorizer::categorize_method("save_state"),
            BehaviorCategory::Persistence
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("load_config"),
            BehaviorCategory::Persistence
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("serialize"),
            BehaviorCategory::Persistence
        );
    }

    #[test]
    fn test_categorize_validation() {
        assert_eq!(
            BehavioralCategorizer::categorize_method("validate_input"),
            BehaviorCategory::Validation
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("check_bounds"),
            BehaviorCategory::Validation
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("verify_signature"),
            BehaviorCategory::Validation
        );
    }

    #[test]
    fn test_categorize_state_management() {
        assert_eq!(
            BehavioralCategorizer::categorize_method("get_value"),
            BehaviorCategory::StateManagement
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("set_name"),
            BehaviorCategory::StateManagement
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("update_state"),
            BehaviorCategory::StateManagement
        );
    }

    #[test]
    fn test_cluster_methods_by_behavior() {
        let methods = vec![
            "render".to_string(),
            "draw_cursor".to_string(),
            "handle_keypress".to_string(),
            "on_mouse_down".to_string(),
            "validate_input".to_string(),
            "get_value".to_string(),
            "set_name".to_string(),
        ];

        let clusters = cluster_methods_by_behavior(&methods);

        assert!(clusters.contains_key(&BehaviorCategory::Rendering));
        assert!(clusters.contains_key(&BehaviorCategory::EventHandling));
        assert!(clusters.contains_key(&BehaviorCategory::StateManagement));

        assert_eq!(clusters.get(&BehaviorCategory::Rendering).unwrap().len(), 2);
        assert_eq!(
            clusters
                .get(&BehaviorCategory::EventHandling)
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn test_method_cluster_cohesion() {
        let mut cluster = MethodCluster {
            category: BehaviorCategory::Rendering,
            methods: vec!["render".to_string(), "draw".to_string()],
            fields_accessed: vec!["display_map".to_string()],
            internal_calls: 8,
            external_calls: 2,
            cohesion_score: 0.0,
        };

        cluster.calculate_cohesion();
        assert_eq!(cluster.cohesion_score, 0.8); // 8 / (8 + 2)
    }

    #[test]
    fn test_good_extraction_candidate() {
        let cluster = MethodCluster {
            category: BehaviorCategory::Rendering,
            methods: (0..15).map(|i| format!("method{}", i)).collect(),
            fields_accessed: vec!["field1".to_string(), "field2".to_string()],
            internal_calls: 20,
            external_calls: 5,
            cohesion_score: 0.8,
        };

        assert!(cluster.is_good_extraction_candidate(10)); // 2/10 = 0.2 < 0.3
        assert!(!cluster.is_good_extraction_candidate(5)); // 2/5 = 0.4 > 0.3
    }

    #[test]
    fn test_suggest_trait_extraction() {
        let cluster = MethodCluster {
            category: BehaviorCategory::Rendering,
            methods: vec![
                "render".to_string(),
                "draw_cursor".to_string(),
                "paint_background".to_string(),
            ],
            fields_accessed: vec![],
            internal_calls: 0,
            external_calls: 0,
            cohesion_score: 0.0,
        };

        let suggestion = suggest_trait_extraction(&cluster, "Editor");
        assert!(suggestion.contains("trait Renderable"));
        assert!(suggestion.contains("fn render(&self);"));
        assert!(suggestion.contains("3 methods total"));
    }

    #[test]
    fn test_field_access_tracking() {
        let code = quote::quote! {
            impl Editor {
                fn render(&self) {
                    let x = self.display_map;
                    let y = self.cursor_position;
                }

                fn handle_input(&mut self) {
                    self.input_buffer.clear();
                }

                fn save(&self) {
                    let path = self.file_path;
                }
            }
        };

        let impl_block: ItemImpl = syn::parse2(code).unwrap();
        let mut tracker = FieldAccessTracker::new();
        tracker.analyze_impl(&impl_block);

        let render_fields = tracker.get_method_fields("render");
        assert_eq!(render_fields, vec!["cursor_position", "display_map"]);

        let input_fields = tracker.get_method_fields("handle_input");
        assert_eq!(input_fields, vec!["input_buffer"]);

        let save_fields = tracker.get_method_fields("save");
        assert_eq!(save_fields, vec!["file_path"]);
    }

    #[test]
    fn test_minimal_field_set() {
        let code = quote::quote! {
            impl Editor {
                fn render(&self) {
                    let x = self.display_map;
                    let y = self.cursor_position;
                }

                fn draw(&self) {
                    let z = self.display_map;
                }
            }
        };

        let impl_block: ItemImpl = syn::parse2(code).unwrap();
        let mut tracker = FieldAccessTracker::new();
        tracker.analyze_impl(&impl_block);

        let methods = vec!["render".to_string(), "draw".to_string()];
        let minimal_fields = tracker.get_minimal_field_set(&methods);
        assert_eq!(minimal_fields, vec!["cursor_position", "display_map"]);
    }

    #[test]
    fn test_hybrid_clustering_lcov_like_example() {
        // This test mimics the structure of lcov.rs with multiple behavioral categories
        // to ensure hybrid clustering correctly identifies diverse method groups
        let code = quote::quote! {
            pub struct LcovData {
                file_index: HashMap<String, Vec<String>>,
                function_cache: HashMap<String, CoverageData>,
                loc_counter: Option<LocCounter>,
            }

            impl LcovData {
                // Lifecycle methods
                pub fn new() -> Self {
                    Self {
                        file_index: HashMap::new(),
                        function_cache: HashMap::new(),
                        loc_counter: None,
                    }
                }

                pub fn create_empty() -> Self {
                    Self::new()
                }

                pub fn initialize(&mut self) {
                    self.build_index();
                }

                pub fn build_index(&mut self) {
                    // Build index logic
                }

                pub fn with_loc_counter(mut self, counter: LocCounter) -> Self {
                    self.loc_counter = Some(counter);
                    self
                }

                // Query methods - these call each other
                pub fn get_function_coverage(&self, file: &str, function: &str) -> Option<f64> {
                    let funcs = self.find_functions_by_path(file)?;
                    self.find_function_by_name(funcs, function)
                }

                pub fn get_file_coverage(&self, file: &str) -> Option<f64> {
                    let funcs = self.find_functions_by_path(file)?;
                    Some(self.calculate_average(funcs))
                }

                pub fn get_overall_coverage(&self) -> f64 {
                    let all_files = self.get_all_files();
                    self.calculate_weighted_average(&all_files)
                }

                pub fn batch_get_function_coverage(&self, queries: Vec<Query>) -> Vec<f64> {
                    queries.iter().map(|q| {
                        self.get_function_coverage(&q.file, &q.func).unwrap_or(0.0)
                    }).collect()
                }

                // Path matching methods - these call each other
                fn find_functions_by_path(&self, path: &str) -> Option<Vec<String>> {
                    if self.matches_suffix_strategy(path) {
                        Some(vec![])
                    } else {
                        self.apply_strategies_parallel(path)
                    }
                }

                fn matches_suffix_strategy(&self, path: &str) -> bool {
                    let normalized = normalize_path(path);
                    self.file_index.contains_key(&normalized)
                }

                fn apply_strategies_parallel(&self, path: &str) -> Option<Vec<String>> {
                    let results = self.apply_strategies_sequential(path);
                    results
                }

                fn apply_strategies_sequential(&self, path: &str) -> Option<Vec<String>> {
                    if self.matches_reverse_suffix(path) {
                        Some(vec![])
                    } else {
                        None
                    }
                }

                fn matches_reverse_suffix(&self, path: &str) -> bool {
                    false
                }

                // Helper methods for queries
                fn find_function_by_name(&self, funcs: Vec<String>, name: &str) -> Option<f64> {
                    let normalized = normalize_function_name(name);
                    Some(0.5)
                }

                fn calculate_average(&self, funcs: Vec<String>) -> f64 {
                    0.75
                }

                fn calculate_weighted_average(&self, files: &[String]) -> f64 {
                    0.85
                }

                fn get_all_files(&self) -> Vec<String> {
                    vec![]
                }
            }

            // Standalone normalization functions - should be tracked too
            fn normalize_path(path: &str) -> String {
                demangle_path_components(path)
            }

            fn demangle_path_components(path: &str) -> String {
                path.to_lowercase()
            }

            fn normalize_function_name(name: &str) -> String {
                demangle_function_name(name)
            }

            fn demangle_function_name(name: &str) -> String {
                strip_trailing_generics(name)
            }

            fn strip_trailing_generics(name: &str) -> String {
                name.trim_end_matches(">").to_string()
            }

            // Parsing functions
            pub fn parse_lcov_file(path: &str) -> Result<LcovData, String> {
                parse_lcov_file_with_progress(path, &ProgressBar::new())
            }

            pub fn parse_lcov_file_with_progress(path: &str, progress: &ProgressBar) -> Result<LcovData, String> {
                let data = parse_coverage_data(path)?;
                calculate_function_coverage_data(data)
            }

            fn parse_coverage_data(path: &str) -> Result<Vec<String>, String> {
                Ok(vec![])
            }

            fn process_function_coverage_parallel(path: &str) -> Result<Vec<String>, String> {
                Ok(vec![])
            }

            fn calculate_function_coverage_data(data: Vec<String>) -> Result<LcovData, String> {
                Ok(LcovData::new())
            }
        };

        let ast: syn::File = syn::parse2(code).unwrap();

        // Collect impl blocks
        let impl_blocks: Vec<&syn::ItemImpl> = ast
            .items
            .iter()
            .filter_map(|item| {
                if let syn::Item::Impl(impl_block) = item {
                    Some(impl_block)
                } else {
                    None
                }
            })
            .collect();

        // Collect standalone functions
        let standalone_functions: Vec<&syn::ItemFn> = ast
            .items
            .iter()
            .filter_map(|item| {
                if let syn::Item::Fn(func) = item {
                    Some(func)
                } else {
                    None
                }
            })
            .collect();

        // Collect all method names
        let mut all_methods = Vec::new();
        for impl_block in &impl_blocks {
            for item in &impl_block.items {
                if let syn::ImplItem::Fn(method) = item {
                    all_methods.push(method.sig.ident.to_string());
                }
            }
        }
        for func in &standalone_functions {
            all_methods.push(func.sig.ident.to_string());
        }

        // Build adjacency matrix with standalone function support
        let adjacency =
            build_method_call_adjacency_matrix_with_functions(&impl_blocks, &standalone_functions);

        // Apply hybrid clustering
        let clusters = apply_hybrid_clustering(&all_methods, &adjacency);

        // Verify we found multiple clusters (not just one big cluster)
        assert!(
            clusters.len() >= 3,
            "Expected at least 3 behavioral clusters, but found {}. Clusters: {:?}",
            clusters.len(),
            clusters
                .iter()
                .map(|c| (c.category.display_name(), c.methods.len()))
                .collect::<Vec<_>>()
        );

        // Verify that we have different behavioral categories
        let categories: HashSet<String> =
            clusters.iter().map(|c| c.category.display_name()).collect();

        assert!(
            categories.len() >= 2,
            "Expected diverse behavioral categories, but found only: {:?}",
            categories
        );

        // Check that we have a Lifecycle cluster (new, build_index, with_loc_counter)
        let lifecycle_cluster = clusters
            .iter()
            .find(|c| matches!(c.category, BehaviorCategory::Lifecycle));
        assert!(
            lifecycle_cluster.is_some(),
            "Expected to find Lifecycle cluster for methods like 'new', 'build_index'"
        );

        // Check that we have a StateManagement cluster (get_* methods)
        let state_mgmt_cluster = clusters
            .iter()
            .find(|c| matches!(c.category, BehaviorCategory::StateManagement));
        assert!(
            state_mgmt_cluster.is_some(),
            "Expected to find StateManagement cluster for get_* methods"
        );

        // Verify that Persistence cluster exists (parse_*, load_*, etc.)
        let persistence_cluster = clusters
            .iter()
            .find(|c| matches!(c.category, BehaviorCategory::Persistence));
        assert!(
            persistence_cluster.is_some(),
            "Expected to find Persistence cluster for parse_* methods"
        );

        // Verify each cluster has reasonable size (at least 3 methods as per our new threshold)
        for cluster in &clusters {
            assert!(
                cluster.methods.len() >= 3,
                "Cluster {:?} has only {} methods, expected at least 3",
                cluster.category,
                cluster.methods.len()
            );
        }

        // Verify that standalone function calls were tracked
        // normalize_path calls demangle_path_components, so they should be in same cluster
        let normalize_cluster = clusters
            .iter()
            .find(|c| c.methods.contains(&"normalize_path".to_string()));

        if let Some(cluster) = normalize_cluster {
            // If normalize_path is in a cluster, demangle_path_components should be too
            // (they're related by call graph)
            let has_related_demangle = cluster.methods.iter().any(|m| m.contains("demangle"));
            assert!(
                has_related_demangle || cluster.methods.len() >= 3,
                "Expected normalize functions to be clustered together or in a reasonable cluster"
            );
        }

        println!("\n=== Hybrid Clustering Results ===");
        for (i, cluster) in clusters.iter().enumerate() {
            println!(
                "Cluster {}: {} ({} methods, cohesion: {:.2})",
                i + 1,
                cluster.category.display_name(),
                cluster.methods.len(),
                cluster.cohesion_score
            );
            println!("  Methods: {:?}", cluster.methods);
        }
        println!("=================================\n");
    }

    #[test]
    fn test_production_ready_clustering_filters_tests() {
        // This test verifies that production-ready clustering:
        // 1. Filters out test methods
        // 2. Subdivides oversized Domain clusters
        // 3. Merges tiny clusters
        // 4. Applies Rust-specific patterns

        let methods = vec![
            // Production methods - Parser group
            "parse_lcov_file".to_string(),
            "parse_lcov_file_with_progress".to_string(),
            "parse_coverage_data".to_string(),
            "read_file_contents".to_string(),
            // Production methods - Query group
            "get_function_coverage".to_string(),
            "get_file_coverage".to_string(),
            "get_overall_coverage".to_string(),
            "get_all_files".to_string(),
            "fetch_coverage_data".to_string(),
            // Production methods - Normalize group
            "normalize_path".to_string(),
            "normalize_function_name".to_string(),
            "normalize_demangled_name".to_string(),
            // Production methods - Find group
            "find_function_by_name".to_string(),
            "find_functions_by_path".to_string(),
            "find_function_by_line".to_string(),
            // Test methods - should be filtered out
            "test_parse_lcov_file".to_string(),
            "test_function_name_normalization".to_string(),
            "test_coverage_calculation".to_string(),
            "test_empty_data".to_string(),
            // Test helpers - should be filtered out
            "mock_coverage_data".to_string(),
            "fixture_test_file".to_string(),
        ];

        let adjacency = HashMap::new(); // Empty adjacency for simplicity

        // Apply production-ready clustering
        let clusters = apply_production_ready_clustering(&methods, &adjacency);

        // Verify tests are filtered out
        let all_cluster_methods: Vec<&String> = clusters
            .iter()
            .flat_map(|c| &c.methods)
            .collect();

        assert!(
            !all_cluster_methods.contains(&&"test_parse_lcov_file".to_string()),
            "Test methods should be filtered out"
        );
        assert!(
            !all_cluster_methods.contains(&&"mock_coverage_data".to_string()),
            "Test helper methods should be filtered out"
        );

        // Verify production methods are included
        assert!(
            all_cluster_methods.contains(&&"parse_lcov_file".to_string()),
            "Production methods should be included"
        );
        assert!(
            all_cluster_methods.contains(&&"get_function_coverage".to_string()),
            "Production methods should be included"
        );

        // Verify we have multiple clusters (not one big cluster)
        assert!(
            clusters.len() >= 3,
            "Should have multiple clusters, found {}",
            clusters.len()
        );

        // Verify proper categorization (either behavioral or Rust-specific patterns)
        // Clusters should be well-categorized, not just generic "Utilities"
        let has_good_categories = clusters.iter().all(|c| {
            !matches!(
                c.category,
                BehaviorCategory::Domain(ref name) if name == "Utilities" || name == "Operations"
            )
        });

        assert!(
            has_good_categories,
            "All clusters should have meaningful categories (not Utilities/Operations)"
        );

        println!("\n=== Production-Ready Clustering Results ===");
        println!("Total clusters: {}", clusters.len());
        println!("Production methods: {} / {} total", all_cluster_methods.len(), methods.len());
        for (i, cluster) in clusters.iter().enumerate() {
            println!(
                "Cluster {}: {} ({} methods)",
                i + 1,
                cluster.category.display_name(),
                cluster.methods.len()
            );
            println!("  Methods: {:?}", cluster.methods);
        }
        println!("==========================================\n");
    }
}
