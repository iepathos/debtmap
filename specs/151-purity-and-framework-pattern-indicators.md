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

- [ ] Purity levels shown for each module split (% pure functions)
- [ ] "Almost pure" functions highlighted with specific violations
- [ ] Framework patterns displayed when detected (Axum handlers, tests, etc.)
- [ ] Rust trait implementations listed (Display, From, etc.)
- [ ] Async patterns shown (async fn count, tokio usage)
- [ ] Builder pattern candidates identified
- [ ] Refactoring opportunities presented with specific actions
- [ ] Generic "Rust PATTERNS" advice replaced with actual detected patterns
- [ ] Test suite includes pattern display formatting tests
- [ ] Documentation explains how to interpret pattern indicators

## Technical Details

### Implementation Approach

**Phase 1: Purity Display**

```rust
#[derive(Debug, Clone)]
pub struct PurityMetrics {
    pub strictly_pure: usize,
    pub locally_pure: usize,
    pub read_only: usize,
    pub impure: usize,
    pub almost_pure: Vec<AlmostPureFunction>,
}

#[derive(Debug, Clone)]
pub struct AlmostPureFunction {
    pub name: String,
    pub violations: Vec<PurityViolation>,
    pub refactoring_suggestion: String,
}

impl PurityMetrics {
    pub fn purity_percentage(&self) -> f64 {
        let total = self.strictly_pure + self.locally_pure + self.read_only + self.impure;
        if total == 0 {
            return 0.0;
        }
        (self.strictly_pure + self.locally_pure) as f64 / total as f64
    }

    pub fn format_for_output(&self) -> String {
        format!(
            "PURITY ANALYSIS:\n\
             - Strictly Pure: {} functions ({:.0}%)\n\
             - Locally Pure: {} functions\n\
             - Read-Only: {} functions\n\
             - Impure: {} functions\n\
             \n\
             REFACTORING OPPORTUNITIES:\n\
             {}",
            self.strictly_pure,
            self.purity_percentage() * 100.0,
            self.locally_pure,
            self.read_only,
            self.impure,
            self.format_almost_pure_functions()
        )
    }

    fn format_almost_pure_functions(&self) -> String {
        if self.almost_pure.is_empty() {
            return "  - No extraction opportunities detected".into();
        }

        self.almost_pure.iter()
            .take(5)  // Show top 5
            .map(|func| format!(
                "  - {} (1 violation): {}\n\
                     → Suggestion: {}",
                func.name,
                func.violations[0],
                func.refactoring_suggestion
            ))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
```

**Phase 2: Framework Pattern Display**

```rust
#[derive(Debug, Clone)]
pub struct FrameworkPatternMetrics {
    pub patterns: Vec<DetectedPattern>,
}

#[derive(Debug, Clone)]
pub struct DetectedPattern {
    pub framework: String,
    pub pattern_type: String,
    pub count: usize,
    pub examples: Vec<String>,
    pub recommendation: String,
}

impl FrameworkPatternMetrics {
    pub fn format_for_output(&self) -> String {
        if self.patterns.is_empty() {
            return String::new();
        }

        let mut output = String::from("FRAMEWORK PATTERNS DETECTED:\n");

        for pattern in &self.patterns {
            output.push_str(&format!(
                "  - {} {} ({}x detected)\n\
                     Examples: {}\n\
                     Recommendation: {}\n",
                pattern.framework,
                pattern.pattern_type,
                pattern.count,
                pattern.examples.join(", "),
                pattern.recommendation
            ));
        }

        output
    }
}

// Example usage
fn detect_framework_patterns(functions: &[FunctionAnalysis]) -> FrameworkPatternMetrics {
    let mut patterns = Vec::new();

    // Detect Axum handlers
    let axum_handlers: Vec<_> = functions.iter()
        .filter(|f| is_axum_handler(f))
        .map(|f| f.name.clone())
        .collect();

    if !axum_handlers.is_empty() {
        patterns.push(DetectedPattern {
            framework: "Axum".into(),
            pattern_type: "HTTP Request Handlers".into(),
            count: axum_handlers.len(),
            examples: axum_handlers.iter().take(3).cloned().collect(),
            recommendation: "Consider grouping handlers by resource (users/, posts/, etc.)".into(),
        });
    }

    // Detect test functions
    let test_functions: Vec<_> = functions.iter()
        .filter(|f| is_test_function(f))
        .map(|f| f.name.clone())
        .collect();

    if !test_functions.is_empty() {
        patterns.push(DetectedPattern {
            framework: "Rust Testing".into(),
            pattern_type: "Test Functions".into(),
            count: test_functions.len(),
            examples: test_functions.iter().take(3).cloned().collect(),
            recommendation: "Tests already well-organized with #[test] attributes".into(),
        });
    }

    FrameworkPatternMetrics { patterns }
}
```

**Phase 3: Rust Pattern Display**

```rust
#[derive(Debug, Clone)]
pub struct RustPatternMetrics {
    pub trait_impls: Vec<TraitImplementation>,
    pub async_patterns: AsyncPatternSummary,
    pub error_handling: ErrorHandlingSummary,
    pub builder_candidates: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TraitImplementation {
    pub trait_name: String,
    pub count: usize,
    pub types: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AsyncPatternSummary {
    pub async_functions: usize,
    pub spawn_calls: usize,
    pub channel_usage: bool,
    pub mutex_usage: bool,
}

#[derive(Debug, Clone)]
pub struct ErrorHandlingSummary {
    pub question_mark_density: f64,  // Avg ? operators per function
    pub custom_error_types: Vec<String>,
    pub unwrap_count: usize,  // Anti-pattern
}

impl RustPatternMetrics {
    pub fn format_for_output(&self) -> String {
        let mut output = String::from("RUST-SPECIFIC PATTERNS:\n");

        // Trait implementations
        if !self.trait_impls.is_empty() {
            output.push_str("  Trait Implementations:\n");
            for trait_impl in &self.trait_impls {
                output.push_str(&format!(
                    "    - {}: {} implementations ({})\n",
                    trait_impl.trait_name,
                    trait_impl.count,
                    trait_impl.types.join(", ")
                ));
            }

            // Suggest macros if repetitive
            let repetitive_traits: Vec<_> = self.trait_impls.iter()
                .filter(|t| t.count >= 5)
                .collect();

            if !repetitive_traits.is_empty() {
                output.push_str("    → Consider using macros for repetitive implementations\n");
            }
        }

        // Async patterns
        if self.async_patterns.async_functions > 0 {
            output.push_str(&format!(
                "  Async/Concurrency:\n\
                   - Async functions: {}\n\
                   - Spawn calls: {}\n\
                   - Channels: {}\n\
                   - Mutex usage: {}\n",
                self.async_patterns.async_functions,
                self.async_patterns.spawn_calls,
                if self.async_patterns.channel_usage { "Yes" } else { "No" },
                if self.async_patterns.mutex_usage { "Yes" } else { "No" }
            ));

            if self.async_patterns.spawn_calls > 0 {
                output.push_str("    → Concurrency management detected - group spawn logic\n");
            }
        }

        // Error handling
        if self.error_handling.question_mark_density > 0.0 {
            output.push_str(&format!(
                "  Error Handling:\n\
                   - Average ? operators per function: {:.1}\n",
                self.error_handling.question_mark_density
            ));

            if self.error_handling.unwrap_count > 0 {
                output.push_str(&format!(
                    "    ⚠ {} unwrap() calls detected - replace with proper error handling\n",
                    self.error_handling.unwrap_count
                ));
            }
        }

        // Builder candidates
        if !self.builder_candidates.is_empty() {
            output.push_str(&format!(
                "  Builder Patterns:\n\
                   - Candidates: {}\n\
                   → Extract builder logic into separate module\n",
                self.builder_candidates.join(", ")
            ));
        }

        output
    }
}
```

**Phase 4: Integration with Recommendation Output**

```rust
// In src/priority/formatter.rs

impl RecommendationFormatter {
    pub fn format_recommendation_with_patterns(
        &self,
        recommendation: &ModuleSplitRecommendation,
        purity_metrics: &PurityMetrics,
        framework_patterns: &FrameworkPatternMetrics,
        rust_patterns: &RustPatternMetrics,
    ) -> String {
        let mut output = String::new();

        // Existing recommendation format
        output.push_str(&self.format_basic_recommendation(recommendation));

        // NEW: Add pattern analysis
        output.push_str("\n");
        output.push_str(&purity_metrics.format_for_output());
        output.push_str("\n");
        output.push_str(&framework_patterns.format_for_output());
        output.push_str("\n");
        output.push_str(&rust_patterns.format_for_output());

        output
    }
}
```

### Architecture Changes

**New Module**: `src/output/pattern_display.rs`
- Purity metrics formatting
- Framework pattern formatting
- Rust pattern formatting
- Refactoring opportunity display

**Modified Module**: `src/priority/formatter.rs`
- Integrate pattern displays into recommendations
- Replace generic "Rust PATTERNS" section with actual detected patterns
- Add pattern-specific refactoring advice

**Modified Module**: `src/organization/god_object_detector.rs`
- Collect purity, framework, and Rust pattern data during analysis
- Attach pattern metrics to recommendations
- Pass pattern data to formatter

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

    #[test]
    fn format_purity_metrics() {
        let metrics = PurityMetrics {
            strictly_pure: 15,
            locally_pure: 10,
            read_only: 5,
            impure: 20,
            almost_pure: vec![
                AlmostPureFunction {
                    name: "calculate_with_logging".into(),
                    violations: vec![PurityViolation::ConsoleOutput { .. }],
                    refactoring_suggestion: "Extract println! to caller".into(),
                },
            ],
        };

        let output = metrics.format_for_output();

        assert!(output.contains("Strictly Pure: 15"));
        assert!(output.contains("30%"));  // (15 / 50) * 100
        assert!(output.contains("REFACTORING OPPORTUNITIES"));
        assert!(output.contains("calculate_with_logging"));
    }

    #[test]
    fn format_framework_patterns() {
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

        let output = metrics.format_for_output();

        assert!(output.contains("Axum"));
        assert!(output.contains("HTTP Request Handlers"));
        assert!(output.contains("12x detected"));
    }

    #[test]
    fn format_rust_patterns() {
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
                custom_error_types: vec!["AnalysisError".into()],
                unwrap_count: 5,
            },
            builder_candidates: vec![],
        };

        let output = metrics.format_for_output();

        assert!(output.contains("Display: 8 implementations"));
        assert!(output.contains("Async functions: 15"));
        assert!(output.contains("⚠ 5 unwrap() calls detected"));
    }
}
```

### Integration Tests

```rust
#[test]
fn patterns_in_full_output() {
    let analysis = analyze_file("src/analyzers/rust.rs");
    let formatted = format_recommendation(&analysis);

    // Should contain pattern sections
    assert!(formatted.contains("PURITY ANALYSIS"));
    assert!(formatted.contains("RUST-SPECIFIC PATTERNS"));

    // Should have specific counts
    assert!(formatted.contains("functions"));  // Purity counts
    assert!(formatted.contains("implementations"));  // Trait impls

    // Should not have generic advice
    assert!(!formatted.contains("Extract traits for shared behavior"));
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
- [ ] "Almost pure" functions identified and highlighted
- [ ] Framework patterns displayed when detected
- [ ] Rust trait implementations listed
- [ ] Async patterns shown with specific counts
- [ ] Error handling anti-patterns flagged (unwrap usage)
- [ ] User feedback: Pattern analysis is helpful for refactoring
- [ ] Zero generic "Rust PATTERNS" advice in output
