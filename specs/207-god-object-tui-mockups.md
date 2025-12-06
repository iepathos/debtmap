# God Object TUI Display Mockups

This document shows how god objects will appear in the TUI after implementing Spec 207.

## List View Display

### Ungrouped List View

God objects will appear alongside other debt items, sorted by score:

```
┌─ Debtmap Results  Total: 127  Debt Score: 1,234  Density: 4.23/1K LOC ─┐
│ Sort: Score (High to Low)  Filters: 0  Grouping: OFF                    │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│ ▸ #1    CRITICAL   50.4    main.rs (God Object)  (LOC:1616 Resp:8 Fns:91)│
│   #2    HIGH       38.2    handle_request::request.rs  (Cov:15% Cog:45) │
│   #3    HIGH       35.7    formatter.rs (God Module)  (LOC:850 Fns:116) │
│   #4    HIGH       32.1    parse_ast::parser.rs  (Cov:0% Cog:38)        │
│   #5    MEDIUM     28.4    calculate_score::scoring.rs  (Cov:45% Cog:22)│
│                                                                          │
├──────────────────────────────────────────────────────────────────────────┤
│ 1/127 items  |  ↑↓/jk:Nav  G:Group  /:Search  s:Sort  f:Filter  ?:Help │
└──────────────────────────────────────────────────────────────────────────┘
```

**Key Features:**
- God objects show file name instead of function name (e.g., `main.rs (God Object)`)
- Metrics show LOC and responsibilities instead of coverage/complexity
- Icon: "God Object" or "God Module" suffix to distinguish from functions
- Color: RED for CRITICAL severity (score >= 50.0)

### Grouped List View

When grouping by location, god objects appear as single items (no grouping since they're file-level):

```
┌─ Debtmap Results  Total: 127  Debt Score: 1,234  Density: 4.23/1K LOC ─┐
│ Sort: Score (High to Low)  Filters: 0  Grouping: ON                     │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│ ▸ #1    CRITICAL   50.4    main.rs (God Object)  (LOC:1616 Resp:8 Fns:91)│
│   #2    HIGH       38.2    request.rs [3]  (Cov:15% Cog:45 Nest:4)      │
│   #3    HIGH       35.7    formatter.rs (God Module)  (LOC:850 Fns:116) │
│   #4    HIGH       32.1    parser.rs  (Cov:0% Cog:38)                   │
│                                                                          │
├──────────────────────────────────────────────────────────────────────────┤
│ 1/127 items  |  ↑↓/jk:Nav  G:Group  /:Search  s:Sort  f:Filter  ?:Help │
└──────────────────────────────────────────────────────────────────────────┘
```

**Note:** God objects don't have multiple function-level issues to group, so they appear as single items.

## Detail View - Overview Page (Page 1/5)

When selecting a god object in the list, the detail view shows file-level metrics:

```
┌─ Item Details (1/5: Overview) ──────────────────────────────────────────┐
│                                                                          │
│ LOCATION                                                                 │
│   File: ./src/main.rs                                                    │
│   Type: God Object (File-level architectural issue)                     │
│   Lines: 1616                                                            │
│                                                                          │
│ SCORE                                                                    │
│   Total: 50.4  [CRITICAL]                                                │
│   Tier: 1 (Critical - Immediate Action Required)                        │
│                                                                          │
│ GOD OBJECT METRICS                                                       │
│   Detection Type: God Class                                              │
│   Methods: 49                                                            │
│   Fields: 87                                                             │
│   Responsibilities: 8                                                    │
│   God Object Score: 100.0 (Definite god object)                         │
│                                                                          │
│ FILE METRICS                                                             │
│   Total Lines: 1616                                                      │
│   Total Functions: 91                                                    │
│   Average Complexity: 2.2                                                │
│   Total Complexity: 200                                                  │
│                                                                          │
│ RECOMMENDED SPLITS                                                       │
│   1. Data processing module (23 functions)                               │
│   2. Input/parsing module (18 functions)                                 │
│   3. Output formatting module (15 functions)                             │
│                                                                          │
│ RECOMMENDATION                                                           │
│   Action: URGENT: Split file by data flow boundaries                    │
│                                                                          │
│   Rationale: File has 8 distinct responsibilities across 49 methods.    │
│   High coupling makes changes risky and testing difficult. Splitting    │
│   by responsibility will improve maintainability and reduce change       │
│   impact.                                                                │
│                                                                          │
├──────────────────────────────────────────────────────────────────────────┤
│ Press ◀▶/hl:Pages  ↑↓/jk:Scroll  ←/q:Back  ?:Help                       │
└──────────────────────────────────────────────────────────────────────────┘
```

**Key Sections:**

1. **LOCATION**
   - Shows file path (not function, since this is file-level)
   - Indicates "God Object (File-level architectural issue)"
   - Shows total line count

2. **SCORE**
   - Displays unified score (0-100 scale)
   - Shows severity label with color
   - Indicates tier (Tier 1 for critical god objects)

3. **GOD OBJECT METRICS**
   - Detection Type: "God Class" or "God File/Module"
   - Methods: Number of methods on the primary struct (for God Class)
   - Fields: Number of fields on the primary struct
   - Responsibilities: Number of distinct responsibilities detected
   - God Object Score: 0-100 score indicating severity

4. **FILE METRICS**
   - Total Lines: Lines of code in the file
   - Total Functions: All functions (methods + module functions)
   - Average Complexity: Average cyclomatic complexity
   - Total Complexity: Sum of all function complexities

5. **RECOMMENDED SPLITS**
   - Shows suggested module boundaries
   - Lists how many functions would go in each module
   - Based on responsibility clustering analysis

6. **RECOMMENDATION**
   - Action: What to do (e.g., "Split file by data flow")
   - Rationale: Why this matters (coupling, testing difficulty, etc.)

## Detail View - God Module Example

For comparison, here's how a **God Module** (file with many functions, not a class) would look:

```
┌─ Item Details (1/5: Overview) ──────────────────────────────────────────┐
│                                                                          │
│ LOCATION                                                                 │
│   File: ./src/priority/formatter.rs                                      │
│   Type: God Module (File-level architectural issue)                     │
│   Lines: 850                                                             │
│                                                                          │
│ SCORE                                                                    │
│   Total: 35.7  [HIGH]                                                    │
│   Tier: 2 (High Priority)                                                │
│                                                                          │
│ GOD MODULE METRICS                                                       │
│   Detection Type: God File                                               │
│   Module Functions: 116                                                  │
│   Methods: 0 (no dominant struct)                                        │
│   Responsibilities: 5                                                    │
│   God Object Score: 78.5 (Likely god module)                            │
│                                                                          │
│ FILE METRICS                                                             │
│   Total Lines: 850                                                       │
│   Total Functions: 116                                                   │
│   Average Complexity: 1.8                                                │
│   Total Complexity: 208                                                  │
│                                                                          │
│ RECOMMENDED SPLITS                                                       │
│   1. Legacy formatter module (48 functions)                              │
│   2. Markdown formatter module (38 functions)                            │
│   3. Helper utilities module (30 functions)                              │
│                                                                          │
│ RECOMMENDATION                                                           │
│   Action: Split formatter.rs into focused submodules                    │
│                                                                          │
│   Rationale: File contains 116 module-level functions with 5 distinct   │
│   responsibilities. Lack of cohesion makes navigation and maintenance   │
│   difficult. Organize by formatter type for better discoverability.     │
│                                                                          │
├──────────────────────────────────────────────────────────────────────────┤
│ Press ◀▶/hl:Pages  ↑↓/jk:Scroll  ←/q:Back  ?:Help                       │
└──────────────────────────────────────────────────────────────────────────┘
```

**Differences from God Object:**
- Detection Type: "God File" instead of "God Class"
- Shows "Module Functions" count instead of "Methods"
- Methods: 0 (indicates no dominant struct)
- Recommendation focuses on module organization instead of class extraction

## Detail View - Impact Page (Page 2/5)

Shows the expected impact of fixing the god object:

```
┌─ Item Details (2/5: Impact) ────────────────────────────────────────────┐
│                                                                          │
│ EXPECTED IMPACT                                                          │
│   Complexity Reduction: -66 (200 → 134 across 3 modules)                │
│   Maintainability Improvement: +248.4 points                            │
│   Risk Reduction: -87.3 points (high coupling → loose coupling)         │
│   Effort Estimate: HIGH (1616 LOC, 91 functions to refactor)            │
│                                                                          │
│ COMPLEXITY BREAKDOWN                                                     │
│   Current Total Complexity: 200                                          │
│   After Split (estimated):                                               │
│     Module 1: 73 complexity (23 functions, avg: 3.2)                    │
│     Module 2: 36 complexity (18 functions, avg: 2.0)                    │
│     Module 3: 25 complexity (15 functions, avg: 1.7)                    │
│   Total Reduction: 66 points (33%)                                       │
│                                                                          │
│ MAINTAINABILITY FACTORS                                                  │
│   Current State:                                                         │
│     - 8 responsibilities in single file                                  │
│     - 49 methods sharing 87 fields (high state coupling)                │
│     - Average function sees 65% of all fields (tight coupling)          │
│                                                                          │
│   After Split:                                                           │
│     - 3 focused modules with 2-3 responsibilities each                  │
│     - Reduced method-to-field ratios (better encapsulation)             │
│     - Clear module boundaries and interfaces                            │
│                                                                          │
│ RISK REDUCTION                                                           │
│   Current Risks:                                                         │
│     - Changes impact multiple unrelated features                        │
│     - Testing requires understanding entire file                        │
│     - Merge conflicts highly likely                                     │
│                                                                          │
│   After Split:                                                           │
│     - Changes isolated to specific modules                              │
│     - Testing focused on single responsibility                          │
│     - Reduced merge conflict probability                                │
│                                                                          │
├──────────────────────────────────────────────────────────────────────────┤
│ Press ◀▶/hl:Pages  ↑↓/jk:Scroll  ←/q:Back  ?:Help                       │
└──────────────────────────────────────────────────────────────────────────┘
```

## Filtering and Sorting

### God Objects in Tier Filters

When filtering by severity:

```
┌─ Filters (press key, Esc to cancel) ────────────────────────────────────┐
│ Severity Filters:                                                        │
│   1. Critical     ← God objects with score >= 50.0                       │
│   2. High         ← God objects with score >= 25.0                       │
│   3. Medium                                                              │
│   4. Low                                                                 │
│                                                                          │
│ Coverage Filters:                                                        │
│   n. No Coverage  ← N/A for god objects (file-level items)              │
│   l. Low (0-30%)                                                         │
│   m. Medium (30-70%)                                                     │
│   h. High (70-100%)                                                      │
│                                                                          │
│   c. Clear all filters                                                   │
│                                                                          │
│ Active filters:                                                          │
│   • Critical                                                             │
└──────────────────────────────────────────────────────────────────────────┘
```

**Note:** Coverage filters don't apply to god objects since they're file-level items.

### Sorting Options

God objects work with all sort criteria:

```
┌─ Sort By (press number, Esc to cancel) ─────────────────────────────────┐
│ 1. Score (High to Low)     ← Default, god objects sorted by score       │
│ 2. Score (Low to High)                                                   │
│ 3. Complexity              ← Sorts by total file complexity             │
│ 4. Coverage Impact         ← N/A for god objects                        │
│ 5. File Name               ← Alphabetical by file path                  │
│ 6. Severity                ← Groups by CRITICAL/HIGH/MEDIUM/LOW         │
└──────────────────────────────────────────────────────────────────────────┘
```

## Search and Navigation

God objects are searchable by file name:

```
┌─ Search (Esc to cancel, Enter to apply) ────────────────────────────────┐
│ Search: main.rs                                                          │
└──────────────────────────────────────────────────────────────────────────┘

Results:
  #1   CRITICAL   50.4    main.rs (God Object)  (LOC:1616 Resp:8 Fns:91)
  #8   MEDIUM     18.3    init_logger::main.rs  (Cov:45% Cog:12)
  #12  MEDIUM     14.7    parse_args::main.rs  (Cov:78% Cog:8)
```

## Integration with Existing TUI Features

### Help Screen

The help screen will be updated to explain god objects:

```
┌─ Help ───────────────────────────────────────────────────────────────────┐
│                                                                          │
│ NAVIGATION                                                               │
│   ↑↓ / j k     Move up/down                                             │
│   Enter        View item details                                        │
│   ← / q        Back to list / Quit                                      │
│   G            Toggle grouping                                           │
│                                                                          │
│ ITEM TYPES                                                               │
│   Function Items:  Individual functions with complexity/coverage issues │
│   God Objects:     Files with too many methods/fields (architectural)   │
│   God Modules:     Files with too many unrelated functions              │
│                                                                          │
│ FILTERING                                                                │
│   f              Open filter menu                                        │
│   /              Search by name                                          │
│   s              Change sort order                                       │
│                                                                          │
│ NOTE: God objects are file-level items that bypass complexity filtering │
│       They appear based on architectural issues, not function complexity │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

## Summary of Visual Indicators

| Item Type | List View Format | Icon/Suffix | Metrics Shown |
|-----------|------------------|-------------|---------------|
| God Object (God Class) | `main.rs (God Object)` | "(God Object)" | `(LOC:1616 Resp:8 Fns:91)` |
| God Module (God File) | `formatter.rs (God Module)` | "(God Module)" | `(LOC:850 Fns:116)` |
| Regular Function | `handle_request::request.rs` | None | `(Cov:15% Cog:45)` |

## Color Coding

- **CRITICAL** (Red): Score >= 50.0 - Immediate action required
- **HIGH** (Light Red): Score >= 25.0 - High priority
- **MEDIUM** (Yellow): Score >= 10.0 - Medium priority
- **LOW** (Green): Score < 10.0 - Low priority

God objects typically score 30-100 based on:
- Number of responsibilities (major factor)
- Number of methods/functions
- Number of fields (for God Classes)
- Total lines of code
- Complexity distribution
