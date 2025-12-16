use crate::priority::score_types::Score0To100;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDebtMetrics {
    pub path: PathBuf,
    pub total_lines: usize,
    pub function_count: usize,
    pub class_count: usize,
    pub avg_complexity: f64,
    pub max_complexity: u32,
    pub total_complexity: u32,
    pub coverage_percent: f64,
    pub uncovered_lines: usize,
    #[serde(alias = "god_object_indicators")]
    pub god_object_analysis: Option<crate::organization::GodObjectAnalysis>,
    pub function_scores: Vec<f64>,
    /// The specific type of god object detected (if any).
    ///
    /// This field contains the classification of god object patterns:
    /// - `GodModule`: A module with too many related structs/types that should be split
    /// - `TraditionalGodObject`: A single class/struct with too many responsibilities
    /// - `Boilerplate`: Repetitive code that should be macro-ified (NOT a god object)
    /// - `Registry`: A lookup table or mapping structure (NOT a god object)
    ///
    /// This type is used to determine the appropriate recommendation, following this precedence:
    /// 1. Boilerplate → recommend macros/codegen
    /// 2. Registry → recommend keeping as-is or data-driven approach
    /// 3. TraditionalGodObject → recommend extracting classes/modules
    /// 4. GodModule → recommend splitting into multiple modules
    /// 5. None → use general refactoring recommendations based on size/complexity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub god_object_type: Option<crate::organization::GodObjectType>,

    /// File type classification for context-aware thresholds (spec 135)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_type: Option<crate::organization::FileType>,

    // === File-level dependency metrics (spec 201) ===
    /// Afferent coupling - number of files that depend on this file
    #[serde(default)]
    pub afferent_coupling: usize,
    /// Efferent coupling - number of files this file depends on
    #[serde(default)]
    pub efferent_coupling: usize,
    /// Instability metric (0.0 = stable, 1.0 = unstable)
    /// Calculated as Ce / (Ca + Ce)
    #[serde(default)]
    pub instability: f64,
    /// List of files that depend on this file (top N)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependents: Vec<String>,
    /// List of files this file depends on (top N)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies_list: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GodObjectIndicators {
    pub methods_count: usize,
    pub fields_count: usize,
    pub responsibilities: usize,
    pub is_god_object: bool,
    pub god_object_score: Score0To100,
    /// Detailed list of identified responsibilities (e.g., "data_access", "validation")
    #[serde(default)]
    pub responsibility_names: Vec<String>,
    /// Recommended module splits with methods to move
    #[serde(default)]
    pub recommended_splits: Vec<ModuleSplit>,
    /// Detailed module structure analysis (for enhanced reporting)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_structure: Option<crate::analysis::ModuleStructure>,
    /// Number of distinct semantic domains detected
    #[serde(default)]
    pub domain_count: usize,
    /// Domain diversity score (0.0 to 1.0)
    #[serde(default)]
    pub domain_diversity: f64,
    /// Ratio of struct definitions to total functions (0.0 to 1.0)
    #[serde(default)]
    pub struct_ratio: f64,
    /// Analysis method used for recommendations
    #[serde(default)]
    pub analysis_method: SplitAnalysisMethod,
    /// Severity of cross-domain mixing (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cross_domain_severity: Option<RecommendationSeverity>,
    /// Domain diversity metrics with detailed distribution (spec 152)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_diversity_metrics: Option<crate::organization::DomainDiversityMetrics>,
    /// Type of god object detection (GodClass, GodFile, or GodModule) (spec 155)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detection_type: Option<crate::organization::DetectionType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSplit {
    pub suggested_name: String,
    pub methods_to_move: Vec<String>,
    #[serde(default)]
    pub structs_to_move: Vec<String>,
    pub responsibility: String,
    pub estimated_lines: usize,
    #[serde(default)]
    pub method_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    #[serde(default)]
    pub priority: Priority,
    /// Semantic domain this split represents
    #[serde(default)]
    pub domain: String,
    /// Explanation of why this split was suggested
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    /// Analysis method that generated this split
    #[serde(default)]
    pub method: SplitAnalysisMethod,
    /// Severity of this recommendation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<RecommendationSeverity>,
    /// Multi-signal classification evidence for this split (spec 148)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification_evidence:
        Option<crate::analysis::multi_signal_aggregation::AggregatedClassification>,
    /// Representative method names to show in recommendations (Spec 178)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub representative_methods: Vec<String>,
    /// Fields from original struct needed by this extracted module (Spec 178)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields_needed: Vec<String>,
    /// Suggested trait extraction for this behavioral group (Spec 178)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trait_suggestion: Option<String>,
    /// Behavioral category for this split (Spec 178)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behavior_category: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Priority {
    High,
    #[default]
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SplitAnalysisMethod {
    #[default]
    None,
    CrossDomain,
    MethodBased,
    Hybrid,
    TypeBased,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RecommendationSeverity {
    Critical,
    High,
    Medium,
    Low,
}

impl From<crate::organization::Priority> for Priority {
    fn from(p: crate::organization::Priority) -> Self {
        match p {
            crate::organization::Priority::High => Priority::High,
            crate::organization::Priority::Medium => Priority::Medium,
            crate::organization::Priority::Low => Priority::Low,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDebtItem {
    pub metrics: FileDebtMetrics,
    #[serde(default)]
    pub score: f64,
    #[serde(default)]
    pub priority_rank: usize,
    #[serde(default = "default_recommendation")]
    pub recommendation: String,
    #[serde(default)]
    pub impact: FileImpact,
}

fn default_recommendation() -> String {
    "Refactor for better maintainability".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileImpact {
    pub complexity_reduction: f64,
    pub maintainability_improvement: f64,
    pub test_effort: f64,
}

impl Default for FileImpact {
    fn default() -> Self {
        Self {
            complexity_reduction: 0.0,
            maintainability_improvement: 0.0,
            test_effort: 0.0,
        }
    }
}

/// Detailed breakdown of file score calculation factors.
///
/// This struct contains all the individual multiplicative factors used to calculate
/// a file's technical debt score, along with the basis values used for each calculation.
/// It enables transparent display of how a file's score was computed.
#[derive(Debug, Clone)]
pub struct FileScoreFactors {
    /// Size impact factor: sqrt(lines / 100)
    pub size_factor: f64,
    /// Total lines of code in the file
    pub size_basis: usize,

    /// Complexity impact factor: (avg_complexity / 5.0) × sqrt(total_complexity / 50.0)
    pub complexity_factor: f64,
    /// Average cyclomatic complexity across functions
    pub avg_complexity: f64,
    /// Sum of all function complexities
    pub total_complexity: u32,

    /// Coverage gap factor: (1.0 - coverage) × 2.0 + 1.0
    pub coverage_factor: f64,
    /// Test coverage percentage (0.0 to 1.0)
    pub coverage_percent: f64,
    /// Coverage gap (1.0 - coverage_percent)
    pub coverage_gap: f64,

    /// Function density factor: 1.0 + (functions - 50) × 0.02 for >50 functions
    pub density_factor: f64,
    /// Number of functions in the file
    pub function_count: usize,

    /// God object penalty multiplier: 2.0 + god_object_score if flagged
    pub god_object_multiplier: f64,
    /// God object detection score
    pub god_object_score: Score0To100,
    /// Whether file is flagged as a god object
    pub is_god_object: bool,

    /// Function score aggregate factor: max(sum / 10.0, 1.0)
    pub function_factor: f64,
    /// Sum of all individual function debt scores
    pub function_score_sum: f64,
}

impl FileDebtMetrics {
    pub fn calculate_score(&self) -> f64 {
        // Size factor: larger files have higher impact
        let size_factor = (self.total_lines as f64 / 100.0).sqrt();

        // Complexity factor: average and total complexity
        let avg_complexity_factor = (self.avg_complexity / 5.0).min(3.0);
        let total_complexity_factor = (self.total_complexity as f64 / 50.0).sqrt();
        let complexity_factor = avg_complexity_factor * total_complexity_factor;

        // Coverage factor: lower coverage = higher score
        let coverage_gap = 1.0 - self.coverage_percent;
        let coverage_factor = (coverage_gap * 2.0) + 1.0;

        // Function density: too many functions = god object
        let density_factor = if self.function_count > 50 {
            1.0 + ((self.function_count - 50) as f64 * 0.02)
        } else {
            1.0
        };

        // God object multiplier - scale score proportionally to avoid extreme inflation
        // Score 0-100 maps to multiplier 1.0x-3.0x (not 2x-102x!)
        // This aligns with contextual risk cap (max 3x) for consistent scoring
        let god_object_multiplier = if let Some(ref analysis) = self.god_object_analysis {
            if analysis.is_god_object {
                1.0 + (analysis.god_object_score.value() / 50.0)
            } else {
                1.0
            }
        } else {
            1.0
        };

        // Aggregate function scores
        let function_score_sum: f64 = self.function_scores.iter().sum();
        let function_factor = (function_score_sum / 10.0).max(1.0);

        // Calculate final score
        size_factor
            * complexity_factor
            * coverage_factor
            * density_factor
            * god_object_multiplier
            * function_factor
    }

    /// Extract individual scoring factors for display purposes.
    ///
    /// This method decomposes the opaque score calculation from `calculate_score()`
    /// into individual factors that can be shown to users for transparency.
    ///
    /// # Returns
    ///
    /// `FileScoreFactors` containing:
    /// - All 6 multiplicative factors (size, complexity, coverage, density, god object, function)
    /// - Basis values used to calculate each factor
    /// - Contextual information for display (e.g., whether flagged as god object)
    ///
    /// # Example
    ///
    /// ```
    /// # use debtmap::priority::file_metrics::{FileDebtMetrics, GodObjectIndicators};
    /// # use std::path::PathBuf;
    /// let mut metrics = FileDebtMetrics::default();
    /// metrics.total_lines = 400;
    /// metrics.coverage_percent = 0.75;
    /// let factors = metrics.get_score_factors();
    /// println!("Coverage factor: {:.2} ({:.0}% coverage)",
    ///          factors.coverage_factor,
    ///          factors.coverage_percent * 100.0);
    /// ```
    pub fn get_score_factors(&self) -> FileScoreFactors {
        // Size factor: larger files have higher impact
        let size_factor = (self.total_lines as f64 / 100.0).sqrt();

        // Complexity factor: average and total complexity
        let avg_complexity_factor = (self.avg_complexity / 5.0).min(3.0);
        let total_complexity_factor = (self.total_complexity as f64 / 50.0).sqrt();
        let complexity_factor = avg_complexity_factor * total_complexity_factor;

        // Coverage factor: lower coverage = higher score
        let coverage_gap = 1.0 - self.coverage_percent;
        let coverage_factor = (coverage_gap * 2.0) + 1.0;

        // Function density: too many functions = god object
        let density_factor = if self.function_count > 50 {
            1.0 + ((self.function_count - 50) as f64 * 0.02)
        } else {
            1.0
        };

        // God object multiplier - scale score proportionally to avoid extreme inflation
        // Score 0-100 maps to multiplier 1.0x-3.0x (not 2x-102x!)
        // This aligns with contextual risk cap (max 3x) for consistent scoring
        let god_object_multiplier = if let Some(ref analysis) = self.god_object_analysis {
            if analysis.is_god_object {
                1.0 + (analysis.god_object_score.value() / 50.0)
            } else {
                1.0
            }
        } else {
            1.0
        };

        // Aggregate function scores
        let function_score_sum: f64 = self.function_scores.iter().sum();
        let function_factor = (function_score_sum / 10.0).max(1.0);

        let (god_object_score, is_god_object) = if let Some(ref analysis) = self.god_object_analysis
        {
            (analysis.god_object_score, analysis.is_god_object)
        } else {
            (Score0To100::new(0.0), false)
        };

        FileScoreFactors {
            size_factor,
            size_basis: self.total_lines,
            complexity_factor,
            avg_complexity: self.avg_complexity,
            total_complexity: self.total_complexity,
            coverage_factor,
            coverage_percent: self.coverage_percent,
            coverage_gap,
            density_factor,
            function_count: self.function_count,
            god_object_multiplier,
            god_object_score,
            is_god_object,
            function_factor,
            function_score_sum,
        }
    }

    /// Generate a recommendation for addressing this file's technical debt.
    ///
    /// This function uses a **precedence-based strategy** to select the most appropriate
    /// recommendation type, checking patterns in this order:
    ///
    /// 1. **Boilerplate Pattern** (highest priority)
    ///    - Detected when file has many repetitive trait implementations
    ///    - Recommendation: Use macros or code generation to reduce repetition
    ///    - Example: ripgrep's flags/defs.rs with 888 Flag trait implementations
    ///
    /// 2. **Registry Pattern**
    ///    - Detected when file is primarily a lookup table or data mapping
    ///    - Recommendation: Keep as-is or convert to data-driven approach
    ///    - Example: Error code registries, configuration tables
    ///
    /// 3. **God Object**
    ///    - Detected when file has too many responsibilities (via god_object_analysis)
    ///    - Recommendation: Split into multiple focused modules
    ///    - Context-specific advice based on file type (parser, cache, analyzer, etc.)
    ///
    /// 4. **Large File** (>500 lines)
    ///    - Recommendation: Extract complex functions to reduce size
    ///
    /// 5. **Complex Functions** (avg complexity >10)
    ///    - Recommendation: Simplify logic, extract helper functions
    ///
    /// 6. **Low Coverage** (<50%)
    ///    - Recommendation: Add tests for uncovered code
    ///
    /// The precedence ensures that boilerplate files don't get incorrectly flagged
    /// as god objects requiring module splitting.
    pub fn generate_recommendation(&self) -> String {
        // First check for boilerplate pattern - highest priority
        if let Some(crate::organization::GodObjectType::BoilerplatePattern {
            recommendation, ..
        }) = &self.god_object_type
        {
            return recommendation.clone();
        }

        // Build base recommendation
        let base_recommendation = if self
            .god_object_analysis
            .as_ref()
            .is_some_and(|a| a.is_god_object)
        {
            // Analyze the file path to provide context-specific recommendations
            let file_name = self
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("module");

            let is_parser = file_name.contains("pars") || file_name.contains("lexer");
            let is_cache = file_name.contains("cache");
            let is_writer = file_name.contains("writer") || file_name.contains("format");
            let is_analyzer = file_name.contains("analyz") || file_name.contains("detect");

            // Generate specific splitting recommendations based on file type
            if is_parser {
                format!(
                    "Split parser into {} modules: 1) Tokenizer/Lexer 2) AST builder 3) Visitor/Walker 4) Error handling. Group by parsing phase, not by node type.",
                    (self.function_count / 20).clamp(3, 5)
                )
            } else if is_cache {
                "Split cache into: 1) Storage backend 2) Eviction policy 3) Serialization 4) Cache operations. Separate policy from mechanism.".to_string()
            } else if is_writer {
                "Split into: 1) Core formatter 2) Section writers (one per major section) 3) Style/theme handling. Max 20 functions per writer module.".to_string()
            } else if is_analyzer {
                "Split by analysis phase: 1) Data collection 2) Pattern detection 3) Scoring/metrics 4) Reporting. Keep related analyses together.".to_string()
            } else {
                // Generic but more specific recommendation
                format!(
                    "URGENT: {} lines, {} functions! Split by data flow: 1) Input/parsing functions 2) Core logic/transformation 3) Output/formatting. Create {} focused modules with <30 functions each.",
                    self.total_lines, self.function_count,
                    (self.function_count / 25).clamp(3, 8)
                )
            }
        } else if self.total_lines > 500 {
            format!(
                "Extract complex functions, reduce file to <500 lines. Current: {} lines",
                self.total_lines
            )
        } else if self.avg_complexity > 10.0 {
            "Simplify complex functions. Consider extracting helper functions or breaking down logic.".to_string()
        } else if self.coverage_percent < 0.5 {
            format!(
                "Increase test coverage from {:.1}% to at least 80%",
                self.coverage_percent * 100.0
            )
        } else {
            "Refactor for better maintainability and testability".to_string()
        };

        // Add coupling and instability context (spec 201)
        let coupling_context = self.generate_coupling_context();
        let instability_context = self.generate_instability_context();

        // Combine recommendations
        if !coupling_context.is_empty() || !instability_context.is_empty() {
            let mut parts = vec![base_recommendation];
            if !coupling_context.is_empty() {
                parts.push(coupling_context);
            }
            if !instability_context.is_empty() {
                parts.push(instability_context);
            }
            parts.join(" ")
        } else {
            base_recommendation
        }
    }

    /// Generate coupling warning context for highly coupled files (spec 201).
    ///
    /// Files with Ca + Ce > 15 are considered highly coupled and should
    /// show a warning in recommendations.
    fn generate_coupling_context(&self) -> String {
        let total_coupling = self.afferent_coupling + self.efferent_coupling;

        if total_coupling > 15 {
            format!(
                "[COUPLING WARNING: Ca={}, Ce={}, total={}. Consider reducing dependencies to improve modularity.]",
                self.afferent_coupling, self.efferent_coupling, total_coupling
            )
        } else {
            String::new()
        }
    }

    /// Generate instability context for files with extreme instability values (spec 201).
    ///
    /// - I > 0.9: Highly unstable - changes here propagate easily, low risk to modify
    /// - I < 0.1: Highly stable - many dependents, changes need careful review
    fn generate_instability_context(&self) -> String {
        // Only provide context when we have coupling data
        let total_coupling = self.afferent_coupling + self.efferent_coupling;
        if total_coupling == 0 {
            return String::new();
        }

        if self.instability > 0.9 {
            "[UNSTABLE: I>0.9 - changes here have few dependents, safe to refactor.]".to_string()
        } else if self.instability < 0.1 {
            "[STABLE: I<0.1 - many files depend on this, changes need careful review.]".to_string()
        } else {
            String::new()
        }
    }
}

impl Default for FileDebtMetrics {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            total_lines: 0,
            function_count: 0,
            class_count: 0,
            avg_complexity: 0.0,
            max_complexity: 0,
            total_complexity: 0,
            coverage_percent: 0.0,
            uncovered_lines: 0,
            god_object_analysis: None,
            function_scores: Vec::new(),
            god_object_type: None,
            file_type: None,
            afferent_coupling: 0,
            efferent_coupling: 0,
            instability: 0.0,
            dependents: Vec::new(),
            dependencies_list: Vec::new(),
        }
    }
}

impl Default for GodObjectIndicators {
    fn default() -> Self {
        Self {
            methods_count: 0,
            fields_count: 0,
            responsibilities: 0,
            is_god_object: false,
            god_object_score: Score0To100::new(0.0),
            responsibility_names: Vec::new(),
            recommended_splits: Vec::new(),
            module_structure: None,
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
            detection_type: None,
        }
    }
}

impl From<crate::organization::GodObjectAnalysis> for GodObjectIndicators {
    fn from(analysis: crate::organization::GodObjectAnalysis) -> Self {
        Self {
            methods_count: analysis.method_count,
            fields_count: analysis.field_count,
            responsibilities: analysis.responsibility_count,
            is_god_object: analysis.is_god_object,
            god_object_score: Score0To100::new(analysis.god_object_score.value() / 100.0), // Convert back from percentage
            responsibility_names: analysis.responsibilities,
            recommended_splits: Vec::new(), // GodObjectAnalysis uses different split structure
            module_structure: analysis.module_structure,
            domain_count: analysis.domain_count,
            domain_diversity: analysis.domain_diversity,
            struct_ratio: analysis.struct_ratio,
            analysis_method: convert_from_org_split_method(analysis.analysis_method),
            cross_domain_severity: analysis
                .cross_domain_severity
                .map(convert_from_org_severity),
            domain_diversity_metrics: analysis.domain_diversity_metrics,
            detection_type: Some(analysis.detection_type),
        }
    }
}

fn convert_from_org_split_method(
    method: crate::organization::SplitAnalysisMethod,
) -> SplitAnalysisMethod {
    match method {
        crate::organization::SplitAnalysisMethod::None => SplitAnalysisMethod::None,
        crate::organization::SplitAnalysisMethod::CrossDomain => SplitAnalysisMethod::CrossDomain,
        crate::organization::SplitAnalysisMethod::MethodBased => SplitAnalysisMethod::MethodBased,
        crate::organization::SplitAnalysisMethod::Hybrid => SplitAnalysisMethod::Hybrid,
        crate::organization::SplitAnalysisMethod::TypeBased => SplitAnalysisMethod::TypeBased,
    }
}

fn convert_from_org_severity(
    severity: crate::organization::RecommendationSeverity,
) -> RecommendationSeverity {
    match severity {
        crate::organization::RecommendationSeverity::Low => RecommendationSeverity::Low,
        crate::organization::RecommendationSeverity::Medium => RecommendationSeverity::Medium,
        crate::organization::RecommendationSeverity::High => RecommendationSeverity::High,
        crate::organization::RecommendationSeverity::Critical => RecommendationSeverity::Critical,
    }
}

// Extension to support legacy JSON format that only has metrics
impl FileDebtItem {
    /// Create a FileDebtItem from metrics with optional file context adjustments.
    ///
    /// # Arguments
    ///
    /// * `metrics` - File debt metrics containing raw calculations
    /// * `context` - Optional file context for score adjustments
    ///
    /// # Context Adjustments
    ///
    /// - Test files (confidence >0.8): 80% reduction
    /// - Probable test files (0.5-0.8): 40% reduction
    /// - Generated files: 90% reduction
    /// - Production files: No adjustment
    ///
    /// # Example
    ///
    /// ```
    /// use debtmap::priority::{FileDebtItem, FileDebtMetrics};
    /// use debtmap::analysis::FileContext;
    /// use std::path::PathBuf;
    ///
    /// # let metrics = FileDebtMetrics::default();
    /// # let context = FileContext::Production;
    /// let item = FileDebtItem::from_metrics(metrics, Some(&context));
    /// // item.score now includes context adjustment
    /// ```
    pub fn from_metrics(
        metrics: FileDebtMetrics,
        context: Option<&crate::analysis::FileContext>,
    ) -> Self {
        use crate::priority::scoring::file_context_scoring::apply_context_adjustments;

        let base_score = metrics.calculate_score();

        // Apply context-aware adjustments
        let score = if let Some(ctx) = context {
            apply_context_adjustments(base_score, ctx)
        } else {
            base_score
        };

        let recommendation = metrics.generate_recommendation();
        let impact = FileImpact {
            complexity_reduction: metrics.avg_complexity * metrics.function_count as f64 * 0.2,
            maintainability_improvement: (metrics.max_complexity as f64 - metrics.avg_complexity)
                * 10.0,
            test_effort: metrics.uncovered_lines as f64 * 0.1,
        };

        FileDebtItem {
            metrics,
            score,
            priority_rank: 0,
            recommendation,
            impact,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_score_basic() {
        let metrics = FileDebtMetrics {
            path: PathBuf::from("test.rs"),
            total_lines: 100,
            function_count: 5,
            class_count: 1,
            avg_complexity: 5.0,
            max_complexity: 10,
            total_complexity: 25,
            coverage_percent: 0.8,
            uncovered_lines: 20,
            god_object_analysis: None,
            function_scores: vec![1.0, 2.0, 3.0],
            god_object_type: None,
            file_type: None,
            ..Default::default()
        };

        let score = metrics.calculate_score();
        assert!(score > 0.0);
        assert!(score < 10.0);
    }

    #[test]
    fn test_calculate_score_with_god_object() {
        use crate::organization::{DetectionType, GodObjectAnalysis, GodObjectConfidence};

        let metrics = FileDebtMetrics {
            path: PathBuf::from("god.rs"),
            total_lines: 1000,
            function_count: 60,
            class_count: 1,
            avg_complexity: 15.0,
            max_complexity: 50,
            total_complexity: 900,
            coverage_percent: 0.3,
            uncovered_lines: 700,
            god_object_analysis: Some(GodObjectAnalysis {
                weighted_method_count: None,
                is_god_object: true,
                method_count: 60,
                field_count: 30,
                responsibility_count: 10,
                lines_of_code: 1000,
                complexity_sum: 900,
                god_object_score: Score0To100::new(0.8),
                recommended_splits: Vec::new(),
                confidence: GodObjectConfidence::Definite,
                responsibilities: Vec::new(),
                responsibility_method_counts: Default::default(),
                purity_distribution: None,
                module_structure: None,
                detection_type: DetectionType::GodClass,
                struct_name: None,
                struct_line: None,
                struct_location: None,
                visibility_breakdown: None,
                domain_count: 0,
                domain_diversity: 0.0,
                struct_ratio: 0.0,
                analysis_method: Default::default(),
                cross_domain_severity: None,
                domain_diversity_metrics: None,
                aggregated_entropy: None,
                aggregated_error_swallowing_count: None,
                aggregated_error_swallowing_patterns: None,
                layering_impact: None,
                anti_pattern_report: None,
                complexity_metrics: None,   // Spec 211
                trait_method_summary: None, // Spec 217
            }),
            function_scores: vec![5.0; 60],
            god_object_type: None,
            file_type: None,
            ..Default::default()
        };

        let score = metrics.calculate_score();
        assert!(score > 50.0, "God object should have high score");
    }

    #[test]
    fn test_calculate_score_low_coverage() {
        let metrics = FileDebtMetrics {
            path: PathBuf::from("untested.rs"),
            total_lines: 200,
            function_count: 10,
            class_count: 2,
            avg_complexity: 3.0,
            max_complexity: 5,
            total_complexity: 30,
            coverage_percent: 0.1,
            uncovered_lines: 180,
            god_object_analysis: None,
            function_scores: vec![1.0; 10],
            god_object_type: None,
            file_type: None,
            ..Default::default()
        };

        let score = metrics.calculate_score();
        let base_metrics = FileDebtMetrics {
            coverage_percent: 0.9,
            uncovered_lines: 20,
            ..metrics
        };
        let base_score = base_metrics.calculate_score();

        assert!(score > base_score, "Low coverage should increase score");
    }

    #[test]
    fn test_calculate_score_high_complexity() {
        let metrics = FileDebtMetrics {
            path: PathBuf::from("complex.rs"),
            total_lines: 300,
            function_count: 15,
            class_count: 1,
            avg_complexity: 20.0,
            max_complexity: 40,
            total_complexity: 300,
            coverage_percent: 0.7,
            uncovered_lines: 90,
            god_object_analysis: None,
            function_scores: vec![3.0; 15],
            god_object_type: None,
            file_type: None,
            ..Default::default()
        };

        let score = metrics.calculate_score();
        assert!(score > 10.0, "High complexity should produce high score");
    }

    #[test]
    fn test_calculate_score_many_functions() {
        let metrics = FileDebtMetrics {
            path: PathBuf::from("dense.rs"),
            total_lines: 500,
            function_count: 75,
            class_count: 1,
            avg_complexity: 4.0,
            max_complexity: 8,
            total_complexity: 300,
            coverage_percent: 0.6,
            uncovered_lines: 200,
            god_object_analysis: None,
            function_scores: vec![2.0; 75],
            god_object_type: None,
            file_type: None,
            ..Default::default()
        };

        let score = metrics.calculate_score();
        assert!(score > 15.0, "Dense files should have higher scores");
    }

    #[test]
    fn test_generate_recommendation_god_object() {
        use crate::organization::{DetectionType, GodObjectAnalysis, GodObjectConfidence};

        let metrics = FileDebtMetrics {
            god_object_analysis: Some(GodObjectAnalysis {
                weighted_method_count: None,
                is_god_object: true,
                method_count: 50,
                field_count: 10,
                responsibility_count: 5,
                lines_of_code: 500,
                complexity_sum: 200,
                god_object_score: Score0To100::new(0.8),
                confidence: GodObjectConfidence::Definite,
                detection_type: DetectionType::GodClass,
                struct_name: None,
                struct_line: None,
                struct_location: None,
                responsibilities: Vec::new(),
                responsibility_method_counts: Default::default(),
                recommended_splits: Vec::new(),
                purity_distribution: None,
                module_structure: None,
                visibility_breakdown: None,
                domain_count: 0,
                domain_diversity: 0.0,
                struct_ratio: 0.0,
                analysis_method: Default::default(),
                cross_domain_severity: None,
                domain_diversity_metrics: None,
                aggregated_entropy: None,
                aggregated_error_swallowing_count: None,
                aggregated_error_swallowing_patterns: None,
                layering_impact: None,
                anti_pattern_report: None,
                complexity_metrics: None,   // Spec 211
                trait_method_summary: None, // Spec 217
            }),
            function_count: 50,
            ..Default::default()
        };

        let rec = metrics.generate_recommendation();
        // With the new format, the generic case says "Split by data flow"
        assert!(rec.contains("Split by data flow") || rec.contains("Split"));
        assert!(rec.contains("modules") || rec.contains("functions"));
    }

    #[test]
    fn test_generate_recommendation_large_file() {
        let metrics = FileDebtMetrics {
            total_lines: 800,
            ..Default::default()
        };

        let rec = metrics.generate_recommendation();
        assert!(rec.contains("Extract complex functions"));
        assert!(rec.contains("800 lines"));
    }

    #[test]
    fn test_generate_recommendation_high_complexity() {
        let metrics = FileDebtMetrics {
            avg_complexity: 15.0,
            total_lines: 200,
            ..Default::default()
        };

        let rec = metrics.generate_recommendation();
        assert!(rec.contains("Simplify complex functions"));
    }

    #[test]
    fn test_generate_recommendation_low_coverage() {
        let metrics = FileDebtMetrics {
            coverage_percent: 0.25,
            total_lines: 200,
            avg_complexity: 3.0,
            ..Default::default()
        };

        let rec = metrics.generate_recommendation();
        assert!(rec.contains("Increase test coverage"));
        assert!(rec.contains("25.0%"));
    }

    #[test]
    fn test_generate_recommendation_general() {
        let metrics = FileDebtMetrics {
            total_lines: 100,
            avg_complexity: 3.0,
            coverage_percent: 0.85,
            ..Default::default()
        };

        let rec = metrics.generate_recommendation();
        assert!(rec.contains("Refactor for better maintainability"));
    }

    #[test]
    fn test_default_file_debt_metrics() {
        let metrics = FileDebtMetrics::default();
        assert_eq!(metrics.total_lines, 0);
        assert_eq!(metrics.function_count, 0);
        assert_eq!(metrics.avg_complexity, 0.0);
        assert_eq!(metrics.coverage_percent, 0.0);
        assert!(metrics.god_object_analysis.is_none());
    }

    #[test]
    fn test_score_factors_multiplication() {
        use crate::organization::{DetectionType, GodObjectAnalysis, GodObjectConfidence};

        let mut metrics = FileDebtMetrics {
            path: PathBuf::from("test.rs"),
            total_lines: 400,
            function_count: 20,
            avg_complexity: 10.0,
            total_complexity: 200,
            coverage_percent: 0.5,
            ..Default::default()
        };

        let score1 = metrics.calculate_score();

        metrics.god_object_analysis = Some(GodObjectAnalysis {
            weighted_method_count: None,
            is_god_object: true,
            method_count: 20,
            field_count: 10,
            responsibility_count: 5,
            lines_of_code: 400,
            complexity_sum: 200,
            god_object_score: Score0To100::new(100.0), // Severe god object for testing multiplication
            confidence: GodObjectConfidence::Definite,
            detection_type: DetectionType::GodClass,
            struct_name: None,
            struct_line: None,
            struct_location: None,
            responsibilities: Vec::new(),
            responsibility_method_counts: Default::default(),
            recommended_splits: Vec::new(),
            purity_distribution: None,
            module_structure: None,
            visibility_breakdown: None,
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: Default::default(),
            cross_domain_severity: None,
            domain_diversity_metrics: None,
            aggregated_entropy: None,
            aggregated_error_swallowing_count: None,
            aggregated_error_swallowing_patterns: None,
            layering_impact: None,
            anti_pattern_report: None,
            complexity_metrics: None,   // Spec 211
            trait_method_summary: None, // Spec 217
        });
        let score2 = metrics.calculate_score();

        assert!(
            score2 > score1 * 2.0,
            "God object should multiply score significantly"
        );
    }

    #[test]
    fn test_function_scores_aggregation() {
        let metrics = FileDebtMetrics {
            path: PathBuf::from("test.rs"),
            total_lines: 100,
            function_count: 3,
            avg_complexity: 2.0,
            total_complexity: 6,
            coverage_percent: 0.5,
            function_scores: vec![10.0, 20.0, 30.0],
            god_object_type: None,
            file_type: None,
            ..Default::default()
        };

        let score = metrics.calculate_score();
        assert!(score > 0.0);

        let metrics_no_functions = FileDebtMetrics {
            path: PathBuf::from("test.rs"),
            total_lines: 100,
            function_count: 3,
            avg_complexity: 2.0,
            total_complexity: 6,
            coverage_percent: 0.5,
            function_scores: vec![],
            god_object_type: None,
            file_type: None,
            ..Default::default()
        };

        let score_no_functions = metrics_no_functions.calculate_score();
        assert!(
            score > score_no_functions,
            "Function scores should increase total score"
        );
    }

    #[test]
    fn test_boilerplate_recommendation_used() {
        use crate::organization::boilerplate_detector::BoilerplatePattern;
        use crate::organization::{
            DetectionType, GodObjectAnalysis, GodObjectConfidence, GodObjectType,
        };

        let boilerplate_type = GodObjectType::BoilerplatePattern {
            pattern: BoilerplatePattern::TraitImplementation {
                trait_name: "Flag".to_string(),
                impl_count: 104,
                shared_methods: vec!["name_long".to_string()],
                method_uniformity: 1.0,
            },
            confidence: 0.878,
            recommendation: "BOILERPLATE DETECTED: Create declarative macro to generate Flag implementations. This is NOT a god object requiring module splitting.".to_string(),
        };

        let metrics = FileDebtMetrics {
            god_object_analysis: Some(GodObjectAnalysis {
                weighted_method_count: None,
                is_god_object: true,
                method_count: 888,
                field_count: 0,
                responsibility_count: 1,
                lines_of_code: 7775,
                complexity_sum: 888,
                god_object_score: Score0To100::new(0.878),
                confidence: GodObjectConfidence::Definite,
                detection_type: DetectionType::GodClass,
                struct_name: None,
                struct_line: None,
                struct_location: None,
                responsibilities: Vec::new(),
                responsibility_method_counts: Default::default(),
                recommended_splits: Vec::new(),
                purity_distribution: None,
                module_structure: None,
                visibility_breakdown: None,
                domain_count: 0,
                domain_diversity: 0.0,
                struct_ratio: 0.0,
                analysis_method: crate::organization::SplitAnalysisMethod::None,
                cross_domain_severity: None,
                domain_diversity_metrics: None,
                aggregated_entropy: None,
                aggregated_error_swallowing_count: None,
                aggregated_error_swallowing_patterns: None,
                layering_impact: None,
                anti_pattern_report: None,
                complexity_metrics: None,   // Spec 211
                trait_method_summary: None, // Spec 217
            }),
            god_object_type: Some(boilerplate_type),
            total_lines: 7775,
            function_count: 888,
            ..Default::default()
        };

        let recommendation = metrics.generate_recommendation();

        assert!(recommendation.contains("BOILERPLATE DETECTED"));
        assert!(recommendation.contains("declarative macro"));
        assert!(recommendation.contains("NOT a god object requiring module splitting"));
    }

    #[test]
    fn test_regular_god_object_still_gets_splitting_advice() {
        use crate::organization::{
            DetectionType, GodObjectAnalysis, GodObjectConfidence, GodObjectType,
        };

        let god_file_type = GodObjectType::GodModule {
            total_structs: 20,
            total_methods: 100,
            largest_struct: crate::organization::StructMetrics {
                name: "Config".to_string(),
                method_count: 50,
                field_count: 30,
                responsibilities: vec!["data_access".to_string()],
                line_span: (0, 1000),
            },
            suggested_splits: vec![],
        };

        let metrics = FileDebtMetrics {
            god_object_analysis: Some(GodObjectAnalysis {
                weighted_method_count: None,
                is_god_object: true,
                method_count: 100,
                field_count: 30,
                responsibility_count: 5,
                lines_of_code: 2000,
                complexity_sum: 500,
                god_object_score: Score0To100::new(0.8),
                confidence: GodObjectConfidence::Definite,
                detection_type: DetectionType::GodClass,
                struct_name: None,
                struct_line: None,
                struct_location: None,
                responsibilities: Vec::new(),
                responsibility_method_counts: Default::default(),
                recommended_splits: Vec::new(),
                purity_distribution: None,
                module_structure: None,
                visibility_breakdown: None,
                domain_count: 0,
                domain_diversity: 0.0,
                struct_ratio: 0.0,
                analysis_method: crate::organization::SplitAnalysisMethod::None,
                cross_domain_severity: None,
                domain_diversity_metrics: None,
                aggregated_entropy: None,
                aggregated_error_swallowing_count: None,
                aggregated_error_swallowing_patterns: None,
                layering_impact: None,
                anti_pattern_report: None,
                complexity_metrics: None,   // Spec 211
                trait_method_summary: None, // Spec 217
            }),
            god_object_type: Some(god_file_type),
            total_lines: 2000,
            function_count: 100,
            ..Default::default()
        };

        let recommendation = metrics.generate_recommendation();

        assert!(recommendation.contains("Split") || recommendation.contains("URGENT"));
        assert!(!recommendation.contains("BOILERPLATE"));
        assert!(!recommendation.contains("macro"));
    }

    #[test]
    fn test_boilerplate_takes_precedence_over_god_object() {
        use crate::organization::boilerplate_detector::BoilerplatePattern;
        use crate::organization::{
            DetectionType, GodObjectAnalysis, GodObjectConfidence, GodObjectType,
        };

        let boilerplate_type = GodObjectType::BoilerplatePattern {
            pattern: BoilerplatePattern::TestBoilerplate {
                test_count: 50,
                shared_structure: "similar test structure".to_string(),
            },
            confidence: 0.92,
            recommendation: "Use a macro to generate these test functions".to_string(),
        };

        let metrics = FileDebtMetrics {
            god_object_analysis: Some(GodObjectAnalysis {
                weighted_method_count: None,
                is_god_object: true,
                method_count: 200,
                field_count: 100,
                responsibility_count: 10,
                lines_of_code: 5000,
                complexity_sum: 1000,
                god_object_score: Score0To100::new(0.95),
                confidence: GodObjectConfidence::Definite,
                detection_type: DetectionType::GodClass,
                struct_name: None,
                struct_line: None,
                struct_location: None,
                responsibilities: Vec::new(),
                responsibility_method_counts: Default::default(),
                recommended_splits: Vec::new(),
                purity_distribution: None,
                module_structure: None,
                visibility_breakdown: None,
                domain_count: 0,
                domain_diversity: 0.0,
                struct_ratio: 0.0,
                analysis_method: crate::organization::SplitAnalysisMethod::None,
                cross_domain_severity: None,
                domain_diversity_metrics: None,
                aggregated_entropy: None,
                aggregated_error_swallowing_count: None,
                aggregated_error_swallowing_patterns: None,
                layering_impact: None,
                anti_pattern_report: None,
                complexity_metrics: None,   // Spec 211
                trait_method_summary: None, // Spec 217
            }),
            god_object_type: Some(boilerplate_type),
            total_lines: 5000,
            function_count: 200,
            ..Default::default()
        };

        let recommendation = metrics.generate_recommendation();

        // Should use boilerplate recommendation, not god object recommendation
        assert!(recommendation.contains("macro"));
        assert!(!recommendation.contains("Split"));
        assert!(!recommendation.contains("URGENT"));
    }

    #[test]
    fn test_get_score_factors_extraction() {
        use crate::organization::{DetectionType, GodObjectAnalysis, GodObjectConfidence};

        let metrics = FileDebtMetrics {
            path: PathBuf::from("test.rs"),
            total_lines: 354,
            function_count: 7,
            avg_complexity: 8.0,
            total_complexity: 56,
            coverage_percent: 0.0,
            god_object_analysis: Some(GodObjectAnalysis {
                weighted_method_count: None,
                is_god_object: true,
                method_count: 60,
                field_count: 30,
                responsibility_count: 10,
                lines_of_code: 354,
                complexity_sum: 56,
                god_object_score: Score0To100::new(7.0),
                confidence: GodObjectConfidence::Definite,
                detection_type: DetectionType::GodClass,
                struct_name: None,
                struct_line: None,
                struct_location: None,
                responsibilities: Vec::new(),
                responsibility_method_counts: Default::default(),
                recommended_splits: Vec::new(),
                purity_distribution: None,
                module_structure: None,
                visibility_breakdown: None,
                domain_count: 0,
                domain_diversity: 0.0,
                struct_ratio: 0.0,
                analysis_method: crate::organization::SplitAnalysisMethod::None,
                cross_domain_severity: None,
                domain_diversity_metrics: None,
                aggregated_entropy: None,
                aggregated_error_swallowing_count: None,
                aggregated_error_swallowing_patterns: None,
                layering_impact: None,
                anti_pattern_report: None,
                complexity_metrics: None,   // Spec 211
                trait_method_summary: None, // Spec 217
            }),
            function_scores: vec![1.5, 1.5, 1.5, 1.5, 1.5, 1.5, 1.5],
            god_object_type: None,
            file_type: None,
            ..Default::default()
        };

        let factors = metrics.get_score_factors();

        assert!((factors.size_factor - 1.88).abs() < 0.01);
        assert_eq!(factors.coverage_factor, 3.0);
        assert!((factors.god_object_multiplier - 1.14).abs() < 0.01); // 1.0 + (7.0 / 50.0)
        assert_eq!(factors.density_factor, 1.0);
        assert_eq!(factors.function_count, 7);
        assert!(factors.is_god_object);
    }

    #[test]
    fn test_score_calculation_matches_factors() {
        use crate::organization::{DetectionType, GodObjectAnalysis, GodObjectConfidence};

        let metrics = FileDebtMetrics {
            path: PathBuf::from("test.rs"),
            total_lines: 400,
            function_count: 20,
            avg_complexity: 10.0,
            total_complexity: 200,
            coverage_percent: 0.5,
            god_object_analysis: Some(GodObjectAnalysis {
                weighted_method_count: None,
                is_god_object: true,
                method_count: 40,
                field_count: 20,
                responsibility_count: 5,
                lines_of_code: 400,
                complexity_sum: 200,
                god_object_score: Score0To100::new(3.0),
                confidence: GodObjectConfidence::Definite,
                detection_type: DetectionType::GodClass,
                struct_name: None,
                struct_line: None,
                struct_location: None,
                responsibilities: Vec::new(),
                responsibility_method_counts: Default::default(),
                recommended_splits: Vec::new(),
                purity_distribution: None,
                module_structure: None,
                visibility_breakdown: None,
                domain_count: 0,
                domain_diversity: 0.0,
                struct_ratio: 0.0,
                analysis_method: crate::organization::SplitAnalysisMethod::None,
                cross_domain_severity: None,
                domain_diversity_metrics: None,
                aggregated_entropy: None,
                aggregated_error_swallowing_count: None,
                aggregated_error_swallowing_patterns: None,
                layering_impact: None,
                anti_pattern_report: None,
                complexity_metrics: None,   // Spec 211
                trait_method_summary: None, // Spec 217
            }),
            function_scores: vec![5.0; 20],
            god_object_type: None,
            file_type: None,
            ..Default::default()
        };

        let factors = metrics.get_score_factors();
        let actual_score = metrics.calculate_score();

        let calculated = factors.size_factor
            * factors.complexity_factor
            * factors.coverage_factor
            * factors.density_factor
            * factors.god_object_multiplier
            * factors.function_factor;

        assert!((calculated - actual_score).abs() < 0.5);
    }

    #[test]
    fn test_factors_coverage_details() {
        let metrics = FileDebtMetrics {
            path: PathBuf::from("test.rs"),
            total_lines: 200,
            function_count: 10,
            coverage_percent: 0.75,
            god_object_analysis: None,
            function_scores: vec![],
            god_object_type: None,
            file_type: None,
            ..Default::default()
        };

        let factors = metrics.get_score_factors();

        assert_eq!(factors.coverage_percent, 0.75);
        assert_eq!(factors.coverage_gap, 0.25);
        assert_eq!(factors.coverage_factor, 1.5); // (0.25 * 2.0) + 1.0
    }

    #[test]
    fn test_factors_density_threshold() {
        let mut metrics = FileDebtMetrics {
            path: PathBuf::from("test.rs"),
            total_lines: 200,
            function_count: 45,
            god_object_analysis: None,
            function_scores: vec![],
            god_object_type: None,
            file_type: None,
            ..Default::default()
        };

        let factors = metrics.get_score_factors();
        assert_eq!(factors.density_factor, 1.0); // Below threshold

        metrics.function_count = 60;
        let factors = metrics.get_score_factors();
        assert_eq!(factors.density_factor, 1.2); // 1.0 + (60 - 50) * 0.02
    }

    #[test]
    fn test_factors_god_object_multiplier() {
        use crate::organization::{DetectionType, GodObjectAnalysis, GodObjectConfidence};

        let mut metrics = FileDebtMetrics {
            path: PathBuf::from("test.rs"),
            total_lines: 200,
            function_count: 10,
            god_object_analysis: Some(GodObjectAnalysis {
                weighted_method_count: None,
                is_god_object: false,
                method_count: 10,
                field_count: 5,
                responsibility_count: 2,
                lines_of_code: 200,
                complexity_sum: 50,
                god_object_score: Score0To100::new(0.0),
                confidence: GodObjectConfidence::Definite,
                detection_type: DetectionType::GodClass,
                struct_name: None,
                struct_line: None,
                struct_location: None,
                responsibilities: Vec::new(),
                responsibility_method_counts: Default::default(),
                recommended_splits: Vec::new(),
                purity_distribution: None,
                module_structure: None,
                visibility_breakdown: None,
                domain_count: 0,
                domain_diversity: 0.0,
                struct_ratio: 0.0,
                analysis_method: Default::default(),
                cross_domain_severity: None,
                domain_diversity_metrics: None,
                aggregated_entropy: None,
                aggregated_error_swallowing_count: None,
                aggregated_error_swallowing_patterns: None,
                layering_impact: None,
                anti_pattern_report: None,
                complexity_metrics: None,   // Spec 211
                trait_method_summary: None, // Spec 217
            }),
            function_scores: vec![],
            god_object_type: None,
            file_type: None,
            ..Default::default()
        };

        let factors = metrics.get_score_factors();
        assert_eq!(factors.god_object_multiplier, 1.0); // Not flagged

        metrics.god_object_analysis = Some(GodObjectAnalysis {
            weighted_method_count: None,
            is_god_object: true,
            method_count: 10,
            field_count: 5,
            responsibility_count: 2,
            lines_of_code: 200,
            complexity_sum: 50,
            god_object_score: Score0To100::new(8.5),
            confidence: GodObjectConfidence::Definite,
            detection_type: DetectionType::GodClass,
            struct_name: None,
            struct_line: None,
            struct_location: None,
            responsibilities: Vec::new(),
            responsibility_method_counts: Default::default(),
            recommended_splits: Vec::new(),
            purity_distribution: None,
            module_structure: None,
            visibility_breakdown: None,
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: Default::default(),
            cross_domain_severity: None,
            domain_diversity_metrics: None,
            aggregated_entropy: None,
            aggregated_error_swallowing_count: None,
            aggregated_error_swallowing_patterns: None,
            layering_impact: None,
            anti_pattern_report: None,
            complexity_metrics: None,   // Spec 211
            trait_method_summary: None, // Spec 217
        });
        let factors = metrics.get_score_factors();
        assert!((factors.god_object_multiplier - 1.17).abs() < 0.01); // 1.0 + (8.5 / 50.0)
    }

    // Tests for spec 168: File context-aware scoring

    fn create_test_metrics() -> FileDebtMetrics {
        FileDebtMetrics {
            path: PathBuf::from("test.rs"),
            total_lines: 354,
            function_count: 7,
            class_count: 0,
            avg_complexity: 12.3,
            max_complexity: 45,
            total_complexity: 86,
            coverage_percent: 0.0,
            uncovered_lines: 354,
            god_object_analysis: None,
            function_scores: vec![],
            god_object_type: None,
            file_type: None,
            ..Default::default()
        }
    }

    #[test]
    fn test_file_item_with_test_context_reduces_score() {
        use crate::analysis::FileContext;

        let metrics = create_test_metrics();
        let base_score = metrics.calculate_score();

        let test_context = FileContext::Test {
            confidence: 0.95,
            test_framework: Some("rust-std".to_string()),
            test_count: 7,
        };

        let item = FileDebtItem::from_metrics(metrics, Some(&test_context));

        // Should be reduced by 80% (multiplied by 0.2)
        assert!(item.score < base_score * 0.25);
        assert!(item.score > base_score * 0.15);
        assert!(item.score < 20.0, "Test file score should be low");
    }

    #[test]
    fn test_file_item_without_context_unchanged() {
        let metrics = create_test_metrics();
        let base_score = metrics.calculate_score();

        let item = FileDebtItem::from_metrics(metrics, None);

        assert_eq!(item.score, base_score);
    }

    #[test]
    fn test_file_item_with_production_context_unchanged() {
        use crate::analysis::FileContext;

        let metrics = create_test_metrics();
        let base_score = metrics.calculate_score();

        let prod_context = FileContext::Production;
        let item = FileDebtItem::from_metrics(metrics, Some(&prod_context));

        assert_eq!(item.score, base_score);
    }

    #[test]
    fn test_file_item_with_generated_context_reduces_90_percent() {
        use crate::analysis::FileContext;

        let metrics = create_test_metrics();
        let base_score = metrics.calculate_score();

        let gen_context = FileContext::Generated {
            generator: "protoc".to_string(),
        };
        let item = FileDebtItem::from_metrics(metrics, Some(&gen_context));

        // Should be reduced by 90% (multiplied by 0.1)
        assert!((item.score - base_score * 0.1).abs() < 0.5);
    }

    #[test]
    fn test_file_item_with_probable_test_context_40_percent_reduction() {
        use crate::analysis::FileContext;

        let metrics = create_test_metrics();
        let base_score = metrics.calculate_score();

        let probable_test_context = FileContext::Test {
            confidence: 0.65, // Probable test file (0.5-0.8)
            test_framework: None,
            test_count: 5,
        };
        let item = FileDebtItem::from_metrics(metrics, Some(&probable_test_context));

        // Should be reduced by 40% (multiplied by 0.6)
        assert!((item.score - base_score * 0.6).abs() < 0.5);
    }

    // Tests for spec 201: File-level coupling in recommendations

    #[test]
    fn test_coupling_warning_for_highly_coupled_files() {
        let metrics = FileDebtMetrics {
            path: PathBuf::from("highly_coupled.rs"),
            total_lines: 200,
            afferent_coupling: 10,
            efferent_coupling: 8,
            instability: 0.44,
            ..Default::default()
        };

        let rec = metrics.generate_recommendation();
        assert!(
            rec.contains("COUPLING WARNING"),
            "Should contain coupling warning for Ca+Ce>15"
        );
        assert!(rec.contains("Ca=10"), "Should show afferent coupling");
        assert!(rec.contains("Ce=8"), "Should show efferent coupling");
        assert!(rec.contains("total=18"), "Should show total coupling");
    }

    #[test]
    fn test_no_coupling_warning_for_normal_files() {
        let metrics = FileDebtMetrics {
            path: PathBuf::from("normal.rs"),
            total_lines: 200,
            afferent_coupling: 5,
            efferent_coupling: 5,
            instability: 0.5,
            ..Default::default()
        };

        let rec = metrics.generate_recommendation();
        assert!(
            !rec.contains("COUPLING WARNING"),
            "Should not contain coupling warning for Ca+Ce<=15"
        );
    }

    #[test]
    fn test_instability_context_for_unstable_files() {
        let metrics = FileDebtMetrics {
            path: PathBuf::from("unstable.rs"),
            total_lines: 200,
            afferent_coupling: 1,
            efferent_coupling: 10,
            instability: 0.91,
            ..Default::default()
        };

        let rec = metrics.generate_recommendation();
        assert!(
            rec.contains("UNSTABLE"),
            "Should contain instability context for I>0.9"
        );
        assert!(
            rec.contains("safe to refactor"),
            "Should indicate safe to refactor"
        );
    }

    #[test]
    fn test_instability_context_for_stable_files() {
        let metrics = FileDebtMetrics {
            path: PathBuf::from("stable.rs"),
            total_lines: 200,
            afferent_coupling: 10,
            efferent_coupling: 1,
            instability: 0.09,
            ..Default::default()
        };

        let rec = metrics.generate_recommendation();
        assert!(
            rec.contains("STABLE"),
            "Should contain stability context for I<0.1"
        );
        assert!(
            rec.contains("careful review"),
            "Should indicate need for careful review"
        );
    }

    #[test]
    fn test_no_instability_context_for_normal_instability() {
        let metrics = FileDebtMetrics {
            path: PathBuf::from("normal.rs"),
            total_lines: 200,
            afferent_coupling: 5,
            efferent_coupling: 5,
            instability: 0.5,
            ..Default::default()
        };

        let rec = metrics.generate_recommendation();
        assert!(
            !rec.contains("UNSTABLE"),
            "Should not contain unstable context"
        );
        assert!(
            !rec.contains("STABLE:"),
            "Should not contain stable context"
        );
    }

    #[test]
    fn test_no_instability_context_when_no_coupling() {
        let metrics = FileDebtMetrics {
            path: PathBuf::from("isolated.rs"),
            total_lines: 200,
            afferent_coupling: 0,
            efferent_coupling: 0,
            instability: 0.0,
            ..Default::default()
        };

        let rec = metrics.generate_recommendation();
        // Should not have any coupling or instability context
        assert!(
            !rec.contains("COUPLING"),
            "Should not contain coupling context"
        );
        assert!(
            !rec.contains("STABLE"),
            "Should not contain stability context"
        );
        assert!(
            !rec.contains("UNSTABLE"),
            "Should not contain instability context"
        );
    }

    #[test]
    fn test_coupling_and_instability_can_combine() {
        let metrics = FileDebtMetrics {
            path: PathBuf::from("highly_coupled_unstable.rs"),
            total_lines: 200,
            afferent_coupling: 2,
            efferent_coupling: 18,
            instability: 0.9,
            ..Default::default()
        };

        let rec = metrics.generate_recommendation();
        // Both warnings should appear when both conditions are met
        assert!(
            rec.contains("COUPLING WARNING"),
            "Should contain coupling warning"
        );
        // Note: instability 0.9 is not > 0.9, so no instability context here
    }
}
