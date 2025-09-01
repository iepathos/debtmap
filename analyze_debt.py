#!/usr/bin/env python3
import json
import sys
from collections import defaultdict
from typing import Dict, List, Tuple, Set

def load_debtmap(filepath: str) -> dict:
    """Load and parse a debtmap JSON file."""
    with open(filepath, 'r') as f:
        return json.load(f)

def create_item_key(item: dict) -> str:
    """Create a unique key for an item based on location."""
    location = item.get('location', {})
    return f"{location.get('file', '')}::{item.get('name', '')}::{location.get('line', 0)}"

def analyze_debt_changes(before_path: str, after_path: str) -> dict:
    """Analyze the differences between two debtmap files."""
    # Load data
    before_data = load_debtmap(before_path)
    after_data = load_debtmap(after_path)
    
    # Get items
    before_items = before_data.get('items', [])
    after_items = after_data.get('items', [])
    
    # Create lookup maps
    before_map = {}
    for item in before_items:
        key = create_item_key(item)
        before_map[key] = item
    
    after_map = {}
    for item in after_items:
        key = create_item_key(item)
        after_map[key] = item
    
    # Analyze changes
    before_keys = set(before_map.keys())
    after_keys = set(after_map.keys())
    
    resolved = before_keys - after_keys
    new_items = after_keys - before_keys
    common = before_keys & after_keys
    
    improved = []
    regressed = []
    unchanged = []
    
    for key in common:
        before_score = before_map[key].get('unified_score', {}).get('final_score', 0)
        after_score = after_map[key].get('unified_score', {}).get('final_score', 0)
        
        if after_score < before_score:
            improved.append((key, before_score, after_score, before_score - after_score))
        elif after_score > before_score:
            regressed.append((key, before_score, after_score, after_score - before_score))
        else:
            unchanged.append(key)
    
    # Calculate totals
    total_before = sum(item.get('unified_score', {}).get('final_score', 0) for item in before_items)
    total_after = sum(item.get('unified_score', {}).get('final_score', 0) for item in after_items)
    
    # Category analysis
    categories_before = defaultdict(float)
    categories_after = defaultdict(float)
    
    for item in before_items:
        for category in item.get('categories', []):
            categories_before[category] += item.get('unified_score', {}).get('final_score', 0)
    
    for item in after_items:
        for category in item.get('categories', []):
            categories_after[category] += item.get('unified_score', {}).get('final_score', 0)
    
    return {
        'total_before': total_before,
        'total_after': total_after,
        'items_before': len(before_items),
        'items_after': len(after_items),
        'resolved': resolved,
        'new_items': new_items,
        'improved': improved,
        'regressed': regressed,
        'unchanged': unchanged,
        'categories_before': dict(categories_before),
        'categories_after': dict(categories_after),
        'before_map': before_map,
        'after_map': after_map
    }

def format_summary(analysis: dict, successful: int, failed: int, total: int) -> str:
    """Format the analysis into a commit message summary."""
    total_before = analysis['total_before']
    total_after = analysis['total_after']
    items_before = analysis['items_before']
    items_after = analysis['items_after']
    
    # Calculate percentages
    debt_reduction = total_before - total_after
    debt_reduction_pct = (debt_reduction / total_before * 100) if total_before > 0 else 0
    items_reduction = items_before - items_after
    items_reduction_pct = (items_reduction / items_before * 100) if items_before > 0 else 0
    
    # Build summary
    lines = []
    lines.append(f"fix: eliminate {successful} technical debt items via MapReduce")
    lines.append("")
    lines.append(f"Processed {total} debt items in parallel:")
    lines.append(f"- Successfully fixed: {successful} items")
    lines.append(f"- Failed to fix: {failed} items")
    lines.append("")
    lines.append("Technical Debt Improvements:")
    lines.append(f"- Total debt score: {total_before:.0f} → {total_after:.0f} ({debt_reduction_pct:+.0f}%)")
    lines.append(f"- Items resolved: {len(analysis['resolved'])} of {total} targeted")
    lines.append(f"- Overall items: {items_before} → {items_after} ({items_reduction_pct:+.0f}%)")
    
    # Category improvements
    if analysis['categories_before'] and analysis['categories_after']:
        lines.append("")
        lines.append("By category:")
        for category in sorted(set(analysis['categories_before'].keys()) | set(analysis['categories_after'].keys())):
            before = analysis['categories_before'].get(category, 0)
            after = analysis['categories_after'].get(category, 0)
            if before > 0:
                change_pct = ((after - before) / before * 100)
                if abs(change_pct) > 0.1:  # Only show meaningful changes
                    lines.append(f"- {category}: {change_pct:+.0f}%")
    
    # Top improvements
    if analysis['improved']:
        lines.append("")
        lines.append("Top improvements:")
        # Sort by improvement amount
        top_improved = sorted(analysis['improved'], key=lambda x: x[3], reverse=True)[:5]
        for i, (key, before_score, after_score, improvement) in enumerate(top_improved, 1):
            # Extract file and function name from key
            parts = key.split('::')
            if len(parts) >= 2:
                file_part = parts[0].split('/')[-1] if '/' in parts[0] else parts[0]
                func_part = parts[1]
                reduction_pct = (improvement / before_score * 100) if before_score > 0 else 0
                lines.append(f"{i}. {file_part}::{func_part}: score {before_score:.0f} → {after_score:.0f} (-{reduction_pct:.0f}%)")
    
    # Top resolved items
    if analysis['resolved']:
        lines.append("")
        lines.append("Items completely resolved:")
        resolved_items = []
        for key in list(analysis['resolved'])[:5]:
            if key in analysis['before_map']:
                item = analysis['before_map'][key]
                score = item.get('unified_score', {}).get('final_score', 0)
                parts = key.split('::')
                if len(parts) >= 2:
                    file_part = parts[0].split('/')[-1] if '/' in parts[0] else parts[0]
                    func_part = parts[1]
                    resolved_items.append((f"{file_part}::{func_part}", score))
        
        resolved_items.sort(key=lambda x: x[1], reverse=True)
        for i, (name, score) in enumerate(resolved_items[:5], 1):
            lines.append(f"{i}. {name}: score {score:.0f} → 0 (resolved)")
    
    # Regressions
    if analysis['regressed']:
        lines.append("")
        lines.append("⚠️ Regressions detected:")
        for key, before_score, after_score, increase in sorted(analysis['regressed'], key=lambda x: x[3], reverse=True)[:3]:
            parts = key.split('::')
            if len(parts) >= 2:
                file_part = parts[0].split('/')[-1] if '/' in parts[0] else parts[0]
                func_part = parts[1]
                increase_pct = (increase / before_score * 100) if before_score > 0 else 0
                lines.append(f"- {file_part}::{func_part}: score {before_score:.0f} → {after_score:.0f} (+{increase_pct:.0f}%)")
    
    # New high-score items
    if analysis['new_items']:
        high_score_new = []
        for key in analysis['new_items']:
            if key in analysis['after_map']:
                item = analysis['after_map'][key]
                score = item.get('unified_score', {}).get('final_score', 0)
                if score > 30:  # Only show significant new items
                    parts = key.split('::')
                    if len(parts) >= 2:
                        file_part = parts[0].split('/')[-1] if '/' in parts[0] else parts[0]
                        func_part = parts[1]
                        high_score_new.append((f"{file_part}::{func_part}", score))
        
        if high_score_new:
            lines.append("")
            lines.append("New debt items introduced:")
            for name, score in sorted(high_score_new, key=lambda x: x[1], reverse=True)[:3]:
                lines.append(f"- NEW: {name}: score {score:.0f}")
    
    lines.append("")
    lines.append("This commit represents the aggregated work of multiple parallel agents.")
    
    return '\n'.join(lines)

if __name__ == '__main__':
    # Parse command line arguments
    before_path = 'debtmap.json'
    after_path = 'debtmap-after.json'
    successful = 0
    failed = 0
    total = 0
    
    # Simple argument parsing
    import sys
    args = sys.argv[1:]
    for i in range(0, len(args), 2):
        if args[i] == '--before' and i+1 < len(args):
            before_path = args[i+1]
        elif args[i] == '--after' and i+1 < len(args):
            after_path = args[i+1]
        elif args[i] == '--successful' and i+1 < len(args):
            successful = int(args[i+1])
        elif args[i] == '--failed' and i+1 < len(args):
            failed = int(args[i+1])
        elif args[i] == '--total' and i+1 < len(args):
            total = int(args[i+1])
    
    # Analyze
    analysis = analyze_debt_changes(before_path, after_path)
    
    # Format and print summary
    summary = format_summary(analysis, successful, failed, total)
    print(summary)
    
    # Save to file for commit message
    with open('commit_message.txt', 'w') as f:
        f.write(summary)