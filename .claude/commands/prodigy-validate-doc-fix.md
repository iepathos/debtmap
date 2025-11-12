# /prodigy-validate-doc-fix

Validate that a documentation fix meets quality standards and all drift issues are resolved.

This command analyzes the fixed documentation file and produces a validation report with completion percentage and remaining gaps.

## Variables

- `--project <name>` - Project name (e.g., "Prodigy")
- `--json <item>` - JSON object containing chapter or subsection details
- `--output <path>` - Path to write validation results (JSON format)

## Execute

### Phase 1: Parse Parameters

**Extract Parameters:**
```bash
PROJECT_NAME="<value from --project parameter>"
ITEM_JSON="<value from --json parameter>"
OUTPUT_PATH="<value from --output parameter, default: .prodigy/validation-result.json>"
```

**Parse Item Details:**
```bash
ITEM_TYPE=$(echo "$ITEM_JSON" | jq -r '.type // "single-file"')
ITEM_ID=$(echo "$ITEM_JSON" | jq -r '.id')
ITEM_FILE=$(echo "$ITEM_JSON" | jq -r '.file')
ITEM_TITLE=$(echo "$ITEM_JSON" | jq -r '.title')

if [ "$ITEM_TYPE" = "subsection" ]; then
  PARENT_CHAPTER_ID=$(echo "$ITEM_JSON" | jq -r '.parent_chapter_id')
  DRIFT_REPORT=".prodigy/book-analysis/drift-${PARENT_CHAPTER_ID}-${ITEM_ID}.json"
else
  DRIFT_REPORT=".prodigy/book-analysis/drift-${ITEM_ID}.json"
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
```

**Determine Minimum Requirements:**
```bash
if [ "$ITEM_TYPE" = "subsection" ]; then
  MIN_LINES=50
  MIN_HEADINGS=3
  MIN_EXAMPLES=2
  MIN_SOURCES=1
else
  # Single-file chapter
  MIN_LINES=100
  MIN_HEADINGS=5
  MIN_EXAMPLES=3
  MIN_SOURCES=2
fi
```

**Calculate Content Score (0-100):**
```bash
# Each metric worth 25% of content score
LINE_SCORE=$((LINE_COUNT >= MIN_LINES ? 25 : (LINE_COUNT * 25 / MIN_LINES)))
HEADING_SCORE=$((HEADING_COUNT >= MIN_HEADINGS ? 25 : (HEADING_COUNT * 25 / MIN_HEADINGS)))
EXAMPLE_SCORE=$((CODE_BLOCK_COUNT >= MIN_EXAMPLES ? 25 : (CODE_BLOCK_COUNT * 25 / MIN_EXAMPLES)))
SOURCE_SCORE=$((SOURCE_REF_COUNT >= MIN_SOURCES ? 25 : (SOURCE_REF_COUNT * 25 / MIN_SOURCES)))

CONTENT_SCORE=$((LINE_SCORE + HEADING_SCORE + EXAMPLE_SCORE + SOURCE_SCORE))
```

### Phase 4: Validate Source References

**Check that referenced files exist:**
```bash
MISSING_FILES=()

# Extract file paths from markdown (patterns like: path/to/file.ext:line or [text](path/to/file.ext))
grep -oE '\([^)]*\.[a-z]{2,4}(:[0-9]+)?\)|\b[a-zA-Z0-9_./\-]+\.[a-z]{2,4}:[0-9]+' "${ITEM_FILE}" | while read -r source_ref; do
  # Extract file path (remove parentheses, line numbers, etc)
  FILE_PATH=$(echo "$source_ref" | sed 's/[():].*//g' | sed 's/^\s*//' | sed 's/\s*$//')

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
        case $SEVERITY in
          critical) RESOLVED_CRITICAL=$((RESOLVED_CRITICAL + 1)) ;;
          high) RESOLVED_HIGH=$((RESOLVED_HIGH + 1)) ;;
          medium) RESOLVED_MEDIUM=$((RESOLVED_MEDIUM + 1)) ;;
          low) RESOLVED_LOW=$((RESOLVED_LOW + 1)) ;;
        esac
      fi
    fi
  fi
done

# Calculate drift resolution score (critical and high issues are mandatory)
if [ $CRITICAL_ISSUES -gt 0 ] || [ $HIGH_ISSUES -gt 0 ]; then
  REQUIRED_FIXED=$((CRITICAL_ISSUES + HIGH_ISSUES))
  ACTUALLY_FIXED=$((RESOLVED_CRITICAL + RESOLVED_HIGH))

  if [ $ACTUALLY_FIXED -eq 0 ]; then
    DRIFT_RESOLUTION_SCORE=0
  else
    DRIFT_RESOLUTION_SCORE=$((ACTUALLY_FIXED * 100 / REQUIRED_FIXED))
  fi
else
  # No critical/high issues, so score medium/low resolution
  if [ $MEDIUM_ISSUES -gt 0 ] || [ $LOW_ISSUES -gt 0 ]; then
    TOTAL_MEDIUM_LOW=$((MEDIUM_ISSUES + LOW_ISSUES))
    FIXED_MEDIUM_LOW=$((RESOLVED_MEDIUM + RESOLVED_LOW))
    DRIFT_RESOLUTION_SCORE=$((FIXED_MEDIUM_LOW * 100 / TOTAL_MEDIUM_LOW))
  else
    # No issues at all - perfect score
    DRIFT_RESOLUTION_SCORE=100
  fi
fi
```

### Phase 6: Calculate Overall Completion

**Weight the scores:**
- Content metrics: 40%
- Source validation: 30%
- Drift resolution: 30%

```bash
OVERALL_COMPLETION=$(( (CONTENT_SCORE * 40 + SOURCE_VALIDATION_SCORE * 30 + DRIFT_RESOLUTION_SCORE * 30) / 100 ))
```

### Phase 7: Identify Remaining Gaps

**Build list of specific gaps:**

```json
{
  "gaps": [
    {
      "category": "content_length",
      "severity": "high",
      "message": "Content too short: ${LINE_COUNT} lines (need ${MIN_LINES})"
    },
    {
      "category": "missing_examples",
      "severity": "high",
      "message": "Insufficient examples: ${CODE_BLOCK_COUNT} (need ${MIN_EXAMPLES})"
    },
    {
      "category": "missing_sources",
      "severity": "critical",
      "message": "Missing source references: ${SOURCE_REF_COUNT} (need ${MIN_SOURCES})"
    },
    {
      "category": "invalid_references",
      "severity": "critical",
      "message": "Referenced files don't exist: ${MISSING_FILES}"
    },
    {
      "category": "unresolved_drift",
      "severity": "critical",
      "message": "Critical drift issues not resolved: ${CRITICAL_ISSUES - RESOLVED_CRITICAL} remaining"
    }
  ]
}
```

### Phase 8: Write Validation Report

**Create JSON output:**

```json
{
  "item_id": "${ITEM_ID}",
  "item_type": "${ITEM_TYPE}",
  "item_title": "${ITEM_TITLE}",
  "item_file": "${ITEM_FILE}",
  "validation_timestamp": "<current-timestamp>",
  "overall_completion": ${OVERALL_COMPLETION},
  "scores": {
    "content_metrics": {
      "score": ${CONTENT_SCORE},
      "weight": 40,
      "details": {
        "lines": ${LINE_COUNT},
        "min_lines": ${MIN_LINES},
        "headings": ${HEADING_COUNT},
        "min_headings": ${MIN_HEADINGS},
        "code_blocks": ${CODE_BLOCK_COUNT},
        "min_code_blocks": ${MIN_EXAMPLES},
        "source_references": ${SOURCE_REF_COUNT},
        "min_source_references": ${MIN_SOURCES}
      }
    },
    "source_validation": {
      "score": ${SOURCE_VALIDATION_SCORE},
      "weight": 30,
      "details": {
        "total_references": ${SOURCE_REF_COUNT},
        "missing_files": ${MISSING_FILE_COUNT},
        "missing_file_list": ${MISSING_FILES}
      }
    },
    "drift_resolution": {
      "score": ${DRIFT_RESOLUTION_SCORE},
      "weight": 30,
      "details": {
        "critical_issues": ${CRITICAL_ISSUES},
        "critical_resolved": ${RESOLVED_CRITICAL},
        "high_issues": ${HIGH_ISSUES},
        "high_resolved": ${RESOLVED_HIGH},
        "medium_issues": ${MEDIUM_ISSUES},
        "medium_resolved": ${RESOLVED_MEDIUM},
        "low_issues": ${LOW_ISSUES},
        "low_resolved": ${RESOLVED_LOW}
      }
    }
  },
  "gaps": [...],
  "status": "${OVERALL_COMPLETION >= 100 ? 'complete' : 'incomplete'}",
  "requires_completion": ${OVERALL_COMPLETION < 100 ? 'true' : 'false'}
}
```

**Write to output file:**
```bash
echo "${VALIDATION_JSON}" > "${OUTPUT_PATH}"
```

### Phase 9: Display Summary

**For user feedback:**

```
📊 Documentation Validation for ${ITEM_TITLE}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Overall Completion: ${OVERALL_COMPLETION}%

Content Metrics: ${CONTENT_SCORE}% (40% weight)
  ├─ Lines: ${LINE_COUNT} / ${MIN_LINES} ✓/✗
  ├─ Headings: ${HEADING_COUNT} / ${MIN_HEADINGS} ✓/✗
  ├─ Examples: ${CODE_BLOCK_COUNT} / ${MIN_EXAMPLES} ✓/✗
  └─ Sources: ${SOURCE_REF_COUNT} / ${MIN_SOURCES} ✓/✗

Source Validation: ${SOURCE_VALIDATION_SCORE}% (30% weight)
  ├─ Total References: ${SOURCE_REF_COUNT}
  └─ Missing Files: ${MISSING_FILE_COUNT}

Drift Resolution: ${DRIFT_RESOLUTION_SCORE}% (30% weight)
  ├─ Critical: ${RESOLVED_CRITICAL} / ${CRITICAL_ISSUES} ✓
  ├─ High: ${RESOLVED_HIGH} / ${HIGH_ISSUES} ✓
  ├─ Medium: ${RESOLVED_MEDIUM} / ${MEDIUM_ISSUES}
  └─ Low: ${RESOLVED_LOW} / ${LOW_ISSUES}

${OVERALL_COMPLETION >= 100 ? '✅ Documentation meets quality standards' : '⚠️  Gaps remaining - completion required'}
```

## Validation Rules

**Critical Gaps (must fix):**
- Content < 50% of minimum lines
- Zero source references
- Any invalid file references
- Critical or high severity drift issues unresolved

**High Priority Gaps:**
- Content < 80% of minimum
- Missing code examples
- Medium severity drift issues unresolved

**Medium Priority Gaps:**
- Content < 100% of minimum
- Insufficient headings
- Low severity drift issues unresolved

**Completion Threshold:**
- Default: 100% (all gaps resolved)
- Can be configured in workflow to accept 80-90% for difficult features

## Notes

- This command only validates, it does not modify files
- Validation results are written to JSON for programmatic use
- The completion command will use the gaps list to make targeted fixes
- Validation is language-agnostic (works with any codebase structure)
