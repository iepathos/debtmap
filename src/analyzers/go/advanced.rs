use crate::analyzers::go::parser::node_text;
use tree_sitter::Node;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GoAdvancedSignals {
    pub patterns: Vec<String>,
    pub error_swallowing_count: u32,
    pub error_swallowing_patterns: Vec<String>,
}

pub fn detect_advanced_signals(
    body: Node,
    source: &str,
    function_name: &str,
    is_test: bool,
    package_variables: &[String],
) -> GoAdvancedSignals {
    let mut signals = collect_signals(body, source, function_name, is_test, package_variables);
    add_repetitive_error_handling(body, source, &mut signals);
    normalize_signals(signals)
}

fn collect_signals(
    node: Node,
    source: &str,
    function_name: &str,
    is_test: bool,
    package_variables: &[String],
) -> GoAdvancedSignals {
    if is_nested_callable(node) {
        return GoAdvancedSignals::default();
    }

    children(node)
        .into_iter()
        .map(|child| collect_signals(child, source, function_name, is_test, package_variables))
        .fold(
            node_signals(node, source, function_name, is_test, package_variables),
            merge_signals,
        )
}

fn node_signals(
    node: Node,
    source: &str,
    function_name: &str,
    is_test: bool,
    package_variables: &[String],
) -> GoAdvancedSignals {
    match node.kind() {
        "go_statement" => pattern("goroutine-without-synchronization"),
        "defer_statement" if has_loop_ancestor(node) => pattern("defer-in-loop"),
        "send_statement" => pattern("channel-operation"),
        "unary_expression" if node_text(&node, source).trim_start().starts_with("<-") => {
            pattern("channel-operation")
        }
        "assignment_statement" => {
            assignment_signals(node, source, function_name, package_variables)
        }
        "call_expression" => call_signals(node, source, function_name, is_test),
        _ => GoAdvancedSignals::default(),
    }
}

fn assignment_signals(
    node: Node,
    source: &str,
    function_name: &str,
    package_variables: &[String],
) -> GoAdvancedSignals {
    let text = node_text(&node, source);
    let mut signals = GoAdvancedSignals::default();

    if swallows_error(text) {
        signals.patterns.push("swallowed-error".to_string());
        signals.error_swallowing_count = 1;
        signals
            .error_swallowing_patterns
            .push("blank-identifier-error".to_string());
    }

    if mutates_pointer_receiver(text, function_name) {
        signals
            .patterns
            .push("pointer-receiver-mutation".to_string());
    }

    if mutates_indexed_value(text) {
        signals.patterns.push("collection-mutation".to_string());
    }

    if mutates_package_variable(text, package_variables) {
        signals.patterns.push("package-global-mutation".to_string());
    }

    signals
}

fn call_signals(node: Node, source: &str, function_name: &str, is_test: bool) -> GoAdvancedSignals {
    let Some(function) = node.child_by_field_name("function") else {
        return GoAdvancedSignals::default();
    };

    match node_text(&function, source) {
        "panic" if !is_test && function_name != "main" => pattern("panic-in-production"),
        "recover" => pattern("recover-without-handling"),
        _ => GoAdvancedSignals::default(),
    }
}

fn add_repetitive_error_handling(body: Node, source: &str, signals: &mut GoAdvancedSignals) {
    if count_error_return_branches(body, source) >= 3 {
        signals
            .patterns
            .push("repetitive-error-handling".to_string());
    }
}

fn count_error_return_branches(node: Node, source: &str) -> u32 {
    if is_nested_callable(node) {
        return 0;
    }

    let current = u32::from(is_error_return_branch(node, source));
    current
        + children(node)
            .into_iter()
            .map(|child| count_error_return_branches(child, source))
            .sum::<u32>()
}

fn is_error_return_branch(node: Node, source: &str) -> bool {
    if node.kind() != "if_statement" {
        return false;
    }

    let text = node_text(&node, source);
    text.contains("err != nil") && text.contains("return")
}

fn swallows_error(text: &str) -> bool {
    let left = text.split_once('=').map(|(left, _)| left).unwrap_or(text);
    left.split(',')
        .map(str::trim)
        .any(|part| part == "_" || part.ends_with(" _"))
}

fn mutates_pointer_receiver(text: &str, function_name: &str) -> bool {
    function_name.contains('.')
        && text
            .split_once('=')
            .is_some_and(|(left, _)| left.contains('.'))
}

fn mutates_indexed_value(text: &str) -> bool {
    text.split_once('=')
        .is_some_and(|(left, _)| left.contains('[') && left.contains(']'))
}

fn mutates_package_variable(text: &str, package_variables: &[String]) -> bool {
    let Some((left, _)) = text.split_once('=') else {
        return false;
    };

    left.split(',')
        .map(clean_assignment_target)
        .any(|target| package_variables.iter().any(|name| name == target))
}

fn clean_assignment_target(target: &str) -> &str {
    target
        .trim()
        .trim_end_matches(':')
        .trim_end_matches('+')
        .trim_end_matches('-')
}

fn has_loop_ancestor(node: Node) -> bool {
    node.parent().is_some_and(|parent| {
        is_loop_node(parent) || (!is_nested_callable(parent) && has_loop_ancestor(parent))
    })
}

fn is_loop_node(node: Node) -> bool {
    node.kind() == "for_statement"
}

fn is_nested_callable(node: Node) -> bool {
    matches!(
        node.kind(),
        "func_literal" | "function_declaration" | "method_declaration"
    )
}

fn pattern(name: &str) -> GoAdvancedSignals {
    GoAdvancedSignals {
        patterns: vec![name.to_string()],
        ..Default::default()
    }
}

fn merge_signals(mut left: GoAdvancedSignals, right: GoAdvancedSignals) -> GoAdvancedSignals {
    left.patterns.extend(right.patterns);
    left.error_swallowing_count += right.error_swallowing_count;
    left.error_swallowing_patterns
        .extend(right.error_swallowing_patterns);
    left
}

fn normalize_signals(mut signals: GoAdvancedSignals) -> GoAdvancedSignals {
    signals.patterns.sort();
    signals.patterns.dedup();
    signals.error_swallowing_patterns.sort();
    signals.error_swallowing_patterns.dedup();
    signals
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

    fn signals(source: &str, name: &str) -> GoAdvancedSignals {
        let ast = parse_source(source, &PathBuf::from("service.go")).unwrap();
        let root = ast.tree.root_node();
        let mut cursor = root.walk();
        let body = root
            .children(&mut cursor)
            .find(|node| matches!(node.kind(), "function_declaration" | "method_declaration"))
            .and_then(|node| node.child_by_field_name("body"))
            .unwrap();
        detect_advanced_signals(body, source, name, false, &[])
    }

    #[test]
    fn detects_repetitive_error_handling() {
        let source = r#"package service

func Load() error {
    if err != nil { return err }
    if err != nil { return err }
    if err != nil { return err }
    return nil
}
"#;
        let signals = signals(source, "Load");

        assert!(
            signals
                .patterns
                .contains(&"repetitive-error-handling".to_string())
        );
    }

    #[test]
    fn detects_swallowed_error() {
        let source = r#"package service

func Load() {
    value, _ := parse()
    _ = value
}
"#;
        let signals = signals(source, "Load");

        assert!(signals.patterns.contains(&"swallowed-error".to_string()));
        assert_eq!(signals.error_swallowing_count, 1);
    }

    #[test]
    fn detects_panic_recover_and_concurrency_risks() {
        let source = r#"package service

func Run(ch chan int) {
    go worker()
    ch <- 1
    defer recover()
    panic("failed")
}
"#;
        let signals = signals(source, "Run");

        assert!(
            signals
                .patterns
                .contains(&"goroutine-without-synchronization".to_string())
        );
        assert!(signals.patterns.contains(&"channel-operation".to_string()));
        assert!(
            signals
                .patterns
                .contains(&"recover-without-handling".to_string())
        );
        assert!(
            signals
                .patterns
                .contains(&"panic-in-production".to_string())
        );
    }

    #[test]
    fn detects_defer_in_loop_and_mutation() {
        let source = r#"package service

func (s *State) Run(items map[string]int) {
    for key := range items {
        defer close()
        s.count = 1
        items[key] = 2
    }
}
"#;
        let signals = signals(source, "State.Run");

        assert!(signals.patterns.contains(&"defer-in-loop".to_string()));
        assert!(
            signals
                .patterns
                .contains(&"pointer-receiver-mutation".to_string())
        );
        assert!(
            signals
                .patterns
                .contains(&"collection-mutation".to_string())
        );
    }

    #[test]
    fn detects_package_global_mutation() {
        let source = r#"package service

var shared int

func Update() {
    shared = 1
}
"#;
        let ast = parse_source(source, &PathBuf::from("service.go")).unwrap();
        let root = ast.tree.root_node();
        let mut cursor = root.walk();
        let body = root
            .children(&mut cursor)
            .find(|node| node.kind() == "function_declaration")
            .and_then(|node| node.child_by_field_name("body"))
            .unwrap();
        let signals = detect_advanced_signals(body, source, "Update", false, &["shared".into()]);

        assert!(
            signals
                .patterns
                .contains(&"package-global-mutation".to_string())
        );
    }
}
