---
number: 14
title: Dependency-Aware ROI Calculation
category: optimization
priority: high
status: draft
dependencies: [12]
created: 2025-01-10
---

# Specification 14: Dependency-Aware ROI Calculation

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [12 - Improve ROI Calculation]

## Context

The current ROI calculation produces uniform results for functions with similar complexity and coverage characteristics, failing to account for the cascade effects of testing highly-depended-upon code. Functions that are used by many other modules have a higher impact when tested, as their reliability affects the entire dependency chain.

Currently, all simple untested functions show identical ROI values (10.0) and risk reduction (40%), making it impossible to prioritize between them effectively. The cascade calculator exists but receives an empty dependency graph, resulting in zero cascade impact for all functions.

## Objective

Enhance the ROI calculation to incorporate dependency information, creating meaningful variation in ROI scores based on how many other modules depend on each function. This will provide more nuanced prioritization that reflects real-world impact.

## Requirements

### Functional Requirements
- Build dependency graph from existing TestTarget data (dependencies and dependents fields)
- Pass populated dependency graph to ROI calculator
- Calculate cascade impact based on number and importance of dependents
- Vary ROI scores based on both direct impact and cascade effects
- Maintain backward compatibility with existing ROI ranges (0.1 to 10.0)

### Non-Functional Requirements
- Performance: Dependency graph construction should not significantly impact analysis time
- Accuracy: Cascade calculations should reflect realistic risk propagation
- Clarity: ROI variations should be explainable and intuitive

## Acceptance Criteria

- [ ] Dependency graph is properly constructed from TestTarget data
- [ ] Functions with more dependents show higher ROI values
- [ ] Cascade impact is reflected in the risk reduction percentages
- [ ] ROI values show meaningful variation (not all 10.0 for similar functions)
- [ ] Entry point and core modules receive appropriate cascade bonuses
- [ ] The rationale strings explain cascade effects when present
- [ ] Performance impact is less than 10% on analysis time
- [ ] All existing tests continue to pass

## Technical Details

### Implementation Approach

1. **Dependency Graph Construction**
   - Convert TestTarget's `dependents` Vec<String> into graph nodes and edges
   - Create DependencyNode for each unique function/module
   - Build edges representing "depends on" relationships
   - Calculate transitive dependencies for cascade depth

2. **Enhanced Cascade Calculation**
   - Use actual dependency count instead of empty graph
   - Apply module type weights (Core/EntryPoint get higher multipliers)
   - Implement decay factor for indirect dependencies
   - Cap maximum cascade bonus to maintain reasonable ROI ranges

3. **ROI Formula Adjustments**
   - Current: `total_impact = direct_impact + cascade_impact * 0.5`
   - Enhanced: Include dependent count and module criticality
   - Ensure total ROI stays within 0.1 to 10.0 bounds

### Architecture Changes

Modify `prioritize_by_roi` function in `src/risk/priority.rs`:
```rust
// Build dependency graph from available data
let dependency_graph = build_dependency_graph(&prioritized);

let context = crate::risk::roi::Context {
    dependency_graph,
    critical_paths: identify_critical_paths(&prioritized),
    historical_data: None,
};
```

### Data Structures

```rust
fn build_dependency_graph(targets: &[TestTarget]) -> DependencyGraph {
    let mut nodes = HashMap::new();
    let mut edges = Vector::new();
    
    // Create nodes for each target
    for target in targets {
        let node = DependencyNode {
            id: target.id.clone(),
            path: target.path.clone(),
            risk: target.current_risk,
            complexity: target.complexity.clone(),
        };
        nodes.insert(target.id.clone(), node);
    }
    
    // Create edges based on dependents
    for target in targets {
        for dependent in &target.dependents {
            edges.push_back(DependencyEdge {
                from: dependent.clone(),
                to: target.id.clone(),
                weight: calculate_edge_weight(target),
            });
        }
    }
    
    DependencyGraph { nodes, edges }
}
```

### APIs and Interfaces

No public API changes required. The enhancement is internal to the ROI calculation.

## Dependencies

- **Prerequisites**: Spec 12 (Improve ROI Calculation) must be completed
- **Affected Components**: 
  - `src/risk/priority.rs` - Dependency graph construction
  - `src/risk/roi/cascade.rs` - Enhanced cascade calculation
  - `src/risk/roi/mod.rs` - Context usage
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: 
  - Test dependency graph construction with various input scenarios
  - Verify cascade calculations with different dependency depths
  - Validate ROI ranges remain within bounds
  
- **Integration Tests**:
  - Test full ROI calculation with real dependency data
  - Verify sorting order reflects dependency importance
  - Check that rationale strings include cascade information

- **Performance Tests**:
  - Measure impact on analysis time with large codebases
  - Profile dependency graph construction overhead

- **User Acceptance**:
  - ROI values should show clear differentiation
  - High-dependency functions should rank higher
  - Recommendations should feel intuitive to developers

## Documentation Requirements

- **Code Documentation**: 
  - Document graph construction algorithm
  - Explain cascade weight calculation
  - Add examples of dependency impact

- **User Documentation**:
  - Update README to explain dependency-aware prioritization
  - Add examples showing ROI variation based on dependencies

- **Architecture Updates**:
  - Update ARCHITECTURE.md with dependency graph usage
  - Document cascade calculation strategy

## Implementation Notes

### Edge Weight Calculation
Edge weights should consider:
- Module type of the dependent (critical modules = higher weight)
- Complexity of the dependency relationship
- Whether it's a direct or transitive dependency

### Cascade Decay
Implement exponential decay for indirect dependencies:
- Direct dependents: 100% impact
- 2nd level: 70% impact  
- 3rd level: 49% impact
- Cap at 3 levels to avoid over-propagation

### Module Type Bonuses
```rust
match module_type {
    ModuleType::EntryPoint => 2.0,  // Highest multiplier
    ModuleType::Core => 1.5,        // High multiplier
    ModuleType::Api => 1.2,         // Moderate multiplier
    ModuleType::Model => 1.1,       // Slight bonus
    _ => 1.0,                       // No bonus
}
```

## Migration and Compatibility

No breaking changes. The enhancement is backward compatible:
- Existing CLI commands work unchanged
- Output format remains the same
- Only the ROI values and rankings change (improvement, not breaking)