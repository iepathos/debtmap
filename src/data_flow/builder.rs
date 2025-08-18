use super::graph::{
    DataFlowEdge, DataFlowGraph, DataFlowNode, EdgeKind, ExpressionKind, NodeId, ScopeId,
    SourceLocation,
};
use crate::security::types::{InputSource, SinkOperation};
use quote::ToTokens;
use std::path::{Path, PathBuf};
use syn::visit::Visit;
use syn::{
    Block, Expr, ExprAssign, ExprCall, ExprField, ExprForLoop, ExprIf, ExprIndex, ExprLoop,
    ExprMatch, ExprMethodCall, ExprReturn, ExprWhile, File, FnArg, ItemFn, Local, Pat, PatIdent,
    PatType, Signature, Stmt,
};

/// Builds a data flow graph from Rust AST
pub struct DataFlowBuilder {
    graph: DataFlowGraph,
    current_file: PathBuf,
    current_function: Option<String>,
    current_scope: ScopeId,
    scope_counter: usize,
    node_counter: usize,
}

impl DataFlowBuilder {
    pub fn new() -> Self {
        let mut graph = DataFlowGraph::new();
        let root_scope = ScopeId::new("root");
        graph.create_scope(root_scope.clone(), None);

        Self {
            graph,
            current_file: PathBuf::new(),
            current_function: None,
            current_scope: ScopeId::new("root"),
            scope_counter: 0,
            node_counter: 0,
        }
    }

    /// Build a data flow graph from a file AST
    pub fn build(&mut self, file: &File, path: &Path) -> DataFlowGraph {
        self.current_file = path.to_path_buf();
        self.graph = DataFlowGraph::new();
        self.graph.create_scope(ScopeId::new("root"), None);
        self.current_scope = ScopeId::new("root");
        self.scope_counter = 0;
        self.node_counter = 0;

        self.visit_file(file);

        std::mem::take(&mut self.graph)
    }

    fn next_node_id(&mut self) -> NodeId {
        self.node_counter += 1;
        NodeId::new(format!("node_{}", self.node_counter))
    }

    fn next_scope_id(&mut self, name: &str) -> ScopeId {
        self.scope_counter += 1;
        self.current_scope
            .child(&format!("{}_{}", name, self.scope_counter))
    }

    fn make_location(&self, line: usize) -> SourceLocation {
        SourceLocation {
            file: self.current_file.clone(),
            line,
            column: None,
        }
    }

    /// Analyze an expression and return its node ID if it produces data flow
    fn analyze_expr(&mut self, expr: &Expr) -> Option<NodeId> {
        match expr {
            Expr::MethodCall(method_call) => self.analyze_method_call(method_call),
            Expr::Call(call) => self.analyze_call(call),
            Expr::Path(path) => self.analyze_path_expr(path),
            Expr::Field(field) => self.analyze_field_access(field),
            Expr::Index(index) => self.analyze_index_access(index),
            Expr::Assign(assign) => self.analyze_assignment(assign),
            Expr::Return(ret) => self.analyze_return(ret),
            Expr::If(if_expr) => self.analyze_if(if_expr),
            Expr::Match(match_expr) => self.analyze_match(match_expr),
            Expr::Loop(loop_expr) => self.analyze_loop(loop_expr),
            Expr::While(while_expr) => self.analyze_while(while_expr),
            Expr::ForLoop(for_expr) => self.analyze_for(for_expr),
            Expr::Reference(ref_expr) => {
                // For references, analyze the inner expression
                self.analyze_expr(&ref_expr.expr)
            }
            Expr::Lit(_) => {
                // Literals don't create data flow
                None
            }
            _ => {
                // For other expression types, visit children
                syn::visit::visit_expr(self, expr);
                None
            }
        }
    }

    fn analyze_method_call(&mut self, call: &ExprMethodCall) -> Option<NodeId> {
        let method_name = call.method.to_string();

        // Check if this is a source method
        if let Some(source) = self.detect_source_method(&method_name, &call.receiver) {
            let node_id = self.next_node_id();
            let node = DataFlowNode::Source {
                kind: source,
                location: self.make_location(0), // Would need span info
            };
            self.graph.add_node(node_id.clone(), node);
            return Some(node_id);
        }

        // Check if this is a sink method
        if let Some(sink) = self.detect_sink_method(&method_name) {
            let node_id = self.next_node_id();
            let node = DataFlowNode::Sink {
                kind: sink,
                location: self.make_location(0),
            };
            self.graph.add_node(node_id.clone(), node);

            // Connect arguments to sink
            for arg in &call.args {
                if let Some(arg_node) = self.analyze_expr(arg) {
                    self.graph.add_edge(DataFlowEdge {
                        from: arg_node,
                        to: node_id.clone(),
                        kind: EdgeKind::MethodCall {
                            method: method_name.clone(),
                        },
                    });
                }
            }

            return Some(node_id);
        }

        // Check if this is a validation method
        if self.is_validation_method(&method_name) {
            let node_id = self.next_node_id();
            let node = DataFlowNode::Validator {
                method: method_name.clone(),
                location: self.make_location(0),
            };
            self.graph.add_node(node_id.clone(), node);

            // Connect receiver to validator
            if let Some(receiver_node) = self.analyze_expr(&call.receiver) {
                self.graph.add_edge(DataFlowEdge {
                    from: receiver_node,
                    to: node_id.clone(),
                    kind: EdgeKind::Validation,
                });
            }

            return Some(node_id);
        }

        // Regular method call - create expression node
        let receiver_node = self.analyze_expr(&call.receiver);
        let node_id = self.next_node_id();
        let node = DataFlowNode::Expression {
            kind: ExpressionKind::MethodCall {
                method: method_name.clone(),
                receiver: receiver_node.clone(),
            },
            location: self.make_location(0),
            scope: self.current_scope.clone(),
        };
        self.graph.add_node(node_id.clone(), node);

        // Connect receiver if present - data flows FROM receiver TO this method call result
        if let Some(receiver) = receiver_node {
            self.graph.add_edge(DataFlowEdge {
                from: receiver,
                to: node_id.clone(),
                kind: EdgeKind::MethodCall {
                    method: method_name.clone(),
                },
            });
        }

        // Connect arguments
        for arg in &call.args {
            if let Some(arg_node) = self.analyze_expr(arg) {
                self.graph.add_edge(DataFlowEdge {
                    from: arg_node,
                    to: node_id.clone(),
                    kind: EdgeKind::Parameter { index: 0 }, // Would need proper indexing
                });
            }
        }

        Some(node_id)
    }

    fn analyze_call(&mut self, call: &ExprCall) -> Option<NodeId> {
        // Check if this is a source function
        // Try to get a better string representation
        let func_str = if let Expr::Path(path) = &*call.func {
            // Build path string from segments
            path.path
                .segments
                .iter()
                .map(|seg| seg.ident.to_string())
                .collect::<Vec<_>>()
                .join("::")
        } else {
            format!("{:?}", call.func)
        };

        if let Some(source) = self.detect_source_function(&func_str) {
            let node_id = self.next_node_id();
            let node = DataFlowNode::Source {
                kind: source,
                location: self.make_location(0),
            };
            self.graph.add_node(node_id.clone(), node);
            return Some(node_id);
        }

        // Check if this is a sink function
        if let Some(sink) = self.detect_sink_function(&func_str) {
            let node_id = self.next_node_id();
            let node = DataFlowNode::Sink {
                kind: sink,
                location: self.make_location(0),
            };
            self.graph.add_node(node_id.clone(), node);

            // Connect arguments to sink
            for arg in &call.args {
                if let Some(arg_node) = self.analyze_expr(arg) {
                    self.graph.add_edge(DataFlowEdge {
                        from: arg_node,
                        to: node_id.clone(),
                        kind: EdgeKind::Parameter { index: 0 },
                    });
                }
            }

            return Some(node_id);
        }

        // Regular function call
        let node_id = self.next_node_id();
        let node = DataFlowNode::Expression {
            kind: ExpressionKind::FunctionCall {
                function: func_str.clone(),
            },
            location: self.make_location(0),
            scope: self.current_scope.clone(),
        };
        self.graph.add_node(node_id.clone(), node);

        // Connect arguments
        for (i, arg) in call.args.iter().enumerate() {
            if let Some(arg_node) = self.analyze_expr(arg) {
                self.graph.add_edge(DataFlowEdge {
                    from: arg_node,
                    to: node_id.clone(),
                    kind: EdgeKind::Parameter { index: i },
                });
            }
        }

        Some(node_id)
    }

    fn analyze_path_expr(&mut self, path: &syn::ExprPath) -> Option<NodeId> {
        // Check if this is a variable reference
        if path.path.segments.len() == 1 {
            let var_name = path.path.segments[0].ident.to_string();

            // Look up variable in scope
            if let Some(var_node) = self.graph.find_variable(&self.current_scope, &var_name) {
                return Some(var_node.clone());
            }
        }

        None
    }

    fn analyze_field_access(&mut self, field: &ExprField) -> Option<NodeId> {
        let base_node = self.analyze_expr(&field.base)?;
        let field_name = field.member.to_token_stream().to_string();

        let node_id = self.next_node_id();
        let node = DataFlowNode::Expression {
            kind: ExpressionKind::FieldAccess {
                field: field_name.clone(),
            },
            location: self.make_location(0),
            scope: self.current_scope.clone(),
        };
        self.graph.add_node(node_id.clone(), node);

        self.graph.add_edge(DataFlowEdge {
            from: base_node,
            to: node_id.clone(),
            kind: EdgeKind::FieldAccess { field: field_name },
        });

        Some(node_id)
    }

    fn analyze_index_access(&mut self, index: &ExprIndex) -> Option<NodeId> {
        let base_node = self.analyze_expr(&index.expr)?;

        let node_id = self.next_node_id();
        let node = DataFlowNode::Expression {
            kind: ExpressionKind::ArrayAccess,
            location: self.make_location(0),
            scope: self.current_scope.clone(),
        };
        self.graph.add_node(node_id.clone(), node);

        self.graph.add_edge(DataFlowEdge {
            from: base_node,
            to: node_id.clone(),
            kind: EdgeKind::IndexAccess,
        });

        // Also analyze index expression
        self.analyze_expr(&index.index);

        Some(node_id)
    }

    fn analyze_assignment(&mut self, assign: &ExprAssign) -> Option<NodeId> {
        // Analyze right-hand side
        let rhs_node = self.analyze_expr(&assign.right)?;

        // Analyze left-hand side (create variable node if needed)
        if let Expr::Path(path) = &*assign.left {
            if path.path.segments.len() == 1 {
                let var_name = path.path.segments[0].ident.to_string();

                // Create or find variable node
                let var_node_id = if let Some(existing) =
                    self.graph.find_variable(&self.current_scope, &var_name)
                {
                    existing.clone()
                } else {
                    let node_id = self.next_node_id();
                    let node = DataFlowNode::Variable {
                        name: var_name.clone(),
                        location: self.make_location(0),
                        scope: self.current_scope.clone(),
                        is_parameter: false,
                    };
                    self.graph.add_node(node_id.clone(), node);
                    node_id
                };

                // Connect RHS to variable
                self.graph.add_edge(DataFlowEdge {
                    from: rhs_node,
                    to: var_node_id.clone(),
                    kind: EdgeKind::Assignment,
                });

                return Some(var_node_id);
            }
        }

        None
    }

    fn analyze_return(&mut self, ret: &ExprReturn) -> Option<NodeId> {
        if let Some(expr) = &ret.expr {
            let expr_node = self.analyze_expr(expr)?;

            if let Some(ref func_name) = self.current_function.clone() {
                let node_id = self.next_node_id();
                let node = DataFlowNode::Return {
                    function: func_name.clone(),
                    location: self.make_location(0),
                };
                self.graph.add_node(node_id.clone(), node);

                self.graph.add_edge(DataFlowEdge {
                    from: expr_node,
                    to: node_id.clone(),
                    kind: EdgeKind::Return,
                });

                return Some(node_id);
            }
        }
        None
    }

    fn analyze_if(&mut self, if_expr: &ExprIf) -> Option<NodeId> {
        // Analyze condition
        self.analyze_expr(&if_expr.cond);

        // Analyze then branch in new scope
        let then_scope = self.next_scope_id("if_then");
        self.graph
            .create_scope(then_scope.clone(), Some(self.current_scope.clone()));
        let prev_scope = std::mem::replace(&mut self.current_scope, then_scope);
        self.visit_block(&if_expr.then_branch);
        self.current_scope = prev_scope.clone();

        // Analyze else branch if present
        if let Some((_, else_expr)) = &if_expr.else_branch {
            let else_scope = self.next_scope_id("if_else");
            self.graph
                .create_scope(else_scope.clone(), Some(prev_scope.clone()));
            let prev_scope = std::mem::replace(&mut self.current_scope, else_scope);
            self.analyze_expr(else_expr);
            self.current_scope = prev_scope;
        }

        None
    }

    fn analyze_match(&mut self, match_expr: &ExprMatch) -> Option<NodeId> {
        // Analyze scrutinee
        self.analyze_expr(&match_expr.expr);

        // Analyze each arm in its own scope
        for arm in &match_expr.arms {
            let arm_scope = self.next_scope_id("match_arm");
            self.graph
                .create_scope(arm_scope.clone(), Some(self.current_scope.clone()));
            let prev_scope = std::mem::replace(&mut self.current_scope, arm_scope);

            // Analyze guard if present
            if let Some((_, guard)) = &arm.guard {
                self.analyze_expr(guard);
            }

            // Analyze arm body
            self.analyze_expr(&arm.body);

            self.current_scope = prev_scope;
        }

        None
    }

    fn analyze_loop(&mut self, loop_expr: &ExprLoop) -> Option<NodeId> {
        let loop_scope = self.next_scope_id("loop");
        self.graph
            .create_scope(loop_scope.clone(), Some(self.current_scope.clone()));
        let prev_scope = std::mem::replace(&mut self.current_scope, loop_scope);
        self.visit_block(&loop_expr.body);
        self.current_scope = prev_scope;
        None
    }

    fn analyze_while(&mut self, while_expr: &ExprWhile) -> Option<NodeId> {
        self.analyze_expr(&while_expr.cond);

        let while_scope = self.next_scope_id("while");
        self.graph
            .create_scope(while_scope.clone(), Some(self.current_scope.clone()));
        let prev_scope = std::mem::replace(&mut self.current_scope, while_scope);
        self.visit_block(&while_expr.body);
        self.current_scope = prev_scope;
        None
    }

    fn analyze_for(&mut self, for_expr: &ExprForLoop) -> Option<NodeId> {
        self.analyze_expr(&for_expr.expr);

        let for_scope = self.next_scope_id("for");
        self.graph
            .create_scope(for_scope.clone(), Some(self.current_scope.clone()));
        let prev_scope = std::mem::replace(&mut self.current_scope, for_scope);

        // Add loop variable to scope
        if let Pat::Ident(PatIdent { ident, .. }) = for_expr.pat.as_ref() {
            let var_name = ident.to_string();
            let node_id = self.next_node_id();
            let node = DataFlowNode::Variable {
                name: var_name.clone(),
                location: self.make_location(0),
                scope: self.current_scope.clone(),
                is_parameter: false,
            };
            self.graph.add_node(node_id, node);
        }

        self.visit_block(&for_expr.body);
        self.current_scope = prev_scope;
        None
    }

    /// Detect if a method call is an input source
    fn detect_source_method(&self, method: &str, receiver: &Expr) -> Option<InputSource> {
        let receiver_str = format!("{:?}", receiver);

        // Check for env::args() - args is a method on env module
        if method == "args" && receiver_str.contains("env") {
            return Some(InputSource::CliArgument);
        }

        // Check for env::var()
        if method == "var" && receiver_str.contains("env") {
            return Some(InputSource::Environment);
        }

        // File operations that read data
        if method == "read" || method == "read_to_string" || method == "read_line" {
            if receiver_str.contains("File") || receiver_str.contains("BufReader") {
                return Some(InputSource::FileInput);
            }
            if receiver_str.contains("stdin") {
                return Some(InputSource::UserInput);
            }
        }

        // Network operations
        if (method == "body" || method == "bytes" || method == "text")
            && (receiver_str.contains("Request") || receiver_str.contains("Response"))
        {
            return Some(InputSource::HttpRequest);
        }

        None
    }

    /// Detect if a function call is an input source
    fn detect_source_function(&self, func: &str) -> Option<InputSource> {
        // Only return Some if this is an actual read operation
        if func.contains("File::open") || func.contains("fs::read") {
            return Some(InputSource::FileInput);
        }

        if func.contains("env::args") {
            return Some(InputSource::CliArgument);
        }

        if func.contains("env::var") {
            return Some(InputSource::Environment);
        }

        if func.contains("stdin") {
            return Some(InputSource::UserInput);
        }

        None
    }

    /// Detect if a function call is a dangerous sink
    fn detect_sink_function(&self, func: &str) -> Option<SinkOperation> {
        if func.contains("File::create") || func.contains("fs::write") {
            return Some(SinkOperation::FileSystem);
        }

        if func.contains("Command::new") {
            return Some(SinkOperation::ProcessExecution);
        }

        None
    }

    /// Detect if a method call is a dangerous sink
    fn detect_sink_method(&self, method: &str) -> Option<SinkOperation> {
        if method == "execute" || method == "query" || method.contains("sql") {
            return Some(SinkOperation::SqlQuery);
        }

        // For process execution, both spawn and arg are dangerous when used with untrusted data
        if method == "spawn" || method == "output" || method == "status" || method == "arg" {
            return Some(SinkOperation::ProcessExecution);
        }

        if method == "write" || method == "write_all" || method == "create" {
            return Some(SinkOperation::FileSystem);
        }

        if method == "deserialize" || method == "from_str" || method == "from_slice" {
            return Some(SinkOperation::Deserialization);
        }

        None
    }

    /// Check if a method is a validation/sanitization method
    fn is_validation_method(&self, method: &str) -> bool {
        method.contains("validate")
            || method.contains("sanitize")
            || method.contains("escape")
            || method.contains("clean")
            || method.contains("verify")
            || method.contains("check")
            || (method == "parse" || method == "from_str") // Parsing with error handling is validation
    }

    fn analyze_function_signature(&mut self, sig: &Signature) {
        // Add parameters as nodes
        for (i, input) in sig.inputs.iter().enumerate() {
            if let FnArg::Typed(PatType { pat, .. }) = input {
                if let Pat::Ident(PatIdent { ident, .. }) = &**pat {
                    let param_name = ident.to_string();
                    let node_id = self.next_node_id();
                    let node = DataFlowNode::Parameter {
                        function: self.current_function.clone().unwrap_or_default(),
                        index: i,
                        name: param_name.clone(),
                        location: self.make_location(0),
                    };
                    self.graph.add_node(node_id.clone(), node);

                    // Also add as variable in scope
                    let var_node_id = self.next_node_id();
                    let var_node = DataFlowNode::Variable {
                        name: param_name.clone(),
                        location: self.make_location(0),
                        scope: self.current_scope.clone(),
                        is_parameter: true,
                    };
                    self.graph.add_node(var_node_id.clone(), var_node);

                    // Connect parameter to variable
                    self.graph.add_edge(DataFlowEdge {
                        from: node_id,
                        to: var_node_id,
                        kind: EdgeKind::Parameter { index: i },
                    });
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for DataFlowBuilder {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let func_name = node.sig.ident.to_string();
        let func_scope = self.next_scope_id(&func_name);
        self.graph
            .create_scope(func_scope.clone(), Some(self.current_scope.clone()));

        let prev_func = self.current_function.clone();
        let prev_scope = std::mem::replace(&mut self.current_scope, func_scope);
        self.current_function = Some(func_name);

        // Analyze function signature
        self.analyze_function_signature(&node.sig);

        // Visit function body
        syn::visit::visit_item_fn(self, node);

        self.current_function = prev_func;
        self.current_scope = prev_scope;
    }

    fn visit_block(&mut self, node: &'ast Block) {
        for stmt in &node.stmts {
            self.visit_stmt(stmt);
        }
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        match stmt {
            Stmt::Local(local) => self.visit_local(local),
            Stmt::Expr(expr, _) => {
                self.analyze_expr(expr);
            }
            Stmt::Macro(_) => {
                // Skip macro statements for now
            }
            _ => syn::visit::visit_stmt(self, stmt),
        }
    }

    fn visit_local(&mut self, local: &'ast Local) {
        // Analyze initializer first
        let init_node = if let Some(init) = &local.init {
            self.analyze_expr(&init.expr)
        } else {
            None
        };

        // Create variable nodes
        if let Pat::Ident(PatIdent { ident, .. }) = &local.pat {
            let var_name = ident.to_string();
            let node_id = self.next_node_id();
            let node = DataFlowNode::Variable {
                name: var_name.clone(),
                location: self.make_location(0),
                scope: self.current_scope.clone(),
                is_parameter: false,
            };
            self.graph.add_node(node_id.clone(), node);

            // Connect initializer to variable if present
            if let Some(init) = init_node {
                self.graph.add_edge(DataFlowEdge {
                    from: init,
                    to: node_id,
                    kind: EdgeKind::Assignment,
                });
            }
        }

        syn::visit::visit_local(self, local);
    }
}

impl Default for DataFlowBuilder {
    fn default() -> Self {
        Self::new()
    }
}
