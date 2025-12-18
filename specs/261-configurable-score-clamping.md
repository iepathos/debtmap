---
number: 261
title: Configurable Score Clamping
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-12-18
---

# Specification 261: Configurable Score Clamping

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently enforces a hard clamp of scores to the 0-100 range via the `Score0To100` type. This clamping happens automatically in `Score0To100::new()` and `normalize_final_score()`, preventing scores from exceeding 100 regardless of how severe the technical debt actually is.

While clamping to 100 provides a normalized scale for comparison, it loses information about extreme outliers. A function with a raw score of 150 and one with 500 both appear as 100, obscuring the relative severity difference.

Users have requested the ability to:
1. See raw unclamped scores for extreme debt items
2. Optionally enable clamping via CLI argument when needed

## Objective

Make score clamping optional and configurable via a `--clamp` CLI argument. By default, scores will be unclamped (no maximum). When `--clamp N` is specified, scores will be clamped to the maximum value N.

## Requirements

### Functional Requirements

1. **Default behavior: No clamping**
   - Scores can exceed 100 to reflect actual debt severity
   - Raw scores are preserved through the pipeline
   - Existing relative ordering is maintained

2. **Optional `--clamp` argument**
   - Syntax: `--clamp <MAX_SCORE>`
   - Example: `--clamp 100` to restore current behavior
   - Example: `--clamp 200` to allow scores up to 200
   - Value must be a positive float

3. **Configuration propagation**
   - Clamp value must flow from CLI to scoring pipeline
   - Clamp value must be available in `AnalyzeConfig`
   - Clamp value must be used in `normalize_final_score()`

4. **Backwards compatibility**
   - Existing configs without clamp should work (unclamped)
   - `Score0To100` type internals unchanged for serialization compatibility
   - JSON output field names unchanged

### Non-Functional Requirements

- No performance impact when clamping is disabled
- Clear documentation in CLI help text
- Minimal code changes to scoring pipeline

## Acceptance Criteria

- [ ] `debtmap analyze .` shows unclamped scores by default (can exceed 100)
- [ ] `debtmap analyze . --clamp 100` clamps scores to maximum of 100
- [ ] `debtmap analyze . --clamp 50` clamps scores to maximum of 50
- [ ] CLI help shows `--clamp` argument with clear description
- [ ] Scores above clamp value show as clamped value in output
- [ ] JSON output includes both raw score and clamped score when clamping is active
- [ ] TUI score breakdown page accurately reflects clamping status
- [ ] Tests verify clamping behavior for edge cases

## Technical Details

### Implementation Approach

#### 1. Add CLI Argument

In `src/cli/args.rs`, add to the `Analyze` command:

```rust
/// Maximum score value (no clamping if not specified).
/// Scores exceeding this value will be clamped to this maximum.
/// Example: --clamp 100 restricts all scores to 0-100 range.
#[arg(long = "clamp", help_heading = "Scoring Options")]
clamp: Option<f64>,
```

#### 2. Add to AnalyzeConfig

In `src/commands/analyze/config.rs`, add field:

```rust
pub clamp: Option<f64>,
```

#### 3. Create Clamping Configuration Type

Create `src/priority/scoring/clamp_config.rs`:

```rust
/// Global clamping configuration for score normalization
#[derive(Debug, Clone, Copy)]
pub struct ClampConfig {
    /// Maximum score value, or None for unclamped
    pub max_score: Option<f64>,
}

impl ClampConfig {
    pub fn unclamped() -> Self {
        Self { max_score: None }
    }

    pub fn clamped(max: f64) -> Self {
        Self { max_score: Some(max) }
    }

    pub fn apply(&self, score: f64) -> f64 {
        match self.max_score {
            Some(max) => score.clamp(0.0, max),
            None => score.max(0.0),  // Still clamp negative to 0
        }
    }
}
```

#### 4. Modify normalize_final_score()

In `src/priority/scoring/calculation.rs`:

```rust
/// Normalize final score with optional clamping
pub fn normalize_final_score_with_clamp(raw_score: f64, clamp_config: &ClampConfig) -> f64 {
    clamp_config.apply(raw_score)
}

/// Normalize final score (backwards compatibility, uses 0-100 clamp)
pub fn normalize_final_score(raw_score: f64) -> f64 {
    raw_score.clamp(0.0, 100.0)
}
```

#### 5. Thread Configuration Through Pipeline

The clamp configuration must flow through:
1. CLI parsing → `AnalyzeConfig`
2. `AnalyzeConfig` → `UnifiedAnalysisContext` or similar
3. Context → scoring functions

Options for threading:
- **Option A**: Add to `UnifiedAnalysisContext` and pass through
- **Option B**: Use thread-local or environment variable (simpler but less elegant)
- **Option C**: Modify scoring functions to accept clamp config (most explicit)

Recommended: **Option C** - Explicit parameter passing for clarity and testability.

#### 6. Update Score0To100 Type

The `Score0To100` type in `src/priority/score_types.rs` should:
- Keep existing `new()` for backwards compatibility (clamped to 100)
- Add `new_with_clamp(value: f64, clamp: Option<f64>)` for configurable clamping
- Alternatively, keep `Score0To100` for internal typing and handle clamping at output

**Recommended**: Keep `Score0To100` unchanged for type safety, apply clamping only at final output stage. The type name becomes a misnomer when unclamped, but changing it would require extensive refactoring.

### Architecture Changes

```
CLI (--clamp N)
    │
    ▼
AnalyzeConfig { clamp: Option<f64> }
    │
    ▼
UnifiedAnalysisContext / ScoringContext
    │
    ▼
normalize_final_score_with_clamp()
    │
    ▼
Final score output (clamped if configured)
```

### Data Structures

No new persistent data structures. `ClampConfig` is a simple runtime configuration.

### APIs and Interfaces

No external API changes. CLI interface extended with `--clamp` argument.

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/cli/args.rs` - CLI argument parsing
  - `src/commands/analyze/config.rs` - Configuration struct
  - `src/priority/scoring/calculation.rs` - Score normalization
  - `src/priority/unified_scorer.rs` - Scoring pipeline
  - `src/tui/results/detail_pages/score_breakdown.rs` - Display logic
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_clamp_config_unclamped() {
    let config = ClampConfig::unclamped();
    assert_eq!(config.apply(150.0), 150.0);
    assert_eq!(config.apply(-5.0), 0.0);  // Still clamps negative
}

#[test]
fn test_clamp_config_clamped_to_100() {
    let config = ClampConfig::clamped(100.0);
    assert_eq!(config.apply(150.0), 100.0);
    assert_eq!(config.apply(50.0), 50.0);
}

#[test]
fn test_clamp_config_clamped_to_custom() {
    let config = ClampConfig::clamped(50.0);
    assert_eq!(config.apply(75.0), 50.0);
    assert_eq!(config.apply(25.0), 25.0);
}
```

### Integration Tests

- Run `debtmap analyze` on test codebase, verify scores can exceed 100
- Run `debtmap analyze --clamp 100`, verify scores capped at 100
- Verify JSON output contains correct score values
- Verify TUI displays appropriate clamping information

### Performance Tests

- Verify no measurable performance difference with/without clamping

### User Acceptance

- Score breakdown page clearly shows when clamping is applied
- Help text explains `--clamp` argument purpose

## Documentation Requirements

- **Code Documentation**: Document `ClampConfig` and modified functions
- **User Documentation**: Update CLI help text and README
- **Architecture Updates**: None needed

## Implementation Notes

### Key Decisions

1. **Keep Score0To100 type unchanged** - Changing the type would require touching 60+ files. Instead, handle clamping at output boundaries.

2. **Default to unclamped** - This is a breaking change in behavior, but provides more information by default. Users who want the old behavior can use `--clamp 100`.

3. **Clamp at final output only** - Intermediate calculations may produce values > 100 (e.g., god object multipliers). Clamping should happen at the very end.

### Edge Cases

- Negative clamp values: Should error or clamp to 0
- Zero clamp value: All scores become 0 (valid but unusual)
- Very large clamp values (e.g., 1000000): Effectively unclamped

### Gotchas

- The `Score0To100` type still clamps internally. The configuration affects final output, not internal representation.
- Sorting and tier classification should use raw scores, not clamped scores, to maintain relative ordering.

## Migration and Compatibility

### Breaking Changes

- **Behavioral change**: Default output now shows unclamped scores
- **Visual change**: Scores may display as > 100 where previously capped

### Migration Path

Users who depend on 0-100 scores can add `--clamp 100` to their commands.

### Compatibility Notes

- JSON schema unchanged (score fields are still f64)
- Existing debtmap.toml configs continue to work
- CI/CD scripts may need updating if they validate score ranges
