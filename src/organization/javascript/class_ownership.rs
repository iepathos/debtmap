use crate::organization::class_ownership::{ClassOwnership, ClassOwnershipAnalyzer};
use crate::organization::language::Language;
use tree_sitter::{Node, Parser};

pub struct JavaScriptClassAnalyzer {
    language_variant: Language,
}

impl JavaScriptClassAnalyzer {
    pub fn new(language: Language) -> Result<Self, String> {
        match language {
            Language::JavaScript | Language::TypeScript => Ok(Self {
                language_variant: language,
            }),
            _ => Err(format!(
                "Invalid language for JavaScriptClassAnalyzer: {:?}",
                language
            )),
        }
    }

    fn create_parser(language: Language) -> Result<Parser, String> {
        let mut parser = Parser::new();

        match language {
            Language::JavaScript => {
                parser
                    .set_language(&tree_sitter_javascript::LANGUAGE.into())
                    .map_err(|e| format!("Failed to set JavaScript parser language: {}", e))?;
            }
            Language::TypeScript => {
                parser
                    .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
                    .map_err(|e| format!("Failed to set TypeScript parser language: {}", e))?;
            }
            _ => {
                return Err(format!("Unsupported language: {:?}", language));
            }
        }

        Ok(parser)
    }

    fn walk_tree(node: &Node, source: &[u8], ownership: &mut ClassOwnership) {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "class_declaration" => {
                    Self::analyze_class(&child, source, ownership);
                }
                "function_declaration" | "arrow_function" | "function" => {
                    // Only top-level functions are standalone
                    if child.parent().is_some_and(|p| p.kind() == "program") {
                        if let Some(name_node) = child.child_by_field_name("name") {
                            if let Ok(name) = name_node.utf8_text(source) {
                                ownership.add_standalone_function(name.to_string());
                            }
                        }
                    }
                }
                _ => {
                    Self::walk_tree(&child, source, ownership);
                }
            }
        }
    }

    fn analyze_class(node: &Node, source: &[u8], ownership: &mut ClassOwnership) {
        let class_name = if let Some(name_node) = node.child_by_field_name("name") {
            name_node.utf8_text(source).unwrap_or("Unknown").to_string()
        } else {
            return;
        };

        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;

        ownership.add_class(class_name.clone(), (start_line, end_line));

        if let Some(body_node) = node.child_by_field_name("body") {
            for child in body_node.children(&mut body_node.walk()) {
                if child.kind() == "method_definition" || child.kind() == "field_definition" {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        if let Ok(method_name) = name_node.utf8_text(source) {
                            // Skip constructor
                            if method_name == "constructor" {
                                continue;
                            }

                            ownership.add_method(&class_name, method_name.to_string());
                        }
                    }
                }
            }
        }
    }
}

impl Default for JavaScriptClassAnalyzer {
    fn default() -> Self {
        Self::new(Language::JavaScript).expect("Failed to create default JavaScriptClassAnalyzer")
    }
}

impl ClassOwnershipAnalyzer for JavaScriptClassAnalyzer {
    fn analyze_file(&self, content: &str) -> Result<ClassOwnership, String> {
        let mut parser = Self::create_parser(self.language_variant)?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| "Failed to parse file".to_string())?;

        let mut ownership = ClassOwnership::new(self.language_variant);
        let root = tree.root_node();

        Self::walk_tree(&root, content.as_bytes(), &mut ownership);

        Ok(ownership)
    }

    fn language(&self) -> Language {
        self.language_variant
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_javascript_class_ownership_basic() {
        let code = r#"
class UserController {
    constructor() {
        this.users = [];
    }

    createUser(data) {
        // ...
    }

    deleteUser(userId) {
        // ...
    }
}

function standaloneFunction() {
    // ...
}
        "#;

        let analyzer = JavaScriptClassAnalyzer::new(Language::JavaScript).unwrap();
        let ownership = analyzer.analyze_file(code).unwrap();

        assert_eq!(ownership.language, Language::JavaScript);
        assert_eq!(ownership.total_classes(), 1);
        assert_eq!(ownership.get_class_method_count("UserController"), 2);

        let methods = ownership.get_methods_for_class("UserController");
        assert!(methods.contains(&"createUser"));
        assert!(methods.contains(&"deleteUser"));
        assert!(!methods.contains(&"constructor")); // Should be excluded

        assert_eq!(ownership.standalone_functions.len(), 1);
        assert!(ownership
            .standalone_functions
            .contains(&"standaloneFunction".to_string()));
    }

    #[test]
    fn test_typescript_class_ownership() {
        let code = r#"
class UserService {
    private users: User[] = [];

    constructor(private config: Config) {}

    public createUser(data: UserData): User {
        // ...
    }

    public async deleteUser(userId: string): Promise<void> {
        // ...
    }

    private validateUser(user: User): boolean {
        // ...
    }
}
        "#;

        let analyzer = JavaScriptClassAnalyzer::new(Language::TypeScript).unwrap();
        let ownership = analyzer.analyze_file(code).unwrap();

        assert_eq!(ownership.language, Language::TypeScript);
        assert_eq!(ownership.total_classes(), 1);
        assert_eq!(ownership.get_class_method_count("UserService"), 3);

        let methods = ownership.get_methods_for_class("UserService");
        assert!(methods.contains(&"createUser"));
        assert!(methods.contains(&"deleteUser"));
        assert!(methods.contains(&"validateUser"));
    }

    #[test]
    fn test_javascript_multiple_classes() {
        let code = r#"
class UserRepository {
    save(user) {}
    find(id) {}
}

class SessionManager {
    createSession(userId) {}
    invalidateSession(sessionId) {}
}
        "#;

        let analyzer = JavaScriptClassAnalyzer::new(Language::JavaScript).unwrap();
        let ownership = analyzer.analyze_file(code).unwrap();

        assert_eq!(ownership.total_classes(), 2);
        assert_eq!(ownership.get_class_method_count("UserRepository"), 2);
        assert_eq!(ownership.get_class_method_count("SessionManager"), 2);
    }

    #[test]
    fn test_method_to_class_mapping() {
        let code = r#"
class UserService {
    createUser() {}
}

class ProductService {
    createProduct() {}
}
        "#;

        let analyzer = JavaScriptClassAnalyzer::new(Language::JavaScript).unwrap();
        let ownership = analyzer.analyze_file(code).unwrap();

        assert_eq!(
            ownership.method_to_class.get("createUser"),
            Some(&"UserService".to_string())
        );
        assert_eq!(
            ownership.method_to_class.get("createProduct"),
            Some(&"ProductService".to_string())
        );
    }

    #[test]
    fn test_class_locations() {
        let code = r#"class MyClass {
    method1() {}
}"#;

        let analyzer = JavaScriptClassAnalyzer::new(Language::JavaScript).unwrap();
        let ownership = analyzer.analyze_file(code).unwrap();

        assert!(ownership.class_locations.contains_key("MyClass"));
    }

    #[test]
    fn test_invalid_language() {
        let result = JavaScriptClassAnalyzer::new(Language::Rust);
        assert!(result.is_err());
    }

    #[test]
    fn test_typescript_static_methods() {
        let code = r#"
class Utils {
    static getInstance() {}
    static formatDate() {}
    regularMethod() {}
}
        "#;

        let analyzer = JavaScriptClassAnalyzer::new(Language::TypeScript).unwrap();
        let ownership = analyzer.analyze_file(code).unwrap();

        assert_eq!(ownership.get_class_method_count("Utils"), 3);
    }

    #[test]
    fn test_typescript_getters_setters() {
        let code = r#"
class Person {
    get name() {}
    set name(value) {}
    greet() {}
}
        "#;

        let analyzer = JavaScriptClassAnalyzer::new(Language::TypeScript).unwrap();
        let ownership = analyzer.analyze_file(code).unwrap();

        // Getters/setters should be counted as methods
        assert!(ownership.get_class_method_count("Person") >= 1);
    }
}
