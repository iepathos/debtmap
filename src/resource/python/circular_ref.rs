/// Circular reference pattern detection for Python
use super::{
    AffectedScope, CircularPattern, ImpactLevel, PythonResourceDetector, PythonResourceIssueType,
    ResourceImpact, ResourceIssue, ResourceLocation, ResourceSeverity,
};
use rustpython_parser::ast::{self, Expr, Stmt};
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub struct PythonCircularRefDetector {
    max_depth: usize,
    _known_patterns: Vec<CircularPatternTemplate>,
}

struct CircularPatternTemplate {
    _pattern_name: String,
}

pub struct ClassInfo {
    _name: String,
    attributes: HashSet<String>,
    references: HashSet<String>,
    line: usize,
}

impl Default for PythonCircularRefDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonCircularRefDetector {
    pub fn new() -> Self {
        let known_patterns = vec![
            CircularPatternTemplate {
                _pattern_name: "self_reference".to_string(),
            },
            CircularPatternTemplate {
                _pattern_name: "parent_child_loop".to_string(),
            },
        ];

        Self {
            max_depth: 5,
            _known_patterns: known_patterns,
        }
    }

    fn analyze_classes(&self, module: &ast::Mod) -> HashMap<String, ClassInfo> {
        let mut classes = HashMap::new();

        if let ast::Mod::Module(module) = module {
            for stmt in &module.body {
                if let Stmt::ClassDef(class_def) = stmt {
                    let mut class_info = ClassInfo {
                        _name: class_def.name.to_string(),
                        attributes: HashSet::new(),
                        references: HashSet::new(),
                        line: 1, // TODO: Track actual line numbers
                    };

                    // Analyze class body for attributes and references
                    for class_stmt in &class_def.body {
                        self.analyze_class_statement(class_stmt, &mut class_info);
                    }

                    classes.insert(class_def.name.to_string(), class_info);
                }
            }
        }

        classes
    }

    fn analyze_class_statement(&self, stmt: &Stmt, class_info: &mut ClassInfo) {
        match stmt {
            Stmt::FunctionDef(func) => {
                // Check all methods, not just __init__
                for func_stmt in &func.body {
                    if &func.name == "__init__" {
                        self.analyze_init_statement(func_stmt, class_info);
                    } else {
                        // Analyze other methods for circular references
                        self.analyze_method_statement(func_stmt, class_info);
                    }
                }
            }
            Stmt::Assign(assign) => {
                // Check for class-level attribute assignments
                for target in &assign.targets {
                    if let Expr::Attribute(attr) = target {
                        if let Expr::Name(name) = attr.value.as_ref() {
                            if &name.id == "self" {
                                class_info.attributes.insert(attr.attr.to_string());
                            }
                        }
                    }
                }

                // Check for references to other classes
                self.extract_class_references(&assign.value, &mut class_info.references);
            }
            _ => {}
        }
    }

    fn analyze_init_statement(&self, stmt: &Stmt, class_info: &mut ClassInfo) {
        if let Stmt::Assign(assign) = stmt {
            for target in &assign.targets {
                if let Expr::Attribute(attr) = target {
                    if let Expr::Name(name) = attr.value.as_ref() {
                        if &name.id == "self" {
                            class_info.attributes.insert(attr.attr.to_string());

                            // Check if the value references another class
                            self.extract_class_references(
                                &assign.value,
                                &mut class_info.references,
                            );
                        }
                    }
                }
            }
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn extract_class_references(&self, expr: &Expr, references: &mut HashSet<String>) {
        match expr {
            Expr::Name(name) => {
                // Simple class reference or self reference
                let name_str = name.id.to_string();
                if name_str == "self" {
                    // Track self references
                    references.insert("<self>".to_string());
                } else if name_str.chars().next().is_some_and(|c| c.is_uppercase()) {
                    references.insert(name_str);
                }
            }
            Expr::Call(call) => {
                // Constructor call
                if let Expr::Name(name) = call.func.as_ref() {
                    let name_str = name.id.to_string();
                    if name_str.chars().next().is_some_and(|c| c.is_uppercase()) {
                        references.insert(name_str);
                    }
                }

                // Check arguments for nested references
                for arg in &call.args {
                    self.extract_class_references(arg, references);
                }
            }
            Expr::Attribute(attr) => {
                // Check for references through attributes
                self.extract_class_references(attr.value.as_ref(), references);
            }
            _ => {}
        }
    }

    fn check_for_circular_pattern(&self, stmt: &Stmt, class_name: &str) -> Option<ResourceIssue> {
        // Check for specific patterns like child.children.append(self)
        if let Stmt::Expr(expr_stmt) = stmt {
            if let Expr::Call(call) = expr_stmt.value.as_ref() {
                if let Expr::Attribute(attr) = call.func.as_ref() {
                    let method = attr.attr.to_string();
                    if method == "append" || method == "add" || method == "insert" {
                        // Check if self is being appended
                        for arg in &call.args {
                            if let Expr::Name(name) = arg {
                                if &name.id == "self" {
                                    return Some(ResourceIssue {
                                        issue_type: PythonResourceIssueType::CircularReference {
                                            classes_involved: vec![class_name.to_string()],
                                            pattern: CircularPattern::SelfReference,
                                        },
                                        severity: ResourceSeverity::High,
                                        location: ResourceLocation {
                                            line: 1,
                                            column: 0,
                                            end_line: None,
                                            end_column: None,
                                        },
                                        suggestion: "Circular reference detected: 'self' is added to a collection. Use weak references to prevent memory leaks.".to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn detect_circular_references(
        &self,
        classes: &HashMap<String, ClassInfo>,
    ) -> Vec<ResourceIssue> {
        let mut issues = Vec::new();

        // Check for direct self-references
        for (class_name, class_info) in classes {
            if class_info.references.contains(class_name)
                || class_info.references.contains("<self>")
            {
                issues.push(ResourceIssue {
                    issue_type: PythonResourceIssueType::CircularReference {
                        classes_involved: vec![class_name.clone()],
                        pattern: CircularPattern::SelfReference,
                    },
                    severity: ResourceSeverity::High,
                    location: ResourceLocation {
                        line: class_info.line,
                        column: 0,
                        end_line: None,
                        end_column: None,
                    },
                    suggestion: format!(
                        "Class '{}' has self-reference. Consider using weak references or refactoring.",
                        class_name
                    ),
                });
            }
        }

        // Check for mutual references
        for (class1, info1) in classes {
            for class2 in &info1.references {
                if let Some(info2) = classes.get(class2) {
                    if info2.references.contains(class1) && class1 != class2 {
                        // Found mutual reference
                        let mut classes_involved = vec![class1.clone(), class2.clone()];
                        classes_involved.sort();

                        // Avoid duplicate detection
                        if class1 < class2 {
                            issues.push(ResourceIssue {
                                issue_type: PythonResourceIssueType::CircularReference {
                                    classes_involved,
                                    pattern: CircularPattern::MutualReference,
                                },
                                severity: ResourceSeverity::High,
                                location: ResourceLocation {
                                    line: info1.line,
                                    column: 0,
                                    end_line: None,
                                    end_column: None,
                                },
                                suggestion: format!(
                                    "Classes '{}' and '{}' have mutual references. Consider using weak references.",
                                    class1, class2
                                ),
                            });
                        }
                    }
                }
            }
        }

        // Check for chain references
        for start_class in classes.keys() {
            if let Some(chain) =
                self.find_reference_chain(start_class, classes, &mut HashSet::new(), 0)
            {
                if chain.len() > 2 {
                    issues.push(ResourceIssue {
                        issue_type: PythonResourceIssueType::CircularReference {
                            classes_involved: chain.clone(),
                            pattern: CircularPattern::ChainReference(chain.len()),
                        },
                        severity: ResourceSeverity::Medium,
                        location: ResourceLocation {
                            line: classes[start_class].line,
                            column: 0,
                            end_line: None,
                            end_column: None,
                        },
                        suggestion: format!(
                            "Circular reference chain detected: {}. Break the cycle with weak references.",
                            chain.join(" -> ")
                        ),
                    });
                }
            }
        }

        issues
    }

    fn find_reference_chain(
        &self,
        current: &str,
        classes: &HashMap<String, ClassInfo>,
        visited: &mut HashSet<String>,
        depth: usize,
    ) -> Option<Vec<String>> {
        if depth > self.max_depth {
            return None;
        }

        if visited.contains(current) {
            // Found a cycle
            return Some(vec![current.to_string()]);
        }

        visited.insert(current.to_string());

        if let Some(info) = classes.get(current) {
            for referenced in &info.references {
                if let Some(mut chain) =
                    self.find_reference_chain(referenced, classes, visited, depth + 1)
                {
                    chain.insert(0, current.to_string());
                    return Some(chain);
                }
            }
        }

        visited.remove(current);
        None
    }

    fn analyze_method_statement(&self, stmt: &Stmt, class_info: &mut ClassInfo) {
        // Analyze non-__init__ methods for circular references
        match stmt {
            Stmt::Expr(expr_stmt) => {
                // Check for method calls that might create circular references
                if let Expr::Call(call) = expr_stmt.value.as_ref() {
                    if let Expr::Attribute(attr) = call.func.as_ref() {
                        // Check for patterns like child.children.append(self)
                        if &attr.attr == "append" || &attr.attr == "add" || &attr.attr == "insert" {
                            for arg in &call.args {
                                if let Expr::Name(name) = arg {
                                    if &name.id == "self" {
                                        // Found self being added to something
                                        class_info.references.insert("<self>".to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Stmt::Assign(assign) => {
                // Check for assignments that create circular references
                for target in &assign.targets {
                    if let Expr::Attribute(_attr) = target {
                        // Check if assigning self to an attribute
                        if let Expr::Name(name) = assign.value.as_ref() {
                            if &name.id == "self" {
                                class_info.references.insert("<self>".to_string());
                            }
                        }
                    }
                }

                // Also extract any class references
                self.extract_class_references(&assign.value, &mut class_info.references);
            }
            _ => {}
        }
    }
}

impl PythonResourceDetector for PythonCircularRefDetector {
    fn detect_issues(&self, module: &ast::Mod, _path: &Path) -> Vec<ResourceIssue> {
        let classes = self.analyze_classes(module);
        let mut issues = self.detect_circular_references(&classes);

        // Also check for direct circular patterns in methods
        if let ast::Mod::Module(module) = module {
            for stmt in &module.body {
                if let Stmt::ClassDef(class_def) = stmt {
                    // Check for circular reference patterns
                    for class_stmt in &class_def.body {
                        if let Stmt::FunctionDef(func) = class_stmt {
                            for func_stmt in &func.body {
                                if let Some(issue) =
                                    self.check_for_circular_pattern(func_stmt, &class_def.name)
                                {
                                    issues.push(issue);
                                }
                            }
                        }
                    }
                }
            }
        }

        issues
    }

    fn assess_resource_impact(&self, issue: &ResourceIssue) -> ResourceImpact {
        let impact_level = match &issue.issue_type {
            PythonResourceIssueType::CircularReference { pattern, .. } => match pattern {
                CircularPattern::SelfReference => ImpactLevel::High,
                CircularPattern::MutualReference => ImpactLevel::High,
                CircularPattern::ChainReference(len) if *len > 3 => ImpactLevel::High,
                CircularPattern::ChainReference(_) => ImpactLevel::Medium,
                CircularPattern::CallbackLoop => ImpactLevel::High,
            },
            _ => ImpactLevel::Medium,
        };

        ResourceImpact {
            impact_level,
            affected_scope: AffectedScope::Module,
            estimated_severity: match impact_level {
                ImpactLevel::Critical => 1.0,
                ImpactLevel::High => 0.8,
                ImpactLevel::Medium => 0.5,
                ImpactLevel::Low => 0.3,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustpython_parser::{parse, Mode};

    fn parse_python_code(code: &str) -> ast::Mod {
        parse(code, Mode::Module, "test.py").unwrap()
    }

    #[test]
    fn test_detect_self_reference() {
        let code = r#"
class Node:
    def __init__(self, value):
        self.value = value
        self.child = self  # Self-reference
        "#;

        let module = parse_python_code(code);
        let detector = PythonCircularRefDetector::new();
        let issues = detector.detect_issues(&module, Path::new("test.py"));

        assert!(!issues.is_empty());
        if let PythonResourceIssueType::CircularReference { pattern, .. } = &issues[0].issue_type {
            assert_eq!(*pattern, CircularPattern::SelfReference);
        } else {
            panic!("Expected circular reference issue");
        }
    }

    #[test]
    fn test_detect_mutual_reference() {
        let code = r#"
class Parent:
    def __init__(self):
        self.child = Child(self)

class Child:
    def __init__(self, parent):
        self.parent = parent
        "#;

        let module = parse_python_code(code);
        let detector = PythonCircularRefDetector::new();
        let issues = detector.detect_issues(&module, Path::new("test.py"));

        // Should detect circular reference pattern
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_analyze_classes() {
        let code = r#"
class TestClass:
    def __init__(self):
        self.attr1 = "value"
        self.attr2 = 42

    def method1(self):
        self.attr3 = "another"
        "#;

        let module = parse_python_code(code);
        let detector = PythonCircularRefDetector::new();
        let classes = detector.analyze_classes(&module);

        assert_eq!(classes.len(), 1);
        assert!(classes.contains_key("TestClass"));

        let test_class = &classes["TestClass"];
        assert!(!test_class.attributes.is_empty());
    }

    #[test]
    fn test_no_circular_reference() {
        let code = r#"
class SimpleClass:
    def __init__(self, value):
        self.value = value
        self.data = []
        "#;

        let module = parse_python_code(code);
        let detector = PythonCircularRefDetector::new();
        let issues = detector.detect_issues(&module, Path::new("test.py"));

        // Should not detect any circular references
        let circular_issues: Vec<_> = issues
            .into_iter()
            .filter(|issue| {
                matches!(
                    issue.issue_type,
                    PythonResourceIssueType::CircularReference { .. }
                )
            })
            .collect();
        assert!(circular_issues.is_empty());
    }

    #[test]
    fn test_assess_resource_impact() {
        let detector = PythonCircularRefDetector::new();

        let issue = ResourceIssue {
            issue_type: PythonResourceIssueType::CircularReference {
                pattern: CircularPattern::SelfReference,
                classes_involved: vec!["Node".to_string()],
            },
            location: ResourceLocation {
                line: 5,
                column: 10,
                end_line: None,
                end_column: None,
            },
            severity: ResourceSeverity::High,
            suggestion: "Avoid self-reference to prevent potential memory issues".to_string(),
        };

        let impact = detector.assess_resource_impact(&issue);
        assert_eq!(impact.impact_level, ImpactLevel::High);
        assert_eq!(impact.estimated_severity, 0.8);
    }

    #[test]
    fn test_chain_reference_impact() {
        let detector = PythonCircularRefDetector::new();

        let short_chain_issue = ResourceIssue {
            issue_type: PythonResourceIssueType::CircularReference {
                pattern: CircularPattern::ChainReference(2),
                classes_involved: vec!["ClassA".to_string(), "ClassB".to_string()],
            },
            location: ResourceLocation {
                line: 5,
                column: 10,
                end_line: None,
                end_column: None,
            },
            severity: ResourceSeverity::Medium,
            suggestion: "Short chain reference found".to_string(),
        };

        let long_chain_issue = ResourceIssue {
            issue_type: PythonResourceIssueType::CircularReference {
                pattern: CircularPattern::ChainReference(5),
                classes_involved: vec![
                    "ClassA".to_string(),
                    "ClassB".to_string(),
                    "ClassC".to_string(),
                ],
            },
            location: ResourceLocation {
                line: 5,
                column: 10,
                end_line: None,
                end_column: None,
            },
            severity: ResourceSeverity::High,
            suggestion: "Long chain reference found".to_string(),
        };

        let short_impact = detector.assess_resource_impact(&short_chain_issue);
        let long_impact = detector.assess_resource_impact(&long_chain_issue);

        assert_eq!(short_impact.impact_level, ImpactLevel::Medium);
        assert_eq!(long_impact.impact_level, ImpactLevel::High);
    }

    #[test]
    fn test_callback_loop_pattern() {
        let code = r#"
class EventHandler:
    def __init__(self):
        self.callback = self.handle_event

    def handle_event(self, event):
        self.callback(event)  # Potential infinite loop
        "#;

        let module = parse_python_code(code);
        let detector = PythonCircularRefDetector::new();
        let issues = detector.detect_issues(&module, Path::new("test.py"));

        // Should detect potential callback loop
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_complex_inheritance_scenario() {
        let code = r#"
class Base:
    def __init__(self):
        self.derived_ref = None

class Derived(Base):
    def __init__(self):
        super().__init__()
        self.derived_ref = self  # Self-reference through inheritance
        "#;

        let module = parse_python_code(code);
        let detector = PythonCircularRefDetector::new();
        let issues = detector.detect_issues(&module, Path::new("test.py"));

        // Should detect issues in inheritance scenarios
        assert!(!issues.is_empty());
    }
}
