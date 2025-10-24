---
number: 144
title: Call Graph Integration for God Object Cohesion Scoring (Phase 2)
category: optimization
priority: medium
status: draft
dependencies: [143]
created: 2025-01-23
related: [143, 145]
---

# Specification 144: Call Graph Integration for God Object Cohesion Scoring (Phase 2)

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 143 (struct ownership foundation)
**Related**: Spec 143 (Phase 1), Spec 145 (Phase 3)

## Context

**Current State**: Spec 143 provides struct-ownership-based god object splitting that recommends modules based on domain classification and size validation. However, these recommendations lack quantitative quality metrics.

**Problem**: Without cohesion scoring and dependency analysis, we cannot:
1. Measure how well methods within a module work together
2. Identify circular dependencies between recommended modules
3. Prioritize splits based on actual coupling/cohesion metrics
4. Validate that recommendations actually improve code organization

**Example Scenario**:
```rust
// config.rs has these structs:
struct ScoringWeights { ... }
impl ScoringWeights {
    fn get_default() -> Self { ... }
    fn apply_multipliers(&self, ...) { ... }  // Calls RoleMultipliers::get()
}

struct RoleMultipliers { ... }
impl RoleMultipliers {
    fn get() -> Self { ... }
    fn apply_to_score(&self, ...) { ... }     // Calls ScoringWeights::normalize()
}
```

**Current Recommendation** (Spec 143):
- scoring.rs: ScoringWeights + RoleMultipliers (domain-based grouping)
- ✅ Correct grouping

**With Cohesion Scoring** (This Spec):
- scoring.rs: Cohesion = 0.85 (8 internal calls / 10 total calls)
- Dependencies: Uses `thresholds::ValidationLimits`
- ✅ High cohesion validates the recommendation

## Objective

Enhance god object refactoring recommendations with quantitative quality metrics by integrating call graph analysis to calculate:
1. **Cohesion scores** (0.0-1.0) measuring how tightly methods within a module relate
2. **Dependency maps** showing what each module uses and what uses it
3. **Circular dependency detection** to warn about problematic splits
4. **Quality-based priority** using cohesion to rank recommendations

**Success Criteria**:
- All module split recommendations include cohesion scores
- Cohesion scores accurately reflect internal vs external coupling
- Circular dependencies are detected and reported
- High-cohesion modules (>0.7) are prioritized over low-cohesion (<0.5)

## Requirements

### Functional Requirements

**FR1: Cohesion Score Calculation**
- Calculate cohesion as ratio of internal calls to total calls
- Formula: `cohesion = internal_calls / (internal_calls + external_calls)`
- Range: 0.0 (no cohesion) to 1.0 (perfect cohesion)
- Handle edge cases (modules with no function calls)

**FR2: Dependency Analysis**
- Track which external modules/structs each recommended module depends on
- Track which modules depend on this module (reverse dependencies)
- Store dependencies in `dependencies_in` and `dependencies_out` fields
- Exclude standard library and external crate dependencies

**FR3: Circular Dependency Detection**
- Detect cycles in recommended module dependencies
- Report circular dependencies as warnings
- Downgrade priority for modules involved in cycles
- Suggest cycle-breaking strategies

**FR4: Priority Assignment Based on Quality**
- Use cohesion score to influence priority
- High cohesion (>0.7) + no cycles → High priority
- Medium cohesion (0.5-0.7) or cycles → Medium priority
- Low cohesion (<0.5) → Low priority (or skip recommendation)
- Override domain-based priority from Spec 143

### Non-Functional Requirements

**NFR1: Performance**
- Call graph integration should add <20% to total god object detection time
- Leverage existing call graph caching where available
- Support incremental call graph analysis

**NFR2: Accuracy**
- Cohesion scores should correlate with actual code organization quality
- False positive rate for circular dependencies <5%
- Dependency lists should be complete and accurate

**NFR3: Integration**
- Seamlessly extend Spec 143 without breaking existing functionality
- Maintain backward compatibility (cohesion optional)
- Work with existing call graph infrastructure (src/analyzers/rust_call_graph.rs)

## Acceptance Criteria

### AC1: Cohesion Score Calculation
- [ ] Implement `calculate_cohesion_score()` function
- [ ] Count internal calls (within recommended module)
- [ ] Count external calls (to other modules/structs)
- [ ] Handle edge case: module with no calls (cohesion = 1.0)
- [ ] Handle edge case: all calls are external (cohesion = 0.0)
- [ ] Unit tests for various cohesion scenarios
- [ ] Integration test on config.rs with expected cohesion ranges

### AC2: Dependency Tracking
- [ ] Extract function call information from call graph
- [ ] Map function calls to struct ownership
- [ ] Identify external dependencies for each recommended module
- [ ] Populate `dependencies_in` field (what this module uses)
- [ ] Populate `dependencies_out` field (what uses this module)
- [ ] Filter out stdlib and external crate dependencies
- [ ] Unit tests for dependency extraction

### AC3: Circular Dependency Detection
- [ ] Implement cycle detection algorithm (DFS-based or Tarjan's)
- [ ] Detect cycles in recommended module dependency graph
- [ ] Generate warnings for circular dependencies
- [ ] Suggest cycle-breaking strategies (which module to keep together)
- [ ] Unit tests for cycle detection (various graph structures)

### AC4: Quality-Based Priority
- [ ] Implement priority assignment based on cohesion
- [ ] High cohesion (>0.7) + no cycles → High priority
- [ ] Medium cohesion (0.5-0.7) → Medium priority
- [ ] Low cohesion (<0.5) → Low priority or filter out
- [ ] Modules in cycles downgraded by one level
- [ ] Unit tests for priority assignment logic

### AC5: Integration with Spec 143
- [ ] Extend `ModuleSplit` with cohesion fields (already defined in Spec 143)
- [ ] Integrate with existing struct ownership analyzer
- [ ] Update output formatter to show cohesion scores
- [ ] Maintain backward compatibility (cohesion optional)
- [ ] Integration test showing full pipeline

### AC6: config.rs Validation
- [ ] Run analysis on src/config.rs
- [ ] Verify cohesion scores are reasonable (>0.6 average)
- [ ] Verify dependency lists are accurate
- [ ] Verify no unexpected circular dependencies
- [ ] Compare with manual code review

## Technical Details

### Architecture

**Integration with Existing Call Graph**:
- Leverage `src/analyzers/rust_call_graph.rs`
- Extend call graph to track cross-struct calls
- Cache call graph results for performance

**New Components**:
- `src/organization/cohesion_calculator.rs` - Cohesion score calculation
- `src/organization/dependency_analyzer.rs` - Dependency extraction
- `src/organization/cycle_detector.rs` - Circular dependency detection

**Modified Components**:
- `src/organization/split_validator.rs` - Add cohesion-based validation
- `src/organization/god_object_analysis.rs` - Integrate call graph data
- `src/priority/formatter.rs` - Display cohesion scores

### Data Structures

```rust
// ModuleSplit extension (fields defined in Spec 143, populated here)
pub struct ModuleSplit {
    // ... existing fields from Spec 143 ...
    pub cohesion_score: Option<f64>,         // NOW POPULATED
    pub dependencies_in: Vec<String>,        // NOW POPULATED
    pub dependencies_out: Vec<String>,       // NOW POPULATED
}

// Call graph integration
pub struct ModuleCohesionAnalysis {
    pub module_name: String,
    pub internal_calls: Vec<FunctionCall>,
    pub external_calls: Vec<FunctionCall>,
    pub cohesion_score: f64,
}

#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub caller: String,        // function name
    pub caller_struct: String, // struct owning caller
    pub callee: String,        // function name
    pub callee_struct: String, // struct owning callee
}

// Dependency graph for cycle detection
pub struct ModuleDependencyGraph {
    pub nodes: Vec<String>,                    // module names
    pub edges: HashMap<String, Vec<String>>,   // module -> [dependencies]
}
```

### Algorithms

#### Cohesion Score Calculation

```rust
/// Calculate cohesion score for a module split recommendation
///
/// Cohesion measures how tightly related the methods within a module are.
/// High cohesion (>0.7) indicates methods work together frequently.
/// Low cohesion (<0.5) suggests the module might be poorly grouped.
///
/// Formula: cohesion = internal_calls / (internal_calls + external_calls)
///
/// # Arguments
/// * `split` - The module split recommendation
/// * `call_graph` - The function call graph for the file
/// * `ownership` - Struct ownership information
///
/// # Returns
/// Cohesion score between 0.0 (no cohesion) and 1.0 (perfect cohesion)
pub fn calculate_cohesion_score(
    split: &ModuleSplit,
    call_graph: &CallGraph,
    ownership: &StructOwnershipAnalyzer,
) -> f64 {
    let structs_in_module: HashSet<&str> = split.structs_to_move
        .iter()
        .map(|s| s.as_str())
        .collect();

    let mut internal_calls = 0;
    let mut external_calls = 0;

    // Iterate through all function calls in the call graph
    for call in call_graph.get_all_calls() {
        let caller_struct = ownership.get_struct_for_method(&call.caller);
        let callee_struct = ownership.get_struct_for_method(&call.callee);

        // Check if caller is in this module
        if let Some(caller_s) = caller_struct {
            if structs_in_module.contains(caller_s) {
                // Caller is in this module
                if let Some(callee_s) = callee_struct {
                    if structs_in_module.contains(callee_s) {
                        // Callee also in this module -> internal call
                        internal_calls += 1;
                    } else {
                        // Callee in different module -> external call
                        external_calls += 1;
                    }
                } else {
                    // Callee is standalone function or external
                    external_calls += 1;
                }
            }
        }
    }

    // Handle edge cases
    let total_calls = internal_calls + external_calls;
    if total_calls == 0 {
        // No calls - assume perfect cohesion (single-purpose module)
        return 1.0;
    }

    internal_calls as f64 / total_calls as f64
}
```

#### Dependency Extraction

```rust
/// Extract dependencies for a module split
///
/// Identifies which external modules/structs this module depends on
/// and which external modules depend on this module.
///
/// # Returns
/// Tuple of (dependencies_in, dependencies_out)
pub fn extract_dependencies(
    split: &ModuleSplit,
    call_graph: &CallGraph,
    ownership: &StructOwnershipAnalyzer,
    all_structs: &[String],
) -> (Vec<String>, Vec<String>) {
    let structs_in_module: HashSet<&str> = split.structs_to_move
        .iter()
        .map(|s| s.as_str())
        .collect();

    let mut dependencies_in = HashSet::new();   // What we depend on
    let mut dependencies_out = HashSet::new();  // What depends on us

    for call in call_graph.get_all_calls() {
        let caller_struct = ownership.get_struct_for_method(&call.caller);
        let callee_struct = ownership.get_struct_for_method(&call.callee);

        if let (Some(caller_s), Some(callee_s)) = (caller_struct, callee_struct) {
            let caller_in_module = structs_in_module.contains(caller_s);
            let callee_in_module = structs_in_module.contains(callee_s);

            if caller_in_module && !callee_in_module {
                // We call external struct
                if all_structs.contains(&callee_s.to_string()) {
                    dependencies_in.insert(callee_s.to_string());
                }
            } else if !caller_in_module && callee_in_module {
                // External struct calls us
                if all_structs.contains(&caller_s.to_string()) {
                    dependencies_out.insert(caller_s.to_string());
                }
            }
        }
    }

    (
        dependencies_in.into_iter().collect(),
        dependencies_out.into_iter().collect(),
    )
}
```

#### Circular Dependency Detection

```rust
/// Detect circular dependencies in module splits
///
/// Uses depth-first search to detect cycles in the module dependency graph.
///
/// # Returns
/// Vector of cycles, where each cycle is a vector of module names
pub fn detect_circular_dependencies(
    splits: &[ModuleSplit],
) -> Vec<Vec<String>> {
    // Build dependency graph
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    for split in splits {
        graph.insert(
            split.suggested_name.clone(),
            split.dependencies_in.clone(),
        );
    }

    // DFS-based cycle detection
    let mut cycles = Vec::new();
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut current_path = Vec::new();

    for node in graph.keys() {
        if !visited.contains(node) {
            dfs_find_cycles(
                node,
                &graph,
                &mut visited,
                &mut rec_stack,
                &mut current_path,
                &mut cycles,
            );
        }
    }

    cycles
}

fn dfs_find_cycles(
    node: &str,
    graph: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    rec_stack: &mut HashSet<String>,
    current_path: &mut Vec<String>,
    cycles: &mut Vec<Vec<String>>,
) {
    visited.insert(node.to_string());
    rec_stack.insert(node.to_string());
    current_path.push(node.to_string());

    if let Some(neighbors) = graph.get(node) {
        for neighbor in neighbors {
            if !visited.contains(neighbor) {
                dfs_find_cycles(neighbor, graph, visited, rec_stack, current_path, cycles);
            } else if rec_stack.contains(neighbor) {
                // Found a cycle
                let cycle_start = current_path.iter()
                    .position(|n| n == neighbor)
                    .unwrap();
                cycles.push(current_path[cycle_start..].to_vec());
            }
        }
    }

    current_path.pop();
    rec_stack.remove(node);
}
```

#### Priority Assignment

```rust
/// Assign priorities based on cohesion score and dependency quality
pub fn assign_cohesion_based_priority(
    splits: &mut [ModuleSplit],
    cycles: &[Vec<String>],
) {
    // Build set of modules involved in cycles
    let modules_in_cycles: HashSet<String> = cycles
        .iter()
        .flat_map(|cycle| cycle.iter().cloned())
        .collect();

    for split in splits.iter_mut() {
        let cohesion = split.cohesion_score.unwrap_or(0.5);
        let in_cycle = modules_in_cycles.contains(&split.suggested_name);

        // Determine priority based on cohesion and cycles
        split.priority = match (cohesion, in_cycle) {
            (c, true) if c > 0.7 => {
                // High cohesion but in cycle - downgrade to medium
                split.warning = Some(format!(
                    "High cohesion ({:.2}) but involved in circular dependency",
                    c
                ));
                Priority::Medium
            }
            (c, false) if c > 0.7 => {
                // High cohesion, no cycles - excellent candidate
                Priority::High
            }
            (c, _) if c > 0.5 => {
                // Medium cohesion
                if in_cycle {
                    split.warning = Some(format!(
                        "Moderate cohesion ({:.2}) and circular dependency",
                        c
                    ));
                }
                Priority::Medium
            }
            (c, _) => {
                // Low cohesion - questionable recommendation
                split.warning = Some(format!(
                    "Low cohesion ({:.2}) - may not improve organization",
                    c
                ));
                Priority::Low
            }
        };
    }
}
```

### Integration with Call Graph

**Leveraging Existing Infrastructure**:

```rust
// Use existing call graph from src/analyzers/rust_call_graph.rs
use crate::analyzers::rust_call_graph::RustCallGraph;

pub fn enhance_splits_with_call_graph(
    splits: Vec<ModuleSplit>,
    file_path: &Path,
    ast: &syn::File,
    ownership: &StructOwnershipAnalyzer,
) -> Vec<ModuleSplit> {
    // Build or retrieve call graph
    let call_graph = RustCallGraph::analyze(file_path, ast);

    // Get all struct names for dependency filtering
    let all_structs: Vec<String> = ownership.all_structs().map(|s| s.to_string()).collect();

    // Enhance each split with cohesion and dependencies
    let enhanced_splits: Vec<ModuleSplit> = splits
        .into_iter()
        .map(|mut split| {
            // Calculate cohesion
            split.cohesion_score = Some(calculate_cohesion_score(
                &split,
                &call_graph,
                ownership,
            ));

            // Extract dependencies
            let (deps_in, deps_out) = extract_dependencies(
                &split,
                &call_graph,
                ownership,
                &all_structs,
            );
            split.dependencies_in = deps_in;
            split.dependencies_out = deps_out;

            split
        })
        .collect();

    // Detect circular dependencies
    let cycles = detect_circular_dependencies(&enhanced_splits);

    // Assign priorities based on cohesion and cycles
    let mut final_splits = enhanced_splits;
    assign_cohesion_based_priority(&mut final_splits, &cycles);

    final_splits
}
```

## Testing Strategy

### Unit Tests

**Cohesion Calculation**:
```rust
#[test]
fn test_perfect_cohesion() {
    // Module where all calls are internal
    let split = create_test_split(vec!["StructA", "StructB"]);
    let call_graph = create_test_call_graph(vec![
        ("StructA::m1", "StructB::m2"),
        ("StructB::m2", "StructA::m3"),
    ]);
    let ownership = create_test_ownership();

    let cohesion = calculate_cohesion_score(&split, &call_graph, &ownership);
    assert_eq!(cohesion, 1.0);
}

#[test]
fn test_zero_cohesion() {
    // Module where all calls are external
    let split = create_test_split(vec!["StructA"]);
    let call_graph = create_test_call_graph(vec![
        ("StructA::m1", "StructB::m2"),  // StructB not in module
        ("StructA::m2", "StructC::m1"),  // StructC not in module
    ]);
    let ownership = create_test_ownership();

    let cohesion = calculate_cohesion_score(&split, &call_graph, &ownership);
    assert_eq!(cohesion, 0.0);
}

#[test]
fn test_mixed_cohesion() {
    // Module with 2 internal and 3 external calls
    let split = create_test_split(vec!["StructA", "StructB"]);
    let call_graph = create_test_call_graph(vec![
        ("StructA::m1", "StructB::m1"),  // Internal
        ("StructB::m1", "StructA::m2"),  // Internal
        ("StructA::m2", "StructC::m1"),  // External
        ("StructA::m3", "StructD::m1"),  // External
        ("StructB::m2", "StructE::m1"),  // External
    ]);
    let ownership = create_test_ownership();

    let cohesion = calculate_cohesion_score(&split, &call_graph, &ownership);
    assert_eq!(cohesion, 0.4); // 2 / 5
}

#[test]
fn test_no_calls_cohesion() {
    // Module with no function calls
    let split = create_test_split(vec!["StructA"]);
    let call_graph = create_test_call_graph(vec![]);
    let ownership = create_test_ownership();

    let cohesion = calculate_cohesion_score(&split, &call_graph, &ownership);
    assert_eq!(cohesion, 1.0); // Perfect cohesion by default
}
```

**Dependency Extraction**:
```rust
#[test]
fn test_dependency_extraction() {
    let split = create_test_split(vec!["StructA", "StructB"]);
    let call_graph = create_test_call_graph(vec![
        ("StructA::m1", "StructC::m1"),  // We depend on StructC
        ("StructD::m1", "StructB::m1"),  // StructD depends on us
    ]);
    let ownership = create_test_ownership();
    let all_structs = vec!["StructA", "StructB", "StructC", "StructD"]
        .into_iter().map(|s| s.to_string()).collect();

    let (deps_in, deps_out) = extract_dependencies(
        &split,
        &call_graph,
        &ownership,
        &all_structs,
    );

    assert_eq!(deps_in, vec!["StructC"]);
    assert_eq!(deps_out, vec!["StructD"]);
}
```

**Cycle Detection**:
```rust
#[test]
fn test_simple_cycle_detection() {
    let splits = vec![
        create_split_with_deps("ModuleA", vec!["ModuleB"], vec![]),
        create_split_with_deps("ModuleB", vec!["ModuleC"], vec![]),
        create_split_with_deps("ModuleC", vec!["ModuleA"], vec![]),
    ];

    let cycles = detect_circular_dependencies(&splits);

    assert_eq!(cycles.len(), 1);
    assert!(cycles[0].contains(&"ModuleA".to_string()));
    assert!(cycles[0].contains(&"ModuleB".to_string()));
    assert!(cycles[0].contains(&"ModuleC".to_string()));
}

#[test]
fn test_no_cycles() {
    let splits = vec![
        create_split_with_deps("ModuleA", vec!["ModuleB"], vec![]),
        create_split_with_deps("ModuleB", vec!["ModuleC"], vec![]),
        create_split_with_deps("ModuleC", vec![], vec![]),
    ];

    let cycles = detect_circular_dependencies(&splits);
    assert_eq!(cycles.len(), 0);
}
```

### Integration Tests

**config.rs with Cohesion Scoring**:
```rust
#[test]
fn test_config_rs_cohesion_scores() {
    let code = std::fs::read_to_string("src/config.rs").unwrap();
    let parsed = syn::parse_file(&code).unwrap();

    let detector = GodObjectDetector::with_source_content(&code);
    let analysis = detector.analyze_enhanced(Path::new("src/config.rs"), &parsed);

    let splits = match analysis.classification {
        GodObjectType::GodModule { suggested_splits, .. } => suggested_splits,
        _ => panic!("Expected GodModule"),
    };

    // All splits should have cohesion scores
    for split in &splits {
        assert!(split.cohesion_score.is_some(),
            "Split {} missing cohesion score", split.suggested_name);

        let cohesion = split.cohesion_score.unwrap();
        assert!(cohesion >= 0.0 && cohesion <= 1.0,
            "Invalid cohesion score: {}", cohesion);
    }

    // Average cohesion should be reasonable (>0.6)
    let avg_cohesion: f64 = splits.iter()
        .filter_map(|s| s.cohesion_score)
        .sum::<f64>() / splits.len() as f64;

    assert!(avg_cohesion > 0.6,
        "Average cohesion too low: {}", avg_cohesion);

    // High-priority splits should have higher cohesion than low-priority
    let high_priority_cohesion: f64 = splits.iter()
        .filter(|s| s.priority == Priority::High)
        .filter_map(|s| s.cohesion_score)
        .sum::<f64>() / splits.iter().filter(|s| s.priority == Priority::High).count() as f64;

    let low_priority_cohesion: f64 = splits.iter()
        .filter(|s| s.priority == Priority::Low)
        .filter_map(|s| s.cohesion_score)
        .sum::<f64>() / splits.iter().filter(|s| s.priority == Priority::Low).count().max(1) as f64;

    assert!(high_priority_cohesion > low_priority_cohesion,
        "High priority should have higher cohesion");
}
```

### Performance Tests

```rust
#[test]
fn test_call_graph_integration_performance() {
    let code = std::fs::read_to_string("src/config.rs").unwrap();
    let parsed = syn::parse_file(&code).unwrap();

    // Baseline (Spec 143 without cohesion)
    let baseline_start = Instant::now();
    let detector_baseline = GodObjectDetector::with_source_content(&code);
    let _analysis_baseline = detector_baseline.analyze_enhanced(
        Path::new("src/config.rs"),
        &parsed
    );
    let baseline_duration = baseline_start.elapsed();

    // With cohesion scoring (this spec)
    let cohesion_start = Instant::now();
    let detector_cohesion = GodObjectDetector::with_source_content(&code);
    let _analysis_cohesion = detector_cohesion.analyze_enhanced_with_cohesion(
        Path::new("src/config.rs"),
        &parsed
    );
    let cohesion_duration = cohesion_start.elapsed();

    // Should add <20% overhead
    let overhead_percent = ((cohesion_duration.as_millis() as f64
        / baseline_duration.as_millis() as f64) - 1.0) * 100.0;

    assert!(overhead_percent < 20.0,
        "Call graph integration adds {}% overhead (max 20%)", overhead_percent);
}
```

## Output Format

### Enhanced Display with Cohesion

```
RECOMMENDED REFACTORING STRATEGY:

Suggested Module Splits (7 modules):

├─ ⭐⭐⭐ config/scoring - scoring
│   → Structs: ScoringWeights, RoleMultipliers (2 structs)
│   → Methods: 18 functions (~360 lines)
│   → Cohesion: 0.85 (Excellent - high internal coupling)
│   → Dependencies: Uses thresholds::ValidationLimits
│
├─ ⭐⭐⭐ config/thresholds - thresholds
│   → Structs: ThresholdsConfig, ValidationLimits (2 structs)
│   → Methods: 14 functions (~280 lines)
│   → Cohesion: 0.78 (Good)
│   → Dependencies: Used by scoring, detection
│
├─ ⭐⭐ config/detection - detection
│   → Structs: PatternDetector, RuleChecker (2 structs)
│   → Methods: 22 functions (~440 lines)
│   → Cohesion: 0.62 (Moderate)
│   → Dependencies: Uses scoring, thresholds
│   ⚠️  22 methods is borderline - consider further splitting
│
├─ ⭐ config/utilities - utilities
│   → Structs: HelperUtils, StringFormatters (2 structs)
│   → Methods: 12 functions (~240 lines)
│   → Cohesion: 0.45 (Low - may not improve organization)
│   ⚠️  Low cohesion (0.45) - may not improve organization
```

## Success Metrics

### Quantitative Metrics

1. **Cohesion Quality**:
   - ✅ Average cohesion score >0.6 for all recommendations
   - ✅ High-priority modules have >0.7 cohesion
   - ✅ No modules with <0.3 cohesion recommended (filter out)

2. **Dependency Accuracy**:
   - ✅ Dependency lists match manual code review
   - ✅ <5% false positives in dependency detection
   - ✅ Circular dependencies correctly identified

3. **Performance**:
   - ✅ <20% increase in total analysis time
   - ✅ Call graph caching reduces repeated analysis

### Qualitative Metrics

1. **Priority Accuracy**:
   - High-priority modules have demonstrably better cohesion
   - Users agree with priority rankings
   - Circular dependency warnings are actionable

2. **Trust**:
   - Cohesion scores help users validate recommendations
   - Dependency information aids implementation planning

## Migration and Compatibility

### Backward Compatibility

- Cohesion scoring is optional (gracefully degrades if call graph unavailable)
- All Spec 143 functionality remains intact
- Output format remains readable without cohesion scores

### Rollout Strategy

1. **v0.3.1**: Introduce call graph integration (this spec)
   - Feature flag: `enable_cohesion_scoring` (default: true)
   - Can be disabled for performance comparison

2. **v0.3.2**: Make cohesion scoring default, remove feature flag

## Implementation Plan

### Week 1: Cohesion Calculation
- [ ] Implement `calculate_cohesion_score()`
- [ ] Unit tests for cohesion calculation
- [ ] Integration with existing call graph

### Week 2: Dependency Analysis
- [ ] Implement dependency extraction
- [ ] Implement cycle detection
- [ ] Update priority assignment
- [ ] Integration tests on config.rs

### Week 3: Polish & Performance
- [ ] Performance optimization
- [ ] Output formatter updates
- [ ] Documentation
- [ ] Final integration testing

## Related Specifications

- **Spec 143**: Struct Ownership Foundation (prerequisite)
- **Spec 145**: Multi-Language Support (follows this spec)

## Notes

- Call graph integration assumes existing `RustCallGraph` infrastructure
- Performance may vary based on call graph complexity
- Cohesion scores should be validated against manual code review for config.rs
- Consider caching call graph results for incremental analysis
