---
number: 127
title: Parallel Execution Pattern Detection
category: optimization
priority: critical
status: draft
dependencies: [111, 121]
created: 2025-10-25
---

# Specification 127: Parallel Execution Pattern Detection

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: Spec 111 (AST Functional Pattern Detection), Spec 121 (Cognitive Complexity)

## Context

Debtmap currently flags functions containing parallel execution patterns (rayon, tokio) as having excessive complexity requiring function extraction. This produces false positives for closure-based parallel code where extraction is impractical and would harm code clarity.

**Real-world example from ripgrep**:
- `search_parallel()`: 77 lines, cyclomatic complexity 15
- Flagged as #5 critical issue: "extract 6 functions"
- **Reality**: Rayon-based parallel file search with closure captures
- Pattern: Setup → parallel iteration with closure → post-processing
- **Not extractable**: Closure captures 6+ variables from outer scope
- **Extraction harmful**: Would require massive context passing, break natural parallel flow

Complexity in parallel code comes from **coordination** (setup, closure captures, aggregation), not algorithmic complexity. Extracting coordination logic reduces clarity without reducing cognitive load.

## Objective

Detect parallel execution patterns (rayon, tokio, std::thread) and apply appropriate complexity adjustments that recognize coordination overhead. Parallel functions should be evaluated based on coordination complexity (setup, closure capture count, aggregation) rather than raw cyclomatic complexity.

## Requirements

### Functional Requirements

1. **Parallel Library Detection**
   - Identify rayon parallel iterators (`par_iter()`, `par_bridge()`)
   - Detect tokio async patterns (`spawn()`, `join!`, `select!`)
   - Recognize std::thread usage (`thread::spawn()`, `thread::scope()`)
   - Find concurrent primitives (`Mutex`, `RwLock`, `AtomicBool`, `Channel`)

2. **Closure Analysis**
   - Count closures in function (nested closures for parallel execution)
   - Identify closure captures (variables from outer scope)
   - Measure closure complexity separately from parent function
   - Detect `move` closures for thread spawning

3. **Coordination Pattern Recognition**
   - Identify setup phase (shared state initialization)
   - Find aggregation phase (result collection, stats)
   - Detect early termination patterns (quit conditions)
   - Measure synchronization points (locks, channels, atomics)

4. **Pattern Classification**
   - Classify as Parallel Execution if:
     - Uses parallel iterator or spawn patterns
     - Contains closures with 3+ captures
     - Has setup + execution + aggregation structure
     - Cyclomatic complexity primarily from closure logic
   - Distinguish from extractable business logic:
     - Parallel: Complexity from coordination
     - Business: Complexity from algorithms

### Non-Functional Requirements

- Detection overhead: < 5% of total analysis time
- Pattern recognition accuracy: > 80% precision and recall
- Zero false negatives on legitimate complex functions
- Language support: Rust (rayon, tokio), extensible to other languages

## Acceptance Criteria

- [ ] Detect rayon parallel iterators in Rust AST
- [ ] Identify closures with capture analysis (count captured variables)
- [ ] Recognize parallel execution pattern (setup → parallel → aggregation)
- [ ] Apply 40% complexity reduction for confirmed parallel patterns
- [ ] Ripgrep's `search_parallel()` (complexity 15) severity drops from CRITICAL to MODERATE
- [ ] Recommendation focuses on closure clarity, not extraction: "Consider extracting complex closure bodies"
- [ ] Non-parallel complex functions still flagged with CRITICAL severity
- [ ] Integration tests validate against ripgrep, rayon, tokio examples
- [ ] Documentation explains why extraction is impractical for coordination code

## Technical Details

### Implementation Approach

**Phase 1: Parallel Library Detection**
```rust
#[derive(Debug, Clone)]
enum ParallelLibrary {
    Rayon,
    Tokio,
    StdThread,
    Crossbeam,
}

struct ParallelUsage {
    library: ParallelLibrary,
    parallel_calls: Vec<String>,  // par_iter, spawn, etc.
    concurrent_types: Vec<String>, // Mutex, AtomicBool, etc.
}

fn detect_parallel_usage(function: &FunctionAst) -> Option<ParallelUsage> {
    // Scan for:
    // - Method calls: .par_iter(), .par_bridge(), .spawn()
    // - Macro invocations: tokio::spawn!, join!, select!
    // - Type usage: Arc<Mutex<T>>, AtomicBool, etc.
}
```

**Phase 2: Closure Analysis**
```rust
struct ClosureInfo {
    line_number: usize,
    captures: Vec<String>,  // Variables captured from outer scope
    is_move: bool,
    closure_complexity: usize,
    lines: usize,
}

fn analyze_closures(function: &FunctionAst) -> Vec<ClosureInfo> {
    // Parse closure expressions: || { ... }, |x| { ... }
    // Identify captured variables (not parameters)
    // Measure closure complexity separately
}
```

**Phase 3: Pattern Detection**
```rust
struct ParallelPattern {
    library: ParallelLibrary,
    closure_count: usize,
    total_captures: usize,
    avg_captures_per_closure: f64,
    setup_lines: usize,
    execution_lines: usize,
    aggregation_lines: usize,
    cyclomatic_complexity: usize,
    coordination_complexity: f64,
}

fn detect_parallel_pattern(
    function: &FunctionAst,
    parallel_usage: &ParallelUsage,
    closures: &[ClosureInfo],
) -> Option<ParallelPattern> {
    // Verify parallel execution pattern:
    // 1. Setup phase (variable initialization before parallel call)
    // 2. Execution phase (parallel iterator or spawn)
    // 3. Aggregation phase (result collection after parallel)

    // Calculate coordination complexity:
    // - Closure count * capture count
    // - Synchronization point count
    // - Result aggregation complexity

    // Return pattern if:
    // - Uses parallel library
    // - Has 1+ closures with 3+ captures
    // - Clear 3-phase structure
}
```

**Phase 4: Scoring Adjustment**
```rust
fn adjust_parallel_score(
    base_score: f64,
    pattern: &ParallelPattern,
) -> f64 {
    // Reduce score for coordination complexity (expected overhead)
    let coordination_factor = if pattern.avg_captures_per_closure > 5.0 {
        0.5 // High capture count = complex coordination, expected
    } else if pattern.avg_captures_per_closure > 3.0 {
        0.6 // Moderate captures
    } else {
        0.8 // Low captures - might be extractable
    };

    // Consider closure complexity
    let closure_factor = if pattern.closure_count > 2 {
        0.9 // Multiple closures = complex coordination
    } else {
        1.0
    };

    base_score * coordination_factor * closure_factor
}
```

### Architecture Changes

**Extend `FunctionAnalysis` struct**:
```rust
pub struct FunctionAnalysis {
    // ... existing fields
    pub closures: Vec<ClosureInfo>,
    pub parallel_usage: Option<ParallelUsage>,
    pub detected_pattern: Option<DetectedPattern>,
}

pub enum DetectedPattern {
    Registry(RegistryPattern),              // Spec 124
    Builder(BuilderPattern),                 // Spec 125
    StructInitialization(StructInitPattern), // Spec 126
    ParallelExecution(ParallelPattern),      // This spec
}
```

**Modify recommendation generation**:
```rust
fn recommend_for_parallel(pattern: &ParallelPattern) -> String {
    if pattern.avg_captures_per_closure > 5.0 {
        format!(
            "Parallel execution with {} closures capturing {} variables. \
             High capture count is expected for coordination. \
             Consider: 1) Extract complex closure bodies (not coordination), \
             2) Simplify shared state if possible, \
             3) Document closure capture rationale.",
            pattern.closure_count,
            pattern.total_captures
        )
    } else {
        format!(
            "Parallel execution is appropriately complex. \
             Coordination overhead ({} captures) is expected.",
            pattern.total_captures
        )
    }
}
```

### Data Structures

```rust
pub struct ParallelPattern {
    /// Parallel library being used
    pub library: ParallelLibrary,

    /// Number of closures in function
    pub closure_count: usize,

    /// Total captured variables across all closures
    pub total_captures: usize,

    /// Average captures per closure
    pub avg_captures_per_closure: f64,

    /// Lines in setup phase (before parallel execution)
    pub setup_lines: usize,

    /// Lines in execution phase (parallel iteration/spawn)
    pub execution_lines: usize,

    /// Lines in aggregation phase (after parallel)
    pub aggregation_lines: usize,

    /// Cyclomatic complexity (for comparison)
    pub cyclomatic_complexity: usize,

    /// Coordination complexity (derived metric)
    pub coordination_complexity: f64,

    /// Synchronization primitives used
    pub sync_primitives: Vec<String>,

    /// Whether closures are move closures
    pub has_move_closures: bool,

    /// Individual closure information
    pub closures: Vec<ClosureInfo>,
}

pub struct ClosureInfo {
    /// Line number where closure starts
    pub line_number: usize,

    /// Variables captured from outer scope
    pub captures: Vec<String>,

    /// Whether closure uses `move` keyword
    pub is_move: bool,

    /// Cyclomatic complexity of closure body
    pub closure_complexity: usize,

    /// Lines of code in closure
    pub lines: usize,

    /// Whether closure could be extracted
    pub extractable: bool,
}
```

### APIs and Interfaces

**Pattern Detection API**:
```rust
pub struct ParallelPatternDetector {
    min_closure_captures: usize,
    min_parallel_calls: usize,
}

impl Default for ParallelPatternDetector {
    fn default() -> Self {
        Self {
            min_closure_captures: 3,
            min_parallel_calls: 1,
        }
    }
}

impl PatternDetector for ParallelPatternDetector {
    fn detect(&self, analysis: &FunctionAnalysis) -> Option<DetectedPattern> {
        // 1. Detect parallel library usage
        // 2. Analyze closures and captures
        // 3. Identify 3-phase structure
        // 4. Calculate coordination complexity
        // 5. Return pattern if thresholds met
    }

    fn confidence(&self) -> f64 {
        // Based on library detection and closure analysis
    }
}
```

**Closure Extraction Analysis**:
```rust
pub fn analyze_closure_extractability(closure: &ClosureInfo) -> ExtractabilityReport {
    ExtractabilityReport {
        extractable: closure.captures.len() <= 2 && closure.complexity > 10,
        reason: if closure.captures.len() > 2 {
            "Too many captures - extraction would require large parameter list"
        } else if closure.complexity <= 10 {
            "Closure is already simple - extraction adds ceremony without benefit"
        } else {
            "Closure is complex and has few captures - extraction beneficial"
        },
        estimated_benefit: calculate_extraction_benefit(closure),
    }
}

pub struct ExtractabilityReport {
    pub extractable: bool,
    pub reason: &'static str,
    pub estimated_benefit: f64,
}
```

## Dependencies

- **Prerequisites**:
  - Spec 111 (AST Functional Pattern Detection) - provides AST parsing
  - Spec 121 (Cognitive Complexity) - alternative complexity metrics
- **Affected Components**:
  - `src/debt/` - scoring algorithms
  - `src/complexity/` - complexity calculation
  - `src/analyzers/rust.rs` - Rust-specific analysis
  - `src/io/output.rs` - recommendation formatting
- **External Dependencies**: None (uses existing syn/tree-sitter)

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_detect_parallel_pattern_ripgrep() {
    let code = r#"
        fn search_parallel(args: &HiArgs) -> Result<bool> {
            let bufwtr = args.buffer_writer();
            let stats = args.stats().map(Mutex::new);
            let matched = AtomicBool::new(false);

            args.walk_builder()?.build_parallel().run(|| {
                let bufwtr = &bufwtr;
                let stats = &stats;
                let matched = &matched;

                Box::new(move |result| {
                    // ... parallel search logic
                    if search_result.has_match() {
                        matched.store(true, Ordering::SeqCst);
                    }
                    WalkState::Continue
                })
            });

            Ok(matched.load(Ordering::SeqCst))
        }
    "#;

    let analysis = analyze_function(code);
    let pattern = ParallelPatternDetector::default().detect(&analysis);

    assert!(pattern.is_some());
    let parallel = pattern.unwrap();
    assert_eq!(parallel.library, ParallelLibrary::Rayon);
    assert!(parallel.closure_count >= 1);
    assert!(parallel.total_captures >= 3);
    assert!(parallel.sync_primitives.contains(&"AtomicBool".to_string()));
}

#[test]
fn test_parallel_score_reduction() {
    let pattern = ParallelPattern {
        library: ParallelLibrary::Rayon,
        closure_count: 1,
        total_captures: 6,
        avg_captures_per_closure: 6.0,
        setup_lines: 10,
        execution_lines: 40,
        aggregation_lines: 5,
        cyclomatic_complexity: 15,
        coordination_complexity: 8.0,
        sync_primitives: vec!["AtomicBool".into(), "Mutex".into()],
        has_move_closures: true,
        closures: vec![],
    };

    let base_score = 1000.0;
    let adjusted = adjust_parallel_score(base_score, &pattern);

    // 40-50% reduction for parallel coordination
    assert!(adjusted < base_score * 0.6);
    assert!(adjusted > base_score * 0.4);
}

#[test]
fn test_closure_capture_analysis() {
    let code = r#"
        let outer_var = 42;
        let another_var = "test";

        items.par_iter().map(|item| {
            // Captures: outer_var, another_var
            process(item, outer_var, another_var)
        });
    "#;

    let closures = analyze_closures(code);
    assert_eq!(closures.len(), 1);
    assert_eq!(closures[0].captures.len(), 2);
    assert!(closures[0].captures.contains(&"outer_var".to_string()));
    assert!(closures[0].captures.contains(&"another_var".to_string()));
}

#[test]
fn test_not_parallel_regular_closure() {
    let code = r#"
        fn process_items(items: &[Item]) -> Vec<Result> {
            items.iter()
                .filter(|item| item.is_valid())
                .map(|item| transform(item))
                .collect()
        }
    "#;

    let analysis = analyze_function(code);
    let pattern = ParallelPatternDetector::default().detect(&analysis);

    // Sequential iterator with closures - not parallel pattern
    assert!(pattern.is_none());
}

#[test]
fn test_tokio_async_detection() {
    let code = r#"
        async fn fetch_all(urls: Vec<String>) -> Vec<Response> {
            let tasks: Vec<_> = urls.into_iter()
                .map(|url| tokio::spawn(async move {
                    fetch(&url).await
                }))
                .collect();

            let results = futures::future::join_all(tasks).await;
            results
        }
    "#;

    let analysis = analyze_function(code);
    let pattern = ParallelPatternDetector::default().detect(&analysis);

    assert!(pattern.is_some());
    let parallel = pattern.unwrap();
    assert_eq!(parallel.library, ParallelLibrary::Tokio);
    assert!(parallel.has_move_closures);
}
```

### Integration Tests

- **Ripgrep validation**: `search_parallel()` severity drops from CRITICAL to MODERATE
- **Rayon examples**: Test against rayon's example suite
- **Tokio examples**: Test against tokio's parallel patterns
- **False negative check**: Ensure non-parallel complex functions still flagged

### Performance Tests

```rust
#[bench]
fn bench_parallel_detection(b: &mut Bencher) {
    let ast = parse_function("test_data/parallel_search_100_lines.rs");
    b.iter(|| {
        ParallelPatternDetector::default().detect(&ast)
    });
}

#[bench]
fn bench_closure_analysis(b: &mut Bencher) {
    let ast = parse_function("test_data/multiple_closures.rs");
    b.iter(|| {
        analyze_closures(&ast)
    });
}
```

## Documentation Requirements

### Code Documentation

- Rustdoc for parallel pattern detection
- Explain coordination vs. algorithmic complexity
- Document closure capture analysis
- Provide examples of extractable vs. non-extractable closures

### User Documentation

**CLI Output Enhancement**:
```
#5 SCORE: 6.2 [MODERATE - FUNCTION - PARALLEL EXECUTION]
├─ ./crates/core/main.rs:160 search_parallel()
├─ PATTERN: Parallel Execution (rayon) - Coordination overhead
├─ WHY: Function uses rayon parallel iteration with closures capturing 6 variables.
│       Cyclomatic complexity (15) reflects coordination logic, not algorithmic
│       complexity. Coordination complexity: 6.2 (more appropriate metric).
├─ COMPLEXITY ANALYSIS:
│  ├─ Cyclomatic complexity: 15 (includes closure logic)
│  ├─ Coordination complexity: 6.2 (setup + captures + aggregation)
│  ├─ Closures: 1 (capturing 6 variables: bufwtr, stats, matched, etc.)
│  ├─ Parallel library: rayon (build_parallel)
│  ├─ Synchronization: AtomicBool, Mutex
│  └─ Structure: Setup (10 lines) → Parallel (40 lines) → Aggregation (5 lines)
├─ ACTION: Parallel coordination complexity is appropriate.
│  ├─ ❌ DO NOT extract coordination logic - closure captures make this impractical
│  ├─ ✅ Consider extracting complex closure BODY if >20 lines (current: 15 lines)
│  ├─ ✅ Document why 6 variables are captured (shared state for parallel execution)
│  └─ ✅ Simplify aggregation logic if possible
├─ CLOSURE EXTRACTABILITY:
│  ├─ Closure 1: 15 lines, 6 captures
│  └─  Extractability: LOW (too many captures for clean extraction)
├─ IMPACT: Moderate priority - pattern is appropriate for parallel execution
├─ METRICS: Closures: 1, Captures: 6, Cyc: 15, Coord: 6.2
├─ COVERAGE: 67.3% (Parallel code is hard to test exhaustively)
└─ PATTERN CONFIDENCE: 87%
```

### Architecture Updates

Update `ARCHITECTURE.md`:
- Document parallel execution pattern detection
- Explain coordination complexity vs. algorithmic complexity
- Describe when closure extraction is appropriate
- Provide guidance on capture count thresholds

## Implementation Notes

### Why Extraction Is Impractical

**Example of closure with many captures**:
```rust
// Closure captures 6 variables from outer scope
args.walk_builder()?.build_parallel().run(|| {
    let bufwtr = &bufwtr;         // Capture 1
    let stats = &stats;           // Capture 2
    let matched = &matched;       // Capture 3
    let searched = &searched;     // Capture 4
    let haystack_builder = &haystack_builder; // Capture 5
    let mut searcher = searcher.clone(); // Capture 6

    Box::new(move |result| {
        // Parallel search logic using all 6 captures
        // Extraction would require passing all 6 as parameters
        // Loses clarity of parallel execution pattern
    })
});
```

**After "extraction" (worse)**:
```rust
fn search_file(
    result: DirEntry,
    bufwtr: &BufferWriter,
    stats: &Option<Mutex<Stats>>,
    matched: &AtomicBool,
    searched: &AtomicBool,
    haystack_builder: &HaystackBuilder,
    searcher: Searcher,
) -> WalkState {
    // Same logic, but loses context of parallel execution
}

// Usage: Verbose, obscures parallel pattern
args.walk_builder()?.build_parallel().run(|| {
    // ... setup captures
    Box::new(move |result| {
        search_file(result, bufwtr, stats, matched, searched, haystack_builder, searcher)
    })
});
```

**When extraction IS appropriate**:
```rust
// Complex closure BODY (not coordination)
items.par_iter().map(|item| {
    // 50 lines of complex transformation logic
    let stage1 = complex_algorithm_1(item);
    let stage2 = complex_algorithm_2(stage1);
    let stage3 = complex_algorithm_3(stage2);
    // ... more complex logic
    stage3
});

// Should extract the complex logic, keeping closure thin:
items.par_iter().map(|item| transform_item(item));

fn transform_item(item: &Item) -> Transformed {
    let stage1 = complex_algorithm_1(item);
    let stage2 = complex_algorithm_2(stage1);
    let stage3 = complex_algorithm_3(stage2);
    stage3
}
```

### Coordination Complexity Calculation

```rust
fn calculate_coordination_complexity(pattern: &ParallelPattern) -> f64 {
    let capture_complexity = pattern.total_captures as f64 * 0.5;
    let closure_complexity = pattern.closure_count as f64 * 1.0;
    let sync_complexity = pattern.sync_primitives.len() as f64 * 0.8;
    let structure_penalty = if has_clear_structure(pattern) { 0.0 } else { 2.0 };

    capture_complexity + closure_complexity + sync_complexity + structure_penalty
}
```

### Edge Cases

- **Nested parallel execution**: Multiple levels of par_iter - aggregate captures
- **Async + parallel**: Tokio spawn inside rayon - detect both patterns
- **Sequential closure in parallel context**: Only count parallel closures
- **Closure without captures**: Low coordination complexity, might be extractable

### Language Extensions

**Python (multiprocessing)**:
```python
def process_parallel(items):
    with multiprocessing.Pool() as pool:
        results = pool.map(process_item, items)
    return results
```

**JavaScript (Promise.all)**:
```javascript
async function fetchAll(urls) {
    const promises = urls.map(url => fetch(url));
    return Promise.all(promises);
}
```

**Go (goroutines)**:
```go
func searchParallel(files []string) {
    var wg sync.WaitGroup
    results := make(chan Result, len(files))

    for _, file := range files {
        wg.Add(1)
        go func(f string) {
            defer wg.Done()
            results <- processFile(f)
        }(file)
    }

    wg.Wait()
    close(results)
}
```

## Migration and Compatibility

### Breaking Changes

None - this is a new feature that improves existing analysis.

### Backward Compatibility

- Functions with parallel patterns may see reduced complexity scores
- Recommendations will shift from "extract functions" to "extract closure bodies"
- Complexity metric will account for coordination overhead

### Migration Path

1. Deploy pattern detection alongside existing analysis
2. Validate against rayon, tokio, std::thread examples
3. Monitor false positive/negative rates
4. Enable scoring adjustments in production
5. Update user documentation with pattern explanations

### Configuration

Add optional configuration for pattern detection:

```toml
[pattern_detection]
enabled = true

[pattern_detection.parallel_execution]
min_closure_captures = 3
min_parallel_calls = 1
score_reduction = 0.40  # 40% reduction

# Coordination complexity calculation
capture_weight = 0.5
closure_weight = 1.0
sync_primitive_weight = 0.8

# Extractability thresholds
max_captures_for_extraction = 2
min_closure_lines_for_extraction = 20
```

## Success Metrics

- **False positive reduction**: 30-40% reduction for parallel execution functions
- **Ripgrep validation**: `search_parallel()` severity drops from CRITICAL to MODERATE
- **Recommendation accuracy**: Developers report closure extraction suggestions are appropriate
- **Pattern detection accuracy**: >80% precision and recall
- **Performance**: <5% analysis overhead
- **User satisfaction**: Fewer reports of impractical extraction recommendations
