---
number: 108
title: Flexible Location Matching for Compare Command
category: compatibility
priority: high
status: draft
dependencies: []
created: 2025-10-07
---

# Specification 108: Flexible Location Matching for Compare Command

**Category**: compatibility
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The `debtmap compare` command currently fails when prodigy workflows specify target items that don't exist in the before analysis. This occurs because the location matching logic in `src/comparison/comparator.rs` only supports exact matches for the format `file:function:line`, where "function" must be an actual function name.

However, debtmap can analyze multiple types of code constructs beyond functions:
- Structs and their implementations
- Modules
- Type definitions
- File-level metrics

When prodigy creates implementation plans, it may specify locations like:
- `src/io/writers/enhanced_markdown/mod.rs:EnhancedMarkdownWriter:1` (struct)
- `src/main.rs:main:1` (module-level)
- `src/utils.rs:parse_config:10` (function)

The current implementation at `src/comparison/comparator.rs:191-219` (`find_item_by_location`) strictly requires all three parts (file, function, line) to match exactly. This causes the workflow to fail with:

```
Error: Target item not found in before analysis at location: src/io/writers/enhanced_markdown/mod.rs:EnhancedMarkdownWriter:1
```

This is a blocking issue for prodigy workflows that need to track improvements across different types of code constructs, not just functions.

## Objective

Implement flexible location matching that can:
1. Match exact locations when all parts are present (`file:function:line`)
2. Fall back to partial matches when exact match fails
3. Support matching at different granularities (function-level, file-level)
4. Maintain backward compatibility with existing location formats
5. Provide clear error messages when no match can be found

## Requirements

### Functional Requirements

1. **Multi-Strategy Matching**: Implement cascading match strategies with decreasing specificity:
   - **Exact match**: `file:function:line` (current behavior)
   - **Function match**: `file:function` (any line in that function)
   - **Approximate match**: Find items in the same file with similar names
   - **File-level match**: Aggregate metrics for all items in the file

2. **Format Validation**: Support multiple location format patterns:
   - `file:function:line` - Full specification (current format)
   - `file:function` - Function-level (any line)
   - `file` - File-level aggregation
   - `file:*:line` - Line-level (any function at that line)

3. **Graceful Degradation**: When exact match fails:
   - Try progressively less specific matches
   - Log which strategy succeeded
   - Include confidence score in results

4. **File-Level Aggregation**: For file-only matches:
   - Aggregate all debt items in the file
   - Calculate combined metrics (sum scores, average complexity)
   - Report on all functions/constructs in the file

5. **Error Handling**: When no match found at any level:
   - Return clear error describing what was searched
   - Suggest closest matches if available
   - Include file path normalization details

### Non-Functional Requirements

1. **Performance**: Matching logic should complete in O(n) time where n is the number of items
2. **Backward Compatibility**: Existing exact-match behavior preserved for valid locations
3. **Testability**: Each matching strategy independently testable
4. **Extensibility**: Easy to add new matching strategies in the future

## Acceptance Criteria

- [ ] Exact location matching works as before for valid `file:function:line` locations
- [ ] Function-level matching succeeds when exact line doesn't match but function exists
- [ ] File-level matching aggregates all items in a file when function not found
- [ ] Location parser handles all supported format patterns correctly
- [ ] Error messages clearly indicate which matching strategies were attempted
- [ ] All existing comparison tests continue to pass
- [ ] New tests cover each matching strategy independently
- [ ] New tests cover edge cases (empty files, no match at any level, path normalization)
- [ ] Prodigy workflow succeeds with struct/module-level target locations
- [ ] Comparison results include metadata about match strategy used

## Technical Details

### Implementation Approach

#### 1. Location Parser Enhancement

Create `LocationPattern` enum to represent different location formats:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum LocationPattern {
    Exact { file: String, function: String, line: usize },
    Function { file: String, function: String },
    File { file: String },
    LineRange { file: String, line: usize },
}

impl LocationPattern {
    /// Parse location string into appropriate pattern
    pub fn parse(location: &str) -> Result<Self> {
        let parts: Vec<&str> = location.split(':').collect();
        match parts.len() {
            1 => Ok(Self::File { file: parts[0].to_string() }),
            2 => Ok(Self::Function {
                file: parts[0].to_string(),
                function: parts[1].to_string(),
            }),
            3 => {
                if parts[1] == "*" {
                    Ok(Self::LineRange {
                        file: parts[0].to_string(),
                        line: parts[2].parse()?,
                    })
                } else {
                    Ok(Self::Exact {
                        file: parts[0].to_string(),
                        function: parts[1].to_string(),
                        line: parts[2].parse()?,
                    })
                }
            }
            _ => Err(anyhow::anyhow!("Invalid location format: {}", location)),
        }
    }
}
```

#### 2. Cascading Match Strategy

Replace `find_item_by_location` with strategy pattern:

```rust
#[derive(Debug, Clone)]
pub enum MatchStrategy {
    Exact,
    FunctionLevel,
    ApproximateName,
    FileLevel,
}

#[derive(Debug, Clone)]
pub struct MatchResult<'a> {
    pub items: Vec<&'a UnifiedDebtItem>,
    pub strategy: MatchStrategy,
    pub confidence: f64,
}

impl Comparator {
    fn find_items_by_location<'a>(
        &self,
        analysis: &'a UnifiedAnalysis,
        location: &str,
    ) -> Result<MatchResult<'a>> {
        let pattern = LocationPattern::parse(location)?;

        // Try strategies in order of specificity
        if let Some(result) = self.try_exact_match(analysis, &pattern) {
            return Ok(result);
        }

        if let Some(result) = self.try_function_match(analysis, &pattern) {
            return Ok(result);
        }

        if let Some(result) = self.try_approximate_match(analysis, &pattern) {
            return Ok(result);
        }

        if let Some(result) = self.try_file_match(analysis, &pattern) {
            return Ok(result);
        }

        Err(anyhow::anyhow!(
            "No items found matching location: {} (tried all strategies)",
            location
        ))
    }

    fn try_exact_match<'a>(
        &self,
        analysis: &'a UnifiedAnalysis,
        pattern: &LocationPattern,
    ) -> Option<MatchResult<'a>> {
        // Current exact matching logic
        // Returns single item with confidence 1.0
    }

    fn try_function_match<'a>(
        &self,
        analysis: &'a UnifiedAnalysis,
        pattern: &LocationPattern,
    ) -> Option<MatchResult<'a>> {
        // Match file + function name, ignore line number
        // Returns items matching that function with confidence 0.8
    }

    fn try_approximate_match<'a>(
        &self,
        analysis: &'a UnifiedAnalysis,
        pattern: &LocationPattern,
    ) -> Option<MatchResult<'a>> {
        // Fuzzy match on function/struct names using edit distance
        // Returns closest matches with confidence 0.5-0.7
    }

    fn try_file_match<'a>(
        &self,
        analysis: &'a UnifiedAnalysis,
        pattern: &LocationPattern,
    ) -> Option<MatchResult<'a>> {
        // Return all items in the file
        // Confidence 0.3-0.5 depending on number of items
    }
}
```

#### 3. Aggregation for Multiple Items

When multiple items match (e.g., file-level), aggregate metrics:

```rust
fn aggregate_metrics(&self, items: &[&UnifiedDebtItem]) -> TargetMetrics {
    let total_score: f64 = items.iter().map(|i| self.get_score(i)).sum();
    let avg_complexity = items.iter()
        .map(|i| i.cyclomatic_complexity + i.cognitive_complexity)
        .sum::<u32>() / items.len() as u32;

    TargetMetrics {
        score: total_score,
        cyclomatic_complexity: avg_complexity / 2, // Approximate split
        cognitive_complexity: avg_complexity / 2,
        coverage: items.iter()
            .filter_map(|i| i.transitive_coverage.as_ref())
            .map(|tc| tc.transitive)
            .sum::<f64>() / items.len() as f64,
        function_length: items.iter().map(|i| i.function_length).sum::<usize>(),
        nesting_depth: items.iter().map(|i| i.nesting_depth).max().unwrap_or(0),
    }
}
```

#### 4. Enhanced Comparison Result Metadata

Add match confidence to results:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetComparison {
    pub location: String,
    pub match_strategy: String,  // NEW
    pub match_confidence: f64,    // NEW
    pub matched_items_count: usize, // NEW
    pub before: TargetMetrics,
    pub after: Option<TargetMetrics>,
    pub improvements: ImprovementMetrics,
    pub status: TargetStatus,
}
```

### Architecture Changes

- **New module**: `src/comparison/location_matcher.rs` containing location parsing and matching strategies
- **Refactor**: Move matching logic out of `Comparator` into dedicated `LocationMatcher` struct
- **Update**: `ComparisonResult` types to include match metadata

### Data Structures

```rust
// In src/comparison/location_matcher.rs
pub struct LocationMatcher {
    normalization_rules: PathNormalizer,
}

impl LocationMatcher {
    pub fn new() -> Self {
        Self {
            normalization_rules: PathNormalizer::default(),
        }
    }

    pub fn find_matches<'a>(
        &self,
        items: &'a [UnifiedDebtItem],
        location: &str,
    ) -> Result<MatchResult<'a>> {
        // Implement cascading strategy logic
    }
}

struct PathNormalizer {
    // Handle ./ prefix, absolute vs relative paths, etc.
}
```

### APIs and Interfaces

```rust
// Public API for compare command
pub fn compare_analyses(
    before: UnifiedAnalysis,
    after: UnifiedAnalysis,
    target_location: Option<String>,
) -> Result<ComparisonResult> {
    let comparator = Comparator::new(before, after, target_location);
    comparator.compare()
}
```

## Dependencies

- **Prerequisites**: None (enhancement to existing functionality)
- **Affected Components**:
  - `src/comparison/comparator.rs` - Core comparison logic
  - `src/comparison/types.rs` - Add match metadata types
- **External Dependencies**:
  - Consider `strsim` crate for fuzzy string matching (optional)
  - `anyhow` for enhanced error messages (already in use)

## Testing Strategy

### Unit Tests

1. **Location Pattern Parsing**:
   - Test all valid format patterns
   - Test invalid formats return errors
   - Test edge cases (empty strings, special characters)

2. **Exact Matching**:
   - Verify current behavior preserved
   - Test path normalization (./ prefix handling)
   - Test case sensitivity

3. **Function-Level Matching**:
   - Match function with different line numbers
   - Match function with no line number specified
   - Handle multiple items for same function

4. **File-Level Matching**:
   - Aggregate multiple items in file
   - Handle empty files
   - Handle single-item files

5. **Error Cases**:
   - No match at any level returns clear error
   - Invalid location formats handled gracefully
   - Path normalization edge cases

### Integration Tests

1. **Prodigy Workflow Integration**:
   - Test with struct-level locations (current failing case)
   - Test with module-level locations
   - Test with function-level locations
   - Verify workflow completes successfully

2. **Real Codebase Tests**:
   - Run compare on debtmap itself
   - Test with various target location formats
   - Verify aggregated metrics are meaningful

3. **Backward Compatibility**:
   - Run all existing comparison tests
   - Verify exact matches still work identically
   - Ensure no performance regression

### Performance Tests

1. **Large Codebases**:
   - Benchmark matching on 10k+ items
   - Verify O(n) time complexity maintained
   - Test memory usage with aggregation

### Test Data

```rust
// Test fixtures
fn create_multi_item_analysis() -> UnifiedAnalysis {
    // File with multiple functions, structs, impls
    // Used to test file-level aggregation
}

fn create_similar_names() -> UnifiedAnalysis {
    // Functions with similar names for fuzzy matching
    // e.g., "parse_config", "parse_configuration", "parser_config"
}
```

## Documentation Requirements

### Code Documentation

- Document each matching strategy with examples
- Add rustdoc to `LocationPattern` and `MatchStrategy` enums
- Include usage examples in `LocationMatcher` docs

### User Documentation

- Update CLI help text for `debtmap compare --target` flag
- Document supported location format patterns
- Add examples showing different granularity levels
- Explain match confidence scoring

### Architecture Updates

- Update ARCHITECTURE.md with comparison matching strategy
- Document when to use each location format
- Add decision flowchart for matching strategies

## Implementation Notes

### Path Normalization

- Always strip `./` prefix for comparison
- Convert to absolute paths when possible
- Handle Windows vs Unix path separators
- Cache normalized paths for performance

### Confidence Scoring Guidelines

- **1.0**: Exact match (file:function:line all match)
- **0.8**: Function match (file:function match, line differs)
- **0.5-0.7**: Approximate match (edit distance < threshold)
- **0.3-0.5**: File match (all items in file)
- **0.0**: No match found

### Fuzzy Matching Threshold

- Use Levenshtein distance ≤ 2 for approximate matching
- Only consider names with at least 50% similarity
- Prioritize exact prefix matches over edit distance

### Performance Considerations

- Build location index on first match attempt (lazy initialization)
- Use HashMap for O(1) exact lookups
- Only compute fuzzy matches if exact/function matches fail
- Limit file-level aggregation to files with < 100 items

## Migration and Compatibility

### Breaking Changes

**None** - This is a pure enhancement that maintains full backward compatibility.

### Migration Requirements

**None** - Existing workflows and scripts continue to work unchanged.

### Compatibility Considerations

1. **JSON Output Format**: Add optional fields with `#[serde(default)]` to maintain compatibility
2. **CLI Interface**: No changes to command-line arguments
3. **API Stability**: Existing `compare()` function signature unchanged
4. **Version Compatibility**: Works with existing before/after JSON files

### Deprecation Path

**None** - No features are being deprecated.

## Success Metrics

1. **Workflow Success Rate**: Prodigy workflows with struct/module targets succeed 100% of time
2. **Match Quality**: 95%+ of approximate matches are semantically correct
3. **Performance**: No measurable performance regression on existing exact matches
4. **Error Clarity**: User feedback indicates error messages are helpful when no match found
5. **Code Coverage**: 90%+ coverage for all matching strategies

## Open Questions

1. **Fuzzy Matching Dependency**: Should we add `strsim` crate or implement simple Levenshtein ourselves?
   - **Recommendation**: Start with simple string prefix matching, add `strsim` only if needed

2. **File-Level Aggregation Semantics**: When aggregating multiple items, should we sum scores or average?
   - **Recommendation**: Sum scores for total file impact, include item count in metadata

3. **Location Format Standardization**: Should we recommend a canonical format in error messages?
   - **Recommendation**: Yes, suggest `file:function:line` in errors when parse fails

4. **Caching Strategy**: Should we cache location index across multiple comparisons?
   - **Recommendation**: Not initially, optimize only if profiling shows bottleneck

## Related Work

- **Similar Tools**: How do other code analysis tools handle location matching?
  - SonarQube: Uses file:line primarily, no function-level
  - ESLint: Uses file:line:column, no fuzzy matching
  - RuboCop: Exact file:line matching only

- **Best Practices**: Industry standards for code location identifiers
  - LSP (Language Server Protocol): Uses URI + Position (line/column)
  - GitHub: Uses file path + line ranges
  - Our approach: More flexible to support various granularities
