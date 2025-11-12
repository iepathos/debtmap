# /prodigy-validate-mkdocs-page

Validate that a MkDocs page fix meets quality standards and all drift issues are resolved.

This command analyzes the fixed documentation page and produces a validation report with completion percentage and remaining gaps.

## Variables

- `--project <name>` - Project name (e.g., "Prodigy")
- `--json <item>` - JSON object containing page details from flattened-items.json
- `--output <path>` - Path to write validation results (JSON format)

## Execute

### Phase 1: Parse Parameters

**Extract Parameters:**
```bash
PROJECT_NAME="<value from --project parameter>"
ITEM_JSON="<value from --json parameter>"
OUTPUT_PATH="<value from --output parameter, default: .prodigy/validation-result.json>"
```

**Parse Page Details:**
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

### Phase 2: Load Drift Report

Read the drift report to understand what issues should have been fixed:

```bash
TOTAL_ISSUES=$(jq '.issues | length' ${DRIFT_REPORT})
CRITICAL_ISSUES=$(jq '[.issues[] | select(.severity == "critical")] | length' ${DRIFT_REPORT})
HIGH_ISSUES=$(jq '[.issues[] | select(.severity == "high")] | length' ${DRIFT_REPORT})
MEDIUM_ISSUES=$(jq '[.issues[] | select(.severity == "medium")] | length' ${DRIFT_REPORT})
LOW_ISSUES=$(jq '[.issues[] | select(.severity == "low")] | length' ${DRIFT_REPORT})
```

### Phase 3: Validate Content Metrics

**Count Content Elements:**
```bash
# Get actual content line count (excluding blank lines and single-word headers)
LINE_COUNT=$(grep -v '^$' "${ITEM_FILE}" | grep -v '^#\s*$' | wc -l | tr -d ' ')

# Count headings (##, ###, ####)
HEADING_COUNT=$(grep '^##' "${ITEM_FILE}" | wc -l | tr -d ' ')

# Count code blocks (```)
CODE_BLOCK_COUNT=$(grep -c '```' "${ITEM_FILE}" || echo 0)

# Count source references (look for file paths with line numbers or "Source:" markers)
SOURCE_REF_COUNT=$(grep -cE 'Source:|[a-zA-Z0-9_./\-]+\.[a-z]{2,4}:[0-9]+' "${ITEM_FILE}" || echo 0)

# Count MkDocs Material admonitions
ADMONITION_COUNT=$(grep -cE '^\!\!\!' "${ITEM_FILE}" || echo 0)
```

**Determine Minimum Requirements:**
```bash
if [ "$ITEM_TYPE" = "section-page" ]; then
  MIN_LINES=50
  MIN_HEADINGS=3
  MIN_EXAMPLES=2
  MIN_SOURCES=1
  MIN_ADMONITIONS=1
else
  # Top-level page or overview
  MIN_LINES=100
  MIN_HEADINGS=5
  MIN_EXAMPLES=3
  MIN_SOURCES=2
  MIN_ADMONITIONS=2
fi
```

**Calculate Content Score (0-100):**
```bash
# Each metric worth a portion of content score
LINE_SCORE=$((LINE_COUNT >= MIN_LINES ? 20 : (LINE_COUNT * 20 / MIN_LINES)))
HEADING_SCORE=$((HEADING_COUNT >= MIN_HEADINGS ? 20 : (HEADING_COUNT * 20 / MIN_HEADINGS)))
EXAMPLE_SCORE=$((CODE_BLOCK_COUNT >= MIN_EXAMPLES ? 20 : (CODE_BLOCK_COUNT * 20 / MIN_EXAMPLES)))
SOURCE_SCORE=$((SOURCE_REF_COUNT >= MIN_SOURCES ? 20 : (SOURCE_REF_COUNT * 20 / MIN_SOURCES)))
ADMONITION_SCORE=$((ADMONITION_COUNT >= MIN_ADMONITIONS ? 20 : (ADMONITION_COUNT * 20 / MIN_ADMONITIONS)))

CONTENT_SCORE=$((LINE_SCORE + HEADING_SCORE + EXAMPLE_SCORE + SOURCE_SCORE + ADMONITION_SCORE))
```

### Phase 4: Validate Source References

**Check that referenced files exist:**
```bash
MISSING_FILES=()

# Extract file paths from markdown (patterns like: path/to/file.ext:line or [text](path/to/file.ext))
grep -oE '\([^)]*\.[a-z]{2,4}(:[0-9]+)?\)|\b[a-zA-Z0-9_./\-]+\.[a-z]{2,4}:[0-9]+|Source:\s*[a-zA-Z0-9_./\-]+\.[a-z]{2,4}' "${ITEM_FILE}" | while read -r source_ref; do
  # Extract file path (remove parentheses, line numbers, "Source:" prefix, etc)
  FILE_PATH=$(echo "$source_ref" | sed 's/[():].*//g' | sed 's/Source:\s*//g' | sed 's/^\s*//' | sed 's/\s*$//')

  if [ -n "$FILE_PATH" ] && [ ! -f "$FILE_PATH" ]; then
    MISSING_FILES+=("$FILE_PATH")
  fi
done

MISSING_FILE_COUNT=${#MISSING_FILES[@]}
```

**Calculate Source Validation Score (0-100):**
```bash
if [ $SOURCE_REF_COUNT -eq 0 ]; then
  SOURCE_VALIDATION_SCORE=0
elif [ $MISSING_FILE_COUNT -eq 0 ]; then
  SOURCE_VALIDATION_SCORE=100
else
  # Deduct points for each missing file
  INVALID_PERCENT=$((MISSING_FILE_COUNT * 100 / SOURCE_REF_COUNT))
  SOURCE_VALIDATION_SCORE=$((100 - INVALID_PERCENT))
fi
```

### Phase 5: Validate Drift Resolution

**Check which drift issues appear to be addressed:**

For each issue in the drift report, check if the relevant section exists in the updated file:

```bash
RESOLVED_CRITICAL=0
RESOLVED_HIGH=0
RESOLVED_MEDIUM=0
RESOLVED_LOW=0

# For each issue, check if the section mentioned exists and has content
jq -r '.issues[] | @json' ${DRIFT_REPORT} | while read -r issue_json; do
  SEVERITY=$(echo "$issue_json" | jq -r '.severity')
  SECTION=$(echo "$issue_json" | jq -r '.section // ""')
  DESCRIPTION=$(echo "$issue_json" | jq -r '.description')

  # Check if section heading exists in file
  if [ -n "$SECTION" ]; then
    if grep -q "^## ${SECTION}" "${ITEM_FILE}"; then
      # Section exists - check it has substantial content (>5 lines after heading)
      SECTION_LINES=$(awk "/^## ${SECTION}/,/^## /" "${ITEM_FILE}" | wc -l)

      if [ $SECTION_LINES -gt 5 ]; then
        case "$SEVERITY" in
          critical) RESOLVED_CRITICAL=$((RESOLVED_CRITICAL + 1)) ;;
          high) RESOLVED_HIGH=$((RESOLVED_HIGH + 1)) ;;
          medium) RESOLVED_MEDIUM=$((RESOLVED_MEDIUM + 1)) ;;
          low) RESOLVED_LOW=$((RESOLVED_LOW + 1)) ;;
        esac
      fi
    fi
  fi
done

TOTAL_RESOLVED=$((RESOLVED_CRITICAL + RESOLVED_HIGH + RESOLVED_MEDIUM + RESOLVED_LOW))
```

**Calculate Drift Resolution Score (0-100):**
```bash
if [ $TOTAL_ISSUES -eq 0 ]; then
  DRIFT_RESOLUTION_SCORE=100
else
  # Weight by severity: critical=50%, high=30%, medium=15%, low=5%
  WEIGHTED_TOTAL=$((CRITICAL_ISSUES * 50 + HIGH_ISSUES * 30 + MEDIUM_ISSUES * 15 + LOW_ISSUES * 5))
  WEIGHTED_RESOLVED=$((RESOLVED_CRITICAL * 50 + RESOLVED_HIGH * 30 + RESOLVED_MEDIUM * 15 + RESOLVED_LOW * 5))

  DRIFT_RESOLUTION_SCORE=$((WEIGHTED_RESOLVED * 100 / WEIGHTED_TOTAL))
fi
```

### Phase 6: Validate MkDocs Material Usage

**Check for MkDocs Material features:**
```bash
# Count admonitions (should be used for important notes)
HAS_ADMONITIONS=$(grep -q '^\!\!\!' "${ITEM_FILE}" && echo "true" || echo "false")

# Check for titled code blocks
HAS_TITLED_CODE=$(grep -q '```[a-z]* title=' "${ITEM_FILE}" && echo "true" || echo "false")

# Check for content tabs (=== syntax)
HAS_TABS=$(grep -q '^===' "${ITEM_FILE}" && echo "true" || echo "false")

# Calculate Material features score
MATERIAL_SCORE=0
[ "$HAS_ADMONITIONS" = "true" ] && MATERIAL_SCORE=$((MATERIAL_SCORE + 40))
[ "$HAS_TITLED_CODE" = "true" ] && MATERIAL_SCORE=$((MATERIAL_SCORE + 30))
[ "$HAS_TABS" = "true" ] && MATERIAL_SCORE=$((MATERIAL_SCORE + 30))
```

### Phase 7: Calculate Overall Completion Score

**Weighted Average:**
```bash
# Content: 35%, Drift Resolution: 40%, Source Validation: 15%, Material Features: 10%
COMPLETION_SCORE=$(( (CONTENT_SCORE * 35 + DRIFT_RESOLUTION_SCORE * 40 + SOURCE_VALIDATION_SCORE * 15 + MATERIAL_SCORE * 10) / 100 ))
```

**Determine Status:**
```bash
if [ $COMPLETION_SCORE -ge 100 ]; then
  STATUS="excellent"
elif [ $COMPLETION_SCORE -ge 90 ]; then
  STATUS="good"
elif [ $COMPLETION_SCORE -ge 75 ]; then
  STATUS="acceptable"
elif [ $COMPLETION_SCORE -ge 60 ]; then
  STATUS="needs_improvement"
else
  STATUS="incomplete"
fi
```

### Phase 8: Identify Remaining Gaps

**Create Gap List:**
```json
{
  "gaps": [
    {
      "category": "content",
      "severity": "medium",
      "message": "Page is below minimum line count (current: X, minimum: Y)",
      "current_value": LINE_COUNT,
      "required_value": MIN_LINES
    },
    {
      "category": "source_attribution",
      "severity": "high",
      "message": "Code examples lack source references (found: X, minimum: Y)",
      "current_value": SOURCE_REF_COUNT,
      "required_value": MIN_SOURCES
    },
    {
      "category": "drift_resolution",
      "severity": "critical",
      "message": "Unresolved critical drift issues",
      "unresolved_issues": [
        "Issue description 1",
        "Issue description 2"
      ]
    },
    {
      "category": "invalid_references",
      "severity": "high",
      "message": "Found invalid file references",
      "missing_files": MISSING_FILES
    },
    {
      "category": "material_features",
      "severity": "low",
      "message": "Could benefit from more MkDocs Material features (admonitions, tabs, titled code blocks)"
    }
  ]
}
```

**Only include gaps that are actually present.**

### Phase 9: Create Validation Report

**Write Validation Result:**

```json
{
  "page_id": "$ITEM_ID",
  "page_file": "$ITEM_FILE",
  "completion_score": COMPLETION_SCORE,
  "status": "$STATUS",
  "breakdown": {
    "content_score": CONTENT_SCORE,
    "drift_resolution_score": DRIFT_RESOLUTION_SCORE,
    "source_validation_score": SOURCE_VALIDATION_SCORE,
    "material_features_score": MATERIAL_SCORE
  },
  "metrics": {
    "line_count": LINE_COUNT,
    "heading_count": HEADING_COUNT,
    "code_block_count": CODE_BLOCK_COUNT,
    "source_reference_count": SOURCE_REF_COUNT,
    "admonition_count": ADMONITION_COUNT
  },
  "drift_resolution": {
    "total_issues": TOTAL_ISSUES,
    "resolved_issues": TOTAL_RESOLVED,
    "unresolved_critical": $((CRITICAL_ISSUES - RESOLVED_CRITICAL)),
    "unresolved_high": $((HIGH_ISSUES - RESOLVED_HIGH)),
    "unresolved_medium": $((MEDIUM_ISSUES - RESOLVED_MEDIUM)),
    "unresolved_low": $((LOW_ISSUES - RESOLVED_LOW))
  },
  "gaps": [
    // Array of remaining gaps from Phase 8
  ],
  "validation_passed": COMPLETION_SCORE >= 100,
  "validation_timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
```

**Save to Output Path:**
```bash
echo "$VALIDATION_REPORT" | jq '.' > "$OUTPUT_PATH"
```

### Phase 10: Display Summary

**Print User-Friendly Summary:**
```
📊 MkDocs Page Validation Results
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Page: $ITEM_TITLE
File: $ITEM_FILE

Overall Score: $COMPLETION_SCORE/100 ($STATUS)

Component Scores:
  • Content Quality:      $CONTENT_SCORE/100
  • Drift Resolution:     $DRIFT_RESOLUTION_SCORE/100
  • Source Validation:    $SOURCE_VALIDATION_SCORE/100
  • Material Features:    $MATERIAL_SCORE/100

Drift Issues:
  ✓ Resolved: $TOTAL_RESOLVED/$TOTAL_ISSUES
  ✗ Unresolved: $((TOTAL_ISSUES - TOTAL_RESOLVED))
    - Critical: $((CRITICAL_ISSUES - RESOLVED_CRITICAL))
    - High:     $((HIGH_ISSUES - RESOLVED_HIGH))
    - Medium:   $((MEDIUM_ISSUES - RESOLVED_MEDIUM))
    - Low:      $((LOW_ISSUES - RESOLVED_LOW))

$(if [ ${#GAPS[@]} -gt 0 ]; then
  echo "🔍 Remaining Gaps:"
  for gap in "${GAPS[@]}"; do
    echo "  • $gap"
  done
fi)

$(if [ $COMPLETION_SCORE -ge 100 ]; then
  echo "✅ Validation PASSED - Documentation meets quality standards"
else
  echo "⚠️  Validation incomplete - Additional work needed"
fi)
```

### Phase 11: Exit with Appropriate Code

```bash
if [ $COMPLETION_SCORE -ge 100 ]; then
  exit 0  # Success - validation passed
elif [ $COMPLETION_SCORE -ge 75 ]; then
  exit 0  # Acceptable - allow workflow to continue
else
  exit 1  # Incomplete - trigger complete-mkdocs-fix
fi
```

### Quality Guidelines

**Accuracy:**
- Accurately count content elements
- Properly detect resolved drift issues
- Validate source references thoroughly
- Check MkDocs Material usage correctly

**Clarity:**
- Provide clear gap descriptions
- Show specific metrics that fell short
- Explain what needs to be fixed
- Give actionable feedback

**Consistency:**
- Use consistent scoring methodology
- Apply same standards to all pages
- Weight severity appropriately
- Threshold values are fair and achievable

### Success Indicators

Validation is successful when:
- All metrics accurately measured
- Drift resolution properly assessed
- Source references validated
- Clear gaps identified
- Validation report saved to disk
- User-friendly summary displayed
- Appropriate exit code returned
