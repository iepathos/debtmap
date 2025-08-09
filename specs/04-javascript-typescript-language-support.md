---
number: 04
title: JavaScript/TypeScript Language Support
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-01-09
---

# Specification 04: JavaScript/TypeScript Language Support

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently supports Rust and Python language analysis, providing comprehensive technical debt detection, complexity metrics, and code quality insights. To expand its utility for modern web development and enterprise applications, we need to add support for JavaScript and TypeScript languages. These languages are widely used in frontend development, Node.js backend services, and full-stack applications.

JavaScript and TypeScript share syntax and structures, with TypeScript adding static type annotations. Both languages have complex module systems (CommonJS, ES modules), various function declaration styles (function declarations, arrow functions, methods), and modern async/await patterns that need to be analyzed for complexity and technical debt.

The implementation should leverage tree-sitter parsing for robust AST generation and follow the existing functional programming patterns established in the codebase.

## Objective

Implement comprehensive JavaScript and TypeScript language support in debtmap, enabling analysis of complexity metrics, technical debt detection, dependency tracking, and code smell identification for JS/TS codebases with the same level of detail and accuracy as existing Rust and Python analyzers.

## Requirements

### Functional Requirements

- **Language Detection**: Automatically detect JavaScript (.js, .jsx) and TypeScript (.ts, .tsx) files
- **AST Parsing**: Parse JavaScript and TypeScript source code into analyzable AST representations
- **Complexity Analysis**: Calculate cyclomatic and cognitive complexity for functions, methods, and arrow functions
- **Debt Detection**: Identify TODO, FIXME, HACK, XXX, and BUG comments with proper context
- **Code Smell Detection**: Detect long parameter lists, large functions, deep nesting, and other JS/TS-specific smells
- **Dependency Analysis**: Track import/export statements, require() calls, and dynamic imports
- **Module Support**: Handle ES modules, CommonJS, AMD, and UMD module patterns
- **TypeScript Features**: Support type annotations, interfaces, generics, decorators, and enums
- **JSX/TSX Support**: Parse and analyze React JSX syntax in JavaScript and TypeScript files
- **Async Pattern Analysis**: Detect and analyze Promise chains, async/await usage, and callback patterns

### Non-Functional Requirements

- **Performance**: Parse and analyze JavaScript/TypeScript files with similar speed to existing analyzers
- **Memory Efficiency**: Handle large JS/TS files without excessive memory usage
- **Error Resilience**: Continue analysis when encountering syntax errors or unsupported language features
- **Extensibility**: Design analyzer to easily add new JS/TS-specific analysis rules
- **Compatibility**: Support ECMAScript 2015+ features and TypeScript 4.0+ syntax

## Acceptance Criteria

- [ ] JavaScript analyzer correctly identifies and parses .js and .jsx files
- [ ] TypeScript analyzer correctly identifies and parses .ts and .tsx files
- [ ] Cyclomatic complexity calculation matches established algorithms for JS/TS control flow
- [ ] Cognitive complexity accounts for JS/TS-specific constructs (callbacks, promises, arrow functions)
- [ ] All debt comment types (TODO, FIXME, etc.) are detected with accurate line numbers
- [ ] Import/export statements are tracked as dependencies
- [ ] CommonJS require() statements are tracked as dependencies
- [ ] Dynamic imports and conditional imports are detected
- [ ] Long parameter lists (>5 parameters) are flagged as code smells
- [ ] Large functions (>50 lines) are flagged as code smells
- [ ] Deep nesting (>4 levels) is detected and reported
- [ ] TypeScript-specific features (interfaces, types, generics) are properly parsed
- [ ] JSX/TSX elements are analyzed for complexity and nested structures
- [ ] Async/await patterns are included in complexity calculations
- [ ] Suppression comments work correctly for JS/TS files
- [ ] Performance benchmarks show <2x slowdown compared to existing analyzers
- [ ] All existing tests continue to pass
- [ ] New comprehensive test suite covers JS/TS specific scenarios

## Technical Details

### Implementation Approach

The JavaScript/TypeScript analyzer will follow the existing analyzer pattern established in the codebase:

1. **Tree-sitter Integration**: Use tree-sitter-javascript and tree-sitter-typescript for robust parsing
2. **Unified Analyzer**: Create a single analyzer that handles both JavaScript and TypeScript with language-specific features
3. **AST Visitor Pattern**: Implement visitor pattern for traversing parsed AST nodes
4. **Functional Architecture**: Maintain pure functional design with immutable data structures

### Architecture Changes

**New Files**:
- `src/analyzers/javascript.rs` - Main JS/TS analyzer implementation
- `src/analyzers/typescript.rs` - TypeScript-specific extensions (if needed)
- `src/analyzers/js_visitor.rs` - AST visitor for complexity and debt detection

**Modified Files**:
- `src/core/mod.rs` - Add JavaScript and TypeScript to Language enum
- `src/analyzers/mod.rs` - Register new JS/TS analyzer
- `Cargo.toml` - Add tree-sitter dependencies

### Data Structures

**Language Enum Extension**:
```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Copy)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Unknown,
}
```

**JS/TS Specific AST Nodes**:
```rust
pub enum JsAstNode {
    Function { name: String, params: Vec<String>, body: Vec<JsAstNode> },
    ArrowFunction { params: Vec<String>, body: Vec<JsAstNode> },
    Method { name: String, params: Vec<String>, body: Vec<JsAstNode> },
    Class { name: String, methods: Vec<JsAstNode> },
    Interface { name: String, members: Vec<String> }, // TypeScript
    Import { source: String, specifiers: Vec<String> },
    Export { specifiers: Vec<String> },
    JSXElement { tag: String, children: Vec<JsAstNode> },
    // ... other node types
}
```

### APIs and Interfaces

**JavaScriptAnalyzer**:
```rust
pub struct JavaScriptAnalyzer {
    parser: tree_sitter::Parser,
    language_config: LanguageConfig,
}

impl Analyzer for JavaScriptAnalyzer {
    fn parse(&self, content: &str, path: PathBuf) -> Result<Ast>;
    fn analyze(&self, ast: &Ast) -> FileMetrics;
    fn language(&self) -> Language;
}
```

**Complexity Calculation**:
```rust
pub fn calculate_js_complexity(node: &JsAstNode) -> ComplexityMetrics;
pub fn calculate_cognitive_complexity_js(node: &JsAstNode) -> u32;
```

## Dependencies

- **Prerequisites**: None (foundational feature)
- **Affected Components**: 
  - `src/core/mod.rs` (Language enum)
  - `src/analyzers/mod.rs` (analyzer registration)
  - `src/cli.rs` (potentially for file detection)
- **External Dependencies**: 
  - `tree-sitter` (already used)
  - `tree-sitter-javascript` (new)
  - `tree-sitter-typescript` (new)

## Testing Strategy

### Unit Tests
- Test AST parsing for various JavaScript constructs
- Test TypeScript-specific syntax parsing
- Test complexity calculation accuracy
- Test debt detection in JS/TS comments
- Test dependency extraction from imports/requires
- Test JSX/TSX parsing and analysis

### Integration Tests
- Test full file analysis workflow for JavaScript files
- Test full file analysis workflow for TypeScript files
- Test performance with large JS/TS files
- Test error handling with malformed syntax
- Test suppression comment functionality

### Performance Tests
- Benchmark parsing speed vs file size
- Memory usage profiling for large codebases
- Comparison benchmarks with existing analyzers

### User Acceptance
- Test with real-world JavaScript projects (React, Node.js)
- Test with real-world TypeScript projects (Angular, NestJS)
- Validate complexity scores against manual analysis
- Verify debt detection accuracy

## Documentation Requirements

### Code Documentation
- Comprehensive rustdoc for all new public APIs
- Inline comments explaining JS/TS parsing logic
- Document tree-sitter grammar usage patterns
- Example usage in documentation

### User Documentation
- Update README.md with JavaScript/TypeScript support
- Add JavaScript/TypeScript examples to CLI help
- Document JS/TS-specific configuration options
- Create language support comparison table

### Architecture Updates
- Update ARCHITECTURE.md with JS/TS analyzer design
- Document tree-sitter integration patterns
- Add JS/TS complexity calculation algorithms
- Update module dependency diagrams

## Implementation Notes

### JavaScript Complexity Considerations
- **Arrow Functions**: Count as function declarations for complexity
- **Callbacks**: Nested callbacks increase cognitive complexity
- **Promises**: `.then()/.catch()` chains add complexity
- **Async/Await**: Try/catch blocks in async functions increase complexity
- **Template Literals**: Complex template expressions may add complexity
- **Destructuring**: Complex destructuring patterns may indicate complexity

### TypeScript-Specific Features
- **Type Guards**: Function complexity should account for type checking logic
- **Generics**: Generic constraints and type parameters may add complexity
- **Decorators**: Decorator usage may indicate additional complexity
- **Union/Intersection Types**: Complex type definitions may warrant analysis
- **Conditional Types**: Advanced TypeScript types may increase complexity

### Module System Handling
- **ES Modules**: Track `import`/`export` statements
- **CommonJS**: Track `require()` and `module.exports`
- **Dynamic Imports**: Track `import()` function calls
- **AMD/UMD**: Support `define()` patterns if encountered

### JSX/TSX Considerations
- **Component Complexity**: Large JSX trees may indicate complexity
- **Event Handlers**: Inline event handlers add to component complexity
- **Conditional Rendering**: Ternary operators and logical AND in JSX
- **Props Spreading**: Complex prop patterns may indicate maintenance issues

## Migration and Compatibility

### Breaking Changes
- None expected (additive feature)

### Configuration Changes
- New language detection patterns for .js, .jsx, .ts, .tsx files
- Optional JS/TS-specific configuration parameters

### Performance Impact
- Initial implementation may be 1.5-2x slower than existing analyzers
- Optimization phase should achieve comparable performance
- Memory usage should remain within acceptable bounds

### Backward Compatibility
- All existing functionality remains unchanged
- Existing configuration files continue to work
- No changes to output format structures