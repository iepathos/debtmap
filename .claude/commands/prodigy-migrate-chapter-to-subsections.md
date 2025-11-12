# /prodigy-migrate-chapter-to-subsections

Migrate a single-file chapter to multi-subsection structure by analyzing existing subsection files and generating subsection definitions.

This command is used when a chapter has been split into subsections (via spec 157's organize_chapters.py script) but the chapters.json still references it as a single-file chapter.

## Variables

- `--chapter-id <id>` - Chapter ID to migrate (e.g., "mapreduce")
- `--chapters-file <path>` - Path to chapters JSON file (e.g., "workflows/data/prodigy-chapters.json")
- `--book-dir <path>` - Book directory path (e.g., "book")

## Execute

### Phase 1: Validate Input and Load Current State

**Parse Parameters:**
```bash
CHAPTER_ID="<value from --chapter-id parameter>"
CHAPTERS_FILE="<value from --chapters-file parameter>"
BOOK_DIR="<value from --book-dir parameter>"
```

**Load Current Chapters:**
Read the chapters JSON file to get current chapter definition.

**Find Target Chapter:**
Locate the chapter by ID in the chapters array.

**Validate Chapter State:**
- Check if chapter already has `type: "multi-subsection"` (skip if already migrated)
- Verify chapter currently has `type: "single-file"` or no type field
- Get chapter's current file path (e.g., "book/src/mapreduce.md")

### Phase 2: Check for Existing Subsection Directory

**Determine Subsection Directory:**
```bash
CHAPTER_DIR="${BOOK_DIR}/src/${CHAPTER_ID}/"
```

**Check Directory Exists:**
- Verify directory exists (e.g., "book/src/mapreduce/")
- If directory doesn't exist, migration not possible (chapter not split yet)
- List all .md files in directory

**Expected Structure:**
```
book/src/mapreduce/
├── index.md              # Chapter overview/introduction
├── checkpoint-and-resume.md
├── dead-letter-queue-dlq.md
├── environment-variables-in-configuration.md
└── ... (other subsection files)
```

### Phase 3: Analyze Existing Subsection Files

**Scan Directory for Subsection Files:**
For each .md file (excluding index.md):

1. **Extract Subsection ID:**
   - Remove .md extension
   - Use filename as subsection ID
   - Example: "checkpoint-and-resume.md" → "checkpoint-and-resume"

2. **Extract Subsection Title:**
   - Read markdown file
   - Find first H1 heading (# Title)
   - Use as subsection title
   - Example: "# Checkpoint and Resume" → "Checkpoint and Resume"
   - If no H1, convert ID to title case

3. **Extract Topics:**
   - Parse markdown headings (H2, H3)
   - Extract key topics from headings
   - Include terms from content analysis
   - Example: ["checkpoints", "resume", "state management", "recovery"]

4. **Determine Validation:**
   - Based on subsection content and title
   - Format: "Check [topic] documented"
   - Example: "Check checkpoint structure and resume behavior documented"

5. **Infer Feature Mapping (Heuristic):**
   - Match subsection topics against common feature patterns
   - Use chapter ID as prefix
   - Example: For "checkpoint-and-resume" in "mapreduce" chapter:
     - Feature mapping: ["mapreduce.checkpoint", "mapreduce.resume"]
   - This is best-effort; can be refined manually later

### Phase 4: Generate Subsection Definitions

**For Each Discovered Subsection File:**

Create subsection definition structure:
```json
{
  "id": "<subsection-id>",
  "title": "<subsection-title>",
  "file": "book/src/<chapter-id>/<subsection-id>.md",
  "topics": ["<topic-1>", "<topic-2>", ...],
  "validation": "<validation-criteria>",
  "feature_mapping": ["<inferred-feature-path>", ...]
}
```

**Maintain Logical Order:**
- Sort subsections alphabetically by ID
- Or maintain order based on file modification time (earliest first)
- Or use SUMMARY.md order if subsections already listed there

### Phase 5: Check for index.md

**Index File Handling:**
- Check if `${CHAPTER_DIR}/index.md` exists
- If exists, use as `index_file` for chapter
- If doesn't exist, create stub index.md:

```markdown
# {Chapter Title}

{Brief overview of this chapter and its subsections}

## Subsections

This chapter is organized into the following subsections:

- [Subsection 1](subsection-1.md)
- [Subsection 2](subsection-2.md)
...

## Overview

{High-level introduction to the chapter topic}
```

### Phase 6: Update Chapter Definition

**Transform Chapter:**
```json
// Before
{
  "id": "mapreduce",
  "title": "MapReduce Workflows",
  "type": "single-file",
  "file": "book/src/mapreduce.md",
  "topics": ["MapReduce mode", "Setup phase", "Map phase"],
  "validation": "Check MapReduce configuration"
}

// After
{
  "id": "mapreduce",
  "title": "MapReduce Workflows",
  "type": "multi-subsection",
  "index_file": "book/src/mapreduce/index.md",
  "subsections": [
    {
      "id": "checkpoint-and-resume",
      "title": "Checkpoint and Resume",
      "file": "book/src/mapreduce/checkpoint-and-resume.md",
      "topics": ["checkpoints", "resume", "recovery"],
      "validation": "Check checkpoint structure documented",
      "feature_mapping": ["mapreduce.checkpoint", "mapreduce.resume"]
    },
    ... (other subsections)
  ],
  "topics": ["MapReduce mode", "Setup phase", "Map phase"],
  "validation": "Check MapReduce configuration"
}
```

**Preserve Original Fields:**
- Keep original `topics` array
- Keep original `validation` string
- These provide chapter-level context

**Remove Old Fields:**
- Remove `file` field (replaced by `index_file`)

**Add New Fields:**
- `type: "multi-subsection"`
- `index_file: "book/src/{chapter-id}/index.md"`
- `subsections: [...]`

### Phase 7: Update SUMMARY.md (If Needed)

**Check Current SUMMARY.md:**
- Read book/src/SUMMARY.md
- Find chapter entry
- Check if subsections already listed

**If Subsections Not Listed:**
- Transform flat entry to nested structure:

```markdown
// Before
- [MapReduce Workflows](mapreduce.md)

// After
- [MapReduce Workflows](mapreduce/index.md)
  - [Checkpoint and Resume](mapreduce/checkpoint-and-resume.md)
  - [Dead Letter Queue](mapreduce/dead-letter-queue-dlq.md)
  ...
```

**If Subsections Already Listed:**
- Verify order matches generated subsection definitions
- Verify file paths are correct
- Update any inconsistencies

### Phase 8: Write Updated Files

**Write Updated chapters.json:**
- Update the chapter definition with new structure
- Write complete file back to disk
- Use proper JSON formatting (2-space indentation)

**Create index.md if Missing:**
- If index.md doesn't exist, create it with stub content
- Reference all subsections in the index

**Write Updated SUMMARY.md:**
- If changes needed, update SUMMARY.md
- Maintain proper indentation and structure

### Phase 9: Validation

**Verify Migration:**
- Chapter type changed to "multi-subsection"
- All subsection files have definitions
- index_file exists and is valid markdown
- SUMMARY.md has correct nested structure
- No subsection IDs conflict
- All file paths are correct

**Test mdBook Build:**
```bash
cd ${BOOK_DIR} && mdbook build
```
- Ensure book builds without errors
- Check for broken links
- Verify subsection pages render correctly

### Phase 10: Commit Changes

**Stage Files:**
```bash
git add ${CHAPTERS_FILE}
git add ${BOOK_DIR}/src/${CHAPTER_ID}/index.md  # if created
git add ${BOOK_DIR}/src/SUMMARY.md  # if modified
```

**Create Commit:**
```bash
git commit -m "docs: migrate ${CHAPTER_ID} chapter to multi-subsection structure

- Converted single-file chapter to multi-subsection format
- Generated ${SUBSECTION_COUNT} subsection definitions
- Created index.md for chapter overview
- Updated SUMMARY.md with nested structure

Subsections:
- ${SUBSECTION_ID_1}
- ${SUBSECTION_ID_2}
...

This enables subsection-level drift detection and targeted updates."
```

### Phase 11: Summary Output

**Display Migration Summary:**
```
✅ Migrated chapter '${CHAPTER_TITLE}' to multi-subsection structure

Chapter ID: ${CHAPTER_ID}
Subsections found: ${SUBSECTION_COUNT}
Index file: ${INDEX_FILE_STATUS}

Subsections:
  1. ${SUBSECTION_1_TITLE} (${SUBSECTION_1_ID})
  2. ${SUBSECTION_2_TITLE} (${SUBSECTION_2_ID)
  ...

Files updated:
  - ${CHAPTERS_FILE}
  - ${BOOK_DIR}/src/SUMMARY.md
  - ${BOOK_DIR}/src/${CHAPTER_ID}/index.md (created)

Next steps:
  - Review generated feature_mapping fields
  - Refine subsection topics if needed
  - Run drift detection workflow to validate subsections
```

## Error Handling

**Chapter Not Found:**
- Error: "Chapter '${CHAPTER_ID}' not found in ${CHAPTERS_FILE}"
- Suggest: List available chapter IDs

**Chapter Already Multi-Subsection:**
- Info: "Chapter '${CHAPTER_ID}' is already multi-subsection format"
- Skip migration, exit successfully

**Subsection Directory Not Found:**
- Error: "Subsection directory ${CHAPTER_DIR} does not exist"
- Suggest: "Run organize_chapters.py to split chapter first, or create subsection directory manually"

**No Subsection Files Found:**
- Error: "No subsection files found in ${CHAPTER_DIR}"
- Suggest: "Chapter may not need subsection structure, or files need to be created first"

**mdBook Build Fails:**
- Error: "mdBook build failed after migration"
- Action: Show build errors
- Suggest: "Fix broken links or markdown syntax errors"

## Notes

**Best Practices:**
- Run this after using organize_chapters.py to split large chapters
- Review generated feature_mapping and refine manually
- Test drift detection workflow after migration
- Keep single-file chapters for small chapters (<300 lines)

**Manual Refinement:**
After migration, you may want to manually:
- Adjust feature_mapping arrays for accuracy
- Refine topics lists based on actual content
- Update validation criteria for specificity
- Reorder subsections for logical flow

**When to Migrate:**
- Chapter has been split into multiple files by organize_chapters.py
- Chapter covers multiple distinct feature areas
- Chapter is >400 lines and has >6 H2 sections
- You want subsection-level drift detection and fixes
