# Visual Dashboard

The debtmap dashboard provides interactive visualizations of your technical debt analysis results.

**[Open Dashboard](https://iepathos.github.io/debtmap/dashboard/)**

## Quick Start

1. Generate JSON output from debtmap:

```bash
debtmap analyze . --format json -o debtmap.json --lcov coverage.lcov --context
```

2. Open the [dashboard](https://iepathos.github.io/debtmap/dashboard/)

3. Click **"Load JSON File"** and select your `debtmap.json`

That's it! Your data is processed entirely in your browser - nothing is uploaded to any server.

## Recommended Flags

For the best dashboard experience, include these flags:

| Flag | Purpose |
|------|---------|
| `--format json` | Required - dashboard reads JSON format |
| `--lcov <path>` | Coverage data enables coverage gap visualization |
| `--context` | Adds git history for churn analysis |
| `--cohesion` | Adds module cohesion metrics |

Example with all options:

```bash
debtmap analyze . \
  --format json \
  -o debtmap.json \
  --lcov target/coverage/lcov.info \
  --context \
  --cohesion
```

## Visualizations

### Risk Quadrant

A scatter plot showing functions positioned by risk factors:

- **Y-axis options**: Cognitive complexity, cyclomatic complexity, or debt score
- **X-axis**: Coverage gap (0% = fully tested, 100% = no tests)
- **Size options**: Debt score, churn frequency, or fixed size
- **Color options**: Priority level, function role, or category

The **danger zone** (top-right) shows high-complexity, untested code. The **healthy zone** (bottom-left) shows simple, well-tested code.

**Interactive features**:
- Hover for detailed metrics
- Click Y-axis, Size, or Color dropdowns to change visualization
- Tooltips automatically reposition near screen edges

### Top Debt Items Table

A sortable table of the highest-priority debt items:

| Column | Description |
|--------|-------------|
| File | Source file path |
| Function | Function name (if function-level item) |
| Score | Overall debt score |
| Priority | Critical, High, Medium, or Low |
| Category | Debt category (Complexity, Testing, Architecture) |
| Debt Type | Specific debt pattern detected |

Click column headers to sort.

### Inter-Module Call Flow (Chord Diagram)

Shows how debt flows between modules:

- **Arcs**: Modules (directories) sized by total debt
- **Ribbons**: Call relationships between modules
- **Hover**: See specific debt scores and call counts

Useful for identifying architectural hotspots where debt clusters.

### Risk Profile Radar

A radar chart comparing the top 5 files across multiple dimensions:

- Complexity
- Coverage gap
- Function count
- Cohesion (if available)
- Churn (if git history available)

Helps identify which files have the most well-rounded problems vs. single-dimension issues.

## Privacy

The dashboard runs entirely client-side:

- Your JSON file is read using the browser's File API
- No data is sent to any server
- Processing happens in JavaScript in your browser
- You can use the dashboard offline after the initial page load

## CI/CD Integration

You can generate dashboard-ready JSON in CI and publish it as an artifact:

```yaml
# .github/workflows/debtmap.yml
- name: Run debtmap
  run: |
    debtmap analyze . --format json -o debtmap.json --context

- name: Upload results
  uses: actions/upload-artifact@v4
  with:
    name: debtmap-results
    path: debtmap.json
```

Team members can download the artifact and load it in the dashboard.

## Offline Use

If you need the dashboard offline:

1. Visit the [dashboard](https://iepathos.github.io/debtmap/dashboard/)
2. Save the page (Ctrl/Cmd + S) as "Complete Webpage"
3. Open the saved HTML file locally

The saved page includes all JavaScript and will work without internet access.

## Development

The dashboard source is in [`viz-dev/`](https://github.com/iepathos/debtmap/tree/master/viz-dev) in the repository. Contributions welcome!

To develop locally:

```bash
cd viz-dev
./serve.sh
# Open http://localhost:8080/viz-dev/dashboard.html
```

Changes to `viz-dev/` automatically deploy to GitHub Pages.
