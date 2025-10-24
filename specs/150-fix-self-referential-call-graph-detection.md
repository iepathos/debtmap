---
number: 150
title: Fix Self-Referential Call Graph Detection
category: testing
priority: high
status: draft
dependencies: [142, 146, 149]
created: 2025-10-24
---

# Specification 150: Fix Self-Referential Call Graph Detection

**Category**: testing
**Priority**: high
**Status**: draft
**Dependencies**: Spec 142 (FunctionId module paths), Spec 146 (cross-module call resolution), Spec 149 (call graph diagnostics)

## Context

**Current state**:
- Test `test_self_referential_call_detection` is marked `#[ignore]` claiming it's "very slow"
- The test actually documents a **failing test** that reproduces a bug
- The test analyzes debtmap's own codebase to verify cross-file call resolution
- Test expects ≥3 callers for `process_rust_files_for_call_graph()` but finds 0

**The Bug**:
According to the test comments and assertions:
```rust
// This test directly reproduces the actual bug:
// process_rust_files_for_call_graph is called from validate.rs and unified_analysis.rs
// but shows 0 callers

assert!(
    caller_names.len() >= 3,
    "BUG REPRODUCED: Expected >= 3 callers for process_rust_files_for_call_graph, found {}",
    caller_names.len()
);
```

The function `src/builders/call_graph.rs::process_rust_files_for_call_graph()` is called from:
- `src/commands/validate.rs`
- `src/builders/unified_analysis.rs`

But the call graph reports 0 callers when analyzing the entire debtmap codebase.

**Related Work**:
- Spec 142 (Oct 23): Populated FunctionId module paths for qualified call resolution
- Commit 2fbc6d7 (Oct 23): Fixed cross-module issues with absolute paths, marked test as ignored
- The test was marked `#[ignore]` with misleading comment "very slow" instead of "failing"

**Problem**:
This violates test-driven development principles:
1. **Ignored failing tests hide technical debt**
2. **Misleading comments** - Claims performance issue when it's actually a bug
3. **False CI confidence** - Green CI with hidden failing test
4. **No tracking** - Bug not documented in specs or issues

## Objective

Investigate and fix the self-referential call graph detection bug, or properly document why the test is invalid, enabling the test to be un-ignored and run in CI.

## Requirements

### Functional Requirements
1. **Investigate Current Behavior**
   - Run `test_self_referential_call_detection` with `--ignored` flag
   - Capture actual vs expected behavior
   - Determine if bug still exists after spec 142 implementation

2. **Root Cause Analysis**
   - If test fails: Identify why `process_rust_files_for_call_graph` shows 0 callers
   - Examine how call graph handles analyzing its own source code
   - Check if module path resolution works for debtmap's own codebase
   - Verify PathResolver correctly matches qualified calls

3. **Fix or Invalidate**
   - **If bug exists**: Fix the underlying call resolution issue
   - **If test passes**: Remove `#[ignore]` attribute and update comment
   - **If test is invalid**: Remove test and document why in commit message

### Non-Functional Requirements
- **No Hidden Failures**: All tests in CI must either pass or be properly documented as known issues with specs
- **Clear Intent**: Test comments must accurately describe why tests are ignored (if any remain)
- **Maintainability**: If issue can't be fixed immediately, create follow-up spec and link from test comment

## Acceptance Criteria

- [ ] Test `test_self_referential_call_detection` runs without `#[ignore]` attribute
- [ ] Test passes in CI (or is removed if invalid)
- [ ] If bug was real and fixed: Add regression test coverage
- [ ] If test was invalid: Document why in commit message
- [ ] No misleading comments about test performance when real issue is correctness
- [ ] All cross-file call resolution works correctly for debtmap's own codebase
- [ ] CI passes with no ignored tests hiding bugs

## Technical Details

### Investigation Approach

1. **Run the Ignored Test**
   ```bash
   cargo test test_self_referential_call_detection -- --ignored --nocapture
   ```

2. **Analyze Output**
   - Check how many callers are actually found
   - Verify if validate.rs and unified_analysis.rs functions are in call graph
   - Examine module_path values for relevant FunctionIds
   - Check PathResolver resolution logs

3. **Possible Root Causes**

   **A. Self-Analysis Exclusion**
   - Call graph might exclude certain files when analyzing project root
   - `.gitignore` or debtmap config might exclude src/commands/ or src/builders/

   **B. Module Path Mismatch**
   - FunctionId.module_path might not match for qualified calls in this specific case
   - Module paths might differ between function definition and call sites

   **C. PathResolver Strategy Gaps**
   - Spec 142 fixed qualified call resolution, but might miss edge cases
   - Strategy for matching calls within same crate might differ from cross-crate

   **D. Test Assumptions Invalid**
   - Function might be called differently than test expects
   - Test might be checking wrong function signature or module path

### Implementation Strategy

**Phase 1: Diagnosis**
```rust
// Add diagnostic output to understand current state
let caller_names = call_graph.get_callers_by_name("process_rust_files_for_call_graph");
println!("Found {} callers", caller_names.len());

// Check if source files are even in the call graph
let all_functions = call_graph.find_all_functions();
let validate_funcs: Vec<_> = all_functions.iter()
    .filter(|f| f.file.to_string_lossy().contains("validate.rs"))
    .collect();
```

**Phase 2: Fix Based on Diagnosis**

*If files are excluded*:
- Update call graph file discovery to include all Rust files
- Check config exclusions and adjust for self-analysis

*If module paths don't match*:
- Debug why module_path differs between definition and call sites
- Extend spec 142 fixes to cover this edge case

*If PathResolver misses strategy*:
- Add new matching strategy for within-crate qualified calls
- Update PathResolver to try additional resolution approaches

*If test is invalid*:
- Determine correct expectations
- Either fix test or remove it with documentation

**Phase 3: Regression Prevention**
- Add unit tests for specific resolution scenarios
- Add integration test for common call patterns
- Document any limitations in ARCHITECTURE.md

### Data Structures

No new data structures needed. Working with existing:
- `CallGraph` - stores function relationships
- `FunctionId` - identifies functions with module_path
- `PathResolver` - resolves qualified calls to FunctionIds

### APIs and Interfaces

No API changes needed. May need to enhance existing:
- `PathResolver::resolve_qualified_call()` - might need additional strategies
- `CallGraph::get_callers_by_name()` - works as-is

## Dependencies

- **Prerequisites**:
  - Spec 142: FunctionId module paths must be populated
  - Spec 146: Cross-module call resolution must work
  - Spec 149: Diagnostic tools available for debugging

- **Affected Components**:
  - `tests/call_graph_cross_file_resolution_test.rs` - remove `#[ignore]`
  - `src/analyzers/call_graph/path_resolver.rs` - may need fixes
  - `src/analyzers/call_graph/module_tree.rs` - module path inference
  - `src/builders/call_graph.rs` - file discovery might need adjustment

- **External Dependencies**: None

## Testing Strategy

### Diagnostic Testing
1. **Run ignored test** with full output
2. **Capture metrics**:
   - Number of functions found in call graph
   - Number of callers detected
   - Module paths for target function and callers
3. **Validate assumptions**:
   - Are validate.rs and unified_analysis.rs in call graph?
   - Is process_rust_files_for_call_graph registered correctly?
   - Do qualified calls resolve properly?

### Unit Tests
- Test PathResolver with debtmap's actual module structure
- Test module_path inference for src/builders/ and src/commands/
- Test qualified call resolution for same-crate calls

### Integration Tests
- Keep `test_self_referential_call_detection` (without #[ignore])
- Add smaller targeted tests for specific scenarios
- Test call graph self-analysis with minimal reproduction cases

### Regression Tests
- If bug is fixed, ensure test covers the specific failure mode
- Add test for any new resolution strategies
- Verify fix works for both absolute and relative paths

## Documentation Requirements

### Code Documentation
- Update test comment to accurately describe what it tests
- Document any limitations in cross-file resolution
- Add inline comments for any new resolution strategies

### User Documentation
- If self-analysis has limitations, document in README
- Update ARCHITECTURE.md with call resolution details

### Architecture Updates
If significant changes to call resolution:
- Document PathResolver strategies in ARCHITECTURE.md
- Explain module path inference approach
- Describe self-analysis capabilities and limitations

## Implementation Notes

### Common Pitfalls
1. **Don't hide failing tests** - Either fix or document properly
2. **Performance is not the issue** - Test appears to be checking correctness, not speed
3. **Self-analysis edge cases** - Analyzing own code might have special requirements

### Best Practices
1. Run the test first before making changes
2. Add diagnostic output to understand current behavior
3. Fix root cause, not symptoms
4. If can't fix now, create proper spec and remove #[ignore]

### Investigation Checklist
- [ ] Run test and capture full output
- [ ] Verify files are included in call graph
- [ ] Check module_path values match expectations
- [ ] Test PathResolver resolution manually
- [ ] Review spec 142 implementation for gaps
- [ ] Check if recent commits (2fbc6d7) actually fixed issue

## Migration and Compatibility

### Breaking Changes
None expected. This is a bug fix.

### Compatibility
- Fix should work for all Rust projects, including debtmap itself
- Self-analysis should work same as analyzing external projects
- No changes to public APIs

### Migration
No migration needed. Simply remove `#[ignore]` when test passes.

## Success Metrics

- **Primary**: Test `test_self_referential_call_detection` passes in CI
- **Secondary**: All cross-file calls correctly detected in debtmap's own codebase
- **Tertiary**: No ignored tests hiding bugs (only performance-related ignores allowed)

## Open Questions

1. **Why was test marked ignored instead of fixed?**
   - Was it actually slow, or just easier to hide the failure?
   - Was there a plan to fix later that was forgotten?

2. **Does the bug still exist after spec 142?**
   - Test hasn't been run since being marked ignored
   - Spec 142 might have already fixed it

3. **Is self-analysis supposed to work?**
   - Maybe debtmap analyzing itself is not a supported use case?
   - If not supported, test should be removed, not ignored

4. **Are there other ignored tests hiding bugs?**
   - Should audit all `#[ignore]` attributes for similar issues

## Related Issues

- Ignored test created: commit 2fbc6d7 (Oct 23, 2025)
- Spec 142 implementation: commit 912b9c1 (Oct 23, 2025)
- Cross-module fixes: commit 2fbc6d7 (Oct 23, 2025)

## Implementation Priority

**High Priority** because:
1. Violates TDD principles (hiding failing tests)
2. Creates false confidence in CI
3. Bug might affect real usage (if it still exists)
4. Quick investigation will determine if real fix is needed
5. Sets bad precedent for handling test failures
