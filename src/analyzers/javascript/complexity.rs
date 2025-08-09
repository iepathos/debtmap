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
    for child in node.children(&mut node.walk()) {
        if child.kind() == "identifier" || child.kind() == "property_identifier" {
            if let Ok(name) = child.utf8_text(source.as_bytes()) {
                return name.to_string();
            }
        }
    }

    // For arrow functions without explicit names, try to find assignment
    if node.kind() == "arrow_function" {
        if let Some(parent) = node.parent() {
            if parent.kind() == "variable_declarator" {
                for child in parent.children(&mut parent.walk()) {
                    if child.kind() == "identifier" {
                        if let Ok(name) = child.utf8_text(source.as_bytes()) {
                            return name.to_string();
                        }
                    }
                }
            }
        }
    }

    "<anonymous>".to_string()
}

pub fn calculate_cyclomatic_complexity(node: Node, source: &str) -> u32 {
    let mut complexity = 1; // Base complexity
    visit_node_for_complexity(node, source, &mut complexity);
    complexity
}

fn visit_node_for_complexity(node: Node, source: &str, complexity: &mut u32) {
    match node.kind() {
        // Control flow statements
        "if_statement" | "ternary_expression" => *complexity += 1,
        "switch_case" | "case_statement" => *complexity += 1,
        "while_statement" | "do_statement" | "for_statement" | "for_in_statement"
        | "for_of_statement" => *complexity += 1,
        "catch_clause" => *complexity += 1,
        // Logical operators create branches
        "binary_expression" => {
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                if text.contains("&&") || text.contains("||") {
                    *complexity += 1;
                }
            }
        }
        // Optional chaining and nullish coalescing
        "optional_chain" => *complexity += 1,
        _ => {}
    }

    for child in node.children(&mut node.walk()) {
        visit_node_for_complexity(child, source, complexity);
    }
}

pub fn calculate_cognitive_complexity(node: Node, source: &str, nesting_level: u32) -> u32 {
    let mut complexity = 0;

    match node.kind() {
        // Structural complexity
        "if_statement" => {
            complexity += 1 + nesting_level;
            for child in node.children(&mut node.walk()) {
                if child.kind() == "else_clause" {
                    complexity += 1; // Additional complexity for else
                }
                complexity += calculate_cognitive_complexity(child, source, nesting_level + 1);
            }
            return complexity;
        }
        "switch_statement" => {
            complexity += nesting_level;
            for child in node.children(&mut node.walk()) {
                complexity += calculate_cognitive_complexity(child, source, nesting_level + 1);
            }
            return complexity;
        }
        "while_statement" | "do_statement" | "for_statement" | "for_in_statement"
        | "for_of_statement" => {
            complexity += 1 + nesting_level;
            for child in node.children(&mut node.walk()) {
                complexity += calculate_cognitive_complexity(child, source, nesting_level + 1);
            }
            return complexity;
        }
        "catch_clause" => {
            complexity += nesting_level;
            for child in node.children(&mut node.walk()) {
                complexity += calculate_cognitive_complexity(child, source, nesting_level + 1);
            }
            return complexity;
        }
        "ternary_expression" => {
            complexity += 1 + nesting_level;
            for child in node.children(&mut node.walk()) {
                complexity += calculate_cognitive_complexity(child, source, nesting_level);
            }
            return complexity;
        }
        // Logical operators
        "binary_expression" => {
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                if text.contains("&&") || text.contains("||") {
                    complexity += 1;
                }
            }
            for child in node.children(&mut node.walk()) {
                complexity += calculate_cognitive_complexity(child, source, nesting_level);
            }
            return complexity;
        }
        _ => {}
    }

    // Continue traversing for other nodes
    for child in node.children(&mut node.walk()) {
        complexity += calculate_cognitive_complexity(child, source, nesting_level);
    }

    complexity
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
