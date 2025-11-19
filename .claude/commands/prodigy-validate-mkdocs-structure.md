# /prodigy-validate-mkdocs-structure

Perform structural validation of the entire mkdocs documentation after map phase completes. This validates cross-cutting concerns that individual subsection fixes cannot detect.

## Variables

- `--project <name>` - Project name (e.g., "Prodigy", "Debtmap")
- `--docs-dir <path>` - Path to mkdocs docs directory (default: "mkdocs")
- `--output <path>` - Path to write validation report (default: ".prodigy/mkdocs-validation.json")
- `--auto-fix <boolean>` - Automatically fix issues found (default: false)

## Execute

### Phase 1: Understand Context

You are performing **structural validation** of the entire mkdocs documentation after the map phase has updated individual chapters/subsections. The map phase focuses on individual files and cannot detect:

1. **Redundancy across chapters** - Multiple files with overlapping Best Practices
2. **Structural inconsistencies** - Some chapters use subsections, others don't
3. **Navigation patterns** - Circular or redundant "See Also" links
4. **Content distribution** - Best Practices scattered vs centralized
5. **Chapter fragmentation** - Too many tiny subsections

**Your Goal**: Identify these cross-cutting issues and either fix them automatically or report them for manual review.

**CRITICAL IMPLEMENTATION REQUIREMENTS:**

1. **Use shell commands directly** - All scanning and auto-fix logic should use bash/sed/awk/grep
2. **Do NOT create Python scripts** - Execute commands inline, don't generate validate_docs.py or auto_fix.py
3. **Whitelist appropriately** - Root-level guides and chapter indexes (without dedicated best-practices.md) can have BP sections
4. **Better reference detection** - Use ratio of reference vs guide indicators, not absolute counts

### Phase 2: Extract Parameters

```bash
PROJECT_NAME="${project:?Error: --project is required}"
DOCS_DIR="${docs_dir:-mkdocs}"
OUTPUT="${output:-.prodigy/mkdocs-validation.json}"
AUTO_FIX="${auto_fix:-false}"
```

**Validate Inputs:**
```bash
if [ ! -d "$DOCS_DIR" ]; then
    echo "Error: MkDocs directory not found: $DOCS_DIR"
    exit 1
fi

if [ ! -f "mkdocs.yml" ]; then
    echo "Error: mkdocs.yml not found in project root"
    exit 1
fi
```

### Phase 3: MkDocs-Specific Validation

**CRITICAL: These checks must pass before proceeding with content validation.**

**Step 1: Verify index.md Exists**

```bash
if [ ! -f "$DOCS_DIR/index.md" ]; then
    echo "❌ CRITICAL: Missing index.md landing page"
    echo "   MkDocs requires a homepage at $DOCS_DIR/index.md"

    if [ "$AUTO_FIX" = "true" ]; then
        echo "   Creating basic index.md..."
        cat > "$DOCS_DIR/index.md" <<'EOF'
# ${PROJECT_NAME} Documentation

Welcome to the ${PROJECT_NAME} documentation.

## Contents

See the navigation menu for available documentation sections.
EOF
        echo "   ✓ Created $DOCS_DIR/index.md"
    else
        exit 1
    fi
fi
```

**Step 2: Validate mkdocs.yml Navigation Completeness**

Check for orphaned files (files in docs/ not referenced in mkdocs.yml nav):

```bash
# Extract all files referenced in mkdocs.yml nav
grep -oP ':\s*\K[^\s]+\.md' mkdocs.yml | sort > /tmp/nav-files.txt

# Find all actual markdown files in docs/
find "$DOCS_DIR" -name "*.md" -type f | sed "s|^$DOCS_DIR/||" | sort > /tmp/actual-files.txt

# Find orphaned files (in docs/ but not in nav)
comm -13 /tmp/nav-files.txt /tmp/actual-files.txt > /tmp/orphaned-files.txt

ORPHANED_COUNT=$(wc -l < /tmp/orphaned-files.txt)

if [ "$ORPHANED_COUNT" -gt 0 ]; then
    echo "⚠ WARNING: $ORPHANED_COUNT file(s) not included in mkdocs.yml nav:"
    cat /tmp/orphaned-files.txt | while read -r FILE; do
        echo "  - $FILE"
    done

    if [ "$AUTO_FIX" = "true" ]; then
        echo ""
        echo "  Note: Orphaned files should be added to mkdocs.yml nav manually"
        echo "  Auto-fix cannot determine correct nav position automatically"
    fi

    # Store for report
    ORPHANED_FILES=$(cat /tmp/orphaned-files.txt)
fi

# Find missing files (in nav but not in docs/)
comm -23 /tmp/nav-files.txt /tmp/actual-files.txt > /tmp/missing-files.txt

MISSING_COUNT=$(wc -l < /tmp/missing-files.txt)

if [ "$MISSING_COUNT" -gt 0 ]; then
    echo "❌ ERROR: $MISSING_COUNT file(s) referenced in nav but missing from docs/:"
    cat /tmp/missing-files.txt | while read -r FILE; do
        echo "  - $FILE"
    done

    if [ "$AUTO_FIX" != "true" ]; then
        exit 1
    fi
fi
```

**Step 3: Run mkdocs build --strict**

```bash
echo "Running mkdocs build --strict to validate links..."

# Capture build output
MKDOCS_OUTPUT=$(mkdocs build --strict 2>&1)
MKDOCS_EXIT=$?

if [ $MKDOCS_EXIT -ne 0 ]; then
    echo "❌ mkdocs build --strict FAILED"
    echo ""
    echo "$MKDOCS_OUTPUT" | head -50
    echo ""

    # Extract broken link count
    BROKEN_LINKS=$(echo "$MKDOCS_OUTPUT" | grep -c "WARNING.*not found")
    ERRORS=$(echo "$MKDOCS_OUTPUT" | grep -c "ERROR")

    echo "Summary:"
    echo "  Broken links: $BROKEN_LINKS"
    echo "  Errors: $ERRORS"

    # Store for report
    MKDOCS_BUILD_FAILED=true
    MKDOCS_BUILD_OUTPUT="$MKDOCS_OUTPUT"

    if [ "$AUTO_FIX" != "true" ]; then
        echo ""
        echo "Fix these issues manually or run with --auto-fix true"
        exit 1
    fi
else
    echo "✓ mkdocs build --strict passed"
    MKDOCS_BUILD_FAILED=false
fi
```

### Phase 4: Scan Documentation Structure

**Step 1: Build Chapter Inventory**

Scan `mkdocs.yml` to understand navigation structure:

```bash
# Extract navigation structure from mkdocs.yml
# MkDocs uses YAML nav structure like:
# nav:
#   - Home: index.md
#   - Getting Started: getting-started.md
#   - Advanced:
#       - Overview: advanced/index.md
#       - Features: advanced/features.md

yq eval '.nav' mkdocs.yml > /tmp/mkdocs-nav.txt
```

For each chapter, determine:
1. **Type**: `single-file` (e.g., `error-handling.md`) or `multi-subsection` (e.g., `environment/index.md`)
2. **Subsection count**: How many files under this chapter
3. **Has dedicated best-practices.md**: Check if `{chapter}/best-practices.md` exists
4. **Has dedicated troubleshooting.md**: Check if `{chapter}/troubleshooting.md` exists

**Step 2: Identify All Files with "Best Practices" Sections**

```bash
# Find all markdown files with Best Practices sections
find "$DOCS_DIR" -name "*.md" -type f -exec grep -l "^## Best Practices\|^### Best Practices" {} \; > /tmp/bp-files.txt
```

For each file:
1. **File path** relative to `$DOCS_DIR/`
2. **File type**: index.md, subsection, standalone, dedicated best-practices.md
3. **Parent chapter**: If subsection, which chapter does it belong to
4. **Line range**: Where the Best Practices section starts/ends

### Phase 5: Detect Anti-Patterns

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
  RELATIVE_PATH="${FILE#$DOCS_DIR/}"

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

#### Anti-Pattern 3: Circular "See Also" References

**Issue**: Subsection A links to B, B links to A, creating circular navigation without hierarchy.

**Detection Logic:**
```bash
# Extract all "See Also" links from all files
find "$DOCS_DIR" -name "*.md" -type f | while read -r FILE; do
  # Find "See Also" section and extract links
  sed -n '/^## See Also/,/^##/p' "$FILE" | grep -oP '\[.*?\]\(\K[^\)]+' | while read -r LINK; do
    # Resolve relative link
    TARGET=$(cd "$(dirname "$FILE")" && realpath --relative-to="$DOCS_DIR" "$LINK" 2>/dev/null)
    echo "$FILE -> $TARGET"
  done
done > /tmp/see-also-graph.txt

# Detect circular references
# If A -> B and B -> A, report as circular
```

#### Anti-Pattern 4: Generic "See Also" Lists

**Issue**: Files list every other subsection in the chapter without explaining why.

**Detection Logic:**
```bash
# For each file with "See Also" section
find "$DOCS_DIR" -name "*.md" -type f | while read -r FILE; do
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

#### Anti-Pattern 5: Over-Fragmented Chapters

**Issue**: Chapters with too many subsections (>10) or subsections with minimal content (<100 lines).

**Detection Logic:**
```bash
# For each multi-subsection chapter
find "$DOCS_DIR" -type d -mindepth 1 | while read -r CHAPTER_DIR; do
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

#### Anti-Pattern 6: Stub Navigation Files

**Issue**: Files that are just navigation boilerplate (<50 lines, mostly links).

**Detection Logic:**
```bash
# Find small files
find "$DOCS_DIR" -name "*.md" -type f -exec sh -c 'wc -l "$1" | awk "$1 < 50 {print \$2}"' _ {} \; | while read -r FILE; do
  # Check if file is mostly links
  LINK_COUNT=$(grep -c '^\s*-\s*\[.*\](' "$FILE")
  LINE_COUNT=$(wc -l < "$FILE")

  # If more than 50% links, it's a navigation stub
  if [ "$((LINK_COUNT * 2))" -gt "$LINE_COUNT" ]; then
    echo "STUB_FILE: $FILE is only $LINE_COUNT lines with $LINK_COUNT links"
  fi
done
```

#### Anti-Pattern 7: Meta-Sections in Feature Chapters

**Issue**: "Best Practices" or "Common Patterns" files appear as subsections within feature-focused chapters (like "Advanced Features").

**Detection Logic:**
```bash
# Check mkdocs.yml for meta-sections under feature chapters
grep -A 20 "Advanced Features\|Advanced Topics" mkdocs.yml | while IFS= read -r LINE; do
  # Check if line is a meta-section under feature chapter
  if echo "$LINE" | grep -qi "Best Practices:\|Common Patterns:"; then
    # Extract file path
    FILE=$(echo "$LINE" | grep -oP ':\s*\K.*\.md')

    # Verify it's under a feature-focused chapter
    if [[ "$FILE" =~ ^advanced/ ]]; then
      echo "META_IN_FEATURES: $FILE is meta-section under feature chapter"
    fi
  fi
done
```

#### Anti-Pattern 8: "Subpages" Labels in Navigation

**Issue**: mkdocs.yml navigation contains intermediate labels like "Basics Subpages:", "Advanced Patterns Subpages:", etc. which create unnecessary navigation items instead of nesting pages directly under their parent.

**CRITICAL**: This is a common mistake that makes navigation clunky and unprofessional.

**Detection Logic:**
```bash
# Check mkdocs.yml for "Subpages" labels in navigation
if grep -qi "Subpages:" mkdocs.yml; then
  echo "❌ CRITICAL: Found 'Subpages' labels in mkdocs.yml navigation"
  grep -n "Subpages:" mkdocs.yml | while IFS=: read -r LINENO LINE; do
    echo "  Line $LINENO: $LINE"
  done

  SUBPAGE_COUNT=$(grep -c "Subpages:" mkdocs.yml)
  echo "  Total occurrences: $SUBPAGE_COUNT"
  echo ""
  echo "  ❌ BAD PATTERN:"
  echo "    - Basics: basics/index.md"
  echo "    - Basics Subpages:        ← WRONG! Creates visible nav item"
  echo "        - Page 1: basics/page1.md"
  echo ""
  echo "  ✓ CORRECT PATTERN:"
  echo "    - Basics:"
  echo "        - basics/index.md"
  echo "        - Page 1: basics/page1.md"
fi
```

**Report Format:**
```json
{
  "type": "subpages_navigation_labels",
  "severity": "critical",
  "description": "'Subpages' labels found in navigation structure",
  "files": [{
    "file": "mkdocs.yml",
    "lines": [77, 94, 103, 115, 128],
    "occurrences": 5,
    "labels_found": [
      "Basics Subpages:",
      "Binary Data Subpages:",
      "Common Options Subpages:",
      "Advanced Patterns Subpages:",
      "Troubleshooting Subpages:"
    ],
    "recommendation": "Replace 'Section Subpages:' with nested structure under 'Section:'"
  }],
  "auto_fixable": true
}

### Phase 6: Generate Structural Validation Report

**Compile All Findings:**

```json
{
  "validation_timestamp": "2025-01-10T15:30:00Z",
  "project": "$PROJECT_NAME",
  "docs_dir": "$DOCS_DIR",
  "total_files": 147,
  "total_chapters": 15,
  "mkdocs_specific": {
    "index_md_exists": true,
    "mkdocs_build_passed": false,
    "orphaned_files": ["advanced/sessions.md", "config/global.md"],
    "missing_files": [],
    "broken_links_count": 23,
    "build_errors_count": 0
  },
  "issues_found": [
    {
      "type": "missing_index",
      "severity": "critical",
      "description": "No index.md landing page found",
      "auto_fixable": true
    },
    {
      "type": "orphaned_files",
      "severity": "high",
      "files": ["advanced/sessions.md", "config/global.md"],
      "description": "Files exist but not referenced in mkdocs.yml nav"
    },
    {
      "type": "mkdocs_build_failed",
      "severity": "high",
      "broken_links": 23,
      "errors": 0,
      "description": "mkdocs build --strict failed with broken links"
    },
    {/* Anti-Pattern 1 findings */},
    {/* Anti-Pattern 2 findings */},
    {/* Anti-Pattern 3 findings */},
    {/* Anti-Pattern 4 findings */},
    {/* Anti-Pattern 5 findings */},
    {/* Anti-Pattern 6 findings */},
    {/* Anti-Pattern 7 findings */}
  ],
  "summary": {
    "missing_index": 0,
    "orphaned_files": 2,
    "missing_nav_files": 0,
    "broken_links": 23,
    "redundant_best_practices": 6,
    "best_practices_in_reference": 6,
    "circular_see_also": 12,
    "generic_see_also": 30,
    "over_fragmented_chapters": 3,
    "stub_navigation_files": 8,
    "meta_sections_in_feature_chapters": 2
  },
  "recommendations": [
    "Add 2 orphaned files to mkdocs.yml nav structure",
    "Fix 23 broken links in markdown files",
    "Remove 6 redundant Best Practices sections",
    "Remove 6 Best Practices sections from technical reference pages",
    "Consolidate 3 over-fragmented chapters",
    "Merge 8 stub navigation files into chapter indexes",
    "Remove 2 meta-sections from feature chapters"
  ],
  "status": "issues_found"
}
```

**Write Report:**
```bash
cat > "$OUTPUT" <<EOF
{validation report JSON}
EOF

echo "✓ Structural validation complete"
echo "  Issues found: ${TOTAL_ISSUES}"
echo "  Report written to: $OUTPUT"
```

### Phase 7: Auto-Fix Mode (Optional)

If `--auto-fix true`, perform automatic fixes for clear-cut issues.

**IMPORTANT: Use direct shell commands, NOT Python scripts.**

#### Fix 1: Remove Redundant Best Practices Sections

```bash
# For each redundant Best Practices section
jq -r '.issues[] | select(.type == "redundant_best_practices") | .files[] | "\(.file) \(.lines[0]) \(.lines[1])"' "$OUTPUT" | while read -r FILE START END; do
  FULL_PATH="$DOCS_DIR/$FILE"

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
  FULL_PATH="$DOCS_DIR/$FILE"

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
  STUB_PATH="$DOCS_DIR/$STUB_FILE"
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

  # Update mkdocs.yml to remove stub reference
  STUB_BASENAME=$(basename "$STUB_FILE")
  sed -i.tmp "/- .*: $STUB_FILE/d" mkdocs.yml
  rm mkdocs.yml.tmp 2>/dev/null || true

  echo "  ✓ Consolidated $STUB_FILE into index.md"
done
```

#### Fix 4: Remove Meta-Sections from Feature Chapters

```bash
# For each meta-section in feature chapters
jq -r '.issues[] | select(.type == "meta_sections_in_feature_chapters") | .files[] | .file' "$OUTPUT" | while read -r META_FILE; do
  META_PATH="$DOCS_DIR/$META_FILE"
  META_BASENAME=$(basename "$META_FILE")

  # Remove the file
  if [ -f "$META_PATH" ]; then
    rm "$META_PATH"
    echo "  ✓ Removed meta-section $META_FILE from feature chapter"
  fi

  # Remove from mkdocs.yml
  sed -i.tmp "/- .*: $META_FILE/d" mkdocs.yml
  rm mkdocs.yml.tmp 2>/dev/null || true

  echo "  ✓ Updated mkdocs.yml to remove $META_BASENAME"
done
```

#### Fix 5: Remove "Subpages" Navigation Labels

**CRITICAL FIX**: This automatically restructures the navigation to remove "Subpages" labels.

```bash
# Check if mkdocs.yml has "Subpages:" labels
if grep -qi "Subpages:" mkdocs.yml; then
  echo "  🔧 Fixing 'Subpages' navigation labels..."

  # Backup mkdocs.yml
  cp mkdocs.yml mkdocs.yml.bak

  # Create temporary Python script to fix YAML structure
  python3 <<'PYTHON_SCRIPT'
import re
import sys

with open('mkdocs.yml', 'r') as f:
    lines = f.readlines()

output_lines = []
i = 0
while i < len(lines):
    line = lines[i]

    # Check if this line contains a "Subpages:" label
    if re.match(r'\s*-\s+\w+.*Subpages:\s*$', line):
        # Get the indentation level of the Subpages line
        subpages_indent = len(line) - len(line.lstrip())

        # Find the parent section line (one level up)
        # Look backwards to find the parent
        parent_idx = i - 1
        while parent_idx >= 0:
            parent_line = lines[parent_idx]
            parent_indent = len(parent_line) - len(parent_line.lstrip())

            # Found parent if it's at the same or less indentation and is a section header
            if parent_indent < subpages_indent and re.match(r'\s*-\s+[\w\s]+:', parent_line):
                # Extract parent section name
                parent_match = re.match(r'(\s*-\s+)([\w\s]+):\s*(.*)$', parent_line)
                if parent_match:
                    indent, section_name, rest = parent_match.groups()

                    # If parent has a file reference like "Section: file.md", convert to nested structure
                    if rest.strip():
                        # Replace parent with nested structure
                        output_lines[parent_idx] = f"{indent}{section_name}:\n"
                        output_lines.append(f"{indent}    - {rest.strip()}\n")

                break
            parent_idx -= 1

        # Skip the "Subpages:" line itself
        i += 1
        continue

    output_lines.append(line)
    i += 1

# Write the fixed YAML
with open('mkdocs.yml', 'w') as f:
    f.writelines(output_lines)

print("✓ Fixed 'Subpages' navigation structure")
PYTHON_SCRIPT

  SUBPAGE_COUNT=$(grep -c "Subpages:" mkdocs.yml.bak)
  echo "  ✓ Removed $SUBPAGE_COUNT 'Subpages' labels from navigation"
fi
```

**Cleanup Backups:**
```bash
# Remove backup files after successful fixes
find "$DOCS_DIR" -name "*.bak" -delete
```

**Commit Auto-Fixes:**
```bash
if [ "$AUTO_FIX" = "true" ]; then
  git add "$DOCS_DIR" mkdocs.yml
  git commit -m "docs: holistic cleanup after drift detection (mkdocs)

- Removed $REDUNDANT_COUNT redundant Best Practices sections
- Removed $REFERENCE_COUNT Best Practices from technical reference pages
- Consolidated $STUB_COUNT stub navigation files

Based on structural validation report: $OUTPUT"

  echo "✓ Auto-fixes committed"
fi
```

### Phase 8: Summary Output

**If Auto-Fix Enabled:**
```
✓ Structural Validation Complete (MkDocs)
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

See detailed report: .prodigy/mkdocs-validation.json
```

**If Reporting Only:**
```
✓ Structural Validation Complete (MkDocs)
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

Detailed report: .prodigy/mkdocs-validation.json
```

### Success Criteria

**MkDocs-Specific Checks:**
- [ ] index.md exists (or created if auto-fix enabled)
- [ ] All files in docs/ are included in mkdocs.yml nav
- [ ] All files in nav exist in docs/
- [ ] mkdocs build --strict passes without errors

**Content Validation:**
- [ ] All chapters scanned and categorized
- [ ] All Best Practices sections identified and validated
- [ ] Redundancy detected across chapters
- [ ] Over-fragmentation detected
- [ ] Stub navigation files identified
- [ ] Circular references detected
- [ ] Validation report generated with severity levels
- [ ] Auto-fix mode works correctly (if enabled)
- [ ] mkdocs build succeeds after auto-fixes

### Error Handling

**MkDocs build fails:**
```
Error: MkDocs build failed after auto-fixes
Run: mkdocs build --strict
Review errors and manually fix broken links.
```

**Invalid mkdocs structure:**
```
Error: Could not parse mkdocs.yml
Ensure mkdocs.yml exists and follows MkDocs YAML format.
```

**No issues found:**
```
✓ MkDocs validation passed
No cross-cutting issues detected.
```
