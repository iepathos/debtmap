//! NatSpec parsing and documentation quality checks for Solidity functions.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use tree_sitter::Node;

use crate::analyzers::solidity::parser::{node_line, node_text};
use crate::core::ast::SolidityAst;
use crate::core::{DebtItem, DebtType, FunctionMetrics, Priority};

const MAX_NATSPEC_LOOKBACK: usize = 20;
const MAX_BLANK_GAP: usize = 1;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NatSpecDoc {
    pub notice: Option<String>,
    pub dev: Option<String>,
    pub params: HashMap<String, String>,
    pub returns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FunctionSignature {
    pub params: Vec<String>,
    pub return_slots: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NatSpecIssue {
    Missing,
    MissingParam(String),
    StaleParam(String),
    MissingReturn,
    PlaceholderNotice,
    PlaceholderDev,
}

pub fn detect_natspec_debt(
    path: &Path,
    ast: &SolidityAst,
    functions: &[FunctionMetrics],
) -> Vec<DebtItem> {
    let lines = ast.source.lines().collect::<Vec<_>>();
    let signatures = function_signatures_by_line(ast);

    functions
        .iter()
        .filter(|function| !function.is_test)
        .filter(|function| matches!(function.visibility.as_deref(), Some("public" | "external")))
        .flat_map(|function| {
            let doc = parse_natspec_before(&lines, function.line);
            let signature = signatures.get(&function.line).cloned().unwrap_or_default();
            issues_for_function(&doc, &signature)
                .into_iter()
                .map(|issue| debt_item_for_issue(path, function, issue))
        })
        .collect()
}

fn issues_for_function(
    doc: &Option<NatSpecDoc>,
    signature: &FunctionSignature,
) -> Vec<NatSpecIssue> {
    let Some(doc) = doc else {
        return vec![NatSpecIssue::Missing];
    };

    let mut issues = Vec::new();
    push_placeholder_issues(doc, &mut issues);
    push_param_issues(doc, signature, &mut issues);
    push_return_issues(doc, signature, &mut issues);
    issues
}

fn push_placeholder_issues(doc: &NatSpecDoc, issues: &mut Vec<NatSpecIssue>) {
    if doc
        .notice
        .as_ref()
        .is_some_and(|text| is_placeholder_text(text))
    {
        issues.push(NatSpecIssue::PlaceholderNotice);
    }
    if doc
        .dev
        .as_ref()
        .is_some_and(|text| is_placeholder_text(text))
    {
        issues.push(NatSpecIssue::PlaceholderDev);
    }
    if doc.notice.is_none() && doc.dev.is_none() {
        issues.push(NatSpecIssue::Missing);
    }
}

fn push_param_issues(
    doc: &NatSpecDoc,
    signature: &FunctionSignature,
    issues: &mut Vec<NatSpecIssue>,
) {
    let documented = doc.params.keys().cloned().collect::<HashSet<_>>();
    let actual = signature.params.iter().cloned().collect::<HashSet<_>>();

    for param in &signature.params {
        if !documented.contains(param) {
            issues.push(NatSpecIssue::MissingParam(param.clone()));
        }
    }
    for param in documented {
        if !actual.contains(&param) {
            issues.push(NatSpecIssue::StaleParam(param));
        }
    }
}

fn push_return_issues(
    doc: &NatSpecDoc,
    signature: &FunctionSignature,
    issues: &mut Vec<NatSpecIssue>,
) {
    if signature.return_slots == 0 {
        return;
    }

    let documented = doc
        .returns
        .iter()
        .filter(|value| !is_placeholder_text(value))
        .count();
    if documented < signature.return_slots {
        issues.push(NatSpecIssue::MissingReturn);
    }
}

pub fn parse_natspec_before(lines: &[&str], function_line: usize) -> Option<NatSpecDoc> {
    let raw_lines = collect_doc_lines(lines, function_line)?;
    (!raw_lines.is_empty()).then(|| parse_doc_lines(&raw_lines))
}

fn collect_doc_lines(lines: &[&str], function_line: usize) -> Option<Vec<String>> {
    if function_line == 0 {
        return None;
    }

    let start = function_line.saturating_sub(1);
    let mut doc_lines = Vec::new();
    let mut blank_gap = 0;

    for index in (0..start).rev() {
        if start - index > MAX_NATSPEC_LOOKBACK {
            break;
        }

        let trimmed = lines[index].trim();
        if let Some(content) = trimmed.strip_prefix("///") {
            doc_lines.insert(0, content.trim().to_string());
            blank_gap = 0;
            continue;
        }

        if trimmed.is_empty() {
            blank_gap += 1;
            if blank_gap > MAX_BLANK_GAP {
                break;
            }
            continue;
        }

        break;
    }

    (!doc_lines.is_empty()).then_some(doc_lines)
}

fn parse_doc_lines(lines: &[String]) -> NatSpecDoc {
    let mut doc = NatSpecDoc::default();
    let mut current_tag: Option<String> = None;

    for line in lines {
        if let Some(tag) = parse_tag_line(line) {
            apply_tag(&mut doc, &tag);
            current_tag = Some(tag.kind);
            continue;
        }

        if let Some(kind) = &current_tag {
            append_continuation(&mut doc, kind, line);
        }
    }

    doc
}

struct ParsedTag {
    kind: String,
    name: Option<String>,
    text: String,
}

fn parse_tag_line(line: &str) -> Option<ParsedTag> {
    let trimmed = line.trim();
    if !trimmed.starts_with('@') {
        return None;
    }

    let rest = trimmed.trim_start_matches('@');
    let (tag, remainder) = rest.split_once(char::is_whitespace)?;
    let kind = tag.to_ascii_lowercase();

    match kind.as_str() {
        "param" => {
            let (name, text) = remainder.split_once(char::is_whitespace)?;
            Some(ParsedTag {
                kind,
                name: Some(name.to_string()),
                text: text.trim().to_string(),
            })
        }
        "return" => Some(ParsedTag {
            kind,
            name: None,
            text: remainder.trim().to_string(),
        }),
        "notice" | "dev" => Some(ParsedTag {
            kind,
            name: None,
            text: remainder.trim().to_string(),
        }),
        _ => None,
    }
}

fn apply_tag(doc: &mut NatSpecDoc, tag: &ParsedTag) {
    match tag.kind.as_str() {
        "notice" => doc.notice = Some(tag.text.clone()),
        "dev" => doc.dev = Some(tag.text.clone()),
        "param" => {
            if let Some(name) = &tag.name {
                doc.params.insert(name.clone(), tag.text.clone());
            }
        }
        "return" => doc.returns.push(tag.text.clone()),
        _ => {}
    }
}

fn append_continuation(doc: &mut NatSpecDoc, kind: &str, line: &str) {
    let text = line.trim();
    if text.is_empty() {
        return;
    }

    match kind {
        "notice" => append_text(&mut doc.notice, text),
        "dev" => append_text(&mut doc.dev, text),
        "return" => append_last(&mut doc.returns, text),
        _ => {}
    }
}

fn append_text(target: &mut Option<String>, text: &str) {
    match target {
        Some(existing) => {
            existing.push(' ');
            existing.push_str(text);
        }
        None => *target = Some(text.to_string()),
    }
}

fn append_last(values: &mut Vec<String>, text: &str) {
    if let Some(last) = values.last_mut() {
        last.push(' ');
        last.push_str(text);
    } else {
        values.push(text.to_string());
    }
}

fn is_placeholder_text(text: &str) -> bool {
    let normalized = text.trim().trim_matches('.').to_ascii_lowercase();
    normalized.is_empty()
        || matches!(
            normalized.as_str(),
            "todo"
                | "tbd"
                | "fixme"
                | "n/a"
                | "na"
                | "none"
                | "placeholder"
                | "xxx"
                | "..."
                | "tbc"
        )
}

fn function_signatures_by_line(ast: &SolidityAst) -> HashMap<usize, FunctionSignature> {
    let mut signatures = HashMap::new();
    collect_signatures(ast.tree.root_node(), &ast.source, &mut signatures);
    signatures
}

fn collect_signatures(
    node: Node,
    source: &str,
    signatures: &mut HashMap<usize, FunctionSignature>,
) {
    if node.kind() == "function_definition" {
        let line = node_line(&node);
        signatures.insert(line, signature_from_function(node, source));
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_signatures(child, source, signatures);
    }
}

fn signature_from_function(node: Node, source: &str) -> FunctionSignature {
    let return_type = node.child_by_field_name("return_type");
    FunctionSignature {
        params: parameter_names(node, source, return_type),
        return_slots: return_slot_count(return_type, source),
    }
}

fn parameter_names(function: Node, source: &str, return_type: Option<Node>) -> Vec<String> {
    let mut names = Vec::new();
    walk_nodes(function, &mut |node| {
        if node.kind() != "parameter" {
            return;
        }
        if is_under_node(node, return_type) {
            return;
        }
        if let Some(name) = node.child_by_field_name("name") {
            names.push(node_text(&name, source).to_string());
        }
    });
    names
}

fn return_slot_count(return_type: Option<Node>, _source: &str) -> usize {
    let Some(return_type) = return_type else {
        return 0;
    };

    let mut count = 0;
    walk_nodes(return_type, &mut |node| {
        if matches!(node.kind(), "parameter" | "return_parameter") {
            count += 1;
        }
    });

    if count > 0 {
        return count;
    }

    return_type
        .children(&mut return_type.walk())
        .any(|child| child.kind() == "type_name")
        .then_some(1)
        .unwrap_or(0)
}

fn is_under_node(node: Node, ancestor: Option<Node>) -> bool {
    let Some(ancestor) = ancestor else {
        return false;
    };

    let mut current = Some(node);
    while let Some(parent) = current {
        if parent == ancestor {
            return true;
        }
        current = parent.parent();
    }
    false
}

fn walk_nodes(node: Node, visit: &mut impl FnMut(Node)) {
    visit(node);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_nodes(child, visit);
    }
}

fn debt_item_for_issue(path: &Path, function: &FunctionMetrics, issue: NatSpecIssue) -> DebtItem {
    let (pattern, message, context) = issue_details(&issue, function);
    DebtItem {
        id: format!("solidity-{pattern}-{}-{}", path.display(), function.line),
        debt_type: DebtType::CodeSmell {
            smell_type: Some(pattern.to_string()),
        },
        priority: Priority::Low,
        file: path.to_path_buf(),
        line: function.line,
        column: None,
        message,
        context: Some(context),
    }
}

fn issue_details(
    issue: &NatSpecIssue,
    function: &FunctionMetrics,
) -> (&'static str, String, String) {
    match issue {
        NatSpecIssue::Missing => (
            "missing-natspec",
            format!(
                "Function '{}' is public/external without NatSpec",
                function.name
            ),
            "Add /// @notice or /// @dev documentation for external callers.".to_string(),
        ),
        NatSpecIssue::MissingParam(param) => (
            "missing-natspec-param",
            format!("Function '{}' is missing @param {param}", function.name),
            "Document each parameter with /// @param name description.".to_string(),
        ),
        NatSpecIssue::StaleParam(param) => (
            "stale-natspec-param",
            format!(
                "Function '{}' documents stale @param {param}",
                function.name
            ),
            "Remove or rename stale NatSpec params to match the function signature.".to_string(),
        ),
        NatSpecIssue::MissingReturn => (
            "missing-natspec-return",
            format!(
                "Function '{}' is missing @return documentation",
                function.name
            ),
            "Document return values with /// @return description.".to_string(),
        ),
        NatSpecIssue::PlaceholderNotice => (
            "placeholder-natspec",
            format!("Function '{}' has placeholder @notice text", function.name),
            "Replace placeholder NatSpec with a meaningful description.".to_string(),
        ),
        NatSpecIssue::PlaceholderDev => (
            "placeholder-natspec",
            format!("Function '{}' has placeholder @dev text", function.name),
            "Replace placeholder NatSpec with a meaningful description.".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::solidity::parser::parse_source;
    use std::path::Path;

    fn lines(source: &str) -> Vec<&str> {
        source.lines().collect()
    }

    #[test]
    fn test_parse_multiline_natspec_block() {
        let source = "/// @notice Transfers tokens\n/// to the recipient.\n/// @param to Recipient\ncontract C { function f(address to) public {} }";
        let doc = parse_natspec_before(&lines(source), 4).expect("doc");
        assert_eq!(
            doc.notice.as_deref(),
            Some("Transfers tokens to the recipient.")
        );
        assert_eq!(doc.params.get("to").map(String::as_str), Some("Recipient"));
    }

    #[test]
    fn test_detects_missing_param_and_stale_param() {
        let doc = NatSpecDoc {
            notice: Some("Does something".to_string()),
            params: HashMap::from([
                ("old".to_string(), "Old param".to_string()),
                ("amount".to_string(), "Amount".to_string()),
            ]),
            ..Default::default()
        };
        let signature = FunctionSignature {
            params: vec!["to".to_string(), "amount".to_string()],
            return_slots: 0,
        };

        let issues = issues_for_function(&Some(doc), &signature);
        assert!(issues.contains(&NatSpecIssue::MissingParam("to".to_string())));
        assert!(issues.contains(&NatSpecIssue::StaleParam("old".to_string())));
    }

    #[test]
    fn test_detects_missing_return_for_named_returns() {
        let doc = NatSpecDoc {
            notice: Some("Returns values".to_string()),
            returns: vec!["The amount".to_string()],
            ..Default::default()
        };
        let signature = FunctionSignature {
            params: vec![],
            return_slots: 2,
        };

        let issues = issues_for_function(&Some(doc), &signature);
        assert!(issues.contains(&NatSpecIssue::MissingReturn));
    }

    #[test]
    fn test_detects_placeholder_notice() {
        let doc = NatSpecDoc {
            notice: Some("TODO".to_string()),
            ..Default::default()
        };
        let issues = issues_for_function(&Some(doc), &FunctionSignature::default());
        assert!(issues.contains(&NatSpecIssue::PlaceholderNotice));
    }

    #[test]
    fn test_ignores_internal_functions_in_debt_detection() {
        let source = r#"pragma solidity 0.8.20;
contract C {
    function hidden(uint256 value) internal {}
}
"#;
        let ast = parse_source(source, Path::new("Internal.sol")).expect("parse");
        let function = FunctionMetrics {
            name: "C.hidden".to_string(),
            file: ast.path.clone(),
            line: 3,
            cyclomatic: 1,
            cognitive: 0,
            nesting: 0,
            length: 1,
            is_test: false,
            visibility: Some("internal".to_string()),
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        };

        assert!(detect_natspec_debt(Path::new("Internal.sol"), &ast, &[function]).is_empty());
    }

    #[test]
    fn test_signature_extraction_for_returns() {
        let source = r#"pragma solidity 0.8.20;
contract C {
    function f(address to, uint256 amount) public returns (uint256 value, bool ok) {}
}
"#;
        let ast = parse_source(source, Path::new("Returns.sol")).expect("parse");
        let signatures = function_signatures_by_line(&ast);
        let signature = signatures.get(&3).expect("signature");
        assert_eq!(signature.params, vec!["to", "amount"]);
        assert_eq!(signature.return_slots, 2);
    }
}
