# Spec 185: Integrated Architecture Analysis

**Status**: Draft
**Dependencies**: [181, 182, 183, 184]
**Priority**: Critical
**Created**: 2025-01-19

## Context

Specs 181-184 introduce complementary analysis approaches for god object refactoring:
- **Spec 181**: Type-based clustering (groups by data types)
- **Spec 182**: Data flow analysis (detects transformation pipelines)
- **Spec 183**: Anti-pattern detection (validates recommendations)
- **Spec 184**: Hidden type extraction (suggests missing types)

**Problem**: These specs operate independently and may produce conflicting recommendations. A single god object could generate:
- Type-based splits (Spec 181): `priority_item.rs`, `god_object_section.rs`
- Pipeline-based splits (Spec 182): `detection.rs`, `metrics.rs`, `recommendation.rs`
- Anti-pattern warnings for either approach
- Multiple hidden type suggestions

Without integration, users receive fragmented, potentially contradictory advice.

## Objective

Define a unified analysis pipeline that orchestrates specs 181-184 to produce coherent, non-conflicting recommendations with clear prioritization and performance budgets.

## Requirements

### 1. Analysis Pipeline Architecture

```rust
// src/organization/integrated_analyzer.rs

use crate::organization::{
    type_based_clustering::{TypeSignatureAnalyzer, TypeAffinityAnalyzer},
    data_flow_analyzer::DataFlowAnalyzer,
    anti_pattern_detector::AntiPatternDetector,
    hidden_type_extractor::HiddenTypeExtractor,
    god_object_analysis::{ModuleSplit, GodObjectAnalysis},
};
use std::time::{Duration, Instant};

/// Orchestrates all architecture analysis specs (181-184)
pub struct IntegratedArchitectureAnalyzer {
    config: AnalysisConfig,
}

/// Configuration for integrated analysis
#[derive(Clone, Debug)]
pub struct AnalysisConfig {
    /// Performance budget for total analysis time (default: 500ms)
    pub max_analysis_time: Duration,

    /// Minimum god object score to trigger advanced analysis (default: 50.0)
    pub advanced_analysis_threshold: f64,

    /// Strategy for resolving conflicting recommendations
    pub conflict_resolution: ConflictResolutionStrategy,

    /// Enable/disable individual analyzers
    pub enabled_analyzers: EnabledAnalyzers,

    /// Quality threshold for accepting recommendations (default: 60.0)
    pub min_quality_score: f64,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            max_analysis_time: Duration::from_millis(500),
            advanced_analysis_threshold: 50.0,
            conflict_resolution: ConflictResolutionStrategy::Hybrid,
            enabled_analyzers: EnabledAnalyzers::all(),
            min_quality_score: 60.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EnabledAnalyzers {
    pub type_based: bool,
    pub data_flow: bool,
    pub anti_pattern: bool,
    pub hidden_types: bool,
}

impl EnabledAnalyzers {
    pub fn all() -> Self {
        Self {
            type_based: true,
            data_flow: true,
            anti_pattern: true,
            hidden_types: true,
        }
    }

    pub fn minimal() -> Self {
        Self {
            type_based: false,
            data_flow: false,
            anti_pattern: true,  // Always run anti-pattern detection
            hidden_types: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConflictResolutionStrategy {
    /// Use type-based clustering exclusively
    TypeBased,

    /// Use data flow analysis exclusively
    DataFlow,

    /// Choose based on confidence scores
    BestConfidence,

    /// Merge both approaches (recommended)
    Hybrid,

    /// Present both to user for selection
    UserChoice,
}
```

### 2. Unified Analysis Pipeline

```rust
impl IntegratedArchitectureAnalyzer {
    pub fn new() -> Self {
        Self {
            config: AnalysisConfig::default(),
        }
    }

    pub fn with_config(config: AnalysisConfig) -> Self {
        Self { config }
    }

    /// Run integrated analysis pipeline
    pub fn analyze(
        &self,
        god_object: &GodObjectAnalysis,
        ast: &syn::File,
        call_graph: &HashMap<String, Vec<String>>,
    ) -> Result<IntegratedAnalysisResult, AnalysisError> {
        let start_time = Instant::now();

        // Phase 1: Fast path - Anti-pattern detection (always run)
        let anti_pattern_result = if self.config.enabled_analyzers.anti_pattern {
            self.run_anti_pattern_detection(god_object, ast)?
        } else {
            None
        };

        // Phase 2: Advanced analysis (conditional on score threshold)
        let advanced_results = if god_object.god_object_score >= self.config.advanced_analysis_threshold {
            self.run_advanced_analysis(god_object, ast, call_graph, start_time)?
        } else {
            AdvancedAnalysisResults::empty()
        };

        // Phase 3: Conflict resolution
        let unified_splits = self.resolve_conflicts(
            advanced_results.type_based_splits,
            advanced_results.data_flow_splits,
        );

        // Phase 4: Quality validation
        let validated_splits = self.validate_quality(
            unified_splits,
            anti_pattern_result.as_ref(),
        );

        // Phase 5: Hidden type enrichment
        let enriched_splits = if self.config.enabled_analyzers.hidden_types {
            self.enrich_with_hidden_types(
                validated_splits,
                advanced_results.hidden_types,
            )
        } else {
            validated_splits
        };

        let elapsed = start_time.elapsed();

        Ok(IntegratedAnalysisResult {
            unified_splits: enriched_splits,
            anti_patterns: anti_pattern_result,
            hidden_types: advanced_results.hidden_types,
            analysis_metadata: AnalysisMetadata {
                total_time: elapsed,
                timeout_occurred: elapsed > self.config.max_analysis_time,
                strategy_used: self.config.conflict_resolution.clone(),
                analyzers_run: self.config.enabled_analyzers.clone(),
            },
        })
    }

    fn run_anti_pattern_detection(
        &self,
        god_object: &GodObjectAnalysis,
        ast: &syn::File,
    ) -> Result<Option<AntiPatternReport>, AnalysisError> {
        let detector = AntiPatternDetector::new();
        let signatures = extract_method_signatures(ast)?;

        let quality_report = detector.calculate_split_quality(
            &god_object.recommended_splits,
            &signatures,
        );

        Ok(Some(AntiPatternReport {
            quality_score: quality_report.quality_score,
            anti_patterns: quality_report.anti_patterns,
        }))
    }

    fn run_advanced_analysis(
        &self,
        god_object: &GodObjectAnalysis,
        ast: &syn::File,
        call_graph: &HashMap<String, Vec<String>>,
        start_time: Instant,
    ) -> Result<AdvancedAnalysisResults, AnalysisError> {
        // Check budget before each expensive operation
        let budget_check = || {
            if start_time.elapsed() > self.config.max_analysis_time {
                Err(AnalysisError::TimeoutExceeded)
            } else {
                Ok(())
            }
        };

        // Extract type signatures (shared by multiple analyzers)
        let signatures = extract_method_signatures(ast)?;
        budget_check()?;

        // Run type-based analysis
        let type_based_splits = if self.config.enabled_analyzers.type_based {
            budget_check()?;
            Some(self.run_type_based_analysis(&signatures, ast)?)
        } else {
            None
        };

        // Run data flow analysis
        let data_flow_splits = if self.config.enabled_analyzers.data_flow {
            budget_check()?;
            Some(self.run_data_flow_analysis(&signatures, call_graph, ast)?)
        } else {
            None
        };

        // Run hidden type extraction
        let hidden_types = if self.config.enabled_analyzers.hidden_types {
            budget_check()?;
            Some(self.run_hidden_type_extraction(&signatures, ast)?)
        } else {
            None
        };

        Ok(AdvancedAnalysisResults {
            type_based_splits,
            data_flow_splits,
            hidden_types,
        })
    }

    fn run_type_based_analysis(
        &self,
        signatures: &[MethodSignature],
        ast: &syn::File,
    ) -> Result<Vec<ModuleSplit>, AnalysisError> {
        let affinity_analyzer = TypeAffinityAnalyzer::new();
        let clusters = affinity_analyzer.cluster_by_type_affinity(signatures);

        // Convert clusters to ModuleSplit
        Ok(clusters.into_iter().map(|cluster| {
            ModuleSplit {
                suggested_name: cluster.primary_type.name.clone(),
                methods_to_move: cluster.methods,
                responsibility: format!(
                    "Manage {} data and transformations",
                    cluster.primary_type.name
                ),
                method: SplitAnalysisMethod::TypeBased,
                ..Default::default()
            }
        }).collect())
    }

    fn run_data_flow_analysis(
        &self,
        signatures: &[MethodSignature],
        call_graph: &HashMap<String, Vec<String>>,
        ast: &syn::File,
    ) -> Result<Vec<ModuleSplit>, AnalysisError> {
        let flow_analyzer = DataFlowAnalyzer::new();
        let flow_graph = flow_analyzer.build_type_flow_graph(signatures, call_graph);
        let stages = flow_analyzer.detect_pipeline_stages(&flow_graph, signatures)?;

        Ok(flow_analyzer.generate_pipeline_recommendations(&stages, ""))
    }

    fn run_hidden_type_extraction(
        &self,
        signatures: &[MethodSignature],
        ast: &syn::File,
    ) -> Result<Vec<HiddenType>, AnalysisError> {
        let extractor = HiddenTypeExtractor::new();
        Ok(extractor.extract_hidden_types(signatures, ast, ""))
    }

    fn resolve_conflicts(
        &self,
        type_based: Option<Vec<ModuleSplit>>,
        data_flow: Option<Vec<ModuleSplit>>,
    ) -> Vec<ModuleSplit> {
        use ConflictResolutionStrategy::*;

        match self.config.conflict_resolution {
            TypeBased => type_based.unwrap_or_default(),
            DataFlow => data_flow.unwrap_or_default(),

            BestConfidence => {
                // Choose approach with higher average cohesion
                let type_avg = type_based.as_ref()
                    .map(|s| avg_cohesion(s))
                    .unwrap_or(0.0);
                let flow_avg = data_flow.as_ref()
                    .map(|s| avg_cohesion(s))
                    .unwrap_or(0.0);

                if type_avg >= flow_avg {
                    type_based.unwrap_or_default()
                } else {
                    data_flow.unwrap_or_default()
                }
            }

            Hybrid => {
                // Merge non-overlapping splits
                self.merge_splits(type_based, data_flow)
            }

            UserChoice => {
                // Return both, marked for user selection
                let mut combined = type_based.unwrap_or_default();
                combined.extend(data_flow.unwrap_or_default());
                combined
            }
        }
    }

    fn merge_splits(
        &self,
        type_based: Option<Vec<ModuleSplit>>,
        data_flow: Option<Vec<ModuleSplit>>,
    ) -> Vec<ModuleSplit> {
        let mut merged = Vec::new();
        let type_splits = type_based.unwrap_or_default();
        let flow_splits = data_flow.unwrap_or_default();

        // Add type-based splits
        for split in type_splits {
            merged.push(split);
        }

        // Add non-overlapping flow splits
        for flow_split in flow_splits {
            let methods: HashSet<_> = flow_split.methods_to_move.iter().collect();
            let overlaps = merged.iter().any(|existing| {
                let existing_methods: HashSet<_> = existing.methods_to_move.iter().collect();
                methods.intersection(&existing_methods).count() > methods.len() / 2
            });

            if !overlaps {
                merged.push(flow_split);
            }
        }

        merged
    }

    fn validate_quality(
        &self,
        splits: Vec<ModuleSplit>,
        anti_pattern_report: Option<&AntiPatternReport>,
    ) -> Vec<ModuleSplit> {
        if let Some(report) = anti_pattern_report {
            if report.quality_score < self.config.min_quality_score {
                // Filter out splits with critical anti-patterns
                return splits.into_iter()
                    .filter(|split| !has_critical_anti_pattern(split, &report.anti_patterns))
                    .collect();
            }
        }
        splits
    }

    fn enrich_with_hidden_types(
        &self,
        mut splits: Vec<ModuleSplit>,
        hidden_types: Option<Vec<HiddenType>>,
    ) -> Vec<ModuleSplit> {
        if let Some(types) = hidden_types {
            for split in &mut splits {
                // Find matching hidden type for this split
                if let Some(hidden_type) = types.iter()
                    .find(|t| split.methods_to_move.iter().any(|m| t.methods.contains(m)))
                {
                    split.implicit_type_suggestion = Some(ImplicitTypeSuggestion {
                        type_name: hidden_type.suggested_name.clone(),
                        fields: hidden_type.fields.iter()
                            .map(|f| (f.name.clone(), f.type_info.name.clone()))
                            .collect(),
                        occurrences: hidden_type.occurrences,
                        confidence: hidden_type.confidence,
                        rationale: hidden_type.rationale.clone(),
                    });
                }
            }
        }
        splits
    }
}

#[derive(Debug, Clone)]
struct AdvancedAnalysisResults {
    type_based_splits: Option<Vec<ModuleSplit>>,
    data_flow_splits: Option<Vec<ModuleSplit>>,
    hidden_types: Option<Vec<HiddenType>>,
}

impl AdvancedAnalysisResults {
    fn empty() -> Self {
        Self {
            type_based_splits: None,
            data_flow_splits: None,
            hidden_types: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IntegratedAnalysisResult {
    pub unified_splits: Vec<ModuleSplit>,
    pub anti_patterns: Option<AntiPatternReport>,
    pub hidden_types: Option<Vec<HiddenType>>,
    pub analysis_metadata: AnalysisMetadata,
}

#[derive(Debug, Clone)]
pub struct AntiPatternReport {
    pub quality_score: f64,
    pub anti_patterns: Vec<AntiPattern>,
}

#[derive(Debug, Clone)]
pub struct AnalysisMetadata {
    pub total_time: Duration,
    pub timeout_occurred: bool,
    pub strategy_used: ConflictResolutionStrategy,
    pub analyzers_run: EnabledAnalyzers,
}

#[derive(Debug, Clone)]
pub enum AnalysisError {
    TimeoutExceeded,
    AstParseError(String),
    DataFlowCycle(String),
}

// Helper functions
fn avg_cohesion(splits: &[ModuleSplit]) -> f64 {
    if splits.is_empty() {
        return 0.0;
    }
    splits.iter()
        .filter_map(|s| s.cohesion_score)
        .sum::<f64>() / splits.len() as f64
}

fn has_critical_anti_pattern(split: &ModuleSplit, patterns: &[AntiPattern]) -> bool {
    patterns.iter().any(|p| {
        p.severity == AntiPatternSeverity::Critical &&
        p.location == split.suggested_name
    })
}

fn extract_method_signatures(ast: &syn::File) -> Result<Vec<MethodSignature>, AnalysisError> {
    // Implementation from Spec 181
    todo!("Extract method signatures from AST")
}

use std::collections::HashSet;
use crate::organization::god_object_analysis::{SplitAnalysisMethod, ImplicitTypeSuggestion};
use crate::organization::type_based_clustering::{MethodSignature, TypeAffinityAnalyzer};
use crate::organization::data_flow_analyzer::DataFlowAnalyzer;
use crate::organization::anti_pattern_detector::{AntiPatternDetector, AntiPattern, AntiPatternSeverity};
use crate::organization::hidden_type_extractor::{HiddenTypeExtractor, HiddenType};
use std::collections::HashMap;
```

### 3. Integration with God Object Detector

```rust
// src/organization/god_object_detector.rs

fn enhance_god_object_analysis_with_architecture(
    mut analysis: GodObjectAnalysis,
    ast: &syn::File,
    call_graph: &HashMap<String, Vec<String>>,
) -> GodObjectAnalysis {
    let integrated_analyzer = IntegratedArchitectureAnalyzer::new();

    match integrated_analyzer.analyze(&analysis, ast, call_graph) {
        Ok(result) => {
            // Replace splits with integrated results
            analysis.recommended_splits = result.unified_splits;

            // Add metadata
            if let Some(anti_pattern_report) = result.anti_patterns {
                analysis.split_quality_score = Some(anti_pattern_report.quality_score);
                analysis.anti_patterns = Some(anti_pattern_report.anti_patterns);
            }

            analysis.hidden_types = result.hidden_types;
            analysis.analysis_time = Some(result.analysis_metadata.total_time);

            analysis
        }
        Err(e) => {
            // Fallback to basic analysis
            eprintln!("Integrated analysis failed: {:?}", e);
            analysis
        }
    }
}
```

## Performance Budgets

### Time Budgets (Per File Analysis)

| Analyzer | Budget | Fallback Strategy |
|----------|--------|-------------------|
| Anti-pattern detection | 50ms | Required, no fallback |
| Type signature extraction | 100ms | Cache and reuse |
| Type-based clustering | 150ms | Skip if timeout |
| Data flow analysis | 150ms | Skip if timeout |
| Hidden type extraction | 50ms | Skip if timeout |
| **Total** | **500ms** | Return partial results |

### Memory Budgets

| Data Structure | Max Size | Mitigation |
|----------------|----------|------------|
| Type flow graph | 10,000 edges | Sample large graphs |
| Affinity matrix | 10,000 pairs | Use sparse matrix |
| Method signatures | 1,000 methods | Process in batches |

### Optimization Strategies

```rust
// Lazy evaluation - only compute when needed
struct LazyTypeFlowGraph {
    graph: OnceCell<TypeFlowGraph>,
    signatures: Arc<[MethodSignature]>,
}

// Caching - reuse expensive computations
struct CachedAnalyzer {
    signature_cache: HashMap<PathBuf, Vec<MethodSignature>>,
    affinity_cache: HashMap<(String, String), f64>,
}

// Parallel processing - use rayon for independent analyses
fn analyze_parallel(files: &[PathBuf]) -> Vec<IntegratedAnalysisResult> {
    files.par_iter()
        .map(|file| analyze_file(file))
        .collect()
}
```

## Testing Strategy

### Integration Tests

```rust
// tests/integrated_analysis.rs

#[test]
fn test_formatter_rs_full_pipeline() {
    let code = include_str!("../fixtures/formatter.rs");
    let ast = syn::parse_file(code).unwrap();
    let call_graph = build_call_graph(&ast);

    let analyzer = IntegratedArchitectureAnalyzer::new();
    let god_object = create_test_god_object();

    let result = analyzer.analyze(&god_object, &ast, &call_graph).unwrap();

    // Verify non-conflicting recommendations
    assert!(result.unified_splits.len() > 0);
    assert!(result.unified_splits.len() < 10); // Not fragmented

    // Verify quality
    if let Some(anti_patterns) = result.anti_patterns {
        assert!(anti_patterns.quality_score >= 60.0);
    }

    // Verify performance
    assert!(result.analysis_metadata.total_time < Duration::from_millis(500));
}

#[test]
fn test_conflict_resolution_hybrid() {
    // Test that hybrid strategy merges non-overlapping splits
    let config = AnalysisConfig {
        conflict_resolution: ConflictResolutionStrategy::Hybrid,
        ..Default::default()
    };

    let analyzer = IntegratedArchitectureAnalyzer::with_config(config);
    // ... test hybrid merging logic
}

#[test]
fn test_timeout_handling() {
    let config = AnalysisConfig {
        max_analysis_time: Duration::from_millis(1),
        ..Default::default()
    };

    let analyzer = IntegratedArchitectureAnalyzer::with_config(config);
    // Should return timeout error or partial results
}

#[test]
fn test_performance_on_large_file() {
    // Test file with 1000+ methods
    let large_ast = generate_large_ast(1000);
    let start = Instant::now();

    let analyzer = IntegratedArchitectureAnalyzer::new();
    let result = analyzer.analyze(&god_object, &large_ast, &call_graph);

    assert!(start.elapsed() < Duration::from_secs(2));
}
```

### Benchmark Suite

```rust
// benches/integrated_analysis_bench.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_type_based_analysis(c: &mut Criterion) {
    let ast = load_test_ast("formatter.rs");
    let signatures = extract_method_signatures(&ast).unwrap();

    c.bench_function("type_based_clustering", |b| {
        b.iter(|| {
            let analyzer = TypeAffinityAnalyzer::new();
            black_box(analyzer.cluster_by_type_affinity(&signatures))
        })
    });
}

fn bench_data_flow_analysis(c: &mut Criterion) {
    let ast = load_test_ast("god_object_analysis.rs");
    let signatures = extract_method_signatures(&ast).unwrap();
    let call_graph = build_call_graph(&ast);

    c.bench_function("data_flow_analysis", |b| {
        b.iter(|| {
            let analyzer = DataFlowAnalyzer::new();
            let graph = analyzer.build_type_flow_graph(&signatures, &call_graph);
            black_box(analyzer.detect_pipeline_stages(&graph, &signatures))
        })
    });
}

criterion_group!(benches, bench_type_based_analysis, bench_data_flow_analysis);
criterion_main!(benches);
```

## Configuration Examples

### Minimal Configuration (Fast)

```toml
[analysis.architecture]
enabled_analyzers.type_based = false
enabled_analyzers.data_flow = false
enabled_analyzers.anti_pattern = true
enabled_analyzers.hidden_types = false
max_analysis_time = 100  # milliseconds
```

### Balanced Configuration (Default)

```toml
[analysis.architecture]
enabled_analyzers.type_based = true
enabled_analyzers.data_flow = true
enabled_analyzers.anti_pattern = true
enabled_analyzers.hidden_types = true
conflict_resolution = "Hybrid"
max_analysis_time = 500
advanced_analysis_threshold = 50.0
min_quality_score = 60.0
```

### Thorough Configuration (Slow but Comprehensive)

```toml
[analysis.architecture]
enabled_analyzers.type_based = true
enabled_analyzers.data_flow = true
enabled_analyzers.anti_pattern = true
enabled_analyzers.hidden_types = true
conflict_resolution = "UserChoice"  # Show all options
max_analysis_time = 2000
advanced_analysis_threshold = 0.0  # Always run
min_quality_score = 80.0
```

## Migration Path

### Phase 1: Infrastructure (Week 1)
- [ ] Create `integrated_analyzer.rs` with basic pipeline
- [ ] Add `AnalysisConfig` and `IntegratedAnalysisResult` types
- [ ] Implement timeout handling and budget checks

### Phase 2: Anti-Pattern Integration (Week 1)
- [ ] Integrate Spec 183 (anti-pattern detection)
- [ ] Add quality validation
- [ ] Test on existing god objects

### Phase 3: Type Analysis Integration (Week 2)
- [ ] Integrate Spec 181 (type-based clustering)
- [ ] Add conflict resolution for type-based vs existing splits
- [ ] Benchmark performance

### Phase 4: Data Flow Integration (Week 3)
- [ ] Integrate Spec 182 (data flow analysis)
- [ ] Implement hybrid conflict resolution
- [ ] Add pipeline stage detection

### Phase 5: Hidden Types Integration (Week 3)
- [ ] Integrate Spec 184 (hidden type extraction)
- [ ] Enrich splits with type suggestions
- [ ] Generate complete refactoring guidance

### Phase 6: Optimization (Week 4)
- [ ] Add caching for expensive computations
- [ ] Implement lazy evaluation
- [ ] Parallelize independent analyses
- [ ] Add comprehensive benchmarks

## Success Criteria

- [ ] All 4 specs (181-184) integrated without conflicts
- [ ] Total analysis time < 500ms for typical god objects (100 methods)
- [ ] Quality score accurately reflects idiomatic Rust adherence
- [ ] No fragmented recommendations (< 10 splits per god object)
- [ ] Anti-pattern detection catches 100% of utilities modules
- [ ] Hybrid strategy produces non-overlapping, coherent splits
- [ ] Graceful degradation on timeout (returns partial results)
- [ ] User documentation explains analysis strategies

## Output Format

```
#1 SCORE: 149 [CRITICAL]
└─ ./src/priority/formatter.rs (3000 lines, 103 functions)

Architecture Analysis:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Analysis Strategy: Hybrid (Type-Based + Data Flow)
Quality Score: 75/100 (Good)
Analysis Time: 234ms

Recommended Refactoring (5 modules):

1. priority_item.rs (25 methods, ~400 lines)
   Strategy: Type-Based Clustering
   Core Type: PriorityItem
   Confidence: 0.95

   Hidden Type Suggestion:
   pub struct PriorityItem {
       pub score: f64,
       pub location: PathBuf,
       pub metrics: FileMetrics,
       pub verbosity: u8,
   }

   Methods: format_header, render_section, validate_item...

2. god_object_section.rs (20 methods, ~350 lines)
   Strategy: Type-Based Clustering
   Core Type: GodObjectSection
   Confidence: 0.88

3. detection.rs (12 methods, ~200 lines)
   Strategy: Data Flow Analysis
   Pipeline Stage: Source
   Input: StructData → Output: GodObjectAnalysis

Anti-Patterns Detected (2):
  [MEDIUM] Parameter Passing in format_header (4 params)
  → Resolved by PriorityItem extraction

  [LOW] Minor naming improvements suggested

Hidden Types Discovered (3):
  ✅ PriorityItem (8 occurrences, confidence: 0.95)
  ✅ GodObjectSection (6 occurrences, confidence: 0.88)
  ⚠ AnalysisContext (3 occurrences, confidence: 0.65)
```

## Future Enhancements

1. **Machine Learning Integration**: Learn optimal conflict resolution from user feedback
2. **Interactive Mode**: Allow user to adjust analysis strategy in real-time
3. **Incremental Analysis**: Cache results across multiple runs
4. **Visualization**: Generate diagrams of type flows and dependencies
5. **Auto-Refactoring**: Generate pull requests with actual code changes

## Appendix: Decision Tree

```
God Object Detected (score > threshold)
│
├─ Anti-Pattern Detection (always run)
│  └─ Quality Score < 60? → Filter critical anti-patterns
│
├─ Score > 50? (advanced analysis threshold)
│  │
│  ├─ Yes: Run Advanced Analysis
│  │  │
│  │  ├─ Type Signatures Extracted (shared)
│  │  │
│  │  ├─ Type-Based Clustering
│  │  │  └─ Produces type-centric splits
│  │  │
│  │  ├─ Data Flow Analysis
│  │  │  └─ Produces pipeline-stage splits
│  │  │
│  │  └─ Hidden Type Extraction
│  │     └─ Suggests missing types
│  │
│  └─ No: Use existing splits
│
├─ Conflict Resolution
│  ├─ Hybrid: Merge non-overlapping splits
│  ├─ BestConfidence: Choose higher cohesion
│  └─ UserChoice: Present both options
│
└─ Enrich with Hidden Types
   └─ Final unified recommendation
```
