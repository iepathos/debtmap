---
number: 132
title: Eliminate Redundant AST Parsing in Call Graph Construction
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-10-26
---

# Specification 132: Eliminate Redundant AST Parsing in Call Graph Construction

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The parallel call graph builder in `src/builders/parallel_call_graph.rs` currently performs redundant parsing operations that significantly impact performance. During the "Extracting cross-file call relationships" phase, files are parsed twice:

1. **Phase 1** (`parallel_parse_files`): Files are read from disk and content is stored as strings
2. **Phase 2** (`parallel_multi_file_extraction`): The same file content is re-parsed using `syn::parse_file`

For a 392-file codebase analysis, this results in:
- **Current**: 784 parse operations (392 files × 2 passes)
- **Optimal**: 392 parse operations (parse once, reuse)

Profiling shows that AST parsing with `syn::parse_file` is one of the most CPU-intensive operations in call graph construction. The redundant parsing causes the "Extracting cross-file call relationships" step to take several seconds when it could be much faster.

**Current Flow**:
```
parallel_parse_files() → Vec<(PathBuf, String)>
    ↓
parallel_multi_file_extraction() → syn::parse_file() AGAIN
    ↓
Call graph extraction
```

**Proposed Flow**:
```
parallel_parse_files() → Vec<(PathBuf, syn::File)>
    ↓
parallel_multi_file_extraction() → Use already-parsed AST
    ↓
Call graph extraction
```

## Objective

Eliminate redundant AST parsing in the parallel call graph construction pipeline by parsing each Rust file exactly once and reusing the parsed AST across all subsequent phases, achieving a 40-50% reduction in call graph construction time.

## Requirements

### Functional Requirements

1. **Single Parse Guarantee**
   - Each Rust source file must be parsed exactly once during call graph construction
   - Parsed AST (`syn::File`) must be reused across all phases
   - No behavioral changes to call graph construction logic

2. **Type Signature Updates**
   - Update `parallel_parse_files()` to return `Vec<(PathBuf, syn::File)>` instead of `Vec<(PathBuf, String)>`
   - Update `parallel_multi_file_extraction()` to accept pre-parsed ASTs
   - Update `parallel_enhanced_analysis()` signature if needed for consistency

3. **Memory Management**
   - Store parsed `syn::File` objects in memory between phases
   - Ensure ASTs are properly dropped after use to avoid memory accumulation
   - Monitor peak memory usage to stay within acceptable limits

4. **Error Handling Preservation**
   - Maintain existing error handling for parse failures
   - Continue to gracefully skip unparseable files
   - Preserve logging and progress reporting behavior

### Non-Functional Requirements

1. **Performance**
   - Achieve 40-50% reduction in call graph construction time
   - For 392-file project: Reduce from ~4-6 seconds to ~2-3 seconds
   - Maintain parallel processing efficiency
   - No regression in other analysis phases

2. **Memory Efficiency**
   - Peak memory increase: < 100MB for 392-file project
   - Memory usage should be proportional to number of files being processed
   - ASTs should be dropped promptly after final use

3. **Code Quality**
   - Maintain functional programming principles (pure functions, immutability)
   - Preserve existing test coverage
   - No new compiler warnings or clippy violations

## Acceptance Criteria

- [ ] `parallel_parse_files()` returns `Vec<(PathBuf, syn::File)>` with parsed ASTs
- [ ] `parallel_multi_file_extraction()` accepts and uses pre-parsed ASTs without re-parsing
- [ ] Each file is parsed exactly once during call graph construction
- [ ] Call graph construction time reduced by 40-50% for 392-file project
- [ ] All existing call graph tests pass without modification
- [ ] No increase in test failure rate or flakiness
- [ ] Memory usage increase is < 100MB for 392-file project
- [ ] No new clippy warnings introduced
- [ ] Progress reporting and error handling behavior unchanged
- [ ] Benchmark results confirm performance improvement

## Technical Details

### Implementation Approach

**File**: `src/builders/parallel_call_graph.rs`

**1. Update `parallel_parse_files` Method**

Current implementation:
```rust
fn parallel_parse_files(
    &self,
    rust_files: &[PathBuf],
    parallel_graph: &Arc<ParallelCallGraph>,
) -> Result<Vec<(PathBuf, String)>> {
    let parsed_files: Vec<_> = rust_files
        .par_iter()
        .filter_map(|file_path| {
            let content = io::read_file(file_path).ok()?;
            parallel_graph.stats().increment_files();
            Some((file_path.clone(), content))  // Returns String
        })
        .collect();
    Ok(parsed_files)
}
```

Optimized implementation:
```rust
fn parallel_parse_files(
    &self,
    rust_files: &[PathBuf],
    parallel_graph: &Arc<ParallelCallGraph>,
) -> Result<Vec<(PathBuf, syn::File)>> {  // Returns syn::File
    let parsed_files: Vec<_> = rust_files
        .par_iter()
        .progress_with(progress)
        .filter_map(|file_path| {
            let content = io::read_file(file_path).ok()?;
            let parsed = syn::parse_file(&content).ok()?;  // Parse once
            parallel_graph.stats().increment_files();
            Some((file_path.clone(), parsed))  // Return parsed AST
        })
        .collect();
    Ok(parsed_files)
}
```

**2. Update `parallel_multi_file_extraction` Method**

Current implementation:
```rust
fn parallel_multi_file_extraction(
    &self,
    parsed_files: &[(PathBuf, String)],  // Receives strings
    parallel_graph: &Arc<ParallelCallGraph>,
) -> Result<()> {
    parsed_files.par_chunks(chunk_size).for_each(|chunk| {
        let parsed_chunk: Vec<_> = chunk
            .iter()
            .filter_map(|(path, content)| {
                syn::parse_file(content)  // RE-PARSING HERE!
                    .ok()
                    .map(|parsed| (parsed, path.clone()))
            })
            .collect();

        if !parsed_chunk.is_empty() {
            let chunk_graph = extract_call_graph_multi_file(&parsed_chunk);
            parallel_graph.merge_concurrent(chunk_graph);
        }
    });
    Ok(())
}
```

Optimized implementation:
```rust
fn parallel_multi_file_extraction(
    &self,
    parsed_files: &[(PathBuf, syn::File)],  // Already parsed!
    parallel_graph: &Arc<ParallelCallGraph>,
) -> Result<()> {
    parsed_files.par_chunks(chunk_size).for_each(|chunk| {
        // No re-parsing needed - already have syn::File
        if !chunk.is_empty() {
            // Convert to expected format: Vec<(syn::File, PathBuf)>
            let chunk_for_extraction: Vec<_> = chunk
                .iter()
                .map(|(path, parsed)| (parsed.clone(), path.clone()))
                .collect();

            let chunk_graph = extract_call_graph_multi_file(&chunk_for_extraction);
            parallel_graph.merge_concurrent(chunk_graph);
        }
    });
    Ok(())
}
```

**3. Update `parallel_enhanced_analysis` Method**

Current implementation:
```rust
fn parallel_enhanced_analysis(
    &self,
    parsed_files: &[(PathBuf, String)],  // Receives strings
    parallel_graph: &Arc<ParallelCallGraph>,
) -> Result<(HashSet<FunctionId>, HashSet<FunctionId>)> {
    let workspace_files: Vec<(PathBuf, syn::File)> = parsed_files
        .iter()
        .filter_map(|(path, content)| {
            syn::parse_file(content)  // THIRD PARSE!
                .ok()
                .map(|parsed| (path.clone(), parsed))
        })
        .collect();
    // ... rest of implementation
}
```

Optimized implementation:
```rust
fn parallel_enhanced_analysis(
    &self,
    parsed_files: &[(PathBuf, syn::File)],  // Already parsed!
    parallel_graph: &Arc<ParallelCallGraph>,
) -> Result<(HashSet<FunctionId>, HashSet<FunctionId>)> {
    // No re-parsing needed - workspace_files is now just a reference conversion
    let workspace_files: Vec<(PathBuf, &syn::File)> = parsed_files
        .iter()
        .map(|(path, parsed)| (path.clone(), parsed))
        .collect();
    // ... rest of implementation (minimal changes)
}
```

### Architecture Changes

**Modified Components**:
- `src/builders/parallel_call_graph.rs`: All three phase methods updated
- Internal data flow changes only - no public API modifications

**Data Flow**:
```
Before:
  File I/O → String → Parse #1 → Process
                   → Parse #2 → Process
                   → Parse #3 → Process

After:
  File I/O → String → Parse (once) → Process all phases
```

### Memory Considerations

**AST Size Estimation**:
- Average `syn::File` size: ~50-200KB per file
- For 392 files: ~20-80MB total
- Peak memory during processing: < 100MB additional

**Memory Lifecycle**:
1. Parse files in parallel → Peak memory usage
2. Process call graph extraction → ASTs still in memory
3. Enhanced analysis → ASTs still in memory
4. Drop `parsed_files` vector → Memory freed

**Optimization**: Consider dropping ASTs in chunks if memory becomes constrained, but current analysis suggests this is unnecessary for typical projects.

## Dependencies

**Prerequisites**: None - this is a localized optimization

**Affected Components**:
- `src/builders/parallel_call_graph.rs`: Primary implementation
- Tests using `ParallelCallGraphBuilder`: May need minor updates

**External Dependencies**: None - uses existing `syn` crate functionality

## Testing Strategy

### Unit Tests

**Test existing behavior is preserved**:
```rust
#[test]
fn test_parallel_parse_files_returns_parsed_asts() {
    let builder = ParallelCallGraphBuilder::new();
    let test_files = vec![PathBuf::from("test.rs")];
    let graph = Arc::new(ParallelCallGraph::new(1));

    let result = builder.parallel_parse_files(&test_files, &graph).unwrap();

    assert_eq!(result.len(), 1);
    assert!(result[0].1.items.len() > 0);  // syn::File has items
}

#[test]
fn test_no_redundant_parsing() {
    // Mock test to verify parse is called once per file
    // Use instrumentation or counter to verify parse count
}
```

### Integration Tests

**Test full pipeline with parsed ASTs**:
```rust
#[test]
fn test_call_graph_construction_with_single_parse() {
    let project_path = PathBuf::from("test_project");
    let base_graph = CallGraph::new();

    let (graph, exclusions, ptr_used) =
        build_call_graph_parallel(&project_path, base_graph, None).unwrap();

    // Verify results are identical to previous implementation
    assert!(graph.get_all_functions().count() > 0);
}
```

### Performance Tests

**Benchmark parsing overhead reduction**:
```rust
#[bench]
fn bench_parallel_call_graph_construction(b: &mut Bencher) {
    let project_path = PathBuf::from("test_large_project");

    b.iter(|| {
        build_call_graph_parallel(&project_path, CallGraph::new(), None)
    });
}
```

**Expected Results**:
- Before: ~4-6 seconds for 392 files
- After: ~2-3 seconds for 392 files
- Improvement: 40-50% reduction

### Regression Tests

- Run full test suite to ensure no behavioral changes
- Verify all existing call graph tests pass
- Check for memory leaks using Valgrind or similar tools

## Documentation Requirements

### Code Documentation

**Update function documentation**:
```rust
/// Parse Rust files in parallel and return parsed ASTs
///
/// This function reads and parses files concurrently, returning
/// `syn::File` objects that can be reused across multiple analysis
/// phases without re-parsing.
///
/// # Performance
///
/// Parsing is performed once per file to eliminate redundant work.
/// The parsed ASTs are stored in memory until all phases complete.
fn parallel_parse_files(...) -> Result<Vec<(PathBuf, syn::File)>>
```

### Architecture Updates

Update `ARCHITECTURE.md` to document the optimization:
```markdown
## Call Graph Construction Pipeline

The parallel call graph builder processes files in three phases:

1. **Phase 1: Parallel Parsing** - Files are read and parsed once
2. **Phase 2: Multi-file Extraction** - Reuses parsed ASTs for call graph
3. **Phase 3: Enhanced Analysis** - Reuses parsed ASTs for trait/framework analysis

**Optimization**: Each file is parsed exactly once and the AST is reused
across all phases, reducing redundant CPU-intensive parsing operations.
```

### Performance Documentation

Add to `book/src/parallel-processing.md`:
```markdown
### AST Parsing Optimization

Debtmap parses each Rust source file exactly once during call graph
construction. The parsed Abstract Syntax Tree (AST) is stored in memory
and reused across multiple analysis phases, eliminating redundant parsing
operations that were previously performed.

**Performance Impact**: This optimization reduces call graph construction
time by 40-50% for large projects (300+ files).
```

## Implementation Notes

### Key Design Decisions

1. **Clone vs Reference**: Using `syn::File` clones is acceptable because:
   - AST clones are relatively cheap (Arc-based internally)
   - Simplifies lifetime management
   - Alternative of references adds complexity without significant benefit

2. **Error Handling**: Preserve existing behavior:
   - Parse errors are logged but don't fail the entire analysis
   - Unparseable files are skipped gracefully
   - Progress reporting continues for parseable files

3. **Memory vs CPU Trade-off**:
   - Accept ~50-100MB additional memory usage
   - Save 40-50% CPU time on parsing
   - Clear win for typical project sizes

### Potential Gotchas

1. **Memory Pressure**: For extremely large projects (5000+ files), consider:
   - Processing in batches
   - Streaming approach with immediate AST drop after use
   - Current implementation is optimized for typical projects (<1000 files)

2. **Type Conversions**: Some methods expect `(syn::File, PathBuf)` while others use `(PathBuf, syn::File)`:
   - Be careful with tuple ordering
   - Use clear variable names to avoid confusion

3. **Progress Reporting**: Ensure progress bars show correct total:
   - Files are only counted once during parse phase
   - Subsequent phases don't re-increment file counts

## Migration and Compatibility

### Breaking Changes

**None** - This is an internal optimization with no public API changes.

### Backward Compatibility

- All existing tests should pass without modification
- No changes to CLI interface or output format
- Cache format remains unchanged

### Rollback Plan

If performance issues are discovered:
1. Revert the three method signatures back to `String` types
2. Re-introduce parsing in each phase
3. No database or cache migrations needed

## Success Metrics

### Performance Metrics

- [ ] Call graph construction time reduced by 40-50%
- [ ] For 392-file project: < 3 seconds (down from 4-6 seconds)
- [ ] Memory usage increase: < 100MB peak

### Quality Metrics

- [ ] All tests pass
- [ ] No new clippy warnings
- [ ] No increase in error rates
- [ ] Code coverage maintained at 85%+

### Validation

Run before/after benchmarks:
```bash
# Before optimization
time debtmap analyze . --lcov target/coverage/lcov.info

# After optimization
time debtmap analyze . --lcov target/coverage/lcov.info

# Compare "Extracting cross-file call relationships" timing
```

Expected output:
```
Before: ⠙ Extracting cross-file call relationships (~4-6s)
After:  ⠙ Extracting cross-file call relationships (~2-3s)
```
