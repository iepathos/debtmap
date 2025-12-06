---
number: 201
title: Error Visibility and Logging
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-12-06
---

# Specification 201: Error Visibility and Logging

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently swallows approximately **95 instances** of errors throughout the codebase without any visibility to users or developers. This violates **Stillwater Principle #3: "Errors Should Tell Stories"** and makes debugging and understanding incomplete analysis results nearly impossible.

### Current Problem Patterns

**Pattern 1: `.ok()` Without Logging (~58 instances)**
```rust
// File I/O errors silently discarded
let content = fs::read_to_string(&m.file).ok()?;  // ❌ No indication of what failed
```

**Pattern 2: `.filter_map(|e| e.ok())` (~3 instances)**
```rust
// Directory traversal errors silently dropped
for entry in WalkDir::new(root)
    .into_iter()
    .filter_map(|e| e.ok())  // ❌ Permission errors invisible
```

**Pattern 3: Empty Error Handlers (~12 instances)**
```rust
Err(_) => return HashSet::new(),  // ❌ Parse failures = empty results, no warning
```

**Pattern 4: Generic Error Conversion (~5 instances)**
```rust
.map_err(|_| AnalysisError::other("Type mismatch"))  // ❌ Original error details lost
```

### Impact

Users experience:
- Incomplete analysis with no explanation of what was skipped
- Silent failures that look like success
- No indication of permission issues, parse failures, or I/O errors
- Debugging requires code inspection rather than log output

Developers experience:
- Production issues with no diagnostic information
- Inability to identify which files cause problems
- No visibility into error patterns or frequency

### Stillwater Philosophy Violation

From Stillwater PHILOSOPHY.md:

> **Errors Should Tell Stories**
>
> Deep call stacks lose context:
> ```
> Error: No such file or directory
> ```
> Which file? What were we trying to do? Why?

Current debtmap practice loses even more context by not reporting errors at all.

## Objective

Add comprehensive error visibility and logging to all error swallowing patterns in debtmap, following these principles:

1. **Make errors visible** - Log or warn for every swallowed error
2. **Preserve context** - Include file paths, operation details, and error messages
3. **Minimal code changes** - Add logging without architectural refactoring
4. **User actionability** - Errors should help users understand and fix issues
5. **Performance neutral** - Logging should not impact analysis performance

This is the **pragmatic immediate fix (Phase 1)** that provides visibility while maintaining current architecture.

## Requirements

### Functional Requirements

1. **Error Logging for `.ok()` Patterns**
   - Add `.map_err()` before all `.ok()` calls in critical paths
   - Log file paths, operations, and error messages
   - Use `eprintln!` for user-visible warnings
   - Use `log::warn!` for debug-level diagnostics
   - Examples:
     ```rust
     // Before
     let content = fs::read_to_string(&m.file).ok()?;

     // After
     let content = fs::read_to_string(&m.file)
         .map_err(|e| eprintln!("Warning: Failed to read {}: {}", m.file.display(), e))
         .ok()?;
     ```

2. **Error Logging for Filter Chains**
   - Replace `.filter_map(|e| e.ok())` with explicit error handling
   - Log each skipped entry with reason
   - Count and summarize skipped entries
   - Examples:
     ```rust
     // Before
     .filter_map(|e| e.ok())

     // After
     .filter_map(|e| match e {
         Ok(entry) => Some(entry),
         Err(err) => {
             eprintln!("Warning: Skipping entry: {}", err);
             None
         }
     })
     ```

3. **Error Logging for Empty Handlers**
   - Add logging to all `Err(_) => {}` or `Err(_) => return default` patterns
   - Include context about what operation failed
   - Log the actual error, not just the pattern
   - Examples:
     ```rust
     // Before
     Err(_) => return HashSet::new(),

     // After
     Err(e) => {
         eprintln!("Warning: Failed to parse dependencies for {}: {}", path.display(), e);
         return HashSet::new();
     }
     ```

4. **Preserve Error Context in Conversions**
   - Replace `.map_err(|_| ...)` with `.map_err(|e| ...)`
   - Include original error in new error message
   - Maintain error chain for debugging
   - Examples:
     ```rust
     // Before
     .map_err(|_| AnalysisError::other("Type mismatch"))

     // After
     .map_err(|e| AnalysisError::other(format!("Type mismatch: {}", e)))
     ```

5. **High-Impact Areas First**
   - Prioritize file I/O operations (file reading, parsing)
   - Directory traversal (WalkDir patterns)
   - Thread pool configuration
   - Analysis pipeline errors
   - Lower priority: TUI cleanup, cache operations

### Non-Functional Requirements

1. **Performance**
   - Logging should not measurably impact analysis performance
   - No blocking I/O in hot paths
   - stderr writes are acceptable (already buffered by OS)

2. **User Experience**
   - Warning messages should be actionable
   - Include enough context to understand the issue
   - Don't overwhelm users with too many warnings
   - Summarize patterns (e.g., "Skipped 15 files due to permission errors")

3. **Developer Experience**
   - Clear error messages for debugging
   - Consistent logging format across codebase
   - Easy to identify error patterns in production

## Acceptance Criteria

- [ ] All 58 `.ok()` calls in critical paths have `.map_err()` logging before them
- [ ] All 3 `.filter_map(|e| e.ok())` patterns replaced with explicit error handling
- [ ] All 12+ empty error handlers (`Err(_) => {}`) have logging added
- [ ] All 5 generic error conversions preserve original error details
- [ ] File I/O errors (reading/parsing) log file path and error message
- [ ] Directory traversal errors log which paths were skipped and why
- [ ] Thread pool configuration failures are logged
- [ ] Cache deserialization errors are logged
- [ ] No performance regression (benchmark within 5%)
- [ ] All existing tests pass
- [ ] No new clippy warnings introduced
- [ ] Error messages tested manually for clarity and actionability

## Technical Details

### Implementation Approach

**Phase 1a: High-Impact Critical Paths**

Target these files first (12-15 instances):

1. **`src/builders/parallel_unified_analysis.rs`**
   - Lines 58-59: File read/parse in purity analysis
   - Line 378: Thread pool configuration
   - Line 973: God object analysis

2. **`src/builders/parallel_call_graph.rs`**
   - Lines 172, 185: File reading and parsing in call graph

3. **`src/organization/codebase_type_analyzer.rs`**
   - Line 177: Directory walking with `.filter_map(|e| e.ok())`

4. **`src/pipeline/stages/standard.rs`**
   - Line 329: File system enumeration with `.filter_map(|e| e.ok())`

**Phase 1b: Medium-Impact Detection Paths**

5. **`src/analysis/framework_patterns_multi/detector.rs`**
   - Lines 294, 329, 344: Framework pattern detection errors

6. **`src/data_flow/population.rs`**
   - Lines 165, 170, 211, 216: Parse/read failures in data flow

7. **`src/io/effects.rs`**
   - Line 287: Cache deserialization errors

**Phase 1c: Lower-Impact Utility Paths**

8. **TUI cleanup operations** (`src/tui/mod.rs`)
9. **Progress tracking** (`src/io/progress.rs`)
10. **Environment variable parsing** (`src/config/multi_source.rs`)

### Logging Patterns

**Pattern 1: File I/O with Path Context**
```rust
// Template for file reading errors
fs::read_to_string(path)
    .map_err(|e| {
        eprintln!("Warning: Failed to read file {}: {}", path.display(), e);
        e  // Preserve original error
    })
    .ok()?;
```

**Pattern 2: Parsing with File Context**
```rust
// Template for parsing errors
syn::parse_file(&content)
    .map_err(|e| {
        eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
        e
    })
    .ok()?;
```

**Pattern 3: Directory Traversal with Summary**
```rust
// Template for directory walking
let mut skipped_count = 0;
let entries: Vec<_> = WalkDir::new(root)
    .into_iter()
    .filter_map(|e| match e {
        Ok(entry) => Some(entry),
        Err(err) => {
            if skipped_count < 10 {  // Limit detailed messages
                eprintln!("Warning: Skipping path: {}", err);
            }
            skipped_count += 1;
            None
        }
    })
    .collect();

if skipped_count > 10 {
    eprintln!("Warning: Skipped {} additional entries", skipped_count - 10);
}
```

**Pattern 4: Error Conversion Preserving Context**
```rust
// Template for error conversions
.map_err(|e| {
    AnalysisError::other(format!("Operation failed: {}", e))
})
```

**Pattern 5: Empty Handler with Logging**
```rust
// Template for empty error handlers
Err(e) => {
    eprintln!("Warning: Operation failed for {}: {}", context, e);
    return default_value;
}
```

### Architecture Changes

**Before:**
```
analyze_file(path)
  ├─ fs::read_to_string().ok()?  // ❌ Silent failure
  └─ syn::parse_file().ok()?     // ❌ Silent failure
```

**After:**
```
analyze_file(path)
  ├─ fs::read_to_string()
  │   .map_err(|e| eprintln!("Warning: ..."))  // ✓ Logged
  │   .ok()?
  └─ syn::parse_file()
      .map_err(|e| eprintln!("Warning: ..."))  // ✓ Logged
      .ok()?
```

No structural changes - just adding visibility layer.

### Error Message Format

**Consistent format for all warnings:**
```
Warning: {Operation} failed for {Context}: {Error}
```

**Examples:**
```
Warning: Failed to read file src/main.rs: Permission denied (os error 13)
Warning: Failed to parse src/main.rs: expected item, found `}`
Warning: Skipping directory entry: IO error for operation on .git: Permission denied
Warning: Failed to configure thread pool with 0 threads: thread count must be positive
Warning: Failed to deserialize cache value: invalid type: string "foo", expected u32
```

### Data Structures

No new data structures needed. All changes are logging additions to existing error paths.

### APIs and Interfaces

**No API changes** - This is purely additive internal logging.

Public APIs remain unchanged. Only internal error handling gains visibility.

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/builders/parallel_unified_analysis.rs`
  - `src/builders/parallel_call_graph.rs`
  - `src/organization/codebase_type_analyzer.rs`
  - `src/pipeline/stages/standard.rs`
  - `src/analysis/framework_patterns_multi/detector.rs`
  - `src/data_flow/population.rs`
  - `src/io/effects.rs`
  - Multiple other files with `.ok()` patterns
- **External Dependencies**: None (uses standard library `eprintln!`)

## Testing Strategy

### Manual Testing

```bash
# Test file permission errors
chmod 000 test.rs
cargo run -- analyze .
# Should see: "Warning: Failed to read file test.rs: Permission denied"

# Test parse errors
echo "invalid rust" > bad.rs
cargo run -- analyze .
# Should see: "Warning: Failed to parse bad.rs: ..."

# Test directory permission errors
chmod 000 secret_dir/
cargo run -- analyze .
# Should see: "Warning: Skipping path: ..."
```

### Automated Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_messages_include_context() {
        // Capture stderr output
        let result = analyze_file_with_error();

        // Verify error was logged (not just swallowed)
        // Note: Testing stderr capture is tricky, may need integration test
    }

    #[test]
    fn test_analysis_continues_after_errors() {
        // Ensure errors don't break analysis of other files
        let results = analyze_files_with_some_errors();

        assert!(!results.is_empty());  // Should have some successes
    }
}
```

### Performance Testing

```bash
# Benchmark before and after logging additions
cargo bench analyze_large_codebase

# Verify < 5% performance impact
```

## Documentation Requirements

### Code Documentation

Add comments explaining the error logging pattern:

```rust
/// Analyzes file with error logging for diagnostics.
///
/// Errors during file reading or parsing are logged to stderr
/// but do not stop analysis of other files. This allows users
/// to see which files caused problems and why.
pub fn analyze_file(path: &Path) -> Option<FileMetrics> {
    // Log file read errors before converting to Option
    let content = fs::read_to_string(path)
        .map_err(|e| eprintln!("Warning: Failed to read {}: {}", path.display(), e))
        .ok()?;

    // Log parse errors before converting to Option
    let ast = syn::parse_file(&content)
        .map_err(|e| eprintln!("Warning: Failed to parse {}: {}", path.display(), e))
        .ok()?;

    Some(calculate_metrics(&ast))
}
```

### User Documentation

No user documentation changes needed. Error messages are self-explanatory.

### Architecture Updates

Add to `ARCHITECTURE.md`:

```markdown
## Error Handling and Visibility

Debtmap follows the principle that errors should be visible and actionable.

### Error Logging Pattern

When errors are non-fatal (analysis can continue), they are logged
before being converted to `Option`:

```rust
// ✓ Good: Log before discarding error
let result = operation()
    .map_err(|e| eprintln!("Warning: Operation failed: {}", e))
    .ok();

// ❌ Bad: Silent error swallowing
let result = operation().ok();
```

### Error Message Format

All error messages follow this format:
```
Warning: {Operation} failed for {Context}: {Error}
```

This provides users with:
- What operation failed
- What file/path/context was involved
- The underlying error message
```

## Implementation Notes

### Implementation Order

1. **High-impact file I/O** (parallel_unified_analysis.rs, parallel_call_graph.rs)
2. **Directory traversal** (codebase_type_analyzer.rs, standard.rs)
3. **Analysis pipeline errors** (framework patterns, data flow)
4. **Cache and config errors** (effects.rs, multi_source.rs)
5. **Low-impact TUI/progress** (tui/mod.rs, progress.rs)

### Common Pitfalls

1. **Too many warnings** - Use summarization for repeated errors
2. **Blocking I/O** - `eprintln!` is buffered, but don't log in tight loops
3. **Lost error chains** - Remember to include original error in conversions
4. **Inconsistent format** - Use standard "Warning: {op} {ctx}: {err}" format

### Verification Checklist

For each changed error path:

- [ ] Error message includes operation description
- [ ] Error message includes file path or context
- [ ] Error message includes original error text
- [ ] Error is logged before `.ok()` call
- [ ] Format matches standard: "Warning: {op} {ctx}: {err}"
- [ ] No performance impact in benchmarks
- [ ] Manual test verifies message appears
- [ ] Message is actionable for users

### Example Commits

```bash
# Commit template for each phase
git commit -m "feat: add error logging to file I/O operations

Add visibility to file reading and parsing errors in parallel
analysis builders. Errors are logged with file paths and error
messages before being converted to Option.

- Log file read errors in parallel_unified_analysis.rs
- Log parse errors in parallel_call_graph.rs
- Include file paths in all error messages
- Preserve original error details

Relates to spec 201."
```

## Migration and Compatibility

### Breaking Changes

**None** - Purely additive logging. No API or behavior changes.

### User-Visible Changes

Users will now see warning messages for:
- Files that couldn't be read (permissions, not found, etc.)
- Files that couldn't be parsed (syntax errors, encoding issues)
- Directories that couldn't be accessed
- Other previously silent failures

This is an improvement - users can now understand incomplete results.

### Migration Steps

No migration needed. New logging appears automatically on next run.

## Success Metrics

- ✅ 95% reduction in silent error swallowing
- ✅ All critical file I/O errors logged with context
- ✅ Users can identify which files cause analysis issues
- ✅ Error messages tested and verified actionable
- ✅ No performance regression (< 5% overhead)
- ✅ All existing tests pass
- ✅ Manual testing confirms error visibility

## Follow-up Work

After this specification:
- **Spec 202**: Error Collection and Reporting (systematic error aggregation)
- **Spec 183**: Analyzer I/O Separation (architectural refactoring)
- **Spec 187**: Extract Pure Functions (eliminate error-prone patterns)

This specification is the **pragmatic immediate fix**. Future specs will address root architectural causes.

## References

- **Stillwater PHILOSOPHY.md** - "Errors Should Tell Stories" principle
- **CLAUDE.md** - Error handling standards
- **Error Swallowing Analysis** - ~95 instances identified across codebase
- **Spec 202** - Error Collection and Reporting (next phase)
- **Spec 183** - Analyzer I/O Separation (architectural fix)
