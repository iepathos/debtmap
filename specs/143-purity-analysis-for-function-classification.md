---
number: 143
title: Purity Analysis for Function Classification
category: foundation
priority: high
status: draft
dependencies: [141]
created: 2025-10-27
---

# Specification 143: Purity Analysis for Function Classification

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 141 (I/O and Side Effect Detection)

## Context

Function **purity** is a critical signal for responsibility classification. A pure function:
- Always returns the same output for the same input
- Has no side effects (no I/O, no mutations, no global state changes)
- Can be safely cached, parallelized, and refactored

Purity analysis builds on Spec 141 (I/O and Side Effect Detection) by classifying functions into a purity spectrum:

- **Strictly Pure**: No I/O, no side effects, deterministic
- **Locally Pure**: Only mutates local variables, returns deterministic results
- **Read-Only**: Reads external state but doesn't modify it
- **Impure**: Performs I/O or modifies external state

This classification is valuable because:
1. Pure functions are ideal candidates for extraction and reuse
2. Impure functions need careful handling in refactoring
3. Purity violations often indicate mixing of concerns
4. Purity enables better test strategies (pure = easy to test)

Purity analysis adds an additional ~5-10% accuracy to responsibility classification and provides actionable refactoring insights.

## Objective

Classify functions on a purity spectrum (strictly pure, locally pure, read-only, impure) using static analysis. Enable responsibility detection to prefer pure computation classification and identify purity violations that indicate mixed concerns.

## Requirements

### Functional Requirements

**Purity Classification**:
- Detect strictly pure functions (no I/O, no side effects, deterministic)
- Detect locally pure functions (only local mutations, deterministic output)
- Detect read-only functions (reads state but doesn't modify)
- Detect impure functions (I/O or external mutations)
- Track purity violations and their locations

**Determinism Analysis**:
- Detect non-deterministic operations (random, time, threading)
- Identify sources of non-determinism in otherwise pure functions
- Track deterministic dependencies (pure calls → pure)

**Purity Propagation**:
- Pure function calling pure function → pure
- Pure function calling impure function → impure
- Track purity through call chains (transitive purity)

**Refactoring Guidance**:
- Identify functions that are "almost pure" (one purity violation)
- Suggest extracting pure portions from impure functions
- Recommend separating I/O from computation

### Non-Functional Requirements

- **Accuracy**: Correctly classify >90% of strictly pure and impure functions
- **Performance**: Purity analysis adds <5% overhead
- **False Positives**: <5% false purity classifications (mislabeling impure as pure)
- **Actionability**: Provide specific purity violation locations for refactoring

## Acceptance Criteria

- [ ] Strictly pure functions are correctly identified (no I/O, no side effects)
- [ ] Locally pure functions are distinguished from strictly pure
- [ ] Non-deterministic operations are detected (random, time, UUID generation)
- [ ] Purity propagates through call chains (pure → pure chain)
- [ ] Purity violations are reported with specific locations
- [ ] "Almost pure" functions are flagged for extraction opportunities
- [ ] Integration with Spec 141: Uses I/O profiles for purity determination
- [ ] Read-only operations are distinguished from mutations
- [ ] Test suite includes debtmap examples (pure parsers, impure I/O)
- [ ] Performance overhead <5% on large files

## Technical Details

### Implementation Approach

**Phase 1: Purity Classification**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PurityLevel {
    /// No I/O, no side effects, deterministic
    StrictlyPure,
    /// Only local mutations, deterministic output
    LocallyPure,
    /// Reads external state, no mutations
    ReadOnly,
    /// Performs I/O or modifies external state
    Impure,
}

#[derive(Debug, Clone)]
pub struct PurityAnalysis {
    pub purity: PurityLevel,
    pub violations: Vec<PurityViolation>,
    pub is_deterministic: bool,
    pub can_be_pure: bool,  // Could be made pure with refactoring
}

#[derive(Debug, Clone)]
pub enum PurityViolation {
    /// I/O operation performed
    IoOperation { kind: IoKind, location: SourceLocation },
    /// External state mutation
    StateMutation { target: String, location: SourceLocation },
    /// Non-deterministic operation
    NonDeterministic { operation: String, location: SourceLocation },
    /// Calls impure function
    ImpureCall { callee: String, location: SourceLocation },
}

pub struct PurityAnalyzer {
    io_analyzer: IoAnalyzer,  // From Spec 141
    call_graph: Option<CallGraph>,  // Optional integration with Spec 142
}

impl PurityAnalyzer {
    pub fn analyze_function(&self, function: &FunctionAst) -> PurityAnalysis {
        let mut violations = Vec::new();

        // Check for I/O operations (from Spec 141)
        let io_profile = self.io_analyzer.analyze_function(function);
        for io_op in io_profile.all_operations() {
            violations.push(PurityViolation::IoOperation {
                kind: io_op.kind(),
                location: io_op.location(),
            });
        }

        // Check for side effects
        for side_effect in io_profile.side_effects {
            if !is_local_mutation(&side_effect, function) {
                violations.push(PurityViolation::StateMutation {
                    target: side_effect.target(),
                    location: side_effect.location(),
                });
            }
        }

        // Check for non-deterministic operations
        for call in function.calls() {
            if is_non_deterministic_operation(&call) {
                violations.push(PurityViolation::NonDeterministic {
                    operation: call.name.clone(),
                    location: call.location,
                });
            }
        }

        // Check for impure function calls
        if let Some(ref call_graph) = self.call_graph {
            for call in function.calls() {
                if let Some(callee_purity) = self.get_callee_purity(&call, call_graph) {
                    if callee_purity != PurityLevel::StrictlyPure {
                        violations.push(PurityViolation::ImpureCall {
                            callee: call.name.clone(),
                            location: call.location,
                        });
                    }
                }
            }
        }

        // Classify purity level
        let purity = self.classify_purity_level(&violations, function);
        let is_deterministic = !violations.iter().any(|v| {
            matches!(v, PurityViolation::NonDeterministic { .. })
        });

        // Check if function can be made pure with refactoring
        let can_be_pure = violations.len() == 1 && can_extract_violation(&violations[0]);

        PurityAnalysis {
            purity,
            violations,
            is_deterministic,
            can_be_pure,
        }
    }

    fn classify_purity_level(
        &self,
        violations: &[PurityViolation],
        function: &FunctionAst,
    ) -> PurityLevel {
        if violations.is_empty() {
            return PurityLevel::StrictlyPure;
        }

        // Check if all violations are local mutations
        let only_local_mutations = violations.iter().all(|v| {
            matches!(v, PurityViolation::StateMutation { .. })
        });

        if only_local_mutations && self.all_mutations_local(violations, function) {
            return PurityLevel::LocallyPure;
        }

        // Check if function only reads state (no writes)
        let only_reads = violations.iter().all(|v| {
            matches!(v, PurityViolation::IoOperation { kind: IoKind::Read, .. })
        });

        if only_reads {
            return PurityLevel::ReadOnly;
        }

        PurityLevel::Impure
    }

    fn all_mutations_local(&self, violations: &[PurityViolation], function: &FunctionAst) -> bool {
        violations.iter().all(|v| {
            if let PurityViolation::StateMutation { target, .. } = v {
                // Check if target is a local variable
                function.local_variables().contains(target)
            } else {
                false
            }
        })
    }
}
```

**Phase 2: Non-Determinism Detection**

```rust
/// Detect non-deterministic operations
fn is_non_deterministic_operation(call: &FunctionCall) -> bool {
    const NON_DETERMINISTIC_PATTERNS: &[&str] = &[
        // Random number generation
        "rand", "random", "Random::new", "thread_rng",
        // Time-based
        "now", "Instant::now", "SystemTime::now", "timestamp", "datetime.now",
        // Threading/concurrency
        "spawn", "thread::spawn", "Arc::new", "Mutex::new",
        // UUIDs
        "Uuid::new", "uuid4", "generate_uuid",
        // Hash with random seed
        "HashMap::new", "HashSet::new",  // Rust's HashMap uses random seed
    ];

    let call_name_lower = call.name.to_lowercase();

    NON_DETERMINISTIC_PATTERNS.iter().any(|pattern| {
        call_name_lower.contains(&pattern.to_lowercase())
    })
}

/// Language-specific non-determinism patterns
pub struct NonDeterminismDetector {
    patterns: HashMap<Language, Vec<String>>,
}

impl NonDeterminismDetector {
    pub fn new() -> Self {
        let mut patterns = HashMap::new();

        // Rust patterns
        patterns.insert(Language::Rust, vec![
            "std::time::Instant::now".into(),
            "std::time::SystemTime::now".into(),
            "rand::".into(),
            "uuid::Uuid::new_v4".into(),
        ]);

        // Python patterns
        patterns.insert(Language::Python, vec![
            "random.".into(),
            "datetime.now".into(),
            "time.time".into(),
            "uuid.uuid4".into(),
        ]);

        // JavaScript patterns
        patterns.insert(Language::JavaScript, vec![
            "Math.random".into(),
            "Date.now".into(),
            "new Date()".into(),
            "crypto.randomUUID".into(),
        ]);

        NonDeterminismDetector { patterns }
    }
}
```

**Phase 3: Purity Propagation**

```rust
impl PurityAnalyzer {
    /// Propagate purity through call graph
    pub fn propagate_purity(
        &self,
        call_graph: &CallGraph,
    ) -> HashMap<FunctionId, PurityLevel> {
        let mut purity_map = HashMap::new();

        // Topological sort: analyze callees before callers
        for function_id in call_graph.reverse_topological_order() {
            let function = call_graph.get_function(function_id);
            let mut analysis = self.analyze_function(function);

            // Check callees' purity
            for callee_id in call_graph.callees(function_id) {
                if let Some(&callee_purity) = purity_map.get(&callee_id) {
                    if callee_purity != PurityLevel::StrictlyPure {
                        analysis.purity = PurityLevel::Impure;
                    }
                }
            }

            purity_map.insert(function_id, analysis.purity);
        }

        purity_map
    }
}
```

**Phase 4: Refactoring Opportunities**

```rust
#[derive(Debug, Clone)]
pub struct PurityRefactoringOpportunity {
    pub function_name: String,
    pub opportunity_type: RefactoringType,
    pub description: String,
    pub estimated_effort: EffortLevel,
}

#[derive(Debug, Clone)]
pub enum RefactoringType {
    /// Extract pure portion from impure function
    ExtractPureCore,
    /// Move I/O to function boundary
    SeparateIoFromLogic,
    /// Replace non-deterministic operation with parameter
    ParameterizeNonDeterminism,
    /// Extract single impure operation
    IsolateSingleViolation,
}

impl PurityAnalyzer {
    pub fn suggest_refactoring(&self, analysis: &PurityAnalysis, function: &FunctionAst) -> Option<PurityRefactoringOpportunity> {
        // Single violation: Easy to extract
        if analysis.violations.len() == 1 {
            return Some(PurityRefactoringOpportunity {
                function_name: function.name.clone(),
                opportunity_type: RefactoringType::IsolateSingleViolation,
                description: format!(
                    "Function has single purity violation: {}. Extract to make core logic pure.",
                    analysis.violations[0]
                ),
                estimated_effort: EffortLevel::Low,
            });
        }

        // All violations are I/O: Separate I/O from logic
        let all_io = analysis.violations.iter().all(|v| {
            matches!(v, PurityViolation::IoOperation { .. })
        });

        if all_io {
            return Some(PurityRefactoringOpportunity {
                function_name: function.name.clone(),
                opportunity_type: RefactoringType::SeparateIoFromLogic,
                description: "Separate I/O operations from business logic. Make computation pure.".into(),
                estimated_effort: EffortLevel::Medium,
            });
        }

        // Non-deterministic: Parameterize
        let has_non_determinism = analysis.violations.iter().any(|v| {
            matches!(v, PurityViolation::NonDeterministic { .. })
        });

        if has_non_determinism {
            return Some(PurityRefactoringOpportunity {
                function_name: function.name.clone(),
                opportunity_type: RefactoringType::ParameterizeNonDeterminism,
                description: "Replace non-deterministic operations (time, random) with parameters for testability.".into(),
                estimated_effort: EffortLevel::Low,
            });
        }

        None
    }
}
```

### Architecture Changes

**New Module**: `src/analysis/purity_analysis.rs`
- Purity classification logic
- Non-determinism detection
- Purity propagation through call graph
- Refactoring opportunity detection

**Integration Point**: `src/organization/god_object_analysis.rs`
- Use purity level as signal for responsibility classification
- Pure functions → "Pure Computation" category
- Almost-pure functions → Flag for refactoring
- Impure functions → Classify by I/O type

**Dependencies**: Reuses Spec 141 (IoAnalyzer) and optionally Spec 142 (CallGraph)

### Data Structures

```rust
#[derive(Debug, Clone)]
pub struct FunctionPurityProfile {
    pub purity: PurityLevel,
    pub is_deterministic: bool,
    pub violations: Vec<PurityViolation>,
    pub can_be_pure: bool,
    pub refactoring_opportunity: Option<PurityRefactoringOpportunity>,
}
```

## Dependencies

- **Prerequisites**: Spec 141 (I/O and Side Effect Detection)
- **Optional Integration**: Spec 142 (Call Graph) for transitive purity
- **Affected Components**:
  - `src/organization/god_object_analysis.rs` - responsibility classification
  - `src/analysis/` - new purity_analysis module

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strictly_pure_function() {
        let code = r#"
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }
        "#;

        let ast = parse_rust(code);
        let analyzer = PurityAnalyzer::new();
        let analysis = analyzer.analyze_function(&ast.functions[0]);

        assert_eq!(analysis.purity, PurityLevel::StrictlyPure);
        assert!(analysis.violations.is_empty());
        assert!(analysis.is_deterministic);
    }

    #[test]
    fn locally_pure_function() {
        let code = r#"
        fn process_items(items: Vec<i32>) -> Vec<i32> {
            let mut result = Vec::new();
            for item in items {
                result.push(item * 2);  // Local mutation
            }
            result
        }
        "#;

        let ast = parse_rust(code);
        let analyzer = PurityAnalyzer::new();
        let analysis = analyzer.analyze_function(&ast.functions[0]);

        assert_eq!(analysis.purity, PurityLevel::LocallyPure);
        assert!(analysis.is_deterministic);
    }

    #[test]
    fn read_only_function() {
        let code = r#"
        fn read_config() -> String {
            std::fs::read_to_string("config.toml").unwrap()
        }
        "#;

        let ast = parse_rust(code);
        let analyzer = PurityAnalyzer::new();
        let analysis = analyzer.analyze_function(&ast.functions[0]);

        assert_eq!(analysis.purity, PurityLevel::ReadOnly);
        assert_eq!(analysis.violations.len(), 1);
        assert!(matches!(
            analysis.violations[0],
            PurityViolation::IoOperation { kind: IoKind::Read, .. }
        ));
    }

    #[test]
    fn impure_function() {
        let code = r#"
        fn save_data(data: &str) {
            std::fs::write("output.txt", data).unwrap();
        }
        "#;

        let ast = parse_rust(code);
        let analyzer = PurityAnalyzer::new();
        let analysis = analyzer.analyze_function(&ast.functions[0]);

        assert_eq!(analysis.purity, PurityLevel::Impure);
        assert!(!analysis.violations.is_empty());
    }

    #[test]
    fn non_deterministic_detection() {
        let code = r#"
        fn generate_id() -> String {
            uuid::Uuid::new_v4().to_string()
        }
        "#;

        let ast = parse_rust(code);
        let analyzer = PurityAnalyzer::new();
        let analysis = analyzer.analyze_function(&ast.functions[0]);

        assert!(!analysis.is_deterministic);
        assert!(analysis.violations.iter().any(|v| {
            matches!(v, PurityViolation::NonDeterministic { .. })
        }));
    }

    #[test]
    fn almost_pure_refactoring_opportunity() {
        let code = r#"
        fn calculate_with_logging(a: i32, b: i32) -> i32 {
            let result = a * b + a / b;
            println!("Result: {}", result);  // Single violation
            result
        }
        "#;

        let ast = parse_rust(code);
        let analyzer = PurityAnalyzer::new();
        let analysis = analyzer.analyze_function(&ast.functions[0]);

        assert!(analysis.can_be_pure);
        let opportunity = analyzer.suggest_refactoring(&analysis, &ast.functions[0]);
        assert!(opportunity.is_some());
    }
}
```

### Integration Tests

```rust
#[test]
fn purity_propagation_through_calls() {
    let code = r#"
    fn pure_helper(x: i32) -> i32 { x * 2 }
    fn impure_helper() { println!("Hello"); }

    fn call_pure() -> i32 { pure_helper(5) }
    fn call_impure() { impure_helper() }
    "#;

    let ast = parse_rust(code);
    let call_graph = CallGraph::from_ast(&ast);
    let analyzer = PurityAnalyzer::with_call_graph(call_graph);

    let purity_map = analyzer.propagate_purity(&call_graph);

    assert_eq!(purity_map[&find_function(&ast, "call_pure")], PurityLevel::StrictlyPure);
    assert_eq!(purity_map[&find_function(&ast, "call_impure")], PurityLevel::Impure);
}
```

## Documentation Requirements

### User Documentation

Update README.md:
```markdown
## Purity Analysis

Debtmap classifies functions by purity level:

**Purity Levels**:
- **Strictly Pure**: No I/O, no side effects, deterministic (ideal for testing)
- **Locally Pure**: Only local mutations, deterministic output
- **Read-Only**: Reads state but doesn't modify
- **Impure**: Performs I/O or modifies external state

**Refactoring Opportunities**:
- "Almost pure" functions flagged for extraction
- Suggests separating I/O from computation
- Identifies non-deterministic operations to parameterize
```

## Implementation Notes

### Handling Language-Specific Purity

Different languages have different purity idioms:

**Rust**:
- Mutable references (`&mut T`) don't break purity if local
- `Result` unwrapping doesn't affect purity
- Interior mutability (`Cell`, `RefCell`) breaks purity

**Python**:
- List comprehensions are pure
- `list.append()` is mutation (impure if non-local)
- Decorators may add hidden side effects

**JavaScript**:
- `const` doesn't guarantee purity (objects are mutable)
- `Array.prototype.map` is pure, `.push` is impure
- Async operations are typically impure

### Performance Optimization

Cache purity analysis results:
```rust
pub struct PurityCache {
    cache: DashMap<FunctionId, PurityAnalysis>,
}
```

## Migration and Compatibility

### Integration with Spec 141

Purity analysis is a direct consumer of I/O detection:
```rust
let io_profile = io_analyzer.analyze_function(function);  // Spec 141
let purity = classify_purity(&io_profile);  // This spec
```

## Expected Impact

### Accuracy Improvement

- **Spec 141 alone**: ~70% accuracy
- **Spec 141 + 143**: ~75% accuracy
- **Improvement**: +5 percentage points

### Refactoring Value

More important than accuracy: Provides actionable refactoring guidance:
- Identifies "almost pure" functions (85% of the benefit with minimal effort)
- Suggests separating I/O from computation
- Enables test strategy optimization (pure = easy to test)

### Examples

```rust
// Almost pure (1 violation)
fn calculate_total(items: &[Item]) -> f64 {
    println!("Calculating...");  // ← Single violation
    items.iter().map(|i| i.price).sum()
}
// Suggestion: Extract pure calculation, move println to caller
```

### Foundation for Multi-Signal (Spec 145)

Purity provides complementary signal to I/O and call graph:
- I/O detection: 40% weight
- Call graph: 30% weight
- Type signatures: 15% weight
- **Purity: 10% weight** ← This spec (implicit in side effects)
- Name heuristics: 5% weight
