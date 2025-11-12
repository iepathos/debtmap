# /prodigy-analyze-book-consistency

Analyze consistency of chapter organization across multiple project documentation books.

## Variables

- `--books <project:path,project:path>` - Comma-separated list of project:book-dir pairs (required)
- `--output <path>` - Path to write consistency report JSON (default: "consistency-report.json")
- `--min-compliance <percentage>` - Minimum compliance threshold (default: 80)

## Execute

### Phase 1: Understanding and Setup

You are analyzing multiple mdBook documentation projects to ensure consistent chapter organization across projects. This helps maintain professional, predictable documentation structure.

**Extract Parameters:**

```bash
BOOKS="${books:?Error: --books is required}"
OUTPUT="${output:-consistency-report.json}"
MIN_COMPLIANCE="${min_compliance:-80}"
```

**Parse book list:**

```bash
# Split comma-separated books into array
IFS=',' read -ra BOOK_LIST <<< "$BOOKS"

# Validate each book
for BOOK_ENTRY in "${BOOK_LIST[@]}"; do
    PROJECT=$(echo "$BOOK_ENTRY" | cut -d: -f1)
    BOOK_DIR=$(echo "$BOOK_ENTRY" | cut -d: -f2)

    if [ ! -d "$BOOK_DIR" ]; then
        echo "Error: Book directory not found: $BOOK_DIR (project: $PROJECT)"
        exit 1
    fi

    if [ ! -f "$BOOK_DIR/src/SUMMARY.md" ]; then
        echo "Error: SUMMARY.md not found in $BOOK_DIR/src (project: $PROJECT)"
        exit 1
    fi
done
```

### Phase 2: Load All Book Structures

For each project, extract chapter organization:

#### Step 1: Parse SUMMARY.md

Read each book's SUMMARY.md and extract:

1. **Chapter list:**
   - Chapter title
   - Chapter file path
   - Subsections (if nested structure exists)

2. **Chapter metadata:**
   ```json
   {
     "project": "prodigy",
     "chapter": "MapReduce Workflows",
     "file": "mapreduce/index.md",
     "has_subsections": true,
     "subsections": [
       "Getting Started",
       "Configuration",
       "State Management"
     ]
   }
   ```

#### Step 2: Detect Chapter Type

For each chapter, infer type:

1. **Read chapter file** (index.md or single file)
2. **Classify based on:**
   - Title keywords (e.g., "Quick Start", "Configuration", "CLI")
   - Content structure (H2 headings, topics)
   - Template matching (using same logic as /prodigy-ai-organize-chapter)

3. **Store detected type:**
   ```json
   {
     "chapter": "MapReduce Workflows",
     "detected_type": "advanced-features",
     "confidence": 0.85
   }
   ```

### Phase 3: Group Chapters by Type

Organize chapters by detected type across all projects:

```json
{
  "chapter_type": "quick-start",
  "chapters": [
    {
      "project": "prodigy",
      "chapter": "Getting Started",
      "subsections": ["Overview", "Installation", "First Example"]
    },
    {
      "project": "debtmap",
      "chapter": "Quick Start",
      "subsections": ["Introduction", "Setup", "Basic Usage"]
    }
  ]
}
```

### Phase 4: Analyze Consistency

For each chapter type, compare organizations:

#### Step 1: Find Common Pattern

Identify the most common structure:

1. **Subsection name analysis:**
   - Extract all subsection names for this chapter type
   - Find most frequently used names
   - Identify naming variations (e.g., "Quick Start" vs "Getting Started")

2. **Common pattern:**
   ```json
   {
     "chapter_type": "quick-start",
     "common_subsections": [
       {"name": "Overview", "frequency": 0.80},
       {"name": "Installation", "frequency": 0.60},
       {"name": "First Example", "frequency": 1.00}
     ],
     "typical_count": 3
   }
   ```

#### Step 2: Identify Deviations

Compare each chapter to the common pattern:

1. **Naming deviations:**
   - Different names for same concept
   - Example: "Getting Started" instead of "Quick Start"

2. **Missing subsections:**
   - Common subsections not present in this chapter
   - Example: Missing "Installation" subsection

3. **Extra subsections:**
   - Subsections present but not in common pattern
   - May indicate project-specific content

4. **Ordering differences:**
   - Same subsections but different order
   - Example: "Installation" before "Overview"

**Deviation record:**

```json
{
  "project": "debtmap",
  "chapter": "Quick Start",
  "deviations": [
    {
      "type": "naming_mismatch",
      "expected": "First Example",
      "actual": "Basic Usage",
      "severity": "medium",
      "suggestion": "Rename 'Basic Usage' to 'First Example' for consistency"
    },
    {
      "type": "missing_subsection",
      "expected": "Next Steps",
      "severity": "low",
      "suggestion": "Add 'Next Steps' subsection with links to advanced topics"
    },
    {
      "type": "ordering_difference",
      "expected_order": ["Overview", "Installation", "First Example"],
      "actual_order": ["Introduction", "Basic Usage", "Setup"],
      "severity": "medium",
      "suggestion": "Reorder subsections to match standard pattern"
    }
  ]
}
```

#### Step 3: Calculate Consistency Score

For each chapter:

```
consistency_score = (
    (matching_names / total_subsections) * 0.4 +
    (correct_order_score) * 0.3 +
    (has_required_subsections) * 0.3
)
```

For each chapter type:

```
type_consistency = average(all_chapters_of_this_type.consistency_score)
```

Overall:

```
overall_consistency = average(all_chapter_types.type_consistency)
```

### Phase 5: Generate Recommendations

Based on deviations, create actionable recommendations:

#### Step 1: Naming Standardization

Identify naming variations and suggest standard:

```json
{
  "recommendation_type": "naming_standardization",
  "variations_found": ["Quick Start", "Getting Started", "Quickstart"],
  "preferred": "Quick Start",
  "affects": [
    {"project": "debtmap", "chapter": "Getting Started"},
    {"project": "ripgrep", "chapter": "Quickstart"}
  ],
  "action": "Rename to 'Quick Start' across all projects",
  "priority": "high"
}
```

#### Step 2: Missing Subsections

Suggest adding missing common subsections:

```json
{
  "recommendation_type": "add_subsection",
  "subsection": "Next Steps",
  "chapter_type": "quick-start",
  "affects": [
    {"project": "debtmap", "chapter": "Quick Start"}
  ],
  "action": "Add 'Next Steps' subsection with links to advanced content",
  "priority": "medium"
}
```

#### Step 3: Reordering

Suggest reordering subsections to match common pattern:

```json
{
  "recommendation_type": "reorder_subsections",
  "chapter_type": "configuration-reference",
  "standard_order": ["Overview", "Options", "Environment Variables", "Examples"],
  "affects": [
    {
      "project": "prodigy",
      "chapter": "Configuration",
      "current_order": ["Options", "Overview", "Environment Variables", "Examples"]
    }
  ],
  "action": "Reorder to match standard pattern",
  "priority": "low"
}
```

### Phase 6: Generate Consistency Report

Create comprehensive JSON report:

```json
{
  "analysis_date": "2025-01-11T10:30:00Z",
  "projects_analyzed": ["prodigy", "debtmap", "ripgrep"],
  "total_chapters": 45,
  "overall_consistency_score": 0.78,
  "compliance_threshold": 80,
  "passes_compliance": false,

  "chapter_type_analysis": [
    {
      "chapter_type": "quick-start",
      "template": "quick-start",
      "chapters_analyzed": 3,
      "consistency_score": 0.85,
      "common_pattern": {
        "subsections": ["Overview", "Installation", "First Example", "Next Steps"],
        "typical_count": 4
      },
      "deviations": [
        {
          "project": "debtmap",
          "chapter": "Quick Start",
          "deviations": [
            {
              "type": "naming_mismatch",
              "expected": "First Example",
              "actual": "Basic Usage",
              "suggestion": "Rename for consistency"
            }
          ],
          "consistency_score": 0.75
        }
      ]
    },
    {
      "chapter_type": "configuration-reference",
      "template": "configuration-reference",
      "chapters_analyzed": 3,
      "consistency_score": 0.92,
      "common_pattern": {
        "subsections": ["Overview", "Options", "Environment Variables", "Examples"],
        "typical_count": 4
      },
      "deviations": []
    }
  ],

  "recommendations": [
    {
      "priority": "high",
      "type": "naming_standardization",
      "description": "Standardize 'Getting Started' to 'Quick Start' across all projects",
      "affects": ["debtmap", "ripgrep"],
      "estimated_effort": "low"
    },
    {
      "priority": "medium",
      "type": "add_subsection",
      "description": "Add 'Next Steps' subsection to Quick Start chapters",
      "affects": ["debtmap"],
      "estimated_effort": "medium"
    },
    {
      "priority": "low",
      "type": "reorder_subsections",
      "description": "Reorder Configuration chapter subsections",
      "affects": ["prodigy"],
      "estimated_effort": "low"
    }
  ],

  "summary": {
    "high_priority_issues": 1,
    "medium_priority_issues": 1,
    "low_priority_issues": 1,
    "chapters_need_attention": 2,
    "chapters_compliant": 43
  }
}
```

**Write report:**

```bash
cat > "$OUTPUT" << 'EOF'
{JSON report content}
EOF

echo "✓ Consistency report saved: $OUTPUT"
```

### Phase 7: Print Human-Readable Summary

```
Cross-Project Documentation Consistency Report
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Projects Analyzed: prodigy, debtmap, ripgrep
Total Chapters: 45
Overall Consistency: 78% (threshold: 80%)
Status: ⚠ Below Compliance Threshold

Chapter Type Analysis:

1. Quick Start Chapters
   Consistency: 85%
   Chapters: 3
   Common Pattern:
     • Overview
     • Installation
     • First Example
     • Next Steps

   Deviations:
     • debtmap: Uses "Basic Usage" instead of "First Example"
     • ripgrep: Missing "Next Steps" subsection

2. Configuration Reference
   Consistency: 92% ✓
   Chapters: 3
   Common Pattern:
     • Overview
     • Options
     • Environment Variables
     • Examples

   No deviations - well aligned!

3. Advanced Features
   Consistency: 68% ⚠
   Chapters: 4
   Issues:
     • Inconsistent subsection naming
     • Different ordering patterns
     • Missing standard sections

Recommendations (High Priority):

1. Naming Standardization
   Standardize "Getting Started" → "Quick Start" across projects
   Affects: debtmap, ripgrep
   Effort: Low

2. Add Missing Subsections
   Add "Next Steps" to Quick Start chapters
   Affects: debtmap
   Effort: Medium

3. Reorder Subsections
   Align subsection ordering with templates
   Affects: prodigy Configuration chapter
   Effort: Low

Summary:
  • 2 chapters need attention
  • 43 chapters compliant
  • 1 high priority issue
  • 1 medium priority issue
  • 1 low priority issue

Next Steps:
  • Review full report: consistency-report.json
  • Apply recommendations to improve consistency
  • Re-run analysis to verify improvements
```

### Quality Guidelines

1. **Fair Comparison:**
   - Only compare chapters of same type
   - Account for project-specific variations
   - Don't penalize legitimate differences

2. **Actionable Recommendations:**
   - Clear, specific actions
   - Prioritized by impact
   - Effort estimation included

3. **Objective Scoring:**
   - Consistent scoring methodology
   - Reproducible results
   - Clear threshold for compliance

### Success Criteria

- [ ] All specified books loaded and parsed
- [ ] Chapters classified by type
- [ ] Common patterns identified for each type
- [ ] Deviations detected and documented
- [ ] Consistency scores calculated
- [ ] Recommendations generated with priorities
- [ ] Report saved to JSON file
- [ ] Human-readable summary printed

### Error Handling

**Book directory not found:**
```
Error: Book directory not found: ../debtmap/book (project: debtmap)
Please verify all book paths are correct.
```

**No chapters of type found:**
```
Warning: No chapters found for type 'api-documentation'
This chapter type will be skipped in consistency analysis.
```

**Low overall consistency:**
```
⚠ Overall consistency: 65% (threshold: 80%)

Your documentation organization is below the compliance threshold.
Consider:
  • Applying high-priority recommendations
  • Using AI organization for new chapters
  • Reviewing template alignment
```
