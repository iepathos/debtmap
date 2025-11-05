---
number: 166
title: Clarify Entropy-Adjusted Complexity in Output
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-01-05
---

# Specification 166: Clarify Entropy-Adjusted Complexity in Output

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

The debtmap output currently shows complexity metrics in the format:

```
COMPLEXITY: cyclomatic=21 (adj:10), est_branches=21, cognitive=48, nesting=4, entropy=0.42
```

The `(adj:10)` notation is confusing to users because:
1. The adjustment formula is not documented in the output
2. The relationship between the original value (21) and adjusted value (10) is unclear
3. Users don't understand what "adj" means (adjusted for what?)
4. The entropy-based dampening factor that drives this adjustment is shown separately but the connection is not explicit

The actual implementation (from `src/priority/scoring/computation.rs:25`) calculates:
```rust
adjusted_cyclomatic = (cyclomatic as f64 * dampening_factor) as u32
```

Where `dampening_factor` ranges from 0.5 to 1.0 based on entropy analysis of:
- Pattern repetition (0.0-1.0, higher = more repetitive)
- Token entropy (0.0-1.0, higher = more complex/diverse)
- Branch similarity (0.0-1.0, higher = more similar branches)

In the example above: `21 * 0.476 ≈ 10`, where 0.476 is derived from the entropy score of 0.42.

## Objective

Make the entropy-adjusted complexity calculation explicit and understandable in the output by:
1. Clarifying what "adj" means in the output
2. Showing the dampening factor used for adjustment
3. Explaining the relationship between entropy and the adjustment
4. Providing sufficient context for users to understand why repetitive code gets lower adjusted complexity

## Requirements

### Functional Requirements

1. **Output Format Enhancement**
   - Replace `(adj:10)` with more explicit notation
   - Show the dampening factor used in the calculation
   - Link the entropy value to the complexity adjustment
   - Maintain backward compatibility with existing parsers if possible

2. **Inline Documentation**
   - Add contextual hints about what the adjustment represents
   - Explain briefly why certain patterns get dampened
   - Show the formula or calculation when verbosity is high

3. **Consistency Across Outputs**
   - Apply consistent formatting to all complexity displays
   - Ensure JSON/markdown/text outputs all provide clear information
   - Maintain alignment between entropy display and adjusted complexity

### Non-Functional Requirements

1. **Clarity**
   - Users should understand the adjustment without reading source code
   - The relationship between entropy and adjustment should be clear
   - Technical accuracy while remaining accessible

2. **Terseness**
   - Don't make the output excessively verbose for default mode
   - Reserve detailed explanations for higher verbosity levels
   - Balance clarity with information density

## Acceptance Criteria

- [ ] Complexity output explicitly shows what "adjusted" means
- [ ] Dampening factor is visible in the output (at verbosity >= 1)
- [ ] Entropy value is visually linked to the complexity adjustment
- [ ] Users can reconstruct the calculation from the displayed information
- [ ] Updated output format is documented in user-facing docs
- [ ] All existing tests pass with updated format
- [ ] New tests verify clarity of adjustment explanation

## Technical Details

### Implementation Approach

**Option 1: Inline Formula (Verbose)**
```
COMPLEXITY: cyclomatic=21 (entropy-adjusted: 21×0.48=10), est_branches=21, cognitive=48, nesting=4, entropy=0.42
```
- Pros: Shows exact calculation, very clear
- Cons: Verbose, may clutter output

**Option 2: Dampening Factor Display (Recommended)**
```
COMPLEXITY: cyclomatic=21 (dampened: 10, factor: 0.48), est_branches=21, cognitive=48, nesting=4, entropy=0.42
```
- Pros: Clear purpose, shows factor, concise
- Cons: Requires mental math to verify

**Option 3: Entropy-Linked Notation**
```
COMPLEXITY: cyclomatic=21→10 (entropy: 0.42, dampening: 0.48), est_branches=21, cognitive=48, nesting=4
```
- Pros: Arrow shows transformation, groups related values
- Cons: Reorders information, may break expectations

**Option 4: Tiered Verbosity**
- Default (verbosity 0): `cyclomatic=21 (adjusted: 10)`
- Medium (verbosity 1): `cyclomatic=21 (dampened: 10, factor: 0.48)`
- High (verbosity 2): `cyclomatic=21 (entropy-dampened: 21×0.48=10, entropy: 0.42)`

### Architecture Changes

**Files to Modify:**
1. `src/priority/formatter_verbosity.rs:732`
   - Update format string for complexity display
   - Add dampening factor to output
   - Link entropy value to adjustment

2. `src/priority/formatter/sections.rs:85-91`
   - Update complexity section formatting
   - Ensure consistency with verbosity formatter

3. `src/io/writers/enhanced_markdown/complexity_analyzer.rs`
   - Update markdown output format
   - Ensure JSON outputs include dampening factor

**New Fields/Data:**
- Consider adding `dampening_explanation` field to `EntropyDetails`
- May need to pass dampening factor through to formatter

### Data Structures

Update `ComplexityInfo` or `EntropyDetails` to include:
```rust
pub struct EntropyDetails {
    pub entropy_score: f64,
    pub pattern_repetition: f64,
    pub original_complexity: u32,
    pub adjusted_complexity: u32,
    pub dampening_factor: f64,
    pub dampening_explanation: Option<String>, // NEW: e.g., "repetitive pattern"
}
```

## Dependencies

**Prerequisites:**
- None (standalone improvement)

**Affected Components:**
- Text output formatter (`formatter_verbosity.rs`)
- Markdown writer
- JSON output (if complexity is serialized)
- Documentation (user guide, examples)

**External Dependencies:**
- None

## Testing Strategy

### Unit Tests

1. **Format String Tests**
   ```rust
   #[test]
   fn test_complexity_format_with_dampening() {
       let entropy = EntropyDetails {
           dampening_factor: 0.476,
           original_complexity: 21,
           adjusted_complexity: 10,
           // ...
       };
       let output = format_complexity_with_dampening(&entropy);
       assert!(output.contains("dampened"));
       assert!(output.contains("0.48")); // Rounded
       assert!(output.contains("21"));
       assert!(output.contains("10"));
   }
   ```

2. **Verbosity Level Tests**
   - Verify default output is concise
   - Verify verbosity 1 shows dampening factor
   - Verify verbosity 2 shows full calculation

3. **Edge Cases**
   - No entropy data available (dampening = 1.0)
   - Maximum dampening (factor = 0.5)
   - No dampening (factor = 1.0, should not show "adjusted")

### Integration Tests

1. **End-to-End Output Test**
   - Run debtmap on known codebase with repetitive patterns
   - Verify output shows clear adjustment with dampening factor
   - Ensure users can understand why complexity is adjusted

2. **Format Consistency Test**
   - Compare text, JSON, and markdown outputs
   - Ensure all formats provide dampening information
   - Verify parsers handle new format

### User Acceptance

1. **Clarity Validation**
   - Show output to users unfamiliar with entropy analysis
   - Ask them to explain what the adjustment means
   - Iterate on format based on feedback

2. **Documentation Review**
   - Update user guide with explanation of adjusted complexity
   - Add FAQ entry explaining dampening factor
   - Include examples showing formula application

## Documentation Requirements

### Code Documentation

1. **Inline Comments**
   - Document the format string changes
   - Explain dampening factor calculation
   - Reference this spec in relevant code

2. **Function Documentation**
   ```rust
   /// Formats complexity metrics with entropy-based dampening factor.
   ///
   /// The dampening factor (0.5-1.0) is calculated from entropy analysis and
   /// reduces cyclomatic complexity for repetitive patterns. For example:
   /// - Factor 0.5: Highly repetitive code (e.g., validation chains)
   /// - Factor 1.0: Unique/diverse code (no dampening)
   ///
   /// Formula: adjusted = original × dampening_factor
   ```

### User Documentation

1. **User Guide Updates** (`book/src/entropy-analysis.md`)
   - Add section explaining adjusted complexity in output
   - Show examples with different dampening factors
   - Explain when and why dampening is applied

2. **FAQ Entry** (`book/src/faq.md`)
   ```markdown
   ### What does "dampened: 10, factor: 0.48" mean in complexity output?

   Debtmap adjusts cyclomatic complexity based on entropy analysis. Repetitive
   code patterns (like validation chains) have lower cognitive load than diverse
   branching logic, even with the same cyclomatic complexity.

   The dampening factor (0.5-1.0) multiplies the original complexity:
   - cyclomatic=21 × factor=0.48 → adjusted=10

   Lower factors indicate more repetitive/pattern-based code.
   ```

3. **Example Updates** (`book/src/examples.md`)
   - Update example outputs to show new format
   - Add before/after comparison
   - Explain interpretation of adjusted values

## Implementation Notes

### Backward Compatibility

- Consider making format change opt-in via config flag initially
- Provide migration guide for tools parsing old format
- Maintain JSON schema compatibility where possible

### Rounding and Precision

- Dampening factor should be rounded to 2 decimal places for display
- Adjusted complexity is already integer (calculation rounds down)
- Ensure displayed formula matches actual calculation

### Performance Considerations

- Format string changes should have negligible performance impact
- Avoid recalculating dampening factor for display (pass through)
- Cache formatted strings if necessary

### Accessibility

- Use clear, non-technical language where possible
- Provide hover text or footnotes for complex terms
- Consider colorization to highlight relationship between values

## Migration and Compatibility

### Breaking Changes

- Output format changes may affect regex parsers
- JSON schema remains compatible (adding fields, not removing)
- Configuration format unchanged

### Migration Path

1. **Phase 1: Add new fields**
   - Include dampening_factor in output structures
   - Keep old format as default

2. **Phase 2: Update default format**
   - Switch to new format for text output
   - Add deprecation notice for old format

3. **Phase 3: Cleanup**
   - Remove old format code
   - Update all documentation

### Compatibility Notes

- Tools parsing JSON should be unaffected (additive change)
- Text parsers may need updates if they rely on exact format
- Configuration files require no changes

## Related Issues

This spec addresses clarity issues identified in the debtmap output evaluation:
- **Issue**: "The adjustment formula isn't clear. Why is 21 adjusted to 10?"
- **Root Cause**: Dampening factor not shown in output
- **Solution**: Make entropy-based dampening explicit and calculable from output

## Future Enhancements

1. **Interactive Explanation**
   - Add `--explain-adjustment` flag for detailed breakdown
   - Show entropy components and how they contribute to dampening

2. **Visual Indicators**
   - Use color gradients to show dampening intensity
   - Add icons or symbols for high/low dampening

3. **Comparative Display**
   - Show both original and adjusted side-by-side
   - Highlight delta to emphasize adjustment

4. **Machine-Readable Format**
   - Provide structured JSON with full entropy breakdown
   - Include formula as metadata for programmatic access
