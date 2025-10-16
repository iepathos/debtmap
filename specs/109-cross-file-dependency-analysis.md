---
number: 109
title: Cross-File Dependency Analysis
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-10-16
---

# Specification 109: Cross-File Dependency Analysis

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

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
   - Memory usage scales linearly with project size
   - Incremental analysis for changed files (with spec 102)
   - Parallel file parsing using `rayon`

2. **Accuracy**
   - False positive rate < 5% for dead code detection
   - Zero false negatives (never mark used code as dead)
   - Handle 95% of common Python import patterns
   - Graceful degradation for complex dynamic imports

3. **Maintainability**
   - Clear separation of import parsing and call graph construction
   - Extensible to JavaScript/TypeScript (future)
   - Comprehensive test coverage (> 90%)
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
- [ ] False positive rate < 5% on promptconstruct-frontend codebase
- [ ] Performance < 2 seconds for 1000-file Python project
- [ ] Memory usage < 500MB for 1000-file project
- [ ] Integration tests validate cross-file analysis
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
}

#[derive(Debug, Clone)]
pub struct Definition {
    pub file: PathBuf,
    pub symbol: String,
    pub kind: DefinitionKind,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DefinitionKind {
    Function,
    Class,
    Method { class_name: String },
    Variable,
    ModuleInstance { class_name: String },
}

impl SymbolResolver {
    pub fn new() -> Self;
    pub fn add_definition(&mut self, file: PathBuf, symbol: String, kind: DefinitionKind);
    pub fn add_import(&mut self, import: Import, source_file: PathBuf);
    pub fn resolve_symbol(&self, file: &Path, name: &str) -> Option<Definition>;
    pub fn get_all_callers(&self, target: &Definition) -> Vec<CallSite>;
}

// src/analysis/call_graph/cross_file.rs
pub struct CrossFileCallGraph {
    // Extends CallGraph with cross-file edges
    call_graph: CallGraph,
    symbol_resolver: SymbolResolver,
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
}

impl CrossFileCallGraph {
    pub fn build(files: &[PathBuf], analyses: &[FileAnalysis]) -> Result<Self>;
    pub fn get_cross_file_callers(&self, function: &FunctionDef) -> Vec<CallSite>;
    pub fn is_function_used(&self, function: &FunctionDef) -> bool;
    pub fn get_usage_locations(&self, function: &FunctionDef) -> Vec<Location>;
}

// Update src/debt/dead_code.rs
pub struct DeadCodeDetector {
    cross_file_graph: CrossFileCallGraph,
    confidence_threshold: f32,
}

impl DeadCodeDetector {
    pub fn detect_with_cross_file_analysis(
        &self,
        function: &FunctionDef,
    ) -> Option<DeadCodeFinding> {
        // Check cross-file usage before marking as dead
        if self.cross_file_graph.is_function_used(function) {
            return None;
        }

        let usage_locations = self.cross_file_graph.get_usage_locations(function);
        if !usage_locations.is_empty() {
            return None; // Used across files
        }

        Some(DeadCodeFinding {
            function: function.clone(),
            confidence: self.calculate_confidence(function),
            reason: "No callers detected in project-wide analysis".to_string(),
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
    // Phase 1: Parse all files
    let file_analyses = files
        .par_iter()
        .map(|path| parse_and_analyze_file(path, config))
        .collect::<Result<Vec<_>>>()?;

    // Phase 2: Build import graph
    let import_graph = ImportGraph::build(&files)?;

    // Phase 3: Build symbol resolver
    let symbol_resolver = SymbolResolver::from_analyses(&file_analyses, &import_graph);

    // Phase 4: Build cross-file call graph
    let call_graph = CrossFileCallGraph::build(&files, &file_analyses, &symbol_resolver)?;

    // Phase 5: Detect dead code with cross-file context
    let dead_code_detector = DeadCodeDetector::new(call_graph);
    let dead_code_findings = dead_code_detector.detect_all(&file_analyses);

    Ok(ProjectAnalysis {
        file_analyses,
        import_graph,
        call_graph,
        dead_code_findings,
    })
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
}
```

### Integration Tests

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

Expected: `process_data` NOT flagged as dead code.

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

Expected: `Service.do_work()` NOT flagged as dead code.

**Test Case 3: Aliased Import**
```python
# tests/fixtures/alias/utils.py
def helper():
    pass

# tests/fixtures/alias/app.py
import utils as u

u.helper()
```

Expected: `helper` NOT flagged as dead code.

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

Expected: `func` NOT flagged as dead code.

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

### Edge Cases

1. **Circular imports**:
   - Python allows circular imports in some cases
   - Detect cycles but don't infinite loop
   - Mark circular dependencies in output

2. **Wildcard imports** (`from X import *`):
   - Can't determine exact symbols used
   - Mark as low-confidence findings
   - Suggest refactoring to explicit imports

3. **Dynamic imports**:
   - `__import__()`, `importlib.import_module()`
   - Cannot fully analyze without runtime info
   - Flag as "may be dynamically imported" with low confidence

4. **Runtime monkeypatching**:
   - `obj.method = lambda: ...`
   - Cannot detect statically
   - Accept limitation, document in user guide

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

1. **JavaScript/TypeScript support**: Extend to other languages
2. **Dynamic import inference**: Heuristics for common dynamic patterns
3. **Type-based analysis**: Use type hints for more accurate resolution
4. **Call graph visualization**: Generate diagrams of project dependencies
5. **Unused import detection**: Flag imports that are never used
6. **Import optimization**: Suggest converting wildcard imports to explicit

## Success Metrics

- **False positive rate**: < 5% (down from 50%)
- **Performance**: < 2 seconds for 1000-file project
- **Coverage**: Handle 95% of common import patterns
- **Adoption**: 0 bug reports about #10-type false positives in 6 months
- **User satisfaction**: No complaints about "dead code detector broke my app"

## Related Specifications

This specification is foundational and will be depended upon by:
- Spec 110: Public API Detection (needs cross-file usage data)
- Spec 111: Design Pattern Recognition (uses call graph)
- Spec 113: Confidence Scoring (uses cross-file analysis for confidence)
