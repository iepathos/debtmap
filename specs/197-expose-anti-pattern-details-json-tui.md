---
number: 197
title: Expose Anti-Pattern Detection Details in JSON and TUI Output
category: compatibility
priority: medium
status: draft
dependencies: []
created: 2025-01-13
---

# Specification 197: Expose Anti-Pattern Detection Details in JSON and TUI Output

**Category**: compatibility
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

Debtmap has comprehensive anti-pattern detection implemented in `src/organization/anti_pattern_detector.rs` that detects:
- UtilitiesModule (catch-all modules)
- TechnicalGrouping (verb-based organization)
- ParameterPassing (4+ parameters)
- MixedDataTypes (3+ unrelated types)
- Feature Envy (excessive external calls)
- Magic Values (hardcoded constants)
- Primitive Obsession
- Data Clumps

However, the `AntiPattern` struct lacks `#[derive(Serialize, Deserialize)]`, so this valuable analysis is NOT exposed in JSON output. This limits dashboard visualizations and API consumers from accessing architectural smell data.

## Objective

Make anti-pattern detection results available in JSON output and enhance TUI display, enabling:
1. D3 dashboards to visualize code smell distribution
2. API consumers to integrate anti-pattern data
3. Better TUI presentation of architectural issues

## Requirements

### Functional Requirements

1. **Serialization**: Add `#[derive(Serialize, Deserialize)]` to:
   - `AntiPattern` struct (anti_pattern_detector.rs)
   - `AntiPatternType` enum
   - `AntiPatternSeverity` enum
   - `SplitQualityReport` struct

2. **JSON Output Structure**: Add `anti_patterns` field to file-level debt items:
   ```json
   {
     "type": "File",
     "anti_patterns": {
       "quality_score": 0.72,
       "patterns": [
         {
           "pattern_type": "FeatureEnvy",
           "severity": "high",
           "location": { "file": "src/foo.rs", "line": 42, "function": "process" },
           "description": "Function 'process' makes 8 external calls vs 2 internal",
           "recommendation": "Consider moving to the envied type"
         }
       ],
       "summary": {
         "critical": 0,
         "high": 2,
         "medium": 3,
         "low": 1
       }
     }
   }
   ```

3. **TUI Display**: Add anti-pattern page or section showing:
   - List of detected patterns with severity icons
   - Quality score gauge
   - Recommendations for each pattern

### Non-Functional Requirements

- Backward compatible: `anti_patterns` field uses `skip_serializing_if = "Option::is_none"`
- Performance: No additional analysis required (data already computed)
- Minimal code changes: Leverage existing Display implementations

## Acceptance Criteria

- [ ] `AntiPattern` and related types derive `Serialize, Deserialize`
- [ ] JSON output includes `anti_patterns` field on file items when patterns detected
- [ ] JSON includes quality_score, pattern list with severity, and summary counts
- [ ] TUI displays anti-patterns in a dedicated section or page
- [ ] All existing tests pass
- [ ] Anti-pattern data accessible via `--format json`

## Technical Details

### Implementation Approach

1. **Add serde derives** to structs in `anti_pattern_detector.rs`:
   - Line ~58: `AntiPattern` struct
   - Line ~70: `AntiPatternType` enum
   - Line ~78: `AntiPatternSeverity` enum
   - Line ~88: `SplitQualityReport` struct

2. **Create output struct** in `unified.rs`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct AntiPatternOutput {
       pub quality_score: f64,
       pub patterns: Vec<AntiPatternItem>,
       pub summary: AntiPatternSummary,
   }
   ```

3. **Wire into FileDebtItemOutput**:
   ```rust
   #[serde(skip_serializing_if = "Option::is_none")]
   pub anti_patterns: Option<AntiPatternOutput>,
   ```

4. **Update conversion function** to populate from `IntegratedAnalysisResult`

### Architecture Changes

- `src/organization/anti_pattern_detector.rs`: Add serde derives
- `src/output/unified.rs`: Add `AntiPatternOutput` struct, wire into `FileDebtItemOutput`
- `src/tui/results/detail_pages/`: Add anti-pattern display page

### Data Flow

```
IntegratedArchitectureAnalyzer.analyze()
  → IntegratedAnalysisResult.anti_patterns: AntiPatternReport
    → convert_to_unified_format()
      → FileDebtItemOutput.anti_patterns: Option<AntiPatternOutput>
        → JSON serialization
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/organization/anti_pattern_detector.rs`
  - `src/output/unified.rs`
  - `src/tui/results/`
- **External Dependencies**: None (serde already a dependency)

## Testing Strategy

- **Unit Tests**: Verify serialization/deserialization of new structs
- **Integration Tests**: Run debtmap on test codebase, verify JSON contains anti_patterns
- **Snapshot Tests**: Compare JSON output before/after for backward compatibility

## Documentation Requirements

- **Code Documentation**: Document new output fields
- **User Documentation**: Update JSON schema documentation

## Implementation Notes

- The `AntiPattern` struct already has a `Display` impl (lines 565-701) that can be reused for TUI
- `OrganizationAntiPattern` enum in `mod.rs` is separate and already serializable
- Quality score formula: `1.0 - (weighted_severity_sum / max_possible_severity)`

## Migration and Compatibility

- No breaking changes - new field is optional
- Existing JSON consumers will see new field only when anti-patterns detected
