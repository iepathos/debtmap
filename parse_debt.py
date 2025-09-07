import json
import sys

# Read the JSON from environment variable
item_json = """${item}"""

# Parse the JSON
item = json.loads(item_json)

# Extract key information
print(f"File: {item['location']['file']}")
print(f"Function: {item['location']['function']}")
print(f"Line: {item['location']['line']}")
print(f"Score: {item['unified_score']['final_score']}")
print(f"Cyclomatic Complexity: {item['cyclomatic_complexity']}")
print(f"Cognitive Complexity: {item['cognitive_complexity']}")
print(f"Nesting Depth: {item['nesting_depth']}")
print(f"Function Length: {item['function_length']}")
print(f"Function Role: {item['function_role']}")
print(f"Primary Action: {item['recommendation']['primary_action']}")
print(f"Entropy Score: {item.get('entropy_details', {}).get('entropy_score', 'N/A')}")
print(f"Pattern Repetition: {item.get('entropy_details', {}).get('pattern_repetition', 'N/A')}")
print(f"Risk Reduction: {item.get('expected_impact', {}).get('risk_reduction', 'N/A')}")
