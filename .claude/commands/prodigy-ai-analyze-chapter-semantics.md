# /prodigy-ai-analyze-chapter-semantics

Analyze a chapter's semantic structure to identify related sections and propose logical groupings.

## Variables

- `--chapter-file <path>` - Path to the chapter markdown file to analyze (required)
- `--output <path>` - Path to write semantic analysis JSON (default: "semantics.json")
- `--min-cluster-size <number>` - Minimum sections per cluster (default: 2)
- `--max-clusters <number>` - Maximum number of clusters to create (default: 8)

## Execute

### Phase 1: Understanding and Setup

You are analyzing a markdown chapter to identify semantic relationships between sections and propose intelligent groupings. This analysis will be used to organize large chapters into logical subsections.

**Extract Parameters:**

```bash
CHAPTER_FILE="${chapter_file:?Error: --chapter-file is required}"
OUTPUT="${output:-semantics.json}"
MIN_CLUSTER_SIZE="${min_cluster_size:-2}"
MAX_CLUSTERS="${max_clusters:-8}"
```

**Validate Input:**

```bash
if [ ! -f "$CHAPTER_FILE" ]; then
    echo "Error: Chapter file not found: $CHAPTER_FILE"
    exit 1
fi
```

### Phase 2: Parse Chapter Structure

Extract all H2 sections from the chapter:

1. **Read the chapter file** and identify all H2 headings (`## `)
2. **Extract section metadata:**
   - Section title
   - Line number
   - Content (all lines until next H2 or EOF)
   - Word count
   - Key topics/terms (frequently used words)

3. **Create section index:**

For each H2 section, extract:
- Title
- Content summary (first paragraph or ~100 words)
- Key terms (nouns, technical terms, proper nouns)
- Content length
- Nested H3/H4 headings

### Phase 3: Semantic Analysis

**Analyze semantic relationships** between sections:

#### Step 1: Topic Extraction

For each section:

1. **Identify key topics** from:
   - Section title
   - First paragraph
   - H3/H4 headings
   - Frequently mentioned terms
   - Code example patterns

2. **Categorize content type:**
   - Introduction/Overview
   - Configuration/Setup
   - Examples/Tutorials
   - Advanced Topics
   - Troubleshooting
   - Reference Material

#### Step 2: Similarity Analysis

Compare sections to find related content:

1. **Title similarity:**
   - Shared words in titles
   - Similar naming patterns
   - Hierarchical relationships (e.g., "Basic X" and "Advanced X")

2. **Topic overlap:**
   - Common key terms
   - Related concepts (e.g., "checkpoint" and "resume")
   - Complementary topics (e.g., "setup" and "configuration")

3. **Sequential relationships:**
   - Natural progression (e.g., "Quick Start" → "Complete Structure")
   - Dependency relationships (e.g., "Prerequisites" → "Installation")

#### Step 3: Cluster Sections

Group related sections using heuristic-based clustering:

**Algorithm:**

1. **Identify natural groups:**
   - Sections with shared topic words (>30% overlap)
   - Sections with similar content types
   - Sections that reference each other

2. **Apply grouping rules:**
   - Keep introduction/overview sections in index
   - Group configuration-related sections together
   - Group examples and tutorials together
   - Separate advanced topics from basics
   - Group troubleshooting and FAQ sections

3. **Balance cluster sizes:**
   - Avoid tiny clusters (<2 sections) - merge with related cluster
   - Avoid huge clusters (>5 sections) - consider splitting
   - Aim for 3-8 total clusters for most chapters

4. **Name clusters:**
   - Use representative title from grouped sections
   - Prefer standard names: "Getting Started", "Configuration", "Advanced Topics"
   - Ensure names reflect grouped content

### Phase 4: Generate Analysis Output

Create a JSON file with semantic analysis results:

**Output Structure:**

```json
{
  "chapter": "path/to/chapter.md",
  "total_sections": 15,
  "analysis_date": "2025-01-11T10:30:00Z",
  "semantic_clusters": [
    {
      "name": "Getting Started",
      "sections": [
        {"title": "Quick Start", "line": 10, "word_count": 250},
        {"title": "Complete Structure", "line": 45, "word_count": 180}
      ],
      "topics": ["basic usage", "workflow syntax", "first steps"],
      "rationale": "Both sections introduce users to basic concepts and syntax",
      "confidence_score": 0.85,
      "cluster_type": "introduction"
    },
    {
      "name": "Configuration",
      "sections": [
        {"title": "Environment Variables", "line": 120, "word_count": 340},
        {"title": "Backoff Strategies", "line": 180, "word_count": 220},
        {"title": "Error Collection", "line": 240, "word_count": 190}
      ],
      "topics": ["configuration", "settings", "customization", "environment"],
      "rationale": "All sections deal with configuring MapReduce behavior",
      "confidence_score": 0.92,
      "cluster_type": "configuration"
    },
    {
      "name": "State Management",
      "sections": [
        {"title": "Checkpoint and Resume", "line": 310, "word_count": 450},
        {"title": "Dead Letter Queue", "line": 420, "word_count": 380}
      ],
      "topics": ["runtime", "state", "recovery", "failure handling", "checkpoint"],
      "rationale": "Sections cover runtime behavior and state management",
      "confidence_score": 0.88,
      "cluster_type": "advanced"
    }
  ],
  "unclustered_sections": [],
  "recommendations": [
    "Keep 'Quick Start' in index.md for immediate access",
    "Consider merging 'Backoff Strategies' and 'Error Collection' if both are short",
    "Add cross-references between 'Checkpoint' and 'Dead Letter Queue'"
  ]
}
```

**Write output:**

```bash
cat > "$OUTPUT" << 'EOF'
{JSON content here}
EOF

echo "✓ Semantic analysis complete: $OUTPUT"
```

### Phase 5: Provide Summary

Print a human-readable summary:

```
Semantic Analysis Summary
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Chapter: mapreduce.md
Total Sections: 15 H2 sections
Analysis Output: semantics.json

Proposed Clusters (5):

1. Getting Started (2 sections)
   • Quick Start
   • Complete Structure
   Topics: basic usage, workflow syntax
   Type: introduction

2. Configuration (3 sections)
   • Environment Variables
   • Backoff Strategies
   • Error Collection
   Topics: configuration, settings, customization
   Type: configuration

3. State Management (2 sections)
   • Checkpoint and Resume
   • Dead Letter Queue
   Topics: runtime, state, recovery
   Type: advanced

4. Performance (1 section)
   • Performance Tuning
   Topics: optimization, tuning
   Type: advanced

5. Examples (2 sections)
   • Real-World Use Cases
   • Troubleshooting
   Topics: examples, debugging
   Type: examples

Recommendations:
  • Keep 'Quick Start' in index.md
  • Consider merging small sections in Configuration
  • Add navigation links between State Management sections

Next Steps:
  • Review proposed clusters in semantics.json
  • Use /prodigy-ai-organize-chapter to apply organization
  • Or manually adjust clusters before applying
```

### Quality Guidelines

1. **Accurate Clustering:**
   - Sections in same cluster should be thematically related
   - Avoid forcing unrelated sections together
   - Respect natural boundaries in content

2. **Balanced Grouping:**
   - Aim for 3-8 clusters per chapter
   - Each cluster should have 2-5 sections
   - Single-section clusters are OK for important standalone topics

3. **Clear Naming:**
   - Cluster names should reflect content
   - Use standard terminology when possible
   - Avoid vague names like "Miscellaneous" or "Other"

4. **Confidence Scoring:**
   - High confidence (>0.8): Clear thematic grouping
   - Medium confidence (0.5-0.8): Reasonable grouping, could vary
   - Low confidence (<0.5): Weak relationship, consider splitting

### Success Criteria

- [ ] All H2 sections identified and extracted
- [ ] Semantic relationships analyzed using topic overlap
- [ ] Sections clustered into logical groups (3-8 clusters)
- [ ] Each cluster has clear rationale and topic list
- [ ] Output JSON written to specified path
- [ ] Summary printed showing proposed organization
- [ ] Recommendations provided for improving organization

### Error Handling

**Chapter file not found:**
```
Error: Chapter file not found: book/src/nonexistent.md
Please verify the path and try again.
```

**No H2 sections found:**
```
Warning: No H2 sections found in chapter
This chapter may not benefit from subsection organization.
Consider adding section headings or skipping organization.
```

**Too few sections:**
```
Info: Only 3 H2 sections found
Chapter may not need subsection organization (threshold: 6+ sections)
Exiting without creating clusters.
```
