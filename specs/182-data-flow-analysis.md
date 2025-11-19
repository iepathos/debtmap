# Spec 182: Data Flow Analysis for Pipeline Detection

**Status**: Draft
**Dependencies**: [181]
**Related**: [178, 179, 180]

## Problem

Functional programming emphasizes data transformation pipelines (Input → Transform → Output), but current behavioral decomposition doesn't detect or recommend pipeline-based module organization. Methods that form natural transformation chains are scattered across technical groupings rather than organized as cohesive pipeline stages.

**Example of missed pipeline in god_object_analysis.rs**:
```
Current behavioral split:
- calculate.rs (calculate_god_object_score, calculate_domain_diversity)
- recommend.rs (recommend_module_splits, suggest_splits_by_domain)
- infer.rs (determine_confidence, determine_cross_domain_severity)

Actual data flow pipeline:
StructData → Detection → Metrics → Diversity → Confidence → Recommendation
```

## Objective

Implement data flow analysis that detects transformation pipelines and recommends modules organized by pipeline stages, each owning its stage's data type and transformation logic.

## Requirements

### 0. ModuleSplit Extensions

Add new fields to `ModuleSplit` struct for data flow analysis:

```rust
// src/organization/god_object_analysis.rs

pub struct ModuleSplit {
    // ... existing fields ...

    /// Pipeline stage type (Source, Transform, Validate, Aggregate, Sink)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_flow_stage: Option<StageType>,

    /// Position in pipeline (0 = input, N = output)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline_position: Option<usize>,

    /// Input types consumed by this module
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_types: Vec<String>,

    /// Output types produced by this module
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_types: Vec<String>,
}

/// Stage type in data transformation pipeline
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum StageType {
    Source,
    Transform,
    Validate,
    Aggregate,
    Sink,
}
```

### 1. Type Flow Graph Construction

Build a directed graph showing how data types flow through method calls:

```rust
// src/organization/data_flow_analyzer.rs

use std::collections::{HashMap, HashSet};
use crate::organization::type_based_clustering::{MethodSignature, TypeInfo};

pub struct DataFlowAnalyzer;

#[derive(Clone, Debug)]
pub struct TypeFlowGraph {
    /// Maps type name to methods that produce it (outputs)
    pub producers: HashMap<String, Vec<String>>,

    /// Maps type name to methods that consume it (inputs)
    pub consumers: HashMap<String, Vec<String>>,

    /// Directed edges: (from_type, to_type, via_method)
    pub edges: Vec<TypeFlowEdge>,
}

#[derive(Clone, Debug)]
pub struct TypeFlowEdge {
    pub from_type: String,
    pub to_type: String,
    pub via_method: String,
    pub transformation_type: TransformationType,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TransformationType {
    /// A → B (pure transformation)
    Direct,

    /// (A, B) → C (aggregation)
    Aggregation,

    /// A → (B, C) (decomposition)
    Decomposition,

    /// A → Result<B> (validation/enrichment)
    Enrichment,

    /// A → Vec<B> (expansion)
    Expansion,
}

impl DataFlowAnalyzer {
    pub fn build_type_flow_graph(
        &self,
        signatures: &[MethodSignature],
        call_graph: &HashMap<String, Vec<String>>,
    ) -> TypeFlowGraph {
        let mut graph = TypeFlowGraph {
            producers: HashMap::new(),
            consumers: HashMap::new(),
            edges: Vec::new(),
        };

        // Build producers and consumers
        for sig in signatures {
            // Return type = produces
            if let Some(ret_type) = &sig.return_type {
                graph.producers
                    .entry(ret_type.name.clone())
                    .or_default()
                    .push(sig.name.clone());
            }

            // Parameter types = consumes
            for param in &sig.param_types {
                graph.consumers
                    .entry(param.name.clone())
                    .or_default()
                    .push(sig.name.clone());
            }
        }

        // Build edges (type transformations)
        for sig in signatures {
            if let Some(ret_type) = &sig.return_type {
                for param in &sig.param_types {
                    let transformation_type = self.classify_transformation(
                        &param.name,
                        &ret_type.name,
                        &sig.param_types,
                        ret_type,
                    );

                    graph.edges.push(TypeFlowEdge {
                        from_type: param.name.clone(),
                        to_type: ret_type.name.clone(),
                        via_method: sig.name.clone(),
                        transformation_type,
                    });
                }
            }
        }

        graph
    }

    fn classify_transformation(
        &self,
        from_type: &str,
        to_type: &str,
        all_params: &[TypeInfo],
        ret_type: &TypeInfo,
    ) -> TransformationType {
        // Multiple inputs → single output = Aggregation
        if all_params.len() > 1 {
            return TransformationType::Aggregation;
        }

        // Result wrapper = Enrichment
        if to_type.starts_with("Result") || to_type.starts_with("Option") {
            return TransformationType::Enrichment;
        }

        // Vec output = Expansion
        if to_type.starts_with("Vec") {
            return TransformationType::Expansion;
        }

        // Tuple output = Decomposition
        if to_type.contains(',') || to_type.starts_with('(') {
            return TransformationType::Decomposition;
        }

        // Default: Direct transformation
        TransformationType::Direct
    }
}
```

### 2. Pipeline Stage Detection

Identify cohesive pipeline stages using graph traversal:

```rust
#[derive(Clone, Debug)]
pub struct PipelineStage {
    pub stage_name: String,
    pub input_types: Vec<String>,
    pub output_types: Vec<String>,
    pub methods: Vec<String>,
    pub stage_type: StageType,
    pub depth: usize, // Position in pipeline (0 = input, N = output)
}

#[derive(Clone, Debug, PartialEq)]
pub enum StageType {
    /// Initial data acquisition/parsing
    Source,

    /// Pure transformation
    Transform,

    /// Validation/enrichment
    Validate,

    /// Aggregation/summarization
    Aggregate,

    /// Final output generation
    Sink,
}

impl DataFlowAnalyzer {
    pub fn detect_pipeline_stages(
        &self,
        graph: &TypeFlowGraph,
        signatures: &[MethodSignature],
    ) -> Result<Vec<PipelineStage>, String> {
        // 1. Find source nodes (no inputs, only outputs)
        let source_types = self.find_source_types(graph);

        // 2. Find sink nodes (no outputs, only inputs)
        let sink_types = self.find_sink_types(graph);

        // 3. Compute depth for each type (distance from source)
        let type_depths = self.compute_type_depths(graph, &source_types)?;

        // 4. Group methods by depth and transformation type
        let mut stages = Vec::new();
        let mut depth_groups: HashMap<usize, Vec<String>> = HashMap::new();

        for sig in signatures {
            if let Some(ret_type) = &sig.return_type {
                let depth = type_depths.get(&ret_type.name).unwrap_or(&0);
                depth_groups.entry(*depth).or_default().push(sig.name.clone());
            }
        }

        // 5. Create stage for each depth level
        for (depth, methods) in depth_groups {
            let stage = self.create_stage_from_methods(
                depth,
                &methods,
                signatures,
                graph,
                &source_types,
                &sink_types,
            );
            stages.push(stage);
        }

        stages.sort_by_key(|s| s.depth);
        Ok(stages)
    }

    fn find_source_types(&self, graph: &TypeFlowGraph) -> HashSet<String> {
        graph.producers.keys()
            .filter(|type_name| !graph.consumers.contains_key(*type_name))
            .cloned()
            .collect()
    }

    fn find_sink_types(&self, graph: &TypeFlowGraph) -> HashSet<String> {
        graph.consumers.keys()
            .filter(|type_name| !graph.producers.contains_key(*type_name))
            .cloned()
            .collect()
    }

    fn compute_type_depths(
        &self,
        graph: &TypeFlowGraph,
        sources: &HashSet<String>,
    ) -> Result<HashMap<String, usize>, String> {
        let mut depths = HashMap::new();
        let mut queue: Vec<(String, usize)> = sources.iter()
            .map(|s| (s.clone(), 0))
            .collect();
        let mut visiting = HashSet::new();

        while let Some((type_name, depth)) = queue.pop() {
            // Already processed this type
            if depths.contains_key(&type_name) {
                continue;
            }

            // Cycle detection: if we're visiting this type again before finishing
            if visiting.contains(&type_name) {
                return Err(format!("Cycle detected involving type: {}", type_name));
            }

            visiting.insert(type_name.clone());
            depths.insert(type_name.clone(), depth);

            // Find all types produced by consuming this type
            for edge in &graph.edges {
                if edge.from_type == type_name {
                    queue.push((edge.to_type.clone(), depth + 1));
                }
            }

            visiting.remove(&type_name);
        }

        Ok(depths)
    }

    fn create_stage_from_methods(
        &self,
        depth: usize,
        methods: &[String],
        signatures: &[MethodSignature],
        graph: &TypeFlowGraph,
        sources: &HashSet<String>,
        sinks: &HashSet<String>,
    ) -> PipelineStage {
        let mut input_types = HashSet::new();
        let mut output_types = HashSet::new();

        for method_name in methods {
            if let Some(sig) = signatures.iter().find(|s| s.name == *method_name) {
                for param in &sig.param_types {
                    input_types.insert(param.name.clone());
                }
                if let Some(ret) = &sig.return_type {
                    output_types.insert(ret.name.clone());
                }
            }
        }

        let stage_type = if depth == 0 {
            StageType::Source
        } else if output_types.iter().any(|t| sinks.contains(t)) {
            StageType::Sink
        } else {
            self.infer_stage_type(methods, signatures, graph)
        };

        let stage_name = self.suggest_stage_name(&stage_type, &output_types);

        PipelineStage {
            stage_name,
            input_types: input_types.into_iter().collect(),
            output_types: output_types.into_iter().collect(),
            methods: methods.to_vec(),
            stage_type,
            depth,
        }
    }

    fn infer_stage_type(
        &self,
        methods: &[String],
        signatures: &[MethodSignature],
        graph: &TypeFlowGraph,
    ) -> StageType {
        let transformations: Vec<_> = graph.edges.iter()
            .filter(|e| methods.contains(&e.via_method))
            .map(|e| &e.transformation_type)
            .collect();

        // Majority vote
        let validate_count = transformations.iter()
            .filter(|t| **t == TransformationType::Enrichment)
            .count();
        let aggregate_count = transformations.iter()
            .filter(|t| **t == TransformationType::Aggregation)
            .count();

        if validate_count > transformations.len() / 2 {
            StageType::Validate
        } else if aggregate_count > transformations.len() / 2 {
            StageType::Aggregate
        } else {
            StageType::Transform
        }
    }

    fn suggest_stage_name(&self, stage_type: &StageType, output_types: &HashSet<String>) -> String {
        // Filter out generic/primitive types to find domain types
        let domain_types: Vec<_> = output_types.iter()
            .filter(|t| !self.is_generic_type(t))
            .collect();

        // Choose most specific type (prefer longer, domain-specific names)
        let primary_type = domain_types.iter()
            .max_by_key(|t| {
                let name = t.as_str();
                // Prefer types ending in domain suffixes
                let domain_bonus = if name.ends_with("Analysis")
                    || name.ends_with("Metrics")
                    || name.ends_with("Result")
                    || name.ends_with("Data") {
                    100
                } else {
                    0
                };
                name.len() + domain_bonus
            })
            .map(|s| s.as_str())
            .unwrap_or("Unknown");

        // Convert to snake_case and append stage suffix
        let snake_case = self.to_snake_case(primary_type);
        match stage_type {
            StageType::Source => snake_case,  // Source types don't need suffix
            StageType::Transform => format!("{}_transform", snake_case),
            StageType::Validate => format!("{}_validation", snake_case),
            StageType::Aggregate => format!("{}_aggregation", snake_case),
            StageType::Sink => format!("{}_output", snake_case),
        }
    }

    fn is_generic_type(&self, type_name: &str) -> bool {
        matches!(
            type_name,
            "String" | "str" | "Vec" | "Option" | "Result" |
            "HashMap" | "HashSet" | "BTreeMap" | "BTreeSet" |
            "usize" | "isize" | "u32" | "i32" | "u64" | "i64" |
            "f32" | "f64" | "bool" | "char"
        )
    }

    fn to_snake_case(&self, s: &str) -> String {
        let mut result = String::new();
        for (i, ch) in s.chars().enumerate() {
            if ch.is_uppercase() && i > 0 {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
        }
        result
    }
}
```

### 3. Pipeline Recommendation Generator

Convert pipeline stages into module split recommendations:

```rust
use crate::organization::god_object_analysis::ModuleSplit;

impl DataFlowAnalyzer {
    pub fn generate_pipeline_recommendations(
        &self,
        stages: &[PipelineStage],
        base_name: &str,
    ) -> Vec<ModuleSplit> {
        stages.iter().map(|stage| {
            let responsibility = self.describe_stage_responsibility(stage);
            let module_name = format!("{}/{}", base_name, stage.stage_name);

            ModuleSplit {
                suggested_name: module_name,
                responsibility,
                methods_to_move: stage.methods.clone(),
                data_flow_stage: Some(stage.stage_type.clone()),
                pipeline_position: Some(stage.depth),
                input_types: stage.input_types.clone(),
                output_types: stage.output_types.clone(),
                ..Default::default()
            }
        }).collect()
    }

    fn describe_stage_responsibility(&self, stage: &PipelineStage) -> String {
        let inputs = stage.input_types.join(", ");
        let outputs = stage.output_types.join(", ");

        match stage.stage_type {
            StageType::Source => {
                format!("Source stage: Produce {} for downstream processing", outputs)
            }
            StageType::Transform => {
                format!("Transform {} into {}", inputs, outputs)
            }
            StageType::Validate => {
                format!("Validate and enrich {} into {}", inputs, outputs)
            }
            StageType::Aggregate => {
                format!("Aggregate {} into {}", inputs, outputs)
            }
            StageType::Sink => {
                format!("Sink stage: Consume {} for final output", inputs)
            }
        }
    }
}
```

### 4. Integration with God Object Detector

```rust
// src/organization/god_object_detector.rs

fn generate_pipeline_based_splits(
    all_methods: &[String],
    call_graph: &HashMap<String, Vec<String>>,
    ast: &syn::File,
    base_name: &str,
) -> Vec<ModuleSplit> {
    use crate::organization::type_based_clustering::TypeSignatureAnalyzer;
    use crate::organization::data_flow_analyzer::DataFlowAnalyzer;

    // Extract type signatures
    let type_analyzer = TypeSignatureAnalyzer;
    let signatures = extract_method_signatures(ast, &type_analyzer);

    // Build type flow graph
    let flow_analyzer = DataFlowAnalyzer;
    let flow_graph = flow_analyzer.build_type_flow_graph(&signatures, call_graph);

    // Detect pipeline stages
    let stages = flow_analyzer.detect_pipeline_stages(&flow_graph, &signatures);

    // Generate recommendations
    flow_analyzer.generate_pipeline_recommendations(&stages, base_name)
}
```

## Enhanced Output Format

```
#4 SCORE: 62.0 [CRITICAL] god_object_analysis.rs (27 methods, 15 structs)

Pipeline-Based Split Recommendation:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Data Flow Pipeline (5 stages):
  StructData → Detection → Metrics → Diversity → Recommendation

Stage 0: detection.rs [Source Stage]
  Responsibility: Produce GodObjectAnalysis for downstream processing
  Input Types: StructData, syn::File
  Output Types: GodObjectAnalysis
  Methods (5):
    - analyze_god_object_patterns
    - detect_responsibility_violations
    - identify_cross_domain_coupling
    + Define: pub struct GodObjectAnalysis { ... }

Stage 1: metrics.rs [Transform Stage]
  Responsibility: Transform GodObjectAnalysis into GodObjectMetrics
  Input Types: GodObjectAnalysis
  Output Types: GodObjectMetrics
  Methods (4):
    - calculate_god_object_score
    - calculate_god_object_score_weighted
    - calculate_struct_ratio
    + Define: pub struct GodObjectMetrics { ... }

Stage 2: diversity.rs [Validate Stage]
  Responsibility: Validate and enrich GodObjectMetrics into DiversityScore
  Input Types: GodObjectMetrics, StructWithMethods
  Output Types: DiversityScore, DomainDiversityMetrics
  Methods (3):
    - calculate_domain_diversity_from_structs
    - count_distinct_domains
    + Define: pub struct DiversityScore { ... }

Stage 3: confidence.rs [Transform Stage]
  Responsibility: Transform DiversityScore into GodObjectConfidence
  Input Types: DiversityScore, GodObjectMetrics
  Output Types: GodObjectConfidence
  Methods (2):
    - determine_confidence
    - determine_cross_domain_severity
    + Define: pub struct GodObjectConfidence { ... }

Stage 4: recommendation.rs [Sink Stage]
  Responsibility: Consume GodObjectConfidence for final output
  Input Types: GodObjectConfidence, DomainDiversityMetrics
  Output Types: Vec<ModuleSplit>
  Methods (7):
    - recommend_module_splits
    - recommend_module_splits_enhanced
    - suggest_module_splits_by_domain
    - suggest_splits_by_struct_grouping
    + Define: pub struct SplitRecommendation { ... }

Functional Architecture Benefits:
  ✓ Clear data transformation pipeline
  ✓ Each stage owns its output type
  ✓ Testable: mock inputs, verify outputs
  ✓ Composable: can reorder or skip stages
  ✓ No utilities modules needed
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_flow_graph_construction() {
        let signatures = vec![
            MethodSignature {
                name: "parse".to_string(),
                param_types: vec![TypeInfo { name: "String".to_string(), .. }],
                return_type: Some(TypeInfo { name: "ParsedData".to_string(), .. }),
                self_type: None,
            },
            MethodSignature {
                name: "validate".to_string(),
                param_types: vec![TypeInfo { name: "ParsedData".to_string(), .. }],
                return_type: Some(TypeInfo { name: "Result<ValidData>".to_string(), .. }),
                self_type: None,
            },
        ];

        let analyzer = DataFlowAnalyzer;
        let graph = analyzer.build_type_flow_graph(&signatures, &HashMap::new());

        assert_eq!(graph.producers.get("ParsedData").unwrap(), &vec!["parse"]);
        assert_eq!(graph.consumers.get("ParsedData").unwrap(), &vec!["validate"]);
        assert_eq!(graph.edges.len(), 2);
    }

    #[test]
    fn test_pipeline_stage_detection() {
        // Build graph: String → ParsedData → Result<ValidData>
        let signatures = vec![
            MethodSignature {
                name: "parse".to_string(),
                param_types: vec![TypeInfo { name: "String".to_string(), .. }],
                return_type: Some(TypeInfo { name: "ParsedData".to_string(), .. }),
                self_type: None,
            },
            MethodSignature {
                name: "validate".to_string(),
                param_types: vec![TypeInfo { name: "ParsedData".to_string(), .. }],
                return_type: Some(TypeInfo { name: "Result<ValidData>".to_string(), .. }),
                self_type: None,
            },
        ];

        let analyzer = DataFlowAnalyzer;
        let graph = analyzer.build_type_flow_graph(&signatures, &HashMap::new());
        let stages = analyzer.detect_pipeline_stages(&graph, &signatures).unwrap();

        assert_eq!(stages.len(), 2);
        assert_eq!(stages[0].depth, 0);
        assert_eq!(stages[0].stage_type, StageType::Source);
        assert_eq!(stages[1].depth, 1);
        assert_eq!(stages[1].stage_type, StageType::Validate);
    }

    #[test]
    fn test_transformation_classification() {
        let analyzer = DataFlowAnalyzer;

        // Enrichment (Result wrapper)
        assert_eq!(
            analyzer.classify_transformation("Data", "Result<ValidData>", &[], &TypeInfo::default()),
            TransformationType::Enrichment
        );

        // Expansion (Vec output)
        assert_eq!(
            analyzer.classify_transformation("Item", "Vec<ProcessedItem>", &[], &TypeInfo::default()),
            TransformationType::Expansion
        );
    }
}
```

### Integration Tests

```rust
// tests/data_flow_analysis_integration.rs

#[test]
fn test_god_object_analysis_pipeline_detection() {
    let code = r#"
        impl GodObjectAnalyzer {
            fn parse(&self, code: String) -> StructData { todo!() }
            fn analyze(&self, data: StructData) -> GodObjectAnalysis { todo!() }
            fn calculate_metrics(&self, analysis: GodObjectAnalysis) -> GodObjectMetrics { todo!() }
            fn recommend(&self, metrics: GodObjectMetrics) -> Vec<ModuleSplit> { todo!() }
        }
    "#;

    let ast = syn::parse_file(code).unwrap();
    let splits = generate_pipeline_based_splits(&[], &HashMap::new(), &ast, "god_object_analysis");

    assert_eq!(splits.len(), 4);
    assert!(splits.iter().any(|s| s.suggested_name.contains("structdata")));
    assert!(splits.iter().any(|s| s.suggested_name.contains("godobjectanalysis")));
    assert!(splits.iter().any(|s| s.suggested_name.contains("metrics")));
    assert!(splits.iter().any(|s| s.suggested_name.contains("recommendation")));
}
```

## Dependencies

- **Spec 181**: Type signature extraction for building flow graph
- **Spec 178**: Call graph infrastructure for method relationships
- Existing `ModuleSplit` structure for output format

## Migration Path

1. **Phase 1**: Implement type flow graph construction
2. **Phase 2**: Add pipeline stage detection algorithm
3. **Phase 3**: Integrate with god object detector output
4. **Phase 4**: Add pipeline visualization to formatted output
5. **Phase 5**: Validate on debtmap's own codebase (god_object_analysis.rs, formatter.rs)

## Success Criteria

- Detects 3+ stage pipelines in god_object_analysis.rs
- Recommends modules organized by transformation stages
- No utilities modules in pipeline-based recommendations
- Each stage clearly shows Input → Output types
- Output describes functional architecture benefits
