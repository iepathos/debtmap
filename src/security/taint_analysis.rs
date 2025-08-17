use crate::security::types::{InputSource, SecurityVulnerability, Severity, SinkOperation};
use petgraph::algo::all_simple_paths;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File, Local, Pat, PatIdent};

#[derive(Debug, Clone)]
pub struct TaintNode {
    pub id: String,
    pub node_type: TaintNodeType,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct Location {
    pub file: PathBuf,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaintNodeType {
    Source(InputSource),
    Sink(SinkOperation),
    Propagator(String), // Function or variable name
    Sanitizer(String),  // Validation function
}

#[derive(Debug, Clone)]
pub struct TaintPath {
    pub source: InputSource,
    pub sink: SinkOperation,
    pub nodes: Vec<String>,
    pub has_sanitizer: bool,
}

pub struct TaintAnalyzer {
    taint_sources: HashSet<String>,
    taint_sinks: HashSet<String>,
    sanitizers: HashSet<String>,
    taint_graph: DiGraph<TaintNode, String>,
    node_indices: HashMap<String, NodeIndex>,
}

impl Default for TaintAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TaintAnalyzer {
    pub fn new() -> Self {
        let taint_sources = vec![
            "std::env::args",
            "std::io::stdin",
            "reqwest::Request",
            "actix_web::HttpRequest",
            "rocket::Request",
            "hyper::Request",
            "warp::filters",
            "File::open",
            "read_to_string",
            "read_line",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let taint_sinks = vec![
            "std::process::Command",
            "std::fs::File",
            "diesel::sql_query",
            "sqlx::query",
            "execute",
            "raw_query",
            "system",
            "eval",
            "deserialize",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let sanitizers = vec![
            "sanitize", "escape", "validate", "clean", "filter", "check", "verify", "strip",
            "encode",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        Self {
            taint_sources,
            taint_sinks,
            sanitizers,
            taint_graph: DiGraph::new(),
            node_indices: HashMap::new(),
        }
    }

    pub fn analyze_data_flow(
        &mut self,
        file: &File,
        file_path: &Path,
    ) -> Vec<SecurityVulnerability> {
        // Build taint propagation graph
        self.build_taint_graph(file, file_path);

        // Find paths from sources to sinks
        let vulnerable_paths = self.find_taint_paths();

        // Convert paths to vulnerabilities
        vulnerable_paths
            .into_iter()
            .map(|taint_path| {
                let severity = self.assess_path_severity(&taint_path);
                SecurityVulnerability::InputValidationGap {
                    input_source: taint_path.source,
                    sink_operation: taint_path.sink,
                    taint_path: taint_path.nodes,
                    severity,
                    line: 0, // Would need span info
                    file: file_path.to_path_buf(),
                }
            })
            .collect()
    }

    fn build_taint_graph(&mut self, file: &File, path: &Path) {
        let mut visitor = TaintGraphBuilder {
            analyzer: self,
            current_file: path.to_path_buf(),
            variable_taints: HashMap::new(),
        };
        visitor.visit_file(file);
    }

    fn find_taint_paths(&self) -> Vec<TaintPath> {
        let mut vulnerable_paths = Vec::new();

        // Get source and sink nodes
        let source_nodes: Vec<_> = self
            .node_indices
            .iter()
            .filter_map(|(_id, &idx)| {
                if let TaintNodeType::Source(source) = &self.taint_graph[idx].node_type {
                    Some((idx, *source))
                } else {
                    None
                }
            })
            .collect();

        let sink_nodes: Vec<_> = self
            .node_indices
            .iter()
            .filter_map(|(_id, &idx)| {
                if let TaintNodeType::Sink(sink) = &self.taint_graph[idx].node_type {
                    Some((idx, *sink))
                } else {
                    None
                }
            })
            .collect();

        // Find all paths from sources to sinks
        for (source_idx, source_type) in source_nodes {
            for (sink_idx, sink_type) in &sink_nodes {
                let paths: Vec<Vec<NodeIndex>> =
                    all_simple_paths(&self.taint_graph, source_idx, *sink_idx, 0, Some(10))
                        .collect();

                for path in paths {
                    let has_sanitizer = self.path_has_sanitizer(&path);

                    if !has_sanitizer {
                        let node_names: Vec<String> = path
                            .iter()
                            .map(|&idx| self.taint_graph[idx].id.clone())
                            .collect();

                        vulnerable_paths.push(TaintPath {
                            source: source_type,
                            sink: *sink_type,
                            nodes: node_names,
                            has_sanitizer: false,
                        });
                    }
                }
            }
        }

        vulnerable_paths
    }

    fn path_has_sanitizer(&self, path: &[NodeIndex]) -> bool {
        path.iter().any(|&idx| {
            if let TaintNodeType::Sanitizer(_) = self.taint_graph[idx].node_type {
                true
            } else if let TaintNodeType::Propagator(ref name) = self.taint_graph[idx].node_type {
                self.sanitizers
                    .iter()
                    .any(|sanitizer| name.contains(sanitizer))
            } else {
                false
            }
        })
    }

    fn assess_path_severity(&self, path: &TaintPath) -> Severity {
        match (path.source, path.sink) {
            (_, SinkOperation::SqlQuery) => Severity::Critical,
            (_, SinkOperation::ProcessExecution) => Severity::Critical,
            (InputSource::HttpRequest, SinkOperation::FileSystem) => Severity::High,
            (InputSource::UserInput, SinkOperation::Deserialization) => Severity::High,
            (_, SinkOperation::NetworkRequest) => Severity::Medium,
            _ => Severity::Medium,
        }
    }

    fn add_node(&mut self, node: TaintNode) -> NodeIndex {
        let id = node.id.clone();

        if let Some(&existing_idx) = self.node_indices.get(&id) {
            existing_idx
        } else {
            let idx = self.taint_graph.add_node(node);
            self.node_indices.insert(id, idx);
            idx
        }
    }

    fn add_edge(&mut self, from: NodeIndex, to: NodeIndex, label: String) {
        self.taint_graph.add_edge(from, to, label);
    }
}

struct TaintGraphBuilder<'a> {
    analyzer: &'a mut TaintAnalyzer,
    current_file: PathBuf,
    variable_taints: HashMap<String, NodeIndex>,
}

impl<'a, 'ast> Visit<'ast> for TaintGraphBuilder<'a> {
    fn visit_local(&mut self, local: &'ast Local) {
        // Track variable assignments
        if let Pat::Ident(PatIdent { ident, .. }) = &local.pat {
            if let Some(init) = &local.init {
                let var_name = ident.to_string();

                // Check if initialization is from a taint source
                if let Some(source_type) = self.detect_source(&init.expr) {
                    let source_node = TaintNode {
                        id: format!("source_{}", var_name),
                        node_type: TaintNodeType::Source(source_type),
                        location: Location {
                            file: self.current_file.clone(),
                            line: 0,
                        },
                    };

                    let source_idx = self.analyzer.add_node(source_node);

                    let var_node = TaintNode {
                        id: var_name.clone(),
                        node_type: TaintNodeType::Propagator(var_name.clone()),
                        location: Location {
                            file: self.current_file.clone(),
                            line: 0,
                        },
                    };

                    let var_idx = self.analyzer.add_node(var_node);
                    self.analyzer
                        .add_edge(source_idx, var_idx, "assignment".to_string());

                    self.variable_taints.insert(var_name, var_idx);
                }
            }
        }

        syn::visit::visit_local(self, local);
    }

    fn visit_expr_method_call(&mut self, method_call: &'ast ExprMethodCall) {
        let method_name = method_call.method.to_string();

        // Check for sink operations
        if self
            .analyzer
            .taint_sinks
            .iter()
            .any(|sink| method_name.contains(sink))
        {
            if let Some(sink_type) = self.detect_sink(&method_name) {
                let sink_node = TaintNode {
                    id: format!("sink_{}", method_name),
                    node_type: TaintNodeType::Sink(sink_type),
                    location: Location {
                        file: self.current_file.clone(),
                        line: 0,
                    },
                };

                let sink_idx = self.analyzer.add_node(sink_node);

                // Check if any arguments are tainted
                for arg in &method_call.args {
                    if let Some(taint_idx) = self.find_taint_in_expr(arg) {
                        self.analyzer
                            .add_edge(taint_idx, sink_idx, "flow".to_string());
                    }
                }
            }
        }

        // Check for sanitizer functions
        if self
            .analyzer
            .sanitizers
            .iter()
            .any(|san| method_name.contains(san))
        {
            let sanitizer_node = TaintNode {
                id: format!("sanitizer_{}", method_name),
                node_type: TaintNodeType::Sanitizer(method_name),
                location: Location {
                    file: self.current_file.clone(),
                    line: 0,
                },
            };

            self.analyzer.add_node(sanitizer_node);
        }

        syn::visit::visit_expr_method_call(self, method_call);
    }
}

impl<'a> TaintGraphBuilder<'a> {
    fn detect_source(&self, expr: &Expr) -> Option<InputSource> {
        let expr_str = quote::quote!(#expr).to_string();

        // Check if expression matches any known taint source
        let is_taint_source = self
            .analyzer
            .taint_sources
            .iter()
            .any(|source| expr_str.contains(source));

        if !is_taint_source {
            return None;
        }

        // Determine the specific type of input source
        if expr_str.contains("args()") || expr_str.contains("env::args") {
            Some(InputSource::CliArgument)
        } else if expr_str.contains("env::var") {
            Some(InputSource::Environment)
        } else if expr_str.contains("Request") || expr_str.contains("HttpRequest") {
            Some(InputSource::HttpRequest)
        } else if expr_str.contains("File::") || expr_str.contains("read_") {
            Some(InputSource::FileInput)
        } else if expr_str.contains("stdin") || expr_str.contains("read_line") {
            Some(InputSource::UserInput)
        } else {
            None
        }
    }

    fn is_sql_sink(method_name: &str) -> bool {
        method_name.contains("query")
            || method_name.contains("execute")
            || method_name.contains("sql")
    }

    fn is_process_sink(method_name: &str) -> bool {
        method_name.contains("Command") || method_name.contains("system")
    }

    fn is_file_sink(method_name: &str) -> bool {
        method_name.contains("File") || method_name.contains("write")
    }

    fn is_network_sink(method_name: &str) -> bool {
        method_name.contains("request") || method_name.contains("http")
    }

    fn is_deserialization_sink(method_name: &str) -> bool {
        method_name.contains("deserialize") || method_name.contains("from_")
    }

    fn detect_sink(&self, method_name: &str) -> Option<SinkOperation> {
        match () {
            _ if Self::is_sql_sink(method_name) => Some(SinkOperation::SqlQuery),
            _ if Self::is_process_sink(method_name) => Some(SinkOperation::ProcessExecution),
            _ if Self::is_file_sink(method_name) => Some(SinkOperation::FileSystem),
            _ if Self::is_network_sink(method_name) => Some(SinkOperation::NetworkRequest),
            _ if Self::is_deserialization_sink(method_name) => Some(SinkOperation::Deserialization),
            _ => None,
        }
    }

    fn find_taint_in_expr(&self, expr: &Expr) -> Option<NodeIndex> {
        let expr_str = quote::quote!(#expr).to_string();

        for (var_name, &idx) in &self.variable_taints {
            if expr_str.contains(var_name) {
                return Some(idx);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_taint_analyzer_creation() {
        let analyzer = TaintAnalyzer::new();
        assert!(!analyzer.taint_sources.is_empty());
        assert!(!analyzer.taint_sinks.is_empty());
        assert!(!analyzer.sanitizers.is_empty());
    }

    #[test]
    fn test_severity_assessment() {
        let analyzer = TaintAnalyzer::new();

        let path = TaintPath {
            source: InputSource::HttpRequest,
            sink: SinkOperation::SqlQuery,
            nodes: vec!["input".to_string(), "query".to_string()],
            has_sanitizer: false,
        };

        assert_eq!(analyzer.assess_path_severity(&path), Severity::Critical);

        let path2 = TaintPath {
            source: InputSource::FileInput,
            sink: SinkOperation::NetworkRequest,
            nodes: vec!["file".to_string(), "request".to_string()],
            has_sanitizer: false,
        };

        assert_eq!(analyzer.assess_path_severity(&path2), Severity::Medium);
    }

    #[test]
    fn test_is_sql_sink() {
        assert!(TaintGraphBuilder::is_sql_sink("execute_query"));
        assert!(TaintGraphBuilder::is_sql_sink("run_sql"));
        assert!(TaintGraphBuilder::is_sql_sink("query_database"));
        assert!(TaintGraphBuilder::is_sql_sink("execute"));
        assert!(!TaintGraphBuilder::is_sql_sink("read_file"));
        assert!(!TaintGraphBuilder::is_sql_sink("send_request"));
    }

    #[test]
    fn test_is_process_sink() {
        assert!(TaintGraphBuilder::is_process_sink("Command::new"));
        assert!(TaintGraphBuilder::is_process_sink("system_call"));
        assert!(TaintGraphBuilder::is_process_sink("run_system"));
        assert!(!TaintGraphBuilder::is_process_sink("query_database"));
        assert!(!TaintGraphBuilder::is_process_sink("write_file"));
    }

    #[test]
    fn test_is_file_sink() {
        assert!(TaintGraphBuilder::is_file_sink("File::create"));
        assert!(TaintGraphBuilder::is_file_sink("write_to_disk"));
        assert!(TaintGraphBuilder::is_file_sink("File::open"));
        assert!(TaintGraphBuilder::is_file_sink("write_bytes"));
        assert!(!TaintGraphBuilder::is_file_sink("execute_query"));
        assert!(!TaintGraphBuilder::is_file_sink("send_request"));
    }

    #[test]
    fn test_is_network_sink() {
        assert!(TaintGraphBuilder::is_network_sink("send_request"));
        assert!(TaintGraphBuilder::is_network_sink("http_post"));
        assert!(TaintGraphBuilder::is_network_sink("make_http_call"));
        assert!(TaintGraphBuilder::is_network_sink("request_api"));
        assert!(!TaintGraphBuilder::is_network_sink("write_file"));
        assert!(!TaintGraphBuilder::is_network_sink("execute_query"));
    }

    #[test]
    fn test_is_deserialization_sink() {
        assert!(TaintGraphBuilder::is_deserialization_sink(
            "deserialize_json"
        ));
        assert!(TaintGraphBuilder::is_deserialization_sink("from_str"));
        assert!(TaintGraphBuilder::is_deserialization_sink("from_bytes"));
        assert!(TaintGraphBuilder::is_deserialization_sink(
            "parse_from_string"
        ));
        assert!(!TaintGraphBuilder::is_deserialization_sink("write_file"));
        assert!(!TaintGraphBuilder::is_deserialization_sink("execute_query"));
    }

    #[test]
    fn test_detect_sink_integration() {
        let mut analyzer = TaintAnalyzer::new();
        let builder = TaintGraphBuilder {
            analyzer: &mut analyzer,
            current_file: PathBuf::from("test.rs"),
            variable_taints: std::collections::HashMap::new(),
        };

        assert_eq!(
            builder.detect_sink("execute_query"),
            Some(SinkOperation::SqlQuery)
        );
        assert_eq!(
            builder.detect_sink("Command::new"),
            Some(SinkOperation::ProcessExecution)
        );
        assert_eq!(
            builder.detect_sink("File::write"),
            Some(SinkOperation::FileSystem)
        );
        assert_eq!(
            builder.detect_sink("send_http_request"),
            Some(SinkOperation::NetworkRequest)
        );
        assert_eq!(
            builder.detect_sink("deserialize_input"),
            Some(SinkOperation::Deserialization)
        );
        assert_eq!(builder.detect_sink("regular_function"), None);
    }

    #[test]
    fn test_detect_sink_edge_cases() {
        let mut analyzer = TaintAnalyzer::new();
        let builder = TaintGraphBuilder {
            analyzer: &mut analyzer,
            current_file: PathBuf::from("test.rs"),
            variable_taints: std::collections::HashMap::new(),
        };

        assert_eq!(builder.detect_sink(""), None);
        assert_eq!(builder.detect_sink("random_method"), None);
        assert_eq!(builder.detect_sink("query"), Some(SinkOperation::SqlQuery));
        assert_eq!(
            builder.detect_sink("execute"),
            Some(SinkOperation::SqlQuery)
        );
        assert_eq!(builder.detect_sink("File"), Some(SinkOperation::FileSystem));
    }
}
