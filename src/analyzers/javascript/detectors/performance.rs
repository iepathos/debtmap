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
    fn generate_message(&self) -> String {
        match self {
            Self::SequentialAsync {
                count, suggestion, ..
            } => format!(
                "Sequential async operations ({} awaits) - {}",
                count, suggestion
            ),
            Self::MissingPromiseAll { awaits, .. } => {
                format!("{} sequential awaits could use Promise.all()", awaits)
            }
            Self::DOMThrashing {
                operations,
                suggestion,
                ..
            } => format!(
                "DOM layout thrashing detected ({} operations) - {}",
                operations, suggestion
            ),
            Self::NestedLoops { depth, .. } => format!("Deeply nested loops (depth: {})", depth),
            Self::RepeatedDOMQueries {
                selector, count, ..
            } => format!(
                "DOM selector '{}' queried {} times - consider caching",
                selector, count
            ),
            Self::SynchronousXHR { .. } => {
                "Synchronous XMLHttpRequest blocks the main thread".to_string()
            }
            Self::LargeImport { module, .. } => {
                format!("Large library import '{}' - consider tree-shaking", module)
            }
        }
    }

    fn determine_priority(&self) -> Priority {
        match self {
            Self::SequentialAsync { .. } | Self::MissingPromiseAll { .. } => Priority::Medium,
            Self::DOMThrashing { .. } => Priority::High,
            Self::NestedLoops { depth, .. } => {
                if *depth > 3 {
                    Priority::High
                } else {
                    Priority::Medium
                }
            }
            Self::RepeatedDOMQueries { .. } => Priority::Medium,
            Self::SynchronousXHR { .. } => Priority::Critical,
            Self::LargeImport { .. } => Priority::Low,
        }
    }

    fn get_location(&self) -> &SourceLocation {
        match self {
            Self::SequentialAsync { location, .. }
            | Self::MissingPromiseAll { location, .. }
            | Self::DOMThrashing { location, .. }
            | Self::NestedLoops { location, .. }
            | Self::RepeatedDOMQueries { location, .. }
            | Self::SynchronousXHR { location }
            | Self::LargeImport { location, .. } => location,
        }
    }

    pub fn to_debt_item(&self, path: &Path) -> DebtItem {
        let message = self.generate_message();
        let priority = self.determine_priority();
        let location = self.get_location();

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{DebtType, Priority};
    use std::path::PathBuf;

    fn create_test_location() -> SourceLocation {
        SourceLocation {
            line: 42,
            column: Some(10),
            end_line: None,
            end_column: None,
        }
    }

    #[test]
    fn test_generate_message_sequential_async() {
        let pattern = PerformanceAntiPattern::SequentialAsync {
            location: create_test_location(),
            count: 5,
            suggestion: "Consider using Promise.all()".to_string(),
        };
        assert_eq!(
            pattern.generate_message(),
            "Sequential async operations (5 awaits) - Consider using Promise.all()"
        );
    }

    #[test]
    fn test_generate_message_missing_promise_all() {
        let pattern = PerformanceAntiPattern::MissingPromiseAll {
            location: create_test_location(),
            awaits: 3,
        };
        assert_eq!(
            pattern.generate_message(),
            "3 sequential awaits could use Promise.all()"
        );
    }

    #[test]
    fn test_generate_message_dom_thrashing() {
        let pattern = PerformanceAntiPattern::DOMThrashing {
            location: create_test_location(),
            operations: 10,
            suggestion: "Batch DOM operations".to_string(),
        };
        assert_eq!(
            pattern.generate_message(),
            "DOM layout thrashing detected (10 operations) - Batch DOM operations"
        );
    }

    #[test]
    fn test_generate_message_nested_loops() {
        let pattern = PerformanceAntiPattern::NestedLoops {
            location: create_test_location(),
            depth: 4,
        };
        assert_eq!(pattern.generate_message(), "Deeply nested loops (depth: 4)");
    }

    #[test]
    fn test_generate_message_repeated_dom_queries() {
        let pattern = PerformanceAntiPattern::RepeatedDOMQueries {
            location: create_test_location(),
            selector: ".my-class".to_string(),
            count: 7,
        };
        assert_eq!(
            pattern.generate_message(),
            "DOM selector '.my-class' queried 7 times - consider caching"
        );
    }

    #[test]
    fn test_generate_message_synchronous_xhr() {
        let pattern = PerformanceAntiPattern::SynchronousXHR {
            location: create_test_location(),
        };
        assert_eq!(
            pattern.generate_message(),
            "Synchronous XMLHttpRequest blocks the main thread"
        );
    }

    #[test]
    fn test_generate_message_large_import() {
        let pattern = PerformanceAntiPattern::LargeImport {
            location: create_test_location(),
            module: "lodash".to_string(),
        };
        assert_eq!(
            pattern.generate_message(),
            "Large library import 'lodash' - consider tree-shaking"
        );
    }

    #[test]
    fn test_determine_priority_sequential_async() {
        let pattern = PerformanceAntiPattern::SequentialAsync {
            location: create_test_location(),
            count: 5,
            suggestion: "test".to_string(),
        };
        assert_eq!(pattern.determine_priority(), Priority::Medium);
    }

    #[test]
    fn test_determine_priority_missing_promise_all() {
        let pattern = PerformanceAntiPattern::MissingPromiseAll {
            location: create_test_location(),
            awaits: 3,
        };
        assert_eq!(pattern.determine_priority(), Priority::Medium);
    }

    #[test]
    fn test_determine_priority_dom_thrashing() {
        let pattern = PerformanceAntiPattern::DOMThrashing {
            location: create_test_location(),
            operations: 10,
            suggestion: "test".to_string(),
        };
        assert_eq!(pattern.determine_priority(), Priority::High);
    }

    #[test]
    fn test_determine_priority_nested_loops_shallow() {
        let pattern = PerformanceAntiPattern::NestedLoops {
            location: create_test_location(),
            depth: 3,
        };
        assert_eq!(pattern.determine_priority(), Priority::Medium);
    }

    #[test]
    fn test_determine_priority_nested_loops_deep() {
        let pattern = PerformanceAntiPattern::NestedLoops {
            location: create_test_location(),
            depth: 5,
        };
        assert_eq!(pattern.determine_priority(), Priority::High);
    }

    #[test]
    fn test_determine_priority_repeated_dom_queries() {
        let pattern = PerformanceAntiPattern::RepeatedDOMQueries {
            location: create_test_location(),
            selector: "test".to_string(),
            count: 5,
        };
        assert_eq!(pattern.determine_priority(), Priority::Medium);
    }

    #[test]
    fn test_determine_priority_synchronous_xhr() {
        let pattern = PerformanceAntiPattern::SynchronousXHR {
            location: create_test_location(),
        };
        assert_eq!(pattern.determine_priority(), Priority::Critical);
    }

    #[test]
    fn test_determine_priority_large_import() {
        let pattern = PerformanceAntiPattern::LargeImport {
            location: create_test_location(),
            module: "test".to_string(),
        };
        assert_eq!(pattern.determine_priority(), Priority::Low);
    }

    #[test]
    fn test_get_location_all_variants() {
        let location = create_test_location();

        let patterns = vec![
            PerformanceAntiPattern::SequentialAsync {
                location: location.clone(),
                count: 1,
                suggestion: "test".to_string(),
            },
            PerformanceAntiPattern::MissingPromiseAll {
                location: location.clone(),
                awaits: 1,
            },
            PerformanceAntiPattern::DOMThrashing {
                location: location.clone(),
                operations: 1,
                suggestion: "test".to_string(),
            },
            PerformanceAntiPattern::NestedLoops {
                location: location.clone(),
                depth: 1,
            },
            PerformanceAntiPattern::RepeatedDOMQueries {
                location: location.clone(),
                selector: "test".to_string(),
                count: 1,
            },
            PerformanceAntiPattern::SynchronousXHR {
                location: location.clone(),
            },
            PerformanceAntiPattern::LargeImport {
                location: location.clone(),
                module: "test".to_string(),
            },
        ];

        for pattern in patterns {
            let retrieved_location = pattern.get_location();
            assert_eq!(retrieved_location.line, 42);
            assert_eq!(retrieved_location.column, Some(10));
        }
    }

    #[test]
    fn test_to_debt_item_integration() {
        let pattern = PerformanceAntiPattern::DOMThrashing {
            location: SourceLocation {
                line: 100,
                column: Some(20),
                end_line: None,
                end_column: None,
            },
            operations: 15,
            suggestion: "Batch DOM operations".to_string(),
        };

        let path = PathBuf::from("test.js");
        let debt_item = pattern.to_debt_item(&path);

        assert_eq!(debt_item.id, "perf-test.js-100");
        assert_eq!(debt_item.debt_type, DebtType::Performance);
        assert_eq!(debt_item.priority, Priority::High);
        assert_eq!(debt_item.file, path);
        assert_eq!(debt_item.line, 100);
        assert_eq!(debt_item.column, Some(20));
        assert_eq!(
            debt_item.message,
            "DOM layout thrashing detected (15 operations) - Batch DOM operations"
        );
        assert_eq!(debt_item.context, None);
    }

    #[test]
    fn test_to_debt_item_nested_loops_edge_case() {
        // Test the edge case where depth is exactly 3 (should be Medium)
        let pattern_medium = PerformanceAntiPattern::NestedLoops {
            location: SourceLocation {
                line: 50,
                column: Some(5),
                end_line: None,
                end_column: None,
            },
            depth: 3,
        };

        let path = PathBuf::from("loops.js");
        let debt_item = pattern_medium.to_debt_item(&path);
        assert_eq!(debt_item.priority, Priority::Medium);

        // Test when depth is 4 (should be High)
        let pattern_high = PerformanceAntiPattern::NestedLoops {
            location: SourceLocation {
                line: 60,
                column: Some(5),
                end_line: None,
                end_column: None,
            },
            depth: 4,
        };

        let debt_item_high = pattern_high.to_debt_item(&path);
        assert_eq!(debt_item_high.priority, Priority::High);
    }
}
