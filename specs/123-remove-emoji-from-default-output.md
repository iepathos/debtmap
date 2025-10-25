---
number: 123
title: Remove Emoji from Default Output
category: compatibility
priority: medium
status: draft
dependencies: []
created: 2025-10-25
---

# Specification 123: Remove Emoji from Default Output

**Category**: compatibility
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently includes emoji in its terminal output by default (via `EmojiMode::Auto`), with the `--plain` flag available to disable them. However, emoji characters can cause issues in various contexts:

1. **Terminal compatibility** - Not all terminals support emoji rendering properly
2. **CI/CD environments** - Build logs with emoji can be harder to parse and less professional
3. **Machine parsing** - Tools that process debtmap output may struggle with Unicode emoji
4. **Professional contexts** - Some users prefer clean, ASCII-only output

The current implementation has emoji enabled by default and requires users to explicitly opt out via the `--plain` flag. This approach assumes emoji support is the common case, but in practice, many users work in environments where plain text is preferable.

## Objective

Remove emoji from debtmap's default output format, making plain ASCII the default behavior. This will simplify the output format, improve compatibility across different environments, and eliminate the need for the emoji-related configuration flags.

## Requirements

### Functional Requirements

1. **Default Output Format**
   - Change `EmojiMode` default from `Auto` to `Never`
   - Remove emoji from all terminal output by default
   - Use ASCII fallback strings (e.g., `[TARGET]` instead of `🎯`)

2. **Code Cleanup**
   - Remove `EmojiMode` enum and related detection logic from `src/formatting/mod.rs`
   - Remove `emoji` field from `FormattingConfig`
   - Remove `emoji()` method from `OutputFormatter` trait
   - Remove `emoji_or_fallback()` helper function
   - Remove `detect_emoji_support()` function

3. **CLI Cleanup**
   - Remove `--plain` flag's emoji-disabling behavior (keep for colors only)
   - Remove any emoji-related environment variables (e.g., `DEBTMAP_NO_EMOJI`)
   - Update CLI help text if necessary

4. **Formatter Updates**
   - Update all calls to `formatter.emoji()` to use ASCII fallback directly
   - Remove emoji-checking logic in builders and formatters
   - Simplify formatting configuration

### Non-Functional Requirements

1. **Backward Compatibility**
   - The `--plain` flag should continue to work (for color disabling)
   - Existing output parsers expecting ASCII should work unchanged
   - No breaking changes to JSON or other output formats

2. **Code Quality**
   - Remove unused code cleanly
   - Maintain test coverage
   - Update documentation to reflect changes

## Acceptance Criteria

- [ ] `EmojiMode` enum removed from `src/formatting/mod.rs`
- [ ] `emoji` field removed from `FormattingConfig`
- [ ] `emoji()` method removed from `OutputFormatter` trait implementations
- [ ] `emoji_or_fallback()` and `detect_emoji_support()` functions removed
- [ ] All `formatter.emoji()` calls replaced with ASCII fallback strings
- [ ] `DEBTMAP_NO_EMOJI` environment variable check removed
- [ ] Terminal output uses ASCII characters only (no emoji)
- [ ] `--plain` flag still disables colors
- [ ] All existing tests pass
- [ ] No emoji-related code remains in codebase

## Technical Details

### Implementation Approach

1. **Phase 1: Update Formatting Module**
   - Remove `EmojiMode` enum (lines 32-55 in `src/formatting/mod.rs`)
   - Remove `emoji` field from `FormattingConfig` (line 60)
   - Remove `emoji()` method from `OutputFormatter` trait (line 117)
   - Remove `emoji()` implementations in `ColoredFormatter` and `PlainFormatter`
   - Remove `emoji_or_fallback()` function (lines 260-280)
   - Remove `detect_emoji_support()` function (lines 253-257)

2. **Phase 2: Update Formatters**
   - In `src/priority/formatter.rs`, replace all `formatter.emoji()` calls with direct ASCII strings
   - In `src/builders/unified_analysis.rs`, remove emoji-checking logic (lines 263, 347)
   - In `src/io/pattern_output.rs`, replace emoji calls with ASCII
   - In `src/io/writers/enhanced_markdown/` files, remove emoji usage

3. **Phase 3: Update CLI**
   - Simplify `FormattingConfig::plain()` method (line 103) to only disable colors
   - Remove any emoji-related documentation from `--plain` flag description
   - Remove `DEBTMAP_NO_EMOJI` environment variable checks

4. **Phase 4: Update Tests**
   - Update `tests/test_formatting.rs` to remove emoji tests
   - Update any snapshot tests that expect emoji output
   - Ensure color-related tests still pass

### Files to Modify

- `src/formatting/mod.rs` - Main formatting module cleanup
- `src/priority/formatter.rs` - Replace emoji calls
- `src/priority/formatter_verbosity.rs` - Replace emoji calls if present
- `src/builders/unified_analysis.rs` - Remove emoji checks
- `src/builders/parallel_unified_analysis.rs` - Remove emoji checks if present
- `src/io/pattern_output.rs` - Replace emoji calls
- `src/io/writers/enhanced_markdown/executive_summary.rs` - Remove emoji
- `src/io/writers/enhanced_markdown/health_writer.rs` - Remove emoji
- `src/io/writers/enhanced_markdown/formatters.rs` - Remove emoji
- `tests/test_formatting.rs` - Update tests
- `tests/test_color_validation.rs` - Update tests if needed
- `tests/coverage_data_optional_test.rs` - Update tests if needed

### Example Changes

**Before** (src/priority/formatter.rs:154):
```rust
formatter.emoji("🎯", "[TARGET]"),
```

**After**:
```rust
"[TARGET]",
```

**Before** (src/formatting/mod.rs:48-54):
```rust
pub fn should_use_emoji(&self) -> bool {
    match self {
        Self::Always => true,
        Self::Never => false,
        Self::Auto => detect_emoji_support(),
    }
}
```

**After**: (Entire enum removed)

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - Terminal output formatting
  - Priority display
  - Markdown writers
  - Pattern output
- **External Dependencies**: None (removing functionality)

## Testing Strategy

### Unit Tests

- Test that `FormattingConfig` no longer has emoji configuration
- Test that output formatters produce ASCII-only strings
- Test that `--plain` flag still disables colors
- Test that color-only configuration still works

### Integration Tests

- Run debtmap analyze and verify no emoji in output
- Verify terminal output is ASCII-only
- Verify JSON output unchanged
- Verify markdown output works correctly

### Regression Tests

- Ensure all existing tests pass with emoji removed
- Verify no performance degradation
- Check that output parsers still work

## Documentation Requirements

### Code Documentation

- Update module documentation for `src/formatting/mod.rs`
- Update function documentation where emoji behavior is mentioned
- Add migration notes if necessary

### User Documentation

- Update CLI help if needed (though emoji was never advertised)
- No major user-facing documentation changes needed
- Update any internal design docs that mention emoji

## Implementation Notes

### Simplification Benefits

1. **Reduced Complexity**: Removing emoji detection and configuration simplifies the codebase
2. **Better Compatibility**: ASCII-only output works everywhere
3. **Cleaner Code**: One less formatting dimension to test and maintain
4. **Professional Output**: More suitable for logs, CI/CD, and professional contexts

### Migration Considerations

This is a breaking change only for users who specifically relied on emoji in output. However:

1. Emoji was never a documented feature
2. ASCII fallbacks always existed and are more reliable
3. No API changes (only output format)
4. The `--plain` flag remains for color disabling

### Backwards Compatibility

- **Output parsers**: Should be unaffected (ASCII was always present as fallback)
- **Scripting**: Scripts parsing output will see cleaner, more consistent output
- **CI/CD**: Build logs will be more readable
- **Configuration**: `FormattingConfig` will be simpler but maintains color options

## Related Work

- The `--plain` flag (line 139 in src/cli.rs) will remain for color disabling
- Color support remains unchanged (ColorMode still functional)
- JSON and markdown output formats remain unchanged
