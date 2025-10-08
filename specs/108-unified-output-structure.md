---
number: 108
title: Unified Output Structure for File and Function Debt Items
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-10-08
---

# Specification 108: Unified Output Structure for File and Function Debt Items

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The current debtmap JSON output uses inconsistent structures for File-level and Function-level debt items, creating several problems for consumers:

### Current Structure Issues

1. **Inconsistent Wrappers**: Items are wrapped as `{File: {...}}` or `{Function: {...}}` instead of having a uniform structure with a `type` field
2. **Different Score Paths**:
   - File items: `.File.score` (flat)
   - Function items: `.Function.unified_score.final_score` (nested)
3. **Different Location Fields**:
   - File items: `.File.metrics.path`
   - Function items: `.Function.location.file`
4. **Complex Filtering**: Requires OR expressions like `(File.score >= 30) OR (Function.unified_score.final_score >= 30)`
5. **Difficult Sorting**: Cannot sort uniformly across both types

### Impact on Consumers

- **Workflow tools** (like Prodigy) need complex JSONPath and filter expressions
- **Custom scripts** must handle two different data structures
- **Data processing pipelines** require special cases for each type
- **UI/reporting tools** need duplicate code paths

### User Feedback

From Prodigy workflow development:
- "wtf you just removed the File part and broke this?"
- "is this a bug in debtmap to have null scores?"
- Confusion about why only 2 items were processed instead of 10

## Objective

Redesign the debtmap JSON output format to use a unified structure for both File-level and Function-level debt items, making it easier to filter, sort, and process debt items programmatically while maintaining backward compatibility through format versioning.

## Requirements

### Functional Requirements

1. **Unified Item Structure**:
   - All items have a consistent top-level structure
   - `type` field distinguishes "File" vs "Function" items
   - `score` field at consistent path for all items
   - `location` field with consistent structure

2. **Consistent Score Representation**:
   - Single `score` field at top level for all items
   - Preserve detailed scoring breakdown in nested object
   - Enable simple comparisons: `score >= 30`

3. **Unified Location Structure**:
   - All items have `location.file` field
   - Function items add `location.function` and `location.line`
   - File items have `location.function = null` and `location.line = null`

4. **Consistent Metrics**:
   - Common metrics at predictable paths
   - Type-specific metrics in nested structures
   - Clear separation of shared vs specialized fields

5. **Format Versioning**:
   - Include `format_version` field in output
   - Support both old and new formats via CLI flag
   - Clear migration path for consumers

### Non-Functional Requirements

1. **Backward Compatibility**:
   - Add `--output-format` flag: `legacy` (default) or `unified`
   - Maintain current format as default initially
   - Provide migration period (2-3 releases)
   - Clear deprecation warnings

2. **Performance**:
   - No performance degradation from format change
   - Minimal additional memory overhead
   - Efficient serialization for both formats

3. **Documentation**:
   - Clear schema documentation
   - Migration guide for consumers
   - JSON Schema definitions for validation

## Acceptance Criteria

- [ ] New unified format supports both File and Function items with consistent structure
- [ ] Single `score` field accessible at same path for all items
- [ ] Unified `location` structure works for both file-level and function-level items
- [ ] `--output-format unified` flag produces new format
- [ ] Default format remains unchanged (backward compatibility)
- [ ] Format version number included in output metadata
- [ ] Simple filter expressions work: `score >= 30`
- [ ] Simple sort expressions work: `score DESC`
- [ ] All existing information preserved in new format
- [ ] JSON Schema provided for new format
- [ ] Migration guide documented
- [ ] Unit tests cover both formats
- [ ] Integration tests validate format conversion
- [ ] Benchmark shows <5% performance impact

## Technical Details

### Unified Output Schema

```json
{
  "format_version": "2.0",
  "metadata": {
    "debtmap_version": "0.2.6",
    "generated_at": "2025-10-08T12:00:00Z",
    "project_root": "/path/to/project",
    "analysis_type": "unified"
  },
  "summary": {
    "total_items": 876,
    "total_debt_score": 8675.0,
    "debt_density": 80.5,
    "total_loc": 107712,
    "by_type": {
      "File": 6,
      "Function": 870
    },
    "by_category": {
      "GodObject": 6,
      "GodModule": 3,
      "TestingGap": 450,
      "ComplexFunction": 420
    },
    "score_distribution": {
      "critical": 10,
      "high": 28,
      "medium": 150,
      "low": 688
    }
  },
  "items": [
    {
      "type": "File",
      "score": 105.42,
      "category": "GodObject",
      "priority": "critical",
      "location": {
        "file": "src/cook/orchestrator/core.rs",
        "line": null,
        "function": null
      },
      "metrics": {
        "lines": 2802,
        "functions": 75,
        "classes": 1,
        "avg_complexity": 3.67,
        "max_complexity": 29,
        "total_complexity": 275,
        "coverage": 0.21,
        "uncovered_lines": 2204
      },
      "god_object_indicators": {
        "methods_count": 56,
        "fields_count": 8,
        "responsibilities": 6,
        "is_god_object": true,
        "god_object_score": 1.0,
        "responsibility_names": [...],
        "recommended_splits": [...]
      },
      "recommendation": {
        "action": "URGENT: 2802 lines, 75 functions! Split by data flow...",
        "priority": "URGENT",
        "implementation_steps": [...]
      },
      "impact": {
        "complexity_reduction": 55.0,
        "maintainability_improvement": 10.54,
        "test_effort": 220.4
      },
      "scoring_details": {
        "file_size_score": 45.0,
        "function_count_score": 30.0,
        "complexity_score": 20.0,
        "coverage_penalty": 10.42
      }
    },
    {
      "type": "Function",
      "score": 30.38,
      "category": "TestingGap",
      "priority": "critical",
      "location": {
        "file": "src/cli/yaml_validator.rs",
        "line": 61,
        "function": "YamlValidator::validate_mapreduce_workflow"
      },
      "metrics": {
        "cyclomatic_complexity": 20,
        "cognitive_complexity": 41,
        "length": 90,
        "nesting_depth": 5,
        "coverage": 0.0,
        "uncovered_lines": [61, 68, 69, ...]
      },
      "debt_type": {
        "TestingGap": {
          "coverage": 0.0,
          "cyclomatic": 20,
          "cognitive": 41
        }
      },
      "function_role": "BusinessLogic",
      "purity_analysis": {
        "is_pure": false,
        "confidence": 0.85,
        "side_effects": ["file_io", "state_mutation"]
      },
      "dependencies": {
        "upstream_count": 0,
        "downstream_count": 5,
        "upstream_callers": [],
        "downstream_callees": [...]
      },
      "recommendation": {
        "action": "Add 9 tests for 100% coverage gap, then refactor...",
        "priority": "CRITICAL",
        "implementation_steps": [...]
      },
      "impact": {
        "coverage_improvement": 50.0,
        "complexity_reduction": 6.0,
        "risk_reduction": 12.8
      },
      "scoring_details": {
        "coverage_score": 4.40,
        "complexity_score": 20.00,
        "dependency_score": 1.00,
        "base_score": 25.40,
        "entropy_dampening": 0.41,
        "role_multiplier": 1.30,
        "final_score": 30.38
      }
    }
  ]
}
```

### Implementation Approach

#### Phase 1: Add Unified Format Support

1. **Create new output module**: `src/output/unified.rs`
2. **Define unified data structures**:
   ```rust
   #[derive(Debug, Serialize, Deserialize)]
   pub struct UnifiedDebtOutput {
       pub format_version: String,
       pub metadata: OutputMetadata,
       pub summary: DebtSummary,
       pub items: Vec<UnifiedDebtItem>,
   }

   #[derive(Debug, Serialize, Deserialize)]
   #[serde(tag = "type")]
   pub enum UnifiedDebtItem {
       File(FileDebtItem),
       Function(FunctionDebtItem),
   }

   #[derive(Debug, Serialize, Deserialize)]
   pub struct FileDebtItem {
       pub score: f64,
       pub category: String,
       pub priority: Priority,
       pub location: Location,
       pub metrics: FileMetrics,
       pub god_object_indicators: Option<GodObjectIndicators>,
       pub recommendation: Recommendation,
       pub impact: Impact,
       pub scoring_details: ScoringDetails,
   }

   #[derive(Debug, Serialize, Deserialize)]
   pub struct FunctionDebtItem {
       pub score: f64,
       pub category: String,
       pub priority: Priority,
       pub location: Location,
       pub metrics: FunctionMetrics,
       pub debt_type: DebtType,
       pub function_role: FunctionRole,
       pub purity_analysis: PurityAnalysis,
       pub dependencies: Dependencies,
       pub recommendation: Recommendation,
       pub impact: Impact,
       pub scoring_details: ScoringDetails,
   }

   #[derive(Debug, Serialize, Deserialize)]
   pub struct Location {
       pub file: String,
       #[serde(skip_serializing_if = "Option::is_none")]
       pub line: Option<usize>,
       #[serde(skip_serializing_if = "Option::is_none")]
       pub function: Option<String>,
   }
   ```

3. **Add CLI flag**: `--output-format <legacy|unified>`
4. **Implement converter**: `legacy_to_unified()` function
5. **Add format version**: Include in output metadata

#### Phase 2: Normalization Logic

1. **Extract common score**:
   - File: Use existing `score` field
   - Function: Use `unified_score.final_score`

2. **Normalize locations**:
   - File: `{file: path, line: null, function: null}`
   - Function: `{file: location.file, line: location.line, function: location.function}`

3. **Categorize debt**:
   - File: Extract from `god_object_indicators`
   - Function: Extract from `debt_type`

4. **Priority mapping**:
   - score >= 100: "critical"
   - score >= 50: "high"
   - score >= 20: "medium"
   - score < 20: "low"

#### Phase 3: Backward Compatibility

1. **Default to legacy format**: Maintain current behavior
2. **Deprecation warnings**: Add when legacy format used
3. **Migration guide**: Document format differences
4. **JSON Schema**: Provide schemas for both formats

#### Phase 4: Testing Strategy

1. **Unit tests**:
   - Test legacy format output
   - Test unified format output
   - Test format conversion
   - Test location normalization
   - Test score extraction

2. **Integration tests**:
   - Analyze sample projects in both formats
   - Validate JSON schema compliance
   - Test CLI flag behavior
   - Test backward compatibility

3. **Performance tests**:
   - Benchmark legacy vs unified serialization
   - Measure memory overhead
   - Profile format conversion

## Architecture Changes

### New Modules

- `src/output/unified.rs` - Unified format data structures
- `src/output/converter.rs` - Legacy to unified conversion
- `src/output/schema.rs` - JSON schema definitions

### Modified Modules

- `src/cli/mod.rs` - Add `--output-format` flag
- `src/core/types.rs` - Extend with unified types
- `src/output/json.rs` - Support both formats

### Configuration

Add to `.debtmap.toml`:
```toml
[output]
format = "unified"  # or "legacy"
include_scoring_details = true
include_metadata = true
```

## Dependencies

### Prerequisites
None - This is a foundational change

### Affected Components
- All output formatting code
- JSON serialization
- CLI argument parsing
- Integration tests

### External Dependencies
- `serde` - Already used
- `serde_json` - Already used
- `jsonschema` (optional) - For schema validation

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_item_to_unified() {
        let legacy = create_legacy_file_item();
        let unified = legacy.to_unified();

        assert_eq!(unified.score, legacy.score);
        assert_eq!(unified.location.file, legacy.metrics.path);
        assert!(unified.location.line.is_none());
        assert!(unified.location.function.is_none());
    }

    #[test]
    fn test_function_item_to_unified() {
        let legacy = create_legacy_function_item();
        let unified = legacy.to_unified();

        assert_eq!(unified.score, legacy.unified_score.final_score);
        assert_eq!(unified.location.file, legacy.location.file);
        assert_eq!(unified.location.line, Some(legacy.location.line));
        assert_eq!(unified.location.function, Some(legacy.location.function));
    }

    #[test]
    fn test_simple_filter_on_unified() {
        let items = load_unified_output();
        let filtered: Vec<_> = items
            .into_iter()
            .filter(|item| item.score >= 30.0)
            .collect();

        assert_eq!(filtered.len(), 28);
    }

    #[test]
    fn test_simple_sort_on_unified() {
        let mut items = load_unified_output();
        items.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        assert!(items[0].score >= items[1].score);
        assert_eq!(items[0].type_name(), "File");
    }
}
```

### Integration Tests

- Analyze real projects (prodigy, debtmap itself)
- Compare legacy vs unified output
- Validate all data preserved
- Test with workflow tools (Prodigy)

### Performance Tests

- Benchmark serialization speed
- Measure memory usage
- Profile conversion overhead
- Test with large codebases (>100k LOC)

## Documentation Requirements

### User Documentation

1. **Output Format Guide**: `docs/output-format.md`
   - Schema documentation
   - Field descriptions
   - Examples for both formats

2. **Migration Guide**: `docs/migration-unified-format.md`
   - Breaking changes
   - How to update scripts/workflows
   - Conversion examples

3. **CLI Reference**: Update `--help` output
   - Document `--output-format` flag
   - Show default behavior
   - Link to schema docs

### Developer Documentation

1. **Architecture docs**: Update `ARCHITECTURE.md`
   - New output module structure
   - Format conversion logic
   - Backward compatibility approach

2. **API docs**: Document new types
   - `UnifiedDebtOutput` and related types
   - Conversion functions
   - Schema validation

3. **Examples**: Add code samples
   - Parsing unified format
   - Filtering and sorting
   - Custom processing

## Implementation Notes

### Sorting Considerations

Unified format enables simple sorting:
```bash
# Legacy (complex)
jq '.items | map(if .File then .File else .Function end) | map({score: (.score // .unified_score.final_score)}) | sort_by(.score) | reverse'

# Unified (simple)
jq '.items | sort_by(.score) | reverse'
```

### Filtering Considerations

Unified format enables simple filtering:
```bash
# Legacy (complex OR expression)
jq '.items[] | select((.File.score >= 30) or (.Function.unified_score.final_score >= 30))'

# Unified (simple)
jq '.items[] | select(.score >= 30)'
```

### JSONPath Considerations

Unified format simplifies JSONPath:
```yaml
# Legacy
json_path: "$.items[*].File"  # Only gets File items!

# Unified
json_path: "$.items[*]"  # Gets all items with consistent structure
filter: "score >= 30"   # Simple filter works for all types
```

## Migration and Compatibility

### Deprecation Timeline

1. **Version 0.3.0**: Add unified format support with `--output-format` flag, legacy remains default
2. **Version 0.3.1-0.3.5**: Warning when legacy format used, encourage migration
3. **Version 0.4.0**: Change default to unified, keep legacy available with flag
4. **Version 0.5.0**: Remove legacy format support

### Breaking Changes

- Default output format will change (with migration period)
- JSONPath expressions may need updates
- Filter expressions simplified
- Custom parsing code needs updates

### Migration Examples

#### Before (Legacy)
```rust
let items = output["items"]
    .as_array()
    .unwrap()
    .iter()
    .filter_map(|item| {
        if let Some(file) = item.get("File") {
            Some((file["score"].as_f64()?, "File"))
        } else if let Some(func) = item.get("Function") {
            Some((func["unified_score"]["final_score"].as_f64()?, "Function"))
        } else {
            None
        }
    })
    .filter(|(score, _)| *score >= 30.0)
    .collect::<Vec<_>>();
```

#### After (Unified)
```rust
let items = output["items"]
    .as_array()
    .unwrap()
    .iter()
    .filter_map(|item| {
        let score = item["score"].as_f64()?;
        let item_type = item["type"].as_str()?;
        Some((score, item_type))
    })
    .filter(|(score, _)| *score >= 30.0)
    .collect::<Vec<_>>();
```

## Success Metrics

- [ ] 100% of legacy data preserved in unified format
- [ ] <5% performance overhead for unified format
- [ ] 50% reduction in filter/sort expression complexity
- [ ] All integration tests pass with unified format
- [ ] JSON schema validates all output
- [ ] Migration guide rated 4+/5 by early adopters
- [ ] Zero data loss issues reported

## Future Enhancements

1. **Custom output fields**: Allow users to select which fields to include
2. **Multiple output formats**: CSV, XML, SQLite support
3. **Streaming output**: For very large codebases
4. **Incremental output**: Only changed items since last run
5. **Compression**: Gzip compressed output option
