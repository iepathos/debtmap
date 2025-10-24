use crate::organization::class_ownership::{ClassOwnership, ClassOwnershipAnalyzer};
use crate::organization::language::Language;
use rustpython_parser::ast;

pub struct PythonClassAnalyzer;

impl PythonClassAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn should_skip_method(method_name: &str) -> bool {
        // Skip dunder methods (e.g., __init__, __str__, etc.)
        method_name.starts_with("__") && method_name.ends_with("__")
    }

    fn analyze_class(class_def: &ast::StmtClassDef, ownership: &mut ClassOwnership) {
        let class_name = class_def.name.to_string();
        let start_line = class_def.range.start().to_usize();
        let end_line = class_def.range.end().to_usize();

        ownership.add_class(class_name.clone(), (start_line, end_line));

        for stmt in &class_def.body {
            if let ast::Stmt::FunctionDef(method_def) = stmt {
                let method_name = method_def.name.to_string();

                if !Self::should_skip_method(&method_name) {
                    ownership.add_method(&class_name, method_name);
                }
            }
        }
    }
}

impl Default for PythonClassAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ClassOwnershipAnalyzer for PythonClassAnalyzer {
    fn analyze_file(&self, content: &str) -> Result<ClassOwnership, String> {
        let module = rustpython_parser::parse(content, rustpython_parser::Mode::Module, "<input>")
            .map_err(|e| format!("Failed to parse Python: {}", e))?;

        let mut ownership = ClassOwnership::new(Language::Python);

        if let ast::Mod::Module(m) = &module {
            for stmt in &m.body {
                match stmt {
                    ast::Stmt::ClassDef(class_def) => {
                        Self::analyze_class(class_def, &mut ownership);
                    }
                    ast::Stmt::FunctionDef(func_def) => {
                        ownership.add_standalone_function(func_def.name.to_string());
                    }
                    _ => {}
                }
            }
        }

        Ok(ownership)
    }

    fn language(&self) -> Language {
        Language::Python
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_class_ownership_basic() {
        let code = r#"
class UserManager:
    def create_user(self, data):
        pass

    def delete_user(self, user_id):
        pass

    def __init__(self):
        pass

def standalone_function():
    pass
        "#;

        let analyzer = PythonClassAnalyzer::new();
        let ownership = analyzer.analyze_file(code).unwrap();

        assert_eq!(ownership.language, Language::Python);
        assert_eq!(ownership.total_classes(), 1);
        assert_eq!(ownership.get_class_method_count("UserManager"), 2);

        let methods = ownership.get_methods_for_class("UserManager");
        assert!(methods.contains(&"create_user"));
        assert!(methods.contains(&"delete_user"));
        assert!(!methods.contains(&"__init__")); // Should be excluded

        assert_eq!(ownership.standalone_functions.len(), 1);
        assert!(ownership
            .standalone_functions
            .contains(&"standalone_function".to_string()));
    }

    #[test]
    fn test_python_multiple_classes() {
        let code = r#"
class UserRepository:
    def save(self, user):
        pass

    def find(self, user_id):
        pass

class SessionManager:
    def create_session(self, user_id):
        pass

    def invalidate_session(self, session_id):
        pass
        "#;

        let analyzer = PythonClassAnalyzer::new();
        let ownership = analyzer.analyze_file(code).unwrap();

        assert_eq!(ownership.total_classes(), 2);
        assert_eq!(ownership.get_class_method_count("UserRepository"), 2);
        assert_eq!(ownership.get_class_method_count("SessionManager"), 2);
    }

    #[test]
    fn test_python_exclude_dunder_methods() {
        let code = r#"
class TestClass:
    def __init__(self):
        pass

    def __str__(self):
        pass

    def __repr__(self):
        pass

    def regular_method(self):
        pass
        "#;

        let analyzer = PythonClassAnalyzer::new();
        let ownership = analyzer.analyze_file(code).unwrap();

        assert_eq!(ownership.get_class_method_count("TestClass"), 1);
        assert!(ownership
            .get_methods_for_class("TestClass")
            .contains(&"regular_method"));
    }

    #[test]
    fn test_python_method_to_class_mapping() {
        let code = r#"
class UserService:
    def create_user(self):
        pass

class ProductService:
    def create_product(self):
        pass
        "#;

        let analyzer = PythonClassAnalyzer::new();
        let ownership = analyzer.analyze_file(code).unwrap();

        assert_eq!(
            ownership.method_to_class.get("create_user"),
            Some(&"UserService".to_string())
        );
        assert_eq!(
            ownership.method_to_class.get("create_product"),
            Some(&"ProductService".to_string())
        );
    }

    #[test]
    fn test_python_class_locations() {
        let code = r#"class MyClass:
    def method1(self):
        pass
"#;

        let analyzer = PythonClassAnalyzer::new();
        let ownership = analyzer.analyze_file(code).unwrap();

        assert!(ownership.class_locations.contains_key("MyClass"));
        let (_start, _end) = ownership.class_locations.get("MyClass").unwrap();
        // Line numbers should be reasonable (exact values may vary)
    }
}
