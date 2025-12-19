//! Integrated Architecture Analysis
//!
//! Orchestrates all architecture analysis specs (181-184) to produce coherent,
//! non-conflicting recommendations with clear prioritization and performance budgets.
//!
//! This module unifies:
//! - Type-based clustering (Spec 181)
//! - Data flow analysis (Spec 182)
//! - Anti-pattern detection (Spec 183)
//! - Hidden type extraction (Spec 184)

use crate::organization::{
    anti_pattern_detector::{AntiPattern, AntiPatternDetector, AntiPatternSeverity},
    data_flow_analyzer::DataFlowAnalyzer,
    hidden_type_extractor::{HiddenType, HiddenTypeExtractor},
    type_based_clustering::{MethodSignature, TypeAffinityAnalyzer},
    GodObjectAnalysis, ModuleSplit, SplitAnalysisMethod,
};

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Orchestrates all architecture analysis specs (181-184)
pub struct IntegratedArchitectureAnalyzer {
    config: AnalysisConfig,
}

/// Configuration for integrated analysis
#[derive(Clone, Debug)]
pub struct AnalysisConfig {
    /// Performance budget for total analysis time (default: 500ms)
    pub max_analysis_time: Duration,

    /// Minimum god object score to trigger advanced analysis (default: 50.0)
    pub advanced_analysis_threshold: f64,

    /// Strategy for resolving conflicting recommendations
    pub conflict_resolution: ConflictResolutionStrategy,

    /// Enable/disable individual analyzers
    pub enabled_analyzers: EnabledAnalyzers,

    /// Quality threshold for accepting recommendations (default: 60.0)
    pub min_quality_score: f64,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            max_analysis_time: Duration::from_millis(500),
            advanced_analysis_threshold: 50.0,
            conflict_resolution: ConflictResolutionStrategy::Hybrid,
            enabled_analyzers: EnabledAnalyzers::all(),
            min_quality_score: 60.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EnabledAnalyzers {
    pub type_based: bool,
    pub data_flow: bool,
    pub anti_pattern: bool,
    pub hidden_types: bool,
}

impl EnabledAnalyzers {
    pub fn all() -> Self {
        Self {
            type_based: true,
            data_flow: true,
            anti_pattern: true,
            hidden_types: true,
        }
    }

    pub fn minimal() -> Self {
        Self {
            type_based: false,
            data_flow: false,
            anti_pattern: true, // Always run anti-pattern detection
            hidden_types: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConflictResolutionStrategy {
    /// Use type-based clustering exclusively
    TypeBased,

    /// Use data flow analysis exclusively
    DataFlow,

    /// Choose based on confidence scores
    BestConfidence,

    /// Merge both approaches (recommended)
    Hybrid,

    /// Present both to user for selection
    UserChoice,
}

impl IntegratedArchitectureAnalyzer {
    pub fn new() -> Self {
        Self {
            config: AnalysisConfig::default(),
        }
    }

    pub fn with_config(config: AnalysisConfig) -> Self {
        Self { config }
    }

    /// Run integrated analysis pipeline
    pub fn analyze(
        &self,
        god_object: &GodObjectAnalysis,
        ast: &syn::File,
        call_graph: &HashMap<String, Vec<String>>,
    ) -> Result<IntegratedAnalysisResult, AnalysisError> {
        let start_time = Instant::now();

        // Phase 1: Fast path - Anti-pattern detection (always run)
        let anti_pattern_result = if self.config.enabled_analyzers.anti_pattern {
            self.run_anti_pattern_detection(god_object, ast)?
        } else {
            None
        };

        // Phase 2: Advanced analysis (conditional on score threshold)
        let advanced_results =
            if god_object.god_object_score >= (self.config.advanced_analysis_threshold).max(0.0) {
                self.run_advanced_analysis(god_object, ast, call_graph, start_time)?
            } else {
                AdvancedAnalysisResults::empty()
            };

        // Phase 3: Conflict resolution
        let unified_splits = self.resolve_conflicts(
            advanced_results.type_based_splits,
            advanced_results.data_flow_splits,
        );

        // Phase 4: Quality validation
        let validated_splits = self.validate_quality(unified_splits, anti_pattern_result.as_ref());

        // Phase 5: Hidden type enrichment
        let enriched_splits = if self.config.enabled_analyzers.hidden_types {
            self.enrich_with_hidden_types(validated_splits, advanced_results.hidden_types.clone())
        } else {
            validated_splits
        };

        let elapsed = start_time.elapsed();

        Ok(IntegratedAnalysisResult {
            unified_splits: enriched_splits,
            anti_patterns: anti_pattern_result,
            hidden_types: advanced_results.hidden_types,
            analysis_metadata: AnalysisMetadata {
                total_time: elapsed,
                timeout_occurred: elapsed > self.config.max_analysis_time,
                strategy_used: self.config.conflict_resolution.clone(),
                analyzers_run: self.config.enabled_analyzers.clone(),
            },
        })
    }

    fn run_anti_pattern_detection(
        &self,
        god_object: &GodObjectAnalysis,
        ast: &syn::File,
    ) -> Result<Option<AntiPatternReport>, AnalysisError> {
        let detector = AntiPatternDetector::new();
        let signatures = extract_method_signatures(ast)?;

        // Convert to type_registry::MethodSignature for anti-pattern detection
        let registry_signatures: Vec<crate::analyzers::type_registry::MethodSignature> = signatures
            .iter()
            .map(convert_to_registry_signature)
            .collect();

        let quality_report =
            detector.calculate_split_quality(&god_object.recommended_splits, &registry_signatures);

        Ok(Some(AntiPatternReport {
            quality_score: quality_report.quality_score,
            anti_patterns: quality_report.anti_patterns,
        }))
    }

    fn run_advanced_analysis(
        &self,
        _god_object: &GodObjectAnalysis,
        ast: &syn::File,
        call_graph: &HashMap<String, Vec<String>>,
        start_time: Instant,
    ) -> Result<AdvancedAnalysisResults, AnalysisError> {
        // Check budget before each expensive operation
        let budget_check = || {
            if start_time.elapsed() > self.config.max_analysis_time {
                Err(AnalysisError::TimeoutExceeded)
            } else {
                Ok(())
            }
        };

        // Extract type signatures (shared by multiple analyzers)
        let signatures = extract_method_signatures(ast)?;
        budget_check()?;

        // Run type-based analysis
        let type_based_splits = if self.config.enabled_analyzers.type_based {
            budget_check()?;
            Some(self.run_type_based_analysis(&signatures)?)
        } else {
            None
        };

        // Run data flow analysis
        let data_flow_splits = if self.config.enabled_analyzers.data_flow {
            budget_check()?;
            Some(self.run_data_flow_analysis(&signatures, call_graph)?)
        } else {
            None
        };

        // Run hidden type extraction
        let hidden_types = if self.config.enabled_analyzers.hidden_types {
            budget_check()?;
            Some(self.run_hidden_type_extraction(&signatures, ast)?)
        } else {
            None
        };

        Ok(AdvancedAnalysisResults {
            type_based_splits,
            data_flow_splits,
            hidden_types,
        })
    }

    fn run_type_based_analysis(
        &self,
        signatures: &[MethodSignature],
    ) -> Result<Vec<ModuleSplit>, AnalysisError> {
        let affinity_analyzer = TypeAffinityAnalyzer;
        let clusters = affinity_analyzer.cluster_by_type_affinity(signatures);

        // Convert clusters to ModuleSplit
        Ok(clusters
            .into_iter()
            .map(|cluster| {
                // Sort HashSet collections for deterministic ordering
                let mut input_types: Vec<_> = cluster.input_types.into_iter().collect();
                input_types.sort();
                let mut output_types: Vec<_> = cluster.output_types.into_iter().collect();
                output_types.sort();

                ModuleSplit {
                    suggested_name: cluster.primary_type.name.clone(),
                    methods_to_move: cluster.methods,
                    responsibility: format!(
                        "Manage {} data and transformations",
                        cluster.primary_type.name
                    ),
                    method: SplitAnalysisMethod::TypeBased,
                    cohesion_score: Some(cluster.type_affinity_score),
                    input_types,
                    output_types,
                    core_type: Some(cluster.primary_type.name),
                    ..Default::default()
                }
            })
            .collect())
    }

    fn run_data_flow_analysis(
        &self,
        signatures: &[MethodSignature],
        call_graph: &HashMap<String, Vec<String>>,
    ) -> Result<Vec<ModuleSplit>, AnalysisError> {
        let flow_analyzer = DataFlowAnalyzer;
        let flow_graph = flow_analyzer.build_type_flow_graph(signatures, call_graph);
        let stages = flow_analyzer
            .detect_pipeline_stages(&flow_graph, signatures)
            .map_err(AnalysisError::DataFlowCycle)?;

        Ok(flow_analyzer.generate_pipeline_recommendations(&stages, ""))
    }

    fn run_hidden_type_extraction(
        &self,
        signatures: &[MethodSignature],
        ast: &syn::File,
    ) -> Result<Vec<HiddenType>, AnalysisError> {
        let extractor = HiddenTypeExtractor::new();
        Ok(extractor.extract_hidden_types(signatures, ast, ""))
    }

    fn resolve_conflicts(
        &self,
        type_based: Option<Vec<ModuleSplit>>,
        data_flow: Option<Vec<ModuleSplit>>,
    ) -> Vec<ModuleSplit> {
        use ConflictResolutionStrategy::*;

        match self.config.conflict_resolution {
            TypeBased => type_based.unwrap_or_default(),
            DataFlow => data_flow.unwrap_or_default(),

            BestConfidence => {
                // Choose approach with higher average cohesion
                let type_avg = type_based.as_ref().map(|s| avg_cohesion(s)).unwrap_or(0.0);
                let flow_avg = data_flow.as_ref().map(|s| avg_cohesion(s)).unwrap_or(0.0);

                if type_avg >= flow_avg {
                    type_based.unwrap_or_default()
                } else {
                    data_flow.unwrap_or_default()
                }
            }

            Hybrid => {
                // Merge non-overlapping splits
                self.merge_splits(type_based, data_flow)
            }

            UserChoice => {
                // Return both, marked for user selection
                let mut combined = type_based.unwrap_or_default();
                combined.extend(data_flow.unwrap_or_default());
                combined
            }
        }
    }

    fn merge_splits(
        &self,
        type_based: Option<Vec<ModuleSplit>>,
        data_flow: Option<Vec<ModuleSplit>>,
    ) -> Vec<ModuleSplit> {
        let mut merged = Vec::new();
        let type_splits = type_based.unwrap_or_default();
        let flow_splits = data_flow.unwrap_or_default();

        // Add type-based splits
        for split in type_splits {
            merged.push(split);
        }

        // Add non-overlapping flow splits
        for flow_split in flow_splits {
            let methods: HashSet<_> = flow_split.methods_to_move.iter().collect();
            let overlaps = merged.iter().any(|existing| {
                let existing_methods: HashSet<_> = existing.methods_to_move.iter().collect();
                methods.intersection(&existing_methods).count() > methods.len() / 2
            });

            if !overlaps {
                merged.push(flow_split);
            }
        }

        merged
    }

    fn validate_quality(
        &self,
        splits: Vec<ModuleSplit>,
        anti_pattern_report: Option<&AntiPatternReport>,
    ) -> Vec<ModuleSplit> {
        if let Some(report) = anti_pattern_report {
            if report.quality_score < self.config.min_quality_score {
                // Filter out splits with critical anti-patterns
                return splits
                    .into_iter()
                    .filter(|split| !has_critical_anti_pattern(split, &report.anti_patterns))
                    .collect();
            }
        }
        splits
    }

    fn enrich_with_hidden_types(
        &self,
        mut splits: Vec<ModuleSplit>,
        hidden_types: Option<Vec<HiddenType>>,
    ) -> Vec<ModuleSplit> {
        if let Some(types) = hidden_types {
            for split in &mut splits {
                // Find matching hidden type for this split
                if let Some(hidden_type) = types.iter().find(|t| {
                    split
                        .methods_to_move
                        .iter()
                        .any(|m| t.methods.iter().any(|tm| &tm.name == m))
                }) {
                    // Enrich split with hidden type information
                    split.suggested_type_definition = Some(hidden_type.example_definition.clone());
                    split.rationale = Some(format!(
                        "{}. Hidden type detected: {} (confidence: {:.2})",
                        split.rationale.as_deref().unwrap_or(""),
                        hidden_type.suggested_name,
                        hidden_type.confidence
                    ));
                }
            }
        }
        splits
    }
}

impl Default for IntegratedArchitectureAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct AdvancedAnalysisResults {
    type_based_splits: Option<Vec<ModuleSplit>>,
    data_flow_splits: Option<Vec<ModuleSplit>>,
    hidden_types: Option<Vec<HiddenType>>,
}

impl AdvancedAnalysisResults {
    fn empty() -> Self {
        Self {
            type_based_splits: None,
            data_flow_splits: None,
            hidden_types: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IntegratedAnalysisResult {
    pub unified_splits: Vec<ModuleSplit>,
    pub anti_patterns: Option<AntiPatternReport>,
    pub hidden_types: Option<Vec<HiddenType>>,
    pub analysis_metadata: AnalysisMetadata,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiPatternReport {
    pub quality_score: f64,
    pub anti_patterns: Vec<AntiPattern>,
}

#[derive(Debug, Clone)]
pub struct AnalysisMetadata {
    pub total_time: Duration,
    pub timeout_occurred: bool,
    pub strategy_used: ConflictResolutionStrategy,
    pub analyzers_run: EnabledAnalyzers,
}

#[derive(Debug, Clone)]
pub enum AnalysisError {
    TimeoutExceeded,
    AstParseError(String),
    DataFlowCycle(String),
}

// Helper functions
fn avg_cohesion(splits: &[ModuleSplit]) -> f64 {
    if splits.is_empty() {
        return 0.0;
    }
    let scores: Vec<f64> = splits.iter().filter_map(|s| s.cohesion_score).collect();
    if scores.is_empty() {
        return 0.0;
    }
    scores.iter().sum::<f64>() / scores.len() as f64
}

fn has_critical_anti_pattern(split: &ModuleSplit, patterns: &[AntiPattern]) -> bool {
    patterns
        .iter()
        .any(|p| p.severity == AntiPatternSeverity::Critical && p.location == split.suggested_name)
}

fn extract_method_signatures(ast: &syn::File) -> Result<Vec<MethodSignature>, AnalysisError> {
    use crate::organization::type_based_clustering::TypeSignatureAnalyzer;

    let analyzer = TypeSignatureAnalyzer;
    let mut signatures = Vec::new();

    // Extract from impl blocks
    for item in &ast.items {
        if let syn::Item::Impl(impl_block) = item {
            for item in &impl_block.items {
                if let syn::ImplItem::Fn(method) = item {
                    signatures.push(analyzer.analyze_method(method));
                }
            }
        }
    }

    // Extract standalone functions
    for item in &ast.items {
        if let syn::Item::Fn(func) = item {
            signatures.push(analyzer.analyze_function(func));
        }
    }

    Ok(signatures)
}

/// Convert type_based_clustering::MethodSignature to type_registry::MethodSignature
fn convert_to_registry_signature(
    sig: &MethodSignature,
) -> crate::analyzers::type_registry::MethodSignature {
    let self_param = sig
        .self_type
        .as_ref()
        .map(|t| crate::analyzers::type_registry::SelfParam {
            is_reference: t.is_reference,
            is_mutable: t.is_mutable,
        });

    let return_type = sig.return_type.as_ref().map(|t| t.name.clone());

    let param_types = sig.param_types.iter().map(|t| t.name.clone()).collect();

    crate::analyzers::type_registry::MethodSignature {
        name: sig.name.clone(),
        self_param,
        return_type,
        param_types,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enabled_analyzers_all() {
        let analyzers = EnabledAnalyzers::all();
        assert!(analyzers.type_based);
        assert!(analyzers.data_flow);
        assert!(analyzers.anti_pattern);
        assert!(analyzers.hidden_types);
    }

    #[test]
    fn test_enabled_analyzers_minimal() {
        let analyzers = EnabledAnalyzers::minimal();
        assert!(!analyzers.type_based);
        assert!(!analyzers.data_flow);
        assert!(analyzers.anti_pattern); // Always enabled
        assert!(!analyzers.hidden_types);
    }

    #[test]
    fn test_default_config() {
        let config = AnalysisConfig::default();
        assert_eq!(config.max_analysis_time, Duration::from_millis(500));
        assert_eq!(config.advanced_analysis_threshold, 50.0);
        assert_eq!(config.min_quality_score, 60.0);
        assert_eq!(
            config.conflict_resolution,
            ConflictResolutionStrategy::Hybrid
        );
    }

    #[test]
    fn test_avg_cohesion_empty() {
        let splits: Vec<ModuleSplit> = vec![];
        assert_eq!(avg_cohesion(&splits), 0.0);
    }

    #[test]
    fn test_avg_cohesion_with_scores() {
        let splits = vec![
            ModuleSplit {
                cohesion_score: Some(0.8),
                ..Default::default()
            },
            ModuleSplit {
                cohesion_score: Some(0.6),
                ..Default::default()
            },
            ModuleSplit {
                cohesion_score: None,
                ..Default::default()
            },
        ];
        assert_eq!(avg_cohesion(&splits), 0.7);
    }

    #[test]
    fn test_conflict_resolution_type_based() {
        let analyzer = IntegratedArchitectureAnalyzer::with_config(AnalysisConfig {
            conflict_resolution: ConflictResolutionStrategy::TypeBased,
            ..Default::default()
        });

        let type_splits = vec![ModuleSplit {
            suggested_name: "type_based".to_string(),
            ..Default::default()
        }];
        let flow_splits = vec![ModuleSplit {
            suggested_name: "flow_based".to_string(),
            ..Default::default()
        }];

        let result = analyzer.resolve_conflicts(Some(type_splits), Some(flow_splits));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].suggested_name, "type_based");
    }

    #[test]
    fn test_conflict_resolution_data_flow() {
        let analyzer = IntegratedArchitectureAnalyzer::with_config(AnalysisConfig {
            conflict_resolution: ConflictResolutionStrategy::DataFlow,
            ..Default::default()
        });

        let type_splits = vec![ModuleSplit {
            suggested_name: "type_based".to_string(),
            ..Default::default()
        }];
        let flow_splits = vec![ModuleSplit {
            suggested_name: "flow_based".to_string(),
            ..Default::default()
        }];

        let result = analyzer.resolve_conflicts(Some(type_splits), Some(flow_splits));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].suggested_name, "flow_based");
    }

    #[test]
    fn test_merge_splits_no_overlap() {
        let analyzer = IntegratedArchitectureAnalyzer::new();

        let type_splits = vec![ModuleSplit {
            methods_to_move: vec!["method1".to_string(), "method2".to_string()],
            ..Default::default()
        }];

        let flow_splits = vec![ModuleSplit {
            methods_to_move: vec!["method3".to_string(), "method4".to_string()],
            ..Default::default()
        }];

        let result = analyzer.merge_splits(Some(type_splits), Some(flow_splits));
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_merge_splits_with_overlap() {
        let analyzer = IntegratedArchitectureAnalyzer::new();

        let type_splits = vec![ModuleSplit {
            methods_to_move: vec!["method1".to_string(), "method2".to_string()],
            ..Default::default()
        }];

        // More than 50% overlap
        let flow_splits = vec![ModuleSplit {
            methods_to_move: vec![
                "method1".to_string(),
                "method2".to_string(),
                "method3".to_string(),
            ],
            ..Default::default()
        }];

        let result = analyzer.merge_splits(Some(type_splits), Some(flow_splits));
        assert_eq!(result.len(), 1); // Overlapping split filtered out
    }
}
