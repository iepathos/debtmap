use crate::analyzers::go::parser::node_text;
use crate::core::PurityLevel;
use tree_sitter::Node;

#[derive(Debug, Clone)]
pub struct GoPurity {
    pub level: PurityLevel,
    pub confidence: f32,
    pub patterns: Vec<String>,
}

pub fn analyze_purity(node: Node, source: &str) -> GoPurity {
    let patterns = impurity_patterns(node, source);

    if patterns.is_empty() {
        GoPurity {
            level: PurityLevel::StrictlyPure,
            confidence: 0.8,
            patterns,
        }
    } else {
        GoPurity {
            level: PurityLevel::Impure,
            confidence: 0.75,
            patterns,
        }
    }
}

fn impurity_patterns(node: Node, source: &str) -> Vec<String> {
    if is_nested_callable(node) {
        return Vec::new();
    }

    let mut patterns = node_impurity_patterns(node, source);
    patterns.extend(
        children(node)
            .into_iter()
            .flat_map(|child| impurity_patterns(child, source)),
    );
    patterns.sort();
    patterns.dedup();
    patterns
}

fn node_impurity_patterns(node: Node, source: &str) -> Vec<String> {
    match node.kind() {
        "go_statement" => vec!["go-statement".to_string()],
        "defer_statement" => vec!["defer-statement".to_string()],
        "send_statement" => vec!["channel-send".to_string()],
        "inc_statement" | "dec_statement" => vec!["mutation".to_string()],
        "assignment_statement" if mutates_external_target(node, source) => {
            vec!["external-mutation".to_string()]
        }
        "call_expression" => call_impurity_patterns(node, source),
        _ => Vec::new(),
    }
}

fn call_impurity_patterns(node: Node, source: &str) -> Vec<String> {
    let Some(function) = node.child_by_field_name("function") else {
        return Vec::new();
    };
    let text = node_text(&function, source);

    if matches!(text, "panic" | "recover") {
        return vec!["panic-recover".to_string()];
    }

    if is_io_call(text) {
        return vec!["io-call".to_string()];
    }

    Vec::new()
}

fn is_io_call(text: &str) -> bool {
    ["os.", "net.", "http.", "sql.", "fmt.Print"]
        .iter()
        .any(|prefix| text.starts_with(prefix))
}

fn mutates_external_target(node: Node, source: &str) -> bool {
    node.child_by_field_name("left")
        .map(|left| node_text(&left, source))
        .map(|text| text.contains('.') || text.contains('[') || text.contains('*'))
        .unwrap_or(false)
}

fn is_nested_callable(node: Node) -> bool {
    matches!(
        node.kind(),
        "func_literal" | "function_declaration" | "method_declaration"
    )
}

fn children(node: Node) -> Vec<Node> {
    let mut cursor = node.walk();
    node.children(&mut cursor).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::go::parser::parse_source;
    use std::path::PathBuf;

    fn analyze_first_function(source: &str) -> GoPurity {
        let ast = parse_source(source, &PathBuf::from("service.go")).unwrap();
        let root = ast.tree.root_node();
        let mut cursor = root.walk();
        let body = root
            .children(&mut cursor)
            .find(|node| node.kind() == "function_declaration")
            .and_then(|node| node.child_by_field_name("body"))
            .unwrap();
        analyze_purity(body, source)
    }

    #[test]
    fn test_pure_function() {
        let source = "package service\n\nfunc add(a int) int { return a + 1 }";
        let purity = analyze_first_function(source);

        assert_eq!(purity.level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_detects_concurrency_and_io_impurity() {
        let source = r#"package service

func run(ch chan int) {
    go worker()
    defer fmt.Println("done")
    ch <- 1
}
"#;
        let purity = analyze_first_function(source);

        assert_eq!(purity.level, PurityLevel::Impure);
        assert!(purity.patterns.contains(&"go-statement".to_string()));
        assert!(purity.patterns.contains(&"defer-statement".to_string()));
        assert!(purity.patterns.contains(&"channel-send".to_string()));
    }

    #[test]
    fn test_detects_external_mutation() {
        let source = "package service\n\nfunc set(s *State) { s.count = 1 }";
        let purity = analyze_first_function(source);

        assert!(purity.patterns.contains(&"external-mutation".to_string()));
    }
}
