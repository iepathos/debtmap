#!/usr/bin/env bash
set -euo pipefail

# Parse command arguments
BEFORE_FILE=""
AFTER_FILE=""
OUTPUT_FILE=".prodigy/debtmap-validation.json"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --before)
            BEFORE_FILE="$2"
            shift 2
            ;;
        --after)
            AFTER_FILE="$2"
            shift 2
            ;;
        --output)
            OUTPUT_FILE="$2"
            shift 2
            ;;
        *)
            echo "Unknown argument: $1"
            exit 1
            ;;
    esac
done

if [[ -z "$BEFORE_FILE" || -z "$AFTER_FILE" ]]; then
    echo "Error: Missing required arguments"
    echo "Usage: $0 --before <file> --after <file> [--output <file>]"
    exit 1
fi

# Check if in automation mode
AUTOMATION_MODE=false
if [[ "${PRODIGY_AUTOMATION:-}" == "true" ]] || [[ "${PRODIGY_VALIDATION:-}" == "true" ]]; then
    AUTOMATION_MODE=true
fi

# Minimal logging in automation mode
log_progress() {
    if [[ "$AUTOMATION_MODE" != "true" ]]; then
        echo "$1"
    fi
}

log_progress "Loading debtmap data from $BEFORE_FILE and $AFTER_FILE..."

# Check if files exist
if [[ ! -f "$BEFORE_FILE" ]]; then
    ERROR_JSON='{
  "completion_percentage": 0.0,
  "status": "failed",
  "improvements": [],
  "remaining_issues": ["Before file not found: '"$BEFORE_FILE"'"],
  "gaps": {},
  "raw_output": "File not found"
}'
    mkdir -p "$(dirname "$OUTPUT_FILE")"
    echo "$ERROR_JSON" > "$OUTPUT_FILE"
    echo "Error: Before file not found: $BEFORE_FILE"
    exit 0
fi

if [[ ! -f "$AFTER_FILE" ]]; then
    ERROR_JSON='{
  "completion_percentage": 0.0,
  "status": "failed",
  "improvements": [],
  "remaining_issues": ["After file not found: '"$AFTER_FILE"'"],
  "gaps": {},
  "raw_output": "File not found"
}'
    mkdir -p "$(dirname "$OUTPUT_FILE")"
    echo "$ERROR_JSON" > "$OUTPUT_FILE"
    echo "Error: After file not found: $AFTER_FILE"
    exit 0
fi

# Create temporary Python script for complex JSON processing
cat << 'EOF' > /tmp/validate_debtmap.py
#!/usr/bin/env python3
import json
import sys
import os
from pathlib import Path

def load_json_file(filepath):
    """Load and parse JSON file."""
    try:
        with open(filepath, 'r') as f:
            return json.load(f)
    except Exception as e:
        return None

def extract_debt_items(data):
    """Extract debt items from debtmap output."""
    items = []
    if isinstance(data, dict):
        # Handle different possible structures
        if 'debt_items' in data:
            items = data['debt_items']
        elif 'items' in data:
            items = data['items']
        elif 'files' in data:
            # Extract from file-based structure
            for file_data in data.get('files', {}).values():
                if isinstance(file_data, dict):
                    items.extend(file_data.get('debt_items', []))
        elif 'results' in data:
            # Extract from results structure
            for result in data.get('results', []):
                if isinstance(result, dict) and 'debt_items' in result:
                    items.extend(result['debt_items'])

    # Normalize items
    normalized = []
    for item in items:
        if isinstance(item, dict):
            normalized.append({
                'location': item.get('location', item.get('file', '')),
                'function': item.get('function', item.get('name', '')),
                'score': float(item.get('score', item.get('severity', item.get('priority', 5)))),
                'complexity': item.get('complexity', item.get('cyclomatic_complexity', 0)),
                'coverage': item.get('coverage', item.get('test_coverage', 0)),
                'description': item.get('description', item.get('reason', '')),
                'type': item.get('type', item.get('debt_type', 'unknown'))
            })
    return normalized

def calculate_metrics(items):
    """Calculate summary metrics from debt items."""
    if not items:
        return {
            'total_items': 0,
            'high_priority_items': 0,
            'critical_items': 0,
            'average_score': 0,
            'average_complexity': 0,
            'low_coverage_count': 0
        }

    high_priority = [i for i in items if i['score'] >= 6]
    critical = [i for i in items if i['score'] >= 8]
    low_coverage = [i for i in items if i['coverage'] < 50]

    return {
        'total_items': len(items),
        'high_priority_items': len(high_priority),
        'critical_items': len(critical),
        'average_score': sum(i['score'] for i in items) / len(items) if items else 0,
        'average_complexity': sum(i['complexity'] for i in items) / len(items) if items else 0,
        'low_coverage_count': len(low_coverage)
    }

def identify_improvements(before_items, after_items):
    """Identify what improved between before and after."""
    improvements = []
    gaps = {}

    # Create lookup maps
    before_map = {(i['location'], i['function']): i for i in before_items}
    after_map = {(i['location'], i['function']): i for i in after_items}

    # Find resolved items
    resolved = []
    for key, item in before_map.items():
        if key not in after_map:
            resolved.append(item)
            if item['score'] >= 8:
                improvements.append(f"Resolved critical debt item: {item['function']} (score {item['score']:.1f})")
            elif item['score'] >= 6:
                improvements.append(f"Resolved high-priority item: {item['function']}")

    # Find improved items
    improved = []
    for key, after_item in after_map.items():
        if key in before_map:
            before_item = before_map[key]
            if after_item['score'] < before_item['score']:
                improved.append(after_item)
                if before_item['score'] >= 8:
                    improvements.append(f"Improved critical item: {after_item['function']} ({before_item['score']:.1f} → {after_item['score']:.1f})")
            elif after_item['complexity'] < before_item['complexity'] - 2:
                improvements.append(f"Reduced complexity in {after_item['function']} ({before_item['complexity']} → {after_item['complexity']})")
            elif after_item['coverage'] > before_item['coverage'] + 20:
                improvements.append(f"Improved test coverage for {after_item['function']} ({before_item['coverage']}% → {after_item['coverage']}%)")

    # Find remaining critical issues
    remaining_critical = []
    for key, item in after_map.items():
        if item['score'] >= 8:
            remaining_critical.append(item)
            gap_key = f"critical_{item['function'].replace(' ', '_')}"
            original = before_map.get(key, {})
            gaps[gap_key] = {
                'description': f"Critical debt item still present: {item['description']}",
                'location': f"{item['location']}:{item['function']}",
                'severity': 'critical',
                'suggested_fix': 'Apply functional programming patterns to reduce complexity',
                'original_score': original.get('score', item['score']),
                'current_score': item['score']
            }

    # Find new issues
    new_issues = []
    for key, item in after_map.items():
        if key not in before_map and item['score'] >= 6:
            new_issues.append(item)
            gaps[f"new_{item['function'].replace(' ', '_')}"] = {
                'description': f"New debt introduced: {item['description']}",
                'location': f"{item['location']}:{item['function']}",
                'severity': 'high' if item['score'] >= 8 else 'medium',
                'suggested_fix': 'Review recent changes and apply proper refactoring',
                'original_score': None,
                'current_score': item['score']
            }

    return improvements, gaps, resolved, improved, remaining_critical, new_issues

def calculate_improvement_score(before_metrics, after_metrics, resolved, improved, remaining_critical, new_issues):
    """Calculate overall improvement percentage."""
    score = 0.0

    # Component 1: Resolved high-priority items (40%)
    if before_metrics['critical_items'] > 0:
        critical_resolved = before_metrics['critical_items'] - after_metrics['critical_items']
        resolved_ratio = max(0, critical_resolved) / before_metrics['critical_items']
        score += resolved_ratio * 40
    elif before_metrics['high_priority_items'] > 0:
        high_resolved = before_metrics['high_priority_items'] - after_metrics['high_priority_items']
        resolved_ratio = max(0, high_resolved) / before_metrics['high_priority_items']
        score += resolved_ratio * 40
    else:
        # No high priority items to resolve
        score += 40

    # Component 2: Overall score improvement (30%)
    if before_metrics['average_score'] > 0:
        score_improvement = (before_metrics['average_score'] - after_metrics['average_score']) / before_metrics['average_score']
        score += max(0, min(1, score_improvement)) * 30
    else:
        score += 30

    # Component 3: Complexity reduction (20%)
    if before_metrics['average_complexity'] > 5:
        complexity_reduction = (before_metrics['average_complexity'] - after_metrics['average_complexity']) / before_metrics['average_complexity']
        score += max(0, min(1, complexity_reduction)) * 20
    else:
        score += 20

    # Component 4: No new critical debt (10%)
    new_critical = [i for i in new_issues if i['score'] >= 8]
    if len(new_critical) == 0:
        score += 10

    # Apply penalties
    if len(remaining_critical) > 0:
        # Each remaining critical item reduces score
        penalty = min(20, len(remaining_critical) * 5)
        score = max(0, score - penalty)

    if len(new_critical) > 0:
        # New critical items are a major penalty
        penalty = min(25, len(new_critical) * 10)
        score = max(0, score - penalty)

    return min(100, max(0, score))

def main():
    before_file = sys.argv[1]
    after_file = sys.argv[2]
    output_file = sys.argv[3]

    # Load data
    before_data = load_json_file(before_file)
    after_data = load_json_file(after_file)

    if before_data is None:
        result = {
            'completion_percentage': 0.0,
            'status': 'failed',
            'improvements': [],
            'remaining_issues': [f'Failed to parse before file: {before_file}'],
            'gaps': {},
            'raw_output': 'JSON parse error'
        }
    elif after_data is None:
        result = {
            'completion_percentage': 0.0,
            'status': 'failed',
            'improvements': [],
            'remaining_issues': [f'Failed to parse after file: {after_file}'],
            'gaps': {},
            'raw_output': 'JSON parse error'
        }
    else:
        # Extract debt items
        before_items = extract_debt_items(before_data)
        after_items = extract_debt_items(after_data)

        # Calculate metrics
        before_metrics = calculate_metrics(before_items)
        after_metrics = calculate_metrics(after_items)

        # Identify improvements
        improvements, gaps, resolved, improved, remaining_critical, new_issues = identify_improvements(
            before_items, after_items
        )

        # Calculate score
        improvement_score = calculate_improvement_score(
            before_metrics, after_metrics,
            resolved, improved, remaining_critical, new_issues
        )

        # Determine status
        if improvement_score >= 75:
            status = 'complete'
        elif improvement_score >= 40:
            status = 'incomplete'
        else:
            status = 'insufficient'

        # Build remaining issues list
        remaining_issues = []
        if len(remaining_critical) > 0:
            remaining_issues.append(f"{len(remaining_critical)} critical debt items still present")
        if len(new_issues) > 0:
            remaining_issues.append(f"{len(new_issues)} new debt items introduced")
        if after_metrics['average_complexity'] > 10:
            remaining_issues.append(f"Average complexity still high ({after_metrics['average_complexity']:.1f})")
        if after_metrics['low_coverage_count'] > before_metrics['low_coverage_count']:
            remaining_issues.append("Test coverage decreased in some functions")

        # Add default improvements if none found
        if not improvements and before_metrics['total_items'] > after_metrics['total_items']:
            improvements.append(f"Reduced total debt items from {before_metrics['total_items']} to {after_metrics['total_items']}")
        if not improvements and after_metrics['average_score'] < before_metrics['average_score']:
            improvements.append(f"Reduced average debt score from {before_metrics['average_score']:.1f} to {after_metrics['average_score']:.1f}")

        result = {
            'completion_percentage': round(improvement_score, 1),
            'status': status,
            'improvements': improvements[:10],  # Limit to top 10
            'remaining_issues': remaining_issues[:10],
            'gaps': gaps,
            'before_summary': {
                'total_items': before_metrics['total_items'],
                'high_priority_items': before_metrics['high_priority_items'],
                'average_score': round(before_metrics['average_score'], 1)
            },
            'after_summary': {
                'total_items': after_metrics['total_items'],
                'high_priority_items': after_metrics['high_priority_items'],
                'average_score': round(after_metrics['average_score'], 1)
            }
        }

    # Write output
    os.makedirs(os.path.dirname(output_file), exist_ok=True)
    with open(output_file, 'w') as f:
        json.dump(result, f, indent=2)

    print(f"Validation complete. Results written to {output_file}")
    if result['status'] != 'failed':
        print(f"Improvement score: {result['completion_percentage']:.1f}%")

if __name__ == '__main__':
    main()
EOF

# Run the Python script
python3 /tmp/validate_debtmap.py "$BEFORE_FILE" "$AFTER_FILE" "$OUTPUT_FILE"

# Clean up
rm -f /tmp/validate_debtmap.py

log_progress "Validation complete. Results written to $OUTPUT_FILE"