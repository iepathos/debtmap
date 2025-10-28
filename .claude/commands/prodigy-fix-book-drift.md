# /prodigy-fix-book-drift

Update book chapters to fix all detected drift issues and ensure documentation matches the current codebase implementation.

## Variables

- `--project <name>` - Project name (e.g., "Prodigy", "Debtmap")
- `--config <path>` - Path to book configuration JSON (e.g., ".prodigy/book-config.json")

## Execute

### Context

You have completed drift analysis across multiple book chapters. Now aggregate the results, update chapters to fix identified issues, and ensure the book builds successfully.

**Parse Parameters:**
- `--project`: The project name (used in output messages and file paths)
- `--config`: Path to the book configuration JSON file

**Load Configuration:**
Read the configuration file specified by `--config` to get:
- `book_dir`: Book root directory
- `book_src`: Book source directory
- Project-specific paths and settings

### Phase 1: Aggregate Drift Reports

**Determine Analysis Directory:**

- Pattern: `.prodigy/book-analysis/`

Collect all drift reports from map phase:

```bash
ANALYSIS_DIR=".{project_lowercase}/book-analysis"

# Aggregate all chapter drift reports
jq -s '{
  total_chapters: length,
  chapters_with_drift: [.[] | select(.drift_detected == true)] | length,
  total_issues: [.[].issues[]] | length,
  severity_breakdown: (group_by(.severity) | map({(.[0].severity): length}) | add),
  all_reports: .
}' $ANALYSIS_DIR/drift-*.json > $ANALYSIS_DIR/drift-summary.json
```

Check if any drift was detected:
```bash
drift_count=$(jq -r '.chapters_with_drift' $ANALYSIS_DIR/drift-summary.json)
if [ "$drift_count" -eq 0 ]; then
  echo "✓ No book drift detected for $PROJECT_NAME - documentation is up to date!"
  exit 0
else
  echo "⚠ Found drift in $drift_count $PROJECT_NAME chapters"
  jq -r '.all_reports[] | select(.drift_detected == true) | "  - \(.chapter_title): \(.severity) severity, \(.issues | length) issues"' \
    $ANALYSIS_DIR/drift-summary.json
fi
```

### Phase 2: Identify Available Data

1. `{analysis_dir}/drift-summary.json` - Aggregated drift report
2. `{analysis_dir}/drift-{chapter_id}.json` - Individual chapter reports
3. `{analysis_dir}/features.json` - Ground truth feature inventory

### Phase 3: Update Book Chapters

#### Step 1: Review Drift Summary

Read `{analysis_dir}/drift-summary.json` and prioritize:
1. Critical/High severity issues first
2. Medium severity issues next
3. Low severity issues last

#### Step 2: Process Each Chapter with Drift

For each chapter with drift:

#### a. Load Chapter Details
- Read drift report: `{analysis_dir}/drift-{chapter_id}.json`
- Read current chapter: From `chapter_file` in drift report
- Read source code: Use `source_reference` from drift issues
- Read feature inventory: `{analysis_dir}/features.json`

#### b. Analyze Issues
- Review all issues for the chapter
- Group related issues
- Identify fix strategy
- Note positive aspects to preserve

#### c. Apply Fixes by Issue Type

**For Missing Content:**
- Add new section or subsection
- Explain the feature clearly
- Provide practical examples
- Link to related content
- Include use cases

**For Outdated Information:**
- Update to current implementation
- Add deprecation notices if needed
- Show migration path if syntax changed
- Update all affected examples

**For Incorrect Examples:**
- Fix YAML syntax
- Add missing required fields
- Correct field types
- Test example actually works
- Add explanatory comments

**For Incomplete Explanation:**
- Expand explanation with more detail
- Add examples for complex features
- Clarify use cases
- Add diagrams if helpful

**For Missing Best Practices:**
- Add common pattern examples
- Document gotchas and workarounds
- Include optimization tips
- Link to advanced topics

**For Unclear Content:**
- Reorganize for better flow
- Simplify complex explanations
- Add more examples
- Break into smaller sections

#### Step 3: Maintain Book Quality

While fixing drift:

#### Preserve Good Content
- Keep clear, well-written explanations
- Maintain helpful examples that work
- Preserve good structure and flow
- Keep diagrams and illustrations

#### Improve Clarity
- Use consistent terminology across chapters
- Add code comments to complex examples
- Use tables for reference information
- Add cross-references between chapters

#### Update Examples
- Ensure all YAML examples are valid
- Use realistic, practical examples
- Show progression from simple to advanced
- Include expected output where helpful

#### Maintain Book Style
- Follow mdBook conventions
- Use consistent heading levels
- Use consistent code block formatting
- Match tone and voice of existing chapters
- Maintain accessibility and readability

#### Step 4: Verify Changes

After updating each chapter:

#### Check Technical Accuracy
- Examples use correct field names and types
- Struct definitions match source code
- Required vs optional fields accurate
- Default values correct

#### Check Completeness
- All major features covered
- Important use cases shown
- Common pitfalls mentioned
- Best practices included

#### Check Clarity and Flow
- Logical progression of concepts
- Clear explanations
- Examples well-integrated
- Transitions between sections smooth

#### Check Cross-References
- Links to other chapters work
- References are accurate
- Related content is linked
- No broken links

#### Step 5: Update SUMMARY.md if Needed

If you added new chapters or reorganized:
- Update `book/src/SUMMARY.md`
- Ensure chapter ordering makes sense
- Check indentation for sub-chapters
- Verify all files are referenced

#### Step 6: Test Book Build

**Use Book Directory from Configuration:**
Read `book_dir` from the config file loaded earlier.

Verify the book builds successfully:
```bash
cd {book_dir} && mdbook build
```

If build fails, fix errors before proceeding.

#### Step 7: Create Update Summary

Write summary to `{analysis_dir}/updates-applied.md`:

```markdown
# {Project Name} Book Documentation Updates

## Summary
- Analyzed: {N} chapters
- Found drift: {N} chapters
- Total issues fixed: {N}
- Severity: {N} high, {N} medium, {N} low

## Chapters Updated

### {Chapter Name} ({Severity} severity - {N} issues fixed)
- ✓ {Issue fixed}
- ✓ {Issue fixed}
- ✓ {Issue fixed}

### {Chapter Name} ({Severity} severity - {N} issues fixed)
- ✓ {Issue fixed}
- ✓ {Issue fixed}

## Examples Updated
- {N} YAML examples corrected
- {N} new examples added
- {N} deprecated examples removed or marked

## Content Added
- {New section added}
- {New explanation added}
- {New best practice documented}

## Deprecation Notices Added
- {Deprecated feature with migration path}

## Source Files Referenced
- {List of key source files used}
```

#### Step 8: Create Git Commit

**Commit Message Format:**
Use the project name from `--project` parameter.

```
docs: fix {project} book drift - update {N} chapters

Updated {project} book documentation to match current implementation:
- {Chapter}: {summary of changes}
- {Chapter}: {summary of changes}
- {Chapter}: {summary of changes}

Issues resolved:
- {N} high severity (missing features, outdated info)
- {N} medium severity (incorrect examples, incomplete explanations)
- {N} low severity (clarity improvements)

Book builds successfully ✓
```

**Commit Contents:**
- Updated `{book_src}/{chapters}.md` (from config)
- Updated `{book_src}/SUMMARY.md` (if needed)
- Created `{analysis_dir}/updates-applied.md`

### Phase 4: Chapter-Specific Guidance

#### For Workflow Basics
- Focus on clarity and simplicity
- Use beginner-friendly examples
- Explain concepts before showing code
- Link to advanced chapters for complex features

#### For MapReduce
- Explain parallel execution model clearly
- Show both simple and complex examples
- Document checkpoint/resume capabilities
- Explain work distribution

#### For Command Types
- Document all command types consistently
- Show common use case for each
- Include field reference table
- Link to related chapters

#### For Variables
- Organize by category (standard, mapreduce, etc.)
- Show practical interpolation examples
- Explain when each variable is available
- Include capture format examples

#### For Environment
- Show progression: simple → profiles → dynamic
- Explain secrets management clearly
- Include security best practices
- Show step-level overrides

#### For Advanced Features
- Assume reader understands basics
- Show practical use cases
- Link back to fundamentals
- Include troubleshooting tips

#### For Error Handling
- Explain workflow vs command level
- Show error recovery patterns
- Document DLQ workflow
- Include debugging tips

#### For Examples
- Use realistic scenarios
- Include complete, working examples
- Explain the "why" not just "what"
- Show expected output

#### For Troubleshooting
- Organize by symptom
- Provide step-by-step solutions
- Include diagnostic commands
- Link to relevant chapters

### Phase 5: Quality Standards

#### Do Add Value
- Explain concepts, not just syntax
- Show practical use cases
- Include best practices
- Highlight common pitfalls

#### Do Maintain Consistency
- Use same terminology throughout
- Follow established patterns
- Match existing tone and voice
- Keep technical level appropriate

#### Don't Over-Document
- Focus on user needs
- Skip internal implementation details
- Avoid redundant explanations
- Keep examples concise but complete

#### Don't Break Working Content
- Preserve good examples
- Keep clear explanations
- Maintain helpful diagrams
- Don't change structure unnecessarily

### Phase 6: Validation

The updated book must:
1. ✓ Fix ALL issues identified in drift reports
2. ✓ Build successfully with `mdbook build`
3. ✓ Have valid, working YAML examples
4. ✓ Be accurate to current codebase
5. ✓ Remain clear and accessible
6. ✓ Follow mdBook best practices
7. ✓ Include cross-references between chapters
8. ✓ Maintain consistent style and tone

### Phase 7: Final Checks

Before committing:

1. **Build Test**: `cd book && mdbook build` succeeds
2. **Link Check**: All internal links work
3. **Example Validation**: All YAML examples are syntactically valid
4. **Completeness**: All drift issues addressed
5. **Quality**: Chapters flow well and read clearly
6. **Consistency**: Terminology and style consistent across chapters
