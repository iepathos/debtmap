use crate::core::FunctionMetrics;
use std::path::Path;
use tree_sitter::Node;

pub fn extract_functions(node: Node, source: &str, path: &Path) -> Vec<FunctionMetrics> {
    collect_functions_from_node(node, source, path)
}

fn collect_functions_from_node(node: Node, source: &str, path: &Path) -> Vec<FunctionMetrics> {
    const FUNCTION_NODES: &[&str] = &[
        "function_declaration",
        "function_expression",
        "arrow_function",
        "method_definition",
        "generator_function_declaration",
    ];

    let mut functions = Vec::new();

    if FUNCTION_NODES.contains(&node.kind()) {
        if let Some(metrics) = analyze_function(node, source, path) {
            functions.push(metrics);
        }
    }

    functions.extend(
        node.children(&mut node.walk())
            .flat_map(|child| collect_functions_from_node(child, source, path)),
    );

    functions
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
    1 + calculate_complexity_sum(node, source)
}

fn calculate_complexity_sum(node: Node, source: &str) -> u32 {
    node_complexity_increment(node, source)
        + node
            .children(&mut node.walk())
            .map(|child| calculate_complexity_sum(child, source))
            .sum::<u32>()
}

fn node_complexity_increment(node: Node, source: &str) -> u32 {
    const CONTROL_FLOW_NODES: &[&str] = &[
        "if_statement",
        "ternary_expression",
        "switch_case",
        "case_statement",
        "while_statement",
        "do_statement",
        "for_statement",
        "for_in_statement",
        "for_of_statement",
        "catch_clause",
        "optional_chain",
    ];

    if CONTROL_FLOW_NODES.contains(&node.kind()) {
        1
    } else if node.kind() == "binary_expression" {
        is_logical_operator(node, source) as u32
    } else {
        0
    }
}

fn is_logical_operator(node: Node, source: &str) -> bool {
    node.utf8_text(source.as_bytes())
        .ok()
        .map(|text| text.contains("&&") || text.contains("||"))
        .unwrap_or(false)
}

pub fn calculate_cognitive_complexity(node: Node, source: &str, nesting_level: u32) -> u32 {
    NodeComplexityHandler::from_node_kind(node.kind()).calculate_complexity(
        node,
        source,
        nesting_level,
    )
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
    fn from_node_kind(kind: &str) -> Self {
        const LOOP_STATEMENTS: &[&str] = &[
            "while_statement",
            "do_statement",
            "for_statement",
            "for_in_statement",
            "for_of_statement",
        ];

        if kind == "if_statement" {
            Self::IfStatement
        } else if kind == "switch_statement" {
            Self::SwitchStatement
        } else if LOOP_STATEMENTS.contains(&kind) {
            Self::LoopStatement
        } else if kind == "catch_clause" {
            Self::CatchClause
        } else if kind == "ternary_expression" {
            Self::TernaryExpression
        } else if kind == "binary_expression" {
            Self::BinaryExpression
        } else {
            Self::Default
        }
    }

    fn calculate_complexity(self, node: Node, source: &str, nesting_level: u32) -> u32 {
        let (base, nesting_inc, _include_else) = self.get_complexity_params();

        match self {
            Self::IfStatement => {
                base + nesting_level
                    + self.sum_child_complexity(node, source, nesting_level + nesting_inc)
                    + self.count_else_clauses(node)
            }
            Self::TernaryExpression => {
                base + nesting_level + self.sum_child_complexity(node, source, nesting_level)
            }
            Self::BinaryExpression => {
                self.logical_operator_complexity(node, source)
                    + self.sum_child_complexity(node, source, nesting_level)
            }
            Self::Default => self.sum_child_complexity(node, source, nesting_level),
            _ => {
                base + nesting_level
                    + self.sum_child_complexity(node, source, nesting_level + nesting_inc)
            }
        }
    }

    fn get_complexity_params(&self) -> (u32, u32, bool) {
        use NodeComplexityHandler::*;
        const PARAMS: [(u32, u32, bool); 7] = [
            (1, 1, true),  // IfStatement
            (0, 1, false), // SwitchStatement
            (1, 1, false), // LoopStatement
            (0, 1, false), // CatchClause
            (1, 0, false), // TernaryExpression
            (0, 0, false), // BinaryExpression
            (0, 0, false), // Default
        ];

        let index = match self {
            IfStatement => 0,
            SwitchStatement => 1,
            LoopStatement => 2,
            CatchClause => 3,
            TernaryExpression => 4,
            BinaryExpression => 5,
            Default => 6,
        };
        PARAMS[index]
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
        is_logical_operator(node, source) as u32
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
