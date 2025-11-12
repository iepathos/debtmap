# /prodigy-fix-subsection-drift

Fix documentation drift for a specific chapter or subsection based on its drift analysis report.

This command supports both single-file chapters (backward compatible) and individual subsections within multi-subsection chapters.

## Variables

- `--project <name>` - Project name (e.g., "Prodigy", "Debtmap")
- `--json <item>` - JSON object containing chapter or subsection details
- `--chapter-id <id>` - (Optional) Chapter ID for subsections
- `--subsection-id <id>` - (Optional) Subsection ID if fixing a subsection

## Execute

### Phase 1: Understand Context

You are fixing documentation drift for either a full chapter or a single subsection. The analysis phase has already created a drift report. Your job is to:
1. Read the drift report
2. Fix all identified issues
3. Update the documentation file
4. Preserve cross-references (especially important for subsections)
5. Commit the changes

**Important for Subsections:**
- Only update the specific subsection file
- Preserve links to other subsections
- Maintain subsection scope and focus
- Don't accidentally modify sibling subsections

### Phase 1.5: Anti-Pattern Prevention (CRITICAL)

**DO NOT add these sections unless explicitly appropriate:**

1. **"Prerequisites" sections** - Only add to:
   - installation.md files (e.g., automated-documentation/installation.md)
   - Root-level getting-started.md or setup guides
   - NOT to chapter index.md files (link to installation.md instead)
   - NOT to subsections or technical reference pages

   **Replacement pattern:**
   ```markdown
   ## Prerequisites

   Before getting started, ensure you have:
   - [Prerequisite 1](installation.md#prerequisite-1)
   - [Prerequisite 2](installation.md#prerequisite-2)

   See the [Installation Guide](installation.md) for detailed setup instructions.
   ```

2. **"Installation" sections** - Only add to:
   - Dedicated installation.md files within each major chapter
   - Root-level installation.md
   - NOT to chapter index.md files
   - NOT to subsections or feature documentation

3. **"Best Practices" sections** - Only add to:
   - Chapter-level index.md files
   - Dedicated best-practices.md files
   - Standalone guide pages at book root
   - NOT to technical reference pages, syntax documentation, or subsections

4. **"See Also" sections** - Only add when:
   - There's a specific prerequisite relationship
   - There's a non-obvious connection between topics
   - NOT a generic list of all other chapter subsections
   - NOT circular references (subsection A → subsection B → subsection A)

5. **"Troubleshooting" sections** - Only add to:
   - Complex features with common pitfalls
   - Chapter-level troubleshooting.md files
   - NOT to simple syntax reference pages
   - NOT to files documenting straightforward configuration options

6. **"Next Steps" / "Related Topics" / "Further Reading"** - Consolidate into:
   - A single section in chapter index.md
   - NOT separate stub files (<100 lines)
   - NOT repeated in every subsection

7. **"Quick Start" sections** - Only add to:
   - Dedicated quick-start.md files
   - Chapter index.md for major features
   - NOT repeated in multiple files within the same chapter
   - NOT in subsections (use "Usage" instead)

**Detection Logic:**
```bash
# Check document type
IS_INDEX_FILE=false
IS_SUBSECTION=false
IS_REFERENCE_PAGE=false

if [[ "$ITEM_FILE" == */index.md ]]; then
  IS_INDEX_FILE=true
elif [[ "$ITEM_FILE" == */* ]] && [ -f "$(dirname "$ITEM_FILE")/index.md" ]; then
  IS_SUBSECTION=true
fi

# Detect reference pages (syntax, configuration, API docs)
if grep -qi "syntax\|configuration\|reference\|api" "$ITEM_FILE" | head -1; then
  IS_REFERENCE_PAGE=true
fi
```

**Before adding any meta-section:**
- Check if chapter already has dedicated file (best-practices.md, troubleshooting.md)
- Verify file is appropriate type (index vs subsection vs reference)
- Ensure content adds value, not just boilerplate

**Special Case: automated-documentation/index.md**
- DO NOT add full Prerequisites list (link to installation.md)
- DO NOT add detailed Installation instructions (link to installation.md)
- DO NOT add multiple Quick Start sections (provide brief overview + links to quick-start.md and tutorial.md)
- DO include: Overview, How It Works, organized links to related topics

**Prerequisite Consolidation Check:**
```bash
# Detect prerequisites in non-installation files
if [[ "$ITEM_FILE" =~ automated-documentation/index\.md$ ]] && grep -q "^## Prerequisites" "$ITEM_FILE"; then
  echo "WARNING: Prerequisites section found in index.md - should link to installation.md instead"
fi

# Check for installation instructions in index files
if [[ "$ITEM_FILE" =~ /index\.md$ ]] && grep -qi "cargo install\|rustup\|npm install" "$ITEM_FILE"; then
  echo "WARNING: Installation commands found in index.md - should link to installation.md instead"
fi
```

### Phase 2: Parse Input and Load Drift Report

**Extract Parameters:**
```bash
PROJECT_NAME="<value from --project parameter>"
ITEM_JSON="<value from --json parameter, may be empty>"
CHAPTER_ID="<value from --chapter-id parameter, may be empty>"
SUBSECTION_ID="<value from --subsection-id parameter, may be empty>"
```

**Determine Item Type and IDs:**

If `ITEM_JSON` is provided:
```bash
ITEM_TYPE=$(echo "$ITEM_JSON" | jq -r '.type // "single-file"')
ITEM_ID=$(echo "$ITEM_JSON" | jq -r '.id')

if [ "$ITEM_TYPE" = "subsection" ]; then
  PARENT_CHAPTER_ID=$(echo "$ITEM_JSON" | jq -r '.parent_chapter_id')
  SUBSECTION_ID="$ITEM_ID"
else
  CHAPTER_ID="$ITEM_ID"
fi
```

If using separate parameters:
```bash
if [ -n "$SUBSECTION_ID" ]; then
  ITEM_TYPE="subsection"
  PARENT_CHAPTER_ID="$CHAPTER_ID"
else
  ITEM_TYPE="chapter"
fi
```

**Determine Drift Report Path:**

**For Subsections:**
- Pattern: `.prodigy/book-analysis/drift-${PARENT_CHAPTER_ID}-${SUBSECTION_ID}.json`
- Example: `.prodigy/book-analysis/drift-mapreduce-checkpoint-and-resume.json`

**For Single-File Chapters:**
- Pattern: `.prodigy/book-analysis/drift-${CHAPTER_ID}.json`
- Example: `.prodigy/book-analysis/drift-workflow-basics.json`

**Load Drift Report:**
Read the drift report JSON file to extract:
- `item_file` or `chapter_file` or `subsection_file`: Path to markdown file
- `issues[]`: List of drift issues with fix suggestions
- `severity`: Overall drift severity
- `improvement_suggestions[]`: Additional recommendations
- `cross_references[]`: Related subsections (for subsections)
- `feature_mappings[]`: Scoped features (for subsections)

### Phase 3: Analyze Drift Issues

**Parse Issues:**
For each issue in the drift report:
- Identify section that needs updating
- Understand what content is missing/outdated/incorrect
- Review `fix_suggestion` and `source_reference`
- Check `current_content` vs `should_be` if provided

**Prioritize Fixes:**
1. **Critical severity** - Missing entire sections, completely outdated
2. **High severity** - Major features undocumented, incorrect examples
3. **Medium severity** - Incomplete explanations, minor inaccuracies
4. **Low severity** - Style issues, missing cross-references

### Phase 3.5: Extract Real Examples from Codebase (MANDATORY)

**CRITICAL: ALL documentation content must be grounded in actual codebase implementation.**

**Step 1: Identify What Needs Code Examples**

From the drift report issues, identify what requires code validation:
- Struct definitions and field names
- YAML syntax and configuration options
- CLI command syntax
- Enum variants
- Function signatures
- Workflow examples

**Step 2: Discover Codebase Structure**

**CRITICAL: Do not assume project structure. Discover it first.**

Before searching for feature code, understand the codebase organization:

```bash
# Discover test locations (common patterns)
TEST_DIRS=$(find . -type d -name "*test*" -o -name "*spec*" | grep -v node_modules | grep -v .git | head -5)

# Discover example/workflow locations (common patterns)
EXAMPLE_DIRS=$(find . -type d -name "*example*" -o -name "*workflow*" -o -name "*sample*" | grep -v node_modules | grep -v .git | head -5)

# Discover source code locations (exclude common non-source directories)
SOURCE_DIRS=$(find . -type f \( -name "*.rs" -o -name "*.py" -o -name "*.js" -o -name "*.ts" -o -name "*.go" -o -name "*.java" \) | sed 's|/[^/]*$||' | sort -u | grep -v node_modules | grep -v .git | head -10)
```

**Step 3: Search for Source Definitions (Language-Agnostic)**

For each feature being documented, **MANDATORY searches:**

**A. Find Type/Struct/Class Definitions:**

Use Claude's Explore agent for intelligent discovery:
```
Task: Find the definition of ${StructName} in the codebase
- Search for struct, class, interface, or type definitions
- Look in source directories discovered above
- Return file path and line numbers
```

Fallback to direct search if needed:
```bash
# Language-agnostic patterns for type definitions
# Rust: struct, enum
# Python: class, TypedDict, dataclass
# TypeScript: interface, type, class
# Go: struct, interface
rg "(struct|class|interface|type|enum)\s+${StructName}" --hidden --iglob '!.git' -A 10
```

**B. Find Field/Property Definitions:**
```bash
# After finding the type definition file, extract fields
# This works across many languages (struct fields, class properties, interface members)
rg "^\s*\w+:" ${TYPE_DEFINITION_FILE} -A 2
```

**C. Find Real Usage in Tests:**

Use Claude's Explore agent for intelligent test discovery:
```
Task: Find test files that use ${FeatureName}
- Search in test directories discovered above
- Look for instantiation, usage, or assertion patterns
- Return relevant code snippets with context
```

Fallback to direct search:
```bash
# Search discovered test directories
for test_dir in $TEST_DIRS; do
  rg "${FeatureName}" "$test_dir" -A 10 --hidden
done
```

**D. Find Real Examples/Workflows:**

Use Claude's Explore agent for example discovery:
```
Task: Find example files or workflows that demonstrate ${FeatureName}
- Search in example/workflow directories
- Look for YAML, JSON, TOML config files
- Look for documented code examples
- Return file paths and relevant sections
```

Fallback to direct search:
```bash
# Search discovered example directories for config usage
for example_dir in $EXAMPLE_DIRS; do
  rg "${yaml_field_name}" "$example_dir" -A 5 --hidden
done
```

**E. Find Existing Documentation Examples:**
```bash
# Check if other documentation has validated examples
rg "${feature_name}" book/src/ -A 10 --hidden

# Only reuse these if they reference source code
```

**Step 4: Validate All Examples**

**For Configuration Examples (YAML/JSON/TOML):**
```bash
# Check field names exist in type definition
# Pattern: For each field in config example, verify it exists in source

# Example validation:
# Config shows:   retry_config:
# Source check: rg "retry_config" --hidden --iglob '!.git' -w
# Result:       MUST FIND MATCH or DON'T USE

# Verify the field is actually a configuration field, not a random match
```

**For Code Examples:**
```bash
# Verify variants/values exist in type definitions
# Pattern: Check each value/variant mentioned

# Example:
# Docs show:   backoff: exponential
# Source check: rg "exponential|Exponential" --hidden --iglob '!.git' -w
# Result:       MUST FIND MATCH (case-insensitive search, but document exact case from source)
```

**For CLI Commands:**
```bash
# Verify command syntax from help text or parser code
# Pattern: Look for CLI argument definitions

# Example:
# Docs show:   prodigy run workflow.yml --profile prod
# Source check: rg "profile.*flag|flag.*profile|arg.*profile" --hidden --iglob '!.git' -i
# Result:       Verify flag exists and document exact syntax from source
```

**Step 5: Extract Real Examples**

**Template for Code-Grounded Examples:**
```markdown
## Configuration

The `RetryConfig` type defines retry behavior (path/to/config.file:45):

\`\`\`yaml
retry_config:
  max_attempts: 3           # Maximum retry attempts (default: 3)
  initial_delay_ms: 100     # Initial delay in milliseconds (default: 100)
  backoff: exponential      # Backoff strategy: exponential, linear, fibonacci
  max_delay_ms: 60000       # Maximum delay cap (default: 60000)
\`\`\`

**Source**: Extracted from configuration type definition in path/to/config.file:45-52

**Backoff Strategies** (from path/to/types.file:BackoffStrategy definition):
- `exponential` - Delay doubles each retry (2^n * initial_delay)
- `linear` - Delay increases linearly (n * initial_delay)
- `fibonacci` - Delay follows fibonacci sequence

## Real-World Example

From path/to/test/file:78-92:

\`\`\`yaml
name: reliable-workflow
retry_config:
  max_attempts: 5
  initial_delay_ms: 500
  backoff: exponential
  max_delay_ms: 30000
\`\`\`
```

**Note**: Replace generic paths with actual discovered file paths from Step 2-3.

**Step 6: Rules for Content Creation**

**ALWAYS:**
- Include source file references for all examples (e.g., "path/to/config.file:45")
- Link to actual test/example files for real-world usage
- Verify field/property names match type definitions exactly
- Verify enum/constant values match source code exactly (document exact case from source)
- Extract examples from actual configuration/example files discovered in Step 2
- Note which features are optional vs required based on type definition
- Use language-agnostic terminology (type instead of struct, property instead of field)

**NEVER:**
- Invent plausible-looking syntax (YAML, JSON, code examples)
- Guess field/property names or types
- Create examples from "common patterns" unless proven in codebase
- Use syntax from other tools or projects
- Assume features exist without verification
- Document features that don't exist in the codebase
- Assume file locations or project structure

**If No Example Exists:**
```markdown
## Usage

This feature is defined in path/to/implementation.file but no examples currently demonstrate it.

See the type definition for available configuration:
- [Source Code](../path/to/implementation.file:line)

**Note**: If you use this feature, consider contributing a documented example!
```

**Step 7: Create Evidence File**

For each subsection/chapter, create a temporary evidence file documenting sources:

```bash
# Create evidence file
cat > .prodigy/book-analysis/evidence-${ITEM_ID}.md <<EOF
# Evidence for ${ITEM_TITLE}

## Source Definitions Found
- RetryConfig type: path/to/config.file:45
- BackoffStrategy type: path/to/types.file:88
- retry_config property: path/to/workflow-config.file:123

## Test Examples Found
- path/to/test/retry_test.file:78 (complete workflow)
- path/to/test/config_test.file:45 (type construction)

## Configuration Examples Found
- path/to/examples/ci-workflow.yml:23 (retry_config usage)
- path/to/examples/sample.json:15 (JSON config example)

## Documentation References
- book/src/error-handling.md:156 (related concept)

## Validation Results
✓ All config fields verified against type definition
✓ All enum/constant values match source
✓ CLI syntax verified against parser definitions
✗ No real-world examples found (using test example instead)

## Discovery Notes
- Test directories found: ${TEST_DIRS}
- Example directories found: ${EXAMPLE_DIRS}
- Source directories searched: ${SOURCE_DIRS}
EOF
```

This evidence file helps verify all content is grounded and provides audit trail.

### Phase 3.75: Resolve and Validate All Links (MANDATORY)

**CRITICAL: All internal links must point to valid files.**

Before writing any markdown content with links, you must discover what documentation exists and calculate correct paths.

**Step 1: Build Documentation Inventory**

Scan `${BOOK_DIR}/src/` to discover all available markdown files:
1. Find all `.md` files recursively
2. For each file, record:
   - Full relative path from `book/src/` (e.g., `mapreduce/checkpoint-and-resume.md`)
   - Chapter ID (directory name or filename without `.md`)
   - Title (extracted from first H1 or H2 heading in the file)
   - Whether it's a subsection (path contains `/`) or single-file chapter
3. Build a lookup map with these keys:
   - By chapter ID: `"mapreduce" → "mapreduce/index.md"`
   - By file path: `"mapreduce/checkpoint-and-resume.md" → "Checkpoint and Resume"`
   - By normalized title: `"checkpoint and resume" → "mapreduce/checkpoint-and-resume.md"`

**Step 2: Calculate Relative Path Prefix for Current File**

Determine where the current file is located (from `$ITEM_FILE` in drift report):
1. Count how many `/` characters are in the path
2. Calculate relative prefix:
   - Depth 1 (e.g., `book/src/intro.md`): No prefix needed
   - Depth 2 (e.g., `book/src/mapreduce/index.md`): Use `../`
   - Depth 3 (e.g., `book/src/mapreduce/subsection/file.md`): Use `../../`
   - Depth N: Use `../` repeated (N-1) times

**Step 3: Resolve Documentation References to Valid Paths**

When you need to link to a chapter or subsection, resolve the reference using this strategy:
1. Normalize the reference (lowercase, replace spaces with dashes)
2. Try these lookups in order:
   - **Direct chapter ID match**: Check if it's in the chapter ID map
   - **Title match**: Check if it matches a title in the map
   - **Directory with index**: Check if `${BOOK_DIR}/src/{normalized}/index.md` exists
   - **Single file**: Check if `${BOOK_DIR}/src/{normalized}.md` exists
   - **Fuzzy match**: Search for chapter IDs containing the normalized reference
3. If a match is found, return the relative path
4. If NO match is found, **flag as error** - do not create this link

**Step 4: Generate Links with Correct Paths**

When writing markdown links in your documentation:
1. For each cross-reference, use the resolution strategy from Step 3
2. Prepend the relative prefix from Step 2
3. **Examples:**
   - Current file: `book/src/workflow-basics/next-steps.md` (depth 2, prefix `../`)
   - Link to "Advanced Features": Resolve to `advanced/index.md`, full link: `../advanced/index.md`
   - Link to "Command Types": Resolve to `commands.md`, full link: `../commands.md`
   - Link to "MapReduce": Resolve to `mapreduce/index.md`, full link: `../mapreduce/index.md`

**Step 5: Validate All Links Before Committing**

After writing your content, extract and validate all internal links:
1. Find all markdown links: `[text](path)`
2. For each internal link (not starting with `http://` or `https://`):
   - Resolve the full file path relative to current file
   - Check if the target file exists
   - If it doesn't exist, **this is an error** - the link is broken
3. **If ANY broken links are found**, fix them before committing
4. Report: "Validated X links, all valid" or "Found Y broken links, fixed them"

**Link Generation Rules:**

1. **NEVER hard-code paths** like `advanced.md`, `commands.md`, `environment.md`
2. **ALWAYS resolve paths** using the discovery and lookup process
3. **ALWAYS validate** that target files exist
4. **Use correct relative prefixes** based on file depth
5. **For same-chapter subsections**, you can use just the filename (e.g., `other-subsection.md`)

### Phase 4: Fix the Documentation

**Read Current File:**
Read the markdown file from the drift report.

**Apply Fixes Based on Item Type:**

**For Subsections:**

1. **Maintain Subsection Scope:**
   - Only add content related to `feature_mappings`
   - Don't document features outside subsection scope
   - Keep content focused on subsection topics

2. **Preserve Cross-References:**
   - Maintain links to sibling subsections
   - Verify cross-references listed in drift report
   - Add new cross-references if needed
   - Example: Checkpoint subsection links to DLQ subsection

3. **Respect Chapter Context:**
   - Ensure subsection fits within parent chapter
   - Don't duplicate content from other subsections
   - Reference related subsections instead of duplicating

4. **Update Subsection Structure:**
   - Keep consistent heading levels (typically H2 and H3)
   - Maintain standard subsection structure
   - Follow parent chapter organization

**For Single-File Chapters:**

1. **Comprehensive Coverage:**
   - Address all major features in chapter scope
   - Ensure broad topic coverage
   - Include complete feature documentation

2. **Chapter Organization:**
   - Maintain logical flow and structure
   - Keep clear introduction and summary
   - Organize sections appropriately

**Common Fix Patterns (Both Types):**

**Missing Content Issues:**
- Add missing section/content
- Follow fix_suggestion guidance
- Include code examples
- Add cross-references

**Outdated Information Issues:**
- Update outdated content
- Replace old syntax with current
- Update examples to match implementation
- Add version notes if needed

**Incorrect Examples Issues:**
- Fix broken examples
- Verify syntax is correct
- Test examples work with current code
- Add explanatory comments

**Incomplete Explanation Issues:**
- Expand brief explanations
- Add practical examples
- Include use cases
- Link to relevant source code

**Preserve Good Content:**
- Keep content from `positive_aspects`
- Maintain chapter/subsection structure and flow
- Preserve working examples
- Keep helpful diagrams

**Apply Improvement Suggestions:**
- Add cross-references
- Include best practices
- Add troubleshooting tips
- Improve organization if needed

### Phase 5: Quality Checks

**For Subsections:**
- Verify content stays within subsection scope
- Check cross-references to other subsections are valid
- Ensure no duplication with sibling subsections
- Validate subsection fits in chapter context

**For Chapters:**
- Verify comprehensive topic coverage
- Check overall structure is logical
- Ensure proper introduction and conclusion

**General Checks:**
- All critical and high severity issues addressed
- **All internal links are valid** (verified against doc_link_map)
- Links use correct relative paths
- No broken cross-references
- All topics from metadata covered
- Examples are practical and current
- Cross-references are valid
- Content is accurate against source code
- Field names and types are correct
- Examples parse correctly
- CLI commands match current syntax

### Phase 5.5: Validate Minimum Content Requirements (MANDATORY)

**CRITICAL: Subsections and chapters MUST meet minimum quality standards before committing.**

**Step 1: Count Lines and Content**

```bash
# Get actual content line count (excluding blank lines and single-word headers)
LINE_COUNT=$(grep -v '^$' ${ITEM_FILE} | grep -v '^#\s*$' | wc -l)
HEADING_COUNT=$(grep '^##' ${ITEM_FILE} | wc -l)
CODE_BLOCK_COUNT=$(grep '```' ${ITEM_FILE} | wc -l)
```

**Step 2: Minimum Content Thresholds**

**For Subsections:**
- **Minimum 50 lines** of actual content (excluding blank lines)
- **Minimum 3 level-2 headings** (## sections)
- **Minimum 2 code examples** (``` blocks)
- **Minimum 1 source reference** to codebase files

**For Single-File Chapters:**
- **Minimum 100 lines** of actual content
- **Minimum 5 level-2 headings**
- **Minimum 3 code examples**
- **Minimum 2 source references**

**Step 3: Content Completeness Check**

**Verify all drift issues addressed:**
```bash
# Count issues by severity from drift report
CRITICAL_ISSUES=$(jq '.issues[] | select(.severity == "critical")' ${DRIFT_REPORT} | jq -s length)
HIGH_ISSUES=$(jq '.issues[] | select(.severity == "high")' ${DRIFT_REPORT} | jq -s length)

# ALL critical and high severity issues MUST be resolved
# Check that updated file addresses each issue's section
```

**Required sections for subsections:**
- Overview/Introduction (what this subsection covers)
- Configuration or Syntax (if applicable)
- At least one practical example
- Best practices or common patterns (if material exists in codebase)
- Cross-references to related subsections (if applicable)

**Step 4: Validation Decision Tree**

**If content meets ALL thresholds:**
- Proceed to Phase 6 (commit)

**If content is too short (< 50 lines for subsection, < 100 for chapter):**

1. **Check if content genuinely doesn't exist in codebase:**
   ```bash
   # Count how many source files relate to this feature
   # Use discovered directories from Phase 3.5 Step 2
   SOURCE_FILE_COUNT=$(rg "${feature_name}" --hidden --iglob '!.git' --iglob '!node_modules' -l | wc -l)

   # If < 3 source files, feature may be too small for subsection
   ```

2. **If feature is genuinely small (<3 source files, <50 lines possible):**
   - Add a prominent note at the top:
   ```markdown
   # ${SUBSECTION_TITLE}

   > **Note**: This feature has minimal implementation. Consider reviewing:
   > - ${PARENT_CHAPTER_ID}/index.md for overview
   > - Source: src/path/to/implementation.rs

   ## Overview

   ${Brief description}

   ## Configuration

   ${Minimal config example from source}

   ## See Also

   - [Related feature](../related.md)
   ```
   - Add warning to commit message: "MINIMAL CONTENT - feature has limited implementation"

3. **If content SHOULD exist but you couldn't find it:**
   - DO NOT COMMIT stub/minimal content
   - Instead, create a TODO file:
   ```bash
   cat > ${ITEM_FILE}.TODO <<EOF
   # TODO: ${SUBSECTION_TITLE}

   This subsection needs substantial content but insufficient material was found in the codebase.

   ## Issues Identified (from drift report)
   $(jq '.issues[] | "- [\(.severity)] \(.description)"' ${DRIFT_REPORT})

   ## Searches Performed
   - Searched for type definitions: ${SEARCHES_DONE}
   - Searched test directories for examples: ${TEST_SEARCHES}
   - Searched example/config directories for usage: ${EXAMPLE_SEARCHES}

   ## Next Steps
   1. Verify feature is implemented (check if feature_mapping is correct)
   2. If implemented, search with different keywords
   3. If not implemented, remove subsection or mark as "Planned Feature"
   4. If implemented but undocumented in code, add rustdoc first

   ## Drift Report
   See: ${DRIFT_REPORT}
   EOF
   ```
   - Log error message:
   ```
   ❌ Cannot fix ${ITEM_TYPE} '${ITEM_TITLE}': insufficient content found in codebase

   Created TODO file: ${ITEM_FILE}.TODO

   Possible reasons:
   1. Feature not yet implemented
   2. Feature_mapping in chapter definition is incorrect
   3. Search keywords need adjustment
   4. Feature exists but needs better code documentation

   Recommended action: Review drift report and verify feature exists
   ```
   - EXIT WITHOUT COMMITTING

**Step 5: Example Quality Validation**

For each code example in the updated documentation:

```bash
# Verify example has source attribution
grep -q "Source:" ${ITEM_FILE} || echo "WARNING: Example missing source attribution"

# Verify example references actual files (extract file paths from markdown)
# Look for patterns like: path/to/file.ext:line or [Source Code](path/to/file.ext)
grep -oE '\([^)]*\.[a-z]{2,4}(:[0-9]+)?\)|\b[a-zA-Z0-9_./\-]+\.[a-z]{2,4}:[0-9]+' ${ITEM_FILE} | while read -r source_ref; do
  # Extract file path (remove parentheses, line numbers, etc)
  FILE_PATH=$(echo "$source_ref" | sed 's/[():].*//g')
  if [ -n "$FILE_PATH" ] && [ ! -f "$FILE_PATH" ]; then
    echo "ERROR: Referenced file does not exist: $FILE_PATH"
  fi
done
```

**All examples MUST:**
- Have a source attribution comment (e.g., "Source: src/config/retry.rs:45")
- Reference files that actually exist
- Use field names that exist in source code
- Use enum variants that match source code exactly

**Step 6: Validation Summary**

Create validation summary for commit message:

```bash
cat > .prodigy/book-analysis/validation-${ITEM_ID}.txt <<EOF
# Validation Summary for ${ITEM_TITLE}

## Content Metrics
- Lines of content: ${LINE_COUNT} (minimum: ${MIN_LINES})
- Headings: ${HEADING_COUNT} (minimum: ${MIN_HEADINGS})
- Code examples: ${CODE_BLOCK_COUNT} (minimum: ${MIN_EXAMPLES})
- Source references: ${SOURCE_REF_COUNT} (minimum: ${MIN_SOURCES})

## Drift Issues Resolved
- Critical: ${CRITICAL_FIXED}/${CRITICAL_ISSUES}
- High: ${HIGH_FIXED}/${HIGH_ISSUES}
- Medium: ${MEDIUM_FIXED}/${MEDIUM_ISSUES}
- Low: ${LOW_FIXED}/${LOW_ISSUES}

## Code Validation
- All struct fields verified: ${STRUCT_VALIDATION}
- All enum variants verified: ${ENUM_VALIDATION}
- All examples have source attribution: ${SOURCE_ATTRIBUTION}
- All referenced files exist: ${FILE_EXISTENCE}

## Quality Gates
✓ Meets minimum content requirements
✓ All critical issues resolved
✓ All high severity issues resolved
✓ All examples grounded in codebase
✓ All source references validated

Status: READY TO COMMIT
EOF
```

**If ANY quality gate fails:**
- DO NOT proceed to commit
- Create detailed TODO file explaining what's missing
- Exit with error message showing validation failures

### Phase 6: Commit the Fix

**Write Updated Documentation:**
Use the Edit tool to update the file with all fixes applied.

**Create Descriptive Commit:**

**For Subsections:**
```bash
CRITICAL_COUNT=<count of critical issues>
HIGH_COUNT=<count of high issues>
TOTAL_ISSUES=<total issues fixed>
SUBSECTION_TITLE="<from drift report>"
PARENT_CHAPTER_TITLE="<parent chapter title>"

git add <subsection_file>
git commit -m "docs: fix ${PROJECT_NAME} subsection '${PARENT_CHAPTER_TITLE} > ${SUBSECTION_TITLE}'

Fixed ${TOTAL_ISSUES} drift issues (${CRITICAL_COUNT} critical, ${HIGH_COUNT} high)

Key updates:
- <list 3-5 most important fixes>

Subsection scope: <feature mappings>
Cross-references preserved: <related subsections>"
```

**For Single-File Chapters:**
```bash
CHAPTER_TITLE="<from drift report>"

git add <chapter_file>
git commit -m "docs: fix ${PROJECT_NAME} book chapter '${CHAPTER_TITLE}'

Fixed ${TOTAL_ISSUES} drift issues (${CRITICAL_COUNT} critical, ${HIGH_COUNT} high)

Key updates:
- <list 3-5 most important fixes>

All examples verified against current implementation."
```

### Phase 7: Validation

**The fix should:**
1. Address all critical and high severity issues
2. Update outdated information to match current code
3. Fix all broken examples
4. Add missing content for major features
5. Preserve positive aspects from drift report
6. Include clear, tested examples
7. Be committed with descriptive message
8. Maintain subsection scope (for subsections)
9. Preserve cross-references (for subsections)

**Don't:**
- Skip critical issues due to complexity
- Add speculative content not in codebase
- Break existing working content
- Remove helpful examples or explanations
- Make unrelated changes
- Document features outside subsection scope (for subsections)
- Duplicate content from other subsections

### Phase 8: Summary Output

**For Subsections:**
```
✅ Fixed drift in ${PARENT_CHAPTER_TITLE} > ${SUBSECTION_TITLE}

Issues addressed:
- ${CRITICAL_COUNT} critical
- ${HIGH_COUNT} high
- ${MEDIUM_COUNT} medium
- ${LOW_COUNT} low

Changes:
- <brief summary of major updates>

Subsection updated: ${SUBSECTION_FILE}
Feature scope: ${FEATURE_MAPPINGS}
Cross-references: ${CROSS_REFS}
```

**For Single-File Chapters:**
```
✅ Fixed drift in ${CHAPTER_TITLE}

Issues addressed:
- ${CRITICAL_COUNT} critical
- ${HIGH_COUNT} high
- ${MEDIUM_COUNT} medium
- ${LOW_COUNT} low

Changes:
- <brief summary of major updates>

Chapter updated: ${CHAPTER_FILE}
```

## Notes

### Subsection-Specific Notes
- Each subsection runs in a separate map agent worktree
- Focus only on the assigned subsection
- Don't modify other subsections even if issues noticed
- Preserve all cross-references to sibling subsections
- Maintain subsection boundaries and scope
- Commits merge to parent worktree automatically

### General Notes
- This command runs during the **map phase** in a separate worktree
- Focus on accuracy - verify against source code
- Include practical, copy-paste ready examples
- Cross-reference related documentation
- The reduce phase handles any merge conflicts
