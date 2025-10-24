---
number: 147
title: Caller/Callee Output Section
category: foundation
priority: high
status: draft
dependencies: [146]
created: 2025-10-24
---

# Specification 147: Caller/Callee Output Section

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 146 (Cross-Module Call Resolution Enhancement)

## Context

Currently, caller/callee information is only visible in verbose mode (`-vv`) as part of the "Dependency Score" line, showing just the count of callers. Users cannot see:
- Which specific functions call a given function
- Which functions are called by a given function
- The call graph relationships at a glance

This makes it difficult to understand function dependencies and impact without verbose output, which includes too much other detail for everyday use.

**User Requirement:**
> "debtmap is supposed to have caller/callee in the output"
> "the callers/callees needs to be accurate"
> "also make sure debtmap output has the callers/callees in it without verbosity it should be its own section outside of the score calculation"

## Objective

Add a dedicated, concise caller/callee section to debtmap's standard output (non-verbose mode) that clearly shows function relationships, making dependency analysis immediately visible without requiring verbose flags.

## Requirements

### Functional Requirements

1. **Caller Display**
   - Show list of functions that call the current function
   - Display function names with file paths for clarity
   - Limit display to top N callers (configurable, default 5)
   - Indicate total count if truncated
   - Handle case where there are 0 callers

2. **Callee Display**
   - Show list of functions called by the current function
   - Display function names with file paths
   - Limit display to top N callees (configurable, default 5)
   - Indicate total count if truncated
   - Handle case where function calls nothing

3. **Formatting**
   - Use consistent, readable formatting
   - Support both ASCII and Unicode output modes
   - Integrate cleanly with existing recommendation output
   - Maintain visual hierarchy and scanability

4. **Filtering and Prioritization**
   - Exclude standard library calls from display
   - Prioritize showing callers/callees in same crate
   - Show most important dependencies first
   - Option to filter by file path pattern

### Non-Functional Requirements

1. **Usability**: Information should be immediately understandable
2. **Performance**: No significant impact on output generation time
3. **Configurability**: Users can control display limits
4. **Consistency**: Match existing debtmap output style

## Acceptance Criteria

- [ ] Caller/callee section appears in standard (non-verbose) output
- [ ] Section shows up to 5 callers and 5 callees by default
- [ ] Functions with 0 callers display "No direct callers detected"
- [ ] Functions calling nothing display "Calls no other functions"
- [ ] Truncated lists show "(showing 5 of 12)" count indicator
- [ ] Function names include file:line for disambiguation
- [ ] Standard library calls are excluded from display
- [ ] ASCII mode works without Unicode characters
- [ ] Output integrates cleanly below complexity/coverage details
- [ ] Configuration options in Config struct for display limits
- [ ] Integration tests verify correct output format
- [ ] Visual alignment matches existing recommendation format

## Technical Details

### Implementation Approach

1. **Phase 1: Data Collection**
   - Extract caller/callee data from existing `DebtItem` fields
   - Use `upstream_caller_names` and `downstream_callee_names`
   - Filter out standard library and external crate calls
   - Sort by relevance (same-file first, then by frequency)

2. **Phase 2: Formatting Logic**
   - Create new formatting functions in `formatter.rs`
   - Add section header with emoji/ASCII variants
   - Format each caller/callee with compact notation
   - Handle truncation and count display

3. **Phase 3: Integration**
   - Add section to `format_recommendation` in `formatter.rs`
   - Position after coverage/complexity, before ACTION
   - Ensure proper indentation and tree structure
   - Test with various caller/callee counts (0, 1, 5, 100+)

### Architecture Changes

**File**: `src/priority/formatter.rs`
- Add `format_caller_callee_section` function
- Add helper `format_function_reference` for compact display
- Add `should_include_in_output` filter logic

**File**: `src/config.rs`
- Add `max_callers_display: usize` (default: 5)
- Add `max_callees_display: usize` (default: 5)
- Add `show_external_calls: bool` (default: false)

**File**: `src/priority/scoring/debt_item.rs`
- Ensure `upstream_caller_names` and `downstream_callee_names` are populated
- Add helper methods for filtering and sorting

### Data Structures

```rust
/// Configuration for caller/callee display
#[derive(Debug, Clone)]
pub struct CallerCalleeConfig {
    /// Maximum number of callers to display (default: 5)
    pub max_callers: usize,

    /// Maximum number of callees to display (default: 5)
    pub max_callees: usize,

    /// Whether to show external crate calls (default: false)
    pub show_external: bool,

    /// Whether to show standard library calls (default: false)
    pub show_std_lib: bool,
}

impl Default for CallerCalleeConfig {
    fn default() -> Self {
        Self {
            max_callers: 5,
            max_callees: 5,
            show_external: false,
            show_std_lib: false,
        }
    }
}
```

### APIs and Interfaces

```rust
/// Format the caller/callee section
pub fn format_caller_callee_section(
    item: &DebtItem,
    config: &CallerCalleeConfig,
    formatter: &OutputFormatter,
) -> Vec<String>;

/// Format a single function reference compactly
pub fn format_function_reference(
    function_name: &str,
    show_file: bool,
) -> String;

/// Filter caller/callee list based on configuration
pub fn filter_dependencies(
    names: &[String],
    config: &CallerCalleeConfig,
) -> Vec<String>;

/// Determine if a function should be included in output
pub fn should_include_in_output(
    function_name: &str,
    config: &CallerCalleeConfig,
) -> bool;
```

### Output Format

```
#2 SCORE: 16.6 [🔴 UNTESTED] [CRITICAL]
├─ LOCATION: ./src/io/writers/enhanced_markdown/health_writer.rs:160 write_quick_wins_section()
├─ WHY: Complex I/O wrapper with 100% gap...
├─ COMPLEXITY: cyclomatic=16 (adj:8), est_branches=16, cognitive=27, nesting=2
├─ COVERAGE: 🔴 UNTESTED - Missing lines: 160-161, 163-164, 166, ...
├─ DEPENDENCIES:
│  ├─ 📞 Called by (1):
│  │     • EnhancedMarkdownWriter::write_executive_summary (mod.rs:126)
│  └─ 📤 Calls (3):
│        • writeln (std - excluded from count)
│        • format_quick_win_item (health_writer.rs:203)
│        • calculate_total_effort (health_writer.rs:185)
├─ ACTION: Add 8 tests for 100% coverage gap...
```

**ASCII Mode:**
```
#2 SCORE: 16.6 [UNTESTED] [CRITICAL]
|- LOCATION: ./src/io/writers/enhanced_markdown/health_writer.rs:160 write_quick_wins_section()
|- WHY: Complex I/O wrapper with 100% gap...
|- COMPLEXITY: cyclomatic=16 (adj:8), est_branches=16, cognitive=27, nesting=2
|- COVERAGE: UNTESTED - Missing lines: 160-161, 163-164, 166, ...
|- DEPENDENCIES:
|  |- Called by (1):
|  |     * EnhancedMarkdownWriter::write_executive_summary (mod.rs:126)
|  +- Calls (3):
|        * writeln (std - excluded from count)
|        * format_quick_win_item (health_writer.rs:203)
|        * calculate_total_effort (health_writer.rs:185)
|- ACTION: Add 8 tests for 100% coverage gap...
```

## Dependencies

- **Prerequisites**: Spec 146 (Cross-Module Call Resolution) - Ensures caller/callee data is accurate
- **Affected Components**:
  - `src/priority/formatter.rs` (add new section)
  - `src/config.rs` (add display configuration)
  - `src/priority/formatter_markdown.rs` (may need updates for markdown output)
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

1. **Formatting Tests** (`src/priority/formatter.rs`)
   ```rust
   #[test]
   fn test_format_caller_callee_with_callers() {
       // Test with 1 caller
       // Test with 5 callers
       // Test with 10+ callers (truncation)
   }

   #[test]
   fn test_format_caller_callee_no_callers() {
       // Verify "No direct callers detected" message
   }

   #[test]
   fn test_format_caller_callee_ascii_mode() {
       // Verify ASCII-only output
   }

   #[test]
   fn test_filter_std_lib_calls() {
       // Verify standard library calls excluded
   }
   ```

2. **Configuration Tests** (`src/config.rs`)
   ```rust
   #[test]
   fn test_caller_callee_config_defaults() {
       // Verify default limits
   }

   #[test]
   fn test_caller_callee_config_from_cli() {
       // Test CLI argument parsing
   }
   ```

### Integration Tests

1. **Output Format Test** (`tests/caller_callee_output_test.rs`)
   - Run debtmap on test codebase
   - Verify caller/callee section present in output
   - Check format matches specification
   - Verify counts are accurate

2. **Cross-Module Test** (`tests/caller_callee_cross_module_test.rs`)
   - Create test files with cross-module calls
   - Verify callers from other modules shown correctly
   - Check file paths are accurate
   - Test truncation with many callers

### User Acceptance

1. **Visual Inspection**
   - Run on debtmap's own codebase
   - Manually verify output is readable and useful
   - Check alignment and visual hierarchy
   - Verify ASCII mode works in plain terminals

2. **Real-World Validation**
   - Run on external Rust projects
   - Gather user feedback on usefulness
   - Identify any confusing output patterns
   - Refine based on feedback

## Documentation Requirements

### Code Documentation

- Document CallerCalleeConfig options and defaults
- Explain filtering logic for std lib and external calls
- Provide examples of output format
- Document truncation behavior

### User Documentation

**README.md sections to update:**
- Output Format → Add caller/callee section example
- Configuration → Document max_callers_display, max_callees_display options
- Examples → Show real output with dependencies section

**CLI Help Text:**
```
--max-callers <N>     Maximum callers to display (default: 5)
--max-callees <N>     Maximum callees to display (default: 5)
--show-external       Include external crate calls in output
```

### Architecture Updates

**ARCHITECTURE.md sections to update:**
- Output Formatting → Document new section structure
- Recommendation Display → Explain dependency visualization

## Implementation Notes

### Filtering Strategy

**Include:**
- Functions in same crate (workspace members)
- Functions in local modules
- Direct dependencies that are meaningful

**Exclude by default:**
- Standard library (std::, core::, alloc::)
- Common utility traits (Iterator, Clone, Debug, etc.)
- Macro-generated code (unless explicitly called)
- External crate functions (optional: can enable with flag)

### Display Prioritization

Order dependencies by importance:
1. Same file calls (highest relevance)
2. Same module calls
3. Other workspace member calls
4. External crate calls (if enabled)

### Truncation Behavior

When truncating, show:
- Most important items first (by priority above)
- Total count: "(showing 5 of 23)"
- Suggestion to use `-vv` for full list

### Edge Cases

- **Recursive calls**: Show as "• self (recursive)"
- **Multiple calls to same function**: Show count: "• foo() ×3"
- **Very long function names**: Truncate to 60 chars with "..."
- **Anonymous closures**: Show as "• <closure> (file:line)"

## Migration and Compatibility

### Breaking Changes

None - this is purely additive functionality.

### Backward Compatibility

- Existing output structure unchanged
- New section integrates seamlessly
- Verbose mode (`-vv`) still shows full details
- Can be disabled with `--no-dependencies` flag (new)

### Configuration Migration

Add new options to Config struct with sensible defaults that maintain current behavior:
```rust
impl Config {
    pub fn default() -> Self {
        Self {
            // ... existing fields ...
            max_callers_display: 5,
            max_callees_display: 5,
            show_external_calls: false,
            show_caller_callee_section: true, // Can disable with --no-dependencies
        }
    }
}
```

## Success Metrics

- **Primary**: Users can see caller/callee relationships without `-vv` flag
- **Secondary**: Dependency section appears in 100% of recommendations with call graph data
- **Tertiary**: User feedback indicates section is useful and readable
- **Performance**: No measurable impact on output generation time

## Related Work

- Spec 146: Cross-Module Call Resolution (provides accurate data)
- Spec 149: Call Graph Debug Tools (helps diagnose data issues)
- Future: Interactive call graph visualization
