---
number: 84
title: Detailed AST-Based Source Mapping
category: optimization
priority: medium
status: draft
dependencies: [80, 90]
created: 2025-09-02
updated: 2025-09-05
---

# Specification 84: Detailed AST-Based Source Mapping

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: 
- [80] Multi-Pass Analysis with Attribution
- [90] Shared Cache Location (for caching source maps)

## Context

The current implementation has fragmented source location tracking:
- `FunctionMetrics` only tracks `line: usize` with no column or span information
- `AstNode` in `core/ast.rs` only has single line numbers
- Attribution engine uses `EstimatedComplexityLocation` with comments stating "Estimated location information"
- Different languages have separate `SourceLocation` implementations (Rust uses `common/source_location.rs`, JavaScript has its own)
- The `generate_source_mappings` function creates mappings using line numbers and estimated lengths

While we have a foundation in `common/source_location.rs` with `UnifiedLocationExtractor` that can extract precise spans from syn AST nodes, this infrastructure is not consistently used across the codebase. This fragmentation limits:
- Precise mapping of complexity points to specific code constructs
- Accurate attribution of complexity to individual statements and expressions
- Navigation from complexity reports to exact code locations
- IDE integration for inline complexity visualization
- Detailed complexity heat maps at the statement level

Unifying and extending the existing source mapping infrastructure is essential for providing developers with precise information about where complexity originates in their code, enabling targeted refactoring and better understanding of complexity distribution.

## Objective

Unify and extend the existing source location infrastructure to provide comprehensive AST-based source mapping that:
1. Builds upon the existing `common/source_location.rs` module
2. Extends `UnifiedLocationExtractor` to support all languages consistently
3. Replaces estimation-based attribution with precise AST node tracking
4. Enables exact navigation from complexity reports to source code
5. Supports rich IDE integrations and source map exports

## Requirements

### Functional Requirements

- **Unified Location System**: Extend `common/source_location.rs` to be the single source of truth
- **Enhanced FunctionMetrics**: Add `SourceLocation` to replace simple `line: usize`
- **Enhanced AstNode**: Add full `SourceLocation` with spans to `core/ast.rs`
- **Attribution Integration**: Replace `EstimatedComplexityLocation` with precise locations
- **Language Unification**: Migrate JavaScript's separate `SourceLocation` to common module
- **Complexity Point Mapping**: Map each complexity point to specific AST nodes with exact spans
- **Cross-Reference Generation**: Build bidirectional maps between complexity and source
- **Cache Integration**: Leverage spec 90's shared cache for source map storage
- **Incremental Updates**: Support incremental mapping updates for code changes
- **Source Map Serialization**: Export source maps in standard formats
- **IDE Protocol Support**: Support Language Server Protocol (LSP) for IDE integration

### Non-Functional Requirements

- **Precision**: 100% accurate mapping to source locations
- **Performance**: Mapping overhead <5% of analysis time
- **Memory Efficiency**: Source maps use <10% additional memory
- **Compatibility**: Support standard source map formats (v3)

## Acceptance Criteria

- [ ] AST nodes contain accurate source location information
- [ ] Every complexity point maps to specific AST node and source location
- [ ] Source ranges correctly encompass entire constructs
- [ ] Bidirectional mapping between complexity and source works correctly
- [ ] Incremental updates maintain mapping consistency
- [ ] All supported languages have full source mapping
- [ ] Source maps exportable in standard v3 format
- [ ] LSP integration provides accurate go-to-definition
- [ ] Performance overhead measured under 5%
- [ ] Memory usage increase stays under 10%

## Technical Details

### Implementation Approach

**Phase 1: Unify and Extend Existing Infrastructure**
```rust
// Extend existing common/source_location.rs
use crate::common::{SourceLocation, LocationConfidence, UnifiedLocationExtractor};

// Enhanced FunctionMetrics with full location
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionMetrics {
    pub name: String,
    pub file: PathBuf,
    pub location: SourceLocation,  // Replace line: usize
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub nesting: u32,
    pub length: usize,
    // ... existing fields ...
}

// Enhanced AstNode with source location
#[derive(Clone, Debug)]
pub struct AstNode {
    pub kind: NodeKind,
    pub name: Option<String>,
    pub location: SourceLocation,  // Replace line: usize
    pub children: Vec<AstNode>,
    pub ast_id: NodeId,  // New: unique identifier for cross-referencing
}

// Extend UnifiedLocationExtractor for all languages
impl UnifiedLocationExtractor {
    // Existing Rust support via syn
    
    // Add Python support
    pub fn extract_python_location(&self, node: &rustpython_parser::ast::Located<T>) -> SourceLocation {
        SourceLocation {
            line: node.location.row(),
            column: Some(node.location.column()),
            end_line: node.end_location.map(|e| e.row()),
            end_column: node.end_location.map(|e| e.column()),
            confidence: LocationConfidence::Exact,
        }
    }
    
    // Unify JavaScript/TypeScript to use common SourceLocation
    pub fn extract_treesitter_location(&self, node: tree_sitter::Node) -> SourceLocation {
        let start = node.start_position();
        let end = node.end_position();
        SourceLocation {
            line: start.row + 1,  // tree-sitter uses 0-based
            column: Some(start.column),
            end_line: if end.row != start.row { Some(end.row + 1) } else { None },
            end_column: Some(end.column),
            confidence: LocationConfidence::Exact,
        }
    }
}
```

**Phase 2: Replace Attribution Engine's Estimation**
```rust
// Update analysis/attribution/mod.rs
use crate::common::{SourceLocation, UnifiedLocationExtractor};

impl AttributionEngine {
    // Replace EstimatedComplexityLocation with precise tracking
    fn generate_source_mappings(&self, functions: &[FunctionMetrics]) -> Vec<SourceMapping> {
        let mut mappings = Vec::new();
        
        for func in functions {
            // Use actual SourceLocation instead of estimation
            mappings.push(SourceMapping {
                complexity_point: func.cyclomatic,
                location: CodeLocation::from_source_location(&func.location),
                ast_path: self.build_ast_path(func),
                context: format!("Function: {}", func.name),
            });
            
            // Add mappings for internal complexity points
            if let Some(ast) = self.get_function_ast(func) {
                self.map_ast_complexity_points(&ast, &mut mappings);
            }
        }
        
        mappings
    }
    
    fn map_ast_complexity_points(&self, ast: &AstNode, mappings: &mut Vec<SourceMapping>) {
        // Map each complexity-contributing node
        match ast.kind {
            NodeKind::If | NodeKind::While | NodeKind::For => {
                mappings.push(SourceMapping {
                    complexity_point: 1,
                    location: CodeLocation::from_source_location(&ast.location),
                    ast_path: self.build_node_path(ast),
                    context: format!("{:?} at line {}", ast.kind, ast.location.line),
                });
            }
            _ => {}
        }
        
        // Recurse through children
        for child in &ast.children {
            self.map_ast_complexity_points(child, mappings);
        }
    }
}

// New complexity point tracker
pub struct ComplexityPointRegistry {
    points: HashMap<NodeId, ComplexityPoint>,
    by_location: BTreeMap<(usize, Option<usize>), Vec<NodeId>>,
    by_function: HashMap<String, Vec<NodeId>>,
}

#[derive(Debug, Clone)]
pub struct ComplexityPoint {
    pub node_id: NodeId,
    pub value: u32,
    pub type_: ComplexityType,
    pub location: SourceLocation,
    pub function_name: String,
    pub contributing_factors: Vec<String>,
}
```

**Phase 3: Cache Integration and Incremental Updates**
```rust
// Integrate with spec 90's shared cache infrastructure
use crate::cache::SharedCache;

pub struct CachedSourceMapper {
    cache: SharedCache,
    mapper: ComplexityPointRegistry,
}

impl CachedSourceMapper {
    pub async fn get_or_compute_mappings(&self, file: &Path) -> Result<Vec<SourceMapping>> {
        let cache_key = format!("source_map:{}", file.display());
        
        // Try to get from cache
        if let Some(cached) = self.cache.get(&cache_key).await? {
            return Ok(cached);
        }
        
        // Compute if not cached
        let mappings = self.compute_mappings(file)?;
        self.cache.set(&cache_key, &mappings).await?;
        Ok(mappings)
    }
    
    pub fn invalidate_mapping(&self, file: &Path) {
        let cache_key = format!("source_map:{}", file.display());
        self.cache.invalidate(&cache_key);
    }
}

// Incremental update support
impl ComplexityPointRegistry {
    pub fn update_incremental(&mut self, file: &Path, changes: &[TextChange]) {
        // Remove affected points
        let affected = self.find_affected_points(file, changes);
        for node_id in affected {
            self.remove_point(node_id);
        }
        
        // Re-analyze changed regions
        for change in changes {
            if let Some(ast) = self.parse_region(file, &change.range) {
                self.analyze_and_register(&ast);
            }
        }
        
        // Adjust line numbers for points after changes
        self.adjust_locations(file, changes);
    }
    
    fn adjust_locations(&mut self, file: &Path, changes: &[TextChange]) {
        for change in changes {
            let line_delta = change.new_line_count as i32 - change.old_line_count as i32;
            if line_delta != 0 {
                for point in self.points.values_mut() {
                    if point.location.line > change.start_line {
                        point.location.line = (point.location.line as i32 + line_delta) as usize;
                        if let Some(end_line) = &mut point.location.end_line {
                            *end_line = (*end_line as i32 + line_delta) as usize;
                        }
                    }
                }
            }
        }
    }
}
```

**Phase 4: Source Map Export and IDE Integration**
```rust
pub struct SourceMapExporter {
    format: SourceMapFormat,
    encoder: SourceMapEncoder,
}

impl SourceMapExporter {
    pub fn export_v3(&self, mapping: &ComplexitySourceMapper) -> String {
        let mut map = SourceMapV3::new();
        
        for point in &mapping.complexity_points {
            map.add_mapping(Mapping {
                generated_line: point.id.0 as u32,
                generated_column: 0,
                source_line: point.source_location.line,
                source_column: point.source_location.column,
                source_file: point.source_location.file.to_str().unwrap(),
                name: Some(format!("complexity_{}", point.type_)),
            });
        }
        
        self.encoder.encode(&map)
    }
}

// LSP integration
pub struct ComplexityLSPServer {
    mapper: ComplexitySourceMapper,
    lsp_server: LspServer,
}

impl ComplexityLSPServer {
    pub fn handle_goto_definition(&self, params: GotoDefinitionParams) -> Option<Location> {
        let position = params.text_document_position_params.position;
        
        // Find complexity point at position
        if let Some(point) = self.mapper.find_complexity_at_position(position) {
            Some(Location {
                uri: params.text_document_position_params.text_document.uri,
                range: self.span_to_range(&point.source_span),
            })
        } else {
            None
        }
    }
}
```

### Architecture Changes

**Modified Components:**
```
src/common/source_location.rs   # Extended with multi-language support
src/core/mod.rs                 # FunctionMetrics with SourceLocation
src/core/ast.rs                 # AstNode with SourceLocation and NodeId
src/analysis/attribution/mod.rs # Replace EstimatedComplexityLocation

src/analyzers/
├── rust.rs                      # Use UnifiedLocationExtractor
├── python.rs                    # Use UnifiedLocationExtractor  
└── javascript/
    └── detectors/mod.rs        # Migrate to common SourceLocation
```

**New Components:**
```
src/analysis/source_mapping/
├── mod.rs                      # Source mapping coordination
├── complexity_registry.rs      # ComplexityPointRegistry implementation
├── cached_mapper.rs           # Cache-integrated source mapper
├── incremental.rs             # Incremental mapping updates
├── export/
│   ├── mod.rs                 # Export functionality
│   ├── sourcemap_v3.rs       # Source map v3 format
│   └── lsp_integration.rs    # LSP server integration
└── tests/
    └── source_mapping_test.rs # Comprehensive tests
```

### Data Structures

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMap {
    pub version: u32,
    pub file: String,
    pub source_root: Option<String>,
    pub sources: Vec<String>,
    pub sources_content: Option<Vec<String>>,
    pub names: Vec<String>,
    pub mappings: String,  // VLQ encoded mappings
    pub complexity_metadata: ComplexityMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetadata {
    pub total_points: u32,
    pub point_details: Vec<ComplexityPointDetail>,
    pub heat_map: ComplexityHeatMap,
    pub navigation_index: NavigationIndex,
}

#[derive(Debug, Clone)]
pub struct ComplexityHeatMap {
    pub resolution: HeatMapResolution,
    pub data: Vec<Vec<f32>>,
    pub color_scale: ColorScale,
}

#[derive(Debug, Clone)]
pub struct NavigationIndex {
    pub by_complexity: BTreeMap<u32, Vec<ComplexityPointId>>,
    pub by_location: BTreeMap<SourceLocation, ComplexityPointId>,
    pub by_type: HashMap<ComplexityType, Vec<ComplexityPointId>>,
}
```

### Language-Specific Implementations

**Rust Source Mapping:**
- Already supported via `syn` crate's `Span` in `UnifiedLocationExtractor`
- Extend to track macro expansion locations
- Handle lifetime and generic parameters in complexity calculation

**JavaScript/TypeScript Mapping:**
- Migrate from separate `SourceLocation` to common module
- Use `UnifiedLocationExtractor::extract_treesitter_location()`
- Handle JSX/TSX syntax in location tracking
- Track async/await transformations

**Python Mapping:**
- Implement `extract_python_location()` using rustpython_parser's location info
- Handle indentation-based blocks correctly
- Track decorator applications and their impact on complexity

## Dependencies

- **Prerequisites**:
  - [80] Multi-Pass Analysis with Attribution (for attribution engine integration)
  - [90] Shared Cache Location (for caching source maps)
- **Affected Components**:
  - `common/source_location.rs` - extend for all languages
  - `core/mod.rs` - update FunctionMetrics
  - `core/ast.rs` - update AstNode structure  
  - `analysis/attribution/mod.rs` - replace estimation logic
  - All language analyzers - migrate to unified location system
- **External Dependencies**:
  - Source map libraries (for v3 format)
  - LSP libraries (for IDE integration)

## Testing Strategy

### Unit Tests
- **Location Accuracy**: Verify AST nodes have correct locations
- **Mapping Correctness**: Test complexity-to-source mapping
- **Incremental Updates**: Validate incremental mapping updates
- **Export Format**: Test source map v3 format generation
- **Cross-Reference**: Verify bidirectional mapping works

### Integration Tests
- **End-to-End Mapping**: Complete mapping for real source files
- **Multi-Language**: Test all supported languages
- **IDE Integration**: Test LSP server functionality
- **Large Files**: Verify performance with large source files
- **Complex Constructs**: Test mapping of nested/complex code

### Performance Tests
- **Mapping Speed**: Measure mapping generation time
- **Memory Usage**: Track memory consumption
- **Incremental Performance**: Test incremental update speed
- **Scalability**: Test with very large codebases

## Documentation Requirements

### Code Documentation
- **Mapping Algorithm**: Document source mapping approach
- **Location Tracking**: Explain location tracking methodology
- **Incremental Strategy**: Document incremental update algorithm
- **Export Formats**: Document supported export formats

### User Documentation
- **Source Map Guide**: How to use and interpret source maps
- **IDE Integration**: Setting up IDE integration
- **Navigation Guide**: Using source maps for code navigation
- **Troubleshooting**: Common source mapping issues

### Architecture Updates
- **Mapping System**: Document source mapping architecture
- **Integration Points**: How mapping integrates with analysis
- **Performance Guide**: Source mapping optimization

## Implementation Notes

### Migration Path

1. **Phase 1**: Extend `UnifiedLocationExtractor` for all languages
2. **Phase 2**: Update data structures (`FunctionMetrics`, `AstNode`)
3. **Phase 3**: Migrate JavaScript to common `SourceLocation`
4. **Phase 4**: Update attribution engine to use precise locations
5. **Phase 5**: Implement caching and incremental updates
6. **Phase 6**: Add export and IDE support

### Challenges

- **Backward Compatibility**: Need migration for existing serialized data
- **JavaScript Migration**: Unifying two different `SourceLocation` implementations
- **Macro Handling**: Rust macros complicate source mapping
- **Performance**: Maintaining <5% overhead with detailed tracking
- **Cross-platform**: Path resolution across different OS

### Optimization Strategies

- Leverage existing `SharedCache` from spec 90
- Use interval trees for efficient location queries
- Lazy computation of source maps
- Incremental updates to avoid full recomputation
- Compress source maps when storing in cache

### Future Enhancements

- Real-time mapping updates during editing
- Visual complexity overlays in IDEs
- Git blame integration for historical mapping
- Cross-file complexity flow visualization
- Integration with debugging tools