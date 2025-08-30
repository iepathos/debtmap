import json
import sys

# Load the JSON files
with open('debtmap.json', 'r') as f:
    before = json.load(f)
    
with open('debtmap-after.json', 'r') as f:
    after = json.load(f)

# Create lookup maps by file and function
flex_before = {}
for item in before['items']:
    key = (item['location']['file'], item.get('function_name', 'unknown'))
    if key not in flex_before:
        flex_before[key] = []
    flex_before[key].append(item)

flex_after = {}  
for item in after['items']:
    key = (item['location']['file'], item.get('function_name', 'unknown'))
    if key not in flex_after:
        flex_after[key] = []
    flex_after[key].append(item)

# Analyze changes
resolved = []
improved = []
regressed = []
unchanged = []
new_items = []

# Check each before item
for key, items_list in flex_before.items():
    if key not in flex_after:
        # Items completely resolved
        for item in items_list:
            resolved.append((key, sum(i['unified_score']['final_score'] for i in items_list)))
    else:
        # Compare scores
        before_score = sum(item['unified_score']['final_score'] for item in items_list)
        after_score = sum(item['unified_score']['final_score'] for item in flex_after[key])
        
        if after_score < before_score - 0.01:
            improved.append({
                'key': key,
                'before_score': before_score,
                'after_score': after_score,
                'improvement': before_score - after_score,
                'pct': ((before_score - after_score) / before_score * 100) if before_score > 0 else 0
            })
        elif after_score > before_score + 0.01:
            regressed.append({
                'key': key,
                'before_score': before_score,
                'after_score': after_score,
                'regression': after_score - before_score,
                'pct': ((after_score - before_score) / before_score * 100) if before_score > 0 else 0
            })
        else:
            unchanged.append(key)

# Find new items
for key in flex_after:
    if key not in flex_before:
        score = sum(item['unified_score']['final_score'] for item in flex_after[key])
        new_items.append((key, score))

# Calculate totals
total_before = sum(item['unified_score']['final_score'] for item in before['items'])
total_after = sum(item['unified_score']['final_score'] for item in after['items'])

# Output results
print("## Overall Metrics")
print(f"Total debt score: {total_before:.1f} → {total_after:.1f} ({(total_after - total_before)/total_before*100:+.1f}%)")
print(f"Total items: {len(before['items'])} → {len(after['items'])} ({len(after['items']) - len(before['items']):+d})")
print()

print("## Change Summary")
print(f"Items resolved: {len(resolved)}")
print(f"Items improved: {len(improved)}")
print(f"Items regressed: {len(regressed)}")
print(f"Items unchanged: {len(unchanged)}")
print(f"New items: {len(new_items)}")
print()

if improved:
    print("## Top Improvements")
    sorted_improved = sorted(improved, key=lambda x: x['improvement'], reverse=True)[:10]
    for i, item in enumerate(sorted_improved, 1):
        file, func = item['key']
        print(f"{i}. {file}::{func}: {item['before_score']:.1f} → {item['after_score']:.1f} (-{item['pct']:.0f}%)")
    print()

if resolved:
    print("## Items Resolved")
    sorted_resolved = sorted(resolved, key=lambda x: x[1], reverse=True)[:5]
    for i, (key, score) in enumerate(sorted_resolved, 1):
        file, func = key
        print(f"{i}. {file}::{func}: score {score:.1f} → 0 (resolved)")
    print()

if regressed:
    print("## ⚠️ Regressions")
    sorted_regressed = sorted(regressed, key=lambda x: x['regression'], reverse=True)[:5]
    for i, item in enumerate(sorted_regressed, 1):
        file, func = item['key']
        print(f"{i}. {file}::{func}: {item['before_score']:.1f} → {item['after_score']:.1f} (+{item['pct']:.0f}%)")
    print()

if new_items:
    print("## New Items Introduced")
    sorted_new = sorted(new_items, key=lambda x: x[1], reverse=True)[:5]
    for i, (key, score) in enumerate(sorted_new, 1):
        file, func = key
        print(f"{i}. {file}::{func}: new score {score:.1f}")

# Calculate success rate from command args
import sys
args = ' '.join(sys.argv[1:])
if '--successful' in args and '--failed' in args:
    try:
        # Parse command line args manually
        parts = args.split()
        succ_idx = parts.index('--successful') + 1
        fail_idx = parts.index('--failed') + 1
        successful = int(parts[succ_idx]) if succ_idx < len(parts) else 0
        failed = int(parts[fail_idx]) if fail_idx < len(parts) else 0
        total = successful + failed
        if total > 0:
            print()
            print(f"## MapReduce Results")
            print(f"Processed {total} debt items in parallel:")
            print(f"- Successfully fixed: {successful} items")
            print(f"- Failed to fix: {failed} items")
            print(f"- Success rate: {successful/total*100:.0f}%")
    except:
        pass
