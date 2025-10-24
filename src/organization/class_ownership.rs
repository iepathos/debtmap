use super::language::Language;
use std::collections::HashMap;

/// Unified class ownership structure across all languages
#[derive(Debug, Clone)]
pub struct ClassOwnership {
    pub language: Language,
    pub class_to_methods: HashMap<String, Vec<String>>,
    pub method_to_class: HashMap<String, String>,
    pub standalone_functions: Vec<String>,
    pub class_locations: HashMap<String, (usize, usize)>,
}

impl ClassOwnership {
    pub fn new(language: Language) -> Self {
        Self {
            language,
            class_to_methods: HashMap::new(),
            method_to_class: HashMap::new(),
            standalone_functions: Vec::new(),
            class_locations: HashMap::new(),
        }
    }

    pub fn add_class(&mut self, class_name: String, location: (usize, usize)) {
        self.class_to_methods.insert(class_name.clone(), Vec::new());
        self.class_locations.insert(class_name, location);
    }

    pub fn add_method(&mut self, class_name: &str, method_name: String) {
        if let Some(methods) = self.class_to_methods.get_mut(class_name) {
            methods.push(method_name.clone());
        }
        self.method_to_class
            .insert(method_name, class_name.to_string());
    }

    pub fn add_standalone_function(&mut self, function_name: String) {
        self.standalone_functions.push(function_name);
    }

    pub fn get_class_method_count(&self, class_name: &str) -> usize {
        self.class_to_methods
            .get(class_name)
            .map(|m| m.len())
            .unwrap_or(0)
    }

    pub fn get_methods_for_class(&self, class_name: &str) -> Vec<&str> {
        self.class_to_methods
            .get(class_name)
            .map(|methods| methods.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    pub fn total_classes(&self) -> usize {
        self.class_to_methods.len()
    }
}

/// Trait for language-agnostic class ownership analysis
pub trait ClassOwnershipAnalyzer {
    fn analyze_file(&self, content: &str) -> Result<ClassOwnership, String>;
    fn language(&self) -> Language;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_ownership_creation() {
        let ownership = ClassOwnership::new(Language::Python);
        assert_eq!(ownership.language, Language::Python);
        assert_eq!(ownership.total_classes(), 0);
    }

    #[test]
    fn test_add_class_and_methods() {
        let mut ownership = ClassOwnership::new(Language::Python);
        ownership.add_class("UserManager".to_string(), (1, 10));
        ownership.add_method("UserManager", "create_user".to_string());
        ownership.add_method("UserManager", "delete_user".to_string());

        assert_eq!(ownership.total_classes(), 1);
        assert_eq!(ownership.get_class_method_count("UserManager"), 2);
        assert_eq!(
            ownership.get_methods_for_class("UserManager"),
            vec!["create_user", "delete_user"]
        );
    }

    #[test]
    fn test_standalone_functions() {
        let mut ownership = ClassOwnership::new(Language::Python);
        ownership.add_standalone_function("helper_func".to_string());
        ownership.add_standalone_function("main".to_string());

        assert_eq!(ownership.standalone_functions.len(), 2);
    }

    #[test]
    fn test_method_to_class_mapping() {
        let mut ownership = ClassOwnership::new(Language::Python);
        ownership.add_class("TestClass".to_string(), (1, 5));
        ownership.add_method("TestClass", "test_method".to_string());

        assert_eq!(
            ownership.method_to_class.get("test_method"),
            Some(&"TestClass".to_string())
        );
    }
}
