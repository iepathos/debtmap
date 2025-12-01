//! Type Flow Tracking System for Python
//!
//! Implements a conservative data flow analysis to track how types propagate through Python
//! code via assignments, method calls, and collection operations. This provides the foundation
//! for accurate observer pattern detection and reduces false positives in dead code analysis.
//!
//! ## Architecture
//!
//! The type flow tracker uses a conservative over-approximation approach:
//! - If a type *might* flow into a location, it's recorded
//! - Prefer false positives (extra types) over false negatives (missed types)
//! - For branches: union all possible types from all branches
//! - For loops: assume all iterations contribute types
//!
//! ## Usage
//!
//! ```rust
//! use debtmap::analysis::type_flow_tracker::{TypeFlowTracker, TypeId};
//! use std::path::PathBuf;
//!
//! let mut tracker = TypeFlowTracker::new();
//! let type_id = TypeId::new("ConcreteObserver".to_string(), Some(PathBuf::from("observer.py")));
//!
//! // Track assignment: x = ConcreteObserver()
//! tracker.record_assignment("x", type_id.clone());
//!
//! // Track collection operation: self.observers.append(observer)
//! tracker.record_collection_add("self.observers", type_id.clone());
//!
//! // Query types in collection
//! let types = tracker.get_collection_types("self.observers");
//! ```

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

// Stub for removed Python AST types
pub mod ast {
    #[derive(Debug)]
    pub enum Expr {
        Name(Name),
        Attribute(Attribute),
        Other,
    }

    #[derive(Debug)]
    pub struct Name {
        pub id: String,
    }

    #[derive(Debug)]
    pub struct Attribute {
        pub value: Box<Expr>,
        pub attr: String,
    }
}

/// Type identifier with source location
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct TypeId {
    /// Type name (e.g., "ConcreteObserver")
    pub name: String,
    /// Module where type is defined
    pub module: Option<PathBuf>,
}

impl TypeId {
    /// Create a new type identifier
    pub fn new(name: String, module: Option<PathBuf>) -> Self {
        Self { name, module }
    }

    /// Create a type ID from a simple name (no module)
    pub fn from_name(name: &str) -> Self {
        Self {
            name: name.to_string(),
            module: None,
        }
    }

    /// Get qualified name for disambiguation
    ///
    /// Format: "module/path:TypeName" or just "TypeName" if no module
    pub fn qualified_name(&self) -> String {
        if let Some(module) = &self.module {
            format!("{}:{}", module.display(), self.name)
        } else {
            self.name.clone()
        }
    }
}

/// Type information with metadata
#[derive(Debug, Clone)]
pub struct TypeInfo {
    /// Type identifier
    pub type_id: TypeId,
    /// Where this type was instantiated/defined
    pub source_location: Location,
    /// Base classes if known
    pub base_classes: Vec<TypeId>,
}

/// Source location for tracking type origins
#[derive(Debug, Clone)]
pub struct Location {
    /// File path
    pub file: PathBuf,
    /// Line number (0 if unknown)
    pub line: usize,
}

impl Location {
    /// Create a new location
    pub fn new(file: PathBuf, line: usize) -> Self {
        Self { file, line }
    }

    /// Create an unknown location
    pub fn unknown() -> Self {
        Self {
            file: PathBuf::from("<unknown>"),
            line: 0,
        }
    }
}

/// Collection operation types
#[derive(Debug, Clone)]
pub enum CollectionOp {
    /// Single item append: list.append(item)
    Append(TypeId),
    /// Multiple items extend: list.extend([item1, item2])
    Extend(Vec<TypeId>),
}

/// Tracks types flowing through Python code
///
/// The tracker maintains three primary mappings:
/// 1. Variable -> Types: tracks type flow through variable assignments
/// 2. Collection -> Types: tracks types added to collections (lists, sets, etc.)
/// 3. Parameter -> Types: tracks types passed as function parameters
///
/// All mappings use conservative over-approximation: if a type *might* flow,
/// it's recorded.
#[derive(Debug)]
pub struct TypeFlowTracker {
    /// Variable -> Set of types that have flowed into it
    variable_types: HashMap<String, HashSet<TypeId>>,

    /// Collection (e.g., "self.observers") -> Types added to it
    collection_types: HashMap<String, HashSet<TypeId>>,

    /// Parameter (func_name, param_index) -> Types passed to it
    parameter_types: HashMap<(String, usize), HashSet<TypeId>>,

    /// Type identifier with source location
    type_registry: HashMap<TypeId, TypeInfo>,
}

impl TypeFlowTracker {
    /// Create a new type flow tracker
    pub fn new() -> Self {
        Self {
            variable_types: HashMap::new(),
            collection_types: HashMap::new(),
            parameter_types: HashMap::new(),
            type_registry: HashMap::new(),
        }
    }

    /// Record that a type flows into a variable
    ///
    /// # Example
    /// ```rust
    /// # use debtmap::analysis::type_flow_tracker::{TypeFlowTracker, TypeId};
    /// let mut tracker = TypeFlowTracker::new();
    /// let type_id = TypeId::from_name("Observer");
    /// tracker.record_assignment("x", type_id);
    /// ```
    pub fn record_assignment(&mut self, target: &str, type_id: TypeId) {
        self.variable_types
            .entry(target.to_string())
            .or_default()
            .insert(type_id);
    }

    /// Record that a type flows into a variable via AST expression
    ///
    /// This is a convenience method for integration with AST analysis
    pub fn record_assignment_expr(&mut self, target: &ast::Expr, type_id: TypeId) {
        let target_name = self.extract_target_name(target);
        if let Some(name) = target_name {
            self.record_assignment(&name, type_id);
        }
    }

    /// Record that a type is added to a collection
    ///
    /// # Example
    /// ```rust
    /// # use debtmap::analysis::type_flow_tracker::{TypeFlowTracker, TypeId};
    /// let mut tracker = TypeFlowTracker::new();
    /// let type_id = TypeId::from_name("ConcreteObserver");
    /// tracker.record_collection_add("self.observers", type_id);
    /// ```
    pub fn record_collection_add(&mut self, collection: &str, type_id: TypeId) {
        self.collection_types
            .entry(collection.to_string())
            .or_default()
            .insert(type_id);
    }

    /// Record that types are added to a collection via extend operation
    pub fn record_collection_extend(&mut self, collection: &str, type_ids: Vec<TypeId>) {
        let entry = self
            .collection_types
            .entry(collection.to_string())
            .or_default();
        for type_id in type_ids {
            entry.insert(type_id);
        }
    }

    /// Track collection operation (append, extend, etc.)
    pub fn track_collection_operation(&mut self, collection: &str, operation: CollectionOp) {
        match operation {
            CollectionOp::Append(type_id) => {
                self.record_collection_add(collection, type_id);
            }
            CollectionOp::Extend(type_ids) => {
                self.record_collection_extend(collection, type_ids);
            }
        }
    }

    /// Record that a type flows into a parameter
    ///
    /// # Example
    /// ```rust
    /// # use debtmap::analysis::type_flow_tracker::{TypeFlowTracker, TypeId};
    /// let mut tracker = TypeFlowTracker::new();
    /// let type_id = TypeId::from_name("Observer");
    /// tracker.record_parameter_flow("Subject.attach", 0, type_id);
    /// ```
    pub fn record_parameter_flow(&mut self, func: &str, param_idx: usize, type_id: TypeId) {
        self.parameter_types
            .entry((func.to_string(), param_idx))
            .or_default()
            .insert(type_id);
    }

    /// Register type information in the type registry
    pub fn register_type(&mut self, type_info: TypeInfo) {
        self.type_registry
            .insert(type_info.type_id.clone(), type_info);
    }

    /// Get all types that have flowed into a variable
    pub fn get_variable_types(&self, variable: &str) -> Vec<&TypeInfo> {
        self.variable_types
            .get(variable)
            .map(|type_ids| {
                type_ids
                    .iter()
                    .filter_map(|id| self.type_registry.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all types that have flowed into a collection
    pub fn get_collection_types(&self, collection: &str) -> Vec<&TypeInfo> {
        self.collection_types
            .get(collection)
            .map(|type_ids| {
                type_ids
                    .iter()
                    .filter_map(|id| self.type_registry.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all type IDs (without full info) that have flowed into a collection
    pub fn get_collection_type_ids(&self, collection: &str) -> Vec<TypeId> {
        self.collection_types
            .get(collection)
            .map(|type_ids| type_ids.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all types that have flowed into a parameter
    pub fn get_parameter_types(&self, func: &str, param_idx: usize) -> Vec<&TypeInfo> {
        self.parameter_types
            .get(&(func.to_string(), param_idx))
            .map(|type_ids| {
                type_ids
                    .iter()
                    .filter_map(|id| self.type_registry.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get a single variable type (returns first if multiple)
    ///
    /// This is useful when you expect a variable to have a single type
    pub fn get_variable_type(&self, variable: &str) -> Option<TypeId> {
        self.variable_types
            .get(variable)
            .and_then(|types| types.iter().next().cloned())
    }

    /// Extract target name from assignment expression
    ///
    /// Handles:
    /// - Simple names: `x`
    /// - Attributes: `self.observers`
    /// - Multiple assignment targets (returns first)
    fn extract_target_name(&self, expr: &ast::Expr) -> Option<String> {
        extract_target_name_impl(expr)
    }
}

/// Extract target name from assignment expression (standalone function for recursion)
fn extract_target_name_impl(expr: &ast::Expr) -> Option<String> {
    match expr {
        ast::Expr::Name(name) => Some(name.id.to_string()),
        ast::Expr::Attribute(attr) => {
            // Build qualified name like "self.observers"
            let base = extract_target_name_impl(&attr.value)?;
            Some(format!("{}.{}", base, attr.attr))
        }
        _ => None,
    }
}

impl Default for TypeFlowTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_assignment() {
        // x = ConcreteObserver()
        let mut tracker = TypeFlowTracker::new();
        let type_id = TypeId::from_name("ConcreteObserver");

        // Register the type
        tracker.register_type(TypeInfo {
            type_id: type_id.clone(),
            source_location: Location::unknown(),
            base_classes: vec![],
        });

        tracker.record_assignment("x", type_id.clone());

        let types = tracker.get_variable_types("x");
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].type_id, type_id);
    }

    #[test]
    fn test_collection_append() {
        // self.observers.append(ConcreteObserver())
        let mut tracker = TypeFlowTracker::new();
        let type_id = TypeId::from_name("ConcreteObserver");

        tracker.register_type(TypeInfo {
            type_id: type_id.clone(),
            source_location: Location::unknown(),
            base_classes: vec![],
        });

        tracker.record_collection_add("self.observers", type_id.clone());

        let types = tracker.get_collection_types("self.observers");
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].type_id.name, "ConcreteObserver");
    }

    #[test]
    fn test_parameter_flow() {
        // subject.attach(observer)
        let mut tracker = TypeFlowTracker::new();
        let type_id = TypeId::from_name("ConcreteObserver");

        tracker.register_type(TypeInfo {
            type_id: type_id.clone(),
            source_location: Location::unknown(),
            base_classes: vec![],
        });

        tracker.record_parameter_flow("Subject.attach", 0, type_id.clone());

        let types = tracker.get_parameter_types("Subject.attach", 0);
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].type_id.name, "ConcreteObserver");
    }

    #[test]
    fn test_multiple_types_in_collection() {
        // observers = [ObsA(), ObsB()]
        let mut tracker = TypeFlowTracker::new();
        let type_a = TypeId::from_name("ObsA");
        let type_b = TypeId::from_name("ObsB");

        tracker.register_type(TypeInfo {
            type_id: type_a.clone(),
            source_location: Location::unknown(),
            base_classes: vec![],
        });

        tracker.register_type(TypeInfo {
            type_id: type_b.clone(),
            source_location: Location::unknown(),
            base_classes: vec![],
        });

        tracker.record_collection_add("observers", type_a);
        tracker.record_collection_add("observers", type_b);

        let types = tracker.get_collection_types("observers");
        assert_eq!(types.len(), 2);
    }

    #[test]
    fn test_collection_extend() {
        let mut tracker = TypeFlowTracker::new();
        let type_a = TypeId::from_name("ObsA");
        let type_b = TypeId::from_name("ObsB");

        tracker.register_type(TypeInfo {
            type_id: type_a.clone(),
            source_location: Location::unknown(),
            base_classes: vec![],
        });

        tracker.register_type(TypeInfo {
            type_id: type_b.clone(),
            source_location: Location::unknown(),
            base_classes: vec![],
        });

        tracker.record_collection_extend("observers", vec![type_a, type_b]);

        let types = tracker.get_collection_types("observers");
        assert_eq!(types.len(), 2);
    }

    #[test]
    fn test_track_collection_operation_append() {
        let mut tracker = TypeFlowTracker::new();
        let type_id = TypeId::from_name("Observer");

        tracker.register_type(TypeInfo {
            type_id: type_id.clone(),
            source_location: Location::unknown(),
            base_classes: vec![],
        });

        tracker.track_collection_operation("observers", CollectionOp::Append(type_id));

        let types = tracker.get_collection_types("observers");
        assert_eq!(types.len(), 1);
    }

    #[test]
    fn test_track_collection_operation_extend() {
        let mut tracker = TypeFlowTracker::new();
        let type_a = TypeId::from_name("ObsA");
        let type_b = TypeId::from_name("ObsB");

        tracker.register_type(TypeInfo {
            type_id: type_a.clone(),
            source_location: Location::unknown(),
            base_classes: vec![],
        });

        tracker.register_type(TypeInfo {
            type_id: type_b.clone(),
            source_location: Location::unknown(),
            base_classes: vec![],
        });

        tracker.track_collection_operation("observers", CollectionOp::Extend(vec![type_a, type_b]));

        let types = tracker.get_collection_types("observers");
        assert_eq!(types.len(), 2);
    }

    #[test]
    fn test_get_variable_type_single() {
        let mut tracker = TypeFlowTracker::new();
        let type_id = TypeId::from_name("MyClass");

        tracker.record_assignment("x", type_id.clone());

        let result = tracker.get_variable_type("x");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), type_id);
    }

    #[test]
    fn test_get_variable_type_none() {
        let tracker = TypeFlowTracker::new();
        let result = tracker.get_variable_type("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_collection_type_ids() {
        let mut tracker = TypeFlowTracker::new();
        let type_a = TypeId::from_name("ObsA");
        let type_b = TypeId::from_name("ObsB");

        tracker.record_collection_add("observers", type_a.clone());
        tracker.record_collection_add("observers", type_b.clone());

        let ids = tracker.get_collection_type_ids("observers");
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&type_a));
        assert!(ids.contains(&type_b));
    }

    #[test]
    fn test_type_id_equality() {
        let type1 = TypeId::from_name("Observer");
        let type2 = TypeId::from_name("Observer");
        let type3 = TypeId::from_name("Handler");

        assert_eq!(type1, type2);
        assert_ne!(type1, type3);
    }

    #[test]
    fn test_type_id_with_module() {
        let type1 = TypeId::new("Observer".to_string(), Some(PathBuf::from("observer.py")));
        let type2 = TypeId::new("Observer".to_string(), Some(PathBuf::from("observer.py")));
        let type3 = TypeId::new("Observer".to_string(), Some(PathBuf::from("handler.py")));
        let type4 = TypeId::new("Observer".to_string(), None);

        assert_eq!(type1, type2);
        assert_ne!(type1, type3);
        assert_ne!(type1, type4);
    }

    #[test]
    fn test_empty_tracker() {
        let tracker = TypeFlowTracker::new();

        assert!(tracker.get_variable_types("x").is_empty());
        assert!(tracker.get_collection_types("observers").is_empty());
        assert!(tracker.get_parameter_types("func", 0).is_empty());
    }

    #[test]
    fn test_conservative_multiple_assignments() {
        // Tracks multiple types assigned to same variable (conservative)
        let mut tracker = TypeFlowTracker::new();
        let type_a = TypeId::from_name("TypeA");
        let type_b = TypeId::from_name("TypeB");

        tracker.register_type(TypeInfo {
            type_id: type_a.clone(),
            source_location: Location::unknown(),
            base_classes: vec![],
        });

        tracker.register_type(TypeInfo {
            type_id: type_b.clone(),
            source_location: Location::unknown(),
            base_classes: vec![],
        });

        tracker.record_assignment("x", type_a);
        tracker.record_assignment("x", type_b);

        let types = tracker.get_variable_types("x");
        assert_eq!(types.len(), 2); // Conservative: both types recorded
    }
}
