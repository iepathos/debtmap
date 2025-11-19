# Spec 186 Addendum: Presentation Format

## Integration with Debtmap Output

### Option 1: Separate Command (Recommended)

```bash
# Standard god object analysis (existing)
debtmap analyze

# Codebase-wide type organization (new)
debtmap analyze --type-organization
# or shorthand
debtmap analyze --types
```

**Rationale**: Type organization is a different concern than god objects. Separate command keeps outputs focused.

### Option 2: Combined Report

```bash
# Include both in one report
debtmap analyze --include-type-organization
```

---

## Output Format Examples

### 1. Summary View (Default)

```
debtmap v0.3.5 - Technical Debt Analysis
Analyzing: /Users/glen/debtmap/src (127 files)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
TYPE ORGANIZATION ANALYSIS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Overall Health: 62/100 (NEEDS IMPROVEMENT)

Issues Detected:
  🔴 5 scattered types (CRITICAL)
  🟡 12 orphaned function groups (HIGH)
  🟡 3 utilities modules with mixed responsibilities (HIGH)
  🟢 0 cross-file technical groupings (GOOD)

Top Priority Issues:

#1 [CRITICAL] FileMetrics scattered across 6 files (23 methods)
   Impact: High coupling, difficult maintenance
   Effort: 3 hours
   → See details: debtmap analyze --types --item 1

#2 [CRITICAL] DebtItem scattered across 5 files (18 methods)
   Impact: Violates single responsibility, hard to test
   Effort: 2.5 hours
   → See details: debtmap analyze --types --item 2

#3 [HIGH] 12 orphaned PriorityItem functions
   Impact: Missing type ownership, non-idiomatic Rust
   Effort: 3 hours
   → See details: debtmap analyze --types --item 3

#4 [HIGH] utils.rs has 50 functions operating on 10 types
   Impact: Utilities anti-pattern, unclear responsibilities
   Effort: 10 hours
   → See details: debtmap analyze --types --item 4

#5 [HIGH] GodObjectAnalysis scattered across 4 files (15 methods)
   Impact: Poor cohesion, scattered behavior
   Effort: 2 hours
   → See details: debtmap analyze --types --item 5

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
SUMMARY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Total Issues: 20
  Critical: 5
  High: 12
  Medium: 3
  Low: 0

Estimated Refactoring Effort: 28 hours

Priority Recommendations:
  1. Start with scattered types (highest impact)
  2. Convert orphaned functions (quick wins)
  3. Break up utilities modules (long-term health)

Run with --detailed for full analysis
Run with --json for machine-readable output
Run with --fix-plan to generate refactoring plan
```

### 2. Detailed View (--detailed)

```bash
debtmap analyze --types --detailed
```

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#1 SCATTERED TYPE: FileMetrics [CRITICAL]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Type Definition:
  📄 src/analysis/file_metrics.rs:45

Methods Scattered Across 6 Files (23 total):

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

Why This Matters:
  ❌ Violates Single Responsibility: FileMetrics behavior is spread across 6 files
  ❌ Poor Cohesion: Related methods are far apart
  ❌ Hard to Maintain: Changes to FileMetrics require editing 6 files
  ❌ Difficult to Test: Can't unit test FileMetrics behavior in isolation
  ❌ Non-idiomatic Rust: Data and behavior should live together

Recommended Fix:

  1. Move all 23 methods to src/analysis/file_metrics.rs

  2. Organize as impl blocks:

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

  3. Update call sites to use methods:

     Before:
       let complexity = calculate_complexity(&metrics);
       let is_high = is_high_complexity(&metrics);

     After:
       let complexity = metrics.complexity();
       let is_high = metrics.is_high_complexity();

Estimated Effort:
  ⏱  3 hours
  📊 Moderate complexity
  ⚠️  Medium risk (many call sites to update)

Files to Modify: 7 (6 source files + 1 destination)
Call Sites to Update: ~45 (estimated from grep)

Automated Fix Available:
  Run: debtmap fix --type-organization --item 1 --dry-run

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#2 SCATTERED TYPE: DebtItem [CRITICAL]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Type Definition:
  📄 src/debt/debt_item.rs:67

Methods Scattered Across 5 Files (18 total):

  [Similar detailed breakdown...]

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#3 ORPHANED FUNCTIONS: PriorityItem [HIGH]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

12 standalone functions should be methods of PriorityItem:

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
     Line 389: batch_priorities(items: Vec<PriorityItem>) -> Vec<ProcessedItem>
     Line 423: filter_priorities(items: Vec<PriorityItem>, threshold: f64)
     Line 467: sort_priorities(items: &mut [PriorityItem])

Why This Matters:
  ❌ Non-idiomatic Rust: Functions take &PriorityItem instead of using self
  ❌ Missing Encapsulation: Behavior separated from data
  ❌ Namespace Pollution: Functions clutter module namespace
  ❌ Harder to Discover: IDE can't autocomplete methods

Recommended Fix:

  Convert to impl methods in src/priority/priority_item.rs:

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

  // Static/utility methods on separate trait or module
  pub mod priority_utils {
      pub fn batch_process(items: Vec<PriorityItem>) -> Vec<ProcessedItem> {
          items.into_iter().map(|item| item.process()).collect()
      }

      pub fn filter_by_threshold(items: Vec<PriorityItem>, threshold: f64) -> Vec<PriorityItem> {
          items.into_iter().filter(|item| item.score() >= threshold).collect()
      }
  }

Estimated Effort:
  ⏱  3 hours
  📊 Simple complexity (mechanical refactoring)
  ✅ Low risk (type safety ensures correctness)

Automated Fix Available:
  Run: debtmap fix --type-organization --item 3 --dry-run

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#4 UTILITIES SPRAWL: utils.rs [HIGH]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📄 src/utils.rs
   50 functions operating on 10 distinct types

Type Distribution:
  FileMetrics:        8 functions (16%)
  PriorityItem:       5 functions (10%)
  DebtItem:           6 functions (12%)
  GodObjectAnalysis:  4 functions (8%)
  ComplexityMetrics:  3 functions (6%)
  CoverageMetrics:    3 functions (6%)
  Path/PathBuf:       7 functions (14%)
  String:             5 functions (10%)
  Config:             4 functions (8%)
  Other:              5 functions (10%)

Why This Matters:
  ❌ Classic "Utilities" Anti-pattern
  ❌ No clear responsibility
  ❌ Mixed concerns (formatting, validation, calculation, parsing)
  ❌ Hard to find relevant code
  ❌ Encourages adding more random functions

Recommended Fix:

  Break up utils.rs by moving functions to appropriate type modules:

  1. FileMetrics functions → src/analysis/file_metrics.rs
     - calculate_complexity
     - calculate_coverage
     - calculate_debt_score
     - format_metrics
     - validate_metrics
     - merge_metrics
     - normalize_metrics
     - aggregate_metrics

  2. PriorityItem functions → src/priority/priority_item.rs
     - format_priority
     - validate_priority
     - calculate_priority_score
     - normalize_priority
     - is_high_priority

  3. DebtItem functions → src/debt/debt_item.rs
     - [6 functions...]

  4. Path utilities → src/utils/path_utils.rs
     - normalize_path
     - relative_path
     - find_project_root
     - [4 more...]

  5. String utilities → src/utils/string_utils.rs
     - truncate_string
     - pad_string
     - [3 more...]

  After Refactoring:
    src/utils.rs → DELETE ✅
    src/utils/path_utils.rs → 7 focused path utilities
    src/utils/string_utils.rs → 5 focused string utilities

Estimated Effort:
  ⏱  10 hours
  📊 Moderate complexity (many functions to move)
  ⚠️  Medium risk (extensive call site updates)

Automated Fix Available:
  Run: debtmap fix --type-organization --item 4 --dry-run

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

### 3. Quick Fix Plan (--fix-plan)

```bash
debtmap analyze --types --fix-plan
```

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
REFACTORING PLAN
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Total Estimated Effort: 28 hours
Recommended Approach: Incremental (1-2 items per PR)

Week 1: High-Impact Quick Wins (8 hours)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  [ ] Day 1-2: Convert PriorityItem orphaned functions (3h)
      Risk: Low | Impact: High | Difficulty: Simple
      → debtmap fix --item 3

  [ ] Day 3-4: Consolidate DebtItem methods (2.5h)
      Risk: Medium | Impact: High | Difficulty: Moderate
      → debtmap fix --item 2

  [ ] Day 5: Consolidate GodObjectAnalysis methods (2h)
      Risk: Medium | Impact: Medium | Difficulty: Moderate
      → debtmap fix --item 5

Week 2: Major Refactoring (13 hours)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  [ ] Day 1-2: Break up utils.rs (10h)
      Risk: Medium | Impact: Critical | Difficulty: Moderate
      → debtmap fix --item 4
      Note: Large refactoring, consider splitting into sub-PRs

  [ ] Day 3: Consolidate FileMetrics methods (3h)
      Risk: Medium | Impact: Critical | Difficulty: Moderate
      → debtmap fix --item 1

Week 3: Remaining Issues (7 hours)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  [ ] Address remaining medium-priority issues
  [ ] Update documentation
  [ ] Final cleanup

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Automation Support:
  ✅ Items 1-5 support --dry-run preview
  ✅ Items 1-5 support --auto-fix with confirmation
  ⚠️  Always review generated code before committing

Git Workflow:
  1. Create feature branch: git checkout -b refactor/type-organization
  2. Fix one item at a time
  3. Run tests after each fix: cargo test
  4. Commit with descriptive message
  5. Create PR when ready

To start:
  debtmap fix --item 3 --dry-run    # Preview changes
  debtmap fix --item 3 --auto-fix   # Apply with confirmation
```

### 4. Single Item Detail (--item N)

```bash
debtmap analyze --types --item 1
```

Shows detailed view of just that item (same as detailed view but focused).

### 5. JSON Output (--json)

```bash
debtmap analyze --types --json > type-analysis.json
```

```json
{
  "version": "0.3.5",
  "timestamp": "2025-01-19T10:30:00Z",
  "codebase": {
    "root": "/Users/glen/debtmap/src",
    "files_analyzed": 127,
    "lines_of_code": 45678
  },
  "overall_health": {
    "score": 62,
    "grade": "C",
    "status": "NEEDS_IMPROVEMENT"
  },
  "issues": {
    "scattered_types": [
      {
        "rank": 1,
        "type_name": "FileMetrics",
        "severity": "CRITICAL",
        "definition_file": "src/analysis/file_metrics.rs",
        "total_methods": 23,
        "file_count": 6,
        "method_locations": {
          "src/utils.rs": [
            {"name": "calculate_complexity", "line": 120},
            {"name": "calculate_coverage", "line": 145}
          ]
        },
        "recommendation": {
          "title": "Consolidate FileMetrics methods",
          "effort_hours": 3.0,
          "complexity": "MODERATE",
          "risk": "MEDIUM",
          "actions": [
            {
              "type": "MOVE_METHOD",
              "from": "src/utils.rs",
              "to": "src/analysis/file_metrics.rs",
              "items": ["calculate_complexity", "calculate_coverage"]
            }
          ]
        }
      }
    ],
    "orphaned_functions": [
      {
        "rank": 3,
        "target_type": "PriorityItem",
        "function_count": 12,
        "severity": "HIGH",
        "functions": [
          {"name": "format_priority", "file": "src/utils.rs", "line": 456}
        ]
      }
    ]
  }
}
```

### 6. Interactive Mode (--interactive)

```bash
debtmap analyze --types --interactive
```

```
Type Organization Analysis - Interactive Mode

What would you like to do?

  1. View summary
  2. Explore scattered types
  3. Explore orphaned functions
  4. Explore utilities sprawl
  5. Generate fix plan
  6. Apply automated fixes
  7. Export report
  q. Quit

Choice: 2

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
SCATTERED TYPES (5 found)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  1. [CRITICAL] FileMetrics - 23 methods across 6 files
  2. [CRITICAL] DebtItem - 18 methods across 5 files
  3. [HIGH] GodObjectAnalysis - 15 methods across 4 files
  4. [MEDIUM] ComplexityMetrics - 8 methods across 3 files
  5. [MEDIUM] CoverageMetrics - 7 methods across 3 files

Select item to view details (1-5) or 'b' to go back: 1

[Shows detailed view of FileMetrics...]

Actions:
  d. View detailed breakdown
  f. Generate fix for this item
  p. Preview automated fix
  a. Apply automated fix
  n. Next item
  b. Back to menu

Choice: p

[Shows git diff preview of proposed changes...]

Apply this fix? (y/n/e to edit):
```

### 7. CI Integration (--ci)

```bash
# In CI pipeline
debtmap analyze --types --ci --threshold 70

# Exit codes:
# 0 = Health score >= threshold (pass)
# 1 = Health score < threshold (fail)
# 2 = Error in analysis
```

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
CI CHECK: Type Organization
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Health Score: 62/100
Threshold: 70/100

❌ FAIL - Health score below threshold

New Issues Introduced (since baseline):
  - FileMetrics: +2 new scattered methods in src/new_utils.rs
  - New orphaned function: process_new_item in src/processing.rs

Recommendation:
  Move new FileMetrics methods to src/analysis/file_metrics.rs
  Convert process_new_item to method on appropriate type

Block this PR until health score >= 70 or baseline is updated.
```

### 8. Diff Mode (--compare)

```bash
# Compare two branches/commits
debtmap analyze --types --compare main..feature-branch
```

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
TYPE ORGANIZATION DELTA (main → feature-branch)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Health Score: 58 → 65 (+7) ✅ IMPROVED

Changes:

  ✅ FIXED
    - Consolidated PriorityItem (was scattered across 3 files)
    - Converted 12 orphaned functions to methods

  ❌ NEW ISSUES
    - FileMetrics: +2 scattered methods in src/new_utils.rs
    - New utilities file: src/helpers2.rs (8 functions, 4 types)

  ⚠️  WORSENED
    - DebtItem: +3 additional scattered methods

Net Change:
  Scattered types: 5 → 6 (+1) ⚠️
  Orphaned functions: 12 → 2 (-10) ✅
  Utilities modules: 3 → 4 (+1) ⚠️

Overall: Mixed results - some improvements, some regressions
```

### 9. Watch Mode (--watch)

```bash
debtmap analyze --types --watch
```

```
Watching for changes in /Users/glen/debtmap/src...
Press Ctrl+C to stop

[09:30:45] File changed: src/utils.rs
[09:30:46] Running analysis...
[09:30:48] ⚠️  New issue detected:
           - Added orphaned function: process_metrics (line 567)
           - Suggestion: Move to src/analysis/file_metrics.rs

[09:32:12] File changed: src/analysis/file_metrics.rs
[09:32:13] Running analysis...
[09:32:15] ✅ Issue resolved:
           - process_metrics converted to impl method

Health Score: 62 → 63 (+1)
```

---

## Visual Enhancements

### Progress Bars

```
Analyzing codebase...
[████████████████████████████████] 127/127 files (100%)

Building type map...
[████████████████████████████████] 423/423 types (100%)

Detecting scattered types...
[████████████████████████████████] 100% Complete

Analysis complete in 2.3s
```

### Color Coding

- 🔴 Red: Critical issues
- 🟡 Yellow: High/Medium issues
- 🟢 Green: No issues / Good
- ⚪ Gray: Low priority / Info

### Icons/Symbols

- ✅ Fixed/Good
- ❌ Issue/Problem
- ⚠️ Warning
- 📄 File
- 📊 Statistics
- ⏱ Time estimate
- 🎯 Priority

### Health Score Gauge

```
Health Score: 62/100

[████████████████████░░░░░░░░░░░░░░░░░░░] 62%

0────25────50────75────100
│     │     │  ^  │     │
Poor  Fair  OK  │  Excellent
               You are here

Grade: C (Needs Improvement)
```

---

## Integration Examples

### Combined with God Object Analysis

```bash
debtmap analyze --all
```

```
debtmap v0.3.5 - Comprehensive Analysis

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
GOD OBJECTS (3 found)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#1 [CRITICAL] formatter.rs (score: 149)
   3000 lines, 103 functions
   → See: debtmap analyze --god-objects --item 1

#2 [CRITICAL] god_object_analysis.rs (score: 89)
   1200 lines, 45 functions
   → See: debtmap analyze --god-objects --item 2

#3 [HIGH] debt_analyzer.rs (score: 67)
   800 lines, 32 functions
   → See: debtmap analyze --god-objects --item 3

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
TYPE ORGANIZATION (Health: 62/100)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

5 scattered types, 12 orphaned function groups, 3 utilities modules
→ See: debtmap analyze --type-organization

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
COMPLEXITY HOTSPOTS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[Existing complexity analysis...]

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
SUMMARY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Overall Codebase Health: 58/100 (Needs Improvement)

Priority Actions:
  1. Refactor god objects (3 files)
  2. Consolidate scattered types (5 types)
  3. Convert orphaned functions (12 groups)
  4. Address complexity hotspots

Estimated Total Effort: 45 hours
```

This gives users a **comprehensive, actionable view** of their codebase architecture with multiple presentation modes for different use cases!