# DebtMap Architecture

## Overview

DebtMap is a high-performance technical debt analyzer that provides unified analysis of code quality metrics across multiple programming languages. The architecture is designed for scalability, performance, and extensibility.

## Core Components

### 1. Language Analyzers
- **FileAnalyzer**: Trait-based abstraction for language-specific analysis
- **RustAnalyzer**: Rust-specific implementation using syn for AST parsing
- **PythonAnalyzer**: Python-specific implementation using tree-sitter
- **Support for**: Rust, Python, JavaScript, TypeScript, Go

### 2. Unified Analysis Engine
- **UnifiedAnalysis**: Coordinates all analysis phases
- **ParallelUnifiedAnalysis**: High-performance parallel implementation
- **DebtAggregator**: Aggregates metrics across functions and files

### 3. Metrics Collection
- **Cyclomatic Complexity**: Control flow complexity measurement
- **Cognitive Complexity**: Human readability assessment
- **Function Metrics**: Lines of code, parameters, nesting depth
- **File Metrics**: Module-level aggregation
- **Test Coverage**: Integration with lcov data via indexed lookups

## Parallel Processing Architecture

### Overview
The parallel processing system leverages Rayon for CPU-bound parallel execution, enabling analysis of large codebases in sub-second time for typical projects.

### Parallelization Strategy

#### Phase 1: Initialization (Parallel)
All initialization tasks run concurrently using Rayon's parallel iterators:
- **Data Flow Graph Construction**: Build control and data flow graphs
- **Purity Analysis**: Identify pure vs impure functions
- **Test Detection**: Optimized O(n) detection with caching
- **Initial Debt Aggregation**: Baseline metric collection

#### Phase 2: Analysis (Parallel with Batching)
- **Function Analysis**: Process functions in configurable batches
- **File Analysis**: Parallel file-level metric aggregation
- **Batch Size**: Default 100 items, tunable via options

#### Phase 3: Aggregation (Sequential)
- **Result Merging**: Combine parallel results
- **Sorting**: Priority-based ranking
- **Final Scoring**: Apply weights and thresholds

### Performance Optimizations

#### Test Detection Optimization
```rust
// Original O(n²) approach
for function in functions {
    for test in tests {
        // Check if function is called by test
    }
}

// Optimized O(n) approach with caching
let test_cache = build_test_cache(&tests);
functions.par_iter().map(|f| {
    test_cache.is_tested(f)  // O(1) lookup
})
```

#### Parallel Configuration
- **Default**: Uses all available CPU cores
- **Configurable**: `--jobs N` flag for explicit control
- **Adaptive**: Batch size adjusts based on workload

### Thread Safety

#### Shared State Management
- **Arc<RwLock>**: For read-heavy shared data (call graphs, metrics)
- **Arc<Mutex>**: For write-heavy operations (progress tracking)
- **Immutable Structures**: Prefer immutable data where possible

#### Lock-Free Operations
- Use atomic operations for counters
- Batch updates to reduce contention
- Local accumulation with final merge

### Performance Targets

| Codebase Size | Target Time | Actual (Parallel) | Actual (Sequential) |
|---------------|-------------|-------------------|---------------------|
| 50 files      | <0.5s       | ~0.3s            | ~1.2s              |
| 250 files     | <1s         | ~0.8s            | ~5s                |
| 1000 files    | <5s         | ~3.5s            | ~20s               |

### Memory Management

#### Streaming Architecture
- Process files in batches to control memory usage
- Release intermediate results after aggregation
- Use iterators over collections where possible

#### Cache Efficiency
- Test detection cache reduces redundant computation
- Function signature caching for call graph
- Metric result caching for unchanged files
- Coverage index for O(1) coverage lookups

### Multi-Index Lookup Architecture

DebtMap uses a multi-index architecture for the call graph to enable fast lookups across different matching strategies without sacrificing memory efficiency.

#### Index Structure

The `CallGraph` maintains four complementary indexes:

1. **Primary Index** (`nodes: HashMap<FunctionId, FunctionNode>`)
   - **Purpose**: Exact lookups with full metadata
   - **Key**: Complete `FunctionId` (file, name, line, module_path)
   - **Complexity**: O(1)
   - **Use**: 92% of lookups hit this index

2. **Fuzzy Index** (`fuzzy_index: HashMap<FuzzyFunctionKey, Vec<FunctionId>>`)
   - **Purpose**: Match by name + file, ignoring line numbers
   - **Key**: `(canonical_file, normalized_name)`
   - **Complexity**: O(1) lookup + O(k) disambiguation (k = candidates)
   - **Use**: Generic functions, line drift scenarios

3. **Name Index** (`name_index: HashMap<String, Vec<FunctionId>>`)
   - **Purpose**: Cross-file lookups by function name only
   - **Key**: Normalized function name (generics stripped)
   - **Complexity**: O(1) lookup + O(n) disambiguation (n = all matching functions)
   - **Use**: Rare cases with incomplete metadata

4. **Caller/Callee Indexes** (`caller_index`, `callee_index`)
   - **Purpose**: Efficient traversal of call graph edges
   - **Key**: `FunctionId`
   - **Value**: `HashSet<FunctionId>` of connected functions
   - **Complexity**: O(1) lookup + O(d) iteration (d = degree of node)
   - **Use**: Reachability analysis, transitive closure

#### Index Maintenance

All indexes are kept in sync automatically:

```rust
pub fn add_function(&mut self, id: FunctionId, ...) {
    // 1. Add to primary index
    self.nodes.insert(id.clone(), node);

    // 2. Populate fuzzy index
    let fuzzy_key = id.fuzzy_key();
    self.fuzzy_index.entry(fuzzy_key).or_default().push(id.clone());

    // 3. Populate name index
    let normalized_name = FunctionId::normalize_name(&id.name);
    self.name_index.entry(normalized_name).or_default().push(id);
}
```

**Invariants Maintained**:
- Every `FunctionId` in `nodes` appears in exactly one `fuzzy_index` entry
- Every `FunctionId` in `nodes` appears in exactly one `name_index` entry
- All `FunctionId` references in `caller_index`/`callee_index` exist in `nodes`

#### Memory Overhead Analysis

**Primary Index**:
- ~200 bytes per function (FunctionId + FunctionNode)
- For 10,000 functions: ~2 MB

**Fuzzy Index**:
- ~100 bytes per unique (file, name) pair
- Typically 90-95% as many entries as primary index (few duplicates)
- For 10,000 functions: ~1 MB

**Name Index**:
- ~80 bytes per unique function name
- Much fewer entries (many functions share names across files)
- For 10,000 functions: ~200 KB

**Caller/Callee Indexes**:
- ~150 bytes per edge
- Typical call graph has 2-3x as many edges as nodes
- For 10,000 functions with 25,000 edges: ~3.75 MB

**Total Overhead**: ~7 MB for a 10,000 function codebase (acceptable)

#### Build Time Performance

Index construction is incremental during graph building:

- **Primary index update**: O(1) per function
- **Fuzzy index update**: O(1) amortized (hash table insertion)
- **Name index update**: O(1) amortized
- **Caller/callee index update**: O(1) per edge

**Overall Complexity**: O(n + e) where n = nodes, e = edges

**Measured Performance** (on debtmap self-analysis):
- 1,200 functions, 3,500 edges
- Index build time: ~8ms (< 5% of total analysis time)

#### Lookup Performance Guarantee

The multi-index architecture provides performance guarantees for all lookup patterns:

| Lookup Pattern | Strategy Used | Worst-Case Complexity |
|---------------|---------------|----------------------|
| Exact match | Primary index | O(1) |
| Same function, different line | Fuzzy index | O(1) + O(k) where k ≈ 2-3 |
| Generic instantiation | Fuzzy index | O(1) + O(1) (single candidate) |
| Cross-file by name | Name index | O(1) + O(m) where m = overloads |
| Find all callers | Caller index | O(1) + O(d) where d = in-degree |
| Find all callees | Callee index | O(1) + O(d) where d = out-degree |

**Key Insight**: The worst-case disambiguation factor (k, m) is bounded by practical limits:
- k ≤ 10 (rarely more than 10 functions with same name in one file)
- m ≤ 50 (rarely more than 50 functions with identical name across codebase)

#### Serialization Strategy

**Challenge**: The fuzzy and name indexes are derived data - they can be rebuilt from the primary index.

**Solution**: Skip serialization of derived indexes to reduce JSON size:

```rust
#[derive(Serialize, Deserialize)]
pub struct CallGraph {
    #[serde(with = "function_id_map")]
    pub nodes: HashMap<FunctionId, FunctionNode>,  // Serialized

    #[serde(skip)]
    pub fuzzy_index: HashMap<FuzzyFunctionKey, Vec<FunctionId>>,  // Rebuilt on load

    #[serde(skip)]
    pub name_index: HashMap<String, Vec<FunctionId>>,  // Rebuilt on load
}
```

**Benefits**:
- 40% smaller serialized size (only primary data stored)
- Faster deserialization (less JSON to parse)
- Rebuild cost is negligible (~8ms for 1,200 functions)

#### Parallel Lookup Safety

All indexes are immutable after construction during the analysis phase:

- **During construction**: Single-threaded, indexes mutated via `add_function()`
- **During analysis**: Multi-threaded, all indexes are read-only

This enables lock-free parallel lookups across all indexes without synchronization overhead.

#### Future Optimizations

**Potential Improvements**:
1. **Compact Index**: Use integer IDs instead of full `FunctionId` in secondary indexes (50% space reduction)
2. **Lazy Name Index**: Build name index on-demand for rare cross-file lookups (save 200 KB)
3. **Bloom Filters**: Add bloom filter for fast negative lookups (eliminate futile searches)
4. **Incremental Updates**: Support adding functions without full rebuild

**Trade-off Analysis**:
- Current design prioritizes simplicity and correctness
- Memory overhead is acceptable for projects up to 100K functions
- Optimization effort should focus on analysis algorithms, not indexing

## Call Graph

### FunctionId Matching Strategies

DebtMap uses a sophisticated multi-level matching strategy to resolve function references in the call graph, enabling accurate call graph construction even when exact metadata (line numbers, module paths) is unavailable or inconsistent.

#### The Problem

Call graph construction faces several challenges:

1. **Generic Functions**: Same function with different type parameters (e.g., `map<T>` vs `map<String>`)
2. **Line Number Drift**: AST line numbers may differ from call site line numbers due to macros, attributes, or comments
3. **Cross-Module Calls**: Calls to functions in other files may lack full metadata
4. **Incomplete Information**: Some analysis passes may only have function names, not full context

Traditional exact matching (all fields must match) causes false negatives in these scenarios, resulting in incomplete call graphs and inaccurate reachability analysis.

#### Three-Tier Matching Strategy

DebtMap implements a fallback chain with three matching strategies:

##### 1. Exact Match (Fastest)
- **Key**: `(file, name, line, module_path)` - all fields must match
- **Use Case**: Most common case when full metadata is available
- **Complexity**: O(1) hash lookup
- **Example**: Looking up `foo` at `src/main.rs:100` with full context

##### 2. Fuzzy Match (Moderate)
- **Key**: `(canonical_file, normalized_name)` - ignores line and module path
- **Normalization**: Strips generic type parameters and whitespace
  - `map<T>` → `map`
  - `process< A , B >` → `process`
- **Use Case**: Generic instantiations, line number drift
- **Complexity**: O(1) hash lookup + O(n) disambiguation if multiple candidates
- **Example**: `map<String>` at line 150 finds `map` defined at line 100

**Disambiguation**: If multiple candidates found (e.g., overloaded functions), choose by:
- **Line Proximity**: Select function closest to query line number
- **Module Path**: Prefer function with matching module path

##### 3. Name-Only Match (Slowest)
- **Key**: `normalized_name` - only function name matters
- **Use Case**: Cross-file calls, incomplete metadata
- **Complexity**: O(1) hash lookup + O(n) disambiguation across all matching functions
- **Example**: Call to `parse_config` without file context finds all `parse_config` functions

**Disambiguation**: Prioritize by:
1. **Module Path Match**: If query has module path, prefer exact match
2. **Line Proximity**: Choose function with closest line number

#### Name Normalization

Function name normalization ensures consistent matching across generic instantiations:

```rust
// Before normalization:
"map<T>"           // Generic parameter
"map<String>"      // Concrete type
"process< A , B >" // Whitespace variation

// After normalization (FunctionId::normalize_name):
"map"              // Generic parameter stripped
"map"              // Concrete type stripped
"process"          // Whitespace and generics stripped
```

**Preserved Elements**:
- Namespace qualifiers: `std::vec::Vec` → `std::vec::Vec`
- Module paths: `crate::module::function` → `crate::module::function`

#### Lookup Flow

```
Query: FunctionId { file: "src/main.rs", name: "map<String>", line: 150, ... }
    ↓
[1. Exact Lookup]
    nodes.get(query) → None (no exact match)
    ↓
[2. Fuzzy Lookup]
    fuzzy_key = (canonical_path("src/main.rs"), normalize("map<String>"))
              = (src/main.rs, "map")
    fuzzy_index.get(fuzzy_key) → [map@100]
    Single candidate → Return map@100 ✓
```

If multiple candidates:
```
[2. Fuzzy Lookup]
    fuzzy_index.get(fuzzy_key) → [map@100, map@200]
    disambiguate_by_line(candidates, 150)
        → abs_diff(100, 150) = 50
        → abs_diff(200, 150) = 50
        → Return map@100 (first match in tie) ✓
```

If fuzzy fails:
```
[3. Name-Only Lookup]
    name_index.get("map") → [src/main.rs:map@100, src/util.rs:map@50]
    disambiguate_by_module(candidates, "main")
        → src/main.rs:map@100 has module "main" → Return ✓
```

#### Performance Characteristics

| Strategy | Lookup Complexity | Disambiguation | Accuracy |
|----------|-------------------|----------------|----------|
| Exact | O(1) | None | 100% (when metadata available) |
| Fuzzy | O(1) + O(k) | k = candidates in same file | 95% (handles generics, line drift) |
| Name-Only | O(1) + O(n) | n = all functions with name | 80% (cross-file, may be ambiguous) |

**Typical Distribution** (empirical data from debtmap self-analysis):
- 92% resolved by exact match
- 7% resolved by fuzzy match
- 1% resolved by name-only match

#### Integration with Call Graph Construction

When adding a function call, the matching strategy determines the target:

```rust
// Example: Processing a call to "map<String>"
let query = FunctionId::new(file, "map<String>".to_string(), 150);
let target = graph.find_function(&query);

match target {
    Some(func_id) => graph.add_call(caller, func_id, CallType::Direct),
    None => {
        // Function not in graph - may be external dependency
        log::warn!("Unresolved call to {}", query.name);
    }
}
```

#### Benefits

- **Reduced False Negatives**: Generic functions and line drift no longer break call graph
- **Improved Reachability**: Cross-file calls correctly identified
- **Graceful Degradation**: Falls back to less precise matching when exact data unavailable
- **Minimal Performance Cost**: Indexing overhead is ~5% of total analysis time

#### Testing

Comprehensive unit tests validate all matching strategies:

- `test_exact_lookup`: Verifies O(1) exact matching
- `test_fuzzy_lookup_different_line`: Line number drift handling
- `test_fuzzy_lookup_generic_function`: Generic type parameter normalization
- `test_name_only_lookup`: Cross-file resolution
- `test_disambiguate_by_line_proximity`: Tie-breaking by line distance
- `test_disambiguate_by_module_path`: Module path preference

See `src/priority/call_graph/graph_operations.rs:367-484` for test implementations.

## Call Graph Debug and Validation Infrastructure

DebtMap includes comprehensive debugging and validation tools for the call graph system, enabling developers and users to understand, troubleshoot, and validate function resolution.

### Architecture Components

#### CallGraphDebugger

Located in `src/analyzers/call_graph/debug.rs`, the debugger provides detailed insights into call resolution:

**Core Responsibilities:**
- Record resolution attempts (successful and failed)
- Track resolution strategies and their effectiveness
- Measure performance metrics (timing percentiles)
- Generate detailed reports in text or JSON format

**Data Structures:**

```rust
pub struct CallGraphDebugger {
    attempts: Vec<ResolutionAttempt>,      // All resolution attempts
    trace_functions: HashSet<String>,       // Functions to trace
    stats: ResolutionStatistics,            // Aggregate statistics
    config: DebugConfig,                    // Output configuration
}

pub struct ResolutionAttempt {
    caller: FunctionId,                     // Calling function
    callee_name: String,                    // Target function name
    strategy_attempts: Vec<StrategyAttempt>, // Strategies tried
    result: Option<FunctionId>,             // Final resolution
    duration: Duration,                     // Time spent
}

pub enum ResolutionStrategy {
    Exact,      // Exact name and location match
    Fuzzy,      // Normalized name with disambiguation
    NameOnly,   // Name-only match across all files
}
```

**Output Formats:**
- **Text**: Human-readable report with sections, statistics, and recommendations
- **JSON**: Machine-parsable format for tooling integration

**Statistics Tracked:**
- Total resolution attempts
- Success/failure rates
- Strategy effectiveness (which strategies work best)
- Performance percentiles (p50, p95, p99)
- Common failure patterns

#### CallGraphValidator

Located in `src/analyzers/call_graph/validation.rs`, the validator checks structural integrity:

**Core Responsibilities:**
- Detect structural issues (dangling edges, orphaned nodes, duplicates)
- Identify heuristic warnings (suspicious patterns)
- Calculate overall health score (0-100)
- Generate actionable validation reports

**Validation Checks:**

1. **Structural Issues** (Critical):
   - **Dangling Edges**: Calls to non-existent functions
   - **Orphaned Nodes**: Functions with no incoming or outgoing edges
   - **Duplicate Nodes**: Same function registered multiple times

2. **Heuristic Warnings** (Suspicious Patterns):
   - **High Fan-In**: Functions with >50 callers (potential bottlenecks)
   - **High Fan-Out**: Functions calling >50 others (potential god objects)
   - **Files with No Calls**: All functions in a file are uncalled (potential dead code)
   - **Unused Public Functions**: Public functions with no callers

**Health Score Calculation:**
```rust
health_score = 100
    - (structural_issues_count × 10)  // Critical: -10 points each
    - (warnings_count × 2)             // Minor: -2 points each
```

**Interpretation:**
- **95-100**: Excellent call graph quality
- **85-94**: Good, acceptable for production
- **<85**: Needs attention, high unresolved rate

#### Integration with Analysis Pipeline

The debug and validation infrastructure integrates into the analyze command at `src/commands/analyze.rs`:

```rust
// After unified analysis completes
if config.debug_call_graph || config.validate_call_graph {
    handle_call_graph_diagnostics(&unified_analysis, &config)?;
}

fn handle_call_graph_diagnostics(...) {
    // 1. Run validation if requested
    if config.validate_call_graph {
        let report = CallGraphValidator::validate(call_graph);
        // Output validation report to stderr
    }

    // 2. Run debug output if requested
    if config.debug_call_graph {
        let mut debugger = CallGraphDebugger::new(config);
        debugger.finalize_statistics();
        debugger.write_report(&mut stdout)?;
    }

    // 3. Show statistics if requested
    if config.call_graph_stats_only {
        // Output quick statistics
    }
}
```

### CLI Flags

**Debug Flags:**
- `--debug-call-graph`: Enable debug mode with detailed resolution reports
- `--debug-format <text|json>`: Output format (default: text)
- `--trace-function <name>`: Trace specific functions (repeatable)

**Validation Flags:**
- `--validate-call-graph`: Run structural validation checks
- `--call-graph-stats-only`: Show only statistics (fast, minimal output)

**Verbosity:**
- `-v`: Show validation warnings in addition to structural issues
- `-vv`: Show successful resolutions in debug output

### Performance Considerations

**Debug Mode Overhead:**
- Baseline: <5% overhead (primarily I/O for report generation)
- With tracing: 10-15% overhead (depends on trace scope)
- Target: <20% overhead per spec 149

**Optimization Strategies:**
1. **Lazy Statistics**: Only calculate percentiles when finalized
2. **Selective Tracing**: Filter by function name to reduce recording
3. **Stream Output**: Write reports incrementally rather than buffering
4. **Minimal Recording**: Record only essential data during resolution

**Memory Usage:**
- Debug mode stores resolution attempts (typically <10MB for 1000 functions)
- Validation mode operates in-place with minimal allocation
- Statistics use aggregated counters, not raw data

### Future Enhancements

**Potential Improvements:**

1. **Deep CallResolver Integration**: Currently the debugger is invoked after analysis completes and reports on the final call graph structure. Future work could instrument `CallResolver::resolve_call()` to record individual resolution attempts with timing and strategy details, providing more granular debugging information.

2. **Interactive Debug Mode**: Real-time resolution tracing with breakpoints

3. **Visual Call Graph**: Generate GraphViz/DOT files for visualization

4. **Resolution Confidence Scores**: Assign confidence levels to resolved calls

5. **Automated Fixes**: Suggest code changes to improve resolution

6. **Continuous Monitoring**: Track resolution quality over time in CI/CD

### Testing

**Integration Tests:** `tests/call_graph_debug_output_test.rs`
- Debug flag produces expected output format
- Validation report includes health score
- JSON format is valid and parseable
- Text format is human-readable
- Performance overhead stays within bounds
- Trace function filtering works correctly
- Combined debug+validate flags work together

**Unit Tests:**
- `src/analyzers/call_graph/debug.rs`: Debugger functionality
- `src/analyzers/call_graph/validation.rs`: Validator checks

### Documentation

**User Documentation:** `README.md` - "Debugging Call Graph Issues" section
- Command examples for common scenarios
- Interpretation guide for health scores and statistics
- Performance considerations for large codebases
- Troubleshooting common issues

**Architecture Documentation:** This section
- Component responsibilities and data structures
- Integration points and control flow
- Performance characteristics and optimization strategies
- Future enhancement opportunities

## Coverage Indexing System

### Overview
The coverage indexing system provides high-performance test coverage lookups during file analysis with minimal overhead. It transforms O(n) linear searches through LCOV data into O(1) hash lookups and O(log n) range queries.

### Design

#### Two-Level Index Architecture
The `CoverageIndex` uses a dual indexing strategy:

1. **Primary Index (HashMap)**: O(1) exact lookups
   - Key: `(PathBuf, String)` - file path and function name
   - Value: `FunctionCoverage` - coverage data including percentage and uncovered lines
   - Use case: When exact function name is known from AST analysis

2. **Secondary Index (BTreeMap)**: O(log n) line-based lookups
   - Outer: `HashMap<PathBuf, BTreeMap<usize, FunctionCoverage>>`
   - Inner BTreeMap: Maps start line → function coverage
   - Use case: Fallback when function names mismatch between AST and LCOV

#### Performance Characteristics

| Operation | Complexity | Use Case |
|-----------|-----------|----------|
| Index Build | O(n) | Once at startup, where n = coverage records |
| Exact Name Lookup | O(1) | Primary lookup method |
| Line-Based Lookup | O(log m) | Fallback, where m = functions in file |
| Batch Parallel Lookup | O(n/p) | Multiple lookups, where p = CPU cores |

#### Memory Footprint
- **Estimated**: ~200 bytes per coverage record
- **Typical**: 1-2 MB for medium projects (5000 functions)
- **Large**: 10-20 MB for large projects (50000 functions)
- **Trade-off**: Acceptable memory overhead for massive performance gain

### Thread Safety

#### Arc-Wrapped Sharing
The coverage index is wrapped in `Arc<CoverageIndex>` for lock-free sharing across parallel threads:

```rust
pub struct LcovData {
    coverage_index: Arc<CoverageIndex>,
    // ...
}
```

#### Benefits
- **Zero-cost sharing**: No mutex locks during reads
- **Clone-friendly**: Arc clone is cheap (atomic refcount increment)
- **Parallel-safe**: Multiple threads can query simultaneously without contention

### Performance Targets

The coverage indexing system maintains performance overhead within acceptable limits:

| Metric | Target | Measured |
|--------|--------|----------|
| Index build time | <50ms for 5000 records | ~20-30ms |
| Lookup time (exact) | <1μs per lookup | ~0.5μs |
| Lookup time (line-based) | <10μs per lookup | ~5-8μs |
| Analysis overhead | ≤3x baseline | ~2.5x actual |

**Baseline**: File analysis without coverage lookups (~53ms for 100 files)
**Target**: File analysis with coverage lookups (≤160ms)
**Actual**: Typically achieves ~130-140ms with indexed lookups

### Usage Patterns

#### During LCOV Parsing
```rust
let data = parse_lcov_file(path)?;
// Index is automatically built at end of parsing
// data.coverage_index is ready for use
```

#### During File Analysis (Parallel)
```rust
files.par_iter().for_each(|file| {
    // Each thread can query the shared Arc<CoverageIndex>
    let coverage = data.get_function_coverage(file, function_name);
    // O(1) lookup with no lock contention
});
```

#### Batch Queries for Efficiency
```rust
let queries = collect_all_function_queries();
let results = data.batch_get_function_coverage(&queries);
// Parallel batch processing using rayon
```

### Implementation Notes

#### Name Matching Strategies
The system tries multiple strategies to match functions:
1. Exact name match (primary)
2. Line-based match with tolerance (±2 lines)
3. Boundary-based match for accurate AST ranges

#### Tolerance for AST/LCOV Discrepancies
Line numbers may differ between AST and LCOV due to:
- Comment handling differences
- Macro expansion
- Attribute processing

The 2-line tolerance handles most real-world cases.

### Future Optimizations
- **Incremental updates**: Rebuild only changed files
- **Compressed storage**: Use compact representations for large datasets
- **Lazy loading**: Build index on-demand per file
- **Persistent cache**: Serialize index to disk for faster startup

## Metric Categories (Spec 118)

### Overview

Debtmap distinguishes between two fundamental categories of metrics to help users understand which metrics are precise measurements versus heuristic estimates. This distinction is critical for proper usage in CI/CD pipelines and decision-making.

### Measured Metrics

**Definition**: Metrics computed directly from Abstract Syntax Tree (AST) analysis.

**Characteristics**:
- **Deterministic**: Same code always produces the same value
- **Precise**: Exact counts from syntax parsing, not approximations
- **Language-specific**: Uses language parsers (syn for Rust, tree-sitter for others)
- **Suitable for thresholds**: Reliable for quality gates and CI/CD enforcement

**Examples**:

| Metric | Description | Computation Method |
|--------|-------------|-------------------|
| `cyclomatic_complexity` | Decision point count | Count if, match, while, for, && , \|\| , ? |
| `cognitive_complexity` | Readability measure | Weighted nesting and control flow analysis |
| `nesting_depth` | Maximum nesting levels | Track depth during AST traversal |
| `loc` | Lines of code | Physical line count from source |
| `parameter_count` | Function parameters | Count items in function signature |

**Usage in CI/CD**:
```bash
# GOOD: Use measured metrics for quality gates
debtmap validate . --threshold-complexity 15 --max-critical 0

# These thresholds are precise and repeatable
```

### Estimated Metrics

**Definition**: Heuristic approximations calculated using formulas, not direct AST measurements.

**Characteristics**:
- **Heuristic**: Based on mathematical formulas and assumptions
- **Approximate**: Close estimates, not exact counts
- **Useful for prioritization**: Help estimate effort and risk
- **Not suitable for hard thresholds**: Use for relative comparisons, not absolute gates

**Examples**:

| Metric | Formula | Purpose | Limitations |
|--------|---------|---------|-------------|
| `est_branches` | `max(nesting, 1) × cyclomatic ÷ 3` | Estimate test cases needed | Project-specific, not comparable across codebases |

**Formula Rationale**:
- **Nesting multiplier**: Deeper nesting creates exponentially more path combinations
- **Cyclomatic base**: More decision points → more paths
- **÷ 3 adjustment**: Empirical factor based on typical branch coverage patterns

**Usage in Analysis**:
```rust
// Internal calculation (example from recommendation.rs)
let est_branches = func.nesting.max(1) * cyclomatic / 3;

// Used in recommendations:
// "With ~12 estimated branches and complexity 15/8,
//  this represents high risk. Minimum 8 test cases needed."
```

### Terminology Evolution

#### Before Spec 118: "branches"
- Displayed as `branches=8` in terminal output
- Caused user confusion:
  - Assumed to be precise AST measurement
  - Confused with cyclomatic complexity
  - Unclear that it was formula-based

#### After Spec 118: "est_branches"
- Renamed to `est_branches=8` to make estimation explicit
- Benefits:
  - **Clear intent**: "est_" prefix indicates approximation
  - **Avoid confusion**: Distinct from cyclomatic complexity
  - **Correct expectations**: Users know it's a heuristic

**Implementation Changes**:
```rust
// Before (misleading):
format!("branches={}", branch_count)

// After (clear):
format!("est_branches={}", branch_count)  // Estimation made explicit

// Added documentation comments:
// est_branches: Estimated execution paths (heuristic)
// Formula: max(nesting, 1) × cyclomatic ÷ 3
// Note: This is an ESTIMATE, not a count from the AST
```

### Design Principles

#### Principle 1: Precision Transparency
Users must know whether a metric is measured or estimated.

**Bad**:
```
complexity=12, branches=8  # Ambiguous: Is "branches" measured or estimated?
```

**Good**:
```
cyclomatic=12, est_branches=8  # Clear: "est_" indicates estimation
```

#### Principle 2: Appropriate Usage
Measured metrics for enforcement, estimated metrics for guidance.

**Measured metrics**:
- CI/CD quality gates
- Code review standards
- Cross-project comparisons
- Compliance requirements

**Estimated metrics**:
- Prioritization heuristics
- Effort estimation
- Risk assessment
- Testing guidance

#### Principle 3: Formula Documentation
All estimated metrics must document their formula and rationale.

Example from `print_metrics_explanation()`:
```rust
println!("### Estimated Metrics");
println!("  • est_branches: Estimated execution paths");
println!("    Formula: max(nesting_depth, 1) × cyclomatic_complexity ÷ 3");
println!("    Purpose: Estimate test cases needed for branch coverage");
println!("    Note: This is an ESTIMATE, not a count from the AST");
```

### Data Flow Integration

```
File Analysis
    ↓
[AST Parsing]
    ↓
MEASURED METRICS:
  ├─ cyclomatic_complexity (count decision points)
  ├─ cognitive_complexity (weighted readability)
  ├─ nesting_depth (track max nesting)
  ├─ loc (count lines)
  └─ parameter_count (count params)
    ↓
ESTIMATED METRICS:
  └─ est_branches = f(nesting, cyclomatic)  [calculated on-demand]
    ↓
Risk Scoring & Prioritization
    ↓
Output Formatting
  ├─ Terminal: Shows est_branches
  ├─ JSON: Only measured metrics serialized
  └─ Verbose: Explains formulas
```

### Future Enhancements

**Planned estimated metrics**:
- `est_test_cases`: Estimated test cases for full coverage
- `est_effort_hours`: Estimated refactoring effort
- `est_bug_density`: Predicted bug probability

**Validation framework**:
- Empirical validation of estimation formulas
- A/B testing of formula variations
- Confidence intervals for estimates

**Metric metadata**:
```rust
pub struct MetricMetadata {
    name: String,
    category: MetricCategory,  // Measured | Estimated
    formula: Option<String>,   // For estimated metrics
    suitable_for_thresholds: bool,
    documentation_url: String,
}
```

### References

- **User Documentation**: `book/src/metrics-reference.md`
- **CLI Help**: `debtmap analyze --explain-metrics`
- **FAQ**: `book/src/faq.md#measured-vs-estimated`
- **Implementation**: `src/priority/scoring/recommendation.rs`

## Data Structures

### FunctionId Keys and Indexes

The call graph uses specialized key types to enable efficient multi-strategy lookups while maintaining type safety and clarity.

#### Core Types

##### FunctionId (Primary Identifier)

```rust
pub struct FunctionId {
    pub file: PathBuf,
    pub name: String,
    pub line: usize,
    pub module_path: String,
}
```

**Purpose**: Uniquely identifies a function in the codebase with complete metadata.

**Design Decisions**:
- **PathBuf for file**: Supports platform-specific paths and canonicalization
- **String for name**: Generic instantiations stored as `map<T>`, `map<String>`, etc.
- **usize for line**: AST-reported line number (1-indexed)
- **String for module_path**: Rust module hierarchy (e.g., `crate::analysis::complexity`)

**Usage**: Primary key in `CallGraph.nodes` HashMap

##### ExactFunctionKey (Exact Match)

```rust
pub struct ExactFunctionKey {
    pub file: PathBuf,
    pub name: String,
    pub line: usize,
    pub module_path: String,
}
```

**Purpose**: Key for exact matching - all fields must match.

**Relationship to FunctionId**: Identical structure but semantically distinct (key vs identifier).

**Generation**: `func_id.exact_key()` clones all fields

**Hash/Eq Implementation**: Derives hash and equality from all four fields

##### FuzzyFunctionKey (Fuzzy Match)

```rust
pub struct FuzzyFunctionKey {
    pub canonical_file: PathBuf,
    pub normalized_name: String,
}
```

**Purpose**: Key for fuzzy matching - ignores line numbers and module paths.

**Normalization**:
- **canonical_file**: Canonicalized path (resolves symlinks, relative paths)
- **normalized_name**: Generic parameters stripped (`map<T>` → `map`)

**Generation**: `func_id.fuzzy_key()`
```rust
FuzzyFunctionKey {
    canonical_file: FunctionId::canonicalize_path(&self.file),
    normalized_name: FunctionId::normalize_name(&self.name),
}
```

**Hash/Eq Implementation**: Only considers file and normalized name

**Example**:
```rust
// These two FunctionIds produce the same FuzzyFunctionKey
let id1 = FunctionId::new("src/main.rs", "map<T>", 100);
let id2 = FunctionId::new("src/main.rs", "map<String>", 150);

assert_eq!(id1.fuzzy_key(), id2.fuzzy_key());
```

##### SimpleFunctionKey (Name-Only Match)

```rust
pub struct SimpleFunctionKey {
    pub normalized_name: String,
}
```

**Purpose**: Key for name-only matching - ignores file, line, and module path.

**Normalization**: Same as `FuzzyFunctionKey` (strips generics)

**Generation**: `func_id.simple_key()`
```rust
SimpleFunctionKey {
    normalized_name: FunctionId::normalize_name(&self.name),
}
```

**Hash/Eq Implementation**: Only considers normalized name

**Example**:
```rust
// These FunctionIds in different files produce the same SimpleFunctionKey
let id1 = FunctionId::new("src/main.rs", "parse_config", 100);
let id2 = FunctionId::new("src/util.rs", "parse_config", 200);

assert_eq!(id1.simple_key(), id2.simple_key());
```

#### Index Data Structures

##### Primary Index
```rust
nodes: im::HashMap<FunctionId, FunctionNode>
```

- **Key Type**: Complete `FunctionId`
- **Value Type**: `FunctionNode` with metadata (complexity, test status, etc.)
- **Lookup**: `nodes.get(&func_id)` - O(1)
- **Purpose**: Exact match lookups

##### Fuzzy Index
```rust
fuzzy_index: std::collections::HashMap<FuzzyFunctionKey, Vec<FunctionId>>
```

- **Key Type**: `FuzzyFunctionKey` (file + normalized name)
- **Value Type**: `Vec<FunctionId>` - multiple functions with same name in file
- **Lookup**: `fuzzy_index.get(&fuzzy_key)` - O(1) + O(k) disambiguation
- **Purpose**: Handle generic functions and line number drift

**Value is Vec because**:
- Multiple functions with same base name in one file (e.g., overloads in trait impls)
- Disambiguation needed via line proximity or module path

##### Name Index
```rust
name_index: std::collections::HashMap<String, Vec<FunctionId>>
```

- **Key Type**: Normalized function name (String)
- **Value Type**: `Vec<FunctionId>` - all functions with this name across all files
- **Lookup**: `name_index.get(&normalized_name)` - O(1) + O(n) disambiguation
- **Purpose**: Cross-file lookups when file information unavailable

**Value is Vec because**:
- Same function name appears in multiple files
- Disambiguation needed via module path or line proximity

#### Type Safety Benefits

**Compile-Time Guarantees**:
1. **No key confusion**: Cannot accidentally use `FuzzyFunctionKey` with exact match logic
2. **Explicit normalization**: `normalize_name()` clearly shows where normalization occurs
3. **Immutable keys**: All key types are `Clone + Hash + Eq` with no mutation methods

**Example - Type System Prevents Errors**:
```rust
// Compile error: cannot use FunctionId directly as fuzzy key
let bad_key: FuzzyFunctionKey = func_id;  // ❌ Type mismatch

// Must explicitly request fuzzy key
let good_key: FuzzyFunctionKey = func_id.fuzzy_key();  // ✓ Explicit conversion
```

#### Memory Layout Optimization

**Key Size Analysis**:
```
FunctionId:         ~150 bytes (PathBuf + 2 Strings + usize)
ExactFunctionKey:   ~150 bytes (identical layout)
FuzzyFunctionKey:   ~100 bytes (PathBuf + String)
SimpleFunctionKey:  ~50 bytes  (String only)
```

**Index Storage**:
- Primary index: `FunctionId` → `FunctionNode` (~350 bytes per entry)
- Fuzzy index: `FuzzyFunctionKey` → `Vec<FunctionId>` (~100 + 150k bytes)
- Name index: `String` → `Vec<FunctionId>` (~50 + 150n bytes)

**Trade-off**: Larger key types for type safety, but overall memory overhead is acceptable (<10 MB for large codebases).

#### Serialization Format

**Challenge**: Keys are derived from `FunctionId`, so we only need to serialize the primary index.

**Implementation**:
```rust
#[derive(Serialize, Deserialize)]
pub struct CallGraph {
    #[serde(with = "function_id_map")]
    pub nodes: HashMap<FunctionId, FunctionNode>,  // ✓ Serialized

    #[serde(skip)]
    pub fuzzy_index: HashMap<FuzzyFunctionKey, Vec<FunctionId>>,  // ✗ Skipped

    #[serde(skip)]
    pub name_index: HashMap<String, Vec<FunctionId>>,  // ✗ Skipped
}
```

**Rationale**:
- Fuzzy and name indexes are deterministic transforms of the primary index
- Rebuild cost is negligible (~8ms for 1,200 functions)
- JSON size reduced by 40% (only essential data serialized)

**Rebuild Logic**:
```rust
impl CallGraph {
    fn rebuild_indexes(&mut self) {
        for (func_id, _) in &self.nodes {
            // Populate fuzzy index
            let fuzzy_key = func_id.fuzzy_key();
            self.fuzzy_index.entry(fuzzy_key).or_default().push(func_id.clone());

            // Populate name index
            let name = FunctionId::normalize_name(&func_id.name);
            self.name_index.entry(name).or_default().push(func_id.clone());
        }
    }
}
```

#### Testing Strategy

**Property Tests** (using `proptest`):
```rust
proptest! {
    // Generic functions should have equal fuzzy keys
    fn generic_normalization_idempotent(base_name: String) {
        let name1 = format!("{}<T>", base_name);
        let name2 = format!("{}<String>", base_name);
        assert_eq!(
            FunctionId::normalize_name(&name1),
            FunctionId::normalize_name(&name2)
        );
    }

    // Fuzzy keys ignore line differences
    fn fuzzy_key_line_independence(name: String, line1: usize, line2: usize) {
        let id1 = FunctionId::new("test.rs".into(), name.clone(), line1);
        let id2 = FunctionId::new("test.rs".into(), name, line2);
        assert_eq!(id1.fuzzy_key(), id2.fuzzy_key());
    }
}
```

**Unit Tests**: See `src/priority/call_graph/types.rs:225-282` for comprehensive key equality tests.

## Data Flow

```
Input Files
    ↓
[Parallel] Parse AST
    ↓
[Parallel] Extract Metrics
    ↓
[Parallel] Build Call Graph
    ↓
[Parallel] Detect Tests
    ↓
[Parallel] Load & Index Coverage (if --lcov provided)
    ↓
[Parallel] Calculate Debt with Coverage Lookups
    ↓
[Sequential] Aggregate Results
    ↓
[Sequential] Apply Weights
    ↓
Output Report
```

## Configuration

### Performance Tuning Options

#### Command Line Flags
- `--jobs N`: Number of parallel jobs (default: CPU count)
- `--batch-size N`: Items per batch (default: 100)
- `--no-parallel`: Disable parallel processing
- `--progress`: Show progress indicators

#### Environment Variables
- `RAYON_NUM_THREADS`: Override thread pool size
- `DEBTMAP_BATCH_SIZE`: Default batch size
- `DEBTMAP_CACHE_DIR`: Cache location for incremental analysis

### Adaptive Behavior
The system automatically adjusts based on:
- Available CPU cores
- System memory
- Codebase size
- File complexity distribution

## Extension Points

### Adding Language Support
1. Implement the `FileAnalyzer` trait
2. Add parser integration (tree-sitter, syn, etc.)
3. Map language constructs to unified metrics
4. Register analyzer in the factory

### Custom Metrics
1. Extend `FunctionMetrics` or `FileMetrics`
2. Add calculation in analyzer implementation
3. Update aggregation logic
4. Modify weight configuration

### Analysis Plugins
1. Implement analysis phase interface
2. Register in unified analysis pipeline
3. Ensure thread-safety for parallel execution
4. Add configuration options

## Testing Strategy

### Unit Tests
- Individual component testing
- Mock dependencies for isolation
- Property-based testing for algorithms

### Integration Tests
- End-to-end analysis validation
- Performance regression tests
- Parallel vs sequential consistency checks

### Benchmarks
- Micro-benchmarks for critical paths
- Macro-benchmarks on real codebases
- Performance comparison suite

## Future Enhancements

### Planned Optimizations
- Incremental analysis with file watching
- Distributed analysis across machines
- GPU acceleration for graph algorithms
- Advanced caching strategies

### Scalability Improvements
- Streaming parser for huge files
- Database backend for enterprise scale
- Cloud-native deployment options
- Real-time analysis integration

## Module Dependency Graph and Dependency Injection

### Module Structure
The codebase follows a layered architecture with dependency injection for reduced coupling:

```mermaid
graph TD
    %% Core Layer
    subgraph "Core Layer"
        core_types[core::types]
        core_traits[core::traits]
        core_cache[core::cache]
        core_injection[core::injection]
        core_adapters[core::adapters]
    end

    %% Analyzer Layer
    subgraph "Analyzer Layer"
        analyzers[analyzers]
        rust_analyzer[analyzers::rust]
        python_analyzer[analyzers::python]
        js_analyzer[analyzers::javascript]
        implementations[analyzers::implementations]
    end

    %% Dependencies
    core_adapters --> core_traits
    core_adapters --> core_cache
    core_injection --> core_traits

    implementations --> rust_analyzer
    implementations --> python_analyzer
    implementations --> js_analyzer
```

### Dependency Injection Architecture

#### Container Pattern
The `AppContainer` in `core::injection` provides centralized dependency management:
- All analyzers created through factories
- Dependencies injected at construction
- Trait boundaries for loose coupling

#### Factory Pattern
`AnalyzerFactory` creates language-specific analyzers:
- `create_rust_analyzer()` - Returns boxed trait object
- `create_python_analyzer()` - Returns boxed trait object
- `create_javascript_analyzer()` - Returns boxed trait object
- `create_typescript_analyzer()` - Returns boxed trait object

#### Adapter Pattern
`CacheAdapter` wraps the concrete `AnalysisCache`:
- Implements generic `Cache` trait
- Provides abstraction boundary
- Enables testing with mock caches

### Module Coupling Improvements
After implementing dependency injection:
- **Direct dependencies reduced by ~40%** through trait boundaries
- **Circular dependencies eliminated** via proper layering
- **Interface segregation** - modules depend only on required traits
- **Dependency inversion** - high-level modules independent of low-level details

## Scoring Architecture

### Unified Scoring Model

DebtMap uses a sophisticated scoring system to prioritize technical debt items based on multiple factors:

#### Base Score Calculation

The base score uses a **weighted sum model** that combines three primary factors:

- **Coverage Factor (40% weight)**: Measures test coverage gaps
- **Complexity Factor (40% weight)**: Assesses code complexity
- **Dependency Factor (20% weight)**: Evaluates impact based on call graph position

**Formula**:
```
base_score = (coverage_score × 0.4) + (complexity_score × 0.4) + (dependency_score × 0.2)
```

#### Two-Stage Role Adjustment Mechanism

DebtMap employs a two-stage role adjustment mechanism to accurately score functions based on their architectural role and testing expectations. This prevents false positives (e.g., entry points flagged for low unit test coverage) while still accounting for role-based importance.

**Stage 1: Role-Based Coverage Weighting**

**Design Decision**: Not all functions need the same level of unit test coverage. Entry points (CLI handlers, HTTP routes, main functions) are typically integration tested rather than unit tested, while pure business logic should have comprehensive unit tests.

**Implementation**: Role-based coverage weights adjust the coverage penalty based on function role:

```rust
// From unified_scorer.rs:236
let adjusted_coverage_pct = 1.0 - ((1.0 - coverage_pct) * coverage_weight_multiplier);
```

**Default Weights** (configurable in `.debtmap.toml` under `[scoring.role_coverage_weights]`):

| Function Role    | Coverage Weight | Rationale                                    |
|------------------|-----------------|----------------------------------------------|
| Entry Point      | 0.6             | Integration tested, orchestrates other code  |
| Orchestrator     | 0.8             | Coordinates logic, partially integration tested |
| Pure Logic       | 1.2             | Should be thoroughly unit tested             |
| I/O Wrapper      | 0.7             | Often tested via integration tests           |
| Pattern Match    | 1.0             | Standard weight                              |
| Unknown          | 1.0             | Default weight                               |

**Example**: An entry point with 0% coverage receives `1.0 - ((1.0 - 0.0) × 0.6) = 0.4` adjusted coverage (40% penalty reduction), while a pure logic function with 0% coverage gets the full penalty.

**Benefits**:
- Prevents entry points from dominating priority lists due to low unit test coverage
- Focuses testing efforts on pure business logic where unit tests provide most value
- Recognizes different testing strategies (unit vs integration) as equally valid

**Stage 2: Role Multiplier**

A role-based multiplier is applied to the final score to reflect function importance and architectural significance:

```rust
// From unified_scorer.rs:261-262
let clamped_role_multiplier = role_multiplier.clamp(clamp_min, clamp_max);
let role_adjusted_score = base_score * clamped_role_multiplier;
```

**Configuration** (`.debtmap.toml` under `[scoring.role_multiplier]`):

```toml
[scoring.role_multiplier]
clamp_min = 0.3           # Minimum multiplier (default: 0.3)
clamp_max = 1.8           # Maximum multiplier (default: 1.8)
enable_clamping = true    # Enable clamping (default: true)
```

**Clamp Range Rationale**:
- **Default [0.3, 1.8]**: Allows significant differentiation without extreme swings
- **Lower bound (0.3)**: Prevents I/O wrappers from becoming invisible (minimum 30% of base score)
- **Upper bound (1.8)**: Prevents critical entry points from overwhelming other issues (maximum 180% of base score)
- **Configurable**: Projects can adjust range based on their priorities

**When to Disable Clamping**:
- **Prototyping**: Testing extreme multiplier values for custom scoring strategies
- **Special cases**: Very specific project needs requiring wide multiplier ranges
- **Not recommended** for production use as it can lead to unstable prioritization

**Key Distinction: Two-Stage Approach**

The separation of coverage weight adjustment and role multiplier ensures they work together without interfering:

1. **Coverage weight** (Stage 1, applied early): Adjusts coverage expectations by role
   - Modifies how much coverage gaps penalize different function types
   - Pure logic gets full coverage penalty (1.2x), entry points get reduced penalty (0.6x)

2. **Role multiplier** (Stage 2, applied late): Small final adjustment for role importance
   - Applied after all other scoring factors are computed
   - Clamped to prevent extreme values (default: [0.3, 1.8])
   - Fine-tunes final priority based on architectural significance

**Example Workflow**:
```
1. Calculate base score from complexity and dependencies
2. Apply coverage weight based on role → adjusted coverage penalty
3. Combine into preliminary score
4. Apply clamped role multiplier → final score
```

This two-stage approach ensures:
- Role-based coverage adjustments don't interfere with the role multiplier
- Both mechanisms contribute independently to the final score
- Clamping prevents extreme multiplier values from distorting priorities
- Configuration flexibility for different project needs

#### Function Role Detection

Function roles are detected automatically through heuristic analysis:

**Entry Point Detection**:
- Name patterns: `main`, `run_*`, `handle_*`, `execute_*`
- Attributes: `#[tokio::main]`, `#[actix_web::main]`, CLI command annotations
- Call graph position: No callers or called only by test harnesses

**Pure Logic Detection**:
- No file I/O operations
- No network calls
- No database access
- Deterministic (no randomness, no system time)
- Returns value without side effects

**Orchestrator Detection**:
- High ratio of function calls to logic statements
- Coordinates multiple sub-operations
- Thin logic wrapper over other functions

**I/O Wrapper Detection**:
- Dominated by I/O operations (file, network, database)
- Thin abstraction over external resources

### Entropy-Based Complexity Adjustment

Debtmap distinguishes between genuinely complex code and pattern-based repetitive code using information theory:

- **Entropy Score**: Measures randomness/diversity in code patterns
- **Pattern Repetition**: Detects repeated structures (e.g., 10 similar match arms)
- **Dampening Factor**: Reduces complexity score for highly repetitive code

This prevents false positives from large but simple pattern-matching code.

## God Object Detection

### Understanding God Object vs God Module Detection

Debtmap distinguishes between two fundamentally different organizational problems that both manifest as large files:

#### GOD OBJECT: A Struct/Class with Too Many Methods

**Definition**: A single struct or class that has accumulated too many methods and too many fields, violating the Single Responsibility Principle.

**Classification Criteria**:
- More than 20 methods on a single struct/class
- More than 5 fields in the struct/class
- Methods operate on shared mutable state (the fields)

**Example (Rust)**:
```rust
// GOD OBJECT detected
pub struct MassiveController {
    // 8 fields
    db_connection: DbPool,
    cache: Cache,
    logger: Logger,
    config: Config,
    session: Session,
    auth: AuthService,
    metrics: Metrics,
    queue: MessageQueue,
}

impl MassiveController {
    // 50 methods operating on the fields above
    pub fn handle_user_login(&mut self, ...) { ... }
    pub fn validate_session(&self, ...) { ... }
    pub fn update_cache(&mut self, ...) { ... }
    pub fn send_notification(&self, ...) { ... }
    // ... 46 more methods
}
```

**Why It's Problematic**:
- Violates Single Responsibility Principle (one class doing too much)
- Methods share mutable state (fields), creating tight coupling
- Hard to test in isolation (need to mock all dependencies)
- Changes to one responsibility affect the entire class
- Difficult to refactor without breaking many dependents

**Recommended Fix**:
- Extract logical groups of methods into separate structs
- Move related fields to the new structs
- Use composition instead of putting everything in one class
- Apply the Single Responsibility Principle

**Example Refactoring**:
```rust
// Split into focused components
pub struct AuthController {
    auth: AuthService,
    session: Session,
}

pub struct CacheController {
    cache: Cache,
    db_connection: DbPool,
}

pub struct NotificationController {
    queue: MessageQueue,
    logger: Logger,
}
```

#### GOD MODULE: A File with Too Many Diverse Functions

**Definition**: A module (file) containing many top-level functions that don't share state but represent diverse, unrelated responsibilities.

**Classification Criteria**:
- More than 20 module-level functions
- Does NOT meet GOD OBJECT criteria (no single struct with >20 methods AND >5 fields)
- Functions serve diverse purposes (not cohesive)

**Example (Rust)**:
```rust
// GOD MODULE detected: utils.rs
// 50 diverse module-level functions, no dominant struct

pub fn parse_json(input: &str) -> Result<Value> { ... }
pub fn validate_email(email: &str) -> bool { ... }
pub fn format_currency(amount: f64) -> String { ... }
pub fn hash_password(password: &str) -> String { ... }
pub fn send_http_request(url: &str) -> Result<Response> { ... }
pub fn compress_data(data: &[u8]) -> Vec<u8> { ... }
// ... 44 more unrelated utility functions
```

**Why It's Problematic**:
- Lacks cohesion (functions serve unrelated purposes)
- Hard to navigate and understand module purpose
- Violates module-level Single Responsibility Principle
- Encourages "dumping ground" for miscellaneous functions
- Changes to one function may require rebuilding entire module

**Recommended Fix**:
- Group related functions into focused modules
- Create domain-specific utility modules
- Use submodules to organize by feature/domain

**Example Refactoring**:
```rust
// Split into cohesive modules
// src/parsing.rs
pub fn parse_json(input: &str) -> Result<Value> { ... }
pub fn parse_xml(input: &str) -> Result<Document> { ... }

// src/validation.rs
pub fn validate_email(email: &str) -> bool { ... }
pub fn validate_url(url: &str) -> bool { ... }

// src/formatting.rs
pub fn format_currency(amount: f64) -> String { ... }
pub fn format_date(date: DateTime) -> String { ... }

// src/crypto.rs
pub fn hash_password(password: &str) -> String { ... }
pub fn verify_hash(password: &str, hash: &str) -> bool { ... }
```

#### Key Distinction Summary

| Aspect | GOD OBJECT | GOD MODULE |
|--------|-----------|-----------|
| **Structure** | One struct/class with many methods | Many module-level functions |
| **State** | Methods share mutable state (fields) | Functions are independent, no shared state |
| **Threshold** | >20 methods AND >5 fields on one struct | >20 module-level functions, NOT a god object |
| **Detection** | Count methods per struct + field count | Count total functions in file |
| **Problem Type** | Object-oriented design issue | Module organization issue |
| **Fix Strategy** | Extract classes, apply SRP | Split into cohesive modules |

#### How Debtmap Classifies Files

Debtmap uses a priority-based classification algorithm:

1. **Check for GOD OBJECT first**:
   - Find the largest struct/class in the file
   - If it has >20 methods AND >5 fields → classify as **GOD OBJECT**
   - Output shows: "GOD OBJECT: MyStruct (50 methods, 8 fields)"

2. **If not a GOD OBJECT, check for GOD MODULE**:
   - Count total module-level functions (excluding test functions)
   - If >20 functions → classify as **GOD MODULE**
   - Output shows: "GOD MODULE (50 module functions)"

3. **Otherwise**:
   - File is not classified as either pattern

#### Output Examples

**GOD OBJECT Detection**:
```
#3 SCORE: 7.5 [HIGH]
├─ GOD OBJECT: src/controller.rs
├─ TYPE: UserController (52 methods, 8 fields)
├─ ACTION: Extract responsibilities into focused classes
├─ WHY: Single class with too many methods and fields
└─ Methods: handle_user_login, validate_session, update_cache, ... (52 total)
```

**GOD MODULE Detection**:
```
#5 SCORE: 6.8 [HIGH]
├─ GOD MODULE: src/utils.rs
├─ TYPE: Module with 47 diverse functions
├─ ACTION: Split into cohesive submodules by domain
├─ WHY: Module lacks focus, contains unrelated utilities
└─ Module Functions: parse_json, validate_email, format_currency, ... (47 total)
```

#### Implementation Details

**Location**: `src/organization/god_object_detector.rs`

**Classification Logic**:
```rust
// Simplified algorithm
fn classify_file(file: &FileMetrics) -> Classification {
    // Priority 1: Check for god objects
    for struct_info in &file.structs {
        if struct_info.methods.len() > 20 && struct_info.fields.len() > 5 {
            return Classification::GodObject {
                struct_name: struct_info.name,
                method_count: struct_info.methods.len(),
                field_count: struct_info.fields.len(),
            };
        }
    }

    // Priority 2: Check for god module
    let module_functions = file.functions.iter()
        .filter(|f| !f.is_test && !f.is_method)
        .count();

    if module_functions > 20 {
        return Classification::GodModule {
            function_count: module_functions,
        };
    }

    Classification::Normal
}
```

**Verbose Output**:
When running with `--verbose`, debtmap shows the classification decision process:

```
Analyzing: src/processor.rs
  Checking for GOD OBJECT...
    Largest struct: DataProcessor (12 methods, 4 fields) - below threshold
  Checking for GOD MODULE...
    Module functions: 35 (threshold: 20) - GOD MODULE detected
  Classification: GOD MODULE
```

### Complexity-Weighted Scoring

**Design Problem**: Traditional god object detection relies on raw method counts, which creates false positives for well-refactored code. A file with 100 simple helper functions (complexity 1-3) should not rank higher than a file with 10 highly complex functions (complexity 17+).

**Solution**: DebtMap uses complexity-weighted god object scoring that assigns each function a weight based on its cyclomatic complexity, ensuring that complex functions contribute more to the god object score than simple ones.

#### Weighting Formula

Each function contributes to the god object score based on this formula:

```
weight = (max(1, complexity) / 3)^1.5
```

**Examples**:
- Complexity 1 (simple getter): weight ≈ 0.19
- Complexity 3 (baseline): weight = 1.0
- Complexity 9 (moderate): weight ≈ 5.2
- Complexity 17 (needs refactoring): weight ≈ 13.5
- Complexity 33 (critical): weight ≈ 36.5

**Key Properties**:
- **Non-linear scaling**: Higher complexity functions are weighted disproportionately more
- **Baseline normalization**: Complexity 3 is normalized to weight 1.0 (typical simple function)
- **Power law**: The 1.5 exponent ensures exponential growth for high complexity

#### God Object Score Calculation

The complexity-weighted god object score combines multiple factors:

```rust
weighted_method_count = sum(calculate_complexity_weight(fn.complexity) for fn in functions)
complexity_penalty = if avg_complexity > 10.0 { 1.5 } else if avg_complexity < 3.0 { 0.7 } else { 1.0 }

god_object_score = (
    (weighted_method_count / thresholds.weighted_methods_high) * 40.0 +
    (fields / thresholds.max_fields) * 20.0 +
    (responsibilities / thresholds.max_responsibilities) * 15.0 +
    (lines_of_code / 500) * 25.0
) * complexity_penalty
```

**Threshold**: A file is considered a god object if `god_object_score >= 70.0`

**Benefits**:
- Files with many simple functions score lower than files with fewer complex functions
- Reduces false positives on utility modules with many small helpers
- Focuses refactoring efforts on truly problematic large, complex modules
- Aligns with actual maintainability concerns (complexity matters more than count)

#### Comparison: Raw vs Weighted

**Example**: Comparing two files

| File | Method Count | Avg Complexity | Raw Approach | Weighted Approach |
|------|--------------|----------------|--------------|-------------------|
| shared_cache.rs | 100 | 1.5 | God object (100 methods) | Normal (weighted: 19.0) |
| legacy_parser.rs | 10 | 17.0 | Borderline (10 methods) | God object (weighted: 135.0) |

The weighted approach correctly identifies `legacy_parser.rs` as the real problem despite having fewer methods.

#### Implementation Details

**Location**: `src/organization/complexity_weighting.rs`

**Key Functions**:
- `calculate_complexity_weight(complexity: u32) -> f64`: Pure function to calculate weight for a single function
- `aggregate_weighted_complexity(functions: &[FunctionComplexityInfo]) -> f64`: Sum weights across all non-test functions
- `calculate_avg_complexity(functions: &[FunctionComplexityInfo]) -> f64`: Calculate average complexity for penalty calculation
- `calculate_complexity_penalty(avg_complexity: f64) -> f64`: Apply bonus/penalty based on average complexity

**Integration**: The god object detector in `src/organization/god_object_detector.rs` automatically uses complexity-weighted scoring when cyclomatic complexity data is available, falling back to raw count scoring otherwise.

**Testing**: Comprehensive unit tests validate the weighting formula and ensure that files with many simple functions score significantly lower than files with fewer complex functions.

### Purity-Weighted God Object Scoring

**Design Problem**: Traditional complexity-weighted scoring treats all functions equally regardless of their design quality. A module with 100 pure, composable helper functions (functional programming style) should not be penalized as heavily as a module with 100 stateful, side-effecting functions (procedural style).

**Solution**: DebtMap extends complexity-weighted scoring with purity analysis, applying differential weights to pure vs impure functions. This rewards functional programming patterns while still identifying truly problematic god objects.

#### Purity Analysis Architecture

**Location**: `src/organization/purity_analyzer.rs`

**Analysis Pipeline**:
```
Function AST
    ↓
Analyze Signature (parameters, return type)
    ↓
Analyze Body (side effects, mutations, I/O)
    ↓
Determine Purity Classification
    ↓
Apply Purity Weight to Complexity Score
```

**Classification Algorithm**:

The purity analyzer examines both function signatures and implementations:

1. **Signature Analysis**:
   - Mutable parameters (`&mut`) → Impure
   - No return value → Likely impure (unless proven otherwise)
   - Return type suggests computation → Potentially pure

2. **Body Analysis** (detects side effects):
   - File I/O operations (`std::fs`, `tokio::fs`)
   - Network calls (`reqwest`, `hyper`, sockets)
   - Database access (SQL, ORM operations)
   - Global state mutation (static mut, unsafe)
   - Logging/printing (`println!`, `log::`)
   - System calls (`std::process`, `Command`)
   - Random number generation
   - Time/clock access

3. **Purity Determination**:
   - **Pure**: No detected side effects, immutable parameters, returns value
   - **Impure**: Any side effect detected or mutable state access

#### Purity Weights

Pure functions receive a reduced weight multiplier:

```rust
// From src/organization/purity_analyzer.rs
const PURE_FUNCTION_WEIGHT: f64 = 0.3;    // 30% weight
const IMPURE_FUNCTION_WEIGHT: f64 = 1.0;  // 100% weight (baseline)
```

**Rationale**:
- **Pure functions** are easier to test, reason about, and maintain
- **Many small pure helpers** indicate good functional decomposition
- **Impure functions** carry inherent complexity beyond their cyclomatic complexity

#### Integration with God Object Detection

The god object detector applies purity weights during weighted complexity calculation:

```rust
// Pseudo-code from god_object_detector.rs
for function in functions {
    complexity_weight = calculate_complexity_weight(function.complexity);
    purity_weight = if is_pure(function) { 0.3 } else { 1.0 };
    total_weighted_complexity += complexity_weight * purity_weight;
}
```

**Combined Weighting**:
- Simple pure function (complexity 1): `0.19 × 0.3 = 0.057`
- Simple impure function (complexity 1): `0.19 × 1.0 = 0.19`
- Complex pure function (complexity 17): `13.5 × 0.3 = 4.05`
- Complex impure function (complexity 17): `13.5 × 1.0 = 13.5`

#### Example Scenario

**Functional Module** (70 pure helpers, 30 impure orchestrators):
```
Pure functions:    70 × avg_weight(2.0) × 0.3 = 42.0
Impure functions:  30 × avg_weight(8.0) × 1.0 = 240.0
Total weighted: 282.0
God object score: ~45.0 (below threshold)
```

**Procedural Module** (100 impure functions):
```
Impure functions:  100 × avg_weight(8.0) × 1.0 = 800.0
Total weighted: 800.0
God object score: ~125.0 (god object detected)
```

The functional module avoids god object classification despite having more total functions, because its pure helpers contribute minimally to the weighted score.

#### Benefits

- **Rewards functional programming**: Modules using functional patterns score lower
- **Penalizes stateful design**: Modules with many side effects score higher
- **Accurate problem detection**: Focuses on truly problematic modules, not well-refactored functional code
- **Encourages refactoring**: Incentivizes extracting pure functions from complex impure ones

#### Verbose Output

When running with `--verbose`, the god object analysis includes purity distribution:

```
GOD OBJECT ANALYSIS: src/core/processor.rs
  Total functions: 107
  PURITY DISTRIBUTION:
    Pure: 70 functions (65%) → complexity weight: 6.3
    Impure: 37 functions (35%) → complexity weight: 14.0
    Total weighted complexity: 20.3
  God object score: 12.0 (threshold: 70.0)
  Status: ✓ Not a god object (functional design)
```

#### Data Flow

The purity analysis integrates into the existing analysis pipeline:

```
File Analysis
    ↓
Extract Functions
    ↓
Calculate Cyclomatic Complexity (existing)
    ↓
[NEW] Perform Purity Analysis
    ↓
[NEW] Apply Purity Weights
    ↓
Calculate Weighted Complexity
    ↓
God Object Detection
    ↓
Generate Report
```

#### Testing

**Unit Tests** (`src/organization/purity_analyzer.rs`):
- Pure function detection accuracy
- Impure function detection (all side effect types)
- Edge cases (empty functions, trait implementations)

**Integration Tests** (`tests/purity_weighted_god_object.rs`):
- Functional modules score lower than procedural modules
- Purity distribution appears in verbose output
- God object threshold calibration with purity weights

**Property Tests**:
- Purity classification is deterministic
- Pure function weight < Impure function weight (always)
- Total weighted complexity >= raw complexity count

## Observer Pattern Detection

### Overview

DebtMap includes sophisticated observer pattern detection that identifies event-driven dispatch patterns across the call graph, reducing false positives in dead code detection for event handlers and callbacks.

### Architecture Components

#### Pattern Recognition
- **Observer Registry Detection**: Identifies registration functions that store callbacks/handlers
- **Observer Dispatch Detection**: Detects loops that notify registered observers
- **Call Graph Integration**: Marks detected patterns in the unified call graph

#### Data Flow

```
File Analysis
    ↓
Extract Functions & Classes
    ↓
[Pattern Recognition]
Identify Observer Registration Patterns
    ↓
[Observer Registry]
Build Registry of Observer Collections
    ↓
[Observer Dispatch Detector]
Detect Dispatch Loops
    ↓
[Call Graph Integration]
Mark Functions as Dispatchers
    ↓
Enhanced Call Graph Analysis
```

### Detection Algorithm

#### Phase 1: Observer Registry Detection

Identifies collections that store callbacks:

**Detection Criteria**:
- Collection fields storing function pointers, closures, or trait objects
- Field names matching observer patterns: `listeners`, `handlers`, `observers`, `callbacks`, `subscribers`
- Type patterns: `Vec<Box<dyn Trait>>`, `Vec<Fn(...)>`, `HashMap<K, Vec<Handler>>`

**Example Detected Patterns**:
```rust
// Simple vector of handlers
pub struct EventBus {
    listeners: Vec<Box<dyn EventHandler>>,  // ← Detected
}

// HashMap of event types to handlers
pub struct Dispatcher {
    handlers: HashMap<EventType, Vec<Callback>>,  // ← Detected
}

// Closure storage
pub struct Notifier {
    callbacks: Vec<Box<dyn Fn(&Event)>>,  // ← Detected
}
```

#### Phase 2: Observer Dispatch Detection

Identifies loops that invoke stored callbacks:

**Detection Criteria**:
1. **Loop Pattern**: Function contains `for` loop iterating over observer collection
2. **Collection Reference**: Loop iterates over field from observer registry
3. **Dispatch Call**: Loop body contains method call or function invocation on iterator element
4. **No Early Exit**: Loop completes all iterations (no `break` statements)

**Example Detected Patterns**:
```rust
// Standard observer loop
fn notify(&self, event: &Event) {
    for listener in &self.listeners {  // ← Loop over registry
        listener.handle(event);        // ← Dispatch call
    }
}

// Inline notification with filter
fn notify_matching(&self, predicate: impl Fn(&Handler) -> bool) {
    for handler in self.handlers.iter().filter(predicate) {
        handler.execute();  // ← Dispatch
    }
}

// HashMap dispatch
fn dispatch(&self, event_type: EventType, data: &Data) {
    if let Some(handlers) = self.handlers.get(&event_type) {
        for handler in handlers {  // ← Nested loop detected
            handler.call(data);    // ← Dispatch call
        }
    }
}
```

#### Phase 3: Call Graph Enhancement

Detected observer dispatch functions are marked in the call graph:

```rust
pub struct CallGraphNode {
    // ... existing fields
    pub is_observer_dispatcher: bool,  // ← Enhanced metadata
}
```

**Integration Points**:
- **Dead Code Detection**: Accounts for dynamic dispatch through observer patterns
- **Complexity Analysis**: Recognizes observer loops as coordination logic (lower complexity penalty)
- **Risk Assessment**: Factors in dynamic call graph expansion from observers

### Class Hierarchy Support

The detection system handles inheritance and trait implementations:

**Scenario**: Observer registry in base class, dispatch in derived class
```rust
struct Base {
    listeners: Vec<Box<dyn Listener>>,  // ← Registry in base
}

struct Derived {
    base: Base,  // ← Inherited field
}

impl Derived {
    fn notify(&self) {
        for listener in &self.base.listeners {  // ← Detected via field access
            listener.on_event();
        }
    }
}
```

**Detection Strategy**:
- Track field access chains: `self.base.listeners`
- Match against registry collections even through indirection
- Support nested field patterns: `self.inner.dispatcher.handlers`

### Performance Characteristics

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| Registry Detection | O(f × c) | f = functions, c = avg fields per class |
| Dispatch Detection | O(f × l) | f = functions, l = avg loops per function |
| Call Graph Enhancement | O(n) | n = call graph nodes |
| Overall Impact | <5% overhead | Measured on medium codebases (1000+ functions) |

### Benefits

**False Positive Reduction**:
- Event handlers no longer flagged as dead code
- Callbacks correctly identified as reachable via dispatch
- Dynamic invocation patterns recognized

**Accuracy Improvement**:
- 80% reduction in false positives for event-driven architectures
- 100% retention of true positives (no regression in callback detection)
- Better call graph completeness for observer-based systems

### Integration with Existing Systems

**Unified Analysis Pipeline**:
```
Parse Files
    ↓
Extract Metrics (existing)
    ↓
Build Call Graph (existing)
    ↓
[NEW] Detect Observer Patterns
    ↓
[NEW] Enhance Call Graph with Dispatch Info
    ↓
Dead Code Detection (enhanced)
    ↓
Technical Debt Scoring
```

**Configuration Options**:
```toml
# .debtmap.toml
[observer_detection]
enabled = true
registry_field_patterns = ["listeners", "handlers", "observers", "callbacks"]
min_confidence = 0.8
```

### Testing Strategy

**Unit Tests**:
- Observer registry detection accuracy
- Dispatch loop pattern recognition
- Class hierarchy field access tracking

**Integration Tests**:
- End-to-end observer pattern detection
- Call graph enhancement validation
- False positive reduction measurement

**Regression Tests**:
- Ensure existing callback detection works
- Verify no true positives lost
- Validate performance impact stays <5%

### Limitations and Future Work

**Current Limitations**:
- Requires explicit loops (doesn't detect `map`/`for_each` patterns yet)
- Limited to Rust syntax patterns
- Doesn't track cross-module observer registration

**Planned Enhancements**:
- Functional iterator pattern detection (`for_each`, `map`)
- Multi-language support (Python, TypeScript)
- Inter-module observer tracking via type analysis
- Confidence scoring for ambiguous patterns

## Struct Initialization Pattern Detection

### Overview

DebtMap includes specialized detection for struct initialization/conversion functions where high cyclomatic complexity arises from conditional field assignment rather than complex algorithmic logic. These functions are often incorrectly flagged as overly complex by traditional metrics.

### Problem Statement

Functions that construct structs from configuration or convert between types often exhibit:
- **High cyclomatic complexity** from field-level conditionals (`unwrap_or`, `match` on `Option<T>`)
- **Many simple branches** rather than deep algorithmic complexity
- **Initialization-focused logic** rather than business logic

Traditional cyclomatic complexity metrics penalize these patterns unfairly, treating them as equivalently complex to nested algorithmic logic.

### Detection Strategy

#### Pattern Recognition
The detector identifies functions matching:
- **Field count threshold**: ≥15 fields in struct literal
- **Initialization ratio**: ≥70% of function lines dedicated to field initialization
- **Low nesting depth**: ≤4 levels (characteristic of simple field mapping)
- **Result wrapping**: Returns `Result<StructName, E>` or `StructName` directly

#### Field-Based Complexity Metric

Instead of cyclomatic complexity, we calculate a field-based complexity score:

```rust
field_score = match field_count {
    0..=20 => 1.0,
    21..=40 => 2.0,
    41..=60 => 3.5,
    _ => 5.0,
} + (max_nesting_depth * 0.5) + (complex_fields.len() * 1.0)
```

This provides a more appropriate complexity measure for initialization patterns.

#### Complex Field Detection
Fields requiring >10 lines of initialization logic are flagged as "complex fields" that may benefit from extraction into helper functions.

#### Field Dependency Analysis
The detector tracks which fields reference other local variables/fields to identify:
- **Interdependencies**: Fields that depend on computed values
- **Derived fields**: Fields calculated from other fields
- **Simple mappings**: Direct parameter-to-field assignments

### Confidence Scoring

Confidence is calculated based on multiple factors:
- **Initialization ratio** (0.35 max): Higher ratio = higher confidence
- **Field count** (0.25 max): More fields = more likely initialization
- **Low nesting** (0.20 max): Shallow nesting typical of initialization
- **Struct name patterns** (0.10 max): Names like `Args`, `Config`, `Options`
- **Complex field penalty**: Many complex fields suggest mixed logic

Threshold: Only patterns with ≥60% confidence are reported.

### Recommendations

Based on detected patterns, the detector provides actionable recommendations:

| Field Count | Max Nesting | Complex Fields | Recommendation |
|-------------|-------------|----------------|----------------|
| >50         | any         | any            | Consider builder pattern |
| any         | any         | >5             | Extract complex field initializations |
| any         | >3          | any            | Reduce nesting depth |
| ≤50         | ≤3          | ≤5             | Appropriately complex |

### Integration

The detector is integrated into DebtMap's Rust analyzer as an `OrganizationDetector`, running alongside other anti-pattern detectors (God Object, Feature Envy, etc.).

Output includes:
- Function name and struct being initialized
- Field count and cyclomatic complexity (for comparison)
- Field-based complexity score
- Confidence percentage
- Specific recommendation

### Example Output

```
Struct initialization pattern in 'from_low_args' - 42 fields,
cyclomatic: 38, field complexity: 2.5, confidence: 85%

Recommendation: Initialization is appropriately complex for field count
(Use field-based complexity 2.5 instead of cyclomatic 38)
```

### Limitations

- **Source content dependency**: Requires file content for span analysis
- **Rust-specific**: Currently only implemented for Rust (syn AST)
- **Simple heuristics**: May miss complex initialization patterns

### Testing

**Unit Tests**: Core detection logic, field dependency analysis, confidence scoring
**Integration Tests**: Real-world struct initialization patterns, false positive prevention
**Property Tests**: Planned for invariant verification

## Dependencies

### Core Dependencies
- **rayon**: Parallel execution framework
- **syn**: Rust AST parsing
- **tree-sitter**: Multi-language parsing
- **serde**: Serialization
- **clap**: CLI argument parsing

### Language-Specific
- **tree-sitter-python**: Python support
- **tree-sitter-javascript**: JS/TS support
- **tree-sitter-go**: Go support

### Development Dependencies
- **cargo-modules**: Module dependency analysis and visualization
- **proptest**: Property-based testing
- **criterion**: Benchmarking framework
- **tempfile**: Test file management

## Error Handling

### Resilience Strategy
- Graceful degradation on parser errors
- Partial results on analysis failure
- Detailed error reporting with context
- Recovery mechanisms for parallel failures

### Monitoring
- Performance metrics collection
- Error rate tracking
- Resource usage monitoring
- Analysis quality metrics