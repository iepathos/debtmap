---
number: 162
title: Almost Pure Function Detection and Refactoring Suggestions
category: foundation
priority: high
status: draft
dependencies: [159, 160a, 160b, 160c]
created: 2025-11-01
updated: 2025-11-03
---

# Specification 162: Almost Pure Function Detection and Refactoring Suggestions

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Specs 159 (Evidence-Based Purity Confidence), 160a-c (Macro Analysis)

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

This function is one line away from being pure (0.3x multiplier = 70% complexity reduction), but currently gets 1.0x multiplier.

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
   - Show multiplier change (1.0x → 0.3x)
   - Calculate complexity reduction (70%)
   - Risk level improvement (Impure → Pure)

4. **Recommendation Format**
   - Show current code with violation highlighted
   - Show suggested refactoring
   - Quantify impact and effort

## Implementation

### Core Data Structures

```rust
use crate::analysis::purity_analysis::PurityViolation;

#[derive(Debug, Clone)]
pub struct AlmostPureFunction {
    pub function_id: FunctionId,
    pub violations: Vec<PurityViolation>,
    pub suggested_strategy: RefactoringStrategy,
    pub current_multiplier: f64,  // Always 1.0 (impure)
    pub potential_multiplier: f64, // 0.3 (pure) or 0.5 (probably pure)
}

/// Note: We use the existing PurityViolation from src/analysis/purity_analysis.rs:
///
/// pub enum PurityViolation {
///     IoOperation { description: String, line: Option<usize> },
///     StateMutation { target: String, line: Option<usize> },
///     NonDeterministic { operation: String, line: Option<usize> },
///     ImpureCall { callee: String, line: Option<usize> },
/// }
///
/// Violation classification for almost-pure detection:
/// - Logging: IoOperation with description containing "print", "log", "write"
/// - Time queries: NonDeterministic with operation containing "time", "now", "clock"
/// - Random: NonDeterministic with operation containing "rand", "random"
/// - Mutation: StateMutation (single instance)

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
    /// Detect almost-pure functions from purity analysis results
    pub fn detect_almost_pure(
        &self,
        func: &FunctionMetrics,
        purity_analysis: &PurityAnalysis
    ) -> Option<AlmostPureFunction> {
        // Must have exactly 1-2 violations
        let violation_count = purity_analysis.violations.len();

        if violation_count == 0 || violation_count > 2 {
            return None;
        }

        // Must currently be impure
        if purity_analysis.purity != PurityLevel::Impure {
            return None;
        }

        let violations = purity_analysis.violations.clone();
        let strategy = self.suggest_refactoring(&violations);

        Some(AlmostPureFunction {
            function_id: FunctionId::from_metrics(func),
            violations,
            suggested_strategy: strategy,
            current_multiplier: 1.0,  // Impure
            potential_multiplier: 0.3, // Pure (or 0.5 for probably pure)
        })
    }

    fn suggest_refactoring(&self, violations: &[PurityViolation])
        -> RefactoringStrategy {
        match violations.first() {
            Some(PurityViolation::IoOperation { description, line })
                if Self::is_logging(description) => {
                RefactoringStrategy {
                    strategy_type: StrategyType::ExtractLogging,
                    effort: Effort::Low,
                    code_before: self.extract_function_code(*line),
                    code_after: self.generate_refactored_logging(*line),
                    explanation: format!(
                        "Move logging to caller. Function becomes pure (0.3x multiplier = 70% reduction)."
                    ),
                }
            }

            Some(PurityViolation::NonDeterministic { operation, line })
                if Self::is_time_query(operation) => {
                RefactoringStrategy {
                    strategy_type: StrategyType::ParameterizeTime,
                    effort: Effort::Low,
                    code_before: self.extract_function_code(*line),
                    code_after: self.generate_parameterized_time(*line),
                    explanation: format!(
                        "Pass time as parameter instead of calling {}. \
                         Enables testing and makes function pure (0.3x multiplier).",
                        operation
                    ),
                }
            }

            Some(PurityViolation::NonDeterministic { operation, line })
                if Self::is_random(operation) => {
                RefactoringStrategy {
                    strategy_type: StrategyType::InjectRandomSeed,
                    effort: Effort::Low,
                    code_before: self.extract_function_code(*line),
                    code_after: self.generate_injected_rng(*line),
                    explanation:
                        "Inject RNG as parameter. Enables deterministic testing (0.3x multiplier)."
                            .to_string(),
                }
            }

            Some(PurityViolation::StateMutation { .. }) => {
                RefactoringStrategy {
                    strategy_type: StrategyType::IsolateSingleViolation,
                    effort: Effort::Low,
                    code_before: String::new(),
                    code_after: String::new(),
                    explanation:
                        "Extract mutation to separate function. Core logic becomes pure."
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

    fn is_logging(description: &str) -> bool {
        description.to_lowercase().contains("print")
            || description.to_lowercase().contains("log")
            || description.to_lowercase().contains("write")
    }

    fn is_time_query(operation: &str) -> bool {
        operation.to_lowercase().contains("time")
            || operation.to_lowercase().contains("now")
            || operation.to_lowercase().contains("clock")
    }

    fn is_random(operation: &str) -> bool {
        operation.to_lowercase().contains("rand")
            || operation.to_lowercase().contains("random")
    }

    fn generate_refactored_logging(&self, line: Option<usize>) -> String {
        // Generate example refactored code
        r#"
// Before (impure, 1.0x multiplier):
fn calculate_total(items: &[Item]) -> f64 {
    let total = items.iter().map(|i| i.price).sum();
    println!("Total: {}", total);  // <-- Violation
    total
}

// After (pure, 0.3x multiplier = 70% complexity reduction):
fn calculate_total(items: &[Item]) -> f64 {
    items.iter().map(|i| i.price).sum()
}

// Caller handles logging:
let total = calculate_total(&items);
println!("Total: {}", total);
        "#.to_string()
    }

    fn generate_parameterized_time(&self, line: Option<usize>) -> String {
        r#"
// Before (impure, 1.0x multiplier):
fn is_expired(deadline: DateTime) -> bool {
    let now = SystemTime::now();  // <-- Violation
    now > deadline
}

// After (pure, 0.3x multiplier):
fn is_expired(deadline: DateTime, now: DateTime) -> bool {
    now > deadline
}

// Caller:
let expired = is_expired(deadline, SystemTime::now());
        "#.to_string()
    }

    fn generate_injected_rng(&self, line: Option<usize>) -> String {
        r#"
// Before (impure, 1.0x multiplier):
fn shuffle_items(items: &mut Vec<Item>) {
    items.shuffle(&mut rand::thread_rng());  // <-- Violation
}

// After (pure, 0.3x multiplier):
fn shuffle_items(items: &mut Vec<Item>, rng: &mut impl Rng) {
    items.shuffle(rng);
}

// Caller:
shuffle_items(&mut items, &mut rand::thread_rng());
        "#.to_string()
    }

    fn extract_function_code(&self, line: Option<usize>) -> String {
        // TODO: Extract actual function source code using AST mapping
        // For now, return placeholder
        format!("// Function at line {:?}", line)
    }
}
```

## Output Format

```
#3 SCORE: 12.4 [MEDIUM] - ALMOST PURE ⭐
├─ LOCATION: src/calculator.rs:45 calculate_total()
├─ COMPLEXITY: cyclomatic=8, cognitive=12
├─ PURITY: Almost Pure (1 violation)
├─ VIOLATION: I/O operation (println!) at line 47
└─ RECOMMENDED REFACTORING: Extract Logging [LOW EFFORT - 5 min]

   Current (impure, 1.0x multiplier):
     fn calculate_total(items: &[Item]) -> f64 {
         let total = items.iter().map(|i| i.price).sum();
         println!("Total: {}", total);  // <-- Only violation
         total
     }

   Suggested (pure, 0.3x multiplier):
     fn calculate_total(items: &[Item]) -> f64 {
         items.iter().map(|i| i.price).sum()
     }

     // Caller handles logging:
     let total = calculate_total(&items);
     println!("Total: {}", total);

   IMPACT:
   ├─ Multiplier: 1.0x → 0.3x (70% complexity reduction)
   ├─ Purity: Impure → Pure
   ├─ Effort: 5 minutes
   └─ Benefits: Pure function easier to test, no mocking needed
```

## Testing

```rust
use crate::analysis::purity_analysis::{PurityAnalysis, PurityLevel, PurityViolation};
use crate::analysis::almost_pure::{AlmostPureAnalyzer, EffortLevel};

#[test]
fn test_detect_single_logging_violation() {
    let analyzer = AlmostPureAnalyzer::new();

    // Simulate purity analysis result with single I/O violation
    let purity = PurityAnalysis {
        purity: PurityLevel::Impure,
        violations: vec![
            PurityViolation::IoOperation {
                description: "println! macro".to_string(),
                line: Some(47),
            }
        ],
        is_deterministic: true,
        can_be_pure: true,
        refactoring_opportunity: None,
    };

    let func = create_test_function_metrics("calculate");
    let almost_pure = analyzer.detect_almost_pure(&func, &purity).unwrap();

    assert_eq!(almost_pure.violations.len(), 1);
    assert!(matches!(
        almost_pure.violations[0],
        PurityViolation::IoOperation { .. }
    ));
    assert_eq!(almost_pure.suggested_strategy.effort, EffortLevel::Low);
    assert_eq!(almost_pure.current_multiplier, 1.0);
    assert_eq!(almost_pure.potential_multiplier, 0.3);
}

#[test]
fn test_two_violations_still_almost_pure() {
    let analyzer = AlmostPureAnalyzer::new();

    let purity = PurityAnalysis {
        purity: PurityLevel::Impure,
        violations: vec![
            PurityViolation::IoOperation {
                description: "println! macro".to_string(),
                line: Some(10),
            },
            PurityViolation::IoOperation {
                description: "println! macro".to_string(),
                line: Some(12),
            },
        ],
        is_deterministic: true,
        can_be_pure: true,
        refactoring_opportunity: None,
    };

    let func = create_test_function_metrics("process");
    let almost_pure = analyzer.detect_almost_pure(&func, &purity).unwrap();
    assert_eq!(almost_pure.violations.len(), 2);
}

#[test]
fn test_three_violations_not_almost_pure() {
    let analyzer = AlmostPureAnalyzer::new();

    let purity = PurityAnalysis {
        purity: PurityLevel::Impure,
        violations: vec![
            PurityViolation::IoOperation {
                description: "println!".to_string(),
                line: Some(10),
            },
            PurityViolation::IoOperation {
                description: "println!".to_string(),
                line: Some(12),
            },
            PurityViolation::IoOperation {
                description: "println!".to_string(),
                line: Some(14),
            },
        ],
        is_deterministic: true,
        can_be_pure: false,
        refactoring_opportunity: None,
    };

    let func = create_test_function_metrics("process");
    assert!(analyzer.detect_almost_pure(&func, &purity).is_none());
}

#[test]
fn test_pure_function_not_almost_pure() {
    let analyzer = AlmostPureAnalyzer::new();

    // Already pure - no refactoring needed
    let purity = PurityAnalysis {
        purity: PurityLevel::StrictlyPure,
        violations: vec![],
        is_deterministic: true,
        can_be_pure: false,
        refactoring_opportunity: None,
    };

    let func = create_test_function_metrics("pure_func");
    assert!(analyzer.detect_almost_pure(&func, &purity).is_none());
}

#[test]
fn test_time_query_violation() {
    let analyzer = AlmostPureAnalyzer::new();

    let purity = PurityAnalysis {
        purity: PurityLevel::Impure,
        violations: vec![
            PurityViolation::NonDeterministic {
                operation: "SystemTime::now()".to_string(),
                line: Some(20),
            }
        ],
        is_deterministic: false,
        can_be_pure: true,
        refactoring_opportunity: None,
    };

    let func = create_test_function_metrics("is_expired");
    let almost_pure = analyzer.detect_almost_pure(&func, &purity).unwrap();

    assert!(matches!(
        almost_pure.suggested_strategy.strategy_type,
        StrategyType::ParameterizeTime
    ));
    assert_eq!(almost_pure.suggested_strategy.effort, EffortLevel::Low);
}

// Helper function for tests
fn create_test_function_metrics(name: &str) -> FunctionMetrics {
    FunctionMetrics {
        name: name.to_string(),
        file: PathBuf::from("test.rs"),
        line: 10,
        cyclomatic: 3,
        cognitive: 5,
        nesting: 1,
        length: 10,
        is_test: false,
        visibility: Some("pub".to_string()),
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: Some(false),
        purity_confidence: Some(0.9),
        purity_reason: None,
        call_dependencies: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
        language_specific: None,
        purity_level: Some(PurityLevel::Impure),
        almost_pure_refactoring: None,
    }
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
- Big benefit (70% complexity reduction via purity multiplier 1.0x → 0.3x)
- Improved testability

Look for the ⭐ indicator in debtmap output.
```

## Integration with Existing Code

### Analysis Pipeline

Almost-pure detection runs as a **post-processing step** after standard purity analysis:

```rust
// In src/analyzers/rust_analyzer.rs (or new analysis/almost_pure.rs)

use crate::analysis::purity_analysis::{PurityAnalyzer, PurityLevel, PurityViolation};
use crate::core::{FunctionMetrics, FunctionId};

pub struct AlmostPureAnalyzer {
    // Configuration and state
}

impl AlmostPureAnalyzer {
    pub fn analyze_functions(
        &self,
        metrics: &[FunctionMetrics],
        purity_results: &HashMap<FunctionId, PurityAnalysis>
    ) -> Vec<AlmostPureFunction> {
        metrics
            .iter()
            .filter_map(|func| {
                let func_id = FunctionId::from_metrics(func);
                purity_results
                    .get(&func_id)
                    .and_then(|purity| self.detect_almost_pure(func, purity))
            })
            .collect()
    }
}
```

### Data Storage

Add new field to `FunctionMetrics` in `src/core/mod.rs`:

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FunctionMetrics {
    // ... existing fields ...

    /// Almost-pure refactoring opportunity (Spec 162)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub almost_pure_refactoring: Option<AlmostPureRefactoring>,
}

/// Refactoring suggestion for almost-pure functions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlmostPureRefactoring {
    pub violation_count: usize,
    pub strategy_type: String,  // "ExtractLogging", "ParameterizeTime", etc.
    pub effort: String,          // "Low", "Medium", "High"
    pub current_multiplier: f64, // 1.0
    pub potential_multiplier: f64, // 0.3
    pub explanation: String,
}
```

### Output Integration

Extend existing console output in `src/output/console.rs`:

```rust
// Add to function output rendering
if let Some(refactoring) = &func.almost_pure_refactoring {
    println!("   ⭐ ALMOST PURE: {} ({} effort)",
        refactoring.strategy_type,
        refactoring.effort);
    println!("      Impact: {}x → {}x ({:.0}% reduction)",
        refactoring.current_multiplier,
        refactoring.potential_multiplier,
        (1.0 - refactoring.potential_multiplier / refactoring.current_multiplier) * 100.0);
    println!("      {}", refactoring.explanation);
}
```

### Execution Flow

1. **Standard Analysis** (existing):
   ```
   RustAnalyzer::analyze_file()
     └─> PurityDetector::is_pure_function()
         └─> Returns PurityAnalysis with violations
   ```

2. **Almost-Pure Detection** (new):
   ```
   AlmostPureAnalyzer::analyze_functions()
     ├─> For each function with purity analysis
     ├─> Check if 1-2 violations and impure
     └─> Generate refactoring suggestion
   ```

3. **Result Storage**:
   ```
   FunctionMetrics.almost_pure_refactoring = Some(refactoring)
   ```

4. **Output Rendering**:
   ```
   Console/JSON output includes ⭐ indicator and suggestions
   ```

### Integration Points

| Component | Location | Modification |
|-----------|----------|--------------|
| Core types | `src/core/mod.rs` | Add `AlmostPureRefactoring` struct |
| Analysis | `src/analysis/almost_pure.rs` (new) | Implement `AlmostPureAnalyzer` |
| Rust analyzer | `src/analyzers/rust_analyzer.rs` | Call analyzer after purity detection |
| Console output | `src/output/console.rs` | Render ⭐ indicator and suggestions |
| JSON output | `src/output/unified.rs` | Include in JSON schema |

### PurityLevel Compatibility

This spec uses `PurityLevel` from `src/analysis/purity_analysis.rs`:

```rust
pub enum PurityLevel {
    StrictlyPure,  // 0.3x multiplier (target)
    LocallyPure,   // 0.5x multiplier (possible target)
    ReadOnly,      // Not applicable for almost-pure
    Impure,        // 1.0x multiplier (current state)
}
```

**Note**: The organization module's `PurityLevel` (Pure/ProbablyPure/Impure) is used for god object detection and has different multipliers. This spec targets the analysis module's more granular classification.

### Dependencies on Other Specs

- **Spec 159**: Confidence scoring affects whether to show refactoring suggestion
  - Only show if confidence > 0.8 (high confidence in classification)
- **Spec 160a-c**: Macro classification improves violation detection
  - Better identification of logging vs other I/O
  - Reduces false positives

### Performance Considerations

- **Overhead**: ~1-5ms per analyzed function (pattern matching on violations)
- **Optimization**: Only analyze functions flagged as impure with 1-2 violations
- **Lazy evaluation**: Generate code examples only when requested (verbose output)

## Migration

- Add `almost_pure_refactoring` field to `FunctionMetrics` (optional, backward compatible)
- Display prominently in output (⭐ indicator with special formatting)
- Sort by effort (low-effort refactorings first in reports)
