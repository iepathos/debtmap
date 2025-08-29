---
number: 69
title: Configurable Output Formatting (Color and Emoji Control)
category: compatibility
priority: medium
status: draft
dependencies: []
created: 2025-08-29
---

# Specification 69: Configurable Output Formatting (Color and Emoji Control)

**Category**: compatibility
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

The current CLI implementation forces colored output and emoji usage regardless of the output environment. This creates several issues:
- Terminal output may not support ANSI color codes, resulting in garbled text
- Machine-readable output (for CI/CD pipelines) becomes difficult to parse
- Emoji characters may not render correctly in all environments
- Users with accessibility needs may prefer plain text output
- Log files become cluttered with ANSI escape sequences

Modern CLI tools should detect terminal capabilities and provide user control over formatting decisions.

## Objective

Provide intelligent, configurable control over output formatting including ANSI colors and emoji usage, ensuring the CLI works correctly across all environments while maintaining user preferences.

## Requirements

### Functional Requirements
- Automatically detect terminal color support using standard environment variables and terminal capabilities
- Provide explicit CLI flags to force color on/off regardless of detection
- Provide separate control for emoji output independent of color control
- Ensure all output formats (terminal, markdown, JSON) respect formatting preferences
- Maintain backwards compatibility with existing output behavior by default

### Non-Functional Requirements
- Zero performance impact from capability detection
- Clear documentation of all formatting options
- Consistent behavior across all supported platforms (Linux, macOS, Windows)
- Graceful degradation when capabilities cannot be determined

## Acceptance Criteria

- [ ] Terminal color support is automatically detected via `TERM`, `NO_COLOR`, and `CLICOLOR` environment variables
- [ ] `--color` flag accepts values: `auto` (default), `always`, `never`
- [ ] `--no-color` flag provides shorthand for `--color=never`
- [ ] `--emoji` flag accepts values: `auto` (default), `always`, `never`
- [ ] `--no-emoji` flag provides shorthand for `--emoji=never`
- [ ] JSON output never includes ANSI codes or emoji regardless of settings
- [ ] Markdown output respects emoji settings but ignores color settings
- [ ] Terminal output respects both color and emoji settings
- [ ] `NO_COLOR` environment variable disables color when set (per no-color.org standard)
- [ ] `CLICOLOR=0` disables color output
- [ ] `CLICOLOR_FORCE=1` forces color output even when piping
- [ ] Color output is disabled by default when stdout is not a TTY (piping/redirection)
- [ ] All existing tests pass with default settings
- [ ] New tests validate all formatting combinations

## Technical Details

### Implementation Approach
1. Add dependency on `supports-color` or similar crate for terminal detection
2. Create `OutputFormatter` trait with methods for styled output
3. Implement `ColoredFormatter` and `PlainFormatter` variants
4. Add CLI argument parsing for formatting flags
5. Thread formatter configuration through all output modules
6. Replace direct color/emoji usage with formatter methods

### Architecture Changes
- New `formatting` module for output control logic
- Modified `cli.rs` to accept new formatting flags
- Updated all output modules to use formatter abstraction
- Enhanced configuration structure to include formatting preferences

### Data Structures
```rust
pub enum ColorMode {
    Auto,    // Detect based on terminal
    Always,  // Force colors on
    Never,   // Force colors off
}

pub enum EmojiMode {
    Auto,    // Use emoji if terminal supports Unicode
    Always,  // Always use emoji
    Never,   // Never use emoji
}

pub struct FormattingConfig {
    pub color: ColorMode,
    pub emoji: EmojiMode,
}
```

### APIs and Interfaces
```rust
pub trait OutputFormatter {
    fn success(&self, text: &str) -> String;
    fn error(&self, text: &str) -> String;
    fn warning(&self, text: &str) -> String;
    fn info(&self, text: &str) -> String;
    fn emoji(&self, emoji: &str, fallback: &str) -> String;
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**: 
  - CLI argument parsing
  - Terminal output module
  - Markdown output module
  - All modules that generate user-facing output
- **External Dependencies**: 
  - `supports-color` or `termcolor` crate for capability detection
  - `atty` crate for TTY detection

## Testing Strategy

- **Unit Tests**: 
  - Test color mode detection logic
  - Test emoji mode detection logic
  - Test formatter implementations
  - Test environment variable handling
- **Integration Tests**: 
  - Test CLI flag parsing and precedence
  - Test output in different modes
  - Test piping scenarios
- **Performance Tests**: 
  - Ensure no performance regression from formatter abstraction
- **User Acceptance**: 
  - Verify output is readable in various terminal emulators
  - Test with screen readers and accessibility tools

## Documentation Requirements

- **Code Documentation**: 
  - Document all formatter trait methods
  - Document environment variable behavior
  - Document fallback strategies
- **User Documentation**: 
  - Add formatting options to CLI help text
  - Document environment variables in README
  - Provide examples of different formatting modes
- **Architecture Updates**: 
  - Update ARCHITECTURE.md with formatting module design

## Implementation Notes

- Follow no-color.org standard for `NO_COLOR` environment variable
- Respect common conventions like `CLICOLOR` and `CLICOLOR_FORCE`
- Consider Windows console API differences for color support
- Emoji detection should consider Unicode support, not just color support
- Provide sensible fallback text for all emoji (e.g., "✓" → "[OK]", "✗" → "[FAIL]")
- Consider supporting `TERM=dumb` as indicator of no formatting support
- Machine-readable formats (JSON) should never include formatting regardless of flags

## Migration and Compatibility

During the prototype phase, we can introduce breaking changes if needed:
- Default behavior changes from "always color/emoji" to "auto-detect"
- Environment variables now affect output (previously ignored)
- New CLI flags may conflict with future planned features

This is acceptable as we're still in prototype phase and users expect potential breaking changes.