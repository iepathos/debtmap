#!/usr/bin/env python3
"""Compare before/after debtmap results and generate summary."""
import json
import sys
from collections import defaultdict
from typing import Dict, List

def load_json(path: str) -> dict:
    """Load and parse JSON file."""
    with open(path, 'r') as f:
        return json.load(f)

def extract_items(data: dict) -> List[dict]:
    """Extract items from debtmap structure."""
    items = []
    for item in data.get('items', []):
        if 'File' in item:
            file_data = item['File']
            metrics = file_data.get('metrics', {})
            items.append({
                'type': 'file',
                'path': metrics.get('path', ''),
                'metrics': metrics
            })
        elif 'Function' in item:
            func_data = item['Function']
            metrics = func_data.get('metrics', {})
            items.append({
                'type': 'function',
                'path': metrics.get('file_path', ''),
                'name': metrics.get('function_name', ''),
                'metrics': metrics
            })
    return items

def get_item_key(item: dict) -> tuple:
    """Create unique key for debt item."""
    if item['type'] == 'file':
        return ('file', item['path'], '')
    else:
        return ('function', item['path'], item.get('name', ''))

def get_score(item: dict) -> float:
    """Extract complexity score from item."""
    metrics = item.get('metrics', {})
    # Try different score fields
    if 'total_complexity' in metrics:
        return float(metrics['total_complexity'])
    elif 'complexity' in metrics:
        return float(metrics['complexity'])
    elif 'cognitive_complexity' in metrics:
        return float(metrics['cognitive_complexity'])
    return 0.0

def is_problematic(item: dict) -> bool:
    """Check if item has significant issues."""
    metrics = item.get('metrics', {})
    
    # Check for god object
    if metrics.get('god_object_indicators', {}).get('is_god_object'):
        return True
    
    # Check for high complexity
    max_complexity = metrics.get('max_complexity', 0)
    avg_complexity = metrics.get('avg_complexity', 0)
    cognitive = metrics.get('cognitive_complexity', 0)
    
    return max_complexity > 10 or avg_complexity > 5 or cognitive > 15

def analyze_changes(before_data: dict, after_data: dict) -> dict:
    """Analyze differences between before and after data."""
    before_items = extract_items(before_data)
    after_items = extract_items(after_data)

    # Create lookup maps
    before_map = {get_item_key(item): item for item in before_items}
    after_map = {get_item_key(item): item for item in after_items}

    # Calculate metrics
    before_keys = set(before_map.keys())
    after_keys = set(after_map.keys())

    resolved = before_keys - after_keys
    new_items = after_keys - before_keys
    common_items = before_keys & after_keys

    improved = []
    regressed = []
    unchanged = []

    for key in common_items:
        before_score = get_score(before_map[key])
        after_score = get_score(after_map[key])
        
        before_problematic = is_problematic(before_map[key])
        after_problematic = is_problematic(after_map[key])

        if after_score < before_score - 0.01 or (before_problematic and not after_problematic):
            improved.append({
                'key': key,
                'before': before_score,
                'after': after_score,
                'change': before_score - after_score,
                'change_pct': ((before_score - after_score) / before_score * 100) if before_score > 0 else 0,
                'item': before_map[key],
                'was_problematic': before_problematic,
                'is_problematic': after_problematic
            })
        elif after_score > before_score + 0.01 or (not before_problematic and after_problematic):
            regressed.append({
                'key': key,
                'before': before_score,
                'after': after_score,
                'change': after_score - before_score,
                'change_pct': ((after_score - before_score) / before_score * 100) if before_score > 0 else 0,
                'item': after_map[key]
            })
        else:
            unchanged.append(key)

    # Calculate total scores
    total_before = sum(get_score(item) for item in before_items)
    total_after = sum(get_score(item) for item in after_items)
    
    # Count problematic items
    problematic_before = sum(1 for item in before_items if is_problematic(item))
    problematic_after = sum(1 for item in after_items if is_problematic(item))

    return {
        'before_count': len(before_items),
        'after_count': len(after_items),
        'total_before': total_before,
        'total_after': total_after,
        'problematic_before': problematic_before,
        'problematic_after': problematic_after,
        'resolved': [(key, get_score(before_map[key]), before_map[key]) for key in resolved if is_problematic(before_map[key])],
        'new_items': [(key, get_score(after_map[key]), after_map[key]) for key in new_items if is_problematic(after_map[key])],
        'improved': improved,
        'regressed': regressed,
        'unchanged_count': len(unchanged)
    }

def format_location(key: tuple) -> str:
    """Format item location for display."""
    item_type, path, name = key
    if name:
        return f"{path}::{name}"
    return path

def describe_issue(item: dict) -> str:
    """Describe the main issues with an item."""
    metrics = item.get('metrics', {})
    issues = []
    
    god_obj = metrics.get('god_object_indicators', {})
    if god_obj.get('is_god_object'):
        score = god_obj.get('god_object_score', 0)
        issues.append(f"god object (score {score:.1f})")
    
    max_complexity = metrics.get('max_complexity', 0)
    if max_complexity > 10:
        issues.append(f"max complexity {max_complexity}")
    
    cognitive = metrics.get('cognitive_complexity', 0)
    if cognitive > 15:
        issues.append(f"cognitive complexity {cognitive}")
    
    return ", ".join(issues) if issues else "complexity issues"

def generate_summary(analysis: dict, successful: int, failed: int, total: int) -> str:
    """Generate markdown summary for commit message."""
    lines = []

    # Overall metrics
    total_before = analysis['total_before']
    total_after = analysis['total_after']
    improvement = total_before - total_after
    improvement_pct = (improvement / total_before * 100) if total_before > 0 else 0
    
    prob_before = analysis['problematic_before']
    prob_after = analysis['problematic_after']
    prob_improvement = prob_before - prob_after

    lines.append("Technical Debt Improvements:")
    lines.append(f"- Total complexity score: {total_before:.1f} → {total_after:.1f} ({improvement:+.1f} points, {improvement_pct:+.1f}%)")
    lines.append(f"- Problematic items: {prob_before} → {prob_after} ({prob_improvement:+d})")
    lines.append(f"- Items resolved: {len(analysis['resolved'])} high-priority items eliminated")
    lines.append(f"- Items improved: {len(analysis['improved'])} with reduced complexity/issues")
    lines.append(f"- Overall items: {analysis['before_count']} → {analysis['after_count']} ({analysis['after_count'] - analysis['before_count']:+d})")
    lines.append("")

    # Top resolved items
    if analysis['resolved']:
        lines.append("Top items completely resolved:")
        resolved_sorted = sorted(analysis['resolved'], key=lambda x: x[1], reverse=True)[:5]
        for i, (key, score, item) in enumerate(resolved_sorted, 1):
            location = format_location(key)
            issue_desc = describe_issue(item)
            lines.append(f"{i}. {location}: {issue_desc} (removed)")
        lines.append("")

    # Top improvements
    if analysis['improved']:
        lines.append("Top improvements (complexity reductions):")
        improved_sorted = sorted(analysis['improved'], key=lambda x: x['change'], reverse=True)[:5]
        for i, item in enumerate(improved_sorted, 1):
            location = format_location(item['key'])
            if item['was_problematic'] and not item['is_problematic']:
                lines.append(f"{i}. {location}: {describe_issue(item['item'])} → resolved")
            else:
                lines.append(f"{i}. {location}: complexity {item['before']:.1f} → {item['after']:.1f} ({item['change_pct']:.1f}%)")
        lines.append("")

    # Regressions
    if analysis['regressed']:
        lines.append("⚠️ Regressions detected:")
        regressed_sorted = sorted(analysis['regressed'], key=lambda x: x['change'], reverse=True)[:3]
        for item in regressed_sorted:
            location = format_location(item['key'])
            lines.append(f"- {location}: complexity {item['before']:.1f} → {item['after']:.1f} (+{item['change_pct']:.1f}%)")
        lines.append("")

    # New problematic items
    if analysis['new_items']:
        lines.append(f"⚠️ New problematic items introduced: {len(analysis['new_items'])}")
        new_sorted = sorted(analysis['new_items'], key=lambda x: x[1], reverse=True)[:3]
        for key, score, item in new_sorted:
            location = format_location(key)
            issue_desc = describe_issue(item)
            lines.append(f"- NEW: {location}: {issue_desc}")
        lines.append("")

    return "\n".join(lines)

def main():
    if len(sys.argv) < 7:
        print("Usage: compare_debt.py --before <before.json> --after <after.json> --successful N --failed M --total T")
        sys.exit(1)

    # Parse arguments
    args = {}
    i = 1
    while i < len(sys.argv):
        if sys.argv[i].startswith('--'):
            key = sys.argv[i][2:]
            if i + 1 < len(sys.argv):
                args[key] = sys.argv[i + 1]
                i += 2
            else:
                i += 1
        else:
            i += 1

    before_path = args.get('before')
    after_path = args.get('after')
    successful = int(args.get('successful', 0))
    failed = int(args.get('failed', 0))
    total = int(args.get('total', 0))

    # Load data
    before_data = load_json(before_path)
    after_data = load_json(after_path)

    # Analyze
    analysis = analyze_changes(before_data, after_data)

    # Generate summary
    summary = generate_summary(analysis, successful, failed, total)

    # Output
    print(summary)

    # Write to file for commit message
    with open('.prodigy/debt-comparison-summary.txt', 'w') as f:
        f.write(summary)

    print(f"\nSummary written to .prodigy/debt-comparison-summary.txt")

if __name__ == '__main__':
    main()
