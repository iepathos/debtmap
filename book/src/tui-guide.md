# TUI Results Viewer Guide

Debtmap includes an interactive TUI (Text User Interface) results viewer that allows you to explore analysis results in detail without re-running the analysis.

## Launching the TUI

After running an analysis that produces a results file, launch the TUI viewer:

```bash
# Run analysis and save results
debtmap analyze . -o results.json --format json

# Launch interactive TUI viewer
debtmap results results.json
```

The TUI provides an interactive interface for exploring technical debt items, with detailed views and keyboard navigation.

## Navigation

### List View (Main Screen)

The main screen shows a list of technical debt items sorted by priority score:

**Keyboard Controls:**
- `↑/↓` or `j/k` - Navigate up/down through the list
- `Enter` - View detailed information for selected item
- `q` - Quit the application
- `?` - Show help

**Display Information:**
- Item rank and priority score
- Function location (file:line)
- Debt type indicator
- Brief description

### Detail View

When you select an item with `Enter`, the detail view opens with multiple pages of information.

## Detail Pages

The detail view provides 5 pages of in-depth analysis for each debt item:

### Page 1: Overview

Displays core information about the technical debt item:

- **Priority Score:** Final unified score and breakdown
- **Location:** File path, line number, and function name
- **Debt Type:** Classification (Complexity, Side Effects, etc.)
- **Description:** Detailed explanation of the issue
- **Function Role:** Role in the codebase (Entry Point, Core Logic, etc.)

**Score Breakdown:**
- Base complexity score
- Impact multipliers (downstream dependencies, test coverage)
- Contextual adjustments
- Final unified score

### Page 2: Metrics

Shows detailed complexity and quality metrics:

**Complexity Metrics:**
- Cyclomatic Complexity
- Cognitive Complexity
- Lines of Code (LOC)
- Nesting Depth

**Understanding Complexity vs Accumulated Complexity:**

The metrics display varies depending on whether you're viewing a regular function or a god object:

- **Regular Functions:** The "Complexity" section shows metrics for the individual function:
  - **Cyclomatic Complexity:** The function's own cyclomatic complexity
  - **Cognitive Complexity:** The function's own cognitive complexity
  - **Nesting Depth:** Maximum nesting depth within the function

- **God Objects:** The "Accumulated Complexity" section shows aggregated metrics across all methods in the class:
  - **Cyclomatic Complexity:** Sum of cyclomatic complexity across all methods
  - **Cognitive Complexity:** Sum of cognitive complexity across all methods
  - **Nesting Depth:** Maximum nesting depth found in any method

The "accumulated" label indicates that these are combined metrics representing the total complexity burden of the entire class, not just a single method. This helps identify classes that have grown too large and may need to be split into smaller, more focused components.

**Quality Indicators:**
- Maintainability Index
- Halstead Metrics (if available)
- Code duplication percentage
- Comment density

**Boilerplate Analysis:**
- Boilerplate percentage
- Pattern repetition score
- Effective complexity (adjusted for boilerplate)

### Page 3: Recommendations

Provides actionable refactoring guidance:

**Refactoring Strategy:**
- Recommended approach (Extract Method, Decompose, etc.)
- Estimated effort (hours)
- Expected benefit/improvement

**Step-by-Step Actions:**
1. Specific refactoring steps
2. Pattern recommendations
3. Testing strategies

**Code Examples:**
- Before/after snippets (when applicable)
- Suggested function signatures
- Pattern implementations

### Page 4: Context

Displays contextual information about the function's role and impact:

**Call Graph Analysis:**
- Upstream callers (who calls this function)
- Downstream dependencies (what this function calls)
- Depth in call graph

**Module Dependencies:**
- Module coupling metrics
- Circular dependency warnings
- Cross-module calls

**Git History Insights:**
- Change frequency
- Recent modifications
- Contributing authors
- Historical complexity trend

### Page 5: Data Flow

Shows detailed data flow analysis for understanding mutations, I/O operations, and variable escape behavior.

**Mutation Analysis:**
- **Total Mutations:** Count of all variable mutations in the function
- **Live Mutations:** Variables that are mutated and their new values are used
- **Dead Stores:** Variables that are assigned but never read (optimization opportunity)

Example display:
```
Total Mutations:     5
Live Mutations:      2
Dead Stores:         1

Live Mutations:
  • counter
  • state

Dead Stores:
  • temp (never read)
```

**I/O Operations:**
- Detected I/O operations by type (File, Network, Database, etc.)
- Line numbers where I/O occurs
- Variables involved in I/O

Example display:
```
File Read at line 105 (variables: file)
Network Call at line 110 (variables: socket)
Database Query at line 120 (variables: db)
```

**Escape Analysis:**
- **Escaping Variables:** Variables whose values escape the function scope
- **Return Dependencies:** Variables that affect the return value

Example display:
```
Escaping Variables:   2

Variables affecting return value:
  • result
  • accumulator
```

**Purity Analysis:**
- **Is Pure:** Whether the function is pure (no side effects, deterministic)
- **Confidence:** Confidence level of the purity assessment (0-100%)
- **Impurity Reasons:** Specific reasons why the function is not pure

Example display:
```
Is Pure:        No
Confidence:     95.0%

Impurity Reasons:
  • Mutates shared state
  • Performs I/O operations
  • Calls impure functions
```

**Navigation in Data Flow Page:**
- Use `←/→` keys to switch between detail pages
- All data flow insights help identify refactoring opportunities
- Pure functions are easier to test and reason about
- High mutation counts suggest opportunities for functional refactoring

## Page Navigation

**In Detail View:**
- `←/→` or `h/l` - Switch between detail pages (1-5)
- `Esc` or `q` - Return to list view
- `?` - Show help

**Page Indicator:**
The bottom of the screen shows which page you're viewing:
```
[1/5] Overview   [2/5] Metrics   [3/5] Recommendations   [4/5] Context   [5/5] Data Flow
```

## Filtering and Sorting

The TUI supports filtering results by various criteria:

### Filter by Debt Type

```bash
# Only show complexity issues
debtmap results results.json --filter complexity

# Only show side effect issues
debtmap results results.json --filter side-effects
```

### Sort Options

Results are pre-sorted by priority score, but you can change the sort order:

```bash
# Sort by cyclomatic complexity
debtmap results results.json --sort complexity

# Sort by lines of code
debtmap results results.json --sort loc

# Sort by file name
debtmap results results.json --sort file
```

## Theme Customization

The TUI supports custom color themes for better readability:

```bash
# Use light theme
debtmap results results.json --theme light

# Use dark theme (default)
debtmap results results.json --theme dark

# Use high-contrast theme
debtmap results results.json --theme contrast
```

## Export from TUI

While viewing results in the TUI, you can export specific items:

- `e` - Export current item to markdown
- `E` - Export all visible items to markdown

The exported markdown includes all detail page information in a readable format.

## Tips and Best Practices

1. **Use Data Flow Page:** The data flow page (Page 5) provides deep insights into mutations and I/O operations, helping identify refactoring opportunities for functional programming patterns.

2. **Focus on Top Items:** Start with the highest-priority items (top 5-10) for maximum impact.

3. **Review Recommendations:** Always check Page 3 (Recommendations) for specific refactoring steps before making changes.

4. **Check Context:** Use Page 4 (Context) to understand the function's role before refactoring to avoid breaking critical paths.

5. **Understand Purity:** Pure functions (shown in Data Flow page) are easier to test and maintain. High mutation counts indicate opportunities for functional refactoring.

6. **Look for Dead Stores:** Variables in the "Dead Stores" section can be removed to simplify code.

7. **Identify I/O Boundaries:** Functions with many I/O operations should isolate I/O from business logic.

8. **Track Git History:** Frequently changing functions (shown in Context page) may indicate design issues.

## Troubleshooting

### TUI Won't Launch

Ensure you have a valid results file:
```bash
# Check file exists and is valid JSON
cat results.json | jq .
```

### Colors Not Showing

Some terminals don't support 256-color mode. Try:
```bash
# Use basic colors
export TERM=xterm-color
debtmap results results.json
```

### Navigation Keys Not Working

Ensure your terminal supports the required key codes. Try alternative keys:
- Use `j/k` instead of arrow keys for up/down
- Use `h/l` instead of arrow keys for left/right

## Integration with Workflows

### CI/CD Integration

Generate results in CI and review locally:

```bash
# In CI pipeline
debtmap analyze . -o ci-results.json --format json

# Download and review locally
scp ci-server:ci-results.json .
debtmap results ci-results.json
```

### Team Review

Share results files with team members:

```bash
# Generate results
debtmap analyze . -o team-review.json --format json

# Commit to repository (small file)
git add team-review.json
git commit -m "Add debtmap analysis results for review"

# Team members can view
debtmap results team-review.json
```

### Progressive Refactoring

Track progress across refactoring sessions:

```bash
# Initial analysis
debtmap analyze . -o before.json --format json

# After refactoring
debtmap analyze . -o after.json --format json

# Compare results
debtmap compare before.json after.json
```

## Advanced Features

### Custom Data Retention

Control how much detail is stored in results:

```bash
# Minimal results (scores only)
debtmap analyze . -o minimal.json --detail-level minimal

# Full results (all metrics and context)
debtmap analyze . -o full.json --detail-level full
```

### Performance Optimization

For large codebases, optimize TUI performance:

```bash
# Limit results to top N items
debtmap analyze . -o top100.json --top 100

# View subset
debtmap results top100.json
```

## See Also

- [Output Formats](output-formats.md) - Other output format options
- [CLI Reference](cli-reference.md) - Complete command reference
- [Configuration](configuration.md) - Customization options
