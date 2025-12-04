# God Object Refactoring Plan - Phase 1 Analysis

**Specification**: 181a - Foundation & Analysis
**Date**: 2025-12-03
**Status**: Analysis Complete

## Executive Summary

### Current State
- **Total lines**: 8,762 lines across 6 files
- **Main files to refactor**:
  - `god_object_detector.rs` - 4,362 lines (66 functions)
  - `god_object_analysis.rs` - 3,304 lines (44 functions + 155 tests)
  - `god_object_metrics.rs` - 367 lines (already modular)
- **Already modularized**:
  - `god_object/ast_visitor.rs` - 365 lines (well-structured)
  - `god_object/metrics.rs` - 349 lines (well-structured)
  - `god_object/mod.rs` - 15 lines (module exports)

### Target State
Split into 7 focused modules under `src/organization/god_object/`:
- `types.rs` - Core data structures (~200 lines)
- `thresholds.rs` - Configuration constants (~100 lines)
- `predicates.rs` - Pure boolean/classification logic (~150 lines)
- `scoring.rs` - Pure scoring algorithms (~200 lines)
- `classifier.rs` - Classification and grouping (~200 lines)
- `recommender.rs` - Split recommendation generation (~250 lines)
- `detector.rs` - Orchestration and AST traversal (~250 lines)

### Risk Assessment
- **Low Risk**: Most functions are already pure or clearly separable
- **Medium Risk**: 11 mixed functions need splitting before moving
- **High Risk**: Large orchestration functions (257, 188, 172 lines)
- **Circular Dependency Risk**: None identified
- **Test Impact**: 6 test files depend on 42+ public exports

### Estimated Effort
- **Phase 1 (Analysis)**: Complete ✓
- **Phase 2 (Extract Types)**: 4 hours
- **Phase 3-8 (Extract remaining modules)**: 12 hours
- **Phase 9 (Final cleanup)**: 2 hours
- **Total**: ~2-3 days

---

## Existing Module Analysis

### ast_visitor.rs (365 lines) - ✓ Well Structured
**Status**: Keep as-is, no refactoring needed

**Responsibilities**:
- AST traversal for type and method collection
- Complexity extraction from functions
- Visibility tracking for methods
- Module function information extraction

**Structure**:
- Clean separation between data structures and visitor logic
- Pure helper functions for extraction
- Implements `syn::visit::Visit` trait correctly
- Follows Stillwater principles (I/O boundary)

**Dependencies**:
- `syn` crate for AST
- Internal: `common::SourceLocation`, `complexity::cyclomatic`
- Exports: `TypeAnalysis`, `TypeVisitor`, `FunctionWeight`, etc.

**Assessment**: This module is well-designed and doesn't need refactoring.

---

### metrics.rs (349 lines) - ✓ Well Structured
**Status**: Keep as-is, no refactoring needed

**Responsibilities**:
- Build per-struct metrics from visitor data
- Calculate weighted metrics (complexity, purity)
- Calculate final god object scores
- Calculate visibility breakdowns
- Integrate visibility into function counts

**Structure**:
- All functions are pure or orchestration-only
- Clear functional composition pattern
- Well-documented with spec references
- Good separation of concerns

**Key Functions**:
- `build_per_struct_metrics` - Pure aggregation
- `calculate_weighted_metrics` - Pure computation
- `calculate_final_god_object_score` - Pure scoring
- `calculate_purity_weights` - Pure analysis
- `calculate_visibility_breakdown` - Pure classification

**Assessment**: Excellent functional design, no changes needed.

---

### mod.rs (15 lines) - Module Exports
**Current Exports**:
```rust
pub mod ast_visitor;
pub mod metrics;

pub use ast_visitor::{
    FunctionParameter, FunctionWeight, ModuleFunctionInfo,
    Responsibility, TypeAnalysis, TypeVisitor,
};
```

**Required After Refactoring**:
All 42+ public API exports must be re-exported from the new module structure.

---

## Public API Inventory

### Complete List of Public Exports (42 items)

Based on analysis of 6 test files (`tests/god_object_*.rs`), the following exports are required:

#### Core Types (18 items)
1. `GodObjectDetector` - Main detection interface
2. `FileMetrics` - Comprehensive analysis results (aka `GodObjectAnalysis`)
3. `EnhancedAnalysis` - Enhanced analysis with classification (aka `EnhancedGodObjectAnalysis`)
4. `GodObjectType` - Classification enum (NotGodObject, GodModule, GodClass)
5. `GodObjectConfidence` - Confidence levels
6. `GodObjectThresholds` - Configuration thresholds
7. `DetectionType` - GodClass vs GodFile enum
8. `GodObjectMetrics` - Metrics tracking
9. `TrendDirection` - Improving/Worsening
10. `StructMetrics` - Per-struct information
11. `RecommendationSeverity` - Critical/High/Medium
12. `BehaviorCategory` - Method categorization
13. `BehavioralCategorizer` - Categorization logic
14. `FieldAccessTracker` - Field access tracking
15. `SplitAnalysisMethod` - Analysis method enum
16. `ModuleSplit` - Recommended module splits
17. `AggregatedClassification` - Classification with confidence
18. `ResponsibilityCategory` - Database/Config/etc.

#### Core Functions (24 items)
19. `GodObjectDetector::with_source_content(&str) -> Self`
20. `detector.analyze_comprehensive(&Path, &File) -> FileMetrics`
21. `detector.analyze_enhanced(&Path, &File) -> EnhancedAnalysis`
22. `calculate_god_object_score(...) -> f64`
23. `determine_confidence(...) -> GodObjectConfidence`
24. `infer_responsibility_with_confidence(&str, Option<&str>) -> ClassificationResult`
25. `recommend_module_splits(...) -> Vec<ModuleSplit>`
26. `recommend_module_splits_with_evidence(...) -> Vec<ModuleSplit>`
27. `emit_classification_metrics(&ClassificationMetrics)`
28. `cluster_methods_by_behavior(&[String]) -> HashMap<BehaviorCategory, Vec<String>>`
29. `BehavioralCategorizer::categorize_method(&str) -> BehaviorCategory`
30. `FieldAccessTracker::new() -> Self`
31. `tracker.analyze_impl(&ItemImpl)`
32. `tracker.get_method_fields(&str) -> Vec<String>`
33. `tracker.get_minimal_field_set(&[String]) -> Vec<String>`
34. `calculate_struct_ratio(usize, usize) -> f64`
35. `count_distinct_domains(&[StructMetrics]) -> usize`
36. `determine_cross_domain_severity(...) -> RecommendationSeverity`
37. `ClassificationMetrics::new()`
38. `metrics.record_classification(Option<&str>)`
39. `GodObjectMetrics::new()`
40. `metrics.record_snapshot(PathBuf, FileMetrics)`
41. `metrics.get_file_trend(&PathBuf) -> Option<FileTrend>`
42. `group_methods_by_responsibility(&[String]) -> HashMap`

**Backward Compatibility Requirement**: All 42 exports must remain accessible after refactoring.

---

## Function Classification Map

### god_object_detector.rs (66 functions)

| Function | Type | Lines | Complexity | Target Module | Notes |
|----------|------|-------|------------|---------------|-------|
| `CallGraphAdapter::from_adjacency_matrix` | Pure | 5 | Low | types | Data structure adapter |
| `CallGraphAdapter::call_count` | Pure | 5 | Low | types | Lookup method |
| `CallGraphAdapter::callees` | Pure | 7 | Low | types | Collection method |
| `CallGraphAdapter::callers` | Pure | 7 | Low | types | Collection method |
| `FieldAccessAdapter::new` | Pure | 3 | Low | types | Constructor |
| `FieldAccessAdapter::fields_accessed_by` | I/O | 3 | Low | types | Tracker query |
| `FieldAccessAdapter::writes_to_field` | I/O | 3 | Low | types | Tracker query |
| `GodObjectDetector::default` | Pure | 8 | Low | types | Constructor |
| `GodObjectDetector::new` | Pure | 3 | Low | types | Constructor |
| `GodObjectDetector::with_source_content` | Pure | 8 | Low | types | Constructor |
| `analyze_enhanced` | Orchestration | 40 | Medium | detector | Pipeline coordination |
| `classify_god_object` | **Mixed** | **172** | **High** | classifier | **Needs splitting** |
| `generate_recommendation` | Pure | 198 | Medium | recommender | String generation |
| `try_module_function_classification` | Mixed | 35 | Medium | classifier | Classifier usage |
| `get_thresholds_for_path` | Pure | 16 | Low | thresholds | Config selection |
| `determine_god_object_type` | Pure | 75 | Medium | classifier | Type determination |
| `estimate_standalone_complexity` | Pure | 3 | Low | scoring | Simple arithmetic |
| `analyze_domains_and_recommend_splits` | **Orchestration** | **257** | **High** | recommender | **Large coordinator** |
| `try_improved_clustering` | **Orchestration** | **188** | **High** | classifier | **Large clustering** |
| `infer_responsibility_from_cluster` | **Mixed** | **123** | **High** | classifier | **Needs splitting** |
| `extract_method_details` | I/O | 49 | Medium | detector | AST traversal |
| `extract_struct_names` | I/O | 22 | Low | detector | AST traversal |
| `extract_common_prefix` | Pure | 34 | Medium | predicates | String tokenization |
| `sanitize_module_name` | Pure | 5 | Low | predicates | String transform |
| `check_if_pure_method` | Pure | 14 | Low | predicates | Pattern match |
| `detect_visibility` | I/O | 18 | Low | detector | AST traversal |
| `estimate_method_complexity` | I/O | 15 | Low | scoring | AST traversal |
| `count_statements` | Pure | 3 | Low | scoring | Block length |
| `detect_io_operations` | Pure | 8 | Low | predicates | Keyword match |
| `generate_behavioral_splits` | **Orchestration** | **163** | **High** | recommender | **Large generator** |
| `capitalize_first` | Pure | 6 | Low | predicates | String util |
| `generate_type_based_splits` | Orchestration | 126 | High | recommender | Type affinity |
| `generate_type_definition_example` | Pure | 96 | Medium | recommender | String generation |
| `generate_pipeline_based_splits` | Orchestration | 65 | Medium | recommender | Data flow |
| `enrich_splits_with_behavioral_analysis` | Mixed | 40 | Medium | recommender | Enrichment |
| `analyze_module_structure_and_visibility` | Mixed | 49 | Medium | detector | Analysis |
| `analyze_comprehensive` | Orchestration | 150 | High | detector | Main pipeline |
| `analyze_with_integrated_architecture` | Orchestration | 36 | Medium | detector | Integrated analyzer |
| `build_call_graph` | I/O | 59 | Medium | detector | AST visitor |
| `CallGraphVisitor::visit_impl_item_fn` | I/O | 7 | Low | detector | Visitor method |
| `CallGraphVisitor::visit_item_fn` | I/O | 7 | Low | detector | Visitor method |
| `CallGraphVisitor::visit_expr_call` | I/O | 14 | Low | detector | Visitor method |
| `CallGraphVisitor::visit_expr_method_call` | I/O | 9 | Low | detector | Visitor method |
| `analyze_type` | I/O | 18 | Low | detector | Type analysis |
| `count_fields` | Pure | 7 | Low | scoring | Pattern match |
| `extract_field_names` | Pure | 9 | Low | detector | Field extraction |
| `classify_god_object_impact` | Pure | 9 | Low | classifier | Impact classification |
| `is_god_object` | Pure | 5 | Low | predicates | Boolean check |
| `suggest_responsibility_split` | Orchestration | 15 | Low | recommender | Split suggestion |
| `create_responsibility_group` | Pure | 12 | Low | types | Struct creation |
| `create_default_responsibility_group` | Pure | 8 | Low | types | Struct creation |
| `group_methods_by_prefix` | Pure | 9 | Low | classifier | Grouping logic |
| `extract_method_prefix` | Pure | 4 | Low | predicates | Prefix extraction |
| `find_matching_prefix` | Pure | 10 | Low | predicates | Array search |
| `extract_first_word` | Pure | 6 | Low | predicates | String split |
| `infer_responsibility_name` | Pure | 3 | Low | classifier | Name inference |
| `classify_responsibility` | Pure | 15 | Low | classifier | Pattern matching |
| `validate_and_improve_splits` | Orchestration | 28 | Medium | recommender | Validation |
| `apply_domain_aware_naming_to_type_based_splits` | Mixed | 26 | Medium | recommender | Naming |
| `detect_domain_pattern_for_methods` | Mixed | 82 | High | classifier | Pattern detection |
| `apply_semantic_naming_to_splits` | Orchestration | 10 | Low | recommender | Name generation |
| `apply_split_quality_limiting` | Pure | 52 | Medium | recommender | Sort + truncate |
| `apply_semantic_naming` | Mixed | 40 | Medium | recommender | Name generation |
| `detect_anti_patterns` | Orchestration | 23 | Medium | detector | Visitor + analysis |
| `detector_name` | Pure | 3 | Low | types | String literal |
| `estimate_maintainability_impact` | Pure | 12 | Low | classifier | Pattern match |

**Summary**:
- Pure Computation: 29 (44%)
- I/O Operations: 14 (21%)
- Orchestration: 13 (20%)
- Mixed (needs splitting): 10 (15%)

### god_object_analysis.rs (44 functions + 155 tests)

| Function | Type | Lines | Complexity | Target Module | Notes |
|----------|------|-------|------------|---------------|-------|
| `GodObjectAnalysis::validate` | Pure | 28 | Low | types | Validation logic |
| `MetricInconsistency::fmt` | Pure | 15 | Low | types | Display trait |
| `FunctionVisibilityBreakdown::total` | Pure | 3 | Low | types | Aggregation |
| `FunctionVisibilityBreakdown::new` | Pure | 7 | Low | types | Constructor |
| `ModuleSplit::validate_name` | Pure | 8 | Low | types | Validation |
| `GodObjectThresholds::default` | Pure | 9 | Low | thresholds | Default config |
| `GodObjectThresholds::for_rust` | Pure | 7 | Low | thresholds | Rust config |
| `GodObjectThresholds::for_python` | Pure | 7 | Low | thresholds | Python config |
| `GodObjectThresholds::for_javascript` | Pure | 7 | Low | thresholds | JS config |
| `calculate_god_object_score` | Pure | 46 | Medium | scoring | Core scoring |
| `calculate_god_object_score_weighted` | Pure | 60 | Medium | scoring | Weighted scoring |
| `determine_confidence` | Pure | 28 | Low | classifier | Confidence calc |
| `group_methods_by_responsibility` | Orchestration | 19 | Low | classifier | Method grouping |
| `infer_responsibility_with_io_detection` | Mixed | 37 | Medium | predicates | I/O detection |
| `map_io_to_traditional_responsibility` | Pure | 8 | Low | predicates | Mapping |
| `infer_responsibility_from_call_patterns` | Pure | 28 | Medium | predicates | Pattern inference |
| `categorize_functions` | Orchestration | 12 | Low | predicates | Categorization |
| `find_dominant_category` | Pure | 6 | Low | predicates | Max finding |
| `infer_responsibility_multi_signal` | Orchestration | 25 | Medium | classifier | Multi-signal |
| `group_methods_by_responsibility_multi_signal` | Orchestration | 15 | Low | classifier | Grouping |
| `group_methods_by_responsibility_with_evidence` | Orchestration | 26 | Medium | classifier | Evidence-based |
| `group_methods_by_responsibility_with_domain_patterns` | Orchestration | 84 | High | classifier | Domain patterns |
| `infer_responsibility_from_method` | Pure | 6 | Low | predicates | Inference |
| `infer_responsibility_with_confidence` | Pure | 35 | Medium | predicates | Confidence-based |
| `normalize_category_name` | Pure | 9 | Low | predicates | Normalization |
| `recommend_module_splits` | Orchestration | 10 | Low | recommender | Split recommendation |
| `recommend_module_splits_enhanced` | Orchestration | 8 | Low | recommender | Enhanced splits |
| `recommend_module_splits_enhanced_with_evidence` | Orchestration | 86 | High | recommender | Evidence-based |
| `recommend_module_splits_with_evidence` | Orchestration | 114 | High | recommender | Full evidence |
| `count_distinct_domains` | Pure | 7 | Low | predicates | Domain counting |
| `calculate_struct_ratio` | Pure | 6 | Low | scoring | Ratio calc |
| `determine_cross_domain_severity` | Pure | 29 | Medium | classifier | Severity calc |
| `suggest_module_splits_by_domain` | Pure | 64 | Medium | recommender | Domain splits |
| `classify_struct_domain` | Pure | 23 | Low | predicates | Domain classification |
| `extract_domain_from_name` | Pure | 20 | Low | predicates | Name parsing |
| `calculate_domain_diversity_from_structs` | Orchestration | 16 | Low | scoring | Diversity calc |
| `is_reserved_keyword` | Pure | 3 | Low | predicates | Keyword check |
| `ensure_not_reserved` | Pure | 6 | Low | predicates | Name sanitization |
| `sanitize_module_name` | Pure | 16 | Low | predicates | Full sanitization |
| `ensure_unique_name` | Pure | 12 | Low | predicates | Uniqueness |
| `suggest_splits_by_struct_grouping` | Orchestration | 97 | High | recommender | Struct grouping |
| `ModuleSplit::default` | Pure | 39 | Low | types | Default constructor |
| `ModuleSplit::eq` | Pure | 20 | Low | types | Equality |
| `FunctionVisibilityBreakdown::default` | Pure | 3 | Low | types | Default constructor |

**Summary**:
- Pure Computation: 24 (55%)
- Orchestration: 16 (36%)
- Mixed: 1 (2%)
- I/O Operation: 0 (0%)

---

## Purity Analysis

### Pure Functions (53 total)

Pure functions have no side effects, are deterministic, and can be easily tested:

#### From god_object_detector.rs (29 pure):
- Data structure methods: `CallGraphAdapter::*`, constructors
- String manipulation: `sanitize_module_name`, `capitalize_first`, `extract_*`
- Pattern matching: `check_if_pure_method`, `detect_io_operations`, `is_god_object`
- Scoring: `estimate_standalone_complexity`, `count_statements`, `count_fields`
- Classification: `classify_god_object_impact`, `classify_responsibility`
- Large pure functions: `generate_recommendation` (198 lines), `generate_type_definition_example` (96 lines)

#### From god_object_analysis.rs (24 pure):
- Type methods: `GodObjectAnalysis::validate`, `ModuleSplit::*`, `FunctionVisibilityBreakdown::*`
- Thresholds: All `GodObjectThresholds::*` factory methods
- Scoring: `calculate_god_object_score*`, `calculate_struct_ratio`
- Predicates: `classify_struct_domain`, `is_reserved_keyword`, `sanitize_module_name`
- Domain analysis: `count_distinct_domains`, `extract_domain_from_name`

### I/O Operations (14 total)

Functions performing AST traversal or external state access:

#### From god_object_detector.rs (14 I/O):
- AST extraction: `extract_method_details`, `extract_struct_names`, `detect_visibility`
- Complexity estimation: `estimate_method_complexity`
- Call graph building: `build_call_graph`, `CallGraphVisitor::*` methods
- Type analysis: `analyze_type`
- Tracker queries: `FieldAccessAdapter::fields_accessed_by`, `FieldAccessAdapter::writes_to_field`

### Orchestration Functions (29 total)

Functions coordinating other operations:

#### From god_object_detector.rs (13 orchestration):
- Main pipelines: `analyze_comprehensive`, `analyze_enhanced`, `analyze_with_integrated_architecture`
- Large coordinators: `analyze_domains_and_recommend_splits` (257 lines), `try_improved_clustering` (188 lines)
- Split generation: `generate_behavioral_splits` (163 lines), `generate_type_based_splits` (126 lines)
- Validation: `validate_and_improve_splits`, `detect_anti_patterns`

#### From god_object_analysis.rs (16 orchestration):
- Grouping: `group_methods_by_responsibility*` variants
- Recommendations: `recommend_module_splits*` variants
- Multi-signal: `infer_responsibility_multi_signal`
- Domain analysis: `calculate_domain_diversity_from_structs`

### Mixed Functions (11 total) - **NEED SPLITTING**

Functions mixing pure logic with I/O or state:

#### From god_object_detector.rs (10 mixed):
1. **`classify_god_object`** (172 lines) - Pattern detection + scoring
   - Split into: pure pattern detection + pure classification logic
2. **`try_module_function_classification`** (35 lines) - Classifier usage
   - Extract: pure classification predicate
3. **`infer_responsibility_from_cluster`** (123 lines) - AST extraction + pattern matching
   - Split into: AST extraction (I/O) + pattern matching (pure)
4. **`enrich_splits_with_behavioral_analysis`** (40 lines) - Field tracking + categorization
   - Extract: pure categorization logic
5. **`analyze_module_structure_and_visibility`** (49 lines) - Visibility breakdown + structure
   - Split into: structure building (pure) + visibility analysis (I/O)
6. **`apply_domain_aware_naming_to_type_based_splits`** (26 lines) - Pattern detection + naming
   - Extract: pure naming logic
7. **`detect_domain_pattern_for_methods`** (82 lines) - Domain pattern detection
   - Split into: pattern extraction + domain classification
8. **`apply_semantic_naming`** (40 lines) - Name generation + metadata
   - Extract: pure name generation

#### From god_object_analysis.rs (1 mixed):
9. **`infer_responsibility_with_io_detection`** (37 lines) - I/O detection + inference
   - Split into: I/O detection + responsibility inference

### Line Range Summary

**Pure Core** (0 side effects):
- Types: Lines with struct/enum definitions
- Predicates: Boolean checks and classifications
- Scoring: Mathematical calculations
- Total: ~2,000 lines of pure computation

**Imperative Shell** (side effects at boundaries):
- AST traversal: Visitor implementations
- File I/O: Configuration loading
- Orchestration: Pipeline coordination
- Total: ~1,500 lines of I/O operations

**Mixed** (needs refactoring before moving):
- 11 functions totaling ~700 lines
- All can be split into pure + I/O components

---

## Module Assignment Table

| Target Module | Functions | Est. Lines | Dependencies | Risk | Notes |
|---------------|-----------|------------|--------------|------|-------|
| **types** | 21 | ~200 | None | Low | Core data structures, constructors, methods |
| **thresholds** | 5 | ~100 | types | Low | Configuration constants and factory methods |
| **predicates** | 22 | ~150 | types, thresholds | Low | Boolean checks, pattern matching, classification |
| **scoring** | 8 | ~200 | types, predicates | Low | Pure mathematical calculations |
| **classifier** | 16 | ~200 | types, predicates, scoring | Medium | Classification and grouping logic |
| **recommender** | 20 | ~250 | types, predicates, classifier | Medium | Split generation and improvement |
| **detector** | 18 | ~250 | All modules | Medium | AST traversal and orchestration |

### types.rs (~200 lines)
**Functions (21)**:
- From detector: `CallGraphAdapter` (4 methods), `FieldAccessAdapter` (3 methods), `GodObjectDetector` (3 constructors), `create_*_group` (2), `detector_name`
- From analysis: `GodObjectAnalysis::validate`, `MetricInconsistency::fmt`, `FunctionVisibilityBreakdown` (3 methods), `ModuleSplit` (3 methods)

**Dependencies**: None (foundation layer)

---

### thresholds.rs (~100 lines)
**Functions (5)**:
- `GodObjectThresholds::default`
- `GodObjectThresholds::for_rust`
- `GodObjectThresholds::for_python`
- `GodObjectThresholds::for_javascript`
- `get_thresholds_for_path` (from detector)

**Dependencies**: types

---

### predicates.rs (~150 lines)
**Functions (22)**:
- From detector: `extract_common_prefix`, `sanitize_module_name`, `check_if_pure_method`, `detect_io_operations`, `capitalize_first`, `is_god_object`, `extract_method_prefix`, `find_matching_prefix`, `extract_first_word`
- From analysis: `map_io_to_traditional_responsibility`, `infer_responsibility_from_call_patterns`, `find_dominant_category`, `infer_responsibility_from_method`, `infer_responsibility_with_confidence`, `normalize_category_name`, `count_distinct_domains`, `classify_struct_domain`, `extract_domain_from_name`, `is_reserved_keyword`, `ensure_not_reserved`, `sanitize_module_name`, `ensure_unique_name`
- Split from mixed: Pure parts of `infer_responsibility_with_io_detection`, `categorize_functions`

**Dependencies**: types, thresholds

---

### scoring.rs (~200 lines)
**Functions (8)**:
- From detector: `estimate_standalone_complexity`, `count_statements`, `count_fields`
- From analysis: `calculate_god_object_score`, `calculate_god_object_score_weighted`, `calculate_struct_ratio`, `calculate_domain_diversity_from_structs`
- From metrics.rs: May import helper functions

**Dependencies**: types, predicates

---

### classifier.rs (~200 lines)
**Functions (16)**:
- From detector: `classify_god_object_impact`, `determine_god_object_type`, `group_methods_by_prefix`, `infer_responsibility_name`, `classify_responsibility`, `estimate_maintainability_impact`
- From detector (split from mixed): Pure parts of `classify_god_object`, `try_module_function_classification`, `infer_responsibility_from_cluster`, `detect_domain_pattern_for_methods`
- From analysis: `determine_confidence`, `group_methods_by_responsibility`, `infer_responsibility_multi_signal`, `group_methods_by_responsibility_multi_signal`, `group_methods_by_responsibility_with_evidence`, `group_methods_by_responsibility_with_domain_patterns`, `determine_cross_domain_severity`

**Dependencies**: types, predicates, scoring

---

### recommender.rs (~250 lines)
**Functions (20)**:
- From detector: `generate_recommendation`, `generate_type_definition_example`, `suggest_responsibility_split`, `validate_and_improve_splits`, `apply_split_quality_limiting`
- From detector (orchestration): `analyze_domains_and_recommend_splits`, `generate_behavioral_splits`, `generate_type_based_splits`, `generate_pipeline_based_splits`, `apply_semantic_naming_to_splits`
- From detector (split from mixed): `enrich_splits_with_behavioral_analysis`, `apply_domain_aware_naming_to_type_based_splits`, `apply_semantic_naming`
- From analysis: `recommend_module_splits`, `recommend_module_splits_enhanced`, `recommend_module_splits_enhanced_with_evidence`, `recommend_module_splits_with_evidence`, `suggest_module_splits_by_domain`, `suggest_splits_by_struct_grouping`

**Dependencies**: types, predicates, classifier

---

### detector.rs (~250 lines)
**Functions (18)**:
- Main pipelines: `analyze_enhanced`, `analyze_comprehensive`, `analyze_with_integrated_architecture`
- AST operations: `extract_method_details`, `extract_struct_names`, `detect_visibility`, `estimate_method_complexity`, `build_call_graph`, `analyze_type`, `extract_field_names`
- Visitor methods: `CallGraphVisitor::visit_*` (4 methods)
- Orchestration: `try_improved_clustering`, `detect_anti_patterns`
- Split from mixed: I/O parts of `analyze_module_structure_and_visibility`

**Dependencies**: All modules (orchestration layer)

---

## Dependency Graph

### Current Dependencies (Problematic)

```
god_object_detector.rs (4362 lines)
    ↓ (imports)
god_object_analysis.rs (3304 lines)
    ↓ (imports)
god_object/metrics.rs (349 lines)
    ↓ (imports)
god_object/ast_visitor.rs (365 lines)

[Large circular coupling between detector and analysis]
```

**Problems**:
- Detector and analysis are tightly coupled
- No clear module boundaries
- Difficult to test in isolation
- Large compilation units

---

### Proposed Dependencies (Acyclic)

```
types.rs (foundation)
  ↑
  ├── thresholds.rs
  ├── predicates.rs
  │     ↑
  │     └── scoring.rs
  │           ↑
  │           ├── classifier.rs
  │           │     ↑
  │           │     └── recommender.rs
  │           └─────────↑
  │                     │
  └─────────────────────┴── detector.rs (orchestration)

Existing modules (keep as-is):
- ast_visitor.rs → types.rs
- metrics.rs → types.rs, scoring.rs
```

**Dependency Rules**:
1. **types** has no dependencies (foundation)
2. **thresholds** depends only on types
3. **predicates** depends on types, thresholds
4. **scoring** depends on types, predicates
5. **classifier** depends on types, predicates, scoring
6. **recommender** depends on types, predicates, classifier
7. **detector** depends on all modules (orchestration layer)

**Verification**: ✓ No circular dependencies

---

## Benchmark Baselines

Benchmarks created in `benches/god_object_bench.rs` to measure:
- God object score calculation
- Confidence determination
- Method grouping by responsibility
- Module split recommendation
- Full analysis pipeline
- Enhanced analysis pipeline

**Baseline Results** (to be captured after benchmark run):
- `calculate_god_object_score`: [PENDING]
- `determine_confidence`: [PENDING]
- `group_methods_by_responsibility`: [PENDING]
- `recommend_module_splits`: [PENDING]
- `full_analysis_pipeline`: [PENDING]
- `enhanced_analysis_pipeline`: [PENDING]

**Performance Goals**:
- No regression > 5% on any benchmark
- Pure function benchmarks should improve (easier inlining)
- Full pipeline may have slight overhead from module boundaries

---

## Risk Assessment

### High Risk Areas

1. **Large Mixed Functions** (11 functions)
   - `classify_god_object` (172 lines) - Core classification logic
   - `analyze_domains_and_recommend_splits` (257 lines) - Main coordinator
   - `infer_responsibility_from_cluster` (123 lines) - Pattern detection
   - **Mitigation**: Split incrementally, add tests for each piece

2. **Public API Surface** (42 exports)
   - Tests depend on specific exports and signatures
   - **Mitigation**: Re-export everything from mod.rs, maintain signatures

3. **Performance Impact**
   - Module boundaries may add overhead
   - **Mitigation**: Benchmarks before/after, inline hints, LTO

### Medium Risk Areas

1. **Orchestration Functions** (29 functions)
   - Complex coordination logic
   - **Mitigation**: Keep orchestration in detector.rs, extract pure parts

2. **Test Compatibility**
   - 6 test files with complex dependencies
   - **Mitigation**: Run tests after each phase, fix imports incrementally

### Low Risk Areas

1. **Pure Functions** (53 functions)
   - Easy to move, no side effects
   - **Mitigation**: None needed, straightforward refactoring

2. **Existing Modules**
   - ast_visitor.rs and metrics.rs are well-designed
   - **Mitigation**: Keep as-is, no changes

### Circular Dependency Concerns

**Status**: ✓ None identified

The proposed dependency graph is acyclic:
- Foundation layer (types, thresholds)
- Pure logic layer (predicates, scoring)
- Domain layer (classifier, recommender)
- Orchestration layer (detector)

No module needs to import from a layer above it.

### Performance Concerns

**Critical Paths**:
1. Full analysis pipeline (benchmarked)
2. Score calculation (benchmarked)
3. Method grouping (benchmarked)

**Expected Impact**:
- Pure functions: Slight improvement (better inlining)
- Orchestration: Slight overhead (module boundaries)
- Overall: < 5% difference expected

**Mitigation**:
- Use `#[inline]` for small pure functions
- Enable LTO in release builds
- Profile after refactoring

### Test Compatibility Concerns

**6 Test Files**:
- `god_object_confidence_classification_test.rs`
- `god_object_config_rs_test.rs`
- `god_object_detection_test.rs`
- `god_object_metrics_test.rs`
- `god_object_struct_recommendations.rs`
- `god_object_type_based_clustering_test.rs`

**Compatibility Strategy**:
1. Maintain all 42 public exports in mod.rs
2. Re-export from new modules
3. Keep function signatures identical
4. Run tests after each phase
5. Update imports only if necessary

**Risk Level**: Medium
- Tests are well-structured
- Clear API dependencies
- Incremental validation possible

---

## Phase 2 Readiness Checklist

- [x] All functions classified (110 total across both files)
- [x] Module boundaries clear (7 modules defined)
- [x] No circular dependencies (acyclic graph verified)
- [x] Benchmarks established (god_object_bench.rs created)
- [x] Public API documented (42 exports identified)
- [x] Existing modules analyzed (ast_visitor.rs, metrics.rs kept as-is)
- [x] Mixed functions identified (11 need splitting)
- [x] Test dependencies mapped (6 test files analyzed)
- [x] Risk assessment complete (High/Medium/Low areas identified)
- [x] Purity analysis complete (Pure/I/O/Orchestration/Mixed categorized)

**Status**: ✅ Ready for Phase 2 (Extract Types & Thresholds)

---

## Next Steps - Phase 2 Preparation

### Phase 2: Extract Types & Thresholds (Spec 181b)

**Objectives**:
1. Create `src/organization/god_object/types.rs`
   - Move 21 type definitions and constructors
   - Extract from both detector and analysis files
   - ~200 lines total

2. Create `src/organization/god_object/thresholds.rs`
   - Move 5 threshold functions
   - Extract from analysis file
   - ~100 lines total

3. Update existing files
   - Remove moved code
   - Add imports from new modules
   - Update mod.rs exports

4. Validate
   - Run all tests
   - Run benchmarks
   - Verify no regressions

**Estimated Time**: 4 hours

**Success Criteria**:
- All tests pass
- No benchmark regressions > 5%
- Clean compilation
- Reduced line counts in detector and analysis files

---

## References

- **Parent Spec**: `specs/181-split-god-object-detector-module.md`
- **Stillwater Philosophy**: `../stillwater/PHILOSOPHY.md`
- **Project Guidelines**: `CLAUDE.md`
- **Benchmarks**: `benches/god_object_bench.rs`
- **Test Files**: `tests/god_object_*.rs` (6 files)

---

## Appendix: Function-to-Module Mapping

### Complete Mapping (110 functions)

<details>
<summary>Click to expand complete function mapping</summary>

#### types.rs (21 functions)
1. `CallGraphAdapter::from_adjacency_matrix` (detector)
2. `CallGraphAdapter::call_count` (detector)
3. `CallGraphAdapter::callees` (detector)
4. `CallGraphAdapter::callers` (detector)
5. `FieldAccessAdapter::new` (detector)
6. `FieldAccessAdapter::fields_accessed_by` (detector)
7. `FieldAccessAdapter::writes_to_field` (detector)
8. `GodObjectDetector::default` (detector)
9. `GodObjectDetector::new` (detector)
10. `GodObjectDetector::with_source_content` (detector)
11. `create_responsibility_group` (detector)
12. `create_default_responsibility_group` (detector)
13. `detector_name` (detector)
14. `GodObjectAnalysis::validate` (analysis)
15. `MetricInconsistency::fmt` (analysis)
16. `FunctionVisibilityBreakdown::total` (analysis)
17. `FunctionVisibilityBreakdown::new` (analysis)
18. `FunctionVisibilityBreakdown::default` (analysis)
19. `ModuleSplit::validate_name` (analysis)
20. `ModuleSplit::default` (analysis)
21. `ModuleSplit::eq` (analysis)

#### thresholds.rs (5 functions)
1. `GodObjectThresholds::default` (analysis)
2. `GodObjectThresholds::for_rust` (analysis)
3. `GodObjectThresholds::for_python` (analysis)
4. `GodObjectThresholds::for_javascript` (analysis)
5. `get_thresholds_for_path` (detector)

#### predicates.rs (22 functions)
1. `extract_common_prefix` (detector)
2. `sanitize_module_name` (detector)
3. `check_if_pure_method` (detector)
4. `detect_io_operations` (detector)
5. `capitalize_first` (detector)
6. `is_god_object` (detector)
7. `extract_method_prefix` (detector)
8. `find_matching_prefix` (detector)
9. `extract_first_word` (detector)
10. `map_io_to_traditional_responsibility` (analysis)
11. `infer_responsibility_from_call_patterns` (analysis)
12. `find_dominant_category` (analysis)
13. `categorize_functions` (analysis)
14. `infer_responsibility_from_method` (analysis)
15. `infer_responsibility_with_confidence` (analysis)
16. `normalize_category_name` (analysis)
17. `count_distinct_domains` (analysis)
18. `classify_struct_domain` (analysis)
19. `extract_domain_from_name` (analysis)
20. `is_reserved_keyword` (analysis)
21. `ensure_not_reserved` (analysis)
22. `ensure_unique_name` (analysis)

#### scoring.rs (8 functions)
1. `estimate_standalone_complexity` (detector)
2. `count_statements` (detector)
3. `count_fields` (detector)
4. `calculate_god_object_score` (analysis)
5. `calculate_god_object_score_weighted` (analysis)
6. `calculate_struct_ratio` (analysis)
7. `calculate_domain_diversity_from_structs` (analysis)
8. `estimate_method_complexity` (detector) - Note: I/O operation

#### classifier.rs (16 functions)
1. `classify_god_object_impact` (detector)
2. `determine_god_object_type` (detector)
3. `group_methods_by_prefix` (detector)
4. `infer_responsibility_name` (detector)
5. `classify_responsibility` (detector)
6. `estimate_maintainability_impact` (detector)
7. `classify_god_object` (detector) - Mixed, needs splitting
8. `try_module_function_classification` (detector) - Mixed
9. `infer_responsibility_from_cluster` (detector) - Mixed
10. `detect_domain_pattern_for_methods` (detector) - Mixed
11. `determine_confidence` (analysis)
12. `group_methods_by_responsibility` (analysis)
13. `infer_responsibility_multi_signal` (analysis)
14. `group_methods_by_responsibility_multi_signal` (analysis)
15. `group_methods_by_responsibility_with_evidence` (analysis)
16. `group_methods_by_responsibility_with_domain_patterns` (analysis)
17. `determine_cross_domain_severity` (analysis)

#### recommender.rs (20 functions)
1. `generate_recommendation` (detector)
2. `generate_type_definition_example` (detector)
3. `suggest_responsibility_split` (detector)
4. `validate_and_improve_splits` (detector)
5. `apply_split_quality_limiting` (detector)
6. `analyze_domains_and_recommend_splits` (detector) - Orchestration
7. `generate_behavioral_splits` (detector) - Orchestration
8. `generate_type_based_splits` (detector) - Orchestration
9. `generate_pipeline_based_splits` (detector) - Orchestration
10. `apply_semantic_naming_to_splits` (detector) - Orchestration
11. `enrich_splits_with_behavioral_analysis` (detector) - Mixed
12. `apply_domain_aware_naming_to_type_based_splits` (detector) - Mixed
13. `apply_semantic_naming` (detector) - Mixed
14. `recommend_module_splits` (analysis)
15. `recommend_module_splits_enhanced` (analysis)
16. `recommend_module_splits_enhanced_with_evidence` (analysis)
17. `recommend_module_splits_with_evidence` (analysis)
18. `suggest_module_splits_by_domain` (analysis)
19. `suggest_splits_by_struct_grouping` (analysis)
20. `infer_responsibility_with_io_detection` (analysis) - Mixed

#### detector.rs (18 functions)
1. `analyze_enhanced` (detector) - Orchestration
2. `analyze_comprehensive` (detector) - Orchestration
3. `analyze_with_integrated_architecture` (detector) - Orchestration
4. `extract_method_details` (detector) - I/O
5. `extract_struct_names` (detector) - I/O
6. `detect_visibility` (detector) - I/O
7. `build_call_graph` (detector) - I/O
8. `CallGraphVisitor::visit_impl_item_fn` (detector) - I/O
9. `CallGraphVisitor::visit_item_fn` (detector) - I/O
10. `CallGraphVisitor::visit_expr_call` (detector) - I/O
11. `CallGraphVisitor::visit_expr_method_call` (detector) - I/O
12. `analyze_type` (detector) - I/O
13. `extract_field_names` (detector) - Pure
14. `try_improved_clustering` (detector) - Orchestration
15. `detect_anti_patterns` (detector) - Orchestration
16. `analyze_module_structure_and_visibility` (detector) - Mixed

</details>

---

**Document Version**: 1.0
**Last Updated**: 2025-12-03
**Next Review**: After Phase 2 completion
