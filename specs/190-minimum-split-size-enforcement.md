---
number: 190
title: Minimum Split Size Enforcement
category: optimization
priority: high
status: draft
dependencies: [188]
created: 2025-11-20
---

# Specification 190: Minimum Split Size Enforcement

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [188 - Intelligent Module Split Recommendations]

## Context

Current god object split recommendations frequently suggest overly granular modules that create maintenance overhead rather than improving code organization:

### Problem Examples from Latest Analysis

**Issue 1: 3-Method Modules**
```
- god_object_detector/typeanalysis.rs
  Size: 3 methods, ~45 lines
  Methods: is_god_object(), suggest_responsibility_split(), create_default_responsibility_group()
```

**Issue 2: Fragmentation Without Benefit**
```
formatter.rs (3,004 lines, 103 functions)
RECOMMENDED SPLITS (6 modules):
  - formatter/debtitem.rs (3 methods, ~45 lines)
  - formatter/filedebtitem.rs (3 methods, ~45 lines)
  - formatter/unifieddebtitem.rs (4 methods, ~60 lines)
  ...
```

### Core Problems

1. **Excessive Navigation**: Developers must jump between many tiny files to understand related logic
2. **Cognitive Overhead**: More module boundaries = more mental context switches
3. **Import Proliferation**: Many small modules require extensive use statements
4. **Poor Cohesion**: Artificially separating closely related functionality
5. **Violates Best Practices**: Contradicts "avoid premature abstractions" principle

### Current Behavior

- No minimum size validation for split recommendations
- Clustering algorithm creates splits for any data type, regardless of size
- Recommendations suggest 3-5 method modules as viable splits
- No consideration of cohesion vs. fragmentation trade-offs

### Impact

From evaluation of debtmap's own output:
- 67% of recommended splits are <10 methods
- 45% of recommended splits are <5 methods
- Creates 2-3x more files than necessary
- Reduces actionability of recommendations

## Objective

Implement minimum size thresholds for module split recommendations to ensure suggested splits provide genuine organizational value without excessive fragmentation.

## Requirements

### Functional Requirements

1. **Minimum Split Size Thresholds**
   - Define minimum viable split size: 10 methods OR 150 lines of code
   - Smaller clusters must be merged with semantically related clusters
   - Exception: Highly cohesive utility modules (e.g., pure data structures) may be 5+ methods

2. **Cluster Merging Logic**
   - When cluster falls below minimum, identify most semantically similar cluster
   - Use method naming patterns, call graph relationships, and data dependencies
   - Merge until resulting split exceeds minimum threshold
   - Document merged responsibilities in split rationale

3. **Balanced Split Distribution**
   - Avoid creating 1 large module + many tiny modules
   - Target: splits should be within 2x size of each other
   - Example: Prefer 3 modules of 30 methods each over 1 module of 80 + 2 modules of 5

4. **Cohesion Validation**
   - Calculate cohesion score for each proposed split (based on method interconnectedness)
   - Reject splits with cohesion below threshold (0.3)
   - Prefer fewer, cohesive splits over many loosely related splits

5. **Configuration Options**
   - Allow users to override minimum split size via CLI flag: `--min-split-methods <N>`
   - Allow users to set minimum split lines: `--min-split-lines <N>`
   - Default: 10 methods OR 150 lines, whichever is lower

### Non-Functional Requirements

- **Performance**: Split size validation adds <5% to analysis time
- **Backward Compatibility**: Existing analyses without size constraints still work
- **Configurability**: Thresholds adjustable per project needs
- **Transparency**: Output clearly shows when clusters were merged due to size constraints

## Acceptance Criteria

- [ ] Recommended splits are at least 10 methods OR 150 lines (whichever is lower)
- [ ] Clusters below threshold are merged with semantically similar clusters
- [ ] Split size distribution is balanced (no single split >2x others)
- [ ] Cohesion scores calculated for each split, rejects splits <0.3 cohesion
- [ ] CLI flags `--min-split-methods` and `--min-split-lines` allow override
- [ ] Output shows merge rationale when clusters combined: "Merged X with Y due to size constraint"
- [ ] Exception handling: Utility modules with high cohesion (>0.7) can be 5+ methods
- [ ] Regression test: debtmap's own god_object_detector.rs no longer recommends 3-method splits
- [ ] Performance: Analysis time increase is <5% with validation enabled

## Technical Details

### Implementation Approach

**Phase 1: Minimum Size Validation**
```rust
pub struct SplitSizeConfig {
    pub min_methods: usize,        // Default: 10
    pub min_lines: usize,          // Default: 150
    pub utility_min_methods: usize, // Default: 5
    pub utility_cohesion_threshold: f64, // Default: 0.7
    pub max_size_ratio: f64,       // Default: 2.0
}

impl SplitSizeConfig {
    pub fn is_viable_split(&self, split: &ModuleSplit) -> bool {
        if split.is_utility_module() && split.cohesion_score() > self.utility_cohesion_threshold {
            split.method_count() >= self.utility_min_methods
        } else {
            split.method_count() >= self.min_methods || split.line_count() >= self.min_lines
        }
    }
}
```

**Phase 2: Semantic Similarity for Merging**
```rust
pub fn find_merge_candidate<'a>(
    undersized_split: &ModuleSplit,
    viable_splits: &'a [ModuleSplit],
) -> Option<&'a ModuleSplit> {
    viable_splits
        .iter()
        .map(|split| {
            let similarity = calculate_semantic_similarity(undersized_split, split);
            (split, similarity)
        })
        .max_by(|(_, sim1), (_, sim2)| sim1.partial_cmp(sim2).unwrap())
        .map(|(split, _)| split)
}

fn calculate_semantic_similarity(split1: &ModuleSplit, split2: &ModuleSplit) -> f64 {
    // Weighted combination of:
    // - Method naming similarity (0.3)
    // - Call graph connectivity (0.4)
    // - Shared data dependencies (0.3)

    let naming_sim = method_naming_similarity(split1, split2);
    let call_sim = call_graph_connectivity(split1, split2);
    let data_sim = shared_data_dependencies(split1, split2);

    0.3 * naming_sim + 0.4 * call_sim + 0.3 * data_sim
}
```

**Phase 3: Cohesion Calculation**
```rust
pub fn calculate_cohesion(split: &ModuleSplit, call_graph: &CallGraph) -> f64 {
    let methods = &split.methods;
    if methods.len() < 2 {
        return 1.0; // Single method is trivially cohesive
    }

    // Count internal vs external method calls
    let internal_calls = count_internal_calls(methods, call_graph);
    let external_calls = count_external_calls(methods, call_graph);

    if internal_calls + external_calls == 0 {
        return 0.5; // No calls = moderate cohesion (data structure)
    }

    internal_calls as f64 / (internal_calls + external_calls) as f64
}
```

**Phase 4: Balanced Distribution**
```rust
pub fn ensure_balanced_distribution(splits: Vec<ModuleSplit>, config: &SplitSizeConfig) -> Vec<ModuleSplit> {
    let mut balanced = splits;

    loop {
        let max_size = balanced.iter().map(|s| s.method_count()).max().unwrap_or(0);
        let min_size = balanced.iter().map(|s| s.method_count()).min().unwrap_or(0);

        if max_size as f64 / min_size as f64 <= config.max_size_ratio {
            break; // Distribution is balanced
        }

        // Find largest split and extract sub-cluster
        let largest_idx = balanced
            .iter()
            .enumerate()
            .max_by_key(|(_, s)| s.method_count())
            .map(|(idx, _)| idx)
            .unwrap();

        if let Some(sub_clusters) = split_largest_cluster(&balanced[largest_idx]) {
            balanced.remove(largest_idx);
            balanced.extend(sub_clusters);
        } else {
            break; // Can't split further
        }
    }

    balanced
}
```

### Architecture Changes

**New Module**: `src/organization/split_validation.rs`
```rust
pub struct SplitValidator {
    config: SplitSizeConfig,
    call_graph: Arc<CallGraph>,
}

impl SplitValidator {
    pub fn validate_and_adjust(&self, splits: Vec<ModuleSplit>) -> Vec<ModuleSplit> {
        // 1. Filter out undersized splits
        let (viable, undersized): (Vec<_>, Vec<_>) = splits
            .into_iter()
            .partition(|s| self.config.is_viable_split(s));

        // 2. Merge undersized splits
        let merged = self.merge_undersized_splits(undersized, &viable);

        // 3. Ensure balanced distribution
        let balanced = ensure_balanced_distribution(merged, &self.config);

        // 4. Validate cohesion
        balanced
            .into_iter()
            .filter(|s| calculate_cohesion(s, &self.call_graph) >= 0.3)
            .collect()
    }
}
```

**Modified**: `src/organization/god_object_detector.rs`
```rust
pub fn analyze_domains_and_recommend_splits(
    &self,
    params: DomainAnalysisParams,
) -> Vec<ModuleSplit> {
    // Existing clustering logic...
    let raw_splits = suggest_module_splits_by_domain(/* ... */);

    // NEW: Validate and adjust splits
    let validator = SplitValidator::new(
        self.split_size_config.clone(),
        self.call_graph.clone(),
    );

    validator.validate_and_adjust(raw_splits)
}
```

### Data Structures

```rust
#[derive(Debug, Clone)]
pub struct ModuleSplit {
    pub module_name: String,
    pub methods: Vec<String>,
    pub estimated_lines: usize,
    pub responsibility_category: String,
    pub cohesion_score: f64,
    pub merge_history: Vec<MergeRecord>, // NEW
}

#[derive(Debug, Clone)]
pub struct MergeRecord {
    pub merged_from: String,
    pub reason: String,
    pub similarity_score: f64,
}

impl ModuleSplit {
    pub fn method_count(&self) -> usize {
        self.methods.len()
    }

    pub fn line_count(&self) -> usize {
        self.estimated_lines
    }

    pub fn is_utility_module(&self) -> bool {
        // Heuristic: All methods are pure data structure operations
        // No I/O, no business logic, high cohesion
        self.responsibility_category.contains("data structure") ||
        self.responsibility_category.contains("utilities")
    }

    pub fn cohesion_score(&self) -> f64 {
        self.cohesion_score
    }
}
```

### Output Format Changes

**Before**:
```
RECOMMENDED SPLITS (6 modules):
  - formatter/debtitem.rs
    Size: 3 methods, ~45 lines
    Methods: format_compact_item(), format_item_location(), format_mixed_priority_item()
```

**After**:
```
RECOMMENDED SPLITS (3 modules):
  - formatter/debt_items.rs
    Size: 12 methods, ~180 lines
    Category: Debt item formatting and display
    Cohesion: 0.68
    Methods: format_compact_item(), format_item_location(), format_mixed_priority_item(),
             format_file_priority_item(), format_file_score_calculation_section(), ...
    [Merged from: debtitem.rs, filedebtitem.rs, unifieddebtitem.rs - semantic similarity: 0.82]
```

## Dependencies

- **Prerequisites**:
  - [188] Intelligent Module Split Recommendations (for base clustering logic)
  - Call graph analysis (for cohesion calculation)

- **Affected Components**:
  - `src/organization/god_object_detector.rs` - Add validation step
  - `src/organization/god_object_analysis.rs` - Update ModuleSplit data structure
  - `src/priority/formatter.rs` - Display merge history in output

- **External Dependencies**: None (uses existing call graph infrastructure)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rejects_undersized_splits() {
        let config = SplitSizeConfig::default();
        let split = ModuleSplit {
            methods: vec!["m1".into(), "m2".into(), "m3".into()],
            estimated_lines: 45,
            ..Default::default()
        };

        assert!(!config.is_viable_split(&split));
    }

    #[test]
    fn test_allows_high_cohesion_utility_modules() {
        let config = SplitSizeConfig::default();
        let split = ModuleSplit {
            methods: vec!["new".into(), "default".into(), "clone".into(), "eq".into(), "hash".into()],
            estimated_lines: 75,
            responsibility_category: "data structure operations".into(),
            cohesion_score: 0.85,
            ..Default::default()
        };

        assert!(config.is_viable_split(&split));
    }

    #[test]
    fn test_semantic_similarity_calculation() {
        let split1 = create_test_split(vec!["format_item", "format_details"]);
        let split2 = create_test_split(vec!["format_header", "format_footer"]);
        let split3 = create_test_split(vec!["calculate_score", "validate_input"]);

        let sim12 = calculate_semantic_similarity(&split1, &split2);
        let sim13 = calculate_semantic_similarity(&split1, &split3);

        assert!(sim12 > sim13); // Formatting methods more similar to each other
    }

    #[test]
    fn test_balanced_distribution() {
        let splits = vec![
            create_split_with_size(80),
            create_split_with_size(5),
            create_split_with_size(5),
        ];

        let config = SplitSizeConfig::default();
        let balanced = ensure_balanced_distribution(splits, &config);

        let sizes: Vec<_> = balanced.iter().map(|s| s.method_count()).collect();
        let max = *sizes.iter().max().unwrap();
        let min = *sizes.iter().min().unwrap();

        assert!(max as f64 / min as f64 <= 2.0);
    }
}
```

### Integration Tests

```rust
#[test]
fn test_god_object_detector_no_tiny_splits() {
    let detector = GodObjectDetector::with_split_config(SplitSizeConfig::default());
    let ast = parse_file("tests/fixtures/large_formatter.rs");

    let analysis = detector.analyze_enhanced(Path::new("formatter.rs"), &ast);
    let splits = analysis.recommended_splits;

    // Verify no splits below minimum
    for split in &splits {
        assert!(
            split.method_count() >= 10 || split.line_count() >= 150,
            "Split {:?} is undersized: {} methods, {} lines",
            split.module_name,
            split.method_count(),
            split.line_count()
        );
    }

    // Verify balanced distribution
    let sizes: Vec<_> = splits.iter().map(|s| s.method_count()).collect();
    let max = *sizes.iter().max().unwrap();
    let min = *sizes.iter().min().unwrap();
    assert!(max as f64 / min as f64 <= 2.5);
}
```

### Regression Tests

```rust
#[test]
fn test_debtmap_self_analysis_improved() {
    // Run debtmap on itself
    let output = run_debtmap_on_itself();

    // Parse god_object_detector.rs recommendations
    let splits = parse_splits_from_output(&output, "god_object_detector.rs");

    // Verify no 3-method splits
    for split in &splits {
        assert!(
            split.method_count >= 10,
            "Found undersized split: {} with {} methods",
            split.name,
            split.method_count
        );
    }
}
```

## Documentation Requirements

### Code Documentation

- Document `SplitSizeConfig` struct and all public methods
- Add inline comments explaining similarity calculation weights
- Document merge decision heuristics

### User Documentation

Add to CLI help and README:

```markdown
## Module Split Size Configuration

By default, debtmap enforces minimum split sizes to prevent over-fragmentation:
- Minimum 10 methods OR 150 lines per module
- Exception: High-cohesion utility modules can be 5+ methods

Override with CLI flags:
```bash
debtmap analyze . --min-split-methods 15 --min-split-lines 200
```

To disable minimum (not recommended):
```bash
debtmap analyze . --min-split-methods 1 --min-split-lines 1
```
```

### Architecture Updates

Update `ARCHITECTURE.md`:

```markdown
## God Object Split Validation

Module split recommendations undergo multi-stage validation:

1. **Size Filtering**: Rejects splits below configurable minimum (default: 10 methods OR 150 lines)
2. **Semantic Merging**: Combines undersized splits with most similar viable splits
3. **Cohesion Validation**: Rejects splits with low internal cohesion (<0.3)
4. **Distribution Balancing**: Ensures no single split is >2x size of others

This prevents the common anti-pattern of splitting one god object into many micro-modules.
```

## Implementation Notes

### Key Design Decisions

1. **Threshold Choice (10 methods)**:
   - Based on industry research: modules <10 methods rarely justify separation
   - Rust convention: Small modules discouraged in favor of coherent files
   - Empirical analysis: 67% of current splits are <10, causing fragmentation

2. **Exception for Utilities**:
   - Pure data structures (new/default/clone/eq) are coherent even at 5 methods
   - High cohesion (>0.7) indicates tight semantic relationship
   - Examples: `Point`, `Rectangle`, `Config` structs

3. **Merge Strategy**:
   - Semantic similarity preferred over alphabetical or size-based merging
   - Preserves domain coherence while meeting size constraints
   - Documents merge rationale for transparency

4. **Performance Considerations**:
   - Cohesion calculation requires call graph traversal
   - Cache cohesion scores to avoid recomputation
   - Similarity calculation is O(n²) but n is typically <20 splits

### Potential Gotchas

1. **Overly Aggressive Merging**: If all clusters are undersized, may merge semantically unrelated code
   - **Mitigation**: Set similarity threshold (0.4) - refuse to merge below this

2. **Loss of Fine-Grained Structure**: Some developers prefer many small modules
   - **Mitigation**: Make thresholds configurable, document rationale in recommendations

3. **Cohesion Calculation Accuracy**: Call graph may be incomplete for complex codebases
   - **Mitigation**: Use multiple signals (naming, data deps, calls) for robustness

## Migration and Compatibility

### Breaking Changes

- **None**: This is additive validation on top of existing clustering

### Configuration Migration

- New CLI flags are optional with sensible defaults
- Existing analyses without flags will use new defaults (improved output)

### Backward Compatibility

- Old format recommendations still generated if `--legacy-splits` flag used
- JSON output includes both filtered and unfiltered splits for comparison

### Rollout Strategy

1. **Phase 1**: Enable validation by default, allow opt-out via `--no-split-validation`
2. **Phase 2**: After 2 releases, remove opt-out (make validation mandatory)
3. **Phase 3**: Deprecate `--legacy-splits` flag

## Success Metrics

- **Reduction in tiny splits**: <10% of splits are <10 methods (down from 67%)
- **Improved actionability**: User survey shows 80%+ find recommendations implementable
- **Cohesion improvement**: Average cohesion score >0.5 (up from ~0.3)
- **Performance impact**: <5% increase in analysis time
- **User adoption**: <5% of users use `--no-split-validation` flag (indicating good defaults)
