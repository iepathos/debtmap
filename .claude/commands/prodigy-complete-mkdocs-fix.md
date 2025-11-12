# /prodigy-complete-mkdocs-fix

Complete MkDocs documentation fixes by addressing specific gaps identified in validation.

This command takes validation gaps and makes targeted improvements to bring the documentation up to quality standards.

## Variables

- `--project <name>` - Project name (e.g., "Prodigy")
- `--json <item>` - JSON object containing page details from flattened-items.json
- `--gaps <json>` - JSON array of gaps from validation report

## Execute

### Phase 1: Parse Parameters and Understand Gaps

**Extract Parameters:**
```bash
PROJECT_NAME="<value from --project parameter>"
ITEM_JSON="<value from --json parameter>"
GAPS_JSON="<value from --gaps parameter>"
```

**Parse Page Details:**
```bash
ITEM_TYPE=$(echo "$ITEM_JSON" | jq -r '.type // "single-file"')
ITEM_ID=$(echo "$ITEM_JSON" | jq -r '.id')
ITEM_FILE=$(echo "$ITEM_JSON" | jq -r '.file')
ITEM_TITLE=$(echo "$ITEM_JSON" | jq -r '.title')

if [ "$ITEM_TYPE" = "section-page" ]; then
  SECTION_ID=$(echo "$ITEM_JSON" | jq -r '.section_id')
  DRIFT_REPORT=".prodigy/mkdocs-analysis/drift-${SECTION_ID}-${ITEM_ID}.json"
else
  DRIFT_REPORT=".prodigy/mkdocs-analysis/drift-${ITEM_ID}.json"
fi
```

**Categorize Gaps by Priority:**
```bash
CRITICAL_GAPS=$(echo "$GAPS_JSON" | jq '[.[] | select(.severity == "critical")]')
HIGH_GAPS=$(echo "$GAPS_JSON" | jq '[.[] | select(.severity == "high")]')
MEDIUM_GAPS=$(echo "$GAPS_JSON" | jq '[.[] | select(.severity == "medium")]')
LOW_GAPS=$(echo "$GAPS_JSON" | jq '[.[] | select(.severity == "low")]')
```

### Phase 2: Read Current Documentation

Read the current file to understand what needs to be added/fixed:

```bash
CURRENT_CONTENT=$(cat "${ITEM_FILE}")
```

### Phase 3: Address Critical Gaps First

**Priority Order:**
1. Invalid file references (remove or fix)
2. Unresolved critical drift issues
3. Missing source references (add from codebase)
4. Unresolved high severity drift issues

### Phase 3.1: Fix Invalid References Gap

**If validation found `invalid_references` gap:**

```bash
# Extract the missing file list from gaps
MISSING_FILES=$(echo "$GAPS_JSON" | jq -r '.[] | select(.category == "invalid_references") | .missing_files[]?')

# For each invalid reference, either:
# 1. Find the correct file path in codebase
# 2. Remove the reference if file truly doesn't exist
```

**Action:** Search for correct paths in codebase:

```bash
for missing_file in $MISSING_FILES; do
  # Extract filename
  FILENAME=$(basename "$missing_file")

  # Search for similar files
  CANDIDATES=$(find . -name "*${FILENAME}*" -type f | head -5)

  if [ -n "$CANDIDATES" ]; then
    echo "Found potential matches for $missing_file:"
    echo "$CANDIDATES"
    # Use the most likely candidate to update the reference
  else
    echo "No match found for $missing_file - removing reference"
    # Remove the invalid reference from the file
  fi
done
```

**Update Documentation:**
- Use the Edit tool to replace invalid references with correct paths
- Or remove references to files that no longer exist

### Phase 3.2: Address Unresolved Critical Drift Issues

**If gaps include unresolved critical issues:**

```bash
# Extract unresolved critical issues from drift report
UNRESOLVED_CRITICAL=$(echo "$GAPS_JSON" | jq -r '.[] | select(.category == "drift_resolution" and .severity == "critical") | .unresolved_issues[]?')
```

**For each unresolved critical issue:**

1. Read the issue details from drift report:
```bash
jq '.issues[] | select(.severity == "critical")' "$DRIFT_REPORT"
```

2. Implement the fix based on issue type:
   - **missing_content**: Add the missing section with complete information
   - **outdated_information**: Update to current implementation
   - **incorrect_examples**: Fix or replace examples

3. Use the Edit tool to make targeted updates

### Phase 3.3: Add Missing Source References

**If validation found `missing_sources` or `source_attribution` gap:**

Re-execute the source discovery from prodigy-fix-mkdocs-drift Phase 3.5:

**Step 1: Discover Codebase Structure**

```bash
# Find test directories
TEST_DIRS=$(find . -type d -name "tests" -o -name "test" | head -5)

# Find example directories
EXAMPLE_DIRS=$(find . -type d -name "examples" -o -name "example" | head -5)

# Find main source directories
SOURCE_DIRS=$(find . -type d -name "src" -o -name "lib" | head -5)

# Find workflow directories
WORKFLOW_DIRS=$(find . -type d -name "workflows" | head -5)
```

**Step 2: Extract Topics from Page**

```bash
TOPICS=$(echo "$ITEM_JSON" | jq -r '.topics[]?')
FEATURE_MAPPINGS=$(jq -r '.feature_mappings[]?' "$DRIFT_REPORT" || echo "")
```

**Step 3: Search for Relevant Code**

For each topic or feature mapping:

```bash
# Search for struct/type definitions
STRUCTS=$(rg "struct.*$(echo $TOPIC | sed 's/[^a-zA-Z]//g')" --type rust -n $SOURCE_DIRS || echo "")

# Search for usage examples in tests
TEST_EXAMPLES=$(rg "$TOPIC" --type rust -A 10 -B 2 $TEST_DIRS || echo "")

# Search for YAML examples in workflows
WORKFLOW_EXAMPLES=$(rg "$TOPIC" --type yaml -A 10 -B 2 $WORKFLOW_DIRS || echo "")
```

**Step 4: Add Source Attribution to Code Blocks**

Scan the documentation file for code blocks without source attribution:

```bash
# Find code blocks without "Source:" comments
CODE_BLOCKS_WITHOUT_SOURCE=$(grep -n '```' "${ITEM_FILE}" | while read line; do
  LINE_NUM=$(echo "$line" | cut -d: -f1)
  # Check if next few lines contain "Source:"
  if ! sed -n "$((LINE_NUM+1)),$((LINE_NUM+3))p" "${ITEM_FILE}" | grep -q "Source:"; then
    echo "$LINE_NUM"
  fi
done)
```

**For each code block without source:**
1. Determine the topic/feature being demonstrated
2. Find matching source file from Step 3 search results
3. Use Edit tool to add source attribution comment after opening ```

**Example fix:**
```markdown
Before:
```yaml
name: example
mode: mapreduce
```

After:
```yaml
# Source: workflows/example-mapreduce.yml
name: example
mode: mapreduce
```
```

### Phase 4: Address High Severity Gaps

**Process each high severity gap:**

**For `drift_resolution` gaps:**
- Review unresolved high severity drift issues from drift report
- Implement fixes for each issue
- Add missing content sections
- Update outdated information
- Fix incorrect examples

**For `content` gaps (insufficient length/structure):**
- Expand brief sections with more detail
- Add more code examples with source attribution
- Include additional subsections
- Add explanatory text

**For `source_validation` gaps:**
- Already addressed in Phase 3.3
- Verify all source references are now valid

### Phase 5: Address Medium and Low Severity Gaps

**For `material_features` gaps:**

Add appropriate MkDocs Material extensions:

**Add Admonitions for Important Information:**
```markdown
!!! note "Configuration Note"
    This setting requires restart to take effect

!!! warning "Common Pitfall"
    Don't forget to set this before that

!!! tip "Best Practice"
    Use this pattern for optimal performance

!!! example "Usage Example"
    Here's how to apply this in practice
```

**Add Titled Code Blocks:**
```markdown
```yaml title="workflow.yml"
# Configuration here
```
```

**Add Content Tabs for Alternatives:**
```markdown
=== "Option A"
    Approach using method A

=== "Option B"
    Alternative approach with method B
```

**For other medium/low gaps:**
- Improve heading structure if needed
- Add more examples if below minimum
- Enhance explanations
- Improve organization

### Phase 6: Enhance Overall Quality

**Beyond Gap Fixes:**

1. **Improve Examples:**
   - Make examples more practical and realistic
   - Show common use cases
   - Include expected output
   - Add edge cases

2. **Add Context:**
   - Explain why features work the way they do
   - Provide motivation for design decisions
   - Link to related concepts

3. **Improve Navigation:**
   - Add clear section headings
   - Use consistent heading hierarchy
   - Add "See Also" links to related pages (when appropriate)

4. **Enhance Readability:**
   - Break up long paragraphs
   - Use bullet points for lists
   - Add emphasis to key terms
   - Improve sentence clarity

### Phase 7: Validate Changes

**Self-Check Before Committing:**

```bash
# Count improvements made
NEW_LINE_COUNT=$(grep -v '^$' "${ITEM_FILE}" | wc -l)
NEW_CODE_BLOCK_COUNT=$(grep -c '```' "${ITEM_FILE}")
NEW_SOURCE_REF_COUNT=$(grep -cE 'Source:|[a-zA-Z0-9_./\-]+\.[a-z]{2,4}:[0-9]+' "${ITEM_FILE}")
NEW_ADMONITION_COUNT=$(grep -cE '^\!\!\!' "${ITEM_FILE}")

echo "Changes made:"
echo "  Lines: $NEW_LINE_COUNT (was $OLD_LINE_COUNT)"
echo "  Code blocks: $NEW_CODE_BLOCK_COUNT"
echo "  Source refs: $NEW_SOURCE_REF_COUNT"
echo "  Admonitions: $NEW_ADMONITION_COUNT"
```

**Verify:**
- All gaps addressed
- Source attributions added
- Examples work correctly
- Links are valid
- MkDocs Material features used
- No new errors introduced

### Phase 8: Commit Changes

**CRITICAL**: Changes must be committed for re-validation.

**Determine Changes Made:**
```bash
GAPS_FIXED=$(echo "$GAPS_JSON" | jq 'length')
```

**Create Commit:**
```bash
git add "$ITEM_FILE"
git commit -m "docs: complete fixes for $PROJECT_NAME page '$ITEM_TITLE'

Addressed $GAPS_FIXED validation gap(s):
$(echo "$GAPS_JSON" | jq -r '.[] | "- [\(.severity)] \(.category): \(.message)"' | head -5)

Page should now meet quality standards."
```

**Verify Commit:**
```bash
git log -1 --stat
```

### Phase 9: Quality Guidelines

**Completeness:**
- Address all critical and high severity gaps
- Make reasonable effort on medium severity gaps
- Improve low severity gaps if time permits
- Don't leave obvious issues unresolved

**Accuracy:**
- All added content must be technically correct
- Examples must work as shown
- Source references must be valid
- Links must resolve correctly

**Clarity:**
- Explanations should be clear and concise
- Examples should be easy to understand
- Organization should be logical
- Language should be accessible

**Consistency:**
- Follow MkDocs Material conventions
- Match existing documentation style
- Use consistent terminology
- Maintain page structure patterns

### Phase 10: Document Limitations

**If Unable to Fully Fix:**

If some gaps cannot be addressed (e.g., missing source files, features not implemented):

1. **Document Known Issues:**
   - Add comment in the file noting the limitation
   - Explain why gap exists
   - Provide workaround if available

2. **Report Back:**
   - List gaps that could not be fixed
   - Explain reasons
   - Suggest next steps

**Example:**
```markdown
!!! warning "Known Limitation"
    This feature is partially documented as implementation is in progress.
    See [issue #123](link) for status updates.
```

### Success Indicators

Completion is successful when:
- All critical gaps addressed
- Most high severity gaps resolved
- Medium severity gaps improved
- Source attributions added throughout
- MkDocs Material features used appropriately
- Changes committed cleanly
- Page quality significantly improved
- Re-validation would likely pass

### Error Handling

**If Phase Fails:**
- Log the failure reason
- Document what was attempted
- Commit partial progress if any
- Report back to workflow with error details

**Common Issues:**
- Unable to find source files → Remove invalid references
- Unable to resolve drift → Add TODO comment and continue
- Unable to add features → Use simpler alternatives
