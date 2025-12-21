---
number: 4
title: Expanded Validation Patterns
category: foundation
priority: medium
status: draft
dependencies: [1]
created: 2025-12-20
---

# Specification 004: Expanded Validation Patterns

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 001 (Stillwater 0.15 Upgrade)

## Context

Debtmap currently uses stillwater's Validation type for configuration validation only (`src/config/validation.rs`). This accumulates ALL configuration errors before failing, which is valuable for user experience.

However, the same pattern isn't applied to:
- Analysis results validation (debt detection rules)
- Phase transition validation (workflow guards)
- Input file validation (parsing errors across multiple files)
- Runtime state validation

Expanding validation patterns would enable:
- Accumulating all debt findings across a codebase
- Collecting all parse errors instead of failing on first error
- Validating multiple configuration sources simultaneously
- Composable validation rules using predicate combinators

## Objective

Expand the validation pattern beyond configuration to analysis results, file processing, and workflow phases, enabling comprehensive error accumulation and composable validation rules using stillwater's predicate combinators.

## Requirements

### Functional Requirements

1. **Analysis Results Validation**
   - Validate all files in parallel, accumulating errors
   - Continue analysis even when some files fail
   - Collect all debt findings as a validation result
   - Return comprehensive error reports

2. **Predicate Combinators for Debt Rules**
   - Define debt detection rules as composable predicates
   - Support AND/OR/NOT composition of rules
   - Enable dynamic rule configuration
   - Provide clear error messages for each rule

3. **Workflow Phase Validation**
   - Validate phase transitions with accumulated errors
   - Guard functions return Validation instead of Result
   - Collect all guard failures before rejecting transition

4. **File Processing Validation**
   - Accumulate parse errors across files
   - Track partial success (some files parsed, some failed)
   - Enable "best effort" analysis mode

### Non-Functional Requirements

- Maintain backwards compatibility with existing Result-based APIs
- Provide clear, actionable error messages
- Support efficient parallel validation
- Enable easy testing of validation rules

## Acceptance Criteria

- [ ] `validate_files()` function accumulates errors across files
- [ ] Predicate combinators available for complexity thresholds
- [ ] Predicate combinators available for naming conventions
- [ ] Predicate combinators available for debt severity rules
- [ ] Phase guard functions return `AnalysisValidation<PhaseTransition>`
- [ ] `ValidatedFileResults` type with partial success semantics
- [ ] `ensure()` extension trait for applying predicates
- [ ] Existing configuration validation continues to work
- [ ] New tests for predicate composition
- [ ] Documentation for custom rule creation

## Technical Details

### Implementation Approach

```rust
// Predicate-based debt rules
use stillwater::predicate::*;

// Complexity thresholds as predicates
pub fn high_complexity_predicate() -> impl Predicate<u32> {
    gt(20).and(lt(100))  // Warning zone: 21-99
}

pub fn critical_complexity_predicate() -> impl Predicate<u32> {
    ge(100)  // Critical: >= 100
}

// Naming convention predicates
pub fn valid_function_name() -> impl Predicate<&str> {
    len_between(2, 50)
        .and(starts_with(|c: char| c.is_lowercase() || c == '_'))
        .and(all_chars(|c| c.is_alphanumeric() || c == '_'))
}

// Compose into debt detection rules
pub fn function_debt_rules() -> impl Predicate<&FunctionMetrics> {
    all_of([
        |f: &FunctionMetrics| high_complexity_predicate().check(&f.cognitive).is_ok(),
        |f: &FunctionMetrics| valid_function_name().check(&f.name).is_ok(),
    ])
}
```

### File Validation with Accumulation

```rust
// Process all files, accumulating successes and failures
pub fn validate_files(
    paths: Vec<PathBuf>,
) -> impl Effect<Output = ValidatedFileResults, Error = Never, Env = RealEnv> {
    let validations: Vec<AnalysisValidation<FileMetrics>> = paths
        .into_par_iter()
        .map(|path| {
            match parse_and_analyze(&path) {
                Ok(metrics) => Validation::success(metrics),
                Err(e) => Validation::failure(AnalysisError::parse(path, e)),
            }
        })
        .collect();

    // Combine all validations
    let combined = Validation::all(validations);

    pure(match combined {
        Ok(metrics) => ValidatedFileResults::AllSucceeded(metrics),
        Err(errors) => ValidatedFileResults::PartialSuccess {
            succeeded: extract_successes(&validations),
            failures: errors,
        },
    })
}

pub enum ValidatedFileResults {
    AllSucceeded(Vec<FileMetrics>),
    PartialSuccess {
        succeeded: Vec<FileMetrics>,
        failures: NonEmptyVec<AnalysisError>,
    },
}
```

### Workflow Phase Guards with Validation

```rust
// Guard returns validation instead of result
pub fn can_transition_to_scoring(
    state: &AnalysisState,
) -> AnalysisValidation<PhaseTransition> {
    Validation::all((
        validate_call_graph_complete(state),
        validate_coverage_loaded_if_required(state),
        validate_purity_analysis_complete(state),
    ))
    .map(|(cg, cov, purity)| PhaseTransition::ToScoring {
        call_graph: cg,
        coverage: cov,
        purity: purity,
    })
}

// Individual guard validations
fn validate_call_graph_complete(state: &AnalysisState) -> AnalysisValidation<CallGraph> {
    match &state.call_graph {
        Some(cg) if cg.is_complete() => Validation::success(cg.clone()),
        Some(_) => Validation::failure(
            AnalysisError::validation("Call graph incomplete")
        ),
        None => Validation::failure(
            AnalysisError::validation("Call graph not built")
        ),
    }
}
```

### Ensure Extension Trait

```rust
/// Extension trait for applying predicates to values
pub trait EnsureExt<T> {
    fn ensure<P, E>(self, predicate: P, error: E) -> Validation<T, E>
    where
        P: Predicate<T>;
}

impl<T> EnsureExt<T> for T {
    fn ensure<P, E>(self, predicate: P, error: E) -> Validation<T, E>
    where
        P: Predicate<T>,
    {
        if predicate.check(&self).is_ok() {
            Validation::success(self)
        } else {
            Validation::failure(error)
        }
    }
}

// Usage
let validated_complexity = metrics.cognitive
    .ensure(lt(100), DebtItem::critical_complexity(path, metrics.cognitive));
```

### Data Structures

```rust
// Rule configuration for dynamic validation
pub struct ValidationRuleSet {
    pub complexity_warning: u32,
    pub complexity_critical: u32,
    pub max_function_length: usize,
    pub max_nesting_depth: usize,
    pub naming_patterns: Vec<NamePattern>,
}

impl ValidationRuleSet {
    pub fn complexity_predicate(&self) -> impl Predicate<u32> {
        lt(self.complexity_critical)
    }

    pub fn function_length_predicate(&self) -> impl Predicate<usize> {
        le(self.max_function_length)
    }
}
```

### Affected Files

- `src/effects/core.rs` - Add EnsureExt trait
- `src/effects/validation.rs` - New module for validation helpers
- `src/config/validation.rs` - Refactor to use predicate combinators
- `src/analysis/workflow/guards.rs` - Return Validation instead of Result
- `src/debt/rules.rs` - New module for predicate-based rules
- `src/analyzers/*.rs` - Use validate_files pattern

## Dependencies

- **Prerequisites**: Spec 001 (Stillwater 0.15 Upgrade)
- **Affected Components**: Effect system, config, workflow, all analyzers
- **External Dependencies**: stillwater 0.15 predicate module

## Testing Strategy

- **Unit Tests**: Test each predicate combinator in isolation
- **Integration Tests**: Full validation with mixed success/failure
- **Property Tests**: Verify predicate composition laws
- **Performance Tests**: Validate parallel validation performance

```rust
#[test]
fn predicate_composition_works() {
    let rule = high_complexity_predicate().and(lt(50));

    assert!(rule.check(&30).is_ok());   // 20 < 30 < 50
    assert!(rule.check(&60).is_err());  // 60 >= 50
    assert!(rule.check(&15).is_err());  // 15 <= 20
}

#[test]
fn file_validation_accumulates_errors() {
    let paths = vec![good_file(), bad_file_1(), bad_file_2()];
    let result = validate_files(paths).run(&env).await;

    match result {
        ValidatedFileResults::PartialSuccess { succeeded, failures } => {
            assert_eq!(succeeded.len(), 1);
            assert_eq!(failures.len(), 2);
        }
        _ => panic!("Expected partial success"),
    }
}
```

## Documentation Requirements

- **Code Documentation**: Document predicate combinators with examples
- **User Documentation**: Explain validation rule configuration
- **Architecture Updates**: Document validation architecture

## Implementation Notes

- Start with complexity predicates, then expand to naming and structure
- Ensure predicates are composable and reusable
- Consider providing a "strict" vs "lenient" mode for file validation
- Cache predicate compilation for performance with dynamic rules
- Provide good error messages that reference the failing rule

## Migration and Compatibility

Backwards compatible. New validation patterns are additive. Existing `validate_config()` continues to work. Guard functions can be migrated incrementally by changing return type from `Result<T, E>` to `Validation<T, NonEmptyVec<E>>`.
