# Debtmap Visualization Dashboard

Interactive dashboard for visualizing debtmap analysis results.

## Hosted Version

The dashboard is hosted at: **https://iepathos.github.io/debtmap/dashboard/**

Simply:
1. Run debtmap to generate JSON: `debtmap analyze . --format json -o debtmap.json`
2. Visit the hosted dashboard
3. Click "Load JSON File" and select your `debtmap.json`

All processing happens client-side - your data never leaves your browser.

## Local Development

For rapid iteration on the dashboard itself:

```bash
# From the viz-dev directory
./serve.sh

# Or from debtmap root
python3 -m http.server 8080
```

Then open: http://localhost:8080/viz-dev/dashboard.html

### Workflow

1. **Start the dev server** - The dashboard will auto-load `../debtmap.json`
2. **Edit dashboard.html** - Make changes to the template
3. **Refresh the browser** - See your changes instantly
4. **No need to re-run debtmap** - Data comes from the cached JSON file

## Files

- `dashboard.html` - Main dashboard (deployed to GitHub Pages)
- `serve.sh` - Simple HTTP server script for local development
- `README.md` - This file

## Features

The dashboard includes:

- **Summary Cards** - Key metrics at a glance (total items, critical/high/medium/low counts)
- **Risk Quadrant** - Functions plotted by complexity vs coverage gap
  - Y-axis: Cognitive/Cyclomatic complexity or debt score
  - X-axis: Coverage gap (right = untested)
  - Size: Debt score, churn, or fixed
  - Color: Priority, function role, or category
- **Top Debt Items Table** - Sortable list of highest priority items
- **Inter-Module Call Flow** - Chord diagram showing debt relationships between modules
- **Risk Profile Radar** - Multi-dimensional comparison of top files

## Customization

### Adding a new visualization

1. Add a container div in the grid:
   ```html
   <div class="viz-card">
       <h2>N. My New Chart</h2>
       <div id="my-chart"></div>
   </div>
   ```

2. Add a render function:
   ```javascript
   function renderMyChart() {
       const container = d3.select("#my-chart");
       container.selectAll("*").remove();
       // ... D3 code ...
   }
   ```

3. Call it from `processAndRender()`:
   ```javascript
   renderMyChart();
   ```

### Data Structure

The `transformedData` object has:

```javascript
{
    summary: {
        totalItems, totalScore, debtDensity, totalLoc,
        critical, high, medium, low
    },
    functions: [
        { name, file, line, score, priority, category, role,
          cyclomatic, cognitive, adjusted, nesting, length, entropy,
          churn, bugDensity, ageDays, authorCount, stability, hasGitHistory }
    ],
    files: [...],
    fileScores: { "path/to/file.rs": { totalScore, count, critical, high } },
    categories: { "CodeQuality": 123, "Architecture": 45 },
    allItems: [...] // Raw items from JSON
}
```

## Tips

- Use browser DevTools to inspect `rawData` and `transformedData` in the console
- The tooltip is shared - automatically repositions near screen edges
- D3.js v7 is loaded - check https://d3js.org/ for docs
- CSS uses dark theme with GitHub's color palette

## Deployment

Changes to this directory automatically trigger deployment via GitHub Actions.
The dashboard is copied to the GitHub Pages site at `/dashboard/`.
