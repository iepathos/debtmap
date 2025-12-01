---
number: 200
title: Visual Breathing Room and Typography Polish
category: optimization
priority: low
status: draft
dependencies: [194, 197]
created: 2025-11-30
---

# Specification 200: Visual Breathing Room and Typography Polish

**Category**: optimization
**Priority**: low
**Status**: draft
**Dependencies**: Specs 194 (Scannable Summary Mode), 197 (Consistent Card Structure)

## Context

Debtmap's terminal output currently has cramped visual presentation:

```
└─ METRICS: Methods: 55, Fields: 5, Responsibilities: 8
└─ SCORING: File size: HIGH | Functions: EXCESSIVE | Complexity: HIGH
└─ DEPENDENCIES: 55 functions may have complex interdependencies

#2 SCORE: 168 [CRITICAL]
```

**Visual issues**:
- **No spacing between items** - Recommendations run together
- **Dense text blocks** - Hard to see where one item ends
- **No visual separation** - Missing horizontal rules or whitespace
- **Inconsistent spacing** - Some areas cramped, others spread out

This reduces readability and makes scanning more difficult.

## Objective

Add visual breathing room by:

1. **Spacing between items** - Blank lines between recommendations
2. **Section separators** - Horizontal rules for major sections
3. **Consistent padding** - Uniform spacing patterns
4. **Professional polish** - Clean, readable output

**Success Metric**: Users find output more comfortable to read and scan (subjective user testing).

## Requirements

### Functional Requirements

1. **Item Separation**
   - Add 1 blank line between recommendations
   - Add horizontal rule between major sections
   - Maintain visual hierarchy with spacing

2. **Section Separators**
   ```
   ────────────────────────────────────────────
   ```
   - Between header and recommendations
   - Between recommendations and summary
   - Configurable width (default 44 chars)

3. **Consistent Padding**
   - 1 line after headers
   - 1 line between items
   - 2 lines before major sections

4. **Tree Symbol Spacing**
   - Consistent indentation (2 spaces)
   - Visual alignment of tree branches
   - No irregular spacing

### Non-Functional Requirements

1. **Readability** - Easier to parse visually
2. **Professional** - Polished appearance
3. **Consistency** - Uniform spacing rules
4. **Configurable** - Spacing adjustable via flags (future)

## Acceptance Criteria

- [ ] Blank lines between recommendations
- [ ] Horizontal rules between major sections
- [ ] Consistent padding throughout
- [ ] Tree symbols aligned properly
- [ ] Header separated with rule
- [ ] Summary separated with rule
- [ ] User testing confirms improved readability
- [ ] No excessive whitespace (< 50% blank lines)

## Technical Details

### Implementation

```rust
// src/io/writers/terminal.rs

const SEPARATOR_WIDTH: usize = 44;
const SECTION_SEPARATOR: &str = "────────────────────────────────────────────";

impl TerminalWriter {
    fn write_header(&mut self, coverage_mode: CoverageMode) -> Result<()> {
        writeln!(self.output, "{}", "=".repeat(SEPARATOR_WIDTH))?;
        writeln!(self.output, "    Debtmap v{}", env!("CARGO_PKG_VERSION"))?;
        writeln!(self.output, "{}", "=".repeat(SEPARATOR_WIDTH))?;

        // Coverage info...

        writeln!(self.output)?; // Blank line after header
        Ok(())
    }

    fn write_recommendations(&mut self, recommendations: &[DebtRecommendation]) -> Result<()> {
        writeln!(self.output, "TOP 10 RECOMMENDATIONS")?;
        writeln!(self.output)?; // Blank line after section header

        for (idx, rec) in recommendations.iter().enumerate() {
            let card = self.create_card(idx + 1, rec);
            writeln!(self.output, "{}", card.format_summary())?;

            // Add blank line between items (except after last)
            if idx < recommendations.len() - 1 {
                writeln!(self.output)?;
            }
        }

        writeln!(self.output)?;
        writeln!(self.output, "{}", SECTION_SEPARATOR)?;
        writeln!(self.output)?;

        Ok(())
    }

    fn write_summary(&mut self, total_score: f64, density: f64) -> Result<()> {
        writeln!(self.output, "TOTAL DEBT SCORE: {:.0}", total_score)?;
        writeln!(self.output, "DEBT DENSITY: {:.1} per 1K LOC", density)?;
        Ok(())
    }
}
```

### Example Output

**Before (Cramped)**:
```
└─ DEPENDENCIES: 55 functions may have complex interdependencies
#2 SCORE: 168 [CRITICAL]
└─ ./src/organization/god_object_analysis.rs (3582 lines, 143 functions)
```

**After (Breathing Room)**:
```
└─ DEPENDENCIES: 55 functions may have complex interdependencies

────────────────────────────────────────────

#2 SCORE: 168 [CRITICAL] IMPACT: -20-35% complexity
├─ LOCATION: src/organization/god_object_analysis.rs
├─ ISSUE: 143 functions across 10 responsibilities in single module
├─ ACTION: [SPLIT] Split into 6 modules by data flow
├─ EFFORT: L · RISK: HIGH
└─ Run with --detail=2 for full analysis

```

## Dependencies

- **Prerequisites**: Specs 194, 197
- **Affected Components**: `src/io/writers/terminal.rs`

## Testing Strategy

```rust
#[test]
fn test_spacing_between_items() {
    let output = generate_output_with_10_items();
    let sections = output.split("\n\n");

    // Should have blank lines between items
    assert!(sections.count() >= 10);
}

#[test]
fn test_section_separators() {
    let output = generate_full_output();
    assert!(output.contains("────────────────────────────────────────────"));
}
```

## Success Metrics

- ✅ Blank lines between items
- ✅ Section separators present
- ✅ User testing confirms improved readability
- ✅ Consistent spacing patterns

## References

- Spec 194: Scannable Summary Mode
- Spec 197: Consistent Card Structure
- Typography principles: whitespace for readability
