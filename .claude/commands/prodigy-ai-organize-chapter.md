# /prodigy-ai-organize-chapter

Intelligently organize a large chapter into semantic subsections using template-based AI organization.

## Variables

- `--chapter-file <path>` - Path to the chapter markdown file (required)
- `--strategy <semantic|structural>` - Organization strategy (default: "semantic")
- `--template <template-name|auto>` - Template to use or "auto" for detection (default: "auto")
- `--max-subsections <number>` - Maximum number of subsections (default: 8)
- `--dry-run <boolean>` - Preview changes without applying (default: false)
- `--output <path>` - Path to write organization proposal JSON (default: "proposal.json")

## Execute

### Phase 1: Understanding and Setup

You are organizing a large mdBook chapter into logical subsections using semantic analysis and template matching. This provides more intelligent organization than mechanical H2-based splitting.

**Extract Parameters:**

```bash
CHAPTER_FILE="${chapter_file:?Error: --chapter-file is required}"
STRATEGY="${strategy:-semantic}"
TEMPLATE="${template:-auto}"
MAX_SUBSECTIONS="${max_subsections:-8}"
DRY_RUN="${dry_run:-false}"
OUTPUT="${output:-proposal.json}"
```

**Validate Inputs:**

```bash
if [ ! -f "$CHAPTER_FILE" ]; then
    echo "Error: Chapter file not found: $CHAPTER_FILE"
    exit 1
fi

if [ "$STRATEGY" != "semantic" ] && [ "$STRATEGY" != "structural" ]; then
    echo "Error: Invalid strategy '$STRATEGY'. Use 'semantic' or 'structural'"
    exit 1
fi
```

### Phase 2: Analyze Chapter Semantics

If strategy is "semantic", run semantic analysis:

```bash
if [ "$STRATEGY" = "semantic" ]; then
    echo "Running semantic analysis..."
    /prodigy-ai-analyze-chapter-semantics \
        --chapter-file "$CHAPTER_FILE" \
        --output "temp-semantics.json" \
        --max-clusters "$MAX_SUBSECTIONS"
fi
```

**Load semantic analysis results** and use clusters as proposed subsections.

### Phase 3: Template Matching

Automatically detect the best matching template:

#### Step 1: Analyze Chapter Characteristics

Extract chapter metadata:

1. **Chapter title** from file or first H1
2. **Section titles** (all H2 headings)
3. **Key topics** from content
4. **Chapter type indicators:**
   - Contains "quick start", "getting started" → quick-start template
   - Contains "configuration", "settings" → configuration-reference template
   - Contains "cli", "commands" → cli-reference template
   - Contains "api", "library" → api-documentation template
   - Contains "advanced", "complex" → advanced-features template
   - Contains "troubleshooting", "debug" → troubleshooting template
   - Contains "examples", "tutorials" → examples-tutorials template

#### Step 2: Load Available Templates

Read all templates from `workflows/data/book-templates/`:

```bash
TEMPLATE_DIR="workflows/data/book-templates"
if [ ! -d "$TEMPLATE_DIR" ]; then
    echo "Warning: Template directory not found: $TEMPLATE_DIR"
    echo "Using structural organization instead"
    STRATEGY="structural"
fi
```

#### Step 3: Score Templates

For each template, calculate a match score:

**Scoring Algorithm:**

1. **Title matching (10 points each):**
   - Chapter title contains template chapter_type keyword
   - Example: "MapReduce Workflows" contains "workflow" → +10 for advanced-features

2. **Section matching (5 points each):**
   - Chapter has H2 matching template subsection alias
   - Example: Chapter has "Quick Start" H2, template has "Quick Start" subsection → +5

3. **Topic matching (2 points each):**
   - Chapter topics overlap with template topics
   - Example: Chapter mentions "configuration", template topics include "configuration" → +2

4. **Calculate confidence:**
   ```
   confidence = template_score / sum(all_template_scores)
   ```

**Select best template:**

```bash
BEST_TEMPLATE=$(highest scoring template)
CONFIDENCE=$(confidence score 0.0-1.0)

if [ "$TEMPLATE" = "auto" ]; then
    TEMPLATE="$BEST_TEMPLATE"
    echo "Auto-detected template: $TEMPLATE (confidence: $CONFIDENCE)"
else
    echo "Using specified template: $TEMPLATE"
fi
```

### Phase 4: Apply Template to Semantic Clusters

Match semantic clusters to template subsections:

#### Step 1: Load Template

Read the selected template YAML file:

```bash
TEMPLATE_FILE="$TEMPLATE_DIR/${TEMPLATE}.yaml"
if [ ! -f "$TEMPLATE_FILE" ]; then
    echo "Error: Template file not found: $TEMPLATE_FILE"
    exit 1
fi
```

#### Step 2: Match Clusters to Template Subsections

For each semantic cluster:

1. **Find best template match:**
   - Compare cluster name to subsection names and aliases
   - Compare cluster topics to subsection topics
   - Use cluster type (introduction, configuration, advanced, examples)

2. **Use template's preferred naming:**
   - Replace cluster name with template's preferred name
   - Example: Cluster "Getting Started" → Template prefers "Quick Start"

3. **Apply template ordering:**
   - Sort clusters by template subsection order
   - Required subsections appear first
   - Optional subsections follow in template order

#### Step 3: Validate Against Template Rules

Check template validation rules:

1. **Required subsections present:**
   - Verify all required subsections have matching clusters
   - Warn if missing required content

2. **Max subsections respected:**
   - Ensure total subsections ≤ template max_subsections
   - Merge smallest clusters if over limit

3. **Naming consistency:**
   - Apply template naming rules (prefer X over Y)

### Phase 5: Generate Organization Proposal

Create a detailed proposal for the reorganization:

**Proposal Structure:**

```json
{
  "chapter": "book/src/mapreduce.md",
  "strategy": "semantic",
  "template_used": "advanced-features",
  "template_confidence": 0.85,
  "proposed_subsections": [
    {
      "title": "Getting Started",
      "sections": ["Quick Start", "Complete Structure"],
      "file": "book/src/mapreduce/getting-started.md",
      "rationale": "Introductory content for new users",
      "template_match": "Getting Started",
      "template_order": 1,
      "required": true
    },
    {
      "title": "Configuration",
      "sections": ["Environment Variables", "Backoff Strategies", "Error Collection"],
      "file": "book/src/mapreduce/configuration.md",
      "rationale": "All configuration-related topics grouped together",
      "template_match": "Configuration",
      "template_order": 2,
      "required": false
    },
    {
      "title": "State Management",
      "sections": ["Checkpoint and Resume", "Dead Letter Queue"],
      "file": "book/src/mapreduce/state-management.md",
      "rationale": "Runtime state and recovery mechanisms",
      "template_match": "State Management",
      "template_order": 3,
      "required": false
    }
  ],
  "validation": {
    "issues": [],
    "warnings": [
      "Template suggests 'Performance' subsection but no matching cluster found"
    ],
    "compliance_score": 0.92
  },
  "changes_required": [
    "Create directory: book/src/mapreduce/",
    "Create file: book/src/mapreduce/index.md",
    "Create file: book/src/mapreduce/getting-started.md",
    "Create file: book/src/mapreduce/configuration.md",
    "Create file: book/src/mapreduce/state-management.md",
    "Update: book/src/SUMMARY.md (add nested structure)",
    "Delete: book/src/mapreduce.md (after migration)"
  ]
}
```

**Write proposal:**

```bash
cat > "$OUTPUT" << 'EOF'
{JSON proposal content}
EOF
```

### Phase 6: Dry-Run vs Apply

#### Dry-Run Mode (--dry-run true)

Print preview of proposed changes:

```
AI Organization Proposal
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Chapter: book/src/mapreduce.md
Template: advanced-features (confidence: 0.85)
Strategy: semantic

Proposed Subsections (3):

1. Getting Started [REQUIRED]
   Template Match: Getting Started (order: 1)
   Sections:
     • Quick Start
     • Complete Structure
   File: book/src/mapreduce/getting-started.md
   Rationale: Introductory content for new users

2. Configuration
   Template Match: Configuration (order: 2)
   Sections:
     • Environment Variables
     • Backoff Strategies
     • Error Collection
   File: book/src/mapreduce/configuration.md
   Rationale: All configuration-related topics

3. State Management
   Template Match: State Management (order: 3)
   Sections:
     • Checkpoint and Resume
     • Dead Letter Queue
   File: book/src/mapreduce/state-management.md
   Rationale: Runtime state and recovery

Validation:
  ✓ All required subsections present
  ⚠ Template suggests 'Performance' subsection (not found)
  Compliance Score: 92%

Changes Required:
  [CREATE] book/src/mapreduce/ (directory)
  [CREATE] book/src/mapreduce/index.md
  [CREATE] book/src/mapreduce/getting-started.md
  [CREATE] book/src/mapreduce/configuration.md
  [CREATE] book/src/mapreduce/state-management.md
  [UPDATE] book/src/SUMMARY.md
  [DELETE] book/src/mapreduce.md

Proposal saved to: proposal.json

Run without --dry-run to apply organization.
```

#### Apply Mode (--dry-run false)

Execute the reorganization:

**Step 1: Create subdirectory structure**

```bash
CHAPTER_DIR=$(dirname "$CHAPTER_FILE")/$(basename "$CHAPTER_FILE" .md)
mkdir -p "$CHAPTER_DIR"
```

**Step 2: Create index.md**

The index.md contains:
- Original chapter introduction (content before first H2)
- First 1-2 subsections (as specified by template)
- Navigation links to other subsections

**Step 3: Create subsection files**

For each proposed subsection:

1. Create markdown file with subsection name
2. Include all H2 sections listed for that subsection
3. Preserve all content, code blocks, nested headings
4. Add navigation links (back to index, next/prev subsection)

**Step 4: Update SUMMARY.md**

Replace:
```markdown
- [MapReduce Workflows](mapreduce.md)
```

With:
```markdown
- [MapReduce Workflows](mapreduce/index.md)
  - [Getting Started](mapreduce/getting-started.md)
  - [Configuration](mapreduce/configuration.md)
  - [State Management](mapreduce/state-management.md)
```

**Step 5: Update cross-references**

Scan all markdown files for links to the original chapter and update:
- `mapreduce.md` → `mapreduce/index.md`
- `mapreduce.md#section` → `mapreduce/subsection.md` (resolve anchor to file)

**Step 6: Validate mdBook build**

```bash
cd book && mdbook build

if [ $? -ne 0 ]; then
    echo "Error: mdbook build failed after reorganization"
    echo "Please review changes and fix issues"
    exit 1
fi
```

**Step 7: Print summary**

```
✓ AI Organization Complete
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Template Used: advanced-features
Subsections Created: 3

  ✓ book/src/mapreduce/index.md
  ✓ book/src/mapreduce/getting-started.md
  ✓ book/src/mapreduce/configuration.md
  ✓ book/src/mapreduce/state-management.md

  ✓ Updated book/src/SUMMARY.md
  ✓ Updated 12 cross-references
  ✓ Validated mdbook build succeeds

Next Steps:
  • Review changes: git diff
  • Preview book: cd book && mdbook serve
  • Commit changes: git add -A && git commit
```

### Phase 7: Git Operations (if applying)

If not dry-run:

```bash
if [ "$DRY_RUN" = "false" ]; then
    cd book
    git add -A
    git commit -m "docs: AI-organized chapter using $TEMPLATE template

Applied semantic organization to $(basename $CHAPTER_FILE):
- Created $NUM_SUBSECTIONS subsections
- Used $TEMPLATE template (confidence: $CONFIDENCE)
- Updated SUMMARY.md and cross-references
- mdbook build validated"
fi
```

### Quality Guidelines

1. **Content Preservation:**
   - Never lose content during reorganization
   - Preserve all formatting, code blocks, images
   - Maintain heading hierarchy

2. **Template Compliance:**
   - Use template naming conventions
   - Follow template ordering
   - Respect required/optional subsections

3. **Cross-Project Consistency:**
   - Similar chapters use same template across projects
   - Naming is standardized (e.g., always "Quick Start")
   - Organization patterns are predictable

4. **User Control:**
   - Dry-run shows exactly what will change
   - Template can be manually specified
   - Proposal can be reviewed before applying

### Success Criteria

- [ ] Chapter analyzed semantically or structurally
- [ ] Best matching template detected (if auto)
- [ ] Semantic clusters matched to template subsections
- [ ] Template naming applied consistently
- [ ] Organization proposal generated with rationale
- [ ] Dry-run shows clear preview of changes
- [ ] Apply mode creates correct subsection structure
- [ ] SUMMARY.md updated with nested structure
- [ ] All cross-references updated correctly
- [ ] mdbook build succeeds after reorganization
- [ ] Changes committed to git (if applying)

### Error Handling

**No template found:**
```
Error: Template not found: custom-template
Available templates:
  - quick-start
  - configuration-reference
  - cli-reference
  - advanced-features
  - troubleshooting
  - examples-tutorials
  - api-documentation
```

**mdbook build fails:**
```
Error: mdbook build failed after reorganization
Common issues:
  - Broken link to subsection
  - Missing file in SUMMARY.md
  - Invalid markdown syntax

Review error output above and fix issues.
```

**Template mismatch:**
```
Warning: Low template confidence (0.42)
The chapter may not fit standard templates well.
Consider:
  - Using --template to specify different template
  - Using --strategy structural for mechanical split
  - Manually organizing this chapter
```
