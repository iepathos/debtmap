use crate::core::ast::{
    Assignment, ClassDef, Expression, MethodDef, ModuleScopeAnalysis, Scope, SingletonInstance,
};
use rustpython_parser::ast;

pub struct PythonAstExtractor;

impl PythonAstExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Extract class decorators from ClassDef AST node
    pub fn extract_class_decorators(&self, class_def: &ast::StmtClassDef) -> Vec<String> {
        class_def
            .decorator_list
            .iter()
            .filter_map(|decorator| self.decorator_to_string(decorator))
            .collect()
    }

    /// Extract base classes from ClassDef AST node
    pub fn extract_base_classes(&self, class_def: &ast::StmtClassDef) -> Vec<String> {
        class_def
            .bases
            .iter()
            .filter_map(Self::expr_to_name)
            .collect()
    }

    /// Extract method decorators from FunctionDef AST node
    pub fn extract_method_decorators(&self, func_def: &ast::StmtFunctionDef) -> Vec<String> {
        func_def
            .decorator_list
            .iter()
            .filter_map(|decorator| self.decorator_to_string(decorator))
            .collect()
    }

    /// Check if method is abstract (has @abstractmethod decorator)
    pub fn is_abstract_method(&self, decorators: &[String]) -> bool {
        decorators.iter().any(|d| d == "abstractmethod")
    }

    /// Extract module-level assignments
    pub fn extract_module_assignments(&self, module: &ast::Mod) -> Vec<Assignment> {
        let mut assignments = Vec::new();

        if let ast::Mod::Module(mod_body) = module {
            for stmt in &mod_body.body {
                if let ast::Stmt::Assign(assign) = stmt {
                    for target in &assign.targets {
                        if let Some(name) = Self::expr_to_name(target) {
                            let value = self.classify_expression(&assign.value);
                            assignments.push(Assignment {
                                name,
                                value,
                                scope: Scope::Module,
                                line: assign.range.start().to_usize(),
                            });
                        }
                    }
                }
            }
        }

        assignments
    }

    /// Classify an expression (ClassInstantiation, FunctionCall, etc.)
    fn classify_expression(&self, expr: &ast::Expr) -> Expression {
        match expr {
            ast::Expr::Call(call) => {
                if let Some(name) = Self::expr_to_name(&call.func) {
                    // Check if it's a class instantiation (capitalized name)
                    if name
                        .chars()
                        .next()
                        .map(|c| c.is_uppercase())
                        .unwrap_or(false)
                    {
                        Expression::ClassInstantiation {
                            class_name: name,
                            args: call.args.iter().filter_map(Self::expr_to_name).collect(),
                        }
                    } else {
                        Expression::FunctionCall {
                            function_name: name,
                            args: call.args.iter().filter_map(Self::expr_to_name).collect(),
                        }
                    }
                } else {
                    Expression::Other
                }
            }
            ast::Expr::Name(name) => Expression::ClassReference {
                class_name: name.id.to_string(),
            },
            _ => Expression::Other,
        }
    }

    /// Convert decorator expression to string
    fn decorator_to_string(&self, expr: &ast::Expr) -> Option<String> {
        match expr {
            ast::Expr::Name(name) => Some(name.id.to_string()),
            ast::Expr::Attribute(attr) => {
                // Handle chained attributes like @abc.abstractmethod
                let base = Self::expr_to_name(&attr.value)?;
                Some(format!("{}.{}", base, attr.attr))
            }
            ast::Expr::Call(call) => {
                // Handle decorators with arguments like @dataclass(frozen=True)
                Self::expr_to_name(&call.func)
            }
            _ => None,
        }
    }

    /// Convert expression to name string
    fn expr_to_name(expr: &ast::Expr) -> Option<String> {
        match expr {
            ast::Expr::Name(name) => Some(name.id.to_string()),
            ast::Expr::Attribute(attr) => {
                let base = Self::expr_to_name(&attr.value)?;
                Some(format!("{}.{}", base, attr.attr))
            }
            _ => None,
        }
    }

    /// Identify singleton instances (module-level class instantiation)
    pub fn extract_singleton_instances(
        &self,
        assignments: &[Assignment],
    ) -> Vec<SingletonInstance> {
        assignments
            .iter()
            .filter_map(|assignment| {
                if let Expression::ClassInstantiation { class_name, .. } = &assignment.value {
                    if assignment.scope == Scope::Module {
                        return Some(SingletonInstance {
                            variable_name: assignment.name.clone(),
                            class_name: class_name.clone(),
                            line: assignment.line,
                        });
                    }
                }
                None
            })
            .collect()
    }

    /// Extract full class definition with decorators and base classes
    pub fn extract_class_definition(&self, class_def: &ast::StmtClassDef) -> ClassDef {
        let decorators = self.extract_class_decorators(class_def);
        let base_classes = self.extract_base_classes(class_def);
        let methods = self.extract_methods_from_class(&class_def.body);

        let is_abstract = base_classes
            .iter()
            .any(|b| b.contains("ABC") || b.contains("Protocol"))
            || methods.iter().any(|m| m.is_abstract);

        ClassDef {
            name: class_def.name.to_string(),
            base_classes,
            methods,
            is_abstract,
            decorators,
            line: class_def.range.start().to_usize(),
        }
    }

    /// Extract methods from class body
    fn extract_methods_from_class(&self, body: &[ast::Stmt]) -> Vec<MethodDef> {
        let mut methods = Vec::new();

        for stmt in body {
            if let ast::Stmt::FunctionDef(func_def) = stmt {
                let decorators = self.extract_method_decorators(func_def);
                let is_abstract = self.is_abstract_method(&decorators);

                methods.push(MethodDef {
                    name: func_def.name.to_string(),
                    is_abstract,
                    decorators,
                    overrides_base: false, // Will be determined by pattern detector
                    line: func_def.range.start().to_usize(),
                });
            }
        }

        methods
    }

    /// Extract all classes from module
    pub fn extract_classes(&self, module: &ast::Mod) -> Vec<ClassDef> {
        let mut classes = Vec::new();

        if let ast::Mod::Module(mod_body) = module {
            for stmt in &mod_body.body {
                if let ast::Stmt::ClassDef(class_def) = stmt {
                    classes.push(self.extract_class_definition(class_def));
                }
            }
        }

        classes
    }

    /// Extract complete module scope analysis
    pub fn extract_module_scope(&self, module: &ast::Mod) -> ModuleScopeAnalysis {
        let assignments = self.extract_module_assignments(module);
        let singleton_instances = self.extract_singleton_instances(&assignments);

        ModuleScopeAnalysis {
            assignments,
            singleton_instances,
        }
    }
}

impl Default for PythonAstExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_python_code(code: &str) -> ast::Mod {
        rustpython_parser::parse(code, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse Python code")
    }

    fn get_first_class(module: &ast::Mod) -> &ast::StmtClassDef {
        if let ast::Mod::Module(mod_body) = module {
            for stmt in &mod_body.body {
                if let ast::Stmt::ClassDef(class_def) = stmt {
                    return class_def;
                }
            }
        }
        panic!("No class found in module");
    }

    fn get_first_method(module: &ast::Mod) -> &ast::StmtFunctionDef {
        if let ast::Mod::Module(mod_body) = module {
            for stmt in &mod_body.body {
                if let ast::Stmt::ClassDef(class_def) = stmt {
                    for class_stmt in &class_def.body {
                        if let ast::Stmt::FunctionDef(func_def) = class_stmt {
                            return func_def;
                        }
                    }
                }
            }
        }
        panic!("No method found in module");
    }

    #[test]
    fn test_extract_class_decorators() {
        let code = r#"
@dataclass
@frozen
class Point:
    x: int
    y: int
        "#;

        let ast = parse_python_code(code);
        let extractor = PythonAstExtractor::new();
        let decorators = extractor.extract_class_decorators(get_first_class(&ast));

        assert_eq!(decorators, vec!["dataclass", "frozen"]);
    }

    #[test]
    fn test_extract_base_classes() {
        let code = r#"
class Observer(ABC, Protocol):
    pass
        "#;

        let ast = parse_python_code(code);
        let extractor = PythonAstExtractor::new();
        let bases = extractor.extract_base_classes(get_first_class(&ast));

        assert_eq!(bases, vec!["ABC", "Protocol"]);
    }

    #[test]
    fn test_extract_method_decorators() {
        let code = r#"
class Observer:
    @abstractmethod
    @property
    def on_event(self):
        pass
        "#;

        let ast = parse_python_code(code);
        let extractor = PythonAstExtractor::new();
        let decorators = extractor.extract_method_decorators(get_first_method(&ast));

        assert_eq!(decorators, vec!["abstractmethod", "property"]);
    }

    #[test]
    fn test_extract_module_assignments() {
        let code = r#"
manager = Manager()
config = load_config()
VALUE = 42
        "#;

        let ast = parse_python_code(code);
        let extractor = PythonAstExtractor::new();
        let assignments = extractor.extract_module_assignments(&ast);

        assert_eq!(assignments.len(), 3);
        assert_eq!(assignments[0].name, "manager");
        assert!(matches!(
            assignments[0].value,
            Expression::ClassInstantiation { .. }
        ));
    }

    #[test]
    fn test_extract_singleton_instances() {
        let code = r#"
class Manager:
    pass

manager = Manager()
        "#;

        let ast = parse_python_code(code);
        let extractor = PythonAstExtractor::new();
        let assignments = extractor.extract_module_assignments(&ast);
        let singletons = extractor.extract_singleton_instances(&assignments);

        assert_eq!(singletons.len(), 1);
        assert_eq!(singletons[0].variable_name, "manager");
        assert_eq!(singletons[0].class_name, "Manager");
    }

    #[test]
    fn test_is_abstract_method() {
        let extractor = PythonAstExtractor::new();

        let decorators = vec!["abstractmethod".to_string()];
        assert!(extractor.is_abstract_method(&decorators));

        let decorators = vec!["property".to_string()];
        assert!(!extractor.is_abstract_method(&decorators));
    }

    #[test]
    fn test_extract_class_definition() {
        let code = r#"
@dataclass
class Observer(ABC):
    @abstractmethod
    def on_event(self):
        pass
        "#;

        let ast = parse_python_code(code);
        let extractor = PythonAstExtractor::new();
        let class_def = extractor.extract_class_definition(get_first_class(&ast));

        assert_eq!(class_def.name, "Observer");
        assert_eq!(class_def.decorators, vec!["dataclass"]);
        assert_eq!(class_def.base_classes, vec!["ABC"]);
        assert!(class_def.is_abstract);
        assert_eq!(class_def.methods.len(), 1);
        assert_eq!(class_def.methods[0].name, "on_event");
        assert!(class_def.methods[0].is_abstract);
    }

    #[test]
    fn test_extract_classes() {
        let code = r#"
class First:
    pass

class Second:
    pass
        "#;

        let ast = parse_python_code(code);
        let extractor = PythonAstExtractor::new();
        let classes = extractor.extract_classes(&ast);

        assert_eq!(classes.len(), 2);
        assert_eq!(classes[0].name, "First");
        assert_eq!(classes[1].name, "Second");
    }

    #[test]
    fn test_expression_is_class_instantiation() {
        let expr = Expression::ClassInstantiation {
            class_name: "Test".to_string(),
            args: vec![],
        };
        assert!(expr.is_class_instantiation());
        assert!(!expr.is_class_reference());
    }

    #[test]
    fn test_expression_is_class_reference() {
        let expr = Expression::ClassReference {
            class_name: "Test".to_string(),
        };
        assert!(expr.is_class_reference());
        assert!(!expr.is_class_instantiation());
    }

    #[test]
    fn test_classify_expression_function_call() {
        let code = r#"
result = process_data()
        "#;

        let ast = parse_python_code(code);
        let extractor = PythonAstExtractor::new();
        let assignments = extractor.extract_module_assignments(&ast);

        assert_eq!(assignments.len(), 1);
        assert!(matches!(
            assignments[0].value,
            Expression::FunctionCall { .. }
        ));
    }

    #[test]
    fn test_decorator_with_module() {
        let code = r#"
class Observer:
    @abc.abstractmethod
    def on_event(self):
        pass
        "#;

        let ast = parse_python_code(code);
        let extractor = PythonAstExtractor::new();
        let decorators = extractor.extract_method_decorators(get_first_method(&ast));

        assert_eq!(decorators, vec!["abc.abstractmethod"]);
    }

    #[test]
    fn test_extract_module_scope() {
        let code = r#"
class Manager:
    pass

manager = Manager()
config = load_config()
        "#;

        let ast = parse_python_code(code);
        let extractor = PythonAstExtractor::new();
        let module_scope = extractor.extract_module_scope(&ast);

        assert_eq!(module_scope.assignments.len(), 2);
        assert_eq!(module_scope.singleton_instances.len(), 1);
        assert_eq!(module_scope.singleton_instances[0].class_name, "Manager");
    }
}
