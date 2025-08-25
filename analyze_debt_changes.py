import json
import sys

# Load the JSON files
with open('debtmap.json', 'r') as f:
    before = json.load(f)
    
with open('debtmap-after.json', 'r') as f:
    after = json.load(f)

# Create lookup maps by location
before_items = {}
for item in before['items']:
    key = (item['location']['file'], item['location']['function'])
    before_items[key] = item

after_items = {}
for item in after['items']:
    key = (item['location']['file'], item['location']['function'])
    after_items[key] = item

# Analyze changes
resolved = []
improved = []
regressed = []
unchanged = []
new_items = []

# Find resolved items (in before but not in after)
for key in before_items:
    if key not in after_items:
        resolved.append((key, before_items[key]['unified_score']['final_score']))

# Find new items (in after but not in before)
for key in after_items:
    if key not in before_items:
        new_items.append((key, after_items[key]['unified_score']['final_score']))

# Compare items that exist in both
for key in before_items:
    if key in after_items:
        before_score = before_items[key]['unified_score']['final_score']
        after_score = after_items[key]['unified_score']['final_score']
        
        if abs(before_score - after_score) < 0.01:  # Essentially unchanged
            unchanged.append((key, before_score))
        elif after_score < before_score:
            improved.append((key, before_score, after_score))
        else:
            regressed.append((key, before_score, after_score))

# Calculate category improvements
categories = ['complexity_factor', 'coverage_factor', 'dependency_factor', 'security_factor']
category_changes = {}

for cat in categories:
    before_total = sum(item['unified_score'].get(cat, 0) for item in before['items'])
    after_total = sum(item['unified_score'].get(cat, 0) for item in after['items'])
    category_changes[cat] = (before_total, after_total)

# Print results
print("=== ANALYSIS RESULTS ===")
print(f"\nItems resolved (removed): {len(resolved)}")
if resolved:
    top_resolved = sorted(resolved, key=lambda x: x[1], reverse=True)[:3]
    for (file, func), score in top_resolved:
        print(f"  - {file}::{func}: score {score:.1f} → 0")

print(f"\nItems improved: {len(improved)}")
if improved:
    top_improved = sorted(improved, key=lambda x: x[1] - x[2], reverse=True)[:3]
    for (file, func), before_score, after_score in top_improved:
        pct = ((before_score - after_score) / before_score) * 100
        print(f"  - {file}::{func}: {before_score:.1f} → {after_score:.1f} (-{pct:.0f}%)")

print(f"\nItems regressed: {len(regressed)}")
if regressed:
    top_regressed = sorted(regressed, key=lambda x: x[2] - x[1], reverse=True)[:3]
    for (file, func), before_score, after_score in top_regressed:
        pct = ((after_score - before_score) / before_score) * 100
        print(f"  - {file}::{func}: {before_score:.1f} → {after_score:.1f} (+{pct:.0f}%)")

print(f"\nNew items added: {len(new_items)}")
if new_items:
    top_new = sorted(new_items, key=lambda x: x[1], reverse=True)[:3]
    for (file, func), score in top_new:
        print(f"  - {file}::{func}: score {score:.1f}")

print(f"\nItems unchanged: {len(unchanged)}")

print("\n=== CATEGORY ANALYSIS ===")
for cat, (before_val, after_val) in category_changes.items():
    if before_val > 0:
        change_pct = ((before_val - after_val) / before_val) * 100
        sign = "-" if after_val < before_val else "+"
        print(f"{cat}: {before_val:.1f} → {after_val:.1f} ({sign}{abs(change_pct):.1f}%)")

# Overall summary
total_before = sum(item['unified_score']['final_score'] for item in before['items'])
total_after = sum(item['unified_score']['final_score'] for item in after['items'])
improvement_pct = ((total_before - total_after) / total_before) * 100

print("\n=== OVERALL SUMMARY ===")
print(f"Total items: {len(before['items'])} → {len(after['items'])}")
print(f"Total debt score: {total_before:.1f} → {total_after:.1f} (-{improvement_pct:.1f}%)")
