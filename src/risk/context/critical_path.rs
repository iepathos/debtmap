use super::{AnalysisTarget, Context, ContextDetails, ContextProvider};
use crate::priority::call_graph::CallGraph;
use anyhow::Result;
use im::{HashSet, Vector};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Entry point into the application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPoint {
    pub function_name: String,
    pub file_path: PathBuf,
    pub entry_type: EntryType,
    pub is_user_facing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EntryType {
    Main,
    ApiEndpoint,
    EventHandler,
    CliCommand,
    TestEntry,
    WebHandler,
}

// CallGraph is now imported from crate::priority::call_graph
// The main CallGraph provides string-based methods for critical path analysis

/// Represents a critical execution path
#[derive(Debug, Clone)]
pub struct CriticalPath {
    pub entry: EntryPoint,
    pub functions: Vector<String>,
    pub weight: f64,
    pub user_facing: bool,
}

/// Analyzes critical paths in the codebase
pub struct CriticalPathAnalyzer {
    entry_points: Vector<EntryPoint>,
    call_graph: CallGraph,
}

impl Default for CriticalPathAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl CriticalPathAnalyzer {
    pub fn new() -> Self {
        Self {
            entry_points: Vector::new(),
            call_graph: CallGraph::new(),
        }
    }

    /// Detect entry points based on function names and patterns
    pub fn detect_entry_points(&mut self, functions: &[(String, PathBuf)]) {
        for (name, path) in functions {
            let entry_type = Self::classify_entry_point(name, path);
            if let Some(entry_type) = entry_type {
                let is_user_facing = matches!(
                    entry_type,
                    EntryType::Main
                        | EntryType::ApiEndpoint
                        | EntryType::CliCommand
                        | EntryType::WebHandler
                );

                self.entry_points.push_back(EntryPoint {
                    function_name: name.clone(),
                    file_path: path.clone(),
                    entry_type,
                    is_user_facing,
                });
            }
        }
    }

    fn classify_entry_point(name: &str, path: &Path) -> Option<EntryType> {
        let path_str = path.to_string_lossy();

        type ClassifierFn = fn(&str, &str) -> bool;
        let classifiers: [(ClassifierFn, EntryType); 6] = [
            (is_main_function, EntryType::Main),
            (is_cli_command, EntryType::CliCommand),
            (is_api_endpoint, EntryType::ApiEndpoint),
            (is_web_handler, EntryType::WebHandler),
            (is_event_handler, EntryType::EventHandler),
            (is_test_entry, EntryType::TestEntry),
        ];

        classifiers
            .into_iter()
            .find_map(|(classifier, entry_type)| classifier(name, &path_str).then_some(entry_type))
    }
}

fn is_main_function(name: &str, _path: &str) -> bool {
    name == "main"
}

fn is_cli_command(name: &str, path: &str) -> bool {
    let has_cli_path = path.contains("cli") || path.contains("command");
    let has_cli_name =
        name.starts_with("cmd_") || name.starts_with("command_") || name.ends_with("_command");
    has_cli_path && has_cli_name
}

fn is_api_endpoint(name: &str, path: &str) -> bool {
    let has_api_path = path.contains("api") || path.contains("handler") || path.contains("route");
    let has_api_name = name.starts_with("handle_")
        || name.ends_with("_handler")
        || name.starts_with("get_")
        || name.starts_with("post_")
        || name.starts_with("put_")
        || name.starts_with("delete_");
    has_api_path && has_api_name
}

fn is_web_handler(name: &str, path: &str) -> bool {
    let has_web_path = path.contains("web") || path.contains("http");
    let has_handler_name = name.contains("route") || name.contains("handler");
    has_web_path && has_handler_name
}

fn is_event_handler(name: &str, _path: &str) -> bool {
    name.starts_with("on_") || name.ends_with("_listener") || name.contains("event")
}

fn is_test_entry(name: &str, path: &str) -> bool {
    path.contains("test") || name.starts_with("test_") || name.contains("test")
}

impl CriticalPathAnalyzer {
    /// Analyze critical paths from all entry points
    pub fn analyze_paths(&self) -> Vector<CriticalPath> {
        let mut paths = Vector::new();

        for entry in &self.entry_points {
            let traversal = self.trace_from_entry(entry);
            paths.push_back(CriticalPath {
                entry: entry.clone(),
                functions: traversal,
                weight: self.calculate_path_weight(entry),
                user_facing: entry.is_user_facing,
            });
        }

        paths
    }

    fn trace_from_entry(&self, entry: &EntryPoint) -> Vector<String> {
        let mut visited = HashSet::new();
        let mut path = Vector::new();

        self.dfs_trace(&entry.function_name, &mut visited, &mut path);
        path
    }

    fn dfs_trace(&self, function: &str, visited: &mut HashSet<String>, path: &mut Vector<String>) {
        if visited.contains(function) {
            return;
        }

        visited.insert(function.to_string());
        path.push_back(function.to_string());

        for callee in self.call_graph.get_callees_by_name(function) {
            self.dfs_trace(&callee, visited, path);
        }
    }

    fn calculate_path_weight(&self, entry: &EntryPoint) -> f64 {
        match entry.entry_type {
            EntryType::Main => 10.0,
            EntryType::ApiEndpoint => 8.0,
            EntryType::CliCommand => 7.0,
            EntryType::WebHandler => 7.0,
            EntryType::EventHandler => 5.0,
            EntryType::TestEntry => 2.0,
        }
    }

    /// Check if a function is on any critical path
    pub fn is_on_critical_path(&self, function_name: &str) -> bool {
        let paths = self.analyze_paths();
        paths
            .iter()
            .any(|path| path.functions.contains(&function_name.to_string()))
    }

    /// Get all entry points that lead to a function
    pub fn get_entry_points_for(&self, function_name: &str) -> Vector<EntryPoint> {
        let paths = self.analyze_paths();
        paths
            .iter()
            .filter(|path| path.functions.contains(&function_name.to_string()))
            .map(|path| path.entry.clone())
            .collect()
    }
}

/// Context provider for critical path analysis
pub struct CriticalPathProvider {
    analyzer: CriticalPathAnalyzer,
}

impl CriticalPathProvider {
    pub fn new(analyzer: CriticalPathAnalyzer) -> Self {
        Self { analyzer }
    }
}

impl CriticalPathProvider {
    /// Calculate contribution based on path weight and user-facing status
    fn calculate_contribution(max_weight: f64, is_user_facing: bool) -> f64 {
        let base_contribution = max_weight / 10.0;
        if is_user_facing {
            base_contribution * 2.0 // Double contribution for user-facing paths
        } else {
            base_contribution
        }
    }

    /// Build context for empty entry points
    fn build_empty_context(&self) -> Context {
        Context {
            provider: self.name().to_string(),
            weight: self.weight(),
            contribution: 0.0,
            details: ContextDetails::CriticalPath {
                entry_points: vec![],
                path_weight: 0.0,
                is_user_facing: false,
            },
        }
    }
}

impl ContextProvider for CriticalPathProvider {
    fn name(&self) -> &str {
        "critical_path"
    }

    fn gather(&self, target: &AnalysisTarget) -> Result<Context> {
        let entry_points = self.analyzer.get_entry_points_for(&target.function_name);

        if entry_points.is_empty() {
            return Ok(self.build_empty_context());
        }

        let is_user_facing = entry_points.iter().any(|e| e.is_user_facing);
        let max_weight = entry_points
            .iter()
            .map(|e| self.analyzer.calculate_path_weight(e))
            .fold(0.0, f64::max);

        let contribution = Self::calculate_contribution(max_weight, is_user_facing);

        Ok(Context {
            provider: self.name().to_string(),
            weight: self.weight(),
            contribution,
            details: ContextDetails::CriticalPath {
                entry_points: entry_points
                    .iter()
                    .map(|e| format!("{} ({:?})", e.function_name, e.entry_type))
                    .collect(),
                path_weight: max_weight,
                is_user_facing,
            },
        })
    }

    fn weight(&self) -> f64 {
        1.5 // Critical paths have high weight
    }

    fn explain(&self, context: &Context) -> String {
        if let ContextDetails::CriticalPath {
            entry_points,
            path_weight,
            is_user_facing,
        } = &context.details
        {
            if entry_points.is_empty() {
                "Not on any critical path".to_string()
            } else {
                format!(
                    "On {} critical path(s) with weight {:.1}{}",
                    entry_points.len(),
                    path_weight,
                    if *is_user_facing {
                        " (user-facing)"
                    } else {
                        ""
                    }
                )
            }
        } else {
            "No critical path information".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_point_detection() {
        let mut analyzer = CriticalPathAnalyzer::new();

        let functions = vec![
            ("main".to_string(), PathBuf::from("src/main.rs")),
            (
                "handle_request".to_string(),
                PathBuf::from("src/api/handler.rs"),
            ),
            ("test_something".to_string(), PathBuf::from("tests/test.rs")),
            ("on_message".to_string(), PathBuf::from("src/events.rs")),
            ("cmd_run".to_string(), PathBuf::from("src/cli/commands.rs")),
        ];

        analyzer.detect_entry_points(&functions);

        assert_eq!(analyzer.entry_points.len(), 5);
        assert!(analyzer
            .entry_points
            .iter()
            .any(|e| e.entry_type == EntryType::Main));
        assert!(analyzer
            .entry_points
            .iter()
            .any(|e| e.entry_type == EntryType::ApiEndpoint));
        assert!(analyzer
            .entry_points
            .iter()
            .any(|e| e.entry_type == EntryType::TestEntry));
        assert!(analyzer
            .entry_points
            .iter()
            .any(|e| e.entry_type == EntryType::EventHandler));
        assert!(analyzer
            .entry_points
            .iter()
            .any(|e| e.entry_type == EntryType::CliCommand));
    }

    #[test]
    fn test_call_graph() {
        let mut graph = CallGraph::new();

        graph.add_edge_by_name(
            "main".to_string(),
            "init".to_string(),
            PathBuf::from("main.rs"),
        );
        graph.add_edge_by_name(
            "init".to_string(),
            "setup".to_string(),
            PathBuf::from("init.rs"),
        );
        graph.add_edge_by_name(
            "main".to_string(),
            "run".to_string(),
            PathBuf::from("main.rs"),
        );

        let callees = graph.get_callees_by_name("main");
        assert_eq!(callees.len(), 2);
        assert!(callees.contains(&"init".to_string()));
        assert!(callees.contains(&"run".to_string()));

        let callers = graph.get_callers_by_name("init");
        assert_eq!(callers.len(), 1);
        assert!(callers.contains(&"main".to_string()));
    }

    #[test]
    fn test_calculate_path_weight_main() {
        let analyzer = CriticalPathAnalyzer::new();
        let entry = EntryPoint {
            function_name: "main".to_string(),
            file_path: PathBuf::from("src/main.rs"),
            entry_type: EntryType::Main,
            is_user_facing: true,
        };

        let weight = analyzer.calculate_path_weight(&entry);
        assert_eq!(weight, 10.0);
    }

    #[test]
    fn test_calculate_path_weight_api_endpoint() {
        let analyzer = CriticalPathAnalyzer::new();
        let entry = EntryPoint {
            function_name: "handle_request".to_string(),
            file_path: PathBuf::from("src/api.rs"),
            entry_type: EntryType::ApiEndpoint,
            is_user_facing: true,
        };

        let weight = analyzer.calculate_path_weight(&entry);
        assert_eq!(weight, 8.0);
    }

    #[test]
    fn test_calculate_path_weight_cli_command() {
        let analyzer = CriticalPathAnalyzer::new();
        let entry = EntryPoint {
            function_name: "cmd_run".to_string(),
            file_path: PathBuf::from("src/cli.rs"),
            entry_type: EntryType::CliCommand,
            is_user_facing: true,
        };

        let weight = analyzer.calculate_path_weight(&entry);
        assert_eq!(weight, 7.0);
    }

    #[test]
    fn test_calculate_path_weight_web_handler() {
        let analyzer = CriticalPathAnalyzer::new();
        let entry = EntryPoint {
            function_name: "handle_web".to_string(),
            file_path: PathBuf::from("src/web.rs"),
            entry_type: EntryType::WebHandler,
            is_user_facing: true,
        };

        let weight = analyzer.calculate_path_weight(&entry);
        assert_eq!(weight, 7.0);
    }

    #[test]
    fn test_calculate_path_weight_event_handler() {
        let analyzer = CriticalPathAnalyzer::new();
        let entry = EntryPoint {
            function_name: "on_message".to_string(),
            file_path: PathBuf::from("src/events.rs"),
            entry_type: EntryType::EventHandler,
            is_user_facing: false,
        };

        let weight = analyzer.calculate_path_weight(&entry);
        assert_eq!(weight, 5.0);
    }

    #[test]
    fn test_calculate_path_weight_test_entry() {
        let analyzer = CriticalPathAnalyzer::new();
        let entry = EntryPoint {
            function_name: "test_something".to_string(),
            file_path: PathBuf::from("tests/test.rs"),
            entry_type: EntryType::TestEntry,
            is_user_facing: false,
        };

        let weight = analyzer.calculate_path_weight(&entry);
        assert_eq!(weight, 2.0);
    }

    #[test]
    fn test_calculate_contribution_non_user_facing() {
        let max_weight = 5.0;
        let contribution = CriticalPathProvider::calculate_contribution(max_weight, false);
        assert_eq!(contribution, 0.5);
    }

    #[test]
    fn test_calculate_contribution_user_facing() {
        let max_weight = 5.0;
        let contribution = CriticalPathProvider::calculate_contribution(max_weight, true);
        assert_eq!(contribution, 1.0);
    }

    #[test]
    fn test_calculate_contribution_zero_weight() {
        assert_eq!(
            CriticalPathProvider::calculate_contribution(0.0, false),
            0.0
        );
        assert_eq!(CriticalPathProvider::calculate_contribution(0.0, true), 0.0);
    }

    #[test]
    fn test_gather_with_no_entry_points() {
        let analyzer = CriticalPathAnalyzer::new();
        let provider = CriticalPathProvider::new(analyzer);
        let target = AnalysisTarget {
            root_path: PathBuf::from("/project"),
            function_name: "some_function".to_string(),
            file_path: PathBuf::from("src/lib.rs"),
            line_range: (10, 20),
        };

        let result = provider.gather(&target).unwrap();

        assert_eq!(result.provider, "critical_path");
        assert_eq!(result.contribution, 0.0);
        if let ContextDetails::CriticalPath {
            entry_points,
            path_weight,
            is_user_facing,
        } = result.details
        {
            assert!(entry_points.is_empty());
            assert_eq!(path_weight, 0.0);
            assert!(!is_user_facing);
        } else {
            panic!("Expected CriticalPath context details");
        }
    }

    #[test]
    fn test_gather_with_non_user_facing_entry_point() {
        let mut analyzer = CriticalPathAnalyzer::new();

        // Add an event handler entry point (non-user-facing)
        analyzer.entry_points.push_back(EntryPoint {
            function_name: "handle_event".to_string(),
            file_path: PathBuf::from("src/events.rs"),
            entry_type: EntryType::EventHandler,
            is_user_facing: false,
        });

        // Add the function to the call graph
        analyzer.call_graph.add_edge_by_name(
            "handle_event".to_string(),
            "process_data".to_string(),
            PathBuf::from("src/events.rs"),
        );

        let provider = CriticalPathProvider::new(analyzer);
        let target = AnalysisTarget {
            root_path: PathBuf::from("/project"),
            function_name: "handle_event".to_string(),
            file_path: PathBuf::from("src/events.rs"),
            line_range: (20, 40),
        };

        let result = provider.gather(&target).unwrap();

        assert_eq!(result.provider, "critical_path");
        assert_eq!(result.contribution, 0.5); // 5.0 / 10.0 for non-user-facing
        if let ContextDetails::CriticalPath {
            entry_points,
            path_weight,
            is_user_facing,
        } = result.details
        {
            assert_eq!(entry_points.len(), 1);
            assert_eq!(path_weight, 5.0);
            assert!(!is_user_facing);
            assert!(entry_points[0].contains("handle_event"));
        } else {
            panic!("Expected CriticalPath context details");
        }
    }

    #[test]
    fn test_gather_with_user_facing_entry_point() {
        let mut analyzer = CriticalPathAnalyzer::new();

        // Add a main entry point (user-facing)
        analyzer.entry_points.push_back(EntryPoint {
            function_name: "main".to_string(),
            file_path: PathBuf::from("src/main.rs"),
            entry_type: EntryType::Main,
            is_user_facing: true,
        });

        // Add the function to the call graph
        analyzer.call_graph.add_edge_by_name(
            "main".to_string(),
            "init".to_string(),
            PathBuf::from("src/main.rs"),
        );

        let provider = CriticalPathProvider::new(analyzer);
        let target = AnalysisTarget {
            root_path: PathBuf::from("/project"),
            function_name: "main".to_string(),
            file_path: PathBuf::from("src/main.rs"),
            line_range: (1, 10),
        };

        let result = provider.gather(&target).unwrap();

        assert_eq!(result.provider, "critical_path");
        assert_eq!(result.contribution, 2.0); // (10.0 / 10.0) * 2.0 for user-facing
        if let ContextDetails::CriticalPath {
            entry_points,
            path_weight,
            is_user_facing,
        } = result.details
        {
            assert_eq!(entry_points.len(), 1);
            assert_eq!(path_weight, 10.0);
            assert!(is_user_facing);
            assert!(entry_points[0].contains("main"));
        } else {
            panic!("Expected CriticalPath context details");
        }
    }

    #[test]
    fn test_gather_with_multiple_entry_points() {
        let mut analyzer = CriticalPathAnalyzer::new();

        // Add multiple entry points with different weights
        analyzer.entry_points.push_back(EntryPoint {
            function_name: "main".to_string(),
            file_path: PathBuf::from("src/main.rs"),
            entry_type: EntryType::Main,
            is_user_facing: true,
        });

        analyzer.entry_points.push_back(EntryPoint {
            function_name: "handle_api".to_string(),
            file_path: PathBuf::from("src/api.rs"),
            entry_type: EntryType::ApiEndpoint,
            is_user_facing: true,
        });

        // Add shared function to both paths
        analyzer.call_graph.add_edge_by_name(
            "main".to_string(),
            "shared_function".to_string(),
            PathBuf::from("src/main.rs"),
        );
        analyzer.call_graph.add_edge_by_name(
            "handle_api".to_string(),
            "shared_function".to_string(),
            PathBuf::from("src/api.rs"),
        );

        let provider = CriticalPathProvider::new(analyzer);
        let target = AnalysisTarget {
            root_path: PathBuf::from("/project"),
            function_name: "shared_function".to_string(),
            file_path: PathBuf::from("src/shared.rs"),
            line_range: (50, 100),
        };

        let result = provider.gather(&target).unwrap();

        assert_eq!(result.provider, "critical_path");
        // Should use the max weight (main = 10.0) and double it for user-facing
        assert_eq!(result.contribution, 2.0); // (10.0 / 10.0) * 2.0
        if let ContextDetails::CriticalPath {
            entry_points,
            path_weight,
            is_user_facing,
        } = result.details
        {
            assert_eq!(entry_points.len(), 2);
            assert_eq!(path_weight, 10.0); // max of main (10.0) and api (8.0)
            assert!(is_user_facing);
            // Check both entry points are included
            let entry_str = entry_points.join(" ");
            assert!(entry_str.contains("main"));
            assert!(entry_str.contains("handle_api"));
        } else {
            panic!("Expected CriticalPath context details");
        }
    }

    #[test]
    fn test_explain_with_empty_entry_points() {
        let analyzer = CriticalPathAnalyzer::new();
        let provider = CriticalPathProvider::new(analyzer);
        let context = Context {
            provider: "critical_path".to_string(),
            weight: 1.5,
            contribution: 0.0,
            details: ContextDetails::CriticalPath {
                entry_points: vec![],
                path_weight: 0.0,
                is_user_facing: false,
            },
        };

        let explanation = provider.explain(&context);
        assert_eq!(explanation, "Not on any critical path");
    }

    #[test]
    fn test_explain_with_single_path_not_user_facing() {
        let analyzer = CriticalPathAnalyzer::new();
        let provider = CriticalPathProvider::new(analyzer);
        let context = Context {
            provider: "critical_path".to_string(),
            weight: 1.5,
            contribution: 0.5,
            details: ContextDetails::CriticalPath {
                entry_points: vec!["main".to_string()],
                path_weight: 5.0,
                is_user_facing: false,
            },
        };

        let explanation = provider.explain(&context);
        assert_eq!(explanation, "On 1 critical path(s) with weight 5.0");
    }

    #[test]
    fn test_explain_with_multiple_paths_user_facing() {
        let analyzer = CriticalPathAnalyzer::new();
        let provider = CriticalPathProvider::new(analyzer);
        let context = Context {
            provider: "critical_path".to_string(),
            weight: 1.5,
            contribution: 0.8,
            details: ContextDetails::CriticalPath {
                entry_points: vec!["main".to_string(), "handle_request".to_string()],
                path_weight: 10.5,
                is_user_facing: true,
            },
        };

        let explanation = provider.explain(&context);
        assert_eq!(
            explanation,
            "On 2 critical path(s) with weight 10.5 (user-facing)"
        );
    }

    #[test]
    fn test_explain_with_non_critical_path_context() {
        let analyzer = CriticalPathAnalyzer::new();
        let provider = CriticalPathProvider::new(analyzer);
        let context = Context {
            provider: "critical_path".to_string(),
            weight: 1.5,
            contribution: 0.0,
            details: ContextDetails::DependencyChain {
                depth: 3,
                propagated_risk: 0.5,
                dependents: vec![],
                blast_radius: 0,
            },
        };

        let explanation = provider.explain(&context);
        assert_eq!(explanation, "No critical path information");
    }

    #[test]
    fn test_explain_with_multiple_entry_points_formatting() {
        let analyzer = CriticalPathAnalyzer::new();
        let provider = CriticalPathProvider::new(analyzer);
        let context = Context {
            provider: "critical_path".to_string(),
            weight: 1.5,
            contribution: 0.6,
            details: ContextDetails::CriticalPath {
                entry_points: vec![
                    "main".to_string(),
                    "api_handler".to_string(),
                    "cli_command".to_string(),
                ],
                path_weight: 8.7,
                is_user_facing: false,
            },
        };

        let explanation = provider.explain(&context);
        assert_eq!(explanation, "On 3 critical path(s) with weight 8.7");
    }

    #[test]
    fn test_gather_with_complex_dependency_chain() {
        let mut analyzer = CriticalPathAnalyzer::new();

        // Create a complex dependency chain
        // main -> init -> database_setup -> connection_pool
        //             -> config_loader -> file_reader
        analyzer.entry_points.push_back(EntryPoint {
            function_name: "main".to_string(),
            file_path: PathBuf::from("src/main.rs"),
            entry_type: EntryType::Main,
            is_user_facing: true,
        });

        // Build the call graph
        analyzer.call_graph.add_edge_by_name(
            "main".to_string(),
            "init".to_string(),
            PathBuf::from("src/main.rs"),
        );
        analyzer.call_graph.add_edge_by_name(
            "init".to_string(),
            "database_setup".to_string(),
            PathBuf::from("src/init.rs"),
        );
        analyzer.call_graph.add_edge_by_name(
            "database_setup".to_string(),
            "connection_pool".to_string(),
            PathBuf::from("src/database.rs"),
        );
        analyzer.call_graph.add_edge_by_name(
            "init".to_string(),
            "config_loader".to_string(),
            PathBuf::from("src/init.rs"),
        );
        analyzer.call_graph.add_edge_by_name(
            "config_loader".to_string(),
            "file_reader".to_string(),
            PathBuf::from("src/config.rs"),
        );

        let provider = CriticalPathProvider::new(analyzer);

        // Test gathering for a deeply nested function
        let target = AnalysisTarget {
            root_path: PathBuf::from("/project"),
            function_name: "connection_pool".to_string(),
            file_path: PathBuf::from("src/database.rs"),
            line_range: (45, 80),
        };

        let result = provider.gather(&target).unwrap();

        assert_eq!(result.provider, "critical_path");
        assert_eq!(result.contribution, 2.0); // user-facing main function
        if let ContextDetails::CriticalPath {
            entry_points,
            path_weight,
            is_user_facing,
        } = result.details
        {
            assert_eq!(entry_points.len(), 1);
            assert_eq!(path_weight, 10.0);
            assert!(is_user_facing);
            assert!(entry_points[0].contains("main"));
        } else {
            panic!("Expected CriticalPath context details");
        }
    }

    #[test]
    fn test_gather_with_mixed_visibility_and_orphaned_functions() {
        let mut analyzer = CriticalPathAnalyzer::new();

        // Add mix of user-facing and non-user-facing entry points
        analyzer.entry_points.push_back(EntryPoint {
            function_name: "main".to_string(),
            file_path: PathBuf::from("src/main.rs"),
            entry_type: EntryType::Main,
            is_user_facing: true,
        });

        analyzer.entry_points.push_back(EntryPoint {
            function_name: "background_worker".to_string(),
            file_path: PathBuf::from("src/worker.rs"),
            entry_type: EntryType::EventHandler,
            is_user_facing: false,
        });

        analyzer.entry_points.push_back(EntryPoint {
            function_name: "api_endpoint".to_string(),
            file_path: PathBuf::from("src/api.rs"),
            entry_type: EntryType::ApiEndpoint,
            is_user_facing: true,
        });

        // Create a shared utility function called by all entry points
        analyzer.call_graph.add_edge_by_name(
            "main".to_string(),
            "shared_utility".to_string(),
            PathBuf::from("src/main.rs"),
        );
        analyzer.call_graph.add_edge_by_name(
            "background_worker".to_string(),
            "shared_utility".to_string(),
            PathBuf::from("src/worker.rs"),
        );
        analyzer.call_graph.add_edge_by_name(
            "api_endpoint".to_string(),
            "shared_utility".to_string(),
            PathBuf::from("src/api.rs"),
        );

        let provider = CriticalPathProvider::new(analyzer);

        // Test gathering for the shared utility function
        let target = AnalysisTarget {
            root_path: PathBuf::from("/project"),
            function_name: "shared_utility".to_string(),
            file_path: PathBuf::from("src/utils.rs"),
            line_range: (1, 30),
        };

        let result = provider.gather(&target).unwrap();

        assert_eq!(result.provider, "critical_path");
        // Should use max weight (main = 10.0) and double it for user-facing
        assert_eq!(result.contribution, 2.0);
        if let ContextDetails::CriticalPath {
            entry_points,
            path_weight,
            is_user_facing,
        } = result.details
        {
            assert_eq!(entry_points.len(), 3);
            assert_eq!(path_weight, 10.0); // max of main (10.0), event (5.0), api (8.0)
            assert!(is_user_facing); // has user-facing paths

            // Verify all entry points are included
            let entry_str = entry_points.join(" ");
            assert!(entry_str.contains("main"));
            assert!(entry_str.contains("background_worker"));
            assert!(entry_str.contains("api_endpoint"));
        } else {
            panic!("Expected CriticalPath context details");
        }

        // Test edge case: orphaned function not in any path
        let orphaned_target = AnalysisTarget {
            root_path: PathBuf::from("/project"),
            function_name: "orphaned_function".to_string(),
            file_path: PathBuf::from("src/orphaned.rs"),
            line_range: (10, 20),
        };

        let orphaned_result = provider.gather(&orphaned_target).unwrap();
        assert_eq!(orphaned_result.contribution, 0.0);
        if let ContextDetails::CriticalPath { entry_points, .. } = orphaned_result.details {
            assert!(entry_points.is_empty());
        } else {
            panic!("Expected CriticalPath context details");
        }
    }
}
