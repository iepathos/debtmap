use crate::analysis::data_flow::DataFlowAnalysis;
use crate::priority::{call_graph::CallGraph, call_graph::FunctionId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub mod population;

mod function_id_serde {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap as StdHashMap;

    pub fn serialize<S, V>(map: &HashMap<FunctionId, V>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: Serialize,
    {
        let string_map: StdHashMap<String, &V> = map
            .iter()
            .map(|(k, v)| (format!("{}:{}:{}", k.file.display(), k.name, k.line), v))
            .collect();
        string_map.serialize(serializer)
    }

    pub fn deserialize<'de, D, V>(deserializer: D) -> Result<HashMap<FunctionId, V>, D::Error>
    where
        D: Deserializer<'de>,
        V: Deserialize<'de>,
    {
        let string_map: StdHashMap<String, V> = StdHashMap::deserialize(deserializer)?;
        let mut result = HashMap::new();
        for (key, value) in string_map {
            let parts: Vec<&str> = key.rsplitn(3, ':').collect();
            if parts.len() == 3 {
                let func_id = FunctionId::new(
                    parts[2].into(),
                    parts[1].to_string(),
                    parts[0].parse().unwrap_or(0),
                );
                result.insert(func_id, value);
            }
        }
        Ok(result)
    }
}

mod function_id_tuple_serde {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap as StdHashMap;

    pub fn serialize<S, V>(
        map: &HashMap<(FunctionId, FunctionId), V>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: Serialize,
    {
        let string_map: StdHashMap<String, &V> = map
            .iter()
            .map(|((k1, k2), v)| {
                let key = format!(
                    "{}:{}:{}|{}:{}:{}",
                    k1.file.display(),
                    k1.name,
                    k1.line,
                    k2.file.display(),
                    k2.name,
                    k2.line
                );
                (key, v)
            })
            .collect();
        string_map.serialize(serializer)
    }

    pub fn deserialize<'de, D, V>(
        deserializer: D,
    ) -> Result<HashMap<(FunctionId, FunctionId), V>, D::Error>
    where
        D: Deserializer<'de>,
        V: Deserialize<'de>,
    {
        let string_map: StdHashMap<String, V> = StdHashMap::deserialize(deserializer)?;
        let mut result = HashMap::new();
        for (key, value) in string_map {
            let parts: Vec<&str> = key.split('|').collect();
            if parts.len() == 2 {
                let parts1: Vec<&str> = parts[0].rsplitn(3, ':').collect();
                let parts2: Vec<&str> = parts[1].rsplitn(3, ':').collect();
                if parts1.len() == 3 && parts2.len() == 3 {
                    let func_id1 = FunctionId::new(
                        parts1[2].into(),
                        parts1[1].to_string(),
                        parts1[0].parse().unwrap_or(0),
                    );
                    let func_id2 = FunctionId::new(
                        parts2[2].into(),
                        parts2[1].to_string(),
                        parts2[0].parse().unwrap_or(0),
                    );
                    result.insert((func_id1, func_id2), value);
                }
            }
        }
        Ok(result)
    }
}

/// DataFlowGraph provides data flow analysis capabilities built on top of the CallGraph.
/// It tracks variable dependencies, data transformations, and information flow between functions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFlowGraph {
    /// The underlying call graph that tracks function relationships
    call_graph: CallGraph,
    /// Variable dependencies within each function (function_id -> set of variables)
    #[serde(with = "function_id_serde")]
    variable_deps: HashMap<FunctionId, HashSet<String>>,
    /// Data transformations tracked between functions
    #[serde(with = "function_id_tuple_serde")]
    data_transformations: HashMap<(FunctionId, FunctionId), DataTransformation>,
    /// I/O operations detected in functions
    #[serde(with = "function_id_serde")]
    io_operations: HashMap<FunctionId, Vec<IoOperation>>,
    /// Pure function analysis results
    #[serde(with = "function_id_serde")]
    purity_analysis: HashMap<FunctionId, PurityInfo>,
    /// Full CFG-based data flow analysis from purity detector
    /// Note: Skipped during serialization due to complexity
    #[serde(skip)]
    cfg_analysis: HashMap<FunctionId, DataFlowAnalysis>,
    /// CFG-based data flow analysis with variable name context
    /// Note: Skipped during serialization due to complexity
    #[serde(skip)]
    cfg_analysis_with_context: HashMap<FunctionId, CfgAnalysisWithContext>,
    /// Mutation analysis (live vs dead mutations)
    #[serde(with = "function_id_serde")]
    mutation_analysis: HashMap<FunctionId, MutationInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTransformation {
    /// Variables passed from caller to callee
    pub input_vars: Vec<String>,
    /// Variables returned or modified
    pub output_vars: Vec<String>,
    /// Type of transformation (e.g., "map", "filter", "reduce")
    pub transformation_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoOperation {
    /// Type of I/O operation (file, network, console, etc.)
    pub operation_type: String,
    /// Variables involved in the I/O operation
    pub variables: Vec<String>,
    /// Line number where the operation occurs
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurityInfo {
    /// Whether the function is pure (no side effects)
    pub is_pure: bool,
    /// Confidence level in the purity analysis (0.0 to 1.0)
    pub confidence: f32,
    /// Reasons why the function may not be pure
    pub impurity_reasons: Vec<String>,
}

/// Mutation analysis information for a function.
/// Uses binary signals for reliability - precise counts are not guaranteed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationInfo {
    /// Whether any mutations were detected in the function
    pub has_mutations: bool,
    /// Whether any mutations escape the function (affect return or external state)
    pub has_escaping_mutations: bool,
    /// Best-effort list of detected mutations (may be incomplete)
    pub detected_mutations: Vec<String>,
    /// Best-effort list of escaping variables (may be incomplete)
    pub escaping_vars: Vec<String>,
}

impl MutationInfo {
    /// Create a MutationInfo indicating no mutations
    pub fn none() -> Self {
        Self {
            has_mutations: false,
            has_escaping_mutations: false,
            detected_mutations: Vec::new(),
            escaping_vars: Vec::new(),
        }
    }

    /// Check if the function is pure (no mutations and no escaping mutations)
    pub fn is_pure(&self) -> bool {
        !self.has_mutations && !self.has_escaping_mutations
    }
}

/// CFG-based data flow analysis with variable name context for translation.
///
/// This wrapper combines `DataFlowAnalysis` (which uses numeric `VarId`s)
/// with the variable name mapping from the CFG, enabling translation of
/// VarIds back to human-readable variable names.
///
/// # Why This Exists
///
/// The CFG uses `VarId { name_id: u32, version: u32 }` for efficiency during
/// analysis. To display results to users, we need to translate these IDs back
/// to variable names like "buffer", "x", "result", etc.
///
/// # Example
///
/// ```ignore
/// let cfg = ControlFlowGraph::from_block(&block);
/// let analysis = DataFlowAnalysis::analyze(&cfg);
/// let ctx = CfgAnalysisWithContext::from_cfg(&cfg, analysis);
///
/// // Translate a VarId to a name
/// let var_id = VarId { name_id: 0, version: 0 };
/// let name = ctx.var_name(var_id); // "x", "buffer", etc.
/// ```
#[derive(Debug, Clone)]
pub struct CfgAnalysisWithContext {
    /// The full data flow analysis results
    pub analysis: DataFlowAnalysis,
    /// Variable name mapping from CFG (VarId.name_id -> variable name)
    pub var_names: Vec<String>,
}

impl CfgAnalysisWithContext {
    /// Create from a ControlFlowGraph and its analysis
    pub fn new(var_names: Vec<String>, analysis: DataFlowAnalysis) -> Self {
        Self {
            analysis,
            var_names,
        }
    }

    /// Translate a VarId to a variable name
    pub fn var_name(&self, var_id: crate::analysis::data_flow::VarId) -> String {
        self.var_names
            .get(var_id.name_id as usize)
            .cloned()
            .unwrap_or_else(|| format!("unknown_{}", var_id.name_id))
    }

    /// Translate multiple VarIds to names
    pub fn var_names_for(
        &self,
        var_ids: impl Iterator<Item = crate::analysis::data_flow::VarId>,
    ) -> Vec<String> {
        var_ids.map(|id| self.var_name(id)).collect()
    }
}

impl DataFlowGraph {
    pub fn new() -> Self {
        Self {
            call_graph: CallGraph::new(),
            variable_deps: HashMap::new(),
            data_transformations: HashMap::new(),
            io_operations: HashMap::new(),
            purity_analysis: HashMap::new(),
            cfg_analysis: HashMap::new(),
            cfg_analysis_with_context: HashMap::new(),
            mutation_analysis: HashMap::new(),
        }
    }

    /// Create a DataFlowGraph from an existing CallGraph
    pub fn from_call_graph(call_graph: CallGraph) -> Self {
        Self {
            call_graph,
            variable_deps: HashMap::new(),
            data_transformations: HashMap::new(),
            io_operations: HashMap::new(),
            purity_analysis: HashMap::new(),
            cfg_analysis: HashMap::new(),
            cfg_analysis_with_context: HashMap::new(),
            mutation_analysis: HashMap::new(),
        }
    }

    /// Get the underlying call graph
    pub fn call_graph(&self) -> &CallGraph {
        &self.call_graph
    }

    /// Get variable dependencies for a function
    pub fn get_variable_dependencies(&self, func_id: &FunctionId) -> Option<&HashSet<String>> {
        self.variable_deps.get(func_id)
    }

    /// Add variable dependencies for a function
    pub fn add_variable_dependencies(&mut self, func_id: FunctionId, variables: HashSet<String>) {
        self.variable_deps.insert(func_id, variables);
    }

    /// Get data transformation between two functions
    pub fn get_data_transformation(
        &self,
        from: &FunctionId,
        to: &FunctionId,
    ) -> Option<&DataTransformation> {
        self.data_transformations.get(&(from.clone(), to.clone()))
    }

    /// Add data transformation between two functions
    pub fn add_data_transformation(
        &mut self,
        from: FunctionId,
        to: FunctionId,
        transformation: DataTransformation,
    ) {
        self.data_transformations.insert((from, to), transformation);
    }

    /// Get I/O operations for a function
    pub fn get_io_operations(&self, func_id: &FunctionId) -> Option<&Vec<IoOperation>> {
        self.io_operations.get(func_id)
    }

    /// Add I/O operation for a function
    pub fn add_io_operation(&mut self, func_id: FunctionId, operation: IoOperation) {
        self.io_operations
            .entry(func_id)
            .or_default()
            .push(operation);
    }

    /// Get purity information for a function
    pub fn get_purity_info(&self, func_id: &FunctionId) -> Option<&PurityInfo> {
        self.purity_analysis.get(func_id)
    }

    /// Set purity information for a function
    pub fn set_purity_info(&mut self, func_id: FunctionId, purity: PurityInfo) {
        self.purity_analysis.insert(func_id, purity);
    }

    /// Get CFG-based data flow analysis for a function
    pub fn get_cfg_analysis(&self, func_id: &FunctionId) -> Option<&DataFlowAnalysis> {
        self.cfg_analysis.get(func_id)
    }

    /// Set CFG-based data flow analysis for a function
    pub fn set_cfg_analysis(&mut self, func_id: FunctionId, analysis: DataFlowAnalysis) {
        self.cfg_analysis.insert(func_id, analysis);
    }

    /// Get mutation analysis for a function
    pub fn get_mutation_info(&self, func_id: &FunctionId) -> Option<&MutationInfo> {
        self.mutation_analysis.get(func_id)
    }

    /// Set mutation analysis for a function
    pub fn set_mutation_info(&mut self, func_id: FunctionId, info: MutationInfo) {
        self.mutation_analysis.insert(func_id, info);
    }

    /// Get CFG analysis with translation context
    pub fn get_cfg_analysis_with_context(
        &self,
        func_id: &FunctionId,
    ) -> Option<&CfgAnalysisWithContext> {
        self.cfg_analysis_with_context.get(func_id)
    }

    /// Set CFG analysis with context
    pub fn set_cfg_analysis_with_context(
        &mut self,
        func_id: FunctionId,
        context: CfgAnalysisWithContext,
    ) {
        self.cfg_analysis_with_context.insert(func_id, context);
    }

    /// Get escaping variable names
    pub fn get_escaping_var_names(&self, func_id: &FunctionId) -> Vec<String> {
        if let Some(ctx) = self.get_cfg_analysis_with_context(func_id) {
            ctx.var_names_for(ctx.analysis.escape_info.escaping_vars.iter().copied())
        } else {
            vec![]
        }
    }

    /// Get return dependency variable names
    pub fn get_return_dependency_names(&self, func_id: &FunctionId) -> Vec<String> {
        if let Some(ctx) = self.get_cfg_analysis_with_context(func_id) {
            ctx.var_names_for(ctx.analysis.escape_info.return_dependencies.iter().copied())
        } else {
            vec![]
        }
    }

    /// Get tainted variable names
    pub fn get_tainted_var_names(&self, func_id: &FunctionId) -> Vec<String> {
        if let Some(ctx) = self.get_cfg_analysis_with_context(func_id) {
            ctx.var_names_for(ctx.analysis.taint_info.tainted_vars.iter().copied())
        } else {
            vec![]
        }
    }

    /// Check if a function has side effects based on data flow analysis
    pub fn has_side_effects(&self, func_id: &FunctionId) -> bool {
        // Check purity analysis first
        if let Some(purity) = self.get_purity_info(func_id) {
            return !purity.is_pure;
        }

        // Check for I/O operations
        if let Some(io_ops) = self.get_io_operations(func_id) {
            return !io_ops.is_empty();
        }

        // Conservative estimate: assume side effects if we don't have analysis data
        true
    }

    /// Get all functions that may be affected by changes to the given function
    pub fn get_downstream_dependencies(&self, func_id: &FunctionId) -> Vec<FunctionId> {
        // Use the call graph to find functions that call this one
        self.call_graph.get_callers(func_id)
    }

    /// Get all functions that the given function depends on
    pub fn get_upstream_dependencies(&self, _func_id: &FunctionId) -> Vec<FunctionId> {
        // This would need to be implemented based on the call graph structure
        // For now, return an empty vector as a placeholder
        Vec::new()
    }

    /// Analyze the data flow impact of modifying a function
    pub fn analyze_modification_impact(&self, func_id: &FunctionId) -> ModificationImpact {
        let downstream = self.get_downstream_dependencies(func_id);
        let upstream = self.get_upstream_dependencies(func_id);
        let has_io = self
            .get_io_operations(func_id)
            .is_some_and(|ops| !ops.is_empty());
        let is_pure = self.get_purity_info(func_id).is_some_and(|p| p.is_pure);

        ModificationImpact {
            affected_functions: downstream.len(),
            dependency_count: upstream.len(),
            has_side_effects: has_io || !is_pure,
            risk_level: self.calculate_risk_level(&downstream, has_io, is_pure),
        }
    }

    fn calculate_risk_level(
        &self,
        downstream: &[FunctionId],
        has_io: bool,
        is_pure: bool,
    ) -> RiskLevel {
        match (downstream.len(), has_io, is_pure) {
            (0, false, true) => RiskLevel::Low,
            (1..=5, false, true) => RiskLevel::Medium,
            (1..=5, true, _) => RiskLevel::High,
            (6.., _, _) => RiskLevel::Critical,
            _ => RiskLevel::Medium,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModificationImpact {
    /// Number of functions that may be affected by changes
    pub affected_functions: usize,
    /// Number of functions this function depends on
    pub dependency_count: usize,
    /// Whether the function has side effects
    pub has_side_effects: bool,
    /// Overall risk level of modifying this function
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for DataFlowGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_function_id(name: &str) -> FunctionId {
        FunctionId::new(PathBuf::from("test.rs"), name.to_string(), 1)
    }

    #[test]
    fn test_data_flow_graph_creation() {
        let graph = DataFlowGraph::new();
        assert_eq!(graph.call_graph().node_count(), 0);
        assert!(graph.variable_deps.is_empty());
        assert!(graph.data_transformations.is_empty());
    }

    #[test]
    fn test_variable_dependencies() {
        let mut graph = DataFlowGraph::new();
        let func_id = create_test_function_id("test_func");

        let mut variables = HashSet::new();
        variables.insert("x".to_string());
        variables.insert("y".to_string());

        graph.add_variable_dependencies(func_id.clone(), variables);

        let deps = graph.get_variable_dependencies(&func_id).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.contains("x"));
        assert!(deps.contains("y"));
    }

    #[test]
    fn test_io_operations() {
        let mut graph = DataFlowGraph::new();
        let func_id = create_test_function_id("io_func");

        let io_op = IoOperation {
            operation_type: "file_read".to_string(),
            variables: vec!["filename".to_string()],
            line: 42,
        };

        graph.add_io_operation(func_id.clone(), io_op);

        let ops = graph.get_io_operations(&func_id).unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_type, "file_read");
        assert_eq!(ops[0].line, 42);
    }

    #[test]
    fn test_purity_analysis() {
        let mut graph = DataFlowGraph::new();
        let func_id = create_test_function_id("pure_func");

        let purity = PurityInfo {
            is_pure: true,
            confidence: 0.95,
            impurity_reasons: vec![],
        };

        graph.set_purity_info(func_id.clone(), purity);

        let purity_info = graph.get_purity_info(&func_id).unwrap();
        assert!(purity_info.is_pure);
        assert_eq!(purity_info.confidence, 0.95);
        assert!(purity_info.impurity_reasons.is_empty());

        assert!(!graph.has_side_effects(&func_id));
    }

    #[test]
    fn test_side_effects_detection() {
        let mut graph = DataFlowGraph::new();
        let func_id = create_test_function_id("impure_func");

        // Function with I/O operations should have side effects
        let io_op = IoOperation {
            operation_type: "console_log".to_string(),
            variables: vec!["message".to_string()],
            line: 10,
        };
        graph.add_io_operation(func_id.clone(), io_op);

        assert!(graph.has_side_effects(&func_id));
    }

    #[test]
    fn test_data_transformation() {
        let mut graph = DataFlowGraph::new();
        let from_func = create_test_function_id("caller");
        let to_func = create_test_function_id("callee");

        let transformation = DataTransformation {
            input_vars: vec!["input".to_string()],
            output_vars: vec!["result".to_string()],
            transformation_type: "map".to_string(),
        };

        graph.add_data_transformation(from_func.clone(), to_func.clone(), transformation);

        let trans = graph.get_data_transformation(&from_func, &to_func).unwrap();
        assert_eq!(trans.transformation_type, "map");
        assert_eq!(trans.input_vars, vec!["input"]);
        assert_eq!(trans.output_vars, vec!["result"]);
    }

    #[test]
    fn test_modification_impact_analysis() {
        let graph = DataFlowGraph::new();
        let func_id = create_test_function_id("test_func");

        let impact = graph.analyze_modification_impact(&func_id);

        // Since we have no call graph data, downstream should be empty
        assert_eq!(impact.affected_functions, 0);
        assert_eq!(impact.dependency_count, 0);
        // Should assume side effects when no data is available
        assert!(impact.has_side_effects);
    }

    #[test]
    fn test_risk_level_calculation() {
        let graph = DataFlowGraph::new();

        // Test low risk: no downstream, no I/O, pure function
        assert_eq!(graph.calculate_risk_level(&[], false, true), RiskLevel::Low);

        // Test high risk: some downstream with I/O
        let downstream = vec![create_test_function_id("caller1")];
        assert_eq!(
            graph.calculate_risk_level(&downstream, true, false),
            RiskLevel::High
        );

        // Test critical risk: many downstream dependencies
        let many_downstream: Vec<FunctionId> = (0..10)
            .map(|i| create_test_function_id(&format!("caller_{}", i)))
            .collect();
        assert_eq!(
            graph.calculate_risk_level(&many_downstream, false, true),
            RiskLevel::Critical
        );
    }

    #[test]
    fn test_from_call_graph() {
        let call_graph = CallGraph::new();
        let graph = DataFlowGraph::from_call_graph(call_graph);

        assert_eq!(graph.call_graph().node_count(), 0);
        assert!(graph.variable_deps.is_empty());
    }

    #[test]
    fn test_varid_translation() {
        use crate::analysis::data_flow::{
            DataFlowAnalysis, EscapeAnalysis, ReachingDefinitions, TaintAnalysis, VarId,
        };
        use std::collections::HashMap;

        let var_names = vec!["x".to_string(), "y".to_string(), "buffer".to_string()];

        // Create minimal analysis
        let analysis = DataFlowAnalysis {
            reaching_defs: ReachingDefinitions::default(),
            escape_info: EscapeAnalysis {
                escaping_vars: HashSet::new(),
                captured_vars: HashSet::new(),
                return_dependencies: HashSet::new(),
            },
            taint_info: TaintAnalysis {
                tainted_vars: HashSet::new(),
                taint_sources: HashMap::new(),
                return_tainted: false,
            },
        };

        let ctx = CfgAnalysisWithContext::new(var_names, analysis);

        let var_id = VarId {
            name_id: 0,
            version: 0,
        };
        assert_eq!(ctx.var_name(var_id), "x");

        let var_id = VarId {
            name_id: 2,
            version: 1,
        };
        assert_eq!(ctx.var_name(var_id), "buffer");
    }

    #[test]
    fn test_translation_with_missing_id() {
        use crate::analysis::data_flow::{
            DataFlowAnalysis, EscapeAnalysis, ReachingDefinitions, TaintAnalysis, VarId,
        };
        use std::collections::HashMap;

        let var_names = vec!["x".to_string()];

        let analysis = DataFlowAnalysis {
            reaching_defs: ReachingDefinitions::default(),
            escape_info: EscapeAnalysis {
                escaping_vars: HashSet::new(),
                captured_vars: HashSet::new(),
                return_dependencies: HashSet::new(),
            },
            taint_info: TaintAnalysis {
                tainted_vars: HashSet::new(),
                taint_sources: HashMap::new(),
                return_tainted: false,
            },
        };

        let ctx = CfgAnalysisWithContext::new(var_names, analysis);

        let invalid_id = VarId {
            name_id: 999,
            version: 0,
        };
        assert_eq!(ctx.var_name(invalid_id), "unknown_999");
    }

    #[test]
    fn test_escaping_var_translation() {
        use crate::analysis::data_flow::{
            DataFlowAnalysis, EscapeAnalysis, ReachingDefinitions, TaintAnalysis, VarId,
        };
        use std::collections::HashMap;

        let mut data_flow = DataFlowGraph::new();
        let func_id = create_test_function_id("test");

        let mut escaping_vars = HashSet::new();
        escaping_vars.insert(VarId {
            name_id: 0,
            version: 0,
        });

        let analysis = DataFlowAnalysis {
            reaching_defs: ReachingDefinitions::default(),
            escape_info: EscapeAnalysis {
                escaping_vars,
                captured_vars: HashSet::new(),
                return_dependencies: HashSet::new(),
            },
            taint_info: TaintAnalysis {
                tainted_vars: HashSet::new(),
                taint_sources: HashMap::new(),
                return_tainted: false,
            },
        };

        let ctx = CfgAnalysisWithContext::new(vec!["result".to_string()], analysis);

        data_flow.set_cfg_analysis_with_context(func_id.clone(), ctx);

        let names = data_flow.get_escaping_var_names(&func_id);
        assert_eq!(names, vec!["result"]);
    }

    #[test]
    fn test_return_dependency_translation() {
        use crate::analysis::data_flow::{
            DataFlowAnalysis, EscapeAnalysis, ReachingDefinitions, TaintAnalysis, VarId,
        };
        use std::collections::HashMap;

        let mut data_flow = DataFlowGraph::new();
        let func_id = create_test_function_id("test");

        let mut return_deps = HashSet::new();
        return_deps.insert(VarId {
            name_id: 0,
            version: 0,
        });
        return_deps.insert(VarId {
            name_id: 1,
            version: 0,
        });

        let analysis = DataFlowAnalysis {
            reaching_defs: ReachingDefinitions::default(),
            escape_info: EscapeAnalysis {
                escaping_vars: HashSet::new(),
                captured_vars: HashSet::new(),
                return_dependencies: return_deps,
            },
            taint_info: TaintAnalysis {
                tainted_vars: HashSet::new(),
                taint_sources: HashMap::new(),
                return_tainted: false,
            },
        };

        let ctx =
            CfgAnalysisWithContext::new(vec!["result".to_string(), "buffer".to_string()], analysis);

        data_flow.set_cfg_analysis_with_context(func_id.clone(), ctx);

        let names = data_flow.get_return_dependency_names(&func_id);
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"result".to_string()));
        assert!(names.contains(&"buffer".to_string()));
    }

    #[test]
    fn test_tainted_var_translation() {
        use crate::analysis::data_flow::{
            DataFlowAnalysis, EscapeAnalysis, ReachingDefinitions, TaintAnalysis, VarId,
        };
        use std::collections::HashMap;

        let mut data_flow = DataFlowGraph::new();
        let func_id = create_test_function_id("test");

        let mut tainted_vars = HashSet::new();
        tainted_vars.insert(VarId {
            name_id: 0,
            version: 0,
        });
        tainted_vars.insert(VarId {
            name_id: 1,
            version: 1,
        });

        let analysis = DataFlowAnalysis {
            reaching_defs: ReachingDefinitions::default(),
            escape_info: EscapeAnalysis {
                escaping_vars: HashSet::new(),
                captured_vars: HashSet::new(),
                return_dependencies: HashSet::new(),
            },
            taint_info: TaintAnalysis {
                tainted_vars,
                taint_sources: HashMap::new(),
                return_tainted: false,
            },
        };

        let ctx = CfgAnalysisWithContext::new(
            vec!["user_input".to_string(), "sanitized".to_string()],
            analysis,
        );

        data_flow.set_cfg_analysis_with_context(func_id.clone(), ctx);

        let names = data_flow.get_tainted_var_names(&func_id);
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"user_input".to_string()));
        assert!(names.contains(&"sanitized".to_string()));
    }
}
