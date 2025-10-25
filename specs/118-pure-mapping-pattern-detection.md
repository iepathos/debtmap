---
number: 118
title: Pure Mapping Pattern Detection for Complexity Adjustment
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-10-25
---

# Specification 118: Pure Mapping Pattern Detection for Complexity Adjustment

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently flags functions with high cyclomatic complexity as technical debt, which is appropriate for most cases. However, certain patterns like exhaustive enum matching with simple return values exhibit high cyclomatic complexity by design, not due to poor code quality.

**Current Problem**:
- `format_pattern_type()` in `src/io/pattern_output.rs:67` scores 14.8 (CRITICAL)
- Cyclomatic complexity: 15 (7 pattern types × 2 code paths)
- Cognitive complexity: LOW (~3-5 estimated)
- This is a pure mapping function doing exactly what it should
- No refactoring would improve this code - it's already optimal

**Real-world Example**:
```rust
fn format_pattern_type(&self, pattern_type: &PatternType) -> String {
    let label = match pattern_type {
        PatternType::Observer => "OBSERVER PATTERN",
        PatternType::Singleton => "SINGLETON PATTERN",
        PatternType::Factory => "FACTORY PATTERN",
        PatternType::Strategy => "STRATEGY PATTERN",
        PatternType::Callback => "CALLBACK PATTERN",
        PatternType::TemplateMethod => "TEMPLATE METHOD",
        PatternType::DependencyInjection => "DEPENDENCY INJECTION",
    };

    if self.plain {
        label.to_string()
    } else {
        match pattern_type {
            PatternType::Observer => label.green().to_string(),
            PatternType::Singleton => label.blue().to_string(),
            PatternType::Factory => label.yellow().to_string(),
            PatternType::Strategy => label.cyan().to_string(),
            PatternType::Callback => label.magenta().to_string(),
            PatternType::TemplateMethod => label.bright_blue().to_string(),
            PatternType::DependencyInjection => label.bright_green().to_string(),
        }
    }
}
```

**Why This Matters**:
- False positives reduce trust in debtmap recommendations
- Developers waste time investigating non-issues
- ~10-15% of flagged items may be pure mapping functions
- Cognitive complexity is already calculated but underutilized

## Objective

Implement pattern detection to identify pure mapping functions and adjust their complexity scoring to prevent false positives, while maintaining detection of genuinely complex code.

## Requirements

### Functional Requirements

1. **Pattern Detection**
   - Detect exhaustive enum `match` expressions with simple return values
   - Identify functions consisting primarily of mapping logic (>80% of function body)
   - Recognize common mapping patterns: enum → string, enum → number, enum → enum
   - Support nested match expressions (e.g., outer match + inner match for styling)
   - Detect similar patterns in other languages (switch statements in JS/TS, if-elif chains in Python)

2. **Complexity Adjustment**
   - When pure mapping pattern detected, calculate adjustment factor
   - Reduce cyclomatic complexity weight in scoring calculation
   - Increase cognitive complexity weight for mapping functions
   - Apply dampening factor (0.3-0.5×) to cyclomatic complexity score
   - Preserve original metrics in output for transparency

3. **Language-Specific Detection**
   - **Rust**: `match` expressions on enums with simple arms
   - **TypeScript/JavaScript**: `switch` statements with simple cases, object literal mappings
   - **Python**: `if-elif-else` chains on enum/literal values, dict-based mappings

### Non-Functional Requirements

- Detection must complete in <10ms per function (avoid performance regression)
- False negative rate <5% (must not miss genuine complexity issues)
- False positive rate <2% (must not incorrectly identify complex code as mapping)
- Transparent: Show when adjustment was applied in debug output
- Configurable: Allow users to disable this feature via config

## Acceptance Criteria

- [ ] `format_pattern_type()` example no longer flagged as CRITICAL (score drops from 14.8 to <10)
- [ ] Exhaustive enum matches with simple returns are detected in Rust code
- [ ] Switch statements with simple cases are detected in JS/TS code
- [ ] If-elif chains with simple returns are detected in Python code
- [ ] Functions with genuine complexity (nested logic, calculations) are NOT adjusted
- [ ] Adjustment factor is shown in output: "complexity: cyclomatic=15 (adj:7 via mapping pattern)"
- [ ] Configuration option `complexity.pure_mapping_detection` enables/disables feature
- [ ] Performance impact <5% on typical codebase analysis
- [ ] All existing tests continue to pass
- [ ] New property-based tests verify detection accuracy on generated code samples

## Technical Details

### Implementation Approach

**Phase 1: AST Pattern Analysis**

Create new module: `src/complexity/mapping_patterns.rs`

```rust
pub struct MappingPatternDetector {
    config: MappingPatternConfig,
}

pub struct MappingPatternResult {
    pub is_pure_mapping: bool,
    pub confidence: f64,
    pub mapping_ratio: f64, // % of function that's mapping logic
    pub complexity_adjustment_factor: f64,
}

impl MappingPatternDetector {
    pub fn analyze_function(&self, function_ast: &FunctionNode) -> MappingPatternResult {
        // 1. Check if function body is primarily a match/switch/if-elif chain
        // 2. Verify all arms/cases are simple (no nested logic)
        // 3. Calculate what % of function is mapping vs other logic
        // 4. Return adjustment factor based on mapping_ratio
    }
}
```

**Phase 2: Integration with Complexity Scoring**

Modify `src/complexity/mod.rs`:
```rust
pub fn calculate_complexity_score(
    function: &FunctionMetrics,
    mapping_result: &MappingPatternResult,
) -> f64 {
    let base_cyclomatic = function.cyclomatic_complexity;

    let adjusted_cyclomatic = if mapping_result.is_pure_mapping {
        base_cyclomatic as f64 * mapping_result.complexity_adjustment_factor
    } else {
        base_cyclomatic as f64
    };

    let cognitive_weight = if mapping_result.is_pure_mapping { 0.7 } else { 0.5 };

    // Combine adjusted cyclomatic with cognitive complexity
    adjusted_cyclomatic * (1.0 - cognitive_weight)
        + function.cognitive_complexity as f64 * cognitive_weight
}
```

**Phase 3: Pattern Recognition Rules**

**Rust Patterns**:
```rust
// Pure mapping: All arms are literals or simple expressions
match enum_value {
    Variant1 => simple_expr,
    Variant2 => simple_expr,
    // ... exhaustive
}

// Simple expression criteria:
// - String/numeric literals
// - Function calls with ≤2 arguments, no nested calls
// - Method calls on literals (e.g., "string".to_string())
// - NOT: if/match/loop inside arm
```

**JavaScript/TypeScript Patterns**:
```javascript
// Switch-based mapping
switch (value) {
    case A: return simple_expr;
    case B: return simple_expr;
    // ... with return in each case
}

// Object literal mapping
const mapping = {
    A: value1,
    B: value2,
};
return mapping[key];
```

**Python Patterns**:
```python
# If-elif chain mapping
if value == A:
    return simple_expr
elif value == B:
    return simple_expr
# ... with consistent pattern

# Dict-based mapping
mapping = {
    A: value1,
    B: value2,
}
return mapping.get(key)
```

### Architecture Changes

**New Module**: `src/complexity/mapping_patterns.rs`
- `MappingPatternDetector` - Main detection logic
- `MappingPatternResult` - Detection result struct
- `is_simple_expression()` - Validate arm/case simplicity
- `calculate_mapping_ratio()` - Determine % of function that's mapping

**Modified Modules**:
- `src/complexity/mod.rs` - Integrate mapping detection into scoring
- `src/priority/scoring/mod.rs` - Apply adjustment factors
- `src/config.rs` - Add configuration options

### Data Structures

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingPatternConfig {
    /// Enable pure mapping pattern detection
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Minimum mapping ratio to qualify (0.0-1.0)
    #[serde(default = "default_min_mapping_ratio")]
    pub min_mapping_ratio: f64,

    /// Complexity adjustment factor for pure mappings (0.0-1.0)
    #[serde(default = "default_adjustment_factor")]
    pub adjustment_factor: f64,

    /// Maximum expression complexity in arms/cases
    #[serde(default = "default_max_arm_complexity")]
    pub max_arm_complexity: u32,
}

impl Default for MappingPatternConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_mapping_ratio: 0.8,
            adjustment_factor: 0.4,
            max_arm_complexity: 2,
        }
    }
}
```

### APIs and Interfaces

**Public API**:
```rust
// Add to FunctionMetrics
pub struct FunctionMetrics {
    // ... existing fields
    pub mapping_pattern_result: Option<MappingPatternResult>,
    pub adjusted_complexity: Option<f64>,
}

// Configuration
impl DebtmapConfig {
    pub fn mapping_pattern_config(&self) -> &MappingPatternConfig;
}
```

**Internal API**:
```rust
trait SimpleExpressionChecker {
    fn is_simple(&self, expr: &AstNode) -> bool;
    fn complexity(&self, expr: &AstNode) -> u32;
}

trait MappingPatternChecker {
    fn check_match_pattern(&self, match_expr: &MatchExpr) -> Option<MappingPatternResult>;
    fn check_switch_pattern(&self, switch_stmt: &SwitchStmt) -> Option<MappingPatternResult>;
    fn check_ifelse_chain(&self, if_stmt: &IfStmt) -> Option<MappingPatternResult>;
}
```

## Dependencies

- **Prerequisites**: None (uses existing AST infrastructure)
- **Affected Components**:
  - `src/complexity/mod.rs` - Complexity calculation
  - `src/priority/scoring/mod.rs` - Scoring system
  - `src/io/formatter.rs` - Output formatting to show adjustments
- **External Dependencies**: None (uses existing tree-sitter parsers)

## Testing Strategy

### Unit Tests

**Pattern Detection Tests**:
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn detects_rust_enum_match_mapping() {
        let code = r#"
            fn format(val: MyEnum) -> &'static str {
                match val {
                    MyEnum::A => "a",
                    MyEnum::B => "b",
                    MyEnum::C => "c",
                }
            }
        "#;
        let result = detect_mapping_pattern(parse_rust(code));
        assert!(result.is_pure_mapping);
        assert!(result.mapping_ratio > 0.95);
    }

    #[test]
    fn rejects_complex_match_arms() {
        let code = r#"
            fn process(val: MyEnum) -> Result<String> {
                match val {
                    MyEnum::A => {
                        if condition {
                            Ok("a".to_string())
                        } else {
                            Err("error")
                        }
                    },
                    MyEnum::B => Ok("b".to_string()),
                }
            }
        "#;
        let result = detect_mapping_pattern(parse_rust(code));
        assert!(!result.is_pure_mapping);
    }
}
```

**Adjustment Calculation Tests**:
```rust
#[test]
fn applies_adjustment_factor_correctly() {
    let function = FunctionMetrics {
        cyclomatic_complexity: 15,
        cognitive_complexity: 3,
        ..Default::default()
    };
    let mapping_result = MappingPatternResult {
        is_pure_mapping: true,
        adjustment_factor: 0.4,
        ..Default::default()
    };

    let adjusted = calculate_adjusted_complexity(&function, &mapping_result);
    assert_eq!(adjusted, 15.0 * 0.4); // ~6
}
```

### Integration Tests

```rust
#[test]
fn format_pattern_type_no_longer_critical() {
    let config = DebtmapConfig::default();
    let analysis = analyze_file("src/io/pattern_output.rs", &config);

    let format_fn = analysis.find_function("format_pattern_type").unwrap();
    assert!(format_fn.score < 10.0, "Should not be flagged as CRITICAL");
    assert!(format_fn.mapping_pattern_result.is_some());
    assert!(format_fn.mapping_pattern_result.unwrap().is_pure_mapping);
}
```

### Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn simple_enum_matches_always_detected(
        num_variants in 3usize..20,
    ) {
        let code = generate_simple_enum_match(num_variants);
        let result = detect_mapping_pattern(parse_rust(&code));
        prop_assert!(result.is_pure_mapping);
    }

    #[test]
    fn adjustment_factor_always_reduces_score(
        cyclomatic in 5u32..50,
    ) {
        let original_score = calculate_score_without_adjustment(cyclomatic);
        let adjusted_score = calculate_score_with_adjustment(cyclomatic);
        prop_assert!(adjusted_score < original_score);
    }
}
```

### Performance Tests

```rust
#[test]
fn detection_completes_within_10ms() {
    let large_match = generate_enum_match_with_n_arms(100);

    let start = Instant::now();
    let _ = detect_mapping_pattern(parse_rust(&large_match));
    let elapsed = start.elapsed();

    assert!(elapsed < Duration::from_millis(10));
}
```

## Documentation Requirements

### Code Documentation

- Document `MappingPatternDetector` with examples of patterns it detects
- Explain adjustment factor calculation in inline comments
- Add rustdoc examples showing before/after scoring

### User Documentation

**README.md update**:
```markdown
## Complexity Scoring

Debtmap intelligently adjusts complexity scores for common patterns:

- **Pure Mapping Functions**: Exhaustive enum matches with simple return values
  receive reduced cyclomatic complexity weighting, as their high branch count
  is by design, not a code smell.

Example: A function with 15 match arms mapping enum variants to strings will
show `cyclomatic=15 (adj:6 via mapping pattern)` instead of being flagged as
overly complex.

Configure via `.debtmap.toml`:
```toml
[complexity.pure_mapping_detection]
enabled = true
min_mapping_ratio = 0.8
adjustment_factor = 0.4
```
```

### Architecture Updates

Add to `ARCHITECTURE.md`:
```markdown
## Complexity Analysis

### Pattern-Aware Complexity Scoring

The complexity scoring system includes pattern detection to avoid false positives:

1. **Mapping Pattern Detection** (`src/complexity/mapping_patterns.rs`)
   - Identifies pure mapping functions (enum → value)
   - Applies adjustment factor to cyclomatic complexity
   - Increases cognitive complexity weight for these functions

2. **Adjustment Factor Calculation**
   - Based on mapping_ratio (% of function that's mapping logic)
   - Default adjustment: 0.4× cyclomatic complexity
   - Configurable per project needs
```

## Implementation Notes

### Key Design Decisions

1. **Why adjust cyclomatic but not cognitive complexity?**
   - Cyclomatic counts branches (high for mappings)
   - Cognitive measures understanding difficulty (low for mappings)
   - Adjusting cyclomatic while emphasizing cognitive gives better signal

2. **Why 80% mapping ratio threshold?**
   - Functions that are mostly mapping (>80%) are fundamentally different
   - Mixed logic + mapping should still be flagged (20% impurity allowed)
   - Configurable if too strict/loose

3. **Simple expression criteria**
   - Literals: Always simple
   - Single function call: Simple if ≤2 args
   - Method chains: Simple if ≤2 methods
   - Control flow: Never simple (if/match/loop in arm = complex)

### Edge Cases

**Nested Match with Simple Arms**:
```rust
match outer {
    A => match inner { X => 1, Y => 2 },
    B => match inner { X => 3, Y => 4 },
}
```
- Still pure mapping if all inner arms are simple
- Calculate combined complexity of outer × inner

**Partial Mapping**:
```rust
match val {
    A => "a",
    B => "b",
    C => complex_calculation(x, y, z),  // Complex arm
}
```
- NOT pure mapping due to complex arm
- Should still be flagged normally

**Match with Guards**:
```rust
match val {
    A if condition => "a",  // Guard = not simple
    B => "b",
}
```
- Guards add complexity, not pure mapping
- Flag normally

### Performance Considerations

- Pattern detection runs during complexity calculation (already traversing AST)
- Cache detection results in `FunctionMetrics`
- Skip detection if cyclomatic_complexity < 10 (not worth checking)
- Early exit on first complex arm detected

## Migration and Compatibility

### Breaking Changes
None - this is a pure enhancement to scoring.

### Configuration Migration
Add new config section with sensible defaults:
```toml
[complexity.pure_mapping_detection]
enabled = true
min_mapping_ratio = 0.8
adjustment_factor = 0.4
max_arm_complexity = 2
```

### Output Format Changes
Add adjustment notation to complexity display:
- Before: `complexity: cyclomatic=15, cognitive=3`
- After: `complexity: cyclomatic=15 (adj:6 via mapping pattern), cognitive=3`

### Backward Compatibility
- Adjustment is opt-in via config (enabled by default)
- Existing scoring tests may need baseline updates
- Document expected score changes in release notes

## Success Metrics

- Reduction in false positive rate by 10-15%
- `format_pattern_type()` and similar functions score <10 (not CRITICAL)
- User feedback: Fewer "this is fine" dismissals in debtmap output
- Maintain <5% false negative rate on genuine complexity issues

## Future Enhancements

- **Builder pattern detection**: Recognize fluent interfaces (method chaining)
- **Visitor pattern detection**: Exhaustive type dispatching
- **Table-driven code**: Detect data structure lookups vs logic
- **ML-based detection**: Learn what humans consider "complex" vs "boilerplate"
