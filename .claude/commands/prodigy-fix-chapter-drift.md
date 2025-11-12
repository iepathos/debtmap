# /prodigy-fix-chapter-drift

Fix documentation drift for a specific book chapter based on its drift analysis report.

## Variables

- `--project <name>` - Project name (e.g., "Prodigy", "Debtmap")
- `--chapter-id <id>` - Chapter ID (e.g., "cli-reference", "examples")

## Execute

### Phase 1: Understand Context

You are fixing documentation drift for a single chapter. The map phase has already analyzed the chapter and created a drift report. Your job is to:
1. Read the drift report
2. Fix all identified issues
3. Update the chapter file
4. Commit the changes

This command runs in a map agent's worktree, so commits will be merged later.

### Phase 2: Parse Input and Load Drift Report

**Extract Parameters:**
```bash
PROJECT_NAME="<value from --project parameter>"
CHAPTER_ID="<value from --chapter-id parameter>"
```

**Determine Paths:**

- Pattern: `.prodigy/book-analysis/drift-${CHAPTER_ID}.json`

**Load Drift Report:**
Read the drift report JSON file to get:
- `chapter_file`: Path to the markdown file to update
- `issues[]`: List of all drift issues with fix suggestions
- `severity`: Overall drift severity
- `improvement_suggestions[]`: Additional recommendations

### Phase 3: Analyze Drift Issues

**Parse the Issues:**
For each issue in the drift report:
- Identify the section that needs updating
- Understand what content is missing/outdated/incorrect
- Review the `fix_suggestion` and `source_reference`
- Check `current_content` vs `should_be` if provided

**Prioritize Fixes:**
1. **Critical severity** - Missing entire sections, completely outdated
2. **High severity** - Major features undocumented, incorrect examples
3. **Medium severity** - Incomplete explanations, minor inaccuracies
4. **Low severity** - Style issues, missing cross-references

### Phase 4: Fix the Chapter

**Read Current Chapter:**
Read the markdown file at `chapter_file` path.

**Apply Fixes:**
For each issue, update the chapter:

1. **Missing Content Issues:**
   - Add the missing section/content
   - Follow the `fix_suggestion` guidance
   - Include code examples where helpful
   - Add cross-references to related chapters

2. **Outdated Information Issues:**
   - Update the outdated content
   - Replace old syntax with current syntax
   - Update examples to match current implementation
   - Add version notes if appropriate

3. **Incorrect Examples Issues:**
   - Fix the broken examples
   - Verify syntax is correct
   - Test examples work with current code
   - Add explanatory comments

4. **Incomplete Explanation Issues:**
   - Expand the brief explanations
   - Add practical examples
   - Include use cases
   - Link to relevant source code

**Preserve Good Content:**
- Keep content mentioned in `positive_aspects`
- Maintain the chapter's structure and flow
- Preserve working examples
- Keep helpful diagrams and explanations

**Apply Improvement Suggestions:**
- Add cross-references
- Include best practices
- Add troubleshooting tips
- Improve organization if needed

### Phase 5: Quality Checks

**Verify Completeness:**
- All critical and high severity issues addressed
- All topics from chapter metadata covered
- Examples are practical and current
- Cross-references are valid

**Verify Accuracy:**
- Check against source code references
- Verify field names and types
- Test that examples parse correctly
- Ensure CLI commands match current syntax

**Verify Clarity:**
- Explanations are clear and concise
- Examples are well-commented
- Structure flows logically
- Technical terms are defined

### Phase 6: Commit the Fix

**Write the Updated Chapter:**
Use the Edit tool to update the chapter file with all fixes applied.

**Create Descriptive Commit:**
```bash
# Count issues fixed
CRITICAL_COUNT=<count of critical issues>
HIGH_COUNT=<count of high issues>
TOTAL_ISSUES=<total issues fixed>

# Get chapter title
CHAPTER_TITLE="<from drift report>"

git add <chapter_file>
git commit -m "docs: fix ${PROJECT_NAME} book chapter '${CHAPTER_TITLE}'

Fixed ${TOTAL_ISSUES} drift issues (${CRITICAL_COUNT} critical, ${HIGH_COUNT} high)

Key updates:
- <list 3-5 most important fixes>

All examples verified against current implementation."
```

### Phase 7: Validation

**The fix should:**
1. Address all critical and high severity issues
2. Update outdated information to match current code
3. Fix all broken examples
4. Add missing content for major features
5. Preserve positive aspects from drift report
6. Include clear, tested examples
7. Be committed with descriptive message
8. Use project name from `--project` parameter

**Don't:**
- Skip critical issues due to complexity
- Add speculative content not in codebase
- Break existing working content
- Remove helpful examples or explanations
- Make unrelated changes

### Phase 8: Summary Output

After committing, provide a brief summary:
```
✅ Fixed drift in ${CHAPTER_TITLE}

Issues addressed:
- ${CRITICAL_COUNT} critical
- ${HIGH_COUNT} high
- ${MEDIUM_COUNT} medium
- ${LOW_COUNT} low

Changes:
- <brief summary of major updates>

Chapter updated: ${CHAPTER_FILE}
```

## Notes

- This command runs during the **map phase** in a separate worktree
- Each map agent fixes one chapter independently
- Commits will be merged to parent worktree automatically
- Focus on accuracy - verify against source code
- Include practical, copy-paste ready examples
- Cross-reference related chapters
- The reduce phase will handle any merge conflicts
