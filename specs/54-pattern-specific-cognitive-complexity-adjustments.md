---
number: 54
title: Pattern-Specific Cognitive Complexity Adjustments
category: optimization
priority: high
status: draft
dependencies: [27, 43, 52, 53]
created: 2025-08-20
---

# Specification 54: Pattern-Specific Cognitive Complexity Adjustments

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [27, 43, 52, 53]

## Context

Analysis of debtmap's self-analysis reveals that certain code patterns receive disproportionately high cognitive complexity scores despite being mentally simple to understand. The most significant false positives are:

1. **Pattern matching functions** (e.g., `detect_file_type()`) that check multiple conditions on the same variable with immediate returns receive cognitive complexity scores of 29+ despite being simple lookup tables in essence.

2. **Simple delegation functions** that create data structures and pass through to another function are flagged as orchestration despite having cyclomatic complexity of 1.

These false positives reduce developer trust in the tool and create noise in the technical debt reports. The current cognitive complexity calculation treats all conditional logic equally, not recognizing that pattern matching is mentally simpler than nested control flow.

## Objective

Implement pattern-specific cognitive complexity adjustments that accurately reflect the mental effort required to understand different code structures, reducing false positives for pattern matching and simple delegation while maintaining sensitivity to genuine complexity issues.

## Requirements

### Functional Requirements

1. **Pattern Matching Detection**
   - Detect sequences of if/else statements checking the same variable
   - Identify immediate return patterns within conditional blocks
   - Recognize string matching patterns (ends_with, starts_with, contains)
   - Support both Rust match expressions and if/else chains
   - Handle OR conditions within pattern matching

2. **Adjusted Complexity Scoring**
   - Apply logarithmic or square root scaling for pattern matching instead of linear
   - Reduce cognitive complexity from O(n) to O(log n) for n conditions
   - Preserve normal scoring for non-pattern matching conditionals
   - Maintain existing scoring for nested control flow

3. **Simple Delegation Recognition**
   - Identify functions with cyclomatic complexity of 1
   - Detect single function call patterns
   - Recognize data transformation without control flow
   - Exclude from orchestration classification when appropriate

4. **Language Support**
   - Implement for Rust analyzer initially
   - Design pattern-agnostic approach for other languages
   - Support JavaScript/TypeScript switch statements
   - Handle Python if/elif chains

### Non-Functional Requirements

- **Performance**: Pattern detection must add <5% overhead to analysis time
- **Accuracy**: Reduce pattern matching false positives by >70%
- **Safety**: Must not mask genuine complexity issues
- **Maintainability**: Clean separation of pattern detection logic
- **Extensibility**: Easy to add new pattern recognizers

## Acceptance Criteria

- [ ] Pattern matching detection correctly identifies `detect_file_type()` style functions
- [ ] Cognitive complexity for pattern matching reduced from 29 to <5
- [ ] Simple delegation functions no longer flagged as orchestration
- [ ] All existing tests continue to pass
- [ ] New tests validate pattern detection accuracy
- [ ] Performance overhead is <5% on large codebases
- [ ] Documentation updated with pattern recognition explanation
- [ ] No real complexity issues are masked by the adjustments

## Technical Details

### Implementation Approach

1. **AST Pattern Analysis**
   ```rust
   struct PatternMatchInfo {
       variable_name: String,
       condition_count: usize,
       has_default: bool,
       pattern_type: PatternType,
   }
   
   enum PatternType {
       StringMatching,    // ends_with, starts_with patterns
       EnumMatching,      // Matching against enum variants
       RangeMatching,     // Numeric range checks
       TypeChecking,      // instanceof or type checks
   }
   ```

2. **Complexity Adjustment Algorithm**
   ```rust
   fn calculate_pattern_complexity(info: &PatternMatchInfo) -> u32 {
       // Use logarithmic scaling for pattern matching
       let base = (info.condition_count as f32).log2().ceil() as u32;
       
       // Small penalty for missing default case
       if !info.has_default { base + 1 } else { base }
   }
   ```

3. **Integration Points**
   - Modify `CognitiveVisitor` in `src/complexity/cognitive.rs`
   - Enhance pattern detection in `src/complexity/patterns.rs`
   - Update orchestration detection in `src/priority/semantic_classifier.rs`

### Architecture Changes

- Add `PatternRecognizer` trait for extensible pattern detection
- Introduce `ComplexityAdjuster` for pattern-specific scoring
- Enhance `CognitiveVisitor` with pattern awareness
- Update `FunctionRole` classification logic

### Data Structures

```rust
pub trait PatternRecognizer {
    fn detect(&self, block: &Block) -> Option<PatternMatchInfo>;
    fn adjust_complexity(&self, info: &PatternMatchInfo, base: u32) -> u32;
}

struct PatternMatchRecognizer;
struct SimpleDelegationRecognizer;
struct FunctionalChainRecognizer;
```

### APIs and Interfaces

```rust
// Enhanced cognitive complexity calculation
pub fn calculate_cognitive_adjusted(block: &Block) -> u32 {
    let recognizers = vec![
        Box::new(PatternMatchRecognizer::new()),
        Box::new(SimpleDelegationRecognizer::new()),
    ];
    
    for recognizer in recognizers {
        if let Some(info) = recognizer.detect(block) {
            return recognizer.adjust_complexity(&info, base_complexity);
        }
    }
    
    // Fall back to standard calculation
    calculate_cognitive(block)
}
```

## Dependencies

- **Prerequisites**: 
  - Spec 27: Context-Aware Complexity Scoring (provides foundation)
  - Spec 43: Context-Aware False Positive Reduction (complementary approach)
  - Spec 52-53: Entropy-Based Complexity Scoring (integration needed)
  
- **Affected Components**:
  - `src/complexity/cognitive.rs` - Core cognitive complexity calculation
  - `src/complexity/patterns.rs` - Pattern detection logic
  - `src/priority/semantic_classifier.rs` - Function role classification
  - `src/analyzers/rust.rs` - Rust-specific analysis

- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**:
  - Test pattern detection with various code structures
  - Validate complexity calculations for patterns vs non-patterns
  - Test edge cases (single condition, mixed patterns, nested patterns)
  - Verify pattern type classification

- **Integration Tests**:
  - Run on known pattern matching functions
  - Verify complexity scores are appropriately reduced
  - Ensure other functions aren't incorrectly adjusted
  - Test with real codebases (servo, rustc, tokio)

- **Performance Tests**:
  - Measure overhead on large codebases
  - Profile pattern detection performance
  - Validate <5% overhead requirement

- **Regression Tests**:
  - Ensure no real complexity issues are masked
  - Verify existing debt detection still works
  - Check that test coverage integration isn't affected

## Documentation Requirements

- **Code Documentation**:
  - Document pattern recognition algorithms
  - Explain complexity adjustment rationale
  - Provide examples of detected patterns

- **User Documentation**:
  - Update README with pattern recognition feature
  - Add section on cognitive complexity adjustments
  - Include examples of reduced false positives

- **Architecture Updates**:
  - Update ARCHITECTURE.md with pattern recognition flow
  - Document new traits and interfaces
  - Explain integration with existing analyzers

## Implementation Notes

### Pattern Detection Algorithm

```rust
fn detect_pattern_matching(block: &Block) -> Option<PatternMatchInfo> {
    let mut conditions = Vec::new();
    let mut variable_name = None;
    
    for stmt in &block.stmts {
        if let Stmt::Expr(Expr::If(if_expr), _) = stmt {
            // Extract tested variable
            if let Some(var) = extract_tested_variable(&if_expr.cond) {
                if variable_name.is_none() {
                    variable_name = Some(var.clone());
                } else if variable_name.as_ref() != Some(&var) {
                    return None; // Different variables
                }
                
                // Check for immediate return
                if !has_immediate_return(&if_expr.then_branch) {
                    return None; // Not pattern matching
                }
                
                conditions.push(if_expr);
            }
        }
    }
    
    // Require multiple conditions on same variable
    if conditions.len() >= 3 {
        Some(PatternMatchInfo { /* ... */ })
    } else {
        None
    }
}
```

### Expected Impact

- **Before**: `detect_file_type()` - Cognitive: 29, Flagged: Yes
- **After**: `detect_file_type()` - Cognitive: 3, Flagged: No

- **Before**: `classify_function_role()` - Orchestration: Yes
- **After**: `classify_function_role()` - Orchestration: No

### Rollout Strategy

1. **Phase 1**: Implement pattern matching detection for Rust
2. **Phase 2**: Add simple delegation recognition
3. **Phase 3**: Extend to JavaScript/TypeScript
4. **Phase 4**: Add Python support

## Migration and Compatibility

- **Breaking Changes**: None - adjustments are transparent
- **Configuration**: 
  ```toml
  [complexity.adjustments]
  pattern_matching = true  # Enable pattern matching adjustments
  logarithmic_scaling = true  # Use log scaling for patterns
  ```
- **Backwards Compatibility**: Tool continues to work without adjustments if disabled
- **Migration Path**: Gradual rollout with feature flag