pub mod context;
pub mod correlation;
pub mod coverage_gap;
pub mod coverage_index;
pub mod delegation;
pub mod effects;
pub mod evidence;
pub mod evidence_calculator;
pub mod function_name_matching;
pub mod insights;
pub mod lcov;
pub mod path_normalization;
pub mod priority;
pub mod roi;
pub mod strategy;
pub mod thresholds;

use crate::core::ComplexityMetrics;
use im::Vector;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionRisk {
    pub file: PathBuf,
    pub function_name: String,
    pub line_range: (usize, usize),
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub coverage_percentage: Option<f64>,
    pub risk_score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contextual_risk: Option<context::ContextualRisk>,
    pub test_effort: TestEffort,
    pub risk_category: RiskCategory,
    pub is_test_function: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RiskCategory {
    Critical,   // High complexity (>15), low coverage (<30%)
    High,       // High complexity (>10), moderate coverage (<60%)
    Medium,     // Moderate complexity (>5), low coverage (<50%)
    Low,        // Low complexity or high coverage
    WellTested, // High complexity with high coverage (good examples)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestEffort {
    pub estimated_difficulty: Difficulty,
    pub cognitive_load: u32,
    pub branch_count: u32,
    pub recommended_test_cases: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Difficulty {
    Trivial,     // Cognitive < 5
    Simple,      // Cognitive 5-10
    Moderate,    // Cognitive 10-20
    Complex,     // Cognitive 20-40
    VeryComplex, // Cognitive > 40
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskInsight {
    pub top_risks: Vector<FunctionRisk>,
    pub risk_reduction_opportunities: Vector<TestingRecommendation>,
    pub codebase_risk_score: f64,
    pub complexity_coverage_correlation: Option<f64>,
    pub risk_distribution: RiskDistribution,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestingRecommendation {
    pub function: String,
    pub file: PathBuf,
    pub line: usize,
    pub current_risk: f64,
    pub potential_risk_reduction: f64,
    pub test_effort_estimate: TestEffort,
    pub rationale: String,
    pub roi: Option<f64>,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskDistribution {
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub well_tested_count: usize,
    pub total_functions: usize,
}

use self::context::{AnalysisTarget, ContextAggregator, ContextualRisk};
use self::strategy::{EnhancedRiskStrategy, RiskCalculator, RiskContext};
use std::sync::Arc;

pub struct RiskAnalyzer {
    strategy: Box<dyn RiskCalculator>,
    debt_score: Option<f64>,
    debt_threshold: Option<f64>,
    context_aggregator: Option<Arc<ContextAggregator>>,
}

impl Clone for RiskAnalyzer {
    /// Clone the risk analyzer, preserving context aggregator.
    ///
    /// The context aggregator is wrapped in Arc, so cloning is cheap (just
    /// an atomic reference count increment) and preserves the shared cache.
    fn clone(&self) -> Self {
        Self {
            strategy: self.strategy.box_clone(),
            debt_score: self.debt_score,
            debt_threshold: self.debt_threshold,
            context_aggregator: self.context_aggregator.clone(), // Arc::clone is cheap!
        }
    }
}

impl Default for RiskAnalyzer {
    fn default() -> Self {
        Self {
            strategy: Box::new(EnhancedRiskStrategy::default()),
            debt_score: None,
            debt_threshold: None,
            context_aggregator: None,
        }
    }
}

impl RiskAnalyzer {
    pub fn with_debt_context(mut self, debt_score: f64, debt_threshold: f64) -> Self {
        self.debt_score = Some(debt_score);
        self.debt_threshold = Some(debt_threshold);
        self
    }

    pub fn with_context_aggregator(mut self, aggregator: ContextAggregator) -> Self {
        self.context_aggregator = Some(Arc::new(aggregator));
        self
    }

    pub fn has_context(&self) -> bool {
        self.context_aggregator.is_some()
    }

    pub fn analyze_function(
        &self,
        file: PathBuf,
        function_name: String,
        line_range: (usize, usize),
        complexity: &ComplexityMetrics,
        coverage: Option<f64>,
        is_test: bool,
    ) -> FunctionRisk {
        let context = RiskContext {
            file,
            function_name,
            line_range,
            complexity: complexity.clone(),
            coverage,
            debt_score: self.debt_score,
            debt_threshold: self.debt_threshold,
            is_test,
            is_recognized_pattern: false,
            pattern_type: None,
            pattern_confidence: 0.0,
        };

        self.strategy.calculate(&context)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn analyze_function_with_context(
        &self,
        file: PathBuf,
        function_name: String,
        line_range: (usize, usize),
        complexity: &ComplexityMetrics,
        coverage: Option<f64>,
        is_test: bool,
        root_path: PathBuf,
    ) -> (FunctionRisk, Option<ContextualRisk>) {
        let mut base_risk = self.analyze_function(
            file.clone(),
            function_name.clone(),
            line_range,
            complexity,
            coverage,
            is_test,
        );

        let contextual_risk = if let Some(ref aggregator) = self.context_aggregator {
            let target = AnalysisTarget {
                root_path,
                file_path: file,
                function_name: function_name.clone(),
                line_range,
            };

            let context_map = aggregator.analyze(&target);
            let ctx_risk = ContextualRisk::new(base_risk.risk_score, &context_map);

            // Update the FunctionRisk with contextual data
            base_risk.contextual_risk = Some(ctx_risk.clone());
            base_risk.risk_score = ctx_risk.contextual_risk;

            // Verbose logging for context contributions
            if log::log_enabled!(log::Level::Debug) {
                log::debug!(
                    "Context analysis for {}::{}: base_risk={:.1}, contextual_risk={:.1}, multiplier={:.2}x",
                    base_risk.file.display(),
                    function_name,
                    ctx_risk.base_risk,
                    ctx_risk.contextual_risk,
                    ctx_risk.contextual_risk / ctx_risk.base_risk.max(0.1)
                );

                for context in &ctx_risk.contexts {
                    log::debug!(
                        "  └─ {}: contribution={:.2}, weight={:.1}, impact=+{:.1}",
                        context.provider,
                        context.contribution,
                        context.weight,
                        context.contribution * context.weight
                    );
                }
            }

            Some(ctx_risk)
        } else {
            None
        };

        (base_risk, contextual_risk)
    }

    pub fn calculate_risk_score(
        &self,
        cyclomatic: u32,
        cognitive: u32,
        coverage: Option<f64>,
    ) -> f64 {
        let context = RiskContext {
            file: PathBuf::new(),
            function_name: String::new(),
            line_range: (0, 0),
            complexity: ComplexityMetrics {
                functions: vec![],
                cyclomatic_complexity: cyclomatic,
                cognitive_complexity: cognitive,
            },
            coverage,
            debt_score: self.debt_score,
            debt_threshold: self.debt_threshold,
            is_test: false,
            is_recognized_pattern: false,
            pattern_type: None,
            pattern_confidence: 0.0,
        };

        self.strategy.calculate_risk_score(&context)
    }

    pub fn calculate_risk_reduction(
        &self,
        current_risk: f64,
        complexity: u32,
        target_coverage: f64,
    ) -> f64 {
        self.strategy
            .calculate_risk_reduction(current_risk, complexity, target_coverage)
    }

    /// Analyze file-level contextual risk for god objects.
    ///
    /// This method specifically handles file-level analysis where there is no
    /// specific function being analyzed. It's designed for god objects where
    /// the entire file represents the technical debt unit.
    ///
    /// # Arguments
    /// * `file_path` - Path to the file being analyzed
    /// * `base_risk` - Base risk score for the god object (from god object scoring)
    /// * `root_path` - Project root path
    ///
    /// # Returns
    /// `Some(ContextualRisk)` if context analysis is enabled, `None` otherwise
    pub fn analyze_file_context(
        &self,
        file_path: PathBuf,
        base_risk: f64,
        root_path: PathBuf,
    ) -> Option<ContextualRisk> {
        let aggregator = self.context_aggregator.as_ref()?;

        let target = AnalysisTarget {
            root_path,
            file_path,
            function_name: String::new(), // Empty for file-level analysis
            line_range: (0, 0),           // Not applicable for file-level
        };

        let context_map = aggregator.analyze(&target);
        Some(ContextualRisk::new(base_risk, &context_map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_analyzer_clone_preserves_context() {
        let aggregator = ContextAggregator::new();

        let analyzer = RiskAnalyzer::default().with_context_aggregator(aggregator);

        let cloned = analyzer.clone();

        assert!(cloned.has_context());
    }

    /// Stress test: analyze many functions with context to detect stack overflow.
    /// This simulates what happens when running `debtmap analyze --context`
    /// on a large codebase like debtmap itself (~4000 functions).
    #[test]
    fn test_analyze_many_functions_with_context_no_stack_overflow() {
        use crate::core::ComplexityMetrics;
        use crate::priority::call_graph::CallGraph;
        use crate::risk::context::critical_path::{
            CriticalPathAnalyzer, CriticalPathProvider, EntryPoint, EntryType,
        };
        use crate::risk::context::dependency::{DependencyGraph, DependencyRiskProvider};

        // Build a realistic call graph
        let mut call_graph = CallGraph::new();
        for i in 0..2000 {
            let caller = format!("func_{}", i);
            let callee = format!("func_{}", i + 1);
            call_graph.add_edge_by_name(caller, callee, PathBuf::from("src/lib.rs"));
        }

        // Create critical path analyzer with entry point
        let mut cp_analyzer = CriticalPathAnalyzer::new();
        cp_analyzer.call_graph = call_graph;
        cp_analyzer.entry_points.push_back(EntryPoint {
            function_name: "func_0".to_string(),
            file_path: PathBuf::from("src/main.rs"),
            entry_type: EntryType::Main,
            is_user_facing: true,
        });

        // Build aggregator with providers
        let aggregator = ContextAggregator::new()
            .with_provider(Box::new(CriticalPathProvider::new(cp_analyzer)))
            .with_provider(Box::new(
                DependencyRiskProvider::new(DependencyGraph::new()),
            ));

        // Create risk analyzer with context
        let analyzer = RiskAnalyzer::default().with_context_aggregator(aggregator);

        // Analyze many functions - this is what crashes in production
        for i in 0..500 {
            let complexity = ComplexityMetrics {
                functions: vec![],
                cyclomatic_complexity: 10,
                cognitive_complexity: 15,
            };

            let (_risk, contextual) = analyzer.analyze_function_with_context(
                PathBuf::from(format!("src/module_{}.rs", i % 50)),
                format!("func_{}", i),
                (1, 50),
                &complexity,
                Some(0.75),
                false,
                PathBuf::from("/project"),
            );

            // Verify we got context
            if i < 100 {
                // First 100 should be cache misses
                assert!(
                    contextual.is_some(),
                    "Should get contextual risk for function {}",
                    i
                );
            }
        }
    }

    /// Test file-level context analysis (for god objects) at scale
    #[test]
    fn test_analyze_file_context_many_files_no_stack_overflow() {
        use crate::risk::context::critical_path::{
            CriticalPathAnalyzer, CriticalPathProvider, EntryPoint, EntryType,
        };
        use crate::risk::context::dependency::{DependencyGraph, DependencyRiskProvider};

        // Create aggregator
        let mut cp_analyzer = CriticalPathAnalyzer::new();
        cp_analyzer.entry_points.push_back(EntryPoint {
            function_name: "main".to_string(),
            file_path: PathBuf::from("src/main.rs"),
            entry_type: EntryType::Main,
            is_user_facing: true,
        });

        let aggregator = ContextAggregator::new()
            .with_provider(Box::new(CriticalPathProvider::new(cp_analyzer)))
            .with_provider(Box::new(
                DependencyRiskProvider::new(DependencyGraph::new()),
            ));

        let analyzer = RiskAnalyzer::default().with_context_aggregator(aggregator);

        // Analyze many files - simulates god object analysis
        for i in 0..200 {
            let result = analyzer.analyze_file_context(
                PathBuf::from(format!("src/large_file_{}.rs", i)),
                40.0,
                PathBuf::from("/project"),
            );

            assert!(result.is_some(), "Should get context for file {}", i);
        }
    }

    /// Minimal mock provider for testing
    struct MockProvider {
        name: &'static str,
    }

    impl context::ContextProvider for MockProvider {
        fn name(&self) -> &str {
            self.name
        }

        fn gather(&self, _target: &context::AnalysisTarget) -> anyhow::Result<context::Context> {
            Ok(context::Context {
                provider: self.name.to_string(),
                weight: 1.0,
                contribution: 0.5,
                details: context::ContextDetails::Historical {
                    change_frequency: 0.1,
                    bug_density: 0.05,
                    age_days: 100,
                    author_count: 3,
                },
            })
        }

        fn weight(&self) -> f64 {
            1.0
        }

        fn explain(&self, _context: &context::Context) -> String {
            "mock".to_string()
        }
    }

    /// Test with 3 providers (matching production: critical_path, dependency, git_history)
    #[test]
    fn test_three_providers_many_iterations() {
        // Build aggregator with 3 mock providers
        let aggregator = ContextAggregator::new()
            .with_provider(Box::new(MockProvider {
                name: "critical_path",
            }))
            .with_provider(Box::new(MockProvider {
                name: "dependency_risk",
            }))
            .with_provider(Box::new(MockProvider {
                name: "git_history",
            }));

        let analyzer = RiskAnalyzer::default().with_context_aggregator(aggregator);

        // Run many iterations - each should be independent
        for i in 0..5000 {
            let result = analyzer.analyze_file_context(
                PathBuf::from(format!("src/file_{}.rs", i)),
                40.0,
                PathBuf::from("/project"),
            );

            assert!(result.is_some(), "Iteration {} should succeed", i);
        }
    }

    /// Test parallel execution with rayon - this is closer to production behavior
    #[test]
    fn test_parallel_context_analysis_with_rayon() {
        use rayon::prelude::*;

        // Build aggregator with 3 mock providers
        let aggregator = ContextAggregator::new()
            .with_provider(Box::new(MockProvider {
                name: "critical_path",
            }))
            .with_provider(Box::new(MockProvider {
                name: "dependency_risk",
            }))
            .with_provider(Box::new(MockProvider {
                name: "git_history",
            }));

        let analyzer = RiskAnalyzer::default().with_context_aggregator(aggregator);

        // Run in parallel - this uses rayon's thread pool with smaller stacks
        let results: Vec<_> = (0..5000)
            .into_par_iter()
            .map(|i| {
                analyzer.analyze_file_context(
                    PathBuf::from(format!("src/file_{}.rs", i)),
                    40.0,
                    PathBuf::from("/project"),
                )
            })
            .collect();

        assert_eq!(results.len(), 5000);
        assert!(results.iter().all(|r| r.is_some()));
    }
}
