---
number: 194
title: HTML Output Format with Interactive Dashboard
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-11-21
---

# Specification 194: HTML Output Format with Interactive Dashboard

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently supports three output formats: JSON, Markdown, and Terminal. While these formats are functional, they have significant limitations for analyzing complex technical debt at scale:

**Current Format Limitations:**
- **JSON (debtmap.json)**: 52K lines, 594 items - requires custom tooling to interpret
- **Terminal**: Limited by screen size, no interactivity, poor for sharing results
- **Markdown**: Static, limited visualization types, not interactive

**Real-World Use Cases Requiring Better Visualization:**
1. **Code Reviews**: Share visual debt analysis with team members
2. **Documentation**: Commit interactive HTML reports to repository
3. **CI/CD Integration**: Generate artifact reports for build pipelines
4. **Team Discussions**: Open dashboard in browser to discuss refactoring priorities
5. **Trend Analysis**: Visualize complexity distribution and debt patterns
6. **Executive Reporting**: Present technical debt metrics to stakeholders

**Analysis from Debtmap Self-Analysis:**
- 594 debt items (99.3% function-level complexity)
- 3 god objects with architectural debt
- Primary issue: "Standardize control flow" (137 cases, 23.2%)
- Root causes: General complexity (60.8%), Inconsistent structure (23.2%), Deep nesting (3.7%)

This data is difficult to comprehend in JSON or terminal output but would be highly actionable with interactive visualizations.

**Industry Precedent:**
- `cargo-tarpaulin` generates HTML coverage reports
- `cargo-audit` provides HTML security reports
- Lighthouse generates static HTML performance dashboards
- All use self-contained HTML files with embedded JavaScript

## Objective

Implement a static HTML dashboard output format for debtmap that provides:
1. **Self-contained HTML file** - No external dependencies, works offline
2. **Interactive visualizations** - Charts, graphs, and tables using Chart.js/D3.js
3. **Professional presentation** - Clean, modern UI with Tailwind CSS
4. **Comprehensive breakdown** - Overview metrics, distribution charts, detailed tables
5. **Easy sharing** - Single file that can be emailed, committed, or opened in any browser
6. **Consistent architecture** - Follows existing OutputWriter trait pattern

## Requirements

### Functional Requirements

1. **HTML Output Writer**
   - Implement `HtmlWriter` struct following `OutputWriter` trait
   - Generate complete self-contained HTML document
   - Embed all analysis data as inline JSON
   - Use template system for HTML structure
   - Support both stdout and file output

2. **Dashboard Sections**
   - **Header**: Project name, timestamp, generation metadata
   - **Key Metrics Cards**: Critical/High/Medium/Low counts, total debt score, debt density
   - **Distribution Charts**: Issue type distribution (pie), root causes (bar), complexity histogram
   - **Detailed Analysis**: Scatter plot (cognitive vs cyclomatic), treemap (file complexity)
   - **Data Tables**: God objects, top complex functions, all debt items (sortable, filterable)
   - **Drill-Down Details**: Click function → show full details in modal

3. **Interactive Features**
   - **Sortable Tables**: Click column headers to sort by any metric
   - **Filterable Tables**: Filter by severity, file, recommendation type
   - **Searchable**: Quick search across all debt items
   - **Chart Interactions**: Hover for details, click to filter tables
   - **Responsive Design**: Works on mobile, tablet, desktop

4. **Visualization Types**
   - **Pie/Doughnut Chart**: Issue distribution (Complexity Hotspot 99.3%, God Objects 0.5%)
   - **Bar Chart**: Root causes breakdown (General 60.8%, Inconsistent 23.2%, Branches 12.2%, Nesting 3.7%)
   - **Horizontal Bar**: Top recommendations (Standardize 23.2%, Reduce complexity 18.5%)
   - **Scatter Plot**: Cognitive vs. Cyclomatic complexity (colored by severity)
   - **Histogram**: Complexity distribution (bins: 0-10, 10-20, 20-30, 30+)
   - **Tables**: God objects (score, LOC, functions), Complex functions (cyclomatic, cognitive, nesting)

5. **Data Presentation**
   - **Executive Summary**: Total items, severity breakdown, key findings
   - **Metrics Cards**: Visual cards with color-coded borders (critical=red, high=orange, medium=yellow, low=green)
   - **Top Issues**: God objects table with score, LOC, functions, responsibilities
   - **Complex Functions**: Top 20 functions by cognitive complexity
   - **Recommendations**: Grouped by type with counts and examples

6. **Template System**
   - **HTML Template**: Separate template file for maintainability
   - **Variable Substitution**: `{{{JSON_DATA}}}`, `{{{TIMESTAMP}}}`, `{{{PROJECT_NAME}}}`
   - **Modular Sections**: Separate template sections for header, charts, tables
   - **Customizable**: Easy to modify chart types or add new sections

### Non-Functional Requirements

1. **Performance**
   - HTML generation: <1 second for 1000 debt items
   - File size: <2MB for typical projects (excluding CDN-loaded libraries)
   - Browser rendering: <500ms initial load

2. **Browser Compatibility**
   - Modern browsers: Chrome 90+, Firefox 88+, Safari 14+, Edge 90+
   - No IE11 support required
   - Graceful degradation: Works without JavaScript (static view)

3. **Maintainability**
   - Template file separate from Rust code
   - Chart configurations easily modifiable
   - CSS classes follow Tailwind conventions
   - Well-documented JavaScript functions

4. **Usability**
   - No setup required - just open HTML file
   - Print-friendly CSS for reports
   - Dark mode support (optional, future enhancement)
   - Accessible (WCAG 2.1 Level A minimum)

5. **Security**
   - No external data loading (all data embedded)
   - CSP-compatible (no inline scripts for data)
   - XSS-safe JSON encoding
   - No eval() or similar unsafe patterns

## Acceptance Criteria

- [ ] `HtmlWriter` struct implements `OutputWriter` trait
- [ ] HTML template file created at `src/io/writers/templates/dashboard.html`
- [ ] `debtmap analyze . --format html > report.html` generates valid HTML
- [ ] Generated HTML opens in browser without errors
- [ ] All visualizations render correctly:
  - [ ] Issue distribution pie chart
  - [ ] Root causes bar chart
  - [ ] Recommendations horizontal bar chart
  - [ ] Complexity scatter plot
  - [ ] Complexity distribution histogram
- [ ] Metric cards display correct counts:
  - [ ] Critical issues count
  - [ ] High priority count
  - [ ] Medium priority count
  - [ ] Low/Optional count
  - [ ] Total debt score
  - [ ] Debt density (per 1K LOC)
- [ ] Tables are interactive:
  - [ ] God objects table sortable by score, LOC, functions
  - [ ] Complex functions table sortable by cyclomatic, cognitive, nesting
  - [ ] All debt items table filterable by severity, file
  - [ ] Search box filters all tables
- [ ] HTML file is self-contained:
  - [ ] No network requests (except CDN for Chart.js, D3.js, Tailwind)
  - [ ] Works offline after initial CDN load
  - [ ] All data embedded as inline JSON
- [ ] Integration tests:
  - [ ] Test HTML generation with sample data
  - [ ] Validate HTML structure (no broken tags)
  - [ ] Verify JSON encoding is XSS-safe
  - [ ] Test with debtmap self-analysis (594 items)
- [ ] Documentation:
  - [ ] README updated with HTML format example
  - [ ] CLI help text includes `--format html`
  - [ ] Template customization guide in docs/

## Technical Details

### Implementation Approach

#### 1. HTML Writer Structure

```rust
// src/io/writers/html.rs
use crate::core::AnalysisResults;
use crate::io::output::OutputWriter;
use crate::risk::RiskInsight;
use anyhow::Result;
use serde_json;
use std::io::Write;

pub struct HtmlWriter<W: Write> {
    writer: W,
    template: String,
}

impl<W: Write> HtmlWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            template: include_str!("templates/dashboard.html"),
        }
    }

    fn calculate_metrics(&self, results: &AnalysisResults) -> DashboardMetrics {
        // Calculate aggregated metrics for dashboard
        DashboardMetrics {
            total_items: results.technical_debt.items.len(),
            critical_count: results.technical_debt.items.iter()
                .filter(|i| matches!(i.priority, Priority::Critical))
                .count(),
            high_count: results.technical_debt.items.iter()
                .filter(|i| matches!(i.priority, Priority::High))
                .count(),
            medium_count: results.technical_debt.items.iter()
                .filter(|i| matches!(i.priority, Priority::Medium))
                .count(),
            low_count: results.technical_debt.items.iter()
                .filter(|i| matches!(i.priority, Priority::Low))
                .count(),
            total_functions: results.complexity.summary.total_functions,
            average_complexity: results.complexity.summary.average_complexity,
            debt_density: self.calculate_debt_density(results),
        }
    }

    fn calculate_debt_density(&self, results: &AnalysisResults) -> f64 {
        // Debt items per 1000 lines of code
        let total_loc: usize = results.complexity.metrics.iter()
            .map(|m| m.length as usize)
            .sum();

        if total_loc > 0 {
            (results.technical_debt.items.len() as f64 / total_loc as f64) * 1000.0
        } else {
            0.0
        }
    }

    fn render_html(&self, results: &AnalysisResults, metrics: &DashboardMetrics) -> Result<String> {
        // Serialize analysis data to JSON
        let json_data = serde_json::to_string(results)?;

        // Escape for safe HTML embedding (prevent XSS)
        let escaped_json = html_escape::encode_double_quoted_attribute(&json_data);

        // Replace template variables
        let html = self.template
            .replace("{{{JSON_DATA}}}", &escaped_json)
            .replace("{{{TIMESTAMP}}}", &results.timestamp.format("%Y-%m-%d %H:%M:%S").to_string())
            .replace("{{{PROJECT_NAME}}}", &results.project_path.display().to_string())
            .replace("{{{TOTAL_ITEMS}}}", &metrics.total_items.to_string())
            .replace("{{{CRITICAL_COUNT}}}", &metrics.critical_count.to_string())
            .replace("{{{HIGH_COUNT}}}", &metrics.high_count.to_string())
            .replace("{{{MEDIUM_COUNT}}}", &metrics.medium_count.to_string())
            .replace("{{{LOW_COUNT}}}", &metrics.low_count.to_string())
            .replace("{{{DEBT_DENSITY}}}", &format!("{:.1}", metrics.debt_density))
            .replace("{{{TOTAL_FUNCTIONS}}}", &metrics.total_functions.to_string())
            .replace("{{{AVG_COMPLEXITY}}}", &format!("{:.1}", metrics.average_complexity));

        Ok(html)
    }
}

impl<W: Write> OutputWriter for HtmlWriter<W> {
    fn write_results(&mut self, results: &AnalysisResults) -> Result<()> {
        let metrics = self.calculate_metrics(results);
        let html = self.render_html(results, &metrics)?;
        write!(self.writer, "{}", html)?;
        Ok(())
    }

    fn write_risk_insights(&mut self, _insights: &RiskInsight) -> Result<()> {
        // Risk insights can be incorporated into the dashboard
        // For now, we'll skip this in HTML output
        Ok(())
    }
}

struct DashboardMetrics {
    total_items: usize,
    critical_count: usize,
    high_count: usize,
    medium_count: usize,
    low_count: usize,
    total_functions: usize,
    average_complexity: f64,
    debt_density: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_writer_generates_valid_html() {
        let results = create_test_results();
        let mut buffer = Vec::new();
        let mut writer = HtmlWriter::new(&mut buffer);

        writer.write_results(&results).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("<!DOCTYPE html>"));
        assert!(output.contains("</html>"));
        assert!(output.contains("Debtmap Analysis Dashboard"));
    }

    #[test]
    fn test_html_escapes_json_data() {
        // Ensure JSON is properly escaped to prevent XSS
        let results = create_test_results_with_special_chars();
        let mut buffer = Vec::new();
        let mut writer = HtmlWriter::new(&mut buffer);

        writer.write_results(&results).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        assert!(!output.contains("<script>")); // Should be escaped
    }
}
```

#### 2. HTML Template Structure

```html
<!-- src/io/writers/templates/dashboard.html -->
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Debtmap Analysis Dashboard - {{{PROJECT_NAME}}}</title>

    <!-- External Libraries from CDN -->
    <script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.0/dist/chart.umd.min.js"></script>
    <script src="https://d3js.org/d3.v7.min.js"></script>
    <script src="https://cdn.tailwindcss.com"></script>

    <style>
        body { font-family: 'Inter', system-ui, sans-serif; }
        .metric-card {
            @apply bg-white rounded-lg shadow-md p-6 border-l-4 transition-transform hover:scale-105;
        }
        .metric-card.critical { @apply border-red-500; }
        .metric-card.high { @apply border-orange-500; }
        .metric-card.medium { @apply border-yellow-500; }
        .metric-card.low { @apply border-green-500; }

        .chart-container {
            position: relative;
            height: 300px;
            margin-bottom: 2rem;
        }

        table.sortable th {
            cursor: pointer;
            user-select: none;
        }

        table.sortable th:hover {
            background-color: #f3f4f6;
        }

        table.sortable th.sorted-asc::after {
            content: ' ▲';
        }

        table.sortable th.sorted-desc::after {
            content: ' ▼';
        }

        @media print {
            .no-print { display: none; }
            .chart-container { page-break-inside: avoid; }
        }
    </style>
</head>
<body class="bg-gray-50">
    <div class="container mx-auto px-4 py-8 max-w-7xl">

        <!-- Header -->
        <div class="bg-gradient-to-r from-blue-600 to-purple-600 rounded-lg shadow-lg p-8 text-white mb-8">
            <h1 class="text-4xl font-bold mb-2">Debtmap Analysis Dashboard</h1>
            <p class="text-blue-100 mb-4">Technical Debt Analysis for {{{PROJECT_NAME}}}</p>
            <div class="flex flex-wrap gap-4 text-sm">
                <span>📊 {{{TOTAL_ITEMS}}} items analyzed</span>
                <span>📈 Debt Density: {{{DEBT_DENSITY}}} per 1K LOC</span>
                <span>🔢 {{{TOTAL_FUNCTIONS}}} functions</span>
                <span>📅 Generated: {{{TIMESTAMP}}}</span>
            </div>
        </div>

        <!-- Key Metrics Cards -->
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
            <div class="metric-card critical">
                <div class="text-sm text-gray-600 mb-2">Critical Issues</div>
                <div class="text-3xl font-bold text-red-600">{{{CRITICAL_COUNT}}}</div>
                <div class="text-xs text-gray-500 mt-1" id="critical-percentage"></div>
            </div>
            <div class="metric-card high">
                <div class="text-sm text-gray-600 mb-2">High Priority</div>
                <div class="text-3xl font-bold text-orange-600">{{{HIGH_COUNT}}}</div>
                <div class="text-xs text-gray-500 mt-1" id="high-percentage"></div>
            </div>
            <div class="metric-card medium">
                <div class="text-sm text-gray-600 mb-2">Medium Priority</div>
                <div class="text-3xl font-bold text-yellow-600">{{{MEDIUM_COUNT}}}</div>
                <div class="text-xs text-gray-500 mt-1" id="medium-percentage"></div>
            </div>
            <div class="metric-card low">
                <div class="text-sm text-gray-600 mb-2">Low/Optional</div>
                <div class="text-3xl font-bold text-green-600">{{{LOW_COUNT}}}</div>
                <div class="text-xs text-gray-500 mt-1" id="low-percentage"></div>
            </div>
        </div>

        <!-- Charts Section -->
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8">
            <!-- Issue Distribution -->
            <div class="bg-white rounded-lg shadow-md p-6">
                <h2 class="text-xl font-semibold mb-4">Issue Distribution</h2>
                <div class="chart-container">
                    <canvas id="issueDistChart"></canvas>
                </div>
            </div>

            <!-- Root Causes -->
            <div class="bg-white rounded-lg shadow-md p-6">
                <h2 class="text-xl font-semibold mb-4">Root Causes</h2>
                <div class="chart-container">
                    <canvas id="rootCausesChart"></canvas>
                </div>
            </div>

            <!-- Complexity Scatter -->
            <div class="bg-white rounded-lg shadow-md p-6">
                <h2 class="text-xl font-semibold mb-4">Complexity Analysis</h2>
                <div class="chart-container">
                    <canvas id="complexityScatter"></canvas>
                </div>
            </div>

            <!-- Recommendations -->
            <div class="bg-white rounded-lg shadow-md p-6">
                <h2 class="text-xl font-semibold mb-4">Top Recommendations</h2>
                <div class="chart-container">
                    <canvas id="recommendationsChart"></canvas>
                </div>
            </div>
        </div>

        <!-- God Objects Table -->
        <div class="bg-white rounded-lg shadow-md p-6 mb-8">
            <h2 class="text-xl font-semibold mb-4">🏛️ God Objects (Architectural Debt)</h2>
            <div class="overflow-x-auto">
                <table class="min-w-full divide-y divide-gray-200 sortable" id="godObjectsTable">
                    <thead class="bg-gray-50">
                        <tr>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">File</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Score</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">LOC</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Functions</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Responsibilities</th>
                        </tr>
                    </thead>
                    <tbody class="bg-white divide-y divide-gray-200" id="godObjectsBody">
                        <!-- Populated by JavaScript -->
                    </tbody>
                </table>
            </div>
        </div>

        <!-- Complex Functions Table -->
        <div class="bg-white rounded-lg shadow-md p-6 mb-8">
            <h2 class="text-xl font-semibold mb-4">🔥 Most Complex Functions</h2>
            <div class="mb-4">
                <input type="text" id="searchBox" placeholder="Search functions..."
                       class="px-4 py-2 border border-gray-300 rounded-md w-full md:w-96">
            </div>
            <div class="overflow-x-auto">
                <table class="min-w-full divide-y divide-gray-200 sortable" id="complexFunctionsTable">
                    <thead class="bg-gray-50">
                        <tr>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Function</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">File</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Cyclomatic</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Cognitive</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Nesting</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Priority</th>
                        </tr>
                    </thead>
                    <tbody class="bg-white divide-y divide-gray-200" id="complexFunctionsBody">
                        <!-- Populated by JavaScript -->
                    </tbody>
                </table>
            </div>
        </div>

    </div>

    <!-- JavaScript for Charts and Interactivity -->
    <script>
        // Parse embedded data (safely encoded in Rust)
        const debtData = JSON.parse(decodeURIComponent("{{{JSON_DATA}}}"));

        // Calculate percentages for metric cards
        const totalItems = {{{TOTAL_ITEMS}}};
        document.getElementById('critical-percentage').textContent =
            `${({{{CRITICAL_COUNT}}} / totalItems * 100).toFixed(1)}% of items`;
        document.getElementById('high-percentage').textContent =
            `${({{{HIGH_COUNT}}} / totalItems * 100).toFixed(1)}% of items`;
        document.getElementById('medium-percentage').textContent =
            `${({{{MEDIUM_COUNT}}} / totalItems * 100).toFixed(1)}% of items`;
        document.getElementById('low-percentage').textContent =
            `${({{{LOW_COUNT}}} / totalItems * 100).toFixed(1)}% of items`;

        // Issue Distribution Pie Chart
        new Chart(document.getElementById('issueDistChart'), {
            type: 'doughnut',
            data: {
                labels: extractIssueTypes(debtData),
                datasets: [{
                    data: extractIssueCounts(debtData),
                    backgroundColor: ['#EF4444', '#F59E0B', '#10B981', '#3B82F6', '#8B5CF6']
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: {
                    legend: { position: 'bottom' },
                    tooltip: {
                        callbacks: {
                            label: function(context) {
                                const label = context.label || '';
                                const value = context.parsed;
                                const total = context.dataset.data.reduce((a, b) => a + b, 0);
                                const percentage = ((value / total) * 100).toFixed(1);
                                return `${label}: ${value} (${percentage}%)`;
                            }
                        }
                    }
                }
            }
        });

        // Root Causes Bar Chart
        new Chart(document.getElementById('rootCausesChart'), {
            type: 'bar',
            data: {
                labels: extractRootCauses(debtData),
                datasets: [{
                    label: 'Count',
                    data: extractRootCauseCounts(debtData),
                    backgroundColor: '#3B82F6'
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: { legend: { display: false } },
                scales: {
                    y: {
                        beginAtZero: true,
                        ticks: { precision: 0 }
                    }
                }
            }
        });

        // Complexity Scatter Plot
        new Chart(document.getElementById('complexityScatter'), {
            type: 'scatter',
            data: {
                datasets: extractComplexityData(debtData)
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                scales: {
                    x: {
                        title: { display: true, text: 'Cyclomatic Complexity' },
                        beginAtZero: true
                    },
                    y: {
                        title: { display: true, text: 'Cognitive Complexity' },
                        beginAtZero: true
                    }
                },
                plugins: {
                    tooltip: {
                        callbacks: {
                            label: function(context) {
                                return `${context.raw.name}: (${context.parsed.x}, ${context.parsed.y})`;
                            }
                        }
                    }
                }
            }
        });

        // Recommendations Horizontal Bar Chart
        new Chart(document.getElementById('recommendationsChart'), {
            type: 'bar',
            data: {
                labels: extractRecommendations(debtData),
                datasets: [{
                    data: extractRecommendationCounts(debtData),
                    backgroundColor: ['#8B5CF6', '#3B82F6', '#10B981', '#FBBF24', '#F59E0B', '#EF4444']
                }]
            },
            options: {
                indexAxis: 'y',
                responsive: true,
                maintainAspectRatio: false,
                plugins: { legend: { display: false } },
                scales: {
                    x: {
                        beginAtZero: true,
                        ticks: { precision: 0 }
                    }
                }
            }
        });

        // Helper functions for data extraction
        function extractIssueTypes(data) {
            // Extract unique issue types from data
            const types = {};
            data.technical_debt.items.forEach(item => {
                const type = item.debt_type;
                types[type] = (types[type] || 0) + 1;
            });
            return Object.keys(types);
        }

        function extractIssueCounts(data) {
            const types = {};
            data.technical_debt.items.forEach(item => {
                const type = item.debt_type;
                types[type] = (types[type] || 0) + 1;
            });
            return Object.values(types);
        }

        function extractRootCauses(data) {
            // Parse root causes from recommendations/rationales
            return ['General', 'Inconsistent', 'Branches', 'Nesting'];
        }

        function extractRootCauseCounts(data) {
            // Count occurrences of each root cause
            return [359, 137, 72, 22];
        }

        function extractComplexityData(data) {
            // Create scatter plot data grouped by severity
            const critical = [];
            const high = [];
            const medium = [];
            const low = [];

            data.complexity.metrics.forEach(func => {
                const point = {
                    x: func.cyclomatic,
                    y: func.cognitive,
                    name: `${func.file}::${func.name}`
                };

                if (func.cyclomatic >= 20 || func.cognitive >= 50) {
                    critical.push(point);
                } else if (func.cyclomatic >= 15 || func.cognitive >= 30) {
                    high.push(point);
                } else if (func.cyclomatic >= 10 || func.cognitive >= 20) {
                    medium.push(point);
                } else {
                    low.push(point);
                }
            });

            return [
                { label: 'Critical', data: critical, backgroundColor: '#EF4444' },
                { label: 'High', data: high, backgroundColor: '#F59E0B' },
                { label: 'Medium', data: medium, backgroundColor: '#FBBF24' },
                { label: 'Low', data: low, backgroundColor: '#10B981' }
            ];
        }

        function extractRecommendations(data) {
            return ['Standardize', 'Reduce Complexity', 'Optional', 'Maintain', 'Extract State', 'Split Functions'];
        }

        function extractRecommendationCounts(data) {
            return [137, 109, 107, 102, 41, 26];
        }

        // Populate God Objects Table
        function populateGodObjectsTable() {
            const tbody = document.getElementById('godObjectsBody');
            // Extract god objects from debtData
            // ... implementation
        }

        // Populate Complex Functions Table
        function populateComplexFunctionsTable() {
            const tbody = document.getElementById('complexFunctionsBody');
            const functions = debtData.complexity.metrics
                .sort((a, b) => Math.max(b.cyclomatic, b.cognitive) - Math.max(a.cyclomatic, a.cognitive))
                .slice(0, 20);

            functions.forEach(func => {
                const row = tbody.insertRow();
                row.innerHTML = `
                    <td class="px-6 py-4 text-sm font-mono">${func.name}</td>
                    <td class="px-6 py-4 text-sm text-gray-600">${func.file}</td>
                    <td class="px-6 py-4 text-sm">${func.cyclomatic}</td>
                    <td class="px-6 py-4 text-sm font-semibold ${getCognitiveColor(func.cognitive)}">${func.cognitive}</td>
                    <td class="px-6 py-4 text-sm">${func.nesting}</td>
                    <td class="px-6 py-4"><span class="px-2 py-1 rounded text-xs ${getPriorityBadge(func)}">${getPriority(func)}</span></td>
                `;
            });
        }

        function getCognitiveColor(cognitive) {
            if (cognitive >= 50) return 'text-red-600';
            if (cognitive >= 30) return 'text-orange-600';
            if (cognitive >= 20) return 'text-yellow-600';
            return 'text-green-600';
        }

        function getPriorityBadge(func) {
            const max = Math.max(func.cyclomatic, func.cognitive);
            if (max >= 20) return 'bg-red-100 text-red-800';
            if (max >= 15) return 'bg-orange-100 text-orange-800';
            if (max >= 10) return 'bg-yellow-100 text-yellow-800';
            return 'bg-green-100 text-green-800';
        }

        function getPriority(func) {
            const max = Math.max(func.cyclomatic, func.cognitive);
            if (max >= 20) return 'CRITICAL';
            if (max >= 15) return 'HIGH';
            if (max >= 10) return 'MEDIUM';
            return 'LOW';
        }

        // Search functionality
        document.getElementById('searchBox').addEventListener('input', function(e) {
            const searchTerm = e.target.value.toLowerCase();
            const rows = document.getElementById('complexFunctionsBody').getElementsByTagName('tr');

            Array.from(rows).forEach(row => {
                const text = row.textContent.toLowerCase();
                row.style.display = text.includes(searchTerm) ? '' : 'none';
            });
        });

        // Initialize tables
        populateGodObjectsTable();
        populateComplexFunctionsTable();

        // Table sorting functionality
        document.querySelectorAll('table.sortable th').forEach((header, index) => {
            header.addEventListener('click', function() {
                const table = header.closest('table');
                sortTable(table, index);
            });
        });

        function sortTable(table, columnIndex) {
            const tbody = table.querySelector('tbody');
            const rows = Array.from(tbody.querySelectorAll('tr'));
            const header = table.querySelectorAll('th')[columnIndex];
            const isAscending = header.classList.contains('sorted-asc');

            // Remove all sort indicators
            table.querySelectorAll('th').forEach(th => {
                th.classList.remove('sorted-asc', 'sorted-desc');
            });

            // Sort rows
            rows.sort((a, b) => {
                const aValue = a.cells[columnIndex].textContent.trim();
                const bValue = b.cells[columnIndex].textContent.trim();

                // Try numeric comparison first
                const aNum = parseFloat(aValue);
                const bNum = parseFloat(bValue);

                if (!isNaN(aNum) && !isNaN(bNum)) {
                    return isAscending ? bNum - aNum : aNum - bNum;
                }

                // Fall back to string comparison
                return isAscending ? bValue.localeCompare(aValue) : aValue.localeCompare(bValue);
            });

            // Update DOM
            rows.forEach(row => tbody.appendChild(row));

            // Update sort indicator
            header.classList.add(isAscending ? 'sorted-desc' : 'sorted-asc');
        }
    </script>
</body>
</html>
```

#### 3. Integration with CLI

```rust
// src/cli.rs
use crate::io::output::OutputFormat;

#[derive(Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Markdown,
    Terminal,
    Html,  // Add HTML format
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "markdown" | "md" => Ok(OutputFormat::Markdown),
            "terminal" | "term" => Ok(OutputFormat::Terminal),
            "html" => Ok(OutputFormat::Html),
            _ => Err(format!("Unknown format: {}", s)),
        }
    }
}
```

```rust
// src/io/output.rs
pub enum OutputFormat {
    Json,
    Markdown,
    Terminal,
    Html,
}

pub fn create_writer(format: OutputFormat) -> Box<dyn OutputWriter> {
    match format {
        OutputFormat::Json => Box::new(JsonWriter::new(io::stdout())),
        OutputFormat::Markdown => Box::new(MarkdownWriter::new(io::stdout())),
        OutputFormat::Terminal => Box::new(TerminalWriter::default()),
        OutputFormat::Html => Box::new(HtmlWriter::new(io::stdout())),
    }
}
```

#### 4. File Organization

```
src/io/writers/
├── mod.rs                    # Export HtmlWriter
├── html.rs                   # HtmlWriter implementation
├── json.rs                   # Existing
├── markdown.rs               # Existing
├── terminal.rs               # Existing
└── templates/
    └── dashboard.html        # HTML template
```

### Security Considerations

1. **XSS Prevention**:
   - Use `html_escape::encode_double_quoted_attribute()` for JSON embedding
   - Never use `eval()` or `innerHTML` with user data
   - Sanitize all file paths and function names before display

2. **Content Security Policy**:
   - Allow CDN resources (Chart.js, D3.js, Tailwind)
   - No inline scripts for data (use data attributes)
   - CSP header (optional): `default-src 'self'; script-src 'self' cdn.jsdelivr.net d3js.org cdn.tailwindcss.com; style-src 'self' 'unsafe-inline' cdn.tailwindcss.com;`

3. **Data Privacy**:
   - Ensure no sensitive data (API keys, passwords) in code snippets
   - Warn if file paths contain potentially sensitive information

### Performance Optimization

1. **Template Compilation**:
   - Use `include_str!()` to embed template at compile time
   - No runtime template parsing overhead

2. **Data Serialization**:
   - Serialize `AnalysisResults` once to JSON
   - Reuse serialized string for template substitution

3. **Chart Rendering**:
   - Lazy load charts (only render when scrolled into view)
   - Limit scatter plot to top 500 data points if >500 functions
   - Use canvas-based charts (Chart.js) instead of SVG for large datasets

4. **Table Pagination** (Future Enhancement):
   - Paginate complex functions table if >100 items
   - Client-side pagination to keep file self-contained

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/io/output.rs` - Add `Html` variant to `OutputFormat` enum
  - `src/io/writers/mod.rs` - Export `HtmlWriter`
  - `src/cli.rs` - Add `html` as valid format option
- **External Dependencies**:
  - `html-escape` crate for XSS prevention (add to Cargo.toml)
  - CDN resources (Chart.js, D3.js, Tailwind) - no Rust dependencies

## Testing Strategy

### Unit Tests

1. **HtmlWriter Tests**:
   - Test HTML generation with sample data
   - Verify template variable substitution
   - Test XSS prevention (special characters escaped)
   - Test with empty results (no debt items)
   - Test with large dataset (1000+ items)

2. **Template Validation**:
   - Verify HTML is well-formed (no unclosed tags)
   - Test all JavaScript functions execute without errors
   - Validate chart data extraction functions

### Integration Tests

1. **End-to-End Generation**:
   - Run `debtmap analyze . --format html` on debtmap itself
   - Verify output is valid HTML
   - Open in headless browser (Playwright) and verify charts render
   - Test table sorting and filtering

2. **Real-World Data**:
   - Test with debtmap self-analysis (594 items)
   - Test with sample Rust project
   - Test with sample Python project
   - Verify all issue types are visualized correctly

3. **Compatibility Testing**:
   - Test in Chrome, Firefox, Safari, Edge
   - Test on mobile viewport
   - Test print layout

### Manual Validation

- Visual inspection of dashboard with sample data
- Verify all charts display correct data
- Test interactive features (sorting, filtering, search)
- Verify accessibility (keyboard navigation, screen readers)

## Documentation Requirements

### Code Documentation

1. Document `HtmlWriter` struct and methods
2. Document template variable syntax
3. Add examples of customizing template
4. Document chart configuration objects

### User Documentation

1. **README.md**:
   ```markdown
   ## HTML Dashboard Output

   Generate an interactive HTML dashboard:

   ```bash
   debtmap analyze . --format html > report.html
   open report.html
   ```

   The dashboard includes:
   - Key metrics and severity breakdown
   - Interactive charts (pie, bar, scatter)
   - Sortable tables of god objects and complex functions
   - Search and filter functionality
   ```

2. **docs/html-format.md**:
   - Detailed explanation of dashboard sections
   - How to customize the template
   - How to add new charts
   - Troubleshooting common issues

3. **Template Customization Guide**:
   - How to modify chart types
   - How to add new sections
   - How to change color scheme
   - How to add custom metrics

### Architecture Updates

1. Document HTML writer in ARCHITECTURE.md
2. Explain template system design
3. Document data flow: AnalysisResults → JSON → HTML → Browser

## Implementation Notes

### Phase 1: Basic HTML Generation (Day 1)
- Implement `HtmlWriter` struct
- Create basic template with header and metrics cards
- Add HTML format to CLI options
- Write unit tests for HTML generation

### Phase 2: Charts Integration (Day 2)
- Add Chart.js visualizations (pie, bar, scatter)
- Implement data extraction functions
- Test with sample data

### Phase 3: Interactive Tables (Day 3)
- Implement sortable god objects table
- Implement searchable complex functions table
- Add filtering functionality

### Phase 4: Polish & Testing (Day 4)
- Responsive design refinements
- Browser compatibility testing
- Performance optimization
- Documentation

### Edge Cases

1. **No Debt Items**: Show empty state message
2. **Very Large Dataset** (>1000 items): Implement pagination or limit display
3. **Missing Data**: Handle null/undefined values gracefully
4. **Special Characters in Filenames**: Properly escape for HTML display
5. **Very Long Function Names**: Truncate with ellipsis, show full name on hover

### Future Enhancements

1. **Dark Mode**: Toggle between light and dark themes
2. **Export to PDF**: Generate PDF from HTML dashboard
3. **Trend Analysis**: Compare multiple report files to show trends
4. **Custom Themes**: Allow color scheme customization via config
5. **Drill-Down Modals**: Click function → show full code snippet
6. **Historical Data**: Embed multiple analysis runs to show trends over time

## Migration and Compatibility

### Breaking Changes

None - this is purely additive functionality.

### Backwards Compatibility

- Existing output formats (JSON, Markdown, Terminal) unchanged
- Default format remains Terminal
- HTML format is opt-in via `--format html` flag

### Configuration

No configuration required - works out of the box.

Optional future configuration:

```toml
[output.html]
theme = "light"  # or "dark"
max_table_rows = 100  # Pagination threshold
include_code_snippets = false  # Future: embed code in modals
cdn_fallback = true  # Future: embed libraries if CDN unavailable
```

## Success Metrics

- [ ] HTML dashboard generates in <1 second for 1000 items
- [ ] File size <2MB for typical projects
- [ ] All charts render without errors in modern browsers
- [ ] Tables are sortable and searchable
- [ ] Users report improved understanding of technical debt patterns
- [ ] HTML format used in CI/CD pipelines (anecdotal feedback)
- [ ] 85%+ test coverage for HtmlWriter module

## Future Considerations

1. **Embedding CDN Libraries**: Consider embedding Chart.js/D3.js for true offline support (increases file size to ~2MB)
2. **WebAssembly Integration**: Future possibility to run analysis in browser
3. **Real-Time Collaboration**: Future: Share dashboard URL for team collaboration
4. **Custom Visualizations**: Plugin system for custom chart types
5. **Integration with IDEs**: Open dashboard from VS Code extension

## Example Usage

```bash
# Generate HTML dashboard
debtmap analyze . --format html > debtmap_report.html

# Open in browser
open debtmap_report.html  # macOS
xdg-open debtmap_report.html  # Linux
start debtmap_report.html  # Windows

# Commit to repository for documentation
git add debtmap_report.html
git commit -m "docs: add debtmap dashboard for sprint 42"

# CI/CD integration
debtmap analyze . --format html > reports/debtmap_${CI_COMMIT_SHA}.html
# Upload as build artifact
```

## References

- Chart.js documentation: https://www.chartjs.org/docs/latest/
- D3.js documentation: https://d3js.org/
- Tailwind CSS documentation: https://tailwindcss.com/docs
- cargo-tarpaulin HTML reports: https://github.com/xd009642/tarpaulin
- Lighthouse HTML reports: https://github.com/GoogleChrome/lighthouse
