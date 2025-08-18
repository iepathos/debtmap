use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Unique identifier for nodes in the data flow graph
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct NodeId(String);

impl NodeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Source location information
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: PathBuf,
    pub line: usize,
    pub column: Option<usize>,
}

/// Types of expressions in the data flow
#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionKind {
    MethodCall {
        method: String,
        receiver: Option<NodeId>,
    },
    FunctionCall {
        function: String,
    },
    Assignment,
    FieldAccess {
        field: String,
    },
    ArrayAccess,
    Return,
    Conditional,
    Loop,
    Match,
    Binary {
        op: String,
    },
    Unary {
        op: String,
    },
    Literal,
    Reference,
    Dereference,
}

/// Node in the data flow graph
#[derive(Debug, Clone)]
pub enum DataFlowNode {
    /// Variable or binding
    Variable {
        name: String,
        location: SourceLocation,
        scope: ScopeId,
        is_parameter: bool,
    },
    /// Expression or computation
    Expression {
        kind: ExpressionKind,
        location: SourceLocation,
        scope: ScopeId,
    },
    /// Function parameter
    Parameter {
        function: String,
        index: usize,
        name: String,
        location: SourceLocation,
    },
    /// Function return value
    Return {
        function: String,
        location: SourceLocation,
    },
    /// Field of a struct
    Field {
        struct_name: String,
        field_name: String,
        location: SourceLocation,
    },
    /// External input source
    Source {
        kind: crate::security::types::InputSource,
        location: SourceLocation,
    },
    /// Dangerous operation sink
    Sink {
        kind: crate::security::types::SinkOperation,
        location: SourceLocation,
    },
    /// Validation or sanitization point
    Validator {
        method: String,
        location: SourceLocation,
    },
}

impl DataFlowNode {
    pub fn location(&self) -> &SourceLocation {
        match self {
            Self::Variable { location, .. }
            | Self::Expression { location, .. }
            | Self::Parameter { location, .. }
            | Self::Return { location, .. }
            | Self::Field { location, .. }
            | Self::Source { location, .. }
            | Self::Sink { location, .. }
            | Self::Validator { location, .. } => location,
        }
    }

    pub fn is_source(&self) -> bool {
        matches!(self, Self::Source { .. })
    }

    pub fn is_sink(&self) -> bool {
        matches!(self, Self::Sink { .. })
    }

    pub fn is_validator(&self) -> bool {
        matches!(self, Self::Validator { .. })
    }
}

/// Edge in the data flow graph
#[derive(Debug, Clone)]
pub struct DataFlowEdge {
    pub from: NodeId,
    pub to: NodeId,
    pub kind: EdgeKind,
}

/// Types of data flow edges
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeKind {
    /// Direct assignment or binding
    Assignment,
    /// Data flows through function parameter
    Parameter { index: usize },
    /// Data flows through function return
    Return,
    /// Data flows through method call
    MethodCall { method: String },
    /// Data flows through field access
    FieldAccess { field: String },
    /// Data flows through array/index access
    IndexAccess,
    /// Control flow dependency
    ControlFlow,
    /// Data transformation
    Transform,
    /// Validation/sanitization
    Validation,
}

/// Scope identifier for tracking variable scopes
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ScopeId(String);

impl ScopeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn child(&self, name: &str) -> Self {
        Self(format!("{}::{}", self.0, name))
    }
}

/// Data flow graph representation
pub struct DataFlowGraph {
    nodes: HashMap<NodeId, DataFlowNode>,
    edges: Vec<DataFlowEdge>,
    entry_points: HashSet<NodeId>,
    scopes: HashMap<ScopeId, Scope>,
    /// Map from variable names to their node IDs in each scope
    variable_map: HashMap<(ScopeId, String), NodeId>,
}

/// Scope information
#[derive(Debug, Clone)]
struct Scope {
    #[allow(dead_code)]
    id: ScopeId,
    parent: Option<ScopeId>,
    variables: HashSet<String>,
}

impl DataFlowGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            entry_points: HashSet::new(),
            scopes: HashMap::new(),
            variable_map: HashMap::new(),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, id: NodeId, node: DataFlowNode) -> NodeId {
        // Track variables in scope
        if let DataFlowNode::Variable { name, scope, .. } = &node {
            self.variable_map
                .insert((scope.clone(), name.clone()), id.clone());
            if let Some(scope_info) = self.scopes.get_mut(scope) {
                scope_info.variables.insert(name.clone());
            }
        }

        self.nodes.insert(id.clone(), node);
        id
    }

    /// Add an edge to the graph
    pub fn add_edge(&mut self, edge: DataFlowEdge) {
        self.edges.push(edge);
    }

    /// Mark a node as an entry point
    pub fn add_entry_point(&mut self, node: NodeId) {
        self.entry_points.insert(node);
    }

    /// Create a new scope
    pub fn create_scope(&mut self, id: ScopeId, parent: Option<ScopeId>) {
        self.scopes.insert(
            id.clone(),
            Scope {
                id: id.clone(),
                parent,
                variables: HashSet::new(),
            },
        );
    }

    /// Find a variable in the current scope or parent scopes
    pub fn find_variable(&self, scope: &ScopeId, name: &str) -> Option<&NodeId> {
        // Check current scope
        if let Some(node_id) = self.variable_map.get(&(scope.clone(), name.to_string())) {
            return Some(node_id);
        }

        // Check parent scopes
        if let Some(scope_info) = self.scopes.get(scope) {
            if let Some(parent) = &scope_info.parent {
                return self.find_variable(parent, name);
            }
        }

        None
    }

    /// Get a node by ID
    pub fn get_node(&self, id: &NodeId) -> Option<&DataFlowNode> {
        self.nodes.get(id)
    }

    /// Get all nodes
    pub fn nodes(&self) -> impl Iterator<Item = (&NodeId, &DataFlowNode)> {
        self.nodes.iter()
    }

    /// Get all edges
    pub fn edges(&self) -> &[DataFlowEdge] {
        &self.edges
    }

    /// Get entry points
    pub fn entry_points(&self) -> &HashSet<NodeId> {
        &self.entry_points
    }

    /// Get outgoing edges from a node
    pub fn outgoing_edges(&self, node: &NodeId) -> Vec<&DataFlowEdge> {
        self.edges
            .iter()
            .filter(|edge| &edge.from == node)
            .collect()
    }

    /// Get incoming edges to a node
    pub fn incoming_edges(&self, node: &NodeId) -> Vec<&DataFlowEdge> {
        self.edges.iter().filter(|edge| &edge.to == node).collect()
    }

    /// Find all source nodes
    pub fn source_nodes(&self) -> Vec<(&NodeId, &DataFlowNode)> {
        self.nodes
            .iter()
            .filter(|(_, node)| node.is_source())
            .collect()
    }

    /// Find all sink nodes
    pub fn sink_nodes(&self) -> Vec<(&NodeId, &DataFlowNode)> {
        self.nodes
            .iter()
            .filter(|(_, node)| node.is_sink())
            .collect()
    }
}

impl Default for DataFlowGraph {
    fn default() -> Self {
        Self::new()
    }
}
