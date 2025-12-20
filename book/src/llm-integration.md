# LLM Integration Guide

How to use debtmap output in AI coding workflows.

## Overview

Debtmap is designed to provide AI coding assistants with the signals they need to understand and fix technical debt. This guide covers:

1. Output formats optimized for LLMs
2. Context suggestions and how to use them
3. Example workflows for different AI tools
4. Interpreting signals for effective prompts

## Output Formats

### LLM Markdown (Recommended)

The `llm-markdown` format is specifically designed for LLM consumption:

```bash
debtmap analyze . --format llm-markdown --top 5
```

**Output structure:**
```markdown
# Technical Debt Analysis

## Summary
- Total items: 47
- Critical: 3
- High: 12
- Moderate: 20
- Low: 12

## Top Priority Items

### #1 [CRITICAL] parse_complex_input
**Location:** src/parser.rs:38-85
**Score:** 8.9/10

**Signals:**
| Metric | Value | Threshold |
|--------|-------|-----------|
| Cyclomatic | 12 | 10 |
| Cognitive | 18 | 15 |
| Coverage | 0% | 80% |
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
  "version": "1.0",
  "timestamp": "2024-01-15T10:30:00Z",
  "summary": {
    "total_items": 47,
    "by_tier": {
      "critical": 3,
      "high": 12,
      "moderate": 20,
      "low": 12
    },
    "total_loc": 15420
  },
  "items": [
    {
      "rank": 1,
      "id": "parse_complex_input_38",
      "tier": "critical",
      "score": 8.9,
      "location": {
        "file": "src/parser.rs",
        "line_start": 38,
        "line_end": 85,
        "function": "parse_complex_input"
      },
      "metrics": {
        "cyclomatic": 12,
        "cognitive": 18,
        "nesting": 4,
        "loc": 47
      },
      "coverage": {
        "line_percent": 0.0,
        "branch_percent": 0.0
      },
      "context": {
        "primary": "src/parser.rs:38-85",
        "callers": [
          {"file": "src/handler.rs", "lines": "100-120", "calls": 12},
          {"file": "src/api.rs", "lines": "45-60", "calls": 8}
        ],
        "tests": [
          {"file": "tests/parser_test.rs", "lines": "50-75"}
        ]
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
# Just get the primary location
debtmap analyze . --format json | jq '.items[0].location'
```

**Full context (most accurate):**
```bash
# Read all suggested files
debtmap analyze . --format llm-markdown --top 1
# Then have the AI read each file in the context section
```

## Example Workflows

### Claude Code Integration

**Direct piping:**
```bash
debtmap analyze . --format llm-markdown --top 3 | claude "Fix the top item. Read the context files first."
```

**Two-step workflow:**
```bash
# Step 1: Get the analysis
debtmap analyze . --format llm-markdown --lcov coverage.lcov --top 5 > debt.md

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
debtmap analyze . --format llm-markdown --lcov coverage.lcov --top 1

# Send the top item to Claude
debtmap analyze . --format llm-markdown --lcov coverage.lcov --top 1 | \
  claude "Add tests for this function to reach 80% coverage"
```

### Cursor Integration

Cursor works best with file-based context:

```bash
# Generate debt report
debtmap analyze . --format llm-markdown --top 10 > .cursor/debt-report.md

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
    ["debtmap", "analyze", ".", "--format", "json", "--top", "10"],
    capture_output=True,
    text=True
)
debt_data = json.loads(result.stdout)

# Process each item
for item in debt_data["items"]:
    # Extract context files
    context_files = []
    context_files.append(item["context"]["primary"])
    context_files.extend([c["file"] for c in item["context"].get("callers", [])])

    # Read context files
    context_content = ""
    for file_spec in context_files:
        file_path, lines = parse_file_spec(file_spec)
        context_content += read_file_lines(file_path, lines)

    # Build prompt
    prompt = f"""
    Fix this technical debt item:

    Location: {item["location"]["file"]}:{item["location"]["line_start"]}
    Function: {item["location"]["function"]}
    Score: {item["score"]}/10

    Signals:
    - Cyclomatic complexity: {item["metrics"]["cyclomatic"]}
    - Test coverage: {item["coverage"]["line_percent"]}%

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
          debtmap analyze . --format json --lcov coverage.lcov > debt.json

      - name: Check for critical items
        run: |
          CRITICAL=$(jq '.summary.by_tier.critical' debt.json)
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

### Severity Score (0-10)

The severity score combines multiple signals:

| Score | Tier | Interpretation |
|-------|------|----------------|
| 8.0-10.0 | Critical | High complexity, no tests, high coupling |
| 5.0-7.9 | High | Moderate risk, coverage gaps |
| 2.0-4.9 | Moderate | Lower risk, monitor |
| 0.0-1.9 | Low | Acceptable state |

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

**Line coverage:**
- 0%: Critical gap, no tests at all
- 1-50%: Poor coverage
- 51-80%: Moderate coverage
- 81%+: Good coverage

**Branch coverage:**
- More important than line coverage
- Missing branches = missing edge cases
- 0% branch = high risk

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

**Solution:** Use `--max-context-lines` to limit:
```bash
debtmap analyze . --format llm-markdown --max-context-lines 300
```

## API Reference

### CLI Options for LLM Integration

| Option | Description |
|--------|-------------|
| `--format llm-markdown` | LLM-optimized markdown output |
| `--format json` | Structured JSON output |
| `--top N` | Limit to top N items |
| `--lcov FILE` | Include coverage data |
| `--min-score N` | Filter items below score N |
| `--output FILE` | Write to file instead of stdout |

### JSON Schema

The JSON output follows a stable schema. See [Output Formats](output-formats.md) for the complete schema definition.

## Next Steps

- **Configure analysis:** See [Configuration](configuration.md)
- **Understand metrics:** See [Metrics Reference](metrics-reference.md)
- **View examples:** See [Examples](examples.md)
