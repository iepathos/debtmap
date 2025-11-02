---
number: 151
title: Purity and Framework Pattern Output Indicators
category: optimization
priority: medium
status: draft
dependencies: [143, 144, 146]
created: 2025-10-28
---

# Specification 151: Purity and Framework Pattern Output Indicators

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Specs 143 (Purity Analysis), 144 (Framework Patterns), 146 (Rust Patterns)

## Context

Specs 143, 144, and 146 provide valuable analysis signals:
- **Spec 143 (Purity)**: Classifies functions as pure, locally pure, read-only, or impure
- **Spec 144 (Frameworks)**: Detects framework-specific patterns (Axum, pytest, Express, etc.)
- **Spec 146 (Rust Patterns)**: Identifies Rust traits, async patterns, error handling

However, **none of this information appears in the output**. The latest debtmap output shows:

```
#8 SCORE: 58.2 [CRITICAL - FILE - GOD OBJECT]
└─ ./src/analyzers/rust.rs (2034 lines, 124 functions)
└─ ACTION: Split by data flow...
  - RECOMMENDED SPLITS (3 modules):
  -  [M] rust_utilities.rs - Utilities (15 methods)
  -  [M] rust_construction.rs - Construction (7 methods)
  -  [M] rust_computation.rs - Computation (6 methods)

  - Rust PATTERNS:
  -  - Extract traits for shared behavior
  -  - Use newtype pattern for domain types
  -  - Consider builder pattern for complex construction
```

**Missing Information**:
1. **Purity indicators**: Which functions are pure? Which are "almost pure" and could be extracted?
2. **Framework patterns**: Are any functions Axum handlers, test functions, CLI parsers?
3. **Rust-specific traits**: Which trait implementations exist? (Display, From, Iterator, etc.)
4. **Async patterns**: Which functions use async/await, tokio::spawn, channels?
5. **Refactoring opportunities**: Functions that could be made pure with minimal changes

**Current "Rust PATTERNS" section is generic advice**, not analysis of actual code patterns detected.

## Objective

Add purity, framework, and Rust-specific pattern indicators to debtmap output, showing users:
1. Which functions are pure (ideal for extraction and testing)
2. Framework patterns detected (HTTP handlers, tests, CLI parsers)
3. Rust trait implementations and async patterns
4. Refactoring opportunities (almost-pure functions, builder candidates)
5. Pattern-specific actionable recommendations

This will provide actionable refactoring guidance beyond generic "split by responsibility" advice.

## Requirements

### Functional Requirements

**Purity Indicators**:
- Show purity level for each module/split (% strictly pure, % locally pure, % impure)
- Highlight "almost pure" functions (1-2 purity violations) as extraction opportunities
- Show specific purity violations (I/O operation, global state mutation, etc.)
- Suggest refactoring to separate pure from impure code

**Framework Pattern Indicators**:
- Detect and display framework patterns (Axum, Actix, pytest, Jest, etc.)
- Show framework-specific refactoring advice
- Group functions by framework role (handlers, fixtures, middleware)
- Suggest framework-appropriate architectural patterns

**Rust Pattern Indicators**:
- List detected trait implementations (Display, From, Iterator, etc.)
- Show async/concurrency patterns (async fn, tokio::spawn, channels)
- Identify error handling patterns (? operator density, custom errors)
- Display builder pattern candidates (chainable methods, finalize methods)

**Refactoring Opportunities**:
- Flag functions that are almost pure (can be made pure with small changes)
- Identify functions that should use macros (repetitive trait implementations)
- Suggest extracting pure portions from impure functions
- Recommend parameterizing non-deterministic operations

**Output Integration**:
- Add pattern analysis section to each recommendation
- Show pattern counts per module split
- Provide pattern-specific actionable advice
- Link patterns to refactoring strategies

### Non-Functional Requirements

- **Performance**: Pattern detection already done in classification, minimal overhead for display
- **Clarity**: Pattern indicators easy to understand for users
- **Actionability**: Each indicator includes specific next steps
- **Consistency**: Same format across all pattern types

## Acceptance Criteria

### Data Structures and Separation of Concerns
- [ ] `PatternAnalysis` container holds all pattern data (purity, frameworks, Rust patterns)
- [ ] All data structures implement `Serialize`/`Deserialize` for potential JSON output
- [ ] All data structures provide `Default` implementations
- [ ] Pure calculation methods separated from formatting (e.g., `purity_percentage()`)
- [ ] All constants extracted and named (e.g., `REPETITIVE_TRAIT_THRESHOLD`)

### Pattern Detection and Aggregation
- [ ] Purity metrics aggregated correctly from function analyses
- [ ] Framework patterns aggregated and sorted by frequency
- [ ] Rust patterns aggregated (traits, async, error handling)
- [ ] "Almost pure" functions identified (1-2 violations only)
- [ ] Aggregation functions are pure (data in, metrics out)

### Formatting Layer
- [ ] `PatternFormatter` contains all formatting logic (no formatting in data structures)
- [ ] Empty metrics produce no output (not "No patterns detected" messages)
- [ ] Output format matches specification examples
- [ ] Unicode in function names handled correctly
- [ ] Empty example lists don't produce malformed output (e.g., "Examples: ,")

### Integration
- [ ] `PatternAnalysis` attached to recommendations as optional field
- [ ] Integration uses simple builder pattern: `recommendation.with_pattern_analysis()`
- [ ] Formatter checks for pattern presence before formatting
- [ ] Generic "Rust PATTERNS" advice completely removed

### Testing
- [ ] Unit tests for pure calculations (purity percentage, total functions, etc.)
- [ ] Property-based tests using `proptest` (percentage always 0.0-1.0, etc.)
- [ ] Formatter tests validate output structure and content
- [ ] Edge case tests (zero functions, Unicode, empty examples)
- [ ] Integration tests verify end-to-end pattern flow
- [ ] Tests verify generic advice is NOT in output

### Performance
- [ ] No premature optimization (no `OnceCell` or lazy evaluation)
- [ ] Aggregation is O(n) over functions (single pass where possible)
- [ ] No redundant analysis (leverages existing Specs 143, 144, 146)

### Documentation
- [ ] User documentation explains how to interpret each pattern type
- [ ] Code examples show before/after output
- [ ] API documentation for public types and methods
- [ ] Configuration options documented (if added)

## Technical Details

### Implementation Approach

**Phase 1: Core Data Structures (Pure Data)**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurityMetrics {
    pub strictly_pure: usize,
    pub locally_pure: usize,
    pub read_only: usize,
    pub impure: usize,
    pub almost_pure: Vec<AlmostPureFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlmostPureFunction {
    pub name: String,
    pub violations: Vec<PurityViolation>,
    pub refactoring_suggestion: String,
}

impl PurityMetrics {
    /// Pure calculation - no formatting or side effects
    pub fn total_functions(&self) -> usize {
        self.strictly_pure + self.locally_pure + self.read_only + self.impure
    }

    /// Returns purity percentage as a value between 0.0 and 1.0
    pub fn purity_percentage(&self) -> f64 {
        let total = self.total_functions();
        if total == 0 {
            0.0
        } else {
            (self.strictly_pure + self.locally_pure) as f64 / total as f64
        }
    }

    /// Returns the top N almost-pure functions by refactoring impact
    pub fn top_refactoring_opportunities(&self, limit: usize) -> &[AlmostPureFunction] {
        let end = self.almost_pure.len().min(limit);
        &self.almost_pure[..end]
    }
}

impl Default for PurityMetrics {
    fn default() -> Self {
        Self {
            strictly_pure: 0,
            locally_pure: 0,
            read_only: 0,
            impure: 0,
            almost_pure: vec![],
        }
    }
}
```

**Phase 2: Framework Pattern Data Structures**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkPatternMetrics {
    pub patterns: Vec<DetectedPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPattern {
    pub framework: String,
    pub pattern_type: String,
    pub count: usize,
    pub examples: Vec<String>,
    pub recommendation: String,
}

impl FrameworkPatternMetrics {
    /// Returns patterns sorted by count (most frequent first)
    pub fn sorted_by_frequency(&self) -> Vec<&DetectedPattern> {
        let mut sorted: Vec<_> = self.patterns.iter().collect();
        sorted.sort_by(|a, b| b.count.cmp(&a.count));
        sorted
    }

    /// Returns true if any framework patterns were detected
    pub fn has_patterns(&self) -> bool {
        !self.patterns.is_empty()
    }
}

impl Default for FrameworkPatternMetrics {
    fn default() -> Self {
        Self { patterns: vec![] }
    }
}

/// Aggregates framework patterns from function analyses (Spec 144)
/// This assumes pattern detection already happened during classification
fn aggregate_framework_patterns(functions: &[FunctionAnalysis]) -> FrameworkPatternMetrics {
    use std::collections::HashMap;

    let grouped: HashMap<(String, String), Vec<String>> = functions
        .iter()
        .filter_map(|f| f.framework_pattern.as_ref().map(|p| (f, p)))
        .fold(HashMap::new(), |mut acc, (func, pattern)| {
            let key = (pattern.framework.clone(), pattern.pattern_type.clone());
            acc.entry(key)
                .or_insert_with(Vec::new)
                .push(func.name.clone());
            acc
        });

    let patterns = grouped
        .into_iter()
        .map(|((framework, pattern_type), examples)| {
            let count = examples.len();
            let recommendation = generate_framework_recommendation(&framework, &pattern_type);
            DetectedPattern {
                framework,
                pattern_type,
                count,
                examples: examples.into_iter().take(3).collect(),
                recommendation,
            }
        })
        .collect();

    FrameworkPatternMetrics { patterns }
}

/// Pure function to generate framework-specific recommendations
fn generate_framework_recommendation(framework: &str, pattern_type: &str) -> String {
    match (framework, pattern_type) {
        ("Axum", "HTTP Request Handlers") => {
            "Consider grouping handlers by resource (users/, posts/, etc.)".into()
        }
        ("Rust Testing", "Test Functions") => {
            "Tests already well-organized with #[test] attributes".into()
        }
        _ => format!("Review {} {} organization", framework, pattern_type),
    }
}
```

**Phase 3: Rust Pattern Data Structures**

```rust
// Constants for thresholds (configurable via settings)
const REPETITIVE_TRAIT_THRESHOLD: usize = 5;
const MAX_DISPLAYED_EXAMPLES: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustPatternMetrics {
    pub trait_impls: Vec<TraitImplementation>,
    pub async_patterns: AsyncPatternSummary,
    pub error_handling: ErrorHandlingSummary,
    pub builder_candidates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitImplementation {
    pub trait_name: String,
    pub count: usize,
    pub types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncPatternSummary {
    pub async_functions: usize,
    pub spawn_calls: usize,
    pub channel_usage: bool,
    pub mutex_usage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorHandlingSummary {
    pub question_mark_density: f64,  // Avg ? operators per function
    pub custom_error_types: Vec<String>,
    pub unwrap_count: usize,  // Anti-pattern
}

impl RustPatternMetrics {
    /// Returns trait implementations that appear repetitively
    pub fn repetitive_traits(&self) -> Vec<&TraitImplementation> {
        self.trait_impls
            .iter()
            .filter(|t| t.count >= REPETITIVE_TRAIT_THRESHOLD)
            .collect()
    }

    /// Returns true if async patterns are present
    pub fn has_async_patterns(&self) -> bool {
        self.async_patterns.async_functions > 0
    }

    /// Returns true if error handling patterns are present
    pub fn has_error_handling(&self) -> bool {
        self.error_handling.question_mark_density > 0.0
    }

    /// Returns true if builder patterns are present
    pub fn has_builder_candidates(&self) -> bool {
        !self.builder_candidates.is_empty()
    }
}

impl Default for RustPatternMetrics {
    fn default() -> Self {
        Self {
            trait_impls: vec![],
            async_patterns: AsyncPatternSummary::default(),
            error_handling: ErrorHandlingSummary::default(),
            builder_candidates: vec![],
        }
    }
}

impl Default for AsyncPatternSummary {
    fn default() -> Self {
        Self {
            async_functions: 0,
            spawn_calls: 0,
            channel_usage: false,
            mutex_usage: false,
        }
    }
}

impl Default for ErrorHandlingSummary {
    fn default() -> Self {
        Self {
            question_mark_density: 0.0,
            custom_error_types: vec![],
            unwrap_count: 0,
        }
    }
}
```

**Phase 4: Pattern Analysis Container**

```rust
/// Container for all pattern analysis results
/// Attached to recommendations for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternAnalysis {
    pub purity: PurityMetrics,
    pub frameworks: FrameworkPatternMetrics,
    pub rust_patterns: RustPatternMetrics,
}

impl PatternAnalysis {
    /// Creates pattern analysis from a collection of function analyses
    pub fn from_functions(functions: &[FunctionAnalysis]) -> Self {
        Self {
            purity: aggregate_purity_metrics(functions),
            frameworks: aggregate_framework_patterns(functions),
            rust_patterns: aggregate_rust_patterns(functions),
        }
    }

    /// Returns true if any patterns were detected
    pub fn has_patterns(&self) -> bool {
        self.purity.total_functions() > 0
            || self.frameworks.has_patterns()
            || self.rust_patterns.has_async_patterns()
            || self.rust_patterns.has_error_handling()
    }
}

impl Default for PatternAnalysis {
    fn default() -> Self {
        Self {
            purity: PurityMetrics::default(),
            frameworks: FrameworkPatternMetrics::default(),
            rust_patterns: RustPatternMetrics::default(),
        }
    }
}

/// Aggregates purity metrics from function analyses (Spec 143)
fn aggregate_purity_metrics(functions: &[FunctionAnalysis]) -> PurityMetrics {
    let mut strictly_pure = 0;
    let mut locally_pure = 0;
    let mut read_only = 0;
    let mut impure = 0;
    let mut almost_pure = Vec::new();

    for func in functions {
        match func.purity_classification {
            PurityLevel::StrictlyPure => strictly_pure += 1,
            PurityLevel::LocallyPure => locally_pure += 1,
            PurityLevel::ReadOnly => read_only += 1,
            PurityLevel::Impure => impure += 1,
        }

        // Identify "almost pure" functions (1-2 violations)
        if func.purity_violations.len() <= 2 && func.purity_violations.len() > 0 {
            almost_pure.push(AlmostPureFunction {
                name: func.name.clone(),
                violations: func.purity_violations.clone(),
                refactoring_suggestion: suggest_purity_refactoring(&func.purity_violations),
            });
        }
    }

    PurityMetrics {
        strictly_pure,
        locally_pure,
        read_only,
        impure,
        almost_pure,
    }
}

/// Pure function to generate purity refactoring suggestions
fn suggest_purity_refactoring(violations: &[PurityViolation]) -> String {
    if violations.is_empty() {
        return "Already pure".into();
    }

    match &violations[0] {
        PurityViolation::ConsoleOutput { .. } => {
            "Extract println!/eprintln! to caller, make core logic pure".into()
        }
        PurityViolation::FileIO { .. } => {
            "Separate file I/O from computation - pass data as parameters".into()
        }
        PurityViolation::GlobalStateMutation { .. } => {
            "Return new state instead of mutating global state".into()
        }
        PurityViolation::NonDeterministic { .. } => {
            "Inject time/random source as parameter for testability".into()
        }
        _ => "Separate side effects from pure computation".into(),
    }
}

/// Aggregates Rust pattern metrics from function analyses (Spec 146)
fn aggregate_rust_patterns(functions: &[FunctionAnalysis]) -> RustPatternMetrics {
    use std::collections::HashMap;

    // Aggregate trait implementations
    let mut trait_counts: HashMap<String, Vec<String>> = HashMap::new();
    for func in functions {
        if let Some(trait_impl) = &func.trait_implementation {
            trait_counts
                .entry(trait_impl.trait_name.clone())
                .or_insert_with(Vec::new)
                .push(trait_impl.type_name.clone());
        }
    }

    let trait_impls = trait_counts
        .into_iter()
        .map(|(trait_name, types)| TraitImplementation {
            trait_name,
            count: types.len(),
            types: types.into_iter().take(MAX_DISPLAYED_EXAMPLES).collect(),
        })
        .collect();

    // Aggregate async patterns
    let async_functions = functions.iter().filter(|f| f.is_async).count();
    let spawn_calls = functions
        .iter()
        .map(|f| f.spawn_call_count.unwrap_or(0))
        .sum();
    let channel_usage = functions.iter().any(|f| f.uses_channels);
    let mutex_usage = functions.iter().any(|f| f.uses_mutex);

    // Aggregate error handling
    let total_question_marks: usize = functions
        .iter()
        .map(|f| f.question_mark_count.unwrap_or(0))
        .sum();
    let question_mark_density = if functions.is_empty() {
        0.0
    } else {
        total_question_marks as f64 / functions.len() as f64
    };
    let unwrap_count = functions.iter().map(|f| f.unwrap_count.unwrap_or(0)).sum();

    // Identify builder candidates
    let builder_candidates = functions
        .iter()
        .filter(|f| f.is_builder_candidate)
        .map(|f| f.name.clone())
        .collect();

    RustPatternMetrics {
        trait_impls,
        async_patterns: AsyncPatternSummary {
            async_functions,
            spawn_calls,
            channel_usage,
            mutex_usage,
        },
        error_handling: ErrorHandlingSummary {
            question_mark_density,
            custom_error_types: vec![], // TODO: Extract from analysis
            unwrap_count,
        },
        builder_candidates,
    }
}
```

**Phase 5: Formatting Layer (Separate from Data)**

```rust
/// Formatter for pattern analysis - pure display logic only
pub struct PatternFormatter;

impl PatternFormatter {
    /// Formats complete pattern analysis for output
    pub fn format(analysis: &PatternAnalysis) -> String {
        if !analysis.has_patterns() {
            return String::new();
        }

        [
            Self::format_purity(&analysis.purity),
            Self::format_frameworks(&analysis.frameworks),
            Self::format_rust_patterns(&analysis.rust_patterns),
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
    }

    fn format_purity(metrics: &PurityMetrics) -> String {
        if metrics.total_functions() == 0 {
            return String::new();
        }

        let mut sections = vec![
            format!(
                "PURITY ANALYSIS:\n\
                 - Strictly Pure: {} functions ({:.0}%)\n\
                 - Locally Pure: {} functions\n\
                 - Read-Only: {} functions\n\
                 - Impure: {} functions",
                metrics.strictly_pure,
                metrics.purity_percentage() * 100.0,
                metrics.locally_pure,
                metrics.read_only,
                metrics.impure
            ),
        ];

        let opportunities = Self::format_refactoring_opportunities(metrics);
        if !opportunities.is_empty() {
            sections.push(opportunities);
        }

        sections.join("\n\n")
    }

    fn format_refactoring_opportunities(metrics: &PurityMetrics) -> String {
        let opportunities = metrics.top_refactoring_opportunities(5);
        if opportunities.is_empty() {
            return String::new();
        }

        let mut output = String::from("REFACTORING OPPORTUNITIES:");
        for func in opportunities {
            output.push_str(&format!(
                "\n  - {} ({} violation{}): {}\n    → Suggestion: {}",
                func.name,
                func.violations.len(),
                if func.violations.len() == 1 { "" } else { "s" },
                func.violations[0],
                func.refactoring_suggestion
            ));
        }
        output
    }

    fn format_frameworks(metrics: &FrameworkPatternMetrics) -> String {
        if !metrics.has_patterns() {
            return String::new();
        }

        let mut output = String::from("FRAMEWORK PATTERNS DETECTED:");
        for pattern in metrics.sorted_by_frequency() {
            output.push_str(&format!(
                "\n  - {} {} ({}x detected)\n    Examples: {}\n    Recommendation: {}",
                pattern.framework,
                pattern.pattern_type,
                pattern.count,
                pattern.examples.join(", "),
                pattern.recommendation
            ));
        }
        output
    }

    fn format_rust_patterns(metrics: &RustPatternMetrics) -> String {
        let sections = [
            Self::format_trait_implementations(metrics),
            Self::format_async_patterns(metrics),
            Self::format_error_handling(metrics),
            Self::format_builder_candidates(metrics),
        ];

        let non_empty: Vec<_> = sections.into_iter().filter(|s| !s.is_empty()).collect();

        if non_empty.is_empty() {
            return String::new();
        }

        format!("RUST-SPECIFIC PATTERNS:\n{}", non_empty.join("\n"))
    }

    fn format_trait_implementations(metrics: &RustPatternMetrics) -> String {
        if metrics.trait_impls.is_empty() {
            return String::new();
        }

        let mut output = String::from("  Trait Implementations:");
        for trait_impl in &metrics.trait_impls {
            output.push_str(&format!(
                "\n    - {}: {} implementations ({})",
                trait_impl.trait_name,
                trait_impl.count,
                trait_impl.types.join(", ")
            ));
        }

        let repetitive = metrics.repetitive_traits();
        if !repetitive.is_empty() {
            output.push_str("\n    → Consider using macros for repetitive implementations");
        }

        output
    }

    fn format_async_patterns(metrics: &RustPatternMetrics) -> String {
        if !metrics.has_async_patterns() {
            return String::new();
        }

        let async_pat = &metrics.async_patterns;
        let mut output = format!(
            "  Async/Concurrency:\n\
               - Async functions: {}\n\
               - Spawn calls: {}\n\
               - Channels: {}\n\
               - Mutex usage: {}",
            async_pat.async_functions,
            async_pat.spawn_calls,
            if async_pat.channel_usage { "Yes" } else { "No" },
            if async_pat.mutex_usage { "Yes" } else { "No" }
        );

        if async_pat.spawn_calls > 0 {
            output.push_str("\n    → Concurrency management detected - group spawn logic");
        }

        output
    }

    fn format_error_handling(metrics: &RustPatternMetrics) -> String {
        if !metrics.has_error_handling() {
            return String::new();
        }

        let mut output = format!(
            "  Error Handling:\n    - Average ? operators per function: {:.1}",
            metrics.error_handling.question_mark_density
        );

        if metrics.error_handling.unwrap_count > 0 {
            output.push_str(&format!(
                "\n    ⚠ {} unwrap() calls detected - replace with proper error handling",
                metrics.error_handling.unwrap_count
            ));
        }

        output
    }

    fn format_builder_candidates(metrics: &RustPatternMetrics) -> String {
        if !metrics.has_builder_candidates() {
            return String::new();
        }

        format!(
            "  Builder Patterns:\n\
               - Candidates: {}\n\
               → Extract builder logic into separate module",
            metrics.builder_candidates.join(", ")
        )
    }
}
```

**Phase 6: Integration with Recommendations**

```rust
// In src/organization/god_object_detector.rs or similar

/// Attach pattern analysis to recommendations
pub struct ModuleSplitRecommendation {
    // ... existing fields ...
    pub pattern_analysis: Option<PatternAnalysis>,
}

impl ModuleSplitRecommendation {
    pub fn with_pattern_analysis(
        mut self,
        functions: &[FunctionAnalysis],
    ) -> Self {
        self.pattern_analysis = Some(PatternAnalysis::from_functions(functions));
        self
    }
}

// In src/priority/formatter.rs

impl RecommendationFormatter {
    pub fn format(&self, recommendation: &ModuleSplitRecommendation) -> String {
        let mut sections = vec![
            self.format_header(recommendation),
            self.format_splits(recommendation),
        ];

        // Add pattern analysis if available
        if let Some(patterns) = &recommendation.pattern_analysis {
            sections.push(PatternFormatter::format(patterns));
        }

        sections.join("\n\n")
    }
}
```

### Architecture Changes

**New Module**: `src/output/pattern_analysis.rs`
- `PatternAnalysis` - container for all pattern data
- `PurityMetrics`, `FrameworkPatternMetrics`, `RustPatternMetrics` - data structures
- Aggregation functions: `aggregate_purity_metrics()`, `aggregate_framework_patterns()`, `aggregate_rust_patterns()`

**New Module**: `src/output/pattern_formatter.rs`
- `PatternFormatter` - pure formatting logic separated from data
- All `format_*()` functions are pure (data in, string out)
- No formatting logic mixed with data structures

**Modified**: `src/organization/god_object_detector.rs`
- Attach `PatternAnalysis` to recommendations via `with_pattern_analysis()`
- Pattern data flows as part of recommendation structure

**Modified**: `src/priority/formatter.rs`
- Use `PatternFormatter::format()` to display patterns
- Remove generic "Rust PATTERNS" section
- Simple integration: just call formatter if patterns exist

## Dependencies

- **Prerequisites**: Specs 143 (Purity), 144 (Frameworks), 146 (Rust Patterns)
- **Affected Components**:
  - `src/priority/formatter.rs` - output formatting
  - `src/organization/god_object_detector.rs` - data collection
  - `src/output/` - new pattern_display module

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use pretty_assertions::assert_eq;

    // Data structure tests (pure functions)
    #[test]
    fn purity_percentage_calculation() {
        let metrics = PurityMetrics {
            strictly_pure: 15,
            locally_pure: 10,
            read_only: 5,
            impure: 20,
            almost_pure: vec![],
        };

        assert_eq!(metrics.total_functions(), 50);
        assert_eq!(metrics.purity_percentage(), 0.5); // (15 + 10) / 50
    }

    #[test]
    fn purity_percentage_zero_functions() {
        let metrics = PurityMetrics::default();
        assert_eq!(metrics.total_functions(), 0);
        assert_eq!(metrics.purity_percentage(), 0.0);
    }

    #[test]
    fn top_refactoring_opportunities_limits() {
        let almost_pure = (0..10)
            .map(|i| AlmostPureFunction {
                name: format!("func_{}", i),
                violations: vec![],
                refactoring_suggestion: "test".into(),
            })
            .collect();

        let metrics = PurityMetrics {
            strictly_pure: 0,
            locally_pure: 0,
            read_only: 0,
            impure: 0,
            almost_pure,
        };

        assert_eq!(metrics.top_refactoring_opportunities(5).len(), 5);
        assert_eq!(metrics.top_refactoring_opportunities(20).len(), 10);
    }

    #[test]
    fn framework_patterns_sorted_by_frequency() {
        let metrics = FrameworkPatternMetrics {
            patterns: vec![
                DetectedPattern {
                    framework: "Axum".into(),
                    pattern_type: "Handlers".into(),
                    count: 5,
                    examples: vec![],
                    recommendation: "".into(),
                },
                DetectedPattern {
                    framework: "Pytest".into(),
                    pattern_type: "Fixtures".into(),
                    count: 15,
                    examples: vec![],
                    recommendation: "".into(),
                },
            ],
        };

        let sorted = metrics.sorted_by_frequency();
        assert_eq!(sorted[0].count, 15); // Pytest first
        assert_eq!(sorted[1].count, 5);  // Axum second
    }

    #[test]
    fn rust_patterns_repetitive_traits() {
        let metrics = RustPatternMetrics {
            trait_impls: vec![
                TraitImplementation {
                    trait_name: "Display".into(),
                    count: 8,
                    types: vec![],
                },
                TraitImplementation {
                    trait_name: "From".into(),
                    count: 3,
                    types: vec![],
                },
            ],
            async_patterns: AsyncPatternSummary::default(),
            error_handling: ErrorHandlingSummary::default(),
            builder_candidates: vec![],
        };

        let repetitive = metrics.repetitive_traits();
        assert_eq!(repetitive.len(), 1);
        assert_eq!(repetitive[0].trait_name, "Display");
    }

    // Property-based tests
    proptest! {
        #[test]
        fn purity_percentage_always_valid(
            pure in 0..1000usize,
            locally in 0..1000usize,
            readonly in 0..1000usize,
            impure in 0..1000usize
        ) {
            let metrics = PurityMetrics {
                strictly_pure: pure,
                locally_pure: locally,
                read_only: readonly,
                impure,
                almost_pure: vec![],
            };

            let pct = metrics.purity_percentage();
            prop_assert!(pct >= 0.0 && pct <= 1.0);
        }

        #[test]
        fn total_functions_equals_sum(
            a in 0..1000usize,
            b in 0..1000usize,
            c in 0..1000usize,
            d in 0..1000usize
        ) {
            let metrics = PurityMetrics {
                strictly_pure: a,
                locally_pure: b,
                read_only: c,
                impure: d,
                almost_pure: vec![],
            };

            prop_assert_eq!(metrics.total_functions(), a + b + c + d);
        }
    }

    // Formatter tests (output validation)
    #[test]
    fn format_purity_contains_expected_sections() {
        let metrics = PurityMetrics {
            strictly_pure: 15,
            locally_pure: 10,
            read_only: 5,
            impure: 20,
            almost_pure: vec![
                AlmostPureFunction {
                    name: "calculate_with_logging".into(),
                    violations: vec![],
                    refactoring_suggestion: "Extract println! to caller".into(),
                },
            ],
        };

        let output = PatternFormatter::format_purity(&metrics);

        assert!(output.contains("PURITY ANALYSIS"));
        assert!(output.contains("Strictly Pure: 15"));
        assert!(output.contains("50%")); // (15 + 10) / 50 * 100
        assert!(output.contains("REFACTORING OPPORTUNITIES"));
        assert!(output.contains("calculate_with_logging"));
    }

    #[test]
    fn format_purity_empty_metrics() {
        let metrics = PurityMetrics::default();
        let output = PatternFormatter::format_purity(&metrics);
        assert!(output.is_empty());
    }

    #[test]
    fn format_framework_patterns_sorted() {
        let metrics = FrameworkPatternMetrics {
            patterns: vec![
                DetectedPattern {
                    framework: "Axum".into(),
                    pattern_type: "HTTP Request Handlers".into(),
                    count: 12,
                    examples: vec!["get_user".into(), "create_post".into()],
                    recommendation: "Group by resource".into(),
                },
            ],
        };

        let output = PatternFormatter::format_frameworks(&metrics);

        assert!(output.contains("FRAMEWORK PATTERNS DETECTED"));
        assert!(output.contains("Axum"));
        assert!(output.contains("12x detected"));
        assert!(output.contains("get_user, create_post"));
    }

    #[test]
    fn format_rust_patterns_with_warnings() {
        let metrics = RustPatternMetrics {
            trait_impls: vec![
                TraitImplementation {
                    trait_name: "Display".into(),
                    count: 8,
                    types: vec!["Error".into(), "Config".into()],
                },
            ],
            async_patterns: AsyncPatternSummary {
                async_functions: 15,
                spawn_calls: 3,
                channel_usage: true,
                mutex_usage: false,
            },
            error_handling: ErrorHandlingSummary {
                question_mark_density: 2.5,
                custom_error_types: vec![],
                unwrap_count: 5,
            },
            builder_candidates: vec![],
        };

        let output = PatternFormatter::format_rust_patterns(&metrics);

        assert!(output.contains("RUST-SPECIFIC PATTERNS"));
        assert!(output.contains("Display: 8 implementations"));
        assert!(output.contains("Async functions: 15"));
        assert!(output.contains("⚠"));
        assert!(output.contains("5 unwrap() calls"));
    }

    #[test]
    fn format_rust_patterns_empty() {
        let metrics = RustPatternMetrics::default();
        let output = PatternFormatter::format_rust_patterns(&metrics);
        assert!(output.is_empty());
    }

    // Edge case tests
    #[test]
    fn format_handles_unicode_in_names() {
        let metrics = PurityMetrics {
            strictly_pure: 1,
            locally_pure: 0,
            read_only: 0,
            impure: 0,
            almost_pure: vec![
                AlmostPureFunction {
                    name: "calculer_π".into(),
                    violations: vec![],
                    refactoring_suggestion: "Extract side effects".into(),
                },
            ],
        };

        let output = PatternFormatter::format_purity(&metrics);
        assert!(output.contains("calculer_π"));
    }

    #[test]
    fn format_handles_empty_examples() {
        let metrics = FrameworkPatternMetrics {
            patterns: vec![
                DetectedPattern {
                    framework: "Test".into(),
                    pattern_type: "Pattern".into(),
                    count: 5,
                    examples: vec![],
                    recommendation: "Do something".into(),
                },
            ],
        };

        let output = PatternFormatter::format_frameworks(&metrics);
        assert!(!output.contains("Examples: ,"));
    }
}
```

### Integration Tests

```rust
#[test]
fn patterns_attached_to_recommendations() {
    // Create sample function analyses
    let functions = vec![
        create_test_function("pure_calc", PurityLevel::StrictlyPure),
        create_test_function("impure_io", PurityLevel::Impure),
    ];

    // Create recommendation with pattern analysis
    let recommendation = ModuleSplitRecommendation::new(/* ... */)
        .with_pattern_analysis(&functions);

    assert!(recommendation.pattern_analysis.is_some());
    let patterns = recommendation.pattern_analysis.unwrap();
    assert_eq!(patterns.purity.strictly_pure, 1);
    assert_eq!(patterns.purity.impure, 1);
}

#[test]
fn full_output_contains_pattern_analysis() {
    let functions = vec![
        create_test_function_with_traits("impl_display", vec!["Display"]),
        create_test_function_async("async_handler", true),
    ];

    let recommendation = ModuleSplitRecommendation::new(/* ... */)
        .with_pattern_analysis(&functions);

    let formatted = RecommendationFormatter::new().format(&recommendation);

    // Should contain all pattern sections
    assert!(formatted.contains("PURITY ANALYSIS"));
    assert!(formatted.contains("RUST-SPECIFIC PATTERNS"));
    assert!(formatted.contains("Trait Implementations"));
    assert!(formatted.contains("Async/Concurrency"));

    // Should NOT contain generic advice
    assert!(!formatted.contains("Extract traits for shared behavior"));
    assert!(!formatted.contains("Use newtype pattern for domain types"));
}

#[test]
fn empty_patterns_produce_no_output() {
    let functions = vec![]; // No functions

    let recommendation = ModuleSplitRecommendation::new(/* ... */)
        .with_pattern_analysis(&functions);

    let formatted = RecommendationFormatter::new().format(&recommendation);

    // Pattern sections should be absent when no patterns detected
    assert!(!formatted.contains("PURITY ANALYSIS"));
    assert!(!formatted.contains("RUST-SPECIFIC PATTERNS"));
}

#[test]
fn aggregation_correctness() {
    // Test that aggregation functions correctly combine data
    let functions = vec![
        FunctionAnalysis {
            name: "func1".into(),
            purity_classification: PurityLevel::StrictlyPure,
            purity_violations: vec![],
            // ...
        },
        FunctionAnalysis {
            name: "func2".into(),
            purity_classification: PurityLevel::LocallyPure,
            purity_violations: vec![],
            // ...
        },
        FunctionAnalysis {
            name: "func3".into(),
            purity_classification: PurityLevel::Impure,
            purity_violations: vec![
                PurityViolation::ConsoleOutput { /* ... */ },
            ],
            // ...
        },
    ];

    let pattern_analysis = PatternAnalysis::from_functions(&functions);

    assert_eq!(pattern_analysis.purity.strictly_pure, 1);
    assert_eq!(pattern_analysis.purity.locally_pure, 1);
    assert_eq!(pattern_analysis.purity.impure, 1);
    assert_eq!(pattern_analysis.purity.total_functions(), 3);
}
```

## Documentation Requirements

### User Documentation

Update README.md:
```markdown
## Pattern Analysis

Debtmap detects and displays code patterns to guide refactoring:

**Purity Analysis**:
- Shows % of pure functions (ideal for testing)
- Identifies "almost pure" functions for extraction
- Suggests separating pure from impure code

**Framework Patterns**:
- Detects Axum/Actix handlers, pytest fixtures, etc.
- Groups functions by framework role
- Provides framework-specific architectural guidance

**Rust Patterns**:
- Lists trait implementations (Display, From, Iterator)
- Shows async/concurrency usage
- Identifies builder pattern candidates
- Flags error handling anti-patterns (unwrap usage)

**Example Output**:
```
PURITY ANALYSIS:
- Strictly Pure: 15 functions (30%)
- Impure: 20 functions

REFACTORING OPPORTUNITIES:
- calculate_with_logging (1 violation): Console output
  → Suggestion: Extract println! to caller, make core logic pure

RUST-SPECIFIC PATTERNS:
Trait Implementations:
  - Display: 8 implementations (Error, Config, ...)
  → Consider using macros for repetitive implementations

Async/Concurrency:
  - Async functions: 15
  - Spawn calls: 3
  → Concurrency management detected - group spawn logic

Error Handling:
  - Average ? operators per function: 2.5
  ⚠ 5 unwrap() calls detected - replace with proper error handling
```
```

## Implementation Notes

### Collecting Pattern Data

Pattern data is already collected during multi-signal classification (Specs 143, 144, 146). This spec primarily adds **display logic**:

```rust
// During analysis (already happening)
let purity_analysis = purity_analyzer.analyze_function(func);  // Spec 143
let framework_patterns = framework_detector.detect_patterns(func);  // Spec 144
let rust_patterns = rust_detector.detect_patterns(func);  // Spec 146

// NEW: Aggregate for display
let purity_metrics = aggregate_purity_metrics(&all_functions);
let framework_metrics = aggregate_framework_patterns(&all_functions);
let rust_metrics = aggregate_rust_patterns(&all_functions);

// NEW: Format for output
let pattern_display = format_patterns(purity_metrics, framework_metrics, rust_metrics);
```

### Performance Optimization

Since pattern data is already collected, formatting adds minimal overhead:

```rust
// Lazy formatting - only format when displaying
pub struct LazyPatternDisplay {
    purity: OnceCell<String>,
    framework: OnceCell<String>,
    rust: OnceCell<String>,
}

impl LazyPatternDisplay {
    pub fn format(&self) -> String {
        let purity = self.purity.get_or_init(|| format_purity_metrics(...));
        let framework = self.framework.get_or_init(|| format_framework_patterns(...));
        let rust = self.rust.get_or_init(|| format_rust_patterns(...));

        format!("{}\n{}\n{}", purity, framework, rust)
    }
}
```

## Migration and Compatibility

### Backward Compatibility

- Pattern displays are additions to existing output
- No breaking changes to output format
- Users can disable pattern analysis with config flag

### Configuration

Add to `debtmap.toml`:
```toml
[output.patterns]
show_purity = true
show_framework = true
show_rust_patterns = true
show_refactoring_opportunities = true
max_opportunities = 5  # Limit displayed opportunities
```

## Expected Impact

### Output Quality Improvement

**Before (generic advice)**:
```
- Rust PATTERNS:
-  - Extract traits for shared behavior
-  - Use newtype pattern for domain types
-  - Consider builder pattern for complex construction
```

**After (actual detected patterns)**:
```
PURITY ANALYSIS:
- Strictly Pure: 15 functions (30%)
- Impure: 20 functions

REFACTORING OPPORTUNITIES:
- calculate_total (1 violation): Single println! call
  → Extract logging to caller, make calculation pure

FRAMEWORK PATTERNS DETECTED:
- Rust Testing: Test Functions (25x detected)
  Examples: test_parser, test_validation, test_integration
  Recommendation: Tests already well-organized with #[test]

RUST-SPECIFIC PATTERNS:
Trait Implementations:
  - Display: 8 implementations (Error, Config, Result)
  - From: 12 implementations (various type conversions)
  → Consider macro_rules! for repetitive From implementations

Async/Concurrency:
  - Async functions: 15
  - Spawn calls: 3
  - Channels: Yes
  → Concurrency management detected - group spawn logic

Error Handling:
  - Average ? operators per function: 2.5
  ⚠ 5 unwrap() calls detected in: analyze_file, parse_config
  → Replace unwrap() with ? operator for better error propagation
```

### User Benefits

- **Actionable insights**: Specific refactoring opportunities, not generic advice
- **Pattern awareness**: Learn what patterns exist in codebase
- **Quality improvement**: Identify anti-patterns (unwrap usage) automatically
- **Testability guidance**: Know which functions are pure and easy to test

## Success Metrics

- [ ] 100% of recommendations include actual pattern analysis (not generic advice)
- [ ] "Almost pure" functions identified and highlighted with actionable suggestions
- [ ] Framework patterns displayed when detected
- [ ] Rust trait implementations listed with repetition warnings
- [ ] Async patterns shown with specific counts
- [ ] Error handling anti-patterns flagged (unwrap usage)
- [ ] Zero generic "Rust PATTERNS" advice in output
- [ ] All tests pass (unit, property-based, integration, edge cases)
- [ ] Code coverage >85% for pattern analysis and formatting modules
- [ ] No performance regression (aggregation adds <5% to analysis time)
- [ ] Clean separation: data structures have no formatting logic
- [ ] User feedback: Pattern analysis is helpful for refactoring

## Summary of Improvements

This specification has been improved to follow functional programming principles and best practices:

### 1. Strict Separation of Concerns
**Before**: Data structures contained formatting methods mixed with calculations.
**After**:
- Data structures are pure data with `Serialize`/`Deserialize`
- Calculation methods are pure functions (data in, value out)
- Formatting is completely separate in `PatternFormatter`

### 2. Simplified Integration
**Before**: Formatters took 4+ parameters, making function signatures complex.
**After**:
- `PatternAnalysis` container wraps all pattern data
- Attached to recommendations as an `Option<PatternAnalysis>` field
- Simple builder pattern: `recommendation.with_pattern_analysis(functions)`
- Formatter just checks if patterns exist and calls `PatternFormatter::format()`

### 3. Removed Premature Optimization
**Before**: Included `LazyPatternDisplay` with `OnceCell` for lazy formatting.
**After**:
- Removed entirely (YAGNI principle)
- String formatting is cheap and not a bottleneck
- Pattern data already computed during analysis
- Simpler code, easier to maintain

### 4. Comprehensive Testing Strategy
**Before**: Basic unit tests for formatters only.
**After**:
- **Unit tests**: Pure calculation functions (percentages, totals, etc.)
- **Property-based tests**: Using `proptest` for invariant verification
- **Formatter tests**: Output structure and content validation
- **Edge case tests**: Empty metrics, Unicode, zero functions
- **Integration tests**: End-to-end pattern flow verification
- **Anti-regression tests**: Verify generic advice is eliminated

### Key Design Decisions

1. **Data-first design**: Types are serializable, composable, and functional
2. **Pure aggregation functions**: Take function analyses, return metrics
3. **Formatter is stateless**: Pure static methods for display logic
4. **Builder pattern for integration**: Clean, discoverable API
5. **Constants for magic numbers**: `REPETITIVE_TRAIT_THRESHOLD`, `MAX_DISPLAYED_EXAMPLES`
6. **Default implementations**: All types have sensible defaults
7. **Helper methods for queries**: `has_patterns()`, `repetitive_traits()`, etc.

### Implementation Complexity

- **Reduced**: Simpler integration, no lazy evaluation complexity
- **Maintained**: Same feature set, better structure
- **Improved testability**: Pure functions are trivial to test
- **Better maintainability**: Clear boundaries between modules
