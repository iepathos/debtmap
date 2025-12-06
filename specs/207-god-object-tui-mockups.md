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

## Detail View - Dependencies Page (Page 2/5)

Shows both file-level AND aggregated function-level dependencies:

```
┌─ Item Details (2/5: Dependencies) ──────────────────────────────────────┐
│                                                                          │
│ FILE-LEVEL DEPENDENCIES                                                  │
│   Files that import this file: 0                                        │
│   Files imported by this file: 23                                       │
│   Import Fanout: 23 (HIGH - imports many modules)                       │
│   Export Fanin: 0 (entry point, not imported)                           │
│                                                                          │
│ AGGREGATED FUNCTION DEPENDENCIES (91 functions)                         │
│   Total Upstream Callers: 247                                           │
│   Total Downstream Callees: 412                                         │
│   Aggregated Blast Radius: 659 (HIGH)                                   │
│   Functions on Critical Path: 34 (37%)                                  │
│                                                                          │
│ TOP 5 MOST CONNECTED FUNCTIONS                                           │
│   1. process_request (67 callers, 45 callees) - line 342                │
│   2. validate_input (52 callers, 18 callees) - line 156                 │
│   3. transform_data (48 callers, 31 callees) - line 892                 │
│   4. handle_error (43 callers, 12 callees) - line 1203                  │
│   5. write_output (39 callers, 23 callees) - line 1405                  │
│                                                                          │
│ DEPENDENCY DISTRIBUTION BY RESPONSIBILITY                                │
│   Input/Argument Parsing: 89 upstream, 142 downstream                   │
│   Data Processing: 67 upstream, 98 downstream                           │
│   Output Formatting: 45 upstream, 87 downstream                         │
│   Error Handling: 23 upstream, 45 downstream                            │
│   File I/O Operations: 12 upstream, 23 downstream                       │
│   Analysis Coordination: 8 upstream, 12 downstream                      │
│   Caching/Memoization: 2 upstream, 3 downstream                         │
│   Logging/Telemetry: 1 upstream, 2 downstream                           │
│                                                                          │
│ COUPLING HOTSPOTS                                                        │
│   High Fanin (many callers):                                            │
│     - process_request (67) - Central coordination function              │
│     - validate_input (52) - Called by all input paths                   │
│                                                                          │
│   High Fanout (many callees):                                           │
│     - process_request (45) - Orchestrates many operations               │
│     - write_output (23) - Calls many formatters                         │
│                                                                          │
│ IMPACT OF SPLITTING                                                      │
│   Current: 659 total dependencies in single file                        │
│   After Split (estimated):                                               │
│     Module 1: ~220 dependencies (contained within module)               │
│     Module 2: ~180 dependencies (contained within module)               │
│     Module 3: ~140 dependencies (contained within module)               │
│     Cross-module: ~119 dependencies (reduced from 659)                  │
│   Benefit: 82% reduction in cross-cutting dependencies                  │
│                                                                          │
├──────────────────────────────────────────────────────────────────────────┤
│ Press ◀▶/hl:Pages  ↑↓/jk:Scroll  ←/q:Back  ?:Help                       │
└──────────────────────────────────────────────────────────────────────────┘
```

## Detail View - Git Context Page (Page 3/5)

Shows git history and change patterns for the god object file:

```
┌─ Item Details (3/5: Git Context) ───────────────────────────────────────┐
│                                                                          │
│ GIT HISTORY                                                              │
│   Total Commits: 247                                                     │
│   First Commit: 2023-01-15 (Initial implementation)                     │
│   Last Modified: 2025-12-01 (2 days ago)                                │
│   File Age: 687 days                                                     │
│                                                                          │
│ CHANGE FREQUENCY                                                         │
│   Commits per Month: 10.8                                                │
│   Changes Last 30 Days: 12                                               │
│   Changes Last 90 Days: 34                                               │
│   Change Trend: INCREASING (high churn)                                 │
│                                                                          │
│ CONTRIBUTORS                                                             │
│   Total Contributors: 8                                                  │
│   Primary Author: alice@example.com (142 commits, 57%)                  │
│   Recent Contributors:                                                   │
│     - bob@example.com (45 commits, 18%)                                 │
│     - charlie@example.com (28 commits, 11%)                             │
│     - diana@example.com (18 commits, 7%)                                │
│     ... 4 more                                                           │
│                                                                          │
│ HOTSPOT ANALYSIS                                                         │
│   This file is a HOTSPOT (high churn + high complexity)                 │
│   Risk Level: CRITICAL                                                   │
│   Priority for Refactoring: URGENT                                      │
│                                                                          │
│ CHANGE IMPACT                                                            │
│   Lines Added: 3,247                                                     │
│   Lines Deleted: 1,631                                                   │
│   Net Growth: +1,616 lines                                               │
│   Growth Rate: Steady increase (indicates accumulating responsibility)  │
│                                                                          │
│ RECENT COMMIT MESSAGES (Last 5)                                          │
│   1. [2 days ago] Fix bug in request handling logic                     │
│   2. [1 week ago] Add new data processing pipeline                      │
│   3. [2 weeks ago] Refactor configuration parsing                       │
│   4. [3 weeks ago] Update error handling                                │
│   5. [1 month ago] Add support for new file format                      │
│                                                                          │
│ NOTE: High churn on this god object indicates ongoing pain. Splitting   │
│       will reduce future merge conflicts and make changes safer.        │
│                                                                          │
├──────────────────────────────────────────────────────────────────────────┤
│ Press ◀▶/hl:Pages  ↑↓/jk:Scroll  ←/q:Back  ?:Help                       │
└──────────────────────────────────────────────────────────────────────────┘
```

## Detail View - Patterns Page (Page 4/5)

Shows aggregated patterns across all functions, grouped by responsibility:

```
┌─ Item Details (4/5: Patterns) ──────────────────────────────────────────┐
│                                                                          │
│ AGGREGATED PATTERN ANALYSIS (91 functions)                              │
│                                                                          │
│ DETECTED ANTI-PATTERNS                                                   │
│   1. God Object (Definite)                                               │
│      - 49 methods, 87 fields, 8 responsibilities                        │
│      - Violates Single Responsibility Principle                         │
│                                                                          │
│   2. High Coupling                                                       │
│      - Imports 23 modules                                                │
│      - Many methods share large portion of fields                       │
│                                                                          │
│   3. Long File                                                           │
│      - 1616 lines (threshold: 500)                                       │
│      - Difficult to navigate and understand                             │
│                                                                          │
│ COMPLEXITY PATTERNS                                                      │
│   Functions with High Complexity (>10): 12 (13%)                        │
│   Functions with Deep Nesting (>3): 8 (9%)                              │
│   Functions with Many Parameters (>4): 15 (16%)                         │
│   Long Functions (>50 LOC): 23 (25%)                                    │
│                                                                          │
│ RESPONSIBILITY BREAKDOWN                                                 │
│   The file has 8 distinct responsibilities:                              │
│                                                                          │
│   1. Input/Argument Parsing (23 functions)                               │
│      - parse_cli_args, validate_input, setup_config                     │
│      - Process command line arguments and configuration                 │
│                                                                          │
│   2. Data Processing (18 functions)                                      │
│      - process_file, transform_data, aggregate_results                  │
│      - Core business logic and transformations                          │
│                                                                          │
│   3. Output Formatting (15 functions)                                    │
│      - format_markdown, write_json, render_tui                          │
│      - Various output format generators                                 │
│                                                                          │
│   4. Error Handling (12 functions)                                       │
│      - handle_error, log_warning, recover_from_failure                  │
│      - Error recovery and logging                                       │
│                                                                          │
│   5. File I/O Operations (10 functions)                                  │
│      - read_file, write_output, scan_directory                          │
│      - Filesystem interactions                                          │
│                                                                          │
│   6. Analysis Coordination (8 functions)                                 │
│      - run_analysis, coordinate_workers, merge_results                  │
│      - Orchestration logic                                              │
│                                                                          │
│   7. Caching/Memoization (3 functions)                                   │
│      - cache_result, invalidate_cache, load_cached                      │
│      - Performance optimization                                         │
│                                                                          │
│   8. Logging/Telemetry (2 functions)                                     │
│      - log_metric, send_telemetry                                       │
│      - Observability                                                    │
│                                                                          │
│ SUGGESTED ORGANIZATION                                                   │
│   Split into 3 focused modules:                                          │
│     - input_handler: Responsibilities 1, 4, 5                           │
│     - core_processor: Responsibilities 2, 6                             │
│     - output_writer: Responsibilities 3, 7, 8                           │
│                                                                          │
├──────────────────────────────────────────────────────────────────────────┤
│ Press ◀▶/hl:Pages  ↑↓/jk:Scroll  ←/q:Back  ?:Help                       │
└──────────────────────────────────────────────────────────────────────────┘
```

## Detail View - Data Flow (Page 5/5)

**Note:** For god objects, page 5 shows **aggregated data flow** across all functions in the file.

Shows accumulated data flow patterns across all 91 functions:

```
┌─ Item Details (5/5: Data Flow Analysis) ────────────────────────────────┐
│                                                                          │
│ AGGREGATED MUTATION ANALYSIS (91 functions)                             │
│   Total Mutations: 847                                                   │
│   Live Mutations: 623 (73.5%)                                            │
│   Dead Stores: 224 (26.5%)                                               │
│   Functions with Mutations: 68 of 91 (75%)                              │
│                                                                          │
│ PURITY ANALYSIS                                                          │
│   Pure Functions: 23 (25%)                                               │
│   Impure Functions: 68 (75%)                                             │
│   Purity Score: 0.25 (LOW - high mutation rate)                         │
│                                                                          │
│   Top 5 Most Impure Functions:                                           │
│     1. process_request (127 mutations) - line 342                        │
│     2. update_state (89 mutations) - line 567                            │
│     3. handle_error (72 mutations) - line 1203                           │
│     4. transform_data (58 mutations) - line 892                          │
│     5. validate_input (43 mutations) - line 156                          │
│                                                                          │
│ I/O OPERATIONS                                                           │
│   Functions with I/O: 34 of 91 (37%)                                    │
│   Total I/O Operations: 156                                              │
│                                                                          │
│   I/O Categories:                                                        │
│     File I/O: 67 operations (43%)                                        │
│     Network I/O: 28 operations (18%)                                     │
│     Console/Logging: 45 operations (29%)                                 │
│     Database: 16 operations (10%)                                        │
│                                                                          │
│   Most I/O-Heavy Functions:                                              │
│     1. write_output (23 file ops) - line 1405                            │
│     2. read_config (18 file ops) - line 89                               │
│     3. send_telemetry (15 network ops) - line 1534                       │
│                                                                          │
│ FUNCTIONAL PURITY BY RESPONSIBILITY                                      │
│   Input/Argument Parsing: 13% pure (3/23)                               │
│   Data Processing: 67% pure (12/18)                                      │
│   Output Formatting: 7% pure (1/15)                                      │
│   Error Handling: 8% pure (1/12)                                         │
│   File I/O Operations: 0% pure (0/10)                                    │
│   Analysis Coordination: 25% pure (2/8)                                  │
│   Caching/Memoization: 33% pure (1/3)                                    │
│   Logging/Telemetry: 0% pure (0/2)                                       │
│                                                                          │
│ SPLITTING RECOMMENDATIONS BASED ON DATA FLOW                             │
│   Module 1 (Data Processing): 67% pure - Good candidate for pure module │
│   Module 2 (I/O Operations): 0% pure - Isolate impure operations        │
│   Module 3 (Coordination): 25% pure - Mixed, needs careful separation   │
│                                                                          │
│ ESCAPE ANALYSIS                                                          │
│   Functions returning mutable references: 23 (25%)                      │
│   Functions with escaped allocations: 41 (45%)                          │
│   Potential memory leaks detected: 3                                     │
│                                                                          │
│ NOTE: High mutation rate and low purity score suggest splitting along   │
│       pure/impure boundaries will improve testability and maintainability│
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
| Regular Function | `handle_request::request.rs` | None | `(Cov:15% Cog:45)` or `(Cov:15% Cog:45 LOC:120)` |

**Note**: All line counts use `LOC:` for consistency (not `Len:`). Functions may optionally show LOC if available.

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

Mapping to unified score:
- god_object_score >= 50.0 → Tier 1 (Critical)
- god_object_score >= 30.0 → Tier 2 (High)
- god_object_score < 30.0 → Tier 3 (Medium)

## Summary: All 5 Detail Pages - Same Structure, Aggregated for God Objects

**Design Principle**: God objects use the SAME 5 pages as functions, but show aggregated data across all functions within the file.

| Page | Title | God Objects (Aggregated) | Regular Functions (Single) |
|------|-------|--------------------------|---------------------------|
| **1/5** | Overview | File-level metrics, god object indicators, recommended splits | Function metrics, coverage, complexity |
| **2/5** | Dependencies | File imports + **aggregated** upstream/downstream (247/412), blast radius (659) | Single function callers/callees |
| **3/5** | Git Context | File-level git history, change frequency, hotspot analysis | File-level git history (same) |
| **4/5** | Patterns | **Aggregated** patterns across 91 functions, responsibility breakdown | Single function patterns (entropy, purity) |
| **5/5** | Data Flow | **Aggregated** mutations (847), purity by responsibility, I/O breakdown | Single function mutations, I/O operations |

### God Objects = Aggregate of Their Functions

**Page 1 (Overview)**:
- Regular Function: Shows that function's complexity, coverage
- God Object: Shows file-level metrics aggregated across 91 functions

**Page 2 (Dependencies)**:
- Regular Function: Shows that function's callers and callees
- God Object: Shows **sum** of all 91 functions' callers (247) and callees (412)

**Page 3 (Git Context)**:
- Same for both (git data is file-level anyway)

**Page 4 (Patterns)**:
- Regular Function: Shows patterns for that function (entropy, purity)
- God Object: Shows **aggregated** patterns across all functions, grouped by responsibility

**Page 5 (Data Flow)**:
- Regular Function: Shows mutations and I/O for that function
- God Object: Shows **total** mutations (847) and I/O (156) across all 91 functions

### No Separate Impact Page

Impact analysis (if added later) would be a feature for BOTH god objects and functions, not a separate page. The 5-page structure remains consistent.

This design ensures god objects have relevant, actionable information on all 5 pages while maintaining consistent navigation and structure.
