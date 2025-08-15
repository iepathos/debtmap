use debtmap::analyzers::rust::RustAnalyzer;
use debtmap::analyzers::rust_call_graph::extract_call_graph;
use debtmap::analyzers::Analyzer;
use debtmap::priority::call_graph::FunctionId;
use debtmap::priority::unified_scorer::is_dead_code_with_exclusions;
use std::collections::HashSet;
use std::path::PathBuf;

#[test]
fn test_field_access_chain_not_dead_code() {
    // This test reproduces the exact pattern from CallGraphBuilder
    // where self.enhanced_graph.framework_patterns.analyze_file() is called
    let rust_code = r#"
use std::path::Path;
use anyhow::Result;

pub struct FrameworkPatternDetector {
    patterns: Vec<String>,
}

impl FrameworkPatternDetector {
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    // This method is called through field access chain but marked as dead code
    pub fn analyze_file(&mut self, file_path: &Path, content: &str) -> Result<()> {
        println!("Analyzing {}", file_path.display());
        self.patterns.push(content.to_string());
        Ok(())
    }
}

pub struct EnhancedGraph {
    pub framework_patterns: FrameworkPatternDetector,
}

impl EnhancedGraph {
    pub fn new() -> Self {
        Self {
            framework_patterns: FrameworkPatternDetector::new(),
        }
    }
}

pub struct CallGraphBuilder {
    enhanced_graph: EnhancedGraph,
    config: AnalysisConfig,
}

pub struct AnalysisConfig {
    pub enable_framework_patterns: bool,
}

impl CallGraphBuilder {
    pub fn new() -> Self {
        Self {
            enhanced_graph: EnhancedGraph::new(),
            config: AnalysisConfig {
                enable_framework_patterns: true,
            },
        }
    }

    // This is the exact pattern from the real code
    pub fn analyze_framework_patterns(
        &mut self,
        file_path: &Path,
        content: &str,
    ) -> Result<&mut Self> {
        if self.config.enable_framework_patterns {
            // This call through field access chain should be recognized
            self.enhanced_graph
                .framework_patterns
                .analyze_file(file_path, content)?;
        }
        Ok(self)
    }
}

fn main() {
    let mut builder = CallGraphBuilder::new();
    let path = Path::new("test.rs");
    builder.analyze_framework_patterns(path, "test content").unwrap();
}
"#;

    let analyzer = RustAnalyzer::new();
    let path = PathBuf::from("test.rs");
    let ast = analyzer
        .parse(rust_code, path.clone())
        .expect("Parse should succeed");
    let metrics = analyzer.analyze(&ast);

    // Parse and extract call graph
    let syntax_tree = syn::parse_file(rust_code).expect("Should parse Rust code");
    let call_graph = extract_call_graph(&syntax_tree, &path);

    // Find the analyze_file function in the metrics
    let analyze_file_func = metrics
        .complexity
        .functions
        .iter()
        .find(|f| f.name.contains("FrameworkPatternDetector::analyze_file"))
        .expect("Should find FrameworkPatternDetector::analyze_file");

    // Create function ID using actual line number
    let analyze_file_id = FunctionId {
        file: path.clone(),
        name: "FrameworkPatternDetector::analyze_file".to_string(),
        line: analyze_file_func.line,
    };

    // Check if it's incorrectly marked as dead code
    let is_dead = is_dead_code_with_exclusions(
        analyze_file_func,
        &call_graph,
        &analyze_file_id,
        &HashSet::new(),
    );

    // The function should NOT be dead code because it's called through field access chain
    assert!(
        !is_dead,
        "FrameworkPatternDetector::analyze_file should not be marked as dead code - it's called via self.enhanced_graph.framework_patterns.analyze_file()"
    );
}

#[test]
fn test_nested_field_access_chain() {
    // Test even deeper nesting
    let rust_code = r#"
pub struct DeepStruct {
    value: i32,
}

impl DeepStruct {
    pub fn process(&mut self) -> i32 {
        self.value += 1;
        self.value
    }
}

pub struct MiddleStruct {
    pub deep: DeepStruct,
}

pub struct TopStruct {
    pub middle: MiddleStruct,
}

pub struct Container {
    top: TopStruct,
}

impl Container {
    pub fn new() -> Self {
        Self {
            top: TopStruct {
                middle: MiddleStruct {
                    deep: DeepStruct { value: 0 },
                },
            },
        }
    }

    pub fn do_work(&mut self) -> i32 {
        // Deep field access chain
        self.top.middle.deep.process()
    }
}

fn main() {
    let mut container = Container::new();
    let result = container.do_work();
    println!("Result: {}", result);
}
"#;

    let analyzer = RustAnalyzer::new();
    let path = PathBuf::from("test.rs");
    let ast = analyzer
        .parse(rust_code, path.clone())
        .expect("Parse should succeed");
    let metrics = analyzer.analyze(&ast);

    let syntax_tree = syn::parse_file(rust_code).expect("Should parse Rust code");
    let call_graph = extract_call_graph(&syntax_tree, &path);

    // Find DeepStruct::process in metrics
    let process_func = metrics
        .complexity
        .functions
        .iter()
        .find(|f| f.name.contains("DeepStruct::process"))
        .expect("Should find DeepStruct::process");

    let process_id = FunctionId {
        file: path.clone(),
        name: "DeepStruct::process".to_string(),
        line: process_func.line,
    };

    let is_dead =
        is_dead_code_with_exclusions(process_func, &call_graph, &process_id, &HashSet::new());

    assert!(
        !is_dead,
        "DeepStruct::process should not be marked as dead code - it's called via self.top.middle.deep.process()"
    );
}

#[test]
fn test_field_access_with_trait_methods() {
    // Test field access chain with trait implementations
    let rust_code = r#"
trait Processor {
    fn process(&mut self) -> i32;
}

pub struct Worker {
    count: i32,
}

impl Processor for Worker {
    fn process(&mut self) -> i32 {
        self.count += 1;
        self.count
    }
}

pub struct Manager {
    pub worker: Worker,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            worker: Worker { count: 0 },
        }
    }

    pub fn execute(&mut self) -> i32 {
        // Trait method called through field
        self.worker.process()
    }
}

fn main() {
    let mut manager = Manager::new();
    manager.execute();
}
"#;

    let analyzer = RustAnalyzer::new();
    let path = PathBuf::from("test.rs");
    let ast = analyzer
        .parse(rust_code, path.clone())
        .expect("Parse should succeed");
    let metrics = analyzer.analyze(&ast);

    let syntax_tree = syn::parse_file(rust_code).expect("Should parse Rust code");
    let call_graph = extract_call_graph(&syntax_tree, &path);

    // Find Worker::process (trait implementation) in metrics
    let process_func = metrics
        .complexity
        .functions
        .iter()
        .find(|f| f.name.contains("Worker::process"))
        .expect("Should find Worker::process");

    let process_id = FunctionId {
        file: path.clone(),
        name: "Worker::process".to_string(),
        line: process_func.line,
    };

    let is_dead =
        is_dead_code_with_exclusions(process_func, &call_graph, &process_id, &HashSet::new());

    assert!(
        !is_dead,
        "Worker::process (trait impl) should not be marked as dead code - it's called via self.worker.process()"
    );
}
