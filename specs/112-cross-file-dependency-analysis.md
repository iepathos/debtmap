---
number: 112
title: Cross-File Dependency Analysis
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-10-16
updated: 2025-10-16
---

# Specification 112: Cross-File Dependency Analysis

**Category**: foundation
**Priority**: critical
**Status**: draft (updated with improvements)
**Dependencies**: None

## Update Summary (2025-10-16)

This specification has been enhanced with the following improvements based on technical evaluation:

**Added Features**:
- ✅ Confidence scoring system (High/Medium/Low/Unknown) for all findings
- ✅ Circuit breaker for circular import detection (max depth: 10)
- ✅ Fallback behavior for unresolvable imports (conservative approach)
- ✅ Span tracking for precise error reporting
- ✅ Lazy symbol resolution with caching for performance
- ✅ Streaming API for memory-efficient large project analysis
- ✅ Property-based tests for invariant verification
- ✅ Regression test for bug #10 (ConversationManager scenario)

**Enhanced Architecture**:
- Immutable builder pattern for `SymbolResolver`
- `Arc<SymbolResolver>` for shared, thread-safe symbol resolution
- `DashMap` for concurrent resolution cache
- Confidence-based dead code detection with configurable thresholds

**Documented Limitations**:
- Object flow analysis scope (module-level singletons only)
- Edge case handling (circular imports, wildcards, dynamic imports)
- Future enhancement roadmap (specs 112.1-112.6)

**Improved Testing**:
- 6 integration test cases with confidence expectations
- 5+ property-based tests for determinism and invariants
- Explicit regression protection for bug #10

## Context

Debtmap v0.2.8 analyzes Python codebases for technical debt but suffers from a **critical limitation**: it only tracks function calls within individual files, missing cross-file dependencies. This leads to dangerous false positives where actively used functions are incorrectly flagged as "dead code" that can be safely removed.

**Real-World Impact from Bug Report**:
- **Critical False Positive #10**: `ConversationManager.add_message()` flagged as dead code
  - Actually called from `mainwindow.py` via singleton instance
  - Removing this would break the entire application
- **Impact**: 50% false positive rate in dead code detection
- **Root Cause**: No analysis of import statements or cross-module usage patterns

**Current Behavior**:
```python
# conversation_manager.py
class ConversationManager:
    def add_message(self, text, sender):  # ❌ Flagged as dead code
        """Add a new message to the end of the current conversation."""
        message, index = self.current_conversation.add_message(text, sender)
        return index

manager = ConversationManager()  # Singleton instance

# mainwindow.py
from conversation_manager import manager
manager.add_message(message, "user")  # ✅ Actually used here!
```

**What Debtmap Currently Sees**:
- `add_message()` has no callers in `conversation_manager.py` → mark as dead code
- Misses the `import manager` and usage in `mainwindow.py`

**Why This is Critical**:
- Users may trust recommendations and remove actively used functions
- Breaks applications in production
- Erodes trust in all debtmap analysis
- Makes dead code detection unusable without extensive manual verification

## Objective

Implement project-wide call graph analysis that tracks function usage across file boundaries, eliminating false positives caused by cross-file dependencies. This will reduce false positive rate from 50% to < 5% for dead code detection.

## Requirements

### Functional Requirements

1. **Import Statement Analysis**
   - Parse all Python import statements (`import`, `from X import Y`, `from X import Y as Z`)
   - Build module-level dependency graph
   - Track which symbols are imported from which modules
   - Handle relative imports (`from . import`, `from .. import`)
   - Support wildcard imports (`from X import *`)

2. **Cross-File Call Graph Construction**
   - Build complete project-wide call graph
   - Track function calls across module boundaries
   - Identify singleton/global instance usage patterns
   - Follow variable assignments to track instance method calls
   - Support attribute access chains (`obj.method()`, `module.obj.method()`)

3. **Symbol Resolution**
   - Map imported names to their source definitions
   - Resolve aliased imports (`import foo as bar`)
   - Handle name shadowing and local vs imported symbols
   - Track class instantiation and method invocations

4. **Instance Method Tracking**
   - Detect singleton pattern (module-level instances)
   - Track class instantiation across files
   - Follow instance variable assignments
   - Connect method definitions to cross-file usage

5. **Dead Code Detection Improvements**
   - Mark function as "used" if called from any file in project
   - Distinguish between "truly unused" and "public API"
   - Report usage locations for detected calls
   - Flag low-confidence findings separately

### Non-Functional Requirements

1. **Performance**
   - Cross-file analysis completes in < 2 seconds for 1000-file project
   - Memory usage < 500MB for 1000-file project (use `Arc<str>` for path deduplication)
   - Incremental analysis for changed files (with spec 102)
   - Parallel file parsing using `rayon`
   - Streaming processing for large projects (don't load all ASTs at once)
   - Lazy symbol resolution with caching

2. **Accuracy**
   - False positive rate < 5% for dead code detection
   - Zero false negatives (never mark used code as dead)
   - Handle 95% of common Python import patterns
   - Graceful degradation for complex dynamic imports
   - Confidence scoring for all findings (High/Medium/Low/Unknown)
   - Circuit breaker for circular import resolution (max depth: 10)

3. **Maintainability**
   - Clear separation of import parsing and call graph construction
   - Language-agnostic import analysis abstraction
   - Extensible to JavaScript/TypeScript (future)
   - Comprehensive test coverage (> 90%)
   - Property-based tests for invariants
   - Well-documented algorithms

## Acceptance Criteria

- [ ] Parser extracts all import statements from Python files
- [ ] Import graph maps symbols to source modules
- [ ] Call graph tracks function calls across file boundaries
- [ ] Singleton pattern usage detected (module-level instances)
- [ ] `ConversationManager.add_message()` example no longer flagged as dead code
- [ ] Cross-file method calls correctly attributed to function definitions
- [ ] Aliased imports resolved correctly (`import foo as bar`)
- [ ] Relative imports handled correctly
- [ ] Wildcard imports marked as low-confidence (many potential users)
- [ ] Confidence scoring implemented for all findings (High/Medium/Low/Unknown)
- [ ] Circular import detection with max depth circuit breaker (10 levels)
- [ ] Fallback behavior for unresolvable imports (mark as unknown confidence)
- [ ] False positive rate < 5% on promptconstruct-frontend codebase
- [ ] Performance < 2 seconds for 1000-file Python project
- [ ] Memory usage < 500MB for 1000-file project
- [ ] Property-based tests for resolution invariants
- [ ] Integration tests validate cross-file analysis
- [ ] Regression test for bug #10 (ConversationManager scenario)
- [ ] Documentation includes architecture diagrams

## Technical Details

### Implementation Approach

**Phase 1: Import Statement Parsing**
1. Add Python import parser in `src/analyzers/python/import_parser.rs`
2. Extract all import types: `import`, `from...import`, relative imports
3. Build per-file import map: `HashMap<PathBuf, Vec<Import>>`

**Phase 2: Symbol Resolution**
1. Create module resolver in `src/analysis/symbol_resolver.rs`
2. Map imported names to source file definitions
3. Build bidirectional symbol-to-file mapping
4. Handle Python module search paths and `__init__.py`

**Phase 3: Cross-File Call Graph**
1. Extend call graph builder in `src/analysis/call_graph/builder.rs`
2. Add cross-file edges to call graph
3. Track instance method calls via imported objects
4. Detect singleton pattern (module-level assignments)

**Phase 4: Dead Code Detection Update**
1. Modify dead code detector in `src/debt/dead_code.rs`
2. Check cross-file usage before marking as dead
3. Report usage locations in output
4. Add confidence levels for findings

### Architecture Changes

```rust
// src/analyzers/python/import_parser.rs
pub struct PythonImportParser {
    file_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportType {
    Module { name: String, alias: Option<String> },
    FromModule { module: String, names: Vec<ImportName>, level: usize },
    Wildcard { module: String, level: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportName {
    pub name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Import {
    pub import_type: ImportType,
    pub line: usize,
    pub source_file: PathBuf,
}

impl PythonImportParser {
    pub fn parse_imports(content: &str, file_path: &Path) -> Result<Vec<Import>>;
    pub fn resolve_relative_import(import: &Import, current_file: &Path) -> Result<PathBuf>;
}

// src/analysis/symbol_resolver.rs
pub struct SymbolResolver {
    // Map from (file, symbol_name) to definition location
    definitions: HashMap<(PathBuf, String), Definition>,
    // Map from (file, imported_name) to original (file, symbol_name)
    imports: HashMap<(PathBuf, String), (PathBuf, String)>,
    // Module-level instances (singletons)
    singletons: HashMap<(PathBuf, String), ClassType>,
    // Lazy resolution cache for performance
    resolution_cache: DashMap<(PathBuf, String), Option<(Definition, ResolutionConfidence)>>,
    // Track resolution depth to prevent circular import deadlock
    max_resolution_depth: usize,
}

#[derive(Debug, Clone)]
pub struct Definition {
    pub file: PathBuf,
    pub symbol: String,
    pub kind: DefinitionKind,
    pub line: usize,
    pub span: Option<Span>,  // For precise error reporting
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DefinitionKind {
    Function,
    Class,
    Method { class_name: String },
    Variable,
    ModuleInstance { class_name: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResolutionConfidence {
    High,      // Direct import with clear path
    Medium,    // Relative import, resolved via heuristics
    Low,       // Wildcard import, dynamic import patterns
    Unknown,   // Cannot resolve statically
}

impl SymbolResolver {
    pub fn new() -> Self;
    pub fn with_max_depth(max_depth: usize) -> Self;

    // Immutable builder pattern
    pub fn with_definition(self, file: PathBuf, symbol: String, kind: DefinitionKind) -> Self;
    pub fn with_import(self, import: Import, source_file: PathBuf) -> Self;
    pub fn build(self) -> Arc<SymbolResolver>;

    // Resolution with confidence scoring
    pub fn resolve_symbol(&self, file: &Path, name: &str) -> Option<(Definition, ResolutionConfidence)>;
    pub fn resolve_with_depth(&self, file: &Path, name: &str, depth: usize) -> Result<(Definition, ResolutionConfidence)>;
    pub fn get_all_callers(&self, target: &Definition) -> Vec<CallSite>;
}

// src/analysis/call_graph/cross_file.rs
pub struct CrossFileCallGraph {
    // Extends CallGraph with cross-file edges
    call_graph: CallGraph,
    symbol_resolver: Arc<SymbolResolver>,
    // Map from function definition to all call sites (including cross-file)
    callers: HashMap<FunctionId, Vec<CallSite>>,
}

#[derive(Debug, Clone)]
pub struct CallSite {
    pub caller_file: PathBuf,
    pub caller_function: Option<String>,
    pub callee_file: PathBuf,
    pub callee_function: String,
    pub line: usize,
    pub confidence: ResolutionConfidence,  // Confidence level for this call
    pub span: Option<Span>,  // Precise location for error reporting
}

impl CrossFileCallGraph {
    pub fn build(files: &[PathBuf], analyses: &[FileAnalysis], resolver: Arc<SymbolResolver>) -> Result<Self>;
    pub fn get_cross_file_callers(&self, function: &FunctionDef) -> Vec<CallSite>;
    pub fn is_function_used(&self, function: &FunctionDef) -> bool;
    pub fn is_function_used_with_confidence(&self, function: &FunctionDef, min_confidence: ResolutionConfidence) -> bool;
    pub fn get_usage_locations(&self, function: &FunctionDef) -> Vec<Location>;
}

// Update src/debt/dead_code.rs
pub struct DeadCodeDetector {
    cross_file_graph: CrossFileCallGraph,
    min_confidence_threshold: ResolutionConfidence,
}

#[derive(Debug, Clone)]
pub struct DeadCodeFinding {
    pub function: FunctionDef,
    pub confidence: ResolutionConfidence,
    pub reason: String,
    pub usage_locations: Vec<CallSite>,  // Include potential usage sites
}

impl DeadCodeDetector {
    pub fn new(cross_file_graph: CrossFileCallGraph) -> Self {
        Self {
            cross_file_graph,
            min_confidence_threshold: ResolutionConfidence::Low,
        }
    }

    pub fn with_confidence_threshold(mut self, threshold: ResolutionConfidence) -> Self {
        self.min_confidence_threshold = threshold;
        self
    }

    pub fn detect_with_cross_file_analysis(
        &self,
        function: &FunctionDef,
    ) -> Option<DeadCodeFinding> {
        // Check cross-file usage with confidence threshold
        if self.cross_file_graph.is_function_used_with_confidence(
            function,
            self.min_confidence_threshold
        ) {
            return None;
        }

        let usage_locations = self.cross_file_graph.get_cross_file_callers(function);

        // If there are low-confidence usages, report them
        if !usage_locations.is_empty() {
            let max_confidence = usage_locations
                .iter()
                .map(|site| site.confidence)
                .max()
                .unwrap_or(ResolutionConfidence::Unknown);

            return Some(DeadCodeFinding {
                function: function.clone(),
                confidence: max_confidence,
                reason: format!(
                    "Function may be used ({} confidence) - found {} potential call sites",
                    format!("{:?}", max_confidence).to_lowercase(),
                    usage_locations.len()
                ),
                usage_locations,
            });
        }

        Some(DeadCodeFinding {
            function: function.clone(),
            confidence: ResolutionConfidence::High,
            reason: "No callers detected in project-wide analysis".to_string(),
            usage_locations: vec![],
        })
    }
}
```

### Data Structures

```rust
// Project-wide analysis context
pub struct ProjectAnalysisContext {
    // All files in project
    pub files: Vec<PathBuf>,
    // Per-file analysis results
    pub file_analyses: HashMap<PathBuf, FileAnalysis>,
    // Project-wide import graph
    pub import_graph: ImportGraph,
    // Project-wide symbol resolver
    pub symbol_resolver: SymbolResolver,
    // Cross-file call graph
    pub call_graph: CrossFileCallGraph,
}

// Import graph (module dependencies)
pub struct ImportGraph {
    // File -> Files it imports from
    pub dependencies: HashMap<PathBuf, HashSet<PathBuf>>,
    // File -> Files that import it
    pub dependents: HashMap<PathBuf, HashSet<PathBuf>>,
    // Detailed import statements
    pub imports: HashMap<PathBuf, Vec<Import>>,
}

impl ImportGraph {
    pub fn build(files: &[PathBuf]) -> Result<Self>;
    pub fn get_dependencies(&self, file: &Path) -> &HashSet<PathBuf>;
    pub fn get_transitive_dependencies(&self, file: &Path) -> HashSet<PathBuf>;
    pub fn detect_circular_imports(&self) -> Vec<Vec<PathBuf>>;
}
```

### APIs and Interfaces

```rust
// Main entry point for cross-file analysis
pub fn analyze_project_with_cross_file_context(
    files: Vec<PathBuf>,
    config: &Config,
) -> Result<ProjectAnalysis> {
    // Phase 1: Parse all files in parallel
    let file_analyses = files
        .par_iter()
        .map(|path| parse_and_analyze_file(path, config))
        .collect::<Result<Vec<_>>>()?;

    // Phase 2: Build import graph
    let import_graph = ImportGraph::build(&files)?;

    // Phase 3: Build symbol resolver with immutable pattern
    let symbol_resolver = SymbolResolver::new()
        .with_max_depth(10)  // Circuit breaker for circular imports
        .build_from_analyses(&file_analyses, &import_graph)
        .build();

    // Phase 4: Build cross-file call graph
    let call_graph = CrossFileCallGraph::build(&files, &file_analyses, symbol_resolver.clone())?;

    // Phase 5: Detect dead code with confidence threshold
    let dead_code_detector = DeadCodeDetector::new(call_graph)
        .with_confidence_threshold(ResolutionConfidence::Low);
    let dead_code_findings = dead_code_detector.detect_all(&file_analyses);

    Ok(ProjectAnalysis {
        file_analyses,
        import_graph,
        symbol_resolver,
        call_graph,
        dead_code_findings,
    })
}

// Streaming API for large projects (memory-efficient)
pub fn analyze_project_streaming(
    files: impl Iterator<Item = PathBuf>,
    config: &Config,
) -> Result<impl Iterator<Item = FileAnalysis>> {
    Ok(files
        .par_bridge()
        .map(|path| parse_and_analyze_file(&path, config))
        .filter_map(Result::ok))
}

// Fallback behavior for unresolvable imports
impl SymbolResolver {
    pub fn resolve_with_fallback(
        &self,
        file: &Path,
        name: &str,
    ) -> (Option<Definition>, ResolutionConfidence) {
        match self.resolve_symbol(file, name) {
            Some((def, confidence)) => (Some(def), confidence),
            None => {
                // Log unresolvable import for debugging
                log::warn!("Could not resolve symbol '{}' in file {:?}", name, file);
                (None, ResolutionConfidence::Unknown)
            }
        }
    }
}
```

### Integration Points

1. **File Discovery** (`src/io/file_walker.rs`)
   - Collect all Python files in project
   - Pass complete file list to cross-file analyzer

2. **Python Analyzer** (`src/analyzers/python_analyzer.rs`)
   - Add import parsing to existing AST analysis
   - Extract function definitions with full context

3. **Call Graph Builder** (`src/analysis/call_graph/`)
   - Extend with cross-file edges
   - Integrate symbol resolution

4. **Dead Code Detector** (`src/debt/dead_code.rs`)
   - Query cross-file call graph before marking as dead
   - Report usage locations in findings

5. **Output Formatters** (`src/io/output/`)
   - Include cross-file usage in recommendations
   - Show call sites for detected usage

## Dependencies

- **Prerequisites**: None (foundation feature)
- **Affected Components**:
  - `src/analyzers/python/` - Add import parser
  - `src/analysis/` - Add symbol resolver and cross-file call graph
  - `src/debt/dead_code.rs` - Update detection logic
  - `src/builders/unified_analysis.rs` - Integrate project-wide analysis
- **External Dependencies**:
  - `rustpython-parser` crate (already used) for Python AST
  - `petgraph` crate for call graph representation

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_import() {
        let code = "import foo";
        let imports = PythonImportParser::parse_imports(code, Path::new("test.py")).unwrap();
        assert_eq!(imports.len(), 1);
        assert!(matches!(imports[0].import_type, ImportType::Module { .. }));
    }

    #[test]
    fn test_parse_from_import() {
        let code = "from foo import bar, baz as qux";
        let imports = PythonImportParser::parse_imports(code, Path::new("test.py")).unwrap();
        assert_eq!(imports.len(), 1);
        match &imports[0].import_type {
            ImportType::FromModule { module, names, .. } => {
                assert_eq!(module, "foo");
                assert_eq!(names.len(), 2);
            }
            _ => panic!("Expected FromModule import"),
        }
    }

    #[test]
    fn test_relative_import_resolution() {
        let import = Import {
            import_type: ImportType::FromModule {
                module: "conversation".to_string(),
                names: vec![],
                level: 1, // from . import
            },
            line: 1,
            source_file: PathBuf::from("src/client/panel.py"),
        };

        let resolved = PythonImportParser::resolve_relative_import(
            &import,
            Path::new("src/client/panel.py"),
        ).unwrap();

        assert_eq!(resolved, PathBuf::from("src/client/conversation.py"));
    }

    #[test]
    fn test_cross_file_call_detection() {
        // Create mock file analyses with cross-file call
        let file1 = create_mock_analysis("manager.py", "def add_message(): pass");
        let file2 = create_mock_analysis("main.py", "from manager import add_message\nadd_message()");

        let call_graph = CrossFileCallGraph::build(
            &[PathBuf::from("manager.py"), PathBuf::from("main.py")],
            &[file1, file2],
        ).unwrap();

        let function_def = FunctionDef::new("add_message", PathBuf::from("manager.py"));
        assert!(call_graph.is_function_used(&function_def));

        let callers = call_graph.get_cross_file_callers(&function_def);
        assert_eq!(callers.len(), 1);
        assert_eq!(callers[0].caller_file, PathBuf::from("main.py"));
    }

    #[test]
    fn test_singleton_pattern_detection() {
        let code = r#"
class ConversationManager:
    def add_message(self, text):
        pass

manager = ConversationManager()
        "#;

        let analysis = analyze_python_code(code, "manager.py");
        let resolver = SymbolResolver::from_analysis(&analysis);

        assert!(resolver.is_singleton("manager.py", "manager"));
        assert_eq!(
            resolver.get_singleton_class("manager.py", "manager"),
            Some("ConversationManager".to_string())
        );
    }

    #[test]
    fn test_circular_import_circuit_breaker() {
        // Create circular import scenario: a.py imports b.py, b.py imports a.py
        let imports = vec![
            create_import("a.py", "b", "function_b"),
            create_import("b.py", "a", "function_a"),
        ];

        let resolver = SymbolResolver::new()
            .with_max_depth(10)
            .build_from_imports(&imports)
            .build();

        // Should not infinite loop, should return error at max depth
        let result = resolver.resolve_with_depth(Path::new("a.py"), "function_b", 0);
        assert!(result.is_err() || result.unwrap().1 == ResolutionConfidence::Low);
    }

    #[test]
    fn test_confidence_scoring() {
        // Direct import = High confidence
        let direct = create_import("main.py", "utils", "helper");
        assert_eq!(classify_import_confidence(&direct), ResolutionConfidence::High);

        // Wildcard import = Low confidence
        let wildcard = create_wildcard_import("main.py", "utils");
        assert_eq!(classify_import_confidence(&wildcard), ResolutionConfidence::Low);

        // Dynamic import = Unknown confidence
        let dynamic = create_dynamic_import("main.py", "module_name");
        assert_eq!(classify_import_confidence(&dynamic), ResolutionConfidence::Unknown);
    }
}

// Property-based tests
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn symbol_resolution_is_deterministic(
            imports in generate_random_imports(10),
            files in generate_file_structure(5)
        ) {
            let resolver1 = SymbolResolver::from_imports(&imports, &files).build();
            let resolver2 = SymbolResolver::from_imports(&imports, &files).build();

            for file in &files {
                for symbol in extract_symbols(&imports, file) {
                    prop_assert_eq!(
                        resolver1.resolve_symbol(file, &symbol),
                        resolver2.resolve_symbol(file, &symbol)
                    );
                }
            }
        }

        #[test]
        fn confidence_ordering_is_consistent(
            call_sites in prop::collection::vec(any::<CallSite>(), 1..100)
        ) {
            let max_confidence = call_sites.iter()
                .map(|site| site.confidence)
                .max()
                .unwrap();

            // Verify that High > Medium > Low > Unknown
            prop_assert!(max_confidence >= ResolutionConfidence::Unknown);
            prop_assert!(ResolutionConfidence::High > ResolutionConfidence::Medium);
            prop_assert!(ResolutionConfidence::Medium > ResolutionConfidence::Low);
            prop_assert!(ResolutionConfidence::Low > ResolutionConfidence::Unknown);
        }

        #[test]
        fn cross_file_analysis_never_marks_used_functions_as_dead(
            project in generate_random_python_project(20)
        ) {
            let analysis = analyze_project_with_cross_file_context(
                project.files.clone(),
                &Config::default()
            ).unwrap();

            // For every function that has callers, it should not be in dead code findings
            for finding in &analysis.dead_code_findings {
                let callers = analysis.call_graph.get_cross_file_callers(&finding.function);
                let high_confidence_callers = callers.iter()
                    .filter(|site| site.confidence >= ResolutionConfidence::Medium)
                    .count();

                prop_assert_eq!(high_confidence_callers, 0,
                    "Function {:?} marked as dead but has {} high-confidence callers",
                    finding.function.name, high_confidence_callers);
            }
        }

        #[test]
        fn import_graph_has_no_self_loops(
            files in generate_file_structure(10)
        ) {
            let import_graph = ImportGraph::build(&files).unwrap();

            for file in &files {
                let deps = import_graph.get_dependencies(file);
                prop_assert!(!deps.contains(file),
                    "File {:?} imports itself", file);
            }
        }
    }

    // Helper functions for property-based testing
    fn generate_random_imports(count: usize) -> impl Strategy<Value = Vec<Import>> {
        prop::collection::vec(
            (any::<String>(), any::<String>(), any::<String>())
                .prop_map(|(file, module, symbol)| {
                    Import {
                        import_type: ImportType::FromModule {
                            module,
                            names: vec![ImportName { name: symbol, alias: None }],
                            level: 0,
                        },
                        line: 1,
                        source_file: PathBuf::from(file),
                    }
                }),
            0..count
        )
    }

    fn generate_file_structure(count: usize) -> impl Strategy<Value = Vec<PathBuf>> {
        prop::collection::vec(
            any::<String>().prop_map(|name| PathBuf::from(format!("{}.py", name))),
            1..count
        )
    }
}
```

### Integration Tests

**Test Case 0: Regression Test for Bug #10 (ConversationManager)**
```python
# tests/fixtures/bug_10/conversation_manager.py
class ConversationManager:
    def add_message(self, text, sender):
        """Add a new message to the end of the current conversation."""
        message, index = self.current_conversation.add_message(text, sender)
        return index

manager = ConversationManager()

# tests/fixtures/bug_10/mainwindow.py
from conversation_manager import manager

def handle_message(message):
    manager.add_message(message, "user")
```

Expected:
- `ConversationManager.add_message()` NOT flagged as dead code
- `manager` singleton detected
- Cross-file call from `mainwindow.py` to `conversation_manager.py` tracked
- Confidence: High

**Test Case 1: Cross-File Function Call**
```python
# tests/fixtures/cross_file/manager.py
def process_data(data):
    return data.upper()

# tests/fixtures/cross_file/main.py
from manager import process_data

def main():
    result = process_data("hello")
    print(result)
```

Expected: `process_data` NOT flagged as dead code (High confidence).

**Test Case 2: Singleton Pattern**
```python
# tests/fixtures/singleton/service.py
class Service:
    def do_work(self):
        pass

service = Service()

# tests/fixtures/singleton/client.py
from service import service

service.do_work()
```

Expected: `Service.do_work()` NOT flagged as dead code (High confidence).

**Test Case 3: Aliased Import**
```python
# tests/fixtures/alias/utils.py
def helper():
    pass

# tests/fixtures/alias/app.py
import utils as u

u.helper()
```

Expected: `helper` NOT flagged as dead code (High confidence).

**Test Case 4: Relative Import**
```python
# tests/fixtures/relative/pkg/__init__.py
# tests/fixtures/relative/pkg/module.py
def func():
    pass

# tests/fixtures/relative/pkg/client.py
from .module import func
func()
```

Expected: `func` NOT flagged as dead code (Medium confidence - relative import).

**Test Case 5: Wildcard Import (Low Confidence)**
```python
# tests/fixtures/wildcard/utils.py
def helper_a():
    pass

def helper_b():
    pass

# tests/fixtures/wildcard/app.py
from utils import *

helper_a()
```

Expected:
- Both `helper_a` and `helper_b` marked as potentially used (Low confidence)
- Recommendation to refactor to explicit imports

**Test Case 6: Unresolvable Import (Fallback)**
```python
# tests/fixtures/dynamic/loader.py
def dynamic_load():
    module_name = get_config("module")
    imported = __import__(module_name)
    return imported.handler()
```

Expected:
- Dynamic import marked as Unknown confidence
- Fallback to conservative detection (don't mark as dead)
- Warning logged about unresolvable import

### Performance Tests

```rust
#[test]
fn test_cross_file_analysis_performance() {
    let temp_dir = create_large_python_project(1000); // 1000 files
    let files = discover_python_files(&temp_dir);

    let start = Instant::now();
    let analysis = analyze_project_with_cross_file_context(files, &Config::default()).unwrap();
    let duration = start.elapsed();

    assert!(duration < Duration::from_secs(2), "Analysis took {:?}", duration);
    assert!(analysis.file_analyses.len() == 1000);
}

#[test]
fn test_memory_usage_scalability() {
    let temp_dir = create_large_python_project(1000);
    let files = discover_python_files(&temp_dir);

    let before = get_memory_usage();
    let analysis = analyze_project_with_cross_file_context(files, &Config::default()).unwrap();
    let after = get_memory_usage();

    let memory_used_mb = (after - before) / 1024 / 1024;
    assert!(memory_used_mb < 500, "Memory usage: {}MB", memory_used_mb);
}
```

## Documentation Requirements

### Code Documentation

- Document import parser algorithm and limitations
- Explain symbol resolution strategy
- Document cross-file call graph construction
- Include examples for each import type

### User Documentation

Add to user guide:

```markdown
## Cross-File Analysis

Debtmap performs project-wide analysis to accurately detect dead code:

### How It Works

1. **Import Analysis**: Parses all import statements across your project
2. **Symbol Resolution**: Maps function calls to their definitions
3. **Cross-File Call Graph**: Tracks function usage across file boundaries
4. **Singleton Detection**: Identifies module-level instances and their usage

### Supported Import Patterns

```python
# Simple import
import module
module.function()

# From import
from module import function
function()

# Aliased import
import module as mod
from module import function as func

# Relative import
from . import sibling
from .. import parent

# Wildcard import (marked as low-confidence)
from module import *
```

### Limitations

- **Dynamic imports**: `__import__()`, `importlib.import_module()` not fully supported
- **Runtime modifications**: Monkeypatching and runtime attribute assignment not tracked
- **Complex inheritance**: Multiple inheritance with dynamic method resolution may be missed

### Performance

- 1000 files analyzed in < 2 seconds
- Memory usage scales linearly with project size
- Incremental analysis caches results (with spec 102)
```

### Architecture Documentation

Update ARCHITECTURE.md:

```markdown
## Cross-File Dependency Analysis

### Overview

Debtmap builds a complete project-wide call graph to detect dead code accurately:

```
┌─────────────────┐
│ File Discovery  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐     ┌──────────────────┐
│ Parse Imports   │────▶│  Import Graph    │
└────────┬────────┘     └──────────────────┘
         │                       │
         ▼                       │
┌─────────────────┐              │
│ Extract AST     │              │
└────────┬────────┘              │
         │                       │
         ▼                       ▼
┌─────────────────┐     ┌──────────────────┐
│ Symbol Resolver │────▶│ Cross-File Graph │
└─────────────────┘     └──────────────────┘
                                 │
                                 ▼
                        ┌──────────────────┐
                        │ Dead Code Check  │
                        └──────────────────┘
```

### Key Algorithms

**Import Resolution**:
1. Parse import statements from Python AST
2. Resolve relative imports using file paths
3. Map imported symbols to source definitions
4. Handle module-level `__init__.py` files

**Call Graph Construction**:
1. Build intra-file call graph for each file
2. Add cross-file edges using import resolution
3. Track instance method calls via object flow
4. Detect singleton pattern for global instances

**Dead Code Detection**:
1. Check if function has any callers in intra-file graph
2. Query cross-file graph for external callers
3. Consider function "used" if found in either graph
4. Report usage locations for transparency
```

## Implementation Notes

### Python Module Resolution

Python's import system is complex. Key considerations:

1. **Module search paths**:
   - Relative to current file
   - Project root (where analysis starts)
   - Assume standard `sys.path` behavior

2. **Package structure**:
   - `__init__.py` makes directories into packages
   - `from pkg import X` may resolve to `pkg/__init__.py` or `pkg/X.py`
   - Need heuristics to handle ambiguity

3. **Namespace packages** (PEP 420):
   - Directories without `__init__.py` can be packages
   - Less common, handle as best effort

### Circuit Breaker for Circular Imports

To prevent infinite loops during symbol resolution:

```rust
const MAX_RESOLUTION_DEPTH: usize = 10;

impl SymbolResolver {
    pub fn resolve_with_depth(
        &self,
        file: &Path,
        name: &str,
        depth: usize,
    ) -> Result<(Definition, ResolutionConfidence)> {
        if depth > MAX_RESOLUTION_DEPTH {
            return Err(anyhow::anyhow!(
                "Resolution depth exceeded ({} > {}) - circular import detected in {:?}",
                depth,
                MAX_RESOLUTION_DEPTH,
                file
            ));
        }

        // Track visited files to detect immediate cycles
        let mut visited = HashSet::new();
        self.resolve_internal(file, name, depth, &mut visited)
    }
}
```

**Behavior**:
- Detect circular imports early (Python allows some circular patterns)
- Return `ResolutionConfidence::Low` for complex circular dependencies
- Log warning for debugging circular import chains

### Fallback Strategy for Unresolvable Imports

When symbol resolution fails, use conservative fallback:

```rust
impl SymbolResolver {
    pub fn resolve_with_fallback(
        &self,
        file: &Path,
        name: &str,
    ) -> (Option<Definition>, ResolutionConfidence) {
        match self.resolve_with_depth(file, name, 0) {
            Ok((def, confidence)) => (Some(def), confidence),
            Err(e) => {
                // Log for debugging but don't fail analysis
                log::warn!(
                    "Could not resolve symbol '{}' in {:?}: {}",
                    name,
                    file,
                    e
                );

                // Conservative: assume it might be used (don't mark as dead)
                (None, ResolutionConfidence::Unknown)
            }
        }
    }
}
```

**Rationale**:
- False positives (marking used code as dead) are worse than false negatives
- Unknown confidence signals to user that manual verification needed
- Enables progressive enhancement as analysis improves

### Confidence Scoring Strategy

Assign confidence based on import type and resolution success:

| Import Type | Example | Confidence | Rationale |
|-------------|---------|------------|-----------|
| Direct module import | `from foo import bar` | High | Exact symbol known |
| Aliased import | `import foo as f` | High | Clear mapping preserved |
| Relative import (simple) | `from . import bar` | Medium | Heuristic path resolution |
| Relative import (complex) | `from ...pkg import bar` | Medium | Multiple parent traversal |
| Wildcard import | `from foo import *` | Low | Unknown which symbols used |
| Dynamic import | `__import__(name)` | Unknown | Runtime-only information |
| Unresolvable | Resolution error | Unknown | Cannot determine usage |

**Implementation**:
```rust
fn classify_import_confidence(import: &Import) -> ResolutionConfidence {
    match &import.import_type {
        ImportType::Module { .. } => ResolutionConfidence::High,
        ImportType::FromModule { level: 0, .. } => ResolutionConfidence::High,
        ImportType::FromModule { level: 1..=2, .. } => ResolutionConfidence::Medium,
        ImportType::FromModule { level, .. } if *level > 2 => ResolutionConfidence::Low,
        ImportType::Wildcard { .. } => ResolutionConfidence::Low,
    }
}
```

### Singleton Pattern Detection

Detect module-level instances:

```python
# Pattern 1: Direct assignment
class Manager:
    pass
manager = Manager()  # ✅ Singleton

# Pattern 2: Factory function
def create_manager():
    return Manager()
manager = create_manager()  # ✅ Singleton (if tracked)

# Pattern 3: Conditional assignment
if config.use_manager:
    manager = Manager()  # ⚠️ Low confidence
```

### Performance Optimization

1. **Parallel file parsing**: Use `rayon::par_iter()` for file-level analysis
2. **Incremental updates**: Cache import graph for unchanged files (spec 102)
3. **Lazy symbol resolution**: Resolve symbols on-demand, not upfront
4. **Efficient data structures**: Use `HashSet` for quick membership checks

### Object Flow Analysis Limitations

**Current Scope (Phase 1)**:
- Module-level singleton instances (e.g., `manager = Manager()`)
- Direct attribute access on imported singletons

**Out of Scope (Future Enhancement)**:
- Instance tracking across function boundaries
- Factory function return value tracking
- Complex object flow through multiple assignments

**Example of unsupported pattern**:
```python
# factory.py
def create():
    return Manager()  # ❌ Return value type not tracked

# main.py
from factory import create
mgr = create()  # ❌ Cannot determine mgr is Manager instance
mgr.process()   # ❌ Cannot connect to Manager.process()
```

**Mitigation**:
- Mark as low confidence when object flow unclear
- Document pattern limitations in user guide
- Plan separate spec (112.1) for advanced object flow analysis

### Edge Cases

1. **Circular imports**:
   - Python allows circular imports in some cases
   - Circuit breaker at depth 10 prevents infinite loops
   - Mark circular dependencies with Low confidence
   - Detect and report circular import chains for user awareness

2. **Wildcard imports** (`from X import *`):
   - Can't determine exact symbols used
   - Mark ALL exported symbols from module as Low confidence
   - Suggest refactoring to explicit imports in output
   - Consider all functions in wildcard module as "potentially used"

3. **Dynamic imports**:
   - `__import__()`, `importlib.import_module()`
   - Cannot fully analyze without runtime info
   - Mark as Unknown confidence with fallback
   - Log warning for user awareness

4. **Runtime monkeypatching**:
   - `obj.method = lambda: ...`
   - Cannot detect statically
   - Accept limitation, document in user guide
   - Out of scope for static analysis

## Migration and Compatibility

### Backward Compatibility

- **No breaking changes**: Existing CLI and JSON output unchanged
- **Additive feature**: Improves accuracy without removing functionality
- **Graceful degradation**: Falls back to intra-file analysis if cross-file fails

### Migration Path

For existing users:

1. **Automatic activation**: Cross-file analysis runs automatically (no flag needed)
2. **Gradual rollout**: Test on progressively larger codebases
3. **Opt-out option**: Add `--no-cross-file-analysis` flag for debugging

### Performance Impact

- Expected: 20-30% slower than file-only analysis
- Acceptable tradeoff for 90% reduction in false positives
- Mitigated by parallel processing and caching

## Future Enhancements

These enhancements are out of scope for this spec and should be separate specifications:

1. **Spec 112.1: Advanced Object Flow Analysis**
   - Track instance creation across function boundaries
   - Follow factory function return values
   - Analyze complex object flow through assignments
   - Handle class instantiation patterns

2. **Spec 112.2: Multi-Language Import Abstraction**
   - Abstract import analysis interface
   - JavaScript/TypeScript import/export analysis
   - Rust module system analysis
   - Common cross-language patterns

3. **Spec 112.3: Dynamic Import Heuristics**
   - Pattern matching for common dynamic imports (plugin systems)
   - Configuration-based import detection
   - Framework-specific import conventions (Django, Flask)

4. **Spec 112.4: Type-Based Symbol Resolution**
   - Use Python type hints for more accurate resolution
   - Track type flow through function calls
   - Integrate with `mypy` for type information

5. **Spec 112.5: Import Optimization Recommendations**
   - Flag unused imports
   - Suggest converting wildcard imports to explicit
   - Detect redundant imports
   - Recommend import ordering/grouping

6. **Spec 112.6: Call Graph Visualization**
   - Generate dependency diagrams
   - Interactive call graph exploration
   - Highlight critical paths and bottlenecks

## Success Metrics

### Accuracy Metrics
- **False positive rate**: < 5% (down from 50%)
- **High-confidence accuracy**: > 98% for High confidence findings
- **Medium-confidence accuracy**: > 90% for Medium confidence findings
- **Zero false negatives**: Never mark actively used code as dead with High confidence
- **Coverage**: Handle 95% of common import patterns

### Performance Metrics
- **Analysis time**: < 2 seconds for 1000-file project
- **Memory usage**: < 500MB for 1000-file project
- **Streaming efficiency**: Process 10,000-file projects without memory errors
- **Parallel speedup**: > 3x on 4-core systems

### Quality Metrics
- **Test coverage**: > 90% for cross-file analysis code
- **Property test coverage**: 5+ invariant tests passing
- **Integration tests**: 6+ real-world scenarios validated
- **Regression protection**: Bug #10 scenario permanently fixed

### Adoption Metrics
- **Bug reports**: 0 reports about #10-type false positives in 6 months
- **User satisfaction**: No complaints about "dead code detector broke my app"
- **Confidence usage**: > 80% of findings have High or Medium confidence
- **Documentation quality**: Users understand confidence levels without support

## Related Specifications

### Dependencies
This specification has no dependencies (foundation feature).

### Dependents
This specification will be depended upon by:
- **Spec 110**: Public API Detection (needs cross-file usage data)
- **Spec 111**: Design Pattern Recognition (uses call graph)
- **Spec 113**: Confidence Scoring (uses cross-file analysis for confidence)
- **Spec 102**: Incremental Analysis (will cache import graph and symbol resolution)

### Future Extensions
Follow-up specifications (defined in Future Enhancements):
- **Spec 112.1**: Advanced Object Flow Analysis
- **Spec 112.2**: Multi-Language Import Abstraction
- **Spec 112.3**: Dynamic Import Heuristics
- **Spec 112.4**: Type-Based Symbol Resolution
- **Spec 112.5**: Import Optimization Recommendations
- **Spec 112.6**: Call Graph Visualization
