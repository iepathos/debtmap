use super::if_else_analyzer::{IfElseChain, RefactoringPattern};
use super::recursive_detector::MatchLocation;
use super::threshold_manager::{ComplexityLevel, ComplexityThresholds};
use crate::core::FunctionMetrics;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::path::PathBuf;

/// Enhanced complexity message with specific details and recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedComplexityMessage {
    pub summary: String,
    pub details: Vec<ComplexityDetail>,
    pub recommendations: Vec<ActionableRecommendation>,
    pub code_examples: Option<RefactoringExample>,
    pub complexity_breakdown: ComplexityBreakdown,
}

/// Specific complexity issue detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityDetail {
    pub issue_type: ComplexityIssueType,
    pub location: SourceLocation,
    pub description: String,
    pub severity: Severity,
}

/// Types of complexity issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplexityIssueType {
    ExcessiveMatchArms {
        count: usize,
        suggested_max: usize,
    },
    DeepNesting {
        depth: u32,
        suggested_max: u32,
    },
    LongIfElseChain {
        count: usize,
        suggested_pattern: RefactoringPattern,
    },
    HighCyclomaticComplexity {
        value: u32,
        sources: Vec<String>,
    },
    HighCognitiveComplexity {
        value: u32,
        sources: Vec<String>,
    },
    MultipleComplexPatterns {
        patterns: Vec<String>,
    },
}

/// Source location information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: PathBuf,
    pub line: usize,
    pub column: Option<usize>,
}

/// Severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Actionable recommendation for improvement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionableRecommendation {
    pub title: String,
    pub description: String,
    pub effort: EstimatedEffort,
    pub pattern: RefactoringPattern,
    pub code_example: Option<String>,
}

/// Refactoring example with before/after code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringExample {
    pub before: String,
    pub after: String,
    pub explanation: String,
    pub estimated_effort: EstimatedEffort,
}

/// Estimated refactoring effort
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EstimatedEffort {
    Low,    // < 30 minutes
    Medium, // 30 minutes - 2 hours
    High,   // 2 hours - 1 day
}

/// Breakdown of complexity sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityBreakdown {
    pub cyclomatic_sources: Vec<String>,
    pub cognitive_sources: Vec<String>,
    pub match_complexity: u32,
    pub if_else_complexity: u32,
    pub loop_complexity: u32,
    pub nesting_penalty: u32,
    pub total_complexity: u32,
}

/// Generate enhanced complexity message based on analysis
pub fn generate_enhanced_message(
    metrics: &FunctionMetrics,
    matches: &[MatchLocation],
    if_else_chains: &[IfElseChain],
    thresholds: &ComplexityThresholds,
) -> EnhancedComplexityMessage {
    let mut details = Vec::new();
    let mut recommendations = Vec::new();

    // Analyze match expressions
    analyze_match_complexity(
        metrics,
        matches,
        thresholds,
        &mut details,
        &mut recommendations,
    );

    // Analyze if-else chains
    analyze_if_else_chains(
        metrics,
        if_else_chains,
        thresholds,
        &mut details,
        &mut recommendations,
    );

    // Analyze general complexity
    analyze_general_complexity(metrics, thresholds, &mut details, &mut recommendations);

    let summary = generate_summary(&details, metrics, thresholds);
    let code_examples = select_best_example(&recommendations);
    let complexity_breakdown = calculate_breakdown(metrics, matches, if_else_chains);

    EnhancedComplexityMessage {
        summary,
        details,
        recommendations,
        code_examples,
        complexity_breakdown,
    }
}

fn analyze_match_complexity(
    metrics: &FunctionMetrics,
    matches: &[MatchLocation],
    thresholds: &ComplexityThresholds,
    details: &mut Vec<ComplexityDetail>,
    recommendations: &mut Vec<ActionableRecommendation>,
) {
    let total_match_arms: usize = matches.iter().map(|m| m.arms).sum();

    if total_match_arms > thresholds.minimum_match_arms * 2 {
        details.push(ComplexityDetail {
            issue_type: ComplexityIssueType::ExcessiveMatchArms {
                count: total_match_arms,
                suggested_max: thresholds.minimum_match_arms * 2,
            },
            location: SourceLocation {
                file: metrics.file.clone(),
                line: matches.first().map(|m| m.line).unwrap_or(metrics.line),
                column: None,
            },
            description: format!(
                "Function contains {} match expressions with {} total arms. Consider extracting match logic to separate functions or using a lookup table.",
                matches.len(),
                total_match_arms
            ),
            severity: if total_match_arms > thresholds.minimum_match_arms * 3 {
                Severity::High
            } else {
                Severity::Medium
            },
        });

        recommendations.push(ActionableRecommendation {
            title: "Extract Match Logic".to_string(),
            description: "Break large match expressions into smaller, focused functions. Each function should handle a specific subset of cases.".to_string(),
            effort: EstimatedEffort::Medium,
            pattern: RefactoringPattern::ExtractMethod,
            code_example: Some(generate_match_extraction_example()),
        });
    }

    // Check for deeply nested matches
    for match_loc in matches {
        if match_loc.context.nesting_depth > 2 {
            details.push(ComplexityDetail {
                issue_type: ComplexityIssueType::DeepNesting {
                    depth: match_loc.context.nesting_depth,
                    suggested_max: 2,
                },
                location: SourceLocation {
                    file: metrics.file.clone(),
                    line: match_loc.line,
                    column: None,
                },
                description: format!(
                    "Match expression at nesting depth {} (recommended max: 2). Deep nesting makes code harder to understand.",
                    match_loc.context.nesting_depth
                ),
                severity: Severity::Medium,
            });
        }
    }
}

fn analyze_if_else_chains(
    metrics: &FunctionMetrics,
    if_else_chains: &[IfElseChain],
    thresholds: &ComplexityThresholds,
    details: &mut Vec<ComplexityDetail>,
    recommendations: &mut Vec<ActionableRecommendation>,
) {
    for chain in if_else_chains {
        if chain.length >= thresholds.minimum_if_else_chain {
            let pattern = chain.suggest_refactoring();

            details.push(ComplexityDetail {
                issue_type: ComplexityIssueType::LongIfElseChain {
                    count: chain.length,
                    suggested_pattern: pattern.clone(),
                },
                location: SourceLocation {
                    file: metrics.file.clone(),
                    line: chain.start_line,
                    column: None,
                },
                description: format!(
                    "If-else chain with {} conditions could be simplified using {}",
                    chain.length,
                    pattern.description()
                ),
                severity: if chain.length > thresholds.minimum_if_else_chain * 2 {
                    Severity::High
                } else {
                    Severity::Medium
                },
            });

            recommendations.push(ActionableRecommendation {
                title: format!("Refactor with {}", pattern.name()),
                description: pattern.description(),
                effort: pattern.estimated_effort(),
                pattern,
                code_example: Some(generate_if_else_refactoring_example(chain)),
            });
        }
    }
}

fn analyze_general_complexity(
    metrics: &FunctionMetrics,
    thresholds: &ComplexityThresholds,
    details: &mut Vec<ComplexityDetail>,
    recommendations: &mut Vec<ActionableRecommendation>,
) {
    // Check cyclomatic complexity
    if metrics.cyclomatic >= thresholds.minimum_cyclomatic_complexity * 2 {
        let sources = identify_cyclomatic_sources(metrics);
        details.push(ComplexityDetail {
            issue_type: ComplexityIssueType::HighCyclomaticComplexity {
                value: metrics.cyclomatic,
                sources: sources.clone(),
            },
            location: SourceLocation {
                file: metrics.file.clone(),
                line: metrics.line,
                column: None,
            },
            description: format!(
                "High cyclomatic complexity of {} (threshold: {}). Main sources: {}",
                metrics.cyclomatic,
                thresholds.minimum_cyclomatic_complexity,
                sources.join(", ")
            ),
            severity: Severity::High,
        });

        recommendations.push(ActionableRecommendation {
            title: "Reduce Branching Complexity".to_string(),
            description: "Extract complex conditions into well-named functions. Use early returns to reduce nesting.".to_string(),
            effort: EstimatedEffort::Medium,
            pattern: RefactoringPattern::GuardClauses,
            code_example: Some(generate_guard_clause_example()),
        });
    }

    // Check cognitive complexity
    if metrics.cognitive >= thresholds.minimum_cognitive_complexity * 2 {
        let sources = identify_cognitive_sources(metrics);
        details.push(ComplexityDetail {
            issue_type: ComplexityIssueType::HighCognitiveComplexity {
                value: metrics.cognitive,
                sources: sources.clone(),
            },
            location: SourceLocation {
                file: metrics.file.clone(),
                line: metrics.line,
                column: None,
            },
            description: format!(
                "High cognitive complexity of {} (threshold: {}). Main sources: {}",
                metrics.cognitive,
                thresholds.minimum_cognitive_complexity,
                sources.join(", ")
            ),
            severity: Severity::High,
        });
    }
}

fn generate_summary(
    details: &[ComplexityDetail],
    metrics: &FunctionMetrics,
    thresholds: &ComplexityThresholds,
) -> String {
    let level = thresholds.get_complexity_level(metrics);
    let issue_count = details.len();
    let high_severity_count = details
        .iter()
        .filter(|d| d.severity == Severity::High)
        .count();

    match level {
        ComplexityLevel::Trivial => {
            format!("Function '{}' has acceptable complexity", metrics.name)
        }
        ComplexityLevel::Moderate => {
            format!(
                "Function '{}' has moderate complexity with {} issue(s) to consider",
                metrics.name, issue_count
            )
        }
        ComplexityLevel::High => {
            format!(
                "Function '{}' has high complexity with {} issue(s), {} high severity",
                metrics.name, issue_count, high_severity_count
            )
        }
        ComplexityLevel::Excessive => {
            format!(
                "Function '{}' has excessive complexity requiring immediate refactoring ({} issues)",
                metrics.name, issue_count
            )
        }
    }
}

fn calculate_breakdown(
    metrics: &FunctionMetrics,
    matches: &[MatchLocation],
    if_else_chains: &[IfElseChain],
) -> ComplexityBreakdown {
    let match_complexity: u32 = matches.iter().map(|m| m.complexity).sum();
    let if_else_complexity: u32 = if_else_chains.iter().map(|c| c.length as u32).sum();

    ComplexityBreakdown {
        cyclomatic_sources: identify_cyclomatic_sources(metrics),
        cognitive_sources: identify_cognitive_sources(metrics),
        match_complexity,
        if_else_complexity,
        loop_complexity: 0, // Would need AST analysis to determine
        nesting_penalty: 0, // Would need AST analysis to determine
        total_complexity: metrics.cyclomatic + metrics.cognitive,
    }
}

fn identify_cyclomatic_sources(_metrics: &FunctionMetrics) -> Vec<String> {
    // In a real implementation, this would analyze the AST
    vec![
        "if/else statements".to_string(),
        "match expressions".to_string(),
        "loops".to_string(),
    ]
}

fn identify_cognitive_sources(_metrics: &FunctionMetrics) -> Vec<String> {
    // In a real implementation, this would analyze the AST
    vec![
        "nested control flow".to_string(),
        "complex conditions".to_string(),
        "cognitive load from branching".to_string(),
    ]
}

fn select_best_example(recommendations: &[ActionableRecommendation]) -> Option<RefactoringExample> {
    // Select the most impactful example
    if recommendations.is_empty() {
        return None;
    }

    Some(RefactoringExample {
        before: "// Complex nested if-else\nif condition1 {\n    if condition2 {\n        // deep nesting\n    }\n}".to_string(),
        after: "// Using guard clauses\nif !condition1 {\n    return early;\n}\nif !condition2 {\n    return early;\n}\n// main logic".to_string(),
        explanation: "Guard clauses reduce nesting and improve readability".to_string(),
        estimated_effort: EstimatedEffort::Low,
    })
}

fn generate_match_extraction_example() -> String {
    r#"// Before: Large match in single function
match value {
    Type::A => { /* 20 lines */ },
    Type::B => { /* 30 lines */ },
    Type::C => { /* 25 lines */ },
}

// After: Extract to separate handlers
match value {
    Type::A => handle_type_a(data),
    Type::B => handle_type_b(data),
    Type::C => handle_type_c(data),
}"#
    .to_string()
}

fn generate_if_else_refactoring_example(chain: &IfElseChain) -> String {
    let pattern = chain.suggest_refactoring();
    match pattern {
        RefactoringPattern::MatchExpression => r#"// Before: Long if-else chain
if value == "a" {
    return 1;
} else if value == "b" {
    return 2;
} else if value == "c" {
    return 3;
}

// After: Match expression
match value {
    "a" => 1,
    "b" => 2,
    "c" => 3,
    _ => 0,
}"#
        .to_string(),
        RefactoringPattern::LookupTable => r#"// Before: Repetitive if-else
if key == "option1" {
    return value1;
} else if key == "option2" {
    return value2;
}

// After: Lookup table
let table = HashMap::from([
    ("option1", value1),
    ("option2", value2),
]);
table.get(key).copied().unwrap_or_default()"#
            .to_string(),
        _ => "// Consider refactoring this pattern".to_string(),
    }
}

fn generate_guard_clause_example() -> String {
    r#"// Before: Nested conditions
if is_valid {
    if has_permission {
        if !is_expired {
            // actual logic
        }
    }
}

// After: Guard clauses
if !is_valid {
    return Err("Invalid");
}
if !has_permission {
    return Err("No permission");
}
if is_expired {
    return Err("Expired");
}
// actual logic with no nesting"#
        .to_string()
}

/// Format enhanced message for display
pub fn format_enhanced_message(message: &EnhancedComplexityMessage) -> String {
    let mut output = String::new();

    writeln!(output, "\n{}", message.summary).unwrap();
    writeln!(output, "{}", "=".repeat(60)).unwrap();

    // Details section
    if !message.details.is_empty() {
        writeln!(output, "\nCOMPLEXITY ISSUES:").unwrap();
        for (i, detail) in message.details.iter().enumerate() {
            let severity_icon = match detail.severity {
                Severity::Low => "[INFO]",
                Severity::Medium => "[WARNING]",
                Severity::High => "[ERROR]",
                Severity::Critical => "[!]",
            };
            writeln!(
                output,
                "  {}. {} {}",
                i + 1,
                severity_icon,
                detail.description
            )
            .unwrap();
            writeln!(
                output,
                "     Location: {}:{}",
                detail.location.file.display(),
                detail.location.line
            )
            .unwrap();
        }
    }

    // Recommendations section
    if !message.recommendations.is_empty() {
        writeln!(output, "\n[TIP] RECOMMENDATIONS:").unwrap();
        for rec in &message.recommendations {
            writeln!(output, "  • {}", rec.title).unwrap();
            writeln!(output, "    {}", rec.description).unwrap();
            writeln!(output, "    Effort: {:?}", rec.effort).unwrap();
        }
    }

    // Code example
    if let Some(example) = &message.code_examples {
        writeln!(output, "\n[REFACTORING EXAMPLE]").unwrap();
        writeln!(output, "  {}", example.explanation).unwrap();
        writeln!(output, "\n  Before:").unwrap();
        for line in example.before.lines() {
            writeln!(output, "    {}", line).unwrap();
        }
        writeln!(output, "\n  After:").unwrap();
        for line in example.after.lines() {
            writeln!(output, "    {}", line).unwrap();
        }
    }

    // Complexity breakdown
    writeln!(output, "\n📈 COMPLEXITY BREAKDOWN:").unwrap();
    writeln!(
        output,
        "  Total: {} (Cyclomatic: {}, Cognitive: {})",
        message.complexity_breakdown.total_complexity,
        message.complexity_breakdown.match_complexity
            + message.complexity_breakdown.if_else_complexity,
        message.complexity_breakdown.nesting_penalty
    )
    .unwrap();

    output
}
