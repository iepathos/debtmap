# /prodigy-split-oversized-chapters

Orchestrate the splitting of all oversized book chapters identified in the structure analysis report. This command reads the structure report, identifies high-priority chapters that need splitting, and systematically migrates each one to multi-subsection format.

## Purpose

This command serves as an **orchestration layer** between structure analysis and the map phase. It bridges the gap between recommendations (from `/prodigy-analyze-chapter-structure`) and execution (via `/prodigy-create-chapter-subsections`).

## Variables

- `--project <name>` - Project name (e.g., "Prodigy")
- `--chapters <path>` - Path to chapter definitions JSON (e.g., "workflows/data/prodigy-chapters.json")
- `--book-dir <path>` - Book directory path (e.g., "book")
- `--structure-report <path>` - Path to structure analysis report (e.g., ".prodigy/book-analysis/structure-report.json")

## Execute

### Phase 1: Parse Parameters and Load Structure Report

**Parse Command Arguments:**
Extract all required parameters:
- `--project`: Project name for output messages
- `--chapters`: Path to chapter definitions JSON
- `--book-dir`: Book directory path
- `--structure-report`: Path to structure analysis report

**Validate Parameters:**
- Ensure all required parameters are provided
- Verify structure report file exists
- Check that chapters file exists
- Verify book directory exists

**Load Structure Report:**
1. Read the structure report JSON file
2. Parse the recommendations array
3. Extract chapters that need splitting

### Phase 2: Filter High-Priority Chapters

**Identify Chapters to Split:**

Filter recommendations based on these criteria:
- `priority == "high"` - Only high-priority oversized chapters
- `recommended_action == "split_into_subsections"` - Explicitly needs splitting
- `proposed_structure` exists - Has concrete subsection recommendations

**Build Split List:**
Create an ordered list of chapters to split:
```json
[
  {
    "chapter_id": "configuration",
    "chapter_title": "Configuration",
    "current_file": "book/src/configuration.md",
    "total_lines": 1843,
    "proposed_subsections": 8,
    "reason": "Oversized chapter with 12 substantial H2 sections"
  },
  ...
]
```

**Handle Empty List:**
- If no chapters need splitting, print success message and exit
- Example: "✅ All chapters are well-sized - no splitting needed"
- This is a valid success state, not an error

### Phase 3: Display Split Plan

**Print Summary:**
```
📋 Book Chapter Splitting Plan
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Project: {project-name}
Chapters to split: {count}

Chapters:
  1. Configuration (1843 lines → 8 subsections)
  2. Error Handling (600 lines → 3 subsections)
  ...

This will:
  • Create multi-subsection directory structure
  • Generate index.md for each chapter
  • Create individual subsection files
  • Update {chapters-file}
  • Update SUMMARY.md
  • Archive original files

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

### Phase 4: Execute Chapter Splitting

**For Each Chapter in Split List:**

1. **Announce Current Chapter:**
   ```
   📖 Splitting chapter {N}/{total}: {chapter-title}
   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
   ```

2. **Invoke Split Command:**
   Use the Read tool to execute the split command:
   ```bash
   /prodigy-create-chapter-subsections \
     --project {project-name} \
     --chapter {chapter-id} \
     --chapters {chapters-file} \
     --book-dir {book-dir} \
     --structure-report {structure-report-path}
   ```

3. **Verify Success:**
   - Check that command completed without errors
   - Verify expected files were created
   - Confirm chapters.json was updated

4. **Track Progress:**
   - Count successful splits
   - Record any failures
   - Continue with remaining chapters even if one fails

5. **Display Result:**
   ```
   ✅ Successfully split {chapter-title} into {N} subsections
   ```
   or
   ```
   ❌ Failed to split {chapter-title}: {error-message}
   ```

### Phase 5: Verify Structural Integrity

**After All Splits Complete:**

1. **Verify Files Created:**
   - Check that all expected directories exist
   - Verify index.md files created
   - Verify subsection files created

2. **Verify Configuration Updates:**
   - Parse chapters.json to ensure all splits are reflected
   - Check that SUMMARY.md has new entries
   - Verify original files were archived

3. **Count Changes:**
   - Total chapters split
   - Total subsections created
   - Total files modified

### Phase 6: Display Final Summary

**Print Comprehensive Summary:**

```
✅ Chapter Splitting Complete
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📊 Results:
  ✓ Chapters split: {successful}/{total}
  ✓ Subsections created: {total-subsections}
  ✓ Directories created: {directories}
  ✓ Files modified: {files-modified}

📁 Structure Changes:
  • chapters.json updated with {N} multi-subsection entries
  • SUMMARY.md updated with nested navigation
  • Original files archived to *.md.bak

{If any failures:}
⚠️  Warnings:
  • Failed to split: {failed-chapter-1}
  • Failed to split: {failed-chapter-2}
  Review errors above for details.

📝 Next Steps:
  1. Review new chapter structure in {book-dir}/src/
  2. Verify SUMMARY.md navigation is correct
  3. Continue to map phase for drift detection
  4. Run 'mdbook build' to test final result

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

### Phase 7: Create Git Commit

**Stage All Changes:**
1. New directories and subsection files
2. Updated chapters.json
3. Updated SUMMARY.md
4. Archived original files

**Generate Commit Message:**
```
docs: split {N} oversized book chapters into subsections

Split the following chapters to improve organization:
- Configuration (1843 lines → 8 subsections)
- Error Handling (600 lines → 3 subsections)
...

Total changes:
- {N} chapters migrated to multi-subsection format
- {M} subsections created
- Updated chapters.json and SUMMARY.md

This prepares documentation for efficient drift detection
in the map phase, where each subsection gets focused attention.
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
- Suggest running `/prodigy-analyze-chapter-structure`

**Handle Split Failures:**
- Log specific error for each failed chapter
- Continue with remaining chapters (don't abort entire operation)
- Include failed chapters in final summary
- Exit with warning if any failures occurred

**Handle File System Errors:**
- Permission denied → clear error message
- Disk full → clear error message
- Invalid paths → validate before attempting split

**Handle Configuration Errors:**
- chapters.json update fails → roll back that chapter's changes
- SUMMARY.md update fails → roll back that chapter's changes
- Provide recovery instructions

### Quality Guidelines

**Orchestration:**
- Clear progress indicators for each chapter
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
- Atomic operations per chapter (all or nothing)
- Don't leave partial state on failure
- Easy to identify and fix issues

## Success Indicators

Operation is successful when:
- All high-priority chapters successfully split
- All expected files created
- Configuration files updated correctly
- Git commit created
- Clear summary provided to user

## Partial Success Handling

If some chapters fail but others succeed:
- Mark as partial success (warning, not error)
- Commit successful changes
- Provide clear list of failures
- Allow workflow to continue (don't block map phase)

## Example Usage

```bash
# Typical usage in workflow
/prodigy-split-oversized-chapters \
  --project Prodigy \
  --chapters workflows/data/prodigy-chapters.json \
  --book-dir book \
  --structure-report .prodigy/book-analysis/structure-report.json

# Result: All oversized chapters split and committed
```

## Integration with Workflow

This command fits between structure analysis and chapter discovery:

1. `/prodigy-analyze-chapter-structure` - Generate recommendations
2. **`/prodigy-split-oversized-chapters`** - Execute splits ← THIS COMMAND
3. Regenerate flattened-items.json (now includes new subsections)
4. Map phase processes optimally-sized chapters

## Scope Notes

This command:
- ✅ Orchestrates multiple chapter splits
- ✅ Provides progress tracking and reporting
- ✅ Creates comprehensive git commit
- ✅ Handles errors gracefully

This command does NOT:
- ❌ Analyze chapter structure (use `/prodigy-analyze-chapter-structure`)
- ❌ Perform the actual splitting (delegates to `/prodigy-create-chapter-subsections`)
- ❌ Detect drift (happens in map phase)
