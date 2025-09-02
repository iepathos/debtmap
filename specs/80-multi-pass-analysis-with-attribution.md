---
number: 80
title: Multi-Pass Analysis with Attribution
category: optimization
priority: medium
status: draft
dependencies: [79]
created: 2025-09-02
---

# Specification 80: Multi-Pass Analysis with Attribution

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: [79] Semantic Normalization for Complexity Calculation

## Context

While semantic normalization (specification 79) eliminates formatting-induced false positives in complexity calculations, developers still need visibility into complexity attribution and sources of change. The current system provides a single complexity score without breaking down the contributing factors or explaining differences between analysis runs.

This creates challenges for developers trying to understand:
- Why complexity scores changed after refactoring
- Which specific code patterns contribute most to complexity
- Whether complexity increases are due to logical changes or measurement artifacts
- How different analysis approaches (raw vs normalized) compare

A multi-pass analysis system that compares raw complexity against semantically normalized complexity provides valuable diagnostic information, helps validate normalization effectiveness, and gives developers insight into the sources and nature of their code's complexity.

## Objective

Implement a multi-pass complexity analysis system that calculates both raw and semantically normalized complexity, provides detailed attribution of complexity sources, and offers comprehensive diagnostics to help developers understand and improve their code's complexity characteristics.

## Requirements

### Functional Requirements

- **Dual-Pass Calculation**: Calculate both raw AST complexity and semantically normalized complexity
- **Attribution Analysis**: Break down complexity by source type (logical, formatting, pattern-based)
- **Change Detection**: Identify and categorize differences between analysis runs
- **Source Mapping**: Map complexity contributions back to specific code locations
- **Comparative Analysis**: Provide before/after complexity comparisons with detailed explanations
- **Diagnostic Reporting**: Generate comprehensive reports explaining complexity decisions and transformations

### Non-Functional Requirements

- **Performance Impact**: Multi-pass analysis overhead must not exceed 25% of single-pass analysis time
- **Memory Efficiency**: Attribution data structures should not double memory usage
- **Incremental Analysis**: Support incremental updates when only portions of code change
- **Configurable Detail**: Allow users to control the level of diagnostic detail
- **Integration Compatibility**: Work seamlessly with existing analysis pipeline and output formats

## Acceptance Criteria

- [ ] **Dual Complexity Calculation**: Both raw and normalized complexity calculated for each analysis unit
- [ ] **Attribution Breakdown**: Complexity attributed to logical structure, formatting artifacts, and pattern recognition
- [ ] **Source Location Mapping**: Each complexity point traceable to specific code location with line/column information
- [ ] **Change Attribution**: Complexity changes between runs attributed to specific modifications (logical vs formatting)
- [ ] **Diagnostic Output Formats**: Support JSON, YAML, and markdown diagnostic reports
- [ ] **Performance Benchmark**: Multi-pass analysis completes within 125% of single-pass analysis time
- [ ] **Memory Usage Control**: Total memory usage increase limited to 50% over single-pass analysis
- [ ] **Integration Testing**: Works correctly with existing risk analysis, entropy calculation, and pattern recognition
- [ ] **User Interface**: CLI flags and configuration options for controlling analysis depth and output detail

## Technical Details

### Implementation Approach

**Phase 1: Multi-Pass Analysis Engine**
```rust
// New module: src/analysis/multi_pass.rs
pub struct MultiPassAnalyzer {
    raw_analyzer: ComplexityAnalyzer,
    normalized_analyzer: ComplexityAnalyzer,
    attribution_engine: AttributionEngine,
}

impl MultiPassAnalyzer {
    pub fn analyze(&self, source: &AnalysisUnit) -> MultiPassResult {
        let raw_result = self.raw_analyzer.analyze(&source.raw_ast);
        let normalized_result = self.normalized_analyzer.analyze(&source.normalized_ast);
        let attribution = self.attribution_engine.attribute(&raw_result, &normalized_result);
        
        MultiPassResult {
            raw_complexity: raw_result,
            normalized_complexity: normalized_result,
            attribution: attribution,
            insights: self.generate_insights(&attribution),
        }
    }
}
```

**Phase 2: Attribution System**
```rust
// Attribution engine for complexity source analysis
pub struct AttributionEngine {
    source_trackers: Vec<Box<dyn SourceTracker>>,
    pattern_analyzers: Vec<Box<dyn PatternAnalyzer>>,
}

pub trait SourceTracker {
    fn track_complexity_source(&self, ast_node: &ASTNode) -> Vec<ComplexityAttribution>;
}

pub trait PatternAnalyzer {  
    fn analyze_pattern_impact(&self, pattern: &DetectedPattern) -> PatternImpactAnalysis;
}
```

**Phase 3: Diagnostic Reporting**
```rust
// Comprehensive diagnostic and reporting system
pub struct DiagnosticReporter {
    output_format: OutputFormat,
    detail_level: DetailLevel,
}

impl DiagnosticReporter {
    pub fn generate_report(&self, result: &MultiPassResult) -> DiagnosticReport {
        DiagnosticReport {
            summary: self.generate_summary(result),
            detailed_attribution: self.generate_attribution_details(result),
            recommendations: self.generate_recommendations(result),
            comparative_analysis: self.generate_comparisons(result),
        }
    }
}
```

### Architecture Changes

**New Components:**
```
src/analysis/
├── multi_pass.rs              # Core multi-pass analysis engine
├── attribution/
│   ├── mod.rs                 # Attribution system coordination
│   ├── source_tracker.rs      # Complexity source tracking
│   ├── pattern_tracker.rs     # Pattern-based attribution
│   └── change_tracker.rs      # Change attribution between runs
├── diagnostics/
│   ├── mod.rs                 # Diagnostic system coordination
│   ├── reporter.rs            # Report generation
│   ├── insights.rs            # Insight generation engine
│   └── recommendations.rs     # Recommendation system
└── comparison/
    ├── mod.rs                 # Analysis comparison utilities
    ├── diff_analyzer.rs       # Code change analysis
    └── impact_calculator.rs   # Change impact calculation
```

**Modified Components:**
- `src/core/metrics.rs`: Extended to support multi-pass metrics
- `src/io/output.rs`: Enhanced with diagnostic output formats
- `src/commands/analyze.rs`: Add multi-pass analysis CLI options
- `src/risk/strategy.rs`: Integration with attribution data

### Data Structures

**Core Multi-Pass Types:**
```rust
#[derive(Debug, Clone)]
pub struct MultiPassResult {
    pub raw_complexity: ComplexityResult,
    pub normalized_complexity: ComplexityResult,  
    pub attribution: ComplexityAttribution,
    pub insights: Vec<ComplexityInsight>,
    pub recommendations: Vec<ComplexityRecommendation>,
}

#[derive(Debug, Clone)]
pub struct ComplexityAttribution {
    pub logical_complexity: AttributedComplexity,
    pub formatting_artifacts: AttributedComplexity,
    pub pattern_complexity: AttributedComplexity,
    pub source_mappings: Vec<SourceMapping>,
}

#[derive(Debug, Clone)]
pub struct AttributedComplexity {
    pub total: u32,
    pub breakdown: Vec<ComplexityComponent>,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct ComplexityComponent {
    pub source_type: ComplexitySourceType,
    pub contribution: u32,
    pub location: CodeLocation,
    pub description: String,
    pub suggestions: Vec<String>,
}
```

**Attribution-Specific Types:**
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ComplexitySourceType {
    LogicalStructure {
        construct_type: LogicalConstruct,
        nesting_level: u32,
    },
    FormattingArtifact {
        artifact_type: FormattingArtifact,
        severity: ArtifactSeverity,
    },
    PatternRecognition {
        pattern_type: RecognizedPattern,
        adjustment_factor: f32,
    },
    LanguageSpecific {
        language: Language,
        feature: LanguageFeature,
    },
}

#[derive(Debug, Clone)]
pub struct SourceMapping {
    pub complexity_point: u32,
    pub location: CodeLocation,
    pub ast_path: Vec<String>,
    pub context: String,
}

#[derive(Debug, Clone)]
pub struct CodeLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub span: Option<(u32, u32)>,
}
```

**Diagnostic Types:**
```rust
#[derive(Debug, Clone)]
pub struct DiagnosticReport {
    pub summary: ComplexitySummary,
    pub detailed_attribution: DetailedAttribution,
    pub recommendations: Vec<ComplexityRecommendation>,
    pub comparative_analysis: Option<ComparativeAnalysis>,
    pub performance_metrics: AnalysisPerformanceMetrics,
}

#[derive(Debug, Clone)]
pub struct ComplexityInsight {
    pub insight_type: InsightType,
    pub description: String,
    pub impact_level: ImpactLevel,
    pub actionable_steps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InsightType {
    FormattingImpact,
    PatternOpportunity,
    RefactoringCandidate,
    ComplexityHotspot,
    ImprovementSuggestion,
}
```

### APIs and Interfaces

**Public Multi-Pass API:**
```rust
// Main entry point for multi-pass analysis
pub fn analyze_with_attribution(
    source: &str,
    language: Language,
    options: MultiPassOptions,
) -> Result<MultiPassResult, AnalysisError> {
    let analyzer = MultiPassAnalyzer::new(options);
    analyzer.analyze(&prepare_analysis_unit(source, language)?)
}

// Comparative analysis between two code versions
pub fn compare_complexity(
    before: &str,
    after: &str,
    language: Language,
) -> Result<ComparativeAnalysis, AnalysisError> {
    let before_result = analyze_with_attribution(before, language, Default::default())?;
    let after_result = analyze_with_attribution(after, language, Default::default())?;
    generate_comparative_analysis(&before_result, &after_result)
}
```

**Configuration Interface:**
```rust
#[derive(Debug, Clone)]
pub struct MultiPassOptions {
    pub detail_level: DetailLevel,
    pub enable_recommendations: bool,
    pub track_source_locations: bool,
    pub generate_insights: bool,
    pub output_format: OutputFormat,
    pub performance_tracking: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DetailLevel {
    Summary,        // Basic complexity scores and high-level attribution
    Standard,       // Detailed attribution with source mapping
    Comprehensive,  // Full diagnostic information with insights
    Debug,          // Complete analysis trace for debugging
}
```

**Integration Interface:**
```rust
// Integration with existing analysis pipeline  
impl ComplexityAnalyzer {
    pub fn with_multi_pass(self, options: MultiPassOptions) -> MultiPassAnalyzer {
        MultiPassAnalyzer::from_analyzer(self, options)
    }
}

// Integration with risk analysis
impl RiskAnalyzer {
    pub fn analyze_with_attribution(
        &self,
        multi_pass_result: &MultiPassResult
    ) -> AttributedRiskAnalysis {
        // Use attribution data to enhance risk analysis
    }
}
```

## Dependencies

- **Prerequisites**:
  - [79] Semantic Normalization for Complexity Calculation (required for normalized complexity analysis)
  - Existing complexity calculation infrastructure
  - Risk analysis system (`src/risk/`)
- **Affected Components**:
  - CLI interface (`src/commands/`)
  - Output formatting system (`src/io/`)
  - Configuration system (`src/config.rs`)
  - Analysis coordination (`src/core/`)
- **External Dependencies**: No new external dependencies required

## Testing Strategy

### Unit Tests
- **Attribution Accuracy**: Verify complexity attribution correctly identifies source types and contributions
- **Source Mapping**: Test that complexity points map correctly to code locations
- **Comparative Analysis**: Validate before/after comparison logic for various code changes
- **Insight Generation**: Test that insights are generated appropriately for different complexity patterns
- **Performance Tracking**: Verify performance metrics accurately measure analysis overhead

### Integration Tests  
- **End-to-End Multi-Pass**: Complete analysis pipeline from source code to diagnostic report
- **Format Compatibility**: Ensure diagnostic reports work with existing output system
- **CLI Integration**: Test command-line interface with various multi-pass options
- **Configuration Loading**: Verify multi-pass options load correctly from configuration files
- **Risk Integration**: Test integration with risk analysis and strategy systems

### Performance Tests
- **Multi-Pass Overhead**: Measure performance impact of dual analysis
- **Memory Usage**: Monitor memory consumption during attribution and diagnostic generation
- **Scalability**: Test performance on large codebases with comprehensive attribution
- **Incremental Analysis**: Verify incremental updates work efficiently

### User Acceptance
- **Developer Experience**: Validate that diagnostic reports provide actionable insights
- **Report Quality**: Ensure diagnostic reports are clear, accurate, and useful
- **Configuration Usability**: Test that configuration options provide appropriate control
- **Integration Workflow**: Verify smooth integration with existing development workflows

## Documentation Requirements

### Code Documentation
- **Attribution Algorithm**: Document how complexity is attributed to different source types
- **Insight Generation**: Document the logic for generating complexity insights and recommendations
- **Performance Characteristics**: Document time and space complexity of multi-pass analysis
- **Extension Points**: Clear documentation for adding new attribution trackers and insight generators

### User Documentation
- **Multi-Pass Analysis Guide**: Comprehensive guide to using multi-pass analysis features
- **Diagnostic Report Interpretation**: Help users understand and act on diagnostic reports
- **Configuration Reference**: Complete reference for multi-pass analysis options
- **Troubleshooting**: Guide for debugging multi-pass analysis issues

### Architecture Updates
- **ARCHITECTURE.md**: Update to document multi-pass analysis architecture and data flow
- **Analysis Pipeline**: Document how multi-pass analysis integrates with existing pipeline
- **Performance Considerations**: Document performance implications and optimization strategies

## Implementation Notes

### Attribution Strategies

**Logical Complexity Attribution:**
- Map complexity to specific AST nodes (if/while/for loops, function calls, etc.)
- Track nesting levels and their contribution to overall complexity
- Identify cyclomatic vs cognitive complexity sources
- Attribute pattern-recognition adjustments to specific patterns

**Formatting Artifact Detection:**
- Compare raw and normalized complexity to identify formatting impact
- Track specific formatting transformations and their complexity effects
- Identify whitespace, parentheses, and line-break related artifacts
- Measure confidence level in artifact identification

**Change Impact Analysis:**
- Compare complexity attribution between code versions
- Identify whether changes are logical or formatting-related
- Track specific code modifications and their complexity impact
- Generate targeted recommendations based on change patterns

### Performance Optimizations

**Caching and Memoization:**
- Cache attribution results for unchanged code segments
- Memoize expensive insight generation calculations
- Use content-based cache keys for incremental analysis
- Implement lazy evaluation for optional diagnostic features

**Parallel Processing:**
- Run raw and normalized complexity analysis in parallel
- Parallelize attribution calculation across different source types
- Use rayon for parallel insight generation
- Implement work-stealing for unbalanced analysis workloads

**Memory Management:**
- Use arena allocation for temporary attribution data structures
- Implement copy-on-write semantics for shared analysis results
- Stream diagnostic report generation to avoid memory spikes
- Provide memory usage controls for resource-constrained environments

### Diagnostic Quality

**Insight Generation:**
- Use statistical analysis to identify meaningful complexity patterns
- Implement confidence scoring for recommendations
- Track user feedback to improve insight quality over time
- Provide context-specific recommendations based on code patterns

**Report Formatting:**
- Support multiple output formats (JSON, YAML, markdown, HTML)
- Implement responsive formatting based on terminal capabilities
- Provide summary views for high-level overview
- Include visual indicators for complexity trends and hotspots

## Migration and Compatibility

### Integration Strategy
- Multi-pass analysis is opt-in initially to avoid breaking existing workflows
- Provide CLI flags (`--multi-pass`, `--attribution`) to enable enhanced analysis
- Maintain full backward compatibility with existing single-pass analysis
- Support gradual migration with feature flags and configuration options

### Configuration Migration
- Extend existing configuration files with multi-pass options
- Provide sensible defaults that don't change existing behavior
- Support both old and new configuration formats during transition
- Offer configuration migration tools for complex setups

### Performance Considerations
- Multi-pass analysis adds 25% overhead but provides significantly more value
- Provide performance tuning options for different use cases
- Support lightweight modes for CI/CD integration
- Optimize for common usage patterns based on user feedback

### Future Extensibility
- Design attribution system to support additional complexity metrics
- Plan for integration with external analysis tools and IDEs
- Support custom insight generators and recommendation engines
- Prepare for machine learning-based complexity analysis improvements