---
number: 268
title: File-Scope Analysis Improvements
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-01-10
---

# Specification 268: File-Scope Analysis Improvements

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

When debtmap analyzes files as `[file-scope]` items (god files/modules), it aggregates metrics in ways that can produce misleading results. The current approach sums all function complexities together, which conflates "many simple functions" with "one god function."

### Problem Analysis

A real-world example from `cargo-cargofmt/src/formatting/overflow.rs`:
- **Cyclomatic Complexity: 182** (sum of all functions)
- **Cognitive Complexity: 139**
- **Lines of Code: 621** (includes ~1280 lines of tests)
- **Flagged as high-priority debt**

However, the actual code structure is:
- **~30 small, focused functions** (average ~6 cyclomatic each)
- **Largest function: 26 lines** (`reflow_arrays`)
- **Most functions: 5-15 lines**
- **Clear separation of concerns** (determine/apply pattern)
- **Comprehensive test coverage** (~1280 lines of tests)

The summed complexity score (182) misrepresents a well-structured file as a "god object."

### Root Cause

1. **Complexity Aggregation**: `aggregate_complexity_metrics()` sums cyclomatic/cognitive without providing distribution context
2. **LOC Includes Tests**: Line count includes `#[cfg(test)]` module content
3. **No Function Distribution**: Missing metrics for max/avg/median function complexity
4. **No "Healthy File" Recognition**: Cannot distinguish distributed complexity from concentrated complexity

## Objective

Improve file-scope analysis to:
1. Report function complexity distribution, not just sums
2. Exclude test code from LOC calculations
3. Distinguish "many simple functions" from "one complex function"
4. Reduce false positives for well-structured multi-function files

## Requirements

### Functional Requirements

#### FR-1: Function Distribution Metrics
For file-scope items, calculate and report:
- `function_count`: Number of functions in file
- `max_function_complexity`: Highest cyclomatic in any single function
- `avg_function_complexity`: Average cyclomatic per function
- `median_function_complexity`: Median cyclomatic (more robust to outliers)
- `functions_exceeding_threshold`: Count of functions over threshold (default: 15)

#### FR-2: Test Code Exclusion from LOC
- Exclude lines inside `#[cfg(test)]` modules from LOC count
- Report `production_loc` and `test_loc` separately
- Use `production_loc` for density calculations

#### FR-3: Complexity Distribution Classification
Add classification based on distribution:
- **Concentrated**: Max complexity > 50% of total → likely god function
- **Distributed**: Max complexity < 20% of total → well-structured file
- **Mixed**: Between 20-50% → needs investigation

#### FR-4: Adjusted File-Scope Scoring
Modify scoring to consider distribution:
- Files with distributed complexity get dampened scores
- Files with concentrated complexity maintain current scoring
- Factor in `max_function_complexity` vs `total_complexity` ratio

#### FR-5: Enhanced Output Format
Update LLM markdown to show distribution:
```markdown
#### Metrics
- Total Cyclomatic Complexity: 182
- Complexity Distribution: Distributed (max: 12, avg: 6, median: 5)
- Functions: 30 total, 0 exceeding threshold
- Production LOC: 620
- Test LOC: 1280
- Distribution Classification: Well-Structured File
```

### Non-Functional Requirements

#### NFR-1: Backwards Compatibility
- Existing fields maintained for backwards compatibility
- New fields are additive
- Total complexity metrics still available

#### NFR-2: Performance
- Distribution calculations should add <2% overhead
- Test LOC detection uses existing AST traversal

## Acceptance Criteria

- [ ] `GodObjectAggregatedMetrics` includes `function_count`, `max_function_complexity`, `avg_function_complexity`
- [ ] `[file-scope]` items show distribution classification in output
- [ ] LOC calculation excludes `#[cfg(test)]` module content
- [ ] `production_loc` and `test_loc` reported separately
- [ ] Distribution-based dampening applied to file-scope scoring
- [ ] Files with distributed complexity (<20% max/total ratio) get reduced scores
- [ ] Files with concentrated complexity (>50% max/total ratio) maintain scores
- [ ] LLM markdown format updated to show distribution metrics
- [ ] Unit tests verify distribution classification logic
- [ ] Integration test shows `overflow.rs`-like file gets lower score with distribution analysis

## Technical Details

### Implementation Approach

#### Phase 1: Distribution Metrics Collection

Update `src/priority/god_object_aggregation.rs`:
```rust
#[derive(Debug, Clone)]
pub struct GodObjectAggregatedMetrics {
    // Existing fields
    pub total_cyclomatic: u32,
    pub total_cognitive: u32,
    pub max_nesting_depth: u32,

    // NEW: Distribution metrics
    pub function_count: usize,
    pub max_function_cyclomatic: u32,
    pub avg_function_cyclomatic: f64,
    pub median_function_cyclomatic: u32,
    pub functions_exceeding_threshold: usize,

    // NEW: LOC separation
    pub production_loc: usize,
    pub test_loc: usize,

    // NEW: Classification
    pub complexity_distribution: ComplexityDistribution,

    // ... existing fields
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplexityDistribution {
    Concentrated,  // Max > 50% of total
    Mixed,         // Max 20-50% of total
    Distributed,   // Max < 20% of total
}
```

Add new aggregation function:
```rust
/// Calculate complexity distribution metrics
pub fn aggregate_distribution_metrics(members: &[&UnifiedDebtItem]) -> DistributionMetrics {
    let complexities: Vec<u32> = members
        .iter()
        .map(|m| m.cyclomatic_complexity)
        .collect();

    let total: u32 = complexities.iter().sum();
    let max = complexities.iter().max().copied().unwrap_or(0);
    let count = complexities.len();

    let avg = if count > 0 {
        total as f64 / count as f64
    } else {
        0.0
    };

    let median = calculate_median(&complexities);

    let exceeding = complexities.iter()
        .filter(|&&c| c > COMPLEXITY_THRESHOLD)
        .count();

    let distribution = classify_distribution(max, total);

    DistributionMetrics {
        function_count: count,
        max_complexity: max,
        avg_complexity: avg,
        median_complexity: median,
        exceeding_threshold: exceeding,
        distribution,
    }
}

fn classify_distribution(max: u32, total: u32) -> ComplexityDistribution {
    if total == 0 {
        return ComplexityDistribution::Distributed;
    }

    let ratio = max as f64 / total as f64;

    if ratio > 0.5 {
        ComplexityDistribution::Concentrated
    } else if ratio > 0.2 {
        ComplexityDistribution::Mixed
    } else {
        ComplexityDistribution::Distributed
    }
}
```

#### Phase 2: Test LOC Exclusion

Update `src/organization/god_object/ast_visitor.rs`:
```rust
impl<'ast> Visit<'ast> for TypeVisitor {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        // Check for #[cfg(test)] attribute
        let is_test_module = item.attrs.iter().any(|attr| {
            attr.path().is_ident("cfg") &&
            attr.parse_args::<syn::Ident>()
                .map(|i| i == "test")
                .unwrap_or(false)
        });

        if is_test_module {
            // Track test LOC separately
            self.test_loc += calculate_item_loc(item);
        } else {
            // Normal visit for production code
            syn::visit::visit_item_mod(self, item);
        }
    }
}
```

#### Phase 3: Scoring Adjustment

Update `src/priority/scoring/calculation.rs`:
```rust
/// Calculate distribution-aware complexity factor for file-scope items
pub fn calculate_file_scope_complexity_factor(
    total_complexity: u32,
    distribution: ComplexityDistribution,
    max_complexity: u32,
) -> f64 {
    let base_factor = calculate_complexity_factor(total_complexity as f64);

    // Apply dampening based on distribution
    let dampening = match distribution {
        ComplexityDistribution::Distributed => 0.4,  // 60% reduction
        ComplexityDistribution::Mixed => 0.7,        // 30% reduction
        ComplexityDistribution::Concentrated => 1.0, // No reduction
    };

    // Also consider if max function is actually complex
    let max_factor = if max_complexity < 15 {
        0.5  // No single function is concerning
    } else if max_complexity < 30 {
        0.75
    } else {
        1.0  // At least one god function exists
    };

    base_factor * dampening * max_factor
}
```

#### Phase 4: Output Format Updates

Update `src/io/writers/llm_markdown.rs`:
```rust
fn write_file_scope_metrics(item: &UnifiedDebtItem, out: &mut String) {
    if let Some(dist) = &item.distribution_metrics {
        writeln!(out, "- Total Cyclomatic Complexity: {}", item.cyclomatic_complexity);
        writeln!(out, "- Complexity Distribution: {} (max: {}, avg: {:.1}, median: {})",
            dist.distribution.display_name(),
            dist.max_complexity,
            dist.avg_complexity,
            dist.median_complexity
        );
        writeln!(out, "- Functions: {} total, {} exceeding threshold",
            dist.function_count,
            dist.exceeding_threshold
        );
        writeln!(out, "- Production LOC: {}", dist.production_loc);
        writeln!(out, "- Test LOC: {}", dist.test_loc);
        writeln!(out, "- Distribution Classification: {}", dist.classification_explanation());
    }
}
```

### Architecture Changes

- Extended `GodObjectAggregatedMetrics` with distribution fields
- New helper functions for distribution calculation
- Modified scoring pipeline for file-scope items
- Enhanced LLM output writer

### Data Structures

```rust
#[derive(Debug, Clone)]
pub struct DistributionMetrics {
    pub function_count: usize,
    pub max_complexity: u32,
    pub avg_complexity: f64,
    pub median_complexity: u32,
    pub exceeding_threshold: usize,
    pub distribution: ComplexityDistribution,
    pub production_loc: usize,
    pub test_loc: usize,
}

impl ComplexityDistribution {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Concentrated => "Concentrated",
            Self::Mixed => "Mixed",
            Self::Distributed => "Distributed",
        }
    }

    pub fn classification_explanation(&self) -> &'static str {
        match self {
            Self::Concentrated => "Contains god function(s) - refactoring recommended",
            Self::Mixed => "Some complexity concentration - review recommended",
            Self::Distributed => "Well-Structured File - complexity evenly distributed",
        }
    }
}
```

### APIs and Interfaces

New public functions:
- `aggregate_distribution_metrics(members) -> DistributionMetrics`
- `classify_distribution(max, total) -> ComplexityDistribution`
- `calculate_file_scope_complexity_factor(total, distribution, max) -> f64`
- `calculate_median(values) -> u32`

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/priority/god_object_aggregation.rs`
  - `src/priority/scoring/calculation.rs`
  - `src/organization/god_object/ast_visitor.rs`
  - `src/io/writers/llm_markdown.rs`
  - `src/output/unified/types.rs`
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_classify_distribution_concentrated() {
    // Single function has 60% of complexity
    assert_eq!(
        classify_distribution(60, 100),
        ComplexityDistribution::Concentrated
    );
}

#[test]
fn test_classify_distribution_distributed() {
    // Max function has only 10% of complexity
    assert_eq!(
        classify_distribution(10, 100),
        ComplexityDistribution::Distributed
    );
}

#[test]
fn test_median_calculation() {
    assert_eq!(calculate_median(&[1, 2, 3, 4, 5]), 3);
    assert_eq!(calculate_median(&[1, 2, 3, 4]), 2); // or 3, depending on convention
    assert_eq!(calculate_median(&[100]), 100);
    assert_eq!(calculate_median(&[]), 0);
}

#[test]
fn test_distributed_file_gets_dampened_score() {
    let concentrated = calculate_file_scope_complexity_factor(100, Concentrated, 60);
    let distributed = calculate_file_scope_complexity_factor(100, Distributed, 5);

    assert!(distributed < concentrated * 0.5);
}
```

### Integration Tests

```rust
#[test]
fn test_well_structured_file_not_flagged_as_god_object() {
    // Create a file with 30 small functions
    let file_content = generate_file_with_small_functions(30);
    let analysis = analyze_file(&file_content);

    assert_eq!(analysis.distribution, ComplexityDistribution::Distributed);
    assert!(analysis.score < 30.0);  // Not high priority
}

#[test]
fn test_god_file_still_flagged() {
    // Create a file with one massive function
    let file_content = generate_file_with_god_function(500);
    let analysis = analyze_file(&file_content);

    assert_eq!(analysis.distribution, ComplexityDistribution::Concentrated);
    assert!(analysis.score > 50.0);  // High priority
}
```

### Performance Tests

- Benchmark distribution calculation on 1000 function files
- Verify <2% overhead compared to baseline

## Documentation Requirements

- **Code Documentation**: Document distribution metrics and classification
- **User Documentation**: Explain how distribution affects scoring
- **Architecture Updates**: Add section on file-scope analysis

## Implementation Notes

### Edge Cases

1. **Empty files**: Return `Distributed` classification
2. **Single-function files**: Use function complexity directly
3. **Files with only test code**: Still analyze but note in output
4. **Mixed impl blocks**: Count all associated functions

### Heuristics Rationale

The 20%/50% thresholds for distribution classification:
- **<20%**: With 5+ functions, max being <20% means good distribution
- **>50%**: One function dominates the file's complexity
- **20-50%**: Gray area requiring human judgment

## Migration and Compatibility

- No breaking changes
- New fields added to existing structures
- Output formats enhanced with additional sections
- Existing consumers see no change in base metrics
