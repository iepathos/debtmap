use crate::core::FunctionMetrics;
use std::path::Path;
use tree_sitter::Node;

pub fn extract_functions(node: Node, source: &str, path: &Path) -> Vec<FunctionMetrics> {
    let mut functions = Vec::new();
    visit_node_for_functions(node, source, path, &mut functions);
    functions
}

fn visit_node_for_functions(
    node: Node,
    source: &str,
    path: &Path,
    functions: &mut Vec<FunctionMetrics>,
) {
    match node.kind() {
        "function_declaration"
        | "function_expression"
        | "arrow_function"
        | "method_definition"
        | "generator_function_declaration" => {
            if let Some(metrics) = analyze_function(node, source, path) {
                functions.push(metrics);
            }
        }
        _ => {}
    }

    for child in node.children(&mut node.walk()) {
        visit_node_for_functions(child, source, path, functions);
    }
}

fn analyze_function(node: Node, source: &str, path: &Path) -> Option<FunctionMetrics> {
    let name = get_function_name(node, source);
    let line = node.start_position().row + 1;
    let mut metrics = FunctionMetrics::new(name, path.to_path_buf(), line);

    // Calculate complexity
    metrics.cyclomatic = calculate_cyclomatic_complexity(node, source);
    metrics.cognitive = calculate_cognitive_complexity(node, source, 0);
    metrics.nesting = calculate_max_nesting(node, 0);
    metrics.length = node.end_position().row - node.start_position().row + 1;

    Some(metrics)
}

fn get_function_name(node: Node, source: &str) -> String {
    // Try to find the identifier node for the function name
    node.children(&mut node.walk())
        .filter(|child| matches!(child.kind(), "identifier" | "property_identifier"))
        .find_map(|child| child.utf8_text(source.as_bytes()).ok())
        .map(String::from)
        .or_else(|| get_arrow_function_name(node, source))
        .unwrap_or_else(|| "<anonymous>".to_string())
}

fn get_arrow_function_name(node: Node, source: &str) -> Option<String> {
    if node.kind() != "arrow_function" {
        return None;
    }

    node.parent()
        .filter(|parent| parent.kind() == "variable_declarator")
        .and_then(|parent| {
            parent
                .children(&mut parent.walk())
                .filter(|child| child.kind() == "identifier")
                .find_map(|child| child.utf8_text(source.as_bytes()).ok())
                .map(String::from)
        })
}

pub fn calculate_cyclomatic_complexity(node: Node, source: &str) -> u32 {
    let mut complexity = 1; // Base complexity
    visit_node_for_complexity(node, source, &mut complexity);
    complexity
}

fn visit_node_for_complexity(node: Node, source: &str, complexity: &mut u32) {
    *complexity += node_complexity_increment(node, source);

    node.children(&mut node.walk())
        .for_each(|child| visit_node_for_complexity(child, source, complexity));
}

fn node_complexity_increment(node: Node, source: &str) -> u32 {
    match node.kind() {
        // Control flow statements
        "if_statement" | "ternary_expression" | "switch_case" | "case_statement"
        | "while_statement" | "do_statement" | "for_statement" | "for_in_statement"
        | "for_of_statement" | "catch_clause" | "optional_chain" => 1,

        // Logical operators create branches
        "binary_expression" => node
            .utf8_text(source.as_bytes())
            .ok()
            .filter(|text| text.contains("&&") || text.contains("||"))
            .map(|_| 1)
            .unwrap_or(0),

        _ => 0,
    }
}

pub fn calculate_cognitive_complexity(node: Node, source: &str, nesting_level: u32) -> u32 {
    use NodeComplexityHandler as Handler;

    let handler = match node.kind() {
        "if_statement" => Handler::IfStatement,
        "switch_statement" => Handler::SwitchStatement,
        "while_statement" | "do_statement" | "for_statement" | "for_in_statement"
        | "for_of_statement" => Handler::LoopStatement,
        "catch_clause" => Handler::CatchClause,
        "ternary_expression" => Handler::TernaryExpression,
        "binary_expression" => Handler::BinaryExpression,
        _ => Handler::Default,
    };

    handler.calculate_complexity(node, source, nesting_level)
}

enum NodeComplexityHandler {
    IfStatement,
    SwitchStatement,
    LoopStatement,
    CatchClause,
    TernaryExpression,
    BinaryExpression,
    Default,
}

impl NodeComplexityHandler {
    fn calculate_complexity(self, node: Node, source: &str, nesting_level: u32) -> u32 {
        match self {
            Self::IfStatement => {
                self.structural_complexity(node, source, nesting_level, 1)
                    + self.count_else_clauses(node)
            }
            Self::SwitchStatement => self.structural_complexity(node, source, nesting_level, 0),
            Self::LoopStatement => self.structural_complexity(node, source, nesting_level, 1),
            Self::CatchClause => self.structural_complexity(node, source, nesting_level, 0),
            Self::TernaryExpression => {
                1 + nesting_level + self.sum_child_complexity(node, source, nesting_level)
            }
            Self::BinaryExpression => {
                self.logical_operator_complexity(node, source)
                    + self.sum_child_complexity(node, source, nesting_level)
            }
            Self::Default => self.sum_child_complexity(node, source, nesting_level),
        }
    }

    fn structural_complexity(
        &self,
        node: Node,
        source: &str,
        nesting_level: u32,
        base_increment: u32,
    ) -> u32 {
        base_increment + nesting_level + self.sum_child_complexity(node, source, nesting_level + 1)
    }

    fn sum_child_complexity(&self, node: Node, source: &str, nesting_level: u32) -> u32 {
        node.children(&mut node.walk())
            .map(|child| calculate_cognitive_complexity(child, source, nesting_level))
            .sum()
    }

    fn count_else_clauses(&self, node: Node) -> u32 {
        node.children(&mut node.walk())
            .filter(|child| child.kind() == "else_clause")
            .count() as u32
    }

    fn logical_operator_complexity(&self, node: Node, source: &str) -> u32 {
        node.utf8_text(source.as_bytes())
            .ok()
            .filter(|text| text.contains("&&") || text.contains("||"))
            .map(|_| 1)
            .unwrap_or(0)
    }
}

fn calculate_max_nesting(node: Node, current_depth: u32) -> u32 {
    let mut max_depth = current_depth;

    let new_depth = match node.kind() {
        "if_statement" | "while_statement" | "do_statement" | "for_statement"
        | "for_in_statement" | "for_of_statement" | "switch_statement" | "try_statement"
        | "catch_clause" => current_depth + 1,
        _ => current_depth,
    };

    for child in node.children(&mut node.walk()) {
        let child_depth = calculate_max_nesting(child, new_depth);
        max_depth = max_depth.max(child_depth);
    }

    max_depth
}
