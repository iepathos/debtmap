---
number: 122
title: Context-Aware Recommendations for Specialized Code Patterns
category: foundation
priority: medium
status: draft
dependencies: [118, 121]
created: 2025-10-25
---

# Specification 122: Context-Aware Recommendations for Specialized Code Patterns

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 118 (Pure Mapping Detection), Spec 121 (Cognitive Weighting)

## Context

Debtmap provides generic refactoring recommendations ("extract functions," "reduce complexity") that apply to most code. However, certain code patterns have specialized contexts where generic advice is suboptimal or misses better alternatives specific to the domain.

**Current Problem**:

Generic recommendations ignore code context:

```
#6 SCORE: 14.8 [CRITICAL]
├─ LOCATION: src/io/pattern_output.rs:67 format_pattern_type()
├─ COMPLEXITY: cyclomatic=15, cognitive=3
└─ ACTION: Refactor into 5 functions with ≤3 complexity each
```

**Why This Falls Short**:

1. **Formatter/Output Code**:
   - High complexity is expected (many output formats)
   - Splitting formatters often makes code harder to follow
   - Better advice: "This is output formatting - complexity is normal"

2. **Parser Code**:
   - Complex match expressions are idiomatic
   - Better advice: "Consider parser combinators or grammar-based approach"

3. **CLI Command Handlers**:
   - Orchestration naturally has many branches
   - Better advice: "Use command pattern or dispatch table"

4. **State Machines**:
   - Exhaustive state transition handling has high complexity
   - Better advice: "Consider state pattern or state machine library"

**Real-World Impact**:
- Developers ignore relevant recommendations (alert fatigue)
- Generic advice doesn't help in specialized domains
- Missing opportunities to suggest better patterns
- Lack of educational value (no learning from recommendations)

## Objective

Implement context detection to provide specialized, actionable recommendations tailored to the code's domain and purpose, improving relevance and educational value while maintaining generic fallbacks.

## Requirements

### Functional Requirements

1. **Context Detection**
   - Identify code patterns by analyzing:
     - Function/file naming conventions
     - AST structure and patterns
     - Import/usage patterns
     - File location in project structure
   - Supported contexts:
     - Formatters/Output generation
     - Parsers/Input processing
     - CLI handlers
     - State machines
     - Configuration code
     - Test helpers
     - Database queries

2. **Context-Specific Recommendations**
   - **Formatters**: Acknowledge complexity is normal, suggest builder pattern if needed
   - **Parsers**: Recommend parser combinators, grammar-based approaches
   - **CLI Handlers**: Suggest command pattern, dispatch tables
   - **State Machines**: Recommend state pattern, state machine libraries
   - **Configuration**: Suggest builder pattern, validation layers
   - **Generic Fallback**: Use existing recommendations when no context detected

3. **Educational Explanations**
   - Explain why complexity is acceptable (or not) for this context
   - Link to relevant design patterns
   - Provide concrete examples from similar codebases
   - Suggest libraries/frameworks specific to domain

4. **Severity Adjustment**
   - Lower severity for expected-complexity contexts (formatters)
   - Maintain severity for avoidable complexity
   - Adjust based on cognitive vs cyclomatic dominance

### Non-Functional Requirements

- Context detection adds <50ms per analysis
- Detection accuracy >80% for common patterns
- Recommendations remain actionable and specific
- Educational content concise (<3 sentences)
- Works across Rust, Python, JavaScript, TypeScript

## Acceptance Criteria

- [ ] Formatter functions show: "Note: Output formatting typically has high complexity"
- [ ] Parser functions recommend: "Consider parser combinators (e.g., nom, pest)"
- [ ] CLI handlers suggest: "Use command pattern or dispatch table for cleaner structure"
- [ ] State machines recommend: "Consider state machine library (e.g., rust-fsm)"
- [ ] Generic recommendations provided when no context detected
- [ ] Context detection accuracy >80% on test suite
- [ ] Severity adjusted based on context appropriateness
- [ ] Educational links included in recommendations
- [ ] Performance impact <5% on analysis time
- [ ] All supported languages have context detection

## Technical Details

### Implementation Approach

**Phase 1: Context Detection Engine**

Create `src/analysis/context_detection.rs`:

```rust
pub struct ContextDetector {
    patterns: Vec<ContextPattern>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionContext {
    Formatter,
    Parser,
    CliHandler,
    StateMachine,
    Configuration,
    TestHelper,
    DatabaseQuery,
    Validator,
    Generic,
}

pub struct ContextPattern {
    pub context: FunctionContext,
    pub name_patterns: Vec<Regex>,
    pub file_patterns: Vec<Regex>,
    pub ast_patterns: Vec<AstPattern>,
    pub import_patterns: Vec<String>,
}

#[derive(Debug)]
pub struct ContextAnalysis {
    pub context: FunctionContext,
    pub confidence: f64,
    pub detected_signals: Vec<String>,
}

impl ContextDetector {
    pub fn detect_context(
        &self,
        function: &FunctionMetrics,
        file_path: &Path,
        ast: &Ast,
    ) -> ContextAnalysis {
        let signals = self.gather_signals(function, file_path, ast);
        let context = self.classify_context(&signals);
        let confidence = self.calculate_confidence(&signals, &context);

        ContextAnalysis {
            context,
            confidence,
            detected_signals: signals.descriptions(),
        }
    }

    fn gather_signals(
        &self,
        function: &FunctionMetrics,
        file_path: &Path,
        ast: &Ast,
    ) -> ContextSignals {
        ContextSignals {
            function_name: function.name.clone(),
            file_path: file_path.to_path_buf(),
            has_format_calls: self.has_format_operations(ast, function),
            has_parse_calls: self.has_parse_operations(ast, function),
            has_io_operations: self.has_io_operations(ast, function),
            return_type: function.return_type.clone(),
            parameter_types: function.parameter_types.clone(),
            imported_crates: self.extract_imports(ast),
        }
    }

    fn classify_context(&self, signals: &ContextSignals) -> FunctionContext {
        // Name-based detection (high confidence)
        if signals.function_name.contains("format") ||
           signals.function_name.contains("render") ||
           signals.function_name.contains("display") {
            return FunctionContext::Formatter;
        }

        if signals.function_name.contains("parse") ||
           signals.function_name.starts_with("read_") ||
           signals.function_name.starts_with("decode_") {
            return FunctionContext::Parser;
        }

        if signals.function_name.starts_with("handle_") ||
           signals.function_name.starts_with("cmd_") ||
           signals.file_path.to_string_lossy().contains("/commands/") {
            return FunctionContext::CliHandler;
        }

        // Behavior-based detection (medium confidence)
        if signals.has_format_calls && signals.return_type.contains("String") {
            return FunctionContext::Formatter;
        }

        if signals.has_parse_calls && signals.parameter_types.iter().any(|t| t.contains("str")) {
            return FunctionContext::Parser;
        }

        // Import-based detection (low-medium confidence)
        if signals.imported_crates.iter().any(|c| c.contains("nom") || c.contains("pest")) {
            return FunctionContext::Parser;
        }

        FunctionContext::Generic
    }

    fn calculate_confidence(&self, signals: &ContextSignals, context: &FunctionContext) -> f64 {
        // Multiple signals increase confidence
        let signal_count = signals.matching_signal_count(context);

        match signal_count {
            0 => 0.1,  // Default/generic
            1 => 0.6,  // Single signal
            2 => 0.8,  // Two signals
            _ => 0.95, // Three or more signals
        }
    }
}

struct ContextSignals {
    function_name: String,
    file_path: PathBuf,
    has_format_calls: bool,
    has_parse_calls: bool,
    has_io_operations: bool,
    return_type: String,
    parameter_types: Vec<String>,
    imported_crates: Vec<String>,
}
```

**Phase 2: Context-Specific Recommendations**

Create `src/priority/recommendations/context_aware.rs`:

```rust
pub struct ContextRecommendationEngine {
    templates: HashMap<FunctionContext, RecommendationTemplate>,
}

pub struct RecommendationTemplate {
    pub explanation: String,
    pub suggestions: Vec<String>,
    pub patterns: Vec<String>,
    pub examples: Vec<String>,
    pub severity_adjustment: f64,
}

impl ContextRecommendationEngine {
    pub fn generate_recommendation(
        &self,
        function: &FunctionMetrics,
        context: &ContextAnalysis,
    ) -> ContextualRecommendation {
        let template = self.templates.get(&context.context)
            .unwrap_or_else(|| self.templates.get(&FunctionContext::Generic).unwrap());

        let adjusted_severity = self.adjust_severity(function.score, template.severity_adjustment);

        ContextualRecommendation {
            context: context.context,
            explanation: self.customize_explanation(template, function, context),
            suggestions: template.suggestions.clone(),
            patterns: template.patterns.clone(),
            examples: template.examples.clone(),
            severity: adjusted_severity,
            confidence: context.confidence,
        }
    }

    fn adjust_severity(&self, base_score: f64, adjustment: f64) -> Severity {
        let adjusted_score = base_score * adjustment;

        if adjusted_score > 20.0 {
            Severity::Critical
        } else if adjusted_score > 12.0 {
            Severity::High
        } else if adjusted_score > 7.0 {
            Severity::Moderate
        } else {
            Severity::Low
        }
    }
}

impl Default for ContextRecommendationEngine {
    fn default() -> Self {
        let mut templates = HashMap::new();

        templates.insert(FunctionContext::Formatter, RecommendationTemplate {
            explanation: "This is output formatting code. High cyclomatic complexity is \
                         typical for formatters with many output variants. Focus on \
                         cognitive complexity and consider builder pattern if deeply nested.".to_string(),
            suggestions: vec![
                "If complexity is from nesting (not just cases), consider builder pattern".to_string(),
                "Extract format helpers for repeated patterns".to_string(),
                "Use match expressions for clean exhaustive formatting".to_string(),
            ],
            patterns: vec![
                "Builder pattern for complex output".to_string(),
                "Template method for format variants".to_string(),
            ],
            examples: vec![
                "colored::Colorize for terminal formatting".to_string(),
                "serde for structured output".to_string(),
            ],
            severity_adjustment: 0.6,  // Lower severity for formatters
        });

        templates.insert(FunctionContext::Parser, RecommendationTemplate {
            explanation: "This is parsing code. Consider using parser combinators or \
                         grammar-based approaches for better maintainability and clearer intent.".to_string(),
            suggestions: vec![
                "Use parser combinator library (nom, pest, combine)".to_string(),
                "Define grammar separately from parsing logic".to_string(),
                "Break parsing into lexing + parsing phases".to_string(),
            ],
            patterns: vec![
                "Parser combinators for composable parsing".to_string(),
                "Recursive descent for simple grammars".to_string(),
            ],
            examples: vec![
                "nom for binary/text parsing".to_string(),
                "pest for grammar-based parsing".to_string(),
                "serde for structured data".to_string(),
            ],
            severity_adjustment: 0.8,
        });

        templates.insert(FunctionContext::CliHandler, RecommendationTemplate {
            explanation: "This is a CLI command handler. Orchestration naturally involves \
                         multiple branches. Consider command pattern or dispatch table for cleaner structure.".to_string(),
            suggestions: vec![
                "Use command pattern with trait-based dispatch".to_string(),
                "Extract validation, execution, and output into separate functions".to_string(),
                "Consider dispatch table for subcommand routing".to_string(),
            ],
            patterns: vec![
                "Command pattern for each subcommand".to_string(),
                "Strategy pattern for different output formats".to_string(),
            ],
            examples: vec![
                "clap::derive for arg parsing".to_string(),
                "Trait-based command dispatch".to_string(),
            ],
            severity_adjustment: 0.7,
        });

        templates.insert(FunctionContext::StateMachine, RecommendationTemplate {
            explanation: "This appears to be state machine logic. Exhaustive state \
                         handling results in high complexity. Consider state pattern or state machine library.".to_string(),
            suggestions: vec![
                "Use state machine library (rust-fsm, machine)".to_string(),
                "Implement state pattern with trait-based states".to_string(),
                "Extract transition logic into separate functions".to_string(),
            ],
            patterns: vec![
                "State pattern for cleaner transitions".to_string(),
                "Type-state pattern for compile-time validation".to_string(),
            ],
            examples: vec![
                "rust-fsm for simple state machines".to_string(),
                "Type-state pattern for API design".to_string(),
            ],
            severity_adjustment: 0.75,
        });

        templates.insert(FunctionContext::Generic, RecommendationTemplate {
            explanation: "This function has high complexity that should be reduced.".to_string(),
            suggestions: vec![
                "Extract pure functions from complex logic".to_string(),
                "Reduce nesting depth with early returns".to_string(),
                "Break into smaller, testable functions".to_string(),
            ],
            patterns: vec![],
            examples: vec![],
            severity_adjustment: 1.0,  // No adjustment
        });

        Self { templates }
    }
}
```

**Phase 3: Integration with Output**

Modify `src/priority/formatter.rs`:

```rust
fn format_contextual_recommendation(
    &self,
    function: &FunctionMetrics,
    recommendation: &ContextualRecommendation,
) -> String {
    let mut output = String::new();

    // Show context if detected with confidence
    if recommendation.confidence > 0.6 {
        writeln!(
            &mut output,
            "├─ 📋 CONTEXT: {:?} (confidence: {:.0}%)",
            recommendation.context,
            recommendation.confidence * 100.0
        )?;
    }

    // Contextual explanation
    writeln!(&mut output, "├─ 💡 {}", recommendation.explanation)?;

    // Suggestions
    if !recommendation.suggestions.is_empty() {
        writeln!(&mut output, "├─ SUGGESTIONS:")?;
        for (i, suggestion) in recommendation.suggestions.iter().enumerate() {
            writeln!(&mut output, "│  {}. {}", i + 1, suggestion)?;
        }
    }

    // Relevant patterns
    if !recommendation.patterns.is_empty() {
        writeln!(&mut output, "├─ PATTERNS:")?;
        for pattern in &recommendation.patterns {
            writeln!(&mut output, "│  • {}", pattern)?;
        }
    }

    // Examples/libraries
    if !recommendation.examples.is_empty() {
        writeln!(&mut output, "└─ RESOURCES:")?;
        for example in &recommendation.examples {
            writeln!(&mut output, "   • {}", example)?;
        }
    }

    output
}
```

**Example Enhanced Output**:

```
#6 SCORE: 8.2 [MODERATE]  ← Adjusted from 14.8 CRITICAL
├─ LOCATION: src/io/pattern_output.rs:67 format_pattern_type()
├─ COMPLEXITY: cyclomatic=15, cognitive=3 → weighted=5.4
├─ 📋 CONTEXT: Formatter (confidence: 85%)
├─ 💡 This is output formatting code. High cyclomatic complexity is typical for
│     formatters with many output variants. Focus on cognitive complexity and
│     consider builder pattern if deeply nested.
├─ SUGGESTIONS:
│  1. If complexity is from nesting (not just cases), consider builder pattern
│  2. Extract format helpers for repeated patterns
│  3. Use match expressions for clean exhaustive formatting
├─ PATTERNS:
│  • Builder pattern for complex output
│  • Template method for format variants
└─ RESOURCES:
   • colored::Colorize for terminal formatting
   • serde for structured output
```

### Architecture Changes

**New Modules**:
- `src/analysis/context_detection.rs` - Context detection engine
- `src/priority/recommendations/context_aware.rs` - Context-specific recommendations
- `src/priority/recommendations/templates.rs` - Recommendation templates

**Modified Modules**:
- `src/priority/scoring/mod.rs` - Integrate context-based severity adjustment
- `src/priority/formatter.rs` - Display contextual recommendations
- `src/config.rs` - Configuration for context detection

### Data Structures

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualRecommendation {
    pub context: FunctionContext,
    pub explanation: String,
    pub suggestions: Vec<String>,
    pub patterns: Vec<String>,
    pub examples: Vec<String>,
    pub severity: Severity,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Low,
    Moderate,
    High,
    Critical,
}
```

### Configuration

```toml
[recommendations.context_aware]
enabled = true
min_confidence = 0.6  # Minimum confidence to show context

# Severity adjustments per context
[recommendations.context_aware.adjustments]
formatter = 0.6
parser = 0.8
cli_handler = 0.7
state_machine = 0.75
generic = 1.0

[recommendations.context_aware.detection]
# Enable specific detection methods
use_name_patterns = true
use_file_patterns = true
use_ast_patterns = true
use_import_patterns = true
```

## Dependencies

- **Prerequisites**:
  - Spec 118 (Pure Mapping Detection) - complementary pattern detection
  - Spec 121 (Cognitive Weighting) - uses complexity metrics
- **Affected Components**:
  - Recommendation generation
  - Output formatting
  - Scoring/severity calculation
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

**Context Detection Tests**:
```rust
#[test]
fn detects_formatter_by_name() {
    let function = create_test_function("format_output");
    let context = detect_context(&function, Path::new("src/output.rs"));

    assert_eq!(context.context, FunctionContext::Formatter);
    assert!(context.confidence > 0.6);
}

#[test]
fn detects_parser_by_imports() {
    let ast = parse_with_imports(vec!["nom", "pest"]);
    let function = create_test_function("parse_input");
    let context = detect_context_with_ast(&function, &ast);

    assert_eq!(context.context, FunctionContext::Parser);
}

#[test]
fn adjusts_severity_for_formatters() {
    let function = create_complex_formatter(15, 3);  // High cyclo, low cognitive
    let context = ContextAnalysis {
        context: FunctionContext::Formatter,
        confidence: 0.85,
        detected_signals: vec![],
    };

    let recommendation = generate_recommendation(&function, &context);
    assert!(matches!(recommendation.severity, Severity::Moderate | Severity::Low));
}
```

### Integration Tests

```rust
#[test]
fn end_to_end_contextual_recommendations() {
    let config = DebtmapConfig::default();
    let analysis = analyze_file("src/io/pattern_output.rs", &config);

    let format_fn = analysis.find_function("format_pattern_type").unwrap();
    assert_eq!(format_fn.context.context, FunctionContext::Formatter);
    assert!(!format_fn.recommendation.suggestions.is_empty());
    assert!(format_fn.recommendation.explanation.contains("formatting"));
}
```

### Context Detection Accuracy Tests

```rust
#[test]
fn context_detection_accuracy() {
    let test_cases = load_labeled_test_cases();  // Hand-labeled ground truth
    let mut correct = 0;

    for case in test_cases {
        let detected = detect_context(&case.function, &case.file_path);
        if detected.context == case.expected_context {
            correct += 1;
        }
    }

    let accuracy = correct as f64 / test_cases.len() as f64;
    assert!(accuracy > 0.80, "Accuracy: {:.2}%", accuracy * 100.0);
}
```

## Documentation Requirements

### User Documentation

```markdown
## Context-Aware Recommendations

Debtmap provides specialized recommendations based on code context:

### Supported Contexts

- **Formatters**: Output generation, rendering, display
- **Parsers**: Input processing, parsing, decoding
- **CLI Handlers**: Command handling, orchestration
- **State Machines**: State transitions, state handling
- **Configuration**: Config parsing, validation

Each context receives tailored advice:
- Why the complexity may be acceptable
- Domain-specific refactoring patterns
- Relevant libraries and tools
- Adjusted severity (formatters get lower severity)

**Example**: A formatter with high cyclomatic but low cognitive complexity
will be marked MODERATE instead of CRITICAL, with suggestions for builder
pattern if needed.
```

## Implementation Notes

### Detection Heuristics

**Priority Order**:
1. Name patterns (highest confidence: 0.9)
2. File location (high confidence: 0.8)
3. AST patterns (medium confidence: 0.7)
4. Import patterns (lower confidence: 0.6)

**Combination Logic**:
- Multiple signals multiplicatively increase confidence
- Conflicting signals reduce confidence
- Require min 0.6 confidence to show context

### False Positive Handling

If context detection is uncertain:
- Show generic recommendations
- Don't adjust severity
- Log detection signals for debugging

## Success Metrics

- Context detection accuracy >80%
- User feedback: Recommendations "more relevant"
- Reduced false positive rate (combined with other specs)
- Increased engagement with recommendations
- Educational value: Users learn patterns

## Future Enhancements

- **More contexts**: Database access, networking, crypto, etc.
- **Custom contexts**: Allow user-defined contexts
- **Learning system**: Improve detection from user feedback
- **Code examples**: Link to actual code examples in popular projects
- **Interactive tutorials**: Step-by-step refactoring guides
