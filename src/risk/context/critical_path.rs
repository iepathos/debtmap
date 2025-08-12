use super::{AnalysisTarget, Context, ContextDetails, ContextProvider};
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

/// Represents a function call relationship
#[derive(Debug, Clone)]
pub struct CallEdge {
    pub from: String,
    pub to: String,
    pub file: PathBuf,
}

/// Call graph for critical path analysis
#[derive(Debug, Clone)]
pub struct CallGraph {
    edges: Vector<CallEdge>,
    nodes: HashSet<String>,
}

impl Default for CallGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl CallGraph {
    pub fn new() -> Self {
        Self {
            edges: Vector::new(),
            nodes: HashSet::new(),
        }
    }

    pub fn add_edge(&mut self, from: String, to: String, file: PathBuf) {
        self.nodes.insert(from.clone());
        self.nodes.insert(to.clone());
        self.edges.push_back(CallEdge { from, to, file });
    }

    pub fn get_callees(&self, function: &str) -> Vector<String> {
        self.edges
            .iter()
            .filter(|e| e.from == function)
            .map(|e| e.to.clone())
            .collect()
    }

    pub fn get_callers(&self, function: &str) -> Vector<String> {
        self.edges
            .iter()
            .filter(|e| e.to == function)
            .map(|e| e.from.clone())
            .collect()
    }
}

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

        for callee in self.call_graph.get_callees(function) {
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

impl ContextProvider for CriticalPathProvider {
    fn name(&self) -> &str {
        "critical_path"
    }

    fn gather(&self, target: &AnalysisTarget) -> Result<Context> {
        let entry_points = self.analyzer.get_entry_points_for(&target.function_name);

        if entry_points.is_empty() {
            return Ok(Context {
                provider: self.name().to_string(),
                weight: self.weight(),
                contribution: 0.0,
                details: ContextDetails::CriticalPath {
                    entry_points: vec![],
                    path_weight: 0.0,
                    is_user_facing: false,
                },
            });
        }

        let is_user_facing = entry_points.iter().any(|e| e.is_user_facing);
        let max_weight = entry_points
            .iter()
            .map(|e| self.analyzer.calculate_path_weight(e))
            .fold(0.0, f64::max);

        let contribution = if is_user_facing {
            (max_weight / 10.0) * 2.0 // Double contribution for user-facing paths
        } else {
            max_weight / 10.0
        };

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

        graph.add_edge(
            "main".to_string(),
            "init".to_string(),
            PathBuf::from("main.rs"),
        );
        graph.add_edge(
            "init".to_string(),
            "setup".to_string(),
            PathBuf::from("init.rs"),
        );
        graph.add_edge(
            "main".to_string(),
            "run".to_string(),
            PathBuf::from("main.rs"),
        );

        let callees = graph.get_callees("main");
        assert_eq!(callees.len(), 2);
        assert!(callees.contains(&"init".to_string()));
        assert!(callees.contains(&"run".to_string()));

        let callers = graph.get_callers("init");
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
}
