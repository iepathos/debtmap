---
number: 178
title: Remove Emoji from Progress Output
category: compatibility
priority: high
status: draft
dependencies: []
created: 2025-11-17
---

# Specification 178: Remove Emoji from Progress Output

**Category**: compatibility
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently uses emoji characters in progress indicators and loading messages throughout the analysis workflow. While visually appealing in some terminal environments, emojis cause rendering issues in many Linux terminals, CI/CD systems, and other environments that don't have proper Unicode/emoji support.

Examples of current emoji usage:
- `🔗` Call graph building progress
- `🔍` Trait resolution and file analysis
- `📊` Coverage loading
- `⚙️` Function analysis
- `📁` File analysis
- Spinner characters: `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` (Braille pattern dots)

This follows the common practice in professional Linux tooling (cargo, rustc, git, grep, etc.) which avoids emoji characters to ensure consistent rendering across all terminal environments.

## Objective

Replace all emoji characters in progress indicators, loading messages, and status output with text-based alternatives that render consistently across all terminal environments while maintaining clear, professional output.

## Requirements

### Functional Requirements

1. **Progress Bar Templates** - Update all progress templates in `src/progress.rs`:
   - Replace emoji prefixes with descriptive text labels
   - Maintain template structure for position, length, percentage, and ETA
   - Ensure templates remain clear and easily scannable

2. **Spinner Characters** - Replace Braille pattern dots with ASCII alternatives:
   - Use standard ASCII spinner characters: `|/-\`
   - Maintain smooth visual animation effect
   - Ensure compatibility with all terminal types

3. **Message Formatting** - Update progress messages throughout codebase:
   - Replace any inline emoji with text equivalents
   - Maintain informative status messages
   - Preserve timing and metric information

4. **Template Constants** - Update all template constants:
   - `TEMPLATE_CALL_GRAPH`: "🔗 ..." → text-based
   - `TEMPLATE_TRAIT_RESOLUTION`: "🔍 ..." → text-based
   - `TEMPLATE_COVERAGE`: "📊 ..." → text-based
   - `TEMPLATE_FUNCTION_ANALYSIS`: "⚙️ ..." → text-based
   - `TEMPLATE_FILE_ANALYSIS`: "📁 ..." → text-based
   - `TEMPLATE_SPINNER`: spinner chars → ASCII-based

### Non-Functional Requirements

1. **Backward Compatibility**: Output changes should not break parsing by external tools
2. **Readability**: Text-based alternatives should be equally clear and professional
3. **Performance**: No performance impact from template changes
4. **Consistency**: All progress output should follow the same text-based style

## Acceptance Criteria

- [ ] All emoji characters removed from `src/progress.rs` template constants
- [ ] Braille pattern spinner characters replaced with ASCII spinner (`|/-\`)
- [ ] All progress messages use descriptive text labels instead of emoji
- [ ] Progress output renders correctly in basic POSIX terminals
- [ ] Progress output renders correctly in CI/CD environments (GitHub Actions, etc.)
- [ ] Manual testing confirms clean output on Linux, macOS, and Windows
- [ ] No emoji characters remain in any progress-related output
- [ ] All existing tests continue to pass
- [ ] Documentation updated to reflect new output format

## Technical Details

### Implementation Approach

1. **Update Progress Templates** (`src/progress.rs`):
   ```rust
   // Before:
   pub const TEMPLATE_CALL_GRAPH: &str = "🔗 {msg} {pos}/{len} files ({percent}%) - {eta}";

   // After:
   pub const TEMPLATE_CALL_GRAPH: &str = "[graph] {msg} {pos}/{len} files ({percent}%) - {eta}";
   ```

2. **Update Spinner Characters**:
   ```rust
   // Before:
   .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")

   // After:
   .tick_chars("|/-\\")
   ```

3. **Suggested Text Labels**:
   - Call graph: `[graph]` or `[calls]`
   - Trait resolution: `[traits]` or `[resolve]`
   - Coverage: `[coverage]` or `[cov]`
   - Function analysis: `[funcs]` or `[analyze]`
   - File analysis: `[files]` or `[scan]`

### Files to Modify

1. **`src/progress.rs`** (primary changes):
   - Lines 41-47: Update template constant definitions
   - Line 143: Update spinner tick_chars
   - Line 161: Update counter tick_chars

2. **Additional files with progress messages**:
   - `src/builders/parallel_call_graph.rs`: Line 227 "Enhanced analysis complete"
   - `src/analyzers/rust_call_graph.rs`: Lines 83, 185 (various progress messages)
   - `src/analysis_utils.rs`: Line 62 "Analyzed {} files"
   - `src/priority/call_graph/cross_file.rs`: Line 308 "Resolved cross-file calls"

### Architecture Changes

No architectural changes required. This is purely a cosmetic update to output formatting.

### Data Structures

No data structure changes required.

### APIs and Interfaces

No API changes. Progress bar interface remains identical; only template strings change.

## Dependencies

**Prerequisites**: None

**Affected Components**:
- `src/progress.rs`: Progress template system
- `src/builders/parallel_call_graph.rs`: Call graph building messages
- `src/analyzers/rust_call_graph.rs`: Analysis progress messages
- `src/analysis_utils.rs`: Analysis completion messages
- `src/priority/call_graph/cross_file.rs`: Cross-file resolution messages

**External Dependencies**: None (indicatif library supports all character sets)

## Testing Strategy

### Unit Tests

- Verify template constants contain no emoji characters
- Test spinner character set is ASCII-only
- Validate template string formatting still works correctly

### Integration Tests

- Run full analysis with `--verbose` flags
- Verify progress output in non-TTY environment (pipe to file)
- Test quiet mode still suppresses output correctly

### Manual Testing

1. **Terminal Compatibility**:
   - Test in basic Linux terminal (xterm, console)
   - Test in macOS Terminal.app
   - Test in Windows Terminal and CMD
   - Test in tmux/screen sessions
   - Test in VS Code integrated terminal

2. **CI/CD Testing**:
   - Run in GitHub Actions workflow
   - Verify clean output in CI logs
   - Ensure no rendering artifacts

### Visual Validation

Compare before/after output to ensure:
- Information density remains the same
- Progress is equally readable
- No visual artifacts or formatting issues
- Professional appearance maintained

## Documentation Requirements

### Code Documentation

- Update module-level doc comments in `src/progress.rs` to reflect text-based templates
- Update example code in doc comments if they reference emoji output
- Add comment explaining ASCII spinner choice for compatibility

### User Documentation

- Update any screenshots or examples in `book/` that show progress output
- Document output format if not already documented
- No breaking changes to document (output is informational only)

### Architecture Updates

Not required - no architectural changes.

## Implementation Notes

### Design Principles

1. **Clarity Over Decoration**: Text labels should be clear and unambiguous
2. **Consistency**: Use consistent label format (bracketed tags recommended)
3. **Brevity**: Keep labels short to avoid taking up too much horizontal space
4. **Professionalism**: Follow conventions from established Linux tooling

### Alternative Label Formats

Consider these formatting approaches:

**Bracketed tags** (recommended):
```
[graph] Building call graph 537/537 files (100%) - 0s
[analyze] Analyzed 535 files, 155557 unresolved calls (0s)
```

**Prefixed text**:
```
GRAPH: Building call graph 537/537 files (100%) - 0s
ANALYZE: Analyzed 535 files, 155557 unresolved calls (0s)
```

**Descriptive labels**:
```
Call graph: 537/537 files (100%) - 0s
Analysis: 535 files, 155557 unresolved calls (0s)
```

### Spinner Animation

The ASCII spinner `|/-\` provides smooth animation:
- Frame 1: `|`
- Frame 2: `/`
- Frame 3: `-`
- Frame 4: `\`

This creates a rotating line effect that works in all terminals.

### Testing in Different Environments

**TTY Detection**: The existing TTY detection in `ProgressConfig::should_show_progress()` ensures progress is only shown in interactive terminals. This change improves output in those interactive sessions.

**Quiet Mode**: The `--quiet` flag and `DEBTMAP_QUIET` environment variable continue to work as before.

**Verbosity Levels**: No changes to verbosity level behavior.

## Migration and Compatibility

### Breaking Changes

None. This is purely a visual change to terminal output.

### Backward Compatibility

- No API changes
- No configuration changes
- No behavior changes
- Only visual output format changes

### User Impact

- **Positive**: Better terminal compatibility, consistent rendering
- **Neutral**: Different visual appearance (text vs emoji)
- **Negative**: None expected

### Migration Path

No migration required. Change is immediate and transparent to users.

### Rollback Plan

If issues arise, simply revert the template strings to previous emoji versions. No other changes needed.

## Performance Considerations

No performance impact expected. Template string changes are purely cosmetic and don't affect rendering performance.

## Security Considerations

None. This is a cosmetic change to terminal output.

## Future Enhancements

### Optional Color Support

Consider adding optional color coding to text labels using ANSI color codes:
```rust
// Example (not part of this spec):
pub const TEMPLATE_CALL_GRAPH: &str = "\x1b[34m[graph]\x1b[0m {msg} ...";
```

This could be controlled by:
- `--color` flag (auto/always/never)
- `NO_COLOR` environment variable
- TTY detection

### Configurable Output Formats

Future consideration: Allow users to choose output format via config file:
- `emoji` - Current emoji style
- `text` - Text labels (this spec)
- `minimal` - Absolute minimal output
- `verbose` - More descriptive text

## Related Work

This follows patterns from established tooling:
- **cargo**: Uses text-based progress (Compiling, Finished, etc.)
- **rustc**: Text-based compiler messages
- **git**: No emoji in status or progress output
- **grep/ripgrep**: Text-based result formatting
- **make**: Text-based build progress

## Success Metrics

- Zero emoji rendering issues reported in terminals
- Consistent output across all supported platforms
- No regression in terminal output clarity or usefulness
- Positive user feedback on terminal compatibility
