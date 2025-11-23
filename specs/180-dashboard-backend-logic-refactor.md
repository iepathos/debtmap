---
number: 180
title: Dashboard Backend Logic Refactor - Move JavaScript to Rust
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-11-23
---

# Specification 180: Dashboard Backend Logic Refactor - Move JavaScript to Rust

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The current HTML dashboard (`src/io/writers/templates/dashboard.html`) contains **1,400+ lines of JavaScript** performing extensive business logic, data transformation, and calculations that belong in the Rust backend. This violates the separation of concerns principle and creates several problems:

### Current Architecture Issues

1. **Heavy Frontend Logic**:
   - Lines 360-388: Total debt score calculation with format detection
   - Lines 390-533: Root cause categorization and extraction (text pattern matching)
   - Lines 534-653: Complexity data extraction and priority categorization
   - Lines 693-978: Recommendation grouping, normalization, and ranking algorithm
   - Lines 1301-1448: Distribution histogram calculations
   - Lines 1531-1703: Table population with data transformation

2. **Dual Format Handling in JavaScript**:
   - Lines 362-421 detect and handle both "unified" and "legacy" data formats
   - This logic is repeated throughout multiple functions
   - Format conversion happens client-side instead of server-side

3. **Business Logic in Template**:
   - Priority ranking algorithm (lines 743-752)
   - Root cause text analysis with pattern matching (lines 471-514)
   - Recommendation normalization (lines 693-717)
   - Entropy color classification thresholds (lines 1087-1093)
   - Complexity categorization thresholds (lines 569-577)

4. **Poor Separation of Concerns**:
   - Presentation (HTML/CSS) tightly coupled with data transformation
   - Chart configurations built client-side from raw data
   - Table HTML generation in JavaScript instead of backend
   - Difficult to test business logic (not in type-safe Rust)

5. **Performance Issues**:
   - Client-side processing of potentially large datasets
   - Repeated calculations on every page load
   - No opportunity for server-side caching

### Alignment with Project Principles

From `CLAUDE.md`:
> **Pure core, imperative shell** - Business logic pure, I/O at edges
> **Functional over imperative** - Prefer transformations over mutations

The current implementation violates these principles by mixing imperative JavaScript transformations with presentation logic.

## Objective

Refactor the HTML dashboard to move all business logic, data transformation, and calculations from JavaScript to Rust backend, resulting in:

1. **Lightweight template** with minimal logic (< 300 lines total, < 50 lines of JavaScript)
2. **Type-safe Rust backend** performing all data processing and transformations
3. **Clear separation of concerns**: Rust for logic, template for presentation, minimal JavaScript for UI interactions only
4. **Single format output** - backend handles all format conversion
5. **Pre-computed values** - all metrics, groupings, and configurations calculated server-side
6. **Testable pure functions** - all logic in Rust with comprehensive unit tests

## Requirements

### Functional Requirements

#### FR1: Backend Data Structures
- Create comprehensive Rust data structures for all dashboard data
- Include pre-computed metrics, chart configurations, and table data
- Support all current dashboard visualizations and features
- Provide serialization for template rendering

#### FR2: Recommendation Processing
- Move recommendation grouping logic to Rust
- Implement normalization algorithm (lines 693-717) in pure Rust functions
- Calculate priority rankings using highest individual priority (lines 743-752)
- Pre-compute aggregated impact metrics (complexity/risk reduction)
- Generate HTML for recommendation cards server-side

#### FR3: Root Cause Categorization
- Implement root cause text analysis in Rust (lines 471-514 patterns)
- Create pure functions for pattern matching and categorization
- Pre-compute root cause distribution for charts
- Generate chart configuration in Rust

#### FR4: Complexity Data Processing
- Extract complexity data transformation to Rust functions
- Pre-categorize functions by priority thresholds (lines 569-577)
- Calculate entropy color classifications (lines 1087-1093)
- Generate scatter plot data points server-side
- Build Chart.js configurations in Rust

#### FR5: Table Generation
- Pre-render complex function table rows in Rust with proper HTML escaping
- Calculate all sortable values server-side
- Apply priority badge CSS classes in backend
- Generate god objects table if applicable

#### FR6: Chart Configuration
- Generate complete Chart.js configuration objects in Rust
- Include datasets, colors, labels, and options
- Support all current chart types: scatter, bar, histogram
- Provide JSON-serializable chart configs

#### FR7: Metric Calculations
- Move all percentage calculations to Rust (lines 348-357)
- Calculate total debt scores server-side (lines 360-388)
- Compute distribution statistics in backend
- Pre-format all display values

### Non-Functional Requirements

#### NFR1: Performance
- All data processing must complete in < 100ms for typical projects
- Reduce client-side JavaScript execution time by > 90%
- Enable server-side caching of pre-computed dashboard data
- Minimize memory allocations through efficient data structures

#### NFR2: Code Quality
- All backend logic must be pure functions where possible
- Comprehensive unit tests for all transformation functions (aim for 85%+ coverage)
- Follow functional programming principles from CLAUDE.md
- Functions under 20 lines with single responsibility

#### NFR3: Maintainability
- Clear separation: Rust for logic, template for presentation
- Type-safe data structures prevent runtime errors
- Self-documenting code with clear function names
- Template easily customizable without touching business logic

#### NFR4: Compatibility
- Maintain all current dashboard features and visualizations
- No breaking changes to dashboard appearance or functionality
- Support graceful degradation if JavaScript disabled
- Work with existing AnalysisResults and UnifiedAnalysis structures

#### NFR5: Testability
- All business logic testable through Rust unit tests
- Mock data structures for template testing
- Integration tests for full dashboard generation
- Property-based tests for data transformations where appropriate

## Acceptance Criteria

- [ ] **AC1: New Module Structure**
  - `src/io/writers/dashboard/mod.rs` module created
  - `dashboard/data.rs` contains all data structures
  - `dashboard/charts.rs` handles chart configuration generation
  - `dashboard/tables.rs` handles table HTML generation
  - `dashboard/recommendations.rs` handles recommendation processing
  - `dashboard/metrics.rs` handles metric calculations

- [ ] **AC2: Template Engine Integration**
  - Tera templating engine integrated (or Handlebars as alternative)
  - Template file reduced to < 300 lines
  - All template variables properly escaped
  - Type-safe template rendering with compile-time checks

- [ ] **AC3: JavaScript Reduction**
  - JavaScript code reduced from 1,400+ lines to < 200 lines
  - Remaining JavaScript only for UI interactions: toggle, filter, sort
  - No data transformation or business logic in JavaScript
  - No format detection or conversion in client-side code

- [ ] **AC4: Recommendation Processing**
  - Recommendation grouping implemented in Rust
  - Normalization algorithm converted to pure Rust function
  - Priority ranking uses highest individual priority
  - Top 10 recommendations pre-computed and sorted
  - HTML for recommendation cards generated server-side
  - Rationale extraction implemented in Rust

- [ ] **AC5: Root Cause Analysis**
  - Root cause categorization patterns implemented in Rust
  - 15+ pattern categories supported (as in lines 476-510)
  - Distribution calculated server-side
  - Chart configuration generated in Rust

- [ ] **AC6: Complexity Processing**
  - Complexity scatter plot data pre-computed
  - Priority thresholds (critical: 20+, high: 15+, medium: 10+) in Rust
  - Entropy-adjusted complexity handling in backend
  - All distribution calculations server-side

- [ ] **AC7: Table Generation**
  - Complex functions table HTML pre-rendered in Rust
  - All 50 rows generated with proper escaping
  - Sortable data attributes included
  - Priority badges and color classes applied server-side
  - God objects table conditionally rendered

- [ ] **AC8: Chart Configurations**
  - Complete Chart.js config objects generated in Rust
  - Complexity scatter chart config with 4 priority datasets
  - Root causes bar chart with dynamic colors
  - Entropy distribution histogram
  - Adjusted complexity comparison chart
  - Recommendations horizontal bar chart

- [ ] **AC9: Metric Pre-computation**
  - All percentages calculated in Rust
  - Total debt score computed server-side
  - Debt density formatted in backend
  - Total LOC calculated from metrics

- [ ] **AC10: Testing**
  - Unit tests for all new Rust functions (85%+ coverage)
  - Tests for recommendation grouping algorithm
  - Tests for root cause categorization
  - Tests for chart configuration generation
  - Tests for HTML escaping and sanitization
  - Integration test for complete dashboard generation

- [ ] **AC11: No Regressions**
  - All current dashboard features still work
  - Charts render identically to current implementation
  - Tables display same data with same formatting
  - Top 10 recommendations show same results
  - Filtering and sorting still functional

- [ ] **AC12: Documentation**
  - Module-level documentation for dashboard package
  - Function documentation for all public APIs
  - Examples in doc comments for key transformations
  - Architecture documentation updated in ARCHITECTURE.md

## Technical Details

### Implementation Approach

#### Phase 1: Create Data Structures (Foundation)

Create `src/io/writers/dashboard/data.rs`:

```rust
use serde::Serialize;
use std::collections::HashMap;

/// Complete dashboard data ready for template rendering
#[derive(Serialize, Debug)]
pub struct DashboardData {
    pub metadata: DashboardMetadata,
    pub metrics: DashboardMetrics,
    pub percentages: PercentageMetrics,
    pub charts: ChartConfigs,
    pub tables: TableData,
    pub recommendations: Vec<RecommendationGroup>,
}

#[derive(Serialize, Debug)]
pub struct DashboardMetadata {
    pub project_name: String,
    pub timestamp: String,
    pub total_items: usize,
    pub total_functions: usize,
}

#[derive(Serialize, Debug)]
pub struct DashboardMetrics {
    pub total_items: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub total_functions: usize,
    pub average_complexity: f64,
    pub debt_density: String,  // Pre-formatted
    pub total_debt_score: String,  // Pre-formatted with commas
    pub total_loc: String,  // Pre-formatted with commas
}

#[derive(Serialize, Debug)]
pub struct PercentageMetrics {
    pub critical_pct: String,  // e.g., "15.3% of items"
    pub high_pct: String,
    pub medium_pct: String,
    pub low_pct: String,
}

#[derive(Serialize, Debug)]
pub struct ChartConfigs {
    pub complexity_scatter: ChartJsConfig,
    pub adjusted_complexity_scatter: Option<ChartJsConfig>,
    pub root_causes: ChartJsConfig,
    pub recommendations: ChartJsConfig,
    pub entropy_distribution: ChartJsConfig,
    pub adjusted_complexity_distribution: Option<ChartJsConfig>,
}

#[derive(Serialize, Debug)]
pub struct ChartJsConfig {
    #[serde(rename = "type")]
    pub chart_type: String,
    pub data: ChartData,
    pub options: ChartOptions,
}

#[derive(Serialize, Debug)]
pub struct ChartData {
    pub labels: Option<Vec<String>>,
    pub datasets: Vec<ChartDataset>,
}

#[derive(Serialize, Debug)]
pub struct ChartDataset {
    pub label: String,
    pub data: Vec<ChartDataPoint>,
    #[serde(rename = "backgroundColor")]
    pub background_color: ChartColor,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "borderColor")]
    pub border_color: Option<ChartColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "borderWidth")]
    pub border_width: Option<u32>,
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum ChartDataPoint {
    Scalar(f64),
    Point { x: f64, y: f64, name: String },
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum ChartColor {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Serialize, Debug)]
pub struct RecommendationGroup {
    pub index: usize,
    pub title: String,
    pub priority: String,
    pub priority_class: String,
    pub total_score: String,  // Pre-formatted
    pub avg_score: String,  // Pre-formatted
    pub count: usize,
    pub rationale: Option<String>,
    pub impact_html: Option<String>,  // Pre-rendered impact section
    pub items_html: String,  // Pre-rendered items HTML
}

#[derive(Serialize, Debug)]
pub struct TableData {
    pub complex_functions_rows: Vec<String>,  // Pre-rendered HTML rows
    pub god_objects_rows: Vec<String>,  // Pre-rendered HTML rows
    pub show_god_objects: bool,
    pub show_adjusted_complexity: bool,
}
```

#### Phase 2: Implement Recommendation Processing

Create `src/io/writers/dashboard/recommendations.rs`:

```rust
use crate::core::{DebtItem, Priority};

/// Normalize recommendation text for grouping
pub fn normalize_recommendation(recommendation: &str) -> String {
    let lower = recommendation.to_lowercase();

    // Match patterns from lines 700-716 of current template
    if lower.contains("split") && lower.contains("focused")
        && lower.contains("function") && lower.contains("decision") {
        "Split into focused functions by decision clusters".to_string()
    } else if lower.contains("add tests") || lower.contains("test coverage") {
        "Add tests".to_string()
    } else if lower.contains("extract") && lower.contains("function") {
        "Extract helper functions".to_string()
    } else if lower.contains("reduce nesting") {
        "Reduce nesting depth".to_string()
    } else if lower.contains("god object") {
        "Refactor god object".to_string()
    } else if lower.contains("standardize") {
        "Standardize control flow patterns".to_string()
    } else {
        // Use first 50 chars as-is
        recommendation.chars().take(50).collect()
    }
}

/// Group debt items by normalized recommendation
pub fn group_recommendations(items: &[DebtItem]) -> Vec<RecommendationGroup> {
    use std::collections::HashMap;

    let mut groups: HashMap<String, Vec<&DebtItem>> = HashMap::new();

    for item in items {
        let normalized = normalize_recommendation(
            item.recommendation.as_ref()
                .and_then(|r| r.action.as_ref())
                .unwrap_or(&item.message)
        );
        groups.entry(normalized).or_default().push(item);
    }

    // Convert to vec and sort by priority rank then total score
    let mut grouped: Vec<_> = groups
        .into_iter()
        .map(|(title, items)| build_recommendation_group(title, items))
        .collect();

    grouped.sort_by(|a, b| {
        b.priority_rank.cmp(&a.priority_rank)
            .then(b.total_score.partial_cmp(&a.total_score).unwrap())
    });

    grouped.into_iter().take(10).collect()
}

/// Calculate priority rank for sorting (lines 743-752)
fn get_priority_rank(priority: &Priority) -> u8 {
    match priority {
        Priority::Critical => 4,
        Priority::High => 3,
        Priority::Medium => 2,
        Priority::Low => 1,
    }
}

/// Build a recommendation group with all computed values
fn build_recommendation_group(
    title: String,
    items: Vec<&DebtItem>
) -> RecommendationGroupInternal {
    let total_score: f64 = items.iter()
        .map(|item| item.unified_score.as_ref()
            .map(|s| s.final_score)
            .unwrap_or(0.0))
        .sum();

    let avg_score = total_score / items.len() as f64;

    // Use highest individual priority (from first item since sorted by score)
    let highest_priority = items[0].priority.clone();

    RecommendationGroupInternal {
        title,
        items,
        total_score,
        avg_score,
        priority: highest_priority.clone(),
        priority_rank: get_priority_rank(&highest_priority),
    }
}
```

#### Phase 3: Implement Root Cause Categorization

Create `src/io/writers/dashboard/metrics.rs`:

```rust
/// Categorize root cause from recommendation text (lines 471-514)
pub fn categorize_root_cause(text: &str) -> &'static str {
    let lower = text.to_lowercase();

    if lower.contains("nesting") || lower.contains("reduce nesting") {
        "Deep nesting"
    } else if lower.contains("split") {
        "Function/file too large"
    } else if lower.contains("extract") {
        "Needs extraction"
    } else if lower.contains("reduce complexity") {
        "High complexity"
    } else if lower.contains("god object") {
        "God object"
    } else if lower.contains("god module") {
        "God module"
    } else if lower.contains("standardize") {
        "Inconsistent patterns"
    } else if lower.contains("dispatcher") || lower.contains("state transitions") {
        "Complex control flow"
    } else if lower.contains("duplication") || lower.contains("duplicate") {
        "Code duplication"
    } else if lower.contains("testing") || lower.contains("coverage") {
        "Testing gaps"
    } else if lower.contains("decision clusters") || lower.contains("focused functions") {
        "Multiple responsibilities"
    } else if lower.contains("urgent") {
        "Critical size violation"
    } else if lower.contains("unbounded growth") {
        "Unbounded collections"
    } else if lower.contains(".unwrap()") || lower.contains(".expect(") {
        "Unsafe unwrap/expect"
    } else if lower.contains(".ok()") || lower.contains("discarding error") {
        "Error information discarded"
    } else if lower.contains("todo") || lower.contains("fixme") {
        "TODOs/FIXMEs"
    } else if lower.contains("bug:") {
        "Known bugs"
    } else {
        "Other"
    }
}

/// Extract root causes and their counts
pub fn extract_root_causes(items: &[DebtItem]) -> (Vec<String>, Vec<usize>) {
    use std::collections::HashMap;

    let mut causes: HashMap<&'static str, usize> = HashMap::new();

    for item in items {
        let text = item.recommendation.as_ref()
            .and_then(|r| r.action.as_ref())
            .unwrap_or(&item.message);

        let cause = categorize_root_cause(text);
        *causes.entry(cause).or_insert(0) += 1;
    }

    // Sort by count, take top 10
    let mut sorted: Vec<_> = causes.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted.truncate(10);

    let labels = sorted.iter().map(|(k, _)| k.to_string()).collect();
    let counts = sorted.iter().map(|(_, v)| *v).collect();

    (labels, counts)
}
```

#### Phase 4: Implement Chart Generation

Create `src/io/writers/dashboard/charts.rs`:

```rust
use crate::io::writers::dashboard::data::*;
use crate::core::FunctionMetrics;

/// Generate complexity scatter plot configuration
pub fn build_complexity_scatter(functions: &[FunctionMetrics]) -> ChartJsConfig {
    let mut critical = Vec::new();
    let mut high = Vec::new();
    let mut medium = Vec::new();
    let mut low = Vec::new();

    for func in functions {
        let point = ChartDataPoint::Point {
            x: func.cyclomatic as f64,
            y: func.cognitive as f64,
            name: func.name.clone(),
        };

        // Thresholds from lines 569-577
        if func.cyclomatic >= 20 || func.cognitive >= 50 {
            critical.push(point);
        } else if func.cyclomatic >= 15 || func.cognitive >= 30 {
            high.push(point);
        } else if func.cyclomatic >= 10 || func.cognitive >= 20 {
            medium.push(point);
        } else {
            low.push(point);
        }
    }

    ChartJsConfig {
        chart_type: "scatter".to_string(),
        data: ChartData {
            labels: None,
            datasets: vec![
                ChartDataset {
                    label: "Critical".to_string(),
                    data: critical,
                    background_color: ChartColor::Single("#EF4444".to_string()),
                    border_color: None,
                    border_width: None,
                },
                ChartDataset {
                    label: "High".to_string(),
                    data: high,
                    background_color: ChartColor::Single("#F59E0B".to_string()),
                    border_color: None,
                    border_width: None,
                },
                ChartDataset {
                    label: "Medium".to_string(),
                    data: medium,
                    background_color: ChartColor::Single("#FBBF24".to_string()),
                    border_color: None,
                    border_width: None,
                },
                ChartDataset {
                    label: "Low".to_string(),
                    data: low,
                    background_color: ChartColor::Single("#10B981".to_string()),
                    border_color: None,
                    border_width: None,
                },
            ],
        },
        options: build_scatter_options(
            "Cyclomatic Complexity",
            "Cognitive Complexity"
        ),
    }
}

/// Generate distinct colors for categories (lines 423-461)
pub fn generate_distinct_colors(count: usize) -> Vec<String> {
    let predefined = vec![
        "#EF4444", "#3B82F6", "#F59E0B", "#10B981", "#8B5CF6",
        "#EAB308", "#EC4899", "#14B8A6", "#F97316", "#6366F1",
        "#84CC16", "#06B6D4", "#D946EF", "#F43F5E", "#A855F7",
    ];

    if count <= predefined.len() {
        predefined.into_iter()
            .take(count)
            .map(String::from)
            .collect()
    } else {
        // Generate additional colors using HSL
        let mut colors = predefined.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let hue_step = 360.0 / (count - predefined.len()) as f64;

        for i in predefined.len()..count {
            let hue = (i - predefined.len()) as f64 * hue_step;
            let sat = 65 + (i % 3) * 10;
            let light = 50 + (i % 2) * 5;
            colors.push(format!("hsl({}, {}%, {}%)", hue, sat, light));
        }

        colors
    }
}
```

#### Phase 5: Implement Table Generation

Create `src/io/writers/dashboard/tables.rs`:

```rust
use crate::core::FunctionMetrics;
use html_escape::encode_text;

/// Generate HTML row for complex function table
pub fn render_function_row(func: &FunctionMetrics) -> String {
    let entropy_display = func.entropy_score
        .map(|e| format!("{:.2}", e))
        .unwrap_or_else(|| "N/A".to_string());

    let entropy_color = get_entropy_color_class(func.entropy_score);

    let dampening = func.adjusted_complexity
        .map(|adj| format!("{:.2}", adj / func.cyclomatic as f64))
        .unwrap_or_else(|| "N/A".to_string());

    let priority_badge = get_priority_badge_class(func);
    let priority_label = get_priority_label(func);

    format!(
        r#"<tr>
            <td class="px-6 py-4 text-sm font-mono">{}</td>
            <td class="px-6 py-4 text-sm text-gray-600">{}</td>
            <td class="px-6 py-4 text-sm">{}</td>
            <td class="px-6 py-4 text-sm font-semibold {}">{}</td>
            <td class="px-6 py-4 text-sm">{}</td>
            <td class="px-6 py-4 text-sm {}">{}</td>
            <td class="px-6 py-4 text-sm text-gray-600">{}</td>
            <td class="px-6 py-4"><span class="px-2 py-1 rounded text-xs {}">{}</span></td>
        </tr>"#,
        encode_text(&func.name),
        encode_text(&func.file.display().to_string()),
        func.cyclomatic,
        get_cognitive_color_class(func.cognitive), func.cognitive,
        func.nesting,
        entropy_color, entropy_display,
        dampening,
        priority_badge, priority_label
    )
}

/// Get entropy color class (lines 1087-1093)
fn get_entropy_color_class(entropy: Option<f64>) -> &'static str {
    match entropy {
        Some(e) if e >= 0.6 => "text-red-600",
        Some(e) if e >= 0.4 => "text-orange-600",
        Some(e) if e >= 0.3 => "text-yellow-600",
        Some(_) => "text-green-600",
        None => "text-gray-400",
    }
}

/// Get cognitive complexity color class (lines 1667-1672)
fn get_cognitive_color_class(cognitive: u32) -> &'static str {
    if cognitive >= 50 { "text-red-600" }
    else if cognitive >= 30 { "text-orange-600" }
    else if cognitive >= 20 { "text-yellow-600" }
    else { "text-green-600" }
}

/// Get priority badge CSS class (lines 1674-1680)
fn get_priority_badge_class(func: &FunctionMetrics) -> &'static str {
    let max = func.cyclomatic.max(func.cognitive);
    if max >= 20 { "bg-red-100 text-red-800" }
    else if max >= 15 { "bg-orange-100 text-orange-800" }
    else if max >= 10 { "bg-yellow-100 text-yellow-800" }
    else { "bg-green-100 text-green-800" }
}

/// Get priority label (lines 1682-1688)
fn get_priority_label(func: &FunctionMetrics) -> &'static str {
    let max = func.cyclomatic.max(func.cognitive);
    if max >= 20 { "CRITICAL" }
    else if max >= 15 { "HIGH" }
    else if max >= 10 { "MEDIUM" }
    else { "LOW" }
}
```

#### Phase 6: Template Engine Integration

Modify `src/io/writers/html.rs`:

```rust
use tera::{Tera, Context};
use crate::io::writers::dashboard::DashboardDataBuilder;

impl<W: Write> HtmlWriter<W> {
    fn render_with_tera(&self, results: &AnalysisResults) -> Result<String> {
        // Build all dashboard data in Rust
        let dashboard_data = DashboardDataBuilder::new()
            .with_results(results)
            .with_unified_analysis(self.unified_analysis.as_ref())
            .build()?;

        // Initialize Tera with embedded template
        let mut tera = Tera::default();
        tera.add_raw_template(
            "dashboard",
            include_str!("templates/dashboard.html.tera")
        )?;

        // Create context and render
        let mut context = Context::new();
        context.insert("dashboard", &dashboard_data);

        Ok(tera.render("dashboard", &context)?)
    }
}
```

#### Phase 7: Simplified Template

New `src/io/writers/templates/dashboard.html.tera` (< 300 lines):

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Debtmap Dashboard - {{ dashboard.metadata.project_name }}</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.0/dist/chart.umd.min.js"></script>
    <script src="https://cdn.tailwindcss.com"></script>
    <!-- CSS stays the same -->
</head>
<body>
    <!-- Header -->
    <div class="header">
        <h1>{{ dashboard.metadata.project_name }}</h1>
        <span>{{ dashboard.metrics.total_items }} items</span>
        <span>Density: {{ dashboard.metrics.debt_density }}</span>
        <span>{{ dashboard.metadata.timestamp }}</span>
    </div>

    <!-- Metrics Cards -->
    <div class="grid">
        <div class="metric-card critical">
            <div class="text-sm">Critical Issues</div>
            <div class="text-3xl">{{ dashboard.metrics.critical_count }}</div>
            <div class="text-xs">{{ dashboard.percentages.critical_pct }}</div>
        </div>
        <!-- Repeat for high, medium, low -->
    </div>

    <!-- Top 10 Recommendations -->
    <div class="recommendations">
        {% for group in dashboard.recommendations %}
        <div class="recommendation-card {{ group.priority_class }}">
            <div onclick="toggleDetails({{ group.index }})">
                <span class="badge {{ group.priority_class }}">{{ group.priority }}</span>
                <span>{{ group.total_score }}</span>
                {% if group.count > 1 %}
                <span>{{ group.count }} items</span>
                {% endif %}
            </div>
            <div>{{ group.title }}</div>

            <div id="details-{{ group.index }}" class="details hidden">
                {% if group.rationale %}
                <div class="rationale">{{ group.rationale }}</div>
                {% endif %}
                {{ group.items_html | safe }}
            </div>
        </div>
        {% endfor %}
    </div>

    <!-- Charts -->
    <canvas id="complexityScatter"></canvas>
    <canvas id="rootCauses"></canvas>

    <!-- Tables -->
    <table id="complexFunctions">
        <thead>
            <tr>
                <th>Function</th>
                <th>File</th>
                <th>Cyclomatic</th>
                <th>Cognitive</th>
                <th>Nesting</th>
                <th>Entropy</th>
                <th>Dampening</th>
                <th>Priority</th>
            </tr>
        </thead>
        <tbody>
            {% for row in dashboard.tables.complex_functions_rows %}
            {{ row | safe }}
            {% endfor %}
        </tbody>
    </table>

    <script>
        // Chart rendering (< 50 lines)
        new Chart(document.getElementById('complexityScatter'),
            {{ dashboard.charts.complexity_scatter | json_encode() }}
        );

        new Chart(document.getElementById('rootCauses'),
            {{ dashboard.charts.root_causes | json_encode() }}
        );

        // UI interactions only (< 100 lines)
        function toggleDetails(index) {
            document.getElementById(`details-${index}`).classList.toggle('hidden');
        }

        // Client-side filtering
        document.getElementById('search').addEventListener('input', function(e) {
            const search = e.target.value.toLowerCase();
            document.querySelectorAll('#complexFunctions tbody tr').forEach(row => {
                row.style.display = row.textContent.toLowerCase().includes(search) ? '' : 'none';
            });
        });

        // Table sorting (operates on pre-rendered HTML)
        // ... minimal sorting logic ...
    </script>
</body>
</html>
```

### Architecture Changes

**New Module Structure**:
```
src/io/writers/
├── dashboard/
│   ├── mod.rs          # Public API, DashboardDataBuilder
│   ├── data.rs         # Data structures (DashboardData, etc.)
│   ├── charts.rs       # Chart configuration generation
│   ├── tables.rs       # Table HTML rendering
│   ├── recommendations.rs  # Recommendation processing
│   └── metrics.rs      # Metric calculations
├── templates/
│   └── dashboard.html.tera  # Simplified template
└── html.rs             # Modified to use dashboard module
```

**Data Flow**:
```
AnalysisResults → DashboardDataBuilder → DashboardData → Tera → HTML
                  (Pure Rust functions)   (Serializable)  (Template)
```

### Dependencies

**New Crate Dependencies**:
```toml
[dependencies]
tera = "1.19"           # Template engine
html-escape = "0.2"     # Already present
serde = { version = "1.0", features = ["derive"] }  # Already present
serde_json = "1.0"      # Already present
```

**Alternative**: Could use Handlebars instead of Tera:
```toml
handlebars = "5.1"
```

### Migration Path

1. **Create dashboard module** with empty data structures
2. **Implement one section at a time**:
   - Start with metrics (simplest)
   - Then recommendations (most complex)
   - Then charts
   - Finally tables
3. **Add Tera integration** alongside existing string replacement
4. **Test both implementations** side-by-side
5. **Switch to Tera** when feature-complete
6. **Remove old template** and string replacement code

## Testing Strategy

### Unit Tests

**Recommendation Processing** (`dashboard/recommendations.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_recommendation_focused_functions() {
        let input = "Split into 3 focused functions by decision clusters";
        assert_eq!(
            normalize_recommendation(input),
            "Split into focused functions by decision clusters"
        );
    }

    #[test]
    fn test_group_recommendations_by_priority() {
        let items = create_test_items();
        let groups = group_recommendations(&items);

        // First group should be highest priority
        assert_eq!(groups[0].priority, Priority::Critical);
        // Within same priority, higher total score first
        assert!(groups[0].total_score >= groups[1].total_score);
    }

    #[test]
    fn test_group_recommendations_takes_top_10() {
        let items = create_many_test_items(50);
        let groups = group_recommendations(&items);
        assert_eq!(groups.len(), 10);
    }
}
```

**Root Cause Categorization** (`dashboard/metrics.rs`):
```rust
#[test]
fn test_categorize_root_cause_patterns() {
    assert_eq!(categorize_root_cause("Reduce nesting depth"), "Deep nesting");
    assert_eq!(categorize_root_cause("Split large function"), "Function/file too large");
    assert_eq!(categorize_root_cause("god object detected"), "God object");
    assert_eq!(categorize_root_cause("Fix .unwrap() call"), "Unsafe unwrap/expect");
}

#[test]
fn test_extract_root_causes_counts() {
    let items = create_test_items_with_patterns();
    let (labels, counts) = extract_root_causes(&items);

    assert_eq!(labels.len(), counts.len());
    assert!(labels.len() <= 10);  // Top 10 only
    // Counts should be in descending order
    assert!(counts.windows(2).all(|w| w[0] >= w[1]));
}
```

**Chart Generation** (`dashboard/charts.rs`):
```rust
#[test]
fn test_complexity_scatter_categorization() {
    let functions = vec![
        create_func(25, 60),  // Critical
        create_func(18, 35),  // High
        create_func(12, 22),  // Medium
        create_func(5, 10),   // Low
    ];

    let config = build_complexity_scatter(&functions);

    assert_eq!(config.data.datasets.len(), 4);
    assert_eq!(config.data.datasets[0].label, "Critical");
    assert_eq!(config.data.datasets[0].data.len(), 1);
}

#[test]
fn test_generate_distinct_colors() {
    let colors_5 = generate_distinct_colors(5);
    assert_eq!(colors_5.len(), 5);

    let colors_20 = generate_distinct_colors(20);
    assert_eq!(colors_20.len(), 20);
    // All should be unique
    let unique: std::collections::HashSet<_> = colors_20.iter().collect();
    assert_eq!(unique.len(), 20);
}
```

**Table Generation** (`dashboard/tables.rs`):
```rust
#[test]
fn test_render_function_row_escaping() {
    let func = FunctionMetrics {
        name: "<script>alert('xss')</script>".to_string(),
        file: PathBuf::from("test.rs"),
        cyclomatic: 10,
        cognitive: 15,
        nesting: 2,
        entropy_score: Some(0.5),
        adjusted_complexity: Some(8.0),
        // ...
    };

    let html = render_function_row(&func);

    // Should be escaped
    assert!(html.contains("&lt;script&gt;"));
    assert!(!html.contains("<script>alert"));
}

#[test]
fn test_priority_badge_classes() {
    let critical = create_func(25, 60);
    assert_eq!(get_priority_badge_class(&critical), "bg-red-100 text-red-800");

    let low = create_func(5, 8);
    assert_eq!(get_priority_badge_class(&low), "bg-green-100 text-green-800");
}
```

### Integration Tests

**Full Dashboard Generation** (`tests/dashboard_generation.rs`):
```rust
#[test]
fn test_complete_dashboard_generation() {
    let results = create_comprehensive_test_results();
    let mut buffer = Vec::new();
    let mut writer = HtmlWriter::new(&mut buffer);

    writer.write_results(&results).unwrap();

    let html = String::from_utf8(buffer).unwrap();

    // Verify structure
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("</html>"));

    // Verify no template variables remain
    assert!(!html.contains("{{"));
    assert!(!html.contains("}}"));

    // Verify charts rendered
    assert!(html.contains("new Chart"));

    // Verify tables rendered
    assert!(html.contains("<table"));
    assert!(html.contains("</table>"));
}

#[test]
fn test_dashboard_with_unified_analysis() {
    let results = create_test_results();
    let unified = create_test_unified_analysis();
    let mut buffer = Vec::new();
    let mut writer = HtmlWriter::with_unified_analysis(&mut buffer, unified);

    writer.write_results(&results).unwrap();

    let html = String::from_utf8(buffer).unwrap();
    // Verify unified format used (no legacy format handling in HTML)
    assert!(html.contains("complexity_scatter"));
}
```

### Property-Based Tests

**Recommendation Grouping** (using `proptest`):
```rust
proptest! {
    #[test]
    fn prop_grouping_preserves_items(items in vec(arb_debt_item(), 0..100)) {
        let groups = group_recommendations(&items);

        let total_grouped: usize = groups.iter().map(|g| g.items.len()).sum();
        let top_10_total = groups.iter().map(|g| g.items.len()).sum::<usize>().min(items.len());

        // Should group top 10, preserving item count
        prop_assert!(total_grouped <= items.len());
    }

    #[test]
    fn prop_groups_sorted_by_priority(items in vec(arb_debt_item(), 10..50)) {
        let groups = group_recommendations(&items);

        // Verify priority ranks are descending
        prop_assert!(groups.windows(2).all(|w| w[0].priority_rank >= w[1].priority_rank));
    }
}
```

### Performance Tests

**Benchmark Data Processing**:
```rust
#[bench]
fn bench_dashboard_data_building(b: &mut Bencher) {
    let results = create_large_test_results(1000);  // 1000 debt items

    b.iter(|| {
        let data = DashboardDataBuilder::new()
            .with_results(&results)
            .build()
            .unwrap();
        black_box(data);
    });
}

#[bench]
fn bench_recommendation_grouping(b: &mut Bencher) {
    let items = create_test_items(1000);

    b.iter(|| {
        let groups = group_recommendations(&items);
        black_box(groups);
    });
}
```

## Documentation Requirements

### Code Documentation

- **Module-level docs** for `dashboard` module explaining architecture
- **Function docs** for all public APIs with examples
- **Doc tests** for key transformation functions
- **Examples** showing how to use DashboardDataBuilder

### Architecture Documentation

Update `ARCHITECTURE.md` with:
- New dashboard module structure
- Data flow diagram
- Separation of concerns explanation
- Template engine integration

### Migration Guide

Document migration for developers:
- What changed and why
- How to modify dashboard template
- Where to add new metrics/charts
- Testing new dashboard features

## Implementation Notes

### Tera vs Handlebars

**Tera Advantages**:
- Better error messages
- More Jinja2-like (familiar to many)
- Slightly faster
- Better whitespace control

**Handlebars Advantages**:
- More mature ecosystem
- Simpler syntax
- JavaScript Handlebars compatible

**Recommendation**: Start with Tera, can swap if needed.

### HTML Escaping

Always use `html_escape::encode_text()` for:
- Function names
- File paths
- User-visible messages
- Any dynamic content

Never escape:
- Pre-rendered HTML marked `| safe` in template
- Chart JSON (already JSON-escaped)

### Performance Considerations

- Pre-compute all metrics once in `DashboardDataBuilder`
- Use `Vec` pre-allocation where size known
- Avoid repeated string allocations
- Consider `Cow<'_, str>` for borrowed vs owned strings
- Cache color generation results

### Functional Programming Principles

All dashboard functions should be:
- **Pure** where possible (no side effects)
- **Composable** (small, focused functions)
- **Testable** (no hidden dependencies)
- **Type-safe** (leverage Rust type system)

Example pure function pipeline:
```rust
pub fn build_recommendations(items: &[DebtItem]) -> Vec<RecommendationGroup> {
    items.iter()
        .map(extract_recommendation_text)      // Pure
        .map(normalize_recommendation)         // Pure
        .fold(HashMap::new(), group_by_text)   // Pure
        .into_iter()
        .map(build_group)                      // Pure
        .sorted_by(priority_then_score)        // Pure
        .take(10)
        .collect()
}
```

## Migration and Compatibility

### Breaking Changes

**None** - This is an internal refactoring. Dashboard output remains identical.

### Backward Compatibility

- Existing `HtmlWriter::new()` constructor continues to work
- Template output is visually identical
- All features remain functional
- Chart configurations produce same visualizations

### Migration Steps

1. Add `tera` dependency to `Cargo.toml`
2. Create `dashboard` module alongside existing code
3. Implement modules one at a time with tests
4. Add feature flag for new implementation
5. Test both implementations in parallel
6. Switch default to new implementation
7. Deprecate old string replacement approach
8. Remove old implementation in next major version

### Rollback Plan

If issues arise:
1. Keep old template as `dashboard_legacy.html`
2. Add feature flag `use_legacy_dashboard`
3. Allow runtime switching via config
4. Fix issues in new implementation
5. Remove legacy after stabilization

## Success Metrics

- [ ] JavaScript reduced from 1,400+ lines to < 200 lines
- [ ] Template reduced from 1,700+ lines to < 300 lines
- [ ] All business logic in Rust with 85%+ test coverage
- [ ] Dashboard generation < 100ms for typical projects
- [ ] Zero visual regressions from current implementation
- [ ] All 12 acceptance criteria met

## Related Specifications

- None (this is a pure refactoring, no functional changes)

## Future Enhancements

After this refactoring, future improvements become easier:

1. **Server-side rendering caching** - Cache DashboardData between runs
2. **Progressive enhancement** - Dashboard works without JavaScript
3. **Custom themes** - Easy to swap CSS without touching logic
4. **Export formats** - Generate PDF/PNG from same data structures
5. **Real-time updates** - WebSocket updates to dashboard data
6. **Dashboard plugins** - Third-party visualizations using standardized data

---

**Estimated Effort**: 3-5 days
**Complexity**: Medium-High
**Risk**: Low (internal refactoring, no API changes)
