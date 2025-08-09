---
number: 02
title: Complete Debtmap Implementation - Technical Debt Detection and Functional Enhancements
category: core
priority: critical
status: draft
dependencies: [01]
created: 2025-08-09
---

# Specification 02: Complete Debtmap Implementation - Technical Debt Detection and Functional Enhancements

**Category**: core
**Priority**: critical
**Status**: draft
**Dependencies**: [01-debtmap-standalone-tool]

## Context

The initial implementation of debtmap (Specification 01) has successfully created a working tool with approximately 75% of planned features. Core complexity analysis works well, but critical technical debt detection features are non-functional or missing. Key issues include broken line number tracking, non-working TODO/FIXME detection, silent duplication analysis, and missing code smell detection beyond complexity metrics.

This specification defines the work required to bring debtmap to 100% feature completion, focusing on fixing broken features, implementing missing technical debt detection capabilities, and enhancing the functional programming architecture as originally specified.

## Objective

Complete the debtmap implementation by fixing all broken features, implementing missing technical debt detection capabilities, adding proper source location tracking, enabling language filtering, implementing caching for incremental analysis, and fully adopting the functional programming patterns specified in the original design.

## Requirements

### Functional Requirements

#### Fix Critical Bugs
- **Line Number Tracking**: Parser must capture and report actual line numbers for all functions
- **TODO/FIXME Detection**: Scan source files for debt markers and include in reports
- **Duplication Reporting**: Enable AST-based duplicate code detection with output
- **Dependency Analysis**: Implement proper module dependency tracking and reporting
- **Language Filtering**: Make --languages flag actually filter analysis by language

#### Complete Technical Debt Detection
- **Code Smell Detection**: Identify patterns beyond complexity
  - Long parameter lists (>5 parameters)
  - Large classes/modules (>300 lines)
  - Feature envy (methods using other class data more than own)
  - Data clumps (repeated groups of parameters)
- **Circular Dependencies**: Detect and report circular module dependencies
- **Coupling Metrics**: Calculate afferent/efferent coupling for modules
- **Dead Code Detection**: Find unused functions and variables

#### Performance and Caching
- **Incremental Analysis**: Cache results based on file content hashes
- **Parallel Processing**: Ensure rayon properly parallelizes file analysis
- **Large Codebase Support**: Optimize for 50k+ line codebases (<5 second analysis)

#### Functional Programming Enhancements
- **Persistent Data Structures**: Migrate to `im` crate for immutable collections
- **Lazy Evaluation**: Implement lazy analysis pipelines for efficiency
- **Monadic Patterns**: Enhanced error handling with combinators
- **Function Composition**: Improve pipeline composability

### Non-Functional Requirements

- **Performance**: Process 50,000+ lines of code in under 5 seconds
- **Memory**: Efficient memory usage with persistent data structures
- **Accuracy**: 100% accurate line number reporting
- **Reliability**: No silent failures in analysis components
- **Maintainability**: Clean functional architecture with high test coverage

## Acceptance Criteria

### Bug Fixes
- [ ] Line numbers correctly reported for all functions in Rust and Python
- [ ] TODO/FIXME/HACK/XXX markers detected and included in debt reports
- [ ] Code duplication analysis produces visible output with similarity percentages
- [ ] Dependency command shows actual module dependencies, not general analysis
- [ ] Language filtering via --languages flag works correctly

### Technical Debt Detection
- [ ] Detects functions with >5 parameters as "long parameter list" smell
- [ ] Identifies files/modules >300 lines as "large class" smell
- [ ] Detects circular dependencies between modules
- [ ] Calculates and reports coupling metrics (afferent/efferent)
- [ ] At least 5 distinct debt types reported (TODO, complexity, duplication, smells, dependencies)

### Performance
- [ ] Analyzes 50,000+ line codebase in under 5 seconds
- [ ] Incremental analysis uses cache for unchanged files
- [ ] Parallel processing utilized for multi-file analysis
- [ ] Memory usage remains stable for large codebases

### Functional Architecture
- [ ] Core data structures use `im` crate for persistence
- [ ] Analysis pipelines support lazy evaluation
- [ ] Monadic error handling with proper combinators
- [ ] Function composition demonstrated in pipeline construction

### Testing
- [ ] Property-based tests using proptest for core algorithms
- [ ] Performance benchmarks for large codebase analysis
- [ ] Integration tests for all debt detection features
- [ ] Unit tests achieve >80% code coverage

## Technical Details

### Implementation Approach

#### Phase 1: Fix Critical Bugs (Priority 1)

1. **Line Number Tracking**
```rust
// Fix in analyzers/rust.rs and analyzers/python.rs
impl Visitor for FunctionVisitor {
    fn visit_item_fn(&mut self, node: &ItemFn) {
        let span = node.span();
        let start = span.start();
        let line = self.source_map.lookup_line(start).line;
        // Store actual line number instead of 0
    }
}
```

2. **TODO/FIXME Detection**
```rust
// Add to debt/patterns.rs
pub fn scan_todos(content: &str) -> Vec<DebtItem> {
    let todo_regex = Regex::new(r"(?i)(TODO|FIXME|HACK|XXX):?\s*(.*)").unwrap();
    content.lines()
        .enumerate()
        .flat_map(|(line_num, line)| {
            todo_regex.captures(line).map(|cap| DebtItem {
                debt_type: DebtType::Todo,
                file: path.clone(),
                line: line_num + 1,
                message: cap.get(2).map_or("", |m| m.as_str()).to_string(),
                severity: match cap.get(1).unwrap().as_str().to_uppercase().as_str() {
                    "FIXME" | "XXX" => Severity::High,
                    "HACK" => Severity::Medium,
                    _ => Severity::Low,
                },
            })
        })
        .collect()
}
```

3. **Enable Duplication Detection**
```rust
// Fix in debt/duplication.rs
pub fn detect_duplicates(files: &[ParsedFile]) -> Vec<DuplicationBlock> {
    let mut duplicates = Vec::new();
    let mut hash_map: HashMap<u64, Vec<CodeBlock>> = HashMap::new();
    
    for file in files {
        for block in extract_code_blocks(file) {
            let hash = calculate_ast_hash(&block);
            hash_map.entry(hash).or_default().push(block);
        }
    }
    
    for (_, blocks) in hash_map.iter() {
        if blocks.len() > 1 {
            duplicates.push(DuplicationBlock {
                locations: blocks.iter().map(|b| b.location.clone()).collect(),
                lines: blocks[0].lines,
                similarity: calculate_similarity(&blocks),
            });
        }
    }
    
    duplicates
}
```

#### Phase 2: Code Smell Detection

1. **Long Parameter Lists**
```rust
pub fn detect_long_parameter_list(func: &Function) -> Option<CodeSmell> {
    if func.params.len() > 5 {
        Some(CodeSmell {
            smell_type: SmellType::LongParameterList,
            location: func.location.clone(),
            message: format!("Function has {} parameters (max: 5)", func.params.len()),
            severity: Severity::Medium,
        })
    } else {
        None
    }
}
```

2. **Large Classes/Modules**
```rust
pub fn detect_large_module(module: &Module) -> Option<CodeSmell> {
    if module.line_count > 300 {
        Some(CodeSmell {
            smell_type: SmellType::LargeClass,
            location: module.location.clone(),
            message: format!("Module has {} lines (max: 300)", module.line_count),
            severity: Severity::Medium,
        })
    } else {
        None
    }
}
```

#### Phase 3: Dependency Analysis

1. **Module Dependency Tracking**
```rust
#[derive(Debug, Clone)]
pub struct ModuleDependency {
    pub module: String,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub afferent_coupling: usize,  // Modules depending on this
    pub efferent_coupling: usize,  // Modules this depends on
}

pub fn analyze_dependencies(modules: &[Module]) -> DependencyGraph {
    let mut graph = DependencyGraph::new();
    
    for module in modules {
        let imports = extract_imports(module);
        let exports = extract_exports(module);
        graph.add_module(module.name.clone(), imports, exports);
    }
    
    graph.calculate_coupling_metrics();
    graph.detect_circular_dependencies();
    graph
}
```

2. **Circular Dependency Detection**
```rust
pub fn detect_circular_dependencies(graph: &DependencyGraph) -> Vec<CircularDependency> {
    let mut visited = HashSet::new();
    let mut path = Vec::new();
    let mut cycles = Vec::new();
    
    fn dfs(node: &str, graph: &DependencyGraph, 
           visited: &mut HashSet<String>, 
           path: &mut Vec<String>,
           cycles: &mut Vec<CircularDependency>) {
        if path.contains(&node.to_string()) {
            let cycle_start = path.iter().position(|n| n == node).unwrap();
            cycles.push(CircularDependency {
                modules: path[cycle_start..].to_vec(),
            });
            return;
        }
        
        if visited.contains(node) {
            return;
        }
        
        visited.insert(node.to_string());
        path.push(node.to_string());
        
        for dep in graph.get_dependencies(node) {
            dfs(dep, graph, visited, path, cycles);
        }
        
        path.pop();
    }
    
    for module in graph.modules() {
        dfs(module, &graph, &mut visited, &mut path, &mut cycles);
    }
    
    cycles
}
```

#### Phase 4: Functional Enhancements

1. **Migrate to Persistent Data Structures**
```rust
use im::{HashMap, Vector, HashSet};

#[derive(Clone, Debug)]
pub struct AnalysisState {
    pub files: Vector<FileAnalysis>,
    pub metrics: HashMap<PathBuf, FileMetrics>,
    pub cache: HashMap<u64, AnalysisResult>,
}

impl AnalysisState {
    pub fn add_file(&self, file: FileAnalysis) -> Self {
        Self {
            files: self.files.push_back(file),
            ..self.clone()
        }
    }
}
```

2. **Lazy Evaluation Pipeline**
```rust
pub struct LazyPipeline<T> {
    source: Box<dyn Iterator<Item = T>>,
    transformers: Vec<Box<dyn Fn(T) -> T>>,
}

impl<T> LazyPipeline<T> {
    pub fn new(source: impl Iterator<Item = T> + 'static) -> Self {
        Self {
            source: Box::new(source),
            transformers: Vec::new(),
        }
    }
    
    pub fn transform(mut self, f: impl Fn(T) -> T + 'static) -> Self {
        self.transformers.push(Box::new(f));
        self
    }
    
    pub fn evaluate(self) -> impl Iterator<Item = T> {
        self.source.map(move |item| {
            self.transformers.iter().fold(item, |acc, f| f(acc))
        })
    }
}
```

3. **Monadic Error Handling**
```rust
use std::result;

pub type Result<T> = result::Result<T, AnalysisError>;

pub trait ResultExt<T> {
    fn and_then_async<F, U>(self, f: F) -> Result<U>
    where
        F: FnOnce(T) -> Result<U>;
    
    fn or_else_with<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>;
    
    fn map_err_context(self, context: &str) -> Result<T>;
}

impl<T> ResultExt<T> for Result<T> {
    fn and_then_async<F, U>(self, f: F) -> Result<U>
    where
        F: FnOnce(T) -> Result<U>,
    {
        self.and_then(f)
    }
    
    fn or_else_with<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>,
    {
        self.or_else(|_| f())
    }
    
    fn map_err_context(self, context: &str) -> Result<T> {
        self.map_err(|e| e.with_context(context))
    }
}
```

#### Phase 5: Caching and Incremental Analysis

1. **Content-Based Caching**
```rust
use sha2::{Sha256, Digest};

#[derive(Clone, Debug)]
pub struct AnalysisCache {
    cache_dir: PathBuf,
    index: HashMap<u64, CacheEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub file_hash: u64,
    pub timestamp: DateTime<Utc>,
    pub metrics: FileMetrics,
}

impl AnalysisCache {
    pub fn get_or_compute<F>(&mut self, path: &Path, compute: F) -> Result<FileMetrics>
    where
        F: FnOnce() -> Result<FileMetrics>,
    {
        let content = std::fs::read_to_string(path)?;
        let hash = calculate_hash(&content);
        
        if let Some(entry) = self.index.get(&hash) {
            if entry.timestamp > last_modified(path)? {
                return Ok(entry.metrics.clone());
            }
        }
        
        let metrics = compute()?;
        self.store(hash, metrics.clone())?;
        Ok(metrics)
    }
}
```

### Architecture Changes

1. **Enhanced Module Structure**
```rust
src/
├── core/
│   ├── cache.rs              // New: Caching implementation
│   ├── lazy.rs               // New: Lazy evaluation pipelines
│   └── monadic.rs            // New: Monadic error handling
├── debt/
│   ├── smells.rs             // New: Code smell detection
│   ├── circular.rs           // New: Circular dependency detection
│   └── coupling.rs           // New: Coupling metrics
└── analyzers/
    ├── source_map.rs         // New: Source location tracking
    └── incremental.rs        // New: Incremental analysis
```

## Dependencies

- **Prerequisites**: Specification 01 (base implementation)
- **New Crate Dependencies**:
  - `im` (v15.1+) - Persistent immutable data structures
  - `proptest` (v1.0+) - Property-based testing
  - `criterion` (v0.5+) - Performance benchmarking
  - `regex` (v1.5+) - Pattern matching for TODOs
  - `petgraph` (v0.6+) - Graph algorithms for dependencies

## Testing Strategy

### Unit Tests
- Test each debt detection algorithm in isolation
- Verify line number accuracy for various code structures
- Test caching with different file modifications
- Validate lazy pipeline evaluation

### Integration Tests
```rust
#[test]
fn test_full_analysis_pipeline() {
    let test_project = create_test_project();
    let results = analyze_project(&test_project).unwrap();
    
    assert!(results.technical_debt.todos.len() > 0);
    assert!(results.technical_debt.duplications.len() > 0);
    assert!(results.dependencies.circular.is_empty());
    assert_eq!(results.complexity.metrics[0].line, 10); // Not 0
}
```

### Property-Based Tests
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_complexity_invariants(code in arb_valid_rust_code()) {
        let metrics = calculate_complexity(&code);
        prop_assert!(metrics.cyclomatic >= 1);
        prop_assert!(metrics.cognitive >= metrics.cyclomatic - 1);
        prop_assert!(metrics.nesting <= metrics.cognitive);
    }
}
```

### Performance Benchmarks
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_large_codebase(c: &mut Criterion) {
    let codebase = generate_large_codebase(50_000);
    
    c.bench_function("analyze_50k_lines", |b| {
        b.iter(|| analyze_project(black_box(&codebase)))
    });
}

criterion_group!(benches, bench_large_codebase);
criterion_main!(benches);
```

## Documentation Requirements

### Code Documentation
- Document all new public APIs with rustdoc
- Include examples for lazy pipeline usage
- Document caching behavior and configuration
- Explain monadic error handling patterns

### User Documentation
- Update README with new debt detection features
- Document cache configuration and management
- Add troubleshooting guide for common issues
- Include performance tuning recommendations

## Migration and Compatibility

### Breaking Changes
- Data structure changes require cache invalidation
- New debt types change JSON output structure
- Persistent data structures change internal APIs

### Migration Path
1. Clear existing cache before first run
2. Update CI/CD scripts for new JSON structure
3. Review threshold configurations for new metrics

## Implementation Notes

### Development Phases
1. **Week 1**: Fix critical bugs (line numbers, TODOs, duplication)
2. **Week 2**: Implement code smell detection
3. **Week 3**: Add dependency analysis and circular detection
4. **Week 4**: Migrate to functional architecture with im crate
5. **Week 5**: Implement caching and performance optimization
6. **Week 6**: Add comprehensive testing and documentation

### Risk Mitigation
- Keep existing functionality working during migration
- Implement feature flags for experimental features
- Maintain backward compatibility for JSON output where possible
- Profile performance impact of persistent data structures

### Success Metrics
- All acceptance criteria met
- Performance target achieved (<5 seconds for 50k lines)
- Test coverage >80%
- Zero regression in existing features
- Clean architecture with functional patterns