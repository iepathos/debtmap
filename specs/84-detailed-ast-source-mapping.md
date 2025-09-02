---
number: 84
title: Detailed AST-Based Source Mapping
category: optimization
priority: low
status: draft
dependencies: [80]
created: 2025-09-02
---

# Specification 84: Detailed AST-Based Source Mapping

**Category**: optimization
**Priority**: low
**Status**: draft
**Dependencies**: [80] Multi-Pass Analysis with Attribution

## Context

The current source location mapping in the multi-pass analysis uses estimation based on function metrics (line numbers and length) rather than actual AST node locations. This simplified approach limits:
- Precise mapping of complexity points to specific code constructs
- Accurate attribution of complexity to individual statements and expressions
- Navigation from complexity reports to exact code locations
- IDE integration for inline complexity visualization
- Detailed complexity heat maps at the statement level

Accurate AST-based source mapping is essential for providing developers with precise information about where complexity originates in their code, enabling targeted refactoring and better understanding of complexity distribution.

## Objective

Implement comprehensive AST-based source mapping that tracks exact locations of all complexity-contributing constructs, enabling precise navigation from complexity reports to source code and supporting rich IDE integrations.

## Requirements

### Functional Requirements

- **AST Node Location Tracking**: Track precise location (line, column, span) for all AST nodes
- **Complexity Point Mapping**: Map each complexity point to specific AST nodes
- **Source Range Calculation**: Calculate exact source ranges for complex constructs
- **Cross-Reference Generation**: Build bidirectional maps between complexity and source
- **Incremental Updates**: Support incremental mapping updates for code changes
- **Multi-Language Support**: Handle source mapping for all supported languages
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

**Phase 1: Enhanced AST with Source Locations**
```rust
// Enhanced AST node with detailed location information
#[derive(Debug, Clone)]
pub struct ASTNodeWithLocation {
    pub node: ASTNode,
    pub location: SourceLocation,
    pub span: SourceSpan,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: PathBuf,
    pub line: u32,
    pub column: u32,
    pub byte_offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSpan {
    pub start: SourceLocation,
    pub end: SourceLocation,
    pub text: String,
}

pub struct SourceMappedAST {
    nodes: HashMap<NodeId, ASTNodeWithLocation>,
    source_index: SourceIndex,
    complexity_map: ComplexityMap,
}
```

**Phase 2: Complexity-to-Source Mapping**
```rust
pub struct ComplexitySourceMapper {
    ast_map: SourceMappedAST,
    complexity_points: Vec<ComplexityPoint>,
    source_map: SourceMap,
}

#[derive(Debug, Clone)]
pub struct ComplexityPoint {
    pub id: ComplexityPointId,
    pub value: u32,
    pub type_: ComplexityType,
    pub ast_node: NodeId,
    pub source_location: SourceLocation,
    pub source_span: SourceSpan,
    pub contributing_factors: Vec<ComplexityFactor>,
}

impl ComplexitySourceMapper {
    pub fn map_complexity_to_source(&mut self, metrics: &FunctionMetrics) {
        // Visit AST nodes and calculate complexity contribution
        self.ast_map.visit_complexity_nodes(|node| {
            let complexity = self.calculate_node_complexity(node);
            if complexity > 0 {
                self.add_complexity_point(ComplexityPoint {
                    id: self.next_id(),
                    value: complexity,
                    type_: self.determine_complexity_type(node),
                    ast_node: node.id,
                    source_location: node.location.clone(),
                    source_span: node.span.clone(),
                    contributing_factors: self.analyze_factors(node),
                });
            }
        });
    }
    
    pub fn get_source_for_complexity(&self, point_id: ComplexityPointId) -> Option<&SourceSpan> {
        self.complexity_points
            .iter()
            .find(|p| p.id == point_id)
            .map(|p| &p.source_span)
    }
}
```

**Phase 3: Incremental Mapping Updates**
```rust
pub struct IncrementalMapper {
    base_map: SourceMap,
    change_tracker: ChangeTracker,
    diff_calculator: DiffCalculator,
}

impl IncrementalMapper {
    pub fn update_mapping(&mut self, changes: &[SourceChange]) -> MappingDelta {
        let mut delta = MappingDelta::new();
        
        for change in changes {
            match change {
                SourceChange::Insert { location, text } => {
                    let new_nodes = self.parse_and_map(text, location);
                    delta.additions.extend(new_nodes);
                    self.adjust_subsequent_locations(location, text.len());
                }
                SourceChange::Delete { span } => {
                    let removed = self.remove_nodes_in_span(span);
                    delta.deletions.extend(removed);
                    self.adjust_subsequent_locations(&span.start, -(span.length() as i32));
                }
                SourceChange::Modify { span, new_text } => {
                    let removed = self.remove_nodes_in_span(span);
                    let added = self.parse_and_map(new_text, &span.start);
                    delta.deletions.extend(removed);
                    delta.additions.extend(added);
                    let diff = new_text.len() as i32 - span.length() as i32;
                    self.adjust_subsequent_locations(&span.start, diff);
                }
            }
        }
        
        delta
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

**New Components:**
```
src/analysis/source_mapping/
├── mod.rs                      # Source mapping coordination
├── ast_mapper.rs               # AST source location tracking
├── complexity_mapper.rs        # Complexity-to-source mapping
├── incremental.rs             # Incremental mapping updates
├── source_index.rs            # Efficient source location indexing
├── export/
│   ├── mod.rs                 # Export functionality
│   ├── sourcemap_v3.rs       # Source map v3 format
│   └── lsp_integration.rs    # LSP server integration
└── languages/
    ├── mod.rs                 # Language-specific mapping
    ├── rust_mapper.rs         # Rust source mapping
    ├── javascript_mapper.rs  # JavaScript source mapping
    ├── typescript_mapper.rs  # TypeScript source mapping
    └── python_mapper.rs      # Python source mapping
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
- Use `syn` crate's `Span` information
- Track macro expansion locations
- Handle lifetime and generic parameters

**JavaScript/TypeScript Mapping:**
- Use tree-sitter's location tracking
- Handle JSX/TSX syntax
- Track async/await transformations

**Python Mapping:**
- Use Python AST's `lineno` and `col_offset`
- Handle indentation-based blocks
- Track decorator applications

## Dependencies

- **Prerequisites**:
  - [80] Multi-Pass Analysis with Attribution (base implementation)
- **Affected Components**:
  - AST parsers for all languages
  - Complexity calculation modules
  - Diagnostic reporters
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

### Challenges

- Maintaining accuracy during code formatting changes
- Handling generated code and macros
- Cross-platform path resolution
- Performance with very large files

### Optimization Strategies

- Use interval trees for efficient location queries
- Cache frequently accessed mappings
- Compress source maps for storage
- Lazy loading of source content

### Future Enhancements

- Real-time mapping updates during editing
- Visual complexity overlays in IDEs
- Git blame integration for historical mapping
- Cross-file complexity flow visualization