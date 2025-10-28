//! Purity Analysis Module
//!
//! This module classifies functions on a purity spectrum (strictly pure, locally pure,
//! read-only, impure) using static analysis. It enables responsibility detection to
//! prefer pure computation classification and identify purity violations that indicate
//! mixed concerns.
//!
//! # Purity Levels
//!
//! - **Strictly Pure**: No I/O, no side effects, deterministic
//! - **Locally Pure**: Only mutates local variables, deterministic results
//! - **Read-Only**: Reads external state but doesn't modify it
//! - **Impure**: Performs I/O or modifies external state
//!
//! # Example
//!
//! ```ignore
//! use debtmap::analysis::purity_analysis::{PurityAnalyzer, PurityLevel};
//!
//! let analyzer = PurityAnalyzer::new();
//! let analysis = analyzer.analyze_code(code, Language::Rust);
//!
//! if analysis.purity == PurityLevel::StrictlyPure {
//!     println!("Function is strictly pure - ideal for testing!");
//! }
//! ```

use crate::analysis::io_detection::{IoDetector, IoProfile, Language, SideEffect};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Purity level classification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

impl PurityLevel {
    /// Convert to a human-readable string
    pub fn as_str(&self) -> &'static str {
        match self {
            PurityLevel::StrictlyPure => "Strictly Pure",
            PurityLevel::LocallyPure => "Locally Pure",
            PurityLevel::ReadOnly => "Read-Only",
            PurityLevel::Impure => "Impure",
        }
    }
}

/// Purity violation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PurityViolation {
    /// I/O operation performed
    IoOperation {
        description: String,
        line: Option<usize>,
    },
    /// External state mutation
    StateMutation { target: String, line: Option<usize> },
    /// Non-deterministic operation
    NonDeterministic {
        operation: String,
        line: Option<usize>,
    },
    /// Calls impure function
    ImpureCall { callee: String, line: Option<usize> },
}

impl PurityViolation {
    /// Get a description of this violation
    pub fn description(&self) -> String {
        match self {
            PurityViolation::IoOperation { description, .. } => {
                format!("I/O operation: {}", description)
            }
            PurityViolation::StateMutation { target, .. } => {
                format!("State mutation: {}", target)
            }
            PurityViolation::NonDeterministic { operation, .. } => {
                format!("Non-deterministic operation: {}", operation)
            }
            PurityViolation::ImpureCall { callee, .. } => {
                format!("Calls impure function: {}", callee)
            }
        }
    }

    /// Get the line number if available
    pub fn line(&self) -> Option<usize> {
        match self {
            PurityViolation::IoOperation { line, .. }
            | PurityViolation::StateMutation { line, .. }
            | PurityViolation::NonDeterministic { line, .. }
            | PurityViolation::ImpureCall { line, .. } => *line,
        }
    }
}

/// Refactoring opportunity type
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl RefactoringType {
    /// Get a description of this refactoring type
    pub fn as_str(&self) -> &'static str {
        match self {
            RefactoringType::ExtractPureCore => "Extract Pure Core",
            RefactoringType::SeparateIoFromLogic => "Separate I/O from Logic",
            RefactoringType::ParameterizeNonDeterminism => "Parameterize Non-Determinism",
            RefactoringType::IsolateSingleViolation => "Isolate Single Violation",
        }
    }
}

/// Effort level for refactoring
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffortLevel {
    Low,
    Medium,
    High,
}

impl EffortLevel {
    /// Convert to a human-readable string
    pub fn as_str(&self) -> &'static str {
        match self {
            EffortLevel::Low => "Low",
            EffortLevel::Medium => "Medium",
            EffortLevel::High => "High",
        }
    }
}

/// Purity refactoring opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurityRefactoringOpportunity {
    pub opportunity_type: RefactoringType,
    pub description: String,
    pub estimated_effort: EffortLevel,
}

/// Complete purity analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurityAnalysis {
    pub purity: PurityLevel,
    pub violations: Vec<PurityViolation>,
    pub is_deterministic: bool,
    pub can_be_pure: bool,
    pub refactoring_opportunity: Option<PurityRefactoringOpportunity>,
}

impl PurityAnalysis {
    /// Create a new purity analysis for a strictly pure function
    pub fn strictly_pure() -> Self {
        Self {
            purity: PurityLevel::StrictlyPure,
            violations: Vec::new(),
            is_deterministic: true,
            can_be_pure: false,
            refactoring_opportunity: None,
        }
    }

    /// Create a new purity analysis for an impure function
    pub fn impure(violations: Vec<PurityViolation>) -> Self {
        let is_deterministic = !violations
            .iter()
            .any(|v| matches!(v, PurityViolation::NonDeterministic { .. }));

        Self {
            purity: PurityLevel::Impure,
            violations,
            is_deterministic,
            can_be_pure: false,
            refactoring_opportunity: None,
        }
    }
}

/// Purity analyzer
pub struct PurityAnalyzer {
    io_detector: IoDetector,
    non_determinism_patterns: HashMap<Language, Vec<String>>,
}

impl PurityAnalyzer {
    /// Create a new purity analyzer
    pub fn new() -> Self {
        Self {
            io_detector: IoDetector::new(),
            non_determinism_patterns: Self::build_non_determinism_patterns(),
        }
    }

    /// Analyze code for purity
    pub fn analyze_code(&self, code: &str, language: Language) -> PurityAnalysis {
        // Get I/O profile from Spec 141
        let io_profile = self.io_detector.detect_io(code, language);

        // Collect violations
        let mut violations = Vec::new();

        // Check for I/O operations
        violations.extend(self.analyze_io_operations(&io_profile, code, language));

        // Check for side effects
        violations.extend(self.analyze_side_effects(&io_profile, code));

        // Check for non-deterministic operations
        violations.extend(self.detect_non_determinism(code, language));

        // Classify purity level
        let purity = self.classify_purity(&violations, &io_profile, code, language);

        // Check determinism
        let is_deterministic = !violations
            .iter()
            .any(|v| matches!(v, PurityViolation::NonDeterministic { .. }));

        // Check if function can be made pure with refactoring
        let can_be_pure = self.can_be_made_pure(&violations);

        // Generate refactoring opportunity if applicable
        let refactoring_opportunity = self.suggest_refactoring(&violations);

        PurityAnalysis {
            purity,
            violations,
            is_deterministic,
            can_be_pure,
            refactoring_opportunity,
        }
    }

    /// Analyze I/O operations from the I/O profile
    fn analyze_io_operations(&self, profile: &IoProfile, _code: &str, _language: Language) -> Vec<PurityViolation> {
        let mut violations = Vec::new();

        // File operations
        for _ in &profile.file_operations {
            violations.push(PurityViolation::IoOperation {
                description: "File I/O operation".to_string(),
                line: None,
            });
        }

        // Network operations
        for _ in &profile.network_operations {
            violations.push(PurityViolation::IoOperation {
                description: "Network I/O operation".to_string(),
                line: None,
            });
        }

        // Console operations
        for _ in &profile.console_operations {
            violations.push(PurityViolation::IoOperation {
                description: "Console I/O operation".to_string(),
                line: None,
            });
        }

        // Database operations
        for _ in &profile.database_operations {
            violations.push(PurityViolation::IoOperation {
                description: "Database I/O operation".to_string(),
                line: None,
            });
        }

        // Environment operations
        for _ in &profile.environment_operations {
            violations.push(PurityViolation::IoOperation {
                description: "Environment variable access".to_string(),
                line: None,
            });
        }

        violations
    }

    /// Analyze side effects from the I/O profile
    fn analyze_side_effects(&self, profile: &IoProfile, code: &str) -> Vec<PurityViolation> {
        let mut violations = Vec::new();

        for side_effect in &profile.side_effects {
            // Check if mutation is local or external
            if !self.is_local_mutation(side_effect, code) {
                match side_effect {
                    SideEffect::FieldMutation { target, field } => {
                        violations.push(PurityViolation::StateMutation {
                            target: format!("{}.{}", target, field),
                            line: None,
                        });
                    }
                    SideEffect::GlobalMutation { name } => {
                        violations.push(PurityViolation::StateMutation {
                            target: name.clone(),
                            line: None,
                        });
                    }
                    SideEffect::CollectionMutation { .. } => {
                        // Collection mutations are considered local unless proven otherwise
                        // This is a simplification - a more sophisticated analysis would
                        // track whether the collection is local or external
                    }
                    SideEffect::ExternalState { description } => {
                        violations.push(PurityViolation::StateMutation {
                            target: description.clone(),
                            line: None,
                        });
                    }
                }
            }
        }

        violations
    }

    /// Check if a mutation is local to the function
    fn is_local_mutation(&self, side_effect: &SideEffect, code: &str) -> bool {
        match side_effect {
            SideEffect::FieldMutation { target, .. } => {
                // If target is "self", it's a field mutation (not local)
                target == "unknown" || !code.contains("self.")
            }
            SideEffect::GlobalMutation { .. } => false, // Global mutations are never local
            SideEffect::CollectionMutation { .. } => {
                // Assume collection mutations are local for now
                // A more sophisticated analysis would track variable scope
                true
            }
            SideEffect::ExternalState { .. } => false,
        }
    }

    /// Detect non-deterministic operations
    fn detect_non_determinism(&self, code: &str, language: Language) -> Vec<PurityViolation> {
        let mut violations = Vec::new();

        if let Some(patterns) = self.non_determinism_patterns.get(&language) {
            for pattern in patterns {
                if code.contains(pattern) {
                    violations.push(PurityViolation::NonDeterministic {
                        operation: pattern.clone(),
                        line: None,
                    });
                }
            }
        }

        violations
    }

    /// Classify the purity level based on violations
    fn classify_purity(&self, violations: &[PurityViolation], profile: &IoProfile, code: &str, language: Language) -> PurityLevel {
        if violations.is_empty() {
            return PurityLevel::StrictlyPure;
        }

        // Check if all violations are local mutations
        let only_local_mutations = violations
            .iter()
            .all(|v| matches!(v, PurityViolation::StateMutation { .. }))
            && !violations.is_empty();

        if only_local_mutations {
            return PurityLevel::LocallyPure;
        }

        // Check if function only reads state (no writes)
        let only_reads = self.only_has_read_operations(profile, code, language);

        if only_reads
            && !violations
                .iter()
                .any(|v| matches!(v, PurityViolation::StateMutation { .. }))
        {
            return PurityLevel::ReadOnly;
        }

        PurityLevel::Impure
    }

    /// Check if the I/O profile only contains read operations
    fn only_has_read_operations(&self, profile: &IoProfile, code: &str, language: Language) -> bool {
        // Check if we have file reads but no other I/O
        let has_file_ops = !profile.file_operations.is_empty();
        let has_network = !profile.network_operations.is_empty();
        let has_console = !profile.console_operations.is_empty();
        let has_db = !profile.database_operations.is_empty();
        let has_mutations = !profile.side_effects.is_empty();

        // If we have network, console, db, or mutations, it's not read-only
        if has_network || has_console || has_db || has_mutations {
            return false;
        }

        // Check if we have file write patterns
        if has_file_ops && self.has_write_operations(code, language) {
            return false;
        }

        // If we only have file operations and no write patterns, consider it read-only
        has_file_ops
    }

    /// Check if code contains write operations
    fn has_write_operations(&self, code: &str, language: Language) -> bool {
        match language {
            Language::Rust => {
                code.contains("::write")
                    || code.contains("File::create")
                    || code.contains("OpenOptions")
                    || code.contains("write_all")
            }
            Language::Python => {
                code.contains("write_text")
                    || code.contains("write_bytes")
                    || code.contains("open(") && code.contains("'w'")
                    || code.contains("open(") && code.contains("\"w\"")
            }
            Language::JavaScript | Language::TypeScript => {
                code.contains("writeFile")
                    || code.contains("createWriteStream")
                    || code.contains("appendFile")
            }
        }
    }

    /// Check if function can be made pure with refactoring
    fn can_be_made_pure(&self, violations: &[PurityViolation]) -> bool {
        // Single violation: Easy to extract
        if violations.len() == 1 {
            return true;
        }

        // All violations are I/O: Can separate I/O from logic
        let all_io = violations
            .iter()
            .all(|v| matches!(v, PurityViolation::IoOperation { .. }));

        if all_io && violations.len() <= 3 {
            return true;
        }

        false
    }

    /// Suggest refactoring opportunities
    fn suggest_refactoring(
        &self,
        violations: &[PurityViolation],
    ) -> Option<PurityRefactoringOpportunity> {
        // Single violation: Easy to extract
        if violations.len() == 1 {
            let description = format!(
                "Function has single purity violation: {}. Extract to make core logic pure.",
                violations[0].description()
            );
            return Some(PurityRefactoringOpportunity {
                opportunity_type: RefactoringType::IsolateSingleViolation,
                description,
                estimated_effort: EffortLevel::Low,
            });
        }

        // All violations are I/O: Separate I/O from logic
        let all_io = violations
            .iter()
            .all(|v| matches!(v, PurityViolation::IoOperation { .. }));

        if all_io {
            return Some(PurityRefactoringOpportunity {
                opportunity_type: RefactoringType::SeparateIoFromLogic,
                description: "Separate I/O operations from business logic. Make computation pure."
                    .to_string(),
                estimated_effort: EffortLevel::Medium,
            });
        }

        // Non-deterministic: Parameterize
        let has_non_determinism = violations
            .iter()
            .any(|v| matches!(v, PurityViolation::NonDeterministic { .. }));

        if has_non_determinism {
            return Some(PurityRefactoringOpportunity {
                opportunity_type: RefactoringType::ParameterizeNonDeterminism,
                description: "Replace non-deterministic operations (time, random) with parameters for testability.".to_string(),
                estimated_effort: EffortLevel::Low,
            });
        }

        None
    }

    /// Build non-determinism patterns for each language
    fn build_non_determinism_patterns() -> HashMap<Language, Vec<String>> {
        let mut patterns = HashMap::new();

        // Rust patterns
        patterns.insert(
            Language::Rust,
            vec![
                "std::time::Instant::now".to_string(),
                "std::time::SystemTime::now".to_string(),
                "Instant::now".to_string(),
                "SystemTime::now".to_string(),
                "rand::".to_string(),
                "thread_rng".to_string(),
                "uuid::Uuid::new_v4".to_string(),
                "Uuid::new_v4".to_string(),
                "HashMap::new".to_string(), // Uses random seed
                "HashSet::new".to_string(), // Uses random seed
            ],
        );

        // Python patterns
        patterns.insert(
            Language::Python,
            vec![
                "random.".to_string(),
                "datetime.now".to_string(),
                "time.time".to_string(),
                "uuid.uuid4".to_string(),
                "time.monotonic".to_string(),
            ],
        );

        // JavaScript patterns
        patterns.insert(
            Language::JavaScript,
            vec![
                "Math.random".to_string(),
                "Date.now".to_string(),
                "new Date()".to_string(),
                "crypto.randomUUID".to_string(),
                "performance.now".to_string(),
            ],
        );

        // TypeScript has same patterns as JavaScript
        patterns.insert(
            Language::TypeScript,
            patterns[&Language::JavaScript].clone(),
        );

        patterns
    }
}

impl Default for PurityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

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

        let analyzer = PurityAnalyzer::new();
        let analysis = analyzer.analyze_code(code, Language::Rust);

        assert_eq!(analysis.purity, PurityLevel::StrictlyPure);
        assert!(analysis.violations.is_empty());
        assert!(analysis.is_deterministic);
    }

    #[test]
    fn read_only_function() {
        let code = r#"
        fn read_config() -> String {
            std::fs::read_to_string("config.toml").unwrap()
        }
        "#;

        let analyzer = PurityAnalyzer::new();
        let analysis = analyzer.analyze_code(code, Language::Rust);

        assert_eq!(analysis.purity, PurityLevel::ReadOnly);
        assert!(!analysis.violations.is_empty());
        assert!(analysis
            .violations
            .iter()
            .any(|v| { matches!(v, PurityViolation::IoOperation { .. }) }));
    }

    #[test]
    fn impure_function() {
        let code = r#"
        fn save_data(data: &str) {
            std::fs::write("output.txt", data).unwrap();
        }
        "#;

        let analyzer = PurityAnalyzer::new();
        let analysis = analyzer.analyze_code(code, Language::Rust);

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

        let analyzer = PurityAnalyzer::new();
        let analysis = analyzer.analyze_code(code, Language::Rust);

        assert!(!analysis.is_deterministic);
        assert!(analysis
            .violations
            .iter()
            .any(|v| { matches!(v, PurityViolation::NonDeterministic { .. }) }));
    }

    #[test]
    fn almost_pure_refactoring_opportunity() {
        let code = r#"
        fn calculate_with_logging(a: i32, b: i32) -> i32 {
            let result = a * b + a / b;
            println!("Result: {}", result);
            result
        }
        "#;

        let analyzer = PurityAnalyzer::new();
        let analysis = analyzer.analyze_code(code, Language::Rust);

        assert!(analysis.can_be_pure);
        assert!(analysis.refactoring_opportunity.is_some());

        if let Some(opportunity) = &analysis.refactoring_opportunity {
            assert!(matches!(
                opportunity.opportunity_type,
                RefactoringType::IsolateSingleViolation
            ));
        }
    }

    #[test]
    fn python_non_deterministic() {
        let code = r#"
def generate_timestamp():
    return datetime.now()
        "#;

        let analyzer = PurityAnalyzer::new();
        let analysis = analyzer.analyze_code(code, Language::Python);

        assert!(!analysis.is_deterministic);
        assert!(analysis
            .violations
            .iter()
            .any(|v| { matches!(v, PurityViolation::NonDeterministic { .. }) }));
    }

    #[test]
    fn javascript_random() {
        let code = r#"
function randomNumber() {
    return Math.random();
}
        "#;

        let analyzer = PurityAnalyzer::new();
        let analysis = analyzer.analyze_code(code, Language::JavaScript);

        assert!(!analysis.is_deterministic);
        assert!(analysis
            .violations
            .iter()
            .any(|v| { matches!(v, PurityViolation::NonDeterministic { .. }) }));
    }

    #[test]
    fn separate_io_refactoring() {
        let code = r#"
        fn process_file(path: &str) -> Result<i32, Error> {
            let content = std::fs::read_to_string(path)?;
            let data = parse_content(&content);
            let result = calculate(&data);
            std::fs::write("output.txt", &result.to_string())?;
            Ok(result)
        }
        "#;

        let analyzer = PurityAnalyzer::new();
        let analysis = analyzer.analyze_code(code, Language::Rust);

        assert_eq!(analysis.purity, PurityLevel::Impure);

        if let Some(opportunity) = &analysis.refactoring_opportunity {
            assert!(matches!(
                opportunity.opportunity_type,
                RefactoringType::SeparateIoFromLogic
            ));
        }
    }
}
