#!/usr/bin/env python3
"""Compare debtmap results before and after fixes."""

import json
import sys
from collections import defaultdict

def load_json(path: str) -> dict:
    """Load JSON file."""
    with open(path, 'r') as f:
        return json.load(f)

def analyze_debt_changes(before_data: dict, after_data: dict) -> dict:
    """Analyze changes between before and after debt data."""
    # Get top-level metrics
    before_score = before_data.get('total_debt_score', 0)
    after_score = after_data.get('total_debt_score', 0)
    before_impact = before_data.get('total_impact', {})
    after_impact = after_data.get('total_impact', {})
    
    before_items = before_data.get('items', [])
    after_items = after_data.get('items', [])

    return {
        'total_before': before_score,
        'total_after': after_score,
        'impact_before': before_impact,
        'impact_after': after_impact,
        'count_before': len(before_items),
        'count_after': len(after_items),
    }

def generate_commit_message(analysis: dict, map_results: list, successful: int, failed: int, total: int) -> str:
    """Generate formatted commit message."""
    lines = []

    # Title
    lines.append(f"fix: eliminate {successful} technical debt items via MapReduce")
    lines.append("")

    # Processing summary
    lines.append(f"Processed {total} debt items in parallel:")
    lines.append(f"- Successfully fixed: {successful} items")
    lines.append(f"- Failed to fix: {failed} items")
    lines.append("")

    # Overall metrics
    total_before = analysis['total_before']
    total_after = analysis['total_after']
    count_before = analysis['count_before']
    count_after = analysis['count_after']
    impact_before = analysis['impact_before']
    impact_after = analysis['impact_after']

    if total_before > 0:
        # Calculate improvement (negative means better)
        score_change = total_after - total_before
        improvement_pct = (score_change / total_before) * 100
        count_change = count_after - count_before

        lines.append("Technical Debt Improvements:")
        lines.append(f"- Total debt score: {total_before:.1f} → {total_after:.1f} ({improvement_pct:+.1f}%)")
        lines.append(f"- Overall items: {count_before} → {count_after} ({count_change:+d})")
        
        # Show impact metrics if available
        if impact_before and impact_after:
            lines.append("")
            lines.append("Impact metrics:")
            
            complexity_before = impact_before.get('complexity_reduction', 0)
            complexity_after = impact_after.get('complexity_reduction', 0)
            if complexity_before > 0:
                complexity_change = complexity_after - complexity_before
                lines.append(f"- Complexity reduction: {complexity_before:.1f} → {complexity_after:.1f} ({complexity_change:+.1f})")
            
            lines_before = impact_before.get('lines_reduction', 0)
            lines_after = impact_after.get('lines_reduction', 0)
            if lines_before > 0:
                lines_change = lines_after - lines_before
                lines.append(f"- Lines reduction opportunity: {lines_before} → {lines_after} ({lines_change:+d})")
            
            risk_before = impact_before.get('risk_reduction', 0)
            risk_after = impact_after.get('risk_reduction', 0)
            if risk_before > 0:
                risk_change = risk_after - risk_before
                lines.append(f"- Risk reduction: {risk_before:.1f} → {risk_after:.1f} ({risk_change:+.1f})")
        
        lines.append("")

    # Map phase details
    if map_results:
        success_count = sum(1 for r in map_results if r.get('status') == 'Success')
        failed_count = sum(1 for r in map_results if r.get('status') == 'Failed')
        
        lines.append("Map phase summary:")
        lines.append(f"- Successful fixes: {success_count}/{len(map_results)}")
        if failed_count > 0:
            lines.append(f"- Failed fixes: {failed_count}/{len(map_results)}")
        lines.append("")

        # Show individual items
        lines.append("Items processed:")
        for result in map_results[:10]:
            item_id = result.get('item_id', 'unknown')
            status = result.get('status', 'unknown')
            symbol = "✓" if status == 'Success' else "✗"
            lines.append(f"  {symbol} {item_id}: {status}")
        
        if len(map_results) > 10:
            lines.append(f"  ... and {len(map_results) - 10} more items")
        lines.append("")

    lines.append("This commit represents the aggregated work of multiple parallel agents.")

    return '\n'.join(lines)

def main():
    """Main entry point."""
    if len(sys.argv) < 7:
        print("Usage: compare_debt.py <before.json> <after.json> <map-results.json> <successful> <failed> <total>")
        sys.exit(1)

    before_path = sys.argv[1]
    after_path = sys.argv[2]
    map_results_path = sys.argv[3]
    successful = int(sys.argv[4])
    failed = int(sys.argv[5])
    total = int(sys.argv[6])

    # Load data
    before_data = load_json(before_path)
    after_data = load_json(after_path)

    map_results = []
    try:
        map_results_data = load_json(map_results_path)
        map_results = map_results_data if isinstance(map_results_data, list) else map_results_data.get('results', [])
    except (FileNotFoundError, json.JSONDecodeError):
        pass  # Map results are optional

    # Analyze changes
    analysis = analyze_debt_changes(before_data, after_data)

    # Generate commit message
    commit_message = generate_commit_message(analysis, map_results, successful, failed, total)

    # Write to file for git commit
    with open('.prodigy/commit-message.txt', 'w') as f:
        f.write(commit_message)

    # Also print to stdout
    print(commit_message)
    print("\n" + "="*80)
    print(f"Commit message written to .prodigy/commit-message.txt")

if __name__ == '__main__':
    main()
