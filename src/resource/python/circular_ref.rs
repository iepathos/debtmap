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
    known_patterns: Vec<CircularPatternTemplate>,
}

struct CircularPatternTemplate {
    pattern_name: String,
}

pub struct ClassInfo {
    name: String,
    attributes: HashSet<String>,
    references: HashSet<String>,
    line: usize,
}

impl PythonCircularRefDetector {
    pub fn new() -> Self {
        let known_patterns = vec![
            CircularPatternTemplate {
                pattern_name: "self_reference".to_string(),
            },
            CircularPatternTemplate {
                pattern_name: "parent_child_loop".to_string(),
            },
        ];

        Self {
            max_depth: 5,
            known_patterns,
        }
    }

    fn analyze_classes(&self, module: &ast::Mod) -> HashMap<String, ClassInfo> {
        let mut classes = HashMap::new();

        if let ast::Mod::Module(module) = module {
            for stmt in &module.body {
                if let Stmt::ClassDef(class_def) = stmt {
                    let mut class_info = ClassInfo {
                        name: class_def.name.to_string(),
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
                // Check __init__ for attribute assignments
                if &func.name == "__init__" {
                    for func_stmt in &func.body {
                        self.analyze_init_statement(func_stmt, class_info);
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

    fn extract_class_references(&self, expr: &Expr, references: &mut HashSet<String>) {
        match expr {
            Expr::Name(name) => {
                // Simple class reference
                let name_str = name.id.to_string();
                if name_str.chars().next().map_or(false, |c| c.is_uppercase()) {
                    references.insert(name_str);
                }
            }
            Expr::Call(call) => {
                // Constructor call
                if let Expr::Name(name) = call.func.as_ref() {
                    let name_str = name.id.to_string();
                    if name_str.chars().next().map_or(false, |c| c.is_uppercase()) {
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

    fn detect_circular_references(
        &self,
        classes: &HashMap<String, ClassInfo>,
    ) -> Vec<ResourceIssue> {
        let mut issues = Vec::new();

        // Check for direct self-references
        for (class_name, class_info) in classes {
            if class_info.references.contains(class_name) {
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
        for (start_class, _) in classes {
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
}

impl PythonResourceDetector for PythonCircularRefDetector {
    fn detect_issues(&self, module: &ast::Mod, _path: &Path) -> Vec<ResourceIssue> {
        let classes = self.analyze_classes(module);
        self.detect_circular_references(&classes)
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
