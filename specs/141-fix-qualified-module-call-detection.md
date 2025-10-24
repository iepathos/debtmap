---
number: 141
title: Fix Qualified Module Call Detection in Call Graph
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-01-23
---

# Specification 141: Fix Qualified Module Call Detection in Call Graph

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

**Discovered Bug**: The Rust call graph builder fails to detect function calls that use qualified module syntax (`module::function()`), leading to critical false positives where actively-used functions are flagged as having "0 callers" and recommended for removal.

**Real-World Impact from Reproduction Test**:

```rust
// In src/builders/unified_analysis.rs:
use super::{call_graph, ...};

fn build_and_cache_graph(...) {
    // Line 555: This call is NOT detected by the call graph
    call_graph::process_rust_files_for_call_graph(...)  // ← MISSED
}
```

**Evidence from Diagnostic Test**:
```
Function: build_and_cache_graph at line 544
  Calls 4 functions:
    -> CallGraphCache::put           ✓ Detected
    -> Database::cache                ✓ Detected
    -> ContextMap::iter               ✓ Detected
    -> DataFlowGraph::call_graph      ✓ Detected
  ✗ Does NOT call process_rust_files_for_call_graph
  ⚠️  BUT SOURCE CODE SHOWS IT DOES (line 555)
  ⚠️  THIS IS THE BUG - the call isn't being detected!
```

**Result**: `process_rust_files_for_call_graph()` shows **0 callers** in debtmap's own analysis despite having **3 actual callers** via qualified module syntax.

**Why This is Critical**:
- Produces false "dead code" warnings for core infrastructure
- Undermines user trust when #4 recommendation is provably wrong
- Causes developers to waste time investigating "unused" code that's actively called
- Affects any codebase using qualified module calls (common Rust pattern)

## Objective

Fix the Rust call graph builder to correctly detect and link function calls that use qualified module syntax (`module::function()`, `crate::module::function()`, `super::module::function()`), ensuring zero false "no callers" warnings for functions called via qualified paths.

**Success Criteria**:
- `process_rust_files_for_call_graph()` correctly shows ≥3 callers in debtmap's own analysis
- All qualified call syntaxes are detected (direct, module-qualified, fully-qualified)
- Test coverage for qualified calls, use statement resolution, and cross-file namespacing

## Requirements

### Functional Requirements

**FR1: Qualified Path Resolution**
- Detect calls using `module::function()` syntax
- Detect calls using `crate::path::to::function()` syntax
- Detect calls using `super::module::function()` syntax
- Detect calls using `self::function()` syntax
- Handle nested module paths (`a::b::c::function()`)

**FR2: Use Statement Tracking**
- Track `use` statements that import modules
- Map module aliases to their actual paths
- Resolve calls through imported module names
- Handle glob imports where feasible (`use module::*`)
- Track re-exports (`pub use`)

**FR3: Call Site Analysis**
- Parse `ExprCall` AST nodes to extract full path
- Distinguish between:
  - Direct calls: `function()`
  - Method calls: `obj.method()`
  - Qualified calls: `module::function()`
  - Fully qualified: `crate::module::function()`
- Extract the function name and module path separately

**FR4: Name Resolution**
- Match qualified calls to function definitions across files
- Resolve `super::` relative to current module hierarchy
- Resolve `crate::` to project root
- Handle ambiguous names by checking import context
- Fall back gracefully when resolution is uncertain

**FR5: Existing Behavior Preservation**
- Don't break detection of unqualified calls
- Don't break method call detection
- Don't break trait method resolution
- Maintain performance characteristics (<10% slowdown)

### Non-Functional Requirements

**NFR1: Accuracy**
- Zero false "no callers" for qualified calls
- <5% false negatives on qualified call detection
- Correctly handle 95%+ of real-world qualified call patterns

**NFR2: Performance**
- Qualified call resolution adds <10% to call graph construction time
- Cache module path resolutions to avoid redundant work
- Use efficient data structures (HashMap for module lookups)

**NFR3: Maintainability**
- Separate qualified call logic from existing call detection
- Pure functions for path resolution
- Clear error messages when resolution fails
- Extensive test coverage (unit + integration)

## Acceptance Criteria

### AC1: Qualified Module Call Detection
- [ ] Detect calls using `module::function()` syntax
- [ ] Create test case: `call_graph::process_rust_files_for_call_graph(...)`
- [ ] Verify call is correctly linked in call graph
- [ ] Test with nested modules: `a::b::c::function()`
- [ ] Test with multiple levels of qualification

### AC2: Crate-Relative Path Detection
- [ ] Detect calls using `crate::module::function()` syntax
- [ ] Resolve `crate::` to project root
- [ ] Test cross-module calls with full paths
- [ ] Handle deep module hierarchies correctly

### AC3: Super-Relative Path Detection
- [ ] Detect calls using `super::module::function()` syntax
- [ ] Resolve `super::` relative to current module
- [ ] Test nested `super::super::` paths
- [ ] Handle edge cases (calling from root module)

### AC4: Use Statement Resolution
- [ ] Track `use module::name;` statements
- [ ] Map imported names to their source modules
- [ ] Resolve calls through imported names
- [ ] Test with aliased imports: `use module::function as func;`
- [ ] Handle multiple imports from same module

### AC5: Fix process_rust_files_for_call_graph False Positive
- [ ] Run debtmap on its own codebase
- [ ] Verify `process_rust_files_for_call_graph` shows ≥3 callers
- [ ] Callers should include:
  - `build_and_cache_graph` (via `call_graph::process_rust_files_for_call_graph`)
  - Functions from validate.rs
  - Functions from unified_analysis.rs (indirectly)
- [ ] Zero "⚠ No callers detected" warnings for known infrastructure functions

### AC6: Comprehensive Test Coverage
- [ ] Unit test: qualified call parsing
- [ ] Unit test: use statement tracking
- [ ] Unit test: module path resolution
- [ ] Integration test: cross-file qualified calls (from reproduction test)
- [ ] Integration test: debtmap self-analysis (no false positives)
- [ ] Regression test: unqualified calls still work

### AC7: Edge Case Handling
- [ ] Handle ambiguous names gracefully (multiple imports)
- [ ] Handle unresolved modules (external crates)
- [ ] Handle glob imports (`use module::*`)
- [ ] Handle re-exports (`pub use`)
- [ ] Log warnings for unresolved qualified calls (don't fail silently)

## Technical Details

### Implementation Approach

**Phase 1: Enhanced AST Call Extraction**

Update the Rust call graph builder to extract qualified paths from call expressions:

```rust
// In src/builders/rust_call_graph.rs or similar

use syn::{Expr, ExprCall, ExprPath};

/// Extract qualified path from a call expression
fn extract_call_path(call_expr: &ExprCall) -> Option<QualifiedPath> {
    if let Expr::Path(ExprPath { path, .. }) = &*call_expr.func {
        Some(QualifiedPath::from_syn_path(path))
    } else {
        None
    }
}

/// Represents a qualified function path
#[derive(Debug, Clone)]
struct QualifiedPath {
    segments: Vec<String>,  // e.g., ["call_graph", "process_rust_files_for_call_graph"]
    is_absolute: bool,      // true if starts with `crate::` or `::`
    is_super: bool,         // true if starts with `super::`
    is_self: bool,          // true if starts with `self::`
}

impl QualifiedPath {
    fn from_syn_path(path: &syn::Path) -> Self {
        let segments: Vec<String> = path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect();

        let first = segments.first().map(|s| s.as_str());

        Self {
            is_absolute: first == Some("crate"),
            is_super: first == Some("super"),
            is_self: first == Some("self"),
            segments,
        }
    }

    fn function_name(&self) -> Option<&str> {
        self.segments.last().map(|s| s.as_str())
    }

    fn module_path(&self) -> Vec<&str> {
        self.segments
            .iter()
            .take(self.segments.len().saturating_sub(1))
            .map(|s| s.as_str())
            .collect()
    }
}
```

**Phase 2: Use Statement Tracking**

Build a map of imported modules and their aliases:

```rust
use syn::{ItemUse, UseTree};
use std::collections::HashMap;

/// Tracks module imports in a file
#[derive(Debug, Clone)]
struct ImportTracker {
    // Maps local name to (module_path, item_name)
    imports: HashMap<String, ImportedItem>,
    current_module: Vec<String>,
}

#[derive(Debug, Clone)]
struct ImportedItem {
    module_path: Vec<String>,  // e.g., ["super", "call_graph"]
    item_name: String,          // e.g., "process_rust_files_for_call_graph"
    is_glob: bool,              // true for `use module::*`
}

impl ImportTracker {
    fn new(file_path: &Path) -> Self {
        Self {
            imports: HashMap::new(),
            current_module: derive_module_path_from_file(file_path),
        }
    }

    fn track_use_statement(&mut self, use_item: &ItemUse) {
        self.track_use_tree(&use_item.tree, vec![]);
    }

    fn track_use_tree(&mut self, tree: &UseTree, prefix: Vec<String>) {
        match tree {
            UseTree::Path(path) => {
                let mut new_prefix = prefix.clone();
                new_prefix.push(path.ident.to_string());
                self.track_use_tree(&path.tree, new_prefix);
            }
            UseTree::Name(name) => {
                let item_name = name.ident.to_string();
                self.imports.insert(
                    item_name.clone(),
                    ImportedItem {
                        module_path: prefix.clone(),
                        item_name,
                        is_glob: false,
                    },
                );
            }
            UseTree::Rename(rename) => {
                let alias = rename.rename.to_string();
                self.imports.insert(
                    alias,
                    ImportedItem {
                        module_path: prefix.clone(),
                        item_name: rename.ident.to_string(),
                        is_glob: false,
                    },
                );
            }
            UseTree::Glob(_) => {
                // Track glob import for best-effort resolution
                self.imports.insert(
                    "*".to_string(),
                    ImportedItem {
                        module_path: prefix,
                        item_name: "*".to_string(),
                        is_glob: true,
                    },
                );
            }
            UseTree::Group(group) => {
                for item in &group.items {
                    self.track_use_tree(item, prefix.clone());
                }
            }
        }
    }

    fn resolve_call(&self, qualified_path: &QualifiedPath) -> Option<FullyQualifiedName> {
        if qualified_path.segments.len() == 1 {
            // Unqualified call - check imports
            let name = &qualified_path.segments[0];
            if let Some(import) = self.imports.get(name) {
                return Some(FullyQualifiedName::from_import(import));
            }
            // Not imported - assume local function
            return Some(FullyQualifiedName {
                module_path: self.current_module.clone(),
                function_name: name.clone(),
            });
        }

        // Qualified call - resolve the path
        let first_segment = &qualified_path.segments[0];

        if qualified_path.is_absolute {
            // crate::module::function
            return Some(self.resolve_absolute_path(qualified_path));
        }

        if qualified_path.is_super {
            // super::module::function
            return Some(self.resolve_super_path(qualified_path));
        }

        // Check if first segment is an imported module
        if let Some(import) = self.imports.get(first_segment) {
            return Some(self.resolve_through_import(qualified_path, import));
        }

        // Assume it's a submodule of current module
        Some(self.resolve_relative_path(qualified_path))
    }

    fn resolve_absolute_path(&self, path: &QualifiedPath) -> FullyQualifiedName {
        FullyQualifiedName {
            module_path: path.module_path().iter().skip(1).map(|s| s.to_string()).collect(),
            function_name: path.function_name().unwrap().to_string(),
        }
    }

    fn resolve_super_path(&self, path: &QualifiedPath) -> FullyQualifiedName {
        let mut module_path = self.current_module.clone();

        // Count 'super' segments and pop from current module
        let super_count = path.segments.iter().take_while(|s| *s == "super").count();
        for _ in 0..super_count {
            module_path.pop();
        }

        // Append remaining path (excluding 'super's and function name)
        module_path.extend(
            path.segments
                .iter()
                .skip(super_count)
                .take(path.segments.len() - super_count - 1)
                .cloned()
        );

        FullyQualifiedName {
            module_path,
            function_name: path.function_name().unwrap().to_string(),
        }
    }

    fn resolve_through_import(
        &self,
        path: &QualifiedPath,
        import: &ImportedItem,
    ) -> FullyQualifiedName {
        let mut module_path = import.module_path.clone();

        // Append middle segments (between module and function)
        module_path.extend(
            path.segments
                .iter()
                .skip(1)
                .take(path.segments.len() - 2)
                .cloned()
        );

        FullyQualifiedName {
            module_path,
            function_name: path.function_name().unwrap().to_string(),
        }
    }

    fn resolve_relative_path(&self, path: &QualifiedPath) -> FullyQualifiedName {
        let mut module_path = self.current_module.clone();
        module_path.extend(path.module_path().iter().map(|s| s.to_string()));

        FullyQualifiedName {
            module_path,
            function_name: path.function_name().unwrap().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FullyQualifiedName {
    module_path: Vec<String>,
    function_name: String,
}

fn derive_module_path_from_file(file: &Path) -> Vec<String> {
    // Convert file path to module path
    // e.g., "src/builders/call_graph.rs" -> ["builders", "call_graph"]
    file.components()
        .filter_map(|c| {
            if let std::path::Component::Normal(os_str) = c {
                os_str.to_str().map(|s| {
                    s.trim_end_matches(".rs")
                     .trim_end_matches("/mod")
                     .to_string()
                })
            } else {
                None
            }
        })
        .filter(|s| s != "src" && s != "lib")
        .collect()
}
```

**Phase 3: Integrate with Call Graph Builder**

Modify the call graph construction to use the new resolution logic:

```rust
// In the call graph builder

impl RustCallGraphBuilder {
    fn analyze_file_with_qualified_calls(
        &mut self,
        file_path: &Path,
        parsed: &syn::File,
    ) -> Result<()> {
        // Step 1: Build import tracker for this file
        let mut import_tracker = ImportTracker::new(file_path);

        for item in &parsed.items {
            if let syn::Item::Use(use_item) = item {
                import_tracker.track_use_statement(use_item);
            }
        }

        // Step 2: Extract function definitions
        let function_definitions = self.extract_function_definitions(file_path, parsed);

        // Step 3: Extract calls with qualified path resolution
        for func_def in &function_definitions {
            let calls = self.extract_calls_from_function(&func_def.item);

            for call in calls {
                // Parse the qualified path
                let qualified_path = extract_call_path(&call)?;

                // Resolve to fully qualified name
                if let Some(resolved) = import_tracker.resolve_call(&qualified_path) {
                    // Find the callee in our function registry
                    if let Some(callee_id) = self.find_function_by_qualified_name(&resolved) {
                        // Add the call edge
                        self.call_graph.add_call(
                            func_def.id.clone(),
                            callee_id,
                            CallType::Direct,
                        );
                    } else {
                        // Function not found - might be in external crate or not yet analyzed
                        log::debug!(
                            "Could not resolve call to {}::{} from {}",
                            resolved.module_path.join("::"),
                            resolved.function_name,
                            func_def.name
                        );
                    }
                }
            }
        }

        Ok(())
    }

    fn find_function_by_qualified_name(
        &self,
        qualified_name: &FullyQualifiedName,
    ) -> Option<FunctionId> {
        // Search all registered functions for one matching this qualified name
        self.function_registry
            .iter()
            .find(|(id, metadata)| {
                metadata.name == qualified_name.function_name
                    && self.module_path_matches(&id.file, &qualified_name.module_path)
            })
            .map(|(id, _)| id.clone())
    }

    fn module_path_matches(&self, file: &PathBuf, expected_path: &[String]) -> bool {
        let file_module = derive_module_path_from_file(file);
        file_module == expected_path
    }
}
```

### Architecture Changes

**Modified Files**:
- `src/builders/rust_call_graph.rs` - Add qualified call detection
- `src/builders/call_graph.rs` - Integration point
- `src/priority/call_graph/types.rs` - May need to store module paths

**New Files**:
- `src/builders/rust_call_graph/qualified_calls.rs` - Qualified call resolution logic
- `src/builders/rust_call_graph/import_tracker.rs` - Use statement tracking

### Data Structures

```rust
/// Represents a qualified path in a call expression
pub struct QualifiedPath {
    pub segments: Vec<String>,
    pub is_absolute: bool,  // crate::
    pub is_super: bool,     // super::
    pub is_self: bool,      // self::
}

/// Tracks imports in a file
pub struct ImportTracker {
    imports: HashMap<String, ImportedItem>,
    current_module: Vec<String>,
}

/// An imported item
pub struct ImportedItem {
    module_path: Vec<String>,
    item_name: String,
    is_glob: bool,
}

/// Fully qualified function name after resolution
pub struct FullyQualifiedName {
    module_path: Vec<String>,
    function_name: String,
}
```

### APIs and Interfaces

**New Public Functions**:

```rust
/// Extract qualified path from a call expression
pub fn extract_call_path(call_expr: &ExprCall) -> Option<QualifiedPath>;

/// Track use statements in a file
pub fn track_imports(file: &syn::File) -> ImportTracker;

/// Resolve a qualified call to a fully qualified name
pub fn resolve_qualified_call(
    path: &QualifiedPath,
    tracker: &ImportTracker,
) -> Option<FullyQualifiedName>;

/// Derive module path from file path
pub fn derive_module_path_from_file(file: &Path) -> Vec<String>;
```

## Dependencies

**Prerequisites**: None

**Affected Components**:
- `src/builders/rust_call_graph.rs` - Core call graph construction
- `src/builders/call_graph.rs` - Integration layer

**External Dependencies**:
- `syn` (already used) - For AST parsing and path extraction

## Testing Strategy

### Unit Tests

**Qualified Path Parsing** (`tests/rust_call_graph/qualified_path_tests.rs`):

```rust
#[test]
fn test_extract_qualified_path_simple() {
    let code = "module::function()";
    let expr = syn::parse_str::<ExprCall>(code).unwrap();
    let path = extract_call_path(&expr).unwrap();

    assert_eq!(path.segments, vec!["module", "function"]);
    assert!(!path.is_absolute);
    assert!(!path.is_super);
}

#[test]
fn test_extract_qualified_path_nested() {
    let code = "a::b::c::function()";
    let expr = syn::parse_str::<ExprCall>(code).unwrap();
    let path = extract_call_path(&expr).unwrap();

    assert_eq!(path.segments, vec!["a", "b", "c", "function"]);
    assert_eq!(path.module_path(), vec!["a", "b", "c"]);
    assert_eq!(path.function_name(), Some("function"));
}

#[test]
fn test_extract_crate_qualified_path() {
    let code = "crate::module::function()";
    let expr = syn::parse_str::<ExprCall>(code).unwrap();
    let path = extract_call_path(&expr).unwrap();

    assert!(path.is_absolute);
    assert_eq!(path.segments, vec!["crate", "module", "function"]);
}

#[test]
fn test_extract_super_qualified_path() {
    let code = "super::module::function()";
    let expr = syn::parse_str::<ExprCall>(code).unwrap();
    let path = extract_call_path(&expr).unwrap();

    assert!(path.is_super);
    assert_eq!(path.segments, vec!["super", "module", "function"]);
}
```

**Import Tracking** (`tests/rust_call_graph/import_tracker_tests.rs`):

```rust
#[test]
fn test_track_simple_import() {
    let code = r#"
        use std::collections::HashMap;

        fn foo() {}
    "#;
    let file = syn::parse_file(code).unwrap();
    let tracker = track_imports(&file);

    assert!(tracker.imports.contains_key("HashMap"));
    let import = &tracker.imports["HashMap"];
    assert_eq!(import.module_path, vec!["std", "collections"]);
    assert_eq!(import.item_name, "HashMap");
}

#[test]
fn test_track_module_import() {
    let code = r#"
        use super::call_graph;

        fn foo() {}
    "#;
    let file = syn::parse_file(code).unwrap();
    let tracker = track_imports(&file);

    assert!(tracker.imports.contains_key("call_graph"));
}

#[test]
fn test_track_renamed_import() {
    let code = r#"
        use std::collections::HashMap as Map;

        fn foo() {}
    "#;
    let file = syn::parse_file(code).unwrap();
    let tracker = track_imports(&file);

    assert!(tracker.imports.contains_key("Map"));
    assert_eq!(tracker.imports["Map"].item_name, "HashMap");
}
```

**Path Resolution** (`tests/rust_call_graph/resolution_tests.rs`):

```rust
#[test]
fn test_resolve_qualified_call() {
    let code = r#"
        use super::call_graph;
    "#;
    let file = syn::parse_file(code).unwrap();
    let mut tracker = track_imports(&file);
    tracker.current_module = vec!["builders".to_string(), "unified_analysis".to_string()];

    let path = QualifiedPath {
        segments: vec!["call_graph".to_string(), "process_rust_files_for_call_graph".to_string()],
        is_absolute: false,
        is_super: false,
        is_self: false,
    };

    let resolved = tracker.resolve_call(&path).unwrap();
    assert_eq!(resolved.module_path, vec!["builders", "call_graph"]);
    assert_eq!(resolved.function_name, "process_rust_files_for_call_graph");
}

#[test]
fn test_resolve_super_path() {
    let mut tracker = ImportTracker::new(Path::new("src/builders/unified_analysis.rs"));
    tracker.current_module = vec!["builders".to_string(), "unified_analysis".to_string()];

    let path = QualifiedPath {
        segments: vec!["super".to_string(), "call_graph".to_string(), "function".to_string()],
        is_absolute: false,
        is_super: true,
        is_self: false,
    };

    let resolved = tracker.resolve_call(&path).unwrap();
    assert_eq!(resolved.module_path, vec!["builders", "call_graph"]);
}
```

### Integration Tests

**Cross-File Qualified Calls** (using existing reproduction test from `call_graph_cross_file_resolution_test.rs`):

```rust
#[test]
fn test_qualified_module_calls_detected() {
    // This is the actual bug - qualified calls not detected
    let test_project = create_test_project();
    let mut call_graph = CallGraph::new();

    process_rust_files_for_call_graph(
        test_project.path(),
        &mut call_graph,
        false,
        false,
    ).unwrap();

    let caller_names = call_graph.get_callers_by_name("build_project_call_graph");

    // Should detect calls via module::function() syntax
    assert!(
        caller_names.len() >= 3,
        "Expected 3 callers via qualified syntax, found {}",
        caller_names.len()
    );
}
```

**Debtmap Self-Analysis** (existing test in `call_graph_diagnostic_test.rs`):

```rust
#[test]
fn test_self_analysis_no_false_positives() {
    let project_path = PathBuf::from(".");
    let mut call_graph = CallGraph::new();

    process_rust_files_for_call_graph(&project_path, &mut call_graph, false, false).unwrap();

    // Check process_rust_files_for_call_graph
    let caller_names = call_graph.get_callers_by_name("process_rust_files_for_call_graph");

    assert!(
        caller_names.len() >= 3,
        "Fixed: process_rust_files_for_call_graph should show ≥3 callers, found {}",
        caller_names.len()
    );

    // Verify build_and_cache_graph is one of the callers
    let build_and_cache_callers = call_graph.get_callers_by_name("build_and_cache_graph");
    assert!(
        !build_and_cache_callers.is_empty(),
        "build_and_cache_graph should be in call graph"
    );
}
```

### Performance Tests

```rust
#[test]
fn bench_qualified_call_overhead() {
    let large_project = create_project_with_n_files(100, 50); // 100 files, 50 functions each

    let start = Instant::now();
    let mut call_graph_without_qualified = CallGraph::new();
    build_call_graph_old_way(&large_project, &mut call_graph_without_qualified);
    let baseline = start.elapsed();

    let start = Instant::now();
    let mut call_graph_with_qualified = CallGraph::new();
    build_call_graph_with_qualified_detection(&large_project, &mut call_graph_with_qualified);
    let with_qualified = start.elapsed();

    let overhead_pct = ((with_qualified.as_millis() as f64 - baseline.as_millis() as f64)
                        / baseline.as_millis() as f64) * 100.0;

    assert!(
        overhead_pct < 10.0,
        "Qualified call detection adds {}% overhead (max 10%)",
        overhead_pct
    );
}
```

## Documentation Requirements

### Code Documentation

1. **Module-level docs** for qualified call logic:
   ```rust
   //! Qualified call resolution for the Rust call graph builder.
   //!
   //! This module handles detection and resolution of function calls that use
   //! qualified paths like `module::function()`, `crate::path::function()`, and
   //! `super::module::function()`.
   //!
   //! # Problem
   //!
   //! The original call graph builder only detected unqualified calls like
   //! `function()`. Qualified calls like `call_graph::process_rust_files_for_call_graph()`
   //! were completely missed, leading to false "no callers" warnings.
   //!
   //! # Solution
   //!
   //! 1. Extract the full qualified path from call expressions
   //! 2. Track `use` statements to build an import map
   //! 3. Resolve qualified paths to fully qualified names
   //! 4. Match resolved names to function definitions
   ```

2. **Function-level examples**:
   ```rust
   /// Resolve a qualified call to a fully qualified name.
   ///
   /// # Example
   ///
   /// ```
   /// // Given this import:
   /// // use super::call_graph;
   /// //
   /// // And this call:
   /// // call_graph::process_rust_files_for_call_graph(...)
   ///
   /// let path = QualifiedPath {
   ///     segments: vec!["call_graph", "process_rust_files_for_call_graph"],
   ///     is_super: false,
   ///     ...
   /// };
   ///
   /// let resolved = tracker.resolve_call(&path).unwrap();
   /// assert_eq!(resolved.module_path, vec!["builders", "call_graph"]);
   /// assert_eq!(resolved.function_name, "process_rust_files_for_call_graph");
   /// ```
   pub fn resolve_call(&self, path: &QualifiedPath) -> Option<FullyQualifiedName>;
   ```

### User Documentation

No user-facing changes - this is a bug fix that improves accuracy transparently.

### Architecture Documentation

Update ARCHITECTURE.md:

```markdown
## Call Graph Construction - Qualified Call Resolution

The Rust call graph builder detects function calls using three strategies:

1. **Unqualified calls**: `function()` - resolved via local scope or imports
2. **Qualified calls**: `module::function()` - resolved via import tracking
3. **Fully qualified calls**: `crate::path::function()` - resolved to absolute path

### Qualified Call Resolution Process

1. **Import Tracking**: Parse all `use` statements in the file
2. **Call Extraction**: Extract qualified paths from `ExprCall` nodes
3. **Path Resolution**: Resolve qualified paths using import map and module hierarchy
4. **Function Matching**: Match resolved names to function definitions in registry

### Example

```rust
// File: src/builders/unified_analysis.rs
use super::call_graph;

fn build_and_cache_graph(...) {
    call_graph::process_rust_files_for_call_graph(...)
    //    ^^^^^^^  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    //    module   function name
}
```

Resolution:
1. `call_graph` → import map → `super::call_graph`
2. `super::` → relative to `builders::unified_analysis` → `builders`
3. Full path: `builders::call_graph::process_rust_files_for_call_graph`
4. Match to `FunctionId { file: "src/builders/call_graph.rs", name: "process_rust_files_for_call_graph", ... }`
```

## Implementation Notes

### Edge Cases to Handle

1. **Ambiguous imports**:
   ```rust
   use std::collections::HashMap;
   use custom::HashMap;  // Name collision
   // Resolution: Use the last import (Rust's shadowing rules)
   ```

2. **Glob imports**:
   ```rust
   use module::*;
   function();  // Could be from glob import
   // Resolution: Best-effort - check if function exists in globbed module
   ```

3. **Re-exports**:
   ```rust
   // In module A:
   pub use other::function;
   // In caller:
   use module_a::function;
   // Resolution: Track re-exports when parsing module definitions
   ```

4. **External crates**:
   ```rust
   use serde::Serialize;
   Serialize::serialize(...)  // External crate
   // Resolution: Can't resolve - log and continue
   ```

5. **Nested super**:
   ```rust
   super::super::module::function()
   // Resolution: Pop current module twice, then resolve
   ```

### Performance Optimizations

1. **Cache import maps** per file to avoid re-parsing
2. **Use HashMap for O(1) import lookups**
3. **Early exit** on unresolved paths (don't retry multiple strategies)
4. **Lazy resolution** - only resolve when building edges

### Testing Gotchas

1. **File paths to module paths**: Ensure `src/builders/call_graph.rs` → `["builders", "call_graph"]`
2. **Module hierarchy**: Track parent-child relationships correctly
3. **Import shadowing**: Later imports shadow earlier ones
4. **Relative paths**: `super::` depends on current file's location

## Migration and Compatibility

### Breaking Changes

None - this is a pure bug fix that improves detection accuracy.

### Backward Compatibility

- Existing unqualified call detection unchanged
- Existing method call detection unchanged
- Call graph API unchanged
- Output format unchanged (just more accurate caller counts)

### Configuration

No new configuration needed. The fix is always enabled.

### Rollout Plan

1. **v0.3.0**: Implement qualified call detection
2. **v0.3.1**: Add comprehensive test coverage
3. **v0.3.2**: Performance optimization if needed
4. **v0.4.0**: Mark as stable, remove "experimental" warnings

## Success Metrics

### Quantitative Goals

1. **Accuracy**:
   - ✅ Zero false "no callers" for `process_rust_files_for_call_graph`
   - ✅ Detects ≥95% of qualified calls in debtmap's own codebase
   - ✅ <5% false negative rate on real-world projects

2. **Performance**:
   - ✅ <10% overhead on call graph construction time
   - ✅ <100ms additional time for debtmap's own analysis

3. **Coverage**:
   - ✅ 100% test coverage for qualified path parsing
   - ✅ 100% test coverage for import tracking
   - ✅ Integration test passes for debtmap self-analysis

### Qualitative Goals

1. **User Trust**:
   - No false "dead code" warnings for qualified calls
   - Recommendations are actionable and accurate
   - Users can rely on caller counts

2. **Code Quality**:
   - Clear separation of concerns (import tracking, path resolution, matching)
   - Pure functions for testability
   - Comprehensive error handling

## Future Enhancements

### Post-v0.3.0 Improvements

1. **Trait method resolution** (v0.4.0):
   - Track trait implementations
   - Resolve trait method calls
   - Handle dynamic dispatch

2. **Macro expansion support** (v0.5.0):
   - Detect calls in macro-generated code
   - Track macro invocations
   - Resolve through macro expansion

3. **Cross-crate analysis** (v0.6.0):
   - Parse dependency sources
   - Build cross-crate call graph
   - Detect external API usage

4. **Type-based resolution** (v0.7.0):
   - Use type information for disambiguation
   - Resolve method calls via type inference
   - Handle UFCS (Uniform Function Call Syntax)

## Related Issues

- Fixes false positive: `process_rust_files_for_call_graph` shows 0 callers
- Improves accuracy for any Rust codebase using qualified module calls
- Addresses #4 recommendation being incorrect in debtmap's own analysis
