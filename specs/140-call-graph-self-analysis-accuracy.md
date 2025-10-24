---
number: 140
title: Call Graph Self-Analysis Accuracy
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-01-23
---

# Specification 140: Call Graph Self-Analysis Accuracy

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

**Current Problem**: Debtmap's call graph analysis has a critical self-referential blindspot where it cannot detect calls to its own call graph building functions, leading to false "dead code" warnings.

**Real-World False Positive**:
```
#4 SCORE: 13.7 [🔴 UNTESTED] [CRITICAL]
├─ LOCATION: ./src/builders/call_graph.rs:53 process_rust_files_for_call_graph()
├─ CALLS: 0 callers, 2 callees
│   ⚠ No callers detected - may be dead code
```

**Reality**: This function is actually called from 3 locations:
```rust
// Called by:
src/commands/validate.rs:258
src/builders/unified_analysis.rs:555
src/builders/unified_analysis.rs:583
```

**Root Cause**: The call graph builder cannot see calls TO itself because:
1. It's building the call graph while being called
2. The unified analysis phase that calls it hasn't been analyzed yet
3. Call graph construction happens before cross-file call resolution
4. Circular dependency: call graph analysis depends on having a call graph

**Impact**:
- **False positives**: Flags actively-used infrastructure as dead code
- **User confusion**: Developers waste time investigating "dead" code that's critical
- **Trust erosion**: When #4 recommendation is completely wrong, users doubt all recommendations
- **Wasted effort**: High-priority false positive diverts attention from real issues

**Frequency**: Affects all functions in:
- `src/builders/call_graph.rs` - Call graph construction
- `src/builders/unified_analysis.rs` - Analysis orchestration
- Any infrastructure function called during analysis but before call graph is complete

## Objective

Eliminate call graph self-analysis blindspots by implementing AST-based verification before flagging functions as "dead code", ensuring debtmap can accurately analyze its own codebase.

**Success Criteria**:
- `process_rust_files_for_call_graph()` correctly shows 3 callers (not 0)
- Zero false "no callers detected" warnings on debtmap's own codebase
- <100ms AST verification overhead per flagged function

## Requirements

### Functional Requirements

**FR1: Pre-Flag AST Verification**
- Before marking a function as "no callers detected", perform AST grep verification
- Search entire codebase for direct calls to the function
- Parse function calls in all languages (Rust, Python, JavaScript, TypeScript)
- Distinguish between function definitions and function calls

**FR2: Enhanced Call Graph Construction**
- Build call graph in multiple passes to capture self-referential calls
- First pass: Collect all function definitions
- Second pass: Resolve calls within each file
- Third pass: Resolve cross-file calls
- Fourth pass: Verify "no callers" claims with AST search

**FR3: Caller Detection Confidence Levels**
- **High confidence "dead code"**: AST verification confirms zero calls
- **Medium confidence**: Call graph shows no callers, AST finds calls in comments/tests
- **Low confidence**: Call graph incomplete, skip "dead code" recommendation
- Show confidence level in output

**FR4: Self-Analysis Mode**
- Detect when analyzing debtmap's own codebase
- Use stricter verification for infrastructure functions
- Flag potential blindspots in analysis output
- Provide warnings when call graph may be incomplete

**FR5: Cross-File Call Tracking**
- Track which files have been analyzed vs not yet analyzed
- Mark functions as "incomplete analysis" if called from unanalyzed files
- Defer "dead code" classification until all files processed
- Re-verify after full analysis complete

### Non-Functional Requirements

**NFR1: Performance**
- AST verification adds <100ms per flagged function
- Cache AST parse results to avoid re-parsing
- Use parallel processing for verification across files
- Early exit on first call found (don't need to find all)

**NFR2: Accuracy**
- False positive rate for "no callers" reduced to <5% (currently ~100% for infrastructure)
- Zero false positives on debtmap's own codebase
- Handle edge cases: macros, generics, trait methods, closures

**NFR3: Maintainability**
- Separate verification logic from call graph construction
- Use pure functions for AST-based call detection
- Make verification strategy pluggable per language
- Support disabling verification via config for performance

## Acceptance Criteria

### AC1: AST-Based Call Verification
- [ ] Implement `verify_function_calls_ast()` function for Rust
- [ ] Search all .rs files for direct function calls (e.g., `process_rust_files_for_call_graph(`)
- [ ] Parse calls in function bodies, not just comments/strings
- [ ] Return list of (file, line) tuples where function is called
- [ ] Handle qualified calls: `call_graph::process_rust_files_for_call_graph()`

### AC2: Python AST Verification
- [ ] Implement AST call verification for Python
- [ ] Detect calls via `ast.Call` nodes
- [ ] Handle method calls: `obj.method()`
- [ ] Handle function calls: `function()`
- [ ] Track import aliases (e.g., `from module import func as f`)

### AC3: JavaScript/TypeScript Verification
- [ ] Implement AST call verification for JS/TS
- [ ] Parse calls using swc or tree-sitter
- [ ] Handle ES6 imports and require()
- [ ] Detect method calls and function calls
- [ ] Handle destructured imports

### AC4: Multi-Pass Call Graph Construction
- [ ] Implement pass 1: Collect all function definitions across all files
- [ ] Implement pass 2: Resolve intra-file calls
- [ ] Implement pass 3: Resolve cross-file calls with full context
- [ ] Implement pass 4: AST verification for "no callers" functions
- [ ] Track analysis progress per file

### AC5: Confidence Scoring
- [ ] Assign confidence score to dead code findings:
  - **1.0 (High)**: Call graph + AST both confirm zero calls
  - **0.7 (Medium)**: Call graph shows no calls, AST verification inconclusive
  - **0.4 (Low)**: Call graph incomplete, analysis may have blindspots
- [ ] Display confidence in output: `⚠ No callers detected (confidence: 42%)`
- [ ] Skip recommendation if confidence <0.5
- [ ] Add to unified scoring calculation

### AC6: Self-Analysis Detection
- [ ] Detect if analyzing debtmap's own codebase (check for src/builders/call_graph.rs)
- [ ] Enable strict verification mode for self-analysis
- [ ] Log warnings for potential blindspots
- [ ] Add metadata to output: "Self-analysis mode: stricter verification enabled"

### AC7: Fix process_rust_files_for_call_graph False Positive
- [ ] Correctly detect 3 callers for `process_rust_files_for_call_graph()`
- [ ] Remove "⚠ No callers detected" warning
- [ ] Update output to show actual callers:
  ```
  ├─ CALLS: 3 callers, 2 callees
  │  ├─ CALLERS: validate::create_validate_call_graph_internal,
  │                unified_analysis::build_call_graph_from_cache,
  │                unified_analysis::build_call_graph_without_cache
  ```
- [ ] Verify with integration test

### AC8: Integration Test Coverage
- [ ] Test on debtmap's own codebase - zero false "no callers" warnings
- [ ] Test on sample project with actual dead code - correctly identified
- [ ] Test on project with infrastructure functions - not flagged as dead
- [ ] Test performance overhead <100ms per verification
- [ ] Test with incomplete analysis (some files excluded) - mark as low confidence

## Technical Details

### Implementation Approach

**Phase 1: AST-Based Call Verification**

Create a new module `src/verification/call_verification.rs`:

```rust
use syn::visit::Visit;
use std::path::Path;

pub struct CallVerificationResult {
    pub function_name: String,
    pub call_locations: Vec<CallLocation>,
    pub confidence: f64,
}

pub struct CallLocation {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub context: String, // surrounding code for context
}

/// Verify if a function is actually called using AST analysis
pub fn verify_function_calls_ast(
    function_name: &str,
    search_paths: &[PathBuf],
    language: Language,
) -> CallVerificationResult {
    match language {
        Language::Rust => verify_rust_calls(function_name, search_paths),
        Language::Python => verify_python_calls(function_name, search_paths),
        Language::JavaScript | Language::TypeScript => {
            verify_js_calls(function_name, search_paths)
        }
    }
}

/// Rust-specific call verification
fn verify_rust_calls(
    function_name: &str,
    search_paths: &[PathBuf],
) -> CallVerificationResult {
    let mut call_locations = Vec::new();

    for path in search_paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(parsed) = syn::parse_file(&content) {
                let visitor = CallVisitor {
                    target_function: function_name,
                    file_path: path.clone(),
                    locations: Vec::new(),
                };

                // Walk AST to find function calls
                let locations = visitor.find_calls(&parsed);
                call_locations.extend(locations);
            }
        }
    }

    let confidence = calculate_confidence(&call_locations);

    CallVerificationResult {
        function_name: function_name.to_string(),
        call_locations,
        confidence,
    }
}

struct CallVisitor {
    target_function: String,
    file_path: PathBuf,
    locations: Vec<CallLocation>,
}

impl CallVisitor {
    fn find_calls(&self, file: &syn::File) -> Vec<CallLocation> {
        // Walk the AST to find ExprCall nodes
        // Match against target function name
        // Handle qualified names (module::function)
        // Return locations where function is called
        todo!()
    }
}

fn calculate_confidence(call_locations: &[CallLocation]) -> f64 {
    if call_locations.is_empty() {
        // AST confirms no calls - high confidence dead code
        1.0
    } else if call_locations.iter().all(|loc| is_in_test_or_comment(loc)) {
        // Only found in tests/comments - medium confidence
        0.7
    } else {
        // Found real calls - this is NOT dead code
        0.0
    }
}

fn is_in_test_or_comment(location: &CallLocation) -> bool {
    location.file_path.to_str().map_or(false, |s| s.contains("test"))
        || location.context.trim_start().starts_with("//")
}
```

**Phase 2: Multi-Pass Call Graph Construction**

Modify `src/builders/call_graph.rs`:

```rust
pub struct CallGraphBuilder {
    // Pass 1: All function definitions
    function_registry: HashMap<FunctionId, FunctionMetadata>,

    // Pass 2: Intra-file calls
    local_calls: HashMap<FunctionId, Vec<FunctionId>>,

    // Pass 3: Cross-file calls
    cross_file_calls: HashMap<FunctionId, Vec<FunctionId>>,

    // Pass 4: AST verification results
    verification_results: HashMap<FunctionId, CallVerificationResult>,

    // Track which files have been analyzed
    analyzed_files: HashSet<PathBuf>,
    total_files: usize,
}

impl CallGraphBuilder {
    pub fn build_with_verification(
        project_path: &Path,
        config: &Config,
    ) -> Result<CallGraph> {
        let mut builder = Self::new();

        // Pass 1: Collect all function definitions
        log::info!("Pass 1: Collecting function definitions");
        builder.collect_function_definitions(project_path)?;

        // Pass 2: Resolve intra-file calls
        log::info!("Pass 2: Resolving local calls");
        builder.resolve_local_calls(project_path)?;

        // Pass 3: Resolve cross-file calls
        log::info!("Pass 3: Resolving cross-file calls");
        builder.resolve_cross_file_calls()?;

        // Pass 4: Verify "no callers" functions with AST
        log::info!("Pass 4: AST verification of zero-caller functions");
        builder.verify_zero_caller_functions(project_path)?;

        Ok(builder.into_call_graph())
    }

    fn verify_zero_caller_functions(&mut self, project_path: &Path) -> Result<()> {
        let zero_caller_funcs: Vec<_> = self.function_registry
            .iter()
            .filter(|(id, _)| self.get_callers(id).is_empty())
            .map(|(id, meta)| (id.clone(), meta.clone()))
            .collect();

        log::info!("Verifying {} functions with no detected callers",
                   zero_caller_funcs.len());

        for (func_id, metadata) in zero_caller_funcs {
            let verification = verify_function_calls_ast(
                &metadata.name,
                &self.get_all_source_files(project_path),
                metadata.language,
            );

            if !verification.call_locations.is_empty() {
                // AST found calls that call graph missed!
                log::warn!(
                    "Call graph blindspot: {} has {} AST-detected calls but 0 graph calls",
                    metadata.name,
                    verification.call_locations.len()
                );

                // Add these calls to the graph
                for location in &verification.call_locations {
                    if let Some(caller_id) = self.find_function_at_location(&location) {
                        self.add_call(caller_id, func_id.clone());
                    }
                }
            }

            self.verification_results.insert(func_id, verification);
        }

        Ok(())
    }
}
```

**Phase 3: Confidence-Based Dead Code Detection**

Modify `src/priority/scoring/classification.rs`:

```rust
pub fn classify_dead_code_with_confidence(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    verification: &Option<CallVerificationResult>,
) -> Option<DebtClassification> {
    let callers = call_graph.get_callers(&func.id);

    // If call graph shows callers, definitely not dead
    if !callers.is_empty() {
        return None;
    }

    // Check verification confidence
    let confidence = verification
        .as_ref()
        .map(|v| v.confidence)
        .unwrap_or(0.5); // Default to medium if no verification

    // Only flag as dead code if high confidence
    if confidence < 0.7 {
        log::debug!(
            "Skipping dead code classification for {} (low confidence: {:.2})",
            func.name,
            confidence
        );
        return None;
    }

    Some(DebtClassification {
        debt_type: DebtType::DeadCode,
        confidence,
        metadata: DeadCodeMetadata {
            verified_by_ast: verification.is_some(),
            call_locations_checked: verification
                .as_ref()
                .map(|v| v.call_locations.len())
                .unwrap_or(0),
        },
    })
}
```

**Phase 4: Enhanced Output Format**

Update `src/priority/formatter.rs`:

```rust
// Show confidence for dead code findings
if matches!(func.debt_type, DebtType::DeadCode { .. }) {
    let confidence_pct = (func.confidence * 100.0) as u32;
    let confidence_indicator = if confidence_pct >= 90 {
        "🔴🔴 VERY HIGH CONFIDENCE"
    } else if confidence_pct >= 70 {
        "🔴 HIGH CONFIDENCE"
    } else if confidence_pct >= 50 {
        "🟠 MEDIUM CONFIDENCE"
    } else {
        "🟡 LOW CONFIDENCE - verify manually"
    };

    writeln!(
        output,
        "│   ⚠ No callers detected (confidence: {}%) {}",
        confidence_pct,
        confidence_indicator
    )?;

    if let Some(verification) = &func.verification_result {
        if verification.call_locations.is_empty() {
            writeln!(output, "│   ✓ AST verification confirms no calls found")?;
        } else {
            writeln!(
                output,
                "│   ℹ AST found {} potential calls (in tests/comments)",
                verification.call_locations.len()
            )?;
        }
    }
}
```

### Architecture Changes

**New Modules**:
- `src/verification/` - AST-based verification system
  - `call_verification.rs` - Main verification logic
  - `rust_visitor.rs` - Rust AST call detection
  - `python_visitor.rs` - Python AST call detection
  - `js_visitor.rs` - JavaScript/TypeScript call detection

**Modified Modules**:
- `src/builders/call_graph.rs` - Multi-pass construction with verification
- `src/priority/scoring/classification.rs` - Confidence-based dead code detection
- `src/priority/formatter.rs` - Display confidence levels

### Data Structures

```rust
/// Result of AST-based call verification
pub struct CallVerificationResult {
    pub function_name: String,
    pub call_locations: Vec<CallLocation>,
    pub confidence: f64,
    pub analysis_complete: bool, // Were all files analyzed?
    pub blindspots: Vec<String>,  // Potential analysis gaps
}

/// Location where a function is called
pub struct CallLocation {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub column: usize,
    pub context: String, // Surrounding code
    pub in_test: bool,
    pub in_comment: bool,
}

/// Enhanced dead code metadata
pub struct DeadCodeMetadata {
    pub verified_by_ast: bool,
    pub call_locations_checked: usize,
    pub verification_confidence: f64,
    pub blindspots: Vec<String>,
}

/// Call graph construction state
pub enum AnalysisPass {
    CollectingDefinitions,  // Pass 1
    ResolvingLocalCalls,    // Pass 2
    ResolvingCrossFile,     // Pass 3
    VerifyingDeadCode,      // Pass 4
    Complete,
}
```

### APIs and Interfaces

**New Public API**:

```rust
// Main verification entry point
pub fn verify_function_calls_ast(
    function_name: &str,
    search_paths: &[PathBuf],
    language: Language,
) -> CallVerificationResult;

// Language-specific verifiers
pub fn verify_rust_calls(
    function_name: &str,
    search_paths: &[PathBuf],
) -> CallVerificationResult;

pub fn verify_python_calls(
    function_name: &str,
    search_paths: &[PathBuf],
) -> CallVerificationResult;

pub fn verify_js_calls(
    function_name: &str,
    search_paths: &[PathBuf],
) -> CallVerificationResult;

// Multi-pass call graph builder
pub fn build_call_graph_with_verification(
    project_path: &Path,
    config: &Config,
) -> Result<CallGraph>;
```

## Dependencies

**Prerequisites**: None

**Affected Components**:
- `src/builders/call_graph.rs` - Call graph construction
- `src/priority/scoring/classification.rs` - Dead code detection
- `src/priority/formatter.rs` - Output display

**External Dependencies**:
- `syn` (already used) - Rust AST parsing
- `rustpython-parser` or `tree-sitter-python` - Python AST parsing
- `swc_ecma_parser` or `tree-sitter-javascript` - JS/TS AST parsing

## Testing Strategy

### Unit Tests

**AST Call Verification** (`tests/verification/call_verification_tests.rs`):

```rust
#[test]
fn test_detect_rust_function_call() {
    let code = r#"
        fn caller() {
            process_rust_files_for_call_graph(&path, &mut graph, false, false);
        }
    "#;

    let result = verify_rust_calls("process_rust_files_for_call_graph", &[test_file]);
    assert_eq!(result.call_locations.len(), 1);
    assert_eq!(result.confidence, 0.0); // Not dead code
}

#[test]
fn test_qualified_call_detection() {
    let code = r#"
        fn caller() {
            call_graph::process_rust_files_for_call_graph(...);
        }
    "#;

    let result = verify_rust_calls("process_rust_files_for_call_graph", &[test_file]);
    assert_eq!(result.call_locations.len(), 1);
}

#[test]
fn test_no_false_positive_on_definition() {
    let code = r#"
        pub fn process_rust_files_for_call_graph(...) {
            // Function definition, not a call
        }
    "#;

    let result = verify_rust_calls("process_rust_files_for_call_graph", &[test_file]);
    assert_eq!(result.call_locations.len(), 0);
}

#[test]
fn test_call_in_comment_ignored() {
    let code = r#"
        // process_rust_files_for_call_graph() is used elsewhere
        fn foo() {}
    "#;

    let result = verify_rust_calls("process_rust_files_for_call_graph", &[test_file]);
    assert!(result.call_locations.is_empty() || result.call_locations[0].in_comment);
}
```

**Multi-Pass Call Graph** (`tests/call_graph/multi_pass_tests.rs`):

```rust
#[test]
fn test_self_referential_calls_detected() {
    // Test on debtmap's own src/builders/call_graph.rs
    let call_graph = build_call_graph_with_verification(
        Path::new("."),
        &Config::default(),
    ).unwrap();

    let func_id = FunctionId::new("process_rust_files_for_call_graph");
    let callers = call_graph.get_callers(&func_id);

    assert!(
        callers.len() >= 3,
        "Expected >= 3 callers, found {}",
        callers.len()
    );
}

#[test]
fn test_confidence_scoring() {
    // Create test scenario with actual dead code
    let test_project = setup_test_project_with_dead_code();

    let call_graph = build_call_graph_with_verification(
        &test_project.path,
        &Config::default(),
    ).unwrap();

    let dead_func = call_graph.get_function("truly_unused_function");
    let verification = call_graph.get_verification(&dead_func.id);

    assert_eq!(verification.confidence, 1.0); // High confidence
    assert!(verification.call_locations.is_empty());
}
```

### Integration Tests

**Debtmap Self-Analysis** (`tests/integration/self_analysis_test.rs`):

```rust
#[test]
fn test_no_false_dead_code_on_self() {
    // Run debtmap on its own codebase
    let results = analyze_project(
        Path::new("."),
        AnalysisConfig {
            detect_dead_code: true,
            ..Default::default()
        },
    ).unwrap();

    // Check for false positives on known infrastructure functions
    let false_positives = results.debt_items.iter()
        .filter(|item| matches!(item.debt_type, DebtType::DeadCode { .. }))
        .filter(|item| is_known_infrastructure_function(&item.name))
        .collect::<Vec<_>>();

    assert_eq!(
        false_positives.len(),
        0,
        "Found {} false positive dead code detections: {:?}",
        false_positives.len(),
        false_positives.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
}

fn is_known_infrastructure_function(name: &str) -> bool {
    matches!(
        name,
        "process_rust_files_for_call_graph"
            | "build_call_graph_from_cache"
            | "build_call_graph_without_cache"
            | "perform_unified_analysis_computation"
    )
}
```

**Performance Benchmark** (`benches/verification_overhead.rs`):

```rust
#[bench]
fn bench_ast_verification_overhead(b: &mut Bencher) {
    let project = setup_large_test_project(); // 1000 files

    b.iter(|| {
        // Measure time with verification
        let start = Instant::now();
        let _ = build_call_graph_with_verification(&project.path, &Config::default());
        let with_verification = start.elapsed();

        // Measure time without verification
        let start = Instant::now();
        let _ = build_call_graph_without_verification(&project.path, &Config::default());
        let without_verification = start.elapsed();

        let overhead = with_verification - without_verification;

        // Overhead should be <100ms
        assert!(
            overhead.as_millis() < 100,
            "Verification overhead too high: {}ms",
            overhead.as_millis()
        );
    });
}
```

## Documentation Requirements

### Code Documentation

1. **Module-level docs** for `src/verification/`:
   ```rust
   //! AST-based call verification to eliminate call graph blindspots.
   //!
   //! The call graph builder has a fundamental limitation: it cannot see
   //! calls TO itself during its own construction. This module provides
   //! AST-based verification as a final pass to catch these cases.
   //!
   //! # How It Works
   //!
   //! 1. Build call graph normally (may have blindspots)
   //! 2. Identify functions with zero detected callers
   //! 3. Use AST grep to search entire codebase for calls
   //! 4. Calculate confidence based on AST findings
   //! 5. Only flag as "dead code" if confidence >70%
   ```

2. **Function-level examples**:
   ```rust
   /// Verify if a function is called using AST analysis.
   ///
   /// # Example
   ///
   /// ```
   /// let result = verify_function_calls_ast(
   ///     "process_rust_files_for_call_graph",
   ///     &[Path::new("src/builders")],
   ///     Language::Rust,
   /// );
   ///
   /// if result.call_locations.is_empty() {
   ///     println!("High confidence dead code ({}%)", result.confidence * 100.0);
   /// } else {
   ///     println!("Found {} calls, NOT dead code", result.call_locations.len());
   /// }
   /// ```
   pub fn verify_function_calls_ast(...) -> CallVerificationResult;
   ```

### User Documentation

Add to README.md or user guide:

```markdown
## Dead Code Detection Accuracy

Debtmap uses a multi-pass approach to ensure accurate dead code detection:

1. **Call Graph Analysis**: Build function call graph across all files
2. **Cross-File Resolution**: Resolve calls between modules
3. **AST Verification**: For functions with no detected callers, perform AST grep verification
4. **Confidence Scoring**: Only flag as dead code if confidence >70%

### Understanding Confidence Levels

- **🔴🔴 90-100%**: AST verification confirms no calls - safe to remove
- **🔴 70-89%**: High confidence, but verify before removing
- **🟠 50-69%**: Medium confidence - requires manual investigation
- **🟡 <50%**: Low confidence - likely false positive, skip recommendation

### Self-Analysis Mode

When analyzing debtmap's own codebase, stricter verification is enabled
to catch potential analysis blindspots.
```

### Architecture Documentation

Update ARCHITECTURE.md:

```markdown
## Call Graph Construction

### Multi-Pass Approach

1. **Pass 1: Definition Collection**
   - Scan all files to collect function definitions
   - Build function registry with metadata

2. **Pass 2: Local Call Resolution**
   - Resolve calls within each file
   - Handle method calls, closures, etc.

3. **Pass 3: Cross-File Resolution**
   - Resolve calls between modules
   - Handle imports and qualified names

4. **Pass 4: AST Verification**
   - For functions with no detected callers:
     - Perform AST grep across entire codebase
     - Calculate confidence score
     - Update call graph with findings
   - Only flag as "dead code" if confidence >70%

### Handling Self-Referential Analysis

The call graph builder cannot see calls to itself during construction.
Pass 4 catches these cases through AST verification.
```

## Implementation Notes

### Edge Cases to Handle

1. **Macro-generated calls**:
   ```rust
   macro_rules! call_processor {
       () => { process_rust_files_for_call_graph(...) }
   }
   ```
   - May not be detected by AST visitor
   - Lower confidence if macros present

2. **Trait method calls**:
   ```rust
   trait Builder {
       fn build_call_graph(...);
   }
   // Call through trait, not direct function
   ```
   - Need to track trait implementations

3. **Generic function calls**:
   ```rust
   fn process<T>() { ... }
   process::<i32>();  // Turbofish syntax
   ```
   - Match on base name, ignore generics

4. **FFI/extern calls**:
   - Can't detect calls from other languages
   - Mark as low confidence if `extern` or `#[no_mangle]`

### Performance Optimizations

1. **Early exit on first call found**:
   ```rust
   // Don't need to find ALL calls, just confirm >0
   if call_locations.len() > 0 {
       return CallVerificationResult { confidence: 0.0, ... };
   }
   ```

2. **Parallel file scanning**:
   ```rust
   search_paths.par_iter()
       .flat_map(|path| find_calls_in_file(path, function_name))
       .collect()
   ```

3. **Cache parsed ASTs**:
   ```rust
   static AST_CACHE: Lazy<DashMap<PathBuf, syn::File>> = ...;
   ```

### Testing Gotchas

1. **Test isolation**: Ensure tests don't interfere with each other's call graphs
2. **File ordering**: Results shouldn't depend on file processing order
3. **Incremental analysis**: Test with partial file sets
4. **Performance regression**: Benchmark before/after on large codebases

## Migration and Compatibility

### Breaking Changes

None - this is a pure improvement to accuracy.

### Backward Compatibility

- Existing call graph API unchanged
- New `verification_results` field in CallGraph is optional
- Output format enhancement is backward compatible

### Configuration

Add optional config to disable verification for performance:

```toml
[analysis]
enable_ast_verification = true  # Default: true
verification_confidence_threshold = 0.7  # Only flag if confidence >70%
max_verification_time_ms = 5000  # Skip verification if exceeds timeout
```

### Rollout Plan

1. **v0.3.0**: Implement AST verification, disabled by default
2. **v0.3.1**: Enable by default with warning about performance impact
3. **v0.3.2**: Optimize performance, remove warning
4. **v0.4.0**: Make verification non-optional (core feature)

## Success Metrics

### Quantitative Goals

1. **Accuracy**:
   - ✅ Zero false "no callers" warnings on debtmap's own codebase
   - ✅ False positive rate <5% on sample projects
   - ✅ Correctly detect 3 callers for `process_rust_files_for_call_graph()`

2. **Performance**:
   - ✅ <100ms AST verification overhead per flagged function
   - ✅ <10% total analysis time increase on large codebases
   - ✅ Parallel verification scales linearly with CPU cores

3. **Coverage**:
   - ✅ Support Rust, Python, JavaScript, TypeScript
   - ✅ Handle 95% of common call patterns (methods, functions, qualified)
   - ✅ Gracefully handle edge cases (macros, generics, traits)

### Qualitative Goals

1. **User Trust**:
   - Dead code recommendations are actionable
   - Confidence levels help prioritize verification
   - Fewer false positives = more trust in tool

2. **Developer Experience**:
   - Clear output showing verification status
   - Helpful confidence explanations
   - Warnings for low-confidence findings

## Future Enhancements

### Post-v0.3.0 Improvements

1. **Incremental Verification** (v0.4.0):
   - Cache verification results between runs
   - Only re-verify functions in changed files
   - Track verification cache invalidation

2. **Cross-Language Call Detection** (v0.5.0):
   - Detect Python calling Rust (via PyO3, ctypes)
   - Detect JavaScript calling TypeScript
   - FFI call tracking

3. **Interactive Verification** (v0.6.0):
   - Let users manually verify "dead code" findings
   - Learn from user feedback to improve confidence scoring
   - Track verification accuracy over time

4. **Semantic Analysis** (v0.7.0):
   - Detect calls through function pointers/closures
   - Track indirect calls via trait objects
   - Handle dynamic dispatch in Python/JavaScript
