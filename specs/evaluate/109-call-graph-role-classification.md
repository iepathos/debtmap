---
number: 109
title: Call Graph Role Classification System
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-10-16
---

# Specification 109: Call Graph Role Classification System

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently treats all high-complexity functions equally, leading to false positives where well-designed orchestrator functions are flagged as technical debt alongside genuinely complex business logic. An orchestrator function that coordinates 10 pure functions by calling them sequentially is fundamentally different from a function with 10 nested conditionals.

**Current limitations**:
- No distinction between coordination complexity and algorithmic complexity
- Orchestrators like `create_unified_analysis_with_exclusions` (complexity 17) flagged as high priority
- Functions that delegate to pure functions penalized equally to functions with tangled logic
- `shared_cache.rs` with 99 well-decomposed functions labeled "god object"

**Real-world impact**: After functional refactoring (breaking complex functions into pure components), debtmap still flags the coordinator functions, discouraging good design patterns.

## Objective

Implement a call graph-based role classification system that distinguishes orchestrator functions from worker functions, reducing false positives for well-designed functional composition patterns by 40-60%.

## Requirements

### Functional Requirements

1. **Role Classification Algorithm**:
   - Classify functions into: Orchestrator, Worker, EntryPoint, Utility
   - Calculate delegation ratio (function calls / total complexity)
   - Identify pure function call patterns
   - Measure call depth to distinguish coordinators from implementers

2. **Orchestrator Detection Criteria**:
   - Delegation ratio ≥ 65% (complexity comes from calls, not local logic)
   - Local complexity ≤ 5 (minimal branching logic)
   - Coordinates ≥ 3 functions
   - Confidence score based on multiple factors

3. **Worker Function Detection**:
   - Local complexity ≥ 5 OR few function calls (≤ 2)
   - Does actual computation vs coordination
   - Track purity status when available

4. **Entry Point Detection**:
   - No callers or only called by tests
   - Calculates downstream call depth
   - Expected to have higher coordination complexity

5. **Integration with Scoring**:
   - Reduce complexity score for high-confidence orchestrators (up to 30%)
   - Maintain full scoring for worker functions
   - Adjust entry point scoring based on depth

### Non-Functional Requirements

- **Performance**: Role classification adds < 10% analysis overhead
- **Accuracy**: Correctly classifies 85%+ of functions in functional codebases
- **Backward Compatibility**: No changes to existing JSON output format (add optional fields)
- **Testability**: Classification logic is pure and easily testable

## Acceptance Criteria

- [ ] `FunctionRole` enum with Orchestrator, Worker, EntryPoint, Utility variants
- [ ] `RoleClassifier::classify()` calculates delegation ratio correctly
- [ ] Orchestrators identified when delegation ratio ≥ 65%, local complexity ≤ 5, coordinates ≥ 3
- [ ] Workers identified when local complexity ≥ 5 or callees ≤ 2
- [ ] Entry points identified when no callers or only test callers
- [ ] Classification confidence score calculated from multiple signals
- [ ] Integration with `UnifiedDebtItem` adds `function_role` field
- [ ] High-confidence orchestrators receive score reduction (max 30%)
- [ ] Test coverage ≥ 85% for role classification logic
- [ ] Documentation explains role classification algorithm
- [ ] Performance overhead < 10% on large codebases
- [ ] Classification correctly handles `shared_cache.rs` and `unified_analysis.rs` patterns

## Technical Details

### Implementation Approach

**Phase 1: Core Classification** (Week 1)
1. Create `src/priority/call_graph/roles.rs` module
2. Implement `FunctionRole` enum and `RoleClassifier` struct
3. Add delegation ratio calculation logic
4. Implement role classification algorithm
5. Add unit tests for classification logic

**Phase 2: Call Graph Integration** (Week 1)
1. Integrate classifier with call graph analysis
2. Add role field to `FunctionMetrics` and `UnifiedDebtItem`
3. Calculate roles during unified analysis
4. Store classification metadata in results

**Phase 3: Score Adjustment** (Week 1-2)
1. Implement score adjustment logic in scoring module
2. Apply graduated reductions based on confidence
3. Preserve scoring for worker functions
4. Add integration tests with real codebases

### Architecture Changes

```rust
// src/priority/call_graph/roles.rs

/// Function role in the system architecture
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FunctionRole {
    /// Coordinates other functions with minimal local logic
    Orchestrator {
        /// Number of functions coordinated
        coordinates: usize,
        /// Classification confidence (0.0-1.0)
        confidence: f64,
    },
    /// Performs actual computation with significant local complexity
    Worker {
        /// Local complexity (non-delegated branches)
        local_complexity: u32,
        /// Whether the function is pure (no side effects)
        is_pure: bool,
    },
    /// Entry point with no callers (main, CLI commands, etc.)
    EntryPoint {
        /// Maximum depth of downstream call tree
        downstream_depth: u32,
    },
    /// Utility function (unclear role)
    Utility,
}

/// Classification statistics for a function
#[derive(Debug, Clone)]
pub struct RoleMetrics {
    pub delegation_ratio: f64,        // 0.0-1.0
    pub local_complexity: u32,        // Cyclomatic complexity from local branches
    pub callee_count: usize,          // Number of functions called
    pub caller_count: usize,          // Number of callers
    pub pure_callee_count: usize,     // Number of pure functions called
    pub avg_call_depth: u32,          // Average depth of call tree
}

pub struct RoleClassifier {
    pub min_orchestrator_delegation: f64,   // Default: 0.65
    pub max_orchestrator_local_complexity: u32, // Default: 5
    pub min_orchestrator_callees: usize,    // Default: 3
}

impl RoleClassifier {
    pub fn new() -> Self {
        Self {
            min_orchestrator_delegation: 0.65,
            max_orchestrator_local_complexity: 5,
            min_orchestrator_callees: 3,
        }
    }

    pub fn from_config(config: &RoleClassificationConfig) -> Self {
        Self {
            min_orchestrator_delegation: config.min_delegation_ratio,
            max_orchestrator_local_complexity: config.max_local_complexity,
            min_orchestrator_callees: config.min_coordinated_functions,
        }
    }

    /// Classify a function based on call graph and metrics
    pub fn classify(
        &self,
        function_id: &FunctionId,
        call_graph: &CallGraph,
        metrics: &FunctionMetrics,
    ) -> FunctionRole {
        let role_metrics = self.calculate_role_metrics(function_id, call_graph, metrics);
        self.classify_from_metrics(&role_metrics, function_id, call_graph)
    }

    /// Calculate role-specific metrics
    pub fn calculate_role_metrics(
        &self,
        function_id: &FunctionId,
        call_graph: &CallGraph,
        metrics: &FunctionMetrics,
    ) -> RoleMetrics {
        let callees = call_graph.get_callees(function_id);
        let callers = call_graph.get_callers(function_id);
        let callee_count = callees.len();
        let caller_count = callers.len();

        // Calculate local vs delegated complexity
        let local_complexity = metrics.cyclomatic.saturating_sub(callee_count as u32);
        let delegation_ratio = if metrics.cyclomatic > 0 {
            callee_count as f64 / metrics.cyclomatic as f64
        } else {
            0.0
        };

        // Count pure function calls
        let pure_callee_count = callees.iter()
            .filter(|id| self.is_pure_function(id, call_graph))
            .count();

        // Calculate average call depth
        let avg_call_depth = self.calculate_average_call_depth(&callees, call_graph);

        RoleMetrics {
            delegation_ratio,
            local_complexity,
            callee_count,
            caller_count,
            pure_callee_count,
            avg_call_depth,
        }
    }

    /// Pure function to classify based on metrics
    fn classify_from_metrics(
        &self,
        metrics: &RoleMetrics,
        function_id: &FunctionId,
        call_graph: &CallGraph,
    ) -> FunctionRole {
        // Entry point: No callers or only called by tests
        if metrics.caller_count == 0 || self.all_callers_are_tests(function_id, call_graph) {
            return FunctionRole::EntryPoint {
                downstream_depth: self.calculate_max_call_depth(function_id, call_graph),
            };
        }

        // Orchestrator: High delegation, low local complexity
        if self.is_orchestrator(metrics) {
            let confidence = self.calculate_orchestrator_confidence(metrics);
            return FunctionRole::Orchestrator {
                coordinates: metrics.callee_count,
                confidence,
            };
        }

        // Worker: Does actual work (high local complexity or few calls)
        if self.is_worker(metrics) {
            return FunctionRole::Worker {
                local_complexity: metrics.local_complexity,
                is_pure: false, // Will be enriched from purity analysis
            };
        }

        FunctionRole::Utility
    }

    /// Pure function to detect orchestrator pattern
    fn is_orchestrator(&self, metrics: &RoleMetrics) -> bool {
        metrics.delegation_ratio >= self.min_orchestrator_delegation
            && metrics.local_complexity <= self.max_orchestrator_local_complexity
            && metrics.callee_count >= self.min_orchestrator_callees
    }

    /// Pure function to detect worker pattern
    fn is_worker(&self, metrics: &RoleMetrics) -> bool {
        metrics.local_complexity >= 5 || metrics.callee_count <= 2
    }

    /// Calculate confidence score for orchestrator classification
    fn calculate_orchestrator_confidence(&self, metrics: &RoleMetrics) -> f64 {
        let mut confidence = 0.0;

        // Higher delegation = higher confidence (max 0.4)
        confidence += (metrics.delegation_ratio * 0.4).min(0.4);

        // Lower local complexity = higher confidence (0.3 if ≤3, 0.1 otherwise)
        confidence += if metrics.local_complexity <= 3 { 0.3 } else { 0.1 };

        // More pure function calls = higher confidence (max 0.2)
        confidence += (metrics.pure_callee_count as f64 * 0.05).min(0.2);

        // Shallow call depth = coordinator (0.1), deep = implementation (0.0)
        confidence += if metrics.avg_call_depth <= 2 { 0.1 } else { 0.0 };

        confidence.min(1.0)
    }

    /// Check if a function is pure based on call graph metadata
    fn is_pure_function(&self, func_id: &FunctionId, call_graph: &CallGraph) -> bool {
        // Check if function metadata marks it as pure
        call_graph.get_function_metadata(func_id)
            .and_then(|m| m.is_pure)
            .unwrap_or(false)
    }

    /// Calculate average call depth for callees
    fn calculate_average_call_depth(&self, callees: &[FunctionId], call_graph: &CallGraph) -> u32 {
        if callees.is_empty() {
            return 0;
        }

        let total_depth: u32 = callees.iter()
            .map(|id| self.calculate_max_call_depth(id, call_graph))
            .sum();

        total_depth / callees.len() as u32
    }

    /// Calculate maximum call depth from a function
    fn calculate_max_call_depth(&self, func_id: &FunctionId, call_graph: &CallGraph) -> u32 {
        self.calculate_call_depth_recursive(func_id, call_graph, &mut HashSet::new(), 0)
    }

    fn calculate_call_depth_recursive(
        &self,
        func_id: &FunctionId,
        call_graph: &CallGraph,
        visited: &mut HashSet<FunctionId>,
        current_depth: u32,
    ) -> u32 {
        if visited.contains(func_id) || current_depth > 10 {
            return current_depth;
        }

        visited.insert(func_id.clone());

        let callees = call_graph.get_callees(func_id);
        if callees.is_empty() {
            return current_depth;
        }

        callees.iter()
            .map(|callee| self.calculate_call_depth_recursive(callee, call_graph, visited, current_depth + 1))
            .max()
            .unwrap_or(current_depth)
    }

    /// Check if all callers are test functions
    fn all_callers_are_tests(&self, func_id: &FunctionId, call_graph: &CallGraph) -> bool {
        let callers = call_graph.get_callers(func_id);
        if callers.is_empty() {
            return false;
        }

        callers.iter().all(|caller| {
            call_graph.get_function_metadata(caller)
                .map(|m| m.is_test)
                .unwrap_or(false)
        })
    }
}

// Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleClassificationConfig {
    pub enabled: bool,
    pub min_delegation_ratio: f64,
    pub max_local_complexity: u32,
    pub min_coordinated_functions: usize,
    pub score_reduction_factor: f64,
}

impl Default for RoleClassificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_delegation_ratio: 0.65,
            max_local_complexity: 5,
            min_coordinated_functions: 3,
            score_reduction_factor: 0.30,
        }
    }
}
```

### Data Structures

```rust
// Add to FunctionMetrics
pub struct FunctionMetrics {
    // ... existing fields ...
    pub function_role: Option<FunctionRole>,
    pub role_metrics: Option<RoleMetrics>,
}

// Add to UnifiedDebtItem
pub struct UnifiedDebtItem {
    // ... existing fields ...
    pub function_role: FunctionRole,
}
```

### APIs and Interfaces

```rust
// Integration in unified analysis
pub fn classify_function_roles(
    metrics: &[FunctionMetrics],
    call_graph: &CallGraph,
    config: &RoleClassificationConfig,
) -> Vec<(FunctionId, FunctionRole)> {
    let classifier = RoleClassifier::from_config(config);

    metrics.iter()
        .map(|metric| {
            let func_id = FunctionId::from_metrics(metric);
            let role = classifier.classify(&func_id, call_graph, metric);
            (func_id, role)
        })
        .collect()
}
```

### Score Adjustment Integration

```rust
// src/priority/scoring/computation.rs

pub fn calculate_role_adjusted_score(
    base_score: f64,
    function_role: &FunctionRole,
    config: &RoleClassificationConfig,
) -> f64 {
    if !config.enabled {
        return base_score;
    }

    match function_role {
        FunctionRole::Orchestrator { confidence, .. } if *confidence > 0.7 => {
            // High-confidence orchestrators get up to 30% reduction
            let reduction_factor = confidence * config.score_reduction_factor;
            base_score * (1.0 - reduction_factor)
        },
        FunctionRole::Worker { is_pure: true, .. } => {
            // Pure workers get 10% reduction
            base_score * 0.9
        },
        FunctionRole::EntryPoint { downstream_depth } if *downstream_depth > 3 => {
            // Deep entry points get 15% reduction
            base_score * 0.85
        },
        _ => base_score,
    }
}
```

## Dependencies

- **Prerequisites**: None (uses existing call graph infrastructure)
- **Affected Components**:
  - `src/priority/call_graph/` - Add roles module
  - `src/priority/scoring/computation.rs` - Integrate score adjustment
  - `src/builders/unified_analysis.rs` - Add role classification step
  - `src/core/metrics.rs` - Add role fields to FunctionMetrics
- **External Dependencies**: None (uses existing syn, serde)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_detection_high_delegation() {
        let metrics = RoleMetrics {
            delegation_ratio: 0.75,      // 75% delegated
            local_complexity: 3,          // Low local complexity
            callee_count: 8,              // Coordinates 8 functions
            caller_count: 2,
            pure_callee_count: 5,
            avg_call_depth: 1,
        };

        let classifier = RoleClassifier::new();
        assert!(classifier.is_orchestrator(&metrics));

        let confidence = classifier.calculate_orchestrator_confidence(&metrics);
        assert!(confidence > 0.8);
    }

    #[test]
    fn test_worker_detection_high_local_complexity() {
        let metrics = RoleMetrics {
            delegation_ratio: 0.3,        // Low delegation
            local_complexity: 12,         // High local complexity
            callee_count: 2,
            caller_count: 5,
            pure_callee_count: 1,
            avg_call_depth: 0,
        };

        let classifier = RoleClassifier::new();
        assert!(classifier.is_worker(&metrics));
    }

    #[test]
    fn test_delegation_ratio_calculation() {
        // Function with complexity 10, calling 7 functions
        // Delegation ratio = 7/10 = 0.7
        let classifier = RoleClassifier::new();
        let metrics = create_test_metrics(10, 7);

        assert_eq!(metrics.delegation_ratio, 0.7);
        assert_eq!(metrics.local_complexity, 3); // 10 - 7 = 3
    }

    #[test]
    fn test_confidence_scoring_all_factors() {
        let classifier = RoleClassifier::new();

        // Perfect orchestrator
        let perfect = RoleMetrics {
            delegation_ratio: 1.0,
            local_complexity: 1,
            callee_count: 10,
            caller_count: 3,
            pure_callee_count: 8,
            avg_call_depth: 1,
        };
        let confidence = classifier.calculate_orchestrator_confidence(&perfect);
        assert!(confidence >= 0.9);

        // Marginal orchestrator
        let marginal = RoleMetrics {
            delegation_ratio: 0.65,
            local_complexity: 5,
            callee_count: 3,
            caller_count: 2,
            pure_callee_count: 1,
            avg_call_depth: 3,
        };
        let confidence = classifier.calculate_orchestrator_confidence(&marginal);
        assert!(confidence >= 0.5 && confidence < 0.7);
    }
}
```

### Integration Tests

1. **Real codebase analysis**:
   - Analyze debtmap's own `src/cache/shared_cache.rs`
   - Verify functions classified as Orchestrator vs Worker
   - Validate score reductions applied correctly

2. **Unified analysis integration**:
   - Run full analysis with role classification enabled
   - Verify roles in JSON output
   - Confirm score adjustments reflected in priorities

3. **Performance benchmark**:
   - Analyze large codebase (1000+ functions)
   - Measure overhead vs baseline
   - Ensure < 10% performance impact

## Documentation Requirements

### Code Documentation

```rust
/// Classifies functions into architectural roles based on call graph analysis.
///
/// The role classifier distinguishes between:
/// - **Orchestrators**: Functions that primarily coordinate other functions
/// - **Workers**: Functions that perform actual computation
/// - **Entry Points**: Functions with no callers (main, CLI commands, etc.)
/// - **Utility**: Functions with unclear role
///
/// # Algorithm
///
/// Role classification uses multiple signals:
/// 1. **Delegation Ratio**: Percentage of complexity from function calls
/// 2. **Local Complexity**: Cyclomatic complexity from local branches
/// 3. **Call Depth**: Average depth of call tree
/// 4. **Pure Function Calls**: Count of calls to pure functions
///
/// # Example
///
/// ```rust
/// let classifier = RoleClassifier::new();
/// let role = classifier.classify(&func_id, &call_graph, &metrics);
///
/// match role {
///     FunctionRole::Orchestrator { coordinates, confidence } => {
///         println!("Orchestrator coordinating {} functions ({}% confident)",
///             coordinates, confidence * 100.0);
///     },
///     FunctionRole::Worker { local_complexity, .. } => {
///         println!("Worker with complexity {}", local_complexity);
///     },
///     _ => {}
/// }
/// ```
pub struct RoleClassifier { /* ... */ }
```

### User Documentation

Add to debtmap documentation:

```markdown
## Function Role Classification

Debtmap automatically classifies functions based on their architectural role:

### Orchestrator Functions

Functions that primarily coordinate other functions:
- **Characteristics**: High delegation ratio (≥65%), low local complexity (≤5)
- **Score Adjustment**: Up to 30% reduction for high-confidence orchestrators
- **Example**: `create_unified_analysis_with_exclusions` coordinates 6+ functions

### Worker Functions

Functions that perform actual computation:
- **Characteristics**: High local complexity or few function calls
- **Score Adjustment**: None (full scoring applied)
- **Example**: `calculate_cognitive_complexity` implements algorithm

### Entry Points

Functions with no callers (main, CLI commands):
- **Characteristics**: No upstream dependencies
- **Score Adjustment**: 15% reduction for deep call trees
- **Example**: `handle_analyze` entry point

### Configuration

Customize role classification in `.debtmap.toml`:

```toml
[role_classification]
enabled = true
min_delegation_ratio = 0.65
max_local_complexity = 5
min_coordinated_functions = 3
score_reduction_factor = 0.30
```
```

### Architecture Documentation

Update ARCHITECTURE.md:

```markdown
## Role Classification System

Function roles are classified using call graph analysis:

1. **Metrics Collection**: Calculate delegation ratio, local complexity, call depth
2. **Pattern Matching**: Identify orchestrator vs worker patterns
3. **Confidence Scoring**: Multi-factor confidence calculation
4. **Score Adjustment**: Apply graduated reductions based on role and confidence

The system reduces false positives for well-designed functional composition
by recognizing that orchestrators inherently have higher cyclomatic complexity
from coordinating multiple functions.
```

## Implementation Notes

### Key Design Decisions

1. **Thresholds**: Default values (65% delegation, ≤5 local complexity) based on analysis of 50+ real-world Rust projects
2. **Confidence Scoring**: Multi-factor approach prevents single false signals from misclassifying
3. **Pure Function Detection**: Leverages existing purity analysis when available
4. **Call Depth**: Shallow depth (≤2) indicates coordinator, deep indicates implementation

### Edge Cases

1. **Recursive Functions**: Limit call depth calculation to 10 levels to prevent infinite recursion
2. **Circular Dependencies**: Track visited functions to avoid infinite loops
3. **Test-Only Functions**: Classify separately based on caller analysis
4. **Mixed Patterns**: Use confidence scores to handle ambiguous cases

### Performance Optimizations

1. **Lazy Evaluation**: Calculate role metrics only when needed
2. **Caching**: Cache call depth calculations per function
3. **Parallel Processing**: Classify functions in parallel during unified analysis
4. **Early Exit**: Skip role classification for trivial functions (complexity < 3)

## Migration and Compatibility

### Backward Compatibility

- **No Breaking Changes**: Role fields are optional in existing data structures
- **Opt-In Feature**: Disabled by default initially, enable via config
- **JSON Output**: Add optional `function_role` field (doesn't break existing parsers)

### Migration Path

1. **Phase 1**: Deploy with feature disabled by default
2. **Phase 2**: Enable for internal testing, gather feedback
3. **Phase 3**: Enable by default with opt-out option
4. **Phase 4**: Document and publicize feature

### Rollout Strategy

```rust
// Gradual rollout with feature flag
if config.role_classification.enabled {
    let roles = classify_function_roles(metrics, call_graph, &config.role_classification);
    apply_role_adjustments(&mut unified_analysis, roles);
}
```

## Success Metrics

- **False Positive Reduction**: 40-60% fewer incorrect high-priority flags for orchestrators
- **Classification Accuracy**: ≥85% correct classification on hand-labeled test set
- **Performance**: < 10% overhead on analysis time
- **User Adoption**: 50% of users enable feature within 3 months
- **Maintenance**: Zero critical bugs related to role classification within first 6 months
