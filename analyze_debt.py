#!/usr/bin/env python3
import json
import sys
from collections import defaultdict

def load_json(path):
    with open(path, 'r') as f:
        return json.load(f)

def main():
    before = load_json('debtmap.json')
    after = load_json('debtmap-after.json')
    
    # Create lookups by file:function
    before_items = {}
    for item in before['items']:
        key = f"{item.get('file', 'unknown')}:{item.get('function', 'unknown')}"
        before_items[key] = item
    
    after_items = {}
    for item in after['items']:
        key = f"{item.get('file', 'unknown')}:{item.get('function', 'unknown')}"
        after_items[key] = item
    
    # Calculate metrics
    total_before = sum(item['unified_score']['final_score'] for item in before['items'])
    total_after = sum(item['unified_score']['final_score'] for item in after['items'])
    
    before_keys = set(before_items.keys())
    after_keys = set(after_items.keys())
    
    resolved = before_keys - after_keys
    new_items = after_keys - before_keys
    common = before_keys & after_keys
    
    improved = []
    regressed = []
    unchanged = []
    
    for key in common:
        before_score = before_items[key]['unified_score']['final_score']
        after_score = after_items[key]['unified_score']['final_score']
        
        if after_score < before_score - 0.01:  # Small tolerance for floating point
            improved.append((key, before_score, after_score))
        elif after_score > before_score + 0.01:
            regressed.append((key, before_score, after_score))
        else:
            unchanged.append(key)
    
    # Sort by improvement amount
    improved.sort(key=lambda x: x[1] - x[2], reverse=True)
    regressed.sort(key=lambda x: x[2] - x[1], reverse=True)
    
    # Print results
    print(f"Overall Metrics:")
    print(f"- Total debt score: {total_before:.1f} → {total_after:.1f} ({(total_after - total_before):.1f}, {((total_after - total_before) / total_before * 100):.1f}%)")
    print(f"- Total items: {len(before['items'])} → {len(after['items'])} ({len(after['items']) - len(before['items'])})")
    print(f"")
    print(f"Item Changes:")
    print(f"- Resolved items: {len(resolved)}")
    print(f"- New items: {len(new_items)}")
    print(f"- Improved items: {len(improved)}")
    print(f"- Regressed items: {len(regressed)}")
    print(f"- Unchanged items: {len(unchanged)}")
    
    if improved:
        print(f"\nTop 5 Improvements:")
        for i, (key, before_score, after_score) in enumerate(improved[:5], 1):
            improvement_pct = (before_score - after_score) / before_score * 100
            print(f"  {i}. {key}: {before_score:.1f} → {after_score:.1f} (-{improvement_pct:.1f}%)")
    
    if regressed:
        print(f"\n⚠️ Top Regressions:")
        for i, (key, before_score, after_score) in enumerate(regressed[:5], 1):
            regression_pct = (after_score - before_score) / before_score * 100
            print(f"  {i}. {key}: {before_score:.1f} → {after_score:.1f} (+{regression_pct:.1f}%)")
    
    if resolved:
        print(f"\nResolved Items (sample):")
        for key in list(resolved)[:5]:
            if key in before_items:
                score = before_items[key]['unified_score']['final_score']
                print(f"  - {key}: score {score:.1f} (removed)")
    
    if new_items:
        print(f"\nNew Items (sample):")
        for key in list(new_items)[:5]:
            if key in after_items:
                score = after_items[key]['unified_score']['final_score']
                print(f"  - {key}: score {score:.1f} (added)")

if __name__ == '__main__':
    main()