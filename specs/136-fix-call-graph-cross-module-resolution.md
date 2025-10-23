---
number: 136
title: Fix Call Graph Cross-Module Function Call Resolution
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-10-22
---

# Specification 136: Fix Call Graph Cross-Module Function Call Resolution

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

The call graph analysis in debtmap currently fails to detect several common Rust function call patterns, leading to false "dead code" warnings for actively-used functions. Analysis of debtmap's self-analysis shows **30% of top 10 recommendations** have incorrect "no callers detected" warnings.

### Current Failures

When analyzing debtmap itself, the following false positives were identified:

**Entry #2**: `write_quick_wins_section()`
- **Location**: `src/io/writers/enhanced_markdown/health_writer.rs:160`
- **Claim**: "⚠ No callers detected - may be dead code"
- **Reality**: Called from `src/io/writers/enhanced_markdown/mod.rs:126`
- **Pattern**: Cross-file call within same module using `use super::function`

**Entry #5**: `process_rust_files_for_call_graph()`
- **Location**: `src/builders/call_graph.rs:53`
- **Claim**: "⚠ No callers detected - may be dead code"
- **Reality**: 3 callers using qualified paths:
  - `src/builders/unified_analysis.rs:555` → `call_graph::process_rust_files_for_call_graph(...)`
  - `src/builders/unified_analysis.rs:583` → `call_graph::process_rust_files_for_call_graph(...)`
  - `src/commands/validate.rs:258` → `call_graph::process_rust_files_for_call_graph(...)`
- **Pattern**: Qualified path calls (`module::function()`)

**Entry #6**: `handle_analyze()`
- **Location**: `src/commands/analyze.rs:69`
- **Claim**: "⚠ No callers detected - may be dead code"
- **Reality**: Multiple callers including:
  - `src/main.rs:446` → calls via wrapper `handle_analyze_command()`
  - `src/main.rs:630` → `debtmap::commands::analyze::handle_analyze(config)`
  - Re-exported: `src/commands/mod.rs:6` → `pub use analyze::handle_analyze;`
- **Pattern**: Re-exports and fully-qualified paths

### Root Causes

The call graph builder (`src/analyzers/call_graph/mod.rs` and `src/builders/call_graph.rs`) fails to resolve:

1. **Cross-file calls within same module** - `use super::function` or `use crate::module::function`
2. **Qualified path calls** - `module::submodule::function()`
3. **Re-exported functions** - `pub use other_module::function`
4. **Fully-qualified crate paths** - `crate::module::function()` or `debtmap::module::function()`

### Impact

- **False Dead Code Warnings**: 30% of top recommendations have incorrect warnings
- **Misleading Prioritization**: Functions incorrectly flagged as low-dependency
- **User Confusion**: Users waste time investigating "dead code" that's actively used
- **Trust Erosion**: Reduces confidence in debtmap's analysis accuracy

## Objective

Enhance call graph construction to accurately resolve all common Rust function call patterns, eliminating false "dead code" warnings and improving dependency tracking accuracy.

## Requirements

### Functional Requirements

1. **Cross-File Same-Module Resolution**
   - Detect calls using `use super::function` imports
   - Track function calls across files in same module
   - Resolve `mod.rs` imports from sibling files
   - Handle both `super::` and absolute module paths

2. **Qualified Path Resolution**
   - Resolve calls like `module::function()`
   - Handle multi-level paths: `module::submodule::function()`
   - Support both relative and absolute module paths
   - Map qualified paths to actual function definitions

3. **Re-Export Tracking**
   - Follow `pub use module::function` declarations
   - Build re-export map during module scanning
   - Resolve calls through re-exported names
   - Track original definition location

4. **Fully-Qualified Path Resolution**
   - Resolve `crate::module::function()` calls
   - Handle external crate references (when analyzing that crate)
   - Support nested module paths
   - Map to canonical function identifiers

5. **Import Statement Analysis**
   - Parse all `use` statements in each file
   - Build import-to-definition mapping
   - Handle glob imports (`use module::*`)
   - Track rename imports (`use module::function as alias`)

### Non-Functional Requirements

1. **Accuracy**: Reduce false "dead code" warnings by 90%+
2. **Performance**: Call graph construction time increase <10%
3. **Memory**: Import mapping should add <5MB to memory usage
4. **Completeness**: Detect 95%+ of actual function calls
5. **Backward Compatibility**: Maintain existing call graph API

## Acceptance Criteria

- [ ] **AC1**: Detect cross-file calls within same module (e.g., `health_writer.rs` → `mod.rs`)
- [ ] **AC2**: Resolve qualified path calls (e.g., `call_graph::process_rust_files_for_call_graph()`)
- [ ] **AC3**: Track function calls through re-exports (e.g., `pub use analyze::handle_analyze`)
- [ ] **AC4**: Resolve fully-qualified paths (e.g., `crate::commands::analyze::handle_analyze()`)
- [ ] **AC5**: Handle aliased imports (e.g., `use module::function as alias`)
- [ ] **AC6**: Reduce false "dead code" warnings from 30% to <3% in self-analysis
- [ ] **AC7**: Maintain existing call graph API (no breaking changes)
- [ ] **AC8**: Add comprehensive test suite covering all resolution patterns
- [ ] **AC9**: Performance overhead <10% for call graph construction
- [ ] **AC10**: Document all supported call patterns and resolution logic

## Technical Details

### Implementation Approach

#### Phase 1: Import Map Construction

Build a comprehensive import map during file analysis:

```rust
pub struct ImportMap {
    /// Maps (file, short_name) -> canonical function path
    imports: HashMap<(PathBuf, String), FunctionId>,

    /// Tracks re-exports: (module, name) -> original definition
    re_exports: HashMap<(String, String), FunctionId>,

    /// Module hierarchy for path resolution
    module_tree: ModuleTree,
}

impl ImportMap {
    /// Parse all use statements in a file
    fn analyze_imports(&mut self, file: &Path, ast: &syn::File) {
        for item in &ast.items {
            match item {
                syn::Item::Use(use_item) => {
                    self.process_use_statement(file, use_item);
                }
                _ => {}
            }
        }
    }

    /// Process a single use statement
    fn process_use_statement(&mut self, file: &Path, use_item: &syn::ItemUse) {
        // Handle:
        // - use module::function
        // - use module::function as alias
        // - use module::*
        // - use super::function
        // - use crate::module::function
    }

    /// Resolve a function call to its canonical ID
    fn resolve_call(&self, file: &Path, call_expr: &syn::ExprCall) -> Option<FunctionId> {
        // Try multiple resolution strategies:
        // 1. Check import map for short name
        // 2. Resolve qualified path
        // 3. Check re-exports
        // 4. Try module hierarchy resolution
    }
}
```

#### Phase 2: Enhanced Call Expression Analysis

Extend call site analysis to use the import map:

```rust
impl CallGraphBuilder {
    fn analyze_function_call(
        &mut self,
        current_file: &Path,
        current_function: &FunctionId,
        call_expr: &syn::ExprCall,
    ) {
        let callee = match self.resolve_callee(current_file, call_expr) {
            Some(id) => id,
            None => {
                // Log unresolved call for debugging
                self.log_unresolved_call(call_expr);
                return;
            }
        };

        self.call_graph.add_edge(current_function, &callee);
    }

    fn resolve_callee(
        &self,
        current_file: &Path,
        call_expr: &syn::ExprCall,
    ) -> Option<FunctionId> {
        // Extract function path from call expression
        let path = self.extract_call_path(call_expr)?;

        // Try resolution strategies in order:

        // 1. Direct function name (imported)
        if path.segments.len() == 1 {
            let name = path.segments[0].ident.to_string();
            if let Some(id) = self.import_map.lookup(current_file, &name) {
                return Some(id);
            }
        }

        // 2. Qualified path (module::function)
        if let Some(id) = self.resolve_qualified_path(&path) {
            return Some(id);
        }

        // 3. Check re-exports
        if let Some(id) = self.resolve_through_reexports(&path) {
            return Some(id);
        }

        None
    }
}
```

#### Phase 3: Module Hierarchy Resolution

Build module tree for path resolution:

```rust
pub struct ModuleTree {
    /// Maps module path -> file path
    modules: HashMap<String, PathBuf>,

    /// Parent-child relationships
    hierarchy: HashMap<String, Vec<String>>,
}

impl ModuleTree {
    /// Resolve a qualified path to a file
    fn resolve_module_path(&self, path: &syn::Path) -> Option<PathBuf> {
        // Convert syn::Path to module string
        let module_path = self.path_to_module_string(path);

        // Handle special cases:
        // - `super::` relative paths
        // - `crate::` absolute paths
        // - External crate paths

        self.modules.get(&module_path).cloned()
    }

    /// Resolve `super::` relative to current module
    fn resolve_super(&self, current_module: &str, segments: &[String]) -> Option<String> {
        let mut current = current_module.to_string();
        let mut segment_idx = 0;

        // Walk up hierarchy for each `super`
        while segment_idx < segments.len() && segments[segment_idx] == "super" {
            current = self.parent_module(&current)?;
            segment_idx += 1;
        }

        // Append remaining segments
        let remaining = &segments[segment_idx..];
        if !remaining.is_empty() {
            current.push_str("::");
            current.push_str(&remaining.join("::"));
        }

        Some(current)
    }
}
```

### Architecture Changes

**Modified Components:**
- `src/analyzers/call_graph/mod.rs` - Add import analysis
- `src/builders/call_graph.rs` - Enhanced call resolution
- `src/priority/call_graph.rs` - Updated FunctionId handling

**New Components:**
- `src/analyzers/call_graph/import_map.rs` - Import tracking
- `src/analyzers/call_graph/module_tree.rs` - Module hierarchy
- `src/analyzers/call_graph/path_resolver.rs` - Path resolution logic

**Data Structures:**

```rust
/// Enhanced function identifier with module context
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct FunctionId {
    pub file_path: PathBuf,
    pub module_path: String,  // NEW: e.g., "debtmap::commands::analyze"
    pub function_name: String,
    pub line_number: usize,
}

/// Import declaration tracking
#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub source_file: PathBuf,
    pub imported_path: Vec<String>,  // e.g., ["crate", "commands", "analyze", "handle_analyze"]
    pub alias: Option<String>,       // For `use x as y`
    pub is_glob: bool,               // For `use module::*`
}

/// Re-export tracking
#[derive(Debug, Clone)]
pub struct ReExport {
    pub exporting_module: String,    // Module doing the re-export
    pub exported_name: String,       // Name being exported
    pub original_definition: FunctionId,  // Original function location
}
```

### Resolution Algorithm

```
For each function call expression:

1. Extract call path (e.g., "module::function" or just "function")

2. If single-segment path (just "function"):
   a. Check import map for current file
   b. Check local functions in same file
   c. Check parent module (implicit imports)

3. If multi-segment path ("module::function"):
   a. Check if first segment is imported module
   b. Resolve qualified path through module tree
   c. Check re-exports at each level

4. Special path prefixes:
   a. "super::" - resolve relative to parent module
   b. "crate::" - resolve from crate root
   c. "self::" - resolve from current module

5. Build canonical FunctionId:
   - Determine absolute file path
   - Extract function name
   - Find line number from definition

6. Add edge to call graph
```

## Dependencies

**Prerequisites**: None

**Affected Components**:
- Call graph builders (`src/analyzers/call_graph/`, `src/builders/call_graph.rs`)
- Priority calculation (uses call graph for dependency scoring)
- Dead code detection (relies on caller count)

**External Dependencies**:
- `syn` crate (already used) - for AST parsing
- `quote` crate (already used) - for path manipulation

## Testing Strategy

### Unit Tests

1. **Import Resolution Tests**
```rust
#[test]
fn test_resolve_simple_import() {
    let code = r#"
        use other_module::helper_function;

        fn my_function() {
            helper_function();  // Should resolve
        }
    "#;

    let graph = build_call_graph(code);
    assert!(graph.has_edge("my_function", "helper_function"));
}

#[test]
fn test_resolve_qualified_path() {
    let code = r#"
        fn my_function() {
            module::submodule::helper();  // Should resolve
        }
    "#;

    let graph = build_call_graph(code);
    assert!(graph.has_edge("my_function", "module::submodule::helper"));
}

#[test]
fn test_resolve_super_import() {
    // File: src/module/submodule/file.rs
    let code = r#"
        use super::parent_function;

        fn child_function() {
            parent_function();  // Should resolve to src/module/parent_function
        }
    "#;

    let graph = build_call_graph_with_hierarchy(code);
    assert!(graph.has_edge("child_function", "parent_function"));
}

#[test]
fn test_resolve_reexport() {
    // File: src/commands/mod.rs
    let reexport = "pub use analyze::handle_analyze;";

    // File: src/main.rs
    let caller = r#"
        use commands::handle_analyze;

        fn main() {
            handle_analyze();  // Should resolve through re-export
        }
    "#;

    let graph = build_call_graph_multi_file(vec![reexport, caller]);
    assert!(graph.has_edge("main", "handle_analyze"));
}
```

2. **Real-World Test Cases**

Test against actual debtmap code patterns:
```rust
#[test]
fn test_debtmap_write_quick_wins() {
    // Should detect call from mod.rs to health_writer.rs
    let graph = analyze_real_files(vec![
        "src/io/writers/enhanced_markdown/health_writer.rs",
        "src/io/writers/enhanced_markdown/mod.rs",
    ]);

    let callers = graph.get_callers("write_quick_wins_section");
    assert!(!callers.is_empty(), "Should detect caller in mod.rs");
}

#[test]
fn test_debtmap_process_rust_files() {
    // Should detect qualified path calls
    let graph = analyze_real_files(vec![
        "src/builders/call_graph.rs",
        "src/builders/unified_analysis.rs",
    ]);

    let callers = graph.get_callers("process_rust_files_for_call_graph");
    assert_eq!(callers.len(), 3, "Should detect all 3 callers");
}
```

### Integration Tests

1. **Self-Analysis Validation**
```rust
#[test]
fn test_self_analysis_dead_code_warnings() {
    // Run debtmap on itself
    let analysis = run_debtmap_on_self();

    // Check known actively-used functions
    let false_positives = vec![
        ("write_quick_wins_section", "health_writer.rs"),
        ("process_rust_files_for_call_graph", "call_graph.rs"),
        ("handle_analyze", "analyze.rs"),
    ];

    for (func, file) in false_positives {
        let item = analysis.find_item(func, file).expect("Function should be analyzed");
        let has_callers = item.call_graph_info.caller_count > 0;
        assert!(has_callers, "{} should have detected callers", func);
    }
}
```

2. **False Positive Rate Measurement**
```rust
#[test]
fn test_false_positive_rate() {
    let analysis = run_debtmap_on_self();

    // Manually verified list of functions with known callers
    let known_used_functions = load_verified_function_list();

    let mut false_positives = 0;
    for func in known_used_functions {
        let item = analysis.find_item(&func.name, &func.file)?;
        if item.call_graph_info.caller_count == 0 {
            false_positives += 1;
        }
    }

    let fp_rate = (false_positives as f64) / (known_used_functions.len() as f64);
    assert!(fp_rate < 0.03, "False positive rate should be <3%, got {:.1}%", fp_rate * 100.0);
}
```

### Performance Tests

```rust
#[test]
fn test_call_graph_performance() {
    let large_codebase = setup_large_test_codebase();  // ~100k LOC

    let start = Instant::now();
    let _ = build_call_graph(&large_codebase);
    let duration = start.elapsed();

    let baseline = Duration::from_secs(5);  // Previous baseline
    let max_allowed = baseline + baseline / 10;  // +10% max

    assert!(duration < max_allowed,
        "Call graph construction took {:?}, baseline {:?} + 10%",
        duration, baseline);
}
```

## Documentation Requirements

### Code Documentation

- Document import map construction algorithm
- Add examples for each resolution pattern
- Explain module tree building process
- Document performance characteristics
- Include troubleshooting guide for unresolved calls

### User Documentation

Update relevant docs:
- Explain improved call graph accuracy
- Document reduction in false positives
- Provide examples of correctly detected patterns
- Add troubleshooting section for remaining edge cases

### Architecture Updates

Update `ARCHITECTURE.md`:
- Document new import map component
- Explain call resolution pipeline
- Add diagram of resolution strategy flow
- Document module hierarchy tracking

## Implementation Notes

### Debugging Unresolved Calls

Add diagnostic output for unresolved calls:

```rust
pub struct UnresolvedCall {
    pub call_site: SourceLocation,
    pub attempted_path: String,
    pub reason: UnresolvedReason,
}

pub enum UnresolvedReason {
    NoImportFound,
    ModuleNotInTree,
    ReExportChainBroken,
    ExternalCrate,
}

// Enable with DEBTMAP_DEBUG_CALLS=1
fn log_unresolved_call(&self, call: UnresolvedCall) {
    if std::env::var("DEBTMAP_DEBUG_CALLS").is_ok() {
        eprintln!("Unresolved call: {:?}", call);
    }
}
```

### Handling Edge Cases

1. **Macro-generated calls**: May not have source location
2. **Trait method calls**: Need trait resolution (future work)
3. **Dynamic dispatch**: Cannot be statically resolved
4. **Conditional compilation**: `#[cfg(...)]` may hide code

Document these limitations clearly.

### Performance Optimization

- Build import map incrementally during file scanning
- Cache module path resolutions
- Use hash-based lookups for import map
- Consider parallel import map construction

## Migration and Compatibility

### Breaking Changes

None - this is a pure enhancement to existing functionality.

### Configuration

Add optional debugging configuration:

```toml
[call_graph]
# Enable verbose logging for call resolution
debug_unresolved = false

# Warn about potentially unresolvable patterns
warn_dynamic_dispatch = true
```

### Gradual Rollout

1. **Phase 1**: Implement import map construction (no behavioral change)
2. **Phase 2**: Enable new resolution for qualified paths only
3. **Phase 3**: Enable full resolution including re-exports
4. **Phase 4**: Make new resolution the default, measure improvement
5. **Phase 5**: Remove fallback to old resolution logic

## Success Metrics

### Quantitative
- Reduce false "dead code" warnings from 30% to <3%
- Detect 95%+ of actual function calls
- Performance overhead <10%
- Zero regression in existing correct detections

### Qualitative
- Users report fewer confusing "dead code" warnings
- Improved trust in debtmap's analysis
- Better prioritization due to accurate dependency tracking
- Reduced time investigating false alarms

## Related Work

### Existing Call Graph Tools
- **rust-analyzer**: LSP-based, has full type information
- **cargo-call-stack**: Static call graph, limited resolution
- **rust-code-analysis**: Mozilla's tool, different focus

### Prior Debtmap Work
- Current call graph implementation (basic resolution)
- Function metrics and complexity analysis
- Dead code detection heuristics

## Future Enhancements

Post-implementation improvements:
1. Trait method call resolution using type inference
2. Integration with rust-analyzer for precise resolution
3. Cross-crate call graph for workspace analysis
4. Visualization of call graph with resolution confidence levels
5. Machine learning to predict likely callers for unresolved calls
