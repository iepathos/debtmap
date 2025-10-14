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

### Phase 3: Identify Documentation Gaps

**Compare Features Against Documentation:**

For each feature area in features.json:
1. Extract the feature category name (e.g., "mapreduce", "command_types", "environment")
2. Check if ANY existing chapter covers this feature area
3. Look for both exact matches and partial matches in:
   - Chapter IDs
   - Chapter titles
   - Chapter topics list
   - Section headings in markdown files

**Classify Gaps by Severity:**

**High Severity (Missing Chapter):**
- Feature area exists in features.json
- No corresponding chapter found in documentation
- Major user-facing capability with no guidance
- Example: "agent_merge" feature exists but no chapter documents it

**Medium Severity (Incomplete Chapter):**
- Chapter exists for the feature area
- But specific sub-capabilities are missing
- Chapter topics list doesn't cover all feature categories
- Example: "retry_configuration" chapter exists but doesn't document "jitter" or "retry_budget"

**Low Severity (Minor Gap):**
- Edge cases or advanced features not documented
- Internal APIs exposed to users
- Less common use cases

**Generate Gap Report:**

Create a structured JSON report documenting all gaps found:
```json
{
  "analysis_date": "<current-timestamp>",
  "features_analyzed": <total-feature-areas>,
  "documented_topics": <count-of-chapters>,
  "gaps_found": <count-of-gaps>,
  "gaps": [
    {
      "severity": "high|medium|low",
      "type": "missing_chapter|incomplete_chapter",
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

### Phase 5: Update Chapter Definitions File

**Read Existing Chapters:**
Load the current contents of the chapters JSON file (e.g., workflows/data/prodigy-chapters.json)

**Check for Duplicates:**
For each new chapter definition:
- Verify the chapter ID doesn't already exist
- Check that the file path isn't already in use
- Normalize and compare titles to avoid near-duplicates

**Append New Chapters:**
Add new chapter definitions to the chapters array

**Write Updated File:**
Write the complete chapters JSON back to disk with proper formatting:
- Use 2-space indentation
- Maintain JSON structure
- Preserve existing chapters

**Record Action:**
Add to actions_taken in gap report:
```json
{
  "action": "created_chapter_definition",
  "chapter_id": "<chapter-id>",
  "file_path": "workflows/data/prodigy-chapters.json"
}
```

### Phase 6: Create Stub Markdown Files

**For Each New Chapter:**

1. **Determine Stub Content:**
   Generate markdown following this template structure:

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

## Use Cases

### {Use Case 1 Name}

{Description and example}

## Examples

### Basic Example

```yaml
# Example workflow
```

## Best Practices

- {Best practice 1}
- {Best practice 2}

## Troubleshooting

### Common Issues

**Issue**: {Common problem}
**Solution**: {How to fix}

## See Also

- [Related documentation](link)
```

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
  "file_path": "<file-path>"
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

**Classify New Chapters:**
For each new chapter, determine appropriate section:
- Basic workflow features → User Guide
- Advanced features (retry, error handling, composition) → Advanced Topics
- Examples and troubleshooting → Reference

**Determine Insertion Point:**
Within the appropriate section:
- Maintain alphabetical order by title
- Or maintain logical order based on dependencies
- Insert after similar topics

**Insert Chapter Entries:**
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
  "file_path": "book/src/SUMMARY.md"
}
```

### Phase 8: Save Gap Report and Commit Changes

**Write Gap Report:**
Save the gap report to disk for auditing:
- Path: `.prodigy/book-analysis/gap-report.json` (or equivalent for project)
- Include all gaps found and actions taken
- Use proper JSON formatting

**Display Summary to User:**
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
  ✓ Created {count} chapter definitions
  ✓ Created {count} stub files
  ✓ Updated SUMMARY.md

📝 Next Steps:
  The map phase will now process these new chapters to populate content.
```

**Stage and Commit Changes:**
If running in automation mode (PRODIGY_AUTOMATION=true):
1. Stage all modified files:
   - Updated prodigy-chapters.json
   - New stub markdown files
   - Updated SUMMARY.md
   - Gap report

2. Create commit with message:
   - Format: "docs: auto-discover missing chapters for [feature names]"
   - Example: "docs: auto-discover missing chapters for agent-merge-workflows, circuit-breaker"
   - Include brief summary of actions taken

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
- Do not modify any files
- Exit successfully

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

### Output Format

The command should output progress and results clearly:

**During Execution:**
```
🔍 Analyzing documentation coverage...
   ✓ Loaded 12 feature areas from features.json
   ✓ Loaded 10 existing chapters
   ✓ Parsed SUMMARY.md structure

📊 Comparing features against documentation...
   ✓ Analyzed workflow_basics: documented ✓
   ✓ Analyzed mapreduce: documented ✓
   ⚠ Analyzed agent_merge: not documented (gap detected)
   ✓ Analyzed command_types: documented ✓
   ⚠ Analyzed circuit_breaker: not documented (gap detected)

📝 Creating missing chapters...
   ✓ Generated definition: agent-merge-workflows
   ✓ Created stub: book/src/agent-merge-workflows.md
   ✓ Generated definition: circuit-breaker
   ✓ Created stub: book/src/circuit-breaker.md
   ✓ Updated SUMMARY.md

💾 Committing changes...
   ✓ Staged 4 files
   ✓ Committed: docs: auto-discover missing chapters for agent-merge-workflows, circuit-breaker
```

**Final Summary:**
```
📊 Documentation Gap Analysis Complete
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Features Analyzed: 12
Documented Topics: 10
Gaps Found: 2

🔴 High Severity Gaps (Missing Chapters): 2
  • agent_merge - Custom merge workflows for map agents
  • circuit_breaker - Workflow error circuit breaking

✅ Actions Taken:
  ✓ Created 2 chapter definitions in workflows/data/prodigy-chapters.json
  ✓ Created 2 stub files in book/src/
  ✓ Updated book/src/SUMMARY.md
  ✓ Committed changes

📝 Next Steps:
  The map phase will now process these new chapters to populate content.
  Review the generated stubs and customize as needed.
```
