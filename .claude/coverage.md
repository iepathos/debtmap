#!/usr/bin/env bash
# Claude command: Automatically improve code coverage
# This command runs coverage analysis, identifies files with lowest coverage,
# implements tests for uncovered code, and commits the improvements

set -euo pipefail

echo "🔍 Running coverage analysis..."
just coverage

echo "📊 Analyzing coverage results..."
# Extract files with lowest coverage from JSON report
LOWEST_COVERAGE_FILE=$(cat target/coverage/tarpaulin-report.json | jq -r '
  .files 
  | to_entries 
  | map({
      file: .value.path | join("/") | sub("^//"; ""),
      coverage: (if .value.coverable > 0 then (.value.covered * 100 / .value.coverable) else 0 end),
      covered: .value.covered,
      coverable: .value.coverable,
      uncovered_lines: [
        .value.traces[] 
        | select(.stats.Line == 0) 
        | .line
      ]
    })
  | map(select(.coverable > 0))
  | sort_by(.coverage)
  | first
')

FILE_PATH=$(echo "$LOWEST_COVERAGE_FILE" | jq -r '.file')
COVERAGE=$(echo "$LOWEST_COVERAGE_FILE" | jq -r '.coverage')
COVERED=$(echo "$LOWEST_COVERAGE_FILE" | jq -r '.covered')
COVERABLE=$(echo "$LOWEST_COVERAGE_FILE" | jq -r '.coverable')
UNCOVERED_LINES=$(echo "$LOWEST_COVERAGE_FILE" | jq -r '.uncovered_lines | @json')

echo "📁 Targeting file: $FILE_PATH"
echo "   Current coverage: ${COVERAGE}% (${COVERED}/${COVERABLE} lines)"

# Use Claude to analyze and improve coverage
echo "🤖 Analyzing uncovered code and generating tests..."
cat <<EOF | claude --no-confirm

You are tasked with improving test coverage for a Rust project. 

Target file: $FILE_PATH
Current coverage: ${COVERAGE}% (${COVERED}/${COVERABLE} lines)
Uncovered lines: $UNCOVERED_LINES

Please:
1. Read and analyze the file at $FILE_PATH
2. Identify the uncovered functionality based on the line numbers
3. Write idiomatic Rust tests that cover the most important uncovered code
4. Prefer functional programming approaches where reasonable:
   - Use pure functions when possible
   - Favor immutability
   - Use map/filter/fold over loops
   - Minimize side effects
5. Ensure tests follow the project's existing patterns
6. Add the tests to the appropriate test module or file
7. Run 'cargo test' to verify the tests pass
8. Run 'cargo clippy -- -D warnings' to ensure code quality
9. Verify coverage improved with 'just coverage'

Focus on testing the most critical functionality first. Aim to add meaningful tests, not just coverage for coverage's sake.

EOF

echo "✅ Coverage improvement complete!"
echo "📈 Running final coverage check..."
just coverage

# Extract new coverage percentage
NEW_COVERAGE=$(cat target/coverage/tarpaulin-report.json | jq -r '
  .files 
  | to_entries 
  | map({
      file: .value.path | join("/") | sub("^//"; ""),
      coverage: (if .value.coverable > 0 then (.value.covered * 100 / .value.coverable) else 100 end)
    })
  | map(select(.file == "'$FILE_PATH'"))
  | first
  | .coverage
')

echo "📊 Coverage for $FILE_PATH improved from ${COVERAGE}% to ${NEW_COVERAGE}%"

# Commit the changes
echo "💾 Committing coverage improvements..."
git add -A
git commit -m "test: improve coverage for $(basename $FILE_PATH)

- Added tests for uncovered functionality
- Coverage improved from ${COVERAGE}% to ${NEW_COVERAGE}%
- Focus on critical paths and edge cases

🤖 Generated with Claude Code (https://claude.ai/code)

Co-Authored-By: Claude <noreply@anthropic.com>"

echo "🎉 Coverage improvement committed successfully!"