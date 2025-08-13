---
number: 23
title: Enhanced Call Graph Analysis for Accurate Dead Code Detection
category: foundation
priority: high
status: draft
dependencies: [21, 22]
created: 2025-01-13
---

# Specification 23: Enhanced Call Graph Analysis for Accurate Dead Code Detection

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: [21 - Dead Code Detection, 22 - Perfect Macro Function Call Detection]

## Context

The current call graph analysis produces false positives in dead code detection due to shallow static analysis that misses several common usage patterns:

1. **Trait Implementation Calls**: Functions like `write_results()` called via trait dispatch are marked as unused
2. **Function Pointer Usage**: Functions passed as closures (e.g., `print_risk_function` via `for_each`) appear unused
3. **Macro-Generated Calls**: Even with spec 22's macro expansion, some dynamic dispatch patterns are missed
4. **Test-Only Usage**: Functions only used in test modules incorrectly flagged as dead code
5. **Framework Pattern Exclusions**: Current exclusions are too narrow and miss valid framework patterns

These false positives create noise in the analysis and undermine user confidence in debtmap's recommendations, leading teams to ignore legitimate dead code warnings.

## Objective

Implement a sophisticated call graph analysis system that accurately tracks function usage across all Rust patterns including trait dispatch, closures, macros, and framework patterns, dramatically reducing false positives in dead code detection while maintaining high precision for genuine unused code.

## Requirements

### Functional Requirements

1. **Trait Dispatch Detection**
   - Identify trait implementations and their usage sites
   - Track calls through trait objects (dyn Trait)
   - Handle generic trait bounds and where clauses
   - Support associated functions and methods

2. **Function Pointer Analysis**
   - Detect functions passed as closure arguments
   - Track higher-order function usage patterns
   - Identify function references in data structures
   - Handle method references and bound methods

3. **Advanced Pattern Recognition**
   - Detect framework callback patterns (handlers, hooks, etc.)
   - Recognize test helper functions and fixtures
   - Identify exported API functions (pub items)
   - Handle proc macro generated code

4. **Cross-Module Analysis**
   - Build complete inter-module call graph
   - Track re-exports and pub use statements
   - Handle workspace-level dependencies
   - Support conditional compilation patterns

5. **Dynamic Dispatch Handling**
   - Analyze enum dispatch patterns
   - Track function tables and lookup patterns
   - Handle runtime reflection usage
   - Support plugin architecture patterns

### Non-Functional Requirements

1. **Performance**: Call graph analysis completes in under 30 seconds for 100K LOC
2. **Accuracy**: Reduce false positives by 90% while maintaining 95% true positive rate
3. **Memory Efficiency**: Use streaming analysis to handle large codebases
4. **Incremental**: Support incremental updates to call graph

## Acceptance Criteria

- [ ] Trait method calls correctly identified as usage (no false positives for `write_results()`)
- [ ] Function pointer usage tracked (no false positives for `print_risk_function`)
- [ ] Test-only functions properly classified and excluded from dead code warnings
- [ ] Framework patterns (main, handlers, traits) automatically excluded
- [ ] Cross-module call tracking works across workspace boundaries
- [ ] Macro-generated calls integrated with existing macro expansion
- [ ] Performance benchmark: <30s for 100K LOC codebase analysis
- [ ] False positive rate reduced to <10% compared to current implementation
- [ ] Integration with existing dead code priority scoring system
- [ ] Comprehensive test suite with real-world usage patterns
- [ ] Documentation for all new call graph analysis features

## Technical Details

### Implementation Approach

1. **Enhanced Call Graph Builder**
```rust
pub struct EnhancedCallGraphBuilder {
    trait_registry: TraitRegistry,
    function_pointer_tracker: FunctionPointerTracker,
    macro_context: MacroContext,
    framework_patterns: FrameworkPatterns,
}

impl EnhancedCallGraphBuilder {
    pub fn build_call_graph(&self, codebase: &Codebase) -> CallGraph {
        let mut graph = CallGraph::new();
        
        // Phase 1: Basic function definitions and direct calls
        self.build_basic_graph(&mut graph, codebase);
        
        // Phase 2: Trait implementation analysis
        self.analyze_trait_implementations(&mut graph, codebase);
        
        // Phase 3: Function pointer and closure analysis
        self.analyze_function_pointers(&mut graph, codebase);
        
        // Phase 4: Framework pattern exclusions
        self.apply_framework_exclusions(&mut graph, codebase);
        
        // Phase 5: Cross-module dependency tracking
        self.analyze_cross_module_usage(&mut graph, codebase);
        
        graph
    }
}
```

2. **Trait Implementation Tracker**
```rust
pub struct TraitRegistry {
    implementations: HashMap<TraitId, Vec<ImplId>>,
    trait_methods: HashMap<TraitId, Vec<MethodSignature>>,
    call_sites: Vec<TraitCallSite>,
}

impl TraitRegistry {
    pub fn register_implementation(&mut self, trait_impl: &ItemImpl) {
        // Track trait implementations
        let trait_id = self.extract_trait_id(trait_impl);
        let impl_id = self.generate_impl_id(trait_impl);
        
        self.implementations
            .entry(trait_id)
            .or_default()
            .push(impl_id);
    }
    
    pub fn track_trait_call(&mut self, call: &MethodCall) -> Vec<FunctionId> {
        // Resolve trait method calls to all possible implementations
        if let Some(trait_id) = self.resolve_trait_call(call) {
            self.implementations
                .get(&trait_id)
                .unwrap_or(&vec![])
                .iter()
                .map(|impl_id| self.resolve_method_in_impl(impl_id, &call.method))
                .collect()
        } else {
            vec![]
        }
    }
}
```

3. **Function Pointer Analysis**
```rust
pub struct FunctionPointerTracker {
    pointer_assignments: HashMap<VariableId, FunctionId>,
    closure_captures: HashMap<ClosureId, Vec<FunctionId>>,
    higher_order_calls: Vec<HigherOrderCall>,
}

impl FunctionPointerTracker {
    pub fn analyze_closures(&mut self, expr: &Expr) {
        match expr {
            Expr::Closure(closure) => {
                self.track_closure_captures(closure);
            }
            Expr::Call(call) => {
                if self.is_higher_order_call(call) {
                    self.track_function_arguments(call);
                }
            }
            Expr::MethodCall(method_call) => {
                // Track methods like .for_each(function_name)
                if self.is_iterator_method(&method_call.method) {
                    self.track_function_arguments_in_method(method_call);
                }
            }
            _ => {}
        }
    }
    
    fn track_function_arguments(&mut self, call: &ExprCall) {
        for arg in &call.args {
            if let Some(func_id) = self.extract_function_reference(arg) {
                self.higher_order_calls.push(HigherOrderCall {
                    caller: self.current_function,
                    callee: func_id,
                    call_type: CallType::FunctionPointer,
                });
            }
        }
    }
}
```

4. **Framework Pattern Recognition**
```rust
pub struct FrameworkPatterns {
    patterns: Vec<Box<dyn FrameworkPattern>>,
}

pub trait FrameworkPattern {
    fn matches(&self, function: &Function) -> bool;
    fn exclusion_reason(&self) -> &str;
}

pub struct TestFrameworkPattern;
impl FrameworkPattern for TestFrameworkPattern {
    fn matches(&self, function: &Function) -> bool {
        // Test functions, test helpers, and fixtures
        function.has_attribute("test") ||
        function.has_attribute("bench") ||
        function.name.starts_with("test_") ||
        function.name.ends_with("_helper") ||
        function.is_in_test_module() ||
        function.has_attribute("fixture")
    }
    
    fn exclusion_reason(&self) -> &str {
        "Test framework function"
    }
}

pub struct WebFrameworkPattern;
impl FrameworkPattern for WebFrameworkPattern {
    fn matches(&self, function: &Function) -> bool {
        // HTTP handlers, route handlers, middleware
        function.has_attribute("handler") ||
        function.has_attribute("route") ||
        function.has_attribute("middleware") ||
        function.return_type_matches("Response") ||
        function.first_param_matches("Request")
    }
    
    fn exclusion_reason(&self) -> &str {
        "Web framework handler"
    }
}
```

5. **Cross-Module Dependency Tracking**
```rust
pub struct CrossModuleTracker {
    exports: HashMap<ModuleId, Vec<ExportedItem>>,
    imports: HashMap<ModuleId, Vec<ImportedItem>>,
    re_exports: HashMap<ModuleId, Vec<ReExport>>,
}

impl CrossModuleTracker {
    pub fn track_public_api(&mut self, module: &Module) {
        for item in &module.items {
            if item.visibility.is_public() {
                self.exports.entry(module.id)
                    .or_default()
                    .push(ExportedItem {
                        name: item.name.clone(),
                        kind: item.kind,
                        location: item.location,
                    });
            }
        }
    }
    
    pub fn resolve_external_usage(&self, function_id: &FunctionId) -> bool {
        // Check if function is used outside its defining module
        let defining_module = self.get_defining_module(function_id);
        
        self.imports.iter()
            .any(|(module_id, imports)| {
                *module_id != defining_module &&
                imports.iter().any(|import| import.resolves_to(function_id))
            })
    }
}
```

### Architecture Changes

1. **Call Graph Module Restructuring**
   - Create `src/analysis/call_graph/` module
   - Implement `CallGraphBuilder` with phases
   - Add `TraitRegistry` for trait dispatch
   - Create `FunctionPointerTracker` for closures

2. **Integration with Existing Systems**
   - Extend macro expansion integration (spec 22)
   - Update dead code detection to use enhanced call graph
   - Integrate with priority scoring system
   - Update output formats to show call graph insights

### Data Structures

```rust
pub struct CallGraph {
    nodes: HashMap<FunctionId, CallGraphNode>,
    edges: Vec<CallEdge>,
    traits: TraitRegistry,
    exclusions: Vec<FrameworkExclusion>,
}

pub struct CallGraphNode {
    pub function_id: FunctionId,
    pub location: Location,
    pub visibility: Visibility,
    pub call_type: NodeType,
    pub callers: Vec<FunctionId>,
    pub callees: Vec<FunctionId>,
}

pub enum NodeType {
    Function,
    Method,
    TraitMethod,
    ClosureFunction,
    TestFunction,
    FrameworkCallback,
    ExportedApi,
}

pub struct CallEdge {
    pub from: FunctionId,
    pub to: FunctionId,
    pub edge_type: EdgeType,
    pub certainty: Certainty,
}

pub enum EdgeType {
    DirectCall,
    TraitMethodCall,
    FunctionPointer,
    MacroGenerated,
    ConditionalCall,
}

pub enum Certainty {
    Definite,   // Direct function call
    Likely,     // Trait method with single impl
    Possible,   // Trait method with multiple impls
    Unknown,    // Dynamic dispatch
}

pub struct FrameworkExclusion {
    pub function_id: FunctionId,
    pub pattern: String,
    pub reason: String,
    pub confidence: f64,
}
```

### APIs and Interfaces

```rust
pub trait CallGraphAnalyzer {
    fn build_call_graph(&self, codebase: &Codebase) -> Result<CallGraph>;
    fn find_unused_functions(&self, graph: &CallGraph) -> Vec<UnusedFunction>;
    fn get_call_chains(&self, graph: &CallGraph, target: &FunctionId) -> Vec<CallChain>;
}

pub struct EnhancedCallGraphAnalyzer {
    builder: EnhancedCallGraphBuilder,
    exclusion_engine: FrameworkExclusionEngine,
    macro_integration: MacroIntegration,
}

impl CallGraphAnalyzer for EnhancedCallGraphAnalyzer {
    fn find_unused_functions(&self, graph: &CallGraph) -> Vec<UnusedFunction> {
        graph.nodes
            .values()
            .filter(|node| {
                node.callers.is_empty() && 
                !self.is_excluded(node) &&
                !self.is_exported_api(node)
            })
            .map(|node| self.create_unused_function(node))
            .collect()
    }
}
```

## Dependencies

- **Prerequisites**:
  - Spec 21 (Dead Code Detection) for base functionality
  - Spec 22 (Perfect Macro Function Call Detection) for macro integration
- **Affected Components**:
  - `src/analysis/call_graph/` - New enhanced module
  - `src/debt/detection.rs` - Update dead code detection
  - `src/debt/mod.rs` - Integration point
  - Tests for existing dead code functionality
- **External Dependencies**:
  - Existing `syn` and `cargo-expand` dependencies

## Testing Strategy

- **Unit Tests**:
  - Test trait dispatch detection with complex hierarchies
  - Validate function pointer tracking in closures
  - Test framework pattern recognition accuracy
  - Verify cross-module usage detection

- **Integration Tests**:
  - Test with real codebases containing trait-heavy code
  - Validate against web framework codebases
  - Test with async/await patterns
  - Performance testing with large codebases (100K+ LOC)

- **Regression Tests**:
  - Ensure no increase in false negatives
  - Validate existing true positives still detected
  - Test backward compatibility with existing configurations

## Documentation Requirements

- **Code Documentation**:
  - Document all new call graph analysis algorithms
  - Explain trait dispatch resolution logic
  - Document framework pattern matching rules
  - Provide examples of complex call patterns

- **User Documentation**:
  - Update dead code detection documentation
  - Explain new accuracy improvements
  - Document framework pattern exclusions
  - Provide troubleshooting guide for edge cases

## Implementation Notes

1. **Performance Considerations**
   - Use incremental analysis where possible
   - Cache trait resolution results
   - Stream large call graphs to disk if needed
   - Parallelize analysis phases

2. **Accuracy Tuning**
   - Start with high precision, gradually improve recall
   - Provide confidence scores for uncertain calls
   - Allow manual pattern override configuration
   - Log analysis decisions for debugging

3. **Framework Integration**
   - Design extensible pattern matching system
   - Support custom framework pattern definitions
   - Integrate with popular Rust framework patterns
   - Plan for future framework evolution

## Migration and Compatibility

- **Non-Breaking Changes**:
  - Enhanced call graph is internal implementation detail
  - Existing CLI flags and output formats preserved
  - Dead code detection API unchanged

- **Performance Impact**:
  - Analysis may be 20-30% slower due to thoroughness
  - Memory usage may increase 10-15% for call graph storage
  - Incremental analysis helps mitigate performance impact

- **Configuration Evolution**:
  - Add optional framework pattern configuration
  - Provide migration guide for custom exclusion rules
  - Maintain backward compatibility with existing configs