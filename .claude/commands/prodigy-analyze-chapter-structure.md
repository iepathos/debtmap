# /prodigy-analyze-chapter-structure

Analyze existing book chapters for size and structural complexity. This command identifies oversized chapters that should be split into subsections and provides recommendations for better content organization.

## Purpose

This command focuses on **content organization** - ensuring chapters are appropriately sized and structured. It does NOT create subsections automatically (see `/prodigy-create-chapter-subsections` for that).

## Variables

- `--project <name>` - Project name (e.g., "Debtmap")
- `--book-dir <path>` - Book directory path (e.g., "book")
- `--chapters <path>` - Path to chapter definitions JSON (e.g., "workflows/data/prodigy-chapters.json")
- `--output <path>` - Path to write structure analysis report (e.g., ".prodigy/book-analysis/structure-report.json")
- `--size-threshold <number>` - Line count threshold for oversized chapters (default: 500)
- `--section-threshold <number>` - Minimum lines for a section to be subsection-worthy (default: 100)

## Execute

### Phase 1: Parse Parameters and Load Data

**Parse Command Arguments:**
Extract all required parameters:
- `--project`: Project name for output messages
- `--book-dir`: Book directory path (default: "book")
- `--chapters`: Path to chapter definitions JSON
- `--output`: Path for structure report JSON
- `--size-threshold`: Maximum ideal chapter size in lines (default: 500)
- `--section-threshold`: Minimum section size for subsection candidate (default: 100)

**Load Configuration Files:**
1. Read `--chapters` file to get all chapter definitions
2. Verify book directory exists at `--book-dir`

### Phase 2: Analyze Each Chapter for Size and Structure

**For Each Chapter in chapters.json:**

**Step 1: Determine Chapter Type and Files**

1. Check chapter `type` field:
   - If `type == "multi-subsection"`: Skip (already organized, analyze subsections instead)
   - If `type == "single-file"` or no type: Analyze as single file

2. Get chapter file path and verify it exists

**Step 2: Analyze Single-File Chapters**

For each single-file chapter:

1. **Count Total Lines:**
   - Read the file and count all lines (including empty lines)
   - Count content lines (excluding empty lines and code blocks)
   - Record total and content line counts

2. **Extract Section Structure:**
   - Parse markdown to find all H2 sections (## headers)
   - For each H2 section:
     - Extract section title
     - Count lines in section (from this H2 to next H2 or end of file)
     - Count content lines in section
     - Identify if section has substantial content (>100 lines)
   - Record section metadata: title, line_count, content_lines, start_line, end_line

3. **Analyze Code Block Density:**
   - Count total code block lines
   - Calculate ratio: code_lines / total_lines
   - High ratio (>0.4) suggests examples-heavy content

4. **Analyze Heading Hierarchy:**
   - Count H1, H2, H3, H4 headings
   - Check for proper hierarchy (no skipped levels)
   - Identify overly deep nesting (H4+)

**Step 3: Classify Chapter Complexity**

Based on analysis, classify each chapter:

**Oversized - Needs Subsections (HIGH priority):**
- Total lines > size_threshold (default 500)
- Has 3+ substantial H2 sections (each >100 lines)
- Each section is cohesive enough to stand alone
- Example: Configuration chapter with 1843 lines and 10+ H2 sections

**Large - Consider Subsections (MEDIUM priority):**
- Total lines > (size_threshold * 0.7) (default 350)
- Has 4+ H2 sections
- Some sections substantial enough for subsections
- Example: Error Handling Analysis with 600 lines and 5 H2 sections

**Well-Sized - No Action Needed (LOW priority):**
- Total lines < size_threshold
- Logical section structure
- No subsection candidates
- Example: Getting Started with 200 lines

**Too Small - May Need Consolidation (INFO):**
- Total lines < 100
- Very few sections
- Consider merging with related chapter
- Example: Threshold Configuration with 50 lines

### Phase 3: Generate Subsection Recommendations

**For Each Oversized or Large Chapter:**

**Analyze Subsection Candidates:**

1. **Identify Section Groups:**
   - Look for related H2 sections that form logical groups
   - Example: In Configuration chapter, group all "role_*" sections together

2. **Evaluate Each H2 Section as Subsection Candidate:**
   - Section has >100 lines of content
   - Section has clear, focused topic
   - Section is not meta-content (avoid "Best Practices", "Troubleshooting" as subsections)
   - Section has enough depth to warrant separate file

3. **Generate Subsection Recommendations:**
   For each viable subsection candidate:
   - Proposed subsection ID (kebab-case from H2 title)
   - Proposed subsection title
   - Proposed file path (e.g., `book/src/configuration/role-based-scoring.md`)
   - Line count and content summary
   - Reason for recommendation

4. **Calculate Organization Improvement:**
   - Current: 1 file with N lines
   - Proposed: 1 index.md + K subsection files
   - Average subsection size
   - Improved navigability score

### Phase 4: Analyze Multi-Subsection Chapters

**For Chapters Already Using Subsections:**

1. **Verify Subsection Balance:**
   - Read each subsection file
   - Count lines per subsection
   - Identify imbalanced subsections (one huge, others tiny)

2. **Check Index.md Quality:**
   - Ensure index.md exists and is not empty
   - Check if index.md has proper overview
   - Verify index.md links to all subsections

3. **Identify Oversized Subsections:**
   - Flag subsections >300 lines (may need further splitting)
   - Recommend breaking into sub-subsections if needed

### Phase 5: Generate Structure Report

**Create Comprehensive JSON Report:**

```json
{
  "analysis_date": "<timestamp>",
  "project": "<project-name>",
  "chapters_analyzed": <count>,
  "thresholds": {
    "size_threshold": <number>,
    "section_threshold": <number>
  },
  "summary": {
    "oversized_chapters": <count>,
    "large_chapters": <count>,
    "well_sized_chapters": <count>,
    "multi_subsection_chapters": <count>
  },
  "recommendations": [
    {
      "chapter_id": "<chapter-id>",
      "chapter_title": "<title>",
      "current_file": "<path>",
      "priority": "high|medium|low",
      "issue": "oversized|large|imbalanced_subsections",
      "metrics": {
        "total_lines": <number>,
        "content_lines": <number>,
        "h2_sections": <count>,
        "substantial_sections": <count>,
        "code_block_ratio": <float>
      },
      "recommended_action": "split_into_subsections|rebalance_subsections|no_action",
      "proposed_structure": {
        "type": "multi-subsection",
        "index_file": "book/src/<chapter-id>/index.md",
        "subsections": [
          {
            "id": "<subsection-id>",
            "title": "<title>",
            "file": "book/src/<chapter-id>/<subsection-id>.md",
            "estimated_lines": <number>,
            "source_sections": ["<H2-title>", ...],
            "reason": "Substantial content focused on specific topic"
          }
        ]
      }
    }
  ],
  "well_organized_chapters": [
    {
      "chapter_id": "<id>",
      "metrics": {
        "total_lines": <number>,
        "h2_sections": <count>
      }
    }
  ]
}
```

**Write Report to Disk:**
- Save to `--output` path
- Use 2-space indentation
- Ensure valid JSON

### Phase 6: Display User-Friendly Summary

**Print Formatted Output:**

```
📏 Chapter Structure Analysis
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Chapters Analyzed: {total}
Size Threshold: {threshold} lines

📊 Summary:
  🔴 Oversized (need subsections): {count}
  🟡 Large (consider subsections): {count}
  ✅ Well-sized: {count}
  📁 Already using subsections: {count}

🔴 HIGH PRIORITY - Split into Subsections:

  • Configuration (1843 lines, 12 sections)
    Recommended: 8 subsections
    - Basic Configuration (200 lines)
    - Role-Based Scoring (250 lines)
    - Orchestration Config (180 lines)
    - Advanced Settings (150 lines)
    ...

  • Error Handling Analysis (600 lines, 5 sections)
    Recommended: 3 subsections
    - Panic Patterns (220 lines)
    - Error Swallowing (200 lines)
    - Error Propagation (180 lines)

🟡 MEDIUM PRIORITY - Consider Subsections:

  • Parallel Processing (450 lines, 4 sections)
    May benefit from subsections if content grows

✅ Well-Organized Chapters: {count}

📝 Next Steps:
  1. Review recommendations in {output-path}
  2. For each oversized chapter, run:
     /prodigy-create-chapter-subsections --chapter <chapter-id>
  3. Update SUMMARY.md to reflect new structure
```

### Phase 7: Validation and Quality Checks

**Verify Analysis Accuracy:**
- Ensure line counts are accurate
- Verify H2 extraction captured all sections
- Check that recommendations make sense

**Validate Recommendations:**
- No recommendations for already well-structured chapters
- Subsection proposals are logical and cohesive
- File paths follow project conventions

**Test Report Generation:**
- Ensure JSON is valid
- Verify all required fields are present
- Check that report is readable

### Error Handling

**Handle Missing Files:**
- If chapter file doesn't exist, log warning and skip
- If book directory missing, error and exit
- If chapters.json missing, error and exit

**Handle Malformed Markdown:**
- If markdown parsing fails, log error but continue
- Use line-based fallback if structure parser fails
- Record parsing errors in report

**Handle Invalid Parameters:**
- Validate threshold values are positive integers
- Ensure output path is writable
- Check book directory exists

### Quality Guidelines

**Accuracy:**
- Precise line counting (handle edge cases like code blocks)
- Accurate section extraction (proper H2 parsing)
- Smart recommendations (avoid false positives)

**Actionability:**
- Clear priorities (high/medium/low)
- Specific recommendations (exact subsection proposals)
- Easy to act on (clear next steps)

**Performance:**
- Analyze typical project (<50 chapters) in <10 seconds
- Efficient markdown parsing
- Minimal memory usage

## Configuration Defaults

If parameters not provided, use these defaults:

```json
{
  "size_threshold": 500,
  "section_threshold": 100,
  "code_block_ratio_threshold": 0.4,
  "min_subsections": 3,
  "max_subsections": 12
}
```

## Success Indicators

Analysis is successful when:
- All chapters analyzed without errors
- Recommendations are accurate and actionable
- Report is valid JSON and readable
- High-priority issues are clearly identified
- User understands next steps

## Scope Notes

This command is **analysis-only**. It does NOT:
- ❌ Modify any chapter files
- ❌ Create subsection files
- ❌ Update SUMMARY.md
- ❌ Update chapters.json

It only generates recommendations. Use `/prodigy-create-chapter-subsections` to act on recommendations.

## Example Usage

```bash
# Analyze with defaults
/prodigy-analyze-chapter-structure \
  --project Debtmap \
  --book-dir book \
  --chapters workflows/data/prodigy-chapters.json \
  --output .prodigy/book-analysis/structure-report.json

# Analyze with custom thresholds
/prodigy-analyze-chapter-structure \
  --project Debtmap \
  --book-dir book \
  --chapters workflows/data/prodigy-chapters.json \
  --output .prodigy/book-analysis/structure-report.json \
  --size-threshold 400 \
  --section-threshold 80
```

## Integration with Workflow

This command should run AFTER drift detection but BEFORE creating subsections:

1. `/prodigy-analyze-features-for-book` - Analyze codebase
2. `/prodigy-detect-documentation-gaps` - Check feature coverage
3. **`/prodigy-analyze-chapter-structure`** - Check chapter sizes ← THIS COMMAND
4. Review recommendations, then run `/prodigy-create-chapter-subsections` for oversized chapters
5. Run drift detection on new structure
