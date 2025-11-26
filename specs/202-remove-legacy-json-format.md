---
number: 202
title: Remove Legacy JSON Format
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-11-25
---

# Specification 202: Remove Legacy JSON Format

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently supports two JSON output formats controlled by the `--output-format` CLI flag:

- **Legacy**: Uses enum wrapper serialization (`{ "File": {...} }` and `{ "Function": {...} }`)
- **Unified** (spec 108): Uses consistent structure with explicit `"type"` field and rich metadata

The legacy format exists solely for backward compatibility from before spec 108 was implemented. The unified format is objectively superior:
- Consistent structure for all item types
- Explicit `"type"` field for easy parsing
- Rich metadata (version, timestamps, summary statistics)
- Better tooling support

Since there are no known consumers of the legacy format and unified is the recommended format, maintaining both adds unnecessary code complexity and cognitive overhead.

## Objective

Remove legacy JSON format support entirely, making unified the only JSON output format. This simplifies the codebase by removing ~200 lines of format-switching logic and the `--output-format` CLI flag.

## Requirements

### Functional Requirements

1. **Remove `--output-format` CLI flag**
   - Remove the `JsonFormat` enum from `src/cli.rs`
   - Remove `json_format` field from the `Analyze` command
   - Remove format-related documentation from CLI help

2. **Simplify output module**
   - Remove format switching logic in `src/output/json.rs`
   - Remove `UnifiedJsonOutput` struct (legacy format structure)
   - Remove `apply_filters_unified()` function
   - Always use unified format path via `src/output/unified.rs`

3. **Update command handler**
   - Remove `json_format` from `AnalyzeConfig` in `src/commands/analyze.rs`
   - Remove format parameter wiring in `src/main.rs`

4. **Update output config**
   - Remove `json_format` from `OutputConfig` in `src/output/mod.rs`
   - Remove default legacy format fallback

5. **Clean up tests**
   - Remove `test_cli_default_output_format_is_legacy()` integration test
   - Update any tests that explicitly use legacy format
   - Ensure unified format tests provide adequate coverage

### Non-Functional Requirements

- **No functionality loss**: All features available in legacy format are present in unified
- **Clean removal**: No dead code, unused imports, or orphaned types
- **Test coverage maintained**: Unified format tests cover all scenarios

## Acceptance Criteria

- [ ] `--output-format` CLI flag is removed
- [ ] `JsonFormat` enum is removed from `src/cli.rs`
- [ ] `json_format` field is removed from `AnalyzeConfig`
- [ ] `output_json_with_format()` no longer takes a format parameter
- [ ] `UnifiedJsonOutput` struct is removed from `src/output/json.rs`
- [ ] `apply_filters_unified()` function is removed
- [ ] Format switching logic is removed from `output_json_with_format()`
- [ ] `OutputConfig.json_format` field is removed
- [ ] Legacy format test is removed from integration tests
- [ ] All existing tests pass
- [ ] JSON output uses unified format structure
- [ ] No compiler warnings related to unused code
- [ ] `cargo clippy` passes without new warnings

## Technical Details

### Files to Modify

| File | Changes |
|------|---------|
| `src/cli.rs` | Remove `JsonFormat` enum (lines 481-487), remove `json_format` arg (lines 53-57) |
| `src/commands/analyze.rs` | Remove `json_format` from `AnalyzeConfig` (line 18) |
| `src/main.rs` | Remove `json_format` wiring (~10 lines around line 775) |
| `src/output/mod.rs` | Remove `json_format` from `OutputConfig` (line 22), remove format routing (lines 96, 111) |
| `src/output/json.rs` | Remove `UnifiedJsonOutput`, `apply_filters_unified()`, simplify `output_json_with_format()` |
| `tests/cli_output_format_integration_test.rs` | Remove `test_cli_default_output_format_is_legacy()` (lines 254-300) |

### Implementation Approach

**Phase 1: Remove CLI and Config**
1. Remove `JsonFormat` enum from `src/cli.rs`
2. Remove `json_format` field from `Analyze` command arguments
3. Remove `json_format` from `AnalyzeConfig` in `src/commands/analyze.rs`
4. Update `main.rs` to remove format wiring

**Phase 2: Simplify Output Module**
1. Remove `json_format` from `OutputConfig`
2. Remove format switching in `output_json_with_format()`
3. Remove `UnifiedJsonOutput` struct
4. Remove `apply_filters_unified()` function
5. Always call unified format conversion

**Phase 3: Update Tests**
1. Remove `test_cli_default_output_format_is_legacy()` test
2. Update any tests that explicitly reference legacy format
3. Ensure unified format tests are comprehensive

**Phase 4: Verify**
1. Run full test suite
2. Run clippy for unused code warnings
3. Verify JSON output structure is unified format

### Code Changes

#### src/cli.rs - Remove JsonFormat

```rust
// DELETE these lines (481-487):
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum JsonFormat {
    /// Legacy format with {File: {...}} and {Function: {...}} wrappers
    Legacy,
    /// Unified format with consistent structure (spec 108)
    Unified,
}

// DELETE these lines from Analyze command (53-57):
        /// JSON output structure format (legacy or unified)
        /// 'legacy': Current format with {File: {...}} and {Function: {...}} wrappers
        /// 'unified': New format with consistent structure and 'type' field (spec 108)
        #[arg(long = "output-format", value_enum, default_value = "legacy")]
        json_format: JsonFormat,
```

#### src/output/json.rs - Simplify

```rust
// REMOVE UnifiedJsonOutput struct (lines 11-19)
// REMOVE apply_filters_unified function (lines 101-128)

// SIMPLIFY output_json_with_format to:
pub fn output_json_with_format(
    analysis: &priority::UnifiedAnalysis,
    top: Option<usize>,
    tail: Option<usize>,
    output_file: Option<PathBuf>,
    include_scoring_details: bool,
) -> Result<()> {
    let unified_output = crate::output::unified::convert_to_unified_format(
        analysis,
        include_scoring_details,
    );
    let filtered = apply_filters_to_unified_output(unified_output, top, tail);
    let json = serde_json::to_string_pretty(&filtered)?;

    if let Some(path) = output_file {
        if let Some(parent) = path.parent() {
            crate::io::ensure_dir(parent)?;
        }
        let mut file = fs::File::create(path)?;
        file.write_all(json.as_bytes())?;
    } else {
        println!("{json}");
    }
    Ok(())
}
```

### Estimated Lines Changed

- **Removed**: ~150 lines
- **Modified**: ~50 lines
- **Net reduction**: ~100 lines

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/cli.rs` - CLI argument parsing
  - `src/commands/analyze.rs` - Command configuration
  - `src/main.rs` - Argument wiring
  - `src/output/mod.rs` - Output configuration
  - `src/output/json.rs` - JSON output generation
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

Existing tests in `src/output/json.rs` that use `UnifiedJsonOutput` will need updating to use the unified format structure instead:
- `test_output_json_with_head_parameter`
- `test_output_json_with_tail_parameter`
- `test_output_json_without_filters`
- `test_output_json_head_larger_than_items`
- `test_output_json_tail_larger_than_items`
- `test_output_json_includes_file_level_items`

These tests currently parse `UnifiedJsonOutput` - they should be updated to parse `crate::output::unified::UnifiedOutput` instead.

### Integration Tests

Keep existing unified format tests in `tests/cli_output_format_integration_test.rs`:
- `test_cli_output_format_unified_produces_valid_structure`
- `test_cli_unified_format_scope_filtering`
- `test_cli_unified_format_metrics_presence`

Remove:
- `test_cli_default_output_format_is_legacy`

### Verification Commands

```bash
# All tests pass
cargo test --all-features

# No clippy warnings
cargo clippy --all-targets --all-features -- -D warnings

# Verify unified format output
cargo run -- analyze src --format json 2>/dev/null | jq '.format_version'
# Should output: "2.0"
```

## Documentation Requirements

### Code Documentation

- Update any doc comments referencing `--output-format` flag
- Update function signatures that lose the format parameter

### User Documentation

- Remove `--output-format` from CLI help (automatic with clap)
- Update any user guides mentioning format selection

## Implementation Notes

### Migration for External Consumers

If any external tools consume debtmap JSON output:
1. They should already support unified format (spec 108 implemented previously)
2. Unified format has `format_version: "2.0"` field for detection
3. Tools can check for this field to ensure compatibility

### Rollback Strategy

If issues are discovered:
1. Revert the commits
2. Consider adding deprecation warning instead of immediate removal
3. Re-evaluate timeline for legacy removal

### Related Work

- Spec 108 introduced the unified format
- Spec 180 mentions dashboard backend handling both formats (should be verified after this change)

## Migration and Compatibility

### Breaking Change

This is a **breaking change** for any tooling that:
1. Uses `--output-format legacy` explicitly
2. Parses legacy format JSON structure (enum wrappers)

### Mitigation

- Unified format has been available since spec 108
- Unified format is strictly superior for parsing
- Any modern integration should already use unified

### Version Bump

Consider minor version bump to signal the change:
- Before: 0.x.y
- After: 0.x+1.0

## Success Metrics

- **Code reduction**: ~100 lines removed
- **Simplification**: Single code path for JSON output
- **Test maintenance**: Fewer test cases to maintain
- **No regressions**: All existing functionality preserved
