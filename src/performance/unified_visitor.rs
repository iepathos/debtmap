use crate::performance::collected_data::*;
use crate::performance::LocationExtractor;
use std::path::Path;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Block, Expr, ExprCall, ExprMethodCall, File, ItemFn, Stmt};

/// A visitor that collects all performance-relevant data in a single AST traversal
pub struct UnifiedPerformanceVisitor {
    /// The collected data
    data: CollectedPerformanceData,

    /// Location extractor for accurate line numbers
    location_extractor: LocationExtractor,

    /// Current function context
    current_function: Option<FunctionId>,

    /// Stack of loops we're currently inside
    loop_stack: Vec<LoopId>,

    /// Current block depth for nesting calculation
    block_depth: usize,

    /// Whether we're in a conditional block
    in_conditional: bool,

    /// Whether we're in an error handler (catch, match Err, etc.)
    in_error_handler: bool,

    /// Whether we're in an async block
    in_async_block: bool,

    /// Counter for generating unique IDs
    next_operation_id: usize,
}

impl UnifiedPerformanceVisitor {
    pub fn new(source_content: String, file_path: &Path) -> Self {
        let location_extractor = LocationExtractor::new(&source_content);
        let data = CollectedPerformanceData::new(source_content, file_path.to_path_buf());

        Self {
            data,
            location_extractor,
            current_function: None,
            loop_stack: Vec::new(),
            block_depth: 0,
            in_conditional: false,
            in_error_handler: false,
            in_async_block: false,
            next_operation_id: 0,
        }
    }

    /// Consume the visitor and return the collected data
    pub fn into_collected_data(self) -> CollectedPerformanceData {
        self.data
    }

    /// Get current operation context
    fn current_context<T: Spanned>(&self, node: &T) -> OperationContext {
        let location = self.location_extractor.extract_location(node);

        OperationContext {
            location,
            containing_function: self.current_function,
            containing_loops: self.loop_stack.clone(),
            loop_depth: self.loop_stack.len(),
            in_conditional: self.in_conditional,
            in_error_handler: self.in_error_handler,
            in_async_block: self.in_async_block,
        }
    }

    /// Generate next operation ID
    fn next_operation_id(&mut self) -> OperationId {
        let id = OperationId(self.next_operation_id);
        self.next_operation_id += 1;
        id
    }

    /// Check if an expression is an I/O operation
    fn check_io_operation(&mut self, expr: &Expr) {
        match expr {
            Expr::MethodCall(method_call) => {
                let method_name = method_call.method.to_string();
                let io_type = self.classify_io_operation(&method_name, method_call);

                if let Some(io_type) = io_type {
                    let is_async = self.is_async_operation(&method_name);
                    let is_buffered = self.is_buffered_operation(&method_name, method_call);

                    let io_op = IOOperation {
                        id: self.next_operation_id(),
                        operation_type: io_type,
                        context: self.current_context(method_call),
                        is_async,
                        is_buffered,
                        method_name,
                        expr: Some(Box::new(expr.clone())),
                    };

                    self.data.io_operations.push(io_op);
                }
            }
            Expr::Call(call) => {
                if let Some(io_type) = self.classify_io_call(call) {
                    let io_op = IOOperation {
                        id: self.next_operation_id(),
                        operation_type: io_type,
                        context: self.current_context(call),
                        is_async: false,
                        is_buffered: false,
                        method_name: self.get_call_name(call),
                        expr: Some(Box::new(expr.clone())),
                    };

                    self.data.io_operations.push(io_op);
                }
            }
            _ => {}
        }
    }

    /// Check if an expression is an allocation
    fn check_allocation(&mut self, expr: &Expr) {
        match expr {
            Expr::MethodCall(method_call) => {
                let method_name = method_call.method.to_string();
                let alloc_type = match method_name.as_str() {
                    "clone" | "cloned" => Some(AllocationType::Clone),
                    "to_string" | "to_owned" => Some(AllocationType::ToString),
                    "collect" => Some(AllocationType::Collect),
                    _ => None,
                };

                if let Some(alloc_type) = alloc_type {
                    let alloc = AllocationInfo {
                        id: self.next_operation_id(),
                        allocation_type: alloc_type,
                        context: self.current_context(method_call),
                        is_in_hot_path: self.loop_stack.len() > 1,
                        estimated_size: None,
                        expr: Some(Box::new(expr.clone())),
                    };

                    self.data.allocations.push(alloc);
                }
            }
            Expr::Call(call) => {
                let call_name = self.get_call_name(call);
                let alloc_type = match call_name.as_str() {
                    "Vec::new" | "Vec::with_capacity" => Some(AllocationType::VecNew),
                    "Box::new" => Some(AllocationType::BoxNew),
                    "format!" => Some(AllocationType::Format),
                    _ => None,
                };

                if let Some(alloc_type) = alloc_type {
                    let alloc = AllocationInfo {
                        id: self.next_operation_id(),
                        allocation_type: alloc_type,
                        context: self.current_context(call),
                        is_in_hot_path: self.loop_stack.len() > 1,
                        estimated_size: None,
                        expr: Some(Box::new(expr.clone())),
                    };

                    self.data.allocations.push(alloc);
                }
            }
            Expr::Binary(binary) if matches!(binary.op, syn::BinOp::Add(_)) => {
                // Check for string concatenation
                self.check_string_concat(expr);
            }
            _ => {}
        }
    }

    /// Check if an expression is a string operation
    fn check_string_operation(&mut self, expr: &Expr) {
        match expr {
            Expr::MethodCall(method_call) => {
                let method_name = method_call.method.to_string();
                let str_type = match method_name.as_str() {
                    "split" | "splitn" | "split_whitespace" => Some(StringOperationType::Split),
                    "replace" | "replacen" => Some(StringOperationType::Replace),
                    "parse" => Some(StringOperationType::Parse),
                    _ => None,
                };

                if let Some(str_type) = str_type {
                    let str_op = StringOperation {
                        id: self.next_operation_id(),
                        operation_type: str_type,
                        context: self.current_context(method_call),
                        is_repeated: !self.loop_stack.is_empty(),
                        expr: Some(Box::new(expr.clone())),
                    };

                    self.data.string_operations.push(str_op);
                }
            }
            Expr::Macro(mac) => {
                if mac.mac.path.is_ident("format") || mac.mac.path.is_ident("println") {
                    let str_op = StringOperation {
                        id: self.next_operation_id(),
                        operation_type: StringOperationType::Format,
                        context: self.current_context(mac),
                        is_repeated: !self.loop_stack.is_empty(),
                        expr: Some(Box::new(expr.clone())),
                    };

                    self.data.string_operations.push(str_op);
                }
            }
            _ => {}
        }
    }

    /// Check for string concatenation
    fn check_string_concat(&mut self, expr: &Expr) {
        if let Expr::Binary(binary) = expr {
            if matches!(binary.op, syn::BinOp::Add(_)) {
                // Simple heuristic: if in a loop, it's likely string concatenation
                if !self.loop_stack.is_empty() {
                    let str_op = StringOperation {
                        id: self.next_operation_id(),
                        operation_type: StringOperationType::Concatenation,
                        context: self.current_context(binary),
                        is_repeated: true,
                        expr: Some(Box::new(expr.clone())),
                    };

                    self.data.string_operations.push(str_op);

                    // Also record as allocation
                    let alloc = AllocationInfo {
                        id: self.next_operation_id(),
                        allocation_type: AllocationType::StringConcat,
                        context: self.current_context(binary),
                        is_in_hot_path: self.loop_stack.len() > 1,
                        estimated_size: None,
                        expr: Some(Box::new(expr.clone())),
                    };

                    self.data.allocations.push(alloc);
                }
            }
        }
    }

    /// Check if an expression is a data structure operation
    fn check_data_structure_operation(&mut self, expr: &Expr) {
        if let Expr::MethodCall(method_call) = expr {
            let method_name = method_call.method.to_string();
            let ds_type = match method_name.as_str() {
                "contains" => Some(DataStructureOpType::VecContains),
                "find" | "position" => Some(DataStructureOpType::VecLinearSearch),
                "insert" if self.is_vec_operation(&method_call.receiver) => {
                    Some(DataStructureOpType::VecInsert)
                }
                "remove" if self.is_vec_operation(&method_call.receiver) => {
                    Some(DataStructureOpType::VecRemove)
                }
                "get" => Some(DataStructureOpType::HashMapGet),
                "range" => Some(DataStructureOpType::BTreeMapRange),
                _ => None,
            };

            if let Some(ds_type) = ds_type {
                let collection_type = self.infer_collection_type(&method_call.receiver);

                let ds_op = DataStructureOp {
                    id: self.next_operation_id(),
                    operation_type: ds_type,
                    context: self.current_context(method_call),
                    collection_type,
                    is_in_hot_path: self.loop_stack.len() > 1,
                    expr: Some(Box::new(expr.clone())),
                };

                self.data.data_structure_ops.push(ds_op);
            }
        }
    }

    /// Classify I/O operation type from method name
    fn classify_io_operation(&self, method_name: &str, _call: &ExprMethodCall) -> Option<IOType> {
        match method_name {
            "read" | "read_to_string" | "read_to_end" | "read_line" => Some(IOType::FileRead),
            "write" | "write_all" | "flush" => Some(IOType::FileWrite),
            "send" | "recv" | "connect" | "accept" => Some(IOType::NetworkRequest),
            "query" | "execute" | "fetch" | "insert" | "update" | "delete" => {
                Some(IOType::DatabaseQuery)
            }
            "spawn" | "output" => Some(IOType::ProcessSpawn),
            _ => None,
        }
    }

    /// Classify I/O call type
    fn classify_io_call(&self, call: &ExprCall) -> Option<IOType> {
        let call_name = self.get_call_name(call);
        match call_name.as_str() {
            "std::fs::read" | "std::fs::read_to_string" => Some(IOType::FileRead),
            "std::fs::write" | "std::fs::create" => Some(IOType::FileWrite),
            "std::process::Command::new" => Some(IOType::ProcessSpawn),
            _ => None,
        }
    }

    /// Check if operation is async
    fn is_async_operation(&self, method_name: &str) -> bool {
        method_name.ends_with("_async") || method_name.starts_with("async_")
    }

    /// Check if I/O is buffered
    fn is_buffered_operation(&self, method_name: &str, _call: &ExprMethodCall) -> bool {
        method_name.contains("buffered") || method_name.contains("buf_")
    }

    /// Check if this is a Vec operation
    fn is_vec_operation(&self, _receiver: &Expr) -> bool {
        // Simplified heuristic - would need type information for accuracy
        true
    }

    /// Infer collection type from receiver expression
    fn infer_collection_type(&self, _receiver: &Expr) -> String {
        // Simplified - would need type information for accuracy
        "Vec".to_string()
    }

    /// Get function/method call name
    fn get_call_name(&self, call: &ExprCall) -> String {
        match &*call.func {
            Expr::Path(path) => path
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::"),
            _ => "<unknown>".to_string(),
        }
    }
}

impl<'ast> Visit<'ast> for UnifiedPerformanceVisitor {
    fn visit_item_fn(&mut self, func: &'ast ItemFn) {
        // Record function information
        let func_id = FunctionId(self.data.functions.len());
        let location = self.location_extractor.extract_location(func);

        let is_test = func.attrs.iter().any(|attr| {
            attr.path().is_ident("test")
                || attr.path().is_ident("tokio::test")
                || attr.path().is_ident("async_std::test")
        });

        let func_info = FunctionInfo {
            id: func_id,
            name: func.sig.ident.to_string(),
            location: location.clone(),
            span: (location.line, location.end_line.unwrap_or(location.line)),
            is_test,
            is_async: func.sig.asyncness.is_some(),
            body_span: func.block.span(),
        };

        self.data
            .function_by_name
            .insert(func_info.name.clone(), func_id);
        self.data.functions.push(func_info);

        // Set current function context
        let prev_function = self.current_function;
        self.current_function = Some(func_id);

        // Visit function body
        self.visit_block(&func.block);

        // Restore previous context
        self.current_function = prev_function;
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        // Check for different types of operations
        self.check_io_operation(expr);
        self.check_allocation(expr);
        self.check_string_operation(expr);
        self.check_data_structure_operation(expr);

        // Handle loop constructs
        match expr {
            Expr::ForLoop(for_loop) => {
                let loop_id = LoopId(self.data.loops.len());
                let location = self.location_extractor.extract_location(for_loop);

                let loop_info = LoopInfo {
                    id: loop_id,
                    loop_type: LoopType::For,
                    location,
                    nesting_level: self.loop_stack.len() + 1,
                    containing_function: self.current_function,
                    parent_loop: self.loop_stack.last().copied(),
                    operations: Vec::new(),
                    is_iterator_chain: false,
                    has_early_exit: self.check_early_exit(&for_loop.body),
                };

                // Record nested loop relationship
                if let Some(parent) = self.loop_stack.last() {
                    self.data
                        .nested_loops
                        .entry(*parent)
                        .or_default()
                        .push(loop_id);
                }

                self.data.loops.push(loop_info);
                self.loop_stack.push(loop_id);

                // Visit loop body
                visit::visit_expr(self, expr);

                self.loop_stack.pop();
                return;
            }
            Expr::While(while_loop) => {
                let loop_id = LoopId(self.data.loops.len());
                let location = self.location_extractor.extract_location(while_loop);

                let loop_info = LoopInfo {
                    id: loop_id,
                    loop_type: LoopType::While,
                    location,
                    nesting_level: self.loop_stack.len() + 1,
                    containing_function: self.current_function,
                    parent_loop: self.loop_stack.last().copied(),
                    operations: Vec::new(),
                    is_iterator_chain: false,
                    has_early_exit: self.check_early_exit(&while_loop.body),
                };

                if let Some(parent) = self.loop_stack.last() {
                    self.data
                        .nested_loops
                        .entry(*parent)
                        .or_default()
                        .push(loop_id);
                }

                self.data.loops.push(loop_info);
                self.loop_stack.push(loop_id);

                visit::visit_expr(self, expr);

                self.loop_stack.pop();
                return;
            }
            Expr::Loop(loop_expr) => {
                let loop_id = LoopId(self.data.loops.len());
                let location = self.location_extractor.extract_location(loop_expr);

                let loop_info = LoopInfo {
                    id: loop_id,
                    loop_type: LoopType::Loop,
                    location,
                    nesting_level: self.loop_stack.len() + 1,
                    containing_function: self.current_function,
                    parent_loop: self.loop_stack.last().copied(),
                    operations: Vec::new(),
                    is_iterator_chain: false,
                    has_early_exit: self.check_early_exit(&loop_expr.body),
                };

                if let Some(parent) = self.loop_stack.last() {
                    self.data
                        .nested_loops
                        .entry(*parent)
                        .or_default()
                        .push(loop_id);
                }

                self.data.loops.push(loop_info);
                self.loop_stack.push(loop_id);

                visit::visit_expr(self, expr);

                self.loop_stack.pop();
                return;
            }
            Expr::MethodCall(method_call) => {
                // Check for iterator chains that act like loops
                let method_name = method_call.method.to_string();
                if matches!(method_name.as_str(), "for_each" | "map" | "filter_map") {
                    let loop_id = LoopId(self.data.loops.len());
                    let location = self.location_extractor.extract_location(method_call);

                    let loop_info = LoopInfo {
                        id: loop_id,
                        loop_type: LoopType::Iterator,
                        location,
                        nesting_level: self.loop_stack.len() + 1,
                        containing_function: self.current_function,
                        parent_loop: self.loop_stack.last().copied(),
                        operations: Vec::new(),
                        is_iterator_chain: true,
                        has_early_exit: false,
                    };

                    if let Some(parent) = self.loop_stack.last() {
                        self.data
                            .nested_loops
                            .entry(*parent)
                            .or_default()
                            .push(loop_id);
                    }

                    self.data.loops.push(loop_info);
                    self.loop_stack.push(loop_id);

                    visit::visit_expr(self, expr);

                    self.loop_stack.pop();
                    return;
                }
            }
            Expr::If(_) => {
                let prev_conditional = self.in_conditional;
                self.in_conditional = true;
                visit::visit_expr(self, expr);
                self.in_conditional = prev_conditional;
                return;
            }
            Expr::Match(_match_expr) => {
                // Check if this is error handling
                let prev_error_handler = self.in_error_handler;
                // Simple heuristic: if matching on Result or Option
                self.in_error_handler = true; // Simplified
                visit::visit_expr(self, expr);
                self.in_error_handler = prev_error_handler;
                return;
            }
            Expr::Async(_) => {
                let prev_async = self.in_async_block;
                self.in_async_block = true;
                visit::visit_expr(self, expr);
                self.in_async_block = prev_async;
                return;
            }
            _ => {}
        }

        // Continue visiting
        visit::visit_expr(self, expr);
    }

    fn visit_block(&mut self, block: &'ast Block) {
        self.block_depth += 1;
        visit::visit_block(self, block);
        self.block_depth -= 1;
    }
}

impl UnifiedPerformanceVisitor {
    /// Check if a block has early exit (break, continue, return)
    fn check_early_exit(&self, block: &Block) -> bool {
        for stmt in &block.stmts {
            if let Stmt::Expr(expr, _) = stmt {
                if self.is_early_exit(expr) {
                    return true;
                }
            }
        }
        false
    }

    /// Check if expression is an early exit
    fn is_early_exit(&self, expr: &Expr) -> bool {
        matches!(expr, Expr::Break(_) | Expr::Continue(_) | Expr::Return(_))
    }
}

/// Collect all performance data from a file in a single pass
pub fn collect_performance_data(
    file: &File,
    path: &Path,
    source_content: &str,
) -> CollectedPerformanceData {
    let mut visitor = UnifiedPerformanceVisitor::new(source_content.to_string(), path);
    visitor.visit_file(file);
    visitor.into_collected_data()
}
