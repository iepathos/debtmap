# /prodigy-fix-mkdocs-drift

Fix documentation drift for a specific MkDocs page based on its drift analysis report.

This command supports both top-level pages and section pages within the MkDocs Material documentation structure.

## Variables

- `--project <name>` - Project name (e.g., "Prodigy")
- `--json <item>` - JSON object containing page details from flattened-items.json

## Execute

### Phase 1: Understand Context

You are fixing documentation drift for a MkDocs page. The analysis phase has already created a drift report. Your job is to:
1. Read the drift report
2. Fix all identified issues
3. Update the documentation file
4. Preserve cross-references
5. Use MkDocs Material conventions
6. Commit the changes

**Important for Section Pages:**
- Only update the specific page file
- Preserve links to other pages in the section
- Maintain page scope and focus
- Don't accidentally modify sibling pages

### Phase 1.5: Anti-Pattern Prevention (CRITICAL)

**DO NOT add these sections unless explicitly appropriate:**

1. **"Prerequisites" sections** - Only add to:
   - installation.md files or getting-started.md
   - NOT to reference pages or technical documentation
   - NOT to section pages

   **Replacement pattern:**
   ```markdown
   ## Prerequisites

   Before getting started, see the [Installation Guide](installation.md) for setup instructions.
   ```

2. **"Installation" sections** - Only add to:
   - Dedicated installation.md files
   - NOT to reference pages or feature documentation
   - NOT to section pages

3. **"Best Practices" sections** - Only add to:
   - Overview pages (index.md, getting-started.md)
   - Guide pages with comprehensive scope
   - NOT to technical reference pages or syntax documentation

4. **"See Also" sections** - Only add when:
   - There's a specific prerequisite relationship
   - There's a non-obvious connection between topics
   - NOT a generic list of all other pages
   - NOT circular references (page A → page B → page A)

5. **"Troubleshooting" sections** - Only add to:
   - Complex features with common pitfalls
   - Dedicated troubleshooting.md files in reference section
   - NOT to simple syntax reference pages
   - NOT to files documenting straightforward configuration options

6. **"Next Steps" / "Related Topics" / "Further Reading"** - Consolidate into:
   - A single section in overview pages
   - NOT separate stub sections
   - NOT repeated in every page

7. **"Quick Start" sections** - Only add to:
   - Dedicated quick-start.md or getting-started.md files
   - Overview pages for major features
   - NOT repeated in multiple pages
   - NOT in reference pages (use "Usage" instead)

**Detection Logic:**
```bash
# Check document type
IS_OVERVIEW_PAGE=false
IS_SECTION_PAGE=false
IS_REFERENCE_PAGE=false

if [[ "$ITEM_ID" == "index" ]] || [[ "$ITEM_ID" == *"overview"* ]] || [[ "$ITEM_ID" == "getting-started" ]]; then
  IS_OVERVIEW_PAGE=true
elif [[ "$ITEM_FILE" == *"/"* ]]; then
  IS_SECTION_PAGE=true
fi

# Detect reference pages (syntax, configuration, API docs)
if [[ "$ITEM_FILE" == *"/reference/"* ]] || grep -qi "syntax\|configuration\|reference\|api" "$ITEM_FILE" | head -1; then
  IS_REFERENCE_PAGE=true
fi
```

**Before adding any meta-section:**
- Check if documentation already has dedicated file (troubleshooting.md, best-practices.md)
- Verify page is appropriate type (overview vs section vs reference)
- Ensure content adds value, not just boilerplate

### Phase 2: Parse Input and Load Drift Report

**Extract Parameters:**
```bash
PROJECT_NAME="<value from --project parameter>"
ITEM_JSON="<value from --json parameter>"
```

**Determine Page Type and IDs:**

```bash
ITEM_TYPE=$(echo "$ITEM_JSON" | jq -r '.type // "single-file"')
ITEM_ID=$(echo "$ITEM_JSON" | jq -r '.id')
ITEM_FILE=$(echo "$ITEM_JSON" | jq -r '.file')
ITEM_TITLE=$(echo "$ITEM_JSON" | jq -r '.title')

if [ "$ITEM_TYPE" = "section-page" ]; then
  SECTION_ID=$(echo "$ITEM_JSON" | jq -r '.section_id')
  DRIFT_REPORT=".prodigy/mkdocs-analysis/drift-${SECTION_ID}-${ITEM_ID}.json"
else
  DRIFT_REPORT=".prodigy/mkdocs-analysis/drift-${ITEM_ID}.json"
fi
```

**Load Drift Report:**
```bash
if [ ! -f "$DRIFT_REPORT" ]; then
  echo "Error: Drift report not found at $DRIFT_REPORT"
  echo "Run analyze phase first"
  exit 1
fi

DRIFT_DATA=$(cat "$DRIFT_REPORT")
TOTAL_ISSUES=$(echo "$DRIFT_DATA" | jq '.issues | length')
```

### Phase 3: Read Current Documentation

```bash
CURRENT_CONTENT=$(cat "$ITEM_FILE")
```

Parse structure:
- Extract existing sections
- Identify code blocks
- Note cross-references
- Check for MkDocs Material extensions (admonitions, tabs, annotations)

### Phase 3.5: Discover Codebase Structure and Add Source Attribution

**CRITICAL: Add source attribution to ALL code examples.**

Before fixing drift issues, discover the codebase structure to enable accurate source references.

**Step 1: Discover Project Structure**

```bash
# Find test directories
TEST_DIRS=$(find . -type d -name "tests" -o -name "test" -o -name "__tests__" | head -5)

# Find example directories
EXAMPLE_DIRS=$(find . -type d -name "examples" -o -name "example" -o -name "demos" | head -5)

# Find main source directories
SOURCE_DIRS=$(find . -type d -name "src" -o -name "lib" -o -name "pkg" | head -5)

# Find workflow directories (project-specific)
WORKFLOW_DIRS=$(find . -type d -name "workflows" -o -name ".github/workflows" | head -5)
```

**Step 2: Extract Feature Context from Drift Report**

```bash
# Get feature mappings from drift report
FEATURE_MAPPINGS=$(echo "$DRIFT_DATA" | jq -r '.feature_mappings[]?' || echo "")

# Get topics from page item
TOPICS=$(echo "$ITEM_JSON" | jq -r '.topics[]?')
```

**Step 3: Search for Type Definitions**

For each feature in feature_mappings or topics:

```bash
# Example: If feature is "mapreduce.checkpoint"
# Search for struct/type definitions related to checkpoints

# Find struct definitions
CHECKPOINT_STRUCTS=$(rg "struct.*Checkpoint" --type rust -n $SOURCE_DIRS || echo "")

# Find enum definitions
CHECKPOINT_ENUMS=$(rg "enum.*Checkpoint" --type rust -n $SOURCE_DIRS || echo "")

# Record source locations for each relevant type
```

**Step 4: Search for Usage Examples**

```bash
# Search test files for examples
TEST_EXAMPLES=$(rg "checkpoint" --type rust -A 10 -B 2 $TEST_DIRS || echo "")

# Search example files
EXAMPLE_FILES=$(rg "checkpoint" --type rust -A 10 -B 2 $EXAMPLE_DIRS || echo "")

# Search workflow files for YAML examples
WORKFLOW_EXAMPLES=$(rg "checkpoint" --type yaml -A 10 -B 2 $WORKFLOW_DIRS || echo "")
```

**Step 5: Build Source Reference Map**

Create a mapping of concepts to source locations:

```json
{
  "checkpoint_structure": {
    "definition": "src/config/mapreduce.rs:45",
    "usage": "tests/mapreduce_test.rs:123",
    "example": "workflows/example-mapreduce.yml:67"
  },
  "resume_behavior": {
    "definition": "src/mapreduce/resume.rs:89",
    "usage": "tests/resume_test.rs:234"
  }
}
```

**Step 6: Add Source Attribution to Code Examples**

When adding or updating code examples in the documentation:

**YAML Examples:**
```markdown
## Configuration

```yaml
# Source: workflows/example-mapreduce.yml
name: example-workflow
mode: mapreduce
# ... rest of example
```
```

**Rust Code Examples:**
```markdown
## Data Structure

```rust
// Source: src/config/mapreduce.rs:45-67
pub struct CheckpointConfig {
    pub enabled: bool,
    pub interval: Duration,
}
```
```

**Test Examples:**
```markdown
## Usage Example

```rust
// Source: tests/mapreduce_test.rs:123-145
#[test]
fn test_checkpoint_creation() {
    // ... example from actual test
}
```
```

**Attribution Format Guidelines:**
- Use `# Source: path/to/file.ext` for YAML/config files
- Use `// Source: path/to/file.ext:line` for code files
- Use `<!-- Source: path/to/file.ext -->` for markdown/HTML
- Include line numbers when referencing specific implementations
- Use line ranges (e.g., `:45-67`) for multi-line code blocks

**Step 7: Validate Source References**

Before committing:
- Ensure all referenced files exist
- Verify line numbers are accurate
- Check that examples match actual code
- Remove references to files that have moved/been deleted

### Phase 4: Fix Drift Issues

Process issues by severity:

**For Each High Severity Issue:**

1. **Missing Content:**
   - Add new section with proper heading
   - Include explanation from feature inventory
   - Add code examples with source attribution (from Phase 3.5)
   - Reference implementation details

2. **Outdated Information:**
   - Update incorrect content
   - Fix deprecated syntax
   - Update examples to current version
   - Add version notes if needed

**For Each Medium Severity Issue:**

1. **Incorrect Examples:**
   - Fix YAML/code syntax
   - Add missing required fields
   - Update to current API
   - Verify examples work

2. **Incomplete Explanation:**
   - Expand brief mentions
   - Add usage examples
   - Clarify use cases
   - Add diagrams if helpful

3. **Broken Links:**
   - Update file paths to current locations
   - Fix relative path issues
   - Use suggestions from drift report
   - Verify all links resolve

**For Each Low Severity Issue:**

1. **Missing Best Practices** (only if appropriate for page type):
   - Add best practices section
   - Include common patterns
   - Note gotchas and pitfalls
   - Provide optimization tips

2. **Unclear Content:**
   - Improve explanations
   - Reorganize sections
   - Add clarifying examples
   - Simplify complex language

### Phase 5: Enhance with MkDocs Material Features

**Use appropriate Material for MkDocs extensions:**

**Admonitions for important notes:**
```markdown
!!! note
    Important information users should know

!!! warning
    Common pitfall or gotcha

!!! tip
    Best practice or optimization

!!! example
    Practical usage example
```

**Code blocks with syntax highlighting:**
```markdown
```yaml title="workflow.yml"
name: example
mode: mapreduce
```
```

**Tabs for multiple options:**
```markdown
=== "Option 1"
    First approach...

=== "Option 2"
    Alternative approach...
```

**Content tabs for different languages/frameworks:**
```markdown
=== "Rust"
    ```rust
    // Rust example
    ```

=== "YAML"
    ```yaml
    # YAML example
    ```
```

### Phase 6: Preserve Documentation Structure

**Maintain:**
- Existing section order (unless reorganization needed)
- Cross-references to other pages
- Code example formatting
- Version compatibility notes
- Anchor links for deep linking

**Update:**
- Outdated examples
- Incorrect syntax
- Broken links
- Missing features

### Phase 7: Validate Changes

**Check:**
- All drift issues addressed
- Examples are syntactically correct
- Links are valid
- MkDocs extensions used correctly
- No typos introduced
- Source attributions added (from Phase 3.5)

**Test:**
```bash
# Validate markdown syntax
markdownlint "$ITEM_FILE" || true

# Check for broken links (basic check)
grep -o '\[.*\](.*\.md[^)]*)' "$ITEM_FILE" | while read link; do
  # Extract target
  target=$(echo "$link" | sed 's/.*(\(.*\))/\1/')
  # Check if exists (relative to file)
  if [ ! -f "$(dirname "$ITEM_FILE")/$target" ]; then
    echo "Warning: Broken link to $target"
  fi
done
```

### Phase 8: Commit Changes

**CRITICAL**: Changes must be committed for validation phase to access them.

**Create Commit:**
```bash
git add "$ITEM_FILE"
git commit -m "docs: fix drift in $PROJECT_NAME page '$ITEM_TITLE'

Fixed $TOTAL_ISSUES issue(s):
$(echo "$DRIFT_DATA" | jq -r '.issues[] | "- [\(.severity)] \(.description)"' | head -5)

Quality improved from '$(echo "$DRIFT_DATA" | jq -r '.quality_assessment')' to expected standard."
```

**Verify Commit:**
```bash
git log -1 --stat
```

### Phase 9: Quality Guidelines

**Accuracy:**
- Fix all high and medium severity issues
- Address as many low severity issues as feasible
- Don't introduce new errors
- Verify examples work

**Clarity:**
- Use clear, concise language
- Organize content logically
- Provide concrete examples
- Explain the "why" not just the "what"

**Completeness:**
- Cover all features in scope
- Include common use cases
- Address known pitfalls
- Provide troubleshooting guidance (where appropriate)

**Consistency:**
- Follow MkDocs Material conventions
- Match existing documentation style
- Use consistent terminology
- Maintain navigation structure

### Success Indicators

The fix is successful when:
- All high severity drift issues resolved
- Most medium severity issues addressed
- Examples are accurate and current
- Links are valid
- MkDocs Material features used appropriately
- Source attributions added to code examples
- Changes committed cleanly
- Documentation builds without errors
