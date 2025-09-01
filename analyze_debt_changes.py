#!/usr/bin/env python3
"""Analyze technical debt improvements between before and after debtmap files."""

import json
import sys
from collections import defaultdict
from typing import Dict, List, Tuple, Any

def load_debtmap(filepath: str) -> Dict[str, Any]:
    """Load and parse a debtmap JSON file."""
    with open(filepath, 'r') as f:
        return json.load(f)

def create_item_key(item: Dict[str, Any]) -> str:
    """Create a unique key for an item based on location."""
    location = item.get('location', {})
    file_path = location.get('file', '')
    function = location.get('function', '')
    line = location.get('line', 0)
    return f"{file_path}::{function}::{line}"

def calculate_total_score(data: Dict[str, Any]) -> float:
    """Calculate total debt score from all items."""
    total = 0.0
    for item in data.get('items', []):
        if 'unified_score' in item and 'final_score' in item['unified_score']:
            total += item['unified_score']['final_score']
    return total

def analyze_changes(before_data: Dict[str, Any], after_data: Dict[str, Any]) -> Dict[str, Any]:
    """Analyze changes between before and after debtmap data."""
    
    # Create item maps for comparison
    before_items = {}
    after_items = {}
    
    for item in before_data.get('items', []):
        key = create_item_key(item)
        before_items[key] = item
    
    for item in after_data.get('items', []):
        key = create_item_key(item)
        after_items[key] = item
    
    # Calculate improvements
    resolved = []
    improved = []
    regressed = []
    unchanged = []
    
    before_keys = set(before_items.keys())
    after_keys = set(after_items.keys())
    
    # Items that were resolved (no longer present)
    for key in before_keys - after_keys:
        item = before_items[key]
        score = item.get('unified_score', {}).get('final_score', 0)
        resolved.append((key, score, 0))
    
    # Items that changed
    for key in before_keys & after_keys:
        before_item = before_items[key]
        after_item = after_items[key]
        before_score = before_item.get('unified_score', {}).get('final_score', 0)
        after_score = after_item.get('unified_score', {}).get('final_score', 0)
        
        if after_score < before_score:
            improved.append((key, before_score, after_score))
        elif after_score > before_score:
            regressed.append((key, before_score, after_score))
        else:
            unchanged.append((key, before_score, after_score))
    
    # New items introduced
    new_items = []
    for key in after_keys - before_keys:
        item = after_items[key]
        score = item.get('unified_score', {}).get('final_score', 0)
        new_items.append((key, 0, score))
    
    # Category analysis
    categories = defaultdict(lambda: {'before': 0, 'after': 0})
    
    for item in before_data.get('items', []):
        for debt_type, metrics in item.get('debt_types', {}).items():
            if isinstance(metrics, dict):
                categories[debt_type]['before'] += metrics.get('score', 0)
    
    for item in after_data.get('items', []):
        for debt_type, metrics in item.get('debt_types', {}).items():
            if isinstance(metrics, dict):
                categories[debt_type]['after'] += metrics.get('score', 0)
    
    return {
        'resolved': resolved,
        'improved': improved,
        'regressed': regressed,
        'unchanged': unchanged,
        'new_items': new_items,
        'categories': dict(categories),
        'total_before': calculate_total_score(before_data),
        'total_after': calculate_total_score(after_data),
        'items_before': len(before_data.get('items', [])),
        'items_after': len(after_data.get('items', []))
    }

def format_location(key: str) -> str:
    """Format location key for display."""
    parts = key.split('::')
    if len(parts) >= 2:
        file_path = parts[0].replace('/Users/glen/.mmm/worktrees/debtmap/session-c5a4cdd0-e311-4550-9758-07dd17fa9bde/', '')
        function = parts[1]
        if function:
            return f"{file_path}::{function}"
        return file_path
    return key

def generate_summary(analysis: Dict[str, Any], map_results: Dict[str, Any]) -> str:
    """Generate a summary report for the commit message."""
    
    total_before = analysis['total_before']
    total_after = analysis['total_after']
    improvement = total_before - total_after
    improvement_pct = (improvement / total_before * 100) if total_before > 0 else 0
    
    items_before = analysis['items_before']
    items_after = analysis['items_after']
    items_removed = items_before - items_after
    items_removed_pct = (items_removed / items_before * 100) if items_before > 0 else 0
    
    successful = map_results.get('successful', 0)
    failed = map_results.get('failed', 0)
    total_targeted = map_results.get('total', successful + failed)
    
    summary = []
    
    # Main metrics
    summary.append(f"Processed {total_targeted} debt items in parallel:")
    summary.append(f"- Successfully fixed: {successful} items")
    if failed > 0:
        summary.append(f"- Failed to fix: {failed} items")
    summary.append("")
    
    summary.append("Technical Debt Improvements:")
    summary.append(f"- Total debt score: {total_before:.0f} → {total_after:.0f} ({improvement:.0f} points, -{improvement_pct:.1f}%)")
    summary.append(f"- Items resolved: {len(analysis['resolved'])} of {total_targeted} targeted")
    summary.append(f"- Overall items: {items_before} → {items_after} ({items_removed} removed, -{items_removed_pct:.1f}%)")
    summary.append("")
    
    # Category improvements
    categories = analysis['categories']
    if categories:
        summary.append("By category:")
        for debt_type, scores in sorted(categories.items(), key=lambda x: x[1]['before'] - x[1]['after'], reverse=True):
            before = scores['before']
            after = scores['after']
            if before > 0:
                change = before - after
                change_pct = (change / before * 100)
                if change > 0:
                    summary.append(f"- {debt_type}: {before:.0f} → {after:.0f} (-{change_pct:.0f}%)")
        summary.append("")
    
    # Top improvements
    if analysis['resolved'] or analysis['improved']:
        summary.append("Top improvements:")
        all_improvements = [(k, b, a) for k, b, a in analysis['resolved']] + analysis['improved']
        all_improvements.sort(key=lambda x: x[1] - x[2], reverse=True)
        
        for i, (key, before_score, after_score) in enumerate(all_improvements[:5], 1):
            location = format_location(key)
            if after_score == 0:
                summary.append(f"{i}. {location}: score {before_score:.0f} → 0 (resolved)")
            else:
                change_pct = ((before_score - after_score) / before_score * 100)
                summary.append(f"{i}. {location}: score {before_score:.0f} → {after_score:.0f} (-{change_pct:.0f}%)")
    
    # Regressions
    if analysis['regressed'] or (analysis['new_items'] and any(score > 30 for _, _, score in analysis['new_items'])):
        summary.append("")
        summary.append("⚠️ Regressions detected:")
        
        for key, before_score, after_score in analysis['regressed'][:3]:
            location = format_location(key)
            change_pct = ((after_score - before_score) / before_score * 100) if before_score > 0 else 100
            summary.append(f"- {location}: score {before_score:.0f} → {after_score:.0f} (+{change_pct:.0f}%)")
        
        high_score_new = [(k, s) for k, _, s in analysis['new_items'] if s > 30]
        for key, score in high_score_new[:2]:
            location = format_location(key)
            summary.append(f"- NEW: {location}: score {score:.0f}")
    
    return '\n'.join(summary)

def main():
    """Main execution function."""
    
    # Parse command line arguments
    import argparse
    parser = argparse.ArgumentParser(description='Compare debt results')
    parser.add_argument('--before', required=True, help='Path to before debtmap.json')
    parser.add_argument('--after', required=True, help='Path to after debtmap.json')
    parser.add_argument('--map-results', default='{}', help='JSON results from map phase')
    parser.add_argument('--successful', type=int, default=0, help='Number of successful fixes')
    parser.add_argument('--failed', type=int, default=0, help='Number of failed fixes')
    parser.add_argument('--total', type=int, default=0, help='Total items processed')
    
    args = parser.parse_args()
    
    # Load data
    print("Loading debtmap files...")
    before_data = load_debtmap(args.before)
    after_data = load_debtmap(args.after)
    
    # Parse map results
    try:
        map_results = json.loads(args.map_results) if args.map_results != '${map.results}' else {}
    except:
        map_results = {}
    
    # Use provided counts
    map_results['successful'] = args.successful
    map_results['failed'] = args.failed
    map_results['total'] = args.total if args.total > 0 else (args.successful + args.failed)
    
    # Analyze changes
    print("Analyzing changes...")
    analysis = analyze_changes(before_data, after_data)
    
    # Generate summary
    print("\n" + "="*60)
    summary = generate_summary(analysis, map_results)
    print(summary)
    print("="*60 + "\n")
    
    # Write summary to file for commit message
    with open('debt_improvement_summary.txt', 'w') as f:
        f.write(summary)
    
    print(f"Summary written to debt_improvement_summary.txt")
    print(f"\nTotal improvements: {len(analysis['resolved'])} resolved, {len(analysis['improved'])} improved")
    if analysis['regressed']:
        print(f"Warning: {len(analysis['regressed'])} items regressed")

if __name__ == '__main__':
    main()