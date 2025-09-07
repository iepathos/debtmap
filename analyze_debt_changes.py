#!/usr/bin/env python3
import json
import sys
from collections import defaultdict

def load_json_file(filepath):
    with open(filepath, 'r') as f:
        return json.load(f)

def analyze_debt_changes(before_data, after_data):
    # Create lookup maps by file and function
    before_items = {}
    after_items = {}
    
    for item in before_data.get('items', []):
        key = (item['location']['file'], item['location'].get('function', 'unknown'))
        before_items[key] = item
    
    for item in after_data.get('items', []):
        key = (item['location']['file'], item['location'].get('function', 'unknown'))
        after_items[key] = item
    
    # Find resolved, improved, regressed, and new items
    resolved = []
    improved = []
    regressed = []
    unchanged = []
    
    for key in before_items:
        before_score = before_items[key]['unified_score']['final_score']
        if key not in after_items:
            resolved.append((key, before_score))
        else:
            after_score = after_items[key]['unified_score']['final_score']
            if after_score < before_score:
                improved.append((key, before_score, after_score))
            elif after_score > before_score:
                regressed.append((key, before_score, after_score))
            else:
                unchanged.append((key, before_score))
    
    new_items = []
    for key in after_items:
        if key not in before_items:
            new_items.append((key, after_items[key]['unified_score']['final_score']))
    
    # Calculate category improvements
    category_changes = defaultdict(lambda: {'before': 0, 'after': 0})
    
    for item in before_data.get('items', []):
        for category in item.get('debt_categories', []):
            category_changes[category]['before'] += 1
    
    for item in after_data.get('items', []):
        for category in item.get('debt_categories', []):
            category_changes[category]['after'] += 1
    
    return {
        'resolved': sorted(resolved, key=lambda x: x[1], reverse=True),
        'improved': sorted(improved, key=lambda x: x[1] - x[2], reverse=True),
        'regressed': sorted(regressed, key=lambda x: x[2] - x[1], reverse=True),
        'unchanged': unchanged,
        'new_items': sorted(new_items, key=lambda x: x[1], reverse=True),
        'category_changes': dict(category_changes)
    }

def main():
    before_data = load_json_file('debtmap.json')
    after_data = load_json_file('debtmap-after.json')
    
    # Overall metrics
    before_score = before_data.get('total_debt_score', 0)
    after_score = after_data.get('total_debt_score', 0)
    before_items = len(before_data.get('items', []))
    after_items = len(after_data.get('items', []))
    
    score_improvement = before_score - after_score
    score_improvement_pct = (score_improvement / before_score) * 100 if before_score > 0 else 0
    
    print(f"Overall Metrics:")
    print(f"- Total debt score: {before_score:.2f} → {after_score:.2f} (-{score_improvement_pct:.1f}%)")
    print(f"- Total items: {before_items} → {after_items} ({after_items - before_items:+d})")
    print()
    
    # Analyze changes
    changes = analyze_debt_changes(before_data, after_data)
    
    print(f"Item-Level Changes:")
    print(f"- Resolved: {len(changes['resolved'])} items")
    print(f"- Improved: {len(changes['improved'])} items")
    print(f"- Regressed: {len(changes['regressed'])} items")
    print(f"- Unchanged: {len(changes['unchanged'])} items")
    print(f"- New: {len(changes['new_items'])} items")
    print()
    
    if changes['resolved']:
        print("Top 5 Resolved Items:")
        for (file, func), score in changes['resolved'][:5]:
            print(f"  - {file}::{func}: score {score:.2f} → 0 (resolved)")
        print()
    
    if changes['improved']:
        print("Top 5 Improvements:")
        for (file, func), before, after in changes['improved'][:5]:
            improvement_pct = ((before - after) / before) * 100
            print(f"  - {file}::{func}: score {before:.2f} → {after:.2f} (-{improvement_pct:.0f}%)")
        print()
    
    if changes['regressed']:
        print("⚠️ Regressions Detected:")
        for (file, func), before, after in changes['regressed'][:5]:
            regression_pct = ((after - before) / before) * 100
            print(f"  - {file}::{func}: score {before:.2f} → {after:.2f} (+{regression_pct:.0f}%)")
        print()
    
    if changes['new_items'] and len(changes['new_items']) > 0:
        high_score_new = [item for item in changes['new_items'] if item[1] >= 5.0]
        if high_score_new:
            print(f"⚠️ New High-Score Items ({len(high_score_new)} with score ≥ 5.0):")
            for (file, func), score in high_score_new[:5]:
                print(f"  - NEW: {file}::{func}: score {score:.2f}")
            print()
    
    # Category analysis
    if changes['category_changes']:
        print("Changes by Category:")
        for category, counts in sorted(changes['category_changes'].items()):
            before_count = counts['before']
            after_count = counts['after']
            change = after_count - before_count
            if before_count > 0:
                change_pct = (change / before_count) * 100
                print(f"  - {category}: {before_count} → {after_count} ({change_pct:+.0f}%)")
            else:
                print(f"  - {category}: {before_count} → {after_count} (new)")
    
    # Impact analysis
    before_impact = before_data.get('total_impact', {})
    after_impact = after_data.get('total_impact', {})
    
    print()
    print("Impact Analysis:")
    print(f"  - Complexity reduction: {before_impact.get('complexity_reduction', 0):.1f} → {after_impact.get('complexity_reduction', 0):.1f}")
    print(f"  - Risk reduction: {before_impact.get('risk_reduction', 0):.1f} → {after_impact.get('risk_reduction', 0):.1f}")
    print(f"  - Coverage improvement: {before_impact.get('coverage_improvement', 0):.1f} → {after_impact.get('coverage_improvement', 0):.1f}")

if __name__ == "__main__":
    main()