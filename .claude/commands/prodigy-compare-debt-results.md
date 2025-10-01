---
name: prodigy-compare-debt-results
description: Compare before/after debtmap results and create a commit documenting improvements
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
  - name: total
    required: false
    description: Total number of items processed in the map phase
---

# Compare Debt Results and Create Commit

## Purpose
Analyze the difference between before and after debtmap results to quantify technical debt improvements made during the MapReduce workflow, then create a git commit documenting these improvements.

## Usage
```
/prodigy-compare-debt-results --before <original-debtmap.json> --after <new-debtmap.json> --map-results '<results>' --successful <count> --failed <count>
```

## Parameters
- `--before`: Path to the original debtmap.json file
- `--after`: Path to the updated debtmap.json file after fixes
- `--map-results`: JSON results from the map phase (optional)
- `--successful`: Number of successfully fixed items
- `--failed`: Number of items that failed to fix

## Process

**CRITICAL**: Use the `debtmap compare` command for efficient comparison. This reduces processing overhead from 40MB JSON files to <10KB output (99.975% reduction).

1. **Run Debtmap Compare Command**
   ```bash
   cargo run --release -- compare \
     --before <before-path> \
     --after <after-path> \
     --output comparison-result.json \
     --format json
   ```

   This command:
   - Efficiently compares large debtmap analyses
   - Identifies resolved, improved, worsened, and new items
   - Calculates project health metrics
   - Generates summary statistics

2. **Parse Comparison Results**
   - Read the comparison-result.json output
   - Extract key metrics from structured output:
     - `project_health`: Overall project metrics before/after
     - `improvements`: Items that were resolved or improved
     - `regressions`: Items that worsened or new items added
     - `summary`: High-level statistics and status

3. **Generate Summary Report**
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

7. **Create Git Commit**
   - Stage all changes with `git add -A`
   - Create a commit with the message (explicitly without Claude signature):
   ```
   fix: eliminate <successful> technical debt items via MapReduce
   
   Processed <total> debt items in parallel:
   - Successfully fixed: <successful> items
   - Failed to fix: <failed> items
   
   Technical Debt Improvements:
   [Include the generated debt analysis summary from step 5]
   
   [Include any regressions from step 6 if present]
   
   This commit represents the aggregated work of multiple parallel agents.
   ```
   
   **IMPORTANT**: Do NOT include the Claude signature ("🤖 Generated with Claude Code" or "Co-Authored-By: Claude") in this commit message to avoid bloating the commit history.

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

## Example Implementation Using Debtmap Compare

**IMPORTANT**: Use the `debtmap compare` command instead of manual jq processing. This is more efficient and accurate.

```bash
# Step 1: Run debtmap compare
cargo run --release -- compare \
  --before "$BEFORE_PATH" \
  --after "$AFTER_PATH" \
  --output comparison-result.json \
  --format json

# Step 2: Extract summary information from comparison result
RESOLVED_COUNT=$(jq -r '.summary.resolved_count' comparison-result.json)
IMPROVED_COUNT=$(jq -r '.summary.total_improvements' comparison-result.json)
WORSENED_COUNT=$(jq -r '.summary.total_regressions' comparison-result.json)
STATUS=$(jq -r '.summary.status' comparison-result.json)

# Step 3: Extract project health metrics
BEFORE_ITEMS=$(jq -r '.project_health.total_items_before' comparison-result.json)
AFTER_ITEMS=$(jq -r '.project_health.total_items_after' comparison-result.json)
BEFORE_HIGH=$(jq -r '.project_health.high_priority_before' comparison-result.json)
AFTER_HIGH=$(jq -r '.project_health.high_priority_after' comparison-result.json)

# Step 4: Format the commit message
cat <<EOF
fix: eliminate technical debt items via MapReduce

Processed debt items in parallel:
- Successfully fixed: ${SUCCESSFUL} items
- Failed to fix: ${FAILED} items
- Total items processed: ${TOTAL}

Technical Debt Improvements:
- Total items: ${BEFORE_ITEMS} → ${AFTER_ITEMS}
- High priority items: ${BEFORE_HIGH} → ${AFTER_HIGH}
- Items resolved: ${RESOLVED_COUNT}
- Items improved: ${IMPROVED_COUNT}
- Status: ${STATUS}
EOF

# Step 5: Create the git commit
git add -A
git commit -m "$(cat comparison-result.json | jq -r '.summary.commit_message // "fix: eliminate technical debt items"')"
```

## Integration Notes
This command is designed to be called from the reduce phase of the MapReduce workflow. It will analyze the debt improvements and automatically create a git commit documenting the results.

The command always creates a commit after analysis to ensure the improvements are properly documented in the git history.