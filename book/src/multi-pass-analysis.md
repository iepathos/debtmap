# Multi-Pass Analysis

Multi-pass analysis is enabled by default in debtmap. It performs two separate complexity analyses on your code to distinguish between genuine logical complexity and complexity artifacts introduced by code formatting. By comparing raw and normalized versions of your code, debtmap can attribute complexity to specific sources and provide actionable insights for refactoring.

## Overview

Traditional complexity analysis treats all code as-is, which means formatting choices like multiline expressions, whitespace, and indentation can artificially inflate complexity metrics. Multi-pass analysis solves this problem by:

1. **Raw Analysis** - Measures complexity of code exactly as written
2. **Normalized Analysis** - Measures complexity after removing formatting artifacts
3. **Attribution** - Compares the two analyses to identify complexity sources

The difference between raw and normalized complexity reveals how much "complexity" comes from formatting versus genuine logical complexity from control flow, branching, and nesting.

## How It Works

### Two-Pass Analysis Process

```
┌─────────────┐
│  Raw Code   │
└──────┬──────┘
       │
       ├─────────────────────┐
       │                     │
       ▼                     ▼
┌──────────────┐    ┌────────────────────┐
│ Raw Analysis │    │ Normalize Formatting│
└──────┬───────┘    └─────────┬──────────┘
       │                      │
       │                      ▼
       │            ┌──────────────────────┐
       │            │ Normalized Analysis  │
       │            └─────────┬────────────┘
       │                      │
       └──────────┬───────────┘
                  ▼
         ┌──────────────────┐
         │ Attribution      │
         │ Engine           │
         └─────────┬────────┘
                   │
         ┌─────────┴──────────┐
         │                    │
         ▼                    ▼
┌─────────────────┐  ┌─────────────────┐
│ Insights        │  │ Recommendations │
└─────────────────┘  └─────────────────┘
```

**Raw Analysis** examines your code as-is, capturing all complexity including:
- Logical control flow (if, loops, match, try/catch)
- Function calls and closures
- Formatting artifacts (multiline expressions, whitespace, indentation)

**Normalized Analysis** processes semantically equivalent code with standardized formatting:
- Removes excessive whitespace
- Normalizes multiline expressions to single lines where appropriate
- Standardizes indentation
- Preserves logical structure

**Attribution Engine** compares the results to categorize complexity sources:
- **Logical Complexity** - From control flow and branching (normalized result)
- **Formatting Artifacts** - From code formatting choices (difference between raw and normalized)
- **Pattern Complexity** - From recognized code patterns (error handling, validation, etc.)

> **Note**: Pattern complexity analysis is part of the standard multi-pass analysis. No additional configuration is required to enable pattern detection.

## CLI Usage

Multi-pass analysis runs by default. You can disable it if needed for performance-constrained scenarios:

```bash
# Basic analysis (multi-pass enabled by default)
debtmap analyze .

# Multi-pass with detailed attribution breakdown
debtmap analyze . --attribution

# Control detail level
debtmap analyze . --attribution --detail-level comprehensive

# Output as JSON for tooling integration
debtmap analyze . --attribution --json

# Disable multi-pass for faster single-pass analysis
debtmap analyze . --no-multi-pass
```

### Available Flags

| Flag | Description |
|------|-------------|
| `--no-multi-pass` | Disable multi-pass analysis (use single-pass for performance) |
| `--attribution` | Show detailed complexity attribution breakdown (requires multi-pass) |
| `--detail-level <level>` | Set output detail: `summary`, `standard`, `comprehensive`, `debug` (CLI accepts lowercase values) |
| `--json` | Output results in JSON format |

> **Note**: The `--attribution` flag requires multi-pass analysis to be enabled (the default), as attribution depends on comparing raw and normalized analyses. Use `--no-multi-pass` only when performance is critical.

## Attribution Engine

The attribution engine breaks down complexity into three main categories, each with detailed tracking and suggestions.

### Logical Complexity

Represents inherent complexity from your code's control flow and structure:

- **Function complexity** - Cyclomatic and cognitive complexity per function
- **Control flow** - If statements, loops, match expressions
- **Error handling** - Try/catch blocks, Result/Option handling
- **Closures and callbacks** - Anonymous functions and callbacks
- **Nesting levels** - Depth of nested control structures

Each logical complexity component includes:
- **Contribution** - Complexity points from this construct
- **Location** - File, line, column, and span information
- **Suggestions** - Specific refactoring recommendations

Example:
```rust
// Function with high logical complexity
fn process_data(items: Vec<Item>) -> Result<Vec<Output>> {
    let mut results = Vec::new();

    for item in items {                          // +1 (loop)
        if item.is_valid() {                     // +1 (if)
            match item.category {                // +1 (match)
                Category::A => {
                    if item.value > 100 {        // +2 (nested if)
                        results.push(transform_a(&item)?);
                    }
                }
                Category::B => {
                    results.push(transform_b(&item)?);
                }
                _ => continue,                   // +1 (match arm)
            }
        }
    }

    Ok(results)
}
// Logical complexity: ~7 points
```

### Formatting Artifacts

Identifies complexity introduced by code formatting choices:

- **Multiline expressions** - Long expressions split across multiple lines
- **Excessive whitespace** - Blank lines within code blocks
- **Inconsistent indentation** - Mixed tabs/spaces or irregular indentation
- **Line breaks in chains** - Method chains split across many lines

Formatting artifacts are categorized by severity:
- **Low** - Minor formatting inconsistencies (<10% impact)
- **Medium** - Noticeable formatting impact (10-25% impact)
- **High** - Significant complexity inflation (>25% impact)

Example:
```rust
// Same function with formatting that inflates complexity
fn process_data(
    items: Vec<Item>
) -> Result<Vec<Output>> {
    let mut results =
        Vec::new();

    for item in
        items
    {
        if item
            .is_valid()
        {
            match item
                .category
            {
                Category::A =>
                {
                    if item
                        .value
                        > 100
                    {
                        results
                            .push(
                                transform_a(
                                    &item
                                )?
                            );
                    }
                }
                Category::B =>
                {
                    results
                        .push(
                            transform_b(
                                &item
                            )?
                        );
                }
                _ => continue,
            }
        }
    }

    Ok(results)
}
// Raw complexity: ~12 points (formatting adds ~5 points)
// Normalized complexity: ~7 points (true logical complexity)
```

### Pattern Complexity

Recognizes common code patterns and their complexity characteristics:

- **Error handling patterns** - Result/Option propagation, error conversion
- **Validation patterns** - Input validation, constraint checking
- **Data transformation** - Map/filter/fold chains, data conversions
- **Builder patterns** - Fluent interfaces and builders
- **State machines** - Explicit state management

Each pattern includes:
- **Confidence score** (0.0-1.0) - How certain the pattern recognition is
- **Opportunities** - Suggestions for pattern extraction or improvement

Example:
```rust
// Error handling pattern (confidence: 0.85)
fn load_config(path: &Path) -> Result<Config> {
    let contents = fs::read_to_string(path)
        .context("Failed to read config file")?;

    let config: Config = serde_json::from_str(&contents)
        .context("Failed to parse config JSON")?;

    config.validate()
        .context("Config validation failed")?;

    Ok(config)
}
// Pattern complexity: moderate error handling overhead
// Suggestion: Consider error enum for better type safety
```

## Understanding Attribution Output

When you run with `--attribution`, you'll see a detailed breakdown:

```bash
$ debtmap analyze src/main.rs --multi-pass --attribution --detail-level comprehensive
```

### Sample Output

```
Multi-Pass Analysis Results
============================

File: src/main.rs
Raw Complexity: 45
Normalized Complexity: 32
Formatting Impact: 28.9%

Attribution Breakdown
---------------------

Logical Complexity: 32 points
├─ Function 'main' (line 10): 8 points
│  ├─ Control flow: 5 points (2 if, 1 match, 2 loops)
│  ├─ Nesting: 3 points (max depth: 3)
│  └─ Suggestions:
│     - Break down into smaller functions
│     - Extract complex conditions into named variables
│
├─ Function 'process_request' (line 45): 12 points
│  ├─ Control flow: 8 points (4 if, 1 match, 3 early returns)
│  ├─ Nesting: 4 points (max depth: 4)
│  └─ Suggestions:
│     - Consider using early returns to reduce nesting
│     - Extract validation logic into separate function
│
└─ Function 'handle_error' (line 89): 12 points
   ├─ Control flow: 9 points (5 match arms, 4 if conditions)
   ├─ Pattern: Error handling (confidence: 0.90)
   └─ Suggestions:
      - Consider error enum instead of multiple match arms

Formatting Artifacts: 13 points (28.9% of raw complexity)
├─ Multiline expressions: 8 points (Medium severity)
│  └─ Locations: lines 23, 45, 67, 89
├─ Excessive whitespace: 3 points (Low severity)
│  └─ Locations: lines 12-14, 56-58
└─ Inconsistent indentation: 2 points (Low severity)
   └─ Locations: lines 34, 78

Pattern Complexity: 3 recognized patterns
├─ Error handling (confidence: 0.85): 8 occurrences
│  └─ Opportunity: Consider centralizing error handling
├─ Validation (confidence: 0.72): 5 occurrences
│  └─ Opportunity: Extract validation to separate module
└─ Data transformation (confidence: 0.68): 3 occurrences
   └─ Opportunity: Review for functional composition
```

### Interpreting the Results

**Logical Complexity Breakdown**
- Each function is listed with its complexity contribution
- Control flow elements are itemized (if, loops, match, etc.)
- Nesting depth shows how deeply structures are nested
- Suggestions are specific to that function's complexity patterns

**Formatting Artifacts**
- Shows percentage of "false" complexity from formatting
- Severity indicates impact on metrics
- Locations help you find the formatting issues
- High formatting impact (>25%) suggests inconsistent style

**Pattern Analysis**
- Confidence score shows pattern recognition certainty
- High confidence (>0.7) means reliable pattern detection
- Low confidence (<0.5) suggests unique code structure
- Opportunities highlight potential refactoring

## Insights and Recommendations

Multi-pass analysis automatically generates insights and recommendations based on the attribution results.

### Insight Types

**FormattingImpact**
- Triggered when formatting contributes >20% of measured complexity
- Suggests using automated formatting tools
- Recommends standardizing team coding style

**PatternOpportunity**
- Triggered when pattern confidence is low (<0.5)
- Suggests extracting common patterns
- Recommends reviewing for code duplication

**RefactoringCandidate**
- Triggered when logical complexity exceeds threshold (>20)
- Identifies functions needing breakdown
- Provides specific refactoring strategies

**ComplexityHotspot**
- Identifies areas of concentrated complexity
- Highlights files or modules needing attention
- Suggests architectural improvements

### Recommendation Structure

Each recommendation includes:
- **Priority**: Low, Medium, High
- **Category**: Refactoring, Pattern, Formatting, General
- **Title**: Brief description of the issue
- **Description**: Detailed explanation
- **Estimated Impact**: Expected complexity reduction (in points)
- **Suggested Actions**: Specific steps to take

### Example Recommendations

```json
{
  "recommendations": [
    {
      "priority": "High",
      "category": "Refactoring",
      "title": "Simplify control flow in 'process_request'",
      "description": "This function contributes 12 complexity points with deeply nested conditions",
      "estimated_impact": 6,
      "suggested_actions": [
        "Extract validation logic into separate function",
        "Use early returns to reduce nesting depth",
        "Consider state pattern for complex branching"
      ]
    },
    {
      "priority": "Medium",
      "category": "Formatting",
      "title": "Formatting contributes 29% of measured complexity",
      "description": "Code formatting choices are inflating complexity metrics",
      "estimated_impact": 13,
      "suggested_actions": [
        "Use automated formatting tools (rustfmt, prettier)",
        "Standardize code formatting across the team",
        "Configure editor to format on save"
      ]
    },
    {
      "priority": "Low",
      "category": "Pattern",
      "title": "Low pattern recognition suggests unique code structure",
      "description": "Pattern confidence score of 0.45 indicates non-standard patterns",
      "estimated_impact": 3,
      "suggested_actions": [
        "Consider extracting common patterns into utilities",
        "Review for code duplication opportunities",
        "Document unique patterns for team understanding"
      ]
    }
  ]
}
```

## Performance Considerations

Multi-pass analysis adds overhead compared to single-pass analysis, but debtmap monitors and limits this overhead.

### Performance Metrics

When performance tracking is enabled, you'll see:

```
Performance Metrics
-------------------
Raw analysis: 145ms
Normalized analysis: 132ms
Attribution: 45ms
Total time: 322ms
Memory used: 12.3 MB

Overhead: 121.7% vs single-pass (145ms baseline)
⚠️  Warning: Overhead exceeds 25% target
```

> **Note**: Memory usage values are estimates based on parallelism level, not precise heap measurements.

**Tracked Metrics:**
- **Raw analysis time** - Time to analyze original code
- **Normalized analysis time** - Time to analyze normalized code
- **Attribution time** - Time to compute attribution breakdown
- **Total time** - Complete multi-pass analysis duration
- **Memory used** - Estimated additional memory for two-pass analysis

### Performance Overhead

**Target Overhead**: ≤25% compared to single-pass analysis

Multi-pass analysis aims to add no more than 25% overhead versus standard single-pass analysis. If overhead exceeds this threshold, a warning is issued.

**Typical Overhead:**
- Attribution adds ~10-15% on average
- Normalization adds ~5-10% on average
- Total overhead usually 15-25%

**Factors Affecting Performance:**
- **File size** - Larger files take proportionally longer
- **Complexity** - More complex code requires more analysis time
- **Language** - Some languages (TypeScript) are slower to parse
- **Parallel processing** - Overhead is per-file, parallel reduces impact

### Optimization Tips

**Disable Performance Tracking in Production**
```rust
MultiPassOptions {
    performance_tracking: false,  // Reduces overhead slightly
    ..Default::default()
}
```

**Use Parallel Processing**
```bash
# Parallel analysis amortizes overhead across cores
# Note: --jobs is a general debtmap flag controlling parallelism for all analysis
debtmap analyze . --multi-pass --jobs 8
```

**Target Specific Files**
```bash
# Analyze only files that need detailed attribution
debtmap analyze src/complex_module.rs --multi-pass --attribution
```

## Comparative Analysis

Multi-pass analysis supports comparing code changes to validate refactoring efforts.

### Basic Comparison

The `compare_complexity` function is a standalone convenience function that performs complete multi-pass analysis on both code versions and returns the computed differences:

```rust
use debtmap::analysis::multi_pass::compare_complexity;
use debtmap::core::Language;

let before_code = r#"
fn process(items: Vec<i32>) -> i32 {
    let mut sum = 0;
    for item in items {
        if item > 0 {
            if item % 2 == 0 {
                sum += item * 2;
            } else {
                sum += item;
            }
        }
    }
    sum
}
"#;

let after_code = r#"
fn process(items: Vec<i32>) -> i32 {
    items
        .into_iter()
        .filter(|&item| item > 0)
        .map(|item| if item % 2 == 0 { item * 2 } else { item })
        .sum()
}
"#;

let comparison = compare_complexity(before_code, after_code, Language::Rust)?;

println!("Complexity change: {}", comparison.complexity_change);
println!("Cognitive complexity change: {}", comparison.cognitive_change);
println!("Formatting impact change: {}", comparison.formatting_impact_change);
```

### Comparison Results

The `ComparativeAnalysis` struct contains the computed differences between before and after analyses:

```rust
pub struct ComparativeAnalysis {
    pub complexity_change: i32,        // Negative = improvement
    pub cognitive_change: i32,         // Negative = improvement
    pub formatting_impact_change: f32, // Negative = less formatting noise
    pub improvements: Vec<String>,
    pub regressions: Vec<String>,
}
```

> **Note**: The `compare_complexity` function performs both analyses internally and returns only the change metrics. To access the full before/after results, perform separate analyses using `MultiPassAnalyzer`.

**Interpreting Changes:**
- **Negative complexity change** - Refactoring reduced complexity ✓
- **Positive complexity change** - Refactoring increased complexity ✗
- **Improvements** - List of detected improvements (reduced nesting, extracted functions, etc.)
- **Regressions** - List of detected regressions (increased complexity, new anti-patterns, etc.)

### Example Output

```
Comparative Analysis
====================

Complexity Changes:
├─ Cyclomatic: 8 → 4 (-4, -50%)
├─ Cognitive: 12 → 5 (-7, -58.3%)
└─ Formatting Impact: 25% → 10% (-15%, -60%)

Improvements Detected:
✓ Reduced nesting depth (3 → 1)
✓ Eliminated mutable state
✓ Replaced imperative loop with functional chain
✓ Improved formatting consistency

No regressions detected.

Verdict: Refactoring reduced complexity by 50% and improved code clarity.
```

## Configuration Options

Configure multi-pass analysis programmatically:

```rust
use debtmap::analysis::multi_pass::{MultiPassAnalyzer, MultiPassOptions};
use debtmap::analysis::diagnostics::{DetailLevel, OutputFormat};
use debtmap::core::Language;

let options = MultiPassOptions {
    language: Language::Rust,
    detail_level: DetailLevel::Comprehensive,
    enable_recommendations: true,
    track_source_locations: true,
    generate_insights: true,
    output_format: OutputFormat::Json, // Also available: Yaml, Markdown, Html, Text
    performance_tracking: true,
};

let analyzer = MultiPassAnalyzer::new(options);
```

### Configuration Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `Language` | `Rust` | Target programming language |
| `detail_level` | `DetailLevel` | `Standard` | Output detail: Summary, Standard, Comprehensive, Debug (CLI uses lowercase: `--detail-level standard`) |
| `enable_recommendations` | `bool` | `true` | Generate actionable recommendations |
| `track_source_locations` | `bool` | `true` | Include file/line/column in attribution |
| `generate_insights` | `bool` | `true` | Automatically generate insights |
| `output_format` | `OutputFormat` | `Json` | Output format: Json, Yaml, Markdown, Html, Text |
| `performance_tracking` | `bool` | `false` | Track and report performance metrics |

## Use Cases

### When to Use Multi-Pass Analysis (Default)

Multi-pass analysis is the default because it provides the most valuable insights:

**Refactoring Validation**
- Compare before/after complexity to validate refactoring
- Ensure complexity actually decreased
- Identify unintended complexity increases

**Formatting Impact Assessment**
- Determine how much formatting affects your metrics
- Justify automated formatting tool adoption
- Identify formatting inconsistencies

**Targeted Refactoring**
- Use attribution to find highest-impact refactoring targets
- Focus on logical complexity, not formatting artifacts
- Prioritize functions with actionable suggestions

**Code Review**
- Provide objective complexity data in pull requests
- Identify genuine complexity increases vs formatting changes
- Guide refactoring discussions with data

**Codebase Health Monitoring**
- Track logical complexity trends over time
- Separate signal (logic) from noise (formatting)
- Identify complexity hotspots for architectural review

### When to Disable Multi-Pass (--no-multi-pass)

Use `--no-multi-pass` for single-pass analysis only when:

**Performance is Critical**
- Fast complexity checks during development
- CI/CD gates where every second matters
- Very large codebases (>100k LOC) where overhead is significant

**Simple Use Cases**
- When overall complexity trends are enough
- No need for detailed attribution
- Formatting is already standardized

**Resource Constraints**
- Limited CPU or memory available
- Running on CI infrastructure with strict time limits

## Future Enhancements

### Spec 84: Detailed AST-Based Source Mapping

The current implementation uses estimated complexity locations based on function metrics. [Spec 84](https://github.com/yourusername/debtmap/blob/master/specs/84-detailed-ast-source-mapping.md) will enhance attribution with precise AST-based source mapping:

**Planned Improvements:**
- **Exact AST node locations** - Precise line, column, and span for each complexity point
- **100% accurate mapping** - No estimation, direct AST-to-source mapping
- **IDE integration** - Jump from complexity reports directly to source code
- **Inline visualization** - Show complexity heat maps in your editor
- **Statement-level tracking** - Complexity attribution at statement granularity

**Current vs Future:**

Current (estimated):
```rust
ComplexityComponent {
    location: CodeLocation {
        line: 45,      // Function start line
        column: 0,     // Estimated
        span: None,    // Not available
    },
    description: "Function: process_request",
}
```

Future (precise):
```rust
ComplexityComponent {
    location: SourceLocation {
        line: 47,           // Exact if statement line
        column: 8,          // Exact column
        span: Some(47, 52), // Exact span of construct
        ast_path: "fn::process_request::body::if[0]",
    },
    description: "If condition: item.is_valid()",
}
```

This will enable:
- Click-to-navigate from reports to exact code locations
- Visual Studio Code / IntelliJ integration for inline complexity display
- More precise refactoring suggestions
- Better complexity trend tracking at fine granularity

## Summary

Multi-pass analysis (enabled by default) provides deep insights into your code's complexity by:

1. **Separating signal from noise** - Distinguishing logical complexity from formatting artifacts
2. **Attributing complexity sources** - Identifying what contributes to complexity and why
3. **Generating actionable insights** - Providing specific refactoring recommendations
4. **Validating refactoring** - Comparing before/after to prove complexity reduction
5. **Monitoring performance** - Ensuring overhead stays within acceptable bounds

Multi-pass analysis runs by default, providing the most valuable insights out of the box. The overhead (typically 15-25%) is worthwhile for understanding *why* code is complex and *how* to improve it.

For performance-critical scenarios or very large codebases, use `--no-multi-pass` to disable multi-pass analysis and run faster single-pass analysis instead. You can also use the `DEBTMAP_SINGLE_PASS=1` environment variable to disable multi-pass analysis globally.

---

**See Also:**
- [Analysis Guide](analysis-guide/index.md) - General analysis capabilities
- [Scoring Strategies](scoring-strategies.md) - How complexity affects debt scores
- [Coverage Integration](coverage-integration.md) - Combining complexity with coverage
- [Examples](examples.md) - Real-world multi-pass analysis examples
