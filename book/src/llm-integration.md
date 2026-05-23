# LLM Integration Guide

How to use debtmap output in AI coding workflows.

## Overview

Debtmap is designed to provide AI coding assistants with the signals they need to understand and fix technical debt. This guide covers:

1. Output formats optimized for LLMs
2. Context suggestions and how to use them
3. Example workflows for different AI tools
4. Interpreting signals for effective prompts

Debtmap currently analyzes Rust, Python, JavaScript, and TypeScript. All languages are enabled by default; use `--languages rust,python,javascript,typescript` when you want to constrain a run.

## Output Formats

### Markdown (Recommended)

The `markdown` format is specifically designed for LLM consumption:

```bash
debtmap analyze . --format markdown --top 5
```

**Output structure:**
```markdown
# Technical Debt Analysis

## Summary
- Total items: 47
- Critical: 3
- High: 12
- Medium: 20
- Low: 12

## Top Priority Items

### #1 [CRITICAL] parse_complex_input
**Location:** src/parser.rs:38-85
**Score:** 128.4

**Signals:**
| Metric | Value | Threshold |
|--------|-------|-----------|
| Cyclomatic | 12 | 10 |
| Cognitive | 18 | 15 |
| Coverage | 0.00 | 0.80 |
| Nesting | 4 | 3 |

**Context to read:**
- Primary: src/parser.rs:38-85
- Caller: src/handler.rs:100-120
- Caller: src/api.rs:45-60
- Test: tests/parser_test.rs:50-75

---

### #2 [CRITICAL] validate_auth
...
```

**Why this format works:**
- Consistent structure across all items
- Tables for easy metric comparison
- Context suggestions inline
- Minimal tokens for maximum information

### JSON

For programmatic access and CI/CD integration:

```bash
debtmap analyze . --format json --output debt.json
```

**Structure:**
```json
{
  "format_version": "3.0",
  "metadata": {
    "debtmap_version": "0.16.5",
    "generated_at": "2026-05-18T10:30:00Z",
    "project_root": ".",
    "analysis_type": "unified"
  },
  "summary": {
    "total_items": 47,
    "total_debt_score": 2140.5,
    "debt_density": 138.8,
    "total_loc": 15420,
    "score_distribution": {
      "critical": 3,
      "high": 12,
      "medium": 20,
      "low": 12
    }
  },
  "items": [
    {
      "type": "Function",
      "score": 128.4,
      "category": "Testing",
      "priority": "critical",
      "location": {
        "file": "src/parser.rs",
        "line": 38,
        "function": "parse_complex_input"
      },
      "metrics": {
        "cyclomatic_complexity": 12,
        "cognitive_complexity": 18,
        "length": 47,
        "nesting_depth": 4,
        "coverage": 0.0
      },
      "dependencies": {
        "upstream_count": 2,
        "downstream_count": 4,
        "blast_radius": 6,
        "critical_path": true
      },
      "context": {
        "primary": {
          "file": "src/parser.rs",
          "start_line": 38,
          "end_line": 85,
          "symbol": "parse_complex_input"
        },
        "related": [],
        "total_lines": 48,
        "completeness_confidence": 0.86
      }
    }
  ]
}
```

### Terminal

For human exploration (not recommended for AI piping):

```bash
debtmap analyze . --format terminal
```

### DOT

For dependency graph visualization:

```bash
debtmap analyze . --format dot -o deps.dot
dot -Tsvg deps.dot -o deps.svg
```

## Context Suggestions

Each debt item includes a `context` field that tells the AI exactly what code to read:

```
CONTEXT:
├─ Primary: src/parser.rs:38-85 (the debt item)
├─ Caller: src/handler.rs:100-120 (understands usage)
├─ Caller: src/api.rs:45-60 (understands usage)
├─ Callee: src/tokenizer.rs:15-40 (understands dependencies)
└─ Test: tests/parser_test.rs:50-75 (understands expected behavior)
```

### Context Types

| Type | What It Contains | Why It Matters |
|------|------------------|----------------|
| Primary | The function with debt | Core code to understand/fix |
| Caller | Functions that call this | Usage patterns, constraints |
| Callee | Functions this calls | Dependencies, side effects |
| Test | Related test files | Expected behavior, test patterns |
| Type | Struct/enum definitions | Data structures being used |

### Using Context Effectively

**Minimal context (fastest):**
```bash
# Just get the primary location for the first item
debtmap analyze . --format json --top 1 | jq '.items[0].location'
```

**Full context (most accurate):**
```bash
# Read all suggested files
debtmap analyze . --context --format markdown --top 1
# Then have the AI read each file in the context section
```

## Example Workflows

### Claude Code Integration

**Direct piping:**
```bash
debtmap analyze . --format markdown --top 3 | claude "Fix the top item. Read the context files first."
```

**Two-step workflow:**
```bash
# Step 1: Get the analysis
debtmap analyze . --format markdown --lcov coverage.lcov --top 5 > debt.md

# Step 2: Send to Claude with context
cat debt.md | claude "
Read the context files for item #1 before making changes.
Then fix the debt item following these rules:
1. Add tests first
2. Refactor only after tests pass
3. Keep functions under 20 lines
"
```

**Focused fix with coverage:**
```bash
# Generate fresh coverage
cargo llvm-cov --lcov --output-path coverage.lcov

# Analyze with coverage
debtmap analyze . --format markdown --lcov coverage.lcov --top 1

# Send the top item to Claude
debtmap analyze . --format markdown --lcov coverage.lcov --top 1 | \
  claude "Add tests for this function to reach 80% coverage"
```

### Cursor Integration

Cursor works best with file-based context:

```bash
# Generate debt report
debtmap analyze . --format markdown --top 10 > .cursor/debt-report.md

# In Cursor, reference the report:
# @debt-report.md Fix the top critical item
```

### Custom Agent Workflow

For building your own AI pipeline:

```python
import json
import subprocess

# Run debtmap analysis
result = subprocess.run(
    ["debtmap", "analyze", ".", "--context", "--format", "json", "--top", "10"],
    capture_output=True,
    text=True
)
debt_data = json.loads(result.stdout)

# Process each item
for item in debt_data["items"]:
    if item["type"] != "Function":
        continue

    # Extract context files
    context_files = []
    if "context" in item:
        primary = item["context"]["primary"]
        context_files.append(primary)
        context_files.extend([related["range"] for related in item["context"].get("related", [])])
    else:
        location = item["location"]
        context_files.append({
            "file": location["file"],
            "start_line": location.get("line", 1),
            "end_line": location.get("line", 1),
            "symbol": location.get("function")
        })

    # Read context files
    context_content = ""
    for file_range in context_files:
        context_content += read_file_lines(
            file_range["file"],
            file_range["start_line"],
            file_range["end_line"]
        )

    # Build prompt
    prompt = f"""
    Fix this technical debt item:

    Location: {item["location"]["file"]}:{item["location"].get("line")}
    Function: {item["location"].get("function")}
    Score: {item["score"]} ({item["priority"]})

    Signals:
    - Cyclomatic complexity: {item["metrics"]["cyclomatic_complexity"]}
    - Cognitive complexity: {item["metrics"]["cognitive_complexity"]}
    - Test coverage: {item["metrics"].get("coverage", "unknown")}

    Context code:
    {context_content}

    Instructions:
    1. Add tests first
    2. Refactor to reduce complexity
    3. Keep the same public API
    """

    # Send to your LLM
    response = call_llm(prompt)
```

### CI/CD Integration

**GitHub Actions example:**
```yaml
name: Debt Analysis

on: [pull_request]

jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install debtmap
        run: cargo install debtmap

      - name: Generate coverage
        run: cargo llvm-cov --lcov --output-path coverage.lcov

      - name: Analyze debt
        run: |
          debtmap analyze . --format json --lcov coverage.lcov --output debt.json

      - name: Check for critical items
        run: |
          CRITICAL=$(jq '.summary.score_distribution.critical' debt.json)
          if [ "$CRITICAL" -gt 0 ]; then
            echo "::warning::Found $CRITICAL critical debt items"
          fi

      - name: Upload report
        uses: actions/upload-artifact@v3
        with:
          name: debt-report
          path: debt.json
```

## Interpreting Signals

### Priority Score

The priority score combines multiple signals:

| Score | Tier | Interpretation |
|-------|------|----------------|
| >= 100 | Critical | High-risk debt, often structural or highly coupled |
| >= 50 | High | Meaningful risk, usually worth addressing soon |
| >= 20 | Medium | Moderate priority, useful for planned cleanup |
| < 20 | Low | Lower-risk maintenance item |

### Complexity Signals

**Cyclomatic complexity:**
- 1-5: Simple, easy to test
- 6-10: Moderate, manageable
- 11-20: Complex, consider splitting
- 21+: Very complex, high priority

**Cognitive complexity:**
- Measures how hard code is to understand
- Penalizes nesting more than cyclomatic
- Higher values = harder to reason about

**Nesting depth:**
- 1-2: Normal
- 3: Getting complex
- 4+: Strongly consider refactoring

### Coverage Signals

**Coverage:**
- `0.0`: Critical gap, no matching coverage
- `0.01-0.50`: Poor coverage
- `0.51-0.80`: Moderate coverage
- `0.81+`: Good coverage

Coverage is read from LCOV. When coverage is present, well-tested code is dampened so untested complex code rises in priority.

### Coupling Signals

**Fan-in (callers):**
- High fan-in = many dependents
- Changes affect many places
- Higher priority for stability

**Fan-out (callees):**
- High fan-out = many dependencies
- Complex testing requirements
- Consider dependency injection

## Best Practices

### For Effective AI Prompts

1. **Always include context files**
   - AI makes better decisions with more context
   - Context suggestions are curated for relevance

2. **Specify your constraints**
   - "Keep the same public API"
   - "Add tests before refactoring"
   - "Functions must be under 20 lines"

3. **One item at a time**
   - Focus on top priority item
   - Complete fix before moving on
   - Re-run analysis after changes

4. **Verify with coverage**
   - Regenerate coverage after changes
   - Run debtmap again to confirm improvement
   - Track score reduction

### For CI/CD Integration

1. **Set appropriate thresholds**
   - Don't fail on existing debt
   - Fail on new critical items
   - Track trends over time

2. **Cache analysis results**
   - Use git-based caching
   - Only re-analyze changed files

3. **Integrate with PR comments**
   - Show debt impact of changes
   - Suggest focus areas for review

## Troubleshooting

### Empty context suggestions

**Cause:** Call graph analysis couldn't resolve callers/callees

**Solution:** Ensure file parsing succeeded:
```bash
debtmap analyze . -vv  # Verbose mode shows parsing issues
```

### Inconsistent scores between runs

**Cause:** Non-deterministic analysis (should not happen)

**Solution:** Report as a bug with reproducible example

### Large context suggestions

**Cause:** High coupling in codebase

**Solution:** Limit the number of reported items:
```bash
debtmap analyze . --context --format markdown --top 1
```

## API Reference

### CLI Options for LLM Integration

| Option | Description |
|--------|-------------|
| `--format markdown` | LLM-optimized markdown output |
| `--format json` | Structured JSON output |
| `--top N` | Limit to top N items |
| `--lcov FILE` | Include coverage data |
| `--context` | Enable context-aware risk analysis |
| `--context-providers LIST` | Select `critical_path`, `dependency`, and/or `git_history` |
| `--min-score N` | Filter items below score N |
| `--output FILE` | Write to file instead of stdout |

### JSON Schema

The JSON output follows a stable schema. See [Output Formats](output-formats.md) for the complete schema definition.

## Next Steps

- **Configure analysis:** See [Configuration](configuration.md)
- **Understand metrics:** See [Metrics Reference](metrics-reference.md)
- **View examples:** See [Examples](examples.md)
