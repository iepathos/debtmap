---
number: 141
title: I/O and Side Effect Detection
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-10-27
---

# Specification 141: I/O and Side Effect Detection

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently uses name-based heuristics to classify function responsibilities (e.g., `format_*` → "Formatting & Output", `parse_*` → "Parsing & Input"). This achieves ~50% accuracy because it doesn't analyze what functions actually DO—only what they're called.

The single biggest improvement to responsibility detection comes from detecting I/O operations and side effects. Functions that read files are performing I/O regardless of whether they're named `read_file()`, `load_config()`, or `get_settings()`. Functions that mutate shared state are performing side effects regardless of naming.

This specification defines comprehensive I/O and side effect detection across all supported languages (Rust, Python, JavaScript, TypeScript), providing a foundational layer for accurate responsibility classification.

## Objective

Implement static analysis to detect I/O operations and side effects in code, enabling responsibility classification based on actual behavior rather than naming conventions. This will increase classification accuracy from ~50% (name-based) to ~65-70% (with I/O detection alone).

## Requirements

### Functional Requirements

**I/O Operation Detection**:
- Detect file system operations (read, write, open, close, create, delete)
- Detect network operations (HTTP requests, socket operations, API calls)
- Detect console I/O (print, log, input, prompt)
- Detect database operations (queries, transactions, connections)
- Detect environment variable access
- Detect system calls and process operations
- Track I/O through standard library functions and common frameworks

**Side Effect Detection**:
- Detect mutable state modifications (field assignments, array mutations)
- Detect global variable access and modification
- Detect static/class variable mutations
- Detect external state changes (cache updates, configuration changes)
- Distinguish between local mutations (acceptable) and external mutations (side effects)

**Multi-Language Support**:
- Rust: std::fs, std::io, std::net, println!, eprintln!, env::var, thread spawning
- Python: open(), pathlib, requests, urllib, print(), input(), os.environ, sys
- JavaScript/TypeScript: fs, fetch, XMLHttpRequest, console, process.env, localStorage
- Language-specific patterns (async/await, Result/Option unwrapping, error handling)

**Classification Output**:
- Categorize functions as: Pure Computation, File I/O, Network I/O, Console I/O, Database I/O, Mixed I/O, Side Effects
- Track I/O intensity (number of I/O operations per function)
- Identify I/O boundaries (where I/O transitions to computation)

### Non-Functional Requirements

- **Performance**: I/O detection must add <10% overhead to existing AST analysis
- **Accuracy**: Achieve >90% precision for I/O operation detection
- **Recall**: Detect >85% of actual I/O operations (including indirect calls)
- **Extensibility**: Support adding new I/O patterns without code changes (configuration-based)

## Acceptance Criteria

- [ ] Rust I/O detection correctly identifies std::fs operations (read_to_string, write, File::open, etc.)
- [ ] Python I/O detection correctly identifies open(), pathlib operations, and requests library
- [ ] JavaScript/TypeScript I/O detection correctly identifies fs module and fetch API
- [ ] Network I/O detection identifies HTTP client usage across all languages
- [ ] Console I/O detection identifies logging and output operations
- [ ] Side effect detection distinguishes local mutations from external state changes
- [ ] I/O detection works through function call chains (indirect I/O)
- [ ] Classification output includes I/O intensity metrics
- [ ] Performance overhead is <10% on real-world codebases
- [ ] Test suite includes examples from debtmap's own codebase (src/io/, src/analyzers/)

## Technical Details

### Implementation Approach

**Phase 1: Direct I/O Detection**
```rust
pub struct IoDetector {
    /// Language-specific I/O patterns
    patterns: HashMap<Language, IoPatternSet>,
    /// Track I/O operations per function
    operation_tracker: HashMap<FunctionId, Vec<IoOperation>>,
}

pub struct IoPatternSet {
    file_ops: Vec<Pattern>,      // std::fs::*, open(), fs.readFile
    network_ops: Vec<Pattern>,   // reqwest, requests, fetch
    console_ops: Vec<Pattern>,   // println!, print(), console.log
    db_ops: Vec<Pattern>,        // diesel, sqlx, prisma
    env_ops: Vec<Pattern>,       // env::var, os.environ, process.env
}

#[derive(Debug, Clone)]
pub enum IoOperation {
    FileRead { path_expr: Option<String> },
    FileWrite { path_expr: Option<String> },
    NetworkRequest { endpoint: Option<String> },
    ConsoleOutput { stream: OutputStream },
    DatabaseQuery { query_type: QueryType },
    EnvironmentAccess { var_name: Option<String> },
}
```

**Phase 2: Call Graph Integration**
```rust
// Detect I/O through function calls
pub fn detect_indirect_io(
    function: &FunctionAst,
    call_graph: &CallGraph,
    io_cache: &mut HashMap<FunctionId, IoProfile>
) -> IoProfile {
    let mut profile = detect_direct_io(function);

    // Propagate I/O through call chain
    for call in function.calls() {
        if let Some(callee_profile) = io_cache.get(&call.target) {
            profile.merge(callee_profile);
        }
    }

    profile
}
```

**Phase 3: Side Effect Detection**
```rust
#[derive(Debug, Clone)]
pub enum SideEffect {
    /// Mutation of field in self or other object
    FieldMutation { target: String, field: String },
    /// Mutation of global/static variable
    GlobalMutation { name: String },
    /// Array/collection mutation
    CollectionMutation { operation: CollectionOp },
    /// External state change (cache, config, etc.)
    ExternalState { description: String },
}

pub fn detect_side_effects(function: &FunctionAst) -> Vec<SideEffect> {
    let mut effects = Vec::new();

    for stmt in function.statements() {
        match stmt {
            // self.field = value
            Assignment { target: FieldAccess { .. }, .. } => {
                effects.push(SideEffect::FieldMutation { .. });
            }
            // GLOBAL_VAR = value
            Assignment { target: Global { .. }, .. } => {
                effects.push(SideEffect::GlobalMutation { .. });
            }
            // vec.push(x), map.insert(k, v)
            MethodCall { method: "push" | "insert" | "remove", .. } => {
                effects.push(SideEffect::CollectionMutation { .. });
            }
            _ => {}
        }
    }

    effects
}
```

### Architecture Changes

**New Module**: `src/analysis/io_detection.rs`
- Core I/O and side effect detection logic
- Language-agnostic detection framework
- I/O pattern matching engine

**New Module**: `src/analysis/io_patterns/`
- `rust.rs` - Rust-specific I/O patterns
- `python.rs` - Python-specific I/O patterns
- `javascript.rs` - JavaScript/TypeScript patterns
- `patterns.toml` - Configuration file for extensible patterns

**Integration Point**: `src/organization/god_object_analysis.rs`
- Enhance `infer_responsibility_from_method()` to use I/O detection
- Replace name-based heuristics with behavior-based classification
- Combine I/O signals with name signals for hybrid approach

### Data Structures

```rust
#[derive(Debug, Clone)]
pub struct IoProfile {
    pub file_operations: Vec<IoOperation>,
    pub network_operations: Vec<IoOperation>,
    pub console_operations: Vec<IoOperation>,
    pub database_operations: Vec<IoOperation>,
    pub side_effects: Vec<SideEffect>,
    pub is_pure: bool,  // No I/O or side effects
}

impl IoProfile {
    /// Classify responsibility based on I/O pattern
    pub fn primary_responsibility(&self) -> Responsibility {
        match (
            self.file_operations.is_empty(),
            self.network_operations.is_empty(),
            self.console_operations.is_empty(),
            self.is_pure
        ) {
            (false, _, _, _) => Responsibility::FileIO,
            (_, false, _, _) => Responsibility::NetworkIO,
            (_, _, false, _) => Responsibility::ConsoleIO,
            (true, true, true, true) => Responsibility::PureComputation,
            _ => Responsibility::MixedIO,
        }
    }

    /// I/O intensity score (higher = more I/O heavy)
    pub fn intensity(&self) -> f64 {
        (self.file_operations.len() +
         self.network_operations.len() +
         self.console_operations.len() +
         self.database_operations.len()) as f64
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Responsibility {
    PureComputation,
    FileIO,
    NetworkIO,
    ConsoleIO,
    DatabaseIO,
    MixedIO,
    SideEffects,
}
```

### APIs and Interfaces

```rust
/// Main API for I/O detection
pub trait IoAnalyzer {
    /// Detect I/O operations in a function
    fn analyze_function(&self, function: &FunctionAst) -> IoProfile;

    /// Detect I/O across entire file
    fn analyze_file(&self, ast: &FileAst) -> HashMap<FunctionId, IoProfile>;
}

/// Language-specific implementations
impl IoAnalyzer for RustIoAnalyzer { ... }
impl IoAnalyzer for PythonIoAnalyzer { ... }
impl IoAnalyzer for JavaScriptIoAnalyzer { ... }
```

## Dependencies

- **Prerequisites**: None - this is a foundational specification
- **Affected Components**:
  - `src/organization/god_object_analysis.rs` - responsibility classification
  - `src/analysis/` - new io_detection module
  - Existing AST parsers (will be queried for I/O patterns)
- **External Dependencies**: None (uses existing tree-sitter and AST infrastructure)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_file_io_detection() {
        let code = r#"
        fn read_config() -> String {
            std::fs::read_to_string("config.toml").unwrap()
        }
        "#;

        let ast = parse_rust(code);
        let profile = RustIoAnalyzer::new().analyze_function(&ast.functions[0]);

        assert_eq!(profile.file_operations.len(), 1);
        assert_eq!(profile.primary_responsibility(), Responsibility::FileIO);
    }

    #[test]
    fn python_network_io_detection() {
        let code = r#"
        def fetch_data():
            response = requests.get('https://api.example.com/data')
            return response.json()
        "#;

        let ast = parse_python(code);
        let profile = PythonIoAnalyzer::new().analyze_function(&ast.functions[0]);

        assert_eq!(profile.network_operations.len(), 1);
        assert_eq!(profile.primary_responsibility(), Responsibility::NetworkIO);
    }

    #[test]
    fn pure_function_detection() {
        let code = r#"
        fn calculate_sum(a: i32, b: i32) -> i32 {
            a + b
        }
        "#;

        let ast = parse_rust(code);
        let profile = RustIoAnalyzer::new().analyze_function(&ast.functions[0]);

        assert!(profile.is_pure);
        assert_eq!(profile.primary_responsibility(), Responsibility::PureComputation);
    }

    #[test]
    fn side_effect_detection() {
        let code = r#"
        fn update_cache(cache: &mut HashMap<String, String>, key: String, value: String) {
            cache.insert(key, value);
        }
        "#;

        let ast = parse_rust(code);
        let profile = RustIoAnalyzer::new().analyze_function(&ast.functions[0]);

        assert_eq!(profile.side_effects.len(), 1);
        assert!(matches!(
            profile.side_effects[0],
            SideEffect::CollectionMutation { .. }
        ));
    }
}
```

### Integration Tests

```rust
#[test]
fn analyze_debtmap_io_module() {
    // Test on debtmap's own code
    let files = vec![
        "src/io/reader.rs",
        "src/io/writer.rs",
        "src/analyzers/rust_analyzer.rs",
    ];

    for file in files {
        let ast = parse_file(file);
        let profiles = analyze_file(&ast);

        // Verify detection accuracy
        assert!(profiles.values().any(|p| !p.file_operations.is_empty()));
    }
}
```

### Performance Tests

```rust
#[test]
fn io_detection_performance() {
    let large_file = parse_file("src/priority/formatter.rs"); // 2889 lines

    let start = Instant::now();
    let _ = analyze_file(&large_file);
    let duration = start.elapsed();

    // Should add <10% overhead
    assert!(duration < Duration::from_millis(100));
}
```

## Documentation Requirements

### Code Documentation

- Comprehensive rustdoc for all public types
- Examples of I/O pattern detection for each language
- Documentation of I/O pattern configuration format

### User Documentation

Update README.md:
```markdown
## Responsibility Detection

Debtmap uses multi-signal analysis to classify function responsibilities:

1. **I/O Detection** (40% weight): Analyzes actual I/O operations
   - File system operations
   - Network requests
   - Console output
   - Database queries

2. **Side Effect Detection**: Identifies state mutations
   - Field assignments
   - Global variable changes
   - Collection modifications

3. **Name Heuristics** (fallback): Function name patterns
```

### Architecture Updates

Update ARCHITECTURE.md with new analysis pipeline:
```markdown
## Analysis Pipeline

1. AST Parsing (language-specific)
2. **I/O Detection** ← NEW
   - Identify I/O operations
   - Detect side effects
   - Build I/O profiles
3. Responsibility Classification (combines I/O + names)
4. God Object Detection
5. Recommendation Generation
```

## Implementation Notes

### Pattern Configuration Format

Create `src/analysis/io_patterns/patterns.toml`:
```toml
[rust.file_io]
patterns = [
    "std::fs::read_to_string",
    "std::fs::write",
    "std::fs::File::open",
    "std::fs::File::create",
    "std::path::Path::read_*",
]

[rust.network_io]
patterns = [
    "reqwest::*",
    "hyper::*",
    "std::net::TcpStream",
]

[python.file_io]
patterns = [
    "open",
    "pathlib.Path.read_text",
    "pathlib.Path.write_text",
    "os.path.*",
]
```

### Indirect I/O Detection

For accurate detection, track I/O through call chains:
```rust
// Direct I/O: Easy to detect
fn read_file() -> String {
    std::fs::read_to_string("file.txt").unwrap()  // ← Detected
}

// Indirect I/O: Requires call graph
fn load_config() -> Config {
    let content = read_file();  // ← Should inherit I/O from read_file()
    parse_config(&content)
}
```

### Performance Optimization

Cache I/O profiles to avoid recomputation:
```rust
pub struct IoDetectionCache {
    cache: DashMap<FunctionId, IoProfile>,
}

impl IoDetectionCache {
    pub fn get_or_compute(
        &self,
        function_id: FunctionId,
        compute: impl FnOnce() -> IoProfile
    ) -> IoProfile {
        self.cache
            .entry(function_id)
            .or_insert_with(|| compute())
            .clone()
    }
}
```

## Migration and Compatibility

### Breaking Changes

None - this is additive functionality.

### Gradual Rollout

1. **Phase 1**: Implement I/O detection without changing classification
   - Add I/O detection module
   - Build test suite
   - Verify accuracy

2. **Phase 2**: Integrate with responsibility classification
   - Combine I/O signals with name heuristics
   - A/B test classification accuracy
   - Tune weights

3. **Phase 3**: Full deployment
   - Make I/O detection primary signal
   - Demote name heuristics to fallback
   - Document behavior change

### Backward Compatibility

Preserve existing classification API:
```rust
// Old API still works (calls enhanced version internally)
pub fn infer_responsibility_from_method(method_name: &str) -> String {
    // Delegates to new classify_responsibility() with I/O detection
    ...
}
```

## Expected Impact

### Accuracy Improvement

- **Current accuracy**: ~50% (name-based heuristics)
- **With I/O detection**: ~65-70% (behavior-based)
- **Improvement**: +15-20 percentage points

### Examples

**Before (name-based)**:
```rust
fn get_settings() -> Settings { ... }  // Classified as "Data Access" (name pattern)
```

**After (I/O-based)**:
```rust
fn get_settings() -> Settings {
    std::fs::read_to_string("settings.toml")  // ← Detected
        .map(|s| parse_toml(&s))
}
// Correctly classified as "File I/O"
```

### Foundation for Multi-Signal

This specification provides the foundation for Spec 145 (Multi-Signal Aggregation):
- I/O detection: 40% weight
- Call graph analysis (Spec 142): 30% weight
- Type signatures (Spec 147): 15% weight
- Side effects (this spec): 10% weight
- Name heuristics: 5% weight
- **Combined accuracy**: ~88%
