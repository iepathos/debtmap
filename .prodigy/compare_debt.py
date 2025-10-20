#!/usr/bin/env python3
"""Compare debt analysis results and generate commit message."""

import json
import sys
from collections import defaultdict
from pathlib import Path


def load_json(path):
    """Load and parse a JSON file."""
    with open(path, 'r') as f:
        return json.load(f)


def get_item_key(item):
    """Create a unique key for a debt item."""
    # Handle both Function and Module variants
    if 'Function' in item:
        loc = item['Function']['location']
        return (loc['file'], loc['function'], loc['line'])
    elif 'Module' in item:
        loc = item['Module']['location']
        return (loc['file'], '', loc.get('line', 0))
    return ('', '', 0)


def get_debt_type(item):
    """Extract debt type from item."""
    if 'Function' in item:
        debt_type = item['Function'].get('debt_type', {})
    elif 'Module' in item:
        debt_type = item['Module'].get('debt_type', {})
    else:
        return 'unknown'

    # Debt type is a dict with one key
    if isinstance(debt_type, dict):
        return list(debt_type.keys())[0] if debt_type else 'unknown'
    return 'unknown'


def get_unified_score(item):
    """Extract unified score from item."""
    if 'Function' in item:
        return item['Function'].get('unified_score', {}).get('final_score', 0)
    elif 'Module' in item:
        return item['Module'].get('unified_score', {}).get('final_score', 0)
    return 0


def calculate_total_score(items):
    """Calculate total debt score from items."""
    return sum(get_unified_score(item) for item in items)


def analyze_debt_changes(before_data, after_data, map_results):
    """Analyze changes between before and after debt data."""
    before_items = before_data.get('items', [])
    after_items = after_data.get('items', [])

    # Create lookup maps
    before_map = {get_item_key(item): item for item in before_items}
    after_map = {get_item_key(item): item for item in after_items}

    # Calculate overall metrics
    total_before = calculate_total_score(before_items)
    total_after = calculate_total_score(after_items)
    improvement_pct = ((total_before - total_after) / total_before * 100) if total_before > 0 else 0

    # Analyze item-level changes
    before_keys = set(before_map.keys())
    after_keys = set(after_map.keys())

    resolved = before_keys - after_keys
    new_items = after_keys - before_keys
    common = before_keys & after_keys

    improved = []
    regressed = []
    unchanged = []

    for key in common:
        before_score = get_unified_score(before_map[key])
        after_score = get_unified_score(after_map[key])

        if after_score < before_score:
            improved.append((key, before_score, after_score))
        elif after_score > before_score:
            regressed.append((key, before_score, after_score))
        else:
            unchanged.append(key)

    # Sort by improvement amount
    improved.sort(key=lambda x: x[1] - x[2], reverse=True)
    regressed.sort(key=lambda x: x[2] - x[1], reverse=True)

    # Category analysis
    category_before = defaultdict(float)
    category_after = defaultdict(float)

    for item in before_items:
        debt_type = get_debt_type(item)
        score = get_unified_score(item)
        category_before[debt_type] += score

    for item in after_items:
        debt_type = get_debt_type(item)
        score = get_unified_score(item)
        category_after[debt_type] += score

    return {
        'total_before': total_before,
        'total_after': total_after,
        'improvement_pct': improvement_pct,
        'items_before': len(before_items),
        'items_after': len(after_items),
        'resolved': resolved,
        'improved': improved,
        'regressed': regressed,
        'new_items': new_items,
        'category_before': category_before,
        'category_after': category_after,
        'before_map': before_map,
        'after_map': after_map,
    }


def format_debt_type(debt_type):
    """Format debt type for display."""
    type_names = {
        'Complexity': 'Complexity',
        'Duplication': 'Duplication',
        'TestingGap': 'Test Coverage',
        'Dependency': 'Dependency',
        'Documentation': 'Documentation',
        'ErrorHandling': 'Error Handling',
        'LargeFunction': 'Large Function',
        'DeepNesting': 'Deep Nesting',
    }
    return type_names.get(debt_type, debt_type.replace('_', ' ').title())


def generate_commit_message(analysis, successful, failed, total, map_results):
    """Generate the commit message from analysis results."""
    lines = []

    # Subject line
    lines.append(f"fix: eliminate {successful} technical debt items via MapReduce")
    lines.append("")

    # Summary
    lines.append(f"Processed {total} debt items in parallel:")
    lines.append(f"- Successfully fixed: {successful} items")
    lines.append(f"- Failed to fix: {failed} items")
    lines.append("")

    # Overall metrics
    lines.append("Technical Debt Improvements:")
    lines.append(f"- Total debt score: {analysis['total_before']:.0f} → {analysis['total_after']:.0f} (-{analysis['improvement_pct']:.1f}%)")
    lines.append(f"- Items resolved: {len(analysis['resolved'])} completely eliminated")
    lines.append(f"- Overall items: {analysis['items_before']} → {analysis['items_after']} ({analysis['items_after'] - analysis['items_before']:+d})")
    lines.append("")

    # Category breakdown
    if analysis['category_before']:
        lines.append("By category:")
        for debt_type in sorted(analysis['category_before'].keys()):
            before = analysis['category_before'][debt_type]
            after = analysis['category_after'].get(debt_type, 0)
            if before > 0:
                pct_change = ((before - after) / before * 100)
                lines.append(f"- {format_debt_type(debt_type)}: {before:.0f} → {after:.0f} (-{pct_change:.1f}%)")
        lines.append("")

    # Top improvements
    if analysis['improved'] or analysis['resolved']:
        lines.append("Top improvements:")

        # Show top resolved items
        resolved_items = []
        for key in list(analysis['resolved'])[:5]:
            item = analysis['before_map'][key]
            file_path, function, line = key
            function = function if function else '(module level)'
            score = get_unified_score(item)
            resolved_items.append((file_path, function, score))

        resolved_items.sort(key=lambda x: x[2], reverse=True)

        count = 1
        for file_path, function, score in resolved_items[:3]:
            lines.append(f"{count}. {file_path}::{function}: score {score:.0f} → 0 (resolved)")
            count += 1

        # Show top improved items
        for key, before_score, after_score in analysis['improved'][:3]:
            file_path, function, line = key
            function = function if function else '(module level)'
            pct = ((before_score - after_score) / before_score * 100)
            lines.append(f"{count}. {file_path}::{function}: score {before_score:.0f} → {after_score:.0f} (-{pct:.1f}%)")
            count += 1
            if count > 5:
                break
        lines.append("")

    # Regressions
    if analysis['regressed'] or analysis['new_items']:
        lines.append("⚠️ Items requiring attention:")

        for key, before_score, after_score in analysis['regressed'][:3]:
            file_path, function, line = key
            function = function if function else '(module level)'
            pct = ((after_score - before_score) / before_score * 100)
            lines.append(f"- {file_path}::{function}: score {before_score:.0f} → {after_score:.0f} (+{pct:.1f}%)")

        for key in list(analysis['new_items'])[:3]:
            item = analysis['after_map'][key]
            file_path, function, line = key
            function = function if function else '(module level)'
            score = get_unified_score(item)
            lines.append(f"- NEW: {file_path}::{function}: score {score:.0f}")

        lines.append("")

    # Map results summary
    if map_results and 'results' in map_results:
        successful_items = [r for r in map_results['results'] if r.get('success')]
        failed_items = [r for r in map_results['results'] if not r.get('success')]

        if failed_items:
            lines.append("Failed items:")
            for result in failed_items[:3]:
                item = result.get('item', {})
                file_path = item.get('file_path', 'unknown')
                function = item.get('function_name', '(module level)')
                error = result.get('error', 'Unknown error')
                lines.append(f"- {file_path}::{function}: {error}")
            lines.append("")

    lines.append("This commit represents the aggregated work of multiple parallel agents.")

    return '\n'.join(lines)


def main():
    before_path = '.prodigy/debtmap-before.json'
    after_path = '.prodigy/debtmap-after.json'
    map_results_path = '.prodigy/map-results.json'
    successful = 9
    failed = 1
    total = 10

    # Load data
    print("Loading debt analysis files...", file=sys.stderr)
    before_data = load_json(before_path)
    after_data = load_json(after_path)

    map_results = None
    if Path(map_results_path).exists():
        map_results = load_json(map_results_path)

    # Analyze changes
    print("Analyzing debt changes...", file=sys.stderr)
    analysis = analyze_debt_changes(before_data, after_data, map_results)

    # Generate commit message
    commit_message = generate_commit_message(analysis, successful, failed, total, map_results)

    # Output commit message
    print(commit_message)


if __name__ == '__main__':
    main()
