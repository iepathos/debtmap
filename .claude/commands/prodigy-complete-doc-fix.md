# /prodigy-complete-doc-fix

Complete documentation fixes by addressing specific gaps identified in validation.

This command takes validation gaps and makes targeted improvements to bring the documentation up to quality standards.

## Variables

- `--project <name>` - Project name (e.g., "Prodigy")
- `--json <item>` - JSON object containing chapter or subsection details
- `--gaps <json>` - JSON array of gaps from validation report

## Execute

### Phase 1: Parse Parameters and Understand Gaps

**Extract Parameters:**
```bash
PROJECT_NAME="<value from --project parameter>"
ITEM_JSON="<value from --json parameter>"
GAPS_JSON="<value from --gaps parameter>"
```

**Parse Item Details:**
```bash
ITEM_TYPE=$(echo "$ITEM_JSON" | jq -r '.type // "single-file"')
ITEM_ID=$(echo "$ITEM_JSON" | jq -r '.id')
ITEM_FILE=$(echo "$ITEM_JSON" | jq -r '.file')
ITEM_TITLE=$(echo "$ITEM_JSON" | jq -r '.title')

if [ "$ITEM_TYPE" = "subsection" ]; then
  PARENT_CHAPTER_ID=$(echo "$ITEM_JSON" | jq -r '.parent_chapter_id')
  DRIFT_REPORT=".prodigy/book-analysis/drift-${PARENT_CHAPTER_ID}-${ITEM_ID}.json"
else
  DRIFT_REPORT=".prodigy/book-analysis/drift-${ITEM_ID}.json"
fi
```

**Categorize Gaps by Priority:**
```bash
CRITICAL_GAPS=$(echo "$GAPS_JSON" | jq '[.[] | select(.severity == "critical")]')
HIGH_GAPS=$(echo "$GAPS_JSON" | jq '[.[] | select(.severity == "high")]')
MEDIUM_GAPS=$(echo "$GAPS_JSON" | jq '[.[] | select(.severity == "medium")]')
```

### Phase 2: Read Current Documentation

Read the current file to understand what needs to be added/fixed:

```bash
CURRENT_CONTENT=$(cat "${ITEM_FILE}")
```

### Phase 3: Address Critical Gaps First

**Priority Order:**
1. Invalid file references (remove or fix)
2. Missing source references (add from codebase)
3. Unresolved critical drift issues
4. Unresolved high severity drift issues

**For Invalid References Gap:**

If validation found `invalid_references` gap:

```bash
# Extract the missing file list from gaps
MISSING_FILES=$(echo "$GAPS_JSON" | jq -r '.[] | select(.category == "invalid_references") | .message')

# For each invalid reference, either:
# 1. Find the correct file path in codebase
# 2. Remove the reference if file truly doesn't exist
```

**Action:** Use the Explore agent to find correct paths:
```
Task: Find the correct paths for these referenced files: ${MISSING_FILES}
- Search codebase for similar filenames
- Look for moved or renamed files
- Report if file genuinely doesn't exist
```

Then update the documentation file to fix references.

**For Missing Source References Gap:**

If validation found `missing_sources` gap:

```bash
# Need to add source references to examples
# Re-run the codebase discovery from fix-subsection-drift Phase 3.5
```

**Action:** Re-execute the code discovery steps:
1. Discover codebase structure (test dirs, example dirs, source dirs)
2. Search for type definitions related to documented features
3. Search for usage examples in tests/examples
4. Add source attribution to each code example

**Example fix:**
```markdown
# Before (no source):
\`\`\`yaml
retry_config:
  max_attempts: 3
\`\`\`

# After (with source):
\`\`\`yaml
retry_config:
  max_attempts: 3
\`\`\`

**Source**: Configuration type definition in src/config/retry.rs:45
**Example from**: tests/integration/retry_test.rs:78
```

**For Unresolved Drift Issues:**

If validation found `unresolved_drift` gap with critical/high issues:

```bash
# Load the drift report and identify which issues are still unresolved
UNRESOLVED_ISSUES=$(jq '.issues[] | select(.severity == "critical" or .severity == "high")' ${DRIFT_REPORT})
```

**Action:** For each unresolved issue:
1. Identify the section that needs to be added/expanded
2. Search codebase for relevant information
3. Add missing content with source references

### Phase 4: Address High Priority Gaps

**For Content Length Gap:**

If validation found `content_length` gap (< 80% of minimum):

**Strategy:**
1. **Don't pad with fluff** - only add substantive content
2. **Search for more examples** in codebase
3. **Add more detailed explanations** of existing examples
4. **Add use case sections** with real scenarios
5. **Add troubleshooting** based on error messages in codebase

**Use Explore agent for content discovery:**
```
Task: Find additional content for ${ITEM_TITLE}
- Search for more usage examples in tests/examples
- Find related error messages and troubleshooting hints
- Identify common patterns in codebase
- Look for configuration variations
```

**For Missing Examples Gap:**

If validation found `missing_examples` gap:

**Action:** Use the discovery process from Phase 3.5 of fix-subsection-drift:
1. Discover test and example directories
2. Search for feature usage: `rg "${feature_name}" $TEST_DIRS $EXAMPLE_DIRS`
3. Extract real code examples (5-20 lines each)
4. Add to documentation with proper source attribution

**Template for adding examples:**
```markdown
### Example: ${Use Case Name}

This example demonstrates ${what it shows}.

\`\`\`${language}
${actual code from codebase}
\`\`\`

**Source**: ${file_path}:${line_number}
**Use case**: ${description of when to use this}
```

### Phase 5: Address Medium Priority Gaps

**For Insufficient Headings:**

If validation found insufficient headings:

**Strategy:**
- Review content and identify natural sections
- Add structure with ## and ### headings
- Common sections to add:
  - ## Configuration
  - ## Usage
  - ### Basic Usage
  - ### Advanced Usage
  - ## Examples
  - ## Best Practices
  - ## Common Issues
  - ## Related Features

**For Unresolved Medium Severity Drift:**

Follow the same process as critical/high drift, but these are lower priority.

### Phase 6: Verify Improvements

**Before committing, check:**
1. All code examples have source references
2. All file references are valid
3. Content is substantial (not just filler)
4. Headings create logical structure
5. No hallucinated examples

**Quick validation check:**
```bash
# Count improvements
NEW_LINE_COUNT=$(grep -v '^$' "${ITEM_FILE}" | grep -v '^#\s*$' | wc -l)
NEW_EXAMPLE_COUNT=$(grep -c '```' "${ITEM_FILE}")
NEW_SOURCE_COUNT=$(grep -cE 'Source:|[a-zA-Z0-9_./\-]+\.[a-z]{2,4}:[0-9]+' "${ITEM_FILE}")

echo "Improvements made:"
echo "  Lines: ${NEW_LINE_COUNT} (was: ${OLD_LINE_COUNT})"
echo "  Examples: ${NEW_EXAMPLE_COUNT} (was: ${OLD_EXAMPLE_COUNT})"
echo "  Sources: ${NEW_SOURCE_COUNT} (was: ${OLD_SOURCE_COUNT})"
```

### Phase 7: Commit Improvements

**Create detailed commit message:**

```bash
git add "${ITEM_FILE}"
git commit -m "docs: complete ${PROJECT_NAME} ${ITEM_TYPE} '${ITEM_TITLE}'

Addressed validation gaps from previous fix attempt:
- ${list critical gaps fixed}
- ${list high priority gaps fixed}
- ${list medium priority gaps fixed}

Content improvements:
- Added ${EXAMPLES_ADDED} code examples with source references
- Expanded ${SECTIONS_EXPANDED} sections with real usage
- Fixed ${REFERENCES_FIXED} invalid file references
- Resolved ${DRIFT_ISSUES_FIXED} remaining drift issues

All examples verified against codebase sources."
```

### Phase 8: Output Summary

```
✅ Completed documentation improvements for ${ITEM_TITLE}

Gaps addressed:
${CRITICAL_GAPS_COUNT} critical gaps ✓
${HIGH_GAPS_COUNT} high priority gaps ✓
${MEDIUM_GAPS_COUNT} medium priority gaps ✓

Content metrics after completion:
  Lines: ${NEW_LINE_COUNT}
  Headings: ${NEW_HEADING_COUNT}
  Examples: ${NEW_EXAMPLE_COUNT}
  Source References: ${NEW_SOURCE_COUNT}

The file should now pass validation on next check.
```

## Strategy for Difficult Cases

**If content genuinely doesn't exist in codebase:**

Don't invent it. Instead:

```markdown
## ${Section}

This feature is defined in ${source_file} but has limited examples in the codebase.

### Configuration

See the type definition for available options:
```${language}
// From ${source_file}:${line}
${actual struct/type definition}
```

### Usage

**Note**: This feature appears to have limited usage in the current codebase. If you have examples of using this feature, please consider contributing them to the documentation.

For implementation details, see:
- [Source Code](${source_file}:${line})
- [Type Definition](${type_file}:${line})
```

This is honest documentation - it acknowledges the limitation rather than inventing examples.

**If feature truly has insufficient content for a subsection:**

Create a TODO file and exit:

```bash
cat > ${ITEM_FILE}.TODO <<EOF
# TODO: ${ITEM_TITLE}

After multiple completion attempts, this subsection cannot be populated with sufficient grounded content.

## Validation Gaps
${GAPS_JSON}

## Recommendation
1. Remove this subsection from chapter definition
2. Merge content into parent chapter's index.md
3. Mark feature as planned/minimal in documentation
4. Wait for feature to be more fully implemented

This subsection should not exist until there is sufficient codebase content to document.
EOF

echo "⚠️  Cannot complete ${ITEM_TITLE} - insufficient content in codebase"
echo "Created TODO file: ${ITEM_FILE}.TODO"
exit 1
```

## Completion Guidelines

**DO:**
- Add real examples from codebase
- Expand existing sections with more detail
- Add source references to all code
- Improve structure with headings
- Add practical use cases from tests

**DON'T:**
- Invent plausible-looking examples
- Add filler content to meet line counts
- Copy examples from other projects
- Assume feature behavior without verification
- Create subsections for unimplemented features

## Notes

- This command runs iteratively (up to max_attempts in workflow)
- Each iteration should make measurable progress
- If no progress possible after 2-3 attempts, exit with TODO file
- Validation will run again after this command to verify improvements
