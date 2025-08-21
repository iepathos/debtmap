import json

# Load the JSON files
with open('debtmap.json', 'r') as f:
    before = json.load(f)
    
with open('debtmap-after.json', 'r') as f:
    after = json.load(f)

# Create lookup maps by file and function
before_items = {}
after_items = {}

for item in before['items']:
    location = item['location']
    key = (location['file'], location.get('function', 'global'))
    before_items[key] = item
    
for item in after['items']:
    location = item['location']
    key = (location['file'], location.get('function', 'global'))
    after_items[key] = item

# Find resolved, improved, regressed items
resolved = []
improved = []
regressed = []
new_items = []

before_keys = set(before_items.keys())
after_keys = set(after_items.keys())

# Items completely resolved
for key in before_keys - after_keys:
    item = before_items[key]
    resolved.append((key, item['unified_score']['final_score']))

# New items introduced
for key in after_keys - before_keys:
    item = after_items[key]
    new_items.append((key, item['unified_score']['final_score']))

# Items that changed
for key in before_keys & after_keys:
    before_score = before_items[key]['unified_score']['final_score']
    after_score = after_items[key]['unified_score']['final_score']
    
    if abs(before_score - after_score) > 0.01:  # Significant change
        if after_score < before_score:
            improved.append((key, before_score, after_score, before_score - after_score))
        else:
            regressed.append((key, before_score, after_score, after_score - before_score))

# Calculate metrics
total_before = before['total_debt_score']
total_after = after['total_debt_score']
improvement = total_before - total_after
improvement_pct = (improvement / total_before) * 100 if total_before > 0 else 0

items_before = len(before['items'])
items_after = len(after['items'])
items_change = items_before - items_after
items_change_pct = (items_change / items_before) * 100 if items_before > 0 else 0

# Print summary
print("## Technical Debt Improvements\n")
print(f"**Overall Impact:**")
print(f"- Total debt score: {total_before:.1f} → {total_after:.1f} ({improvement_pct:+.1f}%)")
print(f"- Total items: {items_before} → {items_after} ({items_change_pct:+.1f}%)")
print()

print(f"**Changes:**")
print(f"- Items resolved: {len(resolved)}")
print(f"- Items improved: {len(improved)}")
if regressed:
    print(f"- Items regressed: {len(regressed)}")
if new_items:
    print(f"- New items: {len(new_items)}")

if resolved and len(resolved) > 0:
    print("\n**Top Resolved Items:**")
    sorted_resolved = sorted(resolved, key=lambda x: x[1], reverse=True)[:5]
    for i, (key, score) in enumerate(sorted_resolved, 1):
        file, func = key
        file = file.replace('./', '')
        print(f"{i}. `{file}::{func}`: {score:.1f} → 0")

if improved and len(improved) > 0:
    print("\n**Top Improvements:**")
    sorted_improved = sorted(improved, key=lambda x: x[3], reverse=True)[:5]
    for i, (key, before_s, after_s, diff) in enumerate(sorted_improved, 1):
        file, func = key
        file = file.replace('./', '')
        pct = (diff / before_s) * 100
        print(f"{i}. `{file}::{func}`: {before_s:.1f} → {after_s:.1f} (-{pct:.0f}%)")

if regressed and len(regressed) > 0:
    print("\n**⚠️ Regressions:**")
    sorted_regressed = sorted(regressed, key=lambda x: x[3], reverse=True)[:3]
    for key, before_s, after_s, diff in sorted_regressed:
        file, func = key
        file = file.replace('./', '')
        pct = (diff / before_s) * 100 if before_s > 0 else 100
        print(f"- `{file}::{func}`: {before_s:.1f} → {after_s:.1f} (+{pct:.0f}%)")

if new_items and len(new_items) > 0:
    high_score_new = [item for item in new_items if item[1] > 5.0]
    if high_score_new:
        print(f"\n**⚠️ New High-Debt Items ({len(high_score_new)} of {len(new_items)} total):**")
        sorted_new = sorted(high_score_new, key=lambda x: x[1], reverse=True)[:3]
        for key, score in sorted_new:
            file, func = key
            file = file.replace('./', '')
            print(f"- `{file}::{func}`: {score:.1f}")

# Print overall assessment
print("\n---")
if improvement > 0:
    print(f"✅ **Net improvement: {improvement:.1f} points reduced (-{improvement_pct:.1f}%)**")
    if len(resolved) > 0:
        print(f"✅ Successfully resolved {len(resolved)} technical debt items")
    if len(improved) > 0:
        print(f"✅ Improved {len(improved)} additional items")
elif improvement < 0:
    print(f"⚠️ **Net regression: {abs(improvement):.1f} points increased (+{abs(improvement_pct):.1f}%)**")
else:
    print(f"➖ **No net change in technical debt score**")

