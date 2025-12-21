---
number: 8
title: Consolidate Markdown Output Formats
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-12-21
---

# Specification 008: Consolidate Markdown Output Formats

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: none

## Context

Debtmap currently has two separate markdown output formats:

1. **`markdown`** (`--format markdown`) - Basic markdown writer (`src/io/writers/markdown/core.rs`, ~271 lines)
   - Simpler, less detailed output
   - Uses `MarkdownWriter` struct
   - Implements basic `OutputWriter` trait

2. **`llm-markdown`** (`--format llm-markdown`) - LLM-optimized markdown (`src/io/writers/llm_markdown.rs`, ~1149 lines)
   - Comprehensive, detailed output designed for LLM consumption
   - Includes entropy analysis, cohesion metrics, anti-patterns
   - Better structured for both human and AI reading
   - Implements `UnifiedOutput` format

Having two markdown formats creates:
- User confusion about which format to use
- Maintenance burden of two separate code paths
- Inconsistent feature coverage between formats
- Unnecessary complexity in CLI argument handling

The `llm-markdown` format is objectively superior - it has all the features of basic markdown plus additional analysis details. There's no use case where the basic markdown format would be preferred.

## Objective

Consolidate the two markdown output formats into a single `markdown` format that uses the LLM-optimized implementation, eliminating the redundant basic markdown writer.

## Requirements

### Functional Requirements

1. **Remove `llm-markdown` CLI option** - The `--format llm-markdown` option should be removed
2. **Upgrade `markdown` to use LLM implementation** - `--format markdown` should produce the current `llm-markdown` output
3. **Maintain backward compatibility** - Existing scripts using `--format markdown` continue to work (but get better output)
4. **Remove dead code** - Delete the unused basic markdown writer code

### Non-Functional Requirements

1. **No regression in output quality** - The consolidated markdown output must include all features currently in `llm-markdown`
2. **Clean deprecation** - If `llm-markdown` format is used, provide a helpful message that it's now just `markdown`
3. **Documentation update** - Update help text and documentation to reflect the change

## Acceptance Criteria

- [ ] `--format markdown` produces the same output as current `--format llm-markdown`
- [ ] `--format llm-markdown` either:
  - (Option A) Works as alias to `markdown` with deprecation warning, OR
  - (Option B) Is removed entirely with clear error message
- [ ] `src/io/writers/markdown/core.rs` basic writer is removed or deprecated
- [ ] CLI help text updated to describe markdown format capabilities
- [ ] All tests pass with the consolidated format
- [ ] No references to `LlmMarkdown` as a separate output format in user-facing code

## Technical Details

### Files to Modify

1. **`src/cli/args.rs`**
   - Remove `LlmMarkdown` variant from `OutputFormat` enum
   - Or keep as hidden alias for backward compatibility

2. **`src/io/output.rs`**
   - Update `OutputFormat` enum
   - Modify `create_writer()` to use LLM markdown for `Markdown` format

3. **`src/io/writers/mod.rs`**
   - Update re-exports
   - Consider renaming `LlmMarkdownWriter` to `MarkdownWriter`

4. **`src/io/writers/llm_markdown.rs`**
   - Rename to `markdown.rs` or integrate into `markdown/` module
   - Update struct names if needed

5. **`src/io/writers/markdown/`**
   - Evaluate what to keep (formatters, risk, etc. may be reusable)
   - Remove or deprecate `core.rs` basic writer

6. **Various command handlers**
   - Update format matching in `src/commands/analyze/pipeline.rs`
   - Update `src/commands/state.rs`
   - Update other files that switch on output format

### Migration Strategy

**Option A: Soft Deprecation**
```rust
pub enum OutputFormat {
    Json,
    Markdown,  // Now uses LLM-optimized implementation
    Terminal,
    Html,
    Dot,
    #[deprecated(note = "Use 'markdown' instead")]
    LlmMarkdown,  // Hidden alias, prints deprecation warning
}
```

**Option B: Hard Removal**
```rust
pub enum OutputFormat {
    Json,
    Markdown,  // Uses LLM-optimized implementation
    Terminal,
    Html,
    Dot,
    // LlmMarkdown removed entirely
}
```

Recommended: **Option A** for one release cycle, then **Option B**.

### Implementation Approach

1. First, wire `Markdown` format to use `LlmMarkdownWriter`
2. Add deprecation warning when `LlmMarkdown` is used
3. Update all documentation and help text
4. Remove basic markdown writer code
5. Optionally rename `LlmMarkdownWriter` to `MarkdownWriter`

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - CLI argument parsing
  - Output writer selection
  - Various command handlers
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Ensure markdown output contains all expected sections
- **Integration Tests**: Verify `--format markdown` produces expected output
- **Regression Tests**: Compare output before/after to ensure no loss of information
- **CLI Tests**: Test both `markdown` format works correctly

## Documentation Requirements

- **Code Documentation**: Update module-level docs for markdown writer
- **User Documentation**: Update CLI help text
- **Architecture Updates**: Update any architecture docs mentioning output formats

## Implementation Notes

- The `EnhancedMarkdownWriter` trait in `markdown/enhanced.rs` may still be useful
- Some formatting utilities in `markdown/formatters.rs` may be worth keeping
- The `markdown/risk.rs` and `markdown/testing.rs` modules may integrate with LLM writer
- Consider keeping helpful pure functions even if the main writer is replaced

## Migration and Compatibility

- Scripts using `--format llm-markdown` should continue to work (with deprecation warning)
- Scripts using `--format markdown` will get improved output (no breaking change in semantics)
- JSON output format is unaffected
- Terminal output format is unaffected
