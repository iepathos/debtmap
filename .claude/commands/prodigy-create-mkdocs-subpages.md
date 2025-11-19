# /prodigy-create-page-subpages

Migrate a single-file page to multi-subpage format by splitting it into an index.md file and individual subpage files. This command handles the structural migration while preserving all content.

## Purpose

This command performs **structure migration** - converting a flat single-file page into an organized multi-subpage structure. It should be run based on recommendations from `/prodigy-analyze-page-structure`.

## Variables

- `--project <name>` - Project name (e.g., "Debtmap")
- `--page <id>` - Page ID to migrate (e.g., "configuration")
- `--pages <path>` - Path to page definitions JSON (e.g., "workflows/data/prodigy-pages.json")
- `--docs-dir <path>` - MkDocs directory path (e.g., "docs")
- `--structure-report <path>` - Optional path to structure report from analysis command
- `--dry-run` - Preview changes without modifying files (default: false)

## Execute

### Phase 1: Parse Parameters and Load Data

**Parse Command Arguments:**
Extract all required parameters:
- `--project`: Project name for output messages
- `--page`: Page ID to migrate
- `--pages`: Path to page definitions JSON
- `--docs-dir`: MkDocs directory path (default: "docs")
- `--structure-report`: Optional path to structure analysis report
- `--dry-run`: If true, show what would be done without modifying files

**Validate Parameters:**
- Ensure `--page` is provided and not empty
- Verify page exists in pages.json
- Check that page is currently `type: "single-file"` or has no type
- Error if page is already `type: "multi-subpage"`

**Load Configuration Files:**
1. Read `--pages` file to get page definition
2. If `--structure-report` provided, read it for recommendations
3. Verify source page file exists

### Phase 2: Analyze Source Page Structure

**Read and Parse Source File:**

1. **Load Page Content:**
   - Read the entire source page file
   - Preserve original content exactly (no modifications yet)

2. **Extract Page Metadata:**
   - Extract H1 title (first # heading)
   - Extract introduction (content before first H2)
   - Record file metadata (creation date, etc.)

3. **Parse Section Structure:**
   - Find all H2 sections (## headers)
   - For each H2 section:
     - Extract section title
     - Extract section slug (for file naming)
     - Capture all content from H2 to next H2 or EOF
     - Identify subpages (H3+ content under this H2)
     - Record line numbers for extraction

4. **Build Section Map:**
   Create data structure:
   ```json
   {
     "page_title": "Configuration",
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

1. **Find Recommendations for This Page:**
   - Look up page in recommendations array
   - Extract proposed subpage structure

2. **Validate Recommendations Match Reality:**
   - Verify proposed subpages match actual H2 sections
   - Check that source_sections exist in parsed content
   - Warn if mismatches found (file may have changed since analysis)

3. **Use Recommended Structure:**
   - Follow proposed subpage grouping
   - Use recommended file names and IDs
   - Apply suggested organization

**If No structure-report:**

1. **Generate Default Subsection Structure:**
   - Each H2 section becomes one subpage
   - Use H2 title for subpage title
   - Generate kebab-case ID from title
   - Create file path: `docs/src/{page-id}/{subpage-id}.md`

2. **Filter Meta-Sections:**
   - Do NOT create subpages for: "Best Practices", "Troubleshooting", "Examples", "Common Patterns"
   - These should stay in index.md as H2 sections

3. **Validate Subsection Viability:**
   - Only create subpages for sections >50 lines
   - Ensure at least 3 subpages (otherwise not worth splitting)
   - Warn if too many subpages (>12 may be over-fragmented)

### Phase 4: Generate New File Structure

**Create Directory Structure:**

1. **Create Page Directory:**
   - Path: `docs/src/{page-id}/`
   - Create if doesn't exist
   - Verify write permissions

**Generate index.md:**

1. **Build Index Content:**
   ```markdown
   # {Page Title}

   {Introduction content from before first H2}

   {Include meta-sections like "Best Practices" as H2s here if they exist}

   ## Best Practices

   {Content from Best Practices section if it existed}
   ```

   **IMPORTANT:** Do NOT include a "Subsections" section listing subpages. MkDocs Material automatically displays all subpages in the left navigation sidebar, making this redundant and creating a poor user experience.

2. **Write index.md:**
   - Path: `docs/src/{page-id}/index.md`
   - Use original introduction content
   - Include any meta-sections that shouldn't be separate files (e.g., "Best Practices", "Troubleshooting", "Common Patterns")
   - Do NOT add subpage navigation links (handled by MkDocs sidebar)

**Generate Subsection Files:**

For each subpage:

1. **Extract Subsection Content:**
   - Get content from H2 to next H2
   - Preserve all formatting, code blocks, links
   - Keep H3+ hierarchy intact

2. **Generate Subsection Header:**
   ```markdown
   # {Subsection Title}

   {Subsection content starts here...}
   ```

   **IMPORTANT:** Do NOT add a "Part of the [Parent](./index.md)" blockquote. This is an anti-pattern that clutters the documentation. MkDocs Material already provides breadcrumb navigation showing the page hierarchy.

3. **Fix Internal Links:**
   - Relative links that worked in single file may break
   - Update anchor links (e.g., `#basic-config` → `./basic-configuration.md`)
   - Fix cross-references to other subpages
   - Update image paths if needed

4. **Write Subsection File:**
   - Path: `docs/src/{page-id}/{subpage-id}.md`
   - Preserve formatting exactly
   - Ensure proper markdown syntax

### Phase 5: Update Page Definitions (pages.json)

**Modify Page Definition:**

Transform from:
```json
{
  "id": "configuration",
  "title": "Configuration",
  "file": "docs/src/configuration.md",
  "topics": [...],
  "validation": "..."
}
```

To:
```json
{
  "id": "configuration",
  "title": "Configuration",
  "type": "multi-subpage",
  "index_file": "docs/src/configuration/index.md",
  "topics": [...],
  "validation": "...",
  "subpages": [
    {
      "id": "basic-configuration",
      "title": "Basic Configuration",
      "file": "docs/src/configuration/basic-configuration.md",
      "topics": ["Config files", "Basic options"],
      "validation": "Check basic configuration options are documented"
    },
    ...
  ]
}
```

**Generate Subsection Metadata:**

For each subpage:
- Extract topics from subpage content (H3 headings)
- Generate validation criteria based on content
- Preserve any manual annotations

**Write Updated pages.json:**
- Maintain JSON formatting (2-space indent)
- Preserve all other pages unchanged
- Keep logical ordering

### Phase 6: Update mkdocs.yml

**Parse Current mkdocs.yml:**

1. **Find Page Entry:**
   - Locate line with page reference
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
   - Match existing mkdocs.yml style
   - Preserve section groupings

4. **Write Updated mkdocs.yml:**
   - Preserve all other entries unchanged
   - Keep proper ordering
   - Ensure valid markdown list syntax

### Phase 7: Archive Old Single File

**Preserve Original for Safety:**

1. **Create Backup:**
   - Move original file to: `docs/src/{page-id}.md.bak`
   - Or copy to: `docs/src/.archive/{page-id}-{timestamp}.md`
   - Preserve for rollback if needed

2. **Record Migration:**
   - Log migration in `.prodigy/migrations.log`
   - Include timestamp, page ID, file count
   - Note any warnings or issues

### Phase 8: Validation and Quality Checks

**Verify Migration Integrity:**

1. **Content Preservation:**
   - Count lines in original file
   - Count total lines in new index.md + subpages
   - Verify counts match (within reason for added headers)
   - Ensure no content was lost

2. **Link Validation:**
   - Test all internal links work
   - Check cross-references between subpages
   - Verify external links preserved

3. **Markdown Validity:**
   - Parse each new file for syntax errors
   - Check code block formatting
   - Verify heading hierarchy

4. **Build Test:**
   - Run `mddocs build` to ensure docs compiles
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
  "page_id": "<page-id>",
  "page_title": "<title>",
  "migration_type": "single-file → multi-subpage",
  "dry_run": true|false,
  "source_file": "docs/src/configuration.md",
  "destination_directory": "docs/src/configuration/",
  "metrics": {
    "original_lines": 1843,
    "index_lines": 150,
    "subpages_created": 8,
    "total_subpage_lines": 1693,
    "content_preservation": "100%"
  },
  "files_created": [
    "docs/src/configuration/index.md",
    "docs/src/configuration/basic-configuration.md",
    ...
  ],
  "files_modified": [
    "workflows/data/prodigy-pages.json",
    "docs/src/mkdocs.yml"
  ],
  "files_archived": [
    "docs/src/configuration.md → docs/src/configuration.md.bak"
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
📁 Page Subsection Migration
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Page: Configuration
Status: ✅ Migration successful

📊 Migration Summary:
  Original: 1 file (1843 lines)
  New Structure:
    - index.md (150 lines)
    - 8 subpages (1693 lines total)

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
  ✓ mddocs build successful

📂 Files Modified:
  ✓ Created docs/src/configuration/ directory
  ✓ Created 9 markdown files
  ✓ Updated workflows/data/prodigy-pages.json
  ✓ Updated docs/src/mkdocs.yml
  ✓ Archived docs/src/configuration.md

📝 Next Steps:
  1. Review new structure in docs/src/configuration/
  2. Run drift detection on subpages
  3. Commit changes when satisfied
```

**Stage and Commit Changes (if not dry-run):**

If running in automation mode (PRODIGY_AUTOMATION=true) and not dry-run:

1. **Stage All Files:**
   - New directory and all subpage files
   - Updated pages.json
   - Updated mkdocs.yml
   - Backup of original file

2. **Create Commit:**
   ```
   docs: migrate {Page Title} to multi-subpage format

   Split {page-id}.md (1843 lines) into:
   - index.md with page overview
   - 8 subpages for better organization

   This improves navigability and allows focused drift detection
   per subpage instead of entire page.

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
- If source page file doesn't exist, error and exit
- If pages.json missing, error and exit
- If mkdocs.yml missing, error and exit

**Handle Migration Failures:**
- If directory creation fails, roll back and error
- If file write fails, roll back and error
- If validation fails, do not commit, report errors

**Handle Edge Cases:**
- Empty sections → skip or include in index.md
- Duplicate section titles → append numeric suffix
- Very large sections → warn but proceed
- No suitable sections for subpages → error (page should stay single-file)

**Rollback on Failure:**
- Restore original file from backup
- Delete newly created directory
- Revert pages.json changes
- Revert mkdocs.yml changes
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
  "min_subpages": 3,
  "max_subpages": 12,
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
- mddocs builds successfully
- pages.json updated correctly
- mkdocs.yml updated correctly
- No validation errors
- User can navigate new structure

## Scope Notes

This command does ONE thing: migrate a page structure. It does NOT:
- ❌ Analyze page sizes (use `/prodigy-analyze-page-structure`)
- ❌ Detect documentation gaps (use `/prodigy-detect-documentation-gaps`)
- ❌ Fix content drift (that happens in map phase after migration)

## Example Usage

```bash
# Preview migration
/prodigy-create-page-subpages \
  --project Debtmap \
  --page configuration \
  --pages workflows/data/prodigy-pages.json \
  --docs-dir docs \
  --dry-run

# Execute migration
/prodigy-create-page-subpages \
  --project Debtmap \
  --page configuration \
  --pages workflows/data/prodigy-pages.json \
  --docs-dir docs

# Use structure analysis recommendations
/prodigy-create-page-subpages \
  --project Debtmap \
  --page configuration \
  --pages workflows/data/prodigy-pages.json \
  --docs-dir docs \
  --structure-report .prodigy/docs-analysis/structure-report.json
```

## Integration with Workflow

This command should be run AFTER analyzing page structure:

1. `/prodigy-analyze-page-structure` - Identifies oversized pages
2. Review recommendations
3. **`/prodigy-create-page-subpages`** - Migrate each oversized page ← THIS COMMAND
4. `/prodigy-detect-documentation-gaps` - Regenerate flattened items with new structure
5. Run drift detection on new subpages
