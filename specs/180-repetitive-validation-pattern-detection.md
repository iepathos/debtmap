---
number: 180
title: Repetitive Validation Pattern Detection
category: foundation
priority: medium
status: draft
dependencies: [179]
created: 2025-11-16
---

# Specification 180: Repetitive Validation Pattern Detection

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: [179] (State Machine/Coordinator Pattern Detection)

## Context

Debtmap currently treats functions with many early returns as high-complexity hotspots, even when those returns follow a simple, repetitive pattern. This leads to misleading complexity assessments and generic recommendations that don't address the real issue.

Example from `validation.rs:30` (the `validate_config` function):
- Cyclomatic complexity: 20 (very high)
- Cognitive complexity: Low (each branch is identical)
- Token entropy: < 0.35 (highly repetitive structure)
- Pattern: 20 consecutive early returns with identical structure: `if field.is_none() { return Err(...) }`
- Current classification: `HighBranching` (misleading)
- Current recommendation: "Extract functions, use lookup tables" (not helpful)
- Desired recommendation: "Replace with declarative validation pattern" (pattern-specific)

The function appears complex by cyclomatic metrics but is actually **low cognitive load** due to its repetitive nature. The real issue is **boilerplate proliferation**, not complexity. The appropriate refactoring is to replace imperative validation with a declarative approach (e.g., validation rules/schema).

### Key Insight

**Low entropy + high branching = repetitive pattern, not true complexity**

When token entropy is low (< 0.35) and cyclomatic complexity is high (>= 10), the function likely contains repetitive logic that should be refactored into a data-driven or declarative approach.

## Objective

Add repetitive validation pattern detection to debtmap's complexity analysis pipeline to:
1. Correctly identify validation boilerplate (vs. true complexity)
2. Provide targeted recommendations for declarative refactoring
3. Adjust complexity scores to reflect actual cognitive load (not just branch count)

## Requirements

### Functional Requirements

1. **Pattern Detection**
   - Detect repetitive validation patterns: many early returns with identical structure
   - Distinguish from legitimate high branching (varied logic per branch)
   - Support detection across Rust, Python, JavaScript, and TypeScript
   - Identify common validation idioms (null checks, range checks, type checks)

2. **Heuristic Criteria**
   - **Low entropy signal**: token_entropy < 0.35 (highly repetitive)
   - **High branching**: cyclomatic >= 10
   - **Early return pattern**: majority of branches are early returns
   - **Structural similarity**: branches follow same pattern (if condition → return error)
   - **Validation keywords**: function name contains "validate", "check", "verify"

3. **Pattern-Specific Recommendations**
   - For validation boilerplate: "Replace with declarative validation pattern"
   - Suggest specific approaches: builder pattern, validation DSL, schema validators
   - Include impact estimates: reduction in LOC, not complexity (LOC may stay same or increase initially)
   - Provide language-specific refactoring examples

4. **Complexity Adjustment**
   - Dampen cyclomatic complexity for repetitive patterns
   - Apply dampening factor: `adjusted = cyclomatic * 0.4` (recognize low cognitive load)
   - Still flag as debt (boilerplate is technical debt), but separate category
   - Recommendation focuses on maintainability, not complexity reduction

### Non-Functional Requirements

- Performance: Pattern detection adds < 5% overhead to analysis time
- Accuracy: Precision >= 75% (avoid false positives on legitimate branching)
- Maintainability: Validation pattern definitions configurable
- Extensibility: Easy to add new validation pattern variants (builder, schema, etc.)

## Acceptance Criteria

- [ ] `ComplexityPattern` enum extended with `RepetitiveValidation` variant
- [ ] Pattern detection uses low entropy + high branching as primary signals
- [ ] Heuristics detect validation patterns with >= 75% precision on test corpus
- [ ] Recommendations include "Replace with declarative validation" with concrete examples
- [ ] Complexity dampening applied: `adjusted_cyclomatic = cyclomatic * 0.4`
- [ ] All tests pass, including new tests for entropy-based detection
- [ ] Documentation updated with validation pattern examples
- [ ] Integration test validates correct recommendation for `validate_config()` example

## Technical Details

### Implementation Approach

#### 1. Extend `ComplexityPattern` Enum

File: `src/priority/complexity_patterns.rs`

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComplexityPattern {
    // ... existing variants ...

    /// Repetitive validation pattern: many early returns with same structure
    RepetitiveValidation {
        validation_count: u32,    // Number of validation checks
        entropy: f64,             // Token entropy (low = repetitive)
        cyclomatic: u32,          // Raw cyclomatic (before dampening)
        adjusted_cyclomatic: u32, // Dampened complexity (reflects cognitive load)
    },
}
```

#### 2. Detection Heuristics

Add pattern detection logic to `ComplexityPattern::detect()`:

```rust
impl ComplexityPattern {
    pub fn detect(metrics: &ComplexityMetrics) -> Self {
        let ratio = metrics.cognitive as f64 / metrics.cyclomatic.max(1) as f64;

        // Priority order (updated with validation pattern):
        // 1. Repetitive Validation (low entropy + high branching)
        // 2. State machine / Coordinator
        // 3. Chaotic structure (high entropy)
        // 4. High nesting / High branching / Mixed
        // 5. Moderate complexity

        // Check for repetitive validation FIRST
        if let Some(entropy) = metrics.entropy_score {
            if is_repetitive_validation(metrics.cyclomatic, entropy, &metrics.validation_signals) {
                let adjusted = dampen_complexity_for_repetition(metrics.cyclomatic, entropy);
                return ComplexityPattern::RepetitiveValidation {
                    validation_count: metrics.validation_signals
                        .as_ref()
                        .map(|v| v.check_count)
                        .unwrap_or(metrics.cyclomatic),
                    entropy,
                    cyclomatic: metrics.cyclomatic,
                    adjusted_cyclomatic: adjusted,
                };
            }
        }

        // ... existing pattern detection logic ...
    }
}

/// Determine if metrics indicate repetitive validation pattern
fn is_repetitive_validation(
    cyclomatic: u32,
    entropy: f64,
    validation_signals: &Option<ValidationSignals>,
) -> bool {
    // Low entropy + high branching is primary signal
    let has_low_entropy_high_branching = entropy < 0.35 && cyclomatic >= 10;

    if !has_low_entropy_high_branching {
        return false;
    }

    // Additional validation signals strengthen confidence
    if let Some(signals) = validation_signals {
        // Require majority of branches to be early returns
        let early_return_ratio = signals.early_return_count as f64 / cyclomatic as f64;
        if early_return_ratio < 0.6 {
            return false;
        }

        // High structural similarity (measured by AST pattern matching)
        if signals.structural_similarity < 0.7 {
            return false;
        }

        true
    } else {
        // Fallback: low entropy + high branching + validation name
        has_low_entropy_high_branching
    }
}

/// Dampen cyclomatic complexity for repetitive patterns
fn dampen_complexity_for_repetition(cyclomatic: u32, entropy: f64) -> u32 {
    // Lower entropy = more dampening (recognition of low cognitive load)
    // entropy < 0.25: 60% dampening (very repetitive)
    // entropy < 0.30: 50% dampening (highly repetitive)
    // entropy < 0.35: 40% dampening (moderately repetitive)
    let dampening_factor = if entropy < 0.25 {
        0.4
    } else if entropy < 0.30 {
        0.5
    } else {
        0.6
    };

    (cyclomatic as f64 * dampening_factor).ceil() as u32
}
```

#### 3. Validation Signal Detection

**New Module**: `src/analyzers/validation_pattern_signals.rs`

```rust
use crate::core::FunctionMetrics;

/// Signals indicating repetitive validation pattern
#[derive(Debug, Clone)]
pub struct ValidationSignals {
    pub check_count: u32,           // Number of validation checks
    pub early_return_count: u32,    // Number of early returns
    pub structural_similarity: f64, // 0.0-1.0, how similar branches are
    pub has_validation_name: bool,  // Function name contains validation keywords
    pub validation_type: ValidationType,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationType {
    NullCheck,      // Checking for null/None/undefined
    RangeCheck,     // Numeric range validation
    TypeCheck,      // Type validation
    FormatCheck,    // String format validation (regex, etc.)
    Mixed,          // Multiple validation types
}

impl ValidationSignals {
    /// Detect validation signals from function metrics and AST
    pub fn detect(metrics: &FunctionMetrics, ast_info: &AstInfo) -> Option<Self> {
        // Detect validation function name pattern
        let has_validation_name = Self::has_validation_name(&metrics.name);

        // Count early returns in AST
        let early_return_count = Self::count_early_returns(ast_info);

        // Measure structural similarity of branches
        let structural_similarity = Self::measure_branch_similarity(ast_info);

        // Classify validation type
        let validation_type = Self::classify_validation_type(ast_info);

        // Calculate confidence score
        let confidence = Self::calculate_confidence(
            has_validation_name,
            early_return_count,
            structural_similarity,
            metrics.cyclomatic,
        );

        if confidence >= 0.6 {
            Some(ValidationSignals {
                check_count: metrics.cyclomatic,
                early_return_count,
                structural_similarity,
                has_validation_name,
                validation_type,
                confidence,
            })
        } else {
            None
        }
    }

    fn has_validation_name(name: &str) -> bool {
        const VALIDATION_KEYWORDS: &[&str] = &[
            "validate", "check", "verify", "ensure",
            "assert", "require", "guard", "sanitize",
        ];
        VALIDATION_KEYWORDS.iter().any(|kw| name.contains(kw))
    }

    fn count_early_returns(ast_info: &AstInfo) -> u32 {
        // Count return statements that are direct children of if expressions
        ast_info.early_return_patterns.len() as u32
    }

    fn measure_branch_similarity(ast_info: &AstInfo) -> f64 {
        // Compare AST structure of each branch
        // Returns 0.0 (completely different) to 1.0 (identical structure)

        if ast_info.branches.len() < 2 {
            return 0.0;
        }

        let first_branch = &ast_info.branches[0];
        let similarities: Vec<f64> = ast_info.branches[1..]
            .iter()
            .map(|branch| Self::compare_branch_structure(first_branch, branch))
            .collect();

        // Average similarity across all branches
        similarities.iter().sum::<f64>() / similarities.len() as f64
    }

    fn compare_branch_structure(branch1: &BranchAst, branch2: &BranchAst) -> f64 {
        // Simplified structural comparison
        // In practice: compare AST node types, depths, patterns

        let mut score = 0.0;

        // Same pattern: if condition { return error }
        if branch1.is_early_return && branch2.is_early_return {
            score += 0.5;
        }

        // Same condition type (e.g., both are is_none checks)
        if branch1.condition_type == branch2.condition_type {
            score += 0.3;
        }

        // Same return type (e.g., both return Result::Err)
        if branch1.return_type == branch2.return_type {
            score += 0.2;
        }

        score
    }

    fn classify_validation_type(ast_info: &AstInfo) -> ValidationType {
        let mut types = std::collections::HashSet::new();

        for branch in &ast_info.branches {
            if branch.condition_type.contains("is_none")
                || branch.condition_type.contains("is_null") {
                types.insert(ValidationType::NullCheck);
            } else if branch.condition_type.contains("range")
                || branch.condition_type.contains(">")
                || branch.condition_type.contains("<") {
                types.insert(ValidationType::RangeCheck);
            } else if branch.condition_type.contains("is_a")
                || branch.condition_type.contains("isinstance") {
                types.insert(ValidationType::TypeCheck);
            }
        }

        if types.len() > 1 {
            ValidationType::Mixed
        } else {
            types.into_iter().next().unwrap_or(ValidationType::Mixed)
        }
    }

    fn calculate_confidence(
        has_validation_name: bool,
        early_return_count: u32,
        structural_similarity: f64,
        cyclomatic: u32,
    ) -> f64 {
        let mut confidence = 0.0;

        // Validation name is strong signal
        if has_validation_name {
            confidence += 0.3;
        }

        // High ratio of early returns
        let early_return_ratio = early_return_count as f64 / cyclomatic.max(1) as f64;
        confidence += early_return_ratio * 0.4;

        // Structural similarity
        confidence += structural_similarity * 0.3;

        confidence.min(1.0)
    }
}
```

#### 4. AST Analysis Requirements

Extend existing analyzers to track validation signals:

**Rust** (`src/analyzers/rust.rs`):
- Detect early return patterns: `if condition { return Err(...) }`
- Track `is_none()`, `is_some()` method calls
- Count consecutive if-return blocks
- Measure structural similarity of conditional branches

**Python** (`src/analyzers/python.rs`):
- Detect early return patterns: `if not value: raise ValueError(...)`
- Track `is None`, `is not None` comparisons
- Identify validation decorators: `@validate`, `@check`
- Count `assert` statements

**JavaScript/TypeScript** (`src/analyzers/javascript/*.rs`):
- Detect early return patterns: `if (!value) throw new Error(...)`
- Track `null` and `undefined` checks
- Identify validation libraries (joi, yup, zod)
- Count guard clauses

#### 5. Recommendation Generation

File: `src/priority/scoring/concise_recommendation.rs`

```rust
fn generate_repetitive_validation_recommendation(
    validation_count: u32,
    entropy: f64,
    cyclomatic: u32,
    adjusted_cyclomatic: u32,
    metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    let language = crate::core::Language::from_path(&metrics.file);
    let boilerplate_reduction = RefactoringImpact::validation_extraction(validation_count);

    let steps = vec![
        ActionStep {
            description: "Replace imperative validation with declarative pattern".to_string(),
            impact: format!(
                "-{} LOC boilerplate, improved maintainability ({})",
                validation_count * 2,  // Each validation ~2 lines
                boilerplate_reduction.confidence.as_str()
            ),
            difficulty: Difficulty::Medium,
            commands: add_declarative_validation_examples(&language, validation_count),
        },
        ActionStep {
            description: "Extract validation rules into data structure".to_string(),
            impact: format!(
                "Single source of truth for {} validation rules",
                validation_count
            ),
            difficulty: Difficulty::Medium,
            commands: vec![
                "# Define validation schema/rules declaratively".to_string(),
                format!("# Example: [{} required fields in config]", validation_count),
            ],
        },
        ActionStep {
            description: "Add comprehensive validation tests".to_string(),
            impact: "Ensure all validation rules covered by tests".to_string(),
            difficulty: Difficulty::Easy,
            commands: vec![
                "cargo test validate_*".to_string(),
                "# Test each validation rule independently".to_string(),
            ],
        },
    ];

    let estimated_effort = (validation_count as f32 / 10.0) * 1.5; // ~1.5hr per 10 validations

    ActionableRecommendation {
        primary_action: format!(
            "Replace {} repetitive validation checks with declarative pattern",
            validation_count
        ),
        rationale: format!(
            "Repetitive validation pattern detected (entropy {:.2}, {} checks). \
             Low entropy ({:.2}) indicates boilerplate, not complexity. \
             Adjusted complexity: {} → {} (reflects actual cognitive load). \
             Refactoring improves maintainability and reduces error-prone boilerplate.",
            entropy,
            validation_count,
            entropy,
            cyclomatic,
            adjusted_cyclomatic
        ),
        implementation_steps: vec![],
        related_items: vec![],
        steps: Some(steps),
        estimated_effort_hours: Some(estimated_effort),
    }
}

fn add_declarative_validation_examples(language: &Language, count: u32) -> Vec<String> {
    match language {
        Language::Rust => vec![
            "# Option 1: Builder pattern with validation".to_string(),
            "# ConfigBuilder::new().required(\"output_dir\").build()?".to_string(),
            "".to_string(),
            "# Option 2: Validation trait".to_string(),
            "# impl Validate for Config { fn validate(&self) -> Result<()> }".to_string(),
            "".to_string(),
            "# Option 3: Macro-based validation".to_string(),
            "# #[validate(required = [\"output_dir\", \"max_workers\", ...])]".to_string(),
        ],
        Language::Python => vec![
            "# Option 1: Pydantic model".to_string(),
            "# class Config(BaseModel):".to_string(),
            "#     output_dir: str".to_string(),
            "#     max_workers: int".to_string(),
            "".to_string(),
            "# Option 2: attrs with validators".to_string(),
            "# @define".to_string(),
            "# class Config:".to_string(),
            "#     output_dir: str = field(validator=instance_of(str))".to_string(),
        ],
        Language::JavaScript | Language::TypeScript => vec![
            "# Option 1: Joi schema".to_string(),
            format!("# const schema = Joi.object({{ /* {} fields */ }});", count),
            "# schema.validate(config);".to_string(),
            "".to_string(),
            "# Option 2: Zod schema".to_string(),
            "# const ConfigSchema = z.object({ ... });".to_string(),
            "# ConfigSchema.parse(config);".to_string(),
        ],
        _ => vec![
            "# Use declarative validation approach for your language".to_string(),
        ],
    }
}
```

#### 6. Impact Estimation

File: `src/priority/refactoring_impact.rs`

```rust
impl RefactoringImpact {
    /// Estimate impact of extracting validation boilerplate
    pub fn validation_extraction(validation_count: u32) -> Self {
        // Validation extraction impact is about maintainability, not complexity reduction
        // - Reduces LOC (lines of code) by consolidating checks
        // - Improves maintainability (single source of truth)
        // - May not reduce cyclomatic complexity initially
        // - Reduces cognitive load for understanding validation logic

        let loc_reduction = validation_count * 2; // Each check ~2 lines

        RefactoringImpact {
            complexity_reduction: 0, // Complexity already dampened
            cognitive_reduction: validation_count, // Easier to understand declarative rules
            cyclomatic_reduction: 0, // May stay same or increase with schema
            loc_reduction: Some(loc_reduction),
            maintainability_improvement: ImpactLevel::High,
            confidence: if validation_count >= 10 {
                ImpactConfidence::High
            } else {
                ImpactConfidence::Medium
            },
            technique: "Declarative validation extraction".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImpactLevel {
    Low,
    Medium,
    High,
}
```

### Architecture Changes

1. **New Module**: `src/analyzers/validation_pattern_signals.rs`
   - Pure functions for detecting validation signals from AST
   - Language-agnostic signal detection interface
   - Structural similarity measurement

2. **Modified Modules**:
   - `src/priority/complexity_patterns.rs`: Add `RepetitiveValidation` variant
   - `src/priority/scoring/concise_recommendation.rs`: Add validation-specific recommendations
   - `src/priority/refactoring_impact.rs`: Add validation impact estimator
   - `src/analyzers/rust.rs`: Track validation signals during AST traversal
   - `src/analyzers/python.rs`: Track validation signals
   - `src/analyzers/javascript/*.rs`: Track validation signals

3. **Configuration**: Add tunable parameters to `debtmap.toml`:
   ```toml
   [patterns.repetitive_validation]
   enabled = true
   min_checks = 10
   max_entropy = 0.35
   min_early_return_ratio = 0.6
   min_structural_similarity = 0.7
   dampening_factor = 0.4  # How much to reduce cyclomatic complexity
   ```

### Data Structures

```rust
/// Information needed to detect validation patterns from AST
#[derive(Debug, Clone)]
pub struct AstInfo {
    pub branches: Vec<BranchAst>,
    pub early_return_patterns: Vec<EarlyReturnPattern>,
}

#[derive(Debug, Clone)]
pub struct BranchAst {
    pub is_early_return: bool,
    pub condition_type: String,  // e.g., "is_none", "range_check"
    pub return_type: String,     // e.g., "Err", "raise"
}

#[derive(Debug, Clone)]
pub struct EarlyReturnPattern {
    pub line: u32,
    pub condition: String,
    pub error_message: String,
}
```

### Example Detection

Input code (Rust):
```rust
pub fn validate_config(config: &Config) -> Result<()> {
    if config.output_dir.is_none() {
        return Err(anyhow!("output_dir required"));
    }
    if config.max_workers.is_none() {
        return Err(anyhow!("max_workers required"));
    }
    // ... 18 more identical checks ...
    Ok(())
}
```

Detected signals:
- Token entropy: 0.28 (very low - highly repetitive) ✓
- Cyclomatic complexity: 20 (high branching) ✓
- Early return count: 20 (all branches) ✓
- Structural similarity: 0.95 (nearly identical) ✓
- Validation name: "validate_config" ✓

Pattern: **RepetitiveValidation** (high confidence)

Complexity adjustment: 20 → 8 (dampened by 0.4 factor)

Recommendation:
```
Replace 20 repetitive validation checks with declarative pattern

RATIONALE: Repetitive validation pattern detected (entropy 0.28, 20 checks).
Low entropy (0.28) indicates boilerplate, not complexity.
Adjusted complexity: 20 → 8 (reflects actual cognitive load).
Refactoring improves maintainability and reduces error-prone boilerplate.

STEPS:
1. Replace imperative validation with declarative pattern
   Impact: -40 LOC boilerplate, improved maintainability (high impact)
   Difficulty: Medium

   # Option 1: Builder pattern with validation
   # ConfigBuilder::new().required("output_dir").build()?

   # Option 2: Validation trait
   # impl Validate for Config { fn validate(&self) -> Result<()> }

   # Option 3: Macro-based validation
   # #[validate(required = ["output_dir", "max_workers", ...])]

2. Extract validation rules into data structure
   Impact: Single source of truth for 20 validation rules
   Difficulty: Medium

3. Add comprehensive validation tests
   Impact: Ensure all validation rules covered by tests
   Difficulty: Easy

Estimated effort: 3.0 hours
```

## Dependencies

**Prerequisites**:
- [179] State Machine/Coordinator Pattern Detection (shares complexity pattern infrastructure)

**Affected Components**:
- `ComplexityPattern` enum (new variant)
- `ComplexityPattern::detect()` (priority changes)
- `generate_complexity_steps()` (new recommendation branch)
- AST analyzers (track validation signals)

**External Dependencies**: None

## Testing Strategy

### Unit Tests

File: `src/priority/complexity_patterns.rs`

```rust
#[test]
fn detect_repetitive_validation_pattern() {
    let metrics = ComplexityMetrics {
        cyclomatic: 20,
        cognitive: 25,  // Ratio ~1.25 (low for cyclomatic)
        nesting: 1,     // Flat structure
        entropy_score: Some(0.28),  // Very low - repetitive
        validation_signals: Some(ValidationSignals {
            check_count: 20,
            early_return_count: 20,
            structural_similarity: 0.95,
            has_validation_name: true,
            validation_type: ValidationType::NullCheck,
            confidence: 0.9,
        }),
    };

    let pattern = ComplexityPattern::detect(&metrics);
    assert!(matches!(pattern, ComplexityPattern::RepetitiveValidation { .. }));

    if let ComplexityPattern::RepetitiveValidation {
        validation_count,
        entropy,
        cyclomatic,
        adjusted_cyclomatic,
    } = pattern
    {
        assert_eq!(validation_count, 20);
        assert_eq!(cyclomatic, 20);
        assert_eq!(adjusted_cyclomatic, 8);  // 20 * 0.4 dampening
        assert!(entropy < 0.35);
    }
}

#[test]
fn validation_pattern_takes_precedence_over_high_branching() {
    // High branching metrics BUT repetitive validation signals
    let metrics = ComplexityMetrics {
        cyclomatic: 18,
        cognitive: 20,
        nesting: 1,
        entropy_score: Some(0.30),  // Low entropy
        validation_signals: Some(ValidationSignals {
            check_count: 18,
            early_return_count: 18,
            structural_similarity: 0.92,
            has_validation_name: true,
            validation_type: ValidationType::NullCheck,
            confidence: 0.85,
        }),
    };

    let pattern = ComplexityPattern::detect(&metrics);
    assert!(
        matches!(pattern, ComplexityPattern::RepetitiveValidation { .. }),
        "Repetitive validation should take precedence over high branching"
    );
}

#[test]
fn high_entropy_prevents_validation_detection() {
    // High branching but HIGH entropy (varied logic)
    let metrics = ComplexityMetrics {
        cyclomatic: 20,
        cognitive: 45,
        nesting: 2,
        entropy_score: Some(0.55),  // High entropy - NOT repetitive
        validation_signals: None,
    };

    let pattern = ComplexityPattern::detect(&metrics);
    assert!(
        !matches!(pattern, ComplexityPattern::RepetitiveValidation { .. }),
        "High entropy should prevent validation pattern detection"
    );
}

#[test]
fn dampening_factor_scales_with_entropy() {
    assert_eq!(dampen_complexity_for_repetition(20, 0.20), 8);  // 0.4 factor
    assert_eq!(dampen_complexity_for_repetition(20, 0.28), 10); // 0.5 factor
    assert_eq!(dampen_complexity_for_repetition(20, 0.33), 12); // 0.6 factor
}
```

### Integration Tests

File: `tests/validation_pattern_detection.rs`

```rust
#[test]
fn analyze_validate_config_function() {
    let code = include_str!("../samples/validation.rs");
    let result = analyze_rust_code(code, "validation.rs");

    // Find validate_config function
    let func = result.functions.iter()
        .find(|f| f.name == "validate_config")
        .expect("validate_config not found");

    // Verify metrics
    assert_eq!(func.cyclomatic, 20);
    assert!(func.entropy_score.as_ref().unwrap().token_entropy < 0.35);

    // Verify pattern detection
    let pattern = ComplexityPattern::detect(&ComplexityMetrics {
        cyclomatic: func.cyclomatic,
        cognitive: func.cognitive,
        nesting: func.nesting,
        entropy_score: func.entropy_score.as_ref().map(|e| e.token_entropy),
        validation_signals: func.validation_signals.clone(),
    });

    assert!(
        matches!(pattern, ComplexityPattern::RepetitiveValidation { .. }),
        "Expected RepetitiveValidation pattern, got: {:?}", pattern
    );

    // Verify complexity dampening
    if let ComplexityPattern::RepetitiveValidation { adjusted_cyclomatic, .. } = pattern {
        assert!(
            adjusted_cyclomatic < 10,
            "Adjusted complexity should be dampened to < 10"
        );
    }

    // Verify recommendation
    let recommendation = generate_concise_recommendation(
        &DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        },
        func,
        FunctionRole::Validator,
        &None,
    );

    assert!(
        recommendation.primary_action.contains("declarative"),
        "Recommendation should mention declarative pattern"
    );

    assert!(
        recommendation.rationale.contains("entropy"),
        "Rationale should explain low entropy signal"
    );
}
```

### Property Tests

```rust
proptest! {
    #[test]
    fn repetitive_validation_requires_low_entropy(
        cyclomatic in 10u32..50,
        entropy in 0.0f64..1.0,
    ) {
        let metrics = ComplexityMetrics {
            cyclomatic,
            cognitive: cyclomatic + 5,
            nesting: 1,
            entropy_score: Some(entropy),
            validation_signals: Some(ValidationSignals {
                check_count: cyclomatic,
                early_return_count: cyclomatic,
                structural_similarity: 0.9,
                has_validation_name: true,
                validation_type: ValidationType::NullCheck,
                confidence: 0.8,
            }),
        };

        let pattern = ComplexityPattern::detect(&metrics);

        // Validation pattern should only trigger if entropy < 0.35
        if matches!(pattern, ComplexityPattern::RepetitiveValidation { .. }) {
            prop_assert!(entropy < 0.35);
        }
    }

    #[test]
    fn dampening_never_increases_complexity(
        cyclomatic in 1u32..100,
        entropy in 0.0f64..0.35,
    ) {
        let adjusted = dampen_complexity_for_repetition(cyclomatic, entropy);
        prop_assert!(adjusted <= cyclomatic);
        prop_assert!(adjusted >= (cyclomatic as f64 * 0.4).ceil() as u32);
    }
}
```

## Documentation Requirements

### Code Documentation

1. **Module-level docs** in `src/analyzers/validation_pattern_signals.rs`:
   - Explain repetitive validation pattern detection
   - Provide examples of validation boilerplate
   - Document heuristics and entropy thresholds

2. **Function docs** for `ComplexityPattern::detect()`:
   - Update priority order to include validation pattern
   - Add examples of validation pattern detection
   - Explain complexity dampening rationale

3. **Examples** in `ComplexityPattern` enum:
   - Show validation pattern detection example
   - Explain adjusted vs. raw cyclomatic complexity
   - Demonstrate declarative refactoring benefits

### User Documentation

Update `README.md` or documentation site:

1. **Pattern Detection Section**:
   - Add repetitive validation to pattern list
   - Explain low entropy signal
   - Clarify difference between complexity and boilerplate

2. **Complexity Adjustment**:
   - Document dampening for repetitive patterns
   - Explain why adjusted complexity reflects cognitive load
   - Show before/after metrics

3. **Configuration Guide**:
   - Document `[patterns.repetitive_validation]` configuration
   - Explain tunable thresholds
   - Provide examples of when to adjust settings

4. **Examples**:
   - Add `validation.rs` to examples directory
   - Show before/after refactoring
   - Include declarative validation examples for each language

### Architecture Updates

Update `ARCHITECTURE.md`:

1. **Pattern Detection Pipeline**:
   - Document validation pattern priority
   - Explain entropy-based detection
   - Show complexity dampening logic

2. **Metrics and Adjustments**:
   - Explain difference between raw and adjusted complexity
   - Document when/why adjustments are applied
   - Show impact on recommendations

## Implementation Notes

### Detection Priority Order

Updated pattern detection priority (highest to lowest):

1. **Repetitive Validation** (low entropy + high branching - boilerplate, not complexity)
2. **State Machine / Coordinator** (specific, high-value patterns)
3. **Chaotic Structure** (high entropy - requires standardization)
4. **High Nesting** (primary driver is depth)
5. **High Branching** (primary driver is decisions)
6. **Mixed Complexity** (both nesting and branching)
7. **Moderate Complexity** (default/fallback)

Rationale: Validation patterns misclassified as high complexity lead to worst user experience.

### Entropy Thresholds

**Challenge**: Choosing the right entropy threshold to distinguish repetitive from varied logic.

**Analysis**:
- Typical validation functions: 0.20 - 0.35 (very repetitive)
- Typical business logic: 0.40 - 0.70 (moderate variety)
- Complex algorithms: 0.60 - 0.80 (high variety)

**Decision**: Use 0.35 as threshold
- Conservative: catches clear repetition
- Minimizes false positives
- Can be tuned via configuration

### Structural Similarity Measurement

Measuring how similar branches are:

```rust
// Compare two branches for structural similarity
// Returns 0.0 (completely different) to 1.0 (identical)
fn compare_branch_structure(branch1: &BranchAst, branch2: &BranchAst) -> f64 {
    let mut score = 0.0;

    // Both are early returns? +0.5
    if branch1.is_early_return && branch2.is_early_return {
        score += 0.5;
    }

    // Same condition pattern? +0.3
    // e.g., both check is_none(), or both check range
    if branch1.condition_type == branch2.condition_type {
        score += 0.3;
    }

    // Same return type? +0.2
    // e.g., both return Err with string message
    if branch1.return_type == branch2.return_type {
        score += 0.2;
    }

    score
}
```

For validation functions, expect similarity > 0.9 (nearly identical structure).

### Complexity Dampening Rationale

**Why dampen cyclomatic complexity?**

Traditional cyclomatic complexity counts decision points but doesn't account for cognitive load. A function with 20 identical validations has:
- High cyclomatic (20 branches)
- Low cognitive load (same pattern 20 times)

Dampening recognizes this:
- Raw cyclomatic: 20 (for tools that expect it)
- Adjusted cyclomatic: 8 (reflects actual cognitive load)
- Recommendations based on adjusted score

**Dampening factors**:
- entropy < 0.25: 0.4 factor (very repetitive - 60% reduction)
- entropy < 0.30: 0.5 factor (highly repetitive - 50% reduction)
- entropy < 0.35: 0.6 factor (moderately repetitive - 40% reduction)

### Language-Specific Considerations

**Rust**:
- Strong typing enables reliable pattern detection
- `is_none()`, `is_some()` are clear null check patterns
- Look for `anyhow!()`, `bail!()` error returns
- Consider validation crates: `validator`, `garde`

**Python**:
- Duck typing makes detection harder
- Look for `if not value:`, `if value is None:`
- Detect `raise ValueError(...)`, `raise TypeError(...)`
- Consider Pydantic, attrs validators

**JavaScript/TypeScript**:
- TypeScript types help (when present)
- Look for `if (!value)`, `if (value == null)`
- Detect `throw new Error(...)`, `throw new ValidationError(...)`
- Consider Joi, Yup, Zod validation libraries

### Refactoring Examples

For `validate_config()`, recommended refactoring:

**Before** (imperative, 92 lines):
```rust
pub fn validate_config(config: &Config) -> Result<()> {
    if config.output_dir.is_none() {
        return Err(anyhow!("output_dir required"));
    }
    if config.max_workers.is_none() {
        return Err(anyhow!("max_workers required"));
    }
    // ... 18 more ...
    Ok(())
}
```

**After** (declarative, ~30 lines with trait):
```rust
impl Validate for Config {
    fn validate(&self) -> Result<()> {
        const REQUIRED_FIELDS: &[(&str, fn(&Config) -> bool)] = &[
            ("output_dir", |c| c.output_dir.is_some()),
            ("max_workers", |c| c.max_workers.is_some()),
            // ... 18 more ...
        ];

        for (field, check) in REQUIRED_FIELDS {
            if !check(self) {
                return Err(anyhow!("{} required", field));
            }
        }

        Ok(())
    }
}

// Usage: config.validate()?
```

**Or with macro** (most concise):
```rust
#[derive(Validate)]
pub struct Config {
    #[validate(required)]
    pub output_dir: Option<String>,
    #[validate(required)]
    pub max_workers: Option<usize>,
    // ... 18 more ...
}

// Usage: config.validate()?
```

Benefits:
- Single source of truth for validation rules
- Easy to add/remove validations
- Less error-prone (no copy-paste errors)
- Better test coverage (test validation logic once)

## Migration and Compatibility

### Breaking Changes

None. This is a pure addition with no API changes.

### Compatibility

- Existing complexity analysis continues to work
- Functions previously classified as `HighBranching` may now be `RepetitiveValidation`
- Adjusted complexity scores are new (doesn't affect existing metrics)
- Configuration is optional (defaults to enabled)

### Migration Path

1. Deploy with validation pattern detection enabled (default)
2. Monitor classification changes (functions moving from HighBranching to RepetitiveValidation)
3. Collect user feedback on recommendation quality
4. Tune thresholds based on false positive/negative rates
5. Add language-specific improvements iteratively

### Rollback Plan

If pattern detection causes issues:
1. Set `patterns.repetitive_validation.enabled = false` in config
2. Functions will fall back to generic pattern detection (HighBranching)
3. No code changes required

## Success Metrics

### Quantitative

- **Detection rate**: >= 60% of validation boilerplate functions detected
- **Precision**: >= 75% (avoid false positives on legitimate branching)
- **Complexity adjustment accuracy**: Adjusted score within ±20% of expert assessment
- **Performance**: < 5% analysis time overhead
- **User satisfaction**: >= 80% find recommendations helpful (survey)

### Qualitative

- Recommendations clearly distinguish boilerplate from complexity
- Users understand why complexity was adjusted
- Declarative refactoring guidance is actionable
- Reduces frustration with misleading complexity scores

### Validation

1. **Test corpus**: Create 100+ examples of validation functions across languages
2. **Benchmark**: Measure detection accuracy on corpus
3. **Expert review**: Compare adjusted scores with expert cognitive load assessments
4. **User study**: Survey 10+ users on recommendation quality
5. **Performance**: Profile analysis time with/without pattern detection

## Open Questions

1. **Should we detect validation library usage explicitly?**
   - Pros: Higher confidence, library-specific recommendations
   - Cons: Maintenance burden, language-specific
   - Decision: Start with generic detection, add library support if needed

2. **How to handle mixed validation (null checks + range checks + type checks)?**
   - Current plan: Still detect as repetitive if entropy is low
   - Alternative: Require homogeneous validation types
   - Decision: Entropy is sufficient signal regardless of validation type mix

3. **Should dampening affect debt scoring?**
   - Current plan: Yes, use adjusted complexity for scoring
   - Alternative: Use raw complexity for scoring, adjusted for display only
   - Decision: Use adjusted for scoring (reflects actual risk/effort)

4. **How to validate dampening factor accuracy?**
   - Current plan: Empirical testing with expert assessments
   - Need: Collect cognitive load ratings for sample functions
   - Decision: Start with 0.4/0.5/0.6 factors, refine based on data

## Future Enhancements

1. **Automated refactoring**: Generate validation trait/schema from existing checks
2. **Library-specific detection**: Detect Pydantic, Joi, Zod usage and provide targeted advice
3. **Validation coverage analysis**: Ensure all fields have validation rules
4. **Custom validation patterns**: Allow users to define project-specific validation patterns
5. **IDE integration**: Real-time validation pattern detection in editor
6. **Validation rule extraction**: Extract existing validation logic to config/schema

## Related Work

- **Spec 179**: State Machine/Coordinator Pattern Detection (similar pattern-based approach)
- **Spec 176**: Entropy vs effective complexity (uses entropy for pattern detection)
- **Spec 177**: Role-aware complexity recommendations (validators are a role)
- **Spec 178**: Fix moderate complexity recommendation logic
- **Spec 116**: Confidence scoring (applies to pattern detection confidence)

## References

- Shannon entropy for code complexity: https://arxiv.org/abs/1602.06516
- Cyclomatic complexity limitations: McCabe, "A Complexity Measure" (1976)
- Declarative validation patterns:
  - Pydantic: https://pydantic-docs.helpmanual.io/
  - Joi: https://joi.dev/
  - Rust validator crate: https://github.com/Keats/validator
- Debtmap issue #XXX (if created for this spec)
