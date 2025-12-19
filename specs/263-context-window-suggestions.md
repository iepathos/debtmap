---
number: 263
title: Context Window Suggestions for AI Agents
category: foundation
priority: critical
status: draft
dependencies: [262]
created: 2024-12-19
---

# Specification 263: Context Window Suggestions for AI Agents

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: [262 - Remove Recommendation Engine]

## Context

When an AI agent (Claude Code, Copilot, Cursor) receives a debt item from debtmap, it needs to read the relevant code to understand and fix the issue. Currently, debtmap only provides the location (file:line) but not guidance on what *related* code the AI should also read.

For effective debt remediation, the AI needs:
1. The problematic code itself
2. Functions that call it (upstream)
3. Functions it calls (downstream)
4. Related types and data structures
5. Test files (if any exist)
6. Sibling functions in the same module

This specification adds "context window suggestions" - explicit guidance on what files and line ranges an AI should read to fully understand a debt item before attempting to fix it.

## Objective

Add a `context` field to debt item output that suggests:
1. **Primary scope** - The exact lines to read for the debt item
2. **Dependency context** - Related functions the AI should understand
3. **Test context** - Relevant test files
4. **Structural context** - Module siblings and type definitions

This enables AI agents to efficiently gather the right context without reading entire codebases or making multiple round-trips.

## Requirements

### Functional Requirements

#### New Data Structure: `ContextSuggestion`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSuggestion {
    /// Primary code to read - the debt item itself
    pub primary: FileRange,

    /// Related code that provides necessary context
    pub related: Vec<RelatedContext>,

    /// Estimated total lines to read
    pub total_lines: u32,

    /// Confidence that this context is sufficient (0.0-1.0)
    pub completeness_confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRange {
    pub file: PathBuf,
    pub start_line: u32,
    pub end_line: u32,
    /// Optional: Function/struct name for clarity
    pub symbol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedContext {
    pub range: FileRange,
    pub relationship: ContextRelationship,
    /// Why this context is relevant
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextRelationship {
    /// Functions that call this function
    Caller,
    /// Functions this function calls
    Callee,
    /// Type definitions used by this code
    TypeDefinition,
    /// Test code for this function/module
    TestCode,
    /// Sibling functions in same impl block
    SiblingMethod,
    /// Trait definition this implements
    TraitDefinition,
    /// Module-level context (imports, constants)
    ModuleHeader,
}
```

#### Context Generation Logic

1. **Primary Scope**:
   - For functions: Start 2 lines before function signature, end at closing brace
   - For files/god objects: Include struct definition + all method impl blocks
   - Include relevant doc comments

2. **Caller Context** (limit to top 3 by importance):
   - Direct callers from call graph
   - Prioritize callers in same module, then same crate
   - Include enough lines to see how the function is used

3. **Callee Context** (limit to top 3 by complexity):
   - Functions called by this function
   - Prioritize complex callees that contribute to debt
   - Skip trivial callees (getters, standard library)

4. **Type Context**:
   - Struct/enum definitions for parameters and return types
   - Trait definitions if function implements a trait

5. **Test Context**:
   - Locate test file by naming convention (module_test.rs, tests/module.rs)
   - Find specific tests that reference the function name
   - Include test helper functions if referenced

6. **Sibling Context** (for god objects):
   - Other methods in same impl block
   - Field definitions for the struct

#### Completeness Confidence Calculation

```rust
fn calculate_completeness_confidence(
    has_callers: bool,
    has_callees: bool,
    has_types: bool,
    has_tests: bool,
    unresolved_dependencies: u32,
) -> f32 {
    let base = 0.5;
    let mut confidence = base;

    if has_callers { confidence += 0.1; }
    if has_callees { confidence += 0.1; }
    if has_types { confidence += 0.1; }
    if has_tests { confidence += 0.1; }

    // Penalize unresolved dependencies
    confidence -= (unresolved_dependencies as f32) * 0.05;

    confidence.clamp(0.0, 1.0)
}
```

### Non-Functional Requirements

- Context generation must not significantly slow analysis (< 10% overhead)
- Total suggested lines should be bounded (configurable, default 500 lines max)
- Context should be deterministic for reproducible AI behavior

## Acceptance Criteria

- [ ] `ContextSuggestion` struct defined in `src/priority/context/` module
- [ ] Context generated for each `UnifiedDebtItem`
- [ ] JSON output includes `context` field with all suggested ranges
- [ ] Markdown output includes "Context to Read" section
- [ ] Completeness confidence calculated and included
- [ ] Total lines bounded by configurable limit
- [ ] Call graph data used for caller/callee suggestions
- [ ] Test files detected by naming conventions
- [ ] Integration tests verify context suggestions are valid file ranges
- [ ] `cargo test` passes
- [ ] `cargo clippy` passes

## Technical Details

### Implementation Approach

**Phase 1: Data Structures**
1. Create `src/priority/context/` module
2. Define `ContextSuggestion`, `FileRange`, `RelatedContext` types
3. Add `context: Option<ContextSuggestion>` to `UnifiedDebtItem`

**Phase 2: Primary Scope Generation**
1. Use existing function span data (start_line, end_line)
2. Expand to include doc comments and attributes
3. For god objects, collect all impl blocks

**Phase 3: Dependency Context**
1. Use call graph to find callers/callees
2. Resolve file locations for each dependency
3. Rank and limit to most important

**Phase 4: Test Detection**
1. Implement test file locator based on conventions
2. Parse test files to find tests referencing the function
3. Include test helper functions

**Phase 5: Output Integration**
1. Add context to JSON output
2. Add "Context to Read" section to markdown
3. Update TUI to show context (separate spec 265)

### Architecture Changes

New module: `src/priority/context/`
```
src/priority/context/
├── mod.rs           # Module exports
├── types.rs         # ContextSuggestion, FileRange, etc.
├── generator.rs     # Main context generation logic
├── callers.rs       # Caller context extraction
├── callees.rs       # Callee context extraction
├── types_ctx.rs     # Type definition context
├── tests_ctx.rs     # Test file detection
└── limits.rs        # Line limiting and prioritization
```

### Data Flow (Stillwater "Pure Core" Pattern)

All context generation functions are **pure** - they take data in, return data out, with no I/O:

```
UnifiedDebtItem + CallGraph + Config  (inputs)
    ↓
ContextGenerator::generate(item, call_graph, config)  [PURE]
    ├── extract_primary_scope(item)           [PURE]
    ├── extract_caller_context(item, graph)   [PURE]
    ├── extract_callee_context(item, graph)   [PURE]
    ├── extract_type_context(item, graph)     [PURE]
    ├── extract_test_context(item)            [PURE]
    └── apply_limits(all_contexts, max_lines) [PURE]
    ↓
ContextSuggestion  (output)
```

**I/O happens only at boundaries**:
- Call graph is pre-built (I/O already done)
- File ranges reference paths but don't read files
- Actual file reading happens when AI agent uses the suggestions

### Configuration

```rust
pub struct ContextConfig {
    /// Maximum total lines to suggest
    pub max_total_lines: u32,  // default: 500

    /// Maximum callers to include
    pub max_callers: u32,  // default: 3

    /// Maximum callees to include
    pub max_callees: u32,  // default: 3

    /// Include test context
    pub include_tests: bool,  // default: true

    /// Include type definitions
    pub include_types: bool,  // default: true
}
```

### Example Output

**JSON:**
```json
{
  "location": {
    "file": "src/analyzers/purity_detector.rs",
    "line": 15,
    "function": "PurityDetector::analyze"
  },
  "score": 295,
  "context": {
    "primary": {
      "file": "src/analyzers/purity_detector.rs",
      "start_line": 10,
      "end_line": 85,
      "symbol": "PurityDetector::analyze"
    },
    "related": [
      {
        "range": {
          "file": "src/analyzers/purity_detector.rs",
          "start_line": 1,
          "end_line": 9,
          "symbol": null
        },
        "relationship": "ModuleHeader",
        "reason": "Module imports and constants"
      },
      {
        "range": {
          "file": "src/extraction/extractor.rs",
          "start_line": 234,
          "end_line": 267,
          "symbol": "extract_purity"
        },
        "relationship": "Caller",
        "reason": "Primary caller - orchestrates purity analysis"
      },
      {
        "range": {
          "file": "src/analyzers/purity_detector.rs",
          "start_line": 1500,
          "end_line": 1600,
          "symbol": "test_purity_detection"
        },
        "relationship": "TestCode",
        "reason": "Unit tests for this function"
      }
    ],
    "total_lines": 168,
    "completeness_confidence": 0.85
  }
}
```

**Markdown:**
```markdown
#1 SCORE: 295 [CRITICAL]
├─ LOCATION: ./src/analyzers/purity_detector.rs:15 PurityDetector::analyze()
├─ COMPLEXITY: cyclomatic=233, cognitive=366, nesting=5
├─ COVERAGE: 78%
├─ DEPENDENCIES: 33 upstream, 20 downstream
└─ CONTEXT TO READ (168 lines, 85% confidence):
   Primary: src/analyzers/purity_detector.rs:10-85 (PurityDetector::analyze)
   Related:
   - src/analyzers/purity_detector.rs:1-9 (ModuleHeader) - imports and constants
   - src/extraction/extractor.rs:234-267 (Caller) - orchestrates purity analysis
   - src/analyzers/purity_detector.rs:1500-1600 (TestCode) - unit tests
```

## Dependencies

- **Prerequisites**: [262 - Remove Recommendation Engine] (recommendation field replacement)
- **Affected Components**: Output formatters, TUI, JSON schema
- **External Dependencies**: None (uses existing call graph)

## Testing Strategy

- **Unit Tests**: Context generation for various debt item types
- **Integration Tests**: Verify suggested file ranges are valid
- **Property Tests**: Context total lines always within bounds
- **User Acceptance**: AI agents can successfully use context to understand debt

## Documentation Requirements

- **Code Documentation**: Explain context suggestion algorithm
- **User Documentation**: How AI agents should consume context
- **Architecture Updates**: Add context generation to analysis pipeline diagram

## Implementation Notes

### Stillwater Design Principles

1. **Pure Functions**: All `extract_*` functions are pure - same inputs always produce same outputs
2. **Composition**: Complex context built from simple, focused extractors
3. **Types Guide**: `ContextSuggestion` type makes it clear what data is available
4. **Fail Completely for Validation**: If implementing validation (e.g., "are all ranges valid?"), use `Validation<T, Vec<Error>>` to collect all issues, not fail-fast

### Handling Missing Data

- If call graph unavailable, skip caller/callee context (reduce confidence)
- If test files not found, skip test context (reduce confidence)
- Always include primary scope

### Performance Considerations

- Context generation should be lazy (only when outputting)
- Cache file line counts to avoid repeated file reads
- Use existing call graph data, don't re-analyze

### Line Range Validation

- Validate all generated ranges against actual file contents
- Clamp to file bounds if necessary
- Log warnings for invalid ranges in debug mode

## Migration and Compatibility

- New `context` field is additive (backward compatible)
- Existing JSON consumers can ignore the field
- CLI flag `--no-context` to disable for performance
