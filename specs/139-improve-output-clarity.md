---
number: 139
title: Improve Output Clarity and Consistency
category: optimization
priority: low
status: draft
dependencies: [134, 135, 136, 138]
created: 2025-10-27
---

# Specification 139: Improve Output Clarity and Consistency

**Category**: optimization
**Priority**: low
**Status**: draft
**Dependencies**: Specs 134, 135, 136, 138

## Context

The current debtmap output has clarity and consistency issues:

**Problems Identified**:

1. **Inconsistent Tree Formatting**
   ```
   #2 SCORE: 174 [CRITICAL - FILE - GOD OBJECT]
   └─ ./crates/printer/src/standard.rs (3987 lines, 172 functions)
   └─ WHY: This module contains 172 module functions across 1 responsibilities...
   └─ ACTION: URGENT: 3987 lines, 172 functions!...
     - SUGGESTED SPLIT (generic - no detailed analysis available):
     -  [1] standard_core.rs - Core business logic
   ```
   - Uses `└─` for different nesting levels inconsistently
   - Switches between tree format and `-` bullets mid-issue
   - Unclear visual hierarchy

2. **Information Overload in ACTION Sections**
   - Some issues have 13-step action plans
   - Mixes high-level and low-level details
   - Difficult to quickly scan for key information

3. **Separating WHY from EVIDENCE**
   - WHY sections mix rationale with metrics
   - Hard to distinguish "what's wrong" from "why it matters"
   - Evidence (metrics, line numbers) buried in prose

4. **Inconsistent Severity Indicators**
   - Uses tags like `[CRITICAL - FILE - GOD OBJECT]`
   - Also uses `[WARN PARTIAL COVERAGE]`
   - Mixes severity with issue type in tags

5. **Poor Scannability**
   - Long text blocks without visual breaks
   - Key metrics buried in sentences
   - No color or emphasis (in terminal output)
   - Hard to find specific information quickly

## Objective

Redesign debtmap output format for clarity, consistency, and scannability while maintaining information density and actionability.

## Requirements

### Functional Requirements

1. **Consistent Visual Hierarchy**
   - Clear nesting levels with consistent indentation
   - Use tree characters (`└─`, `├─`, `│`) correctly
   - Separate sections visually
   - Consistent use of formatting across issue types

2. **Structured Information Sections**
   - **HEADER**: Issue number, severity, score, type
   - **LOCATION**: File path, function name, line numbers
   - **IMPACT**: What's affected, risk level, user impact
   - **EVIDENCE**: Hard metrics, measurements, data
   - **WHY**: Rationale explaining why this matters
   - **ACTION**: Concise, numbered steps (from Spec 138)
   - **DETAILS**: Expandable section for deep dive

3. **Clear Evidence vs Rationale**
   - Separate "what we measured" from "why it matters"
   - Metrics in structured format (not prose)
   - Rationale explains implications clearly
   - Evidence supports the severity rating

4. **Scannability Improvements**
   - Key metrics highlighted or in consistent positions
   - Use of whitespace to separate sections
   - Consistent formatting for similar information
   - Summary view with expandable details
   - Terminal color support (optional, configurable)

5. **Severity and Type Clarity**
   - Clear severity indicator (CRITICAL/HIGH/MEDIUM/LOW)
   - Separate issue type (complexity, coverage, god object)
   - No mixing of severity and type in one tag
   - Visual distinction between severity levels

### Non-Functional Requirements

1. **Consistency**: Same issue type always formatted the same way
2. **Readability**: Users can find key info in <10 seconds
3. **Completeness**: All relevant information still present
4. **Configurability**: Users can choose verbosity level
5. **Terminal Compatibility**: Works in basic terminals (no fancy features required)

## Acceptance Criteria

- [ ] All issues use consistent tree formatting with proper nesting
- [ ] Information sections (LOCATION, EVIDENCE, WHY, ACTION) are clearly separated
- [ ] Evidence section contains only metrics, no rationale
- [ ] WHY section explains implications without repeating metrics
- [ ] ACTION sections are concise (3-5 steps) per Spec 138
- [ ] Severity and issue type are separate, clearly labeled
- [ ] Key metrics appear in consistent positions
- [ ] Visual hierarchy is consistent across all issue types
- [ ] Users can scan output and find critical issues in <30 seconds
- [ ] Terminal color support is optional and configurable
- [ ] JSON/YAML output remains machine-readable
- [ ] Documentation explains new output format
- [ ] `--compact` flag provides summary-only view
- [ ] `--verbose` flag shows all details

## Technical Details

### Implementation Approach

1. **New Output Structure**
   ```rust
   #[derive(Debug, Clone)]
   pub struct IssueOutput {
       header: IssueHeader,
       location: Location,
       impact: Impact,
       evidence: Evidence,
       rationale: Rationale,
       action: Action,
       details: Option<Details>, // Shown only in verbose mode
   }

   #[derive(Debug, Clone)]
   pub struct IssueHeader {
       rank: usize,
       severity: Severity,
       score: f64,
       issue_type: IssueType,
   }

   #[derive(Debug, Clone)]
   pub struct Location {
       file_path: PathBuf,
       function_name: Option<String>,
       line_range: Option<(usize, usize)>,
       size_metrics: SizeMetrics,
   }

   #[derive(Debug, Clone)]
   pub struct Evidence {
       metrics: HashMap<String, MetricValue>,
       measurements: Vec<Measurement>,
   }

   #[derive(Debug, Clone)]
   pub struct Rationale {
       primary_reason: String,
       supporting_reasons: Vec<String>,
       user_impact: String,
   }
   ```

2. **Formatting Strategy**
   ```rust
   pub fn format_issue(issue: &IssueOutput, options: &FormatOptions) -> String {
       let mut output = String::new();

       // Header
       output.push_str(&format_header(&issue.header));

       // Location (always shown)
       output.push_str(&format_location(&issue.location, 1));

       // Impact (always shown)
       output.push_str(&format_impact(&issue.impact, 1));

       // Evidence (compact: key metrics only; verbose: all metrics)
       if options.verbosity >= Verbosity::Normal {
           output.push_str(&format_evidence(&issue.evidence, 1, options.verbosity));
       }

       // Rationale (always shown, but length varies)
       output.push_str(&format_rationale(&issue.rationale, 1, options.verbosity));

       // Action (concise by default, expandable)
       output.push_str(&format_action(&issue.action, 1, options.verbosity));

       // Details (only in verbose mode)
       if options.verbosity >= Verbosity::Verbose {
           if let Some(details) = &issue.details {
               output.push_str(&format_details(details, 1));
           }
       }

       output
   }

   fn format_header(header: &IssueHeader) -> String {
       let severity_str = format_severity(header.severity);
       let type_str = format_issue_type(&header.issue_type);

       format!("\n#{} {} | {} | Score: {:.1}\n",
               header.rank,
               severity_str,
               type_str,
               header.score)
   }

   fn format_location(loc: &Location, indent: usize) -> String {
       let prefix = " ".repeat(indent * 2);

       let mut output = format!("{}📍 LOCATION\n", prefix);

       output.push_str(&format!("{}├─ File: {}\n",
                               prefix,
                               loc.file_path.display()));

       if let Some(func) = &loc.function_name {
           output.push_str(&format!("{}├─ Function: {}\n", prefix, func));
       }

       if let Some((start, end)) = loc.line_range {
           output.push_str(&format!("{}├─ Lines: {}-{}\n", prefix, start, end));
       }

       output.push_str(&format!("{}└─ Size: {} lines, {} functions\n",
                               prefix,
                               loc.size_metrics.lines,
                               loc.size_metrics.functions));

       output
   }

   fn format_evidence(evidence: &Evidence, indent: usize, verbosity: Verbosity) -> String {
       let prefix = " ".repeat(indent * 2);

       let mut output = format!("{}📊 EVIDENCE\n", prefix);

       // Sort metrics by importance
       let sorted_metrics = sort_metrics_by_importance(&evidence.metrics);

       let max_metrics = match verbosity {
           Verbosity::Compact => 3,
           Verbosity::Normal => 6,
           Verbosity::Verbose => usize::MAX,
       };

       for (i, (name, value)) in sorted_metrics.iter().take(max_metrics).enumerate() {
           let connector = if i == sorted_metrics.len() - 1 || i == max_metrics - 1 {
               "└─"
           } else {
               "├─"
           };

           output.push_str(&format!("{}{} {}: {}\n",
                                   prefix, connector, name, value));
       }

       if sorted_metrics.len() > max_metrics {
           output.push_str(&format!("{}   ... {} more metrics (use --verbose)\n",
                                   prefix,
                                   sorted_metrics.len() - max_metrics));
       }

       output
   }

   fn format_rationale(rationale: &Rationale, indent: usize, verbosity: Verbosity) -> String {
       let prefix = " ".repeat(indent * 2);

       let mut output = format!("{}💡 WHY THIS MATTERS\n", prefix);

       output.push_str(&format!("{}├─ {}\n", prefix, rationale.primary_reason));

       if verbosity >= Verbosity::Normal {
           for (i, reason) in rationale.supporting_reasons.iter().enumerate() {
               let connector = if i == rationale.supporting_reasons.len() - 1 {
                   "└─"
               } else {
                   "├─"
               };
               output.push_str(&format!("{}{}   {}\n", prefix, connector, reason));
           }
       }

       output.push_str(&format!("{}└─ Impact: {}\n", prefix, rationale.user_impact));

       output
   }
   ```

3. **Severity Formatting**
   ```rust
   fn format_severity(severity: Severity) -> String {
       match severity {
           Severity::Critical => "🔴 CRITICAL".to_string(),
           Severity::High => "🟠 HIGH".to_string(),
           Severity::Medium => "🟡 MEDIUM".to_string(),
           Severity::Low => "🟢 LOW".to_string(),
       }
   }

   // For terminals without emoji support
   fn format_severity_simple(severity: Severity) -> String {
       match severity {
           Severity::Critical => "[CRITICAL]".to_string(),
           Severity::High => "[HIGH]    ".to_string(),
           Severity::Medium => "[MEDIUM]  ".to_string(),
           Severity::Low => "[LOW]     ".to_string(),
       }
   }
   ```

4. **Verbosity Levels**
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
   pub enum Verbosity {
       Compact,   // Summary only, key metrics
       Normal,    // Standard output (default)
       Verbose,   // Full details
   }

   pub struct FormatOptions {
       verbosity: Verbosity,
       use_color: bool,
       use_emoji: bool,
       max_issues: Option<usize>,
   }
   ```

5. **Example Output**
   ```
   #3 🔴 CRITICAL | Complex Function | Score: 15.8

   📍 LOCATION
   ├─ File: ./crates/core/flags/hiargs.rs
   ├─ Function: HiArgs::from_low_args()
   ├─ Lines: 113-200
   └─ Size: 87 lines

   ⚠️  IMPACT
   ├─ Risk Level: High - untested complex logic
   ├─ Affected: Command-line argument parsing (critical path)
   └─ User Impact: Bugs affect all users, hard to diagnose

   📊 EVIDENCE
   ├─ Cyclomatic Complexity: 42 (threshold: 10)
   ├─ Coverage: 38.7% (gap: 61.3%)
   ├─ Cognitive Complexity: 77
   ├─ Nesting Depth: 4 levels
   ├─ Uncovered Branches: 26 of 42
   └─ Estimated Test Gap: 26 tests needed

   💡 WHY THIS MATTERS
   ├─ High complexity + low coverage = high defect risk
   ├─ Critical path code should have >80% coverage
   ├─ Cyclomatic complexity >20 is very hard to maintain
   └─ Impact: Each bug affects CLI usability for all users

   ✅ RECOMMENDED ACTION
   1. 🟢 Add 7 tests for critical uncovered branches
      Impact: +7 tests, reduce risk by 50%
      Run: cargo test test_from_low_args

   2. 🟡 Extract nested conditionals into 4-5 focused functions
      Impact: -20 complexity, improve testability
      Pattern: Nested conditionals → Guard clauses + predicates

   3. 🟢 Verify complexity reduction
      Impact: Confirmed <10 complexity per function
      Run: cargo test && debtmap analyze src/

   📖 CODE EXAMPLE: Extract Guard Clauses

   Before:
       fn from_low_args(...) {
           if condition1 {
               if condition2 {
                   if condition3 {
                       // nested logic
                   }
               }
           }
       }

   After:
       fn from_low_args(...) {
           if !condition1 { return early_exit(); }
           if !condition2 { return early_exit(); }
           if !condition3 { return early_exit(); }

           // clear main logic
       }

   ─────────────────────────────────────────────────────────
   ```

### Configuration Support

```toml
# .debtmap.toml
[output]
verbosity = "normal"  # compact, normal, verbose
use_color = true
use_emoji = true
max_issues = 10       # Top N issues to show

[output.sections]
show_evidence = true
show_code_examples = true
show_detailed_steps = false  # Only in verbose mode
```

## Dependencies

- **Prerequisites**:
  - Spec 134: Consistent metrics for evidence section
  - Spec 135: Context-aware thresholds for rationale
  - Spec 136: Rebalanced scoring for severity
  - Spec 138: Concise actions for action section
- **Affected Components**:
  - `src/io/output.rs` - Complete refactor of formatting
  - `src/io/formatters/` - New module structure for formatters
  - `src/config.rs` - Add output configuration
- **External Dependencies**:
  - `termcolor` (optional) - For color support
  - `console` (optional) - For better terminal detection

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_consistent_tree_formatting() {
    let issue = create_test_issue();
    let output = format_issue(&issue, &FormatOptions::default());

    // Verify tree structure
    assert!(output.contains("📍 LOCATION"));
    assert!(output.contains("├─ File:"));
    assert!(output.contains("└─ Size:"));

    // Verify no mixing of formats
    assert!(!output.contains("└─") && output.contains("  - "));
}

#[test]
fn test_evidence_separated_from_rationale() {
    let issue = create_test_issue();
    let output = format_issue(&issue, &FormatOptions::default());

    let evidence_section = extract_section(&output, "EVIDENCE");
    let rationale_section = extract_section(&output, "WHY THIS MATTERS");

    // Evidence should only have metrics
    assert!(!evidence_section.contains("this matters"));
    assert!(evidence_section.contains("Complexity:"));

    // Rationale should not repeat metrics
    assert!(!rationale_section.contains("42"));
    assert!(rationale_section.contains("hard to maintain"));
}

#[test]
fn test_verbosity_levels() {
    let issue = create_test_issue_with_many_metrics(20);

    let compact = format_issue(&issue, &FormatOptions {
        verbosity: Verbosity::Compact,
        ..Default::default()
    });

    let verbose = format_issue(&issue, &FormatOptions {
        verbosity: Verbosity::Verbose,
        ..Default::default()
    });

    // Compact should have fewer metrics shown
    assert!(compact.len() < verbose.len());
    assert!(compact.contains("... more metrics"));
}

#[test]
fn test_severity_formatting() {
    for severity in [Severity::Critical, Severity::High, Severity::Medium, Severity::Low] {
        let formatted = format_severity(severity);
        assert!(formatted.contains("CRITICAL") ||
                formatted.contains("HIGH") ||
                formatted.contains("MEDIUM") ||
                formatted.contains("LOW"));
    }
}
```

### Integration Tests

```rust
#[test]
fn test_ripgrep_output_clarity() {
    let issues = analyze_file("../ripgrep/crates/core/flags/hiargs.rs").unwrap();
    let output = format_issues(&issues, &FormatOptions::default());

    // Should be scannable
    let lines: Vec<_> = output.lines().collect();
    let critical_issues: Vec<_> = lines.iter()
        .filter(|l| l.contains("CRITICAL"))
        .collect();

    // Should find critical issues quickly
    assert!(!critical_issues.is_empty());
    assert!(critical_issues[0].contains("Score:"));

    // Sections should be clearly separated
    assert!(output.contains("📍 LOCATION"));
    assert!(output.contains("📊 EVIDENCE"));
    assert!(output.contains("💡 WHY THIS MATTERS"));
}
```

### Manual Review Tests

```rust
#[test]
fn test_output_scannability() {
    let issues = analyze_project("../ripgrep").unwrap();
    let output = format_top_issues(&issues, 10);

    // Manual review criteria:
    // 1. Can find critical issues in <30 seconds?
    // 2. Evidence clearly separated from rationale?
    // 3. Tree structure consistent?
    // 4. Key metrics easy to find?

    println!("{}", output);

    // Automated checks for structure
    assert!(output.contains("📍 LOCATION"));
    assert!(output.contains("📊 EVIDENCE"));
    assert!(output.contains("💡 WHY THIS MATTERS"));
    assert!(output.contains("✅ RECOMMENDED ACTION"));
}
```

## Documentation Requirements

### Code Documentation

- Document output formatting structure
- Explain section purposes and content
- Provide examples of each verbosity level
- Document configuration options

### User Documentation

- Guide to reading debtmap output
- Explanation of each section
- Tips for scanning output efficiently
- Configuration guide for customizing output

### Architecture Updates

Update ARCHITECTURE.md:
- Add section on output formatting architecture
- Document the separation of concerns in formatters
- Explain verbosity levels and their use cases

## Implementation Notes

### Emoji and Color Support

- **Emoji**: Make optional, provide text fallbacks
- **Color**: Detect terminal support, fall back to plain text
- **Configuration**: Allow users to disable both

### Consistent Tree Characters

Rules for tree formatting:
- `├─` for middle items
- `└─` for last item at a level
- `│` for continuation lines (if needed)
- Consistent indentation (2 spaces per level)

### Section Ordering

Standard order for all issues:
1. Header (rank, severity, type, score)
2. Location (file, function, lines, size)
3. Impact (risk, affected area, user impact)
4. Evidence (metrics, measurements)
5. Rationale (why it matters, implications)
6. Action (steps, examples)
7. Details (verbose only)

### Metric Importance Ranking

```rust
fn sort_metrics_by_importance(metrics: &HashMap<String, MetricValue>) -> Vec<(String, MetricValue)> {
    let importance_order = [
        "cyclomatic_complexity",
        "coverage_percentage",
        "cognitive_complexity",
        "nesting_depth",
        "function_count",
        "lines_of_code",
        // ... others
    ];

    let mut sorted: Vec<_> = metrics.iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    sorted.sort_by_key(|(name, _)|
        importance_order.iter()
            .position(|&n| n == name)
            .unwrap_or(usize::MAX)
    );

    sorted
}
```

## Migration and Compatibility

### Breaking Changes

- Output format changes significantly
- May break tools parsing debtmap output (use JSON instead)

### Backward Compatibility

- `--legacy-format` flag for old output style
- JSON/YAML output unchanged (machine-readable)
- Old format deprecated, removed in 2-3 releases

### Migration Guide

Provide examples showing:
- Old vs new format side-by-side
- How to find same information in new format
- Benefits of new format
- Configuration options to adjust output

## Success Metrics

- Users can find critical issues in <30 seconds (user study)
- Evidence and rationale clearly separated (>90% clarity score)
- Tree formatting consistent across all issue types
- No switching between tree and bullet formats mid-issue
- Verbosity levels used by >40% of users
- User satisfaction with output clarity improves >50%
- Support tickets about "confusing output" decrease
