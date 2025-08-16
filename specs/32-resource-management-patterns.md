---
number: 32
title: Resource Management Patterns Detection
category: feature
priority: medium
status: draft
dependencies: []
created: 2025-08-16
---

# Specification 32: Resource Management Patterns Detection

**Category**: feature
**Priority**: medium
**Status**: draft
**Dependencies**: []

## Context

Resource management is critical in systems programming, and poor resource handling patterns create technical debt that can lead to memory leaks, resource exhaustion, and performance degradation. While Rust's ownership system prevents many resource management issues, certain patterns still represent technical debt:

- **Missing Drop Implementation** - Types that hold resources but don't implement proper cleanup
- **Resource Leaks in Async Code** - Resources not properly cleaned up in async contexts
- **Unbounded Collections** - Collections that can grow without limits
- **File Handle Leaks** - Files, network connections not properly closed
- **Memory Pool Misuse** - Inefficient use of custom allocators or object pools
- **RAII Violations** - Resources not following Resource Acquisition Is Initialization patterns

These patterns represent technical debt that can lead to resource exhaustion and system instability in production environments.

## Objective

Implement resource management analysis that identifies patterns affecting resource safety and efficiency by:

1. **Resource Lifecycle Analysis**: Track resource acquisition and release patterns
2. **Drop Implementation Detection**: Identify types that need cleanup but lack Drop implementations
3. **Async Resource Management**: Detect resource handling issues in async code
4. **Collection Growth Analysis**: Find unbounded collection growth patterns
5. **Handle Leak Detection**: Identify file, network, and system resource leaks

## Requirements

### Functional Requirements

1. **Drop Implementation Analysis**
   - Identify types holding resources without Drop implementations
   - Detect types with manual cleanup methods but no Drop
   - Find resource-holding fields in types without proper cleanup
   - Analyze Drop implementation correctness

2. **Resource Lifecycle Tracking**
   - Track file handle acquisition and release
   - Monitor network connection management
   - Detect memory allocation without corresponding deallocation
   - Identify database connection leaks

3. **Async Resource Management**
   - Detect resources not properly cleaned up in async contexts
   - Find .await points that could leak resources on cancellation
   - Identify async functions without proper resource cleanup
   - Detect shared resources without proper synchronization

4. **Unbounded Collection Detection**
   - Find collections that grow without bounds checking
   - Detect caches without eviction policies
   - Identify accumulating data structures in long-running processes
   - Find recursive data structures without termination

5. **RAII Pattern Compliance**
   - Verify resource acquisition in constructors
   - Ensure resource release in destructors
   - Detect resource sharing without proper reference counting
   - Identify manual resource management that should use RAII

### Non-Functional Requirements

1. **Performance**
   - Resource analysis adds <15% overhead to total analysis time
   - Efficient resource flow tracking using dataflow analysis
   - Scalable to large codebases with complex resource usage

2. **Accuracy**
   - >85% precision for resource management issue detection
   - >75% recall for significant resource leaks
   - Minimal false positives for idiomatic Rust patterns

3. **Integration**
   - Works with existing async runtimes (tokio, async-std)
   - Integrates with custom allocator patterns
   - Supports common resource management crates

## Acceptance Criteria

- [ ] **Missing Drop Detection**: Types holding resources without Drop implementation identified
- [ ] **Resource Leak Detection**: File, network, and memory leaks flagged with specific locations
- [ ] **Async Resource Issues**: Resource management problems in async code detected
- [ ] **Unbounded Collections**: Collections without growth limits identified
- [ ] **RAII Compliance**: Violations of RAII patterns flagged with corrections
- [ ] **Resource Flow Analysis**: Complete tracking of resource acquisition and release
- [ ] **Cleanup Suggestions**: Specific recommendations for each resource management issue
- [ ] **Framework Integration**: Works with common Rust frameworks and libraries

## Technical Details

### Implementation Approach

#### 1. Resource Management Analysis Framework (`src/resource/`)

```rust
/// Resource management pattern detection framework
pub mod resource {
    use crate::core::ast::AstNode;
    use crate::core::{DebtItem, Priority};
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum ResourceManagementIssue {
        MissingDrop {
            type_name: String,
            resource_fields: Vec<ResourceField>,
            suggested_drop_impl: String,
            severity: ResourceSeverity,
        },
        ResourceLeak {
            resource_type: ResourceType,
            acquisition_site: SourceLocation,
            leak_site: SourceLocation,
            cleanup_suggestion: String,
        },
        AsyncResourceIssue {
            function_name: String,
            issue_type: AsyncResourceIssueType,
            cancellation_safety: CancellationSafety,
            mitigation_strategy: String,
        },
        UnboundedCollection {
            collection_name: String,
            collection_type: String,
            growth_pattern: GrowthPattern,
            bounding_strategy: BoundingStrategy,
        },
        RaiiViolation {
            violation_type: RaiiViolationType,
            resource_involved: String,
            correct_pattern: String,
        },
        HandleLeak {
            handle_type: HandleType,
            leak_location: SourceLocation,
            proper_cleanup: String,
        },
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum ResourceType {
        FileHandle,
        NetworkConnection,
        DatabaseConnection,
        MemoryAllocation,
        SystemHandle,
        ThreadHandle,
        Mutex,
        Channel,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum AsyncResourceIssueType {
        ResourceNotCleaned,
        CancellationUnsafe,
        SharedResourceRace,
        DropInAsync,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum GrowthPattern {
        UnboundedInsertion,
        NoEviction,
        MemoryAccumulation,
        RecursiveGrowth,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum BoundingStrategy {
        SizeLimit,
        TimeBasedEviction,
        LruEviction,
        CapacityCheck,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum RaiiViolationType {
        ManualResourceManagement,
        ResourceNotInConstructor,
        NoResourceRelease,
        PartialCleanup,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum HandleType {
        File,
        Socket,
        Process,
        Thread,
        Timer,
        Device,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum ResourceSeverity {
        Critical, // System resources that can cause crashes
        High,     // Important resources that affect performance
        Medium,   // Resources with moderate impact
        Low,      // Resources with minimal impact
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum CancellationSafety {
        Safe,       // Resource properly cleaned up on cancellation
        Unsafe,     // Resource may leak on cancellation
        Unknown,    // Cannot determine safety
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub struct ResourceField {
        pub field_name: String,
        pub field_type: String,
        pub is_owning: bool,
        pub cleanup_required: bool,
    }
    
    pub trait ResourceDetector {
        fn detect_issues(&self, ast: &AstNode) -> Vec<ResourceManagementIssue>;
        fn detector_name(&self) -> &'static str;
        fn assess_resource_impact(&self, issue: &ResourceManagementIssue) -> ResourceImpact;
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum ResourceImpact {
        Critical,  // Can cause system failure
        High,      // Significant resource waste
        Medium,    // Moderate resource impact
        Low,       // Minor resource inefficiency
    }
}
```

#### 2. Missing Drop Detector (`src/resource/drop_detector.rs`)

```rust
pub struct DropDetector {
    resource_type_patterns: HashMap<String, ResourcePattern>,
    known_resource_types: HashSet<String>,
}

impl ResourceDetector for DropDetector {
    fn detect_issues(&self, ast: &AstNode) -> Vec<ResourceManagementIssue> {
        let mut issues = Vec::new();
        let type_definitions = self.find_type_definitions(ast);
        
        for type_def in type_definitions {
            let resource_analysis = self.analyze_type_for_resources(&type_def);
            
            if resource_analysis.needs_drop && !resource_analysis.has_drop_impl {
                issues.push(ResourceManagementIssue::MissingDrop {
                    type_name: type_def.name.clone(),
                    resource_fields: resource_analysis.resource_fields,
                    suggested_drop_impl: self.generate_drop_implementation(&resource_analysis),
                    severity: self.assess_resource_severity(&resource_analysis),
                });
            }
        }
        
        issues
    }
}

impl DropDetector {
    fn analyze_type_for_resources(&self, type_def: &TypeDefinition) -> ResourceAnalysis {
        let mut analysis = ResourceAnalysis::default();
        
        // Check if type already implements Drop
        analysis.has_drop_impl = self.has_drop_implementation(type_def);
        
        // Analyze fields for resource types
        for field in &type_def.fields {
            if let Some(resource_info) = self.classify_field_as_resource(field) {
                analysis.resource_fields.push(resource_info);
                analysis.needs_drop = true;
            }
        }
        
        // Check for manual cleanup methods
        analysis.has_manual_cleanup = self.has_manual_cleanup_methods(type_def);
        
        analysis
    }
    
    fn classify_field_as_resource(&self, field: &FieldDefinition) -> Option<ResourceField> {
        // Check against known resource types
        if self.is_known_resource_type(&field.type_name) {
            return Some(ResourceField {
                field_name: field.name.clone(),
                field_type: field.type_name.clone(),
                is_owning: self.is_owning_type(&field.type_name),
                cleanup_required: self.requires_cleanup(&field.type_name),
            });
        }
        
        // Pattern-based detection
        if self.matches_resource_pattern(&field.type_name) {
            return Some(ResourceField {
                field_name: field.name.clone(),
                field_type: field.type_name.clone(),
                is_owning: true,
                cleanup_required: true,
            });
        }
        
        None
    }
    
    fn is_known_resource_type(&self, type_name: &str) -> bool {
        const RESOURCE_TYPES: &[&str] = &[
            "File", "TcpStream", "UdpSocket", "TcpListener",
            "Mutex", "RwLock", "Condvar", "Barrier",
            "Thread", "JoinHandle", "Child", "Process",
            "Box", "Rc", "Arc", "RefCell",
            "BufReader", "BufWriter", "Cursor",
            "Connection", "Client", "Database", "Transaction",
            "Channel", "Sender", "Receiver", "oneshot",
        ];
        
        RESOURCE_TYPES.iter().any(|rt| type_name.contains(rt))
    }
    
    fn matches_resource_pattern(&self, type_name: &str) -> bool {
        // Pattern-based detection for custom resource types
        const RESOURCE_PATTERNS: &[&str] = &[
            "Handle", "Manager", "Pool", "Connection", "Client",
            "Stream", "Reader", "Writer", "Buffer", "Cache",
            "Guard", "Lock", "Session", "Context", "Resource"
        ];
        
        RESOURCE_PATTERNS.iter().any(|pattern| type_name.contains(pattern))
    }
    
    fn generate_drop_implementation(&self, analysis: &ResourceAnalysis) -> String {
        let mut drop_impl = String::new();
        drop_impl.push_str("impl Drop for YourType {\n");
        drop_impl.push_str("    fn drop(&mut self) {\n");
        
        for field in &analysis.resource_fields {
            match field.field_type.as_str() {
                t if t.contains("File") => {
                    drop_impl.push_str(&format!("        // File handles are automatically closed\n"));
                }
                t if t.contains("Thread") => {
                    drop_impl.push_str(&format!("        if let Some(handle) = self.{}.take() {{\n", field.field_name));
                    drop_impl.push_str("            let _ = handle.join();\n");
                    drop_impl.push_str("        }\n");
                }
                t if t.contains("Connection") => {
                    drop_impl.push_str(&format!("        self.{}.close().unwrap_or_else(|e| {{\n", field.field_name));
                    drop_impl.push_str("            eprintln!(\"Failed to close connection: {}\", e);\n");
                    drop_impl.push_str("        });\n");
                }
                _ => {
                    drop_impl.push_str(&format!("        // Cleanup {} resource\n", field.field_name));
                }
            }
        }
        
        drop_impl.push_str("    }\n");
        drop_impl.push_str("}\n");
        drop_impl
    }
    
    fn assess_resource_severity(&self, analysis: &ResourceAnalysis) -> ResourceSeverity {
        let critical_resources = analysis.resource_fields.iter()
            .filter(|field| self.is_critical_resource(&field.field_type))
            .count();
            
        if critical_resources > 0 {
            ResourceSeverity::Critical
        } else if analysis.resource_fields.len() > 3 {
            ResourceSeverity::High
        } else if analysis.resource_fields.len() > 1 {
            ResourceSeverity::Medium
        } else {
            ResourceSeverity::Low
        }
    }
    
    fn is_critical_resource(&self, type_name: &str) -> bool {
        const CRITICAL_TYPES: &[&str] = &[
            "File", "TcpStream", "Process", "Thread", "Connection", "Database"
        ];
        
        CRITICAL_TYPES.iter().any(|ct| type_name.contains(ct))
    }
}

#[derive(Debug, Default)]
struct ResourceAnalysis {
    needs_drop: bool,
    has_drop_impl: bool,
    has_manual_cleanup: bool,
    resource_fields: Vec<ResourceField>,
}
```

#### 3. Async Resource Detector (`src/resource/async_detector.rs`)

```rust
pub struct AsyncResourceDetector {
    cancellation_analyzer: CancellationAnalyzer,
}

impl ResourceDetector for AsyncResourceDetector {
    fn detect_issues(&self, ast: &AstNode) -> Vec<ResourceManagementIssue> {
        let mut issues = Vec::new();
        let async_functions = self.find_async_functions(ast);
        
        for async_fn in async_functions {
            let resource_usage = self.analyze_async_resource_usage(&async_fn);
            
            for issue in resource_usage.issues {
                issues.push(ResourceManagementIssue::AsyncResourceIssue {
                    function_name: async_fn.name.clone(),
                    issue_type: issue.issue_type,
                    cancellation_safety: issue.cancellation_safety,
                    mitigation_strategy: issue.mitigation_strategy,
                });
            }
        }
        
        issues
    }
}

impl AsyncResourceDetector {
    fn analyze_async_resource_usage(&self, async_fn: &AsyncFunction) -> AsyncResourceUsage {
        let mut usage = AsyncResourceUsage::default();
        
        // Track resource acquisition and cleanup across await points
        let await_points = self.find_await_points(&async_fn.body);
        let resource_operations = self.find_resource_operations(&async_fn.body);
        
        for resource_op in resource_operations {
            let cancellation_analysis = self.cancellation_analyzer
                .analyze_resource_cancellation_safety(&resource_op, &await_points);
                
            if !cancellation_analysis.is_safe {
                usage.issues.push(AsyncResourceIssueInfo {
                    issue_type: AsyncResourceIssueType::CancellationUnsafe,
                    cancellation_safety: CancellationSafety::Unsafe,
                    mitigation_strategy: self.suggest_cancellation_mitigation(&resource_op),
                    location: resource_op.location.clone(),
                });
            }
        }
        
        // Check for Drop implementations in async context
        let drop_calls = self.find_drop_calls_in_async(&async_fn.body);
        for drop_call in drop_calls {
            usage.issues.push(AsyncResourceIssueInfo {
                issue_type: AsyncResourceIssueType::DropInAsync,
                cancellation_safety: CancellationSafety::Unknown,
                mitigation_strategy: "Move resource cleanup outside async context".to_string(),
                location: drop_call.location,
            });
        }
        
        usage
    }
    
    fn find_await_points(&self, body: &FunctionBody) -> Vec<AwaitPoint> {
        let mut await_points = Vec::new();
        
        body.traverse(|stmt| {
            if let Statement::Expression(expr) = stmt {
                if let Expression::Await(await_expr) = expr {
                    await_points.push(AwaitPoint {
                        location: await_expr.location.clone(),
                        expression: await_expr.expression.clone(),
                        is_resource_operation: self.is_resource_operation(&await_expr.expression),
                    });
                }
            }
        });
        
        await_points
    }
    
    fn find_resource_operations(&self, body: &FunctionBody) -> Vec<ResourceOperation> {
        let mut operations = Vec::new();
        
        body.traverse(|stmt| {
            match stmt {
                Statement::VariableAssignment(assignment) => {
                    if self.is_resource_assignment(assignment) {
                        operations.push(ResourceOperation {
                            operation_type: ResourceOperationType::Acquisition,
                            resource_type: self.infer_resource_type(&assignment.value),
                            location: assignment.location.clone(),
                            variable_name: Some(assignment.variable_name.clone()),
                        });
                    }
                }
                Statement::FunctionCall(call) => {
                    if self.is_resource_cleanup_call(call) {
                        operations.push(ResourceOperation {
                            operation_type: ResourceOperationType::Release,
                            resource_type: self.infer_resource_type_from_call(call),
                            location: call.location.clone(),
                            variable_name: None,
                        });
                    }
                }
                _ => {}
            }
        });
        
        operations
    }
    
    fn suggest_cancellation_mitigation(&self, resource_op: &ResourceOperation) -> String {
        match resource_op.resource_type {
            ResourceType::FileHandle => {
                "Use tokio::fs or async-std::fs for cancellation-safe file operations".to_string()
            }
            ResourceType::NetworkConnection => {
                "Use connection pools or ensure proper cleanup in Drop implementation".to_string()
            }
            ResourceType::DatabaseConnection => {
                "Use async database drivers with proper cancellation handling".to_string()
            }
            _ => {
                "Ensure resource cleanup in cancellation scenarios using RAII or finally blocks".to_string()
            }
        }
    }
}

#[derive(Debug, Default)]
struct AsyncResourceUsage {
    issues: Vec<AsyncResourceIssueInfo>,
}

#[derive(Debug)]
struct AsyncResourceIssueInfo {
    issue_type: AsyncResourceIssueType,
    cancellation_safety: CancellationSafety,
    mitigation_strategy: String,
    location: SourceLocation,
}

#[derive(Debug)]
struct AwaitPoint {
    location: SourceLocation,
    expression: String,
    is_resource_operation: bool,
}

#[derive(Debug)]
struct ResourceOperation {
    operation_type: ResourceOperationType,
    resource_type: ResourceType,
    location: SourceLocation,
    variable_name: Option<String>,
}

#[derive(Debug)]
enum ResourceOperationType {
    Acquisition,
    Release,
    Transfer,
}
```

#### 4. Unbounded Collection Detector (`src/resource/collection_detector.rs`)

```rust
pub struct UnboundedCollectionDetector {
    growth_analyzer: CollectionGrowthAnalyzer,
}

impl ResourceDetector for UnboundedCollectionDetector {
    fn detect_issues(&self, ast: &AstNode) -> Vec<ResourceManagementIssue> {
        let mut issues = Vec::new();
        let collection_usage = self.analyze_collection_usage(ast);
        
        for usage in collection_usage {
            if usage.is_unbounded {
                issues.push(ResourceManagementIssue::UnboundedCollection {
                    collection_name: usage.name,
                    collection_type: usage.collection_type,
                    growth_pattern: usage.growth_pattern,
                    bounding_strategy: self.suggest_bounding_strategy(&usage),
                });
            }
        }
        
        issues
    }
}

impl UnboundedCollectionDetector {
    fn analyze_collection_usage(&self, ast: &AstNode) -> Vec<CollectionUsage> {
        let mut usages = Vec::new();
        
        // Find collection field declarations
        let collection_fields = self.find_collection_fields(ast);
        
        for field in collection_fields {
            let growth_analysis = self.growth_analyzer.analyze_growth_pattern(&field, ast);
            
            if growth_analysis.has_unbounded_growth {
                usages.push(CollectionUsage {
                    name: field.name.clone(),
                    collection_type: field.type_name.clone(),
                    is_unbounded: true,
                    growth_pattern: growth_analysis.pattern,
                    insert_sites: growth_analysis.insert_sites,
                    remove_sites: growth_analysis.remove_sites,
                });
            }
        }
        
        usages
    }
    
    fn find_collection_fields(&self, ast: &AstNode) -> Vec<FieldDefinition> {
        let mut collection_fields = Vec::new();
        
        ast.traverse_depth_first(|node| {
            if let AstNode::StructDefinition(struct_def) = node {
                for field in &struct_def.fields {
                    if self.is_collection_type(&field.type_name) {
                        collection_fields.push(field.clone());
                    }
                }
            }
        });
        
        collection_fields
    }
    
    fn is_collection_type(&self, type_name: &str) -> bool {
        const COLLECTION_TYPES: &[&str] = &[
            "Vec", "HashMap", "BTreeMap", "HashSet", "BTreeSet",
            "VecDeque", "LinkedList", "BinaryHeap",
            "Cache", "Buffer", "Queue", "Stack"
        ];
        
        COLLECTION_TYPES.iter().any(|ct| type_name.contains(ct))
    }
    
    fn suggest_bounding_strategy(&self, usage: &CollectionUsage) -> BoundingStrategy {
        match usage.growth_pattern {
            GrowthPattern::UnboundedInsertion => {
                if usage.collection_type.contains("Cache") {
                    BoundingStrategy::LruEviction
                } else {
                    BoundingStrategy::SizeLimit
                }
            }
            GrowthPattern::NoEviction => BoundingStrategy::TimeBasedEviction,
            GrowthPattern::MemoryAccumulation => BoundingStrategy::CapacityCheck,
            GrowthPattern::RecursiveGrowth => BoundingStrategy::SizeLimit,
        }
    }
}

#[derive(Debug)]
struct CollectionUsage {
    name: String,
    collection_type: String,
    is_unbounded: bool,
    growth_pattern: GrowthPattern,
    insert_sites: Vec<SourceLocation>,
    remove_sites: Vec<SourceLocation>,
}

pub struct CollectionGrowthAnalyzer;

impl CollectionGrowthAnalyzer {
    fn analyze_growth_pattern(&self, field: &FieldDefinition, ast: &AstNode) -> GrowthAnalysis {
        let mut analysis = GrowthAnalysis::default();
        
        // Find all insertions to this collection
        let insertions = self.find_collection_insertions(field, ast);
        let removals = self.find_collection_removals(field, ast);
        
        analysis.insert_sites = insertions.clone();
        analysis.remove_sites = removals.clone();
        
        // Determine if growth is bounded
        if insertions.len() > 0 && removals.len() == 0 {
            analysis.has_unbounded_growth = true;
            analysis.pattern = GrowthPattern::NoEviction;
        } else if self.has_loop_insertion(&insertions, ast) {
            analysis.has_unbounded_growth = true;
            analysis.pattern = GrowthPattern::UnboundedInsertion;
        } else if self.accumulates_without_bounds(&insertions, &removals) {
            analysis.has_unbounded_growth = true;
            analysis.pattern = GrowthPattern::MemoryAccumulation;
        }
        
        analysis
    }
    
    fn find_collection_insertions(&self, field: &FieldDefinition, ast: &AstNode) -> Vec<SourceLocation> {
        let mut insertions = Vec::new();
        
        ast.traverse_depth_first(|node| {
            if let AstNode::MethodCall(call) = node {
                if self.is_insertion_method(&call.method_name) &&
                   self.targets_field(&call.receiver, &field.name) {
                    insertions.push(call.location.clone());
                }
            }
        });
        
        insertions
    }
    
    fn is_insertion_method(&self, method_name: &str) -> bool {
        const INSERTION_METHODS: &[&str] = &[
            "push", "insert", "add", "put", "append",
            "extend", "push_back", "push_front"
        ];
        
        INSERTION_METHODS.contains(&method_name)
    }
}

#[derive(Debug, Default)]
struct GrowthAnalysis {
    has_unbounded_growth: bool,
    pattern: GrowthPattern,
    insert_sites: Vec<SourceLocation>,
    remove_sites: Vec<SourceLocation>,
}
```

#### 5. Integration with Main Analysis Pipeline

```rust
// In src/analyzers/rust.rs
use crate::resource::{
    ResourceDetector, DropDetector, AsyncResourceDetector,
    UnboundedCollectionDetector, HandleLeakDetector, RaiiViolationDetector
};

fn analyze_resource_patterns(file: &syn::File, path: &Path) -> Vec<DebtItem> {
    let detectors: Vec<Box<dyn ResourceDetector>> = vec![
        Box::new(DropDetector::new()),
        Box::new(AsyncResourceDetector::new()),
        Box::new(UnboundedCollectionDetector::new()),
        Box::new(HandleLeakDetector::new()),
        Box::new(RaiiViolationDetector::new()),
    ];
    
    let ast_node = convert_syn_to_ast_node(file);
    let mut resource_items = Vec::new();
    
    for detector in detectors {
        let issues = detector.detect_issues(&ast_node);
        
        for issue in issues {
            let impact = detector.assess_resource_impact(&issue);
            let debt_item = convert_resource_issue_to_debt_item(issue, impact, path);
            resource_items.push(debt_item);
        }
    }
    
    resource_items
}

fn convert_resource_issue_to_debt_item(
    issue: ResourceManagementIssue,
    impact: ResourceImpact,
    path: &Path
) -> DebtItem {
    let (priority, message, context) = match issue {
        ResourceManagementIssue::MissingDrop { type_name, resource_fields, suggested_drop_impl, .. } => {
            (
                Priority::High,
                format!("Type '{}' holds {} resource(s) but lacks Drop implementation", 
                    type_name, resource_fields.len()),
                Some(format!("Implement Drop:\n{}", suggested_drop_impl))
            )
        }
        ResourceManagementIssue::ResourceLeak { resource_type, cleanup_suggestion, .. } => {
            (
                Priority::Critical,
                format!("Potential {:?} leak detected", resource_type),
                Some(cleanup_suggestion)
            )
        }
        ResourceManagementIssue::AsyncResourceIssue { function_name, issue_type, mitigation_strategy, .. } => {
            (
                Priority::High,
                format!("Async resource issue in '{}': {:?}", function_name, issue_type),
                Some(mitigation_strategy)
            )
        }
        ResourceManagementIssue::UnboundedCollection { collection_name, growth_pattern, bounding_strategy, .. } => {
            (
                Priority::Medium,
                format!("Collection '{}' has unbounded growth: {:?}", collection_name, growth_pattern),
                Some(format!("Consider: {:?}", bounding_strategy))
            )
        }
        ResourceManagementIssue::RaiiViolation { violation_type, resource_involved, correct_pattern } => {
            (
                Priority::Medium,
                format!("RAII violation: {:?} for {}", violation_type, resource_involved),
                Some(format!("Use pattern: {}", correct_pattern))
            )
        }
        ResourceManagementIssue::HandleLeak { handle_type, proper_cleanup, .. } => {
            (
                Priority::High,
                format!("{:?} handle not properly cleaned up", handle_type),
                Some(proper_cleanup)
            )
        }
    };
    
    DebtItem {
        id: format!("resource-{}-{}", path.display(), get_line_from_issue(&issue)),
        debt_type: DebtType::ResourceManagement, // New debt type
        priority,
        file: path.to_path_buf(),
        line: get_line_from_issue(&issue),
        message,
        context,
    }
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_missing_drop_detection() {
        let source = r#"
            struct FileManager {
                files: Vec<std::fs::File>,
                connections: Vec<TcpStream>,
            }
            
            impl FileManager {
                fn new() -> Self {
                    Self {
                        files: Vec::new(),
                        connections: Vec::new(),
                    }
                }
                
                fn cleanup(&mut self) {
                    // Manual cleanup method but no Drop impl
                    self.files.clear();
                }
            }
            
            struct ProperResource {
                file: File,
            }
            
            impl Drop for ProperResource {
                fn drop(&mut self) {
                    // Proper Drop implementation
                }
            }
        "#;
        
        let ast = parse_rust_source(source);
        let detector = DropDetector::new();
        let issues = detector.detect_issues(&ast);
        
        assert_eq!(issues.len(), 1);
        if let ResourceManagementIssue::MissingDrop { type_name, resource_fields, .. } = &issues[0] {
            assert_eq!(type_name, "FileManager");
            assert_eq!(resource_fields.len(), 2);
        } else {
            panic!("Expected missing Drop issue");
        }
    }
    
    #[test]
    fn test_unbounded_collection_detection() {
        let source = r#"
            struct Cache {
                data: HashMap<String, String>,
            }
            
            impl Cache {
                fn insert(&mut self, key: String, value: String) {
                    self.data.insert(key, value); // Unbounded insertion
                }
                
                // No eviction or size limiting
            }
        "#;
        
        let ast = parse_rust_source(source);
        let detector = UnboundedCollectionDetector::new();
        let issues = detector.detect_issues(&ast);
        
        assert!(!issues.is_empty());
        if let ResourceManagementIssue::UnboundedCollection { collection_name, growth_pattern, .. } = &issues[0] {
            assert_eq!(collection_name, "data");
            assert_eq!(*growth_pattern, GrowthPattern::UnboundedInsertion);
        } else {
            panic!("Expected unbounded collection issue");
        }
    }
    
    #[test]
    fn test_async_resource_detection() {
        let source = r#"
            async fn process_files(files: Vec<String>) -> Result<(), Error> {
                for filename in files {
                    let file = File::open(&filename).await?;
                    // Resource may leak if this function is cancelled
                    let data = process_file_data(&file).await?;
                    // File is not explicitly closed
                }
                Ok(())
            }
        "#;
        
        let ast = parse_rust_source(source);
        let detector = AsyncResourceDetector::new();
        let issues = detector.detect_issues(&ast);
        
        assert!(!issues.is_empty());
        let cancellation_issue = issues.iter().find(|i| {
            matches!(i, ResourceManagementIssue::AsyncResourceIssue { 
                issue_type: AsyncResourceIssueType::CancellationUnsafe, 
                .. 
            })
        });
        assert!(cancellation_issue.is_some());
    }
}
```

## Configuration

```toml
[resource]
enabled = true
detectors = ["drop", "async", "collections", "handles", "raii"]

[resource.drop]
detect_missing_drop = true
suggest_implementations = true
analyze_manual_cleanup = true

[resource.async]
check_cancellation_safety = true
detect_resource_leaks = true
analyze_await_points = true

[resource.collections]
detect_unbounded_growth = true
suggest_bounding_strategies = true
memory_threshold = "100MB"

[resource.handles]
track_file_handles = true
track_network_handles = true
track_system_handles = true

[resource.raii]
enforce_raii_patterns = true
detect_manual_management = true
suggest_alternatives = true
```

## Expected Impact

After implementation:

1. **Resource Safety**: Prevention of resource leaks through systematic detection
2. **Memory Efficiency**: Identification of unbounded growth patterns that waste memory
3. **System Stability**: Prevention of resource exhaustion in long-running applications
4. **Async Safety**: Proper resource management in async/await contexts
5. **Maintainability**: Clear guidance on proper resource management patterns

This resource management analysis addresses a critical category of technical debt that can lead to production issues, system instability, and performance degradation, making it an essential component of comprehensive technical debt detection.