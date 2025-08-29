import json
import sys

# Load the JSON files
with open('debtmap.json', 'r') as f:
    before = json.load(f)
    
with open('debtmap-after.json', 'r') as f:
    after = json.load(f)

# Create lookup maps by file and function
def create_lookup(data):
    lookup = {}
    for item in data['items']:
        key = (item['location']['file'], item.get('location', {}).get('function', 'unknown'))
        lookup[key] = item
    return lookup

before_items = create_lookup(before)
after_items = create_lookup(after)

# Find resolved, improved, regressed items
resolved = []
improved = []
regressed = []
unchanged = []

for key, before_item in before_items.items():
    before_score = before_item['unified_score']['final_score']
    if key not in after_items:
        resolved.append((key, before_score, before_item))
    else:
        after_score = after_items[key]['unified_score']['final_score']
        if abs(after_score - before_score) > 0.01:  # Threshold for significance
            if after_score < before_score:
                improved.append((key, before_score, after_score, before_item, after_items[key]))
            else:
                regressed.append((key, before_score, after_score, before_item, after_items[key]))
        else:
            unchanged.append(key)

# Find new items
new_items = []
for key, after_item in after_items.items():
    if key not in before_items:
        new_items.append((key, after_item['unified_score']['final_score'], after_item))

# Sort by improvement/score
improved.sort(key=lambda x: x[1] - x[2], reverse=True)
regressed.sort(key=lambda x: x[2] - x[1], reverse=True)
resolved.sort(key=lambda x: x[1], reverse=True)
new_items.sort(key=lambda x: x[1], reverse=True)

# Calculate debt type improvements
def get_debt_type_name(debt_type):
    if isinstance(debt_type, dict):
        return list(debt_type.keys())[0] if debt_type else 'unknown'
    return str(debt_type)

def sum_by_debt_type(items_dict):
    types = {}
    for item in items_dict.values():
        debt_type_name = get_debt_type_name(item.get('debt_type', 'unknown'))
        if debt_type_name not in types:
            types[debt_type_name] = {'score': 0, 'count': 0}
        types[debt_type_name]['score'] += item['unified_score']['final_score']
        types[debt_type_name]['count'] += 1
    return types

before_types = sum_by_debt_type(before_items)
after_types = sum_by_debt_type(after_items)

# Output results
print("Technical Debt Improvements:")
debt_change = after['total_debt_score'] - before['total_debt_score']
debt_pct = (debt_change / before['total_debt_score'] * 100) if before['total_debt_score'] > 0 else 0
print(f"- Total debt score: {before['total_debt_score']:.0f} → {after['total_debt_score']:.0f} ({debt_pct:+.0f}%)")
print(f"- Items resolved: {len(resolved)} of {len(before_items)} targeted")
print(f"- Overall items: {len(before_items)} → {len(after_items)} ({len(after_items) - len(before_items):+d})")

print("\nBy category:")
all_types = set(before_types.keys()) | set(after_types.keys())
type_improvements = []
for debt_type in sorted(all_types):
    before_val = before_types.get(debt_type, {'score': 0})['score']
    after_val = after_types.get(debt_type, {'score': 0})['score']
    if before_val > 0:
        change = after_val - before_val
        pct = (change / before_val * 100)
        type_improvements.append((debt_type, before_val, after_val, change, pct))

# Sort by improvement percentage
type_improvements.sort(key=lambda x: x[4])
for debt_type, before_val, after_val, change, pct in type_improvements[:5]:
    before_count = before_types.get(debt_type, {'count': 0})['count']
    after_count = after_types.get(debt_type, {'count': 0})['count']
    count_change = after_count - before_count
    print(f"- {debt_type}: {pct:+.0f}% ({count_change:+d} items)")

if improved:
    print("\nTop improvements:")
    for i, (key, before_score, after_score, before_item, after_item) in enumerate(improved[:3], 1):
        file, func = key
        improvement = before_score - after_score
        pct = (improvement / before_score * 100) if before_score > 0 else 0
        print(f"{i}. {file}::{func}: score {before_score:.0f} → {after_score:.0f} (-{pct:.0f}%)")

if resolved:
    print("\nResolved items:")
    for i, (key, score, item) in enumerate(resolved[:3], 1):
        file, func = key
        print(f"{i}. {file}::{func}: score {score:.0f} → 0 (resolved)")

if regressed and len(regressed) > 0:
    print("\n⚠️ Regressions detected:")
    for i, (key, before_score, after_score, before_item, after_item) in enumerate(regressed[:3], 1):
        file, func = key
        increase = after_score - before_score
        pct = (increase / before_score * 100) if before_score > 0 else 0
        print(f"- {file}::{func}: score {before_score:.0f} → {after_score:.0f} (+{pct:.0f}%)")

# Summary statistics
total_improvement = sum(b - a for _, b, a, _, _ in improved)
total_regression = sum(a - b for _, b, a, _, _ in regressed)
total_resolved = sum(score for _, score, _ in resolved)
net_improvement = total_improvement + total_resolved - total_regression

print(f"\nNet improvement: {net_improvement:.1f} debt points")
print(f"Success rate: {len(improved)}/{len(improved) + len(regressed) + len(unchanged)} items improved")
