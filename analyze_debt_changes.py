import json

# Load both files
with open('debtmap.json', 'r') as f:
    before_data = json.load(f)

with open('debtmap-after.json', 'r') as f:
    after_data = json.load(f)

# Create lookup maps by location
def make_key(item):
    loc = item['location']
    return (loc['file'], loc.get('function', ''), loc.get('line', 0))

before_items = {make_key(item): item for item in before_data['items']}
after_items = {make_key(item): item for item in after_data['items']}

# Calculate totals
total_before = sum(item['unified_score']['final_score'] for item in before_data['items'])
total_after = sum(item['unified_score']['final_score'] for item in after_data['items'])
improvement_pct = ((total_before - total_after) / total_before) * 100 if total_before > 0 else 0

# Find changes
resolved = []
improved = []
regressed = []

for key, before_item in before_items.items():
    before_score = before_item['unified_score']['final_score']
    
    if key not in after_items:
        resolved.append((key, before_score))
    else:
        after_score = after_items[key]['unified_score']['final_score']
        if after_score < before_score - 0.01:
            improved.append((key, before_score, after_score))
        elif after_score > before_score + 0.01:
            regressed.append((key, before_score, after_score))

# New items
new_items = []
for key in after_items:
    if key not in before_items:
        new_items.append((key, after_items[key]['unified_score']['final_score']))

# Sort by improvement amount
resolved.sort(key=lambda x: x[1], reverse=True)
improved.sort(key=lambda x: x[1] - x[2], reverse=True)
new_items.sort(key=lambda x: x[1], reverse=True)

print(f"Total debt score: {total_before:.0f} → {total_after:.0f} (-{improvement_pct:.0f}%)")
print(f"Items resolved: {len(resolved)}")
print(f"Items improved: {len(improved)}")
print(f"Overall items: {len(before_items)} → {len(after_items)} ({len(after_items) - len(before_items):+d})")

if resolved:
    print("\nTop resolved items:")
    for (file, func, line), score in resolved[:3]:
        file_short = file.split('/')[-1] if '/' in file else file
        print(f"  - {file_short}::{func or 'module'}: score {score:.0f} → 0")

if improved:
    print("\nTop improvements:")
    for (file, func, line), before, after in improved[:3]:
        file_short = file.split('/')[-1] if '/' in file else file
        pct = ((before - after) / before) * 100
        print(f"  - {file_short}::{func or 'module'}: score {before:.0f} → {after:.0f} (-{pct:.0f}%)")

if regressed and len(regressed) > 0:
    print("\n⚠️ Regressions detected:")
    for (file, func, line), before, after in regressed[:2]:
        file_short = file.split('/')[-1] if '/' in file else file
        pct = ((after - before) / before) * 100
        print(f"  - {file_short}::{func or 'module'}: score {before:.0f} → {after:.0f} (+{pct:.0f}%)")
