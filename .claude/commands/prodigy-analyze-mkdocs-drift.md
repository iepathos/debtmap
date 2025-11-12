# /prodigy-analyze-mkdocs-drift

Analyze a specific MkDocs documentation page for drift against the actual codebase implementation.

This command supports both top-level pages and section pages within the MkDocs Material documentation structure.

## Variables

- `--project <name>` - Project name (e.g., "Prodigy")
- `--json <item>` - JSON object containing page details from flattened-items.json
- `--features <path>` - Path to features.json file (e.g., ".prodigy/mkdocs-analysis/features.json")

## Execute

### Phase 1: Understand Context

You are comparing documentation against the actual codebase to identify drift (discrepancies between documentation and implementation).

**This command handles two types of pages:**
1. **Top-level pages**: Standalone pages like index.md, getting-started.md
2. **Section pages**: Pages within sections (e.g., mapreduce/overview.md, configuration/global-config.md)

### Phase 2: Parse Input and Determine Page Type

**Extract Parameters:**
```bash
PROJECT_NAME="<value from --project parameter>"
ITEM_JSON="<value from --json parameter>"
FEATURES_PATH="<value from --features parameter>"
```

**Parse Item JSON and Determine Type:**
```bash
ITEM_TYPE=$(echo "$ITEM_JSON" | jq -r '.type // "single-file"')
ITEM_ID=$(echo "$ITEM_JSON" | jq -r '.id')
ITEM_TITLE=$(echo "$ITEM_JSON" | jq -r '.title')
ITEM_FILE=$(echo "$ITEM_JSON" | jq -r '.file')
```

**For Section Pages:**
```bash
if [ "$ITEM_TYPE" = "section-page" ]; then
  SECTION_ID=$(echo "$ITEM_JSON" | jq -r '.section_id // ""')
  SECTION_TITLE=$(echo "$ITEM_JSON" | jq -r '.section_title // ""')
  FEATURE_MAPPING=$(echo "$ITEM_JSON" | jq -r '.feature_mapping[]?')
fi
```

### Phase 3: Load and Filter Features

**Read Features Inventory:**
Read the features.json file from `$FEATURES_PATH`.

**Filter Features Based on Page Type:**

**For Pages with feature_mapping:**
- Extract only the features listed in `feature_mapping` array
- This provides focused, page-specific feature context
- Reduces noise from unrelated features
- Example: If `feature_mapping: ["mapreduce.checkpoint", "mapreduce.resume"]`
  - Only compare against checkpoint and resume features
  - Ignore DLQ, environment variables, and other MapReduce features

**For Pages Without feature_mapping:**
- Use all relevant features based on page topics
- Match features by category, keywords, and topic overlap
- Apply broader feature context since page covers entire area

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
- Check for MkDocs Material extensions (admonitions, tabs, code annotations)

### Phase 5: Perform Drift Analysis

**Section Page-Specific Analysis:**

If analyzing a section page:
1. **Focus on Mapped Features Only:**
   - Compare page content ONLY against features in `feature_mapping`
   - Don't flag missing content if feature is outside page scope
   - Example: DLQ page should not be flagged for missing checkpoint docs

2. **Check Cross-References:**
   - Verify links to other pages are valid
   - Ensure related page references are accurate
   - Check that page fits within section context

3. **Validate Page Scope:**
   - Content should stay within page's defined topics
   - Don't document features better suited for other pages
   - Maintain clear page boundaries

**Top-Level Page-Specific Analysis:**

If analyzing a top-level page (like index.md):
1. **Broad Feature Coverage:**
   - Check all features related to page topic
   - Ensure comprehensive coverage of page's area
   - Validate examples span major use cases

2. **Structure and Organization:**
   - Verify logical flow and progression
   - Check section organization is clear
   - Ensure proper introduction and overview

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
- Individual section pages (non-index pages)
- Pages that document specific implementation details
- Reference section pages

**DO flag "Missing Best Practices" for:**
- Top-level guide pages (getting-started.md, overview.md)
- Section overview pages with comprehensive scope
- Tutorial or conceptual pages without practical guidance

**Detection Logic:**
```bash
# Determine if this page should have best practices
SHOULD_HAVE_BEST_PRACTICES=false

# Check if this is an overview or guide page
if [[ "$ITEM_ID" == "index" ]] || [[ "$ITEM_ID" == *"overview"* ]] || [[ "$ITEM_ID" == "getting-started" ]]; then
  SHOULD_HAVE_BEST_PRACTICES=true
fi

# Skip for reference pages
if [[ "$ITEM_FILE" == *"/reference/"* ]]; then
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
- References to moved/renamed pages
- Links that would break on mkdocs build

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
   - If target starts with `/`: Treat as absolute path from `docs/`
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
   - Check if file was moved to a section subdirectory
   - Check if file was renamed
3. Search for similar files:
   - Find all `.md` files in `docs/` containing the basename
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
   - `fix_suggestion`: "Update links to point to current file locations."
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

**For Section Pages:**
- Pattern: `.prodigy/mkdocs-analysis/drift-${SECTION_ID}-${ITEM_ID}.json`
- Example: `.prodigy/mkdocs-analysis/drift-mapreduce-checkpoint-resume.json`

**For Top-Level Pages:**
- Pattern: `.prodigy/mkdocs-analysis/drift-${ITEM_ID}.json`
- Example: `.prodigy/mkdocs-analysis/drift-index.json`

**Create Drift Report:**

**For Section Pages:**
```json
{
  "item_type": "section-page",
  "section_id": "$SECTION_ID",
  "section_title": "$SECTION_TITLE",
  "page_id": "$ITEM_ID",
  "page_title": "$ITEM_TITLE",
  "page_file": "$ITEM_FILE",
  "feature_mappings": ["mapreduce.checkpoint", "mapreduce.resume"],
  "drift_detected": true,
  "severity": "medium",
  "quality_assessment": "Page needs updates to reflect session-job ID mapping",
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
    "Add cross-reference to DLQ page for failure handling",
    "Include troubleshooting section for resume issues"
  ],
  "cross_references": [
    "dlq",
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

**For Top-Level Pages:**
```json
{
  "item_type": "page",
  "page_id": "$ITEM_ID",
  "page_title": "$ITEM_TITLE",
  "page_file": "$ITEM_FILE",
  "drift_detected": true,
  "severity": "high",
  "quality_assessment": "Page needs updates to reflect current syntax",
  "issues": [
    {
      "type": "outdated_information",
      "severity": "high",
      "section": "Quick Start",
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
    "topics_covered": ["Getting started", "Basic workflows"],
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
git add .prodigy/mkdocs-analysis/drift-*.json
git commit -m "analysis: drift report for $PROJECT_NAME page '$ITEM_TITLE'

Drift severity: $SEVERITY
Issues found: $ISSUE_COUNT
Quality: $QUALITY_ASSESSMENT"
```

### Phase 9: Validation

**The drift report should:**
1. Accurately identify drift between documentation and code
2. Focus on features within page scope (for pages with feature_mapping)
3. Categorize issues by type and severity
4. Assess overall quality
5. Provide actionable fix suggestions
6. Include source references
7. Note positive aspects to preserve
8. List cross-references to related pages
9. Be committed to git for later phases

### Quality Guidelines

**Page-Specific Guidelines:**
- Only flag content related to page's feature_mapping (if present)
- Don't penalize for not covering features outside page scope
- Validate cross-references use correct relative paths
- Check MkDocs Material-specific features are used correctly

**Accuracy:**
- Minimize false positives
- Focus on user-visible drift
- Check examples actually work
- Verify syntax is current

**Clarity:**
- Provide specific, actionable fix suggestions
- Reference exact source locations when possible
- Explain why content is outdated or incorrect
- Suggest concrete improvements

**Completeness:**
- Check all major features are documented
- Verify configuration options are complete
- Ensure examples cover common use cases
- Validate troubleshooting addresses real issues
