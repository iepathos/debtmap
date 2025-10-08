# Debtmap Output Format Guide

## Overview

Debtmap uses a unified output format for technical debt reporting that combines file-level and function-level debt into a single, structured JSON format. This guide provides comprehensive documentation of the format, its fields, and usage patterns.

## Format Version

- **Current Version:** 1.0
- **Schema Location:** `docs/schema/unified-format.json`
- **Introduced:** Version 0.3.0
- **Status:** Stable

## Top-Level Structure

The output consists of three main sections:

```json
{
  "metadata": { ... },
  "debt_items": [ ... ],
  "summary": { ... }
}
```

## Metadata Section

Contains analysis context and configuration.

### Schema

```json
{
  "metadata": {
    "version": "string",           // Debtmap version
    "timestamp": "string",          // ISO 8601 timestamp
    "analysis_config": {
      "root_path": "string",        // Project root directory
      "thresholds": {
        "complexity": "integer",    // Cyclomatic complexity threshold
        "cognitive_complexity": "integer",  // Cognitive complexity threshold
        "risk_score": "number"      // Risk score threshold
      }
    }
  }
}
```

### Field Descriptions

| Field | Type | Description | Example |
|-------|------|-------------|---------|
| `version` | string | Debtmap version that generated this output | `"0.3.0"` |
| `timestamp` | string | ISO 8601 timestamp of when analysis was performed | `"2025-10-08T10:30:00Z"` |
| `analysis_config.root_path` | string | Absolute or relative path to project root | `"/home/user/project"` |
| `analysis_config.thresholds` | object | Thresholds used for flagging debt | See below |

### Threshold Configuration

```json
{
  "thresholds": {
    "complexity": 10,              // Cyclomatic complexity warning threshold
    "cognitive_complexity": 15,    // Cognitive complexity warning threshold
    "risk_score": 7.5             // Risk score warning threshold
  }
}
```

## Debt Items Section

Core section containing all identified technical debt items.

### Schema

```json
{
  "debt_items": [
    {
      "scope": "file" | "function",
      "location": { ... },
      "category": "string",
      "severity": "high" | "medium" | "low",
      "title": "string",
      "description": "string",
      "metrics": { ... },
      "suggestions": ["string"],
      "related_items": [integer]
    }
  ]
}
```

### Common Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `scope` | enum | Yes | Either `"file"` or `"function"` |
| `location` | object | Yes | Location information (see below) |
| `category` | enum | Yes | Type of technical debt (see below) |
| `severity` | enum | Yes | `"high"`, `"medium"`, or `"low"` |
| `title` | string | No | Human-readable title |
| `description` | string | No | Detailed explanation |
| `metrics` | object | Yes | Relevant metrics (see below) |
| `suggestions` | array | No | List of refactoring suggestions |
| `related_items` | array | No | Indices of related debt items |

### Location Object

Identifies where the debt exists in the codebase.

```json
{
  "location": {
    "file_path": "src/analysis/complexity.rs",
    "function_name": "calculate_cognitive_complexity",
    "line_start": 234,
    "line_end": 298,
    "column_start": 1,
    "column_end": 2
  }
}
```

| Field | Type | Required | Applies To | Description |
|-------|------|----------|------------|-------------|
| `file_path` | string | Yes | All | Relative path from project root |
| `function_name` | string | No | Functions | Name of the function |
| `line_start` | integer | No | All | Starting line (1-indexed) |
| `line_end` | integer | No | All | Ending line (1-indexed) |
| `column_start` | integer | No | All | Starting column (1-indexed) |
| `column_end` | integer | No | All | Ending column (1-indexed) |

**Notes:**
- `function_name` is only present for function-scoped items
- Line numbers are 1-indexed (first line is 1, not 0)
- Column numbers are optional but recommended for precise location

### Debt Categories

| Category | Description | Typical Scope |
|----------|-------------|---------------|
| `complexity` | High cyclomatic or cognitive complexity | Function |
| `god_object` | File/class with too many responsibilities | File |
| `large_file` | File exceeding size thresholds | File |
| `duplicate_code` | Duplicated code patterns | File/Function |
| `test_coverage` | Insufficient test coverage | File/Function |
| `documentation` | Missing or inadequate documentation | File/Function |
| `error_handling` | Poor or missing error handling | Function |
| `security` | Security concerns or vulnerabilities | File/Function |
| `performance` | Performance bottlenecks | Function |
| `maintainability` | Low maintainability index | File/Function |
| `coupling` | High coupling between modules | File |
| `cohesion` | Low cohesion within module | File |

### Severity Levels

| Level | Description | Typical Thresholds |
|-------|-------------|-------------------|
| `high` | Critical issues requiring immediate attention | Complexity > 20, Risk > 8.0 |
| `medium` | Moderate issues to address soon | Complexity 11-20, Risk 5.0-8.0 |
| `low` | Minor issues for future improvement | Complexity < 11, Risk < 5.0 |

### Metrics Object

Metrics vary based on scope and category.

**File-Level Metrics:**
```json
{
  "metrics": {
    "lines_of_code": 850,
    "function_count": 25,
    "cyclomatic_complexity": 120,
    "cognitive_complexity": 180,
    "risk_score": 8.5,
    "maintainability_index": 42.3
  }
}
```

**Function-Level Metrics:**
```json
{
  "metrics": {
    "cyclomatic_complexity": 15,
    "cognitive_complexity": 28,
    "lines_of_code": 64,
    "nesting_depth": 5,
    "parameter_count": 8,
    "risk_score": 7.2
  }
}
```

#### Metric Definitions

| Metric | Scope | Type | Range | Description |
|--------|-------|------|-------|-------------|
| `cyclomatic_complexity` | Both | integer | 1+ | Number of linearly independent paths |
| `cognitive_complexity` | Both | integer | 0+ | Measure of how hard code is to understand |
| `lines_of_code` | Both | integer | 1+ | Non-comment, non-blank lines |
| `nesting_depth` | Function | integer | 0+ | Maximum nesting level of control structures |
| `parameter_count` | Function | integer | 0+ | Number of function parameters |
| `function_count` | File | integer | 0+ | Number of functions in file |
| `risk_score` | Both | number | 0-10 | Calculated risk assessment |
| `maintainability_index` | Both | number | 0-100 | Maintainability score (higher is better) |

#### Metric Calculations

**Risk Score Formula:**
```
risk_score = (cyclomatic_complexity * 0.3) +
             (cognitive_complexity * 0.4) +
             (nesting_depth * 0.2) +
             (parameter_count * 0.1)
```

**Maintainability Index:**
Based on Halstead metrics and cyclomatic complexity:
```
MI = 171 - 5.2 * ln(Halstead_Volume) -
      0.23 * cyclomatic_complexity -
      16.2 * ln(lines_of_code)
```

## Summary Section

Aggregated statistics across all debt items.

### Schema

```json
{
  "summary": {
    "total_items": 42,
    "by_category": {
      "complexity": 15,
      "large_file": 8,
      "god_object": 3
    },
    "by_severity": {
      "high": 12,
      "medium": 18,
      "low": 12
    }
  }
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `total_items` | integer | Total number of debt items |
| `by_category` | object | Count of items per category |
| `by_severity` | object | Count of items per severity level |

## Complete Example

```json
{
  "metadata": {
    "version": "0.3.0",
    "timestamp": "2025-10-08T10:30:00Z",
    "analysis_config": {
      "root_path": "/home/user/myproject",
      "thresholds": {
        "complexity": 10,
        "cognitive_complexity": 15,
        "risk_score": 7.5
      }
    }
  },
  "debt_items": [
    {
      "scope": "file",
      "location": {
        "file_path": "src/analysis/complexity_analyzer.rs",
        "line_start": 1,
        "line_end": 850
      },
      "category": "large_file",
      "severity": "high",
      "title": "Large file with 850 lines",
      "description": "This file exceeds the recommended size limit and may benefit from being split into smaller, more focused modules.",
      "metrics": {
        "lines_of_code": 850,
        "function_count": 25,
        "cyclomatic_complexity": 120,
        "risk_score": 8.5,
        "maintainability_index": 42.3
      },
      "suggestions": [
        "Split into separate modules by responsibility",
        "Extract utility functions to a shared module",
        "Consider using trait composition for related functionality"
      ],
      "related_items": [1, 2, 5]
    },
    {
      "scope": "function",
      "location": {
        "file_path": "src/analysis/complexity_analyzer.rs",
        "function_name": "calculate_cognitive_complexity",
        "line_start": 234,
        "line_end": 298,
        "column_start": 1,
        "column_end": 2
      },
      "category": "complexity",
      "severity": "high",
      "title": "High complexity in calculate_cognitive_complexity",
      "description": "This function has high cyclomatic and cognitive complexity, making it difficult to understand and maintain.",
      "metrics": {
        "cyclomatic_complexity": 15,
        "cognitive_complexity": 28,
        "lines_of_code": 64,
        "nesting_depth": 5,
        "parameter_count": 3,
        "risk_score": 7.8
      },
      "suggestions": [
        "Extract nested conditionals into separate functions",
        "Use early returns to reduce nesting",
        "Consider the strategy pattern for complex branching logic"
      ],
      "related_items": [0]
    },
    {
      "scope": "function",
      "location": {
        "file_path": "src/parsers/rust_parser.rs",
        "function_name": "parse_function_signature",
        "line_start": 142,
        "line_end": 167
      },
      "category": "error_handling",
      "severity": "medium",
      "title": "Insufficient error handling",
      "description": "Function uses unwrap() instead of proper error propagation.",
      "metrics": {
        "cyclomatic_complexity": 6,
        "cognitive_complexity": 8,
        "lines_of_code": 25,
        "risk_score": 5.5
      },
      "suggestions": [
        "Replace unwrap() calls with ? operator",
        "Add context to error messages using anyhow",
        "Consider returning Result<T> for better error handling"
      ]
    }
  ],
  "summary": {
    "total_items": 3,
    "by_category": {
      "large_file": 1,
      "complexity": 1,
      "error_handling": 1
    },
    "by_severity": {
      "high": 2,
      "medium": 1,
      "low": 0
    }
  }
}
```

## Usage Patterns

### Filtering Debt Items

**Get all high severity items:**
```bash
jq '.debt_items[] | select(.severity == "high")' output.json
```

**Get all function-level debt:**
```bash
jq '.debt_items[] | select(.scope == "function")' output.json
```

**Get items in a specific file:**
```bash
jq '.debt_items[] | select(.location.file_path == "src/main.rs")' output.json
```

**Get complexity issues only:**
```bash
jq '.debt_items[] | select(.category == "complexity")' output.json
```

### Aggregating Data

**Count items by category:**
```bash
jq '.summary.by_category' output.json
```

**List all affected files:**
```bash
jq '[.debt_items[].location.file_path] | unique' output.json
```

**Calculate average complexity:**
```bash
jq '[.debt_items[].metrics.cyclomatic_complexity] | add / length' output.json
```

### Generating Reports

**Top 10 most complex functions:**
```bash
jq '.debt_items[] |
    select(.scope == "function") |
    {file: .location.file_path,
     func: .location.function_name,
     complexity: .metrics.cyclomatic_complexity}' output.json |
jq -s 'sort_by(.complexity) | reverse | .[0:10]'
```

**Files with most debt items:**
```bash
jq 'group_by(.location.file_path) |
    map({file: .[0].location.file_path, count: length}) |
    sort_by(.count) | reverse' output.json
```

## Integration Examples

### CI/CD Pipeline

```yaml
- name: Analyze technical debt
  run: debtmap analyze --output-format unified --output debt.json

- name: Check debt thresholds
  run: |
    HIGH_COUNT=$(jq '[.debt_items[] | select(.severity == "high")] | length' debt.json)
    if [ "$HIGH_COUNT" -gt 10 ]; then
      echo "Too many high-severity debt items: $HIGH_COUNT"
      exit 1
    fi
```

### Python Analysis Script

```python
import json

def analyze_debt(file_path):
    with open(file_path) as f:
        data = json.load(f)

    # Group by file
    by_file = {}
    for item in data["debt_items"]:
        file = item["location"]["file_path"]
        if file not in by_file:
            by_file[file] = []
        by_file[file].append(item)

    # Find worst offenders
    worst_files = sorted(
        by_file.items(),
        key=lambda x: sum(1 for item in x[1] if item["severity"] == "high"),
        reverse=True
    )

    return worst_files[:10]
```

### JavaScript Dashboard

```javascript
async function loadDebtData() {
  const response = await fetch('debt.json');
  const data = await response.json();

  return {
    totalItems: data.summary.total_items,
    bySeverity: data.summary.by_severity,
    byCategory: data.summary.by_category,
    items: data.debt_items
  };
}

function filterHighPriority(items) {
  return items.filter(item =>
    item.severity === 'high' &&
    item.metrics.risk_score > 8.0
  );
}
```

## Schema Validation

Validate output against the JSON Schema:

```bash
# Using ajv-cli
npm install -g ajv-cli
ajv validate -s docs/schema/unified-format.json -d output.json

# Using check-jsonschema
pip install check-jsonschema
check-jsonschema --schemafile docs/schema/unified-format.json output.json

# Using Python jsonschema
python3 -c "
import json
import jsonschema

with open('docs/schema/unified-format.json') as f:
    schema = json.load(f)

with open('output.json') as f:
    data = json.load(f)

jsonschema.validate(data, schema)
print('Valid!')
"
```

## Best Practices

1. **Always validate against schema** before processing
2. **Check metadata.version** to ensure format compatibility
3. **Handle missing optional fields** gracefully
4. **Use JSONPath or jq** for complex queries
5. **Group related items** using `related_items` field
6. **Track metrics over time** using timestamp
7. **Filter by severity** for prioritization
8. **Aggregate at file level** for high-level overview

## FAQ

**Q: What's the difference between cyclomatic and cognitive complexity?**

A: Cyclomatic complexity counts linearly independent paths. Cognitive complexity measures how hard code is to understand, with higher weights for nested structures.

**Q: Can I add custom fields?**

A: The schema allows additional fields at the debt item level, but standard tools may ignore them.

**Q: How are related_items determined?**

A: Items are related if they share a file (function debt relates to file debt) or if fixing one affects the other.

**Q: What if line numbers are missing?**

A: Line numbers are optional but recommended. They may be absent for file-level debt that spans the entire file.

**Q: How often should I run analysis?**

A: Recommended: on every commit (fast categories only), daily (full analysis), and pre-release (comprehensive).

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-10 | Initial unified format release |

## See Also

- [Migration Guide](migration-unified-format.md) - Migrating from legacy format
- [JSON Schema](schema/unified-format.json) - Formal schema definition
- [CLI Reference](cli-reference.md) - Command-line options
