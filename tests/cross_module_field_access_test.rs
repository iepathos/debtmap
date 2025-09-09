use debtmap::analyzers::rust_call_graph::extract_call_graph_multi_file;
use std::path::PathBuf;

fn create_framework_patterns_code() -> &'static str {
    r#"
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

    // This method should NOT be marked as dead code
    // It's called from mod.rs through field access chain
    pub fn analyze_file(&mut self, file_path: &Path, content: &str) -> Result<()> {
        println!("Analyzing {}", file_path.display());
        self.patterns.push(content.to_string());
        Ok(())
    }
    
    pub fn get_patterns(&self) -> &[String] {
        &self.patterns
    }
}
"#
}

fn create_mod_code() -> &'static str {
    r#"
use std::path::Path;
use anyhow::Result;

// Import from the other module (simulated)
pub struct FrameworkPatternDetector;

pub struct RustCallGraph {
    pub framework_patterns: FrameworkPatternDetector,
}

impl RustCallGraph {
    pub fn new() -> Self {
        Self {
            framework_patterns: FrameworkPatternDetector,
        }
    }
}

pub struct AnalysisConfig {
    pub enable_framework_patterns: bool,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            enable_framework_patterns: true,
        }
    }
}

pub struct RustCallGraphBuilder {
    config: AnalysisConfig,
    enhanced_graph: RustCallGraph,
}

impl RustCallGraphBuilder {
    pub fn new() -> Self {
        Self {
            config: AnalysisConfig::default(),
            enhanced_graph: RustCallGraph::new(),
        }
    }

    // This method contains the cross-module field access chain call
    pub fn analyze_framework_patterns(
        &mut self,
        file_path: &Path,
        content: &str,
    ) -> Result<&mut Self> {
        if self.config.enable_framework_patterns {
            // This is the critical call through field access chain
            // that's being incorrectly marked as dead code
            self.enhanced_graph
                .framework_patterns
                .analyze_file(file_path, content)?;
        }
        Ok(self)
    }
}

fn main() {
    let mut builder = RustCallGraphBuilder::new();
    let path = std::path::Path::new("test.rs");
    builder.analyze_framework_patterns(path, "test content").unwrap();
}
"#
}

fn parse_and_extract_call_graph(
    framework_code: &str,
    mod_code: &str,
) -> debtmap::priority::call_graph::CallGraph {
    let framework_file =
        syn::parse_file(framework_code).expect("Should parse framework_patterns code");
    let mod_file = syn::parse_file(mod_code).expect("Should parse mod code");

    let files = vec![
        (framework_file, PathBuf::from("framework_patterns.rs")),
        (mod_file, PathBuf::from("mod.rs")),
    ];

    extract_call_graph_multi_file(&files)
}

fn find_analyze_file_function(
    call_graph: &debtmap::priority::call_graph::CallGraph,
) -> debtmap::priority::call_graph::FunctionId {
    let all_functions = call_graph.find_all_functions();
    all_functions
        .into_iter()
        .find(|f| f.name.contains("analyze_file"))
        .expect("Should find analyze_file function")
}

fn verify_analyze_file_has_callers(
    call_graph: &debtmap::priority::call_graph::CallGraph,
    analyze_file: debtmap::priority::call_graph::FunctionId,
) {
    let callers = call_graph.get_callers(&analyze_file);
    assert!(
        !callers.is_empty(),
        "FrameworkPatternDetector::analyze_file should have callers from RustCallGraphBuilder::analyze_framework_patterns"
    );
}

#[test]
fn test_cross_module_field_access_chain() {
    // This test reproduces the exact cross-module pattern from the actual codebase
    // where RustCallGraphBuilder in mod.rs calls FrameworkPatternDetector::analyze_file
    // in framework_patterns.rs through a field access chain

    let framework_code = create_framework_patterns_code();
    let mod_code = create_mod_code();

    let call_graph = parse_and_extract_call_graph(framework_code, mod_code);
    let analyze_file = find_analyze_file_function(&call_graph);

    verify_analyze_file_has_callers(&call_graph, analyze_file);
}

fn create_realistic_framework_patterns_code() -> &'static str {
    r#"
use std::path::Path;
use anyhow::Result;
use im::Vector;

#[derive(Debug, Clone)]
pub struct Pattern {
    pub name: String,
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct FrameworkPatternDetector {
    patterns: Vector<Pattern>,
}

impl FrameworkPatternDetector {
    pub fn new() -> Self {
        Self {
            patterns: Vector::new(),
        }
    }

    pub fn analyze_file(&mut self, file_path: &Path, _ast: &syn::File) -> Result<()> {
        self.patterns.push_back(Pattern {
            name: "test_pattern".to_string(),
            file: file_path.display().to_string(),
        });
        Ok(())
    }
    
    pub fn get_patterns(&self) -> Vector<Pattern> {
        self.patterns.clone()
    }
}

impl Default for FrameworkPatternDetector {
    fn default() -> Self {
        Self::new()
    }
}
"#
}

fn create_realistic_mod_code() -> &'static str {
    r#"
mod framework_patterns;

use framework_patterns::FrameworkPatternDetector;
use std::path::Path;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct RustCallGraph {
    pub framework_patterns: FrameworkPatternDetector,
}

impl RustCallGraph {
    pub fn new() -> Self {
        Self {
            framework_patterns: FrameworkPatternDetector::new(),
        }
    }
}

pub struct RustCallGraphBuilder {
    enhanced_graph: RustCallGraph,
    enable_framework_patterns: bool,
}

impl RustCallGraphBuilder {
    pub fn new() -> Self {
        Self {
            enhanced_graph: RustCallGraph::new(),
            enable_framework_patterns: true,
        }
    }

    pub fn analyze_framework_patterns(
        &mut self,
        file_path: &Path,
        ast: &syn::File,
    ) -> Result<&mut Self> {
        if self.enable_framework_patterns {
            // Cross-module call through private field access chain
            self.enhanced_graph
                .framework_patterns
                .analyze_file(file_path, ast)?;
        }
        Ok(self)
    }
    
    pub fn build(self) -> RustCallGraph {
        self.enhanced_graph
    }
}

pub fn process_file(path: &Path) -> Result<()> {
    let mut builder = RustCallGraphBuilder::new();
    let ast = syn::File {
        shebang: None,
        attrs: vec![],
        items: vec![],
    };
    builder.analyze_framework_patterns(path, &ast)?;
    let _graph = builder.build();
    Ok(())
}
"#
}

fn debug_print_functions(call_graph: &debtmap::priority::call_graph::CallGraph) {
    let all_functions = call_graph.find_all_functions();
    println!("All functions found:");
    for func in &all_functions {
        println!("  - {} at {}:{}", func.name, func.file.display(), func.line);
    }
}

fn debug_print_analyze_file_info(
    analyze_file: &debtmap::priority::call_graph::FunctionId,
    callers: &[debtmap::priority::call_graph::FunctionId],
) {
    println!(
        "\nChecking function: {} at {}:{}",
        analyze_file.name,
        analyze_file.file.display(),
        analyze_file.line
    );
    println!("Callers of analyze_file: {:?}", callers);
}

#[test]
fn test_cross_module_with_actual_imports() {
    // Even more realistic test with proper module structure

    let framework_code = create_realistic_framework_patterns_code();
    let mod_code = create_realistic_mod_code();

    let call_graph = parse_and_extract_call_graph(framework_code, mod_code);
    debug_print_functions(&call_graph);

    let analyze_file = find_analyze_file_function(&call_graph);
    let callers = call_graph.get_callers(&analyze_file);

    debug_print_analyze_file_info(&analyze_file, &callers);

    assert!(
        !callers.is_empty(),
        "FrameworkPatternDetector::analyze_file should have at least one caller (from RustCallGraphBuilder::analyze_framework_patterns)"
    );
}

fn create_inner_module_code() -> &'static str {
    r#"
pub struct DeepType {
    value: i32,
}

impl DeepType {
    pub fn new() -> Self {
        Self { value: 0 }
    }
    
    pub fn process(&mut self) -> i32 {
        self.value += 1;
        self.value
    }
}
"#
}

fn create_middle_module_code() -> &'static str {
    r#"
pub struct DeepType;

pub struct MiddleType {
    pub deep: DeepType,
}

impl MiddleType {
    pub fn new() -> Self {
        Self {
            deep: DeepType,
        }
    }
}
"#
}

fn create_outer_module_code() -> &'static str {
    r#"
pub struct MiddleType;

pub struct OuterType {
    middle: MiddleType,
}

impl OuterType {
    pub fn new() -> Self {
        Self {
            middle: MiddleType,
        }
    }
    
    pub fn do_work(&mut self) -> i32 {
        // Deep cross-module field access chain
        self.middle.deep.process()
    }
}

fn main() {
    let mut outer = OuterType::new();
    let _result = outer.do_work();
}
"#
}

fn parse_three_modules(
    inner_code: &str,
    middle_code: &str,
    outer_code: &str,
) -> debtmap::priority::call_graph::CallGraph {
    let inner_file = syn::parse_file(inner_code).expect("Should parse inner module");
    let middle_file = syn::parse_file(middle_code).expect("Should parse middle module");
    let outer_file = syn::parse_file(outer_code).expect("Should parse outer module");

    let files = vec![
        (inner_file, PathBuf::from("inner.rs")),
        (middle_file, PathBuf::from("middle.rs")),
        (outer_file, PathBuf::from("outer.rs")),
    ];

    extract_call_graph_multi_file(&files)
}

fn find_process_function(
    call_graph: &debtmap::priority::call_graph::CallGraph,
) -> debtmap::priority::call_graph::FunctionId {
    let all_functions = call_graph.find_all_functions();
    all_functions
        .into_iter()
        .find(|f| f.name.contains("DeepType::process"))
        .expect("Should find DeepType::process")
}

fn verify_process_has_callers(
    call_graph: &debtmap::priority::call_graph::CallGraph,
    process_func: debtmap::priority::call_graph::FunctionId,
) {
    let callers = call_graph.get_callers(&process_func);
    assert!(
        !callers.is_empty(),
        "DeepType::process should have callers through the cross-module field access chain"
    );
}

#[test]
fn test_deep_nested_cross_module_access() {
    // Test even deeper nesting across modules

    let inner_code = create_inner_module_code();
    let middle_code = create_middle_module_code();
    let outer_code = create_outer_module_code();

    let call_graph = parse_three_modules(inner_code, middle_code, outer_code);
    let process_func = find_process_function(&call_graph);

    verify_process_has_callers(&call_graph, process_func);
}
