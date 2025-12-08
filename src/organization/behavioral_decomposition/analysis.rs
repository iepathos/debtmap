/// Field access analysis and refactoring recommendations.
///
/// This module provides tools for analyzing field access patterns and generating
/// recommendations for service extraction and trait-based refactoring:
///
/// - **FieldAccessTracker**: Analyzes which fields each method accesses
/// - **Service extraction**: Identifies methods that can be extracted to service objects
/// - **Trait extraction**: Suggests trait-based decomposition for behavioral clusters
use std::collections::{HashMap, HashSet};
use syn::{visit::Visit, Expr, ExprField, ImplItemFn, ItemImpl};

use super::types::{capitalize_first, BehaviorCategory, FieldAccessStats, MethodCluster};

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
        BehaviorCategory::Utilities => "Utilities".to_string(),
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
