#!/usr/bin/env python3
"""Compare technical debt before and after fixes."""

import json
import sys
from pathlib import Path
from collections import defaultdict
from typing import Dict, List, Tuple

def load_json(path: str) -> dict:
    """Load and parse JSON file."""
    with open(path, 'r') as f:
        return json.load(f)

def get_item_key(item: dict) -> str:
    """Create unique key for a debt item."""
    # Handle File type items
    if 'File' in item:
        metrics = item['File'].get('metrics', {})
        return metrics.get('path', 'unknown')
    # Handle Function type items
    elif 'Function' in item:
        metrics = item['Function'].get('metrics', {})
        return f"{metrics.get('file_path', 'unknown')}::{metrics.get('function_name', 'unknown')}"
    # Handle other types
    return str(item.get('file_path', 'unknown'))

def get_score(item: dict) -> float:
    """Extract unified score from item."""
    # Handle File type items
    if 'File' in item:
        return item['File'].get('score', 0.0)
    # Handle Function type items
    elif 'Function' in item:
        return item['Function'].get('score', 0.0)
    # Fallback
    return item.get('score', 0.0)

def get_category(item: dict) -> str:
    """Get primary debt category."""
    # Handle File type items
    if 'File' in item:
        metrics = item['File'].get('metrics', {})
        god_indicators = metrics.get('god_object_indicators', {})
        if god_indicators.get('is_god_object'):
            return 'God Object / Complexity'
        # Check if it's a coverage issue
        if metrics.get('coverage_percent', 0) < 50:
            return 'Coverage'
        return 'File Structure'
    # Handle Function type items
    elif 'Function' in item:
        metrics = item['Function'].get('metrics', {})
        complexity = metrics.get('cyclomatic_complexity', 0)
        if complexity > 10:
            return 'Function Complexity'
        return 'Function Quality'
    # Fallback
    return 'Other'

def analyze_debt_changes(before_data: dict, after_data: dict) -> dict:
    """Analyze changes in technical debt."""
    before_items = {get_item_key(item): item for item in before_data.get('items', [])}
    after_items = {get_item_key(item): item for item in after_data.get('items', [])}

    # Calculate overall metrics
    total_before = sum(get_score(item) for item in before_items.values())
    total_after = sum(get_score(item) for item in after_items.values())
    improvement_pct = ((total_before - total_after) / total_before * 100) if total_before > 0 else 0

    # Identify changes
    resolved = set(before_items.keys()) - set(after_items.keys())
    new_items = set(after_items.keys()) - set(before_items.keys())
    common = set(before_items.keys()) & set(after_items.keys())

    improved = []
    regressed = []
    unchanged = []

    for key in common:
        before_score = get_score(before_items[key])
        after_score = get_score(after_items[key])

        if after_score < before_score - 0.1:  # Threshold for improvement
            improved.append((key, before_score, after_score))
        elif after_score > before_score + 0.1:  # Threshold for regression
            regressed.append((key, before_score, after_score))
        else:
            unchanged.append(key)

    # Category analysis
    category_before = defaultdict(float)
    category_after = defaultdict(float)

    for item in before_items.values():
        category = get_category(item)
        category_before[category] += get_score(item)

    for item in after_items.values():
        category = get_category(item)
        category_after[category] += get_score(item)

    category_improvements = {}
    for cat in set(category_before.keys()) | set(category_after.keys()):
        before = category_before.get(cat, 0)
        after = category_after.get(cat, 0)
        if before > 0:
            pct_change = ((before - after) / before * 100)
            category_improvements[cat] = {
                'before': before,
                'after': after,
                'change_pct': pct_change
            }

    return {
        'total_before': total_before,
        'total_after': total_after,
        'improvement_pct': improvement_pct,
        'items_before': len(before_items),
        'items_after': len(after_items),
        'resolved': resolved,
        'improved': sorted(improved, key=lambda x: x[1] - x[2], reverse=True),
        'regressed': sorted(regressed, key=lambda x: x[2] - x[1], reverse=True),
        'new_items': new_items,
        'category_improvements': category_improvements,
        'before_items': before_items,
        'after_items': after_items
    }

def format_location(key: str) -> str:
    """Format a location key as readable string."""
    # Key is already formatted as "path" or "path::function"
    if '::' in key:
        parts = key.split('::')
        file_name = Path(parts[0]).name if parts[0] else 'unknown'
        return f"{file_name}::{parts[1]}"
    else:
        file_name = Path(key).name if key else 'unknown'
        return file_name

def generate_commit_message(analysis: dict, successful: int, failed: int, total: int) -> str:
    """Generate commit message from analysis."""
    lines = []

    # Title
    lines.append(f"fix: eliminate {successful} technical debt items via MapReduce")
    lines.append("")

    # Summary
    lines.append(f"Processed {total} debt items in parallel:")
    lines.append(f"- Successfully fixed: {successful} items")
    lines.append(f"- Failed to fix: {failed} items")
    lines.append("")

    # Debt improvements
    lines.append("Technical Debt Improvements:")
    lines.append(f"- Total debt score: {analysis['total_before']:.1f} → {analysis['total_after']:.1f} (-{analysis['improvement_pct']:.1f}%)")
    lines.append(f"- Items resolved: {len(analysis['resolved'])} completely eliminated")
    lines.append(f"- Items improved: {len(analysis['improved'])} with reduced scores")
    lines.append(f"- Overall items: {analysis['items_before']} → {analysis['items_after']} ({analysis['items_after'] - analysis['items_before']:+d})")
    lines.append("")

    # Category improvements (top 5)
    if analysis['category_improvements']:
        lines.append("By category:")
        sorted_cats = sorted(
            analysis['category_improvements'].items(),
            key=lambda x: x[1]['change_pct'],
            reverse=True
        )
        for cat, data in sorted_cats[:5]:
            if data['change_pct'] > 0:
                lines.append(f"- {cat}: -{data['change_pct']:.0f}% ({data['before']:.1f} → {data['after']:.1f})")
        lines.append("")

    # Top improvements (top 5)
    if analysis['resolved'] or analysis['improved']:
        lines.append("Top improvements:")
        count = 1

        # Show resolved items first
        for key in list(analysis['resolved'])[:3]:
            score = get_score(analysis['before_items'][key])
            lines.append(f"{count}. {format_location(key)}: score {score:.1f} → 0 (resolved)")
            count += 1

        # Then show improved items
        for key, before, after in analysis['improved'][:5-count+1]:
            pct = ((before - after) / before * 100) if before > 0 else 0
            lines.append(f"{count}. {format_location(key)}: score {before:.1f} → {after:.1f} (-{pct:.0f}%)")
            count += 1
            if count > 5:
                break
        lines.append("")

    # Regressions
    if analysis['regressed']:
        lines.append("⚠️ Regressions detected:")
        for key, before, after in analysis['regressed'][:3]:
            pct = ((after - before) / before * 100) if before > 0 else 0
            lines.append(f"- {format_location(key)}: score {before:.1f} → {after:.1f} (+{pct:.0f}%)")
        lines.append("")

    # New items
    if analysis['new_items']:
        lines.append(f"ℹ️ New debt items introduced: {len(analysis['new_items'])}")
        for key in list(analysis['new_items'])[:3]:
            score = get_score(analysis['after_items'][key])
            lines.append(f"- {format_location(key)}: score {score:.1f}")
        lines.append("")

    lines.append("This commit represents the aggregated work of multiple parallel agents.")

    return '\n'.join(lines)

def main():
    """Main entry point."""
    before_path = '.prodigy/debtmap-before.json'
    after_path = '.prodigy/debtmap-after.json'
    successful = 10
    failed = 0
    total = 10

    # Parse command line arguments
    args = sys.argv[1:]
    for i, arg in enumerate(args):
        if arg == '--before' and i + 1 < len(args):
            before_path = args[i + 1]
        elif arg == '--after' and i + 1 < len(args):
            after_path = args[i + 1]
        elif arg == '--successful' and i + 1 < len(args):
            successful = int(args[i + 1])
        elif arg == '--failed' and i + 1 < len(args):
            failed = int(args[i + 1])
        elif arg == '--total' and i + 1 < len(args):
            total = int(args[i + 1])

    # Load data
    print("Loading debtmap files...", file=sys.stderr)
    before_data = load_json(before_path)
    after_data = load_json(after_path)

    # Analyze
    print("Analyzing changes...", file=sys.stderr)
    analysis = analyze_debt_changes(before_data, after_data)

    # Generate commit message
    commit_message = generate_commit_message(analysis, successful, failed, total)

    # Output
    print(commit_message)

if __name__ == '__main__':
    main()
