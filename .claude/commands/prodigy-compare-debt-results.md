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

**CRITICAL**: This is a Rust project. Use shell commands with jq for JSON processing. Do NOT create Python scripts.

1. **Load and Parse JSON Files**
   - Read the before and after debtmap.json files using jq
   - Parse the JSON structures with jq queries

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

## Example Implementation Using Shell and jq

**IMPORTANT**: Use shell commands with jq for JSON processing. Do NOT create Python scripts in this Rust project.

```bash
# Example shell implementation using jq
# Load total scores
BEFORE_SCORE=$(jq -r '.total_debt_score // 0' debtmap.json)
AFTER_SCORE=$(jq -r '.total_debt_score // 0' debtmap-after.json)

# Count items
BEFORE_COUNT=$(jq -r '.items | length' debtmap.json)
AFTER_COUNT=$(jq -r '.items | length' debtmap-after.json)

# Calculate percentage improvement
IMPROVEMENT=$(echo "scale=1; (($BEFORE_SCORE - $AFTER_SCORE) / $BEFORE_SCORE) * 100" | bc)

# Format the commit message
cat <<EOF
fix: eliminate technical debt items via MapReduce

Processed debt items in parallel:
- Successfully fixed: ${SUCCESSFUL} items  
- Failed to fix: ${FAILED} items
- Total items processed: ${TOTAL}

Technical Debt Improvements:
- Total debt score: ${BEFORE_SCORE} → ${AFTER_SCORE} (-${IMPROVEMENT}%)
- Total items: ${BEFORE_COUNT} → ${AFTER_COUNT}
EOF

# Create the git commit
git add -A
git commit -m "$(cat <<EOF
fix: eliminate technical debt items via MapReduce

[Include formatted summary here]
EOF
)"
```

## Integration Notes
This command is designed to be called from the reduce phase of the MapReduce workflow. It will analyze the debt improvements and automatically create a git commit documenting the results.

The command always creates a commit after analysis to ensure the improvements are properly documented in the git history.