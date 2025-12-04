#!/bin/bash
# Analyze all chapter files for size and structure

for chapter_file in "$@"; do
    if [[ ! -f "$chapter_file" ]]; then
        echo "MISSING:$chapter_file"
        continue
    fi

    # Count total lines
    total_lines=$(wc -l < "$chapter_file")

    # Count content lines (non-empty)
    content_lines=$(grep -c -v '^\s*$' "$chapter_file" || echo 0)

    # Count H2 sections
    h2_count=$(grep -c '^## ' "$chapter_file" || echo 0)

    # Count H1 sections
    h1_count=$(grep -c '^# ' "$chapter_file" || echo 0)

    # Count H3 sections
    h3_count=$(grep -c '^### ' "$chapter_file" || echo 0)

    # Count H4 sections
    h4_count=$(grep -c '^#### ' "$chapter_file" || echo 0)

    # Count code block lines (lines between triple backticks)
    code_lines=$(awk '/^```/{flag=!flag; next} flag' "$chapter_file" | wc -l || echo 0)

    echo "$chapter_file|$total_lines|$content_lines|$h2_count|$h1_count|$h3_count|$h4_count|$code_lines"
done
