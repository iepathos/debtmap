# Spec 187: Type Organization Presentation Format

**Status**: Draft
**Priority**: High
**Dependencies**: [186 - Codebase Type Organization]
**Created**: 2025-01-19

## Integration with Debtmap Output

Type organization issues are added as **new issue types** alongside existing issues (god objects, complexity, etc.) in debtmap's standard output formats.

### No New Commands Required

```bash
# Standard analysis (includes type organization)
debtmap analyze

# Existing output formats work automatically
debtmap analyze --format json
debtmap analyze --format markdown
```

**Rationale**: Type organization is just another category of technical debt. Integrate seamlessly with existing output.

---

## Output Formats

### 1. Terminal Output (Default)

Type organization issues appear in standard debtmap output alongside other issues:

```
debtmap v0.3.5 - Technical Debt Analysis
Analyzing: ./src (127 files)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
ISSUES DETECTED
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#1 SCATTERED TYPE: FileMetrics [CRITICAL]
└─ Type Definition: src/analysis/file_metrics.rs:45

   23 methods scattered across 6 files:

   📄 src/utils.rs (8 methods)
      Line 120: calculate_complexity
      Line 145: calculate_coverage
      Line 178: calculate_debt_score
      Line 203: format_metrics
      Line 234: validate_metrics
      Line 267: merge_metrics
      Line 298: normalize_metrics
      Line 321: aggregate_metrics

   📄 src/helpers.rs (6 methods)
      Line 56:  get_metric_value
      Line 78:  set_metric_value
      Line 102: update_metric
      Line 134: is_high_complexity
      Line 156: is_low_coverage
      Line 189: has_debt

   📄 src/processing.rs (4 methods)
      Line 45:  process_metrics
      Line 89:  batch_process
      Line 134: filter_metrics
      Line 178: sort_metrics

   📄 src/formatting/metrics.rs (3 methods)
      Line 23:  format_detailed
      Line 67:  format_summary
      Line 102: format_json

   📄 src/validation/metrics.rs (2 methods)
      Line 34:  validate_ranges
      Line 67:  validate_consistency

   RECOMMENDED FIX:
   Consolidate all methods into impl block at src/analysis/file_metrics.rs

   impl FileMetrics {
       pub fn complexity(&self) -> u32 { /* from calculate_complexity */ }
       pub fn coverage(&self) -> f64 { /* from calculate_coverage */ }
       pub fn validate(&self) -> Result<()> { /* from validate_metrics */ }
       pub fn is_high_complexity(&self) -> bool { self.complexity() > THRESHOLD }
   }

   Estimated Effort: 3 hours | Moderate complexity | Medium risk

#2 SCATTERED TYPE: DebtItem [CRITICAL]
└─ Type Definition: src/debt/debt_item.rs:67

   18 methods scattered across 5 files:
   [Similar format...]

#3 ORPHANED FUNCTIONS: PriorityItem [HIGH]
└─ Target Type: src/priority/priority_item.rs

   12 standalone functions should be methods:

   📄 src/utils.rs (5 functions)
      Line 456: format_priority(item: &PriorityItem) -> String
      Line 489: validate_priority(item: &PriorityItem) -> Result<()>
      Line 523: calculate_priority_score(item: &PriorityItem) -> f64
      Line 567: normalize_priority(item: &PriorityItem) -> PriorityItem
      Line 601: is_high_priority(item: &PriorityItem) -> bool

   📄 src/helpers.rs (3 functions)
      Line 234: get_priority_location(item: &PriorityItem) -> &Path
      Line 267: get_priority_metrics(item: &PriorityItem) -> &Metrics
      Line 301: update_priority(item: &mut PriorityItem, score: f64)

   📄 src/processing.rs (4 functions)
      Line 345: process_priority(item: PriorityItem) -> ProcessedItem
      Line 389: batch_priorities(items: Vec<PriorityItem>)
      Line 423: filter_priorities(items: Vec<PriorityItem>, threshold: f64)
      Line 467: sort_priorities(items: &mut [PriorityItem])

   RECOMMENDED FIX:
   Convert to impl methods in src/priority/priority_item.rs

   impl PriorityItem {
       pub fn format(&self) -> String { /* from format_priority */ }
       pub fn validate(&self) -> Result<()> { /* from validate_priority */ }
       pub fn score(&self) -> f64 { /* from calculate_priority_score */ }
       pub fn is_high_priority(&self) -> bool { self.score() > THRESHOLD }
   }

   Estimated Effort: 3 hours | Simple complexity | Low risk

#4 UTILITIES SPRAWL: utils.rs [HIGH]
└─ File: src/utils.rs

   50 functions operating on 10 distinct types

   Type Distribution:
     FileMetrics: 8 functions
     PriorityItem: 5 functions
     DebtItem: 6 functions
     GodObjectAnalysis: 4 functions
     ComplexityMetrics: 3 functions
     [5 more types...]

   RECOMMENDED FIX:
   Break up utils.rs by moving functions to type modules:
     - FileMetrics functions → src/analysis/file_metrics.rs
     - PriorityItem functions → src/priority/priority_item.rs
     - DebtItem functions → src/debt/debt_item.rs

   Estimated Effort: 10 hours | Moderate complexity | Medium risk

#5 GOD OBJECT: formatter.rs [CRITICAL]
└─ ./src/priority/formatter.rs (3000 lines, 103 functions)

   [Existing god object output format...]

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
SUMMARY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Total Issues: 8
  Scattered Types: 2 (CRITICAL)
  Orphaned Functions: 1 (HIGH)
  Utilities Sprawl: 1 (HIGH)
  God Objects: 1 (CRITICAL)
  [Other existing issue types...]

Estimated Total Effort: 31 hours
```

---

### 2. Markdown Output

```bash
debtmap analyze --format markdown
```

Output:

```markdown
# debtmap Analysis Report

**Version**: 0.3.5
**Generated**: 2025-01-19 10:30:00
**Codebase**: ./src (127 files)

---

## Issues Detected

### #1 SCATTERED TYPE: FileMetrics [CRITICAL]

**Type Definition**: `src/analysis/file_metrics.rs:45`

**Problem**: 23 methods scattered across 6 files

#### Method Locations

**src/utils.rs** (8 methods)
- Line 120: `calculate_complexity`
- Line 145: `calculate_coverage`
- Line 178: `calculate_debt_score`
- Line 203: `format_metrics`
- Line 234: `validate_metrics`
- Line 267: `merge_metrics`
- Line 298: `normalize_metrics`
- Line 321: `aggregate_metrics`

**src/helpers.rs** (6 methods)
- Line 56: `get_metric_value`
- Line 78: `set_metric_value`
- Line 102: `update_metric`
- Line 134: `is_high_complexity`
- Line 156: `is_low_coverage`
- Line 189: `has_debt`

**src/processing.rs** (4 methods)
- Line 45: `process_metrics`
- Line 89: `batch_process`
- Line 134: `filter_metrics`
- Line 178: `sort_metrics`

**src/formatting/metrics.rs** (3 methods)
- Line 23: `format_detailed`
- Line 67: `format_summary`
- Line 102: `format_json`

**src/validation/metrics.rs** (2 methods)
- Line 34: `validate_ranges`
- Line 67: `validate_consistency`

#### Why This Matters

- Violates Single Responsibility: FileMetrics behavior is spread across 6 files
- Poor Cohesion: Related methods are far apart
- Hard to Maintain: Changes to FileMetrics require editing 6 files
- Difficult to Test: Can't unit test FileMetrics behavior in isolation
- Non-idiomatic Rust: Data and behavior should live together

#### Recommended Fix

Consolidate all methods into impl block at `src/analysis/file_metrics.rs`:

```rust
impl FileMetrics {
    // Core calculations
    pub fn complexity(&self) -> u32 {
        // Logic from calculate_complexity
    }

    pub fn coverage(&self) -> f64 {
        // Logic from calculate_coverage
    }

    pub fn debt_score(&self) -> f64 {
        // Logic from calculate_debt_score
    }

    // Validation
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Logic from validate_metrics
    }

    pub fn is_high_complexity(&self) -> bool {
        self.complexity() > THRESHOLD
    }

    // Formatting
    pub fn format_detailed(&self) -> String {
        // Logic from format_detailed
    }
}

// Consider separate trait for formatting
impl Display for FileMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_summary())
    }
}
```

Update call sites to use methods:

```rust
// Before:
let complexity = calculate_complexity(&metrics);
let is_high = is_high_complexity(&metrics);

// After:
let complexity = metrics.complexity();
let is_high = metrics.is_high_complexity();
```

**Estimated Effort**: 3 hours
**Complexity**: Moderate
**Risk**: Medium (many call sites to update)

---

### #2 SCATTERED TYPE: DebtItem [CRITICAL]

[Similar format...]

---

### #3 ORPHANED FUNCTIONS: PriorityItem [HIGH]

**Target Type**: `src/priority/priority_item.rs`

**Problem**: 12 standalone functions should be methods

#### Function Locations

**src/utils.rs** (5 functions)
- Line 456: `format_priority(item: &PriorityItem) -> String`
- Line 489: `validate_priority(item: &PriorityItem) -> Result<()>`
- Line 523: `calculate_priority_score(item: &PriorityItem) -> f64`
- Line 567: `normalize_priority(item: &PriorityItem) -> PriorityItem`
- Line 601: `is_high_priority(item: &PriorityItem) -> bool`

**src/helpers.rs** (3 functions)
- Line 234: `get_priority_location(item: &PriorityItem) -> &Path`
- Line 267: `get_priority_metrics(item: &PriorityItem) -> &Metrics`
- Line 301: `update_priority(item: &mut PriorityItem, score: f64)`

**src/processing.rs** (4 functions)
- Line 345: `process_priority(item: PriorityItem) -> ProcessedItem`
- Line 389: `batch_priorities(items: Vec<PriorityItem>)`
- Line 423: `filter_priorities(items: Vec<PriorityItem>, threshold: f64)`
- Line 467: `sort_priorities(items: &mut [PriorityItem])`

#### Why This Matters

- Non-idiomatic Rust: Functions take `&PriorityItem` instead of using `self`
- Missing Encapsulation: Behavior separated from data
- Namespace Pollution: Functions clutter module namespace
- Harder to Discover: IDE can't autocomplete methods

#### Recommended Fix

Convert to impl methods in `src/priority/priority_item.rs`:

```rust
impl PriorityItem {
    // Formatting
    pub fn format(&self) -> String {
        // Logic from format_priority
    }

    // Validation
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Logic from validate_priority
    }

    // Calculation
    pub fn score(&self) -> f64 {
        // Logic from calculate_priority_score
    }

    // Queries
    pub fn is_high_priority(&self) -> bool {
        self.score() > HIGH_THRESHOLD
    }

    pub fn location(&self) -> &Path {
        // Logic from get_priority_location
    }

    pub fn metrics(&self) -> &Metrics {
        // Logic from get_priority_metrics
    }

    // Mutation
    pub fn update_score(&mut self, score: f64) {
        // Logic from update_priority
    }

    pub fn normalize(self) -> Self {
        // Logic from normalize_priority
    }
}
```

**Estimated Effort**: 3 hours
**Complexity**: Simple (mechanical refactoring)
**Risk**: Low (type safety ensures correctness)

---

### #4 UTILITIES SPRAWL: utils.rs [HIGH]

[Similar format...]

---

### #5 GOD OBJECT: formatter.rs [CRITICAL]

[Existing god object format...]

---

## Summary

**Total Issues**: 8

- Scattered Types: 2 (CRITICAL)
- Orphaned Functions: 1 (HIGH)
- Utilities Sprawl: 1 (HIGH)
- God Objects: 1 (CRITICAL)
- [Other existing issue types...]

**Estimated Total Effort**: 31 hours
```

---

### 3. JSON Output

```bash
debtmap analyze --format json
```

Output:

```json
{
  "version": "0.3.5",
  "timestamp": "2025-01-19T10:30:00Z",
  "codebase": {
    "root": "/Users/glen/debtmap/src",
    "files_analyzed": 127,
    "lines_of_code": 45678
  },
  "issues": [
    {
      "rank": 1,
      "type": "SCATTERED_TYPE",
      "severity": "CRITICAL",
      "type_name": "FileMetrics",
      "definition": {
        "file": "src/analysis/file_metrics.rs",
        "line": 45
      },
      "total_methods": 23,
      "file_count": 6,
      "method_locations": [
        {
          "file": "src/utils.rs",
          "method_count": 8,
          "methods": [
            {"name": "calculate_complexity", "line": 120},
            {"name": "calculate_coverage", "line": 145},
            {"name": "calculate_debt_score", "line": 178},
            {"name": "format_metrics", "line": 203},
            {"name": "validate_metrics", "line": 234},
            {"name": "merge_metrics", "line": 267},
            {"name": "normalize_metrics", "line": 298},
            {"name": "aggregate_metrics", "line": 321}
          ]
        },
        {
          "file": "src/helpers.rs",
          "method_count": 6,
          "methods": [
            {"name": "get_metric_value", "line": 56},
            {"name": "set_metric_value", "line": 78},
            {"name": "update_metric", "line": 102},
            {"name": "is_high_complexity", "line": 134},
            {"name": "is_low_coverage", "line": 156},
            {"name": "has_debt", "line": 189}
          ]
        },
        {
          "file": "src/processing.rs",
          "method_count": 4,
          "methods": [
            {"name": "process_metrics", "line": 45},
            {"name": "batch_process", "line": 89},
            {"name": "filter_metrics", "line": 134},
            {"name": "sort_metrics", "line": 178}
          ]
        },
        {
          "file": "src/formatting/metrics.rs",
          "method_count": 3,
          "methods": [
            {"name": "format_detailed", "line": 23},
            {"name": "format_summary", "line": 67},
            {"name": "format_json", "line": 102}
          ]
        },
        {
          "file": "src/validation/metrics.rs",
          "method_count": 2,
          "methods": [
            {"name": "validate_ranges", "line": 34},
            {"name": "validate_consistency", "line": 67}
          ]
        }
      ],
      "recommendation": {
        "title": "Consolidate FileMetrics methods",
        "description": "Move all 23 methods to src/analysis/file_metrics.rs as impl methods",
        "effort_hours": 3.0,
        "complexity": "MODERATE",
        "risk": "MEDIUM",
        "actions": [
          {
            "type": "MOVE_METHODS",
            "from": "src/utils.rs",
            "to": "src/analysis/file_metrics.rs",
            "methods": [
              "calculate_complexity",
              "calculate_coverage",
              "calculate_debt_score",
              "format_metrics",
              "validate_metrics",
              "merge_metrics",
              "normalize_metrics",
              "aggregate_metrics"
            ]
          },
          {
            "type": "MOVE_METHODS",
            "from": "src/helpers.rs",
            "to": "src/analysis/file_metrics.rs",
            "methods": [
              "get_metric_value",
              "set_metric_value",
              "update_metric",
              "is_high_complexity",
              "is_low_coverage",
              "has_debt"
            ]
          },
          {
            "type": "MOVE_METHODS",
            "from": "src/processing.rs",
            "to": "src/analysis/file_metrics.rs",
            "methods": [
              "process_metrics",
              "batch_process",
              "filter_metrics",
              "sort_metrics"
            ]
          },
          {
            "type": "MOVE_METHODS",
            "from": "src/formatting/metrics.rs",
            "to": "src/analysis/file_metrics.rs",
            "methods": [
              "format_detailed",
              "format_summary",
              "format_json"
            ]
          },
          {
            "type": "MOVE_METHODS",
            "from": "src/validation/metrics.rs",
            "to": "src/analysis/file_metrics.rs",
            "methods": [
              "validate_ranges",
              "validate_consistency"
            ]
          },
          {
            "type": "UPDATE_CALL_SITES",
            "pattern": "calculate_complexity(&metrics)",
            "replacement": "metrics.complexity()",
            "estimated_occurrences": 45
          }
        ]
      }
    },
    {
      "rank": 2,
      "type": "SCATTERED_TYPE",
      "severity": "CRITICAL",
      "type_name": "DebtItem",
      "definition": {
        "file": "src/debt/debt_item.rs",
        "line": 67
      },
      "total_methods": 18,
      "file_count": 5,
      "method_locations": [
        "..."
      ],
      "recommendation": {
        "..."
      }
    },
    {
      "rank": 3,
      "type": "ORPHANED_FUNCTIONS",
      "severity": "HIGH",
      "target_type": "PriorityItem",
      "target_file": "src/priority/priority_item.rs",
      "total_functions": 12,
      "function_locations": [
        {
          "file": "src/utils.rs",
          "function_count": 5,
          "functions": [
            {
              "name": "format_priority",
              "line": 456,
              "signature": "format_priority(item: &PriorityItem) -> String"
            },
            {
              "name": "validate_priority",
              "line": 489,
              "signature": "validate_priority(item: &PriorityItem) -> Result<()>"
            },
            {
              "name": "calculate_priority_score",
              "line": 523,
              "signature": "calculate_priority_score(item: &PriorityItem) -> f64"
            },
            {
              "name": "normalize_priority",
              "line": 567,
              "signature": "normalize_priority(item: &PriorityItem) -> PriorityItem"
            },
            {
              "name": "is_high_priority",
              "line": 601,
              "signature": "is_high_priority(item: &PriorityItem) -> bool"
            }
          ]
        },
        {
          "file": "src/helpers.rs",
          "function_count": 3,
          "functions": [
            {
              "name": "get_priority_location",
              "line": 234,
              "signature": "get_priority_location(item: &PriorityItem) -> &Path"
            },
            {
              "name": "get_priority_metrics",
              "line": 267,
              "signature": "get_priority_metrics(item: &PriorityItem) -> &Metrics"
            },
            {
              "name": "update_priority",
              "line": 301,
              "signature": "update_priority(item: &mut PriorityItem, score: f64)"
            }
          ]
        },
        {
          "file": "src/processing.rs",
          "function_count": 4,
          "functions": [
            {
              "name": "process_priority",
              "line": 345,
              "signature": "process_priority(item: PriorityItem) -> ProcessedItem"
            },
            {
              "name": "batch_priorities",
              "line": 389,
              "signature": "batch_priorities(items: Vec<PriorityItem>)"
            },
            {
              "name": "filter_priorities",
              "line": 423,
              "signature": "filter_priorities(items: Vec<PriorityItem>, threshold: f64)"
            },
            {
              "name": "sort_priorities",
              "line": 467,
              "signature": "sort_priorities(items: &mut [PriorityItem])"
            }
          ]
        }
      ],
      "recommendation": {
        "title": "Convert PriorityItem functions to methods",
        "description": "Convert standalone functions to impl methods in src/priority/priority_item.rs",
        "effort_hours": 3.0,
        "complexity": "SIMPLE",
        "risk": "LOW",
        "actions": [
          {
            "type": "CONVERT_TO_METHOD",
            "function": "format_priority",
            "method_name": "format",
            "signature": "pub fn format(&self) -> String"
          },
          {
            "type": "CONVERT_TO_METHOD",
            "function": "validate_priority",
            "method_name": "validate",
            "signature": "pub fn validate(&self) -> Result<(), ValidationError>"
          },
          {
            "type": "CONVERT_TO_METHOD",
            "function": "calculate_priority_score",
            "method_name": "score",
            "signature": "pub fn score(&self) -> f64"
          }
        ]
      }
    },
    {
      "rank": 4,
      "type": "UTILITIES_SPRAWL",
      "severity": "HIGH",
      "file": "src/utils.rs",
      "total_functions": 50,
      "distinct_types": 10,
      "type_distribution": [
        {"type": "FileMetrics", "function_count": 8, "percentage": 16},
        {"type": "PriorityItem", "function_count": 5, "percentage": 10},
        {"type": "DebtItem", "function_count": 6, "percentage": 12},
        {"type": "GodObjectAnalysis", "function_count": 4, "percentage": 8},
        {"type": "ComplexityMetrics", "function_count": 3, "percentage": 6},
        {"type": "CoverageMetrics", "function_count": 3, "percentage": 6},
        {"type": "Path/PathBuf", "function_count": 7, "percentage": 14},
        {"type": "String", "function_count": 5, "percentage": 10},
        {"type": "Config", "function_count": 4, "percentage": 8},
        {"type": "Other", "function_count": 5, "percentage": 10}
      ],
      "recommendation": {
        "title": "Break up utilities sprawl",
        "description": "Move functions to appropriate type modules or create focused utility modules",
        "effort_hours": 10.0,
        "complexity": "MODERATE",
        "risk": "MEDIUM",
        "actions": [
          {
            "type": "MOVE_FUNCTIONS",
            "from": "src/utils.rs",
            "to": "src/analysis/file_metrics.rs",
            "functions": [
              "calculate_complexity",
              "calculate_coverage",
              "calculate_debt_score",
              "format_metrics",
              "validate_metrics",
              "merge_metrics",
              "normalize_metrics",
              "aggregate_metrics"
            ]
          },
          {
            "type": "CREATE_MODULE",
            "path": "src/utils/path_utils.rs",
            "functions": [
              "normalize_path",
              "relative_path",
              "find_project_root"
            ]
          },
          {
            "type": "DELETE_FILE",
            "path": "src/utils.rs"
          }
        ]
      }
    },
    {
      "rank": 5,
      "type": "GOD_OBJECT",
      "severity": "CRITICAL",
      "file": "src/priority/formatter.rs",
      "lines": 3000,
      "functions": 103,
      "score": 149
    }
  ],
  "summary": {
    "total_issues": 8,
    "by_type": {
      "SCATTERED_TYPE": 2,
      "ORPHANED_FUNCTIONS": 1,
      "UTILITIES_SPRAWL": 1,
      "GOD_OBJECT": 1
    },
    "by_severity": {
      "CRITICAL": 4,
      "HIGH": 2,
      "MEDIUM": 2
    },
    "estimated_total_effort_hours": 31
  }
}
```

---

## Implementation Notes

### Issue Type Constants

```rust
pub enum IssueType {
    ScatteredType,
    OrphanedFunctions,
    UtilitiesSprawl,
    GodObject,
    // ... existing types
}

pub enum IssueSeverity {
    Critical,
    High,
    Medium,
    Low,
}
```

### Output Integration

Type organization issues integrate into existing debtmap output pipeline:

1. **Analysis Phase**: Detect scattered types, orphaned functions, utilities sprawl
2. **Ranking Phase**: Rank alongside god objects and other issues by severity
3. **Formatting Phase**: Format according to output mode (terminal/markdown/json)
4. **Display Phase**: Render in standard debtmap output

### Visual Consistency

All issue types use consistent formatting:

- **Severity badges**: `[CRITICAL]`, `[HIGH]`, `[MEDIUM]`, `[LOW]`
- **File paths**: Relative to project root
- **Line numbers**: Precise source locations
- **Effort estimates**: Hours, complexity, risk level
- **Recommended fixes**: Code examples and migration steps

This ensures type organization issues feel like a natural part of debtmap's output, not a separate tool.
