---
number: 25
title: Intelligent Refactoring Guidance System
category: foundation
priority: critical
status: draft
dependencies: [19, 23, 24]
created: 2025-01-13
---

# Specification 25: Intelligent Refactoring Guidance System

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: [19 - Unified Debt Prioritization, 23 - Enhanced Call Graph Analysis, 24 - Refined Risk Scoring Methodology]

## Context

Current debtmap analysis suffers from a fundamental flaw: it detects problems without understanding code patterns or providing actionable guidance. This leads to:

1. **False Positives**: Valid patterns (I/O wrappers, trait implementations) flagged as "technical debt"
2. **Generic Advice**: Vague recommendations like "address technical debt" or "reduce complexity"
3. **No Refactoring Guidance**: Users know something is flagged but not how to improve it
4. **Pattern Blindness**: Cannot distinguish between genuinely problematic code and well-structured code that could be enhanced

For example, `print_risk_function()` is correctly flagged as having improvement potential, but the current system doesn't recognize that the formatting logic is already properly extracted to `format_risk_function()` - making this a good example of clean architecture rather than a problem.

## Objective

Transform debtmap from a generic "technical debt detector" into an **intelligent refactoring advisor** that recognizes code patterns, distinguishes between problems and valid designs, and provides specific, actionable guidance for code improvements with clear explanations of benefits.

## Requirements

### Functional Requirements

1. **Pattern Recognition System**
   - Identify common code patterns (I/O orchestration, pure logic, formatting, etc.)
   - Recognize valid architectural patterns vs. anti-patterns
   - Distinguish between "working code" and "code that could be improved"
   - Classify function roles and expected characteristics

2. **Refactoring Opportunity Detection**
   - Extract formatting logic from I/O functions
   - Identify complex logic mixed with side effects
   - Detect untested business logic
   - Find opportunities for pure function extraction
   - Identify coupling reduction opportunities

3. **Specific Improvement Guidance**
   - Provide concrete refactoring techniques for each issue
   - Explain the benefits of suggested improvements
   - Show before/after examples where helpful
   - Estimate effort and impact of changes

4. **Context-Aware Recommendations**
   - Adjust recommendations based on function role
   - Consider existing test coverage and architecture
   - Factor in team practices and codebase conventions
   - Prioritize improvements by impact and effort

5. **Educational Insights**
   - Explain why patterns are problematic or beneficial
   - Link to refactoring techniques and best practices
   - Help teams learn better patterns over time
   - Provide examples of good patterns to follow

### Non-Functional Requirements

1. **Accuracy**: 95% of recommendations should be genuinely helpful
2. **Actionability**: Every recommendation includes specific steps
3. **Educational Value**: Output helps developers learn better patterns
4. **Performance**: Pattern analysis adds <15% to total analysis time

## Acceptance Criteria

- [ ] Pattern recognition correctly identifies I/O orchestration, pure logic, formatting, and mixed concern functions
- [ ] Valid patterns (like trait implementations) no longer flagged as problems
- [ ] Specific refactoring techniques suggested for each improvement opportunity
- [ ] Clear explanations of why changes improve code quality
- [ ] Integration with existing priority scoring and risk analysis
- [ ] Reduced false positive rate by 85% compared to current system
- [ ] Output includes concrete "before/after" guidance where helpful
- [ ] Educational explanations help teams understand better patterns
- [ ] Performance impact under 15% of total analysis time
- [ ] Comprehensive test suite with diverse code pattern examples

## Technical Details

### Implementation Approach

1. **Pattern Recognition Engine**
```rust
pub struct PatternRecognitionEngine {
    pattern_matchers: Vec<Box<dyn PatternMatcher>>,
    function_classifier: FunctionRoleClassifier,
    refactoring_advisor: RefactoringAdvisor,
}

impl PatternRecognitionEngine {
    pub fn analyze_function(&self, function: &FunctionAnalysis) -> AnalysisResult {
        let role = self.function_classifier.classify(function);
        let patterns = self.identify_patterns(function);
        let opportunities = self.find_refactoring_opportunities(function, &role, &patterns);
        
        AnalysisResult {
            function_role: role,
            detected_patterns: patterns,
            refactoring_opportunities: opportunities,
            quality_assessment: self.assess_quality(function, &patterns),
            recommendations: self.generate_recommendations(&opportunities),
        }
    }
    
    fn identify_patterns(&self, function: &FunctionAnalysis) -> Vec<DetectedPattern> {
        self.pattern_matchers
            .iter()
            .filter_map(|matcher| matcher.match_pattern(function))
            .collect()
    }
}
```

2. **Function Role Classification**
```rust
pub enum FunctionRole {
    PureLogic {
        complexity_tolerance: u32,
        testing_expectation: TestingExpectation,
    },
    IOOrchestrator {
        expected_patterns: Vec<OrchestrationPattern>,
        complexity_tolerance: u32,
    },
    FormattingFunction {
        input_types: Vec<Type>,
        output_type: Type,
        testability_importance: TestabilityImportance,
    },
    TraitImplementation {
        trait_name: String,
        testing_strategy: TraitTestingStrategy,
    },
    FrameworkCallback {
        framework: Framework,
        callback_type: CallbackType,
    },
}

pub struct FunctionRoleClassifier {
    io_detectors: Vec<IoDetector>,
    formatting_detectors: Vec<FormattingDetector>,
    trait_analyzers: Vec<TraitAnalyzer>,
}

impl FunctionRoleClassifier {
    pub fn classify(&self, function: &FunctionAnalysis) -> FunctionRole {
        // Priority order matters - most specific first
        if let Some(trait_info) = self.detect_trait_implementation(function) {
            return FunctionRole::TraitImplementation {
                trait_name: trait_info.trait_name,
                testing_strategy: TraitTestingStrategy::TestThroughCallers,
            };
        }
        
        if let Some(io_info) = self.detect_io_orchestration(function) {
            return FunctionRole::IOOrchestrator {
                expected_patterns: io_info.patterns,
                complexity_tolerance: 5, // I/O functions can be more complex
            };
        }
        
        if let Some(formatting_info) = self.detect_formatting_function(function) {
            return FunctionRole::FormattingFunction {
                input_types: formatting_info.inputs,
                output_type: formatting_info.output,
                testability_importance: TestabilityImportance::High,
            };
        }
        
        // Default to pure logic with strict expectations
        FunctionRole::PureLogic {
            complexity_tolerance: 3,
            testing_expectation: TestingExpectation::HighCoverage,
        }
    }
}
```

3. **Refactoring Opportunity Detection**
```rust
pub struct RefactoringAdvisor {
    opportunity_detectors: Vec<Box<dyn RefactoringDetector>>,
    pattern_library: PatternLibrary,
}

pub trait RefactoringDetector {
    fn detect_opportunities(&self, function: &FunctionAnalysis, role: &FunctionRole) -> Vec<RefactoringOpportunity>;
    fn priority(&self) -> Priority;
}

pub struct ExtractFormattingDetector;
impl RefactoringDetector for ExtractFormattingDetector {
    fn detect_opportunities(&self, function: &FunctionAnalysis, role: &FunctionRole) -> Vec<RefactoringOpportunity> {
        if let FunctionRole::IOOrchestrator { .. } = role {
            if self.has_embedded_formatting_logic(function) {
                return vec![RefactoringOpportunity::ExtractFormattingLogic {
                    current_function: function.name.clone(),
                    suggested_pure_function: self.suggest_formatting_function_name(function),
                    benefits: vec![
                        "Formatting logic becomes unit testable",
                        "I/O function becomes simpler and more focused",
                        "Formatting can be reused in other contexts",
                    ],
                    technique: RefactoringTechnique::ExtractMethod,
                    effort_estimate: EffortEstimate::Low,
                    example: self.generate_before_after_example(function),
                }];
            }
        }
        vec![]
    }
}

pub struct SeparateConcernsDetector;
impl RefactoringDetector for SeparateConcernsDetector {
    fn detect_opportunities(&self, function: &FunctionAnalysis, role: &FunctionRole) -> Vec<RefactoringOpportunity> {
        if matches!(role, FunctionRole::IOOrchestrator { .. }) {
            let business_logic_blocks = self.find_business_logic_blocks(function);
            if !business_logic_blocks.is_empty() {
                return vec![RefactoringOpportunity::SeparateConcerns {
                    mixed_function: function.name.clone(),
                    business_logic_blocks,
                    suggested_pure_functions: self.suggest_extracted_functions(&business_logic_blocks),
                    benefits: vec![
                        "Business logic becomes testable without I/O mocking",
                        "Clearer separation of concerns",
                        "Logic can be reused independently",
                    ],
                    technique: RefactoringTechnique::ExtractClass,
                    effort_estimate: EffortEstimate::Medium,
                }];
            }
        }
        vec![]
    }
}
```

4. **Intelligent Guidance Generation**
```rust
pub enum RefactoringOpportunity {
    ExtractFormattingLogic {
        current_function: String,
        suggested_pure_function: String,
        benefits: Vec<&'static str>,
        technique: RefactoringTechnique,
        effort_estimate: EffortEstimate,
        example: Option<BeforeAfterExample>,
    },
    SeparateConcerns {
        mixed_function: String,
        business_logic_blocks: Vec<LogicBlock>,
        suggested_pure_functions: Vec<String>,
        benefits: Vec<&'static str>,
        technique: RefactoringTechnique,
        effort_estimate: EffortEstimate,
    },
    AddTestCoverage {
        untested_function: String,
        complexity_score: u32,
        critical_paths: Vec<String>,
        testing_strategy: TestingStrategy,
        effort_estimate: EffortEstimate,
    },
    ReduceComplexity {
        complex_function: String,
        current_complexity: u32,
        target_complexity: u32,
        techniques: Vec<RefactoringTechnique>,
        effort_estimate: EffortEstimate,
    },
}

pub struct BeforeAfterExample {
    pub before_code: String,
    pub after_code: String,
    pub explanation: String,
}

pub enum RefactoringTechnique {
    ExtractMethod,
    ExtractClass,
    IntroduceParameterObject,
    ReplaceConditionalWithPolymorphism,
    DecomposeConditional,
    ConsolidateConditionalExpression,
    ExtractVariable,
    RenameMethod,
}

pub enum EffortEstimate {
    Trivial,  // < 15 minutes
    Low,      // 15-60 minutes  
    Medium,   // 1-4 hours
    High,     // 4-8 hours
    Significant, // > 8 hours
}
```

5. **Pattern-Aware Output**
```rust
pub struct IntelligentOutputFormatter {
    role_explanations: RoleExplanationProvider,
    example_generator: ExampleGenerator,
    benefit_calculator: BenefitCalculator,
}

impl IntelligentOutputFormatter {
    pub fn format_analysis(&self, result: &AnalysisResult) -> FormattedOutput {
        match &result.refactoring_opportunities[..] {
            [] => self.format_good_example(result),
            opportunities => self.format_improvement_opportunities(result, opportunities),
        }
    }
    
    fn format_good_example(&self, result: &AnalysisResult) -> FormattedOutput {
        FormattedOutput {
            status: OutputStatus::GoodExample,
            title: format!("✓ Good Example: {}", result.function_name),
            explanation: self.explain_why_good(result),
            role_context: self.explain_role(&result.function_role),
            patterns: self.highlight_good_patterns(&result.detected_patterns),
        }
    }
    
    fn format_improvement_opportunities(&self, result: &AnalysisResult, opportunities: &[RefactoringOpportunity]) -> FormattedOutput {
        FormattedOutput {
            status: OutputStatus::ImprovementOpportunity,
            title: format!("⚡ Refactoring Opportunity: {}", result.function_name),
            opportunities: opportunities.iter().map(|opp| self.format_opportunity(opp)).collect(),
            role_context: self.explain_role(&result.function_role),
            benefits_summary: self.summarize_benefits(opportunities),
        }
    }
}
```

### Architecture Changes

1. **New Module Structure**
   - Create `src/refactoring/` module for pattern recognition and guidance
   - Add `src/refactoring/patterns/` for pattern matching implementations
   - Create `src/refactoring/opportunities/` for refactoring detection
   - Add `src/refactoring/guidance/` for recommendation generation

2. **Integration Points**
   - Replace generic debt scoring in `src/debt/` with pattern-aware analysis
   - Enhance output formatters to use intelligent guidance
   - Integrate with existing priority scoring system
   - Connect to function role classification from spec 19

### Data Structures

```rust
pub struct AnalysisResult {
    pub function_name: String,
    pub function_role: FunctionRole,
    pub detected_patterns: Vec<DetectedPattern>,
    pub refactoring_opportunities: Vec<RefactoringOpportunity>,
    pub quality_assessment: QualityAssessment,
    pub recommendations: Vec<Recommendation>,
}

pub struct DetectedPattern {
    pub pattern_type: PatternType,
    pub confidence: f64,
    pub evidence: PatternEvidence,
    pub assessment: PatternAssessment,
}

pub enum PatternType {
    IOOrchestration(OrchestrationPattern),
    PureFormatting(FormattingPattern),
    MixedConcerns(ConcernMixingPattern),
    TraitImplementation(TraitPattern),
    TestFunction(TestPattern),
}

pub enum PatternAssessment {
    GoodExample {
        strengths: Vec<String>,
        why_good: String,
    },
    ImprovementOpportunity {
        current_issues: Vec<String>,
        potential_benefits: Vec<String>,
        refactoring_suggestions: Vec<RefactoringOpportunity>,
    },
    AntiPattern {
        problems: Vec<String>,
        recommended_patterns: Vec<PatternType>,
        urgency: Urgency,
    },
}

pub struct QualityAssessment {
    pub overall_score: f64,
    pub strengths: Vec<String>,
    pub improvement_areas: Vec<String>,
    pub pattern_compliance: f64,
    pub role_appropriateness: f64,
}
```

## Dependencies

- **Prerequisites**:
  - Spec 19 (Unified Debt Prioritization) for semantic function classification
  - Spec 23 (Enhanced Call Graph Analysis) for accurate usage detection
  - Spec 24 (Refined Risk Scoring Methodology) for evidence-based assessment
- **Affected Components**:
  - `src/debt/` - Replace generic debt detection with pattern-aware analysis
  - `src/io/writers/` - Update output formats for intelligent guidance
  - `src/core/` - Add pattern analysis data structures
  - All existing debt detection tests
- **External Dependencies**: None required

## Testing Strategy

- **Unit Tests**:
  - Test pattern recognition with diverse code examples
  - Validate refactoring opportunity detection accuracy
  - Test guidance generation for different scenarios
  - Verify role classification correctness

- **Integration Tests**:
  - Test with real codebases containing various patterns
  - Validate against manually reviewed refactoring opportunities
  - Compare guidance quality with expert recommendations
  - Performance testing with large codebases

- **Validation Tests**:
  - Expert review of pattern classifications
  - Developer feedback on guidance usefulness
  - Before/after refactoring validation
  - False positive/negative analysis

## Documentation Requirements

- **Code Documentation**:
  - Document all pattern recognition algorithms
  - Explain refactoring opportunity detection logic
  - Document guidance generation strategies
  - Provide pattern matching examples

- **User Documentation**:
  - Pattern recognition guide
  - Refactoring opportunity handbook
  - Best practices for code patterns
  - Example-driven improvement guide

## Implementation Notes

1. **Pattern Library Development**
   - Start with common Rust patterns
   - Build pattern database from real codebases
   - Iterative refinement based on user feedback
   - Community contribution to pattern recognition

2. **Guidance Quality**
   - Focus on actionable, specific recommendations
   - Provide concrete examples where helpful
   - Explain the "why" behind each suggestion
   - Link to refactoring resources and techniques

3. **Performance Optimization**
   - Cache pattern recognition results
   - Parallel pattern analysis where possible
   - Incremental analysis for large codebases
   - Optimize for common pattern detection

## Migration and Compatibility

- **Non-Breaking Changes**:
  - New analysis is addition to existing functionality
  - Existing CLI flags and basic output preserved
  - Enhanced output provides more detail

- **Output Evolution**:
  - Current generic debt items become specific guidance
  - New explanation fields added to output formats
  - Optional verbose mode for detailed pattern analysis

- **User Adoption**:
  - Migration guide from old to new interpretations
  - Side-by-side comparison of old vs. new analysis
  - Training materials for understanding new guidance

## Expected Impact

1. **False Positive Reduction**: 85% fewer irrelevant warnings
2. **Actionability**: Every recommendation includes specific steps
3. **Educational Value**: Teams learn better patterns over time
4. **Developer Satisfaction**: Useful guidance rather than noise
5. **Code Quality**: Targeted improvements based on pattern understanding

This specification transforms debtmap from a problem detector into a refactoring mentor, providing the intelligent guidance that development teams need to continuously improve their codebase.