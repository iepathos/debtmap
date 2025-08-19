---
name: mmm-compare-debt-results
description: Compare before/after debtmap results to measure debt reduction improvements
args:
  - name: before
    required: true
    description: Path to the original debtmap.json file
  - name: after
    required: true
    description: Path to the updated debtmap.json file after fixes
  - name: map-results
    required: false
    description: JSON results from the map phase
  - name: successful
    required: false
    description: Number of successfully fixed items
  - name: failed
    required: false
    description: Number of items that failed to fix
---

# Compare Debt Results

## Purpose
Analyze the difference between before and after debtmap results to quantify technical debt improvements made during the MapReduce workflow.

## Usage
```
/mmm-compare-debt-results --before <original-debtmap.json> --after <new-debtmap.json> --map-results '<results>' --successful <count> --failed <count>
```

## Parameters
- `--before`: Path to the original debtmap.json file
- `--after`: Path to the updated debtmap.json file after fixes
- `--map-results`: JSON results from the map phase (optional)
- `--successful`: Number of successfully fixed items
- `--failed`: Number of items that failed to fix

## Process

1. **Load and Parse JSON Files**
   - Read the before and after debtmap.json files
   - Parse the JSON structures

2. **Calculate Overall Metrics**
   - Compare total debt scores
   - Count total items before/after
   - Calculate percentage improvements

3. **Analyze Item-Level Changes**
   - Match items by location (file, function, line)
   - Identify:
     - Items completely resolved (no longer in after)
     - Items with reduced scores
     - Items with increased scores (regression)
     - New items introduced

4. **Category Analysis**
   - Group improvements by debt type:
     - Complexity debt
     - Duplication debt
     - Coverage debt
     - Dependency debt
   - Show which categories improved most

5. **Generate Summary Report**
   Format a concise summary for the commit message:
   ```
   Technical Debt Improvements:
   - Total debt score: 850 → 620 (-27%)
   - Items resolved: 8 of 10 targeted
   - Overall items: 45 → 37 (-18%)
   
   By category:
   - Complexity: -35% (removed 5 high-complexity functions)
   - Duplication: -42% (eliminated 3 duplicate blocks)
   - Coverage: -15% (added tests for 4 critical functions)
   
   Top improvements:
   1. src/parser.rs::parse_args: score 85 → 0 (resolved)
   2. src/auth.rs::validate: score 72 → 25 (-65%)
   3. src/utils.rs::process: score 68 → 0 (resolved)
   ```

6. **Identify Regressions**
   If any items got worse or new high-score items appeared:
   ```
   ⚠️ Regressions detected:
   - src/main.rs::handle_request: score 45 → 52 (+16%)
   - NEW: src/api.rs::send_data: score 38
   ```

## Output Format
Generate a concise, markdown-formatted summary suitable for inclusion in a git commit message. Focus on:
- Quantitative improvements (percentages and counts)
- Most significant improvements
- Any regressions or concerns
- Overall success rate

## Error Handling
- If files cannot be read, report the error clearly
- If JSON structure is unexpected, provide details
- Handle cases where items may have moved (line number changes)

## Example Implementation Steps

```python
# Pseudo-code structure
before_data = load_json(before_path)
after_data = load_json(after_path)

# Create lookup maps
before_items = {(item.file, item.function): item for item in before_data.items}
after_items = {(item.file, item.function): item for item in after_data.items}

# Calculate improvements
resolved = before_items.keys() - after_items.keys()
improved = []
regressed = []
new_items = after_items.keys() - before_items.keys()

for key in before_items.keys() & after_items.keys():
    before_score = before_items[key].unified_score.final_score
    after_score = after_items[key].unified_score.final_score
    if after_score < before_score:
        improved.append((key, before_score, after_score))
    elif after_score > before_score:
        regressed.append((key, before_score, after_score))

# Generate summary statistics
total_before = sum(item.unified_score.final_score for item in before_data.items)
total_after = sum(item.unified_score.final_score for item in after_data.items)
improvement_pct = ((total_before - total_after) / total_before) * 100

# Format output
print(f"- Total debt score: {total_before} → {total_after} (-{improvement_pct:.0f}%)")
print(f"- Items resolved: {len(resolved)} of {successful + failed} targeted")
# ... continue formatting
```

## Integration Notes
This command is designed to be called from the reduce phase of the MapReduce workflow. The output should be captured and included in the final commit message to document the quantitative improvements achieved.