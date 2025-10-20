//! Observer Registry for tracking observer collections and implementations
//!
//! Maintains mappings between:
//! - Classes and their observer collection fields
//! - Observer interfaces and concrete implementations
//! - Collection names and their interface types

use crate::priority::call_graph::FunctionId;
use std::collections::HashMap;

/// Observer registry for tracking observer collections and implementations
///
/// The registry maintains mappings between classes, observer collections,
/// and concrete implementations to enable accurate call graph construction
/// for observer pattern dispatch.
///
/// # Example
/// ```
/// use debtmap::analysis::python_call_graph::observer_registry::ObserverRegistry;
/// use debtmap::priority::call_graph::FunctionId;
/// use std::path::PathBuf;
///
/// let mut registry = ObserverRegistry::new();
/// registry.register_collection("Subject", "observers", "Observer");
///
/// let func_id = FunctionId {
///     file: PathBuf::from("test.py"),
///     name: "ConcreteObserver.on_event".to_string(),
///     line: 20,
/// };
/// registry.register_implementation("Observer", "on_event", func_id);
///
/// assert!(registry.is_observer_collection("Subject", "observers"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct ObserverRegistry {
    /// Class name -> Collection field name -> Observer interface type
    collections: HashMap<String, HashMap<String, String>>,

    /// Observer interface -> Method name -> List of implementation function IDs
    implementations: HashMap<String, HashMap<String, Vec<FunctionId>>>,

    /// Implementation class -> Interface (reverse lookup)
    class_to_interface: HashMap<String, String>,
}

impl ObserverRegistry {
    /// Create a new empty observer registry
    pub fn new() -> Self {
        Self {
            collections: HashMap::new(),
            implementations: HashMap::new(),
            class_to_interface: HashMap::new(),
        }
    }

    /// Register an observer collection field on a class
    ///
    /// # Arguments
    /// * `class_name` - Name of the class containing the collection
    /// * `field_name` - Name of the collection field (e.g., "observers")
    /// * `interface_type` - Type of the observer interface
    pub fn register_collection(
        &mut self,
        class_name: &str,
        field_name: &str,
        interface_type: &str,
    ) {
        self.collections
            .entry(class_name.to_string())
            .or_default()
            .insert(field_name.to_string(), interface_type.to_string());
    }

    /// Register a concrete implementation of an observer method
    ///
    /// # Arguments
    /// * `interface_name` - Name of the observer interface
    /// * `method_name` - Name of the observer method
    /// * `impl_id` - Function ID of the concrete implementation
    pub fn register_implementation(
        &mut self,
        interface_name: &str,
        method_name: &str,
        impl_id: FunctionId,
    ) {
        self.implementations
            .entry(interface_name.to_string())
            .or_default()
            .entry(method_name.to_string())
            .or_default()
            .push(impl_id);
    }

    /// Register a class as implementing an observer interface
    ///
    /// # Arguments
    /// * `class_name` - Name of the implementation class
    /// * `interface_name` - Name of the observer interface
    pub fn register_class_interface(&mut self, class_name: &str, interface_name: &str) {
        self.class_to_interface
            .insert(class_name.to_string(), interface_name.to_string());
    }

    /// Check if a field is a registered observer collection
    ///
    /// # Arguments
    /// * `class_name` - Name of the class
    /// * `field_name` - Name of the field
    pub fn is_observer_collection(&self, class_name: &str, field_name: &str) -> bool {
        self.collections
            .get(class_name)
            .map(|fields| fields.contains_key(field_name))
            .unwrap_or(false)
    }

    /// Check if a field name looks like an observer collection
    ///
    /// Returns true for common observer collection names even if not explicitly registered
    pub fn is_observer_collection_name(field_name: &str) -> bool {
        matches!(
            field_name,
            "observers" | "listeners" | "callbacks" | "handlers" | "subscribers" | "watchers"
        )
    }

    /// Get the interface type for a collection
    ///
    /// # Arguments
    /// * `class_name` - Name of the class
    /// * `field_name` - Name of the collection field
    pub fn get_collection_interface(&self, class_name: &str, field_name: &str) -> Option<&String> {
        self.collections
            .get(class_name)
            .and_then(|fields| fields.get(field_name))
    }

    /// Get all implementations of a specific observer method
    ///
    /// # Arguments
    /// * `interface_name` - Name of the observer interface
    /// * `method_name` - Name of the method
    pub fn get_implementations(&self, interface_name: &str, method_name: &str) -> Vec<&FunctionId> {
        self.implementations
            .get(interface_name)
            .and_then(|methods| methods.get(method_name))
            .map(|impls| impls.iter().collect())
            .unwrap_or_default()
    }

    /// Get the observer interface for an implementation class
    pub fn get_interface_for_class(&self, class_name: &str) -> Option<&String> {
        self.class_to_interface.get(class_name)
    }

    /// Get all observer collections for a class
    pub fn get_collections_for_class(&self, class_name: &str) -> Vec<(&String, &String)> {
        self.collections
            .get(class_name)
            .map(|fields| fields.iter().collect())
            .unwrap_or_default()
    }

    /// Register a class as an observer interface
    pub fn register_interface(&mut self, interface_name: &str) {
        // Store in a placeholder to track known interfaces
        // We use an empty implementation map to mark it as a known interface
        self.implementations
            .entry(interface_name.to_string())
            .or_default();
    }

    /// Check if a class is a registered observer interface
    pub fn is_interface(&self, class_name: &str) -> bool {
        self.implementations.contains_key(class_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_register_and_lookup_collection() {
        let mut registry = ObserverRegistry::new();
        registry.register_collection("Manager", "observers", "Observer");

        assert!(registry.is_observer_collection("Manager", "observers"));
        assert!(!registry.is_observer_collection("Manager", "handlers"));
        assert!(!registry.is_observer_collection("OtherClass", "observers"));
    }

    #[test]
    fn test_get_collection_interface() {
        let mut registry = ObserverRegistry::new();
        registry.register_collection("Subject", "listeners", "Listener");

        let interface = registry.get_collection_interface("Subject", "listeners");
        assert_eq!(interface, Some(&"Listener".to_string()));

        let missing = registry.get_collection_interface("Subject", "missing");
        assert_eq!(missing, None);
    }

    #[test]
    fn test_register_and_get_implementations() {
        let mut registry = ObserverRegistry::new();
        let func_id = FunctionId {
            file: PathBuf::from("test.py"),
            name: "ConcreteObserver.on_event".to_string(),
            line: 20,
        };
        registry.register_implementation("Observer", "on_event", func_id.clone());

        let impls = registry.get_implementations("Observer", "on_event");
        assert_eq!(impls.len(), 1);
        assert_eq!(impls[0], &func_id);
    }

    #[test]
    fn test_multiple_implementations() {
        let mut registry = ObserverRegistry::new();

        let impl1 = FunctionId {
            file: PathBuf::from("test.py"),
            name: "Observer1.on_event".to_string(),
            line: 10,
        };
        let impl2 = FunctionId {
            file: PathBuf::from("test.py"),
            name: "Observer2.on_event".to_string(),
            line: 20,
        };

        registry.register_implementation("Observer", "on_event", impl1.clone());
        registry.register_implementation("Observer", "on_event", impl2.clone());

        let impls = registry.get_implementations("Observer", "on_event");
        assert_eq!(impls.len(), 2);
        assert!(impls.contains(&&impl1));
        assert!(impls.contains(&&impl2));
    }

    #[test]
    fn test_observer_collection_name_detection() {
        assert!(ObserverRegistry::is_observer_collection_name("observers"));
        assert!(ObserverRegistry::is_observer_collection_name("listeners"));
        assert!(ObserverRegistry::is_observer_collection_name("callbacks"));
        assert!(ObserverRegistry::is_observer_collection_name("handlers"));
        assert!(ObserverRegistry::is_observer_collection_name("subscribers"));
        assert!(ObserverRegistry::is_observer_collection_name("watchers"));

        assert!(!ObserverRegistry::is_observer_collection_name("items"));
        assert!(!ObserverRegistry::is_observer_collection_name("data"));
    }

    #[test]
    fn test_class_interface_mapping() {
        let mut registry = ObserverRegistry::new();
        registry.register_class_interface("ConcreteObserver", "Observer");

        let interface = registry.get_interface_for_class("ConcreteObserver");
        assert_eq!(interface, Some(&"Observer".to_string()));

        let missing = registry.get_interface_for_class("UnknownClass");
        assert_eq!(missing, None);
    }

    #[test]
    fn test_get_collections_for_class() {
        let mut registry = ObserverRegistry::new();
        registry.register_collection("Subject", "observers", "Observer");
        registry.register_collection("Subject", "listeners", "Listener");

        let collections = registry.get_collections_for_class("Subject");
        assert_eq!(collections.len(), 2);
    }
}
