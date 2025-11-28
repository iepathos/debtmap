---
number: 208
title: Optional Split Recommendations Display
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-11-28
---

# Specification 208: Optional Split Recommendations Display

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

The debtmap analyzer includes an experimental feature that recommends how to split large files/god objects into smaller modules. This "RECOMMENDED SPLITS" output appears for god object and large file recommendations in the terminal output, providing:

- Suggested module names and file paths
- Methods to move to each new module
- Fields needed by each split
- Category and priority classifications
- Language-specific advice (Rust patterns, etc.)
- Implementation order guidance

While this feature provides valuable guidance, it has several issues:

1. **Experimental Quality**: The split recommendations are based on heuristics and have mixed results in practice. Sometimes they suggest splits that don't make practical sense due to coupling or architectural constraints.

2. **Output Verbosity**: The splits section significantly increases output length, adding 20-50+ lines per god object item. For a codebase with multiple large files, this can make the output overwhelming.

3. **Actionability Concerns**: Users may take split recommendations as definitive guidance when they should be treated as suggestions requiring human judgment.

4. **Visual Noise**: For users who just want to see the top debt items and their scores, the detailed split recommendations obscure the core information.

### Current Behavior

When analyzing a codebase with god objects, the output includes detailed split sections like:

```
#1 SCORE: 370 [CRITICAL]
└─ ./src/organization/god_object_detector.rs (4363 lines, 55 functions)
└─ WHY THIS MATTERS: This module contains 55 module functions...
└─ ACTION: Split by analysis phase...
  - RECOMMENDED SPLITS (2 modules):

  -  god_object_detector/computation.rs
      Category: AST Traversal | Priority: Medium
      Size: 4 methods, ~60 lines
      Methods (4 total):
        • analyze_comprehensive()
        • analyze_enhanced()
        ...
      Fields needed: location_extractor, source_content

  -  god_object_detector/splits_analyze.rs
      ...

  - Rust PATTERNS:
      - Extract traits for shared behavior
      - Use newtype pattern for domain types
      ...

  - IMPLEMENTATION ORDER:
  -  [1] Start with lowest coupling modules...
  ...
```

This detailed output appears by default for all god object/large file items.

## Objective

Add a CLI flag `--show-splits` to control whether the "RECOMMENDED SPLITS" section is displayed in the output. By default, split recommendations will be hidden, and users must explicitly opt-in to see them.

This change:
1. Reduces default output verbosity for cleaner, more focused reports
2. Signals that the feature is experimental/optional
3. Gives users control over the level of detail they want to see
4. Maintains full functionality for users who find the splits valuable

## Requirements

### Functional Requirements

#### FR1: New CLI Flag
- Add `--show-splits` flag to the `analyze` subcommand
- Add `--show-splits` flag to the `validate` subcommand (for consistency)
- Flag is boolean (presence means true, absence means false)
- No short form needed (feature is opt-in and not frequently used)

#### FR2: Default Behavior Change
- By default (without `--show-splits`), hide the "RECOMMENDED SPLITS" section
- Continue to show all other god object information:
  - File path, line count, function count
  - WHY THIS MATTERS explanation
  - ACTION recommendation (high-level, without detailed splits)
  - IMPACT estimate
  - METRICS summary
  - SCORING breakdown
  - DEPENDENCIES count

#### FR3: Flag Behavior
- When `--show-splits` is provided, display the full split recommendations as currently shown
- Include all existing split information: module names, methods, fields, patterns, implementation order

#### FR4: Output Modification
- When splits are hidden, optionally show a brief note indicating they can be enabled:
  `(Use --show-splits for detailed module split recommendations)`
- This hint should only appear for items that have split recommendations available

#### FR5: Format Support
- Apply the `--show-splits` behavior to terminal format output
- Apply to markdown format output
- JSON output should always include split data (for programmatic consumers)
- HTML dashboard should continue to show splits (interactive context makes them more useful)

### Non-Functional Requirements

#### NFR1: Backward Compatibility
- Users relying on the current detailed output can restore it with `--show-splits`
- JSON output format unchanged (always includes splits data)
- No breaking changes to data structures or APIs

#### NFR2: Performance
- No performance impact - splits are already computed; this only affects display
- Flag parsing has negligible overhead

#### NFR3: Documentation
- Update `--help` output with clear description of the flag
- Document in any user-facing documentation

## Acceptance Criteria

- [ ] **AC1**: `--show-splits` flag added to `analyze` command in `src/cli.rs`
- [ ] **AC2**: `--show-splits` flag added to `validate` command in `src/cli.rs`
- [ ] **AC3**: Terminal formatter (`src/priority/formatter.rs`) respects the flag
- [ ] **AC4**: Markdown formatter (`src/priority/formatter_markdown.rs`) respects the flag
- [ ] **AC5**: Default behavior hides "RECOMMENDED SPLITS" section
- [ ] **AC6**: With `--show-splits`, full split output is displayed
- [ ] **AC7**: Brief hint shown for items with available splits when flag is not used
- [ ] **AC8**: JSON output unchanged (always includes splits)
- [ ] **AC9**: HTML dashboard unchanged (always shows splits)
- [ ] **AC10**: Unit tests verify flag behavior
- [ ] **AC11**: Help text accurately describes the flag purpose

## Technical Details

### Implementation Approach

#### Phase 1: Add CLI Flag

In `src/cli.rs`, add to the `Analyze` command:

```rust
/// Show detailed module split recommendations for god objects/large files.
/// This experimental feature suggests how to decompose large files.
/// Hidden by default due to mixed recommendation quality.
#[arg(long = "show-splits")]
show_splits: bool,
```

Add the same to `Validate` command for consistency.

#### Phase 2: Propagate Flag to Formatter

The formatting configuration needs to carry this flag. Options:

**Option A: Add to FormattingConfig**
```rust
// In src/formatting/mod.rs
pub struct FormattingConfig {
    // ... existing fields
    pub show_splits: bool,
}
```

**Option B: Add to priority formatter options**
```rust
// In src/priority/mod.rs or formatter.rs
pub struct PriorityFormatterOptions {
    pub verbosity: u8,
    pub show_splits: bool,
    // ... other options
}
```

Recommendation: Option A (FormattingConfig) since it's already passed through the system.

#### Phase 3: Modify Terminal Formatter

In `src/priority/formatter.rs`, the `format_god_object_steps` function needs to check the flag:

```rust
fn format_god_object_steps(
    output: &mut String,
    formatter: &PriorityFormatter,
    item: &priority::FileDebtItem,
    verbosity: u8,
    show_splits: bool,  // New parameter
) {
    // ... existing code for file info, WHY THIS MATTERS, ACTION, etc.

    // Only show detailed splits if flag is set
    if show_splits && !indicators.recommended_splits.is_empty() {
        // ... existing split formatting code
    } else if !indicators.recommended_splits.is_empty() {
        // Show hint that splits are available
        writeln!(
            output,
            "  (Use --show-splits for detailed module split recommendations)"
        ).unwrap();
    }

    // Continue with IMPACT, METRICS, etc.
}
```

#### Phase 4: Modify Markdown Formatter

Apply similar changes to `src/priority/formatter_markdown.rs` for consistency.

### Files to Modify

1. `src/cli.rs` - Add `--show-splits` flag to Analyze and Validate commands
2. `src/formatting/mod.rs` - Add `show_splits` to `FormattingConfig`
3. `src/main.rs` - Pass flag value to formatting config
4. `src/priority/formatter.rs` - Conditional split display in terminal output
5. `src/priority/formatter_markdown.rs` - Conditional split display in markdown output
6. Tests files as needed

### Data Flow

```
CLI (--show-splits)
  → main.rs (parse flag)
  → FormattingConfig { show_splits: bool }
  → PriorityFormatter::format()
  → format_god_object_steps(..., show_splits)
  → Conditional output
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_splits_hidden_by_default() {
        let item = create_test_god_object_item();
        let config = FormattingConfig::default();
        assert!(!config.show_splits);

        let output = format_god_object(&item, &config);
        assert!(!output.contains("RECOMMENDED SPLITS"));
        assert!(output.contains("--show-splits"));  // hint shown
    }

    #[test]
    fn test_splits_shown_with_flag() {
        let item = create_test_god_object_item();
        let config = FormattingConfig {
            show_splits: true,
            ..Default::default()
        };

        let output = format_god_object(&item, &config);
        assert!(output.contains("RECOMMENDED SPLITS"));
    }

    #[test]
    fn test_no_hint_when_no_splits_available() {
        let item = create_test_item_without_splits();
        let config = FormattingConfig::default();

        let output = format_god_object(&item, &config);
        assert!(!output.contains("--show-splits"));
    }
}
```

### Integration Tests

```rust
#[test]
fn test_cli_show_splits_flag() {
    let output = Command::new("cargo")
        .args(["run", "--", "analyze", "test_fixtures/large_file.rs", "--show-splits"])
        .output()
        .expect("Failed to run command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("RECOMMENDED SPLITS"));
}

#[test]
fn test_cli_default_hides_splits() {
    let output = Command::new("cargo")
        .args(["run", "--", "analyze", "test_fixtures/large_file.rs"])
        .output()
        .expect("Failed to run command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("RECOMMENDED SPLITS"));
}
```

### Manual Testing

1. Run `debtmap analyze .` and verify splits are hidden
2. Run `debtmap analyze . --show-splits` and verify splits are shown
3. Verify JSON output always includes splits
4. Verify HTML dashboard still shows splits

## Documentation Requirements

### CLI Help Text

```
--show-splits
    Show detailed module split recommendations for god objects and large files.
    This experimental feature suggests how to decompose large files into
    smaller, focused modules. Hidden by default.
```

### User Documentation

Update any relevant documentation to mention:
- The `--show-splits` flag exists
- Split recommendations are experimental
- JSON output always includes split data for automation

## Migration and Compatibility

### Breaking Changes

None - this is purely additive:
- New optional flag
- Default behavior changes to hide splits, but flag restores them
- JSON format unchanged

### Migration Path

Users who rely on the current detailed split output should add `--show-splits` to their commands. This can be:
- Added to shell aliases
- Added to CI scripts
- Noted in team documentation

## Implementation Notes

### Future Considerations

1. **Config File Support**: Consider adding `show_splits = true` to `.debtmap.toml` for users who always want splits
2. **Quality Improvement**: As the split algorithm improves, we may eventually change the default
3. **Selective Display**: Could add `--show-splits=high` to only show high-confidence splits

### Related Code Locations

- Split generation: `src/organization/god_object_analysis.rs`
- Split data structures: `src/priority/file_metrics.rs` (RecommendedSplit)
- Terminal formatting: `src/priority/formatter.rs:858-1088`
- Markdown formatting: `src/priority/formatter_markdown.rs`

## Success Metrics

- Default output is shorter and more focused
- Users can still access split recommendations when needed
- No increase in bug reports about confusing split recommendations
- No breaking changes to existing automation/scripts using JSON output
