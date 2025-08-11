---
number: 19
title: Unified Debt Prioritization with Semantic Analysis
category: optimization
priority: high
status: draft
dependencies: [5, 8, 14, 15, 18]
created: 2025-08-11
supersedes: [15, 18]
---

# Specification 19: Unified Debt Prioritization with Semantic Analysis

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [5 - Complexity-Coverage Risk Analysis, 8 - Testing Prioritization, 14 - Dependency-Aware ROI]
**Supersedes**: [15 - Automated Tech Debt Prioritization, 18 - Semantic Delegation Detection with Coverage Propagation]

## Context

Debtmap currently provides comprehensive technical debt analysis but suffers from several issues that make the output difficult to action:

1. **Disconnected Metrics**: ROI, complexity, coverage, and risk are reported separately, requiring manual correlation to determine what to fix first
2. **False Positives**: Orchestration functions that delegate to tested code are incorrectly prioritized as critical
3. **Information Overload**: Raw analysis dumps overwhelm users who just want to know "what should I fix?"
4. **Inconsistent Prioritization**: Different analysis modes (risk, ROI, complexity) provide conflicting recommendations
5. **Outdated Time Estimates**: Traditional effort estimation has become unreliable with LLM-assisted development fundamentally changing task completion times

This specification unifies all debt analysis into a single, semantic-aware prioritization engine that provides clear, actionable output focused on impact rather than estimated effort.

## Objective

Create a unified technical debt prioritization system that:
- Combines complexity, coverage, ROI, and semantic analysis into a single score
- Distinguishes between orchestration code and business logic to avoid false positives  
- Provides clean, actionable output with clear prioritization
- Supports different verbosity levels for different use cases
- Maintains backward compatibility while dramatically improving usability

## Requirements

### Functional Requirements

1. **Unified Scoring Algorithm**
   - Single priority score incorporating all analysis dimensions
   - Weighted combination of: complexity (25%), coverage gaps (35%), ROI (25%), semantic role (15%)
   - Semantic multipliers based on function classification
   - Dependency cascade effects from call graph analysis

2. **Semantic Function Classification**
   - Build call graph during AST analysis
   - Classify functions into roles: PureLogic, Orchestrator, IOWrapper, EntryPoint, Unknown
   - Apply role-based priority multipliers:
     - PureLogic with no coverage: 1.5x multiplier
     - Orchestrator calling tested functions: 0.2x multiplier
     - IOWrapper: 0.1x multiplier
     - EntryPoint: 0.8x multiplier (integration test priority)

3. **Coverage Propagation**
   - Calculate transitive coverage through call graph
   - Consider orchestration functions "covered" if they delegate to >80% covered functions
   - Track coverage inheritance patterns
   - Distinguish direct vs. transitive coverage in scoring

4. **Clean Priority Output**
   - Default: Show top 10 prioritized items with clear action items
   - `--top N`: Limit to N highest priority items
   - `--priorities-only`: Skip raw analysis, show only prioritized list
   - `--detailed`: Include full analysis with priority overlay
   - Single unified list ordered by composite priority score

5. **Actionable Recommendations**
   - Specific fix recommendations for each priority item
   - Expected impact metrics for each fix
   - Implementation hints and related item groupings
   - Focus on measurable outcomes rather than time estimates

6. **Call Graph Integration**
   - Extract function calls during AST parsing
   - Build directed dependency graph
   - Detect delegation patterns: simple delegation, pipeline composition, error propagation
   - Use for both semantic analysis and dependency-aware ROI calculations

### Non-Functional Requirements

1. **Performance**
   - Unified analysis should add <200ms total to processing time
   - Call graph construction optimized with lazy evaluation
   - Caching for call graphs when files unchanged
   - Memory efficient for codebases with 10k+ functions

2. **Accuracy**
   - >90% accuracy in function role classification on test cases
   - No false positives for simple orchestration patterns
   - Consistent priority ordering across similar codebases
   - Stable scores for unchanged code between runs

3. **Usability**
   - Default output immediately actionable without documentation
   - Progressive disclosure: simple by default, detailed on request
   - Clear visual hierarchy in terminal output
   - Backward compatible CLI interface

## Acceptance Criteria

- [ ] Single priority score combines all analysis dimensions accurately
- [ ] Orchestration functions like `generate_report_if_requested` score <3.0 (down from 8.8)
- [ ] Pure business logic functions with no coverage score >7.0
- [ ] Call graph built from AST with function relationships tracked
- [ ] Transitive coverage calculated and applied to priority scoring
- [ ] Default output shows top 10 items with clear action items
- [ ] `--top N` flag limits output to N items with clean formatting
- [ ] `--priorities-only` provides minimal, action-focused output
- [ ] Performance overhead <200ms on 1000+ function codebases
- [ ] All existing tests pass with unified scoring
- [ ] Output format is immediately clear without reading documentation

## Implementation Notes

### Why No Effort Estimates in Output

Traditional time estimation for software development tasks has become fundamentally unreliable with the advent of LLM-assisted coding:

1. **Variable Skill Amplification**: LLMs amplify developer capabilities differently - a "complex" refactoring might take 20 minutes with AI assistance or 4 hours without
2. **Context-Dependent Acceleration**: Tasks involving boilerplate, testing, or documentation see 5-10x speedups, while architectural decisions remain similar
3. **Individual Variation**: Developer familiarity with LLM tools creates dramatic variance in actual completion times
4. **Dynamic Problem Solving**: AI assistance can change the approach mid-task, making initial estimates meaningless

Instead of potentially misleading time estimates, this specification focuses on:
- **Measurable Impact**: Concrete improvements to code quality metrics
- **Actionable Priority**: Clear ordering based on value delivered
- **Complexity Indicators**: High/medium/low complexity as relative guidance
- **Dependency Awareness**: Understanding what enables other improvements

The unified scoring algorithm still considers effort internally for ROI calculations, but the output emphasizes **what to fix** and **why** rather than **how long it will take**.

## Technical Details

### Unified Scoring Algorithm

```rust
pub struct UnifiedScore {
    pub complexity_factor: f64,    // 0-10, weighted 25%
    pub coverage_factor: f64,      // 0-10, weighted 35%
    pub roi_factor: f64,           // 0-10, weighted 25%
    pub semantic_factor: f64,      // 0-10, weighted 15%
    pub role_multiplier: f64,      // 0.1-1.5x based on function role
    pub final_score: f64,          // Computed composite score
}

pub fn calculate_unified_priority(
    item: &DebtItem,
    call_graph: &CallGraph,
    coverage: &CoverageData
) -> UnifiedScore {
    let complexity_factor = normalize_complexity(item.complexity_score);
    let coverage_factor = calculate_coverage_urgency(item, call_graph, coverage);
    let roi_factor = normalize_roi(item.roi_score);
    
    let role = classify_function_role(item, call_graph);
    let semantic_factor = calculate_semantic_priority(item, role, call_graph);
    let role_multiplier = get_role_multiplier(role);
    
    let base_score = 
        complexity_factor * 0.25 +
        coverage_factor * 0.35 +
        roi_factor * 0.25 +
        semantic_factor * 0.15;
    
    let final_score = base_score * role_multiplier;
    
    UnifiedScore {
        complexity_factor,
        coverage_factor,
        roi_factor,
        semantic_factor,
        role_multiplier,
        final_score,
    }
}
```

### Function Role Classification

```rust
pub enum FunctionRole {
    PureLogic,      // Business logic, high test priority
    Orchestrator,   // Coordinates other functions
    IOWrapper,      // Thin I/O layer
    EntryPoint,     // Main entry points
    Unknown,        // Cannot classify
}

pub fn classify_function_role(
    func: &FunctionMetrics,
    call_graph: &CallGraph
) -> FunctionRole {
    // Simple orchestration: low complexity, mostly delegates
    if func.cyclomatic_complexity <= 2 &&
       func.cognitive_complexity <= 3 &&
       delegates_to_tested_functions(func, call_graph, 0.8) {
        return FunctionRole::Orchestrator;
    }
    
    // I/O wrapper: contains I/O patterns, thin logic
    if contains_io_patterns(func) && func.lines_of_code < 20 {
        return FunctionRole::IOWrapper;
    }
    
    // Entry point: called by main/public API, integration focus
    if is_entry_point(func) {
        return FunctionRole::EntryPoint;
    }
    
    // Pure logic: everything else
    FunctionRole::PureLogic
}
```

### Coverage Propagation

```rust
pub struct TransitiveCoverage {
    pub direct: f64,
    pub transitive: f64,
    pub propagated_from: Vec<FunctionId>,
}

pub fn calculate_transitive_coverage(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: &CoverageData
) -> TransitiveCoverage {
    let direct = coverage.get_function_coverage(func.id);
    
    if direct > 0.0 {
        return TransitiveCoverage {
            direct,
            transitive: direct,
            propagated_from: vec![],
        };
    }
    
    // Calculate coverage from callees
    let callees = call_graph.get_callees(func.id);
    let covered_callees: Vec<_> = callees
        .iter()
        .filter(|&callee| coverage.get_function_coverage(*callee) > 0.8)
        .collect();
    
    let transitive = if callees.is_empty() {
        0.0
    } else {
        covered_callees.len() as f64 / callees.len() as f64
    };
    
    TransitiveCoverage {
        direct,
        transitive,
        propagated_from: covered_callees.into_iter().copied().collect(),
    }
}
```

### Architecture Changes

1. **New Unified Priority Module**: `src/priority/`
   - `unified_scorer.rs`: Main scoring algorithm
   - `semantic_classifier.rs`: Function role classification
   - `call_graph.rs`: Call graph construction and analysis
   - `coverage_propagation.rs`: Transitive coverage calculation
   - `formatter.rs`: Clean output formatting

2. **Enhanced Analysis Pipeline**
   - Build call graph during AST parsing phase
   - Run semantic classification after complexity analysis
   - Calculate unified scores in final priority phase
   - Generate clean recommendations from top scores

3. **CLI Integration**
   ```rust
   // New CLI flags
   --top N              // Show top N priorities (default: 10)
   --priorities-only    // Skip analysis details, priorities only
   --detailed          // Include full analysis with priority overlay
   --semantic-off      // Disable semantic analysis (fallback)
   --explain-score     // Show score breakdown for debugging
   ```

### Data Structures

```rust
pub struct UnifiedDebtItem {
    pub location: Location,
    pub debt_type: DebtType,
    pub unified_score: UnifiedScore,
    pub function_role: FunctionRole,
    pub recommendation: ActionableRecommendation,
    // Note: effort_estimate removed - LLMs make time prediction unreliable
    pub expected_impact: ImpactMetrics,
    pub transitive_coverage: Option<TransitiveCoverage>,
}

pub struct ActionableRecommendation {
    pub primary_action: String,
    pub rationale: String,
    pub implementation_steps: Vec<String>,
    pub related_items: Vec<DebtItemId>,
    // Note: expected_time removed - focus on impact over time estimation
}

pub enum DebtType {
    TestingGap { coverage: f64, complexity: u32 },
    ComplexityHotspot { cyclomatic: u32, cognitive: u32 },
    Orchestration { delegates_to: Vec<FunctionId> },
    Duplication { instances: u32, total_lines: u32 },
    Risk { risk_score: f64, factors: Vec<String> },
}
```

## Clean Output Format

### Default Output (--top 10)
```
════════════════════════════════════════════
    PRIORITY TECHNICAL DEBT FIXES
════════════════════════════════════════════

🎯 TOP 10 RECOMMENDATIONS (by unified priority)

#1  SCORE: 9.2  [CRITICAL]
├─ TEST GAP: src/core/parser.rs:45 parse_config()
├─ ACTION: Add 3 unit tests for branch coverage
├─ IMPACT: +67% module coverage, -4.2 risk score
└─ WHY: Core business logic, zero coverage, high dependencies

#2  SCORE: 8.8  [CRITICAL]  
├─ COMPLEXITY: src/risk/calc.rs:12 calculate_weighted_risk()
├─ ACTION: Extract 2 sub-functions to reduce complexity
├─ IMPACT: -8 cyclomatic complexity, +15% readability
└─ WHY: Highest complexity function, affects all risk calculations

#3  SCORE: 7.9  [HIGH]
├─ DUPLICATION: src/analyzers/*.rs (4 locations)
├─ ACTION: Extract common parser logic into shared module
├─ IMPACT: -180 LOC, single source of truth
└─ WHY: Maintenance burden, risk of inconsistent behavior

[Items #4-10...]

💡 HIGH-IMPACT LOW-COMPLEXITY FIXES
• Add missing error tests: src/io/reader.rs:78
• Remove dead code: src/utils/helpers.rs:156  
• Fix TODO comment: src/core/cache.rs:23

📊 TOTAL IMPACT IF ALL FIXED
• +28% test coverage
• -156 lines of code  
• -35% average complexity
• 10 actionable items prioritized by measurable impact
```

### Minimal Output (--priorities-only)
```
TOP PRIORITIES:
1. Add tests: src/core/parser.rs:45 parse_config()
2. Reduce complexity: src/risk/calc.rs:12 calculate_weighted_risk()
3. Extract duplication: src/analyzers/*.rs
4. Add error handling: src/io/writer.rs:34 write_results()
5. Test entry point: src/main.rs:67 run_analysis()

High-impact items: 3 critical, 2 high priority
Focus on measurable code quality improvements
```

### Detailed Output (--detailed)
```
[Full analysis sections as before...]

════════════════════════════════════════════
    UNIFIED PRIORITY ANALYSIS
════════════════════════════════════════════

#1  parse_config() - UNIFIED SCORE: 9.2
├─ Function Role: PureLogic (1.0x multiplier)
├─ Score Breakdown:
│  ├─ Coverage Factor: 10.0 (0% direct, 0% transitive)
│  ├─ Complexity Factor: 6.8 (CC:3, Cognitive:4)
│  ├─ ROI Factor: 9.5 (high impact, low effort)
│  └─ Semantic Factor: 8.0 (core business logic)
├─ Call Graph: Called by 8 functions, calls 3 utilities
├─ Dependencies: 5 modules depend on this function
└─ Recommendation: Add comprehensive unit tests covering all branches

[Continue with detailed breakdown...]
```

## Implementation Phases

### Phase 1: Call Graph Infrastructure (Week 1)
- Extend AST parsing to extract function calls
- Build CallGraph data structure  
- Implement basic traversal and query operations
- Add integration with existing analysis pipeline

### Phase 2: Semantic Classification (Week 2)
- Implement function role classification algorithm
- Add pattern detection for orchestration and I/O
- Create role-based multipliers
- Test classification accuracy on sample codebases

### Phase 3: Coverage Propagation (Week 3)
- Implement transitive coverage calculation
- Add coverage inheritance through call graph
- Optimize for performance with caching
- Integrate with existing coverage analysis

### Phase 4: Unified Scoring (Week 4)
- Implement unified priority scoring algorithm
- Combine all analysis dimensions with proper weighting
- Add score normalization and calibration
- Test scoring consistency across different codebases

### Phase 5: Clean Output Formatting (Week 5)
- Implement new output formatters for different verbosity levels
- Add CLI flags for output control
- Create clear, actionable recommendation templates
- Polish visual formatting for terminal output

### Phase 6: Integration & Testing (Week 6)
- Integration testing with real codebases
- Performance optimization and profiling
- User acceptance testing and feedback incorporation
- Documentation updates and migration guide

## Dependencies

- **Prerequisites**: 
  - Spec 5: Risk Analysis (complexity and coverage data)
  - Spec 8: Testing Prioritization (ROI calculations)
  - Spec 14: Dependency-Aware ROI (cascade effects)
- **Supersedes**:
  - Spec 15: Automated Tech Debt Prioritization (unified into this spec)
  - Spec 18: Semantic Delegation Detection (semantic analysis integrated)
- **Affected Components**:
  - `src/core/analysis.rs`: Main analysis pipeline integration
  - `src/cli.rs`: New CLI flags and output options
  - `src/io/output.rs`: New output formatters
  - All `src/analyzers/*`: AST parsing enhancements for call extraction

## Testing Strategy

- **Unit Tests**:
  - Unified scoring algorithm with various input combinations
  - Function role classification accuracy on curated examples
  - Coverage propagation with mock call graphs
  - Output formatting for different verbosity levels

- **Integration Tests**:
  - End-to-end unified analysis on real codebases
  - Verify orchestration functions get appropriate scores
  - Test performance with large codebases (1000+ functions)
  - Validate backward compatibility of CLI interface

- **Acceptance Tests**:
  - Verify `generate_report_if_requested` scores <3.0 
  - Confirm pure business logic maintains high priority
  - Test that high-impact low-complexity items are clearly identified
  - Validate output is immediately actionable

## Migration and Compatibility

- **Backward Compatibility**: All existing CLI flags continue to work
- **New Default Behavior**: Clean priority output becomes default
- **Migration Path**: `--legacy-output` flag preserves old format
- **Configuration**: Scoring weights configurable via config file
- **Caching**: New cache format, automatic invalidation on upgrade

## Success Metrics

1. **Usability**: Users can identify what to fix first without reading documentation
2. **Accuracy**: <5% false positives for orchestration vs. business logic
3. **Performance**: <200ms overhead on large codebases
4. **Adoption**: Users prefer new unified output over separate analysis modes
5. **Effectiveness**: Fixes guided by unified priorities show measurable improvement

This specification combines the comprehensive analysis vision from spec 15 with the semantic accuracy improvements from spec 18, while solving the core problem of information overload through clean, prioritized output that guides developers to the highest-impact fixes first.