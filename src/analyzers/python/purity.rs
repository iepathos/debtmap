//! Python purity analysis
//!
//! This module provides purity detection for Python functions.

use crate::extraction::{PurityAnalysisData, PurityLevel};
use std::collections::HashSet;
use tree_sitter::Node;

struct ImpurityReport {
    reasons: Vec<String>,
    level: PurityLevel,
    has_io: bool,
}

impl ImpurityReport {
    fn pure() -> Self {
        Self {
            reasons: Vec::new(),
            level: PurityLevel::StrictlyPure,
            has_io: false,
        }
    }

    fn local() -> Self {
        Self {
            level: PurityLevel::LocallyPure,
            ..Self::pure()
        }
    }

    fn impure(reason: String) -> Self {
        Self {
            reasons: vec![reason],
            level: PurityLevel::Impure,
            has_io: false,
        }
    }

    fn io(reason: String) -> Self {
        Self {
            has_io: true,
            ..Self::impure(reason)
        }
    }

    fn merge(self, other: Self) -> Self {
        let level = merge_purity_levels(self.level, other.level);
        let mut reasons = self.reasons;
        reasons.extend(other.reasons);

        Self {
            reasons,
            level,
            has_io: self.has_io || other.has_io,
        }
    }
}

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
        self.collect_locals(node);
        let report = self.find_impurities(node);

        let is_pure = report.reasons.is_empty();
        let confidence = if is_pure { 0.8 } else { 0.9 };

        PurityAnalysisData {
            is_pure,
            has_mutations: !is_pure && !report.has_io,
            has_io_operations: report.has_io,
            has_unsafe: false,
            local_mutations: if report.level == PurityLevel::LocallyPure {
                vec!["local".to_string()]
            } else {
                vec![]
            },
            upvalue_mutations: report.reasons,
            total_mutations: if is_pure { 0 } else { 1 },
            var_names: std::collections::HashMap::new(),
            confidence,
            purity_level: report.level,
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

    fn find_impurities(&self, node: &Node) -> ImpurityReport {
        let report = self.node_impurity(node);
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .map(|child| self.find_impurities(&child))
            .fold(report, ImpurityReport::merge)
    }

    fn node_impurity(&self, node: &Node) -> ImpurityReport {
        match node.kind() {
            "assignment" => self.assignment_impurity(node),
            "call" => self.call_impurity(node),
            "global_statement" | "nonlocal_statement" => {
                ImpurityReport::impure("Use of global/nonlocal statement".to_string())
            }
            _ => ImpurityReport::pure(),
        }
    }

    fn assignment_impurity(&self, node: &Node) -> ImpurityReport {
        node.child_by_field_name("left")
            .map(|left| self.assignment_target_impurity(&left))
            .unwrap_or_else(ImpurityReport::pure)
    }

    fn assignment_target_impurity(&self, target: &Node) -> ImpurityReport {
        match target.kind() {
            "attribute" => self.attribute_assignment_impurity(target),
            "identifier" => self.identifier_assignment_impurity(target),
            _ => ImpurityReport::pure(),
        }
    }

    fn attribute_assignment_impurity(&self, target: &Node) -> ImpurityReport {
        target
            .child_by_field_name("object")
            .map(|object| self.object_assignment_impurity(self.text(&object)))
            .unwrap_or_else(ImpurityReport::pure)
    }

    fn object_assignment_impurity(&self, object: &str) -> ImpurityReport {
        if object == "self" {
            ImpurityReport::impure("Mutation of 'self' attribute".to_string())
        } else if self.params.contains(object) {
            ImpurityReport::impure(format!("Mutation of parameter attribute: {}", object))
        } else if self.local_vars.contains(object) {
            ImpurityReport::local()
        } else {
            ImpurityReport::impure(format!("Mutation of external object: {}", object))
        }
    }

    fn identifier_assignment_impurity(&self, target: &Node) -> ImpurityReport {
        let name = self.text(target);
        if self.local_vars.contains(name) || self.params.contains(name) {
            ImpurityReport::local()
        } else {
            ImpurityReport::impure(format!("Mutation of global/external variable: {}", name))
        }
    }

    fn call_impurity(&self, node: &Node) -> ImpurityReport {
        node.child_by_field_name("function")
            .map(|function| self.function_call_impurity(&function))
            .unwrap_or_else(ImpurityReport::pure)
    }

    fn function_call_impurity(&self, function: &Node) -> ImpurityReport {
        let name = self.function_name(function);
        if is_io_function(name) {
            ImpurityReport::io(format!("I/O call: {}", name))
        } else if is_mutation_method(name) {
            self.mutation_call_impurity(function, name)
        } else {
            ImpurityReport::pure()
        }
    }

    fn mutation_call_impurity(&self, function: &Node, name: &str) -> ImpurityReport {
        if function.kind() == "attribute" {
            self.attribute_mutation_call_impurity(function, name)
        } else {
            ImpurityReport::impure(format!("Direct call to mutation-named function: {}", name))
        }
    }

    fn attribute_mutation_call_impurity(&self, function: &Node, name: &str) -> ImpurityReport {
        function
            .child_by_field_name("object")
            .map(|object| self.non_local_mutation_call(self.text(&object), name))
            .unwrap_or_else(ImpurityReport::pure)
    }

    fn non_local_mutation_call(&self, object: &str, name: &str) -> ImpurityReport {
        if self.local_vars.contains(object) {
            ImpurityReport::pure()
        } else {
            ImpurityReport::impure(format!(
                "Mutation method '{}' on non-local: {}",
                name, object
            ))
        }
    }

    fn function_name(&self, function: &Node) -> &str {
        if function.kind() == "attribute" {
            function
                .child_by_field_name("attribute")
                .map(|attribute| self.text(&attribute))
                .unwrap_or("")
        } else {
            self.text(function)
        }
    }

    fn text(&self, node: &Node) -> &str {
        &self.source[node.start_byte()..node.end_byte()]
    }
}

fn merge_purity_levels(left: PurityLevel, right: PurityLevel) -> PurityLevel {
    if left == PurityLevel::Impure || right == PurityLevel::Impure {
        PurityLevel::Impure
    } else if left == PurityLevel::LocallyPure || right == PurityLevel::LocallyPure {
        PurityLevel::LocallyPure
    } else {
        PurityLevel::StrictlyPure
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

    fn analyze_py(source: &str, params: Vec<&str>) -> PurityAnalysisData {
        let tree = parse_py(source);
        PythonPurityAnalyzer::analyze(
            &tree.root_node(),
            source,
            params.into_iter().map(str::to_string).collect(),
        )
    }

    #[test]
    fn test_strictly_pure() {
        let source = "def add(a, b): return a + b";
        let analysis = analyze_py(source, vec!["a", "b"]);
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
        let analysis = analyze_py(source, vec!["n"]);
        assert!(analysis.is_pure);
        assert_eq!(analysis.purity_level, PurityLevel::LocallyPure);
    }

    #[test]
    fn test_impure_io() {
        let source = "def print_hello(): print('hello')";
        let analysis = analyze_py(source, vec![]);
        assert!(!analysis.is_pure);
        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }

    #[test]
    fn test_impure_mutation() {
        let source = r#"
def mutate_param(items):
    items.append(1)
"#;
        let analysis = analyze_py(source, vec!["items"]);
        assert!(!analysis.is_pure);
        assert!(analysis.upvalue_mutations[0].contains("Mutation method"));
    }

    #[test]
    fn test_self_attribute_assignment_is_impure() {
        let source = r#"
def set_value(self, value):
    self.value = value
"#;
        let analysis = analyze_py(source, vec!["self", "value"]);
        assert!(!analysis.is_pure);
        assert_eq!(analysis.purity_level, PurityLevel::Impure);
        assert_eq!(
            analysis.upvalue_mutations[0],
            "Mutation of 'self' attribute"
        );
    }

    #[test]
    fn test_global_statement_is_impure() {
        let source = r#"
def update_global(value):
    global current
    current = value
"#;
        let analysis = analyze_py(source, vec!["value"]);
        assert!(!analysis.is_pure);
        assert_eq!(analysis.purity_level, PurityLevel::Impure);
        assert!(analysis
            .upvalue_mutations
            .contains(&"Use of global/nonlocal statement".to_string()));
    }

    #[test]
    fn test_mutation_method_on_local_collection_stays_pure() {
        let source = r#"
def collect(value):
    items = []
    items.append(value)
    return items
"#;
        let analysis = analyze_py(source, vec!["value"]);
        assert!(analysis.is_pure);
        assert_eq!(analysis.purity_level, PurityLevel::LocallyPure);
    }
}
