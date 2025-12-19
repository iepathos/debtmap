---
number: 265
title: TUI Updates for AI Sensor Model
category: foundation
priority: high
status: draft
dependencies: [262, 263]
created: 2024-12-19
---

# Specification 265: TUI Updates for AI Sensor Model

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: [262 - Remove Recommendation Engine, 263 - Context Window Suggestions]

## Context

The TUI currently displays recommendation sections that will be removed per spec 262. Additionally, the TUI should surface the new context suggestions (spec 263) and present data in a way that supports the "AI sensor" model - showing raw signals and metrics rather than interpreted recommendations.

The TUI serves two purposes in the new model:
1. **Human exploration** - Developers browsing debt items to understand their codebase
2. **AI agent debugging** - Understanding what signals debtmap is providing

## Objective

Update the TUI to:
1. Remove recommendation display sections
2. Add context suggestion display
3. Emphasize raw metrics and signals over interpretations
4. Support the "AI sensor" mental model

## Requirements

### Functional Requirements

#### Remove Recommendation Display

1. **Overview Page** (`src/tui/results/detail_pages/overview.rs`):
   - Remove `build_recommendation_section()` function
   - Remove call to it in `render()` function
   - Remove related tests

2. **Text Extraction** (`src/tui/results/actions/text_extraction.rs`):
   - Remove recommendation section from clipboard copy

3. **Data Structures**:
   - Remove or make optional `recommendation` field from `UnifiedDebtItem` display

#### Add Context Suggestions Display

1. **New Context Page** (Page 8 or replace Recommendations on Page 1):
   - Display primary scope with file:line range
   - List related context with relationships
   - Show total lines and completeness confidence
   - Provide copy-to-clipboard for file ranges

2. **Context Section Format**:
   ```
   ┌─ Context to Read ─────────────────────────────┐
   │ Total: 168 lines | Confidence: 85%            │
   │                                               │
   │ PRIMARY                                       │
   │ src/analyzers/purity_detector.rs:10-85       │
   │ └─ PurityDetector::analyze                   │
   │                                               │
   │ RELATED                                       │
   │ [Caller] src/extraction/extractor.rs:234-267 │
   │          └─ extract_purity                   │
   │ [Test]   src/analyzers/purity_detector.rs:   │
   │          1500-1600                           │
   │          └─ test_purity_detection            │
   │ [Module] src/analyzers/purity_detector.rs:1-9│
   │          └─ imports and constants            │
   └───────────────────────────────────────────────┘
   ```

3. **Keyboard Shortcuts**:
   - `c` - Copy all context file ranges to clipboard
   - `p` - Copy primary scope range
   - `Enter` on context item - Copy that specific range

#### Enhance Metrics Display

1. **Overview Page Updates**:
   - More prominent display of complexity breakdown
   - Show all scoring factors (not just final score)
   - Display entropy and dampening clearly
   - Show purity analysis prominently

2. **Score Breakdown Page** (Page 2):
   - Already shows detailed breakdown - ensure it's complete
   - Add any missing factors from new scoring model

3. **New Signals Section** (on Overview or separate):
   ```
   ┌─ Raw Signals ─────────────────────────────────┐
   │ COMPLEXITY                                    │
   │   Cyclomatic: 233 (dampened: 183, factor: 0.5)│
   │   Cognitive: 366                              │
   │   Nesting: 5 levels                           │
   │   Entropy: 0.44 (low variety = repetitive)    │
   │                                               │
   │ COVERAGE                                      │
   │   Direct: 78%                                 │
   │   Transitive: 65%                             │
   │   Uncovered Lines: 506                        │
   │                                               │
   │ COUPLING                                      │
   │   Upstream: 33 callers                        │
   │   Downstream: 20 callees                      │
   │   Instability: 0.38                           │
   │                                               │
   │ PATTERNS                                      │
   │   Type: god_object (92% confidence)           │
   │   Responsibilities: 10 detected               │
   │   Cohesion: 0.28 (low)                        │
   │                                               │
   │ PURITY                                        │
   │   Classification: Impure (95% confidence)     │
   │   Side Effects: mutable ref, HashMap mutation │
   └───────────────────────────────────────────────┘
   ```

#### Update Page Navigation

Current pages:
1. Overview (metrics + recommendations)
2. Score Breakdown
3. Dependencies
4. Git Context
5. Patterns
6. Data Flow
7. Responsibilities

New pages:
1. Overview (metrics only - no recommendations)
2. Score Breakdown (unchanged)
3. Context (NEW - context suggestions)
4. Dependencies (unchanged)
5. Git Context (unchanged)
6. Patterns (unchanged)
7. Data Flow (unchanged)
8. Responsibilities (unchanged)

Or alternatively, add Context as a section within Overview page.

### Non-Functional Requirements

- TUI performance unchanged
- Keyboard navigation unchanged
- Color scheme unchanged
- Responsive to terminal size

## Acceptance Criteria

- [ ] Recommendation section removed from Overview page
- [ ] Recommendation section removed from text extraction
- [ ] Context suggestions displayed on new Context page (or section)
- [ ] All raw signals prominently displayed
- [ ] Scoring breakdown complete with all factors
- [ ] Keyboard shortcuts for copying context ranges
- [ ] Page navigation updated for new structure
- [ ] All existing TUI tests updated
- [ ] New tests for context display
- [ ] `cargo test` passes
- [ ] `cargo clippy` passes

## Technical Details

### Implementation Approach

**Phase 1: Remove Recommendations**
1. Delete `build_recommendation_section()` from overview.rs
2. Remove call from `render()` function
3. Update text extraction to skip recommendations
4. Update tests

**Phase 2: Add Context Display**
1. Create new `context.rs` in `src/tui/results/detail_pages/`
2. Implement context rendering with file ranges
3. Add to page navigation
4. Implement clipboard copy for ranges

**Phase 3: Enhance Metrics Display**
1. Update overview.rs to show all signals
2. Ensure score breakdown is complete
3. Add missing metrics if any

### Files to Modify

```
src/tui/results/detail_pages/
├── mod.rs              # Add context page, update page enum
├── overview.rs         # Remove recommendations, enhance metrics
├── context.rs          # NEW: Context suggestions display
└── score_breakdown.rs  # Ensure complete

src/tui/results/
├── list_view.rs        # May need updates for new columns
└── actions/
    └── text_extraction.rs  # Remove recommendation extraction
```

### Context Page Implementation

```rust
// src/tui/results/detail_pages/context.rs

pub fn render(
    item: &UnifiedDebtItem,
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
) {
    // Header
    let header = format!(
        "Context to Read | {} lines | {}% confidence",
        item.context.total_lines,
        (item.context.completeness_confidence * 100.0) as u32
    );

    // Primary scope
    let primary = &item.context.primary;
    let primary_text = format!(
        "{}:{}-{}",
        primary.file.display(),
        primary.start_line,
        primary.end_line
    );

    // Related contexts
    for related in &item.context.related {
        let rel_text = format!(
            "[{}] {}:{}-{}\n  └─ {}",
            format_relationship(&related.relationship),
            related.range.file.display(),
            related.range.start_line,
            related.range.end_line,
            related.reason
        );
    }
}

fn format_relationship(rel: &ContextRelationship) -> &'static str {
    match rel {
        ContextRelationship::Caller => "Caller",
        ContextRelationship::Callee => "Callee",
        ContextRelationship::TypeDefinition => "Type",
        ContextRelationship::TestCode => "Test",
        ContextRelationship::SiblingMethod => "Sibling",
        ContextRelationship::TraitDefinition => "Trait",
        ContextRelationship::ModuleHeader => "Module",
    }
}
```

### Keyboard Shortcuts

| Key | Action | Page |
|-----|--------|------|
| `c` | Copy all context ranges | Context |
| `p` | Copy primary range | Context |
| `Enter` | Copy selected range | Context |
| `1-8` | Navigate to page | All |
| `Tab` | Next page | All |

### Data Flow

```
UnifiedDebtItem (with context from spec 263)
    ↓
TUI State
    ↓
Page Router (based on current page)
    ↓
├── OverviewPage::render() - metrics, no recommendations
├── ScoreBreakdownPage::render() - unchanged
├── ContextPage::render() - NEW
├── DependenciesPage::render() - unchanged
└── ... other pages
```

## Dependencies

- **Prerequisites**:
  - [262 - Remove Recommendation Engine] (recommendation data gone)
  - [263 - Context Window Suggestions] (context data available)
- **Affected Components**: TUI only
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Each page renders correctly
- **Integration Tests**: Page navigation works
- **Manual Tests**: Visual inspection of all pages
- **User Acceptance**: Information is clear and useful

## Documentation Requirements

- **Code Documentation**: Page documentation updated
- **User Documentation**: TUI navigation guide updated
- **Architecture Updates**: TUI page structure documented

## Implementation Notes

### Graceful Degradation

If `context` field is missing (old data), show placeholder:
```
Context information not available.
Re-run analysis to generate context suggestions.
```

### Terminal Size Handling

Context display should wrap gracefully on narrow terminals:
- Truncate file paths with `...` prefix
- Collapse related items if too many
- Show "more items..." indicator

### Copy to Clipboard

Use existing clipboard infrastructure from text_extraction.rs:
```rust
// Format for AI consumption
fn format_context_for_clipboard(context: &ContextSuggestion) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Primary: {}:{}-{}\n",
        context.primary.file.display(),
        context.primary.start_line,
        context.primary.end_line
    ));
    for rel in &context.related {
        out.push_str(&format!(
            "Related ({}): {}:{}-{}\n",
            format_relationship(&rel.relationship),
            rel.range.file.display(),
            rel.range.start_line,
            rel.range.end_line
        ));
    }
    out
}
```

## Migration and Compatibility

- No breaking changes to TUI interface
- Page numbers may shift (if Context becomes Page 3)
- Existing keyboard shortcuts preserved
- New shortcuts added for context operations
