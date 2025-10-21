---
number: 120
title: Indirect Coverage Detection
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-10-21
---

# Specification 120: Indirect Coverage Detection

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap v0.2.9 only detects **direct test coverage** from lcov data, missing functions that are well-tested **indirectly** through their callers. This results in false positives for utility functions and helper methods.

**Real-World False Positive**:
```rust
// src/context/rules.rs:52 - ContextMatcher::any()
pub fn any() -> Self {
    Self { /* field initialization */ }
}

// Called by parse_config_rule() at line 221
// which is called by load_config_rules() at line 182
// which is called by new() at line 176
// which HAS tests at line 462
```

**Current Analysis**:
```
#1 SCORE: 21.2 [🔴 UNTESTED] [CRITICAL]
├─ COVERAGE: 0% covered
└─ WHY: 100% coverage gap
```

**What's Missing**:
- Debtmap doesn't check if callers are tested
- No transitive coverage propagation through call graph
- Utility functions appear untested even when exercised by tests

**Impact**:
- ~30% false positive rate for helper functions
- Users waste time writing redundant tests
- Inflated risk scores for well-tested code paths
- Reduced trust in analysis accuracy

**Why This is Critical**:
- Many architectural patterns use helper functions (DRY principle)
- Testing helpers directly often duplicates integration tests
- Users need guidance on *what actually needs new tests*

## Objective

Detect and account for indirect test coverage by analyzing the call graph and propagating coverage from tested callers to their callees.

## Requirements

### Functional Requirements

**FR1: Call Graph Coverage Propagation**
- Traverse call graph from tested functions to callees
- Calculate transitive coverage percentage
- Distinguish between direct and indirect coverage

**FR2: Caller Coverage Analysis**
- Identify all callers of a function
- Determine coverage percentage of each caller
- Weight indirect coverage by caller quality

**FR3: Indirect Coverage Thresholds**
- Functions with ≥80% indirect coverage from well-tested callers = "covered indirectly"
- Reduce severity for indirectly covered functions
- Flag functions needing direct tests vs already covered

**FR4: Coverage Source Attribution**
- Report both direct and indirect coverage separately
- Show which callers provide test coverage
- Explain why function is considered tested/untested

**FR5: Multi-Hop Propagation**
- Handle deep call chains (A → B → C → D)
- Limit propagation depth (max 3-5 hops to prevent false propagation)
- Discount coverage with distance (100% → 80% → 60% over hops)

### Non-Functional Requirements

**NFR1: Performance**
- Call graph traversal adds < 10% overhead to analysis time
- Cache indirect coverage calculations
- Use efficient graph algorithms (BFS/DFS with pruning)

**NFR2: Accuracy**
- < 10% false positive rate for indirect coverage
- Conservative approach (better to miss coverage than falsely claim it)
- Configurable thresholds for different projects

**NFR3: Scalability**
- Support large codebases (>100k functions)
- Handle deeply nested call graphs (>10 levels)
- Memory efficient storage of coverage propagation

## Acceptance Criteria

- [x] Indirect coverage detection implemented using call graph traversal
- [x] `ContextMatcher::any()` recognized as tested through `new()` → `load_config_rules()` → `parse_config_rule()`
- [x] Coverage report shows both direct and indirect coverage percentages
- [x] Functions with ≥80% indirect coverage from well-tested callers flagged as "tested indirectly"
- [x] Risk score reduced for indirectly covered functions (severity drops from CRITICAL to LOW/MODERATE)
- [x] Multi-hop propagation limited to 3-5 hops with distance discount
- [x] Performance overhead < 10% for large codebases
- [x] Test suite validates indirect coverage detection across various call patterns
- [x] Documentation explains indirect coverage concept and thresholds
- [x] Configuration allows customization of propagation depth and thresholds

## Technical Details

### Implementation Approach

**Phase 1: Call Graph Coverage Metadata**

```rust
/// Extended coverage information including indirect coverage
#[derive(Debug, Clone)]
pub struct CompleteCoverage {
    /// Direct coverage from tests (lcov data)
    pub direct_coverage: f64,

    /// Indirect coverage from tested callers
    pub indirect_coverage: f64,

    /// Combined effective coverage
    pub effective_coverage: f64,

    /// Callers contributing to indirect coverage
    pub coverage_sources: Vec<CoverageSource>,
}

#[derive(Debug, Clone)]
pub struct CoverageSource {
    /// Function providing coverage
    pub caller: FunctionId,

    /// Coverage percentage of caller
    pub caller_coverage: f64,

    /// Number of hops from tested code
    pub distance: u32,

    /// Discounted coverage contribution
    pub contributed_coverage: f64,
}
```

**Phase 2: Indirect Coverage Calculation**

**File**: `src/risk/evidence/coverage_analyzer.rs`

```rust
impl CoverageRiskAnalyzer {
    /// Calculate indirect coverage from tested callers
    fn calculate_indirect_coverage(
        &self,
        func_id: &FunctionId,
        call_graph: &CallGraph,
        coverage_data: Option<&LcovData>,
    ) -> CompleteCoverage {
        let direct_coverage = self.get_coverage_percentage(func, coverage_data);

        // If already well-tested directly, skip indirect calculation
        if direct_coverage >= 80.0 {
            return CompleteCoverage {
                direct_coverage,
                indirect_coverage: 0.0,
                effective_coverage: direct_coverage,
                coverage_sources: vec![],
            };
        }

        // Find all callers
        let callers = call_graph.get_callers(func_id);
        if callers.is_empty() {
            return CompleteCoverage::direct_only(direct_coverage);
        }

        // Calculate coverage from each caller
        let sources = self.analyze_caller_coverage(
            callers,
            call_graph,
            coverage_data,
            0, // Starting depth
        );

        let indirect_coverage = self.aggregate_indirect_coverage(&sources);
        let effective_coverage = self.combine_coverages(direct_coverage, indirect_coverage);

        CompleteCoverage {
            direct_coverage,
            indirect_coverage,
            effective_coverage,
            coverage_sources: sources,
        }
    }

    /// Analyze coverage contribution from callers (recursive with depth limit)
    fn analyze_caller_coverage(
        &self,
        callers: &[FunctionId],
        call_graph: &CallGraph,
        coverage_data: Option<&LcovData>,
        depth: u32,
    ) -> Vec<CoverageSource> {
        const MAX_DEPTH: u32 = 3;
        const DISTANCE_DISCOUNT: f64 = 0.7; // 70% per hop

        if depth >= MAX_DEPTH {
            return vec![];
        }

        let mut sources = vec![];

        for caller in callers {
            // Get caller's direct coverage
            let caller_coverage = coverage_data
                .and_then(|data| {
                    data.get_function_coverage(&caller.file, &caller.name, caller.line)
                })
                .unwrap_or(0.0);

            // Well-tested caller (≥80%) contributes to indirect coverage
            if caller_coverage >= 80.0 {
                let discount = DISTANCE_DISCOUNT.powi(depth as i32);
                sources.push(CoverageSource {
                    caller: caller.clone(),
                    caller_coverage,
                    distance: depth,
                    contributed_coverage: caller_coverage * discount,
                });
            } else if depth < MAX_DEPTH - 1 {
                // Recursively check caller's callers
                let upstream_callers = call_graph.get_callers(caller);
                sources.extend(self.analyze_caller_coverage(
                    &upstream_callers,
                    call_graph,
                    coverage_data,
                    depth + 1,
                ));
            }
        }

        sources
    }

    /// Aggregate indirect coverage from multiple sources
    fn aggregate_indirect_coverage(&self, sources: &[CoverageSource]) -> f64 {
        if sources.is_empty() {
            return 0.0;
        }

        // Take maximum contribution (not sum, to avoid double-counting)
        sources
            .iter()
            .map(|s| s.contributed_coverage)
            .fold(0.0, f64::max)
    }

    /// Combine direct and indirect coverage
    fn combine_coverages(&self, direct: f64, indirect: f64) -> f64 {
        // Take maximum (indirect doesn't add to direct, it fills the gap)
        direct.max(indirect)
    }
}
```

**Phase 3: Integration with Risk Analysis**

```rust
impl CoverageRiskAnalyzer {
    pub fn analyze(
        &self,
        function: &FunctionAnalysis,
        context: &RiskContext,
        coverage_data: Option<&LcovData>,
        call_graph: &CallGraph, // NEW PARAMETER
        func_id: &FunctionId,
    ) -> RiskFactor {
        // Test functions don't need coverage
        if function.is_test {
            return self.create_test_function_factor(function);
        }

        // Calculate complete coverage (direct + indirect)
        let complete_coverage = self.calculate_indirect_coverage(
            func_id,
            call_graph,
            coverage_data,
        );

        let coverage_percentage = complete_coverage.effective_coverage;

        // If well-covered indirectly, reduce risk
        let critical_paths_uncovered = if complete_coverage.indirect_coverage >= 80.0 {
            // Well-tested through callers - low risk
            0
        } else {
            self.count_uncovered_critical_paths(
                function,
                coverage_percentage,
                context.role,
            )
        };

        // ... rest of existing analysis logic ...
    }
}
```

**Phase 4: Output Formatting**

```rust
/// Format coverage information for user display
fn format_coverage_report(coverage: &CompleteCoverage) -> String {
    if coverage.direct_coverage >= 100.0 {
        return "100% direct coverage".to_string();
    }

    if coverage.indirect_coverage >= 80.0 {
        format!(
            "{}% direct, {}% indirect (tested through {} caller{})",
            coverage.direct_coverage as u32,
            coverage.indirect_coverage as u32,
            coverage.coverage_sources.len(),
            if coverage.coverage_sources.len() == 1 { "" } else { "s" }
        )
    } else if coverage.indirect_coverage > 0.0 {
        format!(
            "{}% direct, {}% indirect (partial caller coverage)",
            coverage.direct_coverage as u32,
            coverage.indirect_coverage as u32
        )
    } else {
        format!("{}% direct coverage", coverage.direct_coverage as u32)
    }
}

/// Show coverage sources for debugging
fn format_coverage_sources(sources: &[CoverageSource]) -> String {
    sources
        .iter()
        .take(3) // Show top 3
        .map(|s| {
            format!(
                "  - {} ({}% coverage, {} hop{})",
                s.caller.name,
                s.caller_coverage as u32,
                s.distance,
                if s.distance == 1 { "" } else { "s" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
```

### Architecture Changes

**Modified Files**:
- `src/risk/evidence/coverage_analyzer.rs` - Add indirect coverage calculation
- `src/priority/unified_scorer.rs` - Pass call graph to coverage analyzer
- `src/priority/formatter.rs` - Display coverage sources
- `src/priority/formatter_verbosity.rs` - Show detailed coverage breakdown

**New Files**:
- `src/risk/coverage/indirect.rs` - Indirect coverage calculation logic
- `src/risk/coverage/propagation.rs` - Coverage propagation algorithms

**Call Graph Enhancements**:
```rust
impl CallGraph {
    /// Get all direct callers of a function
    pub fn get_callers(&self, func_id: &FunctionId) -> Vec<FunctionId> {
        // Existing implementation
    }

    /// Get all callers up to N hops away
    pub fn get_transitive_callers(
        &self,
        func_id: &FunctionId,
        max_depth: u32,
    ) -> HashMap<FunctionId, u32> {
        let mut callers = HashMap::new();
        let mut queue = VecDeque::new();
        queue.push_back((func_id.clone(), 0));

        while let Some((current, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            for caller in self.get_callers(&current) {
                if !callers.contains_key(&caller) {
                    callers.insert(caller.clone(), depth + 1);
                    queue.push_back((caller, depth + 1));
                }
            }
        }

        callers
    }
}
```

### Data Structures

**Coverage Data Model**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCoverageInfo {
    /// Function identifier
    pub function_id: FunctionId,

    /// Direct test coverage (from lcov)
    pub direct_coverage: CoverageStats,

    /// Indirect coverage (from callers)
    pub indirect_coverage: Option<IndirectCoverageStats>,

    /// Effective combined coverage
    pub effective_coverage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageStats {
    pub lines_covered: u32,
    pub lines_total: u32,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndirectCoverageStats {
    /// Maximum indirect coverage from any caller chain
    pub max_indirect_coverage: f64,

    /// All coverage sources (callers)
    pub sources: Vec<CoverageSource>,

    /// Distance to nearest well-tested caller
    pub min_distance: u32,
}
```

### Algorithms

**Coverage Propagation Algorithm**:

1. **Start**: Function F with 0% direct coverage
2. **Find Callers**: Get all direct callers of F
3. **Check Coverage**: For each caller C:
   - If C has ≥80% direct coverage → F gets 100% × 0.7^distance indirect coverage
   - If C has <80% coverage → Recurse to C's callers (up to depth 3)
4. **Aggregate**: Take maximum coverage across all paths
5. **Combine**: Effective coverage = max(direct, indirect)

**Example**:
```
F (0% direct)
├─ C1 (95% coverage, distance=1) → F gets 95% × 0.7 = 66.5% from C1
├─ C2 (40% coverage)
│  └─ C2.1 (90% coverage, distance=2) → F gets 90% × 0.7^2 = 44.1% from C2.1
└─ C3 (untested, no callers) → 0%

Indirect coverage for F = max(66.5%, 44.1%, 0%) = 66.5%
Effective coverage for F = max(0%, 66.5%) = 66.5%
```

## Dependencies

**Prerequisites**:
- Existing call graph implementation
- Lcov coverage data parsing
- Function analysis infrastructure

**Affected Components**:
- Coverage analysis pipeline
- Risk scoring system
- Output formatters
- Call graph traversal

**External Dependencies**: None (uses existing call graph)

## Testing Strategy

### Unit Tests

**Test Coverage Propagation**:
```rust
#[test]
fn test_indirect_coverage_single_hop() {
    let mut call_graph = CallGraph::new();

    // Setup: F called by C (C has 90% coverage)
    let func_f = FunctionId::new("test.rs", "f", 10);
    let caller_c = FunctionId::new("test.rs", "c", 50);

    call_graph.add_function(func_f.clone(), false, false, 5, 20);
    call_graph.add_function(caller_c.clone(), false, false, 8, 40);
    call_graph.add_call(FunctionCall {
        caller: caller_c.clone(),
        callee: func_f.clone(),
        call_type: CallType::Direct,
    });

    // Mock coverage data: C has 90% coverage
    let mut coverage_data = MockLcovData::new();
    coverage_data.set_coverage(&caller_c, 0.9);

    let analyzer = CoverageRiskAnalyzer::new();
    let complete_coverage = analyzer.calculate_indirect_coverage(
        &func_f,
        &call_graph,
        Some(&coverage_data),
    );

    // F should have ~63% indirect coverage (90% × 0.7)
    assert!((complete_coverage.indirect_coverage - 63.0).abs() < 1.0);
    assert_eq!(complete_coverage.coverage_sources.len(), 1);
    assert_eq!(complete_coverage.coverage_sources[0].distance, 0);
}

#[test]
fn test_indirect_coverage_multi_hop() {
    // Setup: F ← C1 ← C2 (C2 has 95% coverage)
    // Expected: F gets 95% × 0.7^2 = 46.55% indirect

    // ... test implementation ...
}

#[test]
fn test_depth_limit_prevents_infinite_recursion() {
    // Setup: Circular call graph A ← B ← C ← A
    // Should not infinite loop, should respect MAX_DEPTH

    // ... test implementation ...
}
```

### Integration Tests

**Regression Test for ContextMatcher::any()**:
```rust
#[test]
fn test_context_matcher_any_indirect_coverage() {
    let analysis = analyze_codebase_with_coverage("src/", "coverage.info");

    let any_func = analysis.find_function("src/context/rules.rs", "any", 52);
    assert!(any_func.is_some());

    let coverage = any_func.unwrap().coverage_info;

    // Should detect indirect coverage through new() → load_config_rules() → parse_config_rule()
    assert!(
        coverage.indirect_coverage >= 70.0,
        "Should detect indirect coverage through tested callers"
    );

    // Risk score should be reduced
    let score = any_func.unwrap().risk_score;
    assert!(
        score < 15.0,
        "Indirectly covered function should have lower risk, got {}",
        score
    );
}
```

**Performance Test**:
```rust
#[test]
fn test_indirect_coverage_performance() {
    let large_codebase = load_large_codebase(); // ~100k functions

    let start = Instant::now();
    let analysis = analyze_with_indirect_coverage(&large_codebase);
    let duration = start.elapsed();

    // Should add < 10% overhead
    let baseline_duration = benchmark_without_indirect_coverage(&large_codebase);
    assert!(
        duration < baseline_duration * 1.1,
        "Indirect coverage adds too much overhead: {:?} vs {:?}",
        duration,
        baseline_duration
    );
}
```

## Documentation Requirements

### Code Documentation

**Module Documentation**:
```rust
//! Indirect coverage detection
//!
//! This module implements coverage propagation through the call graph,
//! detecting functions that are well-tested indirectly through their callers.
//!
//! # Algorithm
//!
//! 1. Start from function F with direct coverage D%
//! 2. Find all callers C₁, C₂, ..., Cₙ
//! 3. For each caller Cᵢ with coverage ≥80%:
//!    - Contribute Cᵢ_coverage × 0.7^distance to F's indirect coverage
//! 4. Recursively check callers' callers up to depth 3
//! 5. Aggregate: indirect_coverage = max(all contributions)
//! 6. Effective coverage = max(direct, indirect)
//!
//! # Distance Discount
//!
//! Coverage contribution decreases with distance:
//! - Distance 0 (direct caller): 100% × 0.7⁰ = 100%
//! - Distance 1 (caller's caller): 100% × 0.7¹ = 70%
//! - Distance 2: 100% × 0.7² = 49%
//! - Distance 3: 100% × 0.7³ = 34%
//!
//! # Example
//!
//! ```
//! // ContextMatcher::any() (0% direct)
//! //   ├─ parse_config_rule() (0% direct)
//! //   │  └─ load_config_rules() (0% direct)
//! //   │     └─ new() (100% direct from tests)
//! //
//! // Distance from any() to new() = 3
//! // Indirect coverage = 100% × 0.7³ = 34%
//! ```
```

### User Documentation

**Update**: `book/src/coverage-analysis.md`

```markdown
## Indirect Coverage Detection

Debtmap detects not only **direct test coverage** (tests explicitly calling your function)
but also **indirect coverage** (tests calling your function through other functions).

### How It Works

When a function F has no direct tests but is called by well-tested functions,
debtmap recognizes F is still being exercised by tests:

```rust
// Helper function - no direct tests
fn parse_port(s: &str) -> u16 {
    s.parse().unwrap_or(8080)
}

// Public API - well-tested (95% coverage)
pub fn create_server(config: &str) -> Server {
    let port = parse_port(config);  // ← parse_port tested here!
    Server::new(port)
}
```

**Debtmap Analysis**:
```
parse_port()
├─ Direct coverage: 0%
├─ Indirect coverage: 66.5% (from create_server @ 95% × 0.7)
└─ Effective coverage: 66.5% [MODERATE]
```

### Distance Discount

Coverage contribution decreases with call distance:

| Distance | Discount | Example |
|----------|----------|---------|
| 0 (direct caller) | 100% | Helper called by tested function |
| 1 (caller's caller) | 70% | Utility called by helper called by tested function |
| 2 | 49% | Deep utility functions |
| 3 (max) | 34% | Very indirect coverage |

### When to Add Direct Tests

Debtmap recommends adding direct tests when:

1. **Low indirect coverage** (<50%) - Not sufficiently exercised by callers
2. **Complex logic** (cyclomatic >10) - Needs targeted edge case testing
3. **Public API** - Direct tests improve API contract clarity

Functions with ≥80% indirect coverage are considered well-tested
and prioritized lower for new test coverage.

### Configuration

```toml
[coverage.indirect]
enabled = true
max_depth = 3           # Maximum call chain depth
distance_discount = 0.7 # Coverage reduction per hop
min_caller_coverage = 80.0  # Minimum coverage to contribute
```
```

## Implementation Notes

### Performance Optimization

**Caching Strategy**:
```rust
pub struct IndirectCoverageCache {
    cache: HashMap<FunctionId, CompleteCoverage>,
}

impl IndirectCoverageCache {
    pub fn get_or_calculate(
        &mut self,
        func_id: &FunctionId,
        calculator: impl FnOnce() -> CompleteCoverage,
    ) -> &CompleteCoverage {
        self.cache.entry(func_id.clone()).or_insert_with(calculator)
    }
}
```

**Graph Traversal Optimization**:
- Use BFS instead of DFS (better cache locality)
- Early termination when max coverage found
- Prune branches with <80% coverage

### Edge Cases

**Circular Dependencies**:
```rust
// A ← B ← C ← A (circular)
// Solution: Track visited nodes, respect depth limit
```

**High Fan-In Functions**:
```rust
// F called by 100+ functions
// Solution: Sample top N callers by coverage, limit traversal
```

**Orphaned Functions**:
```rust
// F has no callers (dead code or entry point)
// Solution: Return 0% indirect coverage
```

## Migration and Compatibility

### Breaking Changes

None - This is a pure enhancement.

### Configuration Migration

**New config section** (`config.toml`):
```toml
[coverage.indirect]
enabled = true
max_depth = 3
distance_discount = 0.7
min_caller_coverage = 80.0
```

### Rollback Plan

Disable via config:
```toml
[coverage.indirect]
enabled = false
```

## Success Metrics

### Quantitative Metrics

- **False Positive Reduction**: 30% reduction for helper functions
- **Coverage Accuracy**: ±10% error vs manual inspection
- **Performance**: <10% overhead on large codebases
- **Propagation Depth**: 95% of indirect coverage detected within 3 hops

### Qualitative Metrics

- **User Trust**: Fewer complaints about "obviously tested" functions marked UNTESTED
- **Test Efficiency**: Users focus on genuinely untested code
- **Clarity**: Users understand indirect vs direct coverage

### Validation

**Before Implementation**:
```
ContextMatcher::any() - SCORE: 21.2 [CRITICAL] [0% coverage]
(Actually well-tested through new())
```

**After Implementation**:
```
ContextMatcher::any() - SCORE: 8.5 [LOW] [66% indirect coverage from new()]
Coverage sources: new() (100%, 3 hops)
```

## Future Enhancements

### Phase 2: Weighted Coverage
Weight indirect coverage by how often caller is executed:
- Frequently called callers contribute more
- Rarely called callers contribute less

### Phase 3: Path-Specific Coverage
Track which execution paths through F are covered:
- F might be 80% covered through C1 (path 1)
- But path 2 never executed by any caller

### Phase 4: Test Suggestion
Recommend where to add tests:
- "Add tests to F directly" vs
- "Add edge case tests to caller C"
