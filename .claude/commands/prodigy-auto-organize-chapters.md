# /prodigy-auto-organize-chapters

Automatically organize large mdBook chapters into subsections for better readability and maintainability.

## Variables

- `--book-dir <path>` - Path to the mdBook root directory (default: "book")
- `--min-h2-sections <number>` - Minimum number of H2 sections to trigger splitting (default: 6)
- `--min-lines <number>` - Minimum lines to trigger splitting (default: 400)
- `--preserve-index-sections <number>` - Number of H2 sections to keep in index.md (default: 2)
- `--dry-run <boolean>` - Preview changes without applying them (default: false)

## Execute

### Phase 1: Understand Context

You are organizing large mdBook chapters into logical subsections to improve readability and maintainability. This command:

1. Scans markdown files in the book
2. Identifies chapters exceeding size/complexity thresholds
3. Splits large chapters into subdirectories with index.md and subsection files
4. Updates SUMMARY.md with nested structure
5. Preserves all links and cross-references

### Phase 2: Parse Input Arguments

**Extract Parameters:**

```bash
# Set defaults
BOOK_DIR="${book_dir:-book}"
MIN_H2_SECTIONS="${min_h2_sections:-6}"
MIN_LINES="${min_lines:-400}"
PRESERVE_INDEX_SECTIONS="${preserve_index_sections:-2}"
DRY_RUN="${dry_run:-false}"
```

**Validate Book Directory:**

```bash
if [ ! -d "$BOOK_DIR/src" ]; then
    echo "Error: Book directory '$BOOK_DIR/src' not found"
    exit 1
fi

if [ ! -f "$BOOK_DIR/src/SUMMARY.md" ]; then
    echo "Error: SUMMARY.md not found in '$BOOK_DIR/src'"
    exit 1
fi
```

### Phase 3: Analyze Chapters

For each markdown file in `$BOOK_DIR/src/`:

#### Step 1: Scan Files

```bash
# Find all markdown files (excluding SUMMARY.md and already-organized subdirectories)
find "$BOOK_DIR/src" -maxdepth 1 -name "*.md" -not -name "SUMMARY.md"
```

#### Step 2: Analyze Each Chapter

For each chapter file:

1. **Count total lines:**
   ```bash
   LINE_COUNT=$(wc -l < "$CHAPTER_FILE")
   ```

2. **Extract H2 sections:**
   - Parse markdown to find all lines starting with `## `
   - Extract section titles and line numbers
   - Count H2 sections

3. **Apply Decision Matrix:**

   | Condition | Action |
   |-----------|--------|
   | < 300 lines | Keep as single file |
   | 300-600 lines, < 6 H2s | Keep as single file |
   | > 400 lines, 6+ H2s | **Split into subsections** |
   | > 600 lines | **Split into subsections** |
   | > 800 lines | **Always split** |

4. **Record chapters to split:**
   - Create a list of chapters exceeding thresholds
   - Note chapter name, line count, H2 count

### Phase 4: Split Chapters

For each chapter identified for splitting:

#### Step 1: Parse Chapter Structure

Read the chapter file and parse:

1. **Extract all H2 sections:**
   - Section title
   - Line number where section starts
   - Content (all lines until next H2 or EOF)
   - Nested H3/H4/H5/H6 sections

2. **Identify content for index.md:**
   - Everything before first H2 (introduction)
   - First N H2 sections (where N = PRESERVE_INDEX_SECTIONS)
   - Quick start or overview sections

3. **Plan subsection files:**
   - Each remaining H2 becomes a separate file
   - Generate filename from section title (lowercase, hyphens)
   - Group small sections (<50 lines) if beneficial

#### Step 2: Create Subdirectory Structure

Example transformation:

**Before:**
```
book/src/mapreduce.md  (1038 lines, 15 H2 sections)
```

**After:**
```
book/src/mapreduce/
├── index.md              # Introduction + Quick Start + Basic Structure
├── environment.md        # Environment Variables section
├── checkpoint-resume.md  # Checkpoint and Resume section
├── dlq.md                # Dead Letter Queue section
├── performance.md        # Performance Tuning section
├── examples.md           # Real-World Examples section
└── troubleshooting.md    # Troubleshooting section
```

#### Step 3: Generate index.md

The index.md should contain:

1. **Original introduction** (content before first H2)
2. **Preserved sections** (first N H2 sections with full content)
3. **Quick navigation links** to other subsections:

```markdown
## Additional Topics

See also:
- [Environment Variables](environment.md)
- [Checkpoint and Resume](checkpoint-resume.md)
- [Dead Letter Queue](dlq.md)
- [Performance Tuning](performance.md)
- [Real-World Examples](examples.md)
- [Troubleshooting](troubleshooting.md)
```

#### Step 4: Generate Subsection Files

For each subsection file:

1. **Create file with section content:**
   - Include the H2 heading
   - Include all content until next H2
   - Include nested H3/H4/H5/H6 sections
   - Preserve code blocks, lists, tables

2. **Add navigation links:**
   - Link back to index.md at top
   - Link to next/previous subsection if logical

### Phase 5: Update SUMMARY.md

#### Step 1: Find Chapter Entry

Locate the line in SUMMARY.md for the split chapter:

```markdown
- [MapReduce Workflows](mapreduce.md)
```

#### Step 2: Replace with Nested Structure

Replace single line with nested structure:

```markdown
- [MapReduce Workflows](mapreduce/index.md)
  - [Environment Variables](mapreduce/environment.md)
  - [Checkpoint and Resume](mapreduce/checkpoint-resume.md)
  - [Dead Letter Queue](mapreduce/dlq.md)
  - [Performance Tuning](mapreduce/performance.md)
  - [Real-World Examples](mapreduce/examples.md)
  - [Troubleshooting](mapreduce/troubleshooting.md)
```

**Rules:**
- Maintain proper indentation (2 spaces per level)
- Preserve surrounding structure
- Keep parent chapter title unchanged
- Add subsections in logical order

### Phase 6: Update Cross-References

#### Step 1: Scan All Markdown Files

Find all markdown files in book that might have links to split chapter:

```bash
find "$BOOK_DIR/src" -name "*.md" -type f
```

#### Step 2: Update Links

For each file, update links following these rules:

1. **Links to chapter without anchor:**
   ```markdown
   Before: [See MapReduce](mapreduce.md)
   After:  [See MapReduce](mapreduce/index.md)
   ```

2. **Links to specific section (with anchor):**
   ```markdown
   Before: [See DLQ](mapreduce.md#dead-letter-queue)
   After:  [See DLQ](mapreduce/dlq.md)
   ```

   **Algorithm:**
   - Parse anchor (e.g., `#dead-letter-queue`)
   - Find which subsection file contains that H2
   - Update link to point to that subsection

3. **Anchor links within same file:**
   ```markdown
   Before: [See below](#checkpoint-resume)  (in mapreduce.md)
   After:  [See Checkpoint](checkpoint-resume.md)  (in mapreduce/index.md)
   ```

4. **External links:**
   - Never modify (preserve as-is)

#### Step 3: Update Relative Image Paths

If the chapter contains images:

```markdown
Before: ![Diagram](../images/mapreduce-flow.png)
After:  ![Diagram](../../images/mapreduce-flow.png)  (adjust for subdirectory nesting)
```

### Phase 7: Dry-Run vs Actual Execution

#### Dry-Run Mode (--dry-run true)

**Output preview:**

```
Analyzing chapters...

Would split: mapreduce.md (1038 lines, 15 H2 sections)
  → mapreduce/index.md (Introduction + Quick Start + Structure)
  → mapreduce/environment.md (Environment Variables)
  → mapreduce/checkpoint-resume.md (Checkpoint and Resume)
  → mapreduce/dlq.md (Dead Letter Queue)
  → mapreduce/performance.md (Performance Tuning)
  → mapreduce/examples.md (Real-World Examples)
  → mapreduce/troubleshooting.md (Troubleshooting)

Would split: commands.md (698 lines, 8 H2 sections)
  → commands/index.md (Introduction + Overview)
  → commands/claude.md (Claude Commands)
  → commands/shell.md (Shell Commands)
  → commands/goal-seek.md (Goal Seek)
  → commands/foreach.md (ForEach Loops)
  → commands/validation.md (Validation)

Summary:
  - 2 chapters would be split
  - 13 subsection files would be created
  - SUMMARY.md would be updated with nested structure
  - 24 cross-references would be updated

Run without --dry-run to apply changes.
```

**Action:** Print preview only, do NOT modify any files.

#### Actual Execution (--dry-run false)

**Perform all operations:**

1. Create subdirectories
2. Write index.md and subsection files
3. Delete original chapter file
4. Update SUMMARY.md
5. Update all cross-references
6. Update image paths

**Output progress:**

```
Organizing chapters...

✓ Split mapreduce.md → mapreduce/ (7 subsections)
✓ Split commands.md → commands/ (6 subsections)
✓ Updated SUMMARY.md with nested structure
✓ Updated 24 cross-references

Summary:
  - 2 chapters split
  - 13 subsection files created
  - SUMMARY.md updated
  - All links validated
```

### Phase 8: Validation

After splitting (if not dry-run):

#### Step 1: Validate mdBook Build

```bash
cd "$BOOK_DIR"
mdbook build

if [ $? -ne 0 ]; then
    echo "Error: mdbook build failed after splitting"
    echo "Please check book structure and fix errors"
    exit 1
fi
```

#### Step 2: Check for Broken Links

Use mdBook's built-in link checker or manually verify:

```bash
# mdbook has --open flag to preview in browser
mdbook build --open
```

**Manual verification:**
- Check that all subsection links work
- Verify navigation between subsections
- Ensure images load correctly
- Test cross-references to other chapters

#### Step 3: Verify Content Preservation

Compare line counts:

```bash
# Original chapter line count should equal sum of subsection line counts
ORIGINAL_LINES=$(wc -l < mapreduce.md.backup)
SUBSECTION_LINES=$(wc -l mapreduce/*.md | tail -1 | awk '{print $1}')

if [ $ORIGINAL_LINES -ne $SUBSECTION_LINES ]; then
    echo "Warning: Line count mismatch (original: $ORIGINAL_LINES, subsections: $SUBSECTION_LINES)"
fi
```

### Phase 9: Git Operations

#### Step 1: Stage Changes

```bash
cd "$BOOK_DIR"
git add -A
```

**Files to stage:**
- New subdirectories (e.g., `src/mapreduce/`)
- All new subsection files
- Updated SUMMARY.md
- Deleted original chapter files
- Updated files with cross-reference changes

#### Step 2: Commit Changes

**Commit message format:**

```bash
git commit -m "docs: organize large chapters into subsections

Split chapters into logical subsections for better readability:
- mapreduce.md → mapreduce/ (7 subsections)
- commands.md → commands/ (6 subsections)

Updated SUMMARY.md with nested structure and fixed all cross-references.
All links validated and mdbook build succeeds."
```

**Note:** Git should detect file moves/splits and preserve history.

### Phase 10: Error Handling

#### mdbook Build Failure

If `mdbook build` fails after splitting:

```bash
echo "Error: mdbook build failed"
echo "Common issues:"
echo "  - Broken link in subsection file"
echo "  - Invalid markdown syntax"
echo "  - Missing file referenced in SUMMARY.md"
echo ""
echo "Review error messages above and fix issues"
exit 1
```

#### Broken Links

If links don't resolve:

```bash
echo "Warning: Found broken links"
echo "  - Check anchor updates for moved sections"
echo "  - Verify subsection file paths in SUMMARY.md"
echo "  - Ensure relative paths updated for nested structure"
```

#### Duplicate Subsections

If chapter already has subdirectory:

```bash
if [ -d "$BOOK_DIR/src/mapreduce" ]; then
    echo "Skipping mapreduce.md (already organized into subdirectory)"
    continue
fi
```

**Respect manual organization:** Don't overwrite existing subsection structure.

### Quality Guidelines

#### Content Preservation

- **Never lose content:** All lines from original chapter must appear in subsections
- **Preserve formatting:** Code blocks, lists, tables must remain intact
- **Maintain nesting:** H3/H4/H5/H6 stay under their parent H2

#### Logical Grouping

- **Related sections together:** Group setup, configuration, and usage if they're small
- **Standalone features:** Large features get their own subsection
- **Progressive disclosure:** Index.md has basics, subsections have details

#### Navigation

- **Clear section titles:** Use descriptive, SEO-friendly filenames
- **Logical ordering:** Arrange subsections from basic → advanced
- **Cross-references:** Link between related subsections

#### Filename Generation

Convert section title to filename:

```bash
# Example: "Checkpoint and Resume" → "checkpoint-resume.md"
SECTION_TITLE="Checkpoint and Resume"
FILENAME=$(echo "$SECTION_TITLE" | tr '[:upper:]' '[:lower:]' | sed 's/ /-/g' | sed 's/[^a-z0-9-]//g').md
```

**Rules:**
- Lowercase only
- Replace spaces with hyphens
- Remove special characters
- Keep alphanumeric and hyphens
- Add .md extension

### Success Criteria

The command succeeds when:

- [ ] All chapters exceeding thresholds are split
- [ ] Subdirectories created with index.md and subsection files
- [ ] SUMMARY.md updated with proper nesting
- [ ] All cross-references work (no broken links)
- [ ] mdbook build succeeds without errors or warnings
- [ ] All content preserved (no lost lines)
- [ ] Git commit created with descriptive message
- [ ] Dry-run mode shows preview without modifying files

### Example Output

**Dry-run:**

```
$ /prodigy-auto-organize-chapters --book-dir book --dry-run true

Analyzing chapters in book/src/...

✓ mapreduce.md (1038 lines, 15 H2s) → Would split
✓ commands.md (698 lines, 8 H2s) → Would split
✓ workflow-basics.md (285 lines, 5 H2s) → Keep as-is
✓ variables.md (142 lines, 4 H2s) → Keep as-is

Preview of changes:
  - 2 chapters would be split
  - 13 subsection files would be created
  - SUMMARY.md would be updated
  - 24 links would be updated

Run without --dry-run to apply changes.
```

**Actual execution:**

```
$ /prodigy-auto-organize-chapters --book-dir book

Organizing chapters in book/src/...

✓ Split mapreduce.md → mapreduce/ (7 subsections)
  - index.md (Introduction, Quick Start, Structure)
  - environment.md
  - checkpoint-resume.md
  - dlq.md
  - performance.md
  - examples.md
  - troubleshooting.md

✓ Split commands.md → commands/ (6 subsections)
  - index.md (Introduction, Overview)
  - claude.md
  - shell.md
  - goal-seek.md
  - foreach.md
  - validation.md

✓ Updated SUMMARY.md with nested structure
✓ Updated 24 cross-references in 8 files
✓ Validated mdbook build succeeds

Summary:
  - 2 chapters reorganized
  - 13 subsection files created
  - All links validated
  - Changes committed to git

Next steps:
  - Review changes: git diff HEAD~1
  - Preview book: cd book && mdbook serve
```
