# /prodigy-detect-documentation-gaps

Detect documentation gaps by analyzing the codebase features against existing book chapters, then automatically create chapter definitions and stub markdown files for undocumented features.

## Variables

- `--project <name>` - Project name (e.g., "Prodigy")
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
- `--chapters`: Path to prodigy-chapters.json
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
2. Read the chapter markdown file to understand documented content
3. Extract section headings and documented capabilities
4. Build a map: `{chapter_id: {title, topics, documented_features}}`

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
2. If `type: "meta"` → **SKIP** - Meta-content should not have chapters
3. If `type: "major_feature"` → Continue to step 2
4. If no type field → Assume major_feature for backward compatibility

**Step 2: Check for Existing Chapter**
1. Extract the feature category name (the JSON key, e.g., "authentication", "data_processing", "api_endpoints")
2. Normalize the name (lowercase, remove underscores/hyphens)
3. Check if ANY existing chapter matches:
   - Chapter ID matches (e.g., "authentication" chapter for authentication feature)
   - Chapter title contains feature name (fuzzy match)
   - Chapter topics include feature name

**Step 3: Determine Gap Type**
- If no chapter found → **High severity gap** (missing chapter)
- If chapter found → Check for subsection gaps (step 4)

**Step 4: Check for Subsection Gaps (Only for Multi-Subsection Chapters)**
1. Count second-level items in the feature structure
   - Example: If feature has nested objects like `phases` and `core_capabilities`, count total items across all nested groups
2. If feature has 5+ second-level items AND chapter exists as `type: "multi-subsection"`:
   - For each second-level item, check if corresponding subsection exists
   - If subsection missing → **Medium severity gap** (missing subsection)
3. If feature has < 5 second-level items → No subsection gaps (document in single-file)
4. If chapter is `type: "single-file"` → No subsection gaps (preserve structure)

**Use Hierarchy and Type to Classify Gaps:**

**High Severity (Missing Major Feature Chapter):**
- Feature has `type: "major_feature"` in features.json
- No corresponding chapter found in chapters.json
- Should create a new single-file chapter
- Example: "authentication" is major_feature but no chapter exists

**Medium Severity (Missing Subsection in Multi-Subsection Chapter):**
- Parent feature is documented with multi-subsection chapter
- Parent has 5+ second-level capabilities in features.json
- Specific second-level capability is not documented as subsection
- Should create new subsection
- Example: "data_processing" chapter exists with subsections, but "batch_operations" subsection missing

**Low Severity (Content Gap - Not a Structure Issue):**
- Chapter exists but may have outdated content
- Will be handled by drift detection in map phase
- Don't create new chapters/subsections for this
- Example: "api_endpoints" chapter exists but missing new "pagination" feature details

**Generate Gap Report:**

Create a structured JSON report documenting all gaps found:
```json
{
  "analysis_date": "<current-timestamp>",
  "features_analyzed": <total-feature-areas>,
  "documented_topics": <count-of-chapters-and-subsections>,
  "gaps_found": <count-of-gaps>,
  "gaps": [
    {
      "severity": "high|medium|low",
      "type": "missing_chapter|missing_subsection|incomplete_chapter|incomplete_subsection",
      "feature_category": "<feature-area-name>",
      "feature_description": "<brief-description>",
      "recommended_chapter_id": "<chapter-id>",
      "recommended_title": "<chapter-title>",
      "recommended_location": "<file-path>",
      "parent_chapter_id": "<parent-id-if-subsection>",
      "is_subsection": true|false
    }
  ],
  "actions_taken": []
}
```

### Phase 4: Generate Chapter Definitions for Missing Chapters

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

### Phase 4a: Determine Subsection Creation Using Hierarchy

**For Each Medium Severity Gap (Missing Subsection in Existing Chapter):**

Use the hierarchical features.json structure to determine if a subsection should be created.

**STEP 1: Check Feature Type - Skip Meta Content**

1. Read the feature from features.json
2. Check if `type: "meta"` → **SKIP entirely, never create chapters/subsections**
3. If `type: "major_feature"` → Continue evaluation

**STEP 2: Evaluate Subsection Necessity Using Structure**

Count second-level capabilities under the major feature:

**For features with nested structure:**
- Count total second-level items across all nested groups
- Example: If feature has nested objects like `phases` (3 items) and `core_capabilities` (3 items) = 6 total second-level items

**Subsection Creation Rules:**
- **5+ second-level capabilities** → Consider multi-subsection structure
- **3-4 second-level capabilities** → Keep as single-file chapter with H2 sections
- **1-2 second-level capabilities** → Definitely single-file, document inline

**STEP 3: Check Existing Chapter Structure**

Before creating subsections:
1. Check if chapter already exists in chapters.json
2. If exists and is `type: "single-file"` → **Preserve it**, don't fragment
3. If exists and is `type: "multi-subsection"` → OK to add subsections
4. If doesn't exist → Create as single-file by default

**STEP 4: Prevent Meta-Subsection Creation**

**NEVER create these as separate subsections:**
- "Best Practices"
- "Troubleshooting"
- "Common Patterns"
- "Examples"

These should be H2 sections within chapter files, not separate subsections.

**Rationale:**
- Meta-content applies across features, not isolated to one area
- Creates navigation confusion (mixes "what" with "how")
- Better as sections in parent chapter or root-level guides

**STEP 5: Conservative Subsection Creation**

Only create subsections when ALL conditions met:
1. Parent feature has `type: "major_feature"`
2. Parent has 5+ second-level capabilities
3. Subsection represents a distinct capability (e.g., "checkpoint_resume", "dlq")
4. Subsection is NOT meta-content
5. Parent chapter is already `type: "multi-subsection"` OR doesn't exist yet

**STEP 6: Generate Subsection Definition**

Only if all conditions in STEP 5 are met:

1. **Generate Subsection ID:**
   - Convert feature category to kebab-case
   - Example: "batch_operations" → "batch-operations"
   - Example: "rate_limiting" → "rate-limiting"
   - Ensure uniqueness within chapter's subsections

2. **Generate Subsection Title:**
   - Convert to title case with spaces
   - Example: "batch_operations" → "Batch Operations"
   - Example: "rate_limiting" → "Rate Limiting"

3. **Determine Subsection File Path:**
   - Use pattern: `${book_src}/${parent_chapter_id}/${subsection_id}.md`
   - Example: `${book_src}/data-processing/batch-operations.md`
   - Ensure parent directory exists

4. **Extract Topics from Feature Description:**
   - Look at the feature's description and capabilities in features.json
   - Convert to topic names relevant to subsection
   - Example: For "batch_operations" with features ["parallel_processing", "error_recovery"]
   - Topics: ["Parallel processing", "Error recovery", "Status tracking"]

5. **Define Feature Mapping:**
   - List specific feature paths this subsection should document
   - Use the JSON path from features.json
   - Example: `["data_processing.core_capabilities.batch_operations"]`
   - This enables focused drift detection in map phase

6. **Define Validation Criteria:**
   - Create validation string based on subsection focus
   - Example: "Check that batch operations and error recovery are documented with examples"
   - Reference the feature's capabilities

7. **Create Subsection Definition Structure:**
```json
{
  "id": "<subsection-id>",
  "title": "<subsection-title>",
  "file": "<subsection-file-path>",
  "topics": ["<topic-1>", "<topic-2>", ...],
  "validation": "<validation-criteria>",
  "feature_mapping": ["<feature-path-1>", "<feature-path-2>", ...],
  "auto_generated": true,
  "source_feature": "<feature-category>"
}
```

### Phase 5: Update Chapter Definitions File and Generate Flattened Output

**Read Existing Chapters:**
Load the current contents of the chapters JSON file specified by `--chapters` parameter

**For New Chapters:**

**Check for Duplicates:**
- Verify the chapter ID doesn't already exist
- Check that the file path isn't already in use
- Normalize and compare titles to avoid near-duplicates

**Append New Chapters:**
- Add new chapter definitions to the chapters array

**Record Action:**
```json
{
  "action": "created_chapter_definition",
  "chapter_id": "<chapter-id>",
  "file_path": "<chapters-file-path from --chapters parameter>"
}
```

**For New Subsections:**

**Find Target Chapter:**
- Locate the parent chapter by ID in chapters array
- Verify chapter type is "multi-subsection"
- If chapter is "single-file", log warning and skip (requires migration first)

**Check for Duplicate Subsections:**
- Check if subsection ID already exists in chapter's subsections array
- Verify file path is unique within chapter
- Compare titles to avoid near-duplicates

**Append Subsection to Chapter:**
- Add subsection definition to chapter's subsections array
- Maintain array order (alphabetical or logical)

**Record Action:**
```json
{
  "action": "created_subsection_definition",
  "chapter_id": "<parent-chapter-id>",
  "subsection_id": "<subsection-id>",
  "file_path": "<chapters-file-path from --chapters parameter>"
}
```

**Write Updated Chapter Definitions:**
Write the complete chapters JSON back to disk with proper formatting (if any gaps were found):
- Use 2-space indentation
- Maintain JSON structure
- Preserve existing chapters and subsections
- Keep subsection order within chapters

**Note**: The flattened-items.json generation has moved to Phase 8 to ensure it always executes.

### Phase 6: Create Stub Markdown Files

**For Each New Chapter and Subsection:**

**For New Chapters:**

1. **Determine Stub Content:**
   Generate markdown following this minimal template structure:

```markdown
# {Chapter Title}

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

**Note**: Do NOT include Prerequisites, Installation, Best Practices, or Troubleshooting sections in chapter stubs. These belong in dedicated files or the chapter index.md only

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

**For New Subsections:**

1. **Determine Stub Content:**
   Generate markdown following this minimal subsection template:

```markdown
# {Subsection Title}

{Brief introduction explaining this specific aspect of the parent chapter}

## Overview

{Focused description of what this subsection covers within the chapter context}

## Configuration

{If applicable, specific configuration options for this feature}

```yaml
# Example configuration
```

## Usage

{Simple examples demonstrating the core functionality}

## Related Subsections

- [Related Subsection](../related-subsection.md)
```

**Note**: Do NOT include Prerequisites, Installation, Best Practices, or Troubleshooting sections in subsection stubs. These belong in the parent chapter index.md or dedicated files

2. **Customize Content for Subsection:**
   - Use subsection title from definition
   - Reference feature_mapping features from features.json
   - Include subsection-specific topics
   - Add cross-references to related subsections
   - Keep content focused on subsection scope

3. **Create Subsection File:**
   - Write stub markdown to subsection file path
   - Ensure parent chapter directory exists (e.g., book/src/{parent-chapter-id}/)
   - Use proper markdown formatting

4. **Validate Markdown:**
   - Ensure valid markdown syntax
   - Check won't break mdbook build
   - Verify cross-references use correct relative paths

5. **Record Action:**
```json
{
  "action": "created_stub_file",
  "file_path": "<subsection-file-path>",
  "type": "subsection",
  "parent_chapter_id": "<parent-chapter-id>"
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

**For New Subsections:**

1. **Locate Parent Chapter:**
   - Find the parent chapter entry in SUMMARY.md
   - Check if chapter already has nested subsections

2. **Add Subsection as Nested List Item:**
   ```markdown
   - [Parent Chapter](parent/index.md)
     - [Subsection 1](parent/subsection-1.md)
     - [New Subsection](parent/new-subsection.md)
   ```

3. **Maintain Subsection Order:**
   - Keep alphabetical or logical ordering within chapter
   - Ensure indentation is correct (2-4 spaces)
   - Follow existing subsection format in SUMMARY.md

**Write Updated SUMMARY.md:**
Write the modified SUMMARY.md back to disk

**Record Action:**
```json
{
  "action": "updated_summary",
  "file_path": "book/src/SUMMARY.md",
  "items_added": [
    {"type": "chapter", "id": "..."},
    {"type": "subsection", "parent": "...", "id": "..."}
  ]
}
```

### Phase 7.5: Validate and Sync Chapter Structure with Reality (MANDATORY)

**CRITICAL: Ensure chapters.json accurately reflects the actual file structure.**

Before generating flattened-items.json, validate that all chapter definitions match reality.

**Step 1: Scan for Multi-Subsection Directories**

1. Find all directories under `${BOOK_DIR}/src/` that contain an `index.md` file
2. For each directory found:
   - Count how many `.md` files exist (excluding `index.md`)
   - If count > 0, this is a multi-subsection chapter
   - Record the chapter ID (directory name) in a list of discovered multi-subsection chapters

**Step 2: Compare Against chapters.json Definitions**

For each discovered multi-subsection chapter:
1. Look up how it's defined in `$CHAPTERS_FILE`
2. Check if `type` field is "multi-subsection" or "single-file"
3. **If type is "single-file" or missing**: This is a MISMATCH - add to mismatches list
4. **If type is "multi-subsection"**: Count subsections in chapters.json and compare to actual file count
   - If counts don't match, add to mismatches list

**Step 3: Check for Orphaned Single-File Definitions**

For each chapter in chapters.json that has `type: "single-file"`:
1. Check if the expected file (e.g., `book/src/chapter-id.md`) exists
2. Check if a directory with that name exists (e.g., `book/src/chapter-id/`)
3. **If file doesn't exist but directory does**: Add to mismatches list

**Step 4: Auto-Migrate Mismatched Chapters**

For each chapter in the mismatches list:
1. Scan the chapter directory to discover all subsection files
2. For each subsection `.md` file (excluding `index.md`):
   - Extract subsection ID from filename (remove `.md` extension)
   - Read the file and extract title from first H1 or H2 heading
   - If no heading found, convert filename to Title Case
   - Extract topics from section headings (H2/H3) in the file
   - Create subsection definition object with: id, title, file path, topics, validation
3. Build complete subsections array from all discovered subsections
4. Update the chapter in chapters.json:
   - Change `type` to "multi-subsection"
   - Change `file` to `index_file` pointing to `book/src/{chapter-id}/index.md`
   - Add `subsections` array with all discovered subsections
   - Preserve existing `topics` and `validation` fields
5. Write updated chapters.json to disk
6. Record this migration in MIGRATION_ACTIONS list

**Step 5: Add Structure Validation to Gap Report**

Add a `structure_validation` section to the gap report JSON:
- `mismatches_found`: Count of chapters that needed migration
- `mismatched_chapters`: Array of chapter IDs that were migrated
- `migrations_performed`: Array of migration actions taken
- `validation_timestamp`: Current timestamp

**Step 6: Commit Structure Fixes (if any)**

If any structure mismatches were found and fixed:
1. Stage the updated chapters.json file
2. Create a commit with message:
   - Title: "docs: sync chapters.json with actual file structure"
   - Body: List of migrated chapters and explanation
3. This commit must happen BEFORE generating flattened-items.json

### Phase 8: Save Gap Report, Generate Flattened Items, and Commit Changes

**STEP 1: Generate Flattened Items for Map Phase (MANDATORY)**

This step MUST execute regardless of whether gaps were found:

1. Read the chapters file from `--chapters` parameter
2. Process each chapter to create flattened array:
   - For `type == "multi-subsection"`: Extract each subsection with parent metadata
   - For `type == "single-file"`: Include chapter with type marker
3. Determine output path from config:
   - Extract `book_dir` from `--config` parameter
   - Create analysis directory: `.{project_lowercase}/book-analysis/`
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

🟡 Medium Severity Gaps (Incomplete Chapters):
  • {chapter_id} - Missing: {missing_topics}

✅ Actions Taken:
  ✓ Generated flattened-items.json for map phase
  ✓ Created {count} chapter definitions (if gaps found)
  ✓ Created {count} stub files (if gaps found)
  ✓ Updated SUMMARY.md (if gaps found)

📝 Next Steps:
  The map phase will now process these chapters to detect drift.
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
   - Format: "docs: auto-discover missing chapters for [feature names]"
   - Example: "docs: auto-discover missing chapters for authentication, rate-limiting"
   - Include brief summary of actions taken

**If NO gaps were found (still need to commit flattened-items.json):**
1. Stage generated files:
   - flattened-items.json (REQUIRED for map phase)
   - Gap report (shows 0 gaps found)

2. Create commit with message:
   - Format: "docs: regenerate flattened-items.json for drift detection (no gaps found)"
   - Include count of chapters/subsections to be processed

### Phase 9: Validation and Quality Checks

**Verify No False Positives:**
- Check that no duplicate chapters were created
- Ensure existing chapters weren't unnecessarily modified
- Verify chapter IDs are unique

**Verify No False Negatives:**
- Check that all obvious undocumented features were detected
- Compare feature areas against documented topics
- Ensure classification (high/medium/low) is appropriate

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

**CRITICAL**: The flattened-items.json file must ALWAYS be generated, even when no gaps are found. This file is required by the map phase to process all chapters for drift detection. Generate it from the existing chapters.json file in Phase 5, regardless of whether gaps were detected.

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
- Use fuzzy matching for topic comparison
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
- Process chapters in parallel if needed

### Configuration Options (Future Enhancement)

The book-config.json could support gap detection settings:

```json
{
  "gap_detection": {
    "enabled": true,
    "min_severity": "medium",
    "auto_create_stubs": true,
    "template_path": "workflows/data/stub-template.md",
    "similarity_threshold": 0.8,
    "dry_run": false
  }
}
```

For now, use sensible defaults:
- enabled: true
- min_severity: "high"
- auto_create_stubs: true
- similarity_threshold: 0.7 (fuzzy matching threshold)

### Success Indicators

Gap detection is successful when:
- All undocumented features are identified
- New chapter definitions are valid and complete
- Stub markdown files are properly formatted
- SUMMARY.md structure is maintained
- Book builds without errors
- No duplicate chapters created
- Changes are committed cleanly
- **`.prodigy/book-analysis/flattened-items.json` file is created** (REQUIRED)

### Output Format

The command should output progress and results clearly:

**During Execution:**
```
🔍 Analyzing documentation coverage...
   ✓ Loaded 12 feature areas from features.json
   ✓ Loaded 10 existing chapters
   ✓ Parsed SUMMARY.md structure

📊 Comparing features against documentation...
   ✓ Analyzed authentication: documented ✓
   ✓ Analyzed data_processing: documented ✓
   ⚠ Analyzed rate_limiting: not documented (gap detected)
   ✓ Analyzed api_endpoints: documented ✓
   ⚠ Analyzed caching: not documented (gap detected)

📝 Creating missing chapters...
   ✓ Generated definition: rate-limiting
   ✓ Created stub: book/src/rate-limiting.md
   ✓ Generated definition: caching
   ✓ Created stub: book/src/caching.md
   ✓ Updated SUMMARY.md

💾 Committing changes...
   ✓ Staged 4 files
   ✓ Committed: docs: auto-discover missing chapters for rate-limiting, caching
```

**Final Summary:**
```
📊 Documentation Gap Analysis Complete
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Features Analyzed: 12
Documented Topics: 10
Gaps Found: 2

🔴 High Severity Gaps (Missing Chapters): 2
  • rate_limiting - Request rate limiting and throttling
  • caching - Response caching strategies

✅ Actions Taken:
  ✓ Created 2 chapter definitions in chapters file
  ✓ Created 2 stub files in book/src/
  ✓ Updated book/src/SUMMARY.md
  ✓ Committed changes

📝 Next Steps:
  The map phase will now process these new chapters to populate content.
  Review the generated stubs and customize as needed.
```

## FINAL CHECKLIST

Before completing this command, verify:

1. ✅ Gap report saved to `${analysis_dir}/gap-report.json`
2. ✅ **`${analysis_dir}/flattened-items.json` created (MANDATORY - even if no gaps found)**
3. ✅ Chapter definitions updated in chapters file (if gaps found)
4. ✅ Stub files created in book/src/ (if gaps found)
5. ✅ SUMMARY.md updated (if gaps found)
6. ✅ Changes committed (if any files modified)

**CRITICAL**: Step 2 (flattened-items.json) is REQUIRED for the workflow to proceed to the map phase. This file must contain all chapters and subsections in a flat array format, ready for parallel processing.
