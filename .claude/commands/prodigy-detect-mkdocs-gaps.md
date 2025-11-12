# /prodigy-detect-mkdocs-gaps

Detect documentation gaps by analyzing codebase features against existing MkDocs Material pages, then automatically create page definitions and stub markdown files for undocumented features.

## Variables

- `--project <name>` - Project name (e.g., "Prodigy")
- `--config <path>` - Path to MkDocs configuration JSON (e.g., ".prodigy/mkdocs-config.json")
- `--features <path>` - Path to features.json from setup phase (e.g., ".prodigy/mkdocs-analysis/features.json")
- `--chapters <path>` - Path to MkDocs page definitions JSON (e.g., "workflows/data/mkdocs-chapters.json")
- `--docs-dir <path>` - Documentation directory path (e.g., "docs")

## Execute

### Phase 1: Parse Parameters and Load Data

**Parse Command Arguments:**
Extract all required parameters from the command:
- `--project`: Project name for output messages
- `--config`: Path to MkDocs configuration
- `--features`: Path to features.json from setup phase
- `--chapters`: Path to mkdocs-chapters.json
- `--docs-dir`: Documentation directory path

**Load Configuration Files:**
Read the following files to understand current state:
1. Read `--features` file to get complete feature inventory from setup phase
2. Read `--chapters` file to get existing page definitions (hierarchical structure with sections)
3. Read `${docs-dir}/index.md` to understand navigation structure
4. Read `--config` file to get project settings

### Phase 2: Analyze Existing Documentation Coverage

**Build Documentation Map:**

The MkDocs chapters file has a hierarchical structure with sections and pages.

For each section in the chapters JSON:
1. Extract section ID and title
2. For each page in the section:
   - Extract page ID, title, file path, and topics
   - Read the page markdown file to understand documented content
   - Extract section headings and documented capabilities
   - Build a map: `{page_id: {title, topics, documented_features, section_id}}`

**Also process top-level pages** (pages not in sections, like index.md):
- These have `type: "single-file"` and appear directly in the pages array
- Add them to the documentation map with `section_id: null`

**Normalize Topic Names for Comparison:**

Create normalized versions of all documented topics:
- Convert to lowercase
- Remove punctuation and special characters
- Trim whitespace
- Extract key terms (e.g., "MapReduce Workflows" → "mapreduce", "workflows")

This helps match feature categories against documented topics accurately.

### Phase 3: Identify Documentation Gaps Using Hierarchy

**Compare Features Against Documentation Using Type and Structure:**

For each feature area in features.json:

**Step 1: Check Feature Type**
1. Read the `type` field from the feature
2. If `type: "meta"` → **SKIP** for page creation (meta-content goes in reference section)
3. If `type: "major_feature"` → Continue to step 2
4. If no type field → Assume major_feature for backward compatibility

**Step 2: Check for Existing Page**
1. Extract the feature category name (the JSON key, e.g., "authentication", "data_processing")
2. Normalize the name (lowercase, remove underscores/hyphens)
3. Check if ANY existing page matches:
   - Page ID matches (e.g., "authentication" page for authentication feature)
   - Page title contains feature name (fuzzy match)
   - Page topics include feature name

**Step 3: Determine Gap Type**
- If no page found → **High severity gap** (missing page)
- If page found → Check for content completeness (handled by drift detection in map phase)

**Step 4: Check for Section-Level Gaps**
1. For features with multiple related capabilities (3+ items), check if appropriate section exists
2. If section exists but missing pages for specific capabilities → **Medium severity gap**
3. Example: "mapreduce" section exists with "overview" page but missing "checkpoint-resume" page

**Use Hierarchy and Type to Classify Gaps:**

**High Severity (Missing Major Feature Page):**
- Feature has `type: "major_feature"` in features.json
- No corresponding page found in any section
- Should create a new page in appropriate section
- Example: "authentication" is major_feature but no page exists

**Medium Severity (Missing Page in Section):**
- Parent section exists (e.g., "mapreduce" section)
- Feature capability is not documented as a page
- Should create new page in the section
- Example: "mapreduce" section exists but "checkpoint-resume" page missing

**Low Severity (Content Gap - Not a Structure Issue):**
- Page exists but may have outdated content
- Will be handled by drift detection in map phase
- Don't create new pages for this
- Example: "overview" page exists but missing new checkpoint features

**Generate Gap Report:**

Create a structured JSON report documenting all gaps found:
```json
{
  "analysis_date": "<current-timestamp>",
  "features_analyzed": <total-feature-areas>,
  "documented_pages": <count-of-pages>,
  "gaps_found": <count-of-gaps>,
  "gaps": [
    {
      "severity": "high|medium|low",
      "type": "missing_page|missing_section|incomplete_page",
      "feature_category": "<feature-area-name>",
      "feature_description": "<brief-description>",
      "recommended_page_id": "<page-id>",
      "recommended_title": "<page-title>",
      "recommended_file": "<file-path>",
      "recommended_section": "<section-id>",
      "is_new_section": true|false
    }
  ],
  "actions_taken": []
}
```

### Phase 4: Generate Page Definitions for Missing Pages

**For Each High Severity Gap (Missing Page):**

1. **Determine Target Section:**
   - Based on feature category, decide which section it belongs to
   - Example: "checkpoint" feature → "mapreduce" section
   - Example: "environment" feature → "configuration" section
   - If no appropriate section exists, create page in new section or as top-level page

2. **Generate Page ID:**
   - Convert feature category to kebab-case
   - Example: "checkpoint_resume" → "checkpoint-resume"
   - Example: "command_types" → "command-types"
   - Ensure uniqueness within section

3. **Generate Page Title:**
   - Convert to title case with spaces
   - Example: "checkpoint_resume" → "Checkpoint and Resume"
   - Example: "command_types" → "Command Types"

4. **Determine File Path:**
   - Pattern: `${docs-dir}/${section-id}/${page-id}.md`
   - For top-level pages: `${docs-dir}/${page-id}.md`
   - Example: "docs/mapreduce/checkpoint-resume.md"
   - Example: "docs/getting-started.md"

5. **Extract Topics from Features:**
   - Look at the feature capabilities in features.json
   - Convert capabilities to topic names
   - Example: For "checkpoint_resume" with capabilities ["checkpoint", "resume", "state"]
   - Topics: ["Checkpoint creation", "Resume behavior", "State management"]

6. **Define Validation Criteria:**
   - Create validation string based on feature type
   - Example: "Check that checkpoint structure and resume behavior are documented"
   - Include references to relevant structs or configs

7. **Define Feature Mapping:**
   - List specific feature paths this page should document
   - Use the JSON path from features.json
   - Example: `["mapreduce.checkpoint", "mapreduce.resume"]`
   - This enables focused drift detection in map phase

8. **Create Page Definition Structure:**
```json
{
  "id": "<page-id>",
  "title": "<page-title>",
  "file": "<file-path>",
  "topics": ["<topic-1>", "<topic-2>", ...],
  "validation": "<validation-criteria>",
  "feature_mapping": ["<feature-path-1>", "<feature-path-2>", ...],
  "auto_generated": true,
  "source_feature": "<feature-category>"
}
```

### Phase 5: Update Page Definitions File and Generate Flattened Output

**Read Existing Pages Structure:**
Load the current contents of the chapters JSON file specified by `--chapters` parameter.
Note: This file has a hierarchical structure with sections containing pages arrays.

**For New Pages:**

**Check for Duplicates:**
- Verify the page ID doesn't already exist in any section
- Check that the file path isn't already in use
- Normalize and compare titles to avoid near-duplicates

**Append New Pages:**
- Find the appropriate section by ID
- If section doesn't exist, create it with `type: "section"` and `pages: []` array
- Add new page definition to the section's `pages` array
- For top-level pages (like index), add to root `pages` array

**Record Action:**
```json
{
  "action": "created_page_definition",
  "page_id": "<page-id>",
  "section_id": "<section-id>",
  "file_path": "<chapters-file-path from --chapters parameter>"
}
```

**Write Updated Page Definitions:**
Write the complete pages JSON back to disk with proper formatting (if any gaps were found):
- Use 2-space indentation
- Maintain hierarchical structure (sections with pages)
- Preserve existing sections and pages
- Keep page order within sections

**Note**: The flattened-items.json generation has moved to Phase 7 to ensure it always executes.

### Phase 6: Create Stub Markdown Files

**For Each New Page:**

1. **Determine Stub Content:**
   Generate markdown following this minimal template structure:

```markdown
# {Page Title}

{Brief introduction explaining the purpose of this feature/capability}

## Overview

{High-level description of what this feature enables}

## Configuration

{If applicable, configuration options and syntax}

```yaml
# Example configuration
```

## Usage

{Basic usage examples}

## See Also

- [Related documentation](link)
```

**Note**: Do NOT include Prerequisites, Installation, Best Practices, or Troubleshooting sections in page stubs. These belong in dedicated reference pages.

2. **Customize Content for Feature:**
   - Use page title from definition
   - Reference the feature category from features.json
   - Include relevant configuration examples
   - Add placeholders for sections

3. **Create File:**
   - Write stub markdown to the file path defined in page definition
   - Ensure parent directory exists (section directory may need to be created)
   - Use proper markdown formatting

4. **Validate Markdown:**
   - Ensure the file is valid markdown
   - Check that it won't break mkdocs build
   - Verify all syntax is correct

5. **Record Action:**
```json
{
  "action": "created_stub_file",
  "file_path": "<file-path>",
  "type": "page",
  "section_id": "<section-id>"
}
```

### Phase 7: Save Gap Report, Generate Flattened Items, and Commit Changes

**STEP 1: Generate Flattened Items for Map Phase (MANDATORY)**

This step MUST execute regardless of whether gaps were found:

1. Read the pages file from `--chapters` parameter
2. Process the hierarchical structure to create flattened array:
   - For pages in sections: Extract each page with section metadata
   - For top-level pages: Include page with type marker
3. Determine output path:
   - Write to: `.prodigy/mkdocs-analysis/flattened-items.json`

Example structure:
```json
[
  {
    "id": "index",
    "title": "Home",
    "file": "docs/index.md",
    "topics": [...],
    "validation": "...",
    "type": "single-file"
  },
  {
    "id": "checkpoint-resume",
    "title": "Checkpoint and Resume",
    "file": "docs/mapreduce/checkpoint-resume.md",
    "section_id": "mapreduce",
    "section_title": "MapReduce Workflows",
    "type": "section-page",
    "topics": [...],
    "feature_mapping": [...]
  }
]
```

**STEP 2: Write Gap Report**

Save the gap report to disk for auditing:
- Path: `.prodigy/mkdocs-analysis/gap-report.json`
- Include all gaps found and actions taken
- Use proper JSON formatting

**STEP 3: Display Summary to User**

Print a formatted summary:
```
📊 MkDocs Documentation Gap Analysis
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Features Analyzed: {count}
Documented Pages: {count}
Gaps Found: {count}

🔴 High Severity Gaps (Missing Pages):
  • {feature_category} - {description}

🟡 Medium Severity Gaps (Incomplete Sections):
  • {section_id} - Missing: {missing_pages}

✅ Actions Taken:
  ✓ Generated flattened-items.json for map phase
  ✓ Created {count} page definitions (if gaps found)
  ✓ Created {count} stub files (if gaps found)

📝 Next Steps:
  The map phase will now process these pages to detect drift.
```

**Stage and Commit Changes:**

**CRITICAL**: The flattened-items.json file MUST be committed, as the map phase depends on it.

If running in automation mode (PRODIGY_AUTOMATION=true):

**If gaps were found:**
1. Stage all modified files:
   - Updated mkdocs-chapters.json file (from --chapters parameter)
   - New stub markdown files
   - Gap report
   - **flattened-items.json (REQUIRED)**

2. Create commit with message:
   - Format: "docs: auto-discover missing pages for [feature names]"
   - Example: "docs: auto-discover missing pages for checkpoint-resume, dlq"
   - Include brief summary of actions taken

**If NO gaps were found (still need to commit flattened-items.json):**
1. Stage generated files:
   - flattened-items.json (REQUIRED for map phase)
   - Gap report (shows 0 gaps found)

2. Create commit with message:
   - Format: "docs: regenerate flattened-items.json for drift detection (no gaps found)"
   - Include count of pages to be processed

### Phase 8: Validation and Quality Checks

**Verify No False Positives:**
- Check that no duplicate pages were created
- Ensure existing pages weren't unnecessarily modified
- Verify page IDs are unique within sections

**Verify No False Negatives:**
- Check that all obvious undocumented features were detected
- Compare feature areas against documented topics
- Ensure classification (high/medium/low) is appropriate

**Test MkDocs Build:**
Run mkdocs build to ensure:
- All new stub files are valid markdown
- Navigation structure is correct
- No broken links created
- Documentation compiles successfully

If build fails:
- Identify the issue
- Fix the problematic file(s)
- Re-run build validation

### Phase 9: Idempotence Check

**Design for Repeated Execution:**
Gap detection should be idempotent:
- Running it multiple times should not create duplicates
- Already-created pages should be recognized
- No unnecessary modifications

**Implementation:**
- Always check for existing pages before creating new ones
- Use normalized comparison for topic matching
- Skip pages that already exist with the same ID
- Only create pages for truly missing features

**Validation:**
If gap detection runs and finds no gaps:
- Print message: "✅ No documentation gaps found - all features are documented"
- Do not modify page definitions file
- **IMPORTANT**: Still generate flattened-items.json from existing pages for map phase
- Exit successfully

**CRITICAL**: The flattened-items.json file must ALWAYS be generated, even when no gaps are found. This file is required by the map phase to process all pages for drift detection. Generate it from the existing mkdocs-chapters.json file in Phase 7, regardless of whether gaps were detected.

### Error Handling

**Handle Missing Files Gracefully:**
- If features.json doesn't exist → error: "Feature analysis must run first"
- If mkdocs-chapters.json doesn't exist → create empty structure
- If index.md doesn't exist → error: "Documentation structure missing"
- If config file missing → use sensible defaults

**Handle Invalid JSON:**
- Validate JSON structure before parsing
- Provide clear error messages for malformed files
- Don't proceed with gap detection if data is invalid

**Handle File Write Failures:**
- Check if docs/ directory exists and is writable
- Verify permissions before writing files
- Roll back changes if commits fail

**Record Failures:**
Include in gap report if any steps fail:
```json
{
  "errors": [
    {
      "phase": "stub_creation",
      "error": "Failed to write file: permission denied",
      "file_path": "docs/mapreduce/checkpoint-resume.md"
    }
  ]
}
```

### Quality Guidelines

**Accuracy:**
- Minimize false positives (no duplicate pages)
- Minimize false negatives (catch all undocumented features)
- Use fuzzy matching for topic comparison
- Consider synonyms and variations

**User Experience:**
- Provide clear, actionable output
- Show progress during analysis
- Summarize actions taken
- Explain what will happen next

**Maintainability:**
- Use configurable thresholds for gap classification
- Support customization via mkdocs-config.json
- Make template structure configurable
- Keep logic modular and testable

**Performance:**
- Complete analysis in <30 seconds for typical projects
- Minimize file I/O operations
- Cache parsed markdown content
- Process pages in parallel if needed

### Success Indicators

Gap detection is successful when:
- All undocumented features are identified
- New page definitions are valid and complete
- Stub markdown files are properly formatted
- Navigation structure is maintained
- MkDocs builds without errors
- No duplicate pages created
- Changes are committed cleanly
- **`.prodigy/mkdocs-analysis/flattened-items.json` file is created** (REQUIRED)

## FINAL CHECKLIST

Before completing this command, verify:

1. ✅ Gap report saved to `.prodigy/mkdocs-analysis/gap-report.json`
2. ✅ **`.prodigy/mkdocs-analysis/flattened-items.json` created (MANDATORY - even if no gaps found)**
3. ✅ Page definitions updated in mkdocs-chapters.json (if gaps found)
4. ✅ Stub files created in docs/ (if gaps found)
5. ✅ Changes committed (if any files modified)

**CRITICAL**: Step 2 (flattened-items.json) is REQUIRED for the workflow to proceed to the map phase. This file must contain all pages in a flat array format, ready for parallel processing.
