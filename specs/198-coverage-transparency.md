---
number: 198
title: Coverage Transparency in Terminal Output
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-11-30
---

# Specification 198: Coverage Transparency in Terminal Output

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

When users run debtmap with coverage data (`--lcov`), the priority rankings change significantly:

**Without coverage**:
```
#2 SCORE: 168 [CRITICAL] - god_object_analysis.rs
```

**With coverage**:
```
#2 SCORE: 75.4 [CRITICAL] - formatter.rs
#3 SCORE: 70.6 [CRITICAL] - god_object_analysis.rs (was #2!)
```

**Problem**: Users don't know they're missing coverage-adjusted priorities until they try `--lcov`. The output doesn't indicate:
- Whether coverage data was included
- That priorities would change with coverage
- Overall coverage percentage
- How coverage affects scoring

## Objective

Make coverage status transparent by:

1. **Showing coverage mode in header** - Indicate if coverage data was used
2. **Displaying overall coverage** - Show project-wide coverage percentage
3. **Explaining priority adjustment** - Clarify that priorities are coverage-aware
4. **Prompting for coverage** - Suggest `--lcov` when not provided
5. **Distinguishing output modes** - Different headers for with/without coverage

**Success Metric**: Users understand immediately whether coverage influenced priorities and what the overall coverage is.

## Requirements

### Functional Requirements

1. **Header Indicates Coverage Mode**
   ```
   ============================================
       Debtmap v0.6.0
   ============================================
   NOTE: Priorities based on complexity only.
         Run with --lcov <path> for coverage-adjusted priorities.

   TOP 10 RECOMMENDATIONS (complexity-based)
   ```

   vs.

   ```
   ============================================
       Debtmap v0.6.0
   ============================================
   ✓ Coverage data included (81.38% overall)
     Priorities adjusted for untested complex code.

   TOP 10 RECOMMENDATIONS (complexity + coverage)
   ```

2. **Overall Coverage Display**
   - Show percentage when coverage provided
   - Format: "81.38% overall"
   - Position: In header, prominently

3. **Priority Mode Label**
   - Without coverage: "TOP 10 RECOMMENDATIONS (complexity-based)"
   - With coverage: "TOP 10 RECOMMENDATIONS (complexity + coverage)"

4. **Coverage Hint**
   - When no coverage: Suggest `--lcov` flag
   - Explain impact: "coverage-adjusted priorities"

5. **Coverage-Specific Items**
   - Mark items that only appear due to coverage
   - Example: "(coverage-driven)" tag

### Non-Functional Requirements

1. **Clarity** - Immediately obvious whether coverage was used
2. **Discoverability** - Users learn about `--lcov` flag
3. **Consistency** - Same terminology across output

## Acceptance Criteria

- [ ] Header shows coverage status
- [ ] Without coverage: Shows note about complexity-only mode
- [ ] Without coverage: Suggests `--lcov` flag
- [ ] With coverage: Shows overall coverage percentage
- [ ] With coverage: Indicates priorities adjusted
- [ ] Mode label distinguishes complexity vs. complexity+coverage
- [ ] Overall coverage displayed prominently
- [ ] Coverage-driven items marked (optional enhancement)
- [ ] User testing confirms mode is clear
- [ ] Documentation updated with both modes

## Technical Details

### Implementation

```rust
// src/io/writers/terminal.rs

impl TerminalWriter {
    fn write_header(&mut self, coverage_mode: CoverageMode) -> Result<()> {
        writeln!(self.output, "{}", "=".repeat(44))?;
        writeln!(self.output, "    Debtmap v{}", env!("CARGO_PKG_VERSION"))?;
        writeln!(self.output, "{}", "=".repeat(44))?;

        match coverage_mode {
            CoverageMode::None => {
                writeln!(self.output, "NOTE: Priorities based on complexity only.")?;
                writeln!(self.output, "      Run with --lcov <path> for coverage-adjusted priorities.")?;
            }
            CoverageMode::Enabled { overall_coverage } => {
                writeln!(
                    self.output,
                    "✓ Coverage data included ({:.2}% overall)",
                    overall_coverage
                )?;
                writeln!(self.output, "  Priorities adjusted for untested complex code.")?;
            }
        }

        writeln!(self.output)?;
        Ok(())
    }

    fn write_recommendations_header(&mut self, coverage_mode: CoverageMode) -> Result<()> {
        let mode_label = match coverage_mode {
            CoverageMode::None => "complexity-based",
            CoverageMode::Enabled { .. } => "complexity + coverage",
        };

        writeln!(self.output, "TOP 10 RECOMMENDATIONS ({})", mode_label)?;
        Ok(())
    }
}

pub enum CoverageMode {
    None,
    Enabled { overall_coverage: f64 },
}
```

### Example Output

**Without Coverage**:
```
============================================
    Debtmap v0.6.0
============================================
NOTE: Priorities based on complexity only.
      Run with --lcov <path> for coverage-adjusted priorities.

TOP 10 RECOMMENDATIONS (complexity-based)

#1 SCORE: 370 [CRITICAL] IMPACT: -16-31% complexity
  └─ god_object_detector.rs · functions: 55, responsibilities: 8
  ...
```

**With Coverage**:
```
============================================
    Debtmap v0.6.0
============================================
✓ Coverage data included (81.38% overall)
  Priorities adjusted for untested complex code.

TOP 10 RECOMMENDATIONS (complexity + coverage)

#1 SCORE: 184 [CRITICAL] IMPACT: -16-31% complexity
  └─ god_object_detector.rs · functions: 55, responsibilities: 8
  ...

#4 SCORE: 18.4 [CRITICAL] IMPACT: +50% coverage (coverage-driven)
  └─ explain_coverage.rs:275 · 0% coverage, needs 9 tests
  ...
```

## Dependencies

- None

## Testing Strategy

```rust
#[test]
fn test_header_without_coverage() {
    let output = generate_output(CoverageMode::None);
    assert!(output.contains("Priorities based on complexity only"));
    assert!(output.contains("--lcov"));
    assert!(output.contains("complexity-based"));
}

#[test]
fn test_header_with_coverage() {
    let output = generate_output(CoverageMode::Enabled { overall_coverage: 81.38 });
    assert!(output.contains("✓ Coverage data included"));
    assert!(output.contains("81.38% overall"));
    assert!(output.contains("complexity + coverage"));
}
```

## Success Metrics

- ✅ Header shows coverage status clearly
- ✅ Overall coverage displayed when available
- ✅ Mode label distinguishes complexity vs coverage
- ✅ Users understand coverage impact (user testing)

## References

- Design Analysis: Debtmap Terminal Output
- src/io/writers/terminal.rs
