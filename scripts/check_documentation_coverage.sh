#!/bin/bash

# Documentation coverage checker for Rust project
# Target: 80% documentation coverage

echo "Checking documentation coverage..."

# Count total public items and documented items
total_public=$(rg "^pub\s+(fn|struct|enum|trait|type|const|static|mod)" --type rust -c | awk -F: '{sum+=$2} END {print sum}')
documented=$(rg "^(\s*)///.*\n\1pub\s+(fn|struct|enum|trait|type|const|static|mod)" --type rust -c -U | awk -F: '{sum+=$2} END {print sum}')

if [ -z "$total_public" ] || [ "$total_public" -eq 0 ]; then
    echo "No public items found"
    exit 1
fi

# Calculate percentage
coverage=$((documented * 100 / total_public))

echo "Documentation Coverage: $coverage% ($documented/$total_public items documented)"
echo "Target: 80%"

if [ "$coverage" -lt 80 ]; then
    echo "⚠️  Documentation coverage is below target (80%)"
    exit 1
else
    echo "✅ Documentation coverage meets target"
    exit 0
fi