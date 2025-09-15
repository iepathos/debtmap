#!/usr/bin/env python3
"""Validate technical debt improvements by comparing before and after debtmap states."""

import json
import os
import sys
from typing import Dict, List, Any, Optional, Tuple
from pathlib import Path


def parse_arguments() -> Tuple[str, str, str]:
    """Parse command line arguments."""
    args = os.environ.get('ARGUMENTS', '').split() or sys.argv[1:]

    # Default values
    before_file = None
    after_file = None
    output_file = ".prodigy/debtmap-validation.json"

    # If no arguments provided, use defaults for testing
    if not args:
        before_file = ".prodigy/debtmap-before.json"
        after_file = ".prodigy/debtmap-after.json"
    else:
        i = 0
        while i < len(args):
            if args[i] == '--before' and i + 1 < len(args):
                before_file = args[i + 1]
                i += 2
            elif args[i] == '--after' and i + 1 < len(args):
                after_file = args[i + 1]
                i += 2
            elif args[i] == '--output' and i + 1 < len(args):
                output_file = args[i + 1]
                i += 2
            else:
                i += 1

    if not before_file or not after_file:
        print("Error: Missing required arguments")
        print("Usage: --before <file> --after <file> [--output <file>]")
        sys.exit(1)

    return before_file, after_file, output_file


def load_debtmap_json(filepath: str) -> Optional[Dict[str, Any]]:
    """Load and validate debtmap JSON file."""
    try:
        with open(filepath, 'r') as f:
            data = json.load(f)
            # Validate it has expected structure
            if not isinstance(data, dict):
                return None
            return data
    except (FileNotFoundError, json.JSONDecodeError) as e:
        print(f"Error loading {filepath}: {e}")
        return None


def extract_debt_items(data: Dict[str, Any]) -> List[Dict[str, Any]]:
    """Extract debt items from debtmap data."""
    items = []

    # Handle different possible structures
    if 'debt_items' in data:
        items = data['debt_items']
    elif 'items' in data:
        items = data['items']
    elif 'functions' in data:
        # Convert function analysis to debt items
        for func in data['functions']:
            if isinstance(func, dict):
                score = func.get('debt_score', func.get('score', 0))
                if score > 0:
                    items.append({
                        'location': f"{func.get('file', 'unknown')}:{func.get('name', 'unknown')}:{func.get('line', 0)}",
                        'score': score,
                        'complexity': func.get('complexity', 0),
                        'coverage': func.get('coverage', 0),
                        'type': func.get('debt_type', 'complexity'),
                        'description': func.get('description', ''),
                        'function': func.get('name', 'unknown')
                    })

    return items


def calculate_metrics(data: Dict[str, Any], items: List[Dict[str, Any]]) -> Dict[str, Any]:
    """Calculate metrics from debtmap data."""
    if not items:
        return {
            'total_items': 0,
            'high_priority_items': 0,
            'critical_items': 0,
            'average_score': 0,
            'total_debt_score': 0,
            'average_complexity': 0,
            'coverage_gaps': 0
        }

    scores = [item.get('score', 0) for item in items]
    complexities = [item.get('complexity', 0) for item in items if item.get('complexity', 0) > 0]

    metrics = {
        'total_items': len(items),
        'high_priority_items': len([i for i in items if i.get('score', 0) >= 6]),
        'critical_items': len([i for i in items if i.get('score', 0) >= 8]),
        'average_score': sum(scores) / len(scores) if scores else 0,
        'total_debt_score': sum(scores),
        'average_complexity': sum(complexities) / len(complexities) if complexities else 0,
        'coverage_gaps': len([i for i in items if i.get('coverage', 100) < 50])
    }

    # Also check for overall metrics in the data
    if 'summary' in data:
        summary = data['summary']
        metrics['project_score'] = summary.get('total_score', metrics['total_debt_score'])
        metrics['project_health'] = summary.get('health_score', 100 - metrics['average_score'] * 10)

    return metrics


def compare_items(before_items: List[Dict[str, Any]], after_items: List[Dict[str, Any]]) -> Dict[str, Any]:
    """Compare individual debt items to identify improvements."""
    # Create maps for easier comparison
    before_map = {item['location']: item for item in before_items if 'location' in item}
    after_map = {item['location']: item for item in after_items if 'location' in item}

    resolved = []
    improved = []
    worsened = []
    new_items = []
    unchanged_critical = []

    # Check items that were in before
    for location, before_item in before_map.items():
        if location not in after_map:
            # Item was resolved
            resolved.append(before_item)
        else:
            after_item = after_map[location]
            before_score = before_item.get('score', 0)
            after_score = after_item.get('score', 0)

            if after_score < before_score:
                improved.append({
                    'item': after_item,
                    'improvement': before_score - after_score,
                    'before_score': before_score,
                    'after_score': after_score
                })
            elif after_score > before_score:
                worsened.append({
                    'item': after_item,
                    'regression': after_score - before_score,
                    'before_score': before_score,
                    'after_score': after_score
                })
            elif before_score >= 8:  # Critical item unchanged
                unchanged_critical.append(before_item)

    # Check for new items
    for location, after_item in after_map.items():
        if location not in before_map:
            new_items.append(after_item)

    return {
        'resolved': resolved,
        'improved': improved,
        'worsened': worsened,
        'new_items': new_items,
        'unchanged_critical': unchanged_critical
    }


def calculate_improvement_score(before_metrics: Dict[str, Any],
                               after_metrics: Dict[str, Any],
                               comparison: Dict[str, Any]) -> float:
    """Calculate weighted improvement score."""

    # Calculate individual components
    resolved_high_priority = 0
    if before_metrics['critical_items'] > 0:
        resolved_critical = len([i for i in comparison['resolved'] if i.get('score', 0) >= 8])
        resolved_high_priority = (resolved_critical / before_metrics['critical_items']) * 100
    elif before_metrics['high_priority_items'] > 0:
        resolved_high = len([i for i in comparison['resolved'] if i.get('score', 0) >= 6])
        resolved_high_priority = (resolved_high / before_metrics['high_priority_items']) * 100

    # Overall score improvement
    score_improvement = 0
    if before_metrics['total_debt_score'] > 0:
        score_reduction = before_metrics['total_debt_score'] - after_metrics['total_debt_score']
        score_improvement = (score_reduction / before_metrics['total_debt_score']) * 100
        score_improvement = max(0, score_improvement)  # Don't go negative

    # Complexity reduction
    complexity_reduction = 0
    if before_metrics['average_complexity'] > 0:
        complexity_diff = before_metrics['average_complexity'] - after_metrics['average_complexity']
        complexity_reduction = (complexity_diff / before_metrics['average_complexity']) * 100
        complexity_reduction = max(0, complexity_reduction)

    # Penalty for new critical debt
    new_critical_penalty = 0
    new_critical = len([i for i in comparison['new_items'] if i.get('score', 0) >= 8])
    if new_critical > 0:
        new_critical_penalty = 25  # 25% penalty for introducing critical debt

    # Calculate weighted score
    improvement_score = (
        resolved_high_priority * 0.4 +
        score_improvement * 0.3 +
        complexity_reduction * 0.2 +
        (100 - new_critical_penalty) * 0.1
    )

    return min(100, max(0, improvement_score))  # Clamp to 0-100


def identify_gaps(before_items: List[Dict[str, Any]],
                 after_items: List[Dict[str, Any]],
                 comparison: Dict[str, Any],
                 improvement_score: float) -> Dict[str, Any]:
    """Identify specific gaps if improvement is insufficient."""
    gaps = {}

    if improvement_score >= 75:
        return gaps  # Good enough improvement

    # Check for unresolved critical items
    for idx, item in enumerate(comparison['unchanged_critical'][:3]):  # Top 3 critical
        gaps[f'critical_debt_remaining_{idx}'] = {
            'description': f"High-priority debt item still present: {item.get('description', 'Complex function')}",
            'location': item.get('location', 'unknown'),
            'severity': 'critical',
            'suggested_fix': 'Apply functional programming patterns to reduce complexity',
            'original_score': item.get('score', 0),
            'current_score': item.get('score', 0)
        }

    # Check for insufficient refactoring
    for idx, improved in enumerate(comparison['improved'][:2]):  # Top 2 partial improvements
        if improved['after_score'] >= 6:  # Still high priority
            gaps[f'insufficient_refactoring_{idx}'] = {
                'description': 'Function complexity reduced but still above threshold',
                'location': improved['item'].get('location', 'unknown'),
                'severity': 'medium' if improved['after_score'] < 8 else 'high',
                'suggested_fix': 'Extract helper functions using pure functional patterns',
                'original_score': improved['before_score'],
                'current_score': improved['after_score'],
                'target_score': 4.0
            }

    # Check for regressions
    for idx, worsened in enumerate(comparison['worsened'][:2]):  # Top 2 regressions
        gaps[f'regression_detected_{idx}'] = {
            'description': 'Debt score increased during refactoring',
            'location': worsened['item'].get('location', 'unknown'),
            'severity': 'high' if worsened['after_score'] >= 8 else 'medium',
            'suggested_fix': 'Review changes and simplify the implementation',
            'original_score': worsened['before_score'],
            'current_score': worsened['after_score']
        }

    # Check for new critical items
    new_critical = [i for i in comparison['new_items'] if i.get('score', 0) >= 8]
    for idx, item in enumerate(new_critical[:2]):  # Top 2 new critical
        gaps[f'new_critical_debt_{idx}'] = {
            'description': 'New critical debt introduced',
            'location': item.get('location', 'unknown'),
            'severity': 'critical',
            'suggested_fix': 'Remove or simplify the newly added complexity',
            'original_score': None,
            'current_score': item.get('score', 0)
        }

    return gaps


def format_improvements(comparison: Dict[str, Any],
                        before_metrics: Dict[str, Any],
                        after_metrics: Dict[str, Any]) -> List[str]:
    """Format improvement messages."""
    improvements = []

    if comparison['resolved']:
        critical_resolved = len([i for i in comparison['resolved'] if i.get('score', 0) >= 8])
        high_resolved = len([i for i in comparison['resolved'] if i.get('score', 0) >= 6])
        if critical_resolved > 0:
            improvements.append(f"Resolved {critical_resolved} critical debt items")
        elif high_resolved > 0:
            improvements.append(f"Resolved {high_resolved} high-priority debt items")
        else:
            improvements.append(f"Resolved {len(comparison['resolved'])} debt items")

    if before_metrics['average_complexity'] > after_metrics['average_complexity']:
        reduction = ((before_metrics['average_complexity'] - after_metrics['average_complexity']) /
                    before_metrics['average_complexity'] * 100)
        improvements.append(f"Reduced average complexity by {reduction:.0f}%")

    if before_metrics['coverage_gaps'] > after_metrics['coverage_gaps']:
        improvements.append(f"Improved test coverage for {before_metrics['coverage_gaps'] - after_metrics['coverage_gaps']} functions")

    if comparison['improved']:
        improvements.append(f"Improved {len(comparison['improved'])} existing debt items")

    return improvements


def format_remaining_issues(comparison: Dict[str, Any], after_metrics: Dict[str, Any]) -> List[str]:
    """Format remaining issues."""
    issues = []

    if comparison['unchanged_critical']:
        issues.append(f"{len(comparison['unchanged_critical'])} critical debt items still present")

    if comparison['new_items']:
        critical_new = len([i for i in comparison['new_items'] if i.get('score', 0) >= 8])
        if critical_new > 0:
            issues.append(f"{critical_new} new critical debt items introduced")
        else:
            issues.append(f"{len(comparison['new_items'])} new debt items introduced")

    if comparison['worsened']:
        issues.append(f"{len(comparison['worsened'])} items worsened")

    if after_metrics['critical_items'] > 0:
        issues.append(f"{after_metrics['critical_items']} critical items remain")

    return issues


def write_validation_result(output_file: str, result: Dict[str, Any]):
    """Write validation result to file."""
    # Ensure directory exists
    output_path = Path(output_file)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    # Write JSON result
    with open(output_file, 'w') as f:
        json.dump(result, f, indent=2)

    print(f"Validation result written to {output_file}")


def main():
    """Main validation logic."""
    # Check for automation mode
    is_automation = os.environ.get('PRODIGY_AUTOMATION') == 'true' or \
                    os.environ.get('PRODIGY_VALIDATION') == 'true'

    if not is_automation:
        print("Starting debtmap improvement validation...")

    # Parse arguments
    before_file, after_file, output_file = parse_arguments()

    if not is_automation:
        print(f"Before file: {before_file}")
        print(f"After file: {after_file}")
        print(f"Output file: {output_file}")

    # Load before data
    before_data = load_debtmap_json(before_file)
    if not before_data:
        result = {
            "completion_percentage": 0.0,
            "status": "failed",
            "improvements": [],
            "remaining_issues": [f"Failed to load before debtmap from {before_file}"],
            "gaps": {},
            "error": "Invalid or missing before file"
        }
        write_validation_result(output_file, result)
        return 0

    # Load after data
    after_data = load_debtmap_json(after_file)
    if not after_data:
        result = {
            "completion_percentage": 0.0,
            "status": "failed",
            "improvements": [],
            "remaining_issues": [f"Failed to load after debtmap from {after_file}"],
            "gaps": {},
            "error": "Invalid or missing after file"
        }
        write_validation_result(output_file, result)
        return 0

    # Extract debt items
    before_items = extract_debt_items(before_data)
    after_items = extract_debt_items(after_data)

    if not is_automation:
        print(f"Found {len(before_items)} debt items before, {len(after_items)} after")

    # Calculate metrics
    before_metrics = calculate_metrics(before_data, before_items)
    after_metrics = calculate_metrics(after_data, after_items)

    # Compare items
    comparison = compare_items(before_items, after_items)

    # Calculate improvement score
    improvement_score = calculate_improvement_score(before_metrics, after_metrics, comparison)

    # Identify gaps if needed
    gaps = identify_gaps(before_items, after_items, comparison, improvement_score)

    # Format results
    improvements = format_improvements(comparison, before_metrics, after_metrics)
    remaining_issues = format_remaining_issues(comparison, after_metrics)

    # Determine status
    if improvement_score >= 75:
        status = "complete"
    elif improvement_score >= 40:
        status = "incomplete"
    else:
        status = "failed"

    # Create result
    result = {
        "completion_percentage": round(improvement_score, 1),
        "status": status,
        "improvements": improvements,
        "remaining_issues": remaining_issues,
        "gaps": gaps,
        "before_summary": {
            "total_items": before_metrics['total_items'],
            "high_priority_items": before_metrics['high_priority_items'],
            "critical_items": before_metrics['critical_items'],
            "average_score": round(before_metrics['average_score'], 2)
        },
        "after_summary": {
            "total_items": after_metrics['total_items'],
            "high_priority_items": after_metrics['high_priority_items'],
            "critical_items": after_metrics['critical_items'],
            "average_score": round(after_metrics['average_score'], 2)
        }
    }

    # Write result
    write_validation_result(output_file, result)

    if not is_automation:
        print(f"Validation complete: {improvement_score:.1f}% improvement ({status})")
        if improvements:
            print("\nImprovements:")
            for imp in improvements[:3]:
                print(f"  - {imp}")
        if remaining_issues and improvement_score < 75:
            print("\nRemaining issues:")
            for issue in remaining_issues[:3]:
                print(f"  - {issue}")

    return 0


if __name__ == "__main__":
    sys.exit(main())