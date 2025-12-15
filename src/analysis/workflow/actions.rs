//! Effectful Actions for Analysis Workflow (Spec 202)
//!
//! Actions are functions that perform side effects (I/O, progress reporting)
//! and state mutations. They receive environment references for side effects.
//!
//! ## Design Principles
//!
//! - **Guards first** - Each action has a corresponding guard checked before execution
//! - **Environment abstraction** - Side effects go through environment traits
//! - **Pure computation extracted** - Actual analysis logic is pure, I/O is at edges
//! - **State updates explicit** - Actions explicitly update state after computation

use super::{
    env::{AnalysisEnv, ProgressReporter},
    guards::*,
    state::{AnalysisPhase, AnalysisState},
};
use crate::priority::UnifiedAnalysisUtils;
use anyhow::{Context, Result};

/// Action: Build call graph from metrics.
///
/// Effectful - reports progress via environment.
pub fn build_call_graph<Env: AnalysisEnv>(state: &mut AnalysisState, env: &mut Env) -> Result<()> {
    debug_assert!(can_start_call_graph(state), "Guard should be checked first");

    env.phase_starting(AnalysisPhase::CallGraphBuilding.display_name());
    state.transition_to(AnalysisPhase::CallGraphBuilding);

    // Build initial call graph from metrics
    let metrics = state
        .results
        .metrics
        .as_ref()
        .expect("guard ensures metrics exist");
    let mut call_graph = crate::builders::call_graph::build_initial_call_graph(metrics);

    // Process files to enrich call graph
    // Note: Parallel mode uses TUI-native progress reporting (Send+Sync compatible)
    // Sequential mode uses environment-based progress reporting
    let (framework_exclusions, function_pointer_used_functions) = if state.config.parallel {
        let (graph, exclusions, used_funcs) =
            crate::builders::parallel_call_graph::build_call_graph_parallel(
                &state.config.project_path,
                call_graph.clone(),
                if state.config.jobs == 0 {
                    None
                } else {
                    Some(state.config.jobs)
                },
                |_progress| {
                    // Progress is handled by TUI directly in parallel mode
                    // This callback is provided for compatibility but doesn't
                    // use the environment since parallel callbacks require Send+Sync
                },
            )
            .context("Failed to build parallel call graph")?;
        call_graph = graph;
        (exclusions, used_funcs)
    } else {
        crate::builders::call_graph::process_rust_files_for_call_graph(
            &state.config.project_path,
            &mut call_graph,
            state.config.verbose_macro_warnings,
            state.config.show_macro_stats,
            |progress| {
                env.phase_progress(progress.current as f64 / progress.total.max(1) as f64 * 0.3);
            },
        )
        .context("Failed to process Rust files for call graph")?
    };

    // Apply trait pattern detection
    {
        use crate::analysis::call_graph::TraitRegistry;
        let trait_registry = TraitRegistry::new();
        trait_registry.detect_common_trait_patterns(&mut call_graph);
    }

    // Store results
    state.results.call_graph = Some(call_graph);
    state.results.framework_exclusions = Some(framework_exclusions);
    state.results.function_pointer_used_functions = Some(function_pointer_used_functions);

    state.transition_to(AnalysisPhase::CallGraphComplete);
    env.phase_complete();

    Ok(())
}

/// Action: Load coverage data from LCOV file.
///
/// Effectful - reads file via environment, reports progress.
pub fn load_coverage<Env: AnalysisEnv>(state: &mut AnalysisState, env: &mut Env) -> Result<()> {
    debug_assert!(can_start_coverage(state), "Guard should be checked first");

    env.phase_starting(AnalysisPhase::CoverageLoading.display_name());
    state.transition_to(AnalysisPhase::CoverageLoading);

    let coverage_path = state
        .config
        .coverage_file
        .as_ref()
        .expect("guard ensures coverage_file exists");

    // Parse LCOV file
    let coverage_data =
        crate::risk::lcov::parse_lcov_file_with_callback(coverage_path, |progress| {
            use crate::risk::lcov::CoverageProgress;
            match progress {
                CoverageProgress::Initializing => env.phase_progress(0.1),
                CoverageProgress::Parsing { current, total } => {
                    env.phase_progress(0.1 + (current as f64 / total.max(1) as f64) * 0.7)
                }
                CoverageProgress::ComputingStats => env.phase_progress(0.9),
                CoverageProgress::Complete => env.phase_progress(1.0),
            }
        })
        .context("Failed to parse LCOV file")?;

    state.results.coverage = Some(coverage_data);

    state.transition_to(AnalysisPhase::CoverageComplete);
    env.phase_complete();

    Ok(())
}

/// Action: Skip coverage loading (no file configured).
///
/// Pure state transition - no side effects except state update.
pub fn skip_coverage(state: &mut AnalysisState) {
    debug_assert!(can_skip_coverage(state), "Guard should be checked first");

    state.results.coverage = None;
    state.transition_to(AnalysisPhase::CoverageComplete);
}

/// Action: Analyze function purity.
///
/// Effectful - reports progress via environment.
pub fn analyze_purity<Env: ProgressReporter>(
    state: &mut AnalysisState,
    env: &mut Env,
) -> Result<()> {
    debug_assert!(can_start_purity(state), "Guard should be checked first");

    env.phase_starting(AnalysisPhase::PurityAnalyzing.display_name());
    state.transition_to(AnalysisPhase::PurityAnalyzing);

    let metrics = state
        .results
        .metrics
        .as_ref()
        .expect("guard ensures metrics exist");
    let call_graph = state
        .results
        .call_graph
        .as_ref()
        .expect("guard ensures call_graph exists");

    // Populate call graph data into metrics
    let enriched_metrics = crate::analyzers::call_graph_integration::populate_call_graph_data(
        metrics.clone(),
        call_graph,
    );

    env.phase_progress(0.3);

    // Run purity propagation using the existing implementation
    use crate::analysis::call_graph::{
        CrossModuleTracker, FrameworkPatternDetector, FunctionPointerTracker, RustCallGraph,
        TraitRegistry,
    };
    use crate::analysis::purity_analysis::PurityAnalyzer;
    use crate::analysis::purity_propagation::{PurityCallGraphAdapter, PurityPropagator};
    use crate::priority::call_graph::FunctionId;

    // Create RustCallGraph wrapper
    let rust_graph = RustCallGraph {
        base_graph: call_graph.clone(),
        trait_registry: TraitRegistry::new(),
        function_pointer_tracker: FunctionPointerTracker::new(),
        framework_patterns: FrameworkPatternDetector::new(),
        cross_module_tracker: CrossModuleTracker::new(),
    };

    let adapter = PurityCallGraphAdapter::from_rust_graph(rust_graph);
    let purity_analyzer = PurityAnalyzer::new();
    let mut propagator = PurityPropagator::new(adapter, purity_analyzer);

    env.phase_progress(0.5);

    // Run propagation - failures are expected when external dependencies are called
    if let Err(e) = propagator.propagate(&enriched_metrics) {
        log::debug!("Purity propagation skipped (external deps): {}", e);
        state.results.enriched_metrics = Some(enriched_metrics);
        state.transition_to(AnalysisPhase::PurityComplete);
        env.phase_complete();
        return Ok(());
    }

    env.phase_progress(0.8);

    // Apply propagation results to metrics
    let final_metrics: Vec<_> = enriched_metrics
        .iter()
        .map(|metric| {
            let func_id = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line);

            if let Some(result) = propagator.get_result(&func_id) {
                let mut updated = metric.clone();
                updated.is_pure = Some(
                    result.level == crate::analysis::purity_analysis::PurityLevel::StrictlyPure,
                );
                updated.purity_confidence = Some(result.confidence as f32);
                updated.purity_reason = Some(format!("{:?}", result.reason));
                updated
            } else {
                metric.clone()
            }
        })
        .collect();

    state.results.enriched_metrics = Some(final_metrics);

    state.transition_to(AnalysisPhase::PurityComplete);
    env.phase_complete();

    Ok(())
}

/// Action: Load context providers.
///
/// Effectful - loads git history and other context.
pub fn load_context<Env: AnalysisEnv>(state: &mut AnalysisState, env: &mut Env) -> Result<()> {
    debug_assert!(can_start_context(state), "Guard should be checked first");

    env.phase_starting(AnalysisPhase::ContextLoading.display_name());
    state.transition_to(AnalysisPhase::ContextLoading);

    let context_aggregator = crate::utils::risk_analyzer::build_context_aggregator(
        &state.config.project_path,
        state.config.enable_context,
        state.config.context_providers.clone(),
        state.config.disable_context.clone(),
    );

    if let Some(aggregator) = context_aggregator {
        let debt_score = 50.0; // Default middle value, will be updated during scoring
        let debt_threshold = 100.0;

        let risk_analyzer = crate::risk::RiskAnalyzer::default()
            .with_debt_context(debt_score, debt_threshold)
            .with_context_aggregator(aggregator);

        state.results.risk_analyzer = Some(risk_analyzer);
    }

    state.transition_to(AnalysisPhase::ContextComplete);
    env.phase_complete();

    Ok(())
}

/// Action: Skip context loading (not enabled).
///
/// Pure state transition - no side effects except state update.
pub fn skip_context(state: &mut AnalysisState) {
    debug_assert!(can_skip_context(state), "Guard should be checked first");

    state.results.risk_analyzer = None;
    state.transition_to(AnalysisPhase::ContextComplete);
}

/// Action: Compute debt scores.
///
/// Effectful - reports progress via environment.
pub fn compute_scores<Env: ProgressReporter>(
    state: &mut AnalysisState,
    env: &mut Env,
) -> Result<()> {
    debug_assert!(can_start_scoring(state), "Guard should be checked first");

    env.phase_starting(AnalysisPhase::ScoringInProgress.display_name());
    state.transition_to(AnalysisPhase::ScoringInProgress);

    let call_graph = state
        .results
        .call_graph
        .as_ref()
        .expect("guard ensures call_graph exists");
    let enriched_metrics = state
        .results
        .enriched_metrics
        .as_ref()
        .expect("guard ensures enriched_metrics exist");
    let framework_exclusions = state
        .results
        .framework_exclusions
        .clone()
        .unwrap_or_default();
    let function_pointer_used = state.results.function_pointer_used_functions.clone();

    // Create unified analysis
    let unified = crate::builders::unified_analysis::create_unified_analysis_with_exclusions(
        enriched_metrics,
        call_graph,
        state.results.coverage.as_ref(),
        &framework_exclusions,
        function_pointer_used.as_ref(),
        None, // debt_items - not used in this path
        state.config.no_aggregation,
        state.config.aggregation_method.clone(),
        state.config.min_problematic,
        state.config.no_god_object,
    );

    env.phase_progress(0.9);

    state.results.unified_analysis = Some(unified);

    state.transition_to(AnalysisPhase::ScoringComplete);
    env.phase_complete();

    Ok(())
}

/// Action: Filter and rank results.
///
/// Effectful - reports progress via environment.
pub fn filter_and_rank<Env: ProgressReporter>(
    state: &mut AnalysisState,
    env: &mut Env,
) -> Result<()> {
    debug_assert!(can_start_filtering(state), "Guard should be checked first");

    env.phase_starting(AnalysisPhase::FilteringInProgress.display_name());
    state.transition_to(AnalysisPhase::FilteringInProgress);

    if let Some(ref mut unified) = state.results.unified_analysis {
        unified.sort_by_priority();
        unified.calculate_total_impact();
    }

    state.transition_to(AnalysisPhase::Complete);
    env.phase_complete();

    Ok(())
}

/// Workflow runner that executes the complete analysis workflow.
///
/// This is the main entry point for running analysis with the state machine.
pub struct WorkflowRunner<Env> {
    state: AnalysisState,
    env: Env,
}

impl<Env: AnalysisEnv> WorkflowRunner<Env> {
    /// Create a new workflow runner with the given state and environment.
    pub fn new(state: AnalysisState, env: Env) -> Self {
        Self { state, env }
    }

    /// Get a reference to the current state.
    pub fn state(&self) -> &AnalysisState {
        &self.state
    }

    /// Get a mutable reference to the current state.
    pub fn state_mut(&mut self) -> &mut AnalysisState {
        &mut self.state
    }

    /// Consume the runner and return the final state.
    pub fn into_state(self) -> AnalysisState {
        self.state
    }

    /// Run a single step of the workflow based on current state.
    ///
    /// Returns `true` if a step was executed, `false` if no transition was possible.
    pub fn step(&mut self) -> Result<bool> {
        // Try each transition in order
        if can_start_call_graph(&self.state) {
            build_call_graph(&mut self.state, &mut self.env)?;
            return Ok(true);
        }

        if can_start_coverage(&self.state) {
            load_coverage(&mut self.state, &mut self.env)?;
            return Ok(true);
        }

        if can_skip_coverage(&self.state) {
            skip_coverage(&mut self.state);
            return Ok(true);
        }

        if can_start_purity(&self.state) {
            analyze_purity(&mut self.state, &mut self.env)?;
            return Ok(true);
        }

        if can_start_context(&self.state) {
            load_context(&mut self.state, &mut self.env)?;
            return Ok(true);
        }

        if can_skip_context(&self.state) {
            skip_context(&mut self.state);
            return Ok(true);
        }

        if can_start_scoring(&self.state) {
            compute_scores(&mut self.state, &mut self.env)?;
            return Ok(true);
        }

        if can_start_filtering(&self.state) {
            filter_and_rank(&mut self.state, &mut self.env)?;
            return Ok(true);
        }

        // No transition possible - either complete or missing prerequisites
        Ok(false)
    }

    /// Run the complete workflow until completion or error.
    pub fn run(mut self) -> Result<AnalysisState> {
        while !self.state.is_complete() {
            if !self.step()? {
                // No progress made - missing prerequisites
                return Err(anyhow::anyhow!(
                    "Workflow stuck at phase {:?} - missing prerequisites",
                    self.state.phase
                ));
            }
        }

        Ok(self.state)
    }
}

/// Run the complete analysis workflow.
///
/// Convenience function that creates a runner and executes the full workflow.
pub fn run_analysis<Env: AnalysisEnv>(state: AnalysisState, env: Env) -> Result<AnalysisState> {
    WorkflowRunner::new(state, env).run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::workflow::env::MockAnalysisEnv;
    use crate::analysis::workflow::state::AnalysisConfig;
    use crate::core::FunctionMetrics;
    use std::path::PathBuf;

    fn create_test_config() -> AnalysisConfig {
        AnalysisConfig {
            project_path: PathBuf::from("."),
            ..Default::default()
        }
    }

    fn create_test_metrics() -> Vec<FunctionMetrics> {
        vec![FunctionMetrics::new(
            "test_fn".to_string(),
            PathBuf::from("test.rs"),
            1,
        )]
    }

    #[test]
    fn test_workflow_runner_creation() {
        let config = create_test_config();
        let state = AnalysisState::with_metrics(config, create_test_metrics());
        let env = MockAnalysisEnv::new();

        let runner = WorkflowRunner::new(state, env);
        assert_eq!(runner.state().phase, AnalysisPhase::Initialized);
    }

    #[test]
    fn test_skip_coverage_action() {
        let config = create_test_config();
        let mut state = AnalysisState::with_metrics(config, create_test_metrics());
        state.phase = AnalysisPhase::CallGraphComplete;
        state.results.call_graph = Some(crate::priority::call_graph::CallGraph::new());

        // Should be able to skip coverage
        assert!(can_skip_coverage(&state));

        skip_coverage(&mut state);

        assert_eq!(state.phase, AnalysisPhase::CoverageComplete);
        assert!(state.results.coverage.is_none());
    }

    #[test]
    fn test_skip_context_action() {
        let config = create_test_config();
        let mut state = AnalysisState::with_metrics(config, create_test_metrics());
        state.phase = AnalysisPhase::PurityComplete;
        state.results.enriched_metrics = Some(create_test_metrics());

        // Should be able to skip context
        assert!(can_skip_context(&state));

        skip_context(&mut state);

        assert_eq!(state.phase, AnalysisPhase::ContextComplete);
        assert!(state.results.risk_analyzer.is_none());
    }

    #[test]
    fn test_workflow_step_no_metrics() {
        let config = create_test_config();
        let state = AnalysisState::new(config); // No metrics
        let env = MockAnalysisEnv::new();

        let mut runner = WorkflowRunner::new(state, env);

        // Should not be able to step without metrics
        let stepped = runner.step().unwrap();
        assert!(!stepped);
    }
}
