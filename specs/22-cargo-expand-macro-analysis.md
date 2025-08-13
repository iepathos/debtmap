---
number: 22
title: Perfect Macro Function Call Detection with cargo-expand
category: optimization
priority: high
status: draft
dependencies: [21]
created: 2025-01-13
---

# Specification 22: Perfect Macro Function Call Detection with cargo-expand

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [21] (Dead Code Detection)

## Context

The current dead code detection system (spec 21) uses syn-based AST analysis to build a call graph and identify unused functions. However, it has a critical limitation: function calls within macros (like `println!`, `format!`, `assert!`) are not detected because syn operates on pre-expansion AST where macro invocations remain as opaque token streams.

This leads to false positives where functions are incorrectly marked as dead code when they're actually called within macros. For example, `format_debt_score` in `terminal.rs` is called within a `println!` macro but appears as dead code to the current analyzer.

While pattern-based parsing of common macros could handle 90% of cases, achieving 100% accuracy requires analyzing the fully expanded Rust code where all macros have been resolved to their actual implementations.

## Objective

Implement a cargo-expand based preprocessing pipeline that achieves perfect accuracy in detecting function calls by analyzing fully expanded Rust code, eliminating all false positives from macro-hidden function calls.

## Requirements

### Functional Requirements

1. **Macro Expansion Pipeline**
   - Integrate cargo-expand as a preprocessing step before AST analysis
   - Handle both workspace and single-package projects
   - Support all Rust editions (2015, 2018, 2021, 2024)
   - Preserve original source location mappings for accurate reporting

2. **Expansion Caching**
   - Cache expanded code to avoid repeated compilation overhead
   - Implement cache invalidation based on source file modification times
   - Store cache in `.debtmap/cache/expanded/` directory
   - Support cache clearing via CLI flag

3. **Fallback Mechanism**
   - Gracefully fall back to current syn-based analysis if expansion fails
   - Detect and handle compilation errors without failing the entire analysis
   - Provide clear error messages when expansion is unavailable

4. **Source Mapping**
   - Map expanded code locations back to original source files
   - Preserve line numbers and function names in reports
   - Handle macro-generated code that doesn't exist in source

5. **Configuration**
   - Add `--expand-macros` CLI flag to enable expansion (opt-in initially)
   - Support `--no-expand-macros` to explicitly disable
   - Configuration file support: `expand_macros: true/false`
   - Environment variable: `DEBTMAP_EXPAND_MACROS=1`

### Non-Functional Requirements

1. **Performance**
   - Expansion should add no more than 2x overhead for initial runs
   - Cached runs should have minimal overhead (<10% slower)
   - Support parallel expansion for multi-crate workspaces
   - Optimize for incremental analysis

2. **Compatibility**
   - Work with cargo workspaces and standalone packages
   - Support custom target directories
   - Handle cross-compilation scenarios
   - Compatible with cargo features and conditional compilation

3. **Reliability**
   - Never fail the entire analysis due to expansion issues
   - Provide diagnostic information for expansion failures
   - Handle edge cases (proc macros, build scripts, etc.)

## Acceptance Criteria

- [ ] cargo-expand integration successfully expands all standard library macros
- [ ] Function calls within `println!`, `format!`, `assert!`, etc. are correctly detected
- [ ] `format_debt_score` is no longer incorrectly marked as dead code
- [ ] Expansion cache reduces subsequent analysis time by at least 50%
- [ ] Source location mapping correctly identifies original file and line numbers
- [ ] Fallback mechanism activates when cargo-expand is unavailable or fails
- [ ] Performance overhead is within acceptable limits (<2x for cold cache)
- [ ] All existing tests pass with expansion enabled
- [ ] New tests validate macro call detection accuracy
- [ ] Documentation includes expansion setup and troubleshooting

## Technical Details

### Implementation Approach

1. **Phase 1: Expansion Infrastructure**
   ```rust
   pub struct MacroExpander {
       cache_dir: PathBuf,
       cargo_path: PathBuf,
       workspace_root: PathBuf,
       cache: HashMap<PathBuf, ExpandedFile>,
   }
   
   pub struct ExpandedFile {
       original_path: PathBuf,
       expanded_content: String,
       source_map: SourceMap,
       timestamp: SystemTime,
   }
   ```

2. **Phase 2: Cargo Integration**
   ```rust
   impl MacroExpander {
       pub fn expand_file(&mut self, path: &Path) -> Result<ExpandedFile> {
           // Check cache first
           if let Some(cached) = self.get_cached(path) {
               return Ok(cached);
           }
           
           // Run cargo expand
           let output = Command::new(&self.cargo_path)
               .args(&["expand", "--lib", "--theme=none"])
               .arg(format!("--manifest-path={}", self.find_manifest(path)?))
               .output()?;
           
           // Parse and cache result
           let expanded = self.parse_expansion(output)?;
           self.cache_expanded(path, expanded)?;
           Ok(expanded)
       }
   }
   ```

3. **Phase 3: Source Mapping**
   ```rust
   pub struct SourceMap {
       mappings: Vec<Mapping>,
   }
   
   pub struct Mapping {
       expanded_line: usize,
       original_file: PathBuf,
       original_line: usize,
       is_macro_generated: bool,
   }
   ```

4. **Phase 4: Call Graph Integration**
   ```rust
   // Modify existing CallGraphExtractor
   impl CallGraphExtractor {
       pub fn extract_from_expanded(&mut self, expanded: &ExpandedFile) {
           let ast = syn::parse_file(&expanded.expanded_content)?;
           self.current_file = expanded.original_path.clone();
           self.source_map = Some(expanded.source_map.clone());
           self.visit_file(&ast);
       }
   }
   ```

### Architecture Changes

1. **New Module**: `src/expansion/`
   - `mod.rs`: Public API for expansion
   - `expander.rs`: Core expansion logic
   - `cache.rs`: Caching implementation
   - `source_map.rs`: Source location mapping

2. **Modified Components**:
   - `analyzers/rust.rs`: Add expansion preprocessing
   - `analyzers/rust_call_graph.rs`: Support expanded AST
   - `cli.rs`: Add expansion-related flags
   - `config.rs`: Add expansion configuration

### Data Structures

```rust
// Configuration
pub struct ExpansionConfig {
    pub enabled: bool,
    pub cache_dir: PathBuf,
    pub fallback_on_error: bool,
    pub parallel: bool,
    pub timeout: Duration,
}

// Cache format (JSON)
pub struct CacheEntry {
    pub version: String,  // Track debtmap version
    pub rust_version: String,  // Track rustc version
    pub original_hash: String,  // SHA-256 of original file
    pub expanded_content: String,
    pub source_mappings: Vec<SourceMapping>,
    pub timestamp: i64,
}
```

### APIs and Interfaces

```rust
// Public expansion API
pub trait MacroExpansion {
    fn expand_file(&mut self, path: &Path) -> Result<ExpandedFile>;
    fn expand_workspace(&mut self) -> Result<HashMap<PathBuf, ExpandedFile>>;
    fn clear_cache(&mut self) -> Result<()>;
    fn is_cache_valid(&self, path: &Path) -> bool;
}

// Integration with existing analyzer
impl RustAnalyzer {
    pub fn analyze_with_expansion(&mut self, config: ExpansionConfig) -> Result<AnalysisResults> {
        let expander = MacroExpander::new(config)?;
        let expanded_files = expander.expand_workspace()?;
        self.analyze_expanded(expanded_files)
    }
}
```

## Dependencies

- **Prerequisites**: 
  - Spec 21 (Dead Code Detection) must be implemented
  - Existing call graph infrastructure
  
- **Affected Components**:
  - `analyzers/rust.rs`: Will support both expanded and non-expanded analysis
  - `priority/call_graph.rs`: Must handle expanded source locations
  - `priority/unified_scorer.rs`: Benefits from improved accuracy
  
- **External Dependencies**:
  - cargo-expand (installed via cargo install cargo-expand)
  - rustc (required by cargo-expand)
  - syn continues to be used for parsing expanded code

## Testing Strategy

- **Unit Tests**:
  - Test expansion caching logic
  - Validate source mapping accuracy
  - Test fallback mechanisms
  - Verify configuration parsing

- **Integration Tests**:
  - Create test crates with various macro patterns
  - Test workspace expansion
  - Validate cross-crate macro calls
  - Test with proc-macro crates

- **Performance Tests**:
  - Benchmark expansion overhead
  - Measure cache effectiveness
  - Test parallel expansion speedup
  - Profile memory usage

- **Accuracy Tests**:
  - Verify all macro types are handled:
    - println!, format!, assert!, debug!
    - vec!, hashmap!, lazy_static!
    - Custom derive macros
    - Procedural macros
  - Compare results with manual verification

## Documentation Requirements

- **Code Documentation**:
  - Document expansion process flow
  - Explain caching strategy
  - Detail source mapping algorithm
  - Include troubleshooting guide

- **User Documentation**:
  - Installation guide for cargo-expand
  - Configuration examples
  - Performance tuning recommendations
  - Common issues and solutions

- **Architecture Updates**:
  - Update ARCHITECTURE.md with expansion pipeline
  - Document cache structure and invalidation
  - Explain fallback behavior

## Implementation Notes

### Key Considerations

1. **Compilation Requirements**:
   - Project must compile successfully for expansion to work
   - Build dependencies must be available
   - Consider Docker/CI environments where compilation might fail

2. **Performance Optimization**:
   - Expand only files that have changed
   - Use workspace-level expansion when possible
   - Consider memory-mapped files for large expansions

3. **Error Handling**:
   - Expansion failures should not block analysis
   - Provide clear diagnostics for common issues
   - Log expansion attempts for debugging

4. **Edge Cases**:
   - Conditional compilation (#[cfg])
   - Platform-specific code
   - Macro recursion limits
   - Generated code from build.rs

### Implementation Phases

1. **Phase 1**: Basic expansion infrastructure (1-2 days)
   - Implement MacroExpander
   - Basic cargo-expand integration
   - Simple file-based caching

2. **Phase 2**: Source mapping (2-3 days)
   - Implement source location tracking
   - Map expanded code to original
   - Handle macro-generated code

3. **Phase 3**: Integration (1-2 days)
   - Integrate with existing analyzers
   - Update call graph extraction
   - Modify dead code detection

4. **Phase 4**: Optimization (1-2 days)
   - Implement smart caching
   - Add parallel expansion
   - Performance tuning

5. **Phase 5**: Testing & Polish (2-3 days)
   - Comprehensive test suite
   - Documentation
   - Edge case handling

## Migration and Compatibility

### Breaking Changes
- None expected - expansion is opt-in via CLI flag

### Migration Path
1. Initial release with `--expand-macros` as experimental feature
2. Gather user feedback and fix issues
3. Enable by default in next major version
4. Provide `--no-expand-macros` for compatibility

### Compatibility Considerations
- Maintain backward compatibility with non-expanded analysis
- Support projects that cannot use cargo-expand
- Handle mixed Rust editions in workspaces
- Work with both stable and nightly Rust

### Configuration Migration
```toml
# .debtmap.toml
[analysis]
expand_macros = true  # New option

[cache]
expanded_code = ".debtmap/cache/expanded"  # New cache location
```

## Success Metrics

1. **Accuracy**: 100% detection rate for macro-hidden function calls
2. **Performance**: <2x overhead with cold cache, <10% with warm cache
3. **Reliability**: Zero analysis failures due to expansion issues
4. **Adoption**: 80% of users enable expansion after stable release
5. **False Positive Reduction**: Eliminate all macro-related false positives

## Risks and Mitigation

### Risk 1: Compilation Requirements
**Risk**: Projects with complex build requirements may fail to expand
**Mitigation**: Robust fallback to standard analysis, clear error messages

### Risk 2: Performance Overhead
**Risk**: Expansion could make analysis too slow for large projects
**Mitigation**: Aggressive caching, incremental expansion, parallel processing

### Risk 3: Version Compatibility
**Risk**: cargo-expand output format might change
**Mitigation**: Version detection, multiple parser implementations, regular testing

### Risk 4: CI/CD Integration
**Risk**: CI environments might not support cargo-expand
**Mitigation**: Automatic detection and fallback, documentation for CI setup

## Alternative Approaches Considered

1. **Pattern-Based Parsing** (Rejected)
   - Would only handle known macros
   - Requires constant maintenance
   - Cannot handle custom macros

2. **rustc Plugin** (Rejected)
   - Requires nightly Rust
   - Complex implementation
   - Unstable API

3. **rust-analyzer Integration** (Rejected)
   - Heavyweight dependency
   - Complex integration
   - Not designed for batch analysis

4. **Manual Macro Database** (Rejected)
   - High maintenance burden
   - Incomplete coverage
   - Version-specific implementations

## Future Enhancements

1. **Incremental Expansion**: Only re-expand changed files
2. **Distributed Cache**: Share expansion cache across team
3. **Macro Complexity Metrics**: Analyze macro usage patterns
4. **Custom Macro Handlers**: Plugin system for project-specific macros
5. **IDE Integration**: Provide expansion data to IDEs