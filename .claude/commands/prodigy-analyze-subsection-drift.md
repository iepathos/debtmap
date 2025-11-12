# /prodigy-analyze-subsection-drift

Analyze a specific chapter or subsection of a project's book for drift against the actual codebase implementation.

This command supports both single-file chapters (backward compatible) and individual subsections within multi-subsection chapters.

## Variables

- `--project <name>` - Project name (e.g., "Prodigy", "Debtmap")
- `--json <item>` - JSON object containing chapter or subsection details
- `--features <path>` - Path to features.json file (e.g., ".prodigy/book-analysis/features.json")
- `--chapter-id <id>` - (Optional) Parent chapter ID if analyzing a subsection

## Execute

### Phase 1: Understand Context

You are comparing documentation (either a full chapter or a subsection) against the actual codebase to identify drift (discrepancies between documentation and implementation).

**This command handles two types of items:**
1. **Single-file chapters**: Traditional flat markdown files
2. **Subsections**: Individual sections within a multi-subsection chapter

### Phase 2: Parse Input and Determine Item Type

**Extract Parameters:**
```bash
PROJECT_NAME="<value from --project parameter>"
ITEM_JSON="<value from --json parameter>"
FEATURES_PATH="<value from --features parameter>"
CHAPTER_ID_PARAM="<value from --chapter-id parameter, may be empty>"
```

**Parse Item JSON and Determine Type:**
```bash
ITEM_TYPE=$(echo "$ITEM_JSON" | jq -r '.type // "single-file"')
ITEM_ID=$(echo "$ITEM_JSON" | jq -r '.id')
ITEM_TITLE=$(echo "$ITEM_JSON" | jq -r '.title')
ITEM_FILE=$(echo "$ITEM_JSON" | jq -r '.file // .index_file')
```

**For Subsections:**
```bash
if [ "$ITEM_TYPE" = "subsection" ]; then
  PARENT_CHAPTER_ID=$(echo "$ITEM_JSON" | jq -r '.parent_chapter_id // ""')
  if [ -z "$PARENT_CHAPTER_ID" ] && [ -n "$CHAPTER_ID_PARAM" ]; then
    PARENT_CHAPTER_ID="$CHAPTER_ID_PARAM"
  fi
  FEATURE_MAPPING=$(echo "$ITEM_JSON" | jq -r '.feature_mapping[]?')
fi
```

### Phase 3: Load and Filter Features

**Read Features Inventory:**
Read the features.json file from `$FEATURES_PATH`.

**Filter Features Based on Item Type:**

**For Subsections with feature_mapping:**
- Extract only the features listed in `feature_mapping` array
- This provides focused, subsection-specific feature context
- Reduces noise from unrelated features
- Example: If `feature_mapping: ["mapreduce.checkpoint", "mapreduce.resume"]`
  - Only compare against checkpoint and resume features
  - Ignore DLQ, environment variables, and other MapReduce features

**For Single-File Chapters:**
- Use all relevant features based on chapter topics
- Match features by category, keywords, and topic overlap
- Apply broader feature context since chapter covers entire area

**Create Filtered Feature Set:**
Store the filtered features for use in drift analysis.

### Phase 4: Read Current Documentation

**Read the markdown file:**
- Path: `$ITEM_FILE`
- Parse markdown structure
- Extract code examples
- Note explanations and descriptions
- Identify section headings
- Check for version compatibility notes

### Phase 5: Perform Drift Analysis

**Subsection-Specific Analysis:**

If analyzing a subsection:
1. **Focus on Mapped Features Only:**
   - Compare subsection content ONLY against features in `feature_mapping`
   - Don't flag missing content if feature is outside subsection scope
   - Example: DLQ subsection should not be flagged for missing checkpoint docs

2. **Check Cross-References:**
   - Verify links to other subsections are valid
   - Ensure related subsection references are accurate
   - Check that subsection fits within chapter context

3. **Validate Subsection Scope:**
   - Content should stay within subsection's defined topics
   - Don't document features better suited for other subsections
   - Maintain clear subsection boundaries

**Chapter-Specific Analysis:**

If analyzing a single-file chapter or multi-subsection index:
1. **Broad Feature Coverage:**
   - Check all features related to chapter topic
   - Ensure comprehensive coverage of chapter's area
   - Validate examples span major use cases

2. **Structure and Organization:**
   - Verify logical flow and progression
   - Check section organization is clear
   - Ensure proper introduction and conclusion

**Common Drift Checks (Both Types):**

#### Missing Content (High Severity):
- Feature exists in filtered feature set but not documented
- Important capability not explained
- Critical field not documented

#### Outdated Information (High Severity):
- Information no longer accurate
- Syntax changed but documentation shows old syntax
- Capabilities changed but not reflected

#### Incorrect Examples (Medium Severity):
- YAML example won't work
- Example uses deprecated syntax
- Example missing required fields

#### Incomplete Explanation (Medium Severity):
- Feature mentioned but not fully explained
- No example provided for complex feature
- Use cases not clear

#### Missing Best Practices (Low Severity - CONTEXT-AWARE):

**IMPORTANT: Only flag this for appropriate document types.**

**DO NOT flag "Missing Best Practices" for:**
- Technical reference pages documenting syntax or configuration options
- Individual subsections within chapters (non-index.md files)
- Pages that document specific implementation details
- Files in chapters that already have a dedicated best-practices.md file
- Files that are themselves subsections (check if parent directory has index.md)

**DO flag "Missing Best Practices" for:**
- Chapter-level index.md files without any best practices content
- Standalone guide pages at the book root level
- Tutorial or conceptual chapters without practical guidance

**Detection Logic:**
```bash
# Determine if this file should have best practices
SHOULD_HAVE_BEST_PRACTICES=false

# Check if this is a chapter index file
if [[ "$ITEM_FILE" == */index.md ]]; then
  SHOULD_HAVE_BEST_PRACTICES=true
fi

# Check if chapter already has dedicated best-practices.md
CHAPTER_DIR=$(dirname "$ITEM_FILE")
if [ -f "$CHAPTER_DIR/best-practices.md" ]; then
  SHOULD_HAVE_BEST_PRACTICES=false
fi

# Skip for subsections (files in directories with index.md)
if [[ "$ITEM_FILE" != */index.md ]] && [ -f "$CHAPTER_DIR/index.md" ]; then
  SHOULD_HAVE_BEST_PRACTICES=false
fi
```

**Only flag if SHOULD_HAVE_BEST_PRACTICES=true AND:**
- Common pattern not documented
- Gotcha not mentioned
- Optimization tip missing

#### Unclear Content (Low Severity):
- Confusing explanation
- Poor organization
- Needs better examples

#### Broken Links (Medium Severity):
- Links to non-existent files
- Incorrect relative paths
- References to moved/renamed chapters
- Links that would break on mdbook build

### Phase 5.5: Validate Internal Links (MANDATORY)

**CRITICAL: Check all internal documentation links are valid.**

Before creating the drift report, scan the current documentation for broken links.

**Step 1: Extract All Internal Links from Current File**

Read `$ITEM_FILE` and find all markdown links:
1. Search for the pattern `[link text](target path)`
2. Extract both the link text and target path
3. For each link found:
   - **Skip external URLs** (starting with `http://` or `https://`)
   - **Skip anchor-only links** (starting with `#`)
   - Keep internal file links for validation

**Step 2: Validate Each Internal Link**

For each internal link found:
1. Determine the current file's directory (parent directory of `$ITEM_FILE`)
2. Resolve the link target relative to the current file:
   - If target starts with `/`: Treat as absolute path from `book/src/`
   - Otherwise: Treat as relative path from current file's directory
3. Remove any anchor fragment (e.g., `file.md#section` → `file.md`)
4. Check if the resolved target file exists
5. **If file does NOT exist**: Record as broken link with:
   - Link text
   - Original target path
   - Expected resolved file path
   - Current file path

**Step 3: Suggest Corrections for Broken Links**

For each broken link:
1. Extract the base filename from the target (remove `.md` extension)
2. Check common migration patterns:
   - **Single-file to multi-subsection**: Check if `${BOOK_DIR}/src/{basename}/index.md` exists
   - If it does, suggest: `{basename}/index.md` (chapter now has subsections)
3. Search for similar files:
   - Find all `.md` files in `${BOOK_DIR}/src/` containing the basename
   - List these as possible matches
4. Record suggested fix for each broken link

**Step 4: Add Broken Links to Drift Issues**

If any broken links were found:
1. Create a drift issue with:
   - `type`: "broken_links"
   - `severity`: "medium"
   - `section`: "Cross-References"
   - `description`: "Found X broken internal link(s)"
   - `broken_links`: Array of broken link details
   - `fix_suggestion`: "Update links to point to current file locations. Check if referenced chapters were migrated to multi-subsection structure."
2. Include suggested corrections for each broken link
3. Add this issue to the drift report's issues array

**Step 5: Report Link Validation Results**

Include in drift analysis output:
- Total internal links found
- Number of broken links
- List of broken links with suggested fixes
- If no broken links: Note that all cross-references are valid

### Phase 6: Assess Quality

**Overall Assessment:**
- **Critical**: Multiple high severity issues, will cause user errors
- **High**: Several missing features or outdated information
- **Medium**: Incorrect examples or incomplete explanations
- **Low**: Minor issues, could be clearer
- **Good**: No significant drift, minor improvements only

### Phase 7: Create Drift Report

**Determine Output Path:**

**For Subsections:**
- Pattern: `.prodigy/book-analysis/drift-${PARENT_CHAPTER_ID}-${ITEM_ID}.json`
- Example: `.prodigy/book-analysis/drift-mapreduce-checkpoint-and-resume.json`

**For Single-File Chapters:**
- Pattern: `.prodigy/book-analysis/drift-${ITEM_ID}.json`
- Example: `.prodigy/book-analysis/drift-workflow-basics.json`

**Create Drift Report:**

**For Subsections:**
```json
{
  "item_type": "subsection",
  "chapter_id": "$PARENT_CHAPTER_ID",
  "subsection_id": "$ITEM_ID",
  "subsection_title": "$ITEM_TITLE",
  "subsection_file": "$ITEM_FILE",
  "feature_mappings": ["mapreduce.checkpoint", "mapreduce.resume"],
  "drift_detected": true,
  "severity": "medium",
  "quality_assessment": "Subsection needs updates to reflect session-job ID mapping",
  "issues": [
    {
      "type": "missing_content",
      "severity": "high",
      "section": "Resume Behavior",
      "description": "Missing documentation for session-job ID mapping",
      "feature_reference": "mapreduce.resume.session_job_mapping",
      "fix_suggestion": "Add section explaining bidirectional session-job mapping",
      "source_reference": "src/mapreduce/resume.rs:SessionJobMapping"
    }
  ],
  "positive_aspects": [
    "Clear explanation of checkpoint structure",
    "Good examples of resume workflow"
  ],
  "improvement_suggestions": [
    "Add cross-reference to DLQ subsection for failure handling",
    "Include troubleshooting section for resume issues"
  ],
  "cross_references": [
    "dead-letter-queue-dlq",
    "event-tracking"
  ],
  "metadata": {
    "analyzed_at": "<timestamp>",
    "feature_inventory": "$FEATURES_PATH",
    "topics_covered": ["checkpoints", "resume", "recovery"],
    "validation_focus": "$VALIDATION"
  }
}
```

**For Single-File Chapters:**
```json
{
  "item_type": "chapter",
  "chapter_id": "$ITEM_ID",
  "chapter_title": "$ITEM_TITLE",
  "chapter_file": "$ITEM_FILE",
  "drift_detected": true,
  "severity": "high",
  "quality_assessment": "Chapter needs updates to reflect current syntax",
  "issues": [
    {
      "type": "outdated_information",
      "severity": "high",
      "section": "Basic Workflow Structure",
      "description": "Example shows deprecated command syntax",
      "current_content": "test: cargo test",
      "should_be": "shell: cargo test",
      "fix_suggestion": "Update all examples to use current command syntax",
      "source_reference": "src/config/command.rs:WorkflowStepCommand"
    }
  ],
  "positive_aspects": [
    "Clear introduction and motivation",
    "Good progression from simple to complex"
  ],
  "improvement_suggestions": [
    "Add more real-world examples",
    "Include common pitfalls section"
  ],
  "metadata": {
    "analyzed_at": "<timestamp>",
    "feature_inventory": "$FEATURES_PATH",
    "topics_covered": ["Standard workflows", "Basic structure"],
    "validation_focus": "$VALIDATION"
  }
}
```

### Phase 8: Commit the Drift Report

**CRITICAL**: The drift report MUST be committed to be accessible in subsequent workflow phases.

**Determine Drift File Path:**
Use the path logic from Phase 7.

**Commit the Report:**
```bash
git add .prodigy/book-analysis/drift-*.json
git commit -m "analysis: drift report for $PROJECT_NAME $ITEM_TYPE '$ITEM_TITLE'

Drift severity: $SEVERITY
Issues found: $ISSUE_COUNT
Quality: $QUALITY_ASSESSMENT"
```

### Phase 9: Validation

**The drift report should:**
1. Accurately identify drift between documentation and code
2. Focus on features within subsection scope (for subsections)
3. Categorize issues by type and severity
4. Assess overall quality
5. Provide actionable fix suggestions
6. Include source references
7. Note positive aspects to preserve
8. List cross-references to related subsections (for subsections)
9. Be committed to git for later phases

### Quality Guidelines

**Subsection-Specific Guidelines:**
- Only flag content related to subsection's feature_mapping
- Don't penalize for not covering other chapter features
- Validate cross-references to sibling subsections
- Ensure subsection maintains focus on its topic area

**Chapter-Level Guidelines:**
- Check comprehensive coverage of all chapter topics
- Validate overall structure and organization
- Ensure proper introduction and summary

**General Guidelines:**
- Think from reader's perspective
- Verify examples actually work
- Check accuracy against source code
- Ensure clarity and completeness
- Link to specific implementation details
