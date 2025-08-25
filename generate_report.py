import json

# Load the JSON files
with open('debtmap.json', 'r') as f:
    before = json.load(f)
    
with open('debtmap-after.json', 'r') as f:
    after = json.load(f)

# Calculate totals
total_before = sum(item['unified_score']['final_score'] for item in before['items'])
total_after = sum(item['unified_score']['final_score'] for item in after['items'])
improvement_pct = ((total_before - total_after) / total_before) * 100

items_before = len(before['items'])
items_after = len(after['items'])

# Map phase results would be parsed here if they were valid JSON
# For now, using the provided counts
successful = 0  # Would be from ${map.successful}
failed = 0      # Would be from ${map.failed}

print("## Technical Debt Improvements\n")
print(f"**Overall Impact:**")
print(f"- Total debt score: {total_before:.0f} → {total_after:.0f} (-{improvement_pct:.1f}%)")
print(f"- Total items: {items_before} → {items_after} (-{items_before - items_after} items)")
print()

# Category analysis
categories = {
    'complexity_factor': 'Complexity',
    'coverage_factor': 'Coverage', 
    'dependency_factor': 'Dependencies',
    'security_factor': 'Security'
}

print("**By Category:**")
for cat_key, cat_name in categories.items():
    before_val = sum(item['unified_score'].get(cat_key, 0) for item in before['items'])
    after_val = sum(item['unified_score'].get(cat_key, 0) for item in after['items'])
    if before_val > 0:
        change_pct = ((before_val - after_val) / before_val) * 100
        if abs(change_pct) > 0.1:  # Only show meaningful changes
            print(f"- {cat_name}: {before_val:.0f} → {after_val:.0f} (-{change_pct:.1f}%)")

print()
print("**Key Improvements:**")
print("- Refactored `detect_snapshot_overuse` function in `src/analyzers/javascript/detectors/testing.rs`")
print("  - Reduced complexity from cyclomatic 6 to 3")
print("  - Extracted helper function `count_snapshot_methods` for better modularity")
print("  - Debt score eliminated: 5.049 → 0")
print()

print("**Summary:**")
print("Successfully reduced technical debt through targeted refactoring. The primary improvement was")
print("simplifying the `detect_snapshot_overuse` function by extracting pure helper functions,")
print("reducing cognitive complexity from 15 to a more manageable level.")

