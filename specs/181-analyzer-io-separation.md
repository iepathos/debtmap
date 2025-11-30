---
number: 181
title: Analyzer I/O Separation - Pure Core Implementation
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-11-30
---

# Specification 181: Analyzer I/O Separation - Pure Core Implementation

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

The current analyzer implementations violate the Stillwater philosophy's "Pure Core, Imperative Shell" principle by mixing file I/O operations directly into analysis methods. This creates several critical issues:

### Current Architecture Problems

1. **Direct File I/O in Analysis Methods**:
   - `analyzers/python.rs:71` - `std::fs::read_to_string(&python_ast.path).unwrap_or_default()`
   - `analyzers/rust.rs:207` - `std::fs::read_to_string(path).unwrap_or_default()`
   - Files are read during parsing, then re-read during analysis

2. **Silent Error Swallowing**:
   - `unwrap_or_default()` hides I/O failures completely
   - No error context or diagnostics for file read failures
   - Analyzers silently proceed with empty content on I/O errors

3. **Debug I/O Side Effects**:
   - `analyzers/rust.rs:243-249` - `eprintln!()` writes to stderr during analysis
   - Timing diagnostics mixed into pure analysis logic
   - Environment variable reads (`std::env::var()`) during analysis

4. **Testing Impediments**:
   - Cannot mock filesystem for unit tests
   - Requires real files on disk for all analyzer tests
   - Difficult to test error conditions

5. **Architectural Inconsistency**:
   - Effect system (`analyzers/effects.rs`) already provides clean I/O boundaries
   - Public API `analyze_file()` receives `content: String` parameter
   - But implementations violate the API contract by re-reading files

### Evidence of Good Design Already Present

The codebase already has the correct patterns:

**Good API** (`analyzers/effects.rs:29-39`):
```rust
pub fn analyze_file_effect(
    path: PathBuf,
    content: String,  // ✅ Content passed as parameter
    language: Language,
) -> AnalysisEffect<FileMetrics>
```

**Good Example** (`JavaScriptAst`):
```rust
pub struct JavaScriptAst {
    pub tree: tree_sitter::Tree,
    pub source: String,  // ✅ Source stored in AST
    pub path: PathBuf,
}
```

**Bad Implementation** (`analyzers/python.rs:58-75`):
```rust
fn analyze(&self, ast: &Ast) -> FileMetrics {
    // ... analysis ...
    let source_content = std::fs::read_to_string(&python_ast.path)
        .unwrap_or_default();  // ❌ Direct I/O, silent failure
    let org_analyzer = PythonOrganizationAnalyzer::new();
    let org_patterns = org_analyzer.analyze(&python_ast.module, &python_ast.path, &source_content);
}
```

## Objective

Eliminate all file I/O operations from analyzer implementations, ensuring perfect separation between pure analysis logic and I/O operations. Source content must flow through the analysis pipeline without re-reading from disk.

## Requirements

### Functional Requirements

1. **Store Source in AST Structures**:
   - Add `source: String` field to `RustAst`
   - Add `source: String` field to `PythonAst`
   - Maintain existing `source: String` in `JavaScriptAst`
   - Add `source: String` field to `TypeScriptAst`

2. **Remove Direct File I/O**:
   - Eliminate all `std::fs::read_to_string()` calls from analyzer `analyze()` methods
   - Remove `read_source_content()` function from `analyzers/rust.rs:206-208`
   - Remove file reading from `analyzers/python.rs:71`

3. **Propagate Source Content**:
   - Parser stores source content in AST during parsing
   - Analysis methods use source from AST, never from filesystem
   - Organization analyzers receive source as parameter

4. **Extract Debug I/O to Effects**:
   - Remove `eprintln!()` calls from `analyzers/rust.rs:243-249`
   - Create `log_timing_effect()` for timing diagnostics
   - Use effect system for all debug output

5. **Proper Error Handling**:
   - Never use `unwrap_or_default()` for I/O operations
   - Propagate I/O errors with full context
   - Use `AnalysisError::io_with_path()` for file errors

### Non-Functional Requirements

1. **Performance**:
   - No performance regression from storing source in AST
   - Source is already in memory from parsing, no additional allocation
   - Eliminate redundant file reads (performance improvement)

2. **Memory**:
   - Source stored once per file in AST
   - No memory overhead compared to current implementation
   - Source can be dropped after analysis completes

3. **Testing**:
   - Analyzers fully testable without filesystem
   - Mock AST creation for unit tests
   - Integration tests use effect system for file I/O

4. **Compatibility**:
   - No breaking changes to public API
   - Effect-based API remains unchanged
   - Internal refactoring only

## Acceptance Criteria

- [ ] `RustAst` struct has `source: String` field
- [ ] `PythonAst` struct has `source: String` field
- [ ] `TypeScriptAst` struct has `source: String` field
- [ ] `JavaScriptAst` already has `source: String` (verify unchanged)
- [ ] All `parse()` methods store source content in AST
- [ ] Zero `std::fs::read_to_string()` calls in `analyze()` methods
- [ ] Zero `unwrap_or_default()` on file I/O operations
- [ ] `read_source_content()` function removed from `rust.rs`
- [ ] Direct file I/O removed from `python.rs:71`
- [ ] Debug `eprintln!()` removed from `rust.rs:243-249`
- [ ] `log_timing_effect()` implemented for timing diagnostics
- [ ] All analyzer unit tests pass without filesystem access
- [ ] Integration tests use effect system for file I/O
- [ ] No performance regression (benchmark existing vs new)
- [ ] All existing functionality preserved
- [ ] Documentation updated to reflect pure analysis design

## Technical Details

### Implementation Approach

#### Phase 1: Add Source to AST Structures

**File**: `src/core/ast.rs`

```rust
#[derive(Clone, Debug)]
pub struct RustAst {
    pub file: syn::File,
    pub path: PathBuf,
    pub source: String,  // NEW: Store source content
}

#[derive(Clone, Debug)]
pub struct PythonAst {
    pub module: rustpython_parser::ast::Mod,
    pub path: PathBuf,
    pub source: String,  // NEW: Store source content
}

#[derive(Clone, Debug)]
pub struct TypeScriptAst {
    pub tree: tree_sitter::Tree,
    pub path: PathBuf,
    pub source: String,  // NEW: Store source content
}

// JavaScriptAst already has source field - no change needed
```

#### Phase 2: Update Parsers to Store Source

**File**: `src/analyzers/python.rs`

```rust
impl Analyzer for PythonAnalyzer {
    fn parse(&self, content: &str, path: PathBuf) -> Result<Ast> {
        let module = rustpython_parser::parse(
            content,
            rustpython_parser::Mode::Module,
            "<module>"
        ).map_err(|e| anyhow::anyhow!("Python parse error: {:?}", e))?;

        Ok(Ast::Python(PythonAst {
            module,
            path,
            source: content.to_string(),  // Store source
        }))
    }
}
```

**File**: `src/analyzers/rust.rs`

```rust
impl Analyzer for RustAnalyzer {
    fn parse(&self, content: &str, path: PathBuf) -> Result<Ast> {
        let file = syn::parse_file(content)
            .map_err(|e| anyhow::anyhow!("Rust parse error: {}", e))?;

        Ok(Ast::Rust(RustAst {
            file,
            path,
            source: content.to_string(),  // Store source
        }))
    }
}
```

#### Phase 3: Remove File I/O from Analysis Methods

**File**: `src/analyzers/python.rs`

**BEFORE**:
```rust
fn analyze(&self, ast: &Ast) -> FileMetrics {
    match ast {
        Ast::Python(python_ast) => {
            let mut metrics = analyze_python_file(python_ast, self.complexity_threshold);

            // ❌ BAD: Direct file I/O
            let source_content = std::fs::read_to_string(&python_ast.path)
                .unwrap_or_default();

            let org_analyzer = PythonOrganizationAnalyzer::new();
            let org_patterns = org_analyzer.analyze(
                &python_ast.module,
                &python_ast.path,
                &source_content
            );
            // ...
        }
    }
}
```

**AFTER**:
```rust
fn analyze(&self, ast: &Ast) -> FileMetrics {
    match ast {
        Ast::Python(python_ast) => {
            let mut metrics = analyze_python_file(python_ast, self.complexity_threshold);

            // ✅ GOOD: Use source from AST
            let org_analyzer = PythonOrganizationAnalyzer::new();
            let org_patterns = org_analyzer.analyze(
                &python_ast.module,
                &python_ast.path,
                &python_ast.source  // Use stored source
            );
            // ...
        }
    }
}
```

**File**: `src/analyzers/rust.rs`

**BEFORE**:
```rust
// Lines 206-208
/// Pure I/O function to read source content  // ❌ LIE: Not pure!
fn read_source_content(path: &std::path::Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

// Usage in analyze()
let source_content = read_source_content(&ast.path);  // ❌ File I/O
```

**AFTER**:
```rust
// Function deleted entirely

// Usage in analyze() - use AST source
let source_content = &rust_ast.source;  // ✅ Use stored source
```

#### Phase 4: Extract Debug I/O to Effects

**File**: `src/analyzers/rust.rs`

**BEFORE**:
```rust
fn analyze_ast_with_content(...) -> AnalysisResult {
    let start = std::time::Instant::now();
    // ... analysis ...
    let total_time = start.elapsed();

    // ❌ BAD: Side effect in pure function
    if std::env::var("DEBTMAP_TIMING").is_ok() {
        eprintln!(
            "[TIMING] analyze_ast_with_content: total={:.2}s...",
            total_time.as_secs_f64(),
        );
    }

    AnalysisResult { ... }
}
```

**AFTER**:
```rust
fn analyze_ast_with_content(...) -> (AnalysisResult, f64) {
    let start = std::time::Instant::now();
    // ... analysis ...
    let total_time = start.elapsed();

    // ✅ GOOD: Return timing, let caller handle I/O
    (AnalysisResult { ... }, total_time.as_secs_f64())
}

// NEW: Effect for timing diagnostics
fn log_timing_effect(message: String) -> AnalysisEffect<()> {
    use crate::effects::effect_from_fn;

    effect_from_fn(move |env: &RealEnv| {
        if env.config().debug_timing.unwrap_or(false) {
            eprintln!("{}", message);
        }
        Ok(())
    })
}

// Usage in effect-based analyzer
fn analyze_effect(path: PathBuf, content: String) -> AnalysisEffect<FileMetrics> {
    analyze_file_effect(path.clone(), content, Language::Rust)
        .and_then(move |metrics| {
            log_timing_effect(format!("[TIMING] analyzed {}", path.display()))
                .map(|_| metrics)
        })
}
```

#### Phase 5: Update Downstream Functions

**File**: `src/analyzers/rust.rs`

All functions that currently call `read_source_content()` need updates:

```rust
// BEFORE
fn some_analysis_function(ast: &RustAst) -> Result<...> {
    let source = read_source_content(&ast.path);  // ❌ I/O
    // ... use source ...
}

// AFTER
fn some_analysis_function(ast: &RustAst) -> Result<...> {
    let source = &ast.source;  // ✅ Use stored source
    // ... use source ...
}
```

Search for all callers:
- `grep -n "read_source_content" src/analyzers/rust.rs`
- Update each call site to use `&ast.source`

### Architecture Changes

#### Before: Mixed I/O and Analysis
```
File on disk
  ↓ (I/O)
Parse to AST (parser reads file)
  ↓
Analyze AST
  ↓ (I/O) ← ❌ PROBLEM: Re-reads file
Read file again
  ↓
Organization analysis
  ↓
Return metrics
```

#### After: Pure Core, I/O Shell
```
File on disk
  ↓ (I/O - at boundary only)
Parse to AST (stores source in AST)
  ↓ (pure)
Analyze AST (uses AST.source)
  ↓ (pure)
Organization analysis (uses AST.source)
  ↓ (pure)
Return metrics
```

### Data Structures

**Updated AST Structures** (all languages):
```rust
pub struct RustAst {
    pub file: syn::File,
    pub path: PathBuf,
    pub source: String,      // NEW
}

pub struct PythonAst {
    pub module: rustpython_parser::ast::Mod,
    pub path: PathBuf,
    pub source: String,      // NEW
}

pub struct TypeScriptAst {
    pub tree: tree_sitter::Tree,
    pub path: PathBuf,
    pub source: String,      // NEW
}

pub struct JavaScriptAst {
    pub tree: tree_sitter::Tree,
    pub source: String,      // Already exists
    pub path: PathBuf,
}
```

### APIs and Interfaces

**No Breaking Changes** - Public API remains unchanged:

```rust
// Public API (unchanged)
pub fn analyze_file(
    content: String,        // Already receives content
    path: std::path::PathBuf,
    analyzer: &dyn Analyzer,
) -> Result<FileMetrics>

// Effect API (unchanged)
pub fn analyze_file_effect(
    path: PathBuf,
    content: String,        // Already receives content
    language: Language,
) -> AnalysisEffect<FileMetrics>
```

**Internal Changes** - Trait implementations updated:

```rust
// Analyzer trait (unchanged signature)
pub trait Analyzer: Send + Sync {
    fn parse(&self, content: &str, path: std::path::PathBuf) -> Result<Ast>;
    fn analyze(&self, ast: &Ast) -> FileMetrics;  // Implementation changes
    fn language(&self) -> crate::core::Language;
}
```

## Dependencies

### Prerequisites
None - this is a foundational refactoring

### Affected Components

1. **Core AST Structures** (`src/core/ast.rs`):
   - `RustAst`, `PythonAst`, `TypeScriptAst` structs
   - Add `source: String` field to each

2. **Parser Implementations**:
   - `src/analyzers/rust.rs` - `RustAnalyzer::parse()`
   - `src/analyzers/python.rs` - `PythonAnalyzer::parse()`
   - `src/analyzers/javascript.rs` - Verify TypeScript support

3. **Analysis Implementations**:
   - `src/analyzers/rust.rs` - `RustAnalyzer::analyze()`
   - `src/analyzers/python.rs` - `PythonAnalyzer::analyze()`
   - All functions calling `read_source_content()`

4. **Organization Analyzers**:
   - Functions receiving source as parameter (verify they work with `&str`)
   - No changes needed if they already accept `&str`

5. **Test Infrastructure**:
   - Update test helpers to create ASTs with source
   - Remove filesystem dependencies from unit tests

### External Dependencies
None - internal refactoring only

## Testing Strategy

### Unit Tests

**Test AST Construction**:
```rust
#[test]
fn test_rust_ast_stores_source() {
    let source = "fn main() {}";
    let analyzer = RustAnalyzer::new();
    let ast = analyzer.parse(source, PathBuf::from("test.rs")).unwrap();

    match ast {
        Ast::Rust(rust_ast) => {
            assert_eq!(rust_ast.source, source);
        }
        _ => panic!("Expected RustAst"),
    }
}

#[test]
fn test_python_ast_stores_source() {
    let source = "def main(): pass";
    let analyzer = PythonAnalyzer::new();
    let ast = analyzer.parse(source, PathBuf::from("test.py")).unwrap();

    match ast {
        Ast::Python(python_ast) => {
            assert_eq!(python_ast.source, source);
        }
        _ => panic!("Expected PythonAst"),
    }
}
```

**Test Analysis Without Filesystem**:
```rust
#[test]
fn test_analyze_without_filesystem() {
    let source = r#"
        fn complex_function() {
            if true {
                for i in 0..10 {
                    println!("test");
                }
            }
        }
    "#;

    let analyzer = RustAnalyzer::new();
    let ast = analyzer.parse(source, PathBuf::from("memory.rs")).unwrap();
    let metrics = analyzer.analyze(&ast);

    assert!(!metrics.complexity.functions.is_empty());
    // No filesystem required!
}
```

**Test Organization Analysis**:
```rust
#[test]
fn test_organization_uses_ast_source() {
    let source = "class GodObject: ...\n";  // Truncated for test
    let analyzer = PythonAnalyzer::new();
    let ast = analyzer.parse(source, PathBuf::from("test.py")).unwrap();
    let metrics = analyzer.analyze(&ast);

    // Verify organization patterns detected
    assert!(!metrics.debt_items.is_empty());
}
```

### Integration Tests

**Test Effect System Integration**:
```rust
#[test]
fn test_analyze_file_effect_integration() {
    use crate::analyzers::effects::analyze_file_effect;
    use crate::effects::run_effect;

    let source = "fn main() {}".to_string();
    let path = PathBuf::from("test.rs");

    let effect = analyze_file_effect(path, source, Language::Rust);
    let config = DebtmapConfig::default();
    let env = RealEnv::new(config);

    let result = run_effect(effect, &env);
    assert!(result.is_ok());
}
```

### Performance Tests

**Benchmark Source Storage Overhead**:
```rust
#[bench]
fn bench_parse_with_source_storage(b: &mut Bencher) {
    let source = include_str!("../examples/complex_file.rs");
    let analyzer = RustAnalyzer::new();

    b.iter(|| {
        let ast = analyzer.parse(source, PathBuf::from("bench.rs")).unwrap();
        black_box(ast);
    });
}

#[bench]
fn bench_analyze_without_io(b: &mut Bencher) {
    let source = include_str!("../examples/complex_file.rs");
    let analyzer = RustAnalyzer::new();
    let ast = analyzer.parse(source, PathBuf::from("bench.rs")).unwrap();

    b.iter(|| {
        let metrics = analyzer.analyze(&ast);
        black_box(metrics);
    });
}
```

**Compare Before/After Performance**:
- Measure current implementation with file I/O
- Measure new implementation without file I/O
- Expect 5-10% performance improvement from eliminating redundant reads

### User Acceptance

**Verify Functionality Unchanged**:
```bash
# Run full analysis on debtmap itself
cargo run -- analyze . --format json > before.json

# After refactoring
cargo run -- analyze . --format json > after.json

# Compare results (should be identical except for timing)
diff <(jq -S 'del(.timestamp)' before.json) \
     <(jq -S 'del(.timestamp)' after.json)
```

## Documentation Requirements

### Code Documentation

**Update AST Module Docs** (`src/core/ast.rs`):
```rust
//! # Source Content Storage
//!
//! All AST structures store the original source content to enable
//! multi-pass analysis without re-reading files. This follows the
//! "Pure Core, Imperative Shell" pattern where I/O happens once at
//! the boundary and pure analysis functions use the stored content.
//!
//! ## Memory Considerations
//!
//! Source is stored as `String` in each AST. For large codebases,
//! this adds ~1MB per 1000 files (assuming 1KB average file size).
//! This is negligible compared to AST structure overhead.
```

**Update Analyzer Trait Docs** (`src/analyzers/mod.rs`):
```rust
/// Analyzer trait for language-specific parsing and analysis.
///
/// # Pure Analysis
///
/// The `analyze()` method must be pure - it receives an AST containing
/// the source content and performs analysis without any I/O operations.
/// All file reading happens in the parsing phase or at the effect boundary.
pub trait Analyzer: Send + Sync {
    /// Parse source content into an AST.
    ///
    /// The parser stores the source content in the AST for use during
    /// analysis without re-reading the file.
    fn parse(&self, content: &str, path: std::path::PathBuf) -> Result<Ast>;

    /// Analyze an AST and produce metrics.
    ///
    /// This method must be pure and perform no I/O. Use `ast.source`
    /// to access the original source content.
    fn analyze(&self, ast: &Ast) -> FileMetrics;

    fn language(&self) -> crate::core::Language;
}
```

### Architecture Updates

**Update ARCHITECTURE.md**:

Add section on I/O Separation:

```markdown
## I/O Separation (Stillwater Philosophy)

Debtmap follows the "Pure Core, Imperative Shell" architectural pattern:

### Data Flow

```
┌─────────────────────────────────────────────────────────────┐
│                     I/O Boundary (Shell)                     │
│  File Discovery → File Reading → Coverage Parsing            │
│  Effect System: AnalysisEffect<T, AnalysisError, RealEnv>  │
└────────────────────────┬────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│                    Parser Layer (Pure)                       │
│  Language Detection → AST Generation (stores source)         │
│  Pure Functions: parse(content, path) -> Result<Ast>        │
└────────────────────────┬────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│                   Analysis Layer (Pure)                      │
│  Complexity Analysis → Debt Detection → Pattern Recognition  │
│  Pure Functions: analyze(ast) -> FileMetrics                │
│  Uses ast.source, never reads filesystem                    │
└────────────────────────┬────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│                 Aggregation Layer (Pure)                     │
│  Combine Metrics → Priority Scoring → Recommendations        │
└────────────────────────┬────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│                    Output Layer (I/O Shell)                  │
│  Format Selection → Report Generation → File Writing         │
└─────────────────────────────────────────────────────────────┘
```

### Source Content Flow

1. **File Reading** (I/O boundary):
   - `read_file_effect()` reads file from disk once
   - Content passed to parser as `String`

2. **Parsing** (pure):
   - Parser stores source in AST structure
   - AST becomes self-contained data structure

3. **Analysis** (pure):
   - All analysis uses `ast.source`
   - Zero file I/O operations
   - Fully testable without filesystem

4. **Benefits**:
   - Testable: Mock AST creation, no filesystem needed
   - Performant: No redundant file reads
   - Debuggable: Clear data flow, no hidden I/O
   - Composable: Pure functions easily composed
```

### User Documentation

No user-facing documentation changes - this is internal refactoring.

## Implementation Notes

### Memory Overhead Analysis

**Source Storage Cost**:
```
Average Rust file: ~500 lines × 40 chars/line = 20KB
Average Python file: ~300 lines × 35 chars/line = 10KB
Average project: 500 files × 15KB average = 7.5MB

Additional heap allocation: 7.5MB per 500-file project
Compared to AST overhead: ~50MB typical
Percentage increase: ~15%
```

**Mitigation**: Source strings are dropped immediately after analysis completes.

### Performance Considerations

**Before** (with file re-reading):
```
1. Read file from disk for parsing: 100µs
2. Parse to AST: 500µs
3. Read file AGAIN for organization: 100µs  ← Redundant!
4. Analyze: 1ms
Total: 1.7ms per file
```

**After** (source in AST):
```
1. Read file from disk for parsing: 100µs
2. Parse to AST (store source): 500µs
3. Analyze (use ast.source): 1ms
Total: 1.6ms per file (6% faster)
```

### Gotchas

1. **AST Cloning**: Cloning AST now includes source string
   - Most ASTs are not cloned, so minimal impact
   - If cloning is needed, consider `Arc<str>` for source

2. **Source Encoding**: Ensure UTF-8 handling is consistent
   - Parsers already assume UTF-8
   - No change needed

3. **Line Ending Normalization**: Some analyzers may normalize `\r\n` to `\n`
   - Store original source as-is
   - Normalize in analysis if needed

## Migration and Compatibility

### Breaking Changes
None - this is an internal refactoring with no public API changes.

### Migration Path

**For Internal Code**:
1. Update AST struct definitions (add `source` field)
2. Update parsers to store source
3. Update analyzers to use `ast.source`
4. Remove `read_source_content()` and direct file I/O
5. Update tests to create ASTs with source

**For External Users**:
- No migration required
- Public API unchanged
- Binary compatibility maintained

### Rollback Plan

If issues arise:
1. Revert AST struct changes
2. Restore file I/O in analyzers
3. Restore `read_source_content()` function
4. Git revert is sufficient

### Compatibility Guarantees

- **Public API**: No changes to `analyze_file()` or `analyze_file_effect()`
- **CLI**: No changes to command-line interface
- **Output**: Identical analysis results
- **Performance**: Equal or better performance

## Success Metrics

### Code Quality Metrics

- [ ] Zero `std::fs` imports in analyzer implementation files
- [ ] Zero `unwrap_or_default()` on I/O operations
- [ ] 100% of analyzer unit tests run without filesystem
- [ ] Clippy clean with `#![deny(clippy::unwrap_used)]` in analyzer modules

### Performance Metrics

- [ ] Analysis throughput >= baseline (measured on debtmap itself)
- [ ] Memory usage <= baseline + 15%
- [ ] No file reads during `analyze()` (verified with strace)

### Testing Metrics

- [ ] 100% of existing tests pass
- [ ] New tests cover AST source storage
- [ ] New tests verify no filesystem access during analysis
- [ ] Integration tests verify effect system compatibility

## References

- **Stillwater Philosophy**: `/Users/glen/memento-mori/stillwater/PHILOSOPHY.md`
- **Current Effect System**: `src/effects.rs`, `src/analyzers/effects.rs`
- **AST Definitions**: `src/core/ast.rs`
- **Python Analyzer**: `src/analyzers/python.rs`
- **Rust Analyzer**: `src/analyzers/rust.rs`

## Related Issues

This specification addresses the architectural debt identified in:
- I/O separation analysis (current conversation)
- Stillwater philosophy alignment gaps
- Testing impediments from filesystem dependencies
