//! Python purity analysis
//!
//! This module provides purity detection for Python functions.

use crate::extraction::{PurityAnalysisData, PurityLevel};
use std::collections::HashSet;
use tree_sitter::Node;

/// Python purity analyzer
pub struct PythonPurityAnalyzer<'a> {
    source: &'a str,
    local_vars: HashSet<String>,
    params: HashSet<String>,
}

impl<'a> PythonPurityAnalyzer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            local_vars: HashSet::new(),
            params: HashSet::new(),
        }
    }

    pub fn analyze(node: &Node, source: &'a str, params: Vec<String>) -> PurityAnalysisData {
        let mut analyzer = Self::new(source);
        analyzer.params = params.into_iter().collect();
        analyzer.analyze_node(node)
    }

    fn analyze_node(&mut self, node: &Node) -> PurityAnalysisData {
        let mut reasons = Vec::new();
        let mut level = PurityLevel::StrictlyPure;
        let mut has_io = false;

        self.collect_locals(node);
        self.find_impurities(node, &mut reasons, &mut level, &mut has_io);

        let is_pure = reasons.is_empty();
        let confidence = if is_pure { 0.8 } else { 0.9 };

        PurityAnalysisData {
            is_pure,
            has_mutations: !is_pure && !has_io,
            has_io_operations: has_io,
            has_unsafe: false,
            local_mutations: if level == PurityLevel::LocallyPure {
                vec!["local".to_string()]
            } else {
                vec![]
            },
            upvalue_mutations: reasons,
            total_mutations: if is_pure { 0 } else { 1 },
            var_names: std::collections::HashMap::new(),
            confidence,
            purity_level: level,
        }
    }

    fn collect_locals(&mut self, node: &Node) {
        let kind = node.kind();
        if kind == "assignment" {
            if let Some(left) = node.child_by_field_name("left") {
                if left.kind() == "identifier" {
                    let name = &self.source[left.start_byte()..left.end_byte()];
                    if !self.params.contains(name) {
                        self.local_vars.insert(name.to_string());
                    }
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_locals(&child);
        }
    }

    fn find_impurities(
        &self,
        node: &Node,
        reasons: &mut Vec<String>,
        level: &mut PurityLevel,
        has_io: &mut bool,
    ) {
        let kind = node.kind();
        // println!("Node kind: {}, text: {}", kind, &self.source[node.start_byte()..node.end_byte()]);

        match kind {
            "assignment" => {
                if let Some(left) = node.child_by_field_name("left") {
                    match left.kind() {
                        "attribute" => {
                            let object = left.child_by_field_name("object").unwrap();
                            let obj_text = &self.source[object.start_byte()..object.end_byte()];
                            if obj_text == "self" {
                                reasons.push("Mutation of 'self' attribute".to_string());
                                *level = PurityLevel::Impure;
                            } else if self.params.contains(obj_text) {
                                reasons
                                    .push(format!("Mutation of parameter attribute: {}", obj_text));
                                *level = PurityLevel::Impure;
                            } else if !self.local_vars.contains(obj_text) {
                                reasons.push(format!("Mutation of external object: {}", obj_text));
                                *level = PurityLevel::Impure;
                            } else if *level == PurityLevel::StrictlyPure {
                                *level = PurityLevel::LocallyPure;
                            }
                        }
                        "identifier" => {
                            let name = &self.source[left.start_byte()..left.end_byte()];
                            if !self.local_vars.contains(name) && !self.params.contains(name) {
                                reasons.push(format!(
                                    "Mutation of global/external variable: {}",
                                    name
                                ));
                                *level = PurityLevel::Impure;
                            } else if *level == PurityLevel::StrictlyPure {
                                *level = PurityLevel::LocallyPure;
                            }
                        }

                        _ => {}
                    }
                }
            }
            "call" => {
                if let Some(func) = node.child_by_field_name("function") {
                    let func_name = if func.kind() == "attribute" {
                        if let Some(attr) = func.child_by_field_name("attribute") {
                            &self.source[attr.start_byte()..attr.end_byte()]
                        } else {
                            ""
                        }
                    } else {
                        &self.source[func.start_byte()..func.end_byte()]
                    };

                    if is_io_function(func_name) {
                        reasons.push(format!("I/O call: {}", func_name));
                        *level = PurityLevel::Impure;
                        *has_io = true;
                    } else if is_mutation_method(func_name) {
                        // Check if it's called on a non-local object
                        if func.kind() == "attribute" {
                            let object = func.child_by_field_name("object").unwrap();
                            let obj_text = &self.source[object.start_byte()..object.end_byte()];
                            if !self.local_vars.contains(obj_text) {
                                reasons.push(format!(
                                    "Mutation method '{}' on non-local: {}",
                                    func_name, obj_text
                                ));
                                *level = PurityLevel::Impure;
                            }
                        } else {
                            // Direct call like append(x) - might be a global or builtin
                            reasons.push(format!(
                                "Direct call to mutation-named function: {}",
                                func_name
                            ));
                            *level = PurityLevel::Impure;
                        }
                    }
                }
            }
            "global_statement" | "nonlocal_statement" => {
                reasons.push("Use of global/nonlocal statement".to_string());
                *level = PurityLevel::Impure;
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.find_impurities(&child, reasons, level, has_io);
        }
    }
}

fn is_io_function(name: &str) -> bool {
    matches!(name, "print" | "input" | "open" | "write" | "read")
        || name.contains("socket")
        || name.contains("request")
        || name.contains("log")
}

fn is_mutation_method(name: &str) -> bool {
    matches!(
        name,
        "append" | "extend" | "insert" | "remove" | "pop" | "clear" | "update" | "add" | "discard"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::python::parser::parse_source;
    use std::path::PathBuf;

    fn parse_py(source: &str) -> tree_sitter::Tree {
        let path = PathBuf::from("test.py");
        let ast = parse_source(source, &path).unwrap();
        ast.tree
    }

    #[test]
    fn test_strictly_pure() {
        let source = "def add(a, b): return a + b";
        let tree = parse_py(source);
        let analysis = PythonPurityAnalyzer::analyze(
            &tree.root_node(),
            source,
            vec!["a".to_string(), "b".to_string()],
        );
        assert!(analysis.is_pure);
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_locally_pure() {
        let source = r#"
def local_mut(n):
    result = 0
    for i in range(n):
        result += i
    return result
"#;
        let tree = parse_py(source);
        let analysis =
            PythonPurityAnalyzer::analyze(&tree.root_node(), source, vec!["n".to_string()]);
        assert!(analysis.is_pure);
        assert_eq!(analysis.purity_level, PurityLevel::LocallyPure);
    }

    #[test]
    fn test_impure_io() {
        let source = "def print_hello(): print('hello')";
        let tree = parse_py(source);
        let analysis = PythonPurityAnalyzer::analyze(&tree.root_node(), source, vec![]);
        assert!(!analysis.is_pure);
        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }

    #[test]
    fn test_impure_mutation() {
        let source = r#"
def mutate_param(items):
    items.append(1)
"#;
        let tree = parse_py(source);
        let analysis =
            PythonPurityAnalyzer::analyze(&tree.root_node(), source, vec!["items".to_string()]);
        assert!(!analysis.is_pure);
        assert!(analysis.upvalue_mutations[0].contains("Mutation method"));
    }
}
