---
number: 105
title: Tiered Priority Display for Debtmap Analysis Output
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-09-26
---

# Specification 105: Tiered Priority Display for Debtmap Analysis Output

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Current debtmap output presents a flat list of debt items ranked by score, making it difficult for users to understand urgency levels and prioritize work. Items with vastly different criticality (e.g., score 97.9 vs 76.3) appear visually similar, burying truly critical issues like god objects (2529-line files) among routine testing gaps.

Analysis of real debtmap output shows 8 items scoring 76-78 with nearly identical complexity patterns, creating noise that obscures actionable insights. Users need clear visual hierarchy to distinguish between "drop everything" critical issues, "this sprint" high priority items, and "next sprint" moderate concerns.

## Objective

Implement a tiered priority display system that groups debt items by urgency level with distinct visual presentation, allowing users to immediately identify and focus on the most critical architectural issues while providing clear progression through lower-priority items.

## Requirements

### Functional Requirements

1. **Score-Based Tier Classification**
   - Critical Tier: Score ≥ 90.0 (immediate action required)
   - High Tier: Score 70.0-89.9 (current sprint priority)
   - Moderate Tier: Score 50.0-69.9 (next sprint planning)
   - Low Tier: Score < 50.0 (backlog consideration)

2. **Visual Hierarchy Implementation**
   - Distinct visual styling for each tier (emojis, headers, spacing)
   - Tier-specific introductory text explaining urgency
   - Limited item display per tier to prevent overwhelming
   - Progressive disclosure for lower-priority items

3. **Tier-Specific Content**
   - Critical: Full detail with immediate action items
   - High: Summary with sprint planning context
   - Moderate: Brief description with effort estimates
   - Low: Condensed format with batch processing suggestions

4. **Batch Grouping for Similar Items**
   - Group identical debt types with similar scores
   - Display count of similar items (e.g., "8 Untested Complex Functions")
   - Provide batch action recommendations

### Non-Functional Requirements

- Maintain backward compatibility with existing output formats
- Performance impact < 100ms for grouping operations
- Preserve existing score calculation methodology
- Support configuration to disable tiered display

## Acceptance Criteria

- [ ] Critical items (score ≥ 90) appear at top with 🚨 CRITICAL header
- [ ] High priority items (70-89) appear under ⚠️ HIGH header
- [ ] Moderate items (50-69) appear under 📊 MODERATE header
- [ ] Low priority items (< 50) appear under 📝 LOW header
- [ ] Similar debt types with close scores are grouped together
- [ ] Each tier shows maximum 5 individual items before grouping
- [ ] God objects and architectural issues always appear in Critical tier
- [ ] Batch items show count and generic action (e.g., "Add test coverage for 8 functions")
- [ ] Empty tiers are omitted from output
- [ ] Tier headers include effort estimates for that category

## Technical Details

### Implementation Approach

1. **Tier Classification Function**
```rust
fn classify_tier(score: f64) -> Tier {
    match score {
        s if s >= 90.0 => Tier::Critical,
        s if s >= 70.0 => Tier::High,
        s if s >= 50.0 => Tier::Moderate,
        _ => Tier::Low,
    }
}
```

2. **Item Grouping Logic**
```rust
fn group_similar_items(items: Vec<DebtItem>) -> Vec<DisplayGroup> {
    items.into_iter()
        .group_by(|item| (classify_debt_type(&item.debt_type), tier_from_score(item.score())))
        .map(|(key, group)| create_display_group(key, group.collect()))
        .collect()
}
```

3. **Display Integration**
   - Modify `MarkdownWriter::write_priority_section()` to use tiered display
   - Add new formatter in `formatter_markdown.rs` for tier-specific styling
   - Extend `OutputFormat` enum to include tiered option

### Architecture Changes

- Add `Tier` enum to `priority/mod.rs`
- Create `TieredDisplayFormatter` in `io/writers/markdown/`
- Extend existing `format_priorities()` function with tier parameter
- Add configuration option in `MarkdownConfig` for tiered display

### Data Structures

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Tier {
    Critical,  // Score ≥ 90.0
    High,      // Score 70.0-89.9
    Moderate,  // Score 50.0-69.9
    Low,       // Score < 50.0
}

#[derive(Debug, Clone)]
pub struct DisplayGroup {
    pub tier: Tier,
    pub debt_type: String,
    pub items: Vec<DebtItem>,
    pub batch_action: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TieredDisplay {
    pub critical: Vec<DisplayGroup>,
    pub high: Vec<DisplayGroup>,
    pub moderate: Vec<DisplayGroup>,
    pub low: Vec<DisplayGroup>,
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `io/writers/markdown/enhanced.rs` - Priority section writer
  - `priority/formatter_markdown.rs` - Markdown formatting
  - `priority/formatter.rs` - Core formatting logic
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**:
  - Test tier classification with boundary values (89.9, 90.0, 90.1)
  - Test grouping logic with mixed debt types
  - Test batch action generation for similar items

- **Integration Tests**:
  - Full tiered display with realistic debt data
  - Verify proper ordering within and across tiers
  - Test empty tier handling

- **User Acceptance**:
  - Compare tiered vs flat display for discoverability
  - Measure time to identify critical issues
  - Validate effort estimation accuracy

## Documentation Requirements

- **Code Documentation**: Document tier classification rationale and thresholds
- **User Documentation**: Explain tiered display interpretation and usage
- **Architecture Updates**: Document new display components and flow

## Implementation Notes

1. **Tier Threshold Rationale**:
   - 90+ score represents architectural/god object issues requiring immediate attention
   - 70-89 represents complex functions needing current sprint attention
   - 50-69 represents moderate debt for next sprint planning
   - <50 represents maintenance backlog items

2. **Grouping Strategy**:
   - Identical debt types within 5-point score range should group
   - God objects, architectural issues never group (always show individually)
   - Testing gaps can group by language/module for batch processing

3. **Performance Considerations**:
   - Grouping should occur after sorting, not during
   - Tier classification should be computed once and cached
   - Batch action text should be pre-generated, not computed per display

## Migration and Compatibility

During prototype phase: This enhancement is additive and maintains full backward compatibility. Default behavior remains unchanged unless explicitly configured for tiered display. No breaking changes to existing APIs or output formats.