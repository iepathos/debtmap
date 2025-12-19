---
number: 264
title: LLM-Optimized Output Format
category: foundation
priority: critical
status: draft
dependencies: [262, 263]
created: 2024-12-19
---

# Specification 264: LLM-Optimized Output Format

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: [262 - Remove Recommendation Engine, 263 - Context Window Suggestions]

## Context

Debtmap's primary consumers are shifting from humans reading terminal output to AI agents (Claude Code, Copilot, Cursor) that need structured data to make decisions. LLMs actually parse markdown very effectively - often better than JSON for understanding context and relationships.

The current output formats were designed for human consumption with visual formatting. This specification creates LLM-optimized output where:

1. **Markdown is primary** - LLMs excel at understanding markdown structure
2. **JSON is equivalent** - Same information, structured for programmatic access
3. **Both formats are AI-first** - Designed for machine consumption, not human readability
4. **No recommendations** - Raw signals only (per spec 262)
5. **Rich context** - Include everything an AI needs to understand and fix the issue

## Objective

Create a unified LLM-optimized output format that:
1. Provides complete information in both markdown and JSON
2. Uses consistent structure between formats
3. Eliminates human-friendly but machine-confusing formatting
4. Includes all raw signals without interpretation
5. Supports context suggestions (per spec 263)

## Requirements

### Functional Requirements

#### LLM Markdown Format

The markdown format should be:
- Hierarchical with consistent heading levels
- Machine-parseable with predictable patterns
- Free of decorative elements (emoji, boxes, separators)
- Complete with all available data

**Structure:**
```markdown
# Debtmap Analysis Report

## Metadata
- Version: 0.9.2
- Generated: 2024-12-19T10:30:00Z
- Project: /path/to/project
- Total Files Analyzed: 150
- Total Functions Analyzed: 1200

## Summary
- Total Debt Score: 13981
- Debt Density: 107.9 per 1K LOC
- Total LOC: 129602
- Items by Severity:
  - Critical: 5
  - High: 12
  - Medium: 45
  - Low: 89

## Debt Items

### Item 1

#### Identification
- ID: purity_detector_rs_15
- Type: Function
- Location: src/analyzers/purity_detector.rs:15
- Function: PurityDetector::analyze
- Category: God Object

#### Severity
- Score: 295
- Priority: Critical
- Tier: Critical (>=100)

#### Metrics
- Cyclomatic Complexity: 233
- Cognitive Complexity: 366
- Nesting Depth: 5
- Lines of Code: 2301
- Entropy Score: 0.44
- Dampening Factor: 0.50
- Dampened Cyclomatic: 183

#### Coverage
- Direct Coverage: 78%
- Transitive Coverage: 65%
- Uncovered Lines: 506

#### Dependencies
- Upstream Callers: 33
- Downstream Callees: 20
- Top Callers:
  - purity_detector.rs:test_mutable_static_is_impure
  - purity_detector.rs:PurityDetector::handle_macro
  - extraction/extractor.rs:extract_purity
- Top Callees:
  - rules.rs:ContextMatcher::any
  - lazy.rs:LazyPipeline::collect
  - macro_definition_collector.rs:collect_definitions

#### Purity Analysis
- Is Pure: false
- Confidence: 0.95
- Detected Side Effects:
  - Mutable reference parameter
  - HashMap mutation

#### Pattern Analysis
- Pattern Type: god_object
- Pattern Confidence: 0.92
- Responsibilities Detected: 10
- Methods: 33
- Fields: 14
- Cohesion Score: 0.28

#### Scoring Breakdown
- Base Score: 250
- Complexity Factor: 0.85 (weight: 0.4)
- Coverage Factor: 0.22 (weight: 0.3)
- Dependency Factor: 0.65 (weight: 0.2)
- Role Multiplier: 1.0 (PureLogic)
- Adjustments:
  - God Object: +45

#### Context to Read
- Total Lines: 168
- Completeness Confidence: 0.85
- Primary:
  - src/analyzers/purity_detector.rs:10-85 (PurityDetector::analyze)
- Related:
  - src/analyzers/purity_detector.rs:1-9 (ModuleHeader)
  - src/extraction/extractor.rs:234-267 (Caller: extract_purity)
  - src/analyzers/purity_detector.rs:1500-1600 (TestCode)

---

### Item 2
[...]
```

#### JSON Format (Equivalent Information)

```json
{
  "format_version": "3.0",
  "metadata": {
    "debtmap_version": "0.9.2",
    "generated_at": "2024-12-19T10:30:00Z",
    "project_root": "/path/to/project",
    "files_analyzed": 150,
    "functions_analyzed": 1200
  },
  "summary": {
    "total_debt_score": 13981,
    "debt_density": 107.9,
    "total_loc": 129602,
    "severity_distribution": {
      "critical": 5,
      "high": 12,
      "medium": 45,
      "low": 89
    }
  },
  "items": [
    {
      "id": "purity_detector_rs_15",
      "type": "Function",
      "location": {
        "file": "src/analyzers/purity_detector.rs",
        "line": 15,
        "function": "PurityDetector::analyze"
      },
      "category": "God Object",
      "severity": {
        "score": 295,
        "priority": "Critical",
        "tier": "Critical"
      },
      "metrics": {
        "cyclomatic_complexity": 233,
        "cognitive_complexity": 366,
        "nesting_depth": 5,
        "lines_of_code": 2301,
        "entropy_score": 0.44,
        "dampening_factor": 0.50,
        "dampened_cyclomatic": 183
      },
      "coverage": {
        "direct": 0.78,
        "transitive": 0.65,
        "uncovered_lines": 506
      },
      "dependencies": {
        "upstream_count": 33,
        "downstream_count": 20,
        "top_callers": [
          "purity_detector.rs:test_mutable_static_is_impure",
          "purity_detector.rs:PurityDetector::handle_macro",
          "extraction/extractor.rs:extract_purity"
        ],
        "top_callees": [
          "rules.rs:ContextMatcher::any",
          "lazy.rs:LazyPipeline::collect",
          "macro_definition_collector.rs:collect_definitions"
        ]
      },
      "purity": {
        "is_pure": false,
        "confidence": 0.95,
        "side_effects": [
          "Mutable reference parameter",
          "HashMap mutation"
        ]
      },
      "pattern": {
        "type": "god_object",
        "confidence": 0.92,
        "details": {
          "responsibilities": 10,
          "methods": 33,
          "fields": 14,
          "cohesion_score": 0.28
        }
      },
      "scoring": {
        "base_score": 250,
        "complexity_factor": 0.85,
        "complexity_weight": 0.4,
        "coverage_factor": 0.22,
        "coverage_weight": 0.3,
        "dependency_factor": 0.65,
        "dependency_weight": 0.2,
        "role_multiplier": 1.0,
        "role": "PureLogic",
        "adjustments": [
          {"name": "God Object", "value": 45}
        ]
      },
      "context": {
        "total_lines": 168,
        "completeness_confidence": 0.85,
        "primary": {
          "file": "src/analyzers/purity_detector.rs",
          "start_line": 10,
          "end_line": 85,
          "symbol": "PurityDetector::analyze"
        },
        "related": [
          {
            "file": "src/analyzers/purity_detector.rs",
            "start_line": 1,
            "end_line": 9,
            "relationship": "ModuleHeader",
            "reason": "Module imports and constants"
          },
          {
            "file": "src/extraction/extractor.rs",
            "start_line": 234,
            "end_line": 267,
            "relationship": "Caller",
            "reason": "Primary caller - orchestrates purity analysis"
          },
          {
            "file": "src/analyzers/purity_detector.rs",
            "start_line": 1500,
            "end_line": 1600,
            "relationship": "TestCode",
            "reason": "Unit tests for this function"
          }
        ]
      }
    }
  ]
}
```

#### Key Design Decisions

1. **No Decorative Elements**:
   - Remove `├─`, `└─`, `→`, `✓` symbols
   - Remove ASCII boxes and separators
   - Remove color codes and formatting

2. **Consistent Structure**:
   - Every item has the same sections in the same order
   - Missing data is explicit (`null`, `N/A`, or omitted section)
   - No conditional formatting based on severity

3. **Complete Information**:
   - All scoring factors exposed, not just final score
   - All metrics included, not just highlights
   - Full dependency lists (or top N with count)

4. **Stable IDs**:
   - Each item has a deterministic ID for AI reference
   - Format: `{filename}_{line}` with special chars replaced

5. **No Interpretation**:
   - Raw metric values, not "high complexity"
   - Scores and factors, not "recommended actions"
   - The AI interprets, debtmap reports

### Non-Functional Requirements

- Output must be deterministic (same input = same output)
- Markdown must be valid CommonMark
- JSON must be valid and parseable
- Both formats must contain identical information
- Output should be streamable for large codebases

## Acceptance Criteria

- [ ] New `--format llm-markdown` option produces LLM-optimized markdown
- [ ] Updated `--format json` produces v3.0 format with all fields
- [ ] Both formats contain identical information
- [ ] No decorative elements in LLM markdown output
- [ ] All scoring factors exposed in both formats
- [ ] Context suggestions included (per spec 263)
- [ ] Stable item IDs generated for each debt item
- [ ] Format is deterministic (reproducible output)
- [ ] Documentation includes format specification for AI consumers
- [ ] `cargo test` passes
- [ ] `cargo clippy` passes

## Technical Details

### Implementation Approach

**Phase 1: Define New Output Types**
1. Create `LlmMarkdownWriter` in `src/io/writers/`
2. Update `JsonWriter` for v3.0 format
3. Define shared output structures

**Phase 2: Implement Markdown Writer**
1. Write hierarchical markdown generator
2. Ensure consistent section ordering
3. Include all data fields

**Phase 3: Update JSON Writer**
1. Add new fields (context, scoring breakdown)
2. Update format version to 3.0
3. Ensure all fields match markdown

**Phase 4: CLI Integration**
1. Add `--format llm-markdown` option
2. Keep `--format markdown` for backward compatibility (human-readable)
3. Update `--format json` to v3.0

### Architecture Changes

New/modified files:
```
src/io/writers/
├── llm_markdown.rs     # NEW: LLM-optimized markdown writer
├── json.rs             # MODIFIED: v3.0 format updates
└── mod.rs              # MODIFIED: export new writer

src/output/
├── unified/
│   ├── llm_types.rs    # NEW: LLM-specific output types
│   └── mod.rs          # MODIFIED: add LLM types
```

### Format Versioning

- JSON format version: `3.0` (breaking change from 2.0)
- LLM markdown: New format, no versioning needed initially
- Human markdown: Unchanged (version 1.x)

### CLI Options

```bash
# LLM-optimized markdown (NEW)
debtmap analyze . --format llm-markdown

# LLM-optimized JSON (updated)
debtmap analyze . --format json

# Human-readable markdown (unchanged)
debtmap analyze . --format markdown

# Human-readable terminal (unchanged, default)
debtmap analyze .
```

### Shared Data Pipeline

```
UnifiedDebtItem
    ↓
OutputPreparer::prepare(item)
    ↓
LlmReadyItem (shared structure)
    ↓
├── LlmMarkdownWriter::write(item) → Markdown
└── JsonWriter::write(item) → JSON
```

## Dependencies

- **Prerequisites**:
  - [262 - Remove Recommendation Engine] (no recommendations in output)
  - [263 - Context Window Suggestions] (context field)
- **Affected Components**: CLI, output writers
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Each section renders correctly in both formats
- **Integration Tests**: Full output parsing and validation
- **Equivalence Tests**: JSON and markdown contain same data
- **Property Tests**: Output is deterministic
- **User Acceptance**: AI agents can consume and understand output

## Documentation Requirements

- **Code Documentation**: Format specification in module docs
- **User Documentation**: LLM output format reference guide
- **Architecture Updates**: Output pipeline diagram

## Implementation Notes

### Markdown Parsing by LLMs

LLMs are excellent at understanding markdown structure:
- Headings create natural hierarchy
- Lists are reliably parsed
- Code blocks are preserved
- Consistent structure enables reliable extraction

The key is **consistency** - LLMs struggle when format varies by item.

### JSON for Programmatic Access

While LLMs read markdown well, downstream tooling may prefer JSON:
- Filtering by severity
- Aggregating scores
- Tracking trends
- Integration with other systems

Both formats should be equally complete.

### Backward Compatibility

- `--format json` v3.0 is breaking (new fields, restructured)
- Old consumers may need to update parsing
- Consider `--format json-v2` for transition period

## Migration and Compatibility

- JSON v3.0 is a breaking change from v2.0
- New `context` field requires spec 263
- No `recommendation` field (per spec 262)
- Human-readable formats (`--format markdown`, terminal) unchanged
