---
number: 178
title: Behavioral Decomposition for God Object Recommendations
category: optimization
priority: high
status: draft
dependencies: [133]
created: 2025-11-18
---

# Specification 178: Behavioral Decomposition for God Object Recommendations

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [133 - God Object Detection Refinement]

## Context

Current debtmap analysis for the Zed codebase revealed that god object recommendations focus primarily on **struct-based decomposition** (grouping supporting data structures) rather than **behavioral decomposition** (extracting cohesive method groups).

**Problem Example** (from Zed editor.rs analysis):
```
RECOMMENDED SPLITS (2 modules):
  - config/misc.rs - misc (0 methods, ~423 lines) [Medium]
     -> Structs: ClipboardSelection, CompletionEdit, ... (45 structs)
  - config/core_config.rs - core_config (0 methods, ~12 lines) [Medium]
     -> Structs: RowHighlightOptions, RewrapOptions, ... (3 structs)
```

**Issues**:
1. Recommends moving 45 structs to "misc.rs" - still a god object!
2. Shows "0 methods" for each module - doesn't address the real problem
3. Doesn't identify which of the 675 Editor impl methods should move
4. "misc" is an anti-pattern category name
5. No guidance on extracting behavioral cohesion from the Editor impl

**Reality**: The Editor struct has 675 methods in its impl block - this is the primary source of complexity, not the 45 supporting structs.

## Objective

Shift god object refactoring recommendations from struct-based organization to **behavioral decomposition**, focusing on extracting cohesive groups of methods into trait implementations or separate service structs.

## Requirements

### Functional Requirements

1. **Method Clustering Analysis**
   - Analyze method call patterns within god object impls
   - Group methods by shared data access patterns
   - Identify methods that form behavioral cohesion units
   - Detect methods that operate on subsets of struct fields
   - Use heuristics: methods that call each other frequently belong together

2. **Behavioral Category Detection**
   - Identify common behavioral patterns in god objects:
     - **Lifecycle methods**: `new()`, `init()`, `shutdown()`, `dispose()`
     - **State management**: getters, setters, state transitions
     - **Rendering/Display**: `render()`, `draw()`, `format()`, `display()`
     - **Event handling**: `handle_*()`, `on_*()`, event dispatchers
     - **Persistence**: `save()`, `load()`, `serialize()`, `deserialize()`
     - **Validation**: `validate_*()`, `check_*()`, `verify_()`
     - **Computation**: Pure calculation methods with no state mutation
   - Categorize methods based on naming conventions and signatures
   - Analyze method purpose from parameters and return types

3. **Trait Extraction Recommendations**
   - Suggest extracting behavioral groups into traits
   - Identify which fields would be needed by each trait impl
   - Recommend trait names based on behavioral category
   - Show example trait signatures with 3-5 representative methods

4. **Service Object Recommendations**
   - Detect methods that could be extracted to service objects
   - Identify methods with minimal field dependencies
   - Suggest service struct names and responsibilities
   - Show which methods would move and what data they need

5. **Method-First Split Recommendations**
   - For each recommended module, show:
     - **Primary methods** being extracted (not structs)
     - Estimated method count (e.g., "~45 methods")
     - Representative method names (show top 5-8)
     - Which fields from original struct are needed
     - Supporting structs (secondary to methods)
   - Avoid "misc" categories - require specific behavioral names

### Non-Functional Requirements

1. **Actionability**: Each recommendation must specify concrete methods to extract
2. **Clarity**: Use behavioral names (e.g., "rendering", "event_handling") not generic names ("misc", "utilities")
3. **Realism**: Don't suggest extracting more than 20 methods at once
4. **Rust-Specific**: Recommend traits, newtypes, or builder patterns where appropriate
5. **Incremental**: Suggest extraction order based on coupling (low coupling first)

## Acceptance Criteria

- [ ] God object recommendations show method extraction first, struct grouping second
- [ ] Each recommended module shows estimated method count and representative method names
- [ ] Behavioral categories used instead of "misc" or "utilities"
- [ ] Trait extraction suggestions provided for cohesive method groups
- [ ] Service object extraction suggested for low-coupling methods
- [ ] Recommendations show which struct fields each extracted module needs
- [ ] Example refactoring pseudo-code shown for top recommendation
- [ ] "misc" category eliminated from all recommendations
- [ ] Recommendations prioritize extracting 10-20 methods at a time
- [ ] When run on Zed editor.rs:
  - Shows specific Editor impl methods to extract
  - Suggests trait-based decomposition
  - Identifies rendering methods as cohesive group
  - Identifies event handling methods as cohesive group
  - Shows field dependencies for each behavioral group

## Technical Details

### Implementation Approach

1. **Method Call Graph Analysis**
   ```rust
   struct MethodCluster {
       category: BehaviorCategory,
       methods: Vec<String>,
       fields_accessed: Vec<String>,
       internal_calls: usize,  // Calls within cluster
       external_calls: usize,  // Calls outside cluster
       cohesion_score: f64,    // High = good extraction candidate
   }

   enum BehaviorCategory {
       Lifecycle,
       StateManagement,
       Rendering,
       EventHandling,
       Persistence,
       Validation,
       Computation,
       Domain(String),  // Custom domain-specific
   }
   ```

2. **Field Dependency Analysis**
   - Track which fields each method accesses
   - Identify field access patterns across methods
   - Calculate field coupling for each method cluster
   - Suggest minimal field sets for extracted traits/services

3. **Recommendation Generation**
   - Prioritize clusters with high internal cohesion
   - Prefer clusters with low field dependencies
   - Generate trait signatures for top 3 clusters
   - Show example service struct extraction

4. **Output Format Enhancement**
   ```
   RECOMMENDED SPLITS (5 modules):

   - editor_rendering.rs - Rendering (42 methods, ~840 lines) [High Priority]
      -> Methods: render(), paint_highlighted_ranges(), draw_cursor(),
                  render_gutter(), paint_background(), ... +37 more
      -> Fields needed: display_map, style, scroll_manager (3 fields)
      -> Trait extraction: impl Render for Editor { ... }
      -> Impact: Reduces Editor impl from 675 to 633 methods

   - editor_events.rs - Event Handling (35 methods, ~700 lines) [High Priority]
      -> Methods: handle_keypress(), on_mouse_down(), on_scroll(),
                  handle_input_event(), dispatch_action(), ... +30 more
      -> Fields needed: focus_handle, event_handlers, buffer (3 fields)
      -> Trait extraction: impl EventHandler for Editor { ... }
      -> Impact: Reduces Editor impl from 633 to 598 methods
   ```

### Method Clustering Algorithm

1. **Build method call adjacency matrix**
2. **Apply community detection algorithm** (e.g., Louvain method)
3. **Score each cluster by cohesion**: `cohesion = internal_calls / (internal_calls + external_calls)`
4. **Filter clusters with cohesion > 0.6 and size 10-50 methods**
5. **Classify cluster by behavior category** using heuristics
6. **Generate extraction recommendations** for top 5 clusters

### Field Access Analysis

```rust
struct FieldAccessPattern {
    field_name: String,
    accessed_by_methods: Vec<String>,
    access_frequency: usize,
    field_type: String,
}

impl FieldAccessPattern {
    fn is_core_dependency(&self, total_methods: usize) -> bool {
        self.accessed_by_methods.len() as f64 / total_methods as f64 > 0.5
    }

    fn is_cluster_specific(&self, cluster_methods: &[String]) -> bool {
        let cluster_accesses = cluster_methods.iter()
            .filter(|m| self.accessed_by_methods.contains(m))
            .count();

        cluster_accesses as f64 / self.accessed_by_methods.len() as f64 > 0.8
    }
}
```

## Dependencies

- **Prerequisites**:
  - [133] God object detection refinement
  - Existing call graph analysis infrastructure
- **Affected Components**:
  - `god_object_detector.rs` - Add behavioral analysis
  - `recommendations/god_object.rs` - Refactor recommendation format
  - Output formatting for god object recommendations
- **External Dependencies**:
  - Consider community detection algorithm crate (e.g., `petgraph` with clustering)

## Testing Strategy

### Unit Tests

1. **Method clustering tests**:
   - Test with synthetic god object (100 methods, 5 behavioral groups)
   - Verify clustering correctly groups related methods
   - Test edge case: all methods independent (no clusters)
   - Test edge case: all methods tightly coupled (one cluster)

2. **Behavioral categorization tests**:
   - Test lifecycle method detection (`new`, `init`, `destroy`)
   - Test rendering method detection (`render_*`, `draw_*`, `paint_*`)
   - Test event handler detection (`handle_*`, `on_*`)
   - Test validation method detection (`validate_*`, `check_*`)

3. **Field dependency analysis tests**:
   - Test field access pattern extraction
   - Test core dependency identification
   - Test cluster-specific field detection
   - Test minimal field set calculation

### Integration Tests

1. **Real-world test cases**:
   - Analyze Zed's editor.rs (675 methods, 152 fields)
   - Analyze Zed's workspace.rs (191 methods, 49 fields)
   - Verify behavioral categories are detected
   - Verify method extraction recommendations are actionable

2. **Output format tests**:
   - Verify "misc" category is never generated
   - Verify method counts are shown for each recommended module
   - Verify representative method names are displayed
   - Verify field dependencies are listed
   - Verify trait extraction suggestions are valid Rust

3. **Regression tests**:
   - Ensure files without god objects are unchanged
   - Ensure god module recommendations still work
   - Ensure output format is backwards compatible

### Validation Metrics

- **Cohesion score**: Clusters should average >0.6 internal cohesion
- **Cluster size**: Recommend 10-50 methods per cluster (sweet spot: 15-25)
- **Field coupling**: Recommended extractions should need <30% of original fields
- **Actionability**: Human reviewers can understand recommendations without code inspection

## Documentation Requirements

### Code Documentation

1. Document clustering algorithm and cohesion scoring
2. Document behavioral category heuristics and rationale
3. Document field dependency analysis approach
4. Add examples of good vs bad method clusters

### User Documentation

1. Update README with examples of behavioral decomposition recommendations
2. Add section explaining method clustering approach
3. Show before/after examples of refactoring based on recommendations
4. Explain trait extraction suggestions for Rust codebases

### Architecture Updates

1. Document method clustering subsystem in ARCHITECTURE.md
2. Explain integration with existing god object detection
3. Document behavioral category taxonomy
4. Add flowchart of recommendation generation process

## Implementation Notes

### Heuristics for Behavioral Categories

**Lifecycle**:
- Method names: `new`, `create`, `init`, `initialize`, `setup`, `destroy`, `cleanup`, `dispose`, `shutdown`, `close`
- Patterns: Constructor patterns, initialization, teardown

**Rendering/Display**:
- Method names: `render`, `draw`, `paint`, `display`, `show`, `present`, `format_*`, `to_string`
- Return types: UI element types, strings, display buffers
- Parameters: Graphics contexts, display configurations

**Event Handling**:
- Method names: `handle_*`, `on_*`, `process_event`, `dispatch`, `trigger`
- Parameters: Event types, input data
- Patterns: Observer pattern, event dispatch

**State Management**:
- Method names: `get_*`, `set_*`, `update_*`, `mutate_*`, `state_*`
- Patterns: Getters, setters, state transitions

**Persistence**:
- Method names: `save`, `load`, `persist`, `restore`, `serialize`, `deserialize`, `write_*`, `read_*`
- Parameters: File paths, streams, serializers

**Validation**:
- Method names: `validate_*`, `check_*`, `verify_*`, `is_valid`, `ensure_*`
- Return types: `Result<(), Error>`, `bool`, `ValidationResult`

**Computation** (extraction priority):
- Pure functions with no field mutations
- Methods with no `&mut self` receiver
- Methods with deterministic outputs
- Mathematical or algorithmic methods

### Rust-Specific Recommendations

1. **Trait extraction**: When cluster has 5+ cohesive methods, suggest trait
2. **Newtype pattern**: For methods operating on single field, suggest newtype
3. **Builder pattern**: For construction/initialization methods
4. **Extension traits**: For adding behavior without modifying struct
5. **Service objects**: For stateless methods with minimal field dependencies

### Incremental Refactoring Guidance

Recommend extraction order:
1. **First**: Pure computation methods (lowest coupling)
2. **Second**: Rendering/display methods (often cohesive)
3. **Third**: Event handling (well-defined boundaries)
4. **Fourth**: Persistence/serialization (IO boundary)
5. **Last**: Core state management (highest coupling)

## Migration and Compatibility

### Breaking Changes

- None - this enhances recommendations without changing detection or scoring

### Backwards Compatibility

- Maintain existing god object detection type (GodClass vs GodFile)
- Keep existing JSON output format
- Add new fields for behavioral analysis (optional)
- Existing tools can ignore new recommendation details

### Gradual Rollout

1. **Phase 1**: Implement method clustering and behavioral categorization
2. **Phase 2**: Enhance recommendation output with method-first approach
3. **Phase 3**: Add trait extraction suggestions
4. **Phase 4**: Add service object extraction suggestions
5. **Phase 5**: Eliminate "misc" category entirely (require behavioral names)

## Success Metrics

- Recommendations show specific methods to extract, not just structs
- "misc" category usage drops to 0%
- Method cluster cohesion averages >0.65
- User feedback indicates recommendations are more actionable
- Trait extraction suggestions are valid and compilable
- Field dependency analysis reduces coupling in suggested extractions

## Related Work

- **Martin Fowler's Refactoring**: Extract Class, Extract Interface patterns
- **Community Detection Algorithms**: Louvain, Girvan-Newman for method clustering
- **Software Clustering Research**: Using call graphs for module identification
- **Rust API Guidelines**: Trait design and composition patterns
