# /prodigy-create-chapter-subsections

Migrate a single-file chapter to multi-subsection format by splitting it into an index.md file and individual subsection files. This command handles the structural migration while preserving all content.

## Purpose

This command performs **structure migration** - converting a flat single-file chapter into an organized multi-subsection structure. It should be run based on recommendations from `/prodigy-analyze-chapter-structure`.

## Variables

- `--project <name>` - Project name (e.g., "Debtmap")
- `--chapter <id>` - Chapter ID to migrate (e.g., "configuration")
- `--chapters <path>` - Path to chapter definitions JSON (e.g., "workflows/data/prodigy-chapters.json")
- `--book-dir <path>` - Book directory path (e.g., "book")
- `--structure-report <path>` - Optional path to structure report from analysis command
- `--dry-run` - Preview changes without modifying files (default: false)

## Execute

### Phase 1: Parse Parameters and Load Data

**Parse Command Arguments:**
Extract all required parameters:
- `--project`: Project name for output messages
- `--chapter`: Chapter ID to migrate
- `--chapters`: Path to chapter definitions JSON
- `--book-dir`: Book directory path (default: "book")
- `--structure-report`: Optional path to structure analysis report
- `--dry-run`: If true, show what would be done without modifying files

**Validate Parameters:**
- Ensure `--chapter` is provided and not empty
- Verify chapter exists in chapters.json
- Check that chapter is currently `type: "single-file"` or has no type
- Error if chapter is already `type: "multi-subsection"`

**Load Configuration Files:**
1. Read `--chapters` file to get chapter definition
2. If `--structure-report` provided, read it for recommendations
3. Verify source chapter file exists

### Phase 2: Analyze Source Chapter Structure

**Read and Parse Source File:**

1. **Load Chapter Content:**
   - Read the entire source chapter file
   - Preserve original content exactly (no modifications yet)

2. **Extract Chapter Metadata:**
   - Extract H1 title (first # heading)
   - Extract introduction (content before first H2)
   - Record file metadata (creation date, etc.)

3. **Parse Section Structure:**
   - Find all H2 sections (## headers)
   - For each H2 section:
     - Extract section title
     - Extract section slug (for file naming)
     - Capture all content from H2 to next H2 or EOF
     - Identify subsections (H3+ content under this H2)
     - Record line numbers for extraction

4. **Build Section Map:**
   Create data structure:
   ```json
   {
     "chapter_title": "Configuration",
     "introduction": "Content before first H2...",
     "sections": [
       {
         "title": "Basic Configuration",
         "slug": "basic-configuration",
         "content": "Full section content...",
         "start_line": 10,
         "end_line": 150,
         "line_count": 140
       },
       ...
     ]
   }
   ```

### Phase 3: Apply Subsection Recommendations (If Available)

**If structure-report Provided:**

1. **Find Recommendations for This Chapter:**
   - Look up chapter in recommendations array
   - Extract proposed subsection structure

2. **Validate Recommendations Match Reality:**
   - Verify proposed subsections match actual H2 sections
   - Check that source_sections exist in parsed content
   - Warn if mismatches found (file may have changed since analysis)

3. **Use Recommended Structure:**
   - Follow proposed subsection grouping
   - Use recommended file names and IDs
   - Apply suggested organization

**If No structure-report:**

1. **Generate Default Subsection Structure:**
   - Each H2 section becomes one subsection
   - Use H2 title for subsection title
   - Generate kebab-case ID from title
   - Create file path: `book/src/{chapter-id}/{subsection-id}.md`

2. **Filter Meta-Sections:**
   - Do NOT create subsections for: "Best Practices", "Troubleshooting", "Examples", "Common Patterns"
   - These should stay in index.md as H2 sections

3. **Validate Subsection Viability:**
   - Only create subsections for sections >50 lines
   - Ensure at least 3 subsections (otherwise not worth splitting)
   - Warn if too many subsections (>12 may be over-fragmented)

### Phase 4: Generate New File Structure

**Create Directory Structure:**

1. **Create Chapter Directory:**
   - Path: `book/src/{chapter-id}/`
   - Create if doesn't exist
   - Verify write permissions

**Generate index.md:**

1. **Build Index Content:**
   ```markdown
   # {Chapter Title}

   {Introduction content from before first H2}

   ## Subsections

   This chapter is organized into the following subsections:

   - [{Subsection 1 Title}](./{subsection-1-id}.md) - {Brief description}
   - [{Subsection 2 Title}](./{subsection-2-id}.md) - {Brief description}
   ...

   {Include meta-sections like "Best Practices" as H2s here}

   ## Best Practices

   {Content from Best Practices section if it existed}
   ```

2. **Write index.md:**
   - Path: `book/src/{chapter-id}/index.md`
   - Use original introduction content
   - Add subsection navigation
   - Include any meta-sections that shouldn't be separate files

**Generate Subsection Files:**

For each subsection:

1. **Extract Subsection Content:**
   - Get content from H2 to next H2
   - Preserve all formatting, code blocks, links
   - Keep H3+ hierarchy intact

2. **Generate Subsection Header:**
   ```markdown
   # {Subsection Title}

   > Part of the [{Chapter Title}](./index.md) chapter

   {Subsection content starts here...}
   ```

3. **Fix Internal Links:**
   - Relative links that worked in single file may break
   - Update anchor links (e.g., `#basic-config` → `./basic-configuration.md`)
   - Fix cross-references to other subsections
   - Update image paths if needed

4. **Write Subsection File:**
   - Path: `book/src/{chapter-id}/{subsection-id}.md`
   - Preserve formatting exactly
   - Ensure proper markdown syntax

### Phase 5: Update Chapter Definitions (chapters.json)

**Modify Chapter Definition:**

Transform from:
```json
{
  "id": "configuration",
  "title": "Configuration",
  "file": "book/src/configuration.md",
  "topics": [...],
  "validation": "..."
}
```

To:
```json
{
  "id": "configuration",
  "title": "Configuration",
  "type": "multi-subsection",
  "index_file": "book/src/configuration/index.md",
  "topics": [...],
  "validation": "...",
  "subsections": [
    {
      "id": "basic-configuration",
      "title": "Basic Configuration",
      "file": "book/src/configuration/basic-configuration.md",
      "topics": ["Config files", "Basic options"],
      "validation": "Check basic configuration options are documented"
    },
    ...
  ]
}
```

**Generate Subsection Metadata:**

For each subsection:
- Extract topics from subsection content (H3 headings)
- Generate validation criteria based on content
- Preserve any manual annotations

**Write Updated chapters.json:**
- Maintain JSON formatting (2-space indent)
- Preserve all other chapters unchanged
- Keep logical ordering

### Phase 6: Update SUMMARY.md

**Parse Current SUMMARY.md:**

1. **Find Chapter Entry:**
   - Locate line with chapter reference
   - Example: `- [Configuration](configuration.md)`

2. **Replace with Multi-Level Structure:**
   ```markdown
   - [Configuration](configuration/index.md)
     - [Basic Configuration](configuration/basic-configuration.md)
     - [Role-Based Scoring](configuration/role-based-scoring.md)
     - [Orchestration Config](configuration/orchestration-config.md)
     ...
   ```

3. **Maintain Indentation:**
   - Use consistent indentation (2 or 4 spaces)
   - Match existing SUMMARY.md style
   - Preserve section groupings

4. **Write Updated SUMMARY.md:**
   - Preserve all other entries unchanged
   - Keep proper ordering
   - Ensure valid markdown list syntax

### Phase 7: Archive Old Single File

**Preserve Original for Safety:**

1. **Create Backup:**
   - Move original file to: `book/src/{chapter-id}.md.bak`
   - Or copy to: `book/src/.archive/{chapter-id}-{timestamp}.md`
   - Preserve for rollback if needed

2. **Record Migration:**
   - Log migration in `.prodigy/migrations.log`
   - Include timestamp, chapter ID, file count
   - Note any warnings or issues

### Phase 8: Validation and Quality Checks

**Verify Migration Integrity:**

1. **Content Preservation:**
   - Count lines in original file
   - Count total lines in new index.md + subsections
   - Verify counts match (within reason for added headers)
   - Ensure no content was lost

2. **Link Validation:**
   - Test all internal links work
   - Check cross-references between subsections
   - Verify external links preserved

3. **Markdown Validity:**
   - Parse each new file for syntax errors
   - Check code block formatting
   - Verify heading hierarchy

4. **Build Test:**
   - Run `mdbook build` to ensure book compiles
   - Check for broken links
   - Verify no errors in output

**If Validation Fails:**
- Log specific errors
- Do NOT commit changes
- Optionally restore from backup
- Provide clear error messages to user

### Phase 9: Generate Migration Report

**Create Detailed Report:**

```json
{
  "migration_date": "<timestamp>",
  "project": "<project-name>",
  "chapter_id": "<chapter-id>",
  "chapter_title": "<title>",
  "migration_type": "single-file → multi-subsection",
  "dry_run": true|false,
  "source_file": "book/src/configuration.md",
  "destination_directory": "book/src/configuration/",
  "metrics": {
    "original_lines": 1843,
    "index_lines": 150,
    "subsections_created": 8,
    "total_subsection_lines": 1693,
    "content_preservation": "100%"
  },
  "files_created": [
    "book/src/configuration/index.md",
    "book/src/configuration/basic-configuration.md",
    ...
  ],
  "files_modified": [
    "workflows/data/prodigy-chapters.json",
    "book/src/SUMMARY.md"
  ],
  "files_archived": [
    "book/src/configuration.md → book/src/configuration.md.bak"
  ],
  "validation": {
    "content_preserved": true,
    "links_valid": true,
    "markdown_valid": true,
    "build_successful": true
  },
  "warnings": [],
  "errors": []
}
```

### Phase 10: Display Summary and Commit

**Print User-Friendly Summary:**

```
📁 Chapter Subsection Migration
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Chapter: Configuration
Status: ✅ Migration successful

📊 Migration Summary:
  Original: 1 file (1843 lines)
  New Structure:
    - index.md (150 lines)
    - 8 subsections (1693 lines total)

📝 Subsections Created:
  ✓ Basic Configuration (200 lines)
  ✓ Role-Based Scoring (250 lines)
  ✓ Orchestration Config (180 lines)
  ✓ Threshold Settings (150 lines)
  ✓ Advanced Options (220 lines)
  ✓ Plugin Configuration (190 lines)
  ✓ Caching Settings (170 lines)
  ✓ Performance Tuning (183 lines)

✅ Validation:
  ✓ Content preservation: 100%
  ✓ Links verified
  ✓ Markdown valid
  ✓ mdbook build successful

📂 Files Modified:
  ✓ Created book/src/configuration/ directory
  ✓ Created 9 markdown files
  ✓ Updated workflows/data/prodigy-chapters.json
  ✓ Updated book/src/SUMMARY.md
  ✓ Archived book/src/configuration.md

📝 Next Steps:
  1. Review new structure in book/src/configuration/
  2. Run drift detection on subsections
  3. Commit changes when satisfied
```

**Stage and Commit Changes (if not dry-run):**

If running in automation mode (PRODIGY_AUTOMATION=true) and not dry-run:

1. **Stage All Files:**
   - New directory and all subsection files
   - Updated chapters.json
   - Updated SUMMARY.md
   - Backup of original file

2. **Create Commit:**
   ```
   docs: migrate {Chapter Title} to multi-subsection format

   Split {chapter-id}.md (1843 lines) into:
   - index.md with chapter overview
   - 8 subsections for better organization

   This improves navigability and allows focused drift detection
   per subsection instead of entire chapter.

   Subsections:
   - Basic Configuration
   - Role-Based Scoring
   - Orchestration Config
   - Threshold Settings
   - Advanced Options
   - Plugin Configuration
   - Caching Settings
   - Performance Tuning
   ```

### Error Handling

**Handle Missing Files:**
- If source chapter file doesn't exist, error and exit
- If chapters.json missing, error and exit
- If SUMMARY.md missing, error and exit

**Handle Migration Failures:**
- If directory creation fails, roll back and error
- If file write fails, roll back and error
- If validation fails, do not commit, report errors

**Handle Edge Cases:**
- Empty sections → skip or include in index.md
- Duplicate section titles → append numeric suffix
- Very large sections → warn but proceed
- No suitable sections for subsections → error (chapter should stay single-file)

**Rollback on Failure:**
- Restore original file from backup
- Delete newly created directory
- Revert chapters.json changes
- Revert SUMMARY.md changes
- Log rollback action

### Quality Guidelines

**Accuracy:**
- Preserve all content exactly (no data loss)
- Maintain formatting and structure
- Keep links working

**Safety:**
- Always create backups before modifying
- Validate before committing
- Support dry-run mode for preview
- Easy rollback if issues found

**User Experience:**
- Clear progress indicators
- Detailed summary of changes
- Actionable error messages
- Easy to understand results

## Configuration Defaults

```json
{
  "min_section_lines": 50,
  "min_subsections": 3,
  "max_subsections": 12,
  "backup_originals": true,
  "validate_before_commit": true
}
```

## Dry-Run Mode

When `--dry-run` is true:
- Analyze and show what would be done
- Do NOT create any files
- Do NOT modify any files
- Do NOT commit changes
- Print detailed preview of changes

## Success Indicators

Migration is successful when:
- All content preserved (verified)
- All links working (verified)
- mdbook builds successfully
- chapters.json updated correctly
- SUMMARY.md updated correctly
- No validation errors
- User can navigate new structure

## Scope Notes

This command does ONE thing: migrate a chapter structure. It does NOT:
- ❌ Analyze chapter sizes (use `/prodigy-analyze-chapter-structure`)
- ❌ Detect documentation gaps (use `/prodigy-detect-documentation-gaps`)
- ❌ Fix content drift (that happens in map phase after migration)

## Example Usage

```bash
# Preview migration
/prodigy-create-chapter-subsections \
  --project Debtmap \
  --chapter configuration \
  --chapters workflows/data/prodigy-chapters.json \
  --book-dir book \
  --dry-run

# Execute migration
/prodigy-create-chapter-subsections \
  --project Debtmap \
  --chapter configuration \
  --chapters workflows/data/prodigy-chapters.json \
  --book-dir book

# Use structure analysis recommendations
/prodigy-create-chapter-subsections \
  --project Debtmap \
  --chapter configuration \
  --chapters workflows/data/prodigy-chapters.json \
  --book-dir book \
  --structure-report .prodigy/book-analysis/structure-report.json
```

## Integration with Workflow

This command should be run AFTER analyzing chapter structure:

1. `/prodigy-analyze-chapter-structure` - Identifies oversized chapters
2. Review recommendations
3. **`/prodigy-create-chapter-subsections`** - Migrate each oversized chapter ← THIS COMMAND
4. `/prodigy-detect-documentation-gaps` - Regenerate flattened items with new structure
5. Run drift detection on new subsections
