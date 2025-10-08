# Migration Guide: Unified Output Format

## Overview

Debtmap's unified output format consolidates file-level and function-level technical debt into a single, consistent structure. This guide helps you migrate from the legacy separate outputs to the new unified format.

## What Changed

### Before (Legacy Format)

Previously, debtmap produced separate outputs for file-level and function-level debt:

**File-level output** (`file_debt.json`):
```json
{
  "files": [
    {
      "path": "src/large_module.rs",
      "lines": 850,
      "functions": 25,
      "issues": ["large_file", "god_object"]
    }
  ]
}
```

**Function-level output** (`function_debt.json`):
```json
{
  "functions": [
    {
      "file": "src/large_module.rs",
      "name": "complex_function",
      "cyclomatic": 15,
      "cognitive": 28
    }
  ]
}
```

### After (Unified Format)

The new unified format combines both into a single structure:

```json
{
  "metadata": {
    "version": "0.1.0",
    "timestamp": "2025-10-08T10:30:00Z",
    "analysis_config": {
      "root_path": "/path/to/project",
      "thresholds": {
        "complexity": 10,
        "cognitive_complexity": 15
      }
    }
  },
  "debt_items": [
    {
      "scope": "file",
      "location": {
        "file_path": "src/large_module.rs",
        "line_start": 1,
        "line_end": 850
      },
      "category": "large_file",
      "severity": "high",
      "title": "Large file with 850 lines",
      "metrics": {
        "lines_of_code": 850,
        "function_count": 25
      }
    },
    {
      "scope": "function",
      "location": {
        "file_path": "src/large_module.rs",
        "function_name": "complex_function",
        "line_start": 234,
        "line_end": 298
      },
      "category": "complexity",
      "severity": "high",
      "title": "High complexity in complex_function",
      "metrics": {
        "cyclomatic_complexity": 15,
        "cognitive_complexity": 28,
        "lines_of_code": 64
      }
    }
  ],
  "summary": {
    "total_items": 2,
    "by_category": {
      "large_file": 1,
      "complexity": 1
    },
    "by_severity": {
      "high": 2,
      "medium": 0,
      "low": 0
    }
  }
}
```

## Breaking Changes

### 1. Output Structure

**Breaking:** The top-level structure has changed from separate arrays (`files`, `functions`) to a unified `debt_items` array.

**Migration:**
- Replace JSONPath `$.files[*]` → `$.debt_items[?(@.scope == "file")]`
- Replace JSONPath `$.functions[*]` → `$.debt_items[?(@.scope == "function")]`

### 2. CLI Flag

**Breaking:** The `--output-format` flag now defaults to `unified`.

**Migration:**
- Explicit format selection: `debtmap analyze --output-format unified`
- Legacy format (if needed): `debtmap analyze --output-format legacy`

### 3. Field Names

**Breaking:** Several field names have been standardized:

| Legacy Field | Unified Field | Notes |
|-------------|---------------|-------|
| `path` | `location.file_path` | Nested in location object |
| `name` | `location.function_name` | Nested in location object |
| `cyclomatic` | `metrics.cyclomatic_complexity` | Nested in metrics object |
| `cognitive` | `metrics.cognitive_complexity` | Nested in metrics object |
| `lines` | `metrics.lines_of_code` | Nested in metrics object |
| `issues` | `category` | Single value, not array |

### 4. Metadata Addition

**New:** All output now includes metadata section with:
- Analysis version
- Timestamp
- Configuration used

**Migration:** Update parsers to handle/skip metadata section if not needed.

## Migration Strategies

### Strategy 1: Update Parsing Code

**For Python:**
```python
# Legacy code
with open("file_debt.json") as f:
    data = json.load(f)
    for file in data["files"]:
        print(file["path"], file["lines"])

# Migrated code
with open("unified_debt.json") as f:
    data = json.load(f)
    for item in data["debt_items"]:
        if item["scope"] == "file":
            print(item["location"]["file_path"],
                  item["metrics"]["lines_of_code"])
```

**For JavaScript:**
```javascript
// Legacy code
const data = JSON.parse(fs.readFileSync("file_debt.json"));
data.files.forEach(file => {
  console.log(file.path, file.lines);
});

// Migrated code
const data = JSON.parse(fs.readFileSync("unified_debt.json"));
data.debt_items
  .filter(item => item.scope === "file")
  .forEach(item => {
    console.log(item.location.file_path,
                item.metrics.lines_of_code);
  });
```

### Strategy 2: JSONPath Queries

Use JSONPath to extract specific data:

**File-level debt:**
```jsonpath
$.debt_items[?(@.scope == "file")]
```

**High severity items:**
```jsonpath
$.debt_items[?(@.severity == "high")]
```

**Functions with high complexity:**
```jsonpath
$.debt_items[?(@.scope == "function" && @.metrics.cyclomatic_complexity > 10)]
```

**All items in a specific file:**
```jsonpath
$.debt_items[?(@.location.file_path == "src/large_module.rs")]
```

### Strategy 3: Use jq for Command-Line Processing

**Extract file-level debt:**
```bash
jq '.debt_items[] | select(.scope == "file")' unified_debt.json
```

**Get files with high complexity:**
```bash
jq '.debt_items[] |
    select(.scope == "file" and .severity == "high") |
    .location.file_path' unified_debt.json
```

**Summary by category:**
```bash
jq '.summary.by_category' unified_debt.json
```

## Conversion Examples

### Example 1: CI/CD Integration

**Legacy script:**
```bash
#!/bin/bash
debtmap analyze --output file_debt.json
HIGH_DEBT=$(jq '.files | map(select(.lines > 500)) | length' file_debt.json)
if [ "$HIGH_DEBT" -gt 5 ]; then
  echo "Too many large files"
  exit 1
fi
```

**Migrated script:**
```bash
#!/bin/bash
debtmap analyze --output-format unified --output unified_debt.json
HIGH_DEBT=$(jq '[.debt_items[] |
                 select(.scope == "file" and
                        .metrics.lines_of_code > 500)] |
                length' unified_debt.json)
if [ "$HIGH_DEBT" -gt 5 ]; then
  echo "Too many large files"
  exit 1
fi
```

### Example 2: Dashboard Integration

**Legacy Python:**
```python
def load_debt_data():
    with open("file_debt.json") as f:
        file_data = json.load(f)
    with open("function_debt.json") as f:
        function_data = json.load(f)

    return {
        "files": file_data["files"],
        "functions": function_data["functions"]
    }
```

**Migrated Python:**
```python
def load_debt_data():
    with open("unified_debt.json") as f:
        data = json.load(f)

    return {
        "files": [item for item in data["debt_items"]
                  if item["scope"] == "file"],
        "functions": [item for item in data["debt_items"]
                      if item["scope"] == "function"]
    }
```

### Example 3: GitHub Actions Workflow

**Legacy workflow:**
```yaml
- name: Analyze debt
  run: debtmap analyze

- name: Check results
  run: |
    FILES=$(jq '.files | length' file_debt.json)
    FUNCS=$(jq '.functions | length' function_debt.json)
    echo "Found $FILES file issues and $FUNCS function issues"
```

**Migrated workflow:**
```yaml
- name: Analyze debt
  run: debtmap analyze --output-format unified

- name: Check results
  run: |
    FILES=$(jq '[.debt_items[] | select(.scope == "file")] | length' unified_debt.json)
    FUNCS=$(jq '[.debt_items[] | select(.scope == "function")] | length' unified_debt.json)
    echo "Found $FILES file issues and $FUNCS function issues"
```

## Backward Compatibility

### Temporary Legacy Support

If you need to maintain the legacy format temporarily:

```bash
# Use legacy format
debtmap analyze --output-format legacy

# Or set environment variable
export DEBTMAP_OUTPUT_FORMAT=legacy
debtmap analyze
```

**Note:** Legacy format support will be removed in version 1.0.0.

### Gradual Migration Path

1. **Phase 1: Dual Output** (Current)
   - Both formats available
   - Default is unified
   - Legacy available via flag

2. **Phase 2: Unified Only** (Version 0.5.0)
   - Unified format only
   - Legacy format deprecated
   - Warning on legacy flag usage

3. **Phase 3: Legacy Removed** (Version 1.0.0)
   - Legacy format completely removed
   - Only unified format supported

## Schema Validation

Validate your output against the JSON Schema:

```bash
# Using ajv-cli
npm install -g ajv-cli
ajv validate -s docs/schema/unified-format.json -d unified_debt.json

# Using check-jsonschema
pip install check-jsonschema
check-jsonschema --schemafile docs/schema/unified-format.json unified_debt.json
```

## Common Migration Issues

### Issue 1: Missing Fields

**Problem:** Code expects `path` field directly.

**Solution:** Update to use `location.file_path`.

```python
# Before
file_path = item["path"]

# After
file_path = item["location"]["file_path"]
```

### Issue 2: Array vs Single Category

**Problem:** Legacy `issues` was an array, `category` is a string.

**Solution:** Remove array handling.

```python
# Before
for issue in item["issues"]:
    process_issue(issue)

# After
process_issue(item["category"])
```

### Issue 3: Merging File and Function Data

**Problem:** Need to correlate file and function debt.

**Solution:** Use `location.file_path` to group.

```python
from collections import defaultdict

debt_by_file = defaultdict(list)
for item in data["debt_items"]:
    file_path = item["location"]["file_path"]
    debt_by_file[file_path].append(item)
```

## Testing Your Migration

1. **Run Both Formats:**
   ```bash
   debtmap analyze --output-format legacy --output legacy_debt.json
   debtmap analyze --output-format unified --output unified_debt.json
   ```

2. **Compare Counts:**
   ```bash
   # Should match
   jq '.files | length' legacy_debt.json
   jq '[.debt_items[] | select(.scope == "file")] | length' unified_debt.json
   ```

3. **Validate Schema:**
   ```bash
   check-jsonschema --schemafile docs/schema/unified-format.json unified_debt.json
   ```

4. **Test Queries:**
   ```bash
   # Verify you can extract needed data
   jq '.debt_items[0]' unified_debt.json
   ```

## Support

For migration assistance:
- **Documentation:** [Output Format Guide](output-format.md)
- **JSON Schema:** `docs/schema/unified-format.json`
- **Examples:** `examples/unified-format/`
- **Issues:** GitHub issues for specific problems

## Timeline

- **v0.3.0** (Current): Unified format introduced, legacy available
- **v0.5.0** (Q2 2026): Legacy format deprecated
- **v1.0.0** (Q4 2026): Legacy format removed

Migrate as soon as possible to avoid disruption.
