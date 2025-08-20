#!/bin/bash

# Find all Rust files and add entropy_score field after in_test_module
for file in $(find tests/ src/ -name "*.rs" -type f); do
    # Check if file contains FunctionMetrics struct initialization
    if grep -q "FunctionMetrics {" "$file"; then
        # Create a temporary file
        temp_file="${file}.temp"
        
        # Process the file
        awk '
        /FunctionMetrics {/ { in_struct = 1 }
        in_struct && /in_test_module:/ {
            print
            if ($0 !~ /entropy_score:/) {
                getline next_line
                if (next_line !~ /entropy_score:/) {
                    print "            entropy_score: None,"
                }
                print next_line
                next
            }
        }
        in_struct && /^[[:space:]]*}/ { in_struct = 0 }
        { print }
        ' "$file" > "$temp_file"
        
        # Replace original file
        mv "$temp_file" "$file"
    fi
done

echo "Fixed entropy_score in all files"