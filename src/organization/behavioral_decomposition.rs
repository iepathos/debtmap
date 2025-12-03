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
    /// Parsing: parse, read, extract, decode, deserialize, unmarshal, scan
    Parsing,
    /// Filtering: filter, select, find, search, query, lookup, match
    Filtering,
    /// Transformation: transform, convert, map, apply, adapt
    Transformation,
    /// Data access: get, set, fetch, retrieve, access
    DataAccess,
    /// Construction: create, build, new, make, construct
    Construction,
    /// Processing: process, handle, execute, run
    Processing,
    /// Communication: send, receive, transmit, broadcast, notify
    Communication,
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
            BehaviorCategory::Parsing => "Parsing".to_string(),
            BehaviorCategory::Filtering => "Filtering".to_string(),
            BehaviorCategory::Transformation => "Transformation".to_string(),
            BehaviorCategory::DataAccess => "Data Access".to_string(),
            BehaviorCategory::Construction => "Construction".to_string(),
            BehaviorCategory::Processing => "Processing".to_string(),
            BehaviorCategory::Communication => "Communication".to_string(),
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
            BehaviorCategory::Parsing => "parsing".to_string(),
            BehaviorCategory::Filtering => "filtering".to_string(),
            BehaviorCategory::Transformation => "transformation".to_string(),
            BehaviorCategory::DataAccess => "data_access".to_string(),
            BehaviorCategory::Construction => "construction".to_string(),
            BehaviorCategory::Processing => "processing".to_string(),
            BehaviorCategory::Communication => "communication".to_string(),
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
    /// Uses heuristics from Spec 208 (unified classification system):
    /// - Construction: create, build, new, make, construct (checked first before lifecycle)
    /// - Lifecycle: new, create, init, destroy, etc.
    /// - Parsing: parse, read, extract, decode, etc.
    /// - Rendering: render, draw, paint, format, etc.
    /// - Event handling: handle_*, on_*, etc.
    /// - Persistence: save, load, serialize, etc.
    /// - Validation: validate_*, check_*, verify_*, etc.
    /// - Computation: calculate, compute, evaluate, etc.
    /// - Filtering: filter, select, find, search, etc.
    /// - Transformation: transform, convert, map, apply, etc.
    /// - State management: get_*, set_*, update_*, etc. (checked before DataAccess for specificity)
    /// - Data access: get, set, fetch, retrieve, access
    /// - Processing: process, handle, execute, run
    /// - Communication: send, receive, transmit, broadcast, notify
    pub fn categorize_method(method_name: &str) -> BehaviorCategory {
        let lower_name = method_name.to_lowercase();

        // Order matters: check more specific categories first

        // Construction (before lifecycle to catch "create_*")
        if Self::is_construction(&lower_name) {
            return BehaviorCategory::Construction;
        }

        // Lifecycle methods
        if Self::is_lifecycle(&lower_name) {
            return BehaviorCategory::Lifecycle;
        }

        // Validation methods (before rendering to prioritize verify_* over *_format)
        if Self::is_validation(&lower_name) {
            return BehaviorCategory::Validation;
        }

        // Parsing (check early as it's common)
        if Self::is_parsing(&lower_name) {
            return BehaviorCategory::Parsing;
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

        // Computation methods
        if Self::is_computation(&lower_name) {
            return BehaviorCategory::Computation;
        }

        // Filtering methods
        if Self::is_filtering(&lower_name) {
            return BehaviorCategory::Filtering;
        }

        // Transformation methods
        if Self::is_transformation(&lower_name) {
            return BehaviorCategory::Transformation;
        }

        // State management methods (check before DataAccess as it's more specific)
        if Self::is_state_management(&lower_name) {
            return BehaviorCategory::StateManagement;
        }

        // Data access methods
        if Self::is_data_access(&lower_name) {
            return BehaviorCategory::DataAccess;
        }

        // Processing methods
        if Self::is_processing(&lower_name) {
            return BehaviorCategory::Processing;
        }

        // Communication methods
        if Self::is_communication(&lower_name) {
            return BehaviorCategory::Communication;
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
            "print", // Per spec 208: print_* methods are rendering/output
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
            "store", // Per spec 208: store_* methods are persistence operations
        ];
        PERSISTENCE_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    fn is_validation(name: &str) -> bool {
        // Per spec 208: "is_*" predicates are validation methods (e.g., is_valid, is_empty)
        const VALIDATION_KEYWORDS: &[&str] = &["validate", "check", "verify", "ensure", "is_"];
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

    fn is_computation(name: &str) -> bool {
        const COMPUTATION_KEYWORDS: &[&str] = &["calculate", "compute", "evaluate", "measure"];
        COMPUTATION_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    fn is_parsing(name: &str) -> bool {
        const PARSING_KEYWORDS: &[&str] = &[
            "parse",
            "read",
            "extract",
            "decode",
            "deserialize",
            "unmarshal",
            "scan",
        ];
        PARSING_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    fn is_filtering(name: &str) -> bool {
        const FILTERING_KEYWORDS: &[&str] = &[
            "filter", "select", "find", "search", "query", "lookup", "match",
        ];
        FILTERING_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    fn is_transformation(name: &str) -> bool {
        const TRANSFORMATION_KEYWORDS: &[&str] = &["transform", "convert", "map", "apply", "adapt"];
        TRANSFORMATION_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    fn is_data_access(name: &str) -> bool {
        const DATA_ACCESS_KEYWORDS: &[&str] = &["get", "set", "fetch", "retrieve", "access"];
        DATA_ACCESS_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    fn is_construction(name: &str) -> bool {
        const CONSTRUCTION_KEYWORDS: &[&str] = &["create", "build", "new", "make", "construct"];
        CONSTRUCTION_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    fn is_processing(name: &str) -> bool {
        const PROCESSING_KEYWORDS: &[&str] = &["process", "handle", "execute", "run"];
        PROCESSING_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    fn is_communication(name: &str) -> bool {
        const COMMUNICATION_KEYWORDS: &[&str] =
            &["send", "receive", "transmit", "broadcast", "notify"];
        COMMUNICATION_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
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
                | BehaviorCategory::Parsing
                | BehaviorCategory::Filtering
                | BehaviorCategory::Transformation
                | BehaviorCategory::DataAccess
                | BehaviorCategory::Construction
                | BehaviorCategory::Processing
                | BehaviorCategory::Communication
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
        } else if !normal_clusters.is_empty() {
            // If we have <3 unmerged methods and no Utilities cluster exists,
            // merge them into the largest existing cluster to avoid creating tiny clusters
            let largest_cluster = normal_clusters
                .iter_mut()
                .max_by_key(|c| c.methods.len())
                .unwrap();

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
        BehaviorCategory::Parsing => "Parser".to_string(),
        BehaviorCategory::Filtering => "Filterable".to_string(),
        BehaviorCategory::Transformation => "Transformer".to_string(),
        BehaviorCategory::DataAccess => "DataAccessor".to_string(),
        BehaviorCategory::Construction => "Constructor".to_string(),
        BehaviorCategory::Processing => "Processor".to_string(),
        BehaviorCategory::Communication => "Communicator".to_string(),
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

    /// Get fields accessed by a method as a HashSet (for clustering integration)
    pub fn fields_for_method(&self, method: &str) -> Option<HashSet<String>> {
        self.method_fields.get(method).cloned()
    }

    /// Check if a method writes to a specific field
    ///
    /// Note: Currently this is a conservative approximation - we treat all field
    /// accesses as potential writes since detecting true writes requires deeper
    /// analysis of assignment contexts. This is acceptable for clustering purposes
    /// where we weight shared field usage.
    pub fn method_writes_to_field(&self, method: &str, field: &str) -> bool {
        self.method_fields
            .get(method)
            .map(|fields| fields.contains(field))
            .unwrap_or(false)
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
        // Per spec 208: "new" is Construction (checked before Lifecycle)
        assert_eq!(
            BehavioralCategorizer::categorize_method("new"),
            BehaviorCategory::Construction
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
    fn test_categorize_parsing_methods() {
        // Per spec 208: Verify parsing methods are correctly categorized
        assert_eq!(
            BehavioralCategorizer::categorize_method("parse_json"),
            BehaviorCategory::Parsing
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("read_config"),
            BehaviorCategory::Parsing
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("extract_data"),
            BehaviorCategory::Parsing
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("decode_message"),
            BehaviorCategory::Parsing
        );
    }

    #[test]
    fn test_categorize_construction_methods() {
        // Per spec 208: Construction methods checked before Lifecycle
        assert_eq!(
            BehavioralCategorizer::categorize_method("create_instance"),
            BehaviorCategory::Construction
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("build_object"),
            BehaviorCategory::Construction
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("make_widget"),
            BehaviorCategory::Construction
        );
    }

    #[test]
    fn test_categorize_filtering_methods() {
        // Per spec 208: Filtering methods correctly identified
        assert_eq!(
            BehavioralCategorizer::categorize_method("filter_results"),
            BehaviorCategory::Filtering
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("select_items"),
            BehaviorCategory::Filtering
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("find_matches"),
            BehaviorCategory::Filtering
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("search_database"),
            BehaviorCategory::Filtering
        );
    }

    #[test]
    fn test_categorize_transformation_methods() {
        // Per spec 208: Transformation methods correctly identified
        assert_eq!(
            BehavioralCategorizer::categorize_method("transform_data"),
            BehaviorCategory::Transformation
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("convert_to_json"),
            BehaviorCategory::Transformation
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("map_values"),
            BehaviorCategory::Transformation
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("apply_transformation"),
            BehaviorCategory::Transformation
        );
    }

    #[test]
    fn test_categorize_data_access_methods() {
        // Per spec 208: DataAccess checked before StateManagement for get_/set_
        assert_eq!(
            BehavioralCategorizer::categorize_method("get_value"),
            BehaviorCategory::DataAccess
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("set_property"),
            BehaviorCategory::DataAccess
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("fetch_record"),
            BehaviorCategory::DataAccess
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("retrieve_data"),
            BehaviorCategory::DataAccess
        );
    }

    #[test]
    fn test_categorize_processing_methods() {
        // Per spec 208: Processing methods correctly identified
        // Note: "handle_" prefix is EventHandling, so use "process", "execute", "run"
        assert_eq!(
            BehavioralCategorizer::categorize_method("process_request"),
            BehaviorCategory::Processing
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("process_message"),
            BehaviorCategory::Processing
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("execute_task"),
            BehaviorCategory::Processing
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("run_pipeline"),
            BehaviorCategory::Processing
        );
    }

    #[test]
    fn test_categorize_communication_methods() {
        // Per spec 208: Communication methods correctly identified
        assert_eq!(
            BehavioralCategorizer::categorize_method("send_message"),
            BehaviorCategory::Communication
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("receive_data"),
            BehaviorCategory::Communication
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("transmit_packet"),
            BehaviorCategory::Communication
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("broadcast_update"),
            BehaviorCategory::Communication
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
        // Per spec 208: get_/set_ are DataAccess (checked before StateManagement)
        assert_eq!(
            BehavioralCategorizer::categorize_method("get_value"),
            BehaviorCategory::DataAccess
        );
        assert_eq!(
            BehavioralCategorizer::categorize_method("set_name"),
            BehaviorCategory::DataAccess
        );
        // update_state contains "_state" so it's still StateManagement
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
        // Per spec 208: get_/set_ are now DataAccess (not StateManagement)
        assert!(clusters.contains_key(&BehaviorCategory::DataAccess));

        assert_eq!(clusters.get(&BehaviorCategory::Rendering).unwrap().len(), 2);
        assert_eq!(
            clusters
                .get(&BehaviorCategory::EventHandling)
                .unwrap()
                .len(),
            2
        );
        assert_eq!(clusters.get(&BehaviorCategory::DataAccess).unwrap().len(), 2);
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

        // Per spec 208: Check that we have a Construction cluster (new, create_empty, build_*)
        let construction_cluster = clusters
            .iter()
            .find(|c| matches!(c.category, BehaviorCategory::Construction));
        assert!(
            construction_cluster.is_some(),
            "Expected to find Construction cluster for methods like 'new', 'create_empty', 'build_index'"
        );

        // Per spec 208: Check that we have a DataAccess cluster (get_* methods)
        let data_access_cluster = clusters
            .iter()
            .find(|c| matches!(c.category, BehaviorCategory::DataAccess));
        assert!(
            data_access_cluster.is_some(),
            "Expected to find DataAccess cluster for get_* methods"
        );

        // Per spec 208: Verify that Parsing cluster exists (parse_* methods checked before Persistence)
        let parsing_cluster = clusters
            .iter()
            .find(|c| matches!(c.category, BehaviorCategory::Parsing));
        assert!(
            parsing_cluster.is_some(),
            "Expected to find Parsing cluster for parse_* methods"
        );

        // Per spec 208: Precedence rules (Construction before Lifecycle, DataAccess before StateManagement,
        // Parsing before Persistence) may result in clusters of varying sizes, including single-method clusters.
        // The important verification is diversity of categories (above), not minimum cluster sizes.

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
        let all_cluster_methods: Vec<&String> = clusters.iter().flat_map(|c| &c.methods).collect();

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
        println!(
            "Production methods: {} / {} total",
            all_cluster_methods.len(),
            methods.len()
        );
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

    #[test]
    fn test_no_method_loss_and_minimum_cluster_size() {
        // Phase 1 requirements test:
        // 1. All methods must be accounted for (no losses)
        // 2. No clusters smaller than 3 methods
        // 3. Low-cohesion methods kept in behavioral categories

        let methods = vec![
            // Rendering group (high cohesion)
            "render_text".to_string(),
            "render_cursor".to_string(),
            "paint_highlights".to_string(),
            "draw_gutter".to_string(),
            // Utilities (low cohesion, no internal calls)
            "format_timestamp".to_string(),
            "clamp_value".to_string(),
            // Single method categories
            "validate_config".to_string(),
            // State management
            "get_state".to_string(),
            "set_state".to_string(),
        ];

        let adjacency = HashMap::from([
            // Rendering cluster has internal calls
            (
                ("render_text".to_string(), "paint_highlights".to_string()),
                1,
            ),
            (("render_cursor".to_string(), "draw_gutter".to_string()), 1),
            // Utilities have zero internal calls (low cohesion)
            // Validation has no calls (isolated)
            // State methods call each other
            (("set_state".to_string(), "get_state".to_string()), 1),
        ]);

        let clusters = apply_production_ready_clustering(&methods, &adjacency);

        // REQUIREMENT 1: All methods must be accounted for
        let clustered_methods: std::collections::HashSet<String> =
            clusters.iter().flat_map(|c| &c.methods).cloned().collect();

        for method in &methods {
            assert!(
                clustered_methods.contains(method),
                "Method '{}' was lost during clustering!",
                method
            );
        }

        assert_eq!(
            clustered_methods.len(),
            methods.len(),
            "Total methods mismatch: {} clustered vs {} input",
            clustered_methods.len(),
            methods.len()
        );

        // REQUIREMENT 2: No clusters smaller than 3 methods
        for cluster in &clusters {
            assert!(
                cluster.methods.len() >= 3,
                "Cluster '{}' has only {} methods (minimum is 3)",
                cluster.category.display_name(),
                cluster.methods.len()
            );
        }

        // REQUIREMENT 3: Low-cohesion methods kept (not filtered out)
        // format_timestamp and clamp_value have zero cohesion but should be in a cluster
        assert!(
            clustered_methods.contains("format_timestamp"),
            "Low-cohesion method 'format_timestamp' should be kept"
        );
        assert!(
            clustered_methods.contains("clamp_value"),
            "Low-cohesion method 'clamp_value' should be kept"
        );

        println!("\n=== No Method Loss Test Results ===");
        println!("Total input methods: {}", methods.len());
        println!("Total clustered methods: {}", clustered_methods.len());
        println!("Clusters created: {}", clusters.len());
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
        println!("=====================================\n");
    }

    // Unit tests for predicate functions (Spec 208 requirement)

    #[test]
    fn test_is_parsing_predicate() {
        // Per spec 208: Test is_parsing predicate function
        assert!(BehavioralCategorizer::is_parsing("parse_json"));
        assert!(BehavioralCategorizer::is_parsing("read_file"));
        assert!(BehavioralCategorizer::is_parsing("extract_data"));
        assert!(BehavioralCategorizer::is_parsing("decode_base64"));
        assert!(BehavioralCategorizer::is_parsing("deserialize_xml"));
        assert!(BehavioralCategorizer::is_parsing("unmarshal_proto"));
        assert!(BehavioralCategorizer::is_parsing("scan_tokens"));

        // Negative cases
        assert!(!BehavioralCategorizer::is_parsing("render_view"));
        assert!(!BehavioralCategorizer::is_parsing("calculate_sum"));
        assert!(!BehavioralCategorizer::is_parsing("validate_input"));
    }

    #[test]
    fn test_is_rendering_predicate() {
        // Per spec 208: Test is_rendering predicate function
        assert!(BehavioralCategorizer::is_rendering("render_template"));
        assert!(BehavioralCategorizer::is_rendering("draw_rectangle"));
        assert!(BehavioralCategorizer::is_rendering("paint_canvas"));
        assert!(BehavioralCategorizer::is_rendering("display_message"));
        assert!(BehavioralCategorizer::is_rendering("show_dialog"));
        assert!(BehavioralCategorizer::is_rendering("present_view"));
        assert!(BehavioralCategorizer::is_rendering("format_output"));
        assert!(BehavioralCategorizer::is_rendering("to_string"));
        assert!(BehavioralCategorizer::is_rendering("print_report"));

        // Negative cases
        assert!(!BehavioralCategorizer::is_rendering("parse_json"));
        assert!(!BehavioralCategorizer::is_rendering("calculate_sum"));
        assert!(!BehavioralCategorizer::is_rendering("validate_input"));
    }

    #[test]
    fn test_is_filtering_predicate() {
        // Per spec 208: Test is_filtering predicate function
        assert!(BehavioralCategorizer::is_filtering("filter_results"));
        assert!(BehavioralCategorizer::is_filtering("select_items"));
        assert!(BehavioralCategorizer::is_filtering("find_matches"));
        assert!(BehavioralCategorizer::is_filtering("search_database"));
        assert!(BehavioralCategorizer::is_filtering("query_records"));
        assert!(BehavioralCategorizer::is_filtering("lookup_value"));
        assert!(BehavioralCategorizer::is_filtering("match_pattern"));

        // Negative cases
        assert!(!BehavioralCategorizer::is_filtering("parse_json"));
        assert!(!BehavioralCategorizer::is_filtering("render_view"));
        assert!(!BehavioralCategorizer::is_filtering("calculate_sum"));
    }

    #[test]
    fn test_is_transformation_predicate() {
        // Per spec 208: Test is_transformation predicate function
        assert!(BehavioralCategorizer::is_transformation("transform_data"));
        assert!(BehavioralCategorizer::is_transformation("convert_format"));
        assert!(BehavioralCategorizer::is_transformation("map_values"));
        assert!(BehavioralCategorizer::is_transformation("apply_rules"));
        assert!(BehavioralCategorizer::is_transformation("adapt_schema"));

        // Negative cases
        assert!(!BehavioralCategorizer::is_transformation("parse_json"));
        assert!(!BehavioralCategorizer::is_transformation("filter_results"));
        assert!(!BehavioralCategorizer::is_transformation("validate_input"));
    }

    #[test]
    fn test_is_construction_predicate() {
        // Per spec 208: Test is_construction predicate function (checked before Lifecycle)
        assert!(BehavioralCategorizer::is_construction("create_instance"));
        assert!(BehavioralCategorizer::is_construction("build_object"));
        assert!(BehavioralCategorizer::is_construction("new_connection"));
        assert!(BehavioralCategorizer::is_construction("make_widget"));
        assert!(BehavioralCategorizer::is_construction("construct_tree"));

        // Negative cases
        assert!(!BehavioralCategorizer::is_construction("parse_json"));
        assert!(!BehavioralCategorizer::is_construction("render_view"));
        assert!(!BehavioralCategorizer::is_construction("validate_input"));
    }

    #[test]
    fn test_is_data_access_predicate() {
        // Per spec 208: Test is_data_access predicate function (checked before StateManagement)
        assert!(BehavioralCategorizer::is_data_access("get_value"));
        assert!(BehavioralCategorizer::is_data_access("set_property"));
        assert!(BehavioralCategorizer::is_data_access("fetch_record"));
        assert!(BehavioralCategorizer::is_data_access("retrieve_data"));
        assert!(BehavioralCategorizer::is_data_access("access_field"));

        // Negative cases
        assert!(!BehavioralCategorizer::is_data_access("parse_json"));
        assert!(!BehavioralCategorizer::is_data_access("render_view"));
        assert!(!BehavioralCategorizer::is_data_access("validate_input"));
    }

    #[test]
    fn test_is_communication_predicate() {
        // Per spec 208: Test is_communication predicate function
        assert!(BehavioralCategorizer::is_communication("send_message"));
        assert!(BehavioralCategorizer::is_communication("receive_data"));
        assert!(BehavioralCategorizer::is_communication("transmit_packet"));
        assert!(BehavioralCategorizer::is_communication("broadcast_event"));
        assert!(BehavioralCategorizer::is_communication("notify_observers"));

        // Negative cases
        assert!(!BehavioralCategorizer::is_communication("parse_json"));
        assert!(!BehavioralCategorizer::is_communication("render_view"));
        assert!(!BehavioralCategorizer::is_communication("validate_input"));
    }

    #[test]
    fn test_no_duplicate_responsibilities_integration() {
        // Per spec 208: Integration test to verify no duplicate responsibilities
        // with different capitalizations. This was a key objective of the spec.

        let methods = vec![
            // Rendering methods (should all map to "Rendering", not "output" or "rendering")
            "format_output".to_string(),
            "format_json".to_string(),
            "render_view".to_string(),
            "draw_chart".to_string(),
            // Parsing methods (should all map to "Parsing", not "parsing" or "PARSING")
            "parse_json".to_string(),
            "parse_xml".to_string(),
            "read_config".to_string(),
            // DataAccess methods (should all map to "Data Access", not "data_access" or "DataAccess")
            "get_value".to_string(),
            "set_property".to_string(),
            "fetch_record".to_string(),
            // Validation methods (should all map to "Validation", not "validation")
            "validate_input".to_string(),
            "check_bounds".to_string(),
            "is_valid".to_string(),
        ];

        let clusters = cluster_methods_by_behavior(&methods);

        // Collect all category display names (which should be Title Case)
        let category_names: Vec<String> = clusters
            .keys()
            .map(|cat| cat.display_name())
            .collect();

        // Check for duplicates (case-insensitive comparison)
        let mut seen_lower = std::collections::HashSet::new();
        let mut duplicates = Vec::new();

        for name in &category_names {
            let lower = name.to_lowercase();
            if seen_lower.contains(&lower) {
                duplicates.push(name.clone());
            }
            seen_lower.insert(lower);
        }

        assert!(
            duplicates.is_empty(),
            "Found duplicate responsibilities with different capitalizations: {:?}\nAll categories: {:?}",
            duplicates,
            category_names
        );

        // Verify that all category names use consistent Title Case
        for name in &category_names {
            // Title Case means first letter uppercase, rest depend on context
            // For single-word categories: "Rendering", "Parsing", "Validation"
            // For multi-word: "Data Access", "State Management"
            let first_char = name.chars().next().unwrap();
            assert!(
                first_char.is_uppercase(),
                "Category '{}' should start with uppercase letter (Title Case)",
                name
            );
        }

        // Verify expected categories are present with correct casing
        let has_rendering = category_names.iter().any(|n| n == "Rendering");
        let has_parsing = category_names.iter().any(|n| n == "Parsing");
        let has_data_access = category_names.iter().any(|n| n == "Data Access");
        let has_validation = category_names.iter().any(|n| n == "Validation");

        assert!(has_rendering, "Expected 'Rendering' category (Title Case)");
        assert!(has_parsing, "Expected 'Parsing' category (Title Case)");
        assert!(has_data_access, "Expected 'Data Access' category (Title Case)");
        assert!(has_validation, "Expected 'Validation' category (Title Case)");

        // Verify NO lowercase versions exist
        assert!(
            !category_names.iter().any(|n| n == "output"),
            "Should not have lowercase 'output' category"
        );
        assert!(
            !category_names.iter().any(|n| n == "parsing"),
            "Should not have lowercase 'parsing' category"
        );
        assert!(
            !category_names.iter().any(|n| n == "data_access"),
            "Should not have snake_case 'data_access' category"
        );
        assert!(
            !category_names.iter().any(|n| n == "validation"),
            "Should not have lowercase 'validation' category"
        );
    }
}
