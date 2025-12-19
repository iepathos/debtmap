---
number: 266
title: Documentation Pivot for AI Sensor Model
category: foundation
priority: high
status: draft
dependencies: [262, 263, 264, 265]
created: 2024-12-19
---

# Specification 266: Documentation Pivot for AI Sensor Model

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: [262, 263, 264, 265]

## Context

Debtmap's documentation, messaging, and positioning were written for a different product vision - a tool that generates recommendations for humans to follow. With the pivot to "AI sensor" model, all documentation needs to be rewritten to reflect:

1. **New purpose** - Provide signals to AI agents, not recommendations to humans
2. **New value proposition** - Accurate identification and quantification, not fix suggestions
3. **New audience** - AI coding tools and their users, not solo developers
4. **New differentiation** - Context suggestions for AI, not heuristic recommendations

## Objective

Comprehensively update all documentation to reflect the AI sensor pivot:
1. README.md - New elevator pitch and positioning
2. Book/docs - Technical documentation for AI integration
3. Why-debtmap - Value proposition rewrite
4. Architecture docs - Updated data flow diagrams
5. API/output format docs - LLM consumption guide

## Requirements

### Functional Requirements

#### README.md Rewrite

**Current positioning** (to remove):
```markdown
# Debtmap
Technical debt analyzer with actionable recommendations...
Tells you what to do about technical debt...
```

**New positioning**:
```markdown
# Debtmap

**Code complexity sensor for AI-assisted development.**

Debtmap identifies technical debt hotspots and provides the structured
data AI coding tools need to understand and fix them. It doesn't tell
you what to do - it tells AI agents where to look and what signals
matter.

## Why Debtmap?

AI coding assistants (Claude Code, Copilot, Cursor) are transforming
how we write code. But they struggle with technical debt:

- They can't see the whole codebase at once
- They don't know which complex code is tested vs untested
- They can't prioritize what to fix first
- They waste context window on irrelevant code

Debtmap solves this by providing:

1. **Prioritized debt items** - What needs attention, ranked by severity
2. **Quantified signals** - Complexity, coverage, coupling metrics
3. **Context suggestions** - Exactly which files/lines the AI should read
4. **Structured output** - JSON and markdown optimized for LLM consumption

## Quick Start

# Analyze and pipe to Claude Code
debtmap analyze . --format llm-markdown | claude "Fix the top debt item"

# Get structured data for your AI workflow
debtmap analyze . --format json --top 10 > debt.json

# Interactive exploration
debtmap analyze . --tui

## How It Works

Debtmap is a **sensor**, not an oracle. It measures:

| Signal | What It Measures | Why It Matters |
|--------|------------------|----------------|
| Complexity | Cyclomatic, cognitive, nesting | How hard code is to understand |
| Coverage | Test coverage gaps | How risky changes are |
| Coupling | Dependencies, call graph | How changes ripple |
| Entropy | Pattern variety | False positive reduction |
| Purity | Side effects | How testable code is |

These signals are combined into a **severity score** that ranks debt items.
The AI uses these signals + the actual code to decide how to fix it.

## For AI Tool Developers

Debtmap output is designed for machine consumption:

- **Context suggestions** - File ranges the AI should read
- **Deterministic output** - Same input = same output
- **Rich metadata** - All scoring factors exposed
- **Stable IDs** - Reference items across runs

See [LLM Integration Guide](docs/llm-integration.md) for details.
```

#### Why-Debtmap Rewrite (`book/src/why-debtmap.md`)

Completely rewrite to focus on:
1. The AI development acceleration problem
2. Why AI struggles with technical debt
3. How debtmap helps AI succeed
4. Comparison with alternatives (not SonarQube - AI native tools)

Key sections:
- "The AI Development Paradox" - AI creates debt faster, then struggles with it
- "What AI Coding Tools Need" - Context, prioritization, signals
- "What Debtmap Provides" - Sensor data, not recommendations
- "What Debtmap Doesn't Do" - No fix suggestions, no "should" statements

#### Architecture Documentation Updates

Update `book/src/architecture.md`:
- Remove recommendation generation flow
- Add context suggestion generation
- Update data flow diagrams
- Emphasize "pure sensor" mental model

#### New Documentation: LLM Integration Guide

Create `book/src/llm-integration.md`:
```markdown
# LLM Integration Guide

How to use debtmap output in AI coding workflows.

## Output Formats

### LLM Markdown (Recommended)
...

### JSON
...

## Context Suggestions

Each debt item includes a `context` field that tells the AI exactly
what code to read:

Primary: src/file.rs:10-85 (the debt item itself)
Related:
- src/caller.rs:100-120 (Caller) - understands usage
- src/callee.rs:50-80 (Callee) - understands dependencies
- tests/file_test.rs:200-250 (Test) - understands expected behavior

## Example Workflows

### Claude Code Integration
...

### Cursor Integration
...

### Custom Agent Workflow
...

## Interpreting Signals

### Severity Score
What the numbers mean and how to prioritize.

### Complexity Signals
Cyclomatic vs cognitive vs entropy-adjusted.

### Coverage Signals
Direct vs transitive, what gaps indicate.

### Coupling Signals
When high coupling matters vs when it doesn't.
```

#### CLI Help Text Updates

Update help text to reflect new positioning:
```
debtmap analyze [PATH]
    Analyze code for technical debt signals.

    Produces prioritized debt items with metrics and context suggestions
    for AI coding tools. Output is optimized for LLM consumption.

    --format <FORMAT>
        Output format:
        - terminal (default): Human-readable terminal output
        - llm-markdown: LLM-optimized markdown
        - json: Structured JSON for programmatic access
        - markdown: Human-readable markdown report
```

#### Error Messages and User-Facing Strings

Audit and update:
- Remove "recommendation" language
- Remove "should" and "consider" language
- Use factual descriptions only
- Focus on signals and measurements

### Non-Functional Requirements

- All documentation must be technically accurate
- Examples must work with actual tool output
- No "marketing speak" - factual positioning only
- Consistent terminology across all docs

## Acceptance Criteria

- [ ] README.md rewritten with AI sensor positioning
- [ ] book/src/why-debtmap.md completely rewritten
- [ ] book/src/architecture.md updated (no recommendation flow)
- [ ] book/src/llm-integration.md created (new)
- [ ] CLI help text updated
- [ ] All "recommendation" language removed
- [ ] All "should do X" language removed
- [ ] Examples updated to show new output format
- [ ] mdbook builds without errors
- [ ] Links all work
- [ ] Terminology consistent across all docs

## Technical Details

### Documentation Structure

```
book/src/
├── SUMMARY.md              # Update TOC
├── why-debtmap.md          # REWRITE: AI sensor value prop
├── getting-started.md      # UPDATE: New quick start examples
├── architecture.md         # UPDATE: Remove recommendation flow
├── llm-integration.md      # NEW: LLM consumption guide
├── metrics-reference.md    # UPDATE: Signal descriptions
├── coverage-analysis.md    # Minor updates
└── faq.md                  # UPDATE: New FAQ for AI use case
```

### Terminology Changes

| Old Term | New Term |
|----------|----------|
| Recommendation | Signal / Metric |
| Action | (removed) |
| Impact prediction | (removed) |
| Should / Consider | (removed) |
| Fix suggestion | (removed) |
| Actionable | Quantified |
| Refactoring guidance | Context suggestion |

### Key Messages

1. **Debtmap is a sensor** - It measures, it doesn't prescribe
2. **AI does the thinking** - Debtmap provides data, AI decides action
3. **Context is key** - Knowing what to read is as valuable as what to fix
4. **Signals over interpretations** - Raw metrics, not template advice

### Example Rewrites

**Before (recommendation-focused)**:
```markdown
Debtmap tells you exactly what to do: "Split into 5 modules" or
"Add 8 unit tests." Each recommendation includes expected impact
so you know the ROI of each fix.
```

**After (sensor-focused)**:
```markdown
Debtmap identifies the `PurityDetector` god object with 33 methods
and 10 responsibilities. It provides context suggestions: read
lines 10-85 for the core, lines 234-267 for the main caller, and
lines 1500-1600 for existing tests. Your AI coding tool uses these
signals to determine the best refactoring approach.
```

## Dependencies

- **Prerequisites**: [262, 263, 264, 265] (all pivot specs)
- **Affected Components**: All documentation
- **External Dependencies**: None

## Testing Strategy

- **Link Checking**: All documentation links work
- **Build Testing**: mdbook builds successfully
- **Example Testing**: Code examples produce expected output
- **Review**: Manual review for consistency and accuracy

## Documentation Requirements

This IS the documentation spec - meta-documentation not needed.

## Implementation Notes

### Writing Style

- Factual, not prescriptive
- Technical, not marketing
- Specific, not vague
- Humble about limitations

### What NOT to Claim

- "Perfect accuracy"
- "Fixes debt automatically"
- "Understands your code"
- "Replaces code review"
- "AI will follow recommendations"

### What TO Claim

- "Identifies complexity hotspots"
- "Quantifies severity objectively"
- "Provides context for AI consumption"
- "Reduces AI context window waste"
- "Enables prioritized debt remediation"

## Migration and Compatibility

- Documentation changes don't affect tool functionality
- Old bookmarks may break if URLs change
- Changelog should note documentation pivot
- Consider blog post explaining the change
