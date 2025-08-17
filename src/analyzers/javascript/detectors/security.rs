// Security pattern detection for JavaScript/TypeScript

use super::{get_node_text, SourceLocation};
use crate::core::{DebtItem, DebtType, Priority};
use std::path::Path;
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator};

#[derive(Debug, Clone)]
pub enum SecurityVulnerability {
    XSS {
        location: SourceLocation,
        sink: String,
        confidence: f32,
    },
    CodeInjection {
        location: SourceLocation,
        vulnerability_type: CodeInjectionType,
    },
    InsecureRandom {
        location: SourceLocation,
        context: String,
    },
    PrototypePollution {
        location: SourceLocation,
        property: String,
    },
    MissingCSRF {
        location: SourceLocation,
        endpoint: String,
    },
    UnsafeDeserialization {
        location: SourceLocation,
        method: String,
    },
}

#[derive(Debug, Clone)]
pub enum CodeInjectionType {
    EvalUsage,
    FunctionConstructor,
    SetTimeout,
    SetInterval,
}

impl SecurityVulnerability {
    pub fn to_debt_item(&self, path: &Path) -> DebtItem {
        let (message, priority) = match self {
            Self::XSS {
                sink, confidence, ..
            } => (
                format!(
                    "Potential XSS vulnerability via {} (confidence: {:.0}%)",
                    sink,
                    confidence * 100.0
                ),
                if *confidence > 0.8 {
                    Priority::Critical
                } else {
                    Priority::High
                },
            ),
            Self::CodeInjection {
                vulnerability_type, ..
            } => {
                let msg = match vulnerability_type {
                    CodeInjectionType::EvalUsage => {
                        "eval() usage detected - high risk of code injection"
                    }
                    CodeInjectionType::FunctionConstructor => {
                        "Function constructor usage - potential code injection"
                    }
                    CodeInjectionType::SetTimeout => {
                        "setTimeout with string argument - potential code injection"
                    }
                    CodeInjectionType::SetInterval => {
                        "setInterval with string argument - potential code injection"
                    }
                };
                (msg.to_string(), Priority::High)
            }
            Self::InsecureRandom { context, .. } => (
                format!(
                    "Math.random() used in {} context - not cryptographically secure",
                    context
                ),
                Priority::Medium,
            ),
            Self::PrototypePollution { property, .. } => (
                format!("Potential prototype pollution via '{}' property", property),
                Priority::High,
            ),
            Self::MissingCSRF { endpoint, .. } => (
                format!("Missing CSRF protection for endpoint '{}'", endpoint),
                Priority::High,
            ),
            Self::UnsafeDeserialization { method, .. } => (
                format!("Unsafe deserialization using {}", method),
                Priority::High,
            ),
        };

        let location = match self {
            Self::XSS { location, .. }
            | Self::CodeInjection { location, .. }
            | Self::InsecureRandom { location, .. }
            | Self::PrototypePollution { location, .. }
            | Self::MissingCSRF { location, .. }
            | Self::UnsafeDeserialization { location, .. } => location,
        };

        DebtItem {
            id: format!("sec-{}-{}", path.display(), location.line),
            debt_type: DebtType::Security,
            priority,
            file: path.to_path_buf(),
            line: location.line,
            column: location.column,
            message,
            context: Some(
                "Consider using safer alternatives or adding proper validation".to_string(),
            ),
        }
    }
}

pub fn detect_security_patterns(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    vulnerabilities: &mut Vec<SecurityVulnerability>,
) {
    detect_xss_vulnerabilities(root, source, language, vulnerabilities);
    detect_eval_usage(root, source, language, vulnerabilities);
    detect_insecure_random(root, source, language, vulnerabilities);
    detect_prototype_pollution(root, source, language, vulnerabilities);
    detect_unsafe_deserialization(root, source, language, vulnerabilities);
}

fn detect_xss_vulnerabilities(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    vulnerabilities: &mut Vec<SecurityVulnerability>,
) {
    // Detect innerHTML assignments
    let query_str = r#"
    (assignment_expression
      left: (member_expression
        property: (property_identifier) @prop
      )
      right: (_) @value
    ) @assignment
    "#;

    if let Ok(query) = Query::new(language, query_str) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            if let Some(prop) = match_.captures.iter().find(|c| c.index == 0) {
                let prop_name = get_node_text(prop.node, source);

                if prop_name == "innerHTML" || prop_name == "outerHTML" {
                    if let Some(value) = match_.captures.iter().find(|c| c.index == 1) {
                        let confidence = if contains_user_input(value.node, source) {
                            0.9
                        } else {
                            0.5
                        };

                        vulnerabilities.push(SecurityVulnerability::XSS {
                            location: SourceLocation::from_node(value.node),
                            sink: prop_name.to_string(),
                            confidence,
                        });
                    }
                }
            }
        }
    }

    // Also detect document.write
    let write_query = r#"
    (call_expression
      function: (member_expression
        object: (identifier) @obj (#eq? @obj "document")
        property: (property_identifier) @method
      )
      arguments: (_) @args
    ) @call
    "#;

    if let Ok(query) = Query::new(language, write_query) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            if let Some(method) = match_.captures.iter().find(|c| c.index == 1) {
                let method_name = get_node_text(method.node, source);

                if method_name == "write" || method_name == "writeln" {
                    if let Some(call) = match_.captures.iter().find(|c| c.index == 3) {
                        vulnerabilities.push(SecurityVulnerability::XSS {
                            location: SourceLocation::from_node(call.node),
                            sink: format!("document.{}", method_name),
                            confidence: 0.8,
                        });
                    }
                }
            }
        }
    }
}

fn detect_eval_usage(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    vulnerabilities: &mut Vec<SecurityVulnerability>,
) {
    let query_str = r#"
    (call_expression
      function: [
        (identifier) @func
        (member_expression
          object: (identifier) @obj
          property: (property_identifier) @prop
        )
      ]
    ) @call
    "#;

    if let Ok(query) = Query::new(language, query_str) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            let mut is_eval = false;
            let mut injection_type = CodeInjectionType::EvalUsage;

            // Check for direct eval
            if let Some(func) = match_.captures.iter().find(|c| c.index == 0) {
                let func_name = get_node_text(func.node, source);
                match func_name {
                    "eval" => {
                        is_eval = true;
                        injection_type = CodeInjectionType::EvalUsage;
                    }
                    "Function" => {
                        is_eval = true;
                        injection_type = CodeInjectionType::FunctionConstructor;
                    }
                    "setTimeout" | "setInterval" => {
                        // Check if first argument is a string (not a function)
                        if is_string_argument(match_.captures.last().unwrap().node, source) {
                            is_eval = true;
                            injection_type = if func_name == "setTimeout" {
                                CodeInjectionType::SetTimeout
                            } else {
                                CodeInjectionType::SetInterval
                            };
                        }
                    }
                    _ => {}
                }
            }

            // Check for window.eval
            if !is_eval {
                if let (Some(obj), Some(prop)) = (
                    match_.captures.iter().find(|c| c.index == 1),
                    match_.captures.iter().find(|c| c.index == 2),
                ) {
                    let obj_name = get_node_text(obj.node, source);
                    let prop_name = get_node_text(prop.node, source);

                    if obj_name == "window" && prop_name == "eval" {
                        is_eval = true;
                        injection_type = CodeInjectionType::EvalUsage;
                    }
                }
            }

            if is_eval {
                if let Some(call) = match_.captures.last() {
                    vulnerabilities.push(SecurityVulnerability::CodeInjection {
                        location: SourceLocation::from_node(call.node),
                        vulnerability_type: injection_type,
                    });
                }
            }
        }
    }
}

fn detect_insecure_random(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    vulnerabilities: &mut Vec<SecurityVulnerability>,
) {
    let query_str = r#"
    (call_expression
      function: (member_expression
        object: (identifier) @obj (#eq? @obj "Math")
        property: (property_identifier) @method (#eq? @method "random")
      )
    ) @call
    "#;

    if let Ok(query) = Query::new(language, query_str) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            if let Some(call) = match_.captures.last() {
                // Try to determine context
                let context = determine_security_context(call.node, source);

                if context != "unknown" {
                    vulnerabilities.push(SecurityVulnerability::InsecureRandom {
                        location: SourceLocation::from_node(call.node),
                        context,
                    });
                }
            }
        }
    }
}

fn detect_prototype_pollution(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    vulnerabilities: &mut Vec<SecurityVulnerability>,
) {
    let query_str = r#"
    (member_expression
      property: [
        (property_identifier) @prop
        (string) @prop_str
      ]
    ) @access
    "#;

    if let Ok(query) = Query::new(language, query_str) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            if let Some(prop) = match_
                .captures
                .iter()
                .find(|c| c.index == 0 || c.index == 1)
            {
                let prop_name = get_node_text(prop.node, source);

                if prop_name == "__proto__"
                    || prop_name == "constructor"
                    || prop_name == "prototype"
                {
                    vulnerabilities.push(SecurityVulnerability::PrototypePollution {
                        location: SourceLocation::from_node(prop.node),
                        property: prop_name.to_string(),
                    });
                }
            }
        }
    }
}

fn detect_unsafe_deserialization(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    vulnerabilities: &mut Vec<SecurityVulnerability>,
) {
    // Detect JSON.parse without validation
    let query_str = r#"
    (call_expression
      function: (member_expression
        object: (identifier) @obj (#eq? @obj "JSON")
        property: (property_identifier) @method (#eq? @method "parse")
      )
      arguments: (arguments
        (_) @input
      )
    ) @call
    "#;

    if let Ok(query) = Query::new(language, query_str) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            if let Some(input) = match_.captures.iter().find(|c| c.index == 2) {
                // Check if input is from untrusted source
                if contains_user_input(input.node, source) {
                    vulnerabilities.push(SecurityVulnerability::UnsafeDeserialization {
                        location: SourceLocation::from_node(input.node),
                        method: "JSON.parse".to_string(),
                    });
                }
            }
        }
    }
}

// Helper functions
fn contains_user_input(node: Node, source: &str) -> bool {
    let text = get_node_text(node, source);
    // Simple heuristic: check for common user input sources
    text.contains("request.")
        || text.contains("req.")
        || text.contains("params")
        || text.contains("query")
        || text.contains("body")
        || text.contains("localStorage")
        || text.contains("sessionStorage")
        || text.contains("location.")
        || text.contains("window.location")
        || text.contains("document.referrer")
        || text.contains("document.cookie")
}

fn is_string_argument(node: Node, _source: &str) -> bool {
    if let Some(args_node) = node.child_by_field_name("arguments") {
        if let Some(first_arg) = args_node.child(1) {
            // Skip opening parenthesis
            return first_arg.kind() == "string";
        }
    }
    false
}

fn determine_security_context(node: Node, source: &str) -> String {
    // Walk up the tree to find context
    let mut current = node;
    for _ in 0..5 {
        if let Some(parent) = current.parent() {
            let parent_text = get_node_text(parent, source);

            if parent_text.contains("token")
                || parent_text.contains("key")
                || parent_text.contains("password")
            {
                return "authentication".to_string();
            }
            if parent_text.contains("crypto")
                || parent_text.contains("encrypt")
                || parent_text.contains("hash")
            {
                return "cryptographic".to_string();
            }
            if parent_text.contains("session") || parent_text.contains("csrf") {
                return "session".to_string();
            }

            current = parent;
        } else {
            break;
        }
    }

    "unknown".to_string()
}
