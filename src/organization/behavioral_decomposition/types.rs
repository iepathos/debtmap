//! Core types for behavioral decomposition analysis.
//!
//! This module contains the fundamental data structures used throughout
//! the behavioral decomposition system.

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

/// Statistics about field access patterns
#[derive(Debug, Clone)]
pub struct FieldAccessStats {
    pub field_name: String,
    pub accessed_by: Vec<String>,
    pub access_frequency: usize,
    pub access_percentage: f64,
}

/// Capitalize first character of a string
pub(crate) fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
