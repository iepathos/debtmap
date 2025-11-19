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
        ) || methods.len() >= 5 // Keep domain clusters only if they have 5+ methods
    });

    clusters
}

/// Build method call adjacency matrix from impl blocks
///
/// This function analyzes method call patterns within impl blocks to build
/// an adjacency matrix showing which methods call which other methods.
///
/// Returns: HashMap<(method_a, method_b), call_count>
pub fn build_method_call_adjacency_matrix(
    impl_blocks: &[&syn::ItemImpl],
) -> HashMap<(String, String), usize> {
    use syn::visit::Visit;

    let mut matrix = HashMap::new();

    for impl_block in impl_blocks {
        for item in &impl_block.items {
            if let syn::ImplItem::Fn(method) = item {
                let method_name = method.sig.ident.to_string();

                // Visit method body to find method calls
                let mut call_visitor = MethodCallVisitor {
                    current_method: method_name.clone(),
                    calls: Vec::new(),
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

    matrix
}

/// Visitor to extract method calls from a method body
struct MethodCallVisitor {
    current_method: String,
    calls: Vec<String>,
}

impl<'ast> Visit<'ast> for MethodCallVisitor {
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
    clusters
        .into_values()
        .filter(|methods| methods.len() >= 5) // Only clusters with 5+ methods
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
        .filter(|cluster| cluster.cohesion_score > 0.3) // Filter low-cohesion clusters
        .collect()
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
}
