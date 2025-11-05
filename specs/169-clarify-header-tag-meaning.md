---
number: 169
title: Clarify Header Tag Meaning and Visual Separation
category: optimization
priority: medium
status: draft
dependencies: [167, 168]
created: 2025-01-05
---

# Specification 169: Clarify Header Tag Meaning and Visual Separation

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 167, Spec 168

## Context

The debtmap output header section displays multiple tags without clear distinction:

```
#2 SCORE: 11.5 [ERROR UNTESTED] [CRITICAL]
```

Users report confusion about what each tag represents:
- First tag (`[ERROR UNTESTED]`) - Coverage status
- Second tag (`[CRITICAL]`) - Priority/severity of the debt item itself

There's no visual separation, labeling, or legend explaining the difference between:
1. **Coverage Status**: How well the code is tested
2. **Item Severity**: How critical this debt item is to fix
3. **Debt Score**: Numeric prioritization score

This makes it hard for users to quickly understand what they're looking at, especially when first using debtmap.

## Objective

Make header tags self-explanatory by adding visual separation, optional labels, and/or an inline legend that clarifies what each tag represents, allowing users to quickly understand the item's priority, coverage, and severity.

## Requirements

### Functional Requirements

1. **Visual Separation**
   - Add clear visual boundaries between different tag types
   - Use spacing, symbols, or separators to distinguish tags
   - Maintain compactness while improving clarity

2. **Tag Labels (Optional)**
   - Consider adding brief labels for each tag type
   - Labels should be concise (5-10 chars max)
   - Make labels optional based on verbosity level

3. **Legend Display**
   - Add legend at start of output (verbosity ≥ 1)
   - Explain what each tag type represents
   - Show legend once, not for every item

4. **Consistent Ordering**
   - Always display tags in same order
   - Recommended: SCORE → COVERAGE → SEVERITY
   - Users can learn pattern

### Non-Functional Requirements

1. **Scannability**
   - Users should quickly identify tag meaning
   - Visual hierarchy guides attention
   - Important information stands out

2. **Compactness**
   - Don't make headers excessively verbose
   - Balance clarity with information density
   - Reserve details for higher verbosity

## Acceptance Criteria

- [ ] Header tags are visually distinct from each other
- [ ] Users can understand tag meaning without referring to docs
- [ ] Legend displays at output start (verbosity ≥ 1)
- [ ] Tag ordering is consistent across all items
- [ ] Tests verify tag separation and formatting
- [ ] User documentation explains header format
- [ ] Verbosity levels control detail display

## Technical Details

### Proposed Format Options

**Option 1: Labeled Tags**
```
#2 SCORE: 11.5 | Coverage: [ERROR UNTESTED] | Priority: [CRITICAL]
```
- Pros: Extremely clear, self-documenting
- Cons: Verbose, takes up horizontal space

**Option 2: Separated with Spacing**
```
#2 SCORE: 11.5    [ERROR UNTESTED]    [CRITICAL]
```
- Pros: Compact, visually distinct
- Cons: Requires legend to understand

**Option 3: Symbol Separators**
```
#2 SCORE: 11.5 • [ERROR UNTESTED] • [CRITICAL]
```
- Pros: Clean, compact, clear boundaries
- Cons: Still requires legend

**Option 4: Emoji/Icon Indicators**
```
#2 SCORE: 11.5 📊 [ERROR UNTESTED] ⚠️  [CRITICAL]
```
- Pros: Visual, easy to scan
- Cons: Terminal compatibility, might be too casual

**Option 5: Tiered Verbosity**
- Default (verbosity 0): `#2 SCORE: 11.5 [ERROR UNTESTED] [CRITICAL]` (current)
- Medium (verbosity 1): `#2 SCORE: 11.5 | [ERROR UNTESTED] | [CRITICAL]` (separators)
- High (verbosity 2): `#2 SCORE: 11.5 | Coverage: [ERROR UNTESTED] | Priority: [CRITICAL]` (labels)

**Recommendation**: **Option 3 + Legend** (Symbol separators with legend at start)
- Maintains compactness
- Clear visual boundaries
- Legend provides context
- Works well in terminals

### Legend Format

Display at start of output (before recommendations):

```
============================================
                 Debtmap v0.3.5
============================================
TOP 10 RECOMMENDATIONS

Legend:
  SCORE: Numeric priority (higher = more important)
  [ERROR/WARN/OK]: Coverage status (how well tested)
  [CRITICAL/HIGH/MEDIUM/LOW]: Item severity (fix urgency)

--------------------------------------------
```

Alternative compact legend:
```
KEY: SCORE • Coverage Status • Item Severity
```

### Architecture Changes

**Files to Modify:**

1. `src/priority/formatter/sections.rs:36-47`
   - Update `format_header_section()`
   - Add visual separators between tags
   - Support verbosity-based formatting

2. `src/priority/formatter_verbosity.rs`
   - Add legend generation function
   - Insert legend before first recommendation
   - Make legend optional based on verbosity

3. **New Module** (Optional): `src/formatting/legend.rs`
   ```rust
   pub struct OutputLegend {
       pub show: bool,
       pub style: LegendStyle,
   }

   pub enum LegendStyle {
       None,
       Compact,
       Detailed,
   }

   impl OutputLegend {
       pub fn format(&self) -> String {
           match self.style {
               LegendStyle::None => String::new(),
               LegendStyle::Compact => "KEY: SCORE • Coverage • Severity\n\n".to_string(),
               LegendStyle::Detailed => {
                   concat!(
                       "Legend:\n",
                       "  SCORE: Numeric priority (higher = more important)\n",
                       "  [ERROR/WARN/OK]: Coverage status\n",
                       "  [CRITICAL/HIGH/MEDIUM/LOW]: Item severity\n\n"
                   ).to_string()
               }
           }
       }
   }
   ```

### Implementation Examples

**Before:**
```rust
fn format_header_section(context: &FormatContext) -> String {
    format!(
        "#{} {} [{}]",
        context.rank,
        format!("SCORE: {}", score_formatter::format_score(context.score)).bright_yellow(),
        context
            .severity_info
            .label
            .color(context.severity_info.color)
            .bold()
    )
}
```

**After (Option 3):**
```rust
fn format_header_section(context: &FormatContext) -> String {
    let separator = " • ".dimmed();

    format!(
        "#{} {}{}{}{}{}",
        context.rank,
        format!("SCORE: {}", score_formatter::format_score(context.score)).bright_yellow(),
        separator,
        format_coverage_tag(&context),  // [ERROR UNTESTED]
        separator,
        context
            .severity_info
            .label
            .color(context.severity_info.color)
            .bold()  // [CRITICAL]
    )
}
```

**After (Option 5 - Verbosity-based):**
```rust
fn format_header_section(context: &FormatContext, verbosity: u8) -> String {
    match verbosity {
        0 => format!(
            "#{} {} [{}]",
            context.rank,
            format!("SCORE: {}", score_formatter::format_score(context.score)).bright_yellow(),
            context.severity_info.label.color(context.severity_info.color).bold()
        ),
        1 => format!(
            "#{} {} | {} | {}",
            context.rank,
            format!("SCORE: {}", score_formatter::format_score(context.score)).bright_yellow(),
            format_coverage_tag(&context),
            context.severity_info.label.color(context.severity_info.color).bold()
        ),
        _ => format!(
            "#{} {} | Coverage: {} | Priority: {}",
            context.rank,
            format!("SCORE: {}", score_formatter::format_score(context.score)).bright_yellow(),
            format_coverage_tag(&context),
            context.severity_info.label.color(context.severity_info.color).bold()
        ),
    }
}
```

### Tag Ordering Rationale

**Recommended Order**: `SCORE → COVERAGE → SEVERITY`

1. **SCORE** - Most important for sorting/prioritization
2. **COVERAGE** - Context about current state
3. **SEVERITY** - Additional urgency indicator

Alternative orders considered:
- `SCORE → SEVERITY → COVERAGE` - Groups priority info together
- `SEVERITY → COVERAGE → SCORE` - Leads with urgency

## Dependencies

**Prerequisites:**
- Spec 167: Fix Redundant Coverage Status Indicators
- Spec 168: Standardize Status Prefix Patterns

**Affected Components:**
- Header formatting (`formatter/sections.rs`)
- Output initialization (for legend)
- Verbosity handling
- All header display tests

**External Dependencies:**
- None

## Testing Strategy

### Unit Tests

1. **Header Format Test**
   ```rust
   #[test]
   fn test_header_visual_separation() {
       let context = create_test_context();

       // Verbosity 0: compact
       let header_v0 = format_header_section(&context, 0);
       assert!(header_v0.contains("SCORE:"));

       // Verbosity 1: separators
       let header_v1 = format_header_section(&context, 1);
       assert!(header_v1.contains("|") || header_v1.contains("•"));

       // Verbosity 2: labels
       let header_v2 = format_header_section(&context, 2);
       assert!(header_v2.contains("Coverage:"));
       assert!(header_v2.contains("Priority:"));
   }
   ```

2. **Legend Generation Test**
   ```rust
   #[test]
   fn test_legend_generation() {
       let legend = OutputLegend {
           show: true,
           style: LegendStyle::Detailed,
       };

       let output = legend.format();
       assert!(output.contains("SCORE"));
       assert!(output.contains("Coverage"));
       assert!(output.contains("Severity"));
   }
   ```

3. **Tag Ordering Test**
   ```rust
   #[test]
   fn test_tag_ordering_consistency() {
       let context = create_test_context();
       let header = format_header_section(&context, 1);

       // Find positions of each component
       let score_pos = header.find("SCORE").unwrap();
       let coverage_pos = header.find("[ERROR").unwrap_or(header.find("[WARN").unwrap_or(0));
       let severity_pos = header.find("[CRITICAL").unwrap_or(header.find("[HIGH").unwrap_or(0));

       // Verify ordering: SCORE < COVERAGE < SEVERITY
       assert!(score_pos < coverage_pos);
       assert!(coverage_pos < severity_pos);
   }
   ```

### Integration Tests

1. **End-to-End Legend Test**
   - Run debtmap with verbosity 1
   - Verify legend appears at start
   - Verify legend appears only once
   - Verify all headers follow pattern

2. **Visual Consistency Test**
   - Generate output with all severity levels
   - Verify all headers have same structure
   - Check alignment and spacing

### User Testing

1. **Comprehension Test**
   - Show output to users unfamiliar with debtmap
   - Ask them to explain what each tag means
   - Measure understanding with/without legend

## Documentation Requirements

### Code Documentation

1. **Function Documentation**
   ```rust
   /// Formats the header section with visual separation between tags.
   ///
   /// Tag order: SCORE → COVERAGE → SEVERITY
   ///
   /// Verbosity levels:
   /// - 0: Compact format with minimal separators
   /// - 1: Pipe separators between tags
   /// - 2: Labeled tags with explicit categories
   ///
   /// # Examples
   /// ```
   /// // Verbosity 0
   /// "#2 SCORE: 11.5 [ERROR UNTESTED] [CRITICAL]"
   ///
   /// // Verbosity 1
   /// "#2 SCORE: 11.5 | [ERROR UNTESTED] | [CRITICAL]"
   ///
   /// // Verbosity 2
   /// "#2 SCORE: 11.5 | Coverage: [ERROR UNTESTED] | Priority: [CRITICAL]"
   /// ```
   fn format_header_section(context: &FormatContext, verbosity: u8) -> String
   ```

2. **Legend Documentation**
   ```rust
   /// Generates an output legend explaining header tags.
   ///
   /// The legend is displayed once at the start of recommendations output
   /// when verbosity >= 1.
   ///
   /// Legend explains:
   /// - SCORE: Numeric priority value
   /// - Coverage tags: Test coverage status
   /// - Severity tags: Item fix urgency
   pub fn generate_legend(style: LegendStyle) -> String
   ```

### User Documentation

1. **Understanding Output** (`book/src/understanding-output.md`)
   ```markdown
   ## Recommendation Headers

   Each recommendation starts with a header showing key information:

   ```
   #2 SCORE: 11.5 • [ERROR UNTESTED] • [CRITICAL]
   ```

   ### Header Components

   1. **Rank Number** (`#2`)
      - Position in prioritized recommendation list
      - Lower numbers = higher priority

   2. **Debt Score** (`SCORE: 11.5`)
      - Numeric priority calculated from complexity, coverage, and risk
      - Higher scores = more important to fix

   3. **Coverage Status** (`[ERROR UNTESTED]`)
      - Shows how well the code is tested
      - ERROR: Untested, WARN: Low/Partial, INFO: Moderate, OK: Good/Excellent

   4. **Item Severity** (`[CRITICAL]`)
      - Urgency of fixing this debt item
      - CRITICAL > HIGH > MEDIUM > LOW

   ### Visual Separators

   - Bullet (•) separates tag groups for clarity
   - Use `--verbosity 2` for labeled tags:
     ```
     #2 SCORE: 11.5 | Coverage: [ERROR UNTESTED] | Priority: [CRITICAL]
     ```
   ```

2. **Quick Start Guide** (`book/src/getting-started.md`)
   - Add section explaining header format
   - Include screenshot/example
   - Point to detailed docs

## Implementation Notes

### Verbosity Levels

| Level | Format | Use Case |
|-------|--------|----------|
| 0 | Compact (current) | CI/CD, scripting, experienced users |
| 1 | Separators | Default interactive use |
| 2 | Labeled tags | Learning, clarity-focused |

### Color Coding

Maintain color consistency:
- SCORE: Yellow (`.bright_yellow()`)
- Coverage tags: Color by severity (red/yellow/cyan/green)
- Severity tags: Color by level (bright red/red/yellow/white)
- Separators: Dimmed (`.dimmed()`)

### Accessibility

- Don't rely solely on color for distinction
- Use symbols/separators that work in monochrome
- Ensure legend is plain text readable

### Terminal Compatibility

- Test separators (•, |) in common terminals
- Provide fallback for limited character sets
- Consider ASCII-only mode flag

## Migration and Compatibility

### Breaking Changes

- Header format changes (minor)
- Legend adds lines to output (may break line-based parsers)
- Separator characters may affect text parsing

### Migration Path

1. **Phase 1: Add Verbosity Support**
   - Keep verbosity 0 as current format
   - Add verbosity 1 with separators
   - Add verbosity 2 with labels

2. **Phase 2: Change Default**
   - Make verbosity 1 the default (in next minor version)
   - Keep verbosity 0 available
   - Document migration

3. **Phase 3: Deprecate Old Format**
   - Announce deprecation in release notes
   - Remove verbosity 0 in next major version (optional)

### Compatibility Notes

- JSON output unchanged
- Markdown output should adopt same pattern
- Parsers using regex may need minor updates

## Related Issues

This spec addresses:
- **Issue #3**: Unclear Distinction Between Header Tags
- **User Feedback**: "What do all these tags mean?"
- **Goal**: Self-explanatory output without requiring documentation lookup

## Future Enhancements

1. **Interactive Mode**
   - Hoverable tooltips in terminal (if supported)
   - Inline help with `--explain-headers` flag

2. **Custom Legends**
   - Allow users to customize legend via config
   - Add/remove tag explanations

3. **Color-Coded Legends**
   - Show legend with same colors as tags
   - Visual learning aid

4. **Machine-Readable Headers**
   - JSON mode includes all tag metadata
   - Structured format for programmatic access
