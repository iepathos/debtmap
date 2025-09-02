use crate::core::FunctionMetrics;
use serde::{Deserialize, Serialize};

pub mod change_tracker;
pub mod pattern_tracker;
pub mod source_tracker;

use self::pattern_tracker::PatternTracker;
use self::source_tracker::{ComplexitySourceType, SourceTracker};

/// Estimated location information for complexity mapping
#[derive(Debug, Clone)]
struct EstimatedComplexityLocation {
    line: u32,
    column: u32,
    span: Option<(u32, u32)>,
    construct_type: String,
    context: String,
}

/// Core attribution engine for complexity source analysis
pub struct AttributionEngine {
    #[allow(dead_code)]
    source_trackers: Vec<Box<dyn SourceTracker>>,
    pattern_tracker: PatternTracker,
}

impl AttributionEngine {
    pub fn new() -> Self {
        Self {
            source_trackers: vec![
                Box::new(source_tracker::LogicalStructureTracker::new()),
                Box::new(source_tracker::FormattingArtifactTracker::new()),
            ],
            pattern_tracker: PatternTracker::new(),
        }
    }

    pub fn attribute(
        &self,
        raw_result: &super::multi_pass::ComplexityResult,
        normalized_result: &super::multi_pass::ComplexityResult,
    ) -> ComplexityAttribution {
        // Calculate logical complexity from normalized result
        let logical_complexity = self.calculate_logical_complexity(normalized_result);

        // Calculate formatting artifacts as difference between raw and normalized
        let formatting_artifacts =
            self.calculate_formatting_artifacts(raw_result, normalized_result);

        // Analyze patterns in the code
        let pattern_complexity = self
            .pattern_tracker
            .analyze_patterns(&normalized_result.functions);

        // Generate source mappings
        let source_mappings = self.generate_source_mappings(&raw_result.functions);

        ComplexityAttribution {
            logical_complexity,
            formatting_artifacts,
            pattern_complexity,
            source_mappings,
        }
    }

    fn calculate_logical_complexity(
        &self,
        normalized_result: &super::multi_pass::ComplexityResult,
    ) -> AttributedComplexity {
        let mut total = 0u32;
        let mut breakdown = Vec::new();

        for func in &normalized_result.functions {
            total += func.cyclomatic;

            breakdown.push(ComplexityComponent {
                source_type: ComplexitySourceType::LogicalStructure {
                    construct_type: LogicalConstruct::Function,
                    nesting_level: func.nesting,
                },
                contribution: func.cyclomatic,
                location: CodeLocation {
                    file: func.file.to_string_lossy().to_string(),
                    line: func.line as u32,
                    column: 0,
                    span: None,
                },
                description: format!("Function: {}", func.name),
                suggestions: if func.cyclomatic > 10 {
                    vec![
                        "Consider breaking down this function".to_string(),
                        "Extract complex conditions into helper functions".to_string(),
                    ]
                } else {
                    vec![]
                },
            });
        }

        AttributedComplexity {
            total,
            breakdown,
            confidence: 0.9, // High confidence for logical complexity
        }
    }

    fn calculate_formatting_artifacts(
        &self,
        raw_result: &super::multi_pass::ComplexityResult,
        normalized_result: &super::multi_pass::ComplexityResult,
    ) -> AttributedComplexity {
        let raw_total = raw_result.total_complexity;
        let normalized_total = normalized_result.total_complexity;

        let artifact_total = if raw_total > normalized_total {
            raw_total - normalized_total
        } else {
            0
        };

        let mut breakdown = Vec::new();

        // Compare function-by-function to identify formatting artifacts
        for (raw_func, norm_func) in raw_result
            .functions
            .iter()
            .zip(normalized_result.functions.iter())
        {
            let diff = if raw_func.cyclomatic > norm_func.cyclomatic {
                raw_func.cyclomatic - norm_func.cyclomatic
            } else {
                0
            };

            if diff > 0 {
                breakdown.push(ComplexityComponent {
                    source_type: ComplexitySourceType::FormattingArtifact {
                        artifact_type: FormattingArtifact::MultilineExpression,
                        severity: ArtifactSeverity::Medium,
                    },
                    contribution: diff,
                    location: CodeLocation {
                        file: raw_func.file.to_string_lossy().to_string(),
                        line: raw_func.line as u32,
                        column: 0,
                        span: None,
                    },
                    description: format!("Formatting in function: {}", raw_func.name),
                    suggestions: vec![
                        "Use consistent formatting".to_string(),
                        "Consider automated formatting tools".to_string(),
                    ],
                });
            }
        }

        AttributedComplexity {
            total: artifact_total,
            breakdown,
            confidence: 0.75, // Medium-high confidence for formatting artifacts
        }
    }

    fn generate_source_mappings(&self, functions: &[FunctionMetrics]) -> Vec<SourceMapping> {
        let mut mappings = Vec::new();

        for func in functions {
            // Create a base mapping for the function
            mappings.push(SourceMapping {
                complexity_point: 1,
                location: CodeLocation {
                    file: func.file.to_string_lossy().to_string(),
                    line: func.line as u32,
                    column: 0,
                    span: Some((func.line as u32, (func.line + func.length) as u32)),
                },
                ast_path: vec![
                    "module".to_string(),
                    "function".to_string(),
                    func.name.clone(),
                ],
                context: format!("Function definition: {}", func.name),
            });

            // Generate estimated mappings for complexity points
            // In a full implementation, this would use AST analysis
            let estimated_complexity_points = self.estimate_complexity_locations(func);
            
            for (point, location_info) in estimated_complexity_points.into_iter().enumerate() {
                if point > 0 {  // Skip first point since it's already added above
                    mappings.push(SourceMapping {
                        complexity_point: (point + 1) as u32,
                        location: CodeLocation {
                            file: func.file.to_string_lossy().to_string(),
                            line: location_info.line,
                            column: location_info.column,
                            span: location_info.span,
                        },
                        ast_path: vec![
                            "module".to_string(),
                            "function".to_string(),
                            func.name.clone(),
                            location_info.construct_type,
                        ],
                        context: location_info.context,
                    });
                }
            }
        }

        mappings
    }

    /// Estimate complexity locations within a function
    /// In a full implementation, this would parse the AST to find actual control flow constructs
    fn estimate_complexity_locations(&self, func: &FunctionMetrics) -> Vec<EstimatedComplexityLocation> {
        let mut locations = Vec::new();
        
        // Create estimated locations based on function properties
        let complexity_per_line = if func.length > 0 {
            func.cyclomatic as f32 / func.length as f32
        } else {
            1.0
        };

        let mut current_line = func.line;
        let line_step = if func.cyclomatic > 1 && func.length > 1 {
            func.length / (func.cyclomatic as usize).max(1)
        } else {
            1
        };

        for i in 0..func.cyclomatic {
            locations.push(EstimatedComplexityLocation {
                line: current_line as u32,
                column: if i == 0 { 0 } else { 4 },  // Estimate indentation
                span: Some((current_line as u32, current_line as u32)),
                construct_type: if i == 0 {
                    "function_signature".to_string()
                } else {
                    format!("control_flow_{}", i)
                },
                context: if i == 0 {
                    format!("Function signature for {}", func.name)
                } else {
                    format!("Control flow construct #{} in {}", i, func.name)
                },
            });
            current_line += line_step;
        }

        locations
    }
}

impl Default for AttributionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete complexity attribution analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityAttribution {
    pub logical_complexity: AttributedComplexity,
    pub formatting_artifacts: AttributedComplexity,
    pub pattern_complexity: AttributedComplexity,
    pub source_mappings: Vec<SourceMapping>,
}

/// Attributed complexity with breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributedComplexity {
    pub total: u32,
    pub breakdown: Vec<ComplexityComponent>,
    pub confidence: f32,
}

/// Individual complexity component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityComponent {
    pub source_type: ComplexitySourceType,
    pub contribution: u32,
    pub location: CodeLocation,
    pub description: String,
    pub suggestions: Vec<String>,
}

/// Source-to-complexity mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMapping {
    pub complexity_point: u32,
    pub location: CodeLocation,
    pub ast_path: Vec<String>,
    pub context: String,
}

/// Code location information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub span: Option<(u32, u32)>,
}

/// Logical construct types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogicalConstruct {
    Function,
    If,
    Loop,
    Match,
    Try,
    Closure,
}

/// Formatting artifact types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FormattingArtifact {
    MultilineExpression,
    ExcessiveWhitespace,
    InconsistentIndentation,
    UnnecessaryParentheses,
    LineBreakPattern,
}

/// Severity of formatting artifacts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArtifactSeverity {
    Low,
    Medium,
    High,
}

/// Recognized pattern types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RecognizedPattern {
    ErrorHandling,
    Validation,
    DataTransformation,
    StateManagement,
    Iterator,
    Builder,
    Factory,
    Observer,
}

/// Language-specific features
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LanguageFeature {
    AsyncAwait,
    PatternMatching,
    Generics,
    Macros,
    Decorators,
    Comprehensions,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_attribution_engine_new() {
        let engine = AttributionEngine::new();
        assert!(!engine.source_trackers.is_empty());
    }

    #[test]
    fn test_attributed_complexity() {
        let complexity = AttributedComplexity {
            total: 10,
            breakdown: vec![],
            confidence: 0.8,
        };

        assert_eq!(complexity.total, 10);
        assert_eq!(complexity.confidence, 0.8);
    }

    #[test]
    fn test_code_location() {
        let location = CodeLocation {
            file: "test.rs".to_string(),
            line: 42,
            column: 5,
            span: Some((42, 50)),
        };

        assert_eq!(location.file, "test.rs");
        assert_eq!(location.line, 42);
        assert_eq!(location.span, Some((42, 50)));
    }

    #[test]
    fn test_source_mapping() {
        let mapping = SourceMapping {
            complexity_point: 3,
            location: CodeLocation {
                file: "main.rs".to_string(),
                line: 10,
                column: 0,
                span: None,
            },
            ast_path: vec!["module".to_string(), "function".to_string()],
            context: "Test context".to_string(),
        };

        assert_eq!(mapping.complexity_point, 3);
        assert_eq!(mapping.ast_path.len(), 2);
    }
}
