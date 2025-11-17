---
number: 181
title: Relocate File Context Tag from Location Line
category: foundation
priority: medium
status: draft
dependencies: [166]
created: 2025-11-16
---

# Specification 181: Relocate File Context Tag from Location Line

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 166 (Test File Detection and Context-Aware Scoring)

## Context

Spec 166 introduced file context detection to classify files as `PRODUCTION`, `TEST FILE`, `PROBABLE TEST`, `GENERATED`, `CONFIG`, or `DOCS`. This classification enables context-aware scoring adjustments (e.g., test files receive 80% score reduction to avoid false positives).

Currently, the file context tag is appended to the end of the `LOCATION` line in debtmap output:

```
├─ LOCATION: ./src/state_reconciliation.rs:81 reconcile_state() [PRODUCTION]
├─ IMPACT: -4 complexity, -1.5 risk
├─ COMPLEXITY: cyclomatic=9 (dampened: 4, factor: 0.51), ...
├─ WHY THIS MATTERS: Approaching complexity threshold (9/16). ...
├─ RECOMMENDED ACTION: Reduce complexity from 9 to ~10
```

### Problem Statement

The `[PRODUCTION]` tag on the location line creates several UX issues:

1. **Semantic Mismatch**: The tag describes *how the score was calculated*, not the function's location. It's metadata about scoring logic, not location information.

2. **Visual Clutter**: The location line becomes crowded and harder to scan:
   - Primary information: file path, line number, function name
   - Secondary metadata: file context tag
   - These should be visually separated

3. **Inconsistent Information Hierarchy**:
   - Location line mixes navigation info (path:line) with scoring metadata (context tag)
   - Users scanning for file locations must parse through irrelevant context tags

4. **Limited Visibility Control**:
   - Context tags are always shown, even in default verbosity
   - Users have no way to hide this metadata when it's not relevant
   - More appropriate for verbose output where scoring details are shown

### User Feedback

> "I think having it at the end of the location line doesn't make any sense. This should either be its own section or be something shown for scoring when we do verbose output we get the scoring breakdown."

## Objective

Relocate the file context tag to a more semantically appropriate location in the output, separating location information from scoring metadata, with appropriate visibility control based on verbosity level.

## Requirements

### Functional Requirements

1. **Remove context tag from location line** in all output formats (tree-style, markdown, unified)
2. **Add dedicated context section** OR **include in scoring breakdown** (depending on verbosity)
3. **Maintain visibility in verbose mode** where scoring details are shown
4. **Hide in default mode** for production files (since `[PRODUCTION]` is the default/expected state)
5. **Show in default mode** for non-production contexts (`TEST FILE`, `GENERATED`, etc.) to alert users about score adjustments

### Non-Functional Requirements

- Clean location line focused solely on navigation information
- Clear visual separation between location and scoring metadata
- Consistent presentation across all output formatters
- Backward compatible with existing test files that validate output format

## Acceptance Criteria

### Verbosity Level 0 (Default)

- [ ] Location line contains only: file path, line number, function name (no context tag)
- [ ] `[PRODUCTION]` context tag is **not shown** (default/expected state)
- [ ] Non-production context tags (`[TEST FILE]`, `[GENERATED]`, etc.) **are shown** in a dedicated section
- [ ] Dedicated section appears after COMPLEXITY or within scoring breakdown
- [ ] Users can quickly identify test/generated files affecting scores

### Verbosity Level 1+ (Verbose)

- [ ] All context tags shown in scoring breakdown section
- [ ] Context tag includes explanation: `File Context: PRODUCTION (no score adjustment)` or `File Context: TEST FILE (80% score reduction)`
- [ ] Scoring breakdown shows context reduction factor: `Context Factor: 1.0` or `Context Factor: 0.2`
- [ ] Clear connection between context and final score calculation

### All Output Formats

- [ ] Tree-style formatter updated (src/priority/formatter_verbosity.rs)
- [ ] Markdown formatter updated (src/priority/formatter_markdown.rs)
- [ ] Unified output format updated if applicable (src/output/unified.rs)
- [ ] Consistent presentation across all formatters

### Testing

- [ ] Existing tests updated to match new output format
- [ ] Tests validate context tag not on location line
- [ ] Tests validate context section presence/absence based on verbosity
- [ ] Tests validate production vs non-production tag visibility

## Technical Details

### Implementation Approach

#### 1. Location Line Cleanup

**File**: `src/priority/formatter_verbosity.rs:648-657`

```rust
// BEFORE
writeln!(
    output,
    "├─ {} {}:{} {}(){}",
    "LOCATION:".bright_blue(),
    item.location.file.display(),
    item.location.line,
    item.location.function.bright_green(),
    file_context_tag.bright_magenta()  // REMOVE THIS
)
.unwrap();

// AFTER
writeln!(
    output,
    "├─ {} {}:{} {}()",
    "LOCATION:".bright_blue(),
    item.location.file.display(),
    item.location.line,
    item.location.function.bright_green()
)
.unwrap();
```

**File**: `src/priority/formatter_markdown.rs:803-812`

```rust
// BEFORE
writeln!(
    output,
    "**Type:** {} | **Location:** `{}:{} {}(){}`",
    format_debt_type(&item.debt_type),
    item.location.file.display(),
    item.location.line,
    item.location.function,
    file_context_tag  // REMOVE THIS
)
.unwrap();

// AFTER
writeln!(
    output,
    "**Type:** {} | **Location:** `{}:{} {}()`",
    format_debt_type(&item.debt_type),
    item.location.file.display(),
    item.location.line,
    item.location.function
)
.unwrap();
```

#### 2. Add Context Section (Default Verbosity)

**When to Show**: Only for non-production contexts (test files, generated files, etc.)

**File**: `src/priority/formatter_verbosity.rs` (after COMPLEXITY section, ~line 670)

```rust
// After COMPLEXITY section, before WHY THIS MATTERS
if let Some(ref context) = item.file_context {
    // Only show non-production contexts in default mode
    if !matches!(context, FileContext::Production) {
        let reduction_pct = ((1.0 - context_reduction_factor(context)) * 100.0) as u32;
        writeln!(
            output,
            "├─ {} {} ({}% score reduction)",
            "FILE CONTEXT:".bright_blue(),
            context_label(context).bright_magenta(),
            reduction_pct
        )
        .unwrap();
    }
}
```

**Example Output** (default mode, test file):
```
├─ LOCATION: ./tests/integration_test.rs:42 test_workflow()
├─ IMPACT: -2 complexity, -0.3 risk
├─ COMPLEXITY: cyclomatic=5 (dampened: 1, factor: 0.20), ...
├─ FILE CONTEXT: TEST FILE (80% score reduction)
├─ WHY THIS MATTERS: ...
```

**Example Output** (default mode, production file):
```
├─ LOCATION: ./src/executor.rs:81 execute_command()
├─ IMPACT: -4 complexity, -1.5 risk
├─ COMPLEXITY: cyclomatic=9 (dampened: 4, factor: 0.51), ...
├─ WHY THIS MATTERS: ...
```

#### 3. Enhanced Scoring Breakdown (Verbose Mode)

**File**: `src/priority/formatter_verbosity.rs` (in verbose scoring section)

Add to scoring breakdown when `verbosity >= 1`:

```rust
if verbosity >= 1 {
    writeln!(output, "").unwrap();
    writeln!(output, "├─ {}", "SCORING BREAKDOWN:".bright_blue()).unwrap();

    // ... existing scoring details ...

    // Add file context scoring
    if let Some(ref context) = item.file_context {
        let factor = context_reduction_factor(context);
        let label = context_label(context);
        let explanation = match context {
            FileContext::Production => "no score adjustment",
            FileContext::Test { confidence, .. } if *confidence > 0.8 => "80% score reduction",
            FileContext::Test { confidence, .. } if *confidence > 0.5 => "40% score reduction",
            FileContext::Generated { .. } => "90% score reduction",
            _ => "no adjustment",
        };

        writeln!(
            output,
            "│  ├─ File Context: {} ({})",
            label.bright_magenta(),
            explanation
        )
        .unwrap();

        writeln!(
            output,
            "│  ├─ Context Factor: {:.2}",
            factor
        )
        .unwrap();
    }
}
```

**Example Output** (verbose mode):
```
├─ SCORING BREAKDOWN:
│  ├─ Base Score: 4.15
│  ├─ Complexity Weight: 0.60
│  ├─ Risk Weight: 0.40
│  ├─ File Context: TEST FILE (80% score reduction)
│  ├─ Context Factor: 0.20
│  └─ Final Score: 0.83
```

### Architecture Changes

**No structural changes required**. This is purely a presentation layer refactoring:

1. Remove context tag formatting from location line builders
2. Add conditional context section rendering
3. Enhance scoring breakdown with context details (verbose mode)

### Data Flow

```
FileContext (from analysis)
    ↓
UnifiedDebtItem.file_context (existing)
    ↓
Formatter (verbosity-aware)
    ↓
Output:
  - Default: Show non-production contexts in dedicated section
  - Verbose: Show all contexts in scoring breakdown
```

### Affected Components

- `src/priority/formatter_verbosity.rs`: Tree-style output (primary format)
- `src/priority/formatter_markdown.rs`: Markdown output (detailed reports)
- `src/output/unified.rs`: Unified data structure (stores context, no change needed)
- `src/priority/scoring/file_context_scoring.rs`: Scoring logic (no change needed)

## Dependencies

- **Spec 166**: Introduced file context detection and scoring adjustments
  - This spec refactors presentation only, not detection logic
  - All file_context infrastructure remains unchanged

## Testing Strategy

### Unit Tests

**File**: `src/priority/formatter_verbosity.rs` (tests module)

```rust
#[test]
fn test_location_line_no_context_tag() {
    let item = create_test_debt_item_with_context(FileContext::Production);
    let output = format_item(&item, 0);

    // Verify location line is clean
    assert!(output.contains("LOCATION: ./src/foo.rs:42 bar()"));
    assert!(!output.contains("[PRODUCTION]"));
}

#[test]
fn test_context_section_shown_for_test_files_default_mode() {
    let item = create_test_debt_item_with_context(FileContext::Test {
        confidence: 0.95,
        test_framework: Some("rust-std".to_string()),
        test_count: 10,
    });
    let output = format_item(&item, 0);

    // Verify context section appears
    assert!(output.contains("FILE CONTEXT: TEST FILE (80% score reduction)"));
    assert!(!output.contains("LOCATION:") && output.contains("[TEST FILE]")); // Not on location line
}

#[test]
fn test_context_hidden_for_production_default_mode() {
    let item = create_test_debt_item_with_context(FileContext::Production);
    let output = format_item(&item, 0);

    // Verify no context section for production
    assert!(!output.contains("FILE CONTEXT:"));
    assert!(!output.contains("[PRODUCTION]"));
}

#[test]
fn test_context_in_scoring_breakdown_verbose() {
    let item = create_test_debt_item_with_context(FileContext::Generated {
        generator: "protobuf".to_string(),
    });
    let output = format_item(&item, 1);

    // Verify context in scoring breakdown
    assert!(output.contains("SCORING BREAKDOWN:"));
    assert!(output.contains("File Context: GENERATED (90% score reduction)"));
    assert!(output.contains("Context Factor: 0.10"));
}
```

### Integration Tests

**File**: `tests/output_format_test.rs` (new file)

```rust
#[test]
fn test_end_to_end_output_production_file() {
    let result = run_debtmap_on_fixture("production_complex.rs");

    // Verify clean location lines
    assert!(result.stdout.contains("LOCATION: ./src/executor.rs:81 execute()"));
    assert!(!result.stdout.contains("[PRODUCTION]"));

    // Verify no context section in default mode
    assert!(!result.stdout.contains("FILE CONTEXT:"));
}

#[test]
fn test_end_to_end_output_test_file() {
    let result = run_debtmap_on_fixture("integration_test.rs");

    // Verify clean location line
    assert!(result.stdout.contains("LOCATION: ./tests/integration_test.rs:42 test_workflow()"));
    assert!(!result.stdout.contains("[TEST FILE]") || !result.stdout.contains("LOCATION:")); // Tag not on location line

    // Verify context section shown
    assert!(result.stdout.contains("FILE CONTEXT: TEST FILE (80% score reduction)"));
}

#[test]
fn test_verbose_mode_shows_all_contexts() {
    let result = run_debtmap_with_verbosity("mixed_files.rs", 1);

    // Verify scoring breakdown includes context
    assert!(result.stdout.contains("SCORING BREAKDOWN:"));
    assert!(result.stdout.contains("File Context:"));
    assert!(result.stdout.contains("Context Factor:"));
}
```

### Regression Tests

Update existing tests that validate output format:

```bash
# Find tests that check for context tags on location line
rg --type rust "LOCATION.*PRODUCTION" tests/
rg --type rust "LOCATION.*TEST FILE" tests/

# Update assertions to match new format
```

## Documentation Requirements

### Code Documentation

- Update formatter module documentation to explain verbosity-based context display
- Add inline comments explaining when context section is shown vs hidden
- Document scoring breakdown enhancement in verbose mode

### User Documentation

**README.md** or **USAGE.md** update:

```markdown
## Understanding Debtmap Output

### File Context Tags

Debtmap classifies files to apply context-aware scoring:

- **Production Code**: Default, no score adjustment
- **Test Files**: 80% score reduction (high confidence) or 40% (probable test)
- **Generated Files**: 90% score reduction
- **Config/Docs**: No adjustment

**Default Output**: Non-production contexts shown with score reduction:
```
├─ FILE CONTEXT: TEST FILE (80% score reduction)
```

**Verbose Output** (`-v`): All contexts shown in scoring breakdown:
```
├─ SCORING BREAKDOWN:
│  ├─ File Context: PRODUCTION (no score adjustment)
│  ├─ Context Factor: 1.0
```

Production files don't show context tags in default mode (expected state).
```

### Architecture Updates

No ARCHITECTURE.md changes needed (presentation layer only).

## Implementation Notes

### Migration Strategy

1. **Phase 1**: Update formatters to remove context tag from location line
2. **Phase 2**: Add context section for non-production files (default mode)
3. **Phase 3**: Enhance scoring breakdown with context details (verbose mode)
4. **Phase 4**: Update all tests to match new format
5. **Phase 5**: Update documentation

### Backward Compatibility

**Breaking Change**: Output format changes

**Impact**:
- Users parsing debtmap output will need to update parsers
- Existing snapshots/golden files will need updates
- Scripts expecting `[PRODUCTION]` on location line will break

**Mitigation**:
- Version bump (0.3.5 → 0.4.0) to indicate breaking change
- Add CHANGELOG.md entry explaining output format change
- Provide migration guide for users parsing output

### Edge Cases

1. **Files without context**: Should not happen (Production is default), but handle gracefully
2. **Low confidence test files** (0.3-0.5): Treated as Production, no tag shown
3. **Multiple contexts** (not currently supported): Design allows future extension
4. **Unicode in paths**: Already handled by existing formatters

### Performance Considerations

- Negligible: Only adds conditional formatting logic
- No additional analysis or computation required
- Context already computed during scoring phase

## Alternative Approaches Considered

### Option 1: Keep on Location Line (Rejected)

**Reasoning**: Semantic mismatch between location and scoring metadata

### Option 2: Always Show in Dedicated Section (Rejected)

**Reasoning**: `[PRODUCTION]` is noise in default output (it's the expected state)

### Option 3: Only Show in Verbose Mode (Rejected)

**Reasoning**: Users need to see test/generated file contexts in default mode to understand score reductions

### Option 4: Show as Prefix Instead of Suffix (Rejected)

```
├─ LOCATION: [PRODUCTION] ./src/foo.rs:42 bar()
```

**Reasoning**: Still clutters location line, doesn't solve semantic mismatch

### Selected Approach: Context Section + Verbosity Control

**Benefits**:
- Clean location line focused on navigation
- Non-production contexts visible by default (alerts users to score adjustments)
- Production context hidden by default (reduces noise)
- Verbose mode provides full scoring transparency
- Semantically appropriate separation of concerns

## Success Metrics

### User Experience

- Developers can quickly scan location lines without metadata clutter
- Non-production file contexts are immediately visible when relevant
- Verbose mode provides complete scoring transparency

### Code Quality

- Clear separation of location and scoring metadata in formatters
- Consistent presentation across all output formats
- Maintainable and extensible for future context types

### Backward Compatibility

- Clean migration path with clear version bump
- Documentation guides users through output format change
- Tests validate new format comprehensively

## Future Enhancements

### Possible Extensions

1. **Color-coded context tags**: Different colors for test/generated/production
2. **Context icons**: Visual indicators instead of text tags
3. **Configurable visibility**: User preference for showing/hiding contexts
4. **Context statistics**: Summary of files by context type
5. **Machine-readable output**: JSON format with structured context metadata

### Compatibility with Future Specs

- Design supports additional context types (e.g., `BENCHMARK`, `EXAMPLE`)
- Scoring breakdown section extensible for additional factors
- Verbosity levels allow gradual information disclosure

## Related Issues

- Spec 166: Test File Detection and Context-Aware Scoring (dependency)
- User feedback: "Having it at the end of the location line doesn't make any sense"

## Conclusion

This specification addresses the UX issue of file context tags cluttering location lines by:

1. **Removing context tags from location lines** across all formatters
2. **Adding dedicated context section** for non-production files (default mode)
3. **Hiding production context** to reduce noise (it's the expected state)
4. **Enhancing scoring breakdown** with context details (verbose mode)

This creates a cleaner, more semantically appropriate output format while maintaining visibility of important score adjustments from test and generated file contexts.
