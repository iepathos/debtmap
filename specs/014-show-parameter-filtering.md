# Specification 014: Show Parameter with Flexible Filtering

## Overview
Add a `--show` parameter to the `debtmap analyze` command that allows users to view filtered subsets of analysis results beyond the default top 5 items per category. This parameter will support flexible filtering syntax to drill down into different sections with custom limits.

## Motivation
Currently, debtmap only shows the top 5 items in each category (testing recommendations, complexity hotspots, critical risks). Users need a way to:
1. View all items in a specific category
2. Set custom limits for each category
3. Filter by specific metrics or thresholds
4. Focus analysis on particular areas of concern

## Design

### Command Line Interface

#### Basic Syntax
```bash
debtmap analyze . --show <filter_spec>
```

#### Filter Specification Format
The filter specification is a comma-separated list of category filters:
```
category:limit[,category:limit,...]
```

Where:
- `category` is one of: `roi`, `complexity`, `risk`, `coverage`, `debt`
- `limit` is either:
  - `all` - show all items
  - A number (e.g., `20`) - show top N items
  - A threshold expression (e.g., `>=5`, `>10`) - show items matching threshold

#### Examples
```bash
# Show all functions with ROI >= 5
debtmap analyze . --show roi:>=5

# Show top 20 ROI items
debtmap analyze . --show roi:20

# Show all ROI items
debtmap analyze . --show roi:all

# Multiple filters: top 20 ROI and top 10 complexity
debtmap analyze . --show roi:20,complexity:10

# Threshold-based filtering
debtmap analyze . --show roi:>=5,complexity:>15,risk:>=8

# Mix of styles
debtmap analyze . --show roi:all,complexity:10,coverage:<30
```

### Categories and Their Meanings

#### 1. ROI (Return on Investment)
- **Field**: `roi` from TestingRecommendation
- **Default Sort**: Descending by ROI score
- **Threshold**: Numeric comparison on ROI value
- **Output**: Testing recommendations with ROI scores

#### 2. Complexity
- **Fields**: `cyclomatic_complexity`, `cognitive_complexity`
- **Default Sort**: Descending by sum of complexities
- **Threshold**: Comparison on either or sum
- **Output**: Functions sorted by complexity

#### 3. Risk
- **Field**: `risk_score` from FunctionRisk
- **Default Sort**: Descending by risk score
- **Threshold**: Numeric comparison on risk score
- **Output**: Functions with risk analysis

#### 4. Coverage
- **Field**: `coverage_percentage` from FunctionRisk
- **Default Sort**: Ascending (lowest coverage first)
- **Threshold**: Percentage comparison
- **Output**: Functions with coverage data

#### 5. Debt
- **Field**: `debt_score` from DebtItem priority
- **Default Sort**: Descending by priority then score
- **Threshold**: By priority level or score
- **Output**: Technical debt items

### Implementation Details

#### 1. Parser Updates (`src/cli.rs`)
```rust
#[derive(Debug, Clone)]
pub struct ShowFilter {
    pub category: FilterCategory,
    pub limit: FilterLimit,
}

#[derive(Debug, Clone)]
pub enum FilterCategory {
    Roi,
    Complexity,
    Risk,
    Coverage,
    Debt,
}

#[derive(Debug, Clone)]
pub enum FilterLimit {
    All,
    Top(usize),
    Threshold(ThresholdOp, f64),
}

#[derive(Debug, Clone)]
pub enum ThresholdOp {
    GreaterThan,
    GreaterOrEqual,
    LessThan,
    LessOrEqual,
    Equal,
}

impl FromStr for ShowFilter {
    // Parse "roi:20", "complexity:>=15", etc.
}
```

Add to Analyze command:
```rust
/// Filter and show specific categories
#[arg(long, value_parser = parse_show_filters)]
show: Option<Vec<ShowFilter>>,
```

#### 2. Filter Application (`src/risk/insights.rs`)
Create new module for filter application:
```rust
pub fn apply_show_filters(
    recommendations: &Vector<TestingRecommendation>,
    risks: &Vector<FunctionRisk>,
    debt_items: &[DebtItem],
    filters: &[ShowFilter],
) -> FilteredResults {
    // Apply each filter and collect results
}

pub struct FilteredResults {
    pub roi_items: Option<Vec<TestingRecommendation>>,
    pub complexity_items: Option<Vec<FunctionRisk>>,
    pub risk_items: Option<Vec<FunctionRisk>>,
    pub coverage_items: Option<Vec<FunctionRisk>>,
    pub debt_items: Option<Vec<DebtItem>>,
}
```

#### 3. Output Formatting Updates
Modify output formatting to handle variable-length lists:
```rust
pub fn format_filtered_results(results: &FilteredResults, filters: &[ShowFilter]) -> String {
    // Format each category based on what was requested
    // Include headers indicating filter applied
}
```

### Output Format Examples

#### Example 1: `--show roi:all`
```
🎯 ALL TESTING RECOMMENDATIONS (ROI-based)
────────────────────────────────────────────────────────────────────────────────
Showing: All items with ROI score

Priority | Function                       | Location                      | ROI
---------|--------------------------------|-------------------------------|------
#1       | process_payment()              | src/payment/processor.rs      | 12.5
#2       | validate_input()               | src/validation/core.rs        | 10.2
#3       | calculate_discount()           | src/pricing/discount.rs       | 8.7
... (continues for all items)
```

#### Example 2: `--show roi:>=5,complexity:>20`
```
🎯 TESTING RECOMMENDATIONS (ROI >= 5)
────────────────────────────────────────────────────────────────────────────────
Found 8 functions with ROI >= 5

Priority | Function                       | Location                      | ROI
---------|--------------------------------|-------------------------------|------
#1       | process_payment()              | src/payment/processor.rs      | 12.5
... (8 items total)

🔥 COMPLEXITY HOTSPOTS (Complexity > 20)
────────────────────────────────────────────────────────────────────────────────
Found 3 functions with complexity > 20

Function                  | Cyclomatic | Cognitive | Total | Location
--------------------------|------------|-----------|--------|------------------
parse_configuration()     | 15         | 18        | 33     | src/config/parser.rs
... (3 items total)
```

### Special Behaviors

#### 1. Default Behavior
When `--show` is not specified, maintain current behavior (top 5 per category).

#### 2. All Filter
`--show all` (without category) shows all items in all categories.

#### 3. Pagination Hints
For very large result sets, add pagination hints:
```
Showing 100 of 245 total items. Use --show roi:all to see all.
```

#### 4. Empty Results
When filters produce no results:
```
No functions found matching: complexity:>50
Try: --show complexity:>20 or --show complexity:all
```

### Testing Strategy

#### Unit Tests
1. Filter parsing tests
2. Threshold evaluation tests
3. Sorting and limiting tests
4. Output formatting tests

#### Integration Tests
1. Test with real codebase analysis
2. Verify filter combinations
3. Performance with large result sets
4. Edge cases (empty results, invalid filters)

### Migration and Compatibility

This is a purely additive feature:
- No breaking changes to existing CLI
- Default behavior unchanged
- Can be combined with existing options

### Performance Considerations

1. **Lazy Evaluation**: Only compute categories requested
2. **Streaming**: For large result sets, consider streaming output
3. **Caching**: Cache computed metrics for multiple filter passes
4. **Memory**: Limit in-memory results for "all" queries on large codebases

### Future Extensions

1. **Complex Filters**:
   ```bash
   --show "roi:>=5 AND complexity:>10"
   ```

2. **Output Formats**:
   ```bash
   --show roi:all --format csv
   ```

3. **Save Filters**:
   ```bash
   --show-preset high-risk  # Predefined filter sets
   ```

4. **Interactive Mode**:
   ```bash
   debtmap analyze . --interactive
   > show roi:10
   > show complexity:all where coverage < 50
   ```

### Example Implementation Timeline

1. **Phase 1**: Basic filter parsing and single-category filtering
2. **Phase 2**: Multi-category filtering and threshold operators
3. **Phase 3**: Output formatting and pagination
4. **Phase 4**: Performance optimizations and caching

### Command Help Text

```
--show <FILTERS>
    Filter and display specific analysis results.
    
    Format: category:limit[,category:limit,...]
    
    Categories:
      roi        - Testing recommendations by return on investment
      complexity - Functions by complexity metrics
      risk       - Functions by risk score
      coverage   - Functions by coverage percentage
      debt       - Technical debt items
    
    Limits:
      all        - Show all items
      N          - Show top N items (e.g., 20)
      >=N        - Show items >= threshold (e.g., >=5)
      >N         - Show items > threshold
      <N         - Show items < threshold
      <=N        - Show items <= threshold
    
    Examples:
      --show roi:all                    Show all ROI items
      --show roi:20                     Show top 20 ROI items
      --show roi:>=5                    Show items with ROI >= 5
      --show roi:20,complexity:10       Multiple filters
      --show complexity:>15,coverage:<30 Threshold combinations
```

## Success Criteria

1. Users can view more than 5 items per category
2. Flexible filtering allows focusing on specific metrics
3. Output clearly indicates what filters were applied
4. Performance remains acceptable for large codebases
5. Help text and error messages guide users effectively

## Dependencies

- No external dependencies required
- Uses existing debtmap analysis infrastructure
- Builds on current risk and insight modules