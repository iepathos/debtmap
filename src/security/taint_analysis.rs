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
    /// Checks if the expression represents CLI argument input
    fn is_cli_argument_source(normalized: &str) -> bool {
        normalized.contains("args()") || normalized.contains("env::args")
    }

    /// Checks if the expression represents environment variable input
    fn is_environment_source(normalized: &str) -> bool {
        normalized.contains("env::var")
    }

    /// Checks if the expression represents HTTP request input
    fn is_http_request_source(normalized: &str) -> bool {
        normalized.contains("Request") || normalized.contains("HttpRequest")
    }

    /// Checks if the expression represents user input from stdin
    fn is_user_input_source(normalized: &str) -> bool {
        normalized.contains("stdin") || normalized.contains("read_line")
    }

    /// Checks if the expression represents file input
    fn is_file_input_source(normalized: &str) -> bool {
        normalized.contains("File::") || normalized.contains("read_")
    }

    /// Classifies an expression string into an input source type
    /// This is a pure function that can be tested in isolation
    fn classify_input_source(expr_str: &str) -> Option<InputSource> {
        // Remove spaces for more reliable matching (quote adds spaces between tokens)
        let normalized = expr_str.replace(" ", "");

        match () {
            _ if Self::is_cli_argument_source(&normalized) => Some(InputSource::CliArgument),
            _ if Self::is_environment_source(&normalized) => Some(InputSource::Environment),
            _ if Self::is_http_request_source(&normalized) => Some(InputSource::HttpRequest),
            // Check for stdin/read_line BEFORE general read_ pattern
            _ if Self::is_user_input_source(&normalized) => Some(InputSource::UserInput),
            _ if Self::is_file_input_source(&normalized) => Some(InputSource::FileInput),
            _ => None,
        }
    }

    fn detect_source(&self, expr: &Expr) -> Option<InputSource> {
        let expr_str = quote::quote!(#expr).to_string();

        // Check if expression matches any known taint source
        let is_taint_source = self
            .analyzer
            .taint_sources
            .iter()
            .any(|source| expr_str.contains(source.as_str()));

        if !is_taint_source {
            return None;
        }

        // Determine the specific type of input source using pure classification function
        Self::classify_input_source(&expr_str)
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

    #[test]
    fn test_classify_input_source_cli_arguments() {
        assert_eq!(
            TaintGraphBuilder::classify_input_source("std::env::args()"),
            Some(InputSource::CliArgument)
        );
        assert_eq!(
            TaintGraphBuilder::classify_input_source("env::args().collect()"),
            Some(InputSource::CliArgument)
        );
        assert_eq!(
            TaintGraphBuilder::classify_input_source("args().nth(2)"),
            Some(InputSource::CliArgument)
        );
    }

    #[test]
    fn test_classify_input_source_environment() {
        assert_eq!(
            TaintGraphBuilder::classify_input_source("env::var(\"HOME\")"),
            Some(InputSource::Environment)
        );
        assert_eq!(
            TaintGraphBuilder::classify_input_source("std::env::var(\"PATH\")"),
            Some(InputSource::Environment)
        );
        assert_eq!(
            TaintGraphBuilder::classify_input_source("env::var_os(\"USER\")"),
            Some(InputSource::Environment)
        );
    }

    #[test]
    fn test_classify_input_source_http_request() {
        assert_eq!(
            TaintGraphBuilder::classify_input_source("Request::new()"),
            Some(InputSource::HttpRequest)
        );
        assert_eq!(
            TaintGraphBuilder::classify_input_source("HttpRequest::from_parts()"),
            Some(InputSource::HttpRequest)
        );
        assert_eq!(TaintGraphBuilder::classify_input_source("req.body()"), None);
        assert_eq!(
            TaintGraphBuilder::classify_input_source("parse_Request_body()"),
            Some(InputSource::HttpRequest)
        );
    }

    #[test]
    fn test_classify_input_source_file_input() {
        assert_eq!(
            TaintGraphBuilder::classify_input_source("File::open(\"data.txt\")"),
            Some(InputSource::FileInput)
        );
        assert_eq!(
            TaintGraphBuilder::classify_input_source("read_to_string(&mut file)"),
            Some(InputSource::FileInput)
        );
        assert_eq!(
            TaintGraphBuilder::classify_input_source("fs::read_file(path)"),
            Some(InputSource::FileInput)
        );
        assert_eq!(
            TaintGraphBuilder::classify_input_source("BufReader::read_line()"),
            Some(InputSource::UserInput)
        );
    }

    #[test]
    fn test_classify_input_source_user_input() {
        assert_eq!(
            TaintGraphBuilder::classify_input_source("io::stdin().read_line()"),
            Some(InputSource::UserInput)
        );
        assert_eq!(
            TaintGraphBuilder::classify_input_source("stdin.lock()"),
            Some(InputSource::UserInput)
        );
        assert_eq!(
            TaintGraphBuilder::classify_input_source("read_line(&mut buffer)"),
            Some(InputSource::UserInput)
        );
    }

    #[test]
    fn test_classify_input_source_no_match() {
        assert_eq!(
            TaintGraphBuilder::classify_input_source("regular_function_call()"),
            None
        );
        assert_eq!(
            TaintGraphBuilder::classify_input_source("calculate_sum(a, b)"),
            None
        );
        assert_eq!(TaintGraphBuilder::classify_input_source(""), None);
        assert_eq!(
            TaintGraphBuilder::classify_input_source("process_data(input)"),
            None
        );
    }

    #[test]
    fn test_classify_input_source_edge_cases() {
        // Test partial matches - "MyRequest" contains "Request" so it matches
        assert_eq!(
            TaintGraphBuilder::classify_input_source("MyRequest"),
            Some(InputSource::HttpRequest)
        );

        // Test case sensitivity - "Request" is case-sensitive
        assert_eq!(TaintGraphBuilder::classify_input_source("REQUEST"), None);

        // Test combined patterns - read_line takes precedence over File::
        assert_eq!(
            TaintGraphBuilder::classify_input_source("File::read_line()"),
            Some(InputSource::UserInput)
        );
    }

    #[test]
    fn test_detect_source_integration() {
        use std::collections::HashSet;
        use syn::parse_quote;

        let mut analyzer = TaintAnalyzer::new();
        analyzer.taint_sources = HashSet::from([
            "args".to_string(),
            "env".to_string(),
            "Request".to_string(),
            "File".to_string(),
            "stdin".to_string(),
        ]);

        let builder = TaintGraphBuilder {
            analyzer: &mut analyzer,
            current_file: PathBuf::from("test.rs"),
            variable_taints: std::collections::HashMap::new(),
        };

        // Test with CLI argument expression
        let expr: Expr = parse_quote!(std::env::args());
        assert_eq!(builder.detect_source(&expr), Some(InputSource::CliArgument));

        // Test with environment variable expression
        let expr: Expr = parse_quote!(env::var("HOME"));
        assert_eq!(builder.detect_source(&expr), Some(InputSource::Environment));

        // Test with file input expression
        let expr: Expr = parse_quote!(File::open("data.txt"));
        assert_eq!(builder.detect_source(&expr), Some(InputSource::FileInput));

        // Test with non-taint expression
        let expr: Expr = parse_quote!(calculate_sum(a, b));
        assert_eq!(builder.detect_source(&expr), None);
    }

    #[test]
    fn test_detect_source_respects_taint_sources() {
        use std::collections::HashSet;
        use syn::parse_quote;

        let mut analyzer = TaintAnalyzer::new();
        // Empty taint sources - should return None for everything
        analyzer.taint_sources = HashSet::new();

        let builder = TaintGraphBuilder {
            analyzer: &mut analyzer,
            current_file: PathBuf::from("test.rs"),
            variable_taints: std::collections::HashMap::new(),
        };

        let expr: Expr = parse_quote!(std::env::args());
        assert_eq!(builder.detect_source(&expr), None);

        // Add "args" to taint sources
        builder.analyzer.taint_sources.insert("args".to_string());
        assert_eq!(builder.detect_source(&expr), Some(InputSource::CliArgument));
    }

    #[test]
    fn test_is_cli_argument_source() {
        // Test with args() pattern
        assert!(TaintGraphBuilder::is_cli_argument_source("args()"));
        assert!(TaintGraphBuilder::is_cli_argument_source(
            "std::env::args()"
        ));
        assert!(TaintGraphBuilder::is_cli_argument_source("env::args"));
        assert!(TaintGraphBuilder::is_cli_argument_source(
            "env::args().collect()"
        ));

        // Test negative cases
        assert!(!TaintGraphBuilder::is_cli_argument_source("environment"));
        assert!(!TaintGraphBuilder::is_cli_argument_source("read_file"));
        assert!(!TaintGraphBuilder::is_cli_argument_source(""));
        assert!(!TaintGraphBuilder::is_cli_argument_source("arg")); // partial match shouldn't work
        assert!(!TaintGraphBuilder::is_cli_argument_source("arguments")); // different word
    }

    #[test]
    fn test_is_environment_source() {
        // Test with env::var pattern
        assert!(TaintGraphBuilder::is_environment_source("env::var"));
        assert!(TaintGraphBuilder::is_environment_source("std::env::var"));
        assert!(TaintGraphBuilder::is_environment_source(
            "env::var(\"HOME\")"
        ));
        assert!(TaintGraphBuilder::is_environment_source("env::var_os"));

        // Test negative cases
        assert!(!TaintGraphBuilder::is_environment_source("env::args"));
        assert!(!TaintGraphBuilder::is_environment_source("environment"));
        assert!(!TaintGraphBuilder::is_environment_source("var"));
        assert!(!TaintGraphBuilder::is_environment_source(""));
        assert!(!TaintGraphBuilder::is_environment_source("getenv"));
    }

    #[test]
    fn test_is_http_request_source() {
        // Test with Request pattern
        assert!(TaintGraphBuilder::is_http_request_source("Request"));
        assert!(TaintGraphBuilder::is_http_request_source("Request::new()"));
        assert!(TaintGraphBuilder::is_http_request_source("HttpRequest"));
        assert!(TaintGraphBuilder::is_http_request_source(
            "HttpRequest::from_parts()"
        ));
        assert!(TaintGraphBuilder::is_http_request_source("MyRequest")); // contains Request
        assert!(TaintGraphBuilder::is_http_request_source(
            "parse_Request_body"
        ));

        // Test negative cases
        assert!(!TaintGraphBuilder::is_http_request_source("request")); // lowercase
        assert!(!TaintGraphBuilder::is_http_request_source("REQUEST")); // uppercase (exact case needed)
        assert!(!TaintGraphBuilder::is_http_request_source(""));
        assert!(!TaintGraphBuilder::is_http_request_source("response"));
        assert!(!TaintGraphBuilder::is_http_request_source("http"));
    }

    #[test]
    fn test_predicate_functions_with_spaces() {
        // Test that the predicates work with normalized input (spaces removed)
        // This tests the integration between classify_input_source normalization and predicates

        // CLI arguments with no spaces
        assert!(TaintGraphBuilder::is_cli_argument_source(
            "env::args().nth(2)"
        ));

        // Environment with no spaces
        assert!(TaintGraphBuilder::is_environment_source(
            "env::var(\"PATH\")"
        ));

        // HTTP request with no spaces
        assert!(TaintGraphBuilder::is_http_request_source(
            "HttpRequest::body()"
        ));
    }

    #[test]
    fn test_predicate_edge_cases() {
        // Test empty strings
        assert!(!TaintGraphBuilder::is_cli_argument_source(""));
        assert!(!TaintGraphBuilder::is_environment_source(""));
        assert!(!TaintGraphBuilder::is_http_request_source(""));

        // Test with special characters
        assert!(TaintGraphBuilder::is_cli_argument_source("::env::args()"));
        assert!(TaintGraphBuilder::is_environment_source(
            "env::var_os(\"USER\")"
        ));
        assert!(TaintGraphBuilder::is_http_request_source("Request<Body>"));

        // Test substring matches work correctly
        assert!(TaintGraphBuilder::is_cli_argument_source(
            "get_args()_from_cli"
        ));
        assert!(TaintGraphBuilder::is_environment_source(
            "read_env::var_from_system"
        ));
    }

    #[test]
    fn test_is_user_input_source() {
        // Test stdin patterns
        assert!(TaintGraphBuilder::is_user_input_source("stdin().read_line"));
        assert!(TaintGraphBuilder::is_user_input_source("io::stdin()"));
        assert!(TaintGraphBuilder::is_user_input_source("std::io::stdin"));

        // Test read_line patterns
        assert!(TaintGraphBuilder::is_user_input_source(
            "buffer.read_line()"
        ));
        assert!(TaintGraphBuilder::is_user_input_source(
            "read_line_from_user"
        ));

        // Test false cases
        assert!(!TaintGraphBuilder::is_user_input_source("read_file"));
        assert!(!TaintGraphBuilder::is_user_input_source(""));
    }

    #[test]
    fn test_is_file_input_source() {
        // Test File:: patterns
        assert!(TaintGraphBuilder::is_file_input_source("File::open"));
        assert!(TaintGraphBuilder::is_file_input_source(
            "std::fs::File::create"
        ));
        assert!(TaintGraphBuilder::is_file_input_source("File::read"));

        // Test read_ patterns
        assert!(TaintGraphBuilder::is_file_input_source("read_to_string"));
        assert!(TaintGraphBuilder::is_file_input_source("read_dir"));
        assert!(TaintGraphBuilder::is_file_input_source("fs::read_file"));

        // Test false cases
        assert!(!TaintGraphBuilder::is_file_input_source("write_file"));
        assert!(!TaintGraphBuilder::is_file_input_source(""));
    }
}
