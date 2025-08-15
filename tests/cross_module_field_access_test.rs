use debtmap::analyzers::rust_call_graph::extract_call_graph_multi_file;
use debtmap::priority::call_graph::FunctionId;
use debtmap::priority::unified_scorer::is_dead_code_with_exclusions;
use std::collections::HashSet;
use std::path::PathBuf;

#[test]
fn test_cross_module_field_access_chain() {
    // This test reproduces the exact cross-module pattern from the actual codebase
    // where RustCallGraphBuilder in mod.rs calls FrameworkPatternDetector::analyze_file
    // in framework_patterns.rs through a field access chain

    // Module 1: framework_patterns.rs equivalent
    let framework_patterns_code = r#"
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
"#;

    // Module 2: mod.rs equivalent
    let mod_code = r#"
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
"#;

    // Parse both files
    let framework_file =
        syn::parse_file(framework_patterns_code).expect("Should parse framework_patterns code");
    let mod_file = syn::parse_file(mod_code).expect("Should parse mod code");

    // Simulate multi-file extraction as done in the actual codebase
    let files = vec![
        (framework_file, PathBuf::from("framework_patterns.rs")),
        (mod_file, PathBuf::from("mod.rs")),
    ];

    let call_graph = extract_call_graph_multi_file(&files);

    // Find the analyze_file function from framework_patterns
    let analyze_file_id = FunctionId {
        file: PathBuf::from("framework_patterns.rs"),
        name: "FrameworkPatternDetector::analyze_file".to_string(),
        line: 13, // Approximate line number
    };

    // Check if it's marked as dead code
    // First, get all functions in the call graph
    let all_functions = call_graph.find_all_functions();
    let analyze_file_exists = all_functions.iter().any(|f| {
        f.name.contains("analyze_file") && f.file.to_str().unwrap().contains("framework_patterns")
    });

    assert!(
        analyze_file_exists,
        "FrameworkPatternDetector::analyze_file should be found in the call graph"
    );

    // Now check if it has any callers
    let callers = call_graph.get_callers(&analyze_file_id);
    assert!(
        !callers.is_empty(),
        "FrameworkPatternDetector::analyze_file should have callers from RustCallGraphBuilder::analyze_framework_patterns"
    );

    // Additional check: verify it's not marked as dead code
    // This would require the FunctionMetrics, but we can at least check the call graph
    let has_incoming_calls = !call_graph.get_callers(&analyze_file_id).is_empty();
    assert!(
        has_incoming_calls,
        "FrameworkPatternDetector::analyze_file should have incoming calls in the call graph"
    );
}

#[test]
fn test_cross_module_with_actual_imports() {
    // Even more realistic test with proper module structure

    // File 1: src/framework_patterns.rs
    let framework_patterns_code = r#"
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
"#;

    // File 2: src/mod.rs (using the framework_patterns module)
    let mod_code = r#"
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
"#;

    // Parse both files
    let framework_file =
        syn::parse_file(framework_patterns_code).expect("Should parse framework_patterns code");
    let mod_file = syn::parse_file(mod_code).expect("Should parse mod code");

    // Create multi-file call graph
    let files = vec![
        (framework_file, PathBuf::from("src/framework_patterns.rs")),
        (mod_file, PathBuf::from("src/mod.rs")),
    ];

    let call_graph = extract_call_graph_multi_file(&files);

    // Debug: Print all functions found
    let all_functions = call_graph.find_all_functions();
    println!("All functions found:");
    for func in &all_functions {
        println!("  - {} at {}:{}", func.name, func.file.display(), func.line);
    }

    // Find analyze_file
    let analyze_file = all_functions
        .iter()
        .find(|f| f.name.contains("analyze_file"))
        .expect("Should find analyze_file function");

    println!(
        "\nChecking function: {} at {}:{}",
        analyze_file.name,
        analyze_file.file.display(),
        analyze_file.line
    );

    // Check for callers
    let callers = call_graph.get_callers(analyze_file);
    println!("Callers of analyze_file: {:?}", callers);

    assert!(
        !callers.is_empty(),
        "FrameworkPatternDetector::analyze_file should have at least one caller (from RustCallGraphBuilder::analyze_framework_patterns)"
    );
}

#[test]
fn test_deep_nested_cross_module_access() {
    // Test even deeper nesting across modules

    // Module 1: Inner type
    let inner_module = r#"
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
"#;

    // Module 2: Middle type that uses Inner
    let middle_module = r#"
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
"#;

    // Module 3: Outer type that uses Middle
    let outer_module = r#"
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
"#;

    // Parse all modules
    let inner_file = syn::parse_file(inner_module).expect("Should parse inner module");
    let middle_file = syn::parse_file(middle_module).expect("Should parse middle module");
    let outer_file = syn::parse_file(outer_module).expect("Should parse outer module");

    // Create multi-file call graph
    let files = vec![
        (inner_file, PathBuf::from("inner.rs")),
        (middle_file, PathBuf::from("middle.rs")),
        (outer_file, PathBuf::from("outer.rs")),
    ];

    let call_graph = extract_call_graph_multi_file(&files);

    // Find DeepType::process
    let all_functions = call_graph.find_all_functions();
    let process_func = all_functions
        .iter()
        .find(|f| f.name.contains("DeepType::process"))
        .expect("Should find DeepType::process");

    // Check if it has callers (it should, from OuterType::do_work)
    let callers = call_graph.get_callers(process_func);

    assert!(
        !callers.is_empty(),
        "DeepType::process should have callers through the cross-module field access chain"
    );
}
