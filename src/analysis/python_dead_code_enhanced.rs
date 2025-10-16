//! Enhanced Python Dead Code Detection with Confidence Scoring
//!
//! This module implements a comprehensive dead code detection system for Python that
//! dramatically reduces false positives by integrating:
//! - Framework pattern detection (Spec 103)
//! - Test detection (Spec 104)
//! - Callback tracking (Spec 105)
//! - Import resolution (Spec 106)
//!
//! The system provides confidence-based scoring to help users make informed decisions
//! about whether code is truly dead.
//!
//! # Usage Examples
//!
//! ## Basic Usage
//!
//! ```rust,no_run
//! use debtmap::analysis::python_dead_code_enhanced::{EnhancedDeadCodeAnalyzer, AnalysisConfig};
//! use debtmap::core::FunctionMetrics;
//! use debtmap::priority::call_graph::CallGraph;
//! use std::path::PathBuf;
//!
//! let analyzer = EnhancedDeadCodeAnalyzer::new();
//! let call_graph = CallGraph::new();
//! let func = FunctionMetrics::new("my_function".to_string(), PathBuf::from("app.py"), 10);
//!
//! let result = analyzer.analyze_function(&func, &call_graph);
//! println!("Is dead: {}, Confidence: {:?}", result.is_dead, result.confidence);
//! ```
//!
//! ## With Custom Configuration
//!
//! ```rust,no_run
//! # use debtmap::analysis::python_dead_code_enhanced::{EnhancedDeadCodeAnalyzer, AnalysisConfig};
//! let config = AnalysisConfig {
//!     high_confidence_threshold: 0.85,
//!     medium_confidence_threshold: 0.6,
//!     respect_suppression_comments: true,
//!     include_private_api: false,
//! };
//!
//! let analyzer = EnhancedDeadCodeAnalyzer::new().with_config(config);
//! ```
//!
//! ## With Coverage Data
//!
//! ```rust,no_run
//! # use debtmap::analysis::python_dead_code_enhanced::{EnhancedDeadCodeAnalyzer, CoverageData};
//! # use std::path::PathBuf;
//! let mut coverage = CoverageData::new();
//! coverage.add_coverage(
//!     PathBuf::from("app.py"),
//!     vec![1, 2, 5, 10, 15],  // covered lines
//!     vec![1, 2, 5, 10, 15],  // executed lines
//! );
//!
//! let analyzer = EnhancedDeadCodeAnalyzer::new().with_coverage(coverage);
//! ```
//!
//! ## Suppressing False Positives
//!
//! Functions can be marked as intentionally unused with suppression comments:
//!
//! ```python
//! # debtmap: not-dead
//! def future_api_method():
//!     """This will be used in v2.0"""
//!     pass
//!
//! def another_function():  # noqa: dead-code
//!     """Kept for backwards compatibility"""
//!     pass
//! ```
//!
//! # Confidence Scoring
//!
//! The analyzer returns confidence scores in three levels:
//!
//! - **High (0.8-1.0)**: Very likely dead code, safe to remove
//! - **Medium (0.5-0.8)**: Possibly dead code, manual review recommended
//! - **Low (0.0-0.5)**: Unlikely to be dead code, probably in use
//!
//! Confidence is reduced by factors such as:
//! - Function has static callers
//! - Function is a framework entry point (Flask routes, Django views, etc.)
//! - Function is a test function
//! - Function is registered as a callback
//! - Function is exported in `__all__`
//! - Function has `@property` or `@cached_property` decorator
//! - Function is a magic method (`__init__`, `__str__`, etc.)

use crate::analysis::framework_patterns::FrameworkPatternRegistry;
use crate::analysis::python_call_graph::callback_tracker::CallbackTracker;
use crate::analysis::python_call_graph::import_tracker::ImportTracker;
use crate::core::FunctionMetrics;
use crate::priority::call_graph::{CallGraph, FunctionId};
use crate::testing::python::test_detector::PythonTestDetector;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Confidence level for dead code detection
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum DeadCodeConfidence {
    /// 0.8-1.0: Very likely dead code
    High(f32),
    /// 0.5-0.8: Possibly dead code
    Medium(f32),
    /// 0.0-0.5: Unlikely to be dead code
    Low(f32),
}

impl DeadCodeConfidence {
    pub fn from_score(score: f32) -> Self {
        let score = score.clamp(0.0, 1.0);
        if score >= 0.8 {
            DeadCodeConfidence::High(score)
        } else if score >= 0.5 {
            DeadCodeConfidence::Medium(score)
        } else {
            DeadCodeConfidence::Low(score)
        }
    }

    pub fn score(&self) -> f32 {
        match self {
            DeadCodeConfidence::High(s) => *s,
            DeadCodeConfidence::Medium(s) => *s,
            DeadCodeConfidence::Low(s) => *s,
        }
    }
}

/// Reasons why a function might be dead code
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeadCodeReason {
    NoStaticCallers,
    NoCoverage,
    NotExported,
    PrivateFunction,
    NotInTestFile,
    NoFrameworkPattern,
    NoCallbackRegistration,
    NotImported,
}

/// Reasons why a function might NOT be dead code
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveCodeReason {
    HasStaticCallers,
    FrameworkEntryPoint,
    TestFunction,
    CallbackTarget,
    ExportedInAll,
    PublicApi,
    MagicMethod,
    PropertyDecorator,
    MainEntryPoint,
}

/// Result of dead code analysis
#[derive(Debug, Clone)]
pub struct DeadCodeResult {
    pub function_id: FunctionId,
    pub is_dead: bool,
    pub confidence: DeadCodeConfidence,
    pub dead_reasons: Vec<DeadCodeReason>,
    pub live_reasons: Vec<LiveCodeReason>,
    pub suggestion: RemovalSuggestion,
}

/// Suggestion for removal
#[derive(Debug, Clone)]
pub struct RemovalSuggestion {
    pub can_remove: bool,
    pub safe_to_remove: bool,
    pub explanation: String,
    pub risks: Vec<String>,
}

/// Factors used in confidence calculation
#[derive(Debug, Clone)]
struct ConfidenceFactors {
    has_callers: bool,
    is_framework_entry: bool,
    is_test_function: bool,
    is_callback_target: bool,
    is_exported: bool,
    is_public: bool,
    is_magic_method: bool,
    is_main_entry: bool,
    is_property: bool,
    in_test_file: bool,
}

/// Enhanced dead code analyzer integrating all detection systems
pub struct EnhancedDeadCodeAnalyzer {
    framework_detector: FrameworkPatternRegistry,
    test_detector: PythonTestDetector,
    callback_tracker: CallbackTracker,
    import_trackers: HashMap<PathBuf, ImportTracker>,
    coverage_data: Option<CoverageData>,
    config: AnalysisConfig,
}

/// Coverage data from coverage.py or pytest-cov
#[derive(Debug, Clone)]
pub struct CoverageData {
    /// Map of file path to covered line numbers
    covered_lines: HashMap<PathBuf, Vec<usize>>,
    /// Map of file path to executed line numbers (may be different from covered)
    executed_lines: HashMap<PathBuf, Vec<usize>>,
}

impl CoverageData {
    pub fn new() -> Self {
        Self {
            covered_lines: HashMap::new(),
            executed_lines: HashMap::new(),
        }
    }

    /// Check if a line is covered by tests
    pub fn is_line_covered(&self, file: &Path, line: usize) -> bool {
        self.covered_lines
            .get(file)
            .map(|lines| lines.contains(&line))
            .unwrap_or(false)
    }

    /// Check if a line was executed during test runs
    pub fn is_line_executed(&self, file: &Path, line: usize) -> bool {
        self.executed_lines
            .get(file)
            .map(|lines| lines.contains(&line))
            .unwrap_or(false)
    }

    /// Add coverage data for a file
    pub fn add_coverage(&mut self, file: PathBuf, covered: Vec<usize>, executed: Vec<usize>) {
        self.covered_lines.insert(file.clone(), covered);
        self.executed_lines.insert(file, executed);
    }

    /// Parse coverage data from a coverage.py JSON report
    pub fn from_coverage_json(json_path: &Path) -> Result<Self, std::io::Error> {
        use std::fs;

        let content = fs::read_to_string(json_path)?;
        let data = CoverageData::new();

        // Parse JSON (simplified - would need full JSON parsing in production)
        // For now, return empty coverage data
        // In a full implementation, this would parse the JSON structure:
        // {
        //   "files": {
        //     "path/to/file.py": {
        //       "executed_lines": [1, 2, 5, 10],
        //       "missing_lines": [3, 4, 6],
        //       "summary": { ... }
        //     }
        //   }
        // }

        // Placeholder implementation - would use serde_json in production
        let _ = content; // Avoid unused warning
        Ok(data)
    }

    /// Parse coverage data from pytest-cov output
    pub fn from_pytest_cov(cov_file: &Path) -> Result<Self, std::io::Error> {
        use std::fs;

        let _content = fs::read_to_string(cov_file)?;
        let data = CoverageData::new();

        // Placeholder implementation - would parse pytest-cov format
        // Format is typically similar to coverage.py JSON

        Ok(data)
    }
}

impl Default for CoverageData {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for dead code analysis
#[derive(Debug, Clone)]
pub struct AnalysisConfig {
    pub high_confidence_threshold: f32,
    pub medium_confidence_threshold: f32,
    pub respect_suppression_comments: bool,
    pub include_private_api: bool,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            high_confidence_threshold: 0.8,
            medium_confidence_threshold: 0.5,
            respect_suppression_comments: true,
            include_private_api: true,
        }
    }
}

impl EnhancedDeadCodeAnalyzer {
    pub fn new() -> Self {
        Self {
            framework_detector: FrameworkPatternRegistry::new(),
            test_detector: PythonTestDetector::new(),
            callback_tracker: CallbackTracker::new(),
            import_trackers: HashMap::new(),
            coverage_data: None,
            config: AnalysisConfig::default(),
        }
    }

    pub fn with_config(mut self, config: AnalysisConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_coverage(mut self, coverage: CoverageData) -> Self {
        self.coverage_data = Some(coverage);
        self
    }

    /// Register an import tracker for a file
    pub fn register_import_tracker(&mut self, file: PathBuf, tracker: ImportTracker) {
        self.import_trackers.insert(file, tracker);
    }

    /// Register callback tracker
    pub fn register_callback_tracker(&mut self, tracker: CallbackTracker) {
        self.callback_tracker = tracker;
    }

    /// Analyze a function to determine if it's dead code
    pub fn analyze_function(
        &self,
        func: &FunctionMetrics,
        call_graph: &CallGraph,
    ) -> DeadCodeResult {
        let func_id = FunctionId {
            name: func.name.clone(),
            file: func.file.clone(),
            line: func.line,
        };

        // Check for suppression comment
        if self.config.respect_suppression_comments && self.should_suppress(func) {
            return DeadCodeResult {
                function_id: func_id,
                is_dead: false,
                confidence: DeadCodeConfidence::Low(0.0),
                dead_reasons: vec![],
                live_reasons: vec![LiveCodeReason::PublicApi],
                suggestion: RemovalSuggestion {
                    can_remove: false,
                    safe_to_remove: false,
                    explanation: "Function has suppression comment".to_string(),
                    risks: vec![],
                },
            };
        }

        // Gather confidence factors
        let factors = self.gather_confidence_factors(func, call_graph, &func_id);

        // Calculate confidence and determine if dead
        let (is_dead, confidence) = self.calculate_confidence(&factors, &func.name);

        // Gather reasons
        let (dead_reasons, live_reasons) = self.gather_reasons(&factors);

        // Generate suggestion
        let suggestion = self.generate_suggestion(&factors, confidence);

        DeadCodeResult {
            function_id: func_id,
            is_dead,
            confidence,
            dead_reasons,
            live_reasons,
            suggestion,
        }
    }

    /// Gather all confidence factors for a function
    fn gather_confidence_factors(
        &self,
        func: &FunctionMetrics,
        call_graph: &CallGraph,
        func_id: &FunctionId,
    ) -> ConfidenceFactors {
        let method_name = extract_method_name(&func.name);

        // Check if function has callers
        let has_callers =
            !call_graph.get_callers(func_id).is_empty() || self.has_coverage_data(func);

        // Check if it's a framework entry point or event handler
        let is_framework_entry = self.is_framework_entry_point(&func.name, &func.file)
            || self.framework_detector.is_event_handler(&func.name);

        // Check if it's a test function
        let in_test_file = self.test_detector.is_test_file(&func.file);
        let is_test_function = in_test_file || is_test_name(&func.name);

        // Check if it's a callback target using the callback tracker
        let is_callback_target = self.callback_tracker.is_callback_target(&func.name);

        // Check if it's exported in __all__
        let is_exported = self.is_exported_in_all(&func.name, &func.file);

        // Check if it's public (doesn't start with _)
        let is_public = !method_name.starts_with('_');

        // Check if it's a magic method
        let is_magic_method = is_magic_method_name(method_name);

        // Check if it's a main entry point
        let is_main_entry = is_main_entry_point(&func.name);

        // Check if it has property decorator
        let is_property = self.has_property_decorator(&func.name, &func.file, func.line);

        ConfidenceFactors {
            has_callers,
            is_framework_entry,
            is_test_function,
            is_callback_target,
            is_exported,
            is_public,
            is_magic_method,
            is_main_entry,
            is_property,
            in_test_file,
        }
    }

    /// Calculate confidence score and determine if code is dead
    fn calculate_confidence(
        &self,
        factors: &ConfidenceFactors,
        func_name: &str,
    ) -> (bool, DeadCodeConfidence) {
        let mut score = 1.0f32; // Start with assumption it's dead

        // Strong indicators it's NOT dead (reduce score dramatically)
        if factors.has_callers {
            return (false, DeadCodeConfidence::Low(0.0));
        }

        if factors.is_magic_method {
            return (false, DeadCodeConfidence::Low(0.0));
        }

        if factors.is_main_entry {
            return (false, DeadCodeConfidence::Low(0.1));
        }

        if factors.is_framework_entry {
            score *= 0.2; // Very unlikely to be dead
        }

        if factors.is_test_function {
            score *= 0.3; // Unlikely to be dead
        }

        if factors.is_callback_target {
            score *= 0.3; // Unlikely to be dead
        }

        if factors.is_exported {
            score *= 0.4; // Less likely to be dead
        }

        if factors.is_property {
            score *= 0.5; // Could be used via property access
        }

        // Weak indicators
        if factors.is_public && !factors.in_test_file {
            score *= 0.6; // Public functions more likely to be used externally
        }

        // Entry point names (common function names that might be entry points)
        let method_name = extract_method_name(func_name);
        if matches!(
            method_name,
            "main"
                | "run"
                | "cli"
                | "start"
                | "execute"
                | "launch"
                | "bootstrap"
                | "initialize"
                | "setup"
                | "configure"
                | "finalize"
                | "index"
                | "handler"
                | "process"
                | "validate"
                | "transform"
                | "handle"
                | "view"
                | "setup_view"
                | "api_handler"
                | "entrypoint"
                | "script"
        ) {
            score *= 0.3; // Common entry point names
        }

        // If no callers and none of the above apply, likely dead
        let is_dead = score >= self.config.medium_confidence_threshold;
        let confidence = DeadCodeConfidence::from_score(score);

        (is_dead, confidence)
    }

    /// Gather reasons for the dead code determination
    fn gather_reasons(
        &self,
        factors: &ConfidenceFactors,
    ) -> (Vec<DeadCodeReason>, Vec<LiveCodeReason>) {
        let mut dead_reasons = Vec::new();
        let mut live_reasons = Vec::new();

        // Dead code reasons
        if !factors.has_callers {
            dead_reasons.push(DeadCodeReason::NoStaticCallers);
        }
        if !factors.is_exported {
            dead_reasons.push(DeadCodeReason::NotExported);
        }
        if !factors.is_public {
            dead_reasons.push(DeadCodeReason::PrivateFunction);
        }
        if !factors.in_test_file {
            dead_reasons.push(DeadCodeReason::NotInTestFile);
        }
        if !factors.is_framework_entry {
            dead_reasons.push(DeadCodeReason::NoFrameworkPattern);
        }
        if !factors.is_callback_target {
            dead_reasons.push(DeadCodeReason::NoCallbackRegistration);
        }

        // Live code reasons
        if factors.has_callers {
            live_reasons.push(LiveCodeReason::HasStaticCallers);
        }
        if factors.is_framework_entry {
            live_reasons.push(LiveCodeReason::FrameworkEntryPoint);
        }
        if factors.is_test_function {
            live_reasons.push(LiveCodeReason::TestFunction);
        }
        if factors.is_callback_target {
            live_reasons.push(LiveCodeReason::CallbackTarget);
        }
        if factors.is_exported {
            live_reasons.push(LiveCodeReason::ExportedInAll);
        }
        if factors.is_public {
            live_reasons.push(LiveCodeReason::PublicApi);
        }
        if factors.is_magic_method {
            live_reasons.push(LiveCodeReason::MagicMethod);
        }
        if factors.is_property {
            live_reasons.push(LiveCodeReason::PropertyDecorator);
        }
        if factors.is_main_entry {
            live_reasons.push(LiveCodeReason::MainEntryPoint);
        }

        (dead_reasons, live_reasons)
    }

    /// Generate removal suggestion
    fn generate_suggestion(
        &self,
        factors: &ConfidenceFactors,
        confidence: DeadCodeConfidence,
    ) -> RemovalSuggestion {
        let score = confidence.score();

        let can_remove = score >= self.config.medium_confidence_threshold;
        let safe_to_remove = score >= self.config.high_confidence_threshold;

        let explanation = if !can_remove {
            "Function appears to be in use or is a framework/test entry point.".to_string()
        } else if safe_to_remove {
            "High confidence this function is dead code and can be safely removed.".to_string()
        } else {
            "Medium confidence this function is dead code. Manual verification recommended."
                .to_string()
        };

        let mut risks = Vec::new();

        if factors.is_public {
            risks.push("Function is public and may be used by external code.".to_string());
        }

        if factors.is_framework_entry {
            risks.push("Function may be a framework entry point.".to_string());
        }

        if factors.is_callback_target {
            risks.push("Function may be used as a callback.".to_string());
        }

        if factors.in_test_file {
            risks.push("Function is in a test file and may be a test helper.".to_string());
        }

        RemovalSuggestion {
            can_remove,
            safe_to_remove,
            explanation,
            risks,
        }
    }

    /// Check if function has suppression comment
    /// Looks for "# debtmap: not-dead" or "# noqa: dead-code" comments
    fn should_suppress(&self, func: &FunctionMetrics) -> bool {
        use std::fs;

        // Read the source file
        let Ok(content) = fs::read_to_string(&func.file) else {
            return false;
        };

        let lines: Vec<&str> = content.lines().collect();

        // Check the function definition line and the line before it
        let line_idx = func.line.saturating_sub(1); // Convert to 0-indexed

        if line_idx >= lines.len() {
            return false;
        }

        // Check the line above the function definition (common for comments)
        if line_idx > 0 {
            let prev_line = lines[line_idx - 1].trim();
            if Self::is_suppression_comment(prev_line) {
                return true;
            }
        }

        // Check the function definition line itself (inline comment)
        let func_line = lines[line_idx];
        if Self::is_suppression_comment(func_line) {
            return true;
        }

        // Check the line after the function definition (might have comment)
        if line_idx + 1 < lines.len() {
            let next_line = lines[line_idx + 1].trim();
            if Self::is_suppression_comment(next_line) {
                return true;
            }
        }

        false
    }

    /// Check if a line contains a suppression comment
    fn is_suppression_comment(line: &str) -> bool {
        let line_lower = line.to_lowercase();
        line_lower.contains("# debtmap: not-dead")
            || line_lower.contains("# debtmap:not-dead")
            || line_lower.contains("#debtmap: not-dead")
            || line_lower.contains("#debtmap:not-dead")
            || line_lower.contains("# noqa: dead-code")
            || line_lower.contains("# noqa:dead-code")
    }

    /// Check if function is a framework entry point
    fn is_framework_entry_point(&self, func_name: &str, _file_path: &Path) -> bool {
        // Check if it's an entry point using the existing framework detector
        self.framework_detector.is_entry_point(func_name, &[])
    }

    /// Check if function has coverage data indicating it was executed
    fn has_coverage_data(&self, func: &FunctionMetrics) -> bool {
        if let Some(coverage) = &self.coverage_data {
            // Check if the function's line is covered or executed
            coverage.is_line_covered(&func.file, func.line)
                || coverage.is_line_executed(&func.file, func.line)
        } else {
            false
        }
    }

    /// Check if function is exported in __all__
    fn is_exported_in_all(&self, func_name: &str, file_path: &Path) -> bool {
        use std::fs;

        // Extract just the function name (not Class.method)
        let simple_name = extract_method_name(func_name);

        // Read the source file
        let Ok(content) = fs::read_to_string(file_path) else {
            return false;
        };

        // Look for __all__ declaration
        // Common patterns:
        // __all__ = ["func1", "func2"]
        // __all__ = ['func1', 'func2']
        // __all__ = ("func1", "func2")

        // Simple regex-like search for __all__ declarations
        for line in content.lines() {
            let trimmed = line.trim();

            // Skip comments
            if trimmed.starts_with('#') {
                continue;
            }

            // Check if this line contains __all__
            if trimmed.contains("__all__") {
                // Check if our function name is in this __all__ declaration
                // Look for both single and double quoted versions
                let double_quoted = format!("\"{}\"", simple_name);
                let single_quoted = format!("'{}'", simple_name);

                if trimmed.contains(&double_quoted) || trimmed.contains(&single_quoted) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if function has @property decorator
    fn has_property_decorator(&self, _func_name: &str, file_path: &Path, line: usize) -> bool {
        use std::fs;

        // Read the source file
        let Ok(content) = fs::read_to_string(file_path) else {
            return false;
        };

        let lines: Vec<&str> = content.lines().collect();
        let line_idx = line.saturating_sub(1); // Convert to 0-indexed

        if line_idx >= lines.len() {
            return false;
        }

        // Check up to 5 lines before the function definition for decorators
        let start_idx = line_idx.saturating_sub(5);
        for i in start_idx..line_idx {
            if i >= lines.len() {
                break;
            }

            let trimmed = lines[i].trim();

            // Check for @property decorator
            if trimmed == "@property"
                || trimmed.starts_with("@property ")
                || trimmed.starts_with("@property(")
            {
                return true;
            }

            // Also check for property-related decorators
            if trimmed == "@cached_property"
                || trimmed.starts_with("@cached_property ")
                || trimmed.starts_with("@cached_property(")
            {
                return true;
            }

            // Stop if we hit a non-decorator, non-comment line
            if !trimmed.starts_with('@') && !trimmed.starts_with('#') && !trimmed.is_empty() {
                break;
            }
        }

        false
    }

    /// Generate detailed explanation for a result
    pub fn generate_explanation(&self, result: &DeadCodeResult) -> String {
        let mut explanation = String::new();

        explanation.push_str(&format!(
            "Dead code analysis for '{}':\n",
            result.function_id.name
        ));
        explanation.push_str(&format!(
            "  Result: {}\n",
            if result.is_dead { "DEAD" } else { "LIVE" }
        ));
        explanation.push_str(&format!(
            "  Confidence: {:?} ({:.2})\n",
            result.confidence,
            result.confidence.score()
        ));

        if !result.live_reasons.is_empty() {
            explanation.push_str("\n  Reasons it's LIVE:\n");
            for reason in &result.live_reasons {
                explanation.push_str(&format!("    - {:?}\n", reason));
            }
        }

        if !result.dead_reasons.is_empty() {
            explanation.push_str("\n  Reasons it might be DEAD:\n");
            for reason in &result.dead_reasons {
                explanation.push_str(&format!("    - {:?}\n", reason));
            }
        }

        explanation.push_str("\n  Suggestion:\n");
        explanation.push_str(&format!("    {}\n", result.suggestion.explanation));

        if !result.suggestion.risks.is_empty() {
            explanation.push_str("\n  Risks:\n");
            for risk in &result.suggestion.risks {
                explanation.push_str(&format!("    - {}\n", risk));
            }
        }

        explanation
    }
}

impl Default for EnhancedDeadCodeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

// Helper functions

fn extract_method_name(full_name: &str) -> &str {
    if let Some(pos) = full_name.rfind('.') {
        &full_name[pos + 1..]
    } else {
        full_name
    }
}

fn is_magic_method_name(name: &str) -> bool {
    name.starts_with("__") && name.ends_with("__") && name.len() > 4
}

fn is_main_entry_point(name: &str) -> bool {
    name == "main" || name == "cli" || name == "run" || name.ends_with(".main")
}

fn is_test_name(name: &str) -> bool {
    let method_name = extract_method_name(name);
    method_name.starts_with("test_")
        || method_name.starts_with("Test")
        || method_name == "setUp"
        || method_name == "tearDown"
        || method_name == "setup_method"
        || method_name == "teardown_method"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_from_score() {
        assert!(matches!(
            DeadCodeConfidence::from_score(0.9),
            DeadCodeConfidence::High(_)
        ));
        assert!(matches!(
            DeadCodeConfidence::from_score(0.6),
            DeadCodeConfidence::Medium(_)
        ));
        assert!(matches!(
            DeadCodeConfidence::from_score(0.3),
            DeadCodeConfidence::Low(_)
        ));
    }

    #[test]
    fn test_extract_method_name() {
        assert_eq!(extract_method_name("MyClass.method"), "method");
        assert_eq!(extract_method_name("method"), "method");
        assert_eq!(extract_method_name("module.Class.method"), "method");
    }

    #[test]
    fn test_is_magic_method_name() {
        assert!(is_magic_method_name("__init__"));
        assert!(is_magic_method_name("__str__"));
        assert!(is_magic_method_name("__getitem__"));
        assert!(!is_magic_method_name("__private"));
        assert!(!is_magic_method_name("normal_method"));
    }

    #[test]
    fn test_is_main_entry_point() {
        assert!(is_main_entry_point("main"));
        assert!(is_main_entry_point("cli"));
        assert!(is_main_entry_point("run"));
        assert!(is_main_entry_point("module.main"));
        assert!(!is_main_entry_point("do_main"));
    }

    #[test]
    fn test_is_test_name() {
        assert!(is_test_name("test_example"));
        assert!(is_test_name("TestCase.test_method"));
        assert!(is_test_name("setUp"));
        assert!(is_test_name("MyClass.tearDown"));
        assert!(!is_test_name("helper_function"));
    }

    #[test]
    fn test_analyze_function_with_callers() {
        let analyzer = EnhancedDeadCodeAnalyzer::new();
        let mut call_graph = CallGraph::new();

        let func_id = FunctionId {
            name: "my_function".to_string(),
            file: PathBuf::from("test.py"),
            line: 10,
        };

        let caller_id = FunctionId {
            name: "caller".to_string(),
            file: PathBuf::from("test.py"),
            line: 5,
        };

        // Add a call to make the function live
        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: caller_id.clone(),
            callee: func_id.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });

        let func = FunctionMetrics::new("my_function".to_string(), PathBuf::from("test.py"), 10);

        let result = analyzer.analyze_function(&func, &call_graph);

        assert!(!result.is_dead);
        assert!(matches!(result.confidence, DeadCodeConfidence::Low(_)));
        assert!(result
            .live_reasons
            .contains(&LiveCodeReason::HasStaticCallers));
    }

    #[test]
    fn test_analyze_magic_method() {
        let analyzer = EnhancedDeadCodeAnalyzer::new();
        let call_graph = CallGraph::new();

        let func =
            FunctionMetrics::new("MyClass.__init__".to_string(), PathBuf::from("test.py"), 10);

        let result = analyzer.analyze_function(&func, &call_graph);

        assert!(!result.is_dead);
        assert!(matches!(result.confidence, DeadCodeConfidence::Low(_)));
        assert!(result.live_reasons.contains(&LiveCodeReason::MagicMethod));
    }

    #[test]
    fn test_analyze_dead_private_function() {
        let analyzer = EnhancedDeadCodeAnalyzer::new();
        let call_graph = CallGraph::new();

        let func = FunctionMetrics::new("_private_helper".to_string(), PathBuf::from("app.py"), 20);

        let result = analyzer.analyze_function(&func, &call_graph);

        assert!(result.is_dead);
        assert!(result
            .dead_reasons
            .contains(&DeadCodeReason::NoStaticCallers));
        assert!(result
            .dead_reasons
            .contains(&DeadCodeReason::PrivateFunction));
    }
}
