# /prodigy-validate-book-holistically

Perform holistic validation of the entire book after map phase completes. This validates cross-cutting concerns that individual subsection fixes cannot detect.

## Variables

- `--project <name>` - Project name (e.g., "Prodigy", "Debtmap")
- `--book-dir <path>` - Path to book directory (default: "book")
- `--output <path>` - Path to write validation report (default: ".prodigy/book-validation.json")
- `--auto-fix <boolean>` - Automatically fix issues found (default: false)

## Execute

### Phase 1: Understand Context

You are performing **holistic validation** of the entire book after the map phase has updated individual chapters/subsections. The map phase focuses on individual files and cannot detect:

1. **Redundancy across chapters** - Multiple files with overlapping Best Practices
2. **Structural inconsistencies** - Some chapters use subsections, others don't
3. **Navigation patterns** - Circular or redundant "See Also" links
4. **Content distribution** - Best Practices scattered vs centralized
5. **Chapter fragmentation** - Too many tiny subsections

**Your Goal**: Identify these cross-cutting issues and either fix them automatically or report them for manual review.

**CRITICAL IMPLEMENTATION REQUIREMENTS:**

1. **Use shell commands directly** - All scanning and auto-fix logic should use bash/sed/awk/grep
2. **Do NOT create Python scripts** - Execute commands inline, don't generate validate_book.py or auto_fix.py
3. **Whitelist appropriately** - Root-level guides and chapter indexes (without dedicated best-practices.md) can have BP sections
4. **Better reference detection** - Use ratio of reference vs guide indicators, not absolute counts

### Phase 2: Extract Parameters

```bash
PROJECT_NAME="${project:?Error: --project is required}"
BOOK_DIR="${book_dir:-book}"
OUTPUT="${output:-.prodigy/book-validation.json}"
AUTO_FIX="${auto_fix:-false}"
```

**Validate Inputs:**
```bash
if [ ! -d "$BOOK_DIR/src" ]; then
    echo "Error: Book directory not found: $BOOK_DIR/src"
    exit 1
fi
```

### Phase 3: Scan Book Structure

**Step 1: Build Chapter Inventory**

Scan `$BOOK_DIR/src/SUMMARY.md` to understand chapter structure:

```bash
# Extract all chapters and their subsections
CHAPTERS=$(grep -E '^\s*-\s+\[' "$BOOK_DIR/src/SUMMARY.md")
```

For each chapter, determine:
1. **Type**: `single-file` (e.g., `error-handling.md`) or `multi-subsection` (e.g., `environment/index.md`)
2. **Subsection count**: How many files under this chapter
3. **Has dedicated best-practices.md**: Check if `{chapter}/best-practices.md` exists
4. **Has dedicated troubleshooting.md**: Check if `{chapter}/troubleshooting.md` exists

**Step 2: Identify All Files with "Best Practices" Sections**

```bash
# Find all markdown files with Best Practices sections
find "$BOOK_DIR/src" -name "*.md" -type f -exec grep -l "^## Best Practices\|^### Best Practices" {} \; > /tmp/bp-files.txt
```

For each file:
1. **File path** relative to `$BOOK_DIR/src/`
2. **File type**: index.md, subsection, standalone, dedicated best-practices.md
3. **Parent chapter**: If subsection, which chapter does it belong to
4. **Line range**: Where the Best Practices section starts/ends

### Phase 4: Detect Anti-Patterns

#### Anti-Pattern 1: Redundant Best Practices Sections

**Issue**: Subsection files have "Best Practices" sections when their chapter has a dedicated `best-practices.md`.

**Detection Logic:**
```bash
# For each file with Best Practices section
while read -r FILE; do
  # Get parent chapter directory
  CHAPTER_DIR=$(dirname "$FILE")

  # Check if this is a subsection (not index.md, not standalone)
  if [[ "$FILE" != */index.md ]] && [[ "$FILE" == */* ]]; then
    # Check if chapter has dedicated best-practices.md
    if [ -f "$CHAPTER_DIR/best-practices.md" ]; then
      echo "REDUNDANT: $FILE has Best Practices but $CHAPTER_DIR/best-practices.md exists"
    fi
  fi
done < /tmp/bp-files.txt
```

**Report Format:**
```json
{
  "type": "redundant_best_practices",
  "severity": "high",
  "files": [
    {
      "file": "environment/index.md",
      "lines": [244, 265],
      "redundant_with": "environment/best-practices.md",
      "recommendation": "Remove section, content covered in dedicated file"
    },
    {
      "file": "retry-configuration/retry-budget.md",
      "lines": [129, 184],
      "redundant_with": "retry-configuration/best-practices.md",
      "recommendation": "Remove section, migrate useful content to dedicated file"
    }
  ]
}
```

#### Anti-Pattern 2: Best Practices in Technical Reference Pages

**Issue**: Technical reference pages (syntax, configuration, API) have Best Practices sections.

**IMPORTANT: Whitelist root-level guides and chapter indexes**

**Detection Logic:**
```bash
# Identify technical reference pages
while read -r FILE; do
  BASENAME=$(basename "$FILE")
  RELATIVE_PATH="${FILE#$BOOK_DIR/src/}"

  # SKIP: Root-level guide files (appropriate for Best Practices)
  if [[ "$RELATIVE_PATH" == *.md ]] && [[ ! "$RELATIVE_PATH" =~ / ]]; then
    # Root-level files like error-handling.md, workflow-basics.md are guides
    continue
  fi

  # SKIP: Chapter index.md files (appropriate for Best Practices)
  if [[ "$BASENAME" == "index.md" ]]; then
    # Check if chapter has dedicated best-practices.md
    CHAPTER_DIR=$(dirname "$FILE")
    if [ ! -f "$CHAPTER_DIR/best-practices.md" ]; then
      # No dedicated file, index.md can have BP section
      continue
    fi
  fi

  # SKIP: Files explicitly marked as guides/tutorials
  if grep -qi "^# .*\(guide\|tutorial\|introduction\|overview\|getting started\)" "$FILE" | head -1; then
    continue
  fi

  # Check file content for reference page indicators
  REFERENCE_COUNT=$(grep -ci "syntax\|reference\|configuration\|fields\|options\|parameters\|properties\|attributes" "$FILE" | head -20)
  GUIDE_COUNT=$(grep -ci "tutorial\|guide\|walkthrough\|how to\|step-by-step" "$FILE" | head -20)

  # If reference indicators > guide indicators, it's likely a reference page
  if [ "$REFERENCE_COUNT" -gt "$((GUIDE_COUNT * 2))" ]; then
    echo "REFERENCE_PAGE: $FILE is technical reference with Best Practices section"
  fi
done < /tmp/bp-files.txt
```

**Report Format:**
```json
{
  "type": "best_practices_in_reference",
  "severity": "medium",
  "files": [
    {
      "file": "workflow-basics/command-level-options.md",
      "lines": [468, 527],
      "file_type": "technical_reference",
      "recommendation": "Remove Best Practices section - this is API documentation"
    }
  ]
}
```

#### Anti-Pattern 3: Circular "See Also" References

**Issue**: Subsection A links to B, B links to A, creating circular navigation without hierarchy.

**Detection Logic:**
```bash
# Extract all "See Also" links from all files
find "$BOOK_DIR/src" -name "*.md" -type f | while read -r FILE; do
  # Find "See Also" section and extract links
  sed -n '/^## See Also/,/^##/p' "$FILE" | grep -oP '\[.*?\]\(\K[^\)]+' | while read -r LINK; do
    # Resolve relative link
    TARGET=$(cd "$(dirname "$FILE")" && realpath --relative-to="$BOOK_DIR/src" "$LINK" 2>/dev/null)
    echo "$FILE -> $TARGET"
  done
done > /tmp/see-also-graph.txt

# Detect circular references
# If A -> B and B -> A, report as circular
```

**Report Format:**
```json
{
  "type": "circular_see_also",
  "severity": "low",
  "patterns": [
    {
      "files": ["mapreduce/checkpoint-and-resume.md", "mapreduce/dead-letter-queue-dlq.md"],
      "description": "Mutual references without explaining specific relationship"
    }
  ]
}
```

#### Anti-Pattern 4: Generic "See Also" Lists

**Issue**: Files list every other subsection in the chapter without explaining why.

**Detection Logic:**
```bash
# For each file with "See Also" section
find "$BOOK_DIR/src" -name "*.md" -type f | while read -r FILE; do
  # Count links in "See Also" section
  LINK_COUNT=$(sed -n '/^## See Also/,/^##/p' "$FILE" | grep -c '^\s*-')

  # If more than 5 links, likely a generic list
  if [ "$LINK_COUNT" -gt 5 ]; then
    # Check if links have explanations (text after the link)
    EXPLAINED_LINKS=$(sed -n '/^## See Also/,/^##/p' "$FILE" | grep -c '\](.*) -')

    if [ "$EXPLAINED_LINKS" -lt "$((LINK_COUNT / 2))" ]; then
      echo "GENERIC_SEE_ALSO: $FILE lists $LINK_COUNT links without explanations"
    fi
  fi
done
```

**Report Format:**
```json
{
  "type": "generic_see_also",
  "severity": "low",
  "files": [
    {
      "file": "mapreduce/checkpoint-and-resume.md",
      "link_count": 8,
      "explained_count": 2,
      "recommendation": "Reduce to 3-4 most relevant links with specific relationships"
    }
  ]
}
```

#### Anti-Pattern 5: Over-Fragmented Chapters

**Issue**: Chapters with too many subsections (>10) or subsections with minimal content (<100 lines).

**Detection Logic:**
```bash
# For each multi-subsection chapter
find "$BOOK_DIR/src" -type d -mindepth 1 | while read -r CHAPTER_DIR; do
  # Count subsection files (exclude index.md)
  SUBSECTION_COUNT=$(find "$CHAPTER_DIR" -name "*.md" -not -name "index.md" | wc -l)

  if [ "$SUBSECTION_COUNT" -gt 10 ]; then
    # Check average file size
    AVG_LINES=$(find "$CHAPTER_DIR" -name "*.md" -not -name "index.md" -exec wc -l {} \; | awk '{sum+=$1; count++} END {print sum/count}')

    if [ "$AVG_LINES" -lt 100 ]; then
      echo "OVER_FRAGMENTED: $CHAPTER_DIR has $SUBSECTION_COUNT subsections averaging $AVG_LINES lines"
    fi
  fi
done
```

**Report Format:**
```json
{
  "type": "over_fragmented_chapter",
  "severity": "medium",
  "chapters": [
    {
      "chapter": "retry-configuration",
      "subsection_count": 15,
      "average_lines": 87,
      "recommendation": "Consolidate related subsections - target 6-8 focused subsections"
    }
  ]
}
```

#### Anti-Pattern 6: Stub Navigation Files

**Issue**: Files that are just navigation boilerplate (<50 lines, mostly links).

**Detection Logic:**
```bash
# Find small files
find "$BOOK_DIR/src" -name "*.md" -type f -exec sh -c 'wc -l "$1" | awk "\$1 < 50 {print \$2}"' _ {} \; | while read -r FILE; do
  # Check if file is mostly links
  LINK_COUNT=$(grep -c '^\s*-\s*\[.*\](' "$FILE")
  LINE_COUNT=$(wc -l < "$FILE")

  # If more than 50% links, it's a navigation stub
  if [ "$((LINK_COUNT * 2))" -gt "$LINE_COUNT" ]; then
    echo "STUB_FILE: $FILE is only $LINE_COUNT lines with $LINK_COUNT links"
  fi
done
```

**Report Format:**
```json
{
  "type": "stub_navigation_file",
  "severity": "medium",
  "files": [
    {
      "file": "composition/related-chapters.md",
      "lines": 14,
      "link_percentage": 71,
      "recommendation": "Consolidate into composition/index.md"
    }
  ]
}
```

#### Anti-Pattern 7: Meta-Sections in Feature Chapters

**Issue**: "Best Practices" or "Common Patterns" files appear as subsections within feature-focused chapters (like "Advanced Features").

**Why This Is Wrong:**
- Feature chapters should only contain features/capabilities, not meta-guidance
- "Best Practices" and "Common Patterns" are general workflow advice, not specific features
- This mixes "what" (features) with "how to use well" (guidance) at the wrong level
- Appropriate locations: chapter-level dedicated files for unified topics (environment, retry-configuration) or root-level guides

**Detection Logic:**
```bash
# Check SUMMARY.md for meta-sections under feature chapters
grep -A 20 "Advanced Features\|Advanced Topics" "$BOOK_DIR/SUMMARY.md" | while IFS= read -r LINE; do
  # Check if line is a meta-section under feature chapter
  if echo "$LINE" | grep -qi "\- \[Best Practices\]\|\- \[Common Patterns\]"; then
    # Extract file path
    FILE=$(echo "$LINE" | grep -oP '\[.*?\]\(\K[^\)]+')

    # Verify it's under a feature-focused chapter (not environment, retry-configuration, etc.)
    if [[ "$FILE" =~ ^advanced/ ]]; then
      echo "META_IN_FEATURES: $FILE is meta-section under feature chapter"
    fi
  fi
done
```

**Report Format:**
```json
{
  "type": "meta_sections_in_feature_chapters",
  "severity": "medium",
  "files": [
    {
      "file": "advanced/best-practices.md",
      "parent_chapter": "Advanced Features",
      "meta_type": "Best Practices",
      "recommendation": "Remove - general workflow guidance doesn't belong in feature chapter"
    },
    {
      "file": "advanced/common-patterns.md",
      "parent_chapter": "Advanced Features",
      "meta_type": "Common Patterns",
      "recommendation": "Remove - patterns should be in workflow-basics or distributed to relevant chapters"
    }
  ]
}
```

**Appropriate Locations for Meta-Sections:**
- `environment/best-practices.md` ✅ - Specific to environment configuration
- `retry-configuration/best-practices.md` ✅ - Specific to retry strategies
- `composition/best-practices.md` ✅ - Specific to workflow composition
- `advanced/best-practices.md` ❌ - Too general, mixed features
- `workflow-basics.md` ✅ - Root-level guide can have general best practices

### Phase 5: Generate Holistic Validation Report

**Compile All Findings:**

```json
{
  "validation_timestamp": "2025-01-10T15:30:00Z",
  "project": "$PROJECT_NAME",
  "book_dir": "$BOOK_DIR",
  "total_files": 147,
  "total_chapters": 15,
  "issues_found": [
    {/* Anti-Pattern 1 findings */},
    {/* Anti-Pattern 2 findings */},
    {/* Anti-Pattern 3 findings */},
    {/* Anti-Pattern 4 findings */},
    {/* Anti-Pattern 5 findings */},
    {/* Anti-Pattern 6 findings */},
    {/* Anti-Pattern 7 findings */}
  ],
  "summary": {
    "redundant_best_practices": 6,
    "best_practices_in_reference": 6,
    "circular_see_also": 12,
    "generic_see_also": 30,
    "over_fragmented_chapters": 3,
    "stub_navigation_files": 8,
    "meta_sections_in_feature_chapters": 2
  },
  "recommendations": [
    "Remove 6 redundant Best Practices sections",
    "Remove 6 Best Practices sections from technical reference pages",
    "Consolidate 3 over-fragmented chapters",
    "Merge 8 stub navigation files into chapter indexes",
    "Remove 2 meta-sections from feature chapters"
  ]
}
```

**Write Report:**
```bash
cat > "$OUTPUT" <<EOF
{validation report JSON}
EOF

echo "✓ Holistic validation complete"
echo "  Issues found: ${TOTAL_ISSUES}"
echo "  Report written to: $OUTPUT"
```

### Phase 6: Auto-Fix Mode (Optional)

If `--auto-fix true`, perform automatic fixes for clear-cut issues.

**IMPORTANT: Use direct shell commands, NOT Python scripts.**

The auto-fix implementation should:
1. Read the validation.json report
2. Use sed/awk/grep to make in-place edits
3. Not generate separate Python scripts
4. Apply fixes atomically (backup before editing)

#### Fix 1: Remove Redundant Best Practices Sections

```bash
# For each redundant Best Practices section
jq -r '.issues[] | select(.type == "redundant_best_practices") | .files[] | "\(.file) \(.lines[0]) \(.lines[1])"' "$OUTPUT" | while read -r FILE START END; do
  FULL_PATH="$BOOK_DIR/src/$FILE"

  # Backup file before editing
  cp "$FULL_PATH" "$FULL_PATH.bak"

  # Remove lines START to END (inclusive)
  sed -i.tmp "${START},${END}d" "$FULL_PATH"
  rm "$FULL_PATH.tmp" 2>/dev/null || true

  echo "  ✓ Removed redundant Best Practices from $FILE (lines $START-$END)"
done
```

#### Fix 2: Remove Best Practices from Reference Pages

```bash
# For each Best Practices section in reference pages
jq -r '.issues[] | select(.type == "best_practices_in_reference") | .files[] | "\(.file) \(.lines[0]) \(.lines[1])"' "$OUTPUT" | while read -r FILE START END; do
  FULL_PATH="$BOOK_DIR/src/$FILE"

  # Skip if already processed by redundant_best_practices
  if [ ! -f "$FULL_PATH.bak" ]; then
    cp "$FULL_PATH" "$FULL_PATH.bak"
    sed -i.tmp "${START},${END}d" "$FULL_PATH"
    rm "$FULL_PATH.tmp" 2>/dev/null || true
    echo "  ✓ Removed Best Practices from reference page $FILE (lines $START-$END)"
  fi
done
```

#### Fix 3: Consolidate Stub Navigation Files

```bash
# For each stub navigation file
jq -r '.issues[] | select(.type == "stub_navigation_file") | .files[] | .file' "$OUTPUT" | while read -r STUB_FILE; do
  STUB_PATH="$BOOK_DIR/src/$STUB_FILE"
  CHAPTER_DIR=$(dirname "$STUB_PATH")
  INDEX_FILE="$CHAPTER_DIR/index.md"

  if [ ! -f "$INDEX_FILE" ]; then
    echo "  ⚠ Warning: No index.md found for $STUB_FILE, skipping"
    continue
  fi

  # Backup index before appending
  cp "$INDEX_FILE" "$INDEX_FILE.bak"

  # Append stub content to index.md with separator
  echo "" >> "$INDEX_FILE"
  echo "---" >> "$INDEX_FILE"
  echo "" >> "$INDEX_FILE"
  cat "$STUB_PATH" >> "$INDEX_FILE"

  # Remove stub file
  rm "$STUB_PATH"

  # Update SUMMARY.md to remove stub reference
  STUB_BASENAME=$(basename "$STUB_FILE")
  sed -i.tmp "/\[$STUB_BASENAME\]/d" "$BOOK_DIR/src/SUMMARY.md"
  rm "$BOOK_DIR/src/SUMMARY.md.tmp" 2>/dev/null || true

  echo "  ✓ Consolidated $STUB_FILE into index.md"
done
```

#### Fix 4: Remove Meta-Sections from Feature Chapters

```bash
# For each meta-section in feature chapters
jq -r '.issues[] | select(.type == "meta_sections_in_feature_chapters") | .files[] | .file' "$OUTPUT" | while read -r META_FILE; do
  META_PATH="$BOOK_DIR/src/$META_FILE"
  META_BASENAME=$(basename "$META_FILE")

  # Remove the file
  if [ -f "$META_PATH" ]; then
    rm "$META_PATH"
    echo "  ✓ Removed meta-section $META_FILE from feature chapter"
  fi

  # Remove from SUMMARY.md
  # Match lines like "  - [Best Practices](advanced/best-practices.md)"
  sed -i.tmp "/\[.*\]($META_FILE)/d" "$BOOK_DIR/src/SUMMARY.md"
  rm "$BOOK_DIR/src/SUMMARY.md.tmp" 2>/dev/null || true

  echo "  ✓ Updated SUMMARY.md to remove $META_BASENAME"
done
```

**Cleanup Backups:**
```bash
# Remove backup files after successful fixes
find "$BOOK_DIR/src" -name "*.bak" -delete
```

**Commit Auto-Fixes:**
```bash
if [ "$AUTO_FIX" = "true" ]; then
  git add "$BOOK_DIR/src"
  git commit -m "docs: holistic cleanup after drift detection

- Removed $REDUNDANT_COUNT redundant Best Practices sections
- Removed $REFERENCE_COUNT Best Practices from technical reference pages
- Consolidated $STUB_COUNT stub navigation files

Based on holistic validation report: $OUTPUT"

  echo "✓ Auto-fixes committed"
fi
```

### Phase 7: Summary Output

**If Auto-Fix Enabled:**
```
✓ Holistic Validation Complete
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Issues Found: 65
Auto-Fixed: 47

Fixes Applied:
  ✓ Removed 6 redundant Best Practices sections
  ✓ Removed 6 Best Practices from reference pages
  ✓ Consolidated 8 stub navigation files

Manual Review Required: 18 issues
  ⚠ 3 over-fragmented chapters (manual consolidation recommended)
  ⚠ 12 circular See Also references (need context-specific fixes)
  ⚠ 3 other structural issues

See detailed report: .prodigy/book-validation.json
```

**If Reporting Only:**
```
✓ Holistic Validation Complete
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Issues Found: 65

High Priority (12):
  • 6 redundant Best Practices sections
  • 6 Best Practices in technical reference pages

Medium Priority (35):
  • 3 over-fragmented chapters
  • 8 stub navigation files
  • 24 other structural issues

Low Priority (18):
  • 12 circular See Also references
  • 6 generic See Also lists

Recommendations:
  1. Run with --auto-fix to resolve 47 issues automatically
  2. Manually review over-fragmented chapters for consolidation
  3. Simplify circular See Also references

Detailed report: .prodigy/book-validation.json
```

### Success Criteria

- [ ] All chapters scanned and categorized
- [ ] All Best Practices sections identified and validated
- [ ] Redundancy detected across chapters
- [ ] Over-fragmentation detected
- [ ] Stub navigation files identified
- [ ] Circular references detected
- [ ] Validation report generated with severity levels
- [ ] Auto-fix mode works correctly (if enabled)
- [ ] mdbook build succeeds after auto-fixes

### Error Handling

**Book build fails:**
```
Error: Book build failed after auto-fixes
Run: cd book && mdbook build
Review errors and manually fix broken links.
```

**Invalid book structure:**
```
Error: Could not parse SUMMARY.md
Ensure SUMMARY.md exists and follows mdBook format.
```

**No issues found:**
```
✓ Book validation passed
No cross-cutting issues detected.
```
