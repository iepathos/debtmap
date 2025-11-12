# /prodigy-validate-feature-consistency

Validate that MkDocs feature usage is consistent across all documentation pages after map phase enhancement. This runs in the reduce phase as a final quality check.

## Variables

- `--project <name>` - Project name (e.g., "Prodigy")
- `--docs-dir <path>` - Path to mkdocs docs directory (default: "docs")
- `--output <path>` - Path to write consistency report (default: ".prodigy/feature-consistency.json")

## Execute

### Phase 1: Understand Context

You are performing a **consistency check** after the map phase has enhanced individual pages. This is NOT about adding features (that happens per-page in map phase), but about checking that:

1. **Mermaid is enabled** in mkdocs.yml
2. **All pages have reasonable admonition density** (no pages left behind)
3. **Diagram coverage is good** for complex topics
4. **Feature usage is consistent** (not wildly different across pages)

**Your Goal**: Generate a report identifying any pages that may have been missed or need manual review.

### Phase 2: Extract Parameters

```bash
PROJECT_NAME="${project:?Error: --project is required}"
DOCS_DIR="${docs_dir:-docs}"
OUTPUT="${output:-.prodigy/feature-consistency.json}"

# Validate
if [ ! -d "$DOCS_DIR" ]; then
    echo "Error: Docs directory not found: $DOCS_DIR"
    exit 1
fi

if [ ! -f "mkdocs.yml" ]; then
    echo "Error: mkdocs.yml not found"
    exit 1
fi

echo "Validating feature consistency for $PROJECT_NAME"
echo "  Docs directory: $DOCS_DIR"
```

### Phase 3: Check Feature Enablement

```bash
# Check if Mermaid is enabled
if grep -q "pymdownx.superfences" mkdocs.yml && grep -q "mermaid" mkdocs.yml; then
    MERMAID_ENABLED=true
    echo "✓ Mermaid diagrams enabled"
else
    MERMAID_ENABLED=false
    echo "⚠ Mermaid NOT enabled"
fi

# Check other features
ADMONITIONS_ENABLED=$(grep -q "^  - admonition" mkdocs.yml && echo "true" || echo "false")
TABBED_ENABLED=$(grep -q "pymdownx.tabbed" mkdocs.yml && echo "true" || echo "false")
```

### Phase 4: Measure Feature Usage Across All Pages

```bash
echo ""
echo "Measuring feature usage across documentation..."

# Count total features
TOTAL_FILES=$(find "$DOCS_DIR" -name "*.md" -type f | wc -l)
TOTAL_DIAGRAMS=$(find "$DOCS_DIR" -name "*.md" -exec grep -c "^\`\`\`mermaid" {} + 2>/dev/null | awk '{sum+=$1} END {print sum}')
TOTAL_ADMONITIONS=$(find "$DOCS_DIR" -name "*.md" -exec grep -c "^!!!" {} + 2>/dev/null | awk '{sum+=$1} END {print sum}')
TOTAL_TABS=$(find "$DOCS_DIR" -name "*.md" -exec grep -c "^=== " {} + 2>/dev/null | awk '{sum+=$1} END {print sum}')

# Calculate metrics
FILES_WITH_DIAGRAMS=$(find "$DOCS_DIR" -name "*.md" -exec grep -l "^\`\`\`mermaid" {} + 2>/dev/null | wc -l)
FILES_WITH_ADMONITIONS=$(find "$DOCS_DIR" -name "*.md" -exec grep -l "^!!!" {} + 2>/dev/null | wc -l)

DIAGRAM_COVERAGE=$(awk "BEGIN {printf \"%.1f\", ($FILES_WITH_DIAGRAMS / $TOTAL_FILES) * 100}")
ADMONITION_COVERAGE=$(awk "BEGIN {printf \"%.1f\", ($FILES_WITH_ADMONITIONS / $TOTAL_FILES) * 100}")

echo "  Total files: $TOTAL_FILES"
echo "  Total diagrams: $TOTAL_DIAGRAMS"
echo "  Total admonitions: $TOTAL_ADMONITIONS"
echo "  Total tabs: $TOTAL_TABS"
echo ""
echo "  Files with diagrams: $FILES_WITH_DIAGRAMS ($DIAGRAM_COVERAGE%)"
echo "  Files with admonitions: $FILES_WITH_ADMONITIONS ($ADMONITION_COVERAGE%)"
```

### Phase 5: Identify Low-Quality Pages

```bash
echo ""
echo "Identifying pages that may need review..."

# Find pages with low admonition density
find "$DOCS_DIR" -name "*.md" -type f | while read -r FILE; do
    LINES=$(wc -l < "$FILE")
    ADMONS=$(grep -c "^!!!" "$FILE" 2>/dev/null || echo 0)

    # Skip very short files
    if [ "$LINES" -lt 50 ]; then
        continue
    fi

    # Calculate density
    DENSITY=$(awk "BEGIN {printf \"%.2f\", ($ADMONS / $LINES) * 100}")

    # Flag if density < 0.5 (less than 1 admonition per 200 lines)
    if (( $(echo "$DENSITY < 0.5" | bc -l) )); then
        echo "$FILE|$LINES|$ADMONS|$DENSITY"
    fi
done > /tmp/low-quality-pages.txt

LOW_QUALITY_COUNT=$(wc -l < /tmp/low-quality-pages.txt)

if [ "$LOW_QUALITY_COUNT" -gt 0 ]; then
    echo "⚠ Found $LOW_QUALITY_COUNT page(s) with low admonition density:"
    head -5 /tmp/low-quality-pages.txt | while IFS='|' read -r FILE LINES ADMONS DENSITY; do
        echo "  - ${FILE#$DOCS_DIR/} ($ADMONS admonitions in $LINES lines)"
    done
    [ "$LOW_QUALITY_COUNT" -gt 5 ] && echo "  ... and $((LOW_QUALITY_COUNT - 5)) more"
fi
```

### Phase 6: Check Complex Topics Have Diagrams

```bash
# List of files that should have diagrams (complex topics)
COMPLEX_TOPICS=(
    "mapreduce/overview.md"
    "advanced/git-integration.md"
    "workflow-basics/error-handling.md"
    "advanced/sessions.md"
)

echo ""
echo "Checking diagram coverage for complex topics..."

MISSING_DIAGRAMS=()
for TOPIC in "${COMPLEX_TOPICS[@]}"; do
    FILE="$DOCS_DIR/$TOPIC"
    if [ -f "$FILE" ]; then
        if ! grep -q "^\`\`\`mermaid" "$FILE"; then
            MISSING_DIAGRAMS+=("$TOPIC")
            echo "  ⚠ $TOPIC has no diagram"
        else
            echo "  ✓ $TOPIC has diagram"
        fi
    fi
done

MISSING_DIAGRAM_COUNT=${#MISSING_DIAGRAMS[@]}
```

### Phase 7: Generate Consistency Report

```bash
cat > "$OUTPUT" <<EOF
{
  "validation_timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "project": "$PROJECT_NAME",
  "docs_dir": "$DOCS_DIR",

  "feature_enablement": {
    "mermaid": $MERMAID_ENABLED,
    "admonitions": $ADMONITIONS_ENABLED,
    "tabbed": $TABBED_ENABLED
  },

  "overall_metrics": {
    "total_files": $TOTAL_FILES,
    "total_diagrams": $TOTAL_DIAGRAMS,
    "total_admonitions": $TOTAL_ADMONITIONS,
    "total_tabs": $TOTAL_TABS,
    "files_with_diagrams": $FILES_WITH_DIAGRAMS,
    "files_with_admonitions": $FILES_WITH_ADMONITIONS,
    "diagram_coverage_percent": $DIAGRAM_COVERAGE,
    "admonition_coverage_percent": $ADMONITION_COVERAGE
  },

  "quality_issues": {
    "low_quality_pages": $LOW_QUALITY_COUNT,
    "missing_diagrams": $MISSING_DIAGRAM_COUNT,
    "mermaid_not_enabled": $([ "$MERMAID_ENABLED" = "false" ] && echo "true" || echo "false")
  },

  "recommendations": [
    $([ "$MERMAID_ENABLED" = "false" ] && echo '"Enable Mermaid in mkdocs.yml",')
    $([ "$LOW_QUALITY_COUNT" -gt 0 ] && echo "\"Review $LOW_QUALITY_COUNT low-quality pages\",")
    $([ "$MISSING_DIAGRAM_COUNT" -gt 0 ] && echo "\"Add diagrams to $MISSING_DIAGRAM_COUNT complex topics\",")
    null
  ],

  "status": "$([ "$LOW_QUALITY_COUNT" -eq 0 ] && [ "$MISSING_DIAGRAM_COUNT" -eq 0 ] && echo "excellent" || echo "needs_review")"
}
EOF

# Clean up null entries
sed -i.tmp 's/,\s*null//g; s/\[\s*null\s*\]/[]/' "$OUTPUT"
rm "$OUTPUT.tmp" 2>/dev/null || true

echo ""
echo "✓ Consistency report written to: $OUTPUT"
```

### Phase 8: Summary Output

```bash
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✓ Feature Consistency Check Complete"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "Overall Quality:"
echo "  Total files: $TOTAL_FILES"
echo "  Diagrams: $TOTAL_DIAGRAMS ($DIAGRAM_COVERAGE% coverage)"
echo "  Admonitions: $TOTAL_ADMONITIONS ($ADMONITION_COVERAGE% coverage)"
echo "  Tabs: $TOTAL_TABS"
echo ""

if [ "$LOW_QUALITY_COUNT" -eq 0 ] && [ "$MISSING_DIAGRAM_COUNT" -eq 0 ]; then
    echo "✅ All pages meet quality standards"
    echo "No manual review needed."
else
    echo "Issues Requiring Review:"
    [ "$LOW_QUALITY_COUNT" -gt 0 ] && echo "  • $LOW_QUALITY_COUNT page(s) with low admonition density"
    [ "$MISSING_DIAGRAM_COUNT" -gt 0 ] && echo "  • $MISSING_DIAGRAM_COUNT complex topic(s) missing diagrams"
    echo ""
    echo "See detailed report: $OUTPUT"
fi

echo ""

# Exit with appropriate code
if [ "$LOW_QUALITY_COUNT" -gt 5 ] || [ "$MISSING_DIAGRAM_COUNT" -gt 2 ]; then
    exit 1  # Significant quality issues
else
    exit 0  # Minor or no issues
fi
```

### Success Criteria

- [ ] Feature enablement checked (Mermaid, admonitions, tabs)
- [ ] Overall metrics calculated (coverage, totals)
- [ ] Low-quality pages identified
- [ ] Complex topics checked for diagrams
- [ ] Consistency report generated
- [ ] Summary output provided

### Quality Thresholds

**Good Quality:**
- Admonition coverage: >80%
- Diagram coverage: >40% (not all pages need diagrams)
- Low-quality pages: <5
- All complex topics have diagrams

**Needs Review:**
- Admonition coverage: 50-80%
- Diagram coverage: 20-40%
- Low-quality pages: 5-10
- Some complex topics missing diagrams

**Poor Quality:**
- Admonition coverage: <50%
- Diagram coverage: <20%
- Low-quality pages: >10
- Many complex topics missing diagrams

### Error Handling

**No issues found:**
```
✅ All pages meet quality standards
Overall quality: Excellent
```

**Some issues found:**
```
⚠ Feature Consistency Issues
  • 3 page(s) with low admonition density
  • 1 complex topic missing diagram

See report for details.
```

**Mermaid not enabled:**
```
❌ CRITICAL: Mermaid not enabled
Cannot validate diagram usage.
Enable Mermaid in mkdocs.yml first.
```
