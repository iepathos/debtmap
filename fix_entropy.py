#!/usr/bin/env python3
import os
import re

def fix_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()
    
    # Pattern to find FunctionMetrics struct initialization
    pattern = r'(FunctionMetrics\s*\{[^}]*in_test_module:\s*[^,\n]+,?)(\s*)(\})'
    
    def replacer(match):
        struct_content = match.group(1)
        whitespace = match.group(2)
        closing = match.group(3)
        
        # Check if entropy_score is already present
        if 'entropy_score:' in struct_content:
            return match.group(0)
        
        # Add entropy_score field
        if struct_content.rstrip().endswith(','):
            return struct_content + whitespace + '    entropy_score: None,' + whitespace + closing
        else:
            return struct_content + ',' + whitespace + '    entropy_score: None,' + whitespace + closing
    
    # Apply the replacement
    new_content = re.sub(pattern, replacer, content, flags=re.DOTALL)
    
    if new_content != content:
        with open(filepath, 'w') as f:
            f.write(new_content)
        return True
    return False

# Process all Rust files
for root, dirs, files in os.walk('.'):
    # Skip target and .git directories
    if 'target' in root or '.git' in root:
        continue
    
    for file in files:
        if file.endswith('.rs'):
            filepath = os.path.join(root, file)
            if fix_file(filepath):
                print(f"Fixed: {filepath}")

print("Done!")