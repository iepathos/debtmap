//! Data flow analysis for detecting transformation pipelines.
//!
//! This module analyzes how data types flow through method calls to detect
//! functional transformation pipelines (Input → Transform → Output).

use crate::organization::god_object::types::{ModuleSplit, StageType};
use crate::organization::type_based_clustering::{MethodSignature, TypeInfo};
use std::collections::{HashMap, HashSet, VecDeque};

/// Analyzes data flow patterns in god objects to detect pipeline structures
pub struct DataFlowAnalyzer;

/// Directed graph showing how types flow through methods
#[derive(Clone, Debug)]
pub struct TypeFlowGraph {
    /// Maps type name to methods that produce it (outputs)
    pub producers: HashMap<String, Vec<String>>,
    /// Maps type name to methods that consume it (inputs)
    pub consumers: HashMap<String, Vec<String>>,
    /// Directed edges: (from_type, to_type, via_method)
    pub edges: Vec<TypeFlowEdge>,
}

/// Edge in the type flow graph representing a transformation
#[derive(Clone, Debug)]
pub struct TypeFlowEdge {
    pub from_type: String,
    pub to_type: String,
    pub via_method: String,
    pub transformation_type: TransformationType,
}

/// Classification of type transformations
#[derive(Clone, Debug, PartialEq)]
pub enum TransformationType {
    /// A → B (pure transformation)
    Direct,
    /// (A, B) → C (aggregation)
    Aggregation,
    /// A → (B, C) (decomposition)
    Decomposition,
    /// A → `Result<B>` (validation/enrichment)
    Enrichment,
    /// A → `Vec<B>` (expansion)
    Expansion,
}

/// A stage in a data transformation pipeline
#[derive(Clone, Debug)]
pub struct PipelineStage {
    pub stage_name: String,
    pub input_types: Vec<String>,
    pub output_types: Vec<String>,
    pub methods: Vec<String>,
    pub stage_type: StageType,
    pub depth: usize,
}

impl DataFlowAnalyzer {
    /// Build a type flow graph from method signatures
    pub fn build_type_flow_graph(
        &self,
        signatures: &[MethodSignature],
        _call_graph: &HashMap<String, Vec<String>>,
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
                graph
                    .producers
                    .entry(ret_type.name.clone())
                    .or_default()
                    .push(sig.name.clone());
            }

            // Parameter types = consumes
            for param in &sig.param_types {
                graph
                    .consumers
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

    /// Classify the type of transformation
    fn classify_transformation(
        &self,
        _from_type: &str,
        to_type: &str,
        all_params: &[TypeInfo],
        _ret_type: &TypeInfo,
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

    /// Detect pipeline stages from type flow graph
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
        let mut depth_groups: HashMap<usize, Vec<String>> = HashMap::new();

        for sig in signatures {
            if let Some(ret_type) = &sig.return_type {
                let depth = type_depths.get(&ret_type.name).unwrap_or(&0);
                depth_groups
                    .entry(*depth)
                    .or_default()
                    .push(sig.name.clone());
            }
        }

        // 5. Create stage for each depth level
        let mut stages = Vec::new();
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

    /// Find types that are only produced (sources)
    fn find_source_types(&self, graph: &TypeFlowGraph) -> HashSet<String> {
        graph
            .producers
            .keys()
            .filter(|type_name| !graph.consumers.contains_key(*type_name))
            .cloned()
            .collect()
    }

    /// Find types that are only consumed (sinks)
    fn find_sink_types(&self, graph: &TypeFlowGraph) -> HashSet<String> {
        graph
            .consumers
            .keys()
            .filter(|type_name| !graph.producers.contains_key(*type_name))
            .cloned()
            .collect()
    }

    /// Compute depths for each type in the flow graph using cycle-aware algorithm
    fn compute_type_depths(
        &self,
        graph: &TypeFlowGraph,
        sources: &HashSet<String>,
    ) -> Result<HashMap<String, usize>, String> {
        // Step 1: Detect cycles using Tarjan's algorithm
        let sccs = self.find_strongly_connected_components(graph)?;

        // Step 2: Build SCC graph (DAG of components)
        let scc_graph = self.build_scc_graph(graph, &sccs);

        // Step 3: Compute depths using topological sort
        let mut depths = HashMap::new();
        let mut scc_depths = HashMap::new();

        // Find source SCCs
        let source_sccs: Vec<_> = sccs
            .iter()
            .enumerate()
            .filter(|(_, component)| component.iter().any(|node| sources.contains(node)))
            .map(|(idx, _)| idx)
            .collect();

        // BFS from source SCCs
        let mut queue: VecDeque<(usize, usize)> = source_sccs.iter().map(|&idx| (idx, 0)).collect();

        while let Some((scc_idx, depth)) = queue.pop_front() {
            // Skip if already processed with shorter depth
            if let Some(&existing_depth) = scc_depths.get(&scc_idx) {
                if existing_depth <= depth {
                    continue;
                }
            }

            scc_depths.insert(scc_idx, depth);

            // Assign depth to all nodes in this SCC
            for node in &sccs[scc_idx] {
                depths.insert(node.clone(), depth);
            }

            // Add successor SCCs to queue
            if let Some(successors) = scc_graph.get(&scc_idx) {
                for &succ_idx in successors {
                    queue.push_back((succ_idx, depth + 1));
                }
            }
        }

        Ok(depths)
    }

    /// Find strongly connected components using Tarjan's algorithm
    fn find_strongly_connected_components(
        &self,
        graph: &TypeFlowGraph,
    ) -> Result<Vec<Vec<String>>, String> {
        let mut index = 0;
        let mut stack = Vec::new();
        let mut indices: HashMap<String, usize> = HashMap::new();
        let mut lowlinks: HashMap<String, usize> = HashMap::new();
        let mut on_stack: HashSet<String> = HashSet::new();
        let mut sccs = Vec::new();

        // Get all nodes
        let mut nodes: HashSet<String> = HashSet::new();
        for edge in &graph.edges {
            nodes.insert(edge.from_type.clone());
            nodes.insert(edge.to_type.clone());
        }

        // Run Tarjan's for each unvisited node
        for node in nodes {
            if !indices.contains_key(&node) {
                self.tarjan_visit(
                    &node,
                    graph,
                    &mut index,
                    &mut stack,
                    &mut indices,
                    &mut lowlinks,
                    &mut on_stack,
                    &mut sccs,
                );
            }
        }

        Ok(sccs)
    }

    /// Tarjan's algorithm visit function
    #[allow(clippy::too_many_arguments, clippy::only_used_in_recursion)]
    fn tarjan_visit(
        &self,
        node: &str,
        graph: &TypeFlowGraph,
        index: &mut usize,
        stack: &mut Vec<String>,
        indices: &mut HashMap<String, usize>,
        lowlinks: &mut HashMap<String, usize>,
        on_stack: &mut HashSet<String>,
        sccs: &mut Vec<Vec<String>>,
    ) {
        indices.insert(node.to_string(), *index);
        lowlinks.insert(node.to_string(), *index);
        *index += 1;
        stack.push(node.to_string());
        on_stack.insert(node.to_string());

        // Visit successors
        for edge in &graph.edges {
            if edge.from_type == node {
                let successor = &edge.to_type;

                if !indices.contains_key(successor) {
                    // Successor not visited, recurse
                    self.tarjan_visit(
                        successor, graph, index, stack, indices, lowlinks, on_stack, sccs,
                    );
                    let succ_lowlink = lowlinks[successor];
                    // Safe: we inserted node into lowlinks at the start of this function
                    if let Some(node_lowlink) = lowlinks.get_mut(node) {
                        *node_lowlink = (*node_lowlink).min(succ_lowlink);
                    }
                } else if on_stack.contains(successor) {
                    // Successor on stack, update lowlink
                    let succ_index = indices[successor];
                    // Safe: we inserted node into lowlinks at the start of this function
                    if let Some(node_lowlink) = lowlinks.get_mut(node) {
                        *node_lowlink = (*node_lowlink).min(succ_index);
                    }
                }
            }
        }

        // If node is root of SCC, pop the SCC
        if lowlinks[node] == indices[node] {
            let mut component = Vec::new();
            loop {
                // Safe: Tarjan's algorithm guarantees stack is not empty here
                // because we pushed node at the start of this function
                let Some(w) = stack.pop() else {
                    break; // Defensive: shouldn't happen but avoid panic
                };
                on_stack.remove(&w);
                component.push(w.clone());
                if w == node {
                    break;
                }
            }
            if !component.is_empty() {
                sccs.push(component);
            }
        }
    }

    /// Build graph of SCCs (condensation graph)
    fn build_scc_graph(
        &self,
        graph: &TypeFlowGraph,
        sccs: &[Vec<String>],
    ) -> HashMap<usize, Vec<usize>> {
        // Map each node to its SCC index
        let mut node_to_scc: HashMap<String, usize> = HashMap::new();
        for (idx, component) in sccs.iter().enumerate() {
            for node in component {
                node_to_scc.insert(node.clone(), idx);
            }
        }

        // Build SCC graph
        let mut scc_graph: HashMap<usize, Vec<usize>> = HashMap::new();

        for edge in &graph.edges {
            let from_scc = node_to_scc[&edge.from_type];
            let to_scc = node_to_scc[&edge.to_type];

            // Only add edge if crossing SCC boundary
            if from_scc != to_scc {
                scc_graph.entry(from_scc).or_default().push(to_scc);
            }
        }

        // Deduplicate edges
        for edges in scc_graph.values_mut() {
            edges.sort_unstable();
            edges.dedup();
        }

        scc_graph
    }

    /// Create a pipeline stage from a group of methods
    #[allow(clippy::too_many_arguments)]
    fn create_stage_from_methods(
        &self,
        depth: usize,
        methods: &[String],
        signatures: &[MethodSignature],
        graph: &TypeFlowGraph,
        _sources: &HashSet<String>,
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

    /// Infer stage type from transformation patterns
    fn infer_stage_type(
        &self,
        methods: &[String],
        _signatures: &[MethodSignature],
        graph: &TypeFlowGraph,
    ) -> StageType {
        let transformations: Vec<_> = graph
            .edges
            .iter()
            .filter(|e| methods.contains(&e.via_method))
            .map(|e| &e.transformation_type)
            .collect();

        // Majority vote
        let validate_count = transformations
            .iter()
            .filter(|t| ***t == TransformationType::Enrichment)
            .count();
        let aggregate_count = transformations
            .iter()
            .filter(|t| ***t == TransformationType::Aggregation)
            .count();

        if validate_count > transformations.len() / 2 {
            StageType::Validate
        } else if aggregate_count > transformations.len() / 2 {
            StageType::Aggregate
        } else {
            StageType::Transform
        }
    }

    /// Suggest a stage name based on output types
    fn suggest_stage_name(&self, stage_type: &StageType, output_types: &HashSet<String>) -> String {
        // Filter out generic/primitive types to find domain types
        let domain_types: Vec<_> = output_types
            .iter()
            .filter(|t| !self.is_generic_type(t))
            .collect();

        // Choose most specific type (prefer longer, domain-specific names)
        let primary_type = domain_types
            .iter()
            .max_by_key(|t| {
                let name = t.as_str();
                // Prefer types ending in domain suffixes
                let domain_bonus = if name.ends_with("Analysis")
                    || name.ends_with("Metrics")
                    || name.ends_with("Result")
                    || name.ends_with("Data")
                {
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
            StageType::Source => snake_case,
            StageType::Transform => format!("{}_transform", snake_case),
            StageType::Validate => format!("{}_validation", snake_case),
            StageType::Aggregate => format!("{}_aggregation", snake_case),
            StageType::Sink => format!("{}_output", snake_case),
        }
    }

    /// Check if a type is generic/primitive
    fn is_generic_type(&self, type_name: &str) -> bool {
        matches!(
            type_name,
            "String"
                | "str"
                | "Vec"
                | "Option"
                | "Result"
                | "HashMap"
                | "HashSet"
                | "BTreeMap"
                | "BTreeSet"
                | "usize"
                | "isize"
                | "u32"
                | "i32"
                | "u64"
                | "i64"
                | "f32"
                | "f64"
                | "bool"
                | "char"
        )
    }

    /// Convert CamelCase to snake_case
    fn to_snake_case(&self, s: &str) -> String {
        let mut result = String::new();
        for (i, ch) in s.chars().enumerate() {
            if ch.is_uppercase() && i > 0 {
                result.push('_');
            }
            // Safe: to_lowercase() always returns at least one character for any char
            // Use unwrap_or with the original char as fallback
            result.push(ch.to_lowercase().next().unwrap_or(ch));
        }
        result
    }

    /// Generate module split recommendations from pipeline stages
    pub fn generate_pipeline_recommendations(
        &self,
        stages: &[PipelineStage],
        base_name: &str,
    ) -> Vec<ModuleSplit> {
        stages
            .iter()
            .map(|stage| {
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
                    method_count: stage.methods.len(),
                    ..Default::default()
                }
            })
            .collect()
    }

    /// Describe the responsibility of a pipeline stage
    fn describe_stage_responsibility(&self, stage: &PipelineStage) -> String {
        let inputs = stage.input_types.join(", ");
        let outputs = stage.output_types.join(", ");

        match stage.stage_type {
            StageType::Source => format!(
                "Source stage: Produce {} for downstream processing",
                outputs
            ),
            StageType::Transform => format!("Transform {} into {}", inputs, outputs),
            StageType::Validate => format!("Validate and enrich {} into {}", inputs, outputs),
            StageType::Aggregate => format!("Aggregate {} into {}", inputs, outputs),
            StageType::Sink => format!("Sink stage: Consume {} for final output", inputs),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_flow_graph_construction() {
        let signatures = vec![
            MethodSignature {
                name: "parse".to_string(),
                param_types: vec![TypeInfo {
                    name: "String".to_string(),
                    ..Default::default()
                }],
                return_type: Some(TypeInfo {
                    name: "ParsedData".to_string(),
                    ..Default::default()
                }),
                self_type: None,
            },
            MethodSignature {
                name: "validate".to_string(),
                param_types: vec![TypeInfo {
                    name: "ParsedData".to_string(),
                    ..Default::default()
                }],
                return_type: Some(TypeInfo {
                    name: "Result".to_string(),
                    ..Default::default()
                }),
                self_type: None,
            },
        ];

        let analyzer = DataFlowAnalyzer;
        let graph = analyzer.build_type_flow_graph(&signatures, &HashMap::new());

        assert_eq!(graph.producers.get("ParsedData").unwrap(), &vec!["parse"]);
        assert_eq!(
            graph.consumers.get("ParsedData").unwrap(),
            &vec!["validate"]
        );
        assert_eq!(graph.edges.len(), 2);
    }

    #[test]
    fn test_transformation_classification() {
        let analyzer = DataFlowAnalyzer;

        // Enrichment (Result wrapper)
        let result_transform = analyzer.classify_transformation(
            "Data",
            "Result",
            &[],
            &TypeInfo {
                name: "Result".to_string(),
                ..Default::default()
            },
        );
        assert_eq!(result_transform, TransformationType::Enrichment);

        // Expansion (Vec output)
        let vec_transform = analyzer.classify_transformation(
            "Item",
            "Vec",
            &[],
            &TypeInfo {
                name: "Vec".to_string(),
                ..Default::default()
            },
        );
        assert_eq!(vec_transform, TransformationType::Expansion);
    }

    #[test]
    fn test_pipeline_stage_detection() {
        let signatures = vec![
            MethodSignature {
                name: "parse".to_string(),
                param_types: vec![TypeInfo {
                    name: "String".to_string(),
                    ..Default::default()
                }],
                return_type: Some(TypeInfo {
                    name: "ParsedData".to_string(),
                    ..Default::default()
                }),
                self_type: None,
            },
            MethodSignature {
                name: "validate".to_string(),
                param_types: vec![TypeInfo {
                    name: "ParsedData".to_string(),
                    ..Default::default()
                }],
                return_type: Some(TypeInfo {
                    name: "Result".to_string(),
                    ..Default::default()
                }),
                self_type: None,
            },
        ];

        let analyzer = DataFlowAnalyzer;
        let graph = analyzer.build_type_flow_graph(&signatures, &HashMap::new());
        let stages = analyzer
            .detect_pipeline_stages(&graph, &signatures)
            .unwrap();

        // Should detect at least one stage with pipeline structure
        assert!(!stages.is_empty());
        // First stage should be at depth 0
        assert_eq!(stages[0].depth, 0);
    }
}
