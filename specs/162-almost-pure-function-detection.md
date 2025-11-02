---
number: 162
title: Almost Pure Function Detection and Refactoring Suggestions
category: foundation
priority: high
status: draft
dependencies: [156, 157, 158]
created: 2025-11-01
---

# Specification 162: Almost Pure Function Detection and Refactoring Suggestions

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Specs 156-158 (Purity Analysis)

## Context

**Missing Feature**: Functions with 1-2 purity violations get same treatment as deeply impure functions. These "almost pure" functions are **easy wins** for refactoring.

**Example**:
```rust
fn calculate_total(items: &[Item]) -> f64 {
    let total = items.iter().map(|i| i.price).sum();
    println!("Total: {}", total);  // <-- Only violation
    total
}
```

This function is one line away from being pure (0.70x multiplier), but currently gets 1.0x multiplier.

## Objective

Detect "almost pure" functions with 1-2 violations and provide **specific, low-effort refactoring suggestions** with quantified impact.

## Requirements

1. **Violation Detection**
   - Identify functions with exactly 1-2 purity violations
   - Classify violation types (logging, time query, random, file I/O)
   - Track violation locations (line numbers)

2. **Refactoring Strategies**
   - Extract Logging (effort: low)
   - Parameterize Time (effort: low)
   - Inject Random Seed (effort: low)
   - Separate I/O from Logic (effort: medium)

3. **Impact Quantification**
   - Show multiplier change (1.0x → 0.70x)
   - Calculate complexity reduction (30%)
   - Risk level improvement (High → Low)

4. **Recommendation Format**
   - Show current code with violation highlighted
   - Show suggested refactoring
   - Quantify impact and effort

## Implementation

```rust
#[derive(Debug, Clone)]
pub struct AlmostPureFunction {
    pub function_id: FunctionId,
    pub violations: Vec<PurityViolation>,
    pub suggested_strategy: RefactoringStrategy,
    pub current_multiplier: f64,
    pub potential_multiplier: f64,
}

#[derive(Debug, Clone)]
pub enum PurityViolation {
    Logging { line: usize, macro_name: String },
    TimeQuery { line: usize, method: String },
    RandomGen { line: usize },
    FileRead { line: usize, path: Option<String> },
    SingleMutation { line: usize, target: String },
}

#[derive(Debug, Clone)]
pub struct RefactoringStrategy {
    pub strategy_type: StrategyType,
    pub effort: Effort,
    pub code_before: String,
    pub code_after: String,
    pub explanation: String,
}

#[derive(Debug, Clone)]
pub enum StrategyType {
    ExtractLogging,
    ParameterizeTime,
    InjectRandomSeed,
    SeparateIoFromLogic,
    IsolateSingleViolation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effort {
    Low,     // <5 min
    Medium,  // 5-15 min
    High,    // 15+ min
}

impl AlmostPureAnalyzer {
    pub fn detect_almost_pure(&self, func: &FunctionMetrics)
        -> Option<AlmostPureFunction> {
        // Must have exactly 1-2 violations
        let violation_count = self.count_violations(func);

        if violation_count == 0 || violation_count > 2 {
            return None;
        }

        let violations = self.classify_violations(func);
        let strategy = self.suggest_refactoring(&violations);

        Some(AlmostPureFunction {
            function_id: FunctionId::from_metrics(func),
            violations,
            suggested_strategy: strategy,
            current_multiplier: 1.0,
            potential_multiplier: 0.70,
        })
    }

    fn classify_violations(&self, func: &FunctionMetrics)
        -> Vec<PurityViolation> {
        let mut violations = Vec::new();

        for effect in &func.side_effects {
            match effect {
                SideEffect::Logging { line, macro_name } => {
                    violations.push(PurityViolation::Logging {
                        line: *line,
                        macro_name: macro_name.clone(),
                    });
                }
                SideEffect::TimeQuery { line, method } => {
                    violations.push(PurityViolation::TimeQuery {
                        line: *line,
                        method: method.clone(),
                    });
                }
                SideEffect::RandomGen { line } => {
                    violations.push(PurityViolation::RandomGen { line: *line });
                }
                _ => {}
            }
        }

        violations
    }

    fn suggest_refactoring(&self, violations: &[PurityViolation])
        -> RefactoringStrategy {
        match violations.first() {
            Some(PurityViolation::Logging { line, macro_name }) => {
                RefactoringStrategy {
                    strategy_type: StrategyType::ExtractLogging,
                    effort: Effort::Low,
                    code_before: self.extract_function_code(*line),
                    code_after: self.generate_refactored_logging(*line),
                    explanation: format!(
                        "Move {} to caller. Function becomes pure (0.70x multiplier).",
                        macro_name
                    ),
                }
            }

            Some(PurityViolation::TimeQuery { line, method }) => {
                RefactoringStrategy {
                    strategy_type: StrategyType::ParameterizeTime,
                    effort: Effort::Low,
                    code_before: self.extract_function_code(*line),
                    code_after: self.generate_parameterized_time(*line),
                    explanation: format!(
                        "Pass time as parameter instead of calling {}. \
                         Enables testing and makes function pure.",
                        method
                    ),
                }
            }

            Some(PurityViolation::RandomGen { line }) => {
                RefactoringStrategy {
                    strategy_type: StrategyType::InjectRandomSeed,
                    effort: Effort::Low,
                    code_before: self.extract_function_code(*line),
                    code_after: self.generate_injected_rng(*line),
                    explanation:
                        "Inject RNG as parameter. Enables deterministic testing."
                            .to_string(),
                }
            }

            _ => RefactoringStrategy {
                strategy_type: StrategyType::SeparateIoFromLogic,
                effort: Effort::Medium,
                code_before: String::new(),
                code_after: String::new(),
                explanation:
                    "Extract pure business logic from I/O operations.".to_string(),
            }
        }
    }

    fn generate_refactored_logging(&self, line: usize) -> String {
        // Generate example refactored code
        r#"
// Before (impure, 1.0x multiplier):
fn calculate_total(items: &[Item]) -> f64 {
    let total = items.iter().map(|i| i.price).sum();
    println!("Total: {}", total);  // <-- Violation
    total
}

// After (pure, 0.70x multiplier):
fn calculate_total(items: &[Item]) -> f64 {
    items.iter().map(|i| i.price).sum()
}

// Caller handles logging:
let total = calculate_total(&items);
println!("Total: {}", total);
        "#.to_string()
    }

    fn generate_parameterized_time(&self, line: usize) -> String {
        r#"
// Before (impure):
fn is_expired(deadline: DateTime) -> bool {
    let now = SystemTime::now();  // <-- Violation
    now > deadline
}

// After (pure):
fn is_expired(deadline: DateTime, now: DateTime) -> bool {
    now > deadline
}

// Caller:
let expired = is_expired(deadline, SystemTime::now());
        "#.to_string()
    }
}
```

## Output Format

```
#3 SCORE: 12.4 [MEDIUM] - ALMOST PURE ⭐
├─ LOCATION: src/calculator.rs:45 calculate_total()
├─ COMPLEXITY: cyclomatic=8, cognitive=12
├─ PURITY: Almost Pure (1 violation)
├─ VIOLATION: println! at line 47
└─ RECOMMENDED REFACTORING: Extract Logging [LOW EFFORT - 5 min]

   Current (impure, 1.0x multiplier):
     fn calculate_total(items: &[Item]) -> f64 {
         let total = items.iter().map(|i| i.price).sum();
         println!("Total: {}", total);  // <-- Only violation
         total
     }

   Suggested (pure, 0.70x multiplier):
     fn calculate_total(items: &[Item]) -> f64 {
         items.iter().map(|i| i.price).sum()
     }

     // Caller handles logging:
     let total = calculate_total(&items);
     println!("Total: {}", total);

   IMPACT:
   ├─ Multiplier: 1.0x → 0.70x (30% complexity reduction)
   ├─ Risk: Medium → Low
   ├─ Effort: 5 minutes
   └─ Benefits: Pure function easier to test, no mocking needed
```

## Testing

```rust
#[test]
fn test_detect_single_logging_violation() {
    let code = r#"
        fn calculate(x: i32) -> i32 {
            let result = x * 2;
            println!("{}", result);
            result
        }
    "#;

    let almost_pure = detect_almost_pure(code).unwrap();
    assert_eq!(almost_pure.violations.len(), 1);
    assert!(matches!(
        almost_pure.violations[0],
        PurityViolation::Logging { .. }
    ));
    assert_eq!(almost_pure.suggested_strategy.effort, Effort::Low);
}

#[test]
fn test_two_violations_still_almost_pure() {
    let code = r#"
        fn process(x: i32) -> i32 {
            println!("Processing {}", x);
            let result = x * 2;
            println!("Result: {}", result);
            result
        }
    "#;

    let almost_pure = detect_almost_pure(code).unwrap();
    assert_eq!(almost_pure.violations.len(), 2);
}

#[test]
fn test_three_violations_not_almost_pure() {
    let code = r#"
        fn process(x: i32) -> i32 {
            println!("Start");
            let result = x * 2;
            println!("Middle");
            let final = result + 1;
            println!("End");
            final
        }
    "#;

    assert!(detect_almost_pure(code).is_none());
}
```

## Documentation

Add to user docs:

```markdown
## Almost Pure Functions

Debtmap identifies "almost pure" functions - functions that are one or two
simple changes away from being pure.

These are **easy wins** with high impact:
- Low effort (typically <5 minutes)
- Big benefit (30% complexity reduction via purity multiplier)
- Improved testability

Look for the ⭐ indicator in debtmap output.
```

## Migration

- Add `almost_pure` flag to debt items
- Display prominently in output (special formatting)
- Sort by effort (low-effort refactorings first)
