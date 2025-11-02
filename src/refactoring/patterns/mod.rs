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

impl FunctionalCompositionMatcher {
    /// Pure function to detect functional composition patterns in function names
    fn has_functional_patterns(name: &str) -> bool {
        const FUNCTIONAL_PATTERNS: &[&str] = &["map", "filter", "fold", "compose"];
        FUNCTIONAL_PATTERNS
            .iter()
            .any(|pattern| name.contains(pattern))
    }

    /// Pure function to create pattern evidence for functional composition
    fn create_pattern_evidence(function_line: usize) -> PatternEvidence {
        PatternEvidence {
            code_snippets: vec![],
            line_numbers: vec![function_line as u32],
            confidence_factors: vec!["Function name suggests functional pattern".to_string()],
        }
    }
}

impl PatternMatcher for FunctionalCompositionMatcher {
    fn match_pattern(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<DetectedPattern> {
        if Self::has_functional_patterns(&function.name) {
            Some(DetectedPattern {
                pattern_type: PatternType::FunctionalComposition,
                confidence: 0.8,
                evidence: Self::create_pattern_evidence(function.line),
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

impl MutableStateMatcher {
    /// Pure function to detect mutation patterns in function names
    fn has_mutation_patterns(name: &str) -> bool {
        const MUTATION_PATTERNS: &[&str] = &["update", "modify", "set_", "mut"];
        MUTATION_PATTERNS
            .iter()
            .any(|pattern| name.contains(pattern))
    }

    /// Pure function to create a mutable state pattern detection
    fn create_mutable_state_pattern(line: usize) -> DetectedPattern {
        DetectedPattern {
            pattern_type: PatternType::MutableState,
            confidence: 0.6,
            evidence: PatternEvidence {
                code_snippets: vec![],
                line_numbers: vec![line as u32],
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
        }
    }
}

impl PatternMatcher for MutableStateMatcher {
    fn match_pattern(
        &self,
        function: &FunctionMetrics,
        _file: &FileMetrics,
    ) -> Option<DetectedPattern> {
        if Self::has_mutation_patterns(&function.name) {
            Some(Self::create_mutable_state_pattern(function.line))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ComplexityMetrics, Language};
    use std::path::PathBuf;

    fn create_test_function(name: &str, cyclomatic: u32) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from("test.rs"),
            line: 42,
            cyclomatic,
            cognitive: 0,
            nesting: 0,
            length: 10,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
        }
    }

    fn create_test_file() -> FileMetrics {
        FileMetrics {
            path: PathBuf::from("test.rs"),
            language: Language::Rust,
            complexity: ComplexityMetrics {
                functions: vec![],
                cyclomatic_complexity: 10,
                cognitive_complexity: 5,
            },
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            module_scope: None,
            classes: None,
        }
    }

    #[test]
    fn test_functional_composition_matcher_detects_map() {
        let matcher = FunctionalCompositionMatcher;
        let function = create_test_function("map_values", 3);
        let file = create_test_file();

        let result = matcher.match_pattern(&function, &file);
        assert!(result.is_some());

        let pattern = result.unwrap();
        assert_eq!(pattern.confidence, 0.8);
        assert!(matches!(
            pattern.pattern_type,
            PatternType::FunctionalComposition
        ));
        assert!(matches!(
            pattern.assessment,
            PatternAssessment::GoodExample { .. }
        ));
    }

    #[test]
    fn test_functional_composition_matcher_detects_filter() {
        let matcher = FunctionalCompositionMatcher;
        let function = create_test_function("filter_items", 2);
        let file = create_test_file();

        let result = matcher.match_pattern(&function, &file);
        assert!(result.is_some());

        let pattern = result.unwrap();
        assert!(matches!(
            pattern.pattern_type,
            PatternType::FunctionalComposition
        ));
        assert_eq!(pattern.evidence.line_numbers, vec![42]);
    }

    #[test]
    fn test_functional_composition_matcher_detects_fold() {
        let matcher = FunctionalCompositionMatcher;
        let function = create_test_function("fold_results", 4);
        let file = create_test_file();

        let result = matcher.match_pattern(&function, &file);
        assert!(result.is_some());

        let pattern = result.unwrap();
        assert_eq!(pattern.confidence, 0.8);
        assert!(!pattern.evidence.confidence_factors.is_empty());
    }

    #[test]
    fn test_functional_composition_matcher_detects_compose() {
        let matcher = FunctionalCompositionMatcher;
        let function = create_test_function("compose_functions", 5);
        let file = create_test_file();

        let result = matcher.match_pattern(&function, &file);
        assert!(result.is_some());

        let pattern = result.unwrap();
        if let PatternAssessment::GoodExample { strengths, .. } = pattern.assessment {
            assert_eq!(strengths.len(), 2);
            assert!(strengths[0].contains("functional composition"));
        } else {
            panic!("Expected GoodExample assessment");
        }
    }

    #[test]
    fn test_functional_composition_matcher_rejects_non_functional() {
        let matcher = FunctionalCompositionMatcher;
        let function = create_test_function("process_data", 6);
        let file = create_test_file();

        let result = matcher.match_pattern(&function, &file);
        assert!(result.is_none());
    }

    #[test]
    fn test_functional_composition_matcher_rejects_imperative_loop() {
        let matcher = FunctionalCompositionMatcher;
        let function = create_test_function("update_records", 8);
        let file = create_test_file();

        let result = matcher.match_pattern(&function, &file);
        assert!(result.is_none());
    }

    #[test]
    fn test_has_functional_patterns_helper() {
        assert!(FunctionalCompositionMatcher::has_functional_patterns(
            "map_values"
        ));
        assert!(FunctionalCompositionMatcher::has_functional_patterns(
            "filter_items"
        ));
        assert!(FunctionalCompositionMatcher::has_functional_patterns(
            "fold_sum"
        ));
        assert!(FunctionalCompositionMatcher::has_functional_patterns(
            "compose_transforms"
        ));
        assert!(!FunctionalCompositionMatcher::has_functional_patterns(
            "calculate_total"
        ));
        assert!(!FunctionalCompositionMatcher::has_functional_patterns(
            "update_state"
        ));
    }

    #[test]
    fn test_create_pattern_evidence_helper() {
        let evidence = FunctionalCompositionMatcher::create_pattern_evidence(100);
        assert_eq!(evidence.line_numbers, vec![100]);
        assert_eq!(evidence.code_snippets.len(), 0);
        assert_eq!(evidence.confidence_factors.len(), 1);
        assert!(evidence.confidence_factors[0].contains("functional pattern"));
    }

    #[test]
    fn test_mutable_state_matcher_detects_update() {
        let matcher = MutableStateMatcher;
        let function = create_test_function("update_user", 3);
        let file = create_test_file();

        let result = matcher.match_pattern(&function, &file);
        assert!(result.is_some());
        let pattern = result.unwrap();
        assert!(matches!(pattern.pattern_type, PatternType::MutableState));
        assert_eq!(pattern.confidence, 0.6);
    }

    #[test]
    fn test_mutable_state_matcher_detects_modify() {
        let matcher = MutableStateMatcher;
        let function = create_test_function("modify_state", 5);
        let file = create_test_file();

        let result = matcher.match_pattern(&function, &file);
        assert!(result.is_some());
        let pattern = result.unwrap();
        assert!(matches!(pattern.pattern_type, PatternType::MutableState));
        assert_eq!(pattern.evidence.line_numbers, vec![42]);
    }

    #[test]
    fn test_mutable_state_matcher_detects_set() {
        let matcher = MutableStateMatcher;
        let function = create_test_function("set_value", 2);
        let file = create_test_file();

        let result = matcher.match_pattern(&function, &file);
        assert!(result.is_some());
        let pattern = result.unwrap();
        assert!(matches!(pattern.pattern_type, PatternType::MutableState));
        assert!(matches!(
            pattern.assessment,
            PatternAssessment::ImprovementOpportunity { .. }
        ));
    }

    #[test]
    fn test_mutable_state_matcher_detects_mut() {
        let matcher = MutableStateMatcher;
        let function = create_test_function("get_mut_ref", 4);
        let file = create_test_file();

        let result = matcher.match_pattern(&function, &file);
        assert!(result.is_some());
        assert!(matches!(
            result.unwrap().pattern_type,
            PatternType::MutableState
        ));
    }

    #[test]
    fn test_mutable_state_matcher_rejects_pure_functions() {
        let matcher = MutableStateMatcher;
        let function = create_test_function("calculate_sum", 3);
        let file = create_test_file();

        let result = matcher.match_pattern(&function, &file);
        assert!(result.is_none());
    }

    #[test]
    fn test_has_mutation_patterns_helper() {
        assert!(MutableStateMatcher::has_mutation_patterns("update_user"));
        assert!(MutableStateMatcher::has_mutation_patterns("modify_state"));
        assert!(MutableStateMatcher::has_mutation_patterns("set_value"));
        assert!(MutableStateMatcher::has_mutation_patterns("get_mut"));
        assert!(!MutableStateMatcher::has_mutation_patterns("calculate"));
        assert!(!MutableStateMatcher::has_mutation_patterns("get_value"));
        assert!(!MutableStateMatcher::has_mutation_patterns("filter_items"));
    }

    #[test]
    fn test_create_mutable_state_pattern_helper() {
        let pattern = MutableStateMatcher::create_mutable_state_pattern(250);
        assert!(matches!(pattern.pattern_type, PatternType::MutableState));
        assert_eq!(pattern.confidence, 0.6);
        assert_eq!(pattern.evidence.line_numbers, vec![250]);
        assert!(pattern.evidence.confidence_factors[0].contains("mutation"));

        if let PatternAssessment::ImprovementOpportunity {
            current_issues,
            potential_benefits,
            ..
        } = pattern.assessment
        {
            assert_eq!(current_issues.len(), 2);
            assert_eq!(potential_benefits.len(), 3);
            assert!(current_issues[0].contains("Mutable state"));
            assert!(potential_benefits[0].contains("Immutable"));
        } else {
            panic!("Expected ImprovementOpportunity assessment");
        }
    }
}
