use crate::common::SourceLocation;
use std::collections::HashMap;
use syn::Expr;

/// Unique identifier for a function in the collected data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FunctionId(pub usize);

/// Unique identifier for a loop in the collected data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoopId(pub usize);

/// Unique identifier for an operation in the collected data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationId(pub usize);

/// Type of loop construct
#[derive(Debug, Clone, PartialEq)]
pub enum LoopType {
    For,
    While,
    Loop,
    Iterator, // .iter(), .into_iter(), etc.
}

/// Type of I/O operation
#[derive(Debug, Clone, PartialEq)]
pub enum IOType {
    FileRead,
    FileWrite,
    NetworkRequest,
    DatabaseQuery,
    ProcessSpawn,
    Sync,  // Synchronous I/O
    Async, // Async I/O
}

/// Type of allocation operation
#[derive(Debug, Clone, PartialEq)]
pub enum AllocationType {
    Clone,
    StringConcat,
    VecNew,
    BoxNew,
    ToString,
    Collect,
    Format,
}

/// Type of string operation
#[derive(Debug, Clone, PartialEq)]
pub enum StringOperationType {
    Concatenation,
    Format,
    RegexCompile,
    Parse,
    Split,
    Replace,
}

/// Type of data structure operation
#[derive(Debug, Clone, PartialEq)]
pub enum DataStructureOpType {
    VecContains,
    VecLinearSearch,
    VecInsert,
    VecRemove,
    HashMapGet,
    HashSetContains,
    BTreeMapRange,
}

/// Information about a function in the AST
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub id: FunctionId,
    pub name: String,
    pub location: SourceLocation,
    pub span: (usize, usize), // line range
    pub is_test: bool,
    pub is_async: bool,
    pub body_span: proc_macro2::Span,
}

/// Information about a loop in the AST
#[derive(Debug, Clone)]
pub struct LoopInfo {
    pub id: LoopId,
    pub loop_type: LoopType,
    pub location: SourceLocation,
    pub nesting_level: usize,
    pub containing_function: Option<FunctionId>,
    pub parent_loop: Option<LoopId>,
    pub operations: Vec<OperationId>,
    pub is_iterator_chain: bool,
    pub has_early_exit: bool,
}

/// Context information for any operation
#[derive(Debug, Clone)]
pub struct OperationContext {
    pub location: SourceLocation,
    pub containing_function: Option<FunctionId>,
    pub containing_loops: Vec<LoopId>,
    pub loop_depth: usize,
    pub in_conditional: bool,
    pub in_error_handler: bool,
    pub in_async_block: bool,
}

/// Information about an I/O operation
#[derive(Debug, Clone)]
pub struct IOOperation {
    pub id: OperationId,
    pub operation_type: IOType,
    pub context: OperationContext,
    pub is_async: bool,
    pub is_buffered: bool,
    pub method_name: String,
    pub expr: Option<Box<Expr>>, // Store the expression for further analysis
}

/// Information about a memory allocation
#[derive(Debug, Clone)]
pub struct AllocationInfo {
    pub id: OperationId,
    pub allocation_type: AllocationType,
    pub context: OperationContext,
    pub is_in_hot_path: bool,
    pub estimated_size: Option<usize>,
    pub expr: Option<Box<Expr>>,
}

/// Information about a string operation
#[derive(Debug, Clone)]
pub struct StringOperation {
    pub id: OperationId,
    pub operation_type: StringOperationType,
    pub context: OperationContext,
    pub is_repeated: bool,
    pub expr: Option<Box<Expr>>,
}

/// Information about a data structure operation
#[derive(Debug, Clone)]
pub struct DataStructureOp {
    pub id: OperationId,
    pub operation_type: DataStructureOpType,
    pub context: OperationContext,
    pub collection_type: String,
    pub is_in_hot_path: bool,
    pub expr: Option<Box<Expr>>,
}

/// Information about a function or method call
#[derive(Debug, Clone)]
pub struct CallSite {
    pub id: OperationId,
    pub function_name: String,
    pub context: OperationContext,
    pub is_method_call: bool,
    pub receiver_type: Option<String>,
    pub args_count: usize,
}

/// All performance-relevant data collected from a single file
#[derive(Debug, Clone)]
pub struct CollectedPerformanceData {
    /// All functions found in the file
    pub functions: Vec<FunctionInfo>,

    /// All loops found in the file
    pub loops: Vec<LoopInfo>,

    /// All I/O operations found
    pub io_operations: Vec<IOOperation>,

    /// All memory allocations found
    pub allocations: Vec<AllocationInfo>,

    /// All string operations found
    pub string_operations: Vec<StringOperation>,

    /// All data structure operations found
    pub data_structure_ops: Vec<DataStructureOp>,

    /// All function/method calls found
    pub call_sites: Vec<CallSite>,

    /// Mapping from function names to IDs for quick lookup
    pub function_by_name: HashMap<String, FunctionId>,

    /// Mapping from loop IDs to their nested loops
    pub nested_loops: HashMap<LoopId, Vec<LoopId>>,

    /// Source file content for location extraction
    pub source_content: String,

    /// File path for reference
    pub file_path: std::path::PathBuf,
}

impl CollectedPerformanceData {
    pub fn new(source_content: String, file_path: std::path::PathBuf) -> Self {
        Self {
            functions: Vec::new(),
            loops: Vec::new(),
            io_operations: Vec::new(),
            allocations: Vec::new(),
            string_operations: Vec::new(),
            data_structure_ops: Vec::new(),
            call_sites: Vec::new(),
            function_by_name: HashMap::new(),
            nested_loops: HashMap::new(),
            source_content,
            file_path,
        }
    }

    /// Get function by ID
    pub fn get_function(&self, id: FunctionId) -> Option<&FunctionInfo> {
        self.functions.get(id.0)
    }

    /// Get loop by ID
    pub fn get_loop(&self, id: LoopId) -> Option<&LoopInfo> {
        self.loops.get(id.0)
    }

    /// Get all loops contained within a function
    pub fn get_function_loops(&self, function_id: FunctionId) -> Vec<&LoopInfo> {
        self.loops
            .iter()
            .filter(|l| l.containing_function == Some(function_id))
            .collect()
    }

    /// Get all operations within a loop
    pub fn get_loop_operations(&self, loop_id: LoopId) -> LoopOperations {
        let _loop_info = match self.get_loop(loop_id) {
            Some(l) => l,
            None => return LoopOperations::default(),
        };

        let mut ops = LoopOperations::default();

        // Collect I/O operations in this loop
        for io_op in &self.io_operations {
            if io_op.context.containing_loops.contains(&loop_id) {
                ops.io_operations.push(io_op.clone());
            }
        }

        // Collect allocations in this loop
        for alloc in &self.allocations {
            if alloc.context.containing_loops.contains(&loop_id) {
                ops.allocations.push(alloc.clone());
            }
        }

        // Collect string operations in this loop
        for str_op in &self.string_operations {
            if str_op.context.containing_loops.contains(&loop_id) {
                ops.string_operations.push(str_op.clone());
            }
        }

        // Collect data structure operations in this loop
        for ds_op in &self.data_structure_ops {
            if ds_op.context.containing_loops.contains(&loop_id) {
                ops.data_structure_ops.push(ds_op.clone());
            }
        }

        ops
    }

    /// Calculate maximum nesting depth in the file
    pub fn max_nesting_depth(&self) -> usize {
        self.loops
            .iter()
            .map(|l| l.nesting_level)
            .max()
            .unwrap_or(0)
    }

    /// Get loops with specific nesting level
    pub fn get_loops_by_nesting(&self, level: usize) -> Vec<&LoopInfo> {
        self.loops
            .iter()
            .filter(|l| l.nesting_level == level)
            .collect()
    }

    /// Check if a function contains any I/O operations
    pub fn function_has_io(&self, function_id: FunctionId) -> bool {
        self.io_operations
            .iter()
            .any(|op| op.context.containing_function == Some(function_id))
    }

    /// Get hot path operations (operations in nested loops)
    pub fn get_hot_path_operations(&self) -> HotPathOperations {
        let mut hot_ops = HotPathOperations::default();

        // Operations in loops with depth > 1 are considered hot path
        for io_op in &self.io_operations {
            if io_op.context.loop_depth > 1 {
                hot_ops.io_operations.push(io_op.clone());
            }
        }

        for alloc in &self.allocations {
            if alloc.context.loop_depth > 1 {
                hot_ops.allocations.push(alloc.clone());
            }
        }

        for str_op in &self.string_operations {
            if str_op.context.loop_depth > 1 {
                hot_ops.string_operations.push(str_op.clone());
            }
        }

        for ds_op in &self.data_structure_ops {
            if ds_op.context.loop_depth > 1 {
                hot_ops.data_structure_ops.push(ds_op.clone());
            }
        }

        hot_ops
    }
}

/// Operations found within a specific loop
#[derive(Debug, Default, Clone)]
pub struct LoopOperations {
    pub io_operations: Vec<IOOperation>,
    pub allocations: Vec<AllocationInfo>,
    pub string_operations: Vec<StringOperation>,
    pub data_structure_ops: Vec<DataStructureOp>,
}

/// Operations found in hot paths (nested loops)
#[derive(Debug, Default, Clone)]
pub struct HotPathOperations {
    pub io_operations: Vec<IOOperation>,
    pub allocations: Vec<AllocationInfo>,
    pub string_operations: Vec<StringOperation>,
    pub data_structure_ops: Vec<DataStructureOp>,
}
