#!/usr/bin/env python3
"""
Validates debtmap improvements by comparing before and after states.
"""

import json
import sys
import os
from pathlib import Path
from typing import Dict, List, Any, Tuple, Optional
from dataclasses import dataclass, asdict
import argparse


@dataclass
class DebtItem:
    """Represents a single debt item."""
    location: Dict[str, Any]
    debt_type: Dict[str, Any]
    unified_score: float
    complexity: int
    coverage: float
    function_role: str
    recommendation: Dict[str, Any]

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'DebtItem':
        """Create DebtItem from dictionary."""
        # Extract complexity - try multiple sources
        complexity = data.get('cyclomatic_complexity', 0)
        if complexity == 0 and 'ComplexityHotspot' in data.get('debt_type', {}):
            complexity = data['debt_type']['ComplexityHotspot'].get('cyclomatic', 0)

        # Extract coverage
        coverage = 0.0
        if data.get('transitive_coverage'):
            coverage = data['transitive_coverage'].get('direct', 0.0)

        return cls(
            location=data.get('location', {}),
            debt_type=data.get('debt_type', {}),
            unified_score=data.get('unified_score', {}).get('final_score', 0.0),
            complexity=complexity,
            coverage=coverage,
            function_role=data.get('function_role', ''),
            recommendation=data.get('recommendation', {})
        )

    def get_key(self) -> str:
        """Get unique key for this debt item."""
        loc = self.location
        return f"{loc.get('file', '')}:{loc.get('function', '')}:{loc.get('line', 0)}"


@dataclass
class ValidationResult:
    """Validation result structure."""
    completion_percentage: float
    status: str
    improvements: List[str]
    remaining_issues: List[str]
    gaps: Dict[str, Any]
    before_summary: Dict[str, Any]
    after_summary: Dict[str, Any]


class DebtmapValidator:
    """Validates debtmap improvements."""

    # Thresholds for scoring (based on unified_score from debtmap)
    CRITICAL_SCORE_THRESHOLD = 20.0  # Very high priority items
    HIGH_SCORE_THRESHOLD = 10.0      # High priority items
    MEDIUM_SCORE_THRESHOLD = 5.0     # Medium priority items

    # Weight factors for improvement scoring
    WEIGHT_CRITICAL_RESOLVED = 0.4
    WEIGHT_OVERALL_IMPROVEMENT = 0.3
    WEIGHT_COMPLEXITY_REDUCTION = 0.2
    WEIGHT_NO_REGRESSION = 0.1

    # Completion thresholds
    EXCELLENT_THRESHOLD = 90
    GOOD_THRESHOLD = 75
    MODERATE_THRESHOLD = 60
    MINOR_THRESHOLD = 40

    def __init__(self, before_path: str, after_path: str):
        """Initialize validator with file paths."""
        self.before_path = Path(before_path)
        self.after_path = Path(after_path)
        self.before_data = None
        self.after_data = None
        self.before_items = []
        self.after_items = []

    def load_data(self) -> bool:
        """Load and parse JSON data."""
        try:
            # Load before data
            with open(self.before_path, 'r') as f:
                self.before_data = json.load(f)

            # Load after data
            with open(self.after_path, 'r') as f:
                self.after_data = json.load(f)

            # Parse items
            self.before_items = [
                DebtItem.from_dict(item)
                for item in self.before_data.get('items', [])
            ]
            self.after_items = [
                DebtItem.from_dict(item)
                for item in self.after_data.get('items', [])
            ]

            return True

        except Exception as e:
            print(f"Error loading data: {e}", file=sys.stderr)
            return False

    def categorize_items(self, items: List[DebtItem]) -> Dict[str, List[DebtItem]]:
        """Categorize items by priority."""
        categories = {
            'critical': [],
            'high': [],
            'medium': [],
            'low': []
        }

        for item in items:
            score = item.unified_score
            if score >= self.CRITICAL_SCORE_THRESHOLD:
                categories['critical'].append(item)
            elif score >= self.HIGH_SCORE_THRESHOLD:
                categories['high'].append(item)
            elif score >= self.MEDIUM_SCORE_THRESHOLD:
                categories['medium'].append(item)
            else:
                categories['low'].append(item)

        return categories

    def compare_items(self) -> Tuple[List[DebtItem], List[DebtItem], List[DebtItem], Dict[str, Tuple[DebtItem, DebtItem]]]:
        """Compare before and after items."""
        before_map = {item.get_key(): item for item in self.before_items}
        after_map = {item.get_key(): item for item in self.after_items}

        # Find resolved items (in before but not in after)
        resolved = []
        for key, item in before_map.items():
            if key not in after_map:
                resolved.append(item)

        # Find new items (in after but not in before)
        new_items = []
        for key, item in after_map.items():
            if key not in before_map:
                new_items.append(item)

        # Find improved items (present in both but with better metrics)
        improved = []
        improved_map = {}
        for key in set(before_map.keys()) & set(after_map.keys()):
            before_item = before_map[key]
            after_item = after_map[key]

            # Check if improved
            if (after_item.unified_score < before_item.unified_score or
                after_item.complexity < before_item.complexity or
                after_item.coverage > before_item.coverage):
                improved.append(after_item)
                improved_map[key] = (before_item, after_item)

        return resolved, new_items, improved, improved_map

    def calculate_improvement_score(self, resolved: List[DebtItem],
                                   new_items: List[DebtItem],
                                   improved: List[DebtItem]) -> float:
        """Calculate overall improvement score."""
        before_categories = self.categorize_items(self.before_items)
        after_categories = self.categorize_items(self.after_items)

        # Calculate critical items resolved percentage
        critical_before = len(before_categories['critical'])
        critical_after = len(after_categories['critical'])
        critical_resolved_pct = 0.0
        if critical_before > 0:
            critical_resolved_pct = max(0, (critical_before - critical_after) / critical_before * 100)

        # Calculate overall score improvement
        before_avg_score = sum(item.unified_score for item in self.before_items) / max(1, len(self.before_items))
        after_avg_score = sum(item.unified_score for item in self.after_items) / max(1, len(self.after_items))
        score_improvement_pct = max(0, (before_avg_score - after_avg_score) / max(1, before_avg_score) * 100)

        # Calculate complexity reduction
        before_avg_complexity = sum(item.complexity for item in self.before_items) / max(1, len(self.before_items))
        after_avg_complexity = sum(item.complexity for item in self.after_items) / max(1, len(self.after_items))
        complexity_reduction_pct = max(0, (before_avg_complexity - after_avg_complexity) / max(1, before_avg_complexity) * 100)

        # Check for regression (new critical items)
        new_critical = [item for item in new_items if item.unified_score >= self.CRITICAL_SCORE_THRESHOLD]
        no_regression_score = 100 if len(new_critical) == 0 else 0

        # Calculate weighted score
        improvement_score = (
            self.WEIGHT_CRITICAL_RESOLVED * critical_resolved_pct +
            self.WEIGHT_OVERALL_IMPROVEMENT * score_improvement_pct +
            self.WEIGHT_COMPLEXITY_REDUCTION * complexity_reduction_pct +
            self.WEIGHT_NO_REGRESSION * no_regression_score
        )

        return min(100, improvement_score)

    def identify_gaps(self, resolved: List[DebtItem],
                     new_items: List[DebtItem],
                     improved_map: Dict[str, Tuple[DebtItem, DebtItem]]) -> Dict[str, Any]:
        """Identify improvement gaps."""
        gaps = {}

        # Find unresolved critical items
        after_categories = self.categorize_items(self.after_items)
        critical_remaining = after_categories['critical']

        if critical_remaining:
            for i, item in enumerate(critical_remaining[:3]):  # Top 3 critical items
                gaps[f"critical_debt_remaining_{i+1}"] = {
                    "description": f"High-priority debt item still present in {item.location.get('function', 'unknown')}",
                    "location": f"{item.location.get('file', '')}:{item.location.get('function', '')}:{item.location.get('line', 0)}",
                    "severity": "critical",
                    "suggested_fix": item.recommendation.get('primary_action', 'Apply functional programming patterns to reduce complexity'),
                    "current_score": item.unified_score
                }

        # Check for insufficient refactoring
        for key, (before_item, after_item) in improved_map.items():
            if after_item.complexity > 8:  # Still too complex
                gaps[f"insufficient_refactoring_{len(gaps)+1}"] = {
                    "description": "Function complexity reduced but still above threshold",
                    "location": f"{after_item.location.get('file', '')}:{after_item.location.get('function', '')}:{after_item.location.get('line', 0)}",
                    "severity": "medium",
                    "suggested_fix": "Extract helper functions using pure functional patterns",
                    "original_complexity": before_item.complexity,
                    "current_complexity": after_item.complexity,
                    "target_complexity": 8
                }

        # Check for regression
        new_critical = [item for item in new_items if item.unified_score >= self.CRITICAL_SCORE_THRESHOLD]
        if new_critical:
            for i, item in enumerate(new_critical[:2]):  # Top 2 new critical items
                gaps[f"regression_detected_{i+1}"] = {
                    "description": "New complexity introduced during refactoring",
                    "location": f"{item.location.get('file', '')}:{item.location.get('function', '')}:{item.location.get('line', 0)}",
                    "severity": "high",
                    "suggested_fix": "Simplify the newly added conditional logic",
                    "current_score": item.unified_score
                }

        return gaps

    def generate_summary(self, items: List[DebtItem]) -> Dict[str, Any]:
        """Generate summary statistics."""
        categories = self.categorize_items(items)

        return {
            "total_items": len(items),
            "critical_items": len(categories['critical']),
            "high_priority_items": len(categories['high']),
            "average_score": sum(item.unified_score for item in items) / max(1, len(items)),
            "average_complexity": sum(item.complexity for item in items) / max(1, len(items)),
            "average_coverage": sum(item.coverage for item in items) / max(1, len(items))
        }

    def validate(self) -> ValidationResult:
        """Perform validation and return results."""
        if not self.load_data():
            return ValidationResult(
                completion_percentage=0.0,
                status="failed",
                improvements=[],
                remaining_issues=["Unable to load or parse debtmap JSON files"],
                gaps={},
                before_summary={},
                after_summary={}
            )

        # Compare items
        resolved, new_items, improved, improved_map = self.compare_items()

        # Calculate improvement score
        improvement_score = self.calculate_improvement_score(resolved, new_items, improved)

        # Determine status
        if improvement_score >= self.GOOD_THRESHOLD:
            status = "complete"
        elif improvement_score >= self.MINOR_THRESHOLD:
            status = "incomplete"
        else:
            status = "insufficient"

        # Build improvements list
        improvements = []
        if resolved:
            improvements.append(f"Resolved {len(resolved)} debt items")
        if improved:
            improvements.append(f"Improved {len(improved)} debt items")

        before_summary = self.generate_summary(self.before_items)
        after_summary = self.generate_summary(self.after_items)

        if after_summary['average_score'] < before_summary['average_score']:
            reduction_pct = (before_summary['average_score'] - after_summary['average_score']) / before_summary['average_score'] * 100
            improvements.append(f"Reduced average debt score by {reduction_pct:.1f}%")

        if after_summary['average_complexity'] < before_summary['average_complexity']:
            reduction_pct = (before_summary['average_complexity'] - after_summary['average_complexity']) / before_summary['average_complexity'] * 100
            improvements.append(f"Reduced average complexity by {reduction_pct:.1f}%")

        # Build remaining issues list
        remaining_issues = []
        after_categories = self.categorize_items(self.after_items)
        if after_categories['critical']:
            remaining_issues.append(f"{len(after_categories['critical'])} critical debt items still present")
        if new_items:
            remaining_issues.append(f"{len(new_items)} new debt items introduced")

        # Identify gaps if incomplete
        gaps = {}
        if status != "complete":
            gaps = self.identify_gaps(resolved, new_items, improved_map)

        return ValidationResult(
            completion_percentage=round(improvement_score, 1),
            status=status,
            improvements=improvements,
            remaining_issues=remaining_issues,
            gaps=gaps,
            before_summary=before_summary,
            after_summary=after_summary
        )


def main():
    """Main entry point."""
    # Check for automation mode
    is_automation = os.environ.get('PRODIGY_AUTOMATION') == 'true' or \
                    os.environ.get('PRODIGY_VALIDATION') == 'true'

    # Parse arguments
    parser = argparse.ArgumentParser(description='Validate debtmap improvements')
    parser.add_argument('--before', required=True, help='Path to before debtmap JSON')
    parser.add_argument('--after', required=True, help='Path to after debtmap JSON')
    parser.add_argument('--output', default='.prodigy/debtmap-validation.json',
                       help='Output file path (default: .prodigy/debtmap-validation.json)')

    # Parse from command line arguments
    args_str = ' '.join(sys.argv[1:]) if len(sys.argv) > 1 else os.environ.get('ARGUMENTS', '')

    # Manual parsing for flexibility
    args_dict = {
        'before': None,
        'after': None,
        'output': '.prodigy/debtmap-validation.json'
    }

    parts = args_str.split()
    i = 0
    while i < len(parts):
        if parts[i] == '--before' and i + 1 < len(parts):
            args_dict['before'] = parts[i + 1]
            i += 2
        elif parts[i] == '--after' and i + 1 < len(parts):
            args_dict['after'] = parts[i + 1]
            i += 2
        elif parts[i] == '--output' and i + 1 < len(parts):
            args_dict['output'] = parts[i + 1]
            i += 2
        else:
            i += 1

    # Validate required arguments
    if not args_dict['before'] or not args_dict['after']:
        print("Error: Missing required arguments --before and --after", file=sys.stderr)
        sys.exit(1)

    if not is_automation:
        print(f"Validating debtmap improvements...")
        print(f"  Before: {args_dict['before']}")
        print(f"  After: {args_dict['after']}")
        print(f"  Output: {args_dict['output']}")

    # Create validator and run validation
    validator = DebtmapValidator(args_dict['before'], args_dict['after'])
    result = validator.validate()

    # Prepare output
    output_data = {
        "completion_percentage": result.completion_percentage,
        "status": result.status,
        "improvements": result.improvements,
        "remaining_issues": result.remaining_issues,
        "gaps": result.gaps,
        "before_summary": result.before_summary,
        "after_summary": result.after_summary
    }

    # Write to output file
    output_path = Path(args_dict['output'])
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, 'w') as f:
        json.dump(output_data, f, indent=2)

    if not is_automation:
        print(f"\nValidation complete!")
        print(f"  Completion: {result.completion_percentage}%")
        print(f"  Status: {result.status}")
        print(f"  Result written to: {output_path}")

    # Exit based on status
    sys.exit(0)


if __name__ == '__main__':
    main()