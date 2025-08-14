use crate::core::{FileMetrics, FunctionMetrics};
use crate::refactoring::{
    ConcernMixingPattern, DetectedPattern, FormattingDetector, FormattingInfo, IoDetector, IoInfo,
    OrchestrationPattern, PatternAssessment, PatternEvidence, PatternMatcher, PatternType,
    SeparationDifficulty, TraitAnalyzer, TraitInfo, Urgency,
};
use std::sync::Arc;

mod functional_patterns;
mod imperative_patterns;
mod io_patterns;

pub use functional_patterns::*;
pub use imperative_patterns::*;
pub use io_patterns::*;

pub fn create_pattern_matchers() -> Vec<Arc<dyn PatternMatcher>> {
    vec![
        Arc::new(FunctionalCompositionMatcher),
        Arc::new(ImperativeLoopMatcher),
        Arc::new(MutableStateMatcher),
        Arc::new(SideEffectMatcher),
        Arc::new(MixedConcernsMatcher),
    ]
}

pub fn create_io_detectors() -> Vec<Arc<dyn IoDetector>> {
    vec![
        Arc::new(FileIoDetector),
        Arc::new(NetworkIoDetector),
        Arc::new(DatabaseIoDetector),
    ]
}

pub fn create_formatting_detectors() -> Vec<Arc<dyn FormattingDetector>> {
    vec![
        Arc::new(StringFormattingDetector),
        Arc::new(JsonFormattingDetector),
        Arc::new(MarkdownFormattingDetector),
    ]
}

pub fn create_trait_analyzers() -> Vec<Arc<dyn TraitAnalyzer>> {
    vec![
        Arc::new(StandardTraitAnalyzer),
        Arc::new(VisitorTraitAnalyzer),
    ]
}

// Pattern matcher implementations
struct FunctionalCompositionMatcher;

impl PatternMatcher for FunctionalCompositionMatcher {
    fn match_pattern(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<DetectedPattern> {
        // Look for functional composition patterns like map, filter, fold
        let has_functional_patterns = function.name.contains("map")
            || function.name.contains("filter")
            || function.name.contains("fold")
            || function.name.contains("compose");

        if has_functional_patterns {
            Some(DetectedPattern {
                pattern_type: PatternType::FunctionalComposition,
                confidence: 0.8,
                evidence: PatternEvidence {
                    code_snippets: vec![],
                    line_numbers: vec![function.line as u32],
                    confidence_factors: vec![
                        "Function name suggests functional pattern".to_string()
                    ],
                },
                assessment: PatternAssessment::GoodExample {
                    strengths: vec![
                        "Uses functional composition patterns".to_string(),
                        "Likely pure and testable".to_string(),
                    ],
                    why_good: "Functional composition promotes code reusability and testability"
                        .to_string(),
                },
            })
        } else {
            None
        }
    }
}

struct ImperativeLoopMatcher;

impl PatternMatcher for ImperativeLoopMatcher {
    fn match_pattern(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<DetectedPattern> {
        // High cyclomatic complexity often indicates loops
        if function.cyclomatic > 5
            && !function.name.contains("map")
            && !function.name.contains("filter")
        {
            Some(DetectedPattern {
                pattern_type: PatternType::ImperativeLoop,
                confidence: 0.7,
                evidence: PatternEvidence {
                    code_snippets: vec![],
                    line_numbers: vec![function.line as u32],
                    confidence_factors: vec![format!(
                        "High cyclomatic complexity: {}",
                        function.cyclomatic
                    )],
                },
                assessment: PatternAssessment::ImprovementOpportunity {
                    current_issues: vec![
                        "Imperative loops are harder to test and reason about".to_string(),
                        "Could be replaced with functional patterns".to_string(),
                    ],
                    potential_benefits: vec![
                        "Map/filter/fold patterns are more declarative".to_string(),
                        "Functional patterns are easier to test".to_string(),
                        "Immutability prevents bugs".to_string(),
                    ],
                    refactoring_suggestions: vec![],
                },
            })
        } else {
            None
        }
    }
}

struct MutableStateMatcher;

impl PatternMatcher for MutableStateMatcher {
    fn match_pattern(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<DetectedPattern> {
        // Functions with "update", "modify", "set" often mutate state
        let has_mutation_patterns = function.name.contains("update")
            || function.name.contains("modify")
            || function.name.contains("set_")
            || function.name.contains("mut");

        if has_mutation_patterns {
            Some(DetectedPattern {
                pattern_type: PatternType::MutableState,
                confidence: 0.6,
                evidence: PatternEvidence {
                    code_snippets: vec![],
                    line_numbers: vec![function.line as u32],
                    confidence_factors: vec!["Function name suggests state mutation".to_string()],
                },
                assessment: PatternAssessment::ImprovementOpportunity {
                    current_issues: vec![
                        "Mutable state makes code harder to reason about".to_string(),
                        "Difficult to test and parallelize".to_string(),
                    ],
                    potential_benefits: vec![
                        "Immutable transformations are safer".to_string(),
                        "Pure functions are easier to test".to_string(),
                        "Thread-safe by default".to_string(),
                    ],
                    refactoring_suggestions: vec![],
                },
            })
        } else {
            None
        }
    }
}

struct SideEffectMatcher;

impl SideEffectMatcher {
    /// Pure function to detect I/O operation patterns in function names
    fn has_io_pattern(name: &str) -> bool {
        const IO_PATTERNS: &[&str] = &["write", "read", "print", "save", "load", "fetch"];
        IO_PATTERNS.iter().any(|pattern| name.contains(pattern))
    }

    /// Pure function to determine if function has mixed concerns
    fn has_mixed_concerns(has_io: bool, complexity: u32) -> bool {
        has_io && complexity > 3
    }
}

impl PatternMatcher for SideEffectMatcher {
    fn match_pattern(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<DetectedPattern> {
        let has_io = Self::has_io_pattern(&function.name);

        if Self::has_mixed_concerns(has_io, function.cyclomatic) {
            Some(DetectedPattern {
                pattern_type: PatternType::SideEffects,
                confidence: 0.7,
                evidence: PatternEvidence {
                    code_snippets: vec![],
                    line_numbers: vec![function.line as u32],
                    confidence_factors: vec![
                        "Function name suggests I/O operations".to_string(),
                        "Complexity indicates mixed concerns".to_string(),
                    ],
                },
                assessment: PatternAssessment::ImprovementOpportunity {
                    current_issues: vec![
                        "Mixing business logic with I/O".to_string(),
                        "Hard to test without mocking".to_string(),
                    ],
                    potential_benefits: vec![
                        "Extract pure functions for business logic".to_string(),
                        "Keep I/O at boundaries".to_string(),
                        "Enable unit testing without mocks".to_string(),
                    ],
                    refactoring_suggestions: vec![],
                },
            })
        } else {
            None
        }
    }
}

struct MixedConcernsMatcher;

impl MixedConcernsMatcher {
    /// Classifies the concerns present in a function based on its name and metrics
    fn classify_concerns(function: &FunctionMetrics) -> Vec<String> {
        let mut concerns = vec![];

        // Check for I/O operations
        if Self::has_io_operations(&function.name) {
            concerns.push("I/O Operations".to_string());
        }

        // Check for business logic
        if function.cyclomatic > 5 {
            concerns.push("Business Logic".to_string());
        }

        // Check for formatting
        if Self::has_formatting_operations(&function.name) {
            concerns.push("Formatting".to_string());
        }

        concerns
    }

    /// Determines if a function name indicates I/O operations
    fn has_io_operations(name: &str) -> bool {
        name.contains("write")
            || name.contains("read")
            || name.contains("print")
            || name.contains("save")
    }

    /// Determines if a function name indicates formatting operations
    fn has_formatting_operations(name: &str) -> bool {
        name.contains("format") || name.contains("display")
    }

    /// Classifies the difficulty of separating concerns based on complexity
    fn classify_separation_difficulty(cyclomatic: u32) -> SeparationDifficulty {
        if cyclomatic > 10 {
            SeparationDifficulty::High
        } else {
            SeparationDifficulty::Medium
        }
    }

    /// Determines the urgency of refactoring based on complexity
    fn classify_urgency(cyclomatic: u32) -> Urgency {
        if cyclomatic > 15 {
            Urgency::High
        } else {
            Urgency::Medium
        }
    }
}

impl PatternMatcher for MixedConcernsMatcher {
    fn match_pattern(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<DetectedPattern> {
        let concerns = Self::classify_concerns(function);

        if concerns.len() > 1 {
            Some(DetectedPattern {
                pattern_type: PatternType::MixedConcerns(ConcernMixingPattern {
                    concerns: concerns.clone(),
                    separation_difficulty: Self::classify_separation_difficulty(
                        function.cyclomatic,
                    ),
                }),
                confidence: 0.8,
                evidence: PatternEvidence {
                    code_snippets: vec![],
                    line_numbers: vec![function.line as u32],
                    confidence_factors: vec![
                        format!("Multiple concerns detected: {}", concerns.join(", ")),
                        format!("Complexity: {}", function.cyclomatic),
                    ],
                },
                assessment: PatternAssessment::AntiPattern {
                    problems: vec![
                        "Function violates single responsibility principle".to_string(),
                        "Difficult to test in isolation".to_string(),
                        "Changes to one concern affect others".to_string(),
                    ],
                    recommended_patterns: vec![PatternType::FunctionalComposition],
                    urgency: Self::classify_urgency(function.cyclomatic),
                },
            })
        } else {
            None
        }
    }
}

// I/O detectors
struct FileIoDetector;

impl IoDetector for FileIoDetector {
    fn detect_io_orchestration(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<IoInfo> {
        if function.name.contains("read_file")
            || function.name.contains("write_file")
            || function.name.contains("save")
            || function.name.contains("load")
        {
            Some(IoInfo {
                patterns: vec![OrchestrationPattern {
                    pattern_type: "FileIO".to_string(),
                    description: "File system operations".to_string(),
                }],
                io_operations: vec!["File reading/writing".to_string()],
            })
        } else {
            None
        }
    }
}

struct NetworkIoDetector;

impl IoDetector for NetworkIoDetector {
    fn detect_io_orchestration(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<IoInfo> {
        if function.name.contains("fetch")
            || function.name.contains("request")
            || function.name.contains("http")
            || function.name.contains("api")
        {
            Some(IoInfo {
                patterns: vec![OrchestrationPattern {
                    pattern_type: "NetworkIO".to_string(),
                    description: "Network operations".to_string(),
                }],
                io_operations: vec!["HTTP requests".to_string()],
            })
        } else {
            None
        }
    }
}

struct DatabaseIoDetector;

impl IoDetector for DatabaseIoDetector {
    fn detect_io_orchestration(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<IoInfo> {
        if function.name.contains("query")
            || function.name.contains("insert")
            || function.name.contains("update")
            || function.name.contains("delete")
            || function.name.contains("database")
            || function.name.contains("db_")
        {
            Some(IoInfo {
                patterns: vec![OrchestrationPattern {
                    pattern_type: "DatabaseIO".to_string(),
                    description: "Database operations".to_string(),
                }],
                io_operations: vec!["Database queries".to_string()],
            })
        } else {
            None
        }
    }
}

// Formatting detectors
struct StringFormattingDetector;

impl FormattingDetector for StringFormattingDetector {
    fn detect_formatting_function(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<FormattingInfo> {
        if function.name.contains("format")
            || function.name.contains("to_string")
            || function.name.contains("display")
        {
            Some(FormattingInfo {
                inputs: vec!["data".to_string()],
                output: "String".to_string(),
                format_type: "String formatting".to_string(),
            })
        } else {
            None
        }
    }
}

struct JsonFormattingDetector;

impl FormattingDetector for JsonFormattingDetector {
    fn detect_formatting_function(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<FormattingInfo> {
        if function.name.contains("to_json") || function.name.contains("serialize") {
            Some(FormattingInfo {
                inputs: vec!["data".to_string()],
                output: "JSON".to_string(),
                format_type: "JSON serialization".to_string(),
            })
        } else {
            None
        }
    }
}

struct MarkdownFormattingDetector;

impl FormattingDetector for MarkdownFormattingDetector {
    fn detect_formatting_function(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<FormattingInfo> {
        if function.name.contains("markdown") || function.name.contains("md_") {
            Some(FormattingInfo {
                inputs: vec!["data".to_string()],
                output: "Markdown".to_string(),
                format_type: "Markdown formatting".to_string(),
            })
        } else {
            None
        }
    }
}

// Trait analyzers
struct StandardTraitAnalyzer;

impl StandardTraitAnalyzer {
    // Pure function to classify trait method names
    fn classify_trait_method(method_name: &str) -> Option<&'static str> {
        match method_name {
            "fmt" => Some("Display/Debug"),
            "clone" => Some("Clone"),
            "eq" => Some("PartialEq"),
            "hash" => Some("Hash"),
            "default" => Some("Default"),
            "from" => Some("From"),
            "try_from" => Some("TryFrom"),
            _ => None,
        }
    }
}

impl TraitAnalyzer for StandardTraitAnalyzer {
    fn detect_trait_implementation(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<TraitInfo> {
        Self::classify_trait_method(&function.name).map(|trait_name| TraitInfo {
            trait_name: trait_name.to_string(),
            method_name: function.name.clone(),
        })
    }
}

struct VisitorTraitAnalyzer;

impl TraitAnalyzer for VisitorTraitAnalyzer {
    fn detect_trait_implementation(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<TraitInfo> {
        // Check for visitor pattern methods
        if function.name.starts_with("visit_") || function.name.starts_with("analyze_") {
            Some(TraitInfo {
                trait_name: "Visit".to_string(),
                method_name: function.name.clone(),
            })
        } else {
            None
        }
    }
}
