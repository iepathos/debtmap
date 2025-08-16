---
number: 34
title: Error Swallowing Debt Detection
category: foundation
priority: medium
status: draft
dependencies: []
created: 2025-01-16
---

# Specification 34: Error Swallowing Debt Detection

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

Error swallowing through patterns like `if let Ok(...)` without proper error handling is a common anti-pattern in Rust that violates functional programming principles. This pattern hides failures, makes debugging difficult, and can lead to incorrect program behavior when errors are silently ignored.

Currently, debtmap detects various forms of technical debt including TODOs, FIXMEs, code smells, and complexity issues, but it does not identify error swallowing patterns. Adding this capability would help teams identify and address poor error handling practices systematically, improving code reliability and maintainability.

## Objective

Extend debtmap's technical debt detection capabilities to identify error swallowing anti-patterns in Rust code, categorizing them as a specific type of technical debt with appropriate priority levels based on context and impact.

## Requirements

### Functional Requirements

1. **Pattern Detection**
   - Detect `if let Ok(...)` patterns without error handling in else branch
   - Detect `if let Err(_)` patterns that don't propagate or log errors
   - Detect `.ok()` usage that discards error information without handling
   - Detect `let _ = ` assignments of Result types
   - Detect `.unwrap_or()` and `.unwrap_or_default()` without error logging
   - Detect `match` expressions on Results that ignore error variants

2. **Context Analysis**
   - Distinguish between acceptable and problematic error swallowing
   - Consider whether errors are logged before being discarded
   - Identify if fallback values are appropriate for the context
   - Detect if errors occur in critical vs non-critical code paths

3. **Priority Classification**
   - **Critical**: Error swallowing in main control flow or data processing
   - **High**: Error swallowing in configuration loading or I/O operations
   - **Medium**: Error swallowing without any logging or context
   - **Low**: Error swallowing in test code or with reasonable defaults

4. **Integration**
   - Add new `DebtType::ErrorSwallowing` variant
   - Support suppression via existing suppression comment system
   - Include in existing debt scoring and reporting mechanisms
   - Provide actionable remediation suggestions

### Non-Functional Requirements

1. **Performance**: Pattern detection should not significantly impact analysis speed
2. **Accuracy**: Minimize false positives by understanding context
3. **Compatibility**: Work with existing debtmap infrastructure
4. **Configurability**: Allow users to configure detection sensitivity

## Acceptance Criteria

- [ ] New `DebtType::ErrorSwallowing` variant added to debt type enum
- [ ] Pattern detection correctly identifies all common error swallowing patterns
- [ ] Context analysis reduces false positives for acceptable patterns
- [ ] Priority levels accurately reflect impact and criticality
- [ ] Suppression comments work for error swallowing detection
- [ ] Error swallowing items appear in all output formats (JSON, Markdown, Terminal)
- [ ] Configuration options allow adjusting detection sensitivity
- [ ] Unit tests cover all detection patterns and edge cases
- [ ] Integration tests verify end-to-end functionality
- [ ] Documentation explains patterns detected and remediation approaches

## Technical Details

### Implementation Approach

1. **AST-Based Pattern Matching**
   - Leverage existing syn-based AST parsing infrastructure
   - Create visitor pattern implementation for error swallowing detection
   - Analyze expression patterns to identify error swallowing

2. **Pattern Recognition Rules**
   ```rust
   // Patterns to detect:
   
   // 1. if let Ok without else
   if let Ok(value) = result { /* ... */ }
   
   // 2. if let Ok with empty else
   if let Ok(value) = result { /* ... */ } else { }
   
   // 3. Discarding Results with let _
   let _ = function_returning_result();
   
   // 4. Using .ok() to discard errors
   result.ok().map(|v| /* ... */);
   
   // 5. Match with ignored Err
   match result {
       Ok(v) => /* ... */,
       Err(_) => {},  // or Err(_) => ()
   }
   
   // 6. Unwrap_or without logging
   result.unwrap_or(default);
   ```

3. **Context Evaluation**
   - Check for logging/tracing calls near error swallowing
   - Identify if in test function (lower priority)
   - Detect if in initialization vs runtime code
   - Consider function visibility (public vs private)

### Architecture Changes

1. **New Module**: `src/debt/error_swallowing.rs`
   - Contains all error swallowing detection logic
   - Implements visitor pattern for AST traversal
   - Provides context analysis functions

2. **Core Type Updates**
   - Add `ErrorSwallowing` to `DebtType` enum
   - Update display implementation for new debt type
   - Add to debt scoring weight calculation

3. **Integration Points**
   - Hook into existing Rust analyzer pipeline
   - Use existing suppression infrastructure
   - Leverage existing output formatting

### Data Structures

```rust
// In src/core/mod.rs
pub enum DebtType {
    // ... existing variants ...
    ErrorSwallowing,
}

// In src/debt/error_swallowing.rs
pub struct ErrorSwallowingDetector {
    items: Vec<DebtItem>,
    current_file: PathBuf,
    suppression: Option<SuppressionContext>,
}

pub enum ErrorSwallowingPattern {
    IfLetOkNoElse,
    IfLetOkEmptyElse,
    LetUnderscoreResult,
    OkMethodDiscard,
    MatchIgnoredErr,
    UnwrapOrNoLog,
}
```

### APIs and Interfaces

```rust
// Public API
pub fn detect_error_swallowing(
    file: &syn::File,
    file_path: &Path,
    suppression: Option<&SuppressionContext>,
) -> Vec<DebtItem>;

// Configuration
pub struct ErrorSwallowingConfig {
    pub enabled: bool,
    pub include_tests: bool,
    pub require_logging: bool,
    pub severity_overrides: HashMap<ErrorSwallowingPattern, Priority>,
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/core/mod.rs` - Add new debt type
  - `src/debt/mod.rs` - Include new detection module
  - `src/analyzers/rust.rs` - Integrate error swallowing detection
  - `src/debt/suppression.rs` - Ensure suppression works with new type
  - `src/io/output.rs` - Format error swallowing items in output
- **External Dependencies**: None (uses existing syn crate)

## Testing Strategy

- **Unit Tests**:
  - Test each error swallowing pattern detection
  - Test priority classification logic
  - Test context evaluation
  - Test suppression integration
  
- **Integration Tests**:
  - Create test files with various error swallowing patterns
  - Verify detection in full analysis pipeline
  - Test all output formats include error swallowing items
  - Test configuration options

- **Performance Tests**:
  - Measure impact on analysis time for large codebases
  - Ensure detection scales linearly with file size

- **User Acceptance**:
  - Run on real codebases to validate detection accuracy
  - Gather feedback on false positive rate
  - Validate priority assignments match user expectations

## Documentation Requirements

- **Code Documentation**:
  - Document all error swallowing patterns detected
  - Explain context evaluation logic
  - Provide examples of each pattern

- **User Documentation**:
  - Add error swallowing section to README
  - Explain configuration options
  - Provide remediation guidance for each pattern
  - Show suppression comment examples

- **Architecture Updates**:
  - Update ARCHITECTURE.md with error swallowing detection module
  - Document integration with existing debt detection

## Implementation Notes

### Detection Algorithm

1. **First Pass**: Identify potential error swallowing patterns
2. **Context Analysis**: Evaluate surrounding code for mitigating factors
3. **Priority Assignment**: Assign priority based on pattern and context
4. **Suppression Check**: Filter out suppressed items
5. **Deduplication**: Remove duplicate detections from same location

### False Positive Mitigation

- Don't flag if error is logged before swallowing
- Lower priority for test functions
- Recognize common acceptable patterns (e.g., optional operations)
- Allow configuration of detection sensitivity

### Remediation Suggestions

For each pattern, provide specific fix suggestions:
- `if let Ok` → Use `?` operator or handle error case
- `let _` → Add error handling or logging
- `.ok()` → Use `map_err` to log before converting
- `unwrap_or` → Use `unwrap_or_else` with logging

## Migration and Compatibility

- Fully backwards compatible - new debt type is additive
- Existing configurations continue to work unchanged
- Can be disabled via configuration if needed
- Default weights ensure error swallowing doesn't dominate debt scores
- Suppression comments immediately work with new detection