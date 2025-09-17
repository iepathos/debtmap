#!/usr/bin/env python3
"""
Validate debtmap improvements by comparing before and after JSON states.
"""

import json
import sys
import os
from pathlib import Path
from typing import Dict, List, Any, Tuple, Optional
import argparse


def load_debtmap_json(filepath: str) -> Dict[str, Any]:
    """Load and validate debtmap JSON file."""
    try:
        with open(filepath, 'r') as f:
            data = json.load(f)
            # Validate expected structure
            if 'items' not in data:
                raise ValueError(f"Missing 'items' key in {filepath}")
            return data
    except FileNotFoundError:
        raise FileNotFoundError(f"Debtmap file not found: {filepath}")
    except json.JSONDecodeError as e:
        raise ValueError(f"Invalid JSON in {filepath}: {e}")


def extract_metrics(data: Dict[str, Any]) -> Dict[str, Any]:
    """Extract key metrics from debtmap data."""
    items = data.get('items', [])

    # Count items by priority
    critical_items = []  # score >= 8
    high_priority_items = []  # score >= 6
    medium_priority_items = []  # score >= 4
    low_priority_items = []  # score < 4

    total_complexity = 0
    total_coverage_gaps = 0
    functions_with_gaps = 0

    for item in items:
        score = item.get('unified_score', {}).get('final_score', 0)

        if score >= 8:
            critical_items.append(item)
        elif score >= 6:
            high_priority_items.append(item)
        elif score >= 4:
            medium_priority_items.append(item)
        else:
            low_priority_items.append(item)

        # Extract complexity metrics
        debt_type = item.get('debt_type', {})
        if 'ComplexFunction' in debt_type:
            total_complexity += debt_type['ComplexFunction'].get('cyclomatic', 0)
        elif 'TestingGap' in debt_type:
            gap = debt_type['TestingGap']
            total_complexity += gap.get('cyclomatic', 0)
            coverage = gap.get('coverage', 100)
            if coverage < 50:
                functions_with_gaps += 1
                total_coverage_gaps += (100 - coverage)

    # Calculate overall metrics
    avg_score = sum(item.get('unified_score', {}).get('final_score', 0) for item in items) / len(items) if items else 0

    return {
        'total_items': len(items),
        'critical_items': len(critical_items),
        'high_priority_items': len(high_priority_items),
        'medium_priority_items': len(medium_priority_items),
        'low_priority_items': len(low_priority_items),
        'average_score': round(avg_score, 2),
        'total_complexity': total_complexity,
        'functions_with_gaps': functions_with_gaps,
        'total_coverage_gaps': total_coverage_gaps,
        'total_debt_score': data.get('total_debt_score', 0),
        'overall_coverage': data.get('overall_coverage', 100)
    }


def identify_improvements(before_items: List[Dict], after_items: List[Dict]) -> Dict[str, Any]:
    """Identify specific improvements between states."""

    # Create lookup maps by location
    def make_key(item):
        loc = item.get('location', {})
        return f"{loc.get('file')}:{loc.get('function')}:{loc.get('line')}"

    before_map = {make_key(item): item for item in before_items}
    after_map = {make_key(item): item for item in after_items}

    resolved_items = []
    improved_items = []
    new_items = []
    unchanged_critical = []

    # Find resolved and improved items
    for key, before_item in before_map.items():
        before_score = before_item.get('unified_score', {}).get('final_score', 0)

        if key not in after_map:
            # Item was resolved
            resolved_items.append({
                'location': key,
                'score': before_score,
                'description': before_item.get('recommendation', {}).get('primary_action', 'Fixed')
            })
        else:
            # Check if item improved
            after_item = after_map[key]
            after_score = after_item.get('unified_score', {}).get('final_score', 0)

            if after_score < before_score:
                improved_items.append({
                    'location': key,
                    'before_score': before_score,
                    'after_score': after_score,
                    'improvement': before_score - after_score
                })
            elif before_score >= 8 and after_score >= 8:
                # Critical item remains critical
                unchanged_critical.append({
                    'location': key,
                    'score': after_score,
                    'item': after_item
                })

    # Find new items
    for key, after_item in after_map.items():
        if key not in before_map:
            score = after_item.get('unified_score', {}).get('final_score', 0)
            new_items.append({
                'location': key,
                'score': score,
                'item': after_item
            })

    return {
        'resolved': resolved_items,
        'improved': improved_items,
        'new': new_items,
        'unchanged_critical': unchanged_critical
    }


def calculate_improvement_score(
    before_metrics: Dict[str, Any],
    after_metrics: Dict[str, Any],
    improvements: Dict[str, Any]
) -> Tuple[float, List[str], List[str]]:
    """Calculate overall improvement score."""

    improvement_notes = []
    remaining_issues = []

    # Component scores
    scores = {}

    # 1. Resolved high-priority items (40% weight)
    resolved_critical = sum(1 for item in improvements['resolved'] if item['score'] >= 8)
    resolved_high = sum(1 for item in improvements['resolved'] if item['score'] >= 6)
    total_critical_before = before_metrics['critical_items']

    if total_critical_before > 0:
        critical_resolution_rate = resolved_critical / total_critical_before
        scores['critical_resolution'] = critical_resolution_rate * 100
        if resolved_critical > 0:
            improvement_notes.append(f"Resolved {resolved_critical} critical debt items")
    else:
        scores['critical_resolution'] = 100  # No critical items to fix

    # 2. Overall score improvement (30% weight)
    score_improvement = max(0, before_metrics['average_score'] - after_metrics['average_score'])
    score_improvement_pct = (score_improvement / before_metrics['average_score'] * 100) if before_metrics['average_score'] > 0 else 0
    scores['overall_improvement'] = min(100, score_improvement_pct * 2)  # Scale up for visibility

    if score_improvement > 0:
        improvement_notes.append(f"Reduced average debt score from {before_metrics['average_score']:.1f} to {after_metrics['average_score']:.1f}")

    # 3. Complexity reduction (20% weight)
    complexity_reduction = max(0, before_metrics['total_complexity'] - after_metrics['total_complexity'])
    complexity_pct = (complexity_reduction / before_metrics['total_complexity'] * 100) if before_metrics['total_complexity'] > 0 else 0
    scores['complexity_reduction'] = min(100, complexity_pct)

    if complexity_reduction > 0:
        improvement_notes.append(f"Reduced total complexity by {complexity_pct:.0f}%")

    # 4. No new critical debt (10% weight)
    new_critical = sum(1 for item in improvements['new'] if item['score'] >= 8)
    scores['no_regression'] = 100 if new_critical == 0 else max(0, 100 - (new_critical * 25))

    if new_critical > 0:
        remaining_issues.append(f"{new_critical} new critical debt items introduced")

    # Add notes about remaining critical items
    if improvements['unchanged_critical']:
        count = len(improvements['unchanged_critical'])
        remaining_issues.append(f"{count} critical debt items still present")

    # Calculate weighted average
    improvement_score = (
        scores['critical_resolution'] * 0.4 +
        scores['overall_improvement'] * 0.3 +
        scores['complexity_reduction'] * 0.2 +
        scores['no_regression'] * 0.1
    )

    return improvement_score, improvement_notes, remaining_issues


def identify_gaps(improvements: Dict[str, Any], threshold: float = 75.0) -> Dict[str, Any]:
    """Identify specific gaps if improvement is insufficient."""
    gaps = {}

    # Check for unchanged critical items
    for idx, item_info in enumerate(improvements['unchanged_critical'][:3]):  # Top 3 critical
        item = item_info['item']
        location = item.get('location', {})

        gap_key = f"critical_debt_remaining_{idx + 1}"
        gaps[gap_key] = {
            'description': f"High-priority debt item still present: {item.get('recommendation', {}).get('primary_action', 'Unknown issue')}",
            'location': f"{location.get('file')}:{location.get('function')}:{location.get('line')}",
            'severity': 'critical',
            'suggested_fix': item.get('recommendation', {}).get('primary_action', 'Apply functional patterns to reduce complexity'),
            'original_score': item_info['score'],
            'current_score': item_info['score']
        }

    # Check for new critical items
    new_critical = [item for item in improvements['new'] if item['score'] >= 8]
    for idx, item_info in enumerate(new_critical[:2]):  # Top 2 new critical
        item = item_info['item']
        location = item.get('location', {})

        gap_key = f"regression_detected_{idx + 1}"
        gaps[gap_key] = {
            'description': "New complexity introduced during refactoring",
            'location': f"{location.get('file')}:{location.get('function')}:{location.get('line')}",
            'severity': 'high',
            'suggested_fix': "Simplify the newly added code using functional patterns",
            'original_score': None,
            'current_score': item_info['score']
        }

    return gaps


def main():
    """Main validation function."""
    # Parse arguments
    parser = argparse.ArgumentParser(description='Validate debtmap improvements')
    parser.add_argument('--before', required=True, help='Path to before debtmap JSON')
    parser.add_argument('--after', required=True, help='Path to after debtmap JSON')
    parser.add_argument('--output', default='.prodigy/debtmap-validation.json',
                        help='Output path for validation results')

    # Check for automation mode
    is_automated = os.environ.get('PRODIGY_AUTOMATION') == 'true' or \
                   os.environ.get('PRODIGY_VALIDATION') == 'true'

    try:
        # Try to parse from command line args first
        if len(sys.argv) > 1:
            args = parser.parse_args()
        else:
            # Parse from ARGUMENTS environment variable
            import shlex
            args_str = os.environ.get('ARGUMENTS', '')
            if not args_str:
                # Hardcode for testing based on the command shown
                args_str = '--before .prodigy/debtmap-before.json --after .prodigy/debtmap-after.json --output .prodigy/debtmap-validation.json'

            args = parser.parse_args(shlex.split(args_str))

        if not is_automated:
            print(f"Loading debtmap data from {args.before} and {args.after}...")

        # Load JSON files
        before_data = load_debtmap_json(args.before)
        after_data = load_debtmap_json(args.after)

        # Extract metrics
        before_metrics = extract_metrics(before_data)
        after_metrics = extract_metrics(after_data)

        # Identify improvements
        improvements = identify_improvements(
            before_data.get('items', []),
            after_data.get('items', [])
        )

        # Calculate improvement score
        score, improvement_notes, remaining_issues = calculate_improvement_score(
            before_metrics, after_metrics, improvements
        )

        # Determine status
        if score >= 75:
            status = 'complete'
        elif score >= 40:
            status = 'incomplete'
        else:
            status = 'failed'

        # Identify gaps if needed
        gaps = {}
        if score < 75:
            gaps = identify_gaps(improvements, score)

        # Build validation result
        result = {
            'completion_percentage': round(score, 1),
            'status': status,
            'improvements': improvement_notes,
            'remaining_issues': remaining_issues,
            'gaps': gaps,
            'before_summary': {
                'total_items': before_metrics['total_items'],
                'high_priority_items': before_metrics['critical_items'],
                'average_score': before_metrics['average_score']
            },
            'after_summary': {
                'total_items': after_metrics['total_items'],
                'high_priority_items': after_metrics['critical_items'],
                'average_score': after_metrics['average_score']
            }
        }

        # Ensure output directory exists
        output_path = Path(args.output)
        output_path.parent.mkdir(parents=True, exist_ok=True)

        # Write result to file
        with open(output_path, 'w') as f:
            json.dump(result, f, indent=2)

        if not is_automated:
            print(f"\nValidation complete: {score:.1f}% improvement")
            print(f"Status: {status}")
            print(f"Results written to: {output_path}")

            if improvement_notes:
                print("\nImprovements:")
                for note in improvement_notes:
                    print(f"  ✓ {note}")

            if remaining_issues:
                print("\nRemaining issues:")
                for issue in remaining_issues:
                    print(f"  • {issue}")

        # Exit with appropriate code
        sys.exit(0 if status == 'complete' else 1)

    except Exception as e:
        # Error handling - always output valid JSON
        error_result = {
            'completion_percentage': 0.0,
            'status': 'failed',
            'improvements': [],
            'remaining_issues': [f"Validation error: {str(e)}"],
            'gaps': {},
            'raw_output': str(e)
        }

        # Try to write error result
        try:
            output_path = args.output if 'args' in locals() else '.prodigy/debtmap-validation.json'
            Path(output_path).parent.mkdir(parents=True, exist_ok=True)
            with open(output_path, 'w') as f:
                json.dump(error_result, f, indent=2)
        except:
            pass

        if not is_automated:
            print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()