// Resource management detection for JavaScript/TypeScript

use super::{get_node_text, SourceLocation};
use crate::core::{DebtItem, DebtType, Priority};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator};

#[derive(Debug, Clone)]
pub enum ResourceManagementIssue {
    EventListenerLeak {
        location: SourceLocation,
        event_type: String,
        missing_cleanup: String,
    },
    TimerLeak {
        location: SourceLocation,
        timer_type: String,
    },
    WebSocketLeak {
        location: SourceLocation,
    },
    WorkerLeak {
        location: SourceLocation,
        worker_type: String,
    },
    MemoryRetention {
        location: SourceLocation,
        cause: String,
    },
    ObserverLeak {
        location: SourceLocation,
        observer_type: String,
    },
}

impl ResourceManagementIssue {
    pub fn to_debt_item(&self, path: &Path) -> DebtItem {
        let (message, priority) = match self {
            Self::EventListenerLeak {
                event_type,
                missing_cleanup,
                ..
            } => (
                format!(
                    "Event listener '{}' not removed - {}",
                    event_type, missing_cleanup
                ),
                Priority::Medium,
            ),
            Self::TimerLeak { timer_type, .. } => (
                format!("{} not cleared - potential memory leak", timer_type),
                Priority::Medium,
            ),
            Self::WebSocketLeak { .. } => (
                "WebSocket connection not properly closed".to_string(),
                Priority::High,
            ),
            Self::WorkerLeak { worker_type, .. } => (
                format!("{} not terminated - resource leak", worker_type),
                Priority::High,
            ),
            Self::MemoryRetention { cause, .. } => (
                format!("Potential memory retention: {}", cause),
                Priority::Medium,
            ),
            Self::ObserverLeak { observer_type, .. } => (
                format!("{} not disconnected - memory leak", observer_type),
                Priority::Medium,
            ),
        };

        let location = match self {
            Self::EventListenerLeak { location, .. }
            | Self::TimerLeak { location, .. }
            | Self::WebSocketLeak { location }
            | Self::WorkerLeak { location, .. }
            | Self::MemoryRetention { location, .. }
            | Self::ObserverLeak { location, .. } => location,
        };

        DebtItem {
            id: format!("resource-{}-{}", path.display(), location.line),
            debt_type: DebtType::ResourceManagement,
            priority,
            file: path.to_path_buf(),
            line: location.line,
            column: location.column,
            message,
            context: Some(
                "Ensure proper cleanup in component lifecycle or cleanup functions".to_string(),
            ),
        }
    }
}

pub fn detect_resource_patterns(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    issues: &mut Vec<ResourceManagementIssue>,
) {
    detect_event_listener_leaks(root, source, language, issues);
    detect_timer_leaks(root, source, language, issues);
    detect_websocket_leaks(root, source, language, issues);
    detect_worker_leaks(root, source, language, issues);
    detect_observer_leaks(root, source, language, issues);
    detect_memory_retention(root, source, language, issues);
}

fn detect_event_listener_leaks(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    issues: &mut Vec<ResourceManagementIssue>,
) {
    // Find addEventListener calls
    let add_query = r#"
    (call_expression
      function: (member_expression
        property: (property_identifier) @method (#eq? @method "addEventListener")
      )
      arguments: (arguments
        (string) @event
        (_) @handler
      )
    ) @add_call
    "#;

    // Find removeEventListener calls
    let remove_query = r#"
    (call_expression
      function: (member_expression
        property: (property_identifier) @method (#eq? @method "removeEventListener")
      )
      arguments: (arguments
        (string) @event
        (_) @handler
      )
    ) @remove_call
    "#;

    let mut added_listeners = HashMap::new();
    let mut removed_listeners = HashSet::new();

    // Collect all addEventListener calls
    if let Ok(query) = Query::new(language, add_query) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            if let (Some(event), Some(handler), Some(call)) = (
                match_.captures.iter().find(|c| c.index == 1),
                match_.captures.iter().find(|c| c.index == 2),
                match_.captures.iter().find(|c| c.index == 3),
            ) {
                let event_type = get_node_text(event.node, source)
                    .trim_matches('"')
                    .trim_matches('\'');
                let handler_text = get_node_text(handler.node, source);
                added_listeners.insert(
                    (event_type.to_string(), handler_text.to_string()),
                    SourceLocation::from_node(call.node),
                );
            }
        }
    }

    // Collect all removeEventListener calls
    if let Ok(query) = Query::new(language, remove_query) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            if let (Some(event), Some(handler)) = (
                match_.captures.iter().find(|c| c.index == 1),
                match_.captures.iter().find(|c| c.index == 2),
            ) {
                let event_type = get_node_text(event.node, source)
                    .trim_matches('"')
                    .trim_matches('\'');
                let handler_text = get_node_text(handler.node, source);
                removed_listeners.insert((event_type.to_string(), handler_text.to_string()));
            }
        }
    }

    // Find listeners that were added but not removed
    for ((event_type, _handler), location) in added_listeners {
        if !removed_listeners.iter().any(|(e, _)| e == &event_type) {
            issues.push(ResourceManagementIssue::EventListenerLeak {
                location,
                event_type: event_type.clone(),
                missing_cleanup: format!("Call removeEventListener for '{}'", event_type),
            });
        }
    }
}

fn detect_timer_leaks(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    issues: &mut Vec<ResourceManagementIssue>,
) {
    // Detect setInterval/setTimeout without clearInterval/clearTimeout
    let timer_query = r#"
    (call_expression
      function: (identifier) @func
    ) @timer_call
    "#;

    let clear_query = r#"
    (call_expression
      function: (identifier) @func
    ) @clear_call
    "#;

    let mut timer_vars = HashSet::new();
    let mut cleared_vars = HashSet::new();

    if let Ok(query) = Query::new(language, timer_query) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            if let Some(func) = match_.captures.iter().find(|c| c.index == 0) {
                let func_name = get_node_text(func.node, source);

                if func_name == "setTimeout" || func_name == "setInterval" {
                    // Check if the result is assigned to a variable
                    if let Some(parent) = match_.captures.last().unwrap().node.parent() {
                        if parent.kind() == "variable_declarator"
                            || parent.kind() == "assignment_expression"
                        {
                            // Try to get variable name
                            if let Some(var_name) = extract_variable_name(parent, source) {
                                timer_vars.insert((var_name.to_string(), func_name.to_string()));
                            } else {
                                // Timer not stored, definite leak
                                issues.push(ResourceManagementIssue::TimerLeak {
                                    location: SourceLocation::from_node(
                                        match_.captures.last().unwrap().node,
                                    ),
                                    timer_type: func_name.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Check for clear calls
    if let Ok(query) = Query::new(language, clear_query) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            if let Some(func) = match_.captures.iter().find(|c| c.index == 0) {
                let func_name = get_node_text(func.node, source);

                if func_name == "clearTimeout" || func_name == "clearInterval" {
                    // Try to get the argument (timer ID variable)
                    if let Some(call_node) = match_.captures.last() {
                        if let Some(args) = call_node.node.child_by_field_name("arguments") {
                            if let Some(first_arg) = args.child(1) {
                                let arg_text = get_node_text(first_arg, source);
                                cleared_vars.insert(arg_text.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // Report timers that weren't cleared
    for (var_name, timer_type) in timer_vars {
        if !cleared_vars.contains(&var_name) {
            issues.push(ResourceManagementIssue::TimerLeak {
                location: SourceLocation::from_node(root), // Simplified location
                timer_type,
            });
        }
    }
}

fn detect_websocket_leaks(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    issues: &mut Vec<ResourceManagementIssue>,
) {
    let ws_query = r#"
    (new_expression
      constructor: (identifier) @constructor (#eq? @constructor "WebSocket")
    ) @ws_creation
    "#;

    let close_query = r#"
    (call_expression
      function: (member_expression
        property: (property_identifier) @method (#eq? @method "close")
      )
    ) @close_call
    "#;

    let mut websocket_count = 0;
    let mut close_count = 0;

    if let Ok(query) = Query::new(language, ws_query) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());
        while matches.next().is_some() {
            websocket_count += 1;
        }
    }

    if let Ok(query) = Query::new(language, close_query) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());
        while matches.next().is_some() {
            close_count += 1;
        }
    }

    if websocket_count > close_count {
        issues.push(ResourceManagementIssue::WebSocketLeak {
            location: SourceLocation::from_node(root),
        });
    }
}

fn detect_worker_leaks(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    issues: &mut Vec<ResourceManagementIssue>,
) {
    let worker_query = r#"
    (new_expression
      constructor: (identifier) @constructor
    ) @worker_creation
    "#;

    let terminate_query = r#"
    (call_expression
      function: (member_expression
        property: (property_identifier) @method (#eq? @method "terminate")
      )
    ) @terminate_call
    "#;

    let mut workers = Vec::new();

    if let Ok(query) = Query::new(language, worker_query) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            if let Some(constructor) = match_.captures.iter().find(|c| c.index == 0) {
                let constructor_name = get_node_text(constructor.node, source);

                if constructor_name == "Worker"
                    || constructor_name == "SharedWorker"
                    || constructor_name == "ServiceWorker"
                {
                    workers.push((
                        constructor_name.to_string(),
                        SourceLocation::from_node(match_.captures.last().unwrap().node),
                    ));
                }
            }
        }
    }

    let mut terminate_count = 0;
    if let Ok(query) = Query::new(language, terminate_query) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());
        while matches.next().is_some() {
            terminate_count += 1;
        }
    }

    if workers.len() > terminate_count {
        for (worker_type, location) in workers.into_iter().skip(terminate_count) {
            issues.push(ResourceManagementIssue::WorkerLeak {
                location,
                worker_type,
            });
        }
    }
}

// Pure function to check if a constructor is an observer type
fn is_observer_type(constructor_name: &str) -> bool {
    matches!(
        constructor_name,
        "MutationObserver" | "IntersectionObserver" | "ResizeObserver" | "PerformanceObserver"
    )
}

// Pure function to extract observer locations from query matches
fn extract_observer_locations(
    query: &Query,
    root: Node,
    source: &str,
) -> HashMap<String, Vec<SourceLocation>> {
    let mut observer_count = HashMap::new();
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, root, source.as_bytes());

    while let Some(match_) = matches.next() {
        if let Some(constructor) = match_.captures.iter().find(|c| c.index == 0) {
            let constructor_name = get_node_text(constructor.node, source);

            if is_observer_type(constructor_name) {
                let location = SourceLocation::from_node(match_.captures.last().unwrap().node);
                observer_count
                    .entry(constructor_name.to_string())
                    .or_insert(Vec::new())
                    .push(location);
            }
        }
    }

    observer_count
}

// Pure function to count disconnect calls
fn count_disconnect_calls(query: &Query, root: Node, source: &str) -> usize {
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, root, source.as_bytes());
    let mut count = 0;
    while matches.next().is_some() {
        count += 1;
    }
    count
}

// Pure function to identify leaked observers
fn identify_leaked_observers(
    observer_count: HashMap<String, Vec<SourceLocation>>,
    disconnect_count: usize,
) -> Vec<ResourceManagementIssue> {
    observer_count
        .into_iter()
        .filter(|(_, locations)| locations.len() > disconnect_count)
        .filter_map(|(observer_type, locations)| {
            locations
                .first()
                .map(|location| ResourceManagementIssue::ObserverLeak {
                    location: location.clone(),
                    observer_type,
                })
        })
        .collect()
}

fn detect_observer_leaks(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    issues: &mut Vec<ResourceManagementIssue>,
) {
    let observer_query = r#"
    (new_expression
      constructor: (identifier) @constructor
    ) @observer_creation
    "#;

    let disconnect_query = r#"
    (call_expression
      function: (member_expression
        property: (property_identifier) @method (#eq? @method "disconnect")
      )
    ) @disconnect_call
    "#;

    // Extract observer locations
    let observer_count = if let Ok(query) = Query::new(language, observer_query) {
        extract_observer_locations(&query, root, source)
    } else {
        HashMap::new()
    };

    // Count disconnect calls
    let disconnect_count = if let Ok(query) = Query::new(language, disconnect_query) {
        count_disconnect_calls(&query, root, source)
    } else {
        0
    };

    // Identify and report leaked observers
    let leaked_observers = identify_leaked_observers(observer_count, disconnect_count);
    issues.extend(leaked_observers);
}

fn detect_memory_retention(
    root: Node,
    source: &str,
    language: &tree_sitter::Language,
    issues: &mut Vec<ResourceManagementIssue>,
) {
    // Detect large closures that might retain memory
    let closure_query = r#"
    (arrow_function
      body: (block_statement) @body
    ) @closure
    "#;

    if let Ok(query) = Query::new(language, closure_query) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());

        while let Some(match_) = matches.next() {
            if let Some(body) = match_.captures.iter().find(|c| c.index == 0) {
                let body_text = get_node_text(body.node, source);

                // Check for potential memory retention patterns
                if body_text.len() > 500 && contains_external_references(body_text) {
                    issues.push(ResourceManagementIssue::MemoryRetention {
                        location: SourceLocation::from_node(body.node),
                        cause: "Large closure capturing external scope".to_string(),
                    });
                }
            }
        }
    }
}

// Helper functions
fn extract_variable_name<'a>(node: Node, source: &'a str) -> Option<&'a str> {
    if node.kind() == "variable_declarator" {
        if let Some(name_node) = node.child_by_field_name("name") {
            return Some(get_node_text(name_node, source));
        }
    } else if node.kind() == "assignment_expression" {
        if let Some(left_node) = node.child_by_field_name("left") {
            return Some(get_node_text(left_node, source));
        }
    }
    None
}

fn contains_external_references(text: &str) -> bool {
    // Simple heuristic: check for common patterns that indicate external references
    text.contains("this.")
        || text.contains("window.")
        || text.contains("document.")
        || text.lines().count() > 20 // Large closures likely capture external scope
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_observer_type() {
        // Test valid observer types
        assert!(is_observer_type("MutationObserver"));
        assert!(is_observer_type("IntersectionObserver"));
        assert!(is_observer_type("ResizeObserver"));
        assert!(is_observer_type("PerformanceObserver"));

        // Test invalid observer types
        assert!(!is_observer_type("Observer"));
        assert!(!is_observer_type("CustomObserver"));
        assert!(!is_observer_type(""));
        assert!(!is_observer_type("MutationListener"));
    }

    #[test]
    fn test_identify_leaked_observers_no_leaks() {
        let mut observer_count = HashMap::new();
        observer_count.insert(
            "MutationObserver".to_string(),
            vec![SourceLocation {
                line: 10,
                column: Some(5),
                end_line: Some(10),
                end_column: Some(15),
            }],
        );

        // One observer, one disconnect - no leak
        let leaked = identify_leaked_observers(observer_count, 1);
        assert!(leaked.is_empty());
    }

    #[test]
    fn test_identify_leaked_observers_with_leaks() {
        let mut observer_count = HashMap::new();
        let location = SourceLocation {
            line: 10,
            column: Some(5),
            end_line: Some(10),
            end_column: Some(15),
        };
        observer_count.insert(
            "MutationObserver".to_string(),
            vec![location.clone(), location.clone()],
        );

        // Two observers, one disconnect - one leak
        let leaked = identify_leaked_observers(observer_count, 1);
        assert_eq!(leaked.len(), 1);

        if let ResourceManagementIssue::ObserverLeak { observer_type, .. } = &leaked[0] {
            assert_eq!(observer_type, "MutationObserver");
        } else {
            panic!("Expected ObserverLeak issue");
        }
    }

    #[test]
    fn test_identify_leaked_observers_multiple_types() {
        let mut observer_count = HashMap::new();

        let location1 = SourceLocation {
            line: 10,
            column: Some(5),
            end_line: Some(10),
            end_column: Some(15),
        };
        let location2 = SourceLocation {
            line: 20,
            column: Some(5),
            end_line: Some(20),
            end_column: Some(15),
        };

        // Two mutation observers, one resize observer
        observer_count.insert(
            "MutationObserver".to_string(),
            vec![location1.clone(), location1.clone()],
        );
        observer_count.insert("ResizeObserver".to_string(), vec![location2]);

        // One disconnect - both types leak
        let leaked = identify_leaked_observers(observer_count, 0);
        assert_eq!(leaked.len(), 2);

        let observer_types: Vec<String> = leaked
            .iter()
            .map(|issue| {
                if let ResourceManagementIssue::ObserverLeak { observer_type, .. } = issue {
                    observer_type.clone()
                } else {
                    String::new()
                }
            })
            .collect();

        assert!(observer_types.contains(&"MutationObserver".to_string()));
        assert!(observer_types.contains(&"ResizeObserver".to_string()));
    }

    #[test]
    fn test_contains_external_references() {
        // Test cases with external references
        assert!(contains_external_references("this.property = value"));
        assert!(contains_external_references("window.location.href"));
        assert!(contains_external_references("document.getElementById"));

        // Test case with large closure (>20 lines)
        let large_text = (0..25)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(contains_external_references(&large_text));

        // Test cases without external references
        assert!(!contains_external_references("const x = 5;"));
        assert!(!contains_external_references(
            "function add(a, b) { return a + b; }"
        ));
        assert!(!contains_external_references(""));
    }
}
