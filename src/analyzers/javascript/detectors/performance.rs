// Performance pattern detection for JavaScript/TypeScript

use super::{get_node_text, SourceLocation};
use crate::core::{DebtItem, DebtType, Priority};
use std::path::Path;
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator};

#[derive(Debug, Clone)]
pub enum PerformanceAntiPattern {
    SequentialAsync {
        location: SourceLocation,
        count: usize,
        suggestion: String,
    },
    MissingPromiseAll {
        location: SourceLocation,
        awaits: usize,
    },
    DOMThrashing {
        location: SourceLocation,
        operations: usize,
        suggestion: String,
    },
    NestedLoops {
        location: SourceLocation,
        depth: usize,
    },
    RepeatedDOMQueries {
        location: SourceLocation,
        selector: String,
        count: usize,
    },
    SynchronousXHR {
        location: SourceLocation,
    },
    LargeImport {
        location: SourceLocation,
        module: String,
    },
}

impl PerformanceAntiPattern {
    pub fn to_debt_item(&self, path: &Path) -> DebtItem {
        let (message, priority) = match self {
            Self::SequentialAsync {
                count, suggestion, ..
            } => (
                format!(
                    "Sequential async operations ({} awaits) - {}",
                    count, suggestion
                ),
                Priority::Medium,
            ),
            Self::MissingPromiseAll { awaits, .. } => (
                format!("{} sequential awaits could use Promise.all()", awaits),
                Priority::Medium,
            ),
            Self::DOMThrashing {
                operations,
                suggestion,
                ..
            } => (
                format!(
                    "DOM layout thrashing detected ({} operations) - {}",
                    operations, suggestion
                ),
                Priority::High,
            ),
            Self::NestedLoops { depth, .. } => (
                format!("Deeply nested loops (depth: {})", depth),
                if *depth > 3 {
                    Priority::High
                } else {
                    Priority::Medium
                },
            ),
            Self::RepeatedDOMQueries {
                selector, count, ..
            } => (
                format!(
                    "DOM selector '{}' queried {} times - consider caching",
                    selector, count
                ),
                Priority::Medium,
            ),
            Self::SynchronousXHR { .. } => (
                "Synchronous XMLHttpRequest blocks the main thread".to_string(),
                Priority::Critical,
            ),
            Self::LargeImport { module, .. } => (
                format!("Large library import '{}' - consider tree-shaking", module),
                Priority::Low,
            ),
        };

        let location = match self {
            Self::SequentialAsync { location, .. }
            | Self::MissingPromiseAll { location, .. }
            | Self::DOMThrashing { location, .. }
            | Self::NestedLoops { location, .. }
            | Self::RepeatedDOMQueries { location, .. }
            | Self::SynchronousXHR { location }
            | Self::LargeImport { location, .. } => location,
        };

        DebtItem {
            id: format!("perf-{}-{}", path.display(), location.line),
            debt_type: DebtType::Performance,
            priority,
            file: path.to_path_buf(),
            line: location.line,
            column: location.column,
            message,
            context: None,
        }
    }
}

pub fn detect_performance_patterns(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    patterns: &mut Vec<PerformanceAntiPattern>,
) {
    detect_sequential_awaits(root, source, language, patterns);
    detect_missing_promise_all(root, source, language, patterns);
    detect_layout_thrashing(root, source, language, patterns);
    detect_nested_loops(root, source, language, patterns);
    detect_repeated_dom_queries(root, source, language, patterns);
    detect_synchronous_xhr(root, source, language, patterns);
}

fn detect_sequential_awaits(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    patterns: &mut Vec<PerformanceAntiPattern>,
) {
    let query_str = r#"
    (block_statement
      (expression_statement
        (await_expression) @await1
      )
      (expression_statement
        (await_expression) @await2
      )
    )
    "#;

    if let Ok(query) = Query::new(language, query_str) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            if match_.captures.len() >= 2 {
                let await1 = match_.captures[0].node;
                let location = SourceLocation::from_node(await1);

                patterns.push(PerformanceAntiPattern::SequentialAsync {
                    location,
                    count: 2,
                    suggestion: "Consider using Promise.all() for parallel execution".to_string(),
                });
            }
        }
    }
}

fn detect_missing_promise_all(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    patterns: &mut Vec<PerformanceAntiPattern>,
) {
    // Detect patterns like:
    // const a = await fetchA();
    // const b = await fetchB();
    // const c = await fetchC();
    let query_str = r#"
    (variable_declaration
      (variable_declarator
        init: (await_expression
          (call_expression) @async_call
        )
      )
    ) @declaration
    "#;

    if let Ok(query) = Query::new(language, query_str) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        // Look for consecutive await declarations
        let mut consecutive_count = 0;
        let mut prev_end_row = None;
        let mut last_node = None;

        while let Some(match_) = matches.next() {
            if let Some(node) = match_.captures.iter().find(|c| c.index == 0) {
                let start_row = node.node.start_position().row;

                if let Some(prev) = prev_end_row {
                    if start_row == prev + 1 || start_row == prev {
                        consecutive_count += 1;
                    } else if consecutive_count >= 2 {
                        patterns.push(PerformanceAntiPattern::MissingPromiseAll {
                            location: SourceLocation::from_node(node.node),
                            awaits: consecutive_count + 1,
                        });
                        consecutive_count = 0;
                    }
                }

                prev_end_row = Some(node.node.end_position().row);
                last_node = Some(node.node);
            }
        }

        // Check final sequence
        if consecutive_count >= 2 {
            if let Some(node) = last_node {
                patterns.push(PerformanceAntiPattern::MissingPromiseAll {
                    location: SourceLocation::from_node(node),
                    awaits: consecutive_count + 1,
                });
            }
        }
    }
}

fn detect_layout_thrashing(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    patterns: &mut Vec<PerformanceAntiPattern>,
) {
    let query_str = r#"
    (member_expression
      property: (property_identifier) @prop
    ) @access
    "#;

    if let Ok(query) = Query::new(language, query_str) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        let layout_props = [
            "offsetWidth",
            "offsetHeight",
            "offsetTop",
            "offsetLeft",
            "scrollWidth",
            "scrollHeight",
            "scrollTop",
            "scrollLeft",
            "clientWidth",
            "clientHeight",
            "clientTop",
            "clientLeft",
        ];

        let mut accesses = Vec::new();
        while let Some(match_) = matches.next() {
            if let Some(prop_capture) = match_.captures.iter().find(|c| c.index == 0) {
                let prop_name = get_node_text(prop_capture.node, source);
                if layout_props.contains(&prop_name) {
                    accesses.push((prop_name, prop_capture.node));
                }
            }
        }

        // Check for interleaved reads and writes (simplified detection)
        if accesses.len() > 4 {
            let location = SourceLocation::from_node(accesses[0].1);
            patterns.push(PerformanceAntiPattern::DOMThrashing {
                location,
                operations: accesses.len(),
                suggestion: "Batch DOM reads and writes separately".to_string(),
            });
        }
    }
}

fn detect_nested_loops(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    patterns: &mut Vec<PerformanceAntiPattern>,
) {
    let query_str = r#"
    (for_statement
      body: (block_statement
        (for_statement) @inner_loop
      )
    ) @outer_loop
    "#;

    if let Ok(query) = Query::new(language, query_str) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            if let Some(outer) = match_.captures.iter().find(|c| c.index == 1) {
                let location = SourceLocation::from_node(outer.node);
                patterns.push(PerformanceAntiPattern::NestedLoops {
                    location,
                    depth: 2, // Simple detection - could be enhanced to count deeper nesting
                });
            }
        }
    }
}

fn detect_repeated_dom_queries(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    patterns: &mut Vec<PerformanceAntiPattern>,
) {
    let query_str = r#"
    (call_expression
      function: (member_expression
        object: (identifier) @obj
        property: (property_identifier) @method
      )
      arguments: (arguments
        (string) @selector
      )
    ) @query_call
    "#;

    if let Ok(query) = Query::new(language, query_str) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        let mut selector_counts = std::collections::HashMap::new();

        while let Some(match_) = matches.next() {
            let obj = match_
                .captures
                .iter()
                .find(|c| c.index == 0)
                .map(|c| get_node_text(c.node, source));
            let method = match_
                .captures
                .iter()
                .find(|c| c.index == 1)
                .map(|c| get_node_text(c.node, source));
            let selector = match_
                .captures
                .iter()
                .find(|c| c.index == 2)
                .map(|c| get_node_text(c.node, source));

            if let (Some("document"), Some(method), Some(selector)) = (obj, method, selector) {
                if method == "querySelector"
                    || method == "querySelectorAll"
                    || method == "getElementById"
                    || method == "getElementsByClassName"
                {
                    let entry = selector_counts
                        .entry(selector.to_string())
                        .or_insert((0, None));
                    entry.0 += 1;
                    if entry.1.is_none() {
                        if let Some(node) = match_.captures.iter().find(|c| c.index == 3) {
                            entry.1 = Some(node.node);
                        }
                    }
                }
            }
        }

        for (selector, (count, node)) in selector_counts {
            if count > 2 {
                if let Some(node) = node {
                    patterns.push(PerformanceAntiPattern::RepeatedDOMQueries {
                        location: SourceLocation::from_node(node),
                        selector,
                        count,
                    });
                }
            }
        }
    }
}

fn detect_synchronous_xhr(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    patterns: &mut Vec<PerformanceAntiPattern>,
) {
    let query_str = r#"
    (call_expression
      function: (member_expression
        object: (identifier) @xhr
        property: (property_identifier) @method
      )
      arguments: (arguments
        (_)
        (_)
        (false) @sync
      )
    ) @xhr_call
    "#;

    if let Ok(query) = Query::new(language, query_str) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            if let Some(method_capture) = match_.captures.iter().find(|c| c.index == 1) {
                let method = get_node_text(method_capture.node, source);
                if method == "open" {
                    if let Some(call) = match_.captures.iter().find(|c| c.index == 3) {
                        patterns.push(PerformanceAntiPattern::SynchronousXHR {
                            location: SourceLocation::from_node(call.node),
                        });
                    }
                }
            }
        }
    }
}
