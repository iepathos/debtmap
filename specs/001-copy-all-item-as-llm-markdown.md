---
number: 1
title: Copy All Item Data as LLM Markdown
category: optimization
priority: high
status: draft
dependencies: []
created: 2026-01-02
---

# Specification 1: Copy All Item Data as LLM Markdown

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

When using debtmap's interactive TUI to review technical debt items, users often want to copy the complete item data to pass to an LLM for fixing. Currently, the `c` key only copies the content of the current detail page. Users must navigate through all 8 pages (Overview, Score Breakdown, Context, Dependencies, Git Context, Patterns, Data Flow, Responsibilities), pressing `c` on each page and pasting the content section by section. This is tedious and error-prone.

The codebase already has comprehensive LLM-optimized markdown output in `src/io/writers/llm_markdown.rs` (Spec 264) which produces machine-parseable markdown designed for AI agent consumption with:
- Hierarchical structure with consistent heading levels
- No decorative elements (emoji, boxes, separators)
- Complete data with all available metrics
- Stable item IDs for reference

This same format should be available via a single keypress in the TUI.

## Objective

Add a new keyboard shortcut in the TUI detail view that copies the complete debt item data as LLM-optimized markdown to the clipboard in a single action, eliminating the need to copy content page-by-page.

## Requirements

### Functional Requirements

1. **New Key Binding**: Add a new key (suggested: `C` - uppercase, since `c` is already used for page copy) that copies all item data as LLM markdown
2. **Format Matching**: The copied content must match the format produced by `LlmMarkdownWriter` in `src/io/writers/llm_markdown.rs`
3. **Complete Data**: Include all sections that would appear in the LLM markdown output:
   - Identification (ID, type, location, function, category)
   - Severity (score, priority, tier)
   - Metrics (cyclomatic, cognitive, entropy, etc.)
   - Coverage (if available)
   - Dependencies (upstream/downstream counts, blast radius, coupling)
   - Purity Analysis (if available)
   - Pattern Analysis (if available)
   - Scoring Breakdown (all multipliers and factors)
   - Context to Read (if available)
   - Git History (if available)
4. **Single Item Focus**: Copy only the currently selected item (not all items in the analysis)
5. **Status Feedback**: Display a status message indicating success (e.g., "Copied LLM markdown to clipboard")

### Non-Functional Requirements

1. **Performance**: Copy operation should complete in under 100ms
2. **Pure Core / Imperative Shell**: Follow existing Stillwater pattern - pure formatting functions, thin I/O shell
3. **Consistency**: Use same formatting logic as CLI `--format llm` output to avoid drift
4. **Error Handling**: Gracefully handle clipboard unavailability (SSH, headless environments)

## Acceptance Criteria

- [ ] Pressing `C` (uppercase) in detail view copies complete item data as LLM markdown
- [ ] Copied content matches format of `debtmap --format llm` for a single item
- [ ] Status bar shows "Copied LLM markdown to clipboard" on success
- [ ] Status bar shows appropriate error message if clipboard unavailable
- [ ] Action works from any detail page (Overview, Context, etc.)
- [ ] Help overlay (`?`) shows the new `C` key binding
- [ ] Unit tests verify the markdown formatting matches expected output
- [ ] Integration test verifies clipboard operation

## Technical Details

### Implementation Approach

1. **Add DetailAction variant**: Extend `DetailAction` enum in `src/tui/results/detail_actions.rs`:
   ```rust
   pub enum DetailAction {
       // ... existing variants ...

       /// Copy complete item as LLM-optimized markdown
       CopyItemAsLlm,
   }
   ```

2. **Add key binding**: Update `classify_detail_key` to handle uppercase `C`:
   ```rust
   KeyCode::Char('C') => Some(DetailAction::CopyItemAsLlm),
   ```

3. **Create pure formatting function**: Add to `src/tui/results/actions/text_extraction.rs`:
   ```rust
   /// Extract complete item data as LLM-optimized markdown.
   /// Uses the same format module as LlmMarkdownWriter for consistency.
   pub fn extract_item_as_llm_markdown(item: &UnifiedDebtItem, app: &ResultsApp) -> String
   ```

4. **Reuse LLM format logic**: Import and reuse the pure `format::*` functions from `src/io/writers/llm_markdown.rs` rather than duplicating formatting logic

5. **Add action handler**: In the detail view event handler, execute:
   ```rust
   DetailAction::CopyItemAsLlm => {
       let content = extract_item_as_llm_markdown(item, app);
       let status = copy_to_clipboard(&content, "LLM markdown");
       app.set_status(status);
   }
   ```

### Architecture Changes

- Expose the `format` module from `src/io/writers/llm_markdown.rs` as `pub mod format` so TUI can reuse it
- Alternatively, move the pure formatting functions to a shared location (e.g., `src/output/llm_format.rs`)

### Data Structures

No new data structures required - reuse existing `UnifiedDebtItem` and formatting infrastructure.

### APIs and Interfaces

- `text_extraction::extract_item_as_llm_markdown(item, app) -> String` - new pure function
- Reuse existing `clipboard::copy_to_clipboard(text, description) -> Result<String>`

## Dependencies

- **Prerequisites**: None - all infrastructure exists
- **Affected Components**:
  - `src/tui/results/detail_actions.rs` - add action variant and key binding
  - `src/tui/results/actions/text_extraction.rs` - add formatting function
  - `src/tui/results/actions/mod.rs` - wire up action handler
  - `src/io/writers/llm_markdown.rs` - expose format module
  - TUI help overlay - document new key
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**:
  - Test `extract_item_as_llm_markdown` produces expected format sections
  - Test key binding `C` maps to `CopyItemAsLlm` action
  - Test action works from all detail pages
- **Integration Tests**:
  - Test full clipboard copy flow (where clipboard is available)
  - Test graceful degradation when clipboard unavailable
- **Performance Tests**: Not required - simple string operations

## Documentation Requirements

- **Code Documentation**: Add doc comments to new functions
- **User Documentation**: Update TUI help text and keyboard shortcuts documentation
- **Architecture Updates**: None needed - follows existing patterns

## Implementation Notes

The key insight is to reuse the existing pure formatting functions from `LlmMarkdownWriter`:

```rust
// In src/io/writers/llm_markdown.rs, the `format` module is already pure:
mod format {
    pub fn identification(location, category) -> String
    pub fn severity(score, priority) -> String
    pub fn metrics(m, adj) -> String
    // ... etc
}
```

By making this module `pub`, the TUI can compose these same functions to format a single item, ensuring the output matches the CLI `--format llm` output exactly.

For file-level items vs function-level items, the implementation should:
1. Check if item is `UnifiedDebtItemOutput::Function` or `::File`
2. Use appropriate formatters (`format::identification` vs `format::file_identification`, etc.)

The `UnifiedDebtItem` from the TUI needs to be converted to `FunctionDebtItemOutput` or `FileDebtItemOutput` first - check how the existing JSON/markdown output does this conversion.

## Migration and Compatibility

No breaking changes. This is purely additive functionality.

The existing `c` (lowercase) behavior for copying page content remains unchanged.
