#!/usr/bin/env python3
import json
import sys
from collections import defaultdict
from typing import Dict, List, Tuple, Set

def load_debtmap(filepath: str) -> dict:
    """Load and parse a debtmap JSON file."""
    with open(filepath, 'r') as f:
        return json.load(f)

def create_item_key(item: dict) -> tuple:
    """Create a unique key for an item based on location."""
    location = item.get('location', {})
    return (
        location.get('file', ''),
        location.get('function', ''),
        location.get('line', 0)
    )

def analyze_debt_changes(before_path: str, after_path: str):
    """Analyze changes between before and after debtmap files."""
    
    # Load data
    print(f"Loading {before_path}...")
    before_data = load_debtmap(before_path)
    print(f"Loading {after_path}...")
    after_data = load_debtmap(after_path)
    
    # Extract items
    before_items = before_data.get('items', [])
    after_items = after_data.get('items', [])
    
    print(f"\nTotal items before: {len(before_items)}")
    print(f"Total items after: {len(after_items)}")
    
    # Create lookups by file and function (ignore line numbers as they may change)
    before_map = {}
    for item in before_items:
        location = item.get('location', {})
        key = (location.get('file', ''), location.get('function', ''))
        before_map[key] = item
    
    after_map = {}
    for item in after_items:
        location = item.get('location', {})
        key = (location.get('file', ''), location.get('function', ''))
        after_map[key] = item
    
    # Calculate changes
    resolved_items = []
    improved_items = []
    regressed_items = []
    new_items = []
    
    # Find resolved and improved items
    for key, before_item in before_map.items():
        before_score = before_item.get('unified_score', {}).get('final_score', 0)
        
        if key not in after_map:
            resolved_items.append((key, before_item, before_score))
        else:
            after_item = after_map[key]
            after_score = after_item.get('unified_score', {}).get('final_score', 0)
            
            if after_score < before_score:
                improved_items.append((key, before_item, after_item, before_score, after_score))
            elif after_score > before_score:
                regressed_items.append((key, before_item, after_item, before_score, after_score))
    
    # Find new items
    for key, after_item in after_map.items():
        if key not in before_map:
            after_score = after_item.get('unified_score', {}).get('final_score', 0)
            new_items.append((key, after_item, after_score))
    
    # Calculate total scores
    total_before = sum(item.get('unified_score', {}).get('final_score', 0) for item in before_items)
    total_after = sum(item.get('unified_score', {}).get('final_score', 0) for item in after_items)
    
    # Category analysis
    category_scores_before = defaultdict(float)
    category_scores_after = defaultdict(float)
    
    for item in before_items:
        for category in item.get('categories', []):
            category_scores_before[category] += item.get('unified_score', {}).get('final_score', 0)
    
    for item in after_items:
        for category in item.get('categories', []):
            category_scores_after[category] += item.get('unified_score', {}).get('final_score', 0)
    
    # Generate report
    print("\n" + "="*60)
    print("TECHNICAL DEBT ANALYSIS REPORT")
    print("="*60)
    
    print("\n## Overall Metrics")
    if total_before > 0:
        improvement_pct = ((total_before - total_after) / total_before) * 100
        print(f"- Total debt score: {total_before:.0f} → {total_after:.0f} ({improvement_pct:+.1f}%)")
    else:
        print(f"- Total debt score: {total_before:.0f} → {total_after:.0f}")
    
    print(f"- Total items: {len(before_items)} → {len(after_items)} ({len(after_items) - len(before_items):+d})")
    print(f"- Items resolved: {len(resolved_items)}")
    print(f"- Items improved: {len(improved_items)}")
    print(f"- Items regressed: {len(regressed_items)}")
    print(f"- New items: {len(new_items)}")
    
    print("\n## Category Analysis")
    for category in sorted(set(category_scores_before.keys()) | set(category_scores_after.keys())):
        before = category_scores_before.get(category, 0)
        after = category_scores_after.get(category, 0)
        if before > 0:
            change_pct = ((before - after) / before) * 100
            print(f"- {category}: {before:.0f} → {after:.0f} ({change_pct:+.1f}%)")
        else:
            print(f"- {category}: {before:.0f} → {after:.0f}")
    
    print("\n## Top Improvements")
    # Sort by score reduction
    improvements = sorted(resolved_items + improved_items, 
                         key=lambda x: x[2] if len(x) == 3 else x[3] - x[4], 
                         reverse=True)[:10]
    
    for i, item in enumerate(improvements[:5], 1):
        if len(item) == 3:  # Resolved
            key, before_item, score = item
            print(f"{i}. {key[0]}::{key[1]}: score {score:.0f} → 0 (resolved)")
        else:  # Improved
            key, before_item, after_item, before_score, after_score = item
            reduction_pct = ((before_score - after_score) / before_score) * 100
            print(f"{i}. {key[0]}::{key[1]}: score {before_score:.0f} → {after_score:.0f} (-{reduction_pct:.0f}%)")
    
    if regressed_items:
        print("\n## ⚠️ Regressions Detected")
        for key, before_item, after_item, before_score, after_score in regressed_items[:5]:
            increase_pct = ((after_score - before_score) / before_score) * 100
            print(f"- {key[0]}::{key[1]}: score {before_score:.0f} → {after_score:.0f} (+{increase_pct:.0f}%)")
    
    if new_items:
        high_score_new = sorted(new_items, key=lambda x: x[2], reverse=True)[:3]
        if any(score > 30 for _, _, score in high_score_new):
            print("\n## New High-Score Items")
            for key, item, score in high_score_new:
                if score > 30:
                    print(f"- NEW: {key[0]}::{key[1]}: score {score:.0f}")
    
    # Return summary for commit message
    return {
        'total_before': total_before,
        'total_after': total_after,
        'items_before': len(before_items),
        'items_after': len(after_items),
        'resolved': len(resolved_items),
        'improved': len(improved_items),
        'regressed': len(regressed_items),
        'new_items': len(new_items),
        'top_improvements': improvements[:3],
        'has_regressions': len(regressed_items) > 0
    }

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: python analyze_debt_improvements.py <before.json> <after.json>")
        sys.exit(1)
    
    summary = analyze_debt_changes(sys.argv[1], sys.argv[2])