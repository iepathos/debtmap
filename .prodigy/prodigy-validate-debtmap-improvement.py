#!/usr/bin/env python3
"""
Validate debtmap improvements by comparing before and after JSON outputs.
"""

import json
import sys
import os
import argparse
from pathlib import Path
from typing import Dict, List, Any, Optional, Tuple


def parse_arguments(args_string: str) -> Tuple[str, str, str]:
    """Parse command line arguments from the provided string."""
    parser = argparse.ArgumentParser(description='Validate debtmap improvements')
    parser.add_argument('--before', required=True, help='Path to before JSON file')
    parser.add_argument('--after', required=True, help='Path to after JSON file')
    parser.add_argument('--output', default='.prodigy/debtmap-validation.json',
                        help='Path to output validation JSON file')

    # Parse the arguments from the string
    args = parser.parse_args(args_string.split())
    return args.before, args.after, args.output


def load_json_file(filepath: str) -> Optional[Dict[str, Any]]:
    """Load and parse a JSON file."""
    try:
        with open(filepath, 'r') as f:
            return json.load(f)
    except (FileNotFoundError, json.JSONDecodeError) as e:
        print(f"Error loading {filepath}: {e}", file=sys.stderr)
        return None


def format_location_string(location_obj: Any) -> str:
    """Format location object into a standardized string."""
    if isinstance(location_obj, dict):
        file_name = location_obj.get('file', 'unknown')
        function_name = location_obj.get('function', 'unknown')
        line_number = location_obj.get('line', 0)
        return f"{file_name}:{function_name}:{line_number}"
    return str(location_obj)


def calculate_score_from_item(item: Dict[str, Any]) -> float:
    """Calculate debt score from item, with fallback logic."""
    # Try unified_score first
    unified_score = item.get('unified_score', {})
    score = unified_score.get('final_score', 0)

    if score > 0:
        return score

    # Fallback to complexity metrics
    cyclomatic = item.get('cyclomatic_complexity', 0)
    cognitive = item.get('cognitive_complexity', 0)
    return max(cyclomatic, cognitive)


def extract_complexity_from_item(item: Dict[str, Any]) -> int:
    """Extract complexity metric from item with fallback logic."""
    complexity = item.get('cyclomatic_complexity', 0)

    if complexity > 0:
        return complexity

    # Check debt_type for complexity info
    debt_type = item.get('debt_type', {})
    if isinstance(debt_type, dict) and 'ComplexityHotspot' in debt_type:
        return debt_type['ComplexityHotspot'].get('cyclomatic', 0)

    return 0


def calculate_coverage_from_factor(coverage_factor: float) -> float:
    """Calculate coverage percentage from coverage factor."""
    if coverage_factor > 0:
        return 100 - (coverage_factor * 10)
    return 100


def determine_priority_from_score(score: float) -> str:
    """Determine priority level based on debt score."""
    if score >= 8:
        return 'critical'
    elif score >= 6:
        return 'high'
    elif score >= 4:
        return 'medium'
    else:
        return 'low'


def transform_current_format_item(item: Dict[str, Any]) -> Dict[str, Any]:
    """Transform a current format debt item into standardized format."""
    location_str = format_location_string(item.get('location', {}))
    score = calculate_score_from_item(item)
    complexity = extract_complexity_from_item(item)

    coverage_factor = item.get('unified_score', {}).get('coverage_factor', 0)
    coverage = calculate_coverage_from_factor(coverage_factor)

    priority = determine_priority_from_score(score)
    description = item.get('recommendation', {}).get('primary_action', '')

    return {
        'location': location_str,
        'score': score,
        'complexity': complexity,
        'coverage': coverage,
        'type': 'function',
        'description': description,
        'priority': priority,
        'raw_item': item
    }


def transform_legacy_format_item(func: Dict[str, Any]) -> Dict[str, Any]:
    """Transform a legacy format function into standardized format."""
    file_name = func.get('file', 'unknown')
    function_name = func.get('name', 'unknown')
    line_number = func.get('line', 0)
    location_str = f"{file_name}:{function_name}:{line_number}"

    debt_score = func.get('debt_score', 0)
    score = debt_score if debt_score > 0 else func.get('complexity', 0)
    priority = determine_priority_from_score(debt_score)

    return {
        'location': location_str,
        'score': score,
        'complexity': func.get('complexity', 0),
        'coverage': func.get('coverage', 0),
        'type': func.get('type', 'function'),
        'description': func.get('description', ''),
        'priority': priority
    }


def extract_items_from_current_format(data: Dict[str, Any]) -> List[Dict[str, Any]]:
    """Extract items from current debtmap format."""
    return [transform_current_format_item(item) for item in data['items']]


def extract_items_from_legacy_format(data: Dict[str, Any]) -> List[Dict[str, Any]]:
    """Extract items from legacy functions format."""
    return [
        transform_legacy_format_item(func)
        for func in data['functions']
        if func.get('debt_score', 0) > 0
    ]


def extract_debt_items(data: Dict[str, Any]) -> List[Dict[str, Any]]:
    """Extract debt items from debtmap JSON output."""
    if 'items' in data:
        return extract_items_from_current_format(data)
    elif 'debt_items' in data:
        return data['debt_items']
    elif 'functions' in data:
        return extract_items_from_legacy_format(data)
    else:
        return []


def calculate_metrics(items: List[Dict[str, Any]]) -> Dict[str, Any]:
    """Calculate summary metrics from debt items."""
    if not items:
        return {
            'total_items': 0,
            'high_priority_items': 0,
            'critical_items': 0,
            'average_score': 0,
            'average_complexity': 0,
            'total_score': 0
        }

    total_score = sum(item.get('score', 0) for item in items)
    total_complexity = sum(item.get('complexity', 0) for item in items)
    critical_items = [item for item in items if item.get('score', 0) >= 8]
    high_priority_items = [item for item in items if item.get('score', 0) >= 6]

    return {
        'total_items': len(items),
        'high_priority_items': len(high_priority_items),
        'critical_items': len(critical_items),
        'average_score': total_score / len(items) if items else 0,
        'average_complexity': total_complexity / len(items) if items else 0,
        'total_score': total_score
    }


def identify_improvements(before_items: List[Dict], after_items: List[Dict]) -> Dict[str, Any]:
    """Identify specific improvements between before and after states."""
    # Create location-based maps for comparison
    before_map = {item['location']: item for item in before_items}
    after_map = {item['location']: item for item in after_items}

    resolved_items = []
    improved_items = []
    new_items = []
    unchanged_critical = []

    # Find resolved and improved items
    for location, before_item in before_map.items():
        if location not in after_map:
            resolved_items.append(before_item)
        else:
            after_item = after_map[location]
            if after_item['score'] < before_item['score']:
                improved_items.append({
                    'location': location,
                    'before_score': before_item['score'],
                    'after_score': after_item['score'],
                    'improvement': before_item['score'] - after_item['score']
                })
            elif before_item['score'] >= 8 and after_item['score'] >= 8:
                unchanged_critical.append(after_item)

    # Find new items
    for location, after_item in after_map.items():
        if location not in before_map:
            new_items.append(after_item)

    return {
        'resolved_items': resolved_items,
        'improved_items': improved_items,
        'new_items': new_items,
        'unchanged_critical': unchanged_critical
    }


def calculate_improvement_score(before_metrics: Dict, after_metrics: Dict,
                                improvements: Dict) -> float:
    """Calculate overall improvement score based on weighted factors."""
    # Calculate component scores

    # 1. Resolved high-priority items (40% weight)
    resolved_critical = len([item for item in improvements['resolved_items']
                            if item.get('score', 0) >= 8])
    total_critical_before = before_metrics['critical_items']
    resolved_priority_score = (resolved_critical / total_critical_before * 100) \
                             if total_critical_before > 0 else 0

    # 2. Overall score improvement (30% weight)
    score_reduction = before_metrics['total_score'] - after_metrics['total_score']
    score_improvement_pct = (score_reduction / before_metrics['total_score'] * 100) \
                           if before_metrics['total_score'] > 0 else 0

    # 3. Complexity reduction (20% weight)
    complexity_reduction = before_metrics['average_complexity'] - after_metrics['average_complexity']
    complexity_improvement_pct = (complexity_reduction / before_metrics['average_complexity'] * 100) \
                                if before_metrics['average_complexity'] > 0 else 0

    # 4. No new critical debt (10% weight)
    new_critical = len([item for item in improvements['new_items']
                       if item.get('score', 0) >= 8])
    no_regression_score = 100 if new_critical == 0 else max(0, 100 - (new_critical * 25))

    # Calculate weighted average
    improvement_score = (
        resolved_priority_score * 0.4 +
        score_improvement_pct * 0.3 +
        complexity_improvement_pct * 0.2 +
        no_regression_score * 0.1
    )

    return min(100, max(0, improvement_score))  # Clamp to 0-100


def identify_gaps(improvements: Dict, after_items: List[Dict],
                 improvement_score: float) -> Dict[str, Any]:
    """Identify specific gaps if improvement is insufficient."""
    gaps = {}

    # Check for remaining critical items
    for item in improvements['unchanged_critical']:
        gap_id = f"critical_debt_{item['location'].replace(':', '_').replace('/', '_')}"
        gaps[gap_id] = {
            'description': f"High-priority debt item still present in {item.get('description', 'function')}",
            'location': item['location'],
            'severity': 'critical',
            'suggested_fix': 'Apply functional programming patterns to reduce complexity',
            'original_score': item['score'],
            'current_score': item['score']
        }

    # Check for insufficient improvements
    for improved in improvements['improved_items']:
        if improved['after_score'] >= 8:  # Still critical after improvement
            gap_id = f"insufficient_{improved['location'].replace(':', '_').replace('/', '_')}"
            gaps[gap_id] = {
                'description': 'Function complexity reduced but still above critical threshold',
                'location': improved['location'],
                'severity': 'high',
                'suggested_fix': 'Extract helper functions using pure functional patterns',
                'original_score': improved['before_score'],
                'current_score': improved['after_score'],
                'target_score': 6.0
            }

    # Check for new critical items (regression)
    for new_item in improvements['new_items']:
        if new_item.get('score', 0) >= 8:
            gap_id = f"regression_{new_item['location'].replace(':', '_').replace('/', '_')}"
            gaps[gap_id] = {
                'description': 'New complexity introduced during refactoring',
                'location': new_item['location'],
                'severity': 'critical',
                'suggested_fix': 'Simplify the newly added code or split into smaller functions',
                'original_score': None,
                'current_score': new_item['score']
            }

    return gaps


def generate_validation_result(before_data: Dict, after_data: Dict) -> Dict[str, Any]:
    """Generate complete validation result comparing before and after states."""
    # Extract debt items
    before_items = extract_debt_items(before_data)
    after_items = extract_debt_items(after_data)

    # Calculate metrics
    before_metrics = calculate_metrics(before_items)
    after_metrics = calculate_metrics(after_items)

    # Identify improvements
    improvements = identify_improvements(before_items, after_items)

    # Calculate improvement score
    improvement_score = calculate_improvement_score(before_metrics, after_metrics, improvements)

    # Determine status
    status = 'complete' if improvement_score >= 75 else 'incomplete' if improvement_score >= 40 else 'failed'

    # Build improvement descriptions
    improvement_descriptions = []
    if improvements['resolved_items']:
        resolved_critical = len([item for item in improvements['resolved_items']
                               if item.get('score', 0) >= 8])
        if resolved_critical > 0:
            improvement_descriptions.append(f"Resolved {resolved_critical} critical debt items")

    if improvements['improved_items']:
        improvement_descriptions.append(f"Improved {len(improvements['improved_items'])} functions")

    if after_metrics['average_complexity'] < before_metrics['average_complexity']:
        reduction_pct = ((before_metrics['average_complexity'] - after_metrics['average_complexity']) /
                        before_metrics['average_complexity'] * 100)
        improvement_descriptions.append(f"Reduced average complexity by {reduction_pct:.1f}%")

    if after_metrics['total_score'] < before_metrics['total_score']:
        score_reduction = before_metrics['total_score'] - after_metrics['total_score']
        improvement_descriptions.append(f"Reduced total debt score by {score_reduction:.1f} points")

    # Build remaining issues
    remaining_issues = []
    if improvements['unchanged_critical']:
        remaining_issues.append(f"{len(improvements['unchanged_critical'])} critical debt items still present")

    if improvements['new_items']:
        new_critical = len([item for item in improvements['new_items']
                          if item.get('score', 0) >= 8])
        if new_critical > 0:
            remaining_issues.append(f"{new_critical} new critical issues introduced")

    # Identify gaps if improvement is insufficient
    gaps = identify_gaps(improvements, after_items, improvement_score) if improvement_score < 75 else {}

    return {
        'completion_percentage': round(improvement_score, 1),
        'status': status,
        'improvements': improvement_descriptions,
        'remaining_issues': remaining_issues,
        'gaps': gaps,
        'before_summary': {
            'total_items': before_metrics['total_items'],
            'high_priority_items': before_metrics['high_priority_items'],
            'average_score': round(before_metrics['average_score'], 2)
        },
        'after_summary': {
            'total_items': after_metrics['total_items'],
            'high_priority_items': after_metrics['high_priority_items'],
            'average_score': round(after_metrics['average_score'], 2)
        }
    }


def main():
    """Main validation function."""
    # Check if running in automation mode
    is_automation = os.environ.get('PRODIGY_AUTOMATION') == 'true' or \
                    os.environ.get('PRODIGY_VALIDATION') == 'true'

    # Get arguments from environment or command line
    if len(sys.argv) > 1:
        args_string = ' '.join(sys.argv[1:])
    elif 'ARGUMENTS' in os.environ:
        args_string = os.environ['ARGUMENTS']
    else:
        print("Error: No arguments provided. Use --before, --after, and optionally --output", file=sys.stderr)
        sys.exit(1)

    try:
        before_path, after_path, output_path = parse_arguments(args_string)
    except SystemExit:
        # argparse calls sys.exit on error
        validation_result = {
            'completion_percentage': 0.0,
            'status': 'failed',
            'improvements': [],
            'remaining_issues': ['Failed to parse command arguments'],
            'gaps': {},
            'raw_output': f"Invalid arguments: {args_string}"
        }
        output_path = '.prodigy/debtmap-validation.json'
        os.makedirs(os.path.dirname(output_path), exist_ok=True)
        with open(output_path, 'w') as f:
            json.dump(validation_result, f, indent=2)
        sys.exit(1)

    if not is_automation:
        print(f"Loading before state from: {before_path}")
        print(f"Loading after state from: {after_path}")

    # Load JSON files
    before_data = load_json_file(before_path)
    after_data = load_json_file(after_path)

    if not before_data or not after_data:
        validation_result = {
            'completion_percentage': 0.0,
            'status': 'failed',
            'improvements': [],
            'remaining_issues': ['Unable to load debtmap JSON files'],
            'gaps': {},
            'raw_output': f"Failed to load: before={before_path}, after={after_path}"
        }
    else:
        # Generate validation result
        validation_result = generate_validation_result(before_data, after_data)

        if not is_automation:
            print(f"\nValidation complete:")
            print(f"  Improvement: {validation_result['completion_percentage']:.1f}%")
            print(f"  Status: {validation_result['status']}")

            if validation_result['improvements']:
                print("\nImprovements made:")
                for improvement in validation_result['improvements']:
                    print(f"  ✓ {improvement}")

            if validation_result['remaining_issues']:
                print("\nRemaining issues:")
                for issue in validation_result['remaining_issues']:
                    print(f"  ⚠ {issue}")

    # Write result to output file
    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    with open(output_path, 'w') as f:
        json.dump(validation_result, f, indent=2)

    if not is_automation:
        print(f"\nValidation result written to: {output_path}")

    # Exit with appropriate code
    sys.exit(0)


if __name__ == '__main__':
    main()