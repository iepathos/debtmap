# Debtmap Visualization Development

This directory contains tools for rapid iteration on HTML dashboard templates.

## Quick Start

```bash
# From the viz-dev directory
./serve.sh

# Or from debtmap root
python3 -m http.server 8080
```

Then open: http://localhost:8080/viz-dev/dashboard.html

## Workflow

1. **Start the dev server** - The dashboard will auto-load `../debtmap.json`
2. **Edit dashboard.html** - Make changes to the template
3. **Refresh the browser** - See your changes instantly
4. **No need to re-run debtmap** - Data comes from the cached JSON file

## Files

- `dashboard.html` - Main development template (edit this!)
- `serve.sh` - Simple HTTP server script

## Features

The dashboard.html template includes:

- **Data Loader** - Load JSON via file picker or auto-load from ../debtmap.json
- **Summary Cards** - Key metrics at a glance
- **Risk Quadrant** - Functions plotted by complexity vs coverage
- **Debt Table** - Sortable list of top debt items
- **Complexity Histogram** - Distribution of complexity scores
- **Category Pie Chart** - Debt breakdown by category
- **Priority Bars** - Distribution by priority level
- **File Hotspots** - Files with highest debt concentration

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
          cyclomatic, cognitive, adjusted, nesting, length, entropy }
    ],
    files: [...],
    fileScores: { "path/to/file.rs": { totalScore, count, critical, high } },
    categories: { "CodeQuality": 123, "Architecture": 45 },
    allItems: [...] // Raw items from JSON
}
```

## Integrating with Mockups

The mockups in the parent directory use hardcoded data. To integrate:

1. Copy visualization code from `../viz-mockups-v2.html` or `../viz-mockups-combined.html`
2. Replace hardcoded data with `transformedData` fields
3. You may need to add transformation logic in `transformUnifiedFormat()`

## Tips

- Use browser DevTools to inspect `rawData` and `transformedData` in the console
- The tooltip is shared - use it for hover information
- D3.js v7 is loaded - check https://d3js.org/ for docs
- CSS uses dark theme with GitHub's color palette
