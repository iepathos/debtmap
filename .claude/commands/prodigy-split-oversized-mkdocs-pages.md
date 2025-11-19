# /prodigy-split-oversized-mkdocs-pages

Orchestrate the splitting of all oversized MkDocs pages identified in the structure analysis report. This command reads the structure report, identifies high-priority pages that need splitting, and systematically migrates each one to multi-subpage format.

## Purpose

This command serves as an **orchestration layer** between structure analysis and the map phase. It bridges the gap between recommendations (from `/prodigy-analyze-mkdocs-structure`) and execution (via `/prodigy-create-mkdocs-subpages`).

## Variables

- `--project <name>` - Project name (e.g., "Prodigy")
- `--pages <path>` - Path to page definitions JSON (e.g., "workflows/data/mkdocs-chapters.json")
- `--docs-dir <path>` - MkDocs directory path (e.g., "docs")
- `--structure-report <path>` - Path to structure analysis report (e.g., ".prodigy/mkdocs-analysis/structure-report.json")

## Execute

### Phase 1: Parse Parameters and Load Structure Report

**Parse Command Arguments:**
Extract all required parameters:
- `--project`: Project name for output messages
- `--pages`: Path to page definitions JSON
- `--docs-dir`: MkDocs directory path
- `--structure-report`: Path to structure analysis report

**Validate Parameters:**
- Ensure all required parameters are provided
- Verify structure report file exists
- Check that pages file exists
- Verify docs directory exists

**Load Structure Report:**
1. Read the structure report JSON file
2. Parse the recommendations array
3. Extract pages that need splitting

### Phase 2: Filter High-Priority Pages

**Identify Pages to Split:**

Filter recommendations based on these criteria:
- `priority == "high"` - Only high-priority oversized pages
- `recommended_action == "split_into_subsections"` - Explicitly needs splitting
- `proposed_structure` exists - Has concrete subsection recommendations

**Build Split List:**
Create an ordered list of pages to split:
```json
[
  {
    "page_id": "configuration",
    "page_title": "Configuration",
    "current_file": "docs/src/configuration.md",
    "total_lines": 1843,
    "proposed_subsections": 8,
    "reason": "Oversized page with 12 substantial H2 sections"
  },
  ...
]
```

**Handle Empty List:**
- If no pages need splitting, print success message and exit
- Example: "✅ All pages are well-sized - no splitting needed"
- This is a valid success state, not an error

### Phase 3: Display Split Plan

**Print Summary:**
```
📋 MkDocs Page Splitting Plan
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Project: {project-name}
Pages to split: {count}

Pages:
  1. Configuration (1843 lines → 8 subpages)
  2. Error Handling (600 lines → 3 subpages)
  ...

This will:
  • Create multi-subpage directory structure
  • Generate index.md for each page
  • Create individual subpage files
  • Update {pages-file}
  • Update mkdocs.yml
  • Archive original files

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

### Phase 4: Execute Page Splitting

**For Each Page in Split List:**

1. **Announce Current Page:**
   ```
   📄 Splitting page {N}/{total}: {page-title}
   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
   ```

2. **Invoke Split Command:**
   Use the Read tool to execute the split command:
   ```bash
   /prodigy-create-mkdocs-subpages \
     --project {project-name} \
     --page {page-id} \
     --pages {pages-file} \
     --docs-dir {docs-dir} \
     --structure-report {structure-report-path}
   ```

3. **Verify Success:**
   - Check that command completed without errors
   - Verify expected files were created
   - Confirm pages.json was updated

4. **Track Progress:**
   - Count successful splits
   - Record any failures
   - Continue with remaining pages even if one fails

5. **Display Result:**
   ```
   ✅ Successfully split {page-title} into {N} subpages
   ```
   or
   ```
   ❌ Failed to split {page-title}: {error-message}
   ```

### Phase 5: Verify Structural Integrity

**After All Splits Complete:**

1. **Verify Files Created:**
   - Check that all expected directories exist
   - Verify index.md files created
   - Verify subpage files created

2. **Verify Configuration Updates:**
   - Parse pages.json to ensure all splits are reflected
   - Check that mkdocs.yml has new entries
   - Verify original files were archived

3. **Count Changes:**
   - Total pages split
   - Total subpages created
   - Total files modified

### Phase 6: Display Final Summary

**Print Comprehensive Summary:**

```
✅ Page Splitting Complete
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📊 Results:
  ✓ Pages split: {successful}/{total}
  ✓ Subpages created: {total-subpages}
  ✓ Directories created: {directories}
  ✓ Files modified: {files-modified}

📁 Structure Changes:
  • pages.json updated with {N} multi-subpage entries
  • mkdocs.yml updated with nested navigation
  • Original files archived to *.md.bak

{If any failures:}
⚠️  Warnings:
  • Failed to split: {failed-page-1}
  • Failed to split: {failed-page-2}
  Review errors above for details.

📝 Next Steps:
  1. Review new page structure in {docs-dir}
  2. Verify mkdocs.yml navigation is correct
  3. Continue to map phase for drift detection
  4. Run 'mkdocs build' to test final result

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

### Phase 7: Create Git Commit

**Stage All Changes:**
1. New directories and subpage files
2. Updated pages.json
3. Updated mkdocs.yml
4. Archived original files

**Generate Commit Message:**
```
docs: split {N} oversized MkDocs pages into subpages

Split the following pages to improve organization:
- Configuration (1843 lines → 8 subpages)
- Error Handling (600 lines → 3 subpages)
...

Total changes:
- {N} pages migrated to multi-subpage format
- {M} subpages created
- Updated pages.json and mkdocs.yml

This prepares documentation for efficient drift detection
in the map phase, where each subpage gets focused attention.
```

**Execute Commit:**
```bash
git add -A
git commit -m "{commit-message}"
```

### Error Handling

**Handle Missing Structure Report:**
- If structure report doesn't exist, error and exit
- Provide clear message about running analysis first
- Suggest running `/prodigy-analyze-mkdocs-structure`

**Handle Split Failures:**
- Log specific error for each failed page
- Continue with remaining pages (don't abort entire operation)
- Include failed pages in final summary
- Exit with warning if any failures occurred

**Handle File System Errors:**
- Permission denied → clear error message
- Disk full → clear error message
- Invalid paths → validate before attempting split

**Handle Configuration Errors:**
- Pages.json update fails → roll back that page's changes
- mkdocs.yml update fails → roll back that page's changes
- Provide recovery instructions

### Quality Guidelines

**Orchestration:**
- Clear progress indicators for each page
- Detailed logging of all operations
- Comprehensive error reporting
- Graceful degradation (continue on partial failure)

**User Experience:**
- Show what's happening in real-time
- Provide actionable error messages
- Clear final summary with next steps
- Make it obvious if manual intervention needed

**Safety:**
- Verify preconditions before starting
- Atomic operations per page (all or nothing)
- Don't leave partial state on failure
- Easy to identify and fix issues

## Success Indicators

Operation is successful when:
- All high-priority pages successfully split
- All expected files created
- Configuration files updated correctly
- Git commit created
- Clear summary provided to user

## Partial Success Handling

If some pages fail but others succeed:
- Mark as partial success (warning, not error)
- Commit successful changes
- Provide clear list of failures
- Allow workflow to continue (don't block map phase)

## Example Usage

```bash
# Typical usage in workflow
/prodigy-split-oversized-mkdocs-pages \
  --project Prodigy \
  --pages workflows/data/mkdocs-chapters.json \
  --docs-dir docs \
  --structure-report .prodigy/mkdocs-analysis/structure-report.json

# Result: All oversized pages split and committed
```

## Integration with Workflow

This command fits between structure analysis and page discovery:

1. `/prodigy-analyze-mkdocs-structure` - Generate recommendations
2. **`/prodigy-split-oversized-mkdocs-pages`** - Execute splits ← THIS COMMAND
3. Auto-discover pages (now includes new subpages)
4. Map phase processes optimally-sized pages

## Scope Notes

This command:
- ✅ Orchestrates multiple page splits
- ✅ Provides progress tracking and reporting
- ✅ Creates comprehensive git commit
- ✅ Handles errors gracefully

This command does NOT:
- ❌ Analyze page structure (use `/prodigy-analyze-mkdocs-structure`)
- ❌ Perform the actual splitting (delegates to `/prodigy-create-mkdocs-subpages`)
- ❌ Detect drift (happens in map phase)
