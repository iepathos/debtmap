use crate::refactoring::{
    ComplexityLevel, EffortEstimate, FunctionalPattern, Priority, QualityAssessment,
    Recommendation, RefactoringAnalysis, RefactoringExample, RefactoringOpportunity,
};

#[derive(Default)]
pub struct RefactoringGuidanceGenerator;

impl RefactoringGuidanceGenerator {
    pub fn new() -> Self {
        Self
    }

    fn format_complexity_level(level: &ComplexityLevel) -> &'static str {
        match level {
            ComplexityLevel::Low => "LOW",
            ComplexityLevel::Moderate => "MODERATE",
            ComplexityLevel::High => "HIGH",
            ComplexityLevel::Severe => "SEVERE",
        }
    }

    fn format_refactoring_strategy(level: &ComplexityLevel) -> &'static str {
        match level {
            ComplexityLevel::Moderate => "direct functional transformation",
            ComplexityLevel::High => "decompose-then-transform",
            ComplexityLevel::Severe => "architectural refactoring",
            _ => "no",
        }
    }

    fn format_benefits_list(benefits: &[String]) -> String {
        if benefits.is_empty() {
            String::new()
        } else {
            let mut output = String::from("BENEFITS:\n");
            for benefit in benefits {
                output.push_str(&format!("  • {}\n", benefit));
            }
            output
        }
    }

    fn get_priority_icon(priority: &Priority) -> &'static str {
        match priority {
            Priority::Critical => "🔴",
            Priority::High => "🟠",
            Priority::Medium => "🟡",
            Priority::Low => "🟢",
        }
    }

    fn get_effort_string(effort: &EffortEstimate) -> &'static str {
        match effort {
            EffortEstimate::Trivial => "< 15 min",
            EffortEstimate::Low => "15-60 min",
            EffortEstimate::Medium => "1-4 hours",
            EffortEstimate::High => "4-8 hours",
            EffortEstimate::Significant => "> 8 hours",
        }
    }

    pub fn generate_guidance(&self, analysis: &RefactoringAnalysis) -> String {
        let mut output = String::new();

        // Add header based on quality assessment
        if analysis.quality_assessment.overall_score > 0.8 {
            output.push_str(&format!("✓ Good Example: {}\n", analysis.function_name));
            output.push_str(&self.format_strengths(&analysis.quality_assessment));
        } else {
            output.push_str(&format!(
                "⚡ Refactoring Opportunity: {}\n",
                analysis.function_name
            ));
            output.push_str(&self.format_opportunities(&analysis.refactoring_opportunities));
        }

        // Add recommendations
        if !analysis.recommendations.is_empty() {
            output.push_str("\n## Recommendations\n\n");
            for rec in &analysis.recommendations {
                output.push_str(&self.format_recommendation(rec));
            }
        }

        output
    }

    fn format_strengths(&self, quality: &QualityAssessment) -> String {
        let mut output = String::new();

        if !quality.strengths.is_empty() {
            output.push_str("Strengths:\n");
            for strength in &quality.strengths {
                output.push_str(&format!("  • {}\n", strength));
            }
        }

        output
    }

    fn format_opportunities(&self, opportunities: &[RefactoringOpportunity]) -> String {
        let mut output = String::new();

        for opportunity in opportunities {
            match opportunity {
                RefactoringOpportunity::ExtractPureFunctions {
                    complexity_level,
                    suggested_functions,
                    functional_patterns,
                    benefits,
                    ..
                } => {
                    output.push_str(&format!(
                        "\n{} Complexity Detected\n",
                        Self::format_complexity_level(complexity_level)
                    ));

                    output.push_str(&format!(
                        "ACTION: Extract {} pure functions using {} strategy\n",
                        suggested_functions.len(),
                        Self::format_refactoring_strategy(complexity_level)
                    ));

                    if !functional_patterns.is_empty() {
                        output.push_str("PATTERNS: ");
                        let patterns: Vec<String> = functional_patterns
                            .iter()
                            .map(|p| self.pattern_to_string(p))
                            .collect();
                        output.push_str(&patterns.join(", "));
                        output.push('\n');
                    }

                    output.push_str(&Self::format_benefits_list(benefits));
                }
                RefactoringOpportunity::ConvertToFunctionalStyle {
                    target_patterns,
                    benefits,
                    ..
                } => {
                    output.push_str("\nFunctional Transformation Opportunity\n");
                    output.push_str("ACTION: Apply functional patterns: ");
                    let patterns: Vec<String> = target_patterns
                        .iter()
                        .map(|p| self.pattern_to_string(p))
                        .collect();
                    output.push_str(&patterns.join(", "));
                    output.push('\n');

                    output.push_str(&Self::format_benefits_list(benefits));
                }
                RefactoringOpportunity::ExtractSideEffects {
                    pure_core,
                    benefits,
                    ..
                } => {
                    output.push_str("\nSide Effect Extraction Needed\n");
                    output.push_str(&format!(
                        "ACTION: Extract pure function '{}' and move I/O to boundaries\n",
                        pure_core.name
                    ));

                    output.push_str(&Self::format_benefits_list(benefits));
                }
            }
        }

        output
    }

    fn format_recommendation(&self, rec: &Recommendation) -> String {
        let mut output = String::new();

        let priority_icon = Self::get_priority_icon(&rec.priority);
        let effort_str = Self::get_effort_string(&rec.effort_estimate);

        output.push_str(&format!(
            "{} {} [Effort: {}]\n",
            priority_icon, rec.title, effort_str
        ));
        output.push_str(&format!("   {}\n", rec.description));

        if let Some(example) = &rec.example {
            output.push_str(&self.format_example(example));
        }

        output.push('\n');
        output
    }

    fn format_example(&self, example: &RefactoringExample) -> String {
        let mut output = String::new();

        output.push_str("\n   Example:\n");
        output.push_str("   Before:\n");
        for line in example.before.lines() {
            output.push_str(&format!("     {}\n", line));
        }
        output.push_str("   After:\n");
        for line in example.after.lines() {
            output.push_str(&format!("     {}\n", line));
        }
        if !example.explanation.is_empty() {
            output.push_str(&format!("   Patterns Applied: {}\n", example.explanation));
        }

        output
    }

    fn monadic_pattern_to_str(pattern: &crate::refactoring::MonadicPattern) -> &'static str {
        match pattern {
            crate::refactoring::MonadicPattern::Option => "Option monad",
            crate::refactoring::MonadicPattern::Result => "Result monad",
            crate::refactoring::MonadicPattern::Future => "Future monad",
            crate::refactoring::MonadicPattern::State => "State monad",
        }
    }

    fn pattern_to_string(&self, pattern: &FunctionalPattern) -> String {
        match pattern {
            FunctionalPattern::MapOverLoop => "Replace loops with map",
            FunctionalPattern::FilterPredicate => "Extract filter predicates",
            FunctionalPattern::FoldAccumulation => "Use fold for aggregation",
            FunctionalPattern::PatternMatchOverIfElse => "Pattern matching",
            FunctionalPattern::ComposeFunctions => "Compose functions",
            FunctionalPattern::PartialApplication => "Partial application",
            FunctionalPattern::Monadic(m) => Self::monadic_pattern_to_str(m),
            FunctionalPattern::Pipeline => "Function pipeline",
            FunctionalPattern::Recursion => "Recursion",
        }
        .to_string()
    }
}

pub struct EducationalContentGenerator;

impl EducationalContentGenerator {
    pub fn generate_functional_programming_tips() -> Vec<String> {
        vec![
            "💡 Pure functions have no side effects and always return the same output for the same input".to_string(),
            "💡 Use map() to transform collections instead of for loops with push()".to_string(),
            "💡 Use filter() to select items instead of if statements in loops".to_string(),
            "💡 Use fold() to aggregate values instead of mutable accumulators".to_string(),
            "💡 Keep I/O operations at the boundaries of your application".to_string(),
            "💡 Compose small, focused functions to build complex behavior".to_string(),
            "💡 Prefer immutable data structures to prevent unexpected mutations".to_string(),
            "💡 Use Result<T, E> for error handling instead of exceptions".to_string(),
            "💡 Pattern matching is more expressive than if-else chains".to_string(),
            "💡 Property-based testing works great with pure functions".to_string(),
        ]
    }

    pub fn explain_functional_benefits() -> String {
        r#"
## Why Extract Pure Functions?

Pure functions provide several key benefits:

1. **Testability**: Pure functions are trivial to test - just provide input and assert output
2. **Composability**: Pure functions can be easily combined to create complex behavior
3. **Reasoning**: No hidden state or side effects makes code easier to understand
4. **Parallelization**: Pure functions are thread-safe by default
5. **Debugging**: Predictable behavior makes bugs easier to find and fix
6. **Reusability**: Pure functions can be used in any context

## Functional Core / Imperative Shell

This architecture pattern separates your application into:
- **Functional Core**: Pure business logic with no side effects
- **Imperative Shell**: Thin layer handling I/O and orchestration

This gives you the best of both worlds: testable business logic and necessary I/O operations.
"#
        .to_string()
    }
}
