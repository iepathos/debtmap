---
number: 28
title: Enhanced Markdown Output Formatting
category: optimization
priority: high
status: draft
dependencies: [19, 21, 24]
created: 2025-08-15
---

# Specification 28: Enhanced Markdown Output Formatting

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [19 - Unified Debt Prioritization, 21 - Dead Code Detection, 24 - Refined Risk Scoring]

## Context

The current markdown output formatting in debtmap is functional but lacks comprehensive coverage of the tool's advanced features. While the terminal output includes rich formatting for unified prioritization, risk analysis, and evidence-based recommendations, the markdown output only covers basic complexity and technical debt reporting.

Key gaps in the current markdown output:
- No unified priority scoring visualization
- Missing evidence-based risk analysis details
- No dead code detection results
- Lacks semantic function classification information
- No ROI-based testing recommendations
- Missing coverage propagation insights
- No call graph dependency information
- Lacks verbosity levels for detailed analysis

This limitation makes it difficult to generate comprehensive reports for documentation, CI/CD integration, or stakeholder review where markdown format is preferred over terminal output.

## Objective

Transform the markdown output formatter to provide feature parity with terminal output, including all advanced analysis features while maintaining clean, readable markdown structure suitable for rendering in documentation systems, GitHub, and other markdown-supporting platforms.

## Requirements

### Functional Requirements

1. **Unified Priority Integration**
   - Display unified priority scores with clear formatting
   - Include top N prioritized items based on CLI flags
   - Support --priorities-only, --detailed, --top, --tail modes
   - Show priority score breakdown when verbosity is enabled

2. **Evidence-Based Risk Reporting**
   - Include risk classification (Critical, High, Medium, Low, WellDesigned)
   - Display risk evidence factors with confidence scores
   - Show statistical baselines (P50, P90, P95, P99)
   - Include actionable remediation recommendations

3. **Dead Code Analysis**
   - Separate section for dead code findings
   - Visibility-based recommendations (private/public/crate)
   - Framework pattern exclusions noted
   - Usage hints based on complexity

4. **Semantic Classification**
   - Display function roles (PureLogic, Orchestrator, IOWrapper, EntryPoint)
   - Show role-based priority adjustments
   - Include coverage propagation information

5. **Call Graph Insights**
   - Dependency impact visualization
   - Module criticality scores
   - Cascade effect calculations
   - Entry point identification

6. **Testing Recommendations**
   - ROI-based test prioritization
   - Effort estimation tables
   - Module type classifications
   - Test coverage gaps with risk correlation

7. **Verbosity Support**
   - Progressive detail levels (-v, -vv, -vvv)
   - Collapsible sections using HTML details tags
   - Score calculation breakdowns at higher verbosity

8. **Visual Enhancements**
   - Use tables for structured data
   - Include progress bars using Unicode characters
   - Add emoji indicators for severity/priority
   - Use code blocks for file references

### Non-Functional Requirements

1. **Compatibility**
   - Valid CommonMark specification compliance
   - GitHub Flavored Markdown support
   - Proper escaping of special characters
   - Cross-platform emoji rendering

2. **Performance**
   - Minimal overhead compared to terminal output
   - Efficient string building without excessive allocations
   - Streaming output support for large reports

3. **Maintainability**
   - Modular formatting functions
   - Shared formatting utilities with terminal output
   - Clear separation of concerns

## Acceptance Criteria

- [ ] Markdown output includes unified priority scoring section
- [ ] Risk analysis section shows evidence-based assessments
- [ ] Dead code detection results are clearly formatted
- [ ] Semantic function classifications are displayed
- [ ] Call graph dependencies are visualized in tables
- [ ] Testing recommendations include ROI calculations
- [ ] Verbosity levels produce progressively detailed output
- [ ] Tables render correctly in GitHub and common markdown viewers
- [ ] Score breakdowns are shown in collapsible sections
- [ ] Output file size is reasonable for typical projects (<1MB)
- [ ] All existing markdown tests pass
- [ ] New tests cover enhanced formatting features
- [ ] Documentation updated with markdown output examples

## Technical Details

### Implementation Approach

1. **Extend MarkdownWriter**
   - Add methods for each new section type
   - Implement verbosity-aware formatting
   - Share formatting logic with terminal output where possible

2. **Section Structure**
   ```markdown
   # Debtmap Analysis Report
   
   ## Executive Summary
   [existing summary]
   
   ## Priority Technical Debt
   ### Top 10 Priority Items
   | Rank | Score | Function | Type | Issue |
   
   ### Score Breakdown (--verbose)
   <details>
   <summary>Click to expand</summary>
   [detailed calculations]
   </details>
   
   ## Risk Analysis
   ### Evidence-Based Assessment
   | Function | Risk Level | Confidence | Factors |
   
   ### Statistical Baselines
   | Metric | P50 | P90 | P95 | P99 |
   
   ## Dead Code Detection
   ### Unused Functions
   | Function | Visibility | Complexity | Recommendation |
   
   ## Testing Recommendations
   ### ROI-Based Priorities
   | Function | ROI | Effort | Risk Reduction | Coverage |
   
   ## Call Graph Analysis
   ### Module Dependencies
   [dependency matrix or tree]
   ```

3. **Formatting Utilities**
   - Table generation helpers
   - Progress bar rendering (e.g., `[████████░░] 80%`)
   - Score formatting with precision control
   - Emoji/icon mapping for cross-platform support

### Architecture Changes

1. **New Modules**
   - `src/io/writers/markdown_priority.rs` - Priority formatting
   - `src/io/writers/markdown_risk.rs` - Risk analysis formatting
   - `src/io/writers/markdown_tables.rs` - Table utilities
   - `src/io/writers/markdown_shared.rs` - Shared formatting

2. **Modified Components**
   - `MarkdownWriter` trait implementation extended
   - `OutputWriter` trait methods for new sections
   - Integration with `UnifiedAnalysis` data structures

### Data Structures

```rust
pub struct MarkdownSection {
    title: String,
    level: u8,  // Heading level (1-6)
    content: SectionContent,
    collapsible: bool,
    verbosity_required: u8,
}

pub enum SectionContent {
    Paragraph(String),
    Table(TableData),
    CodeBlock(String, String), // (language, content)
    List(Vec<ListItem>),
    Details(String, Box<SectionContent>), // (summary, content)
}

pub struct TableData {
    headers: Vec<String>,
    alignments: Vec<Alignment>,
    rows: Vec<Vec<String>>,
}
```

### APIs and Interfaces

```rust
impl OutputWriter for MarkdownWriter<W> {
    fn write_unified_priorities(&mut self, 
        analysis: &UnifiedAnalysis, 
        format: OutputFormat,
        verbosity: u8) -> Result<()>;
    
    fn write_evidence_risks(&mut self, 
        risks: &[EvidenceBasedRisk]) -> Result<()>;
    
    fn write_dead_code(&mut self, 
        dead_code: &[DeadCodeItem]) -> Result<()>;
    
    fn write_call_graph_insights(&mut self, 
        graph: &CallGraph) -> Result<()>;
}
```

## Dependencies

- **Prerequisites**: 
  - Spec 19: Unified prioritization data structures
  - Spec 21: Dead code detection results
  - Spec 24: Evidence-based risk analysis
  
- **Affected Components**:
  - `src/io/writers/markdown.rs`
  - `src/io/output.rs`
  - `src/main.rs` (output routing)
  
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**:
  - Each section formatter tested independently
  - Table generation with various data types
  - Verbosity level filtering
  - Special character escaping

- **Integration Tests**:
  - Full report generation with all sections
  - Comparison with terminal output data
  - File size and performance benchmarks
  - Cross-platform rendering validation

- **Performance Tests**:
  - Large project report generation time
  - Memory usage during formatting
  - Streaming vs buffered output comparison

- **User Acceptance**:
  - Render in GitHub README
  - Import into documentation systems
  - CI/CD report integration
  - Stakeholder readability review

## Documentation Requirements

- **Code Documentation**:
  - Formatting function documentation
  - Section structure explanations
  - Verbosity level descriptions

- **User Documentation**:
  - Markdown output format guide
  - Example reports with annotations
  - CI/CD integration examples
  - Comparison with other output formats

- **Architecture Updates**:
  - ARCHITECTURE.md updated with new modules
  - Data flow diagram for markdown generation

## Implementation Notes

1. **Progressive Enhancement**: Start with basic sections and add complexity incrementally
2. **Emoji Fallbacks**: Provide text alternatives for environments without emoji support
3. **Table Width**: Consider terminal width limitations when generating tables
4. **File Size**: Implement pagination or summary modes for very large reports
5. **Streaming**: Support streaming output for real-time report generation
6. **Caching**: Cache formatted sections that don't change between runs
7. **Templates**: Consider template engine for complex formatting (future enhancement)

## Migration and Compatibility

- **Breaking Changes**: None - existing markdown output structure preserved
- **New Flags**: Optional flags for controlling markdown verbosity and sections
- **Default Behavior**: Basic output remains unchanged unless new features explicitly requested
- **Version Detection**: Include debtmap version in report metadata for compatibility