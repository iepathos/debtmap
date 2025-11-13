# /prodigy-detect-documentation-gaps

Detect documentation gaps by analyzing codebase features against existing book chapters. This command focuses on **feature coverage** - ensuring all major features have documentation. It does NOT analyze chapter sizes or create subsections (see `/prodigy-analyze-chapter-structure` for that).

## Variables

- `--project <name>` - Project name (e.g., "Debtmap")
- `--config <path>` - Path to book configuration JSON (e.g., ".prodigy/book-config.json")
- `--features <path>` - Path to features.json from setup phase (e.g., ".prodigy/book-analysis/features.json")
- `--chapters <path>` - Path to chapter definitions JSON (e.g., "workflows/data/prodigy-chapters.json")
- `--book-dir <path>` - Book directory path (e.g., "book")

## Execute

### Phase 1: Parse Parameters and Load Data

**Parse Command Arguments:**
Extract all required parameters from the command:
- `--project`: Project name for output messages
- `--config`: Path to book configuration
- `--features`: Path to features.json from setup phase
- `--chapters`: Path to chapter definitions JSON
- `--book-dir`: Book directory path

**Load Configuration Files:**
Read the following files to understand current state:
1. Read `--features` file to get complete feature inventory from setup phase
2. Read `--chapters` file to get existing chapter definitions
3. Read `${book-dir}/src/SUMMARY.md` to get book structure
4. Read `--config` file to get project settings

### Phase 2: Analyze Existing Documentation Coverage

**Build Documentation Map:**

For each chapter in the chapters JSON file:
1. Extract the chapter ID, title, file path, and topics
2. Check if chapter file exists (handle both single-file and multi-subsection chapters)
3. Build a map: `{chapter_id: {title, topics, type}}`

**Normalize Topic Names for Comparison:**

Create normalized versions of all documented topics:
- Convert to lowercase
- Remove punctuation and special characters
- Trim whitespace
- Extract key terms (e.g., "MapReduce Workflows" → "mapreduce", "workflows")

This helps match feature categories against documented topics accurately.

### Phase 3: Identify Missing Feature Documentation

**Compare Features Against Documentation:**

For each feature area in features.json:

**Step 1: Check Feature Type**
1. Read the `type` field from the feature
2. If `type: "meta"` → **SKIP** - Meta-content should not have chapters
3. If `type: "major_feature"` → Continue to step 2
4. If no type field → Assume major_feature for backward compatibility

**Step 2: Check for Existing Chapter**
1. Extract the feature category name (the JSON key, e.g., "authentication", "data_processing")
2. Normalize the name (lowercase, remove underscores/hyphens)
3. Check if ANY existing chapter matches:
   - Chapter ID matches (e.g., "authentication" chapter for authentication feature)
   - Chapter title contains feature name (fuzzy match with 0.7 threshold)
   - Chapter topics include feature name

**Step 3: Classify Gap**
- If no chapter found → **High severity gap** (missing chapter for major feature)
- If chapter found → No gap (content drift will be handled by map phase)

**Generate Gap Report:**

Create a structured JSON report documenting all gaps found:
```json
{
  "analysis_date": "<current-timestamp>",
  "features_analyzed": <total-feature-areas>,
  "documented_topics": <count-of-chapters>,
  "gaps_found": <count-of-missing-chapters>,
  "gaps": [
    {
      "severity": "high",
      "type": "missing_chapter",
      "feature_category": "<feature-area-name>",
      "feature_description": "<brief-description>",
      "recommended_chapter_id": "<chapter-id>",
      "recommended_title": "<chapter-title>",
      "recommended_location": "<file-path>"
    }
  ],
  "actions_taken": []
}
```

### Phase 4: Generate Chapter Definitions for Missing Features

**For Each High Severity Gap (Missing Chapter):**

1. **Generate Chapter ID:**
   - Convert feature category to kebab-case
   - Example: "agent_merge" → "agent-merge-workflows"
   - Example: "circuit_breaker" → "circuit-breaker"
   - Ensure uniqueness against existing chapter IDs

2. **Generate Chapter Title:**
   - Convert to title case with spaces
   - Add descriptive suffix if needed
   - Example: "agent_merge" → "Agent Merge Workflows"
   - Example: "circuit_breaker" → "Circuit Breaker"

3. **Determine File Path:**
   - Use book_src from config (typically "book/src")
   - Create filename from chapter ID
   - Format: `${book_src}/${chapter_id}.md`
   - Example: "book/src/agent-merge-workflows.md"

4. **Extract Topics from Features:**
   - Look at the feature capabilities in features.json
   - Convert capabilities to topic names
   - Example: For "agent_merge" feature with capabilities ["validation", "merge_config", "error_handling"]
   - Topics: ["Agent merge configuration", "Merge validation", "Error handling in merges"]

5. **Define Validation Criteria:**
   - Create validation string based on feature type
   - Example: "Check that agent_merge syntax and variables are documented"
   - Include references to relevant structs or configs

6. **Create Chapter Definition Structure:**
```json
{
  "id": "<chapter-id>",
  "title": "<chapter-title>",
  "file": "<file-path>",
  "topics": ["<topic-1>", "<topic-2>", ...],
  "validation": "<validation-criteria>",
  "auto_generated": true,
  "source_feature": "<feature-category>"
}
```

### Phase 5: Update Chapter Definitions File

**Read Existing Chapters:**
Load the current contents of the chapters JSON file specified by `--chapters` parameter

**For New Chapters:**

**Check for Duplicates:**
- Verify the chapter ID doesn't already exist
- Check that the file path isn't already in use
- Normalize and compare titles to avoid near-duplicates

**Append New Chapters:**
- Add new chapter definitions to the chapters array
- Maintain logical ordering (by section: User Guide, Advanced Topics, Reference)

**Record Action:**
```json
{
  "action": "created_chapter_definition",
  "chapter_id": "<chapter-id>",
  "file_path": "<chapters-file-path from --chapters parameter>"
}
```

**Write Updated Chapter Definitions:**
Write the complete chapters JSON back to disk with proper formatting (only if gaps were found):
- Use 2-space indentation
- Maintain JSON structure
- Preserve existing chapters
- Keep logical order

### Phase 6: Create Stub Markdown Files

**For Each New Chapter:**

1. **Determine Stub Content:**
   Generate markdown following this minimal template structure:

```markdown
# {Chapter Title}

{Brief introduction explaining the purpose of this feature/capability}

## Overview

{High-level description of what this feature enables}

## Configuration

{If applicable, configuration options and syntax}

\`\`\`yaml
# Example configuration
\`\`\`

## Usage

{Basic usage examples}

## See Also

- [Related documentation](link)
```

**Note**: Do NOT include Prerequisites, Installation, Best Practices, or Troubleshooting sections in chapter stubs. These belong in dedicated root-level files.

2. **Customize Content for Feature:**
   - Use chapter title from definition
   - Reference the feature category from features.json
   - Include relevant configuration examples
   - Add placeholders for sections

3. **Create File:**
   - Write stub markdown to the file path defined in chapter definition
   - Ensure directory exists (book/src should already exist)
   - Use proper markdown formatting

4. **Validate Markdown:**
   - Ensure the file is valid markdown
   - Check that it won't break mdbook build
   - Verify all syntax is correct

5. **Record Action:**
```json
{
  "action": "created_stub_file",
  "file_path": "<file-path>",
  "type": "chapter"
}
```

### Phase 7: Update SUMMARY.md

**Read Current SUMMARY.md:**
Load the book's SUMMARY.md file to understand structure

**Parse Structure:**
Identify sections:
- Introduction (always at top)
- User Guide (basic features)
- Advanced Topics (complex features)
- Reference (examples, troubleshooting)

**For New Chapters:**

1. **Classify New Chapters:**
   - Basic workflow features → User Guide
   - Advanced features (retry, error handling, composition) → Advanced Topics
   - Examples and troubleshooting → Reference

2. **Determine Insertion Point:**
   - Maintain alphabetical order by title
   - Or maintain logical order based on dependencies
   - Insert after similar topics

3. **Insert Chapter Entries:**
   Add entries in markdown list format:
   ```markdown
   - [Chapter Title](chapter-file.md)
   ```

**Write Updated SUMMARY.md:**
Write the modified SUMMARY.md back to disk

**Record Action:**
```json
{
  "action": "updated_summary",
  "file_path": "book/src/SUMMARY.md",
  "items_added": [
    {"type": "chapter", "id": "..."}
  ]
}
```

### Phase 8: Generate Flattened Items and Save Gap Report

**STEP 1: Generate Flattened Items for Map Phase (MANDATORY)**

This step MUST execute regardless of whether gaps were found:

1. Read the chapters file from `--chapters` parameter
2. Process each chapter to create flattened array:
   - For `type == "multi-subsection"`: Extract each subsection with parent metadata
   - For `type == "single-file"` or no type: Include chapter with type marker
3. Determine output path from config:
   - Extract `book_dir` from `--config` parameter (default to "book")
   - Extract `project` name from `--project` parameter
   - Create analysis directory: `.${project_lowercase}/book-analysis/`
   - Write to `${analysis_dir}/flattened-items.json`

Example structure:
```json
[
  {
    "id": "authentication",
    "title": "Authentication",
    "file": "book/src/authentication.md",
    "topics": [...],
    "validation": "...",
    "type": "single-file"
  },
  {
    "id": "batch-operations",
    "title": "Batch Operations",
    "file": "book/src/data-processing/batch-operations.md",
    "parent_chapter_id": "data-processing",
    "parent_chapter_title": "Data Processing",
    "type": "subsection",
    "topics": [...],
    "feature_mapping": [...]
  }
]
```

**STEP 2: Write Gap Report**

Save the gap report to disk for auditing:
- Use same analysis directory as flattened-items.json
- Path: `${analysis_dir}/gap-report.json`
- Include all gaps found and actions taken
- Use proper JSON formatting

**STEP 3: Display Summary to User**

Print a formatted summary:
```
📊 Documentation Gap Analysis
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Features Analyzed: {count}
Documented Topics: {count}
Gaps Found: {count}

🔴 High Severity Gaps (Missing Chapters):
  • {feature_category} - {description}

✅ Actions Taken:
  ✓ Generated flattened-items.json for map phase ({count} items)
  ✓ Created {count} chapter definitions (if gaps found)
  ✓ Created {count} stub files (if gaps found)
  ✓ Updated SUMMARY.md (if gaps found)

📝 Next Steps:
  The map phase will process all {count} chapters to detect drift.
  Run /prodigy-analyze-chapter-structure to check for oversized chapters.
```

**Stage and Commit Changes:**

**CRITICAL**: The flattened-items.json file MUST be committed, as the map phase depends on it.

If running in automation mode (PRODIGY_AUTOMATION=true):

**If gaps were found:**
1. Stage all modified files:
   - Updated chapters.json file (from --chapters parameter)
   - New stub markdown files
   - Updated SUMMARY.md
   - Gap report
   - **flattened-items.json (REQUIRED)**

2. Create commit with message:
   - Format: "docs: add missing chapters for [feature names]"
   - Example: "docs: add missing chapters for authentication, rate-limiting"
   - Include brief summary of actions taken

**If NO gaps were found (still need to commit flattened-items.json):**
1. Stage generated files:
   - flattened-items.json (REQUIRED for map phase)
   - Gap report (shows 0 gaps found)

2. Create commit with message:
   - Format: "docs: regenerate flattened-items.json for drift detection (no gaps found)"
   - Body: "Generated flattened items array containing all {count} chapters for map phase processing.\nAll major feature areas are fully documented - no documentation gaps detected."

### Phase 9: Validation and Quality Checks

**Verify No False Positives:**
- Check that no duplicate chapters were created
- Ensure existing chapters weren't unnecessarily modified
- Verify chapter IDs are unique

**Verify No False Negatives:**
- Check that all obvious undocumented features were detected
- Compare feature areas against documented topics
- Ensure classification is appropriate

**Test Book Build:**
Run mdbook build to ensure:
- All new stub files are valid markdown
- SUMMARY.md references are correct
- No broken links created
- Book compiles successfully

If build fails:
- Identify the issue
- Fix the problematic file(s)
- Re-run build validation

### Phase 10: Idempotence Check

**Design for Repeated Execution:**
Gap detection should be idempotent:
- Running it multiple times should not create duplicates
- Already-created chapters should be recognized
- No unnecessary modifications

**Implementation:**
- Always check for existing chapters before creating new ones
- Use normalized comparison for topic matching
- Skip chapters that already exist with the same ID
- Only create chapters for truly missing features

**Validation:**
If gap detection runs and finds no gaps:
- Print message: "✅ No documentation gaps found - all features are documented"
- Do not modify chapter definitions file
- **IMPORTANT**: Still generate flattened-items.json from existing chapters for map phase
- Exit successfully

**CRITICAL**: The flattened-items.json file must ALWAYS be generated, even when no gaps are found. This file is required by the map phase to process all chapters for drift detection.

### Error Handling

**Handle Missing Files Gracefully:**
- If features.json doesn't exist → error: "Feature analysis must run first"
- If chapters.json doesn't exist → create empty structure
- If SUMMARY.md doesn't exist → error: "Book structure missing"
- If config file missing → use sensible defaults

**Handle Invalid JSON:**
- Validate JSON structure before parsing
- Provide clear error messages for malformed files
- Don't proceed with gap detection if data is invalid

**Handle File Write Failures:**
- Check if book/src directory exists and is writable
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
      "file_path": "book/src/agent-merge-workflows.md"
    }
  ]
}
```

### Quality Guidelines

**Accuracy:**
- Minimize false positives (no duplicate chapters)
- Minimize false negatives (catch all undocumented features)
- Use fuzzy matching for topic comparison (0.7 threshold)
- Consider synonyms and variations

**User Experience:**
- Provide clear, actionable output
- Show progress during analysis
- Summarize actions taken
- Explain what will happen next

**Maintainability:**
- Use configurable thresholds for gap classification
- Support customization via book-config.json
- Make template structure configurable
- Keep logic modular and testable

**Performance:**
- Complete analysis in <30 seconds for typical projects
- Minimize file I/O operations
- Cache parsed markdown content

## FINAL CHECKLIST

Before completing this command, verify:

1. ✅ Gap report saved to `${analysis_dir}/gap-report.json`
2. ✅ **`${analysis_dir}/flattened-items.json` created (MANDATORY - even if no gaps found)**
3. ✅ Chapter definitions updated in chapters file (if gaps found)
4. ✅ Stub files created in book/src/ (if gaps found)
5. ✅ SUMMARY.md updated (if gaps found)
6. ✅ Changes committed (if any files modified)

**CRITICAL**: Step 2 (flattened-items.json) is REQUIRED for the workflow to proceed to the map phase. This file must contain all chapters and subsections in a flat array format, ready for parallel processing.

## Scope Notes

This command is **deliberately focused** on feature coverage only. It does NOT:
- ❌ Analyze existing chapters for size/complexity
- ❌ Create subsections for oversized chapters
- ❌ Migrate single-file chapters to multi-subsection format
- ❌ Analyze chapter content organization

For those tasks, use:
- `/prodigy-analyze-chapter-structure` - Analyze chapter sizes and recommend subsections
- `/prodigy-create-chapter-subsections` - Migrate chapters to subsection format
