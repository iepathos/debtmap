#!/usr/bin/env python3
import json
import sys
from collections import defaultdict

def load_debtmap(filepath: str) -> dict:
    """Load and parse a debtmap JSON file."""
    with open(filepath, 'r') as f:
        return json.load(f)

def generate_commit_message(before_path: str, after_path: str, successful: int, failed: int, total: int):
    """Generate a commit message based on debt improvements."""
    
    # Load data
    before_data = load_debtmap(before_path)
    after_data = load_debtmap(after_path)
    
    # Extract items
    before_items = before_data.get('items', [])
    after_items = after_data.get('items', [])
    
    # Create lookups by file and function
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
    
    for key, before_item in before_map.items():
        before_score = before_item.get('unified_score', {}).get('final_score', 0)
        
        if key not in after_map:
            resolved_items.append((key, before_item, before_score))
        else:
            after_item = after_map[key]
            after_score = after_item.get('unified_score', {}).get('final_score', 0)
            
            if after_score < before_score:
                improved_items.append((key, before_item, after_item, before_score, after_score))
    
    # Calculate total scores
    total_before = sum(item.get('unified_score', {}).get('final_score', 0) for item in before_items)
    total_after = sum(item.get('unified_score', {}).get('final_score', 0) for item in after_items)
    
    # Category analysis
    category_improvements = defaultdict(lambda: {'before': 0, 'after': 0, 'resolved_count': 0})
    
    # Track resolved items by category
    for key, item, score in resolved_items:
        for category in item.get('categories', []):
            category_improvements[category]['resolved_count'] += 1
            category_improvements[category]['before'] += score
    
    # Track all items for totals
    for item in before_items:
        for category in item.get('categories', []):
            if category not in category_improvements:
                category_improvements[category]['before'] = 0
            category_improvements[category]['before'] += item.get('unified_score', {}).get('final_score', 0)
    
    for item in after_items:
        for category in item.get('categories', []):
            if category not in category_improvements:
                category_improvements[category]['after'] = 0
            category_improvements[category]['after'] += item.get('unified_score', {}).get('final_score', 0)
    
    # Build commit message
    commit_lines = []
    commit_lines.append(f"fix: eliminate {len(resolved_items)} technical debt items via MapReduce")
    commit_lines.append("")
    commit_lines.append(f"Processed {total} debt items in parallel:")
    commit_lines.append(f"- Successfully fixed: {successful} items")
    commit_lines.append(f"- Failed to fix: {failed} items")
    commit_lines.append("")
    commit_lines.append("Technical Debt Improvements:")
    
    if total_before > 0:
        improvement_pct = ((total_before - total_after) / total_before) * 100
        commit_lines.append(f"- Total debt score: {total_before:.0f} → {total_after:.0f} ({improvement_pct:+.1f}%)")
    else:
        commit_lines.append(f"- Total debt score: {total_before:.0f} → {total_after:.0f}")
    
    commit_lines.append(f"- Items resolved: {len(resolved_items)} of {successful + failed} targeted")
    commit_lines.append(f"- Overall items: {len(before_items)} → {len(after_items)} ({len(after_items) - len(before_items):+d})")
    
    # Add category breakdown if we have improvements
    if category_improvements:
        commit_lines.append("")
        commit_lines.append("By category:")
        for category in sorted(category_improvements.keys()):
            data = category_improvements[category]
            before = data['before']
            after = data['after']
            resolved = data['resolved_count']
            if before > 0:
                change_pct = ((before - after) / before) * 100
                if resolved > 0:
                    commit_lines.append(f"- {category}: {change_pct:+.0f}% (eliminated {resolved} items)")
                elif change_pct != 0:
                    commit_lines.append(f"- {category}: {change_pct:+.0f}%")
    
    # Add top improvements
    improvements = sorted(resolved_items + improved_items, 
                         key=lambda x: x[2] if len(x) == 3 else x[3] - x[4], 
                         reverse=True)
    
    if improvements[:3]:
        commit_lines.append("")
        commit_lines.append("Top improvements:")
        for i, item in enumerate(improvements[:3], 1):
            if len(item) == 3:  # Resolved
                key, _, score = item
                file = key[0].replace('src/', '')
                func = key[1]
                commit_lines.append(f"{i}. {file}::{func}: score {score:.0f} → 0 (resolved)")
            else:  # Improved
                key, _, _, before_score, after_score = item
                file = key[0].replace('src/', '')
                func = key[1]
                reduction_pct = ((before_score - after_score) / before_score) * 100
                commit_lines.append(f"{i}. {file}::{func}: score {before_score:.0f} → {after_score:.0f} (-{reduction_pct:.0f}%)")
    
    commit_lines.append("")
    commit_lines.append("This commit represents the aggregated work of multiple parallel agents.")
    
    return "\n".join(commit_lines)

if __name__ == "__main__":
    # Parse command line arguments
    successful = int(sys.argv[3]) if len(sys.argv) > 3 else 0
    failed = int(sys.argv[4]) if len(sys.argv) > 4 else 0
    total = int(sys.argv[5]) if len(sys.argv) > 5 else successful + failed
    
    message = generate_commit_message(sys.argv[1], sys.argv[2], successful, failed, total)
    print(message)