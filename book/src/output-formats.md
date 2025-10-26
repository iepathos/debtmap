# Output Formats

Debtmap provides multiple output formats to suit different workflows, from interactive terminal reports to machine-readable JSON for CI/CD integration. This chapter covers all available formats and how to use them effectively.

## Format Selection

Select the output format using the `-f` or `--format` flag:

```bash
# Terminal output (default) - human-readable with colors
debtmap analyze .

# JSON output - machine-readable for tooling
debtmap analyze . --format json

# Markdown output - documentation and reports
debtmap analyze . --format markdown
```

Available formats:
- **terminal** (default): Interactive output with colors, emoji, and formatting
- **json**: Structured data for programmatic processing
- **markdown**: Reports suitable for documentation and PR comments

### Writing to Files

By default, output goes to stdout. Use `-o` or `--output` to write to a file:

```bash
# Write JSON to file
debtmap analyze . --format json -o report.json

# Write markdown report
debtmap analyze . --format markdown -o DEBT_REPORT.md

# Terminal output to file (preserves colors)
debtmap analyze . -o analysis.txt
```

## Terminal Output

The terminal format provides an interactive, color-coded report designed for developer workflows. It's the default format and optimized for readability.

### Output Structure

Terminal output is organized into five main sections:

1. **Header** - Analysis report title
2. **Codebase Summary** - High-level metrics and debt score
3. **Complexity Hotspots** - Top 5 most complex functions with refactoring guidance
4. **Technical Debt** - High-priority debt items requiring attention
5. **Pass/Fail Status** - Overall quality assessment

### Example Terminal Output

```
═══════════════════════════════════════════
           DEBTMAP ANALYSIS REPORT
═══════════════════════════════════════════

📊 CODEBASE Summary
───────────────────────────────────────────
  Files analyzed:      42
  Total functions:     287
  Average complexity:  6.3
  Debt items:          15
  Total debt score:    156 (threshold: 100)

⚠️  COMPLEXITY HOTSPOTS (Top 5)
───────────────────────────────────────────
  1. src/analyzers/rust.rs:245 parse_function() - Cyclomatic: 18, Cognitive: 24
     ACTION: Extract 3-5 pure functions using decompose-then-transform strategy
     PATTERNS: Decompose into logical units, then apply functional patterns
     BENEFIT: Pure functions are easily testable and composable

  2. src/debt/smells.rs:196 detect_data_clumps() - Cyclomatic: 15, Cognitive: 20
     ↓ Entropy: 0.32, Repetition: 85%, Effective: 0.6x
       High pattern repetition detected (85%)

🔧 TECHNICAL DEBT (15 items)
───────────────────────────────────────────
  High Priority (5):
    - src/risk/scoring.rs:142 - TODO: Implement caching for score calculations
    - src/core/metrics.rs:89 - High complexity: cyclomatic=16
    - src/debt/patterns.rs:201 - Code duplication: 65 lines duplicated

✓ Pass/Fail: PASS
```

### Color Coding and Symbols

The terminal output uses colors and symbols for quick visual scanning:

**Status Indicators:**
- ✓ Green: Passing, good, well-tested
- ⚠️  Yellow: Warning, moderate complexity
- ✗ Red: Failing, critical, high complexity
- 📊 Blue: Information, metrics
- 🔧 Orange: Technical debt items
- 🎯 Cyan: Recommendations

**Complexity Classification:**
- **LOW** (0-5): Green - Simple, easy to maintain
- **MODERATE** (6-10): Yellow - Consider refactoring
- **HIGH** (11-15): Orange - Should refactor
- **SEVERE** (>15): Red - Urgent refactoring needed

> **Note:** These levels match the `ComplexityLevel` enum in the implementation.

**Debt Score Thresholds:**

The default debt threshold is **100**. Scores are colored based on this threshold:
- **Green (≤50)**: Healthy - Below half threshold (score ≤ threshold/2)
- **Yellow (51-100)**: Attention needed - Between half and full threshold (threshold/2 < score ≤ threshold)
- **Red (>100)**: Action required - Exceeds threshold (score > threshold)

> **Note:** Boundary values use strict inequalities: 50 is Green, 100 is Yellow (not Red), 101+ is Red.

### Refactoring Guidance

For complex functions (cyclomatic complexity > 5), the terminal output provides actionable refactoring recommendations:

```
ACTION: Extract 3-5 pure functions using decompose-then-transform strategy
PATTERNS: Decompose into logical units, then apply functional patterns
BENEFIT: Pure functions are easily testable and composable
```

Guidance levels:
- **Moderate** (6-10): Extract 2-3 pure functions using direct functional transformation
- **High** (11-15): Extract 3-5 pure functions using decompose-then-transform strategy
- **Severe** (>15): Extract 5+ pure functions into modules with functional core/imperative shell

See the [Analysis Guide](./analysis-guide.md) for metric explanations.

### Plain Terminal Mode

For environments without color support or when piping to tools, use `--plain`:

```bash
# ASCII-only output, no colors
debtmap analyze . --plain
```

Plain mode:
- Removes ANSI color codes
- Uses ASCII box-drawing characters
- Machine-parseable structure

> **Note:** Terminal output formatting can be customized via `FormattingConfig`, which controls color mode. The `--plain` flag uses this configuration to disable colors. Additionally, you can control formatting through environment variables:
> - `NO_COLOR=1` - Disables colors (per [no-color.org](https://no-color.org) standard)
> - `CLICOLOR=0` - Disables colors
> - `CLICOLOR_FORCE=1` - Forces colors even when output is not a terminal

### Verbosity Levels

Control detail level with `-v` flags (can be repeated):

```bash
# Standard output
debtmap analyze .

# Level 1: Show main score factors
debtmap analyze . -v

# Level 2: Show detailed calculations
debtmap analyze . -vv

# Level 3: Show all debug information
debtmap analyze . -vvv
```

**Verbosity features:**
- `-v`: Show main score factors (complexity, coverage, dependency breakdown)
- `-vv`: Show detailed calculations with formulas and intermediate values
- `-vvv`: Show all debug information including entropy metrics, role detection, and cache hits

> **Note:** Verbosity flags affect terminal output only. JSON and markdown formats include all data regardless of verbosity level.

Each level includes all information from the previous levels, progressively adding more detail to help understand how scores are calculated.

**Example Output Differences:**

Standard output shows basic metrics:
```
Total debt score: 156 (threshold: 100)
```

Level 1 (`-v`) adds score breakdowns:
```
Total debt score: 156 (threshold: 100)
  Complexity contribution: 85 (54%)
  Coverage gaps: 45 (29%)
  Dependency issues: 26 (17%)
```

Level 2 (`-vv`) adds detailed calculations:
```
Total debt score: 156 (threshold: 100)
  Complexity contribution: 85 (54%)
    Formula: sum(cyclomatic_weight * severity_multiplier)
    High complexity functions: 5 × 12 = 60
    Medium complexity: 8 × 3 = 24
    Base penalty: 1
  Coverage gaps: 45 (29%)
    Uncovered complex functions: 3 × 15 = 45
```

Level 3 (`-vvv`) adds all internal details:
```
Total debt score: 156 (threshold: 100)
  ... (all level 2 output) ...
  Debug info:
    Entropy metrics cached: 42/50 functions
    Function role detection: BusinessLogic=12, Utility=8, TestHelper=5
    Cache hit rate: 84%
```

### Understanding Metrics

To get detailed explanations of how metrics are calculated, use the `--explain-metrics` flag:

```bash
# Get explanations of metric definitions and formulas
debtmap analyze . --explain-metrics
```

This flag provides:
- **Metric definitions** - Detailed explanations of what each metric measures
- **Calculation formulas** - How scores are computed from raw data
- **Measured vs estimated** - Which metrics are exact and which are heuristic-based
- **Interpretation guidance** - How to understand and act on metric values

The explanations appear inline with the analysis output, helping you understand:
- What cyclomatic and cognitive complexity measure
- How debt scores are calculated
- What entropy metrics indicate
- How risk scores are determined

This is particularly useful when:
- Learning how debtmap evaluates code quality
- Understanding why certain functions have high scores
- Explaining analysis results to team members
- Tuning thresholds based on metric meanings

### Risk Analysis Output

When coverage data is provided via `--lcov`, terminal output includes a dedicated risk analysis section:

```
═══════════════════════════════════════════
           RISK ANALYSIS REPORT
═══════════════════════════════════════════

📈 RISK Summary
───────────────────────────────────────────
Codebase Risk Score: 45.5 (MEDIUM)
Complexity-Coverage Correlation: -0.65

Risk Distribution:
  Critical: 2 functions
  High: 5 functions
  Medium: 10 functions
  Low: 15 functions
  Well Tested: 20 functions

🎯 CRITICAL RISKS
───────────────────────────────────────────
1. src/core/parser.rs:142 parse_complex_ast()
   Risk: 85.0 | Complexity: 15 | Coverage: 0%
   Recommendation: Add 5 unit tests (est: 2-3 hours)
   Impact: -40 risk reduction

💡 RECOMMENDATIONS (by ROI)
───────────────────────────────────────────
1. test_me() - ROI: 5.0x
   Current Risk: 75 | Reduction: 40 | Effort: Moderate
   Rationale: High risk function with low coverage
```

**Risk Level Classification:**
- **LOW** (<30): Green - score < 30.0
- **MEDIUM** (30-59): Yellow - 30.0 ≤ score < 60.0
- **HIGH** (≥60): Red - score ≥ 60.0

> **Note:** 60 is the start of HIGH risk level.

## JSON Output

JSON output provides complete analysis results in a machine-readable format, ideal for CI/CD pipelines, custom tooling, and programmatic analysis.

### Basic Usage

```bash
# Generate JSON output
debtmap analyze . --format json

# Save to file
debtmap analyze . --format json -o report.json

# Pretty-printed by default for readability
debtmap analyze . --format json | jq .
```

> **Note:** JSON output is automatically pretty-printed for readability.

### JSON Schema Structure

Debtmap outputs a structured JSON document with the following top-level fields:

```json
{
  "project_path": "/path/to/project",
  "timestamp": "2025-01-09T12:00:00Z",
  "complexity": { ... },
  "technical_debt": { ... },
  "dependencies": { ... },
  "duplications": [ ... ]
}
```

### Full Schema Example

Here's a complete annotated JSON output example:

```json
{
  // Project metadata
  "project_path": "/Users/dev/myproject",
  "timestamp": "2025-01-09T15:30:00Z",

  // Complexity analysis results
  "complexity": {
    "metrics": [
      {
        "name": "calculate_risk_score",
        "file": "src/risk/scoring.rs",
        "line": 142,
        "cyclomatic": 12,
        "cognitive": 18,
        "nesting": 4,
        "length": 85,
        "is_test": false,
        "visibility": "pub",
        "is_trait_method": false,
        "in_test_module": false,
        "entropy_score": {
          "token_entropy": 0.65,
          "pattern_repetition": 0.30,
          "branch_similarity": 0.45,
          "effective_complexity": 0.85
        },
        "is_pure": false,
        "purity_confidence": 0.75,
        "detected_patterns": ["nested_loops", "complex_conditionals"],
        "upstream_callers": ["analyze_codebase", "generate_report"],
        "downstream_callees": ["get_metrics", "apply_weights"]
      }
    ],
    "summary": {
      "total_functions": 287,
      "average_complexity": 6.3,
      "max_complexity": 24,
      "high_complexity_count": 12
    }
  },

  // Technical debt items
  "technical_debt": {
    "items": [
      {
        "id": "debt_001",
        "debt_type": "Complexity",
        "priority": "High",
        "file": "src/analyzers/rust.rs",
        "line": 245,
        "column": 5,
        "message": "High cyclomatic complexity: 18",
        "context": "Function parse_function has excessive branching"
      },
      {
        "id": "debt_002",
        "debt_type": "Todo",
        "priority": "Medium",
        "file": "src/core/cache.rs",
        "line": 89,
        "column": null,
        "message": "TODO: Implement LRU eviction policy",
        "context": null
      }
    ],
    "by_type": {
      "Complexity": [ /* same structure as items */ ],
      "Todo": [ /* ... */ ],
      "Duplication": [ /* ... */ ]
    },
    "priorities": ["Low", "Medium", "High", "Critical"]
  },

  // Dependency analysis
  "dependencies": {
    "modules": [
      {
        "module": "risk::scoring",
        "dependencies": ["core::metrics", "debt::patterns"],
        "dependents": ["commands::analyze", "io::output"]
      }
    ],
    "circular": [
      {
        "cycle": ["module_a", "module_b", "module_c", "module_a"]
      }
    ]
  },

  // Code duplication blocks
  "duplications": [
    {
      "hash": "abc123def456",
      "lines": 15,
      "locations": [
        {
          "file": "src/parser/rust.rs",
          "start_line": 42,
          "end_line": 57
        },
        {
          "file": "src/parser/python.rs",
          "start_line": 89,
          "end_line": 104
        }
      ]
    }
  ]
}
```

### Field Descriptions

**FunctionMetrics Fields:**
- `name`: Function name
- `file`: Path to source file
- `line`: Line number where function is defined
- `cyclomatic`: Cyclomatic complexity score
- `cognitive`: Cognitive complexity score
- `nesting`: Maximum nesting depth
- `length`: Lines of code in function
- `is_test`: Whether this is a test function
- `visibility`: Rust visibility modifier (pub, pub(crate), or null)
- `is_trait_method`: Whether this implements a trait
- `in_test_module`: Whether inside #[cfg(test)]
- `entropy_score`: Optional entropy analysis with structure:
  ```json
  {
    "token_entropy": 0.65,        // Token distribution entropy (0-1): measures variety of tokens
    "pattern_repetition": 0.30,   // Pattern repetition score (0-1): detects repeated code patterns
    "branch_similarity": 0.45,    // Branch similarity metric (0-1): compares similarity between branches
    "effective_complexity": 0.85  // Adjusted complexity multiplier: complexity adjusted for entropy
  }
  ```

  **EntropyScore Fields:**
  - `token_entropy`: Measures the variety and distribution of tokens in the function (0-1, higher = more variety)
  - `pattern_repetition`: Detects repeated code patterns within the function (0-1, higher = more repetition)
  - `branch_similarity`: Measures similarity between different code branches (0-1, higher = more similar)
  - `effective_complexity`: The overall complexity multiplier adjusted for entropy effects
- `is_pure`: Whether function is pure (no side effects)
- `purity_confidence`: Confidence level (0.0-1.0)
- `detected_patterns`: List of detected code patterns
- `upstream_callers`: Functions that call this one
- `downstream_callees`: Functions this one calls

**DebtItem Fields:**
- `id`: Unique identifier
- `debt_type`: Type of debt (see DebtType enum below)
- `priority`: Priority level (Low, Medium, High, Critical)
- `file`: Path to file containing debt
- `line`: Line number
- `column`: Optional column number
- `message`: Human-readable description
- `context`: Optional additional context

**DebtType Enum:**
- `Todo`: TODO markers
- `Fixme`: FIXME markers
- `CodeSmell`: Code smell patterns
- `Duplication`: Duplicated code
- `Complexity`: Excessive complexity
- `Dependency`: Dependency issues
- `ErrorSwallowing`: Suppressed errors
- `ResourceManagement`: Resource management issues
- `CodeOrganization`: Organizational problems
- `TestComplexity`: Complex test code
- `TestTodo`: TODOs in tests
- `TestDuplication`: Duplicated test code
- `TestQuality`: Test quality issues

### JSON Format Variants

Debtmap supports two JSON output formats:

```bash
# Legacy format (default) - backward compatible
debtmap analyze . --format json --output-format legacy

# Unified format - new consistent structure
debtmap analyze . --format json --output-format unified
```

> **Note:** The `--output-format` flag only applies when using `--format json`. It has no effect with markdown or terminal formats.

#### Format Comparison

**Legacy format:** Uses `{File: {...}}` and `{Function: {...}}` wrappers for backward compatibility with existing tooling.

**Unified format:** Consistent structure with a `type` field, making parsing simpler and more predictable. Recommended for new integrations.

**When to use each format:**

- **Use legacy format if:**
  - You have existing tooling that expects the old structure
  - You need backward compatibility with version 1.x parsers
  - You're integrating with third-party tools expecting the legacy format

- **Use unified format for:**
  - All new integrations and tooling
  - Cleaner, more predictable JSON parsing
  - Future-proof implementations
  - Simpler type discrimination in statically-typed languages

**Migration strategy:**

The legacy format will be maintained for backward compatibility, but unified is the recommended format going forward. If you're starting a new integration, use unified format from the beginning. If migrating existing tooling:

1. Test unified format with a subset of your codebase
2. Update parsers to handle the `type` field instead of key-based discrimination
3. Validate results match between legacy and unified formats
4. Switch to unified format once validation passes

#### Structural Differences

**Legacy format example:**
```json
{
  "complexity": {
    "metrics": [
      {
        "File": {
          "path": "src/main.rs",
          "functions": 12,
          "average_complexity": 5.3
        }
      },
      {
        "Function": {
          "name": "calculate_score",
          "file": "src/scoring.rs",
          "line": 42,
          "cyclomatic": 8
        }
      }
    ]
  }
}
```

**Unified format example:**
```json
{
  "complexity": {
    "metrics": [
      {
        "type": "File",
        "path": "src/main.rs",
        "functions": 12,
        "average_complexity": 5.3
      },
      {
        "type": "Function",
        "name": "calculate_score",
        "file": "src/scoring.rs",
        "line": 42,
        "cyclomatic": 8
      }
    ]
  }
}
```

**Key difference:** Legacy uses `{File: {...}}` wrapper objects, while unified uses a flat structure with `"type": "File"` field. This makes unified format easier to parse in most programming languages.

### Risk Insights JSON

When using `--lcov`, debtmap also outputs risk analysis in JSON:

```json
{
  "items": [
    {
      "location": {
        "file": "src/risk/scoring.rs",
        "function": "calculate_priority",
        "line": 66
      },
      "debt_type": "TestGap",
      "unified_score": {
        "complexity_factor": 3.2,
        "coverage_factor": 10.0,
        "dependency_factor": 2.5,
        "role_multiplier": 1.2,
        "final_score": 9.4
      },
      "function_role": "BusinessLogic",
      "recommendation": {
        "action": "Add unit tests",
        "details": "Add 6 unit tests for full coverage",
        "effort_estimate": "2-3 hours"
      },
      "expected_impact": {
        "risk_reduction": 3.9,
        "complexity_reduction": 0,
        "coverage_improvement": 100
      },
      "upstream_dependencies": 0,
      "downstream_dependencies": 3,
      "nesting_depth": 1,
      "function_length": 13
    }
  ],
  "call_graph": {
    "total_functions": 1523,
    "entry_points": 12,
    "test_functions": 456,
    "max_depth": 8
  },
  "overall_coverage": 82.3,
  "total_impact": {
    "risk_reduction": 45.2,
    "complexity_reduction": 12.3,
    "coverage_improvement": 18.5
  }
}
```

## Markdown Output

Markdown format generates documentation-friendly reports suitable for README files, PR comments, and technical documentation.

### Basic Usage

```bash
# Generate markdown report
debtmap analyze . --format markdown

# Save to documentation
debtmap analyze . --format markdown -o docs/DEBT_REPORT.md
```

### Markdown Structure

Markdown output includes:

1. **Executive Summary** - High-level metrics and health dashboard
2. **Complexity Analysis** - Detailed complexity breakdown by file
3. **Technical Debt** - Categorized debt items with priorities
4. **Dependencies** - Module dependencies and circular references
5. **Recommendations** - Prioritized action items

### Example Markdown Output

```markdown
# Debtmap Analysis Report

**Generated:** 2025-01-09 15:30:00 UTC
**Project:** /Users/dev/myproject

## Executive Summary

- **Files Analyzed:** 42
- **Total Functions:** 287
- **Average Complexity:** 6.3
- **Total Debt Items:** 15
- **Debt Score:** 156/100 ⚠️

### Health Dashboard

| Metric | Value | Status |
|--------|-------|--------|
| Complexity | 6.3 avg | ✅ Good |
| Debt Score | 156 | ⚠️ Attention |
| High Priority Items | 5 | ⚠️ Action Needed |

## Complexity Analysis

### Top 5 Complex Functions

| Function | File | Cyclomatic | Cognitive | Priority |
|----------|------|-----------|-----------|----------|
| parse_function | src/analyzers/rust.rs:245 | 18 | 24 | High |
| detect_data_clumps | src/debt/smells.rs:196 | 15 | 20 | Medium |
| analyze_dependencies | src/core/deps.rs:89 | 14 | 18 | Medium |

### Refactoring Recommendations

**src/analyzers/rust.rs:245** - `parse_function()`
- **Complexity:** Cyclomatic: 18, Cognitive: 24
- **Action:** Extract 3-5 pure functions using decompose-then-transform strategy
- **Patterns:** Decompose into logical units, then apply functional patterns
- **Benefit:** Improved testability and maintainability

## Technical Debt

### High Priority (5 items)

- **src/risk/scoring.rs:142** - TODO: Implement caching for score calculations
- **src/core/metrics.rs:89** - High complexity: cyclomatic=16
- **src/debt/patterns.rs:201** - Code duplication: 65 lines duplicated

### Medium Priority (8 items)

...

## Dependencies

### Circular Dependencies

- `risk::scoring` → `core::metrics` → `risk::scoring`

## Recommendations

1. **Refactor parse_function** (High Priority)
   - Reduce complexity from 18 to <10
   - Extract helper functions
   - Estimated effort: 4-6 hours

2. **Add tests for scoring module** (High Priority)
   - Current coverage: 35%
   - Target coverage: 80%
   - Estimated effort: 2-3 hours
```

### Enhanced Markdown Features

The standard markdown output already includes comprehensive analysis sections. The codebase includes additional enhanced markdown capabilities (`EnhancedMarkdownWriter` trait in `src/io/writers/markdown/enhanced.rs`) that provide:

- **Priority-based debt rankings** - Debt items ranked by unified priority scores
- **Dead code detection** - Identification and reporting of unused code
- **Call graph insights** - Function dependency and usage analysis
- **Testing recommendations** - Targeted suggestions for improving test coverage

These enhanced features are available through the `EnhancedMarkdownWriter` trait when using debtmap as a library. The standard `--format markdown` CLI output uses the base `MarkdownWriter` which provides comprehensive reports including:

- Executive summary with health dashboard
- Complexity analysis with refactoring recommendations
- Technical debt categorization by priority
- Dependency analysis with circular reference detection
- Actionable recommendations

For additional visualization capabilities, the `src/io/writers/enhanced_markdown/` module provides building blocks for custom report generation when using debtmap as a library in your own tools.

### Rendering to HTML/PDF

Markdown reports can be converted to other formats:

```bash
# Generate markdown
debtmap analyze . --format markdown -o report.md

# Convert to HTML with pandoc
pandoc report.md -o report.html --standalone --css style.css

# Convert to PDF
pandoc report.md -o report.pdf --pdf-engine=xelatex
```

## Tool Integration

### CI/CD Pipelines

Debtmap JSON output integrates seamlessly with CI/CD systems.

#### GitHub Actions

```yaml
name: Code Quality

on: [pull_request]

jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install debtmap
        run: cargo install debtmap

      - name: Run analysis
        run: |
          debtmap analyze . \
            --format json \
            --output analysis.json \
            --lcov coverage/lcov.info

      - name: Check thresholds
        run: |
          DEBT_SCORE=$(jq '.technical_debt.items | length' analysis.json)
          if [ "$DEBT_SCORE" -gt 100 ]; then
            echo "❌ Debt score too high: $DEBT_SCORE"
            exit 1
          fi

      - name: Comment on PR
        uses: actions/github-script@v6
        with:
          script: |
            const fs = require('fs');
            const analysis = JSON.parse(fs.readFileSync('analysis.json'));
            const summary = `## Debtmap Analysis

            - **Debt Items:** ${analysis.technical_debt.items.length}
            - **Average Complexity:** ${analysis.complexity.summary.average_complexity}
            - **High Complexity Functions:** ${analysis.complexity.summary.high_complexity_count}
            `;
            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: summary
            });
```

#### GitLab CI

```yaml
code_quality:
  stage: test
  script:
    - cargo install debtmap
    - debtmap analyze . --format json --output gl-code-quality.json
    - |
      DEBT=$(jq '.technical_debt.items | length' gl-code-quality.json)
      if [ "$DEBT" -gt 50 ]; then
        echo "Debt threshold exceeded"
        exit 1
      fi
  artifacts:
    reports:
      codequality: gl-code-quality.json
```

#### Jenkins Pipeline

```groovy
pipeline {
    agent any

    stages {
        stage('Analyze') {
            steps {
                sh 'debtmap analyze . --format json -o report.json'

                script {
                    def json = readJSON file: 'report.json'
                    def debtScore = json.technical_debt.items.size()

                    if (debtScore > 100) {
                        error("Debt score ${debtScore} exceeds threshold")
                    }
                }
            }
        }
    }

    post {
        always {
            archiveArtifacts artifacts: 'report.json'
        }
    }
}
```

### Querying JSON with jq

Common jq queries for analyzing debtmap output:

```bash
# Get total debt items
jq '.technical_debt.items | length' report.json

# Get high-priority items only
jq '.technical_debt.items[] | select(.priority == "High")' report.json

# Get functions with complexity > 10
jq '.complexity.metrics[] | select(.cyclomatic > 10)' report.json

# Calculate average complexity
jq '.complexity.summary.average_complexity' report.json

# Get all TODO items
jq '.technical_debt.items[] | select(.debt_type == "Todo")' report.json

# Get top 5 complex functions
jq '.complexity.metrics | sort_by(-.cyclomatic) | .[0:5] | .[] | {name, file, cyclomatic}' report.json

# Get files with circular dependencies
jq '.dependencies.circular[] | .cycle' report.json

# Count debt items by type
jq '.technical_debt.items | group_by(.debt_type) | map({type: .[0].debt_type, count: length})' report.json

# Get functions with 0% coverage (when using --lcov)
jq '.complexity.metrics[] | select(.coverage == 0)' report.json

# Extract file paths with high debt
jq '.technical_debt.items[] | select(.priority == "High" or .priority == "Critical") | .file' report.json | sort -u
```

### Filtering and Transformation Examples

#### Python Script to Parse JSON

```python
#!/usr/bin/env python3
import json
import sys

def analyze_debtmap_output(json_file):
    with open(json_file) as f:
        data = json.load(f)

    # Get high-priority items
    high_priority = [
        item for item in data['technical_debt']['items']
        if item['priority'] in ['High', 'Critical']
    ]

    # Group by file
    by_file = {}
    for item in high_priority:
        file = item['file']
        if file not in by_file:
            by_file[file] = []
        by_file[file].append(item)

    # Print summary
    print(f"High-priority debt items: {len(high_priority)}")
    print(f"Files affected: {len(by_file)}")
    print("\nBy file:")
    for file, items in sorted(by_file.items(), key=lambda x: -len(x[1])):
        print(f"  {file}: {len(items)} items")

    return by_file

if __name__ == '__main__':
    analyze_debtmap_output(sys.argv[1])
```

#### Shell Script for Threshold Checking

```bash
#!/bin/bash
set -e

REPORT="$1"
DEBT_THRESHOLD=100
COMPLEXITY_THRESHOLD=10

# Check debt score
DEBT_SCORE=$(jq '.technical_debt.items | length' "$REPORT")
if [ "$DEBT_SCORE" -gt "$DEBT_THRESHOLD" ]; then
    echo "❌ Debt score $DEBT_SCORE exceeds threshold $DEBT_THRESHOLD"
    exit 1
fi

# Check average complexity
AVG_COMPLEXITY=$(jq '.complexity.summary.average_complexity' "$REPORT")
if (( $(echo "$AVG_COMPLEXITY > $COMPLEXITY_THRESHOLD" | bc -l) )); then
    echo "❌ Average complexity $AVG_COMPLEXITY exceeds threshold $COMPLEXITY_THRESHOLD"
    exit 1
fi

echo "✅ All quality checks passed"
echo "   Debt score: $DEBT_SCORE/$DEBT_THRESHOLD"
echo "   Avg complexity: $AVG_COMPLEXITY"
```

### Editor Integration

#### VS Code Tasks

Create `.vscode/tasks.json`:

```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "Debtmap: Analyze",
      "type": "shell",
      "command": "debtmap",
      "args": [
        "analyze",
        ".",
        "--format",
        "terminal"
      ],
      "problemMatcher": [],
      "presentation": {
        "reveal": "always",
        "panel": "new"
      }
    },
    {
      "label": "Debtmap: Generate Report",
      "type": "shell",
      "command": "debtmap",
      "args": [
        "analyze",
        ".",
        "--format",
        "markdown",
        "-o",
        "DEBT_REPORT.md"
      ],
      "problemMatcher": []
    }
  ]
}
```

#### Problem Matcher for VS Code

Parse debtmap output in VS Code's Problems panel:

```json
{
  "problemMatcher": {
    "owner": "debtmap",
    "fileLocation": "absolute",
    "pattern": {
      "regexp": "^(.+?):(\\d+):(\\d+)?\\s*-\\s*(.+)$",
      "file": 1,
      "line": 2,
      "column": 3,
      "message": 4
    }
  }
}
```

### Webhook Integration

Send debtmap results to webhooks for notifications:

```bash
#!/bin/bash

# Run analysis
debtmap analyze . --format json -o report.json

# Send to Slack
DEBT_SCORE=$(jq '.technical_debt.items | length' report.json)
curl -X POST "$SLACK_WEBHOOK_URL" \
  -H 'Content-Type: application/json' \
  -d "{\"text\": \"Debtmap Analysis Complete\n• Debt Score: $DEBT_SCORE\n• High Priority: $(jq '[.technical_debt.items[] | select(.priority == "High")] | length' report.json)\"}"

# Send to custom webhook
curl -X POST "$CUSTOM_WEBHOOK_URL" \
  -H 'Content-Type: application/json' \
  -d @report.json
```

## Output Filtering

Debtmap provides several flags to filter and limit output:

> **Note:** Filtering options (`--top`, `--tail`, `--summary`, `--filter`) apply to all output formats (terminal, JSON, and markdown). The filtered data is applied at the analysis level before formatting, ensuring consistent results across all output types.

### Limiting Results

```bash
# Show only top 10 priority items
debtmap analyze . --top 10

# Show bottom 5 lowest priority items
debtmap analyze . --tail 5
```

### Priority Filtering

```bash
# Show only high and critical priority items
debtmap analyze . --min-priority high

# Filter by specific debt categories
debtmap analyze . --filter Architecture,Testing
```

Available categories:
- `Architecture`: God objects, complexity hotspots, dead code
- `Testing`: Testing gaps, coverage issues
- `Performance`: Resource leaks, inefficient patterns
- `CodeQuality`: Code smells, maintainability

### Grouping Output

```bash
# Group results by debt category
debtmap analyze . --group-by-category

# Combine filters for focused analysis
debtmap analyze . --filter Architecture --min-priority high --top 5
```

### Summary Mode

```bash
# Compact tiered priority display
debtmap analyze . --summary

# Combines well with filtering
debtmap analyze . --summary --min-priority medium
```

## Best Practices

### When to Use Each Format

**Use Terminal Format When:**
- Developing locally and reviewing code
- Getting quick feedback on changes
- Presenting results to team members
- Exploring complexity hotspots interactively

**Use JSON Format When:**
- Integrating with CI/CD pipelines
- Building custom analysis tools
- Tracking metrics over time
- Programmatically processing results
- Feeding into dashboards or monitoring systems

**Use Markdown Format When:**
- Generating documentation
- Creating PR comments
- Sharing reports with stakeholders
- Archiving analysis results
- Producing executive summaries

### Quick Reference Table

| Format | Best For | Machine Readable | Human Readable | File Extension |
|--------|----------|------------------|----------------|----------------|
| Terminal | Development | No | Yes | .txt |
| JSON | Automation | Yes | No | .json |
| Markdown | Documentation | Partially | Yes | .md |

### Combining Formats

Use multiple formats for comprehensive workflows:

```bash
# Generate terminal output for review
debtmap analyze .

# Generate JSON for automation
debtmap analyze . --format json -o ci-report.json

# Generate markdown for documentation
debtmap analyze . --format markdown -o docs/DEBT.md
```

### Performance Considerations

- **Terminal format**: Fastest, minimal overhead
- **JSON format**: Fast serialization, efficient for large codebases
- **Markdown format**: Slightly slower due to formatting, but still performant

For very large codebases (>10,000 files), use `--top` or `--filter` to limit output size.

## Troubleshooting

### Common Issues

**Colors not showing in terminal:**
- Check if terminal supports ANSI colors
- Use `--plain` flag for ASCII-only output
- Some CI systems may not support color codes

**JSON parsing errors:**
- Ensure output is complete (check for errors during analysis)
- Validate JSON with `jq` or online validators
- Check for special characters in file paths

**Markdown rendering issues:**
- Some markdown renderers don't support all features
- Use standard markdown for maximum compatibility
- Test with pandoc or GitHub/GitLab preview

**File encoding problems:**
- Ensure UTF-8 encoding for all output files
- Use `--plain` for pure ASCII output
- Check locale settings (LC_ALL, LANG environment variables)

### Exit Codes

> **IMPORTANT:** Exit codes 1 and 2 are NOT YET IMPLEMENTED. Current behavior: Always returns `0` on successful analysis, regardless of threshold violations.
>
> Planned behavior includes:
> - `0`: Success, all checks passed
> - `1`: Analysis completed, but validation thresholds exceeded
> - `2`: Error during analysis (invalid path, parsing error, etc.)

For now, use the `validate` command with threshold checks to enforce quality gates:

```bash
# Use validate command for threshold enforcement
debtmap validate . --config debtmap.toml

# Or parse JSON output for threshold checking
debtmap analyze . --format json -o report.json
DEBT_SCORE=$(jq '.technical_debt.items | length' report.json)
if [ "$DEBT_SCORE" -gt 100 ]; then
    echo "Debt threshold exceeded"
    exit 1
fi
```

## See Also

- [Getting Started](./getting-started.md) - Basic usage and examples
- [Analysis Guide](./analysis-guide.md) - Understanding metrics and scores
- [Configuration](./configuration.md) - Customizing analysis behavior
