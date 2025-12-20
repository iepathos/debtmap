use crate::{
    analysis::ContextDetector,
    analyzers::FileAnalyzer,
    core::FunctionMetrics,
    data_flow::DataFlowGraph,
    extraction::ExtractedFileData,
    priority::{
        call_graph::{CallGraph, FunctionId},
        debt_aggregator::{DebtAggregator, FunctionId as AggregatorFunctionId},
        file_metrics::FileDebtItem,
        scoring::ContextRecommendationEngine,
        UnifiedAnalysis, UnifiedAnalysisUtils, UnifiedDebtItem,
    },
    progress::ProgressManager,
    risk::lcov::LcovData,
};
use indicatif::ParallelProgressIterator;
use parking_lot::Mutex;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tracing::{debug_span, warn};

// Pure functional transformations module
mod transformations {
    use super::*;

    /// Pure function to create function mappings from metrics
    pub fn create_function_mappings(
        metrics: &[FunctionMetrics],
    ) -> Vec<(AggregatorFunctionId, usize, usize)> {
        metrics
            .iter()
            .map(|m| {
                let func_id = AggregatorFunctionId::new(m.file.clone(), m.name.clone(), m.line);
                (func_id, m.line, m.line + m.length)
            })
            .collect()
    }

    /// Pure function to transform metrics into purity map
    pub fn metrics_to_purity_map(metrics: &[FunctionMetrics]) -> HashMap<String, bool> {
        metrics
            .iter()
            .map(|m| (m.name.clone(), m.is_pure.unwrap_or(false)))
            .collect()
    }

    // Note: extract_purity_analysis removed in spec 213.
    // The fallback path now uses UnifiedFileExtractor + populate_all_from_extracted instead.
}

// Pure predicates module for filtering logic
mod predicates {
    use super::*;

    /// Pure predicate: should skip test functions
    pub fn is_test_function(metric: &FunctionMetrics) -> bool {
        metric.is_test || metric.in_test_module
    }

    /// Pure predicate: is closure function
    pub fn is_closure(metric: &FunctionMetrics) -> bool {
        metric.name.contains("<closure@")
    }

    /// Pure predicate: is trivial function
    pub fn is_trivial_function(metric: &FunctionMetrics, callee_count: usize) -> bool {
        metric.cyclomatic == 1 && metric.cognitive == 0 && metric.length <= 3 && callee_count == 1
    }

    /// Pure predicate: should process metric
    pub fn should_process_metric(
        metric: &FunctionMetrics,
        test_only_functions: &HashSet<FunctionId>,
        callee_count: usize,
    ) -> bool {
        // Early returns for test functions and closures
        if is_test_function(metric) || is_closure(metric) {
            return false;
        }

        let func_id = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line);

        // Skip if in test-only functions set
        if test_only_functions.contains(&func_id) {
            return false;
        }

        // Skip trivial functions
        !is_trivial_function(metric, callee_count)
    }
}

// Pure file analysis transformations
mod file_analysis {
    use super::*;
    use crate::analyzers::file_analyzer::UnifiedFileAnalyzer;
    use crate::priority::file_metrics::FileDebtMetrics;

    /// Pure function to aggregate function metrics into file metrics
    pub fn aggregate_file_metrics(
        functions: &[FunctionMetrics],
        coverage_data: Option<&LcovData>,
    ) -> FileDebtMetrics {
        let file_analyzer = UnifiedFileAnalyzer::new(coverage_data.cloned());
        file_analyzer.aggregate_functions(functions)
    }

    /// Pure function to analyze god object from file content
    pub fn analyze_god_object(
        content: &str,
        file_path: &Path,
        coverage_data: Option<&LcovData>,
    ) -> Result<Option<crate::organization::GodObjectAnalysis>, String> {
        let file_analyzer = UnifiedFileAnalyzer::new(coverage_data.cloned());
        file_analyzer
            .analyze_file(file_path, content)
            .map(|analyzed| analyzed.god_object_analysis)
            .map_err(|e| format!("Failed to analyze god object: {}", e))
    }

    /// Pure function to determine if file should be included based on score
    pub fn should_include_file(score: f64) -> bool {
        score > 50.0
    }
}

/// Options for parallel unified analysis
pub struct ParallelUnifiedAnalysisOptions {
    pub parallel: bool,
    pub jobs: Option<usize>,
    pub batch_size: usize,
    pub progress: bool,
}

impl Default for ParallelUnifiedAnalysisOptions {
    fn default() -> Self {
        Self {
            parallel: true,
            jobs: None, // Use all available cores
            batch_size: 100,
            progress: true,
        }
    }
}

/// Timing information for analysis phases
#[derive(Debug, Clone)]
pub struct AnalysisPhaseTimings {
    pub call_graph_building: Duration,
    pub trait_resolution: Duration,
    pub coverage_loading: Duration,
    pub data_flow_creation: Duration,
    pub purity_analysis: Duration,
    pub test_detection: Duration,
    pub debt_aggregation: Duration,
    pub function_analysis: Duration,
    pub file_analysis: Duration,
    pub aggregation: Duration,
    pub sorting: Duration,
    pub total: Duration,
}

impl Default for AnalysisPhaseTimings {
    fn default() -> Self {
        Self {
            call_graph_building: Duration::from_secs(0),
            trait_resolution: Duration::from_secs(0),
            coverage_loading: Duration::from_secs(0),
            data_flow_creation: Duration::from_secs(0),
            purity_analysis: Duration::from_secs(0),
            test_detection: Duration::from_secs(0),
            debt_aggregation: Duration::from_secs(0),
            function_analysis: Duration::from_secs(0),
            file_analysis: Duration::from_secs(0),
            aggregation: Duration::from_secs(0),
            sorting: Duration::from_secs(0),
            total: Duration::from_secs(0),
        }
    }
}

/// Context for function analysis - groups all dependencies
struct FunctionAnalysisContext<'a> {
    call_graph: &'a CallGraph,
    debt_aggregator: &'a DebtAggregator,
    data_flow_graph: &'a DataFlowGraph,
    coverage_data: Option<&'a LcovData>,
    framework_exclusions: &'a HashSet<FunctionId>,
    function_pointer_used_functions: Option<&'a HashSet<FunctionId>>,
    risk_analyzer: Option<&'a crate::risk::RiskAnalyzer>,
    project_path: &'a Path,
    // Shared detectors to avoid per-metric regex compilation (spec 196 optimization)
    context_detector: &'a ContextDetector,
    recommendation_engine: &'a ContextRecommendationEngine,
}

/// Optimized test detector with caching
pub struct OptimizedTestDetector {
    call_graph: Arc<CallGraph>,
    test_roots: HashSet<FunctionId>,
    reachability_cache: Arc<RwLock<HashMap<FunctionId, bool>>>,
}

impl OptimizedTestDetector {
    pub fn new(call_graph: Arc<CallGraph>) -> Self {
        let test_roots = Self::find_test_roots(&call_graph);
        Self {
            call_graph,
            test_roots,
            reachability_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn find_test_roots(call_graph: &Arc<CallGraph>) -> HashSet<FunctionId> {
        let mut test_roots = HashSet::new();

        // Find all functions that are test roots (have no callers and are test functions)
        for func_id in call_graph.get_all_functions() {
            let callers = call_graph.get_callers(func_id);
            if callers.is_empty() {
                // Check if this is a test function
                if func_id.name.starts_with("test_")
                    || func_id.name.contains("::test")
                    || func_id.file.to_string_lossy().contains("/tests/")
                    || func_id.file.to_string_lossy().contains("_test.rs")
                {
                    test_roots.insert(func_id.clone());
                }
            }
        }

        test_roots
    }

    pub fn is_test_only(&self, func_id: &FunctionId) -> bool {
        // Check cache first
        if let Ok(cache) = self.reachability_cache.read() {
            if let Some(&result) = cache.get(func_id) {
                return result;
            }
        }

        // If it's a test root, it's test-only
        if self.test_roots.contains(func_id) {
            if let Ok(mut cache) = self.reachability_cache.write() {
                cache.insert(func_id.clone(), true);
            }
            return true;
        }

        // Check if all callers are test-only
        let callers = self.call_graph.get_callers(func_id);
        if callers.is_empty() {
            // No callers and not a test root means it's not test-only
            if let Ok(mut cache) = self.reachability_cache.write() {
                cache.insert(func_id.clone(), false);
            }
            return false;
        }

        // Use BFS to check if reachable from non-test code
        let is_test_only = self.is_reachable_only_from_tests(func_id);

        // Cache the result
        if let Ok(mut cache) = self.reachability_cache.write() {
            cache.insert(func_id.clone(), is_test_only);
        }

        is_test_only
    }

    fn is_reachable_only_from_tests(&self, func_id: &FunctionId) -> bool {
        let mut visited = HashSet::new();
        let mut queue = vec![func_id.clone()];

        while let Some(current) = queue.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }

            let callers = self.call_graph.get_callers(&current);
            if callers.is_empty() {
                // Found a root that's not a test
                if !self.test_roots.contains(&current) {
                    return false;
                }
            } else {
                for caller in callers {
                    if !visited.contains(&caller) {
                        queue.push(caller);
                    }
                }
            }
        }

        true
    }

    pub fn find_all_test_only_functions(&self) -> HashSet<FunctionId> {
        let all_functions: Vec<FunctionId> = self.call_graph.get_all_functions().cloned().collect();

        // Parallel detection of test-only functions
        all_functions
            .par_iter()
            .filter(|func_id| self.is_test_only(func_id))
            .cloned()
            .collect()
    }
}

/// Builder for parallel unified analysis
pub struct ParallelUnifiedAnalysisBuilder {
    call_graph: Arc<CallGraph>,
    options: ParallelUnifiedAnalysisOptions,
    timings: AnalysisPhaseTimings,
    risk_analyzer: Option<crate::risk::RiskAnalyzer>,
    project_path: PathBuf,
    /// Cached line counts from Phase 1 analysis, keyed by file path.
    /// Used to avoid redundant file I/O in Phase 3 (spec 195).
    line_count_index: HashMap<PathBuf, usize>,
    /// Pre-extracted file data from unified extraction phase (spec 213).
    /// When present, avoids re-parsing files during analysis.
    extracted_data: Option<Arc<HashMap<PathBuf, ExtractedFileData>>>,
}

impl ParallelUnifiedAnalysisBuilder {
    pub fn new(call_graph: CallGraph, options: ParallelUnifiedAnalysisOptions) -> Self {
        Self {
            call_graph: Arc::new(call_graph),
            options,
            timings: AnalysisPhaseTimings::default(),
            risk_analyzer: None,
            project_path: PathBuf::from("."),
            line_count_index: HashMap::new(),
            extracted_data: None,
        }
    }

    /// Set pre-extracted file data from unified extraction phase (spec 213).
    ///
    /// When extracted data is provided, the builder uses it to populate data flow
    /// analysis without re-parsing files. This prevents proc-macro2 SourceMap overflow.
    pub fn with_extracted_data(mut self, extracted: HashMap<PathBuf, ExtractedFileData>) -> Self {
        self.extracted_data = Some(Arc::new(extracted));
        self
    }

    /// Set the line count index from Phase 1 FileMetrics (spec 195).
    /// This avoids redundant file I/O in Phase 3 by caching total_lines per file.
    pub fn with_line_count_index(mut self, index: HashMap<PathBuf, usize>) -> Self {
        self.line_count_index = index;
        self
    }

    /// Build a line count index from FileMetrics (spec 195).
    /// Call this before execute_phase3_parallel to enable caching.
    pub fn build_line_count_index(
        file_metrics: &[crate::core::FileMetrics],
    ) -> HashMap<PathBuf, usize> {
        file_metrics
            .iter()
            .filter(|fm| fm.total_lines > 0)
            .map(|fm| (fm.path.clone(), fm.total_lines))
            .collect()
    }

    /// Set the risk analyzer for contextual risk analysis
    pub fn with_risk_analyzer(mut self, risk_analyzer: crate::risk::RiskAnalyzer) -> Self {
        self.risk_analyzer = Some(risk_analyzer);
        self
    }

    /// Set the project path for contextual risk analysis
    pub fn with_project_path(mut self, project_path: PathBuf) -> Self {
        self.project_path = project_path;
        self
    }

    /// Set preliminary timing values (call graph and coverage loading)
    pub fn set_preliminary_timings(
        &mut self,
        call_graph_building: Duration,
        coverage_loading: Duration,
    ) {
        self.timings.call_graph_building = call_graph_building;
        self.timings.trait_resolution = Duration::from_secs(0);
        self.timings.coverage_loading = coverage_loading;
    }

    /// Execute phase 1: Parallel initialization
    pub fn execute_phase1_parallel(
        &mut self,
        metrics: &[FunctionMetrics],
        debt_items: Option<&[crate::core::DebtItem]>,
    ) -> (
        DataFlowGraph,
        HashMap<String, bool>, // purity analysis
        HashSet<FunctionId>,   // test-only functions
        DebtAggregator,
    ) {
        let start = Instant::now();

        // Subtask 0: Aggregate debt (data flow graph, purity, test detection, debt aggregation) - PARALLEL
        if let Some(manager) = ProgressManager::global() {
            manager.tui_update_subtask(5, 0, crate::tui::app::StageStatus::Active, None);
        }

        // Execute parallel initialization tasks
        let (data_flow, purity, test_funcs, debt_agg) =
            self.execute_phase1_tasks(metrics, debt_items);

        let phase1_time = start.elapsed();
        self.report_phase1_completion(phase1_time);

        if let Some(manager) = ProgressManager::global() {
            manager.tui_update_subtask(5, 0, crate::tui::app::StageStatus::Completed, None);
            std::thread::sleep(std::time::Duration::from_millis(150));
        }

        (data_flow, purity, test_funcs, debt_agg)
    }

    /// Execute the 4 parallel initialization tasks
    fn execute_phase1_tasks(
        &mut self,
        metrics: &[FunctionMetrics],
        debt_items: Option<&[crate::core::DebtItem]>,
    ) -> (
        DataFlowGraph,
        HashMap<String, bool>,
        HashSet<FunctionId>,
        DebtAggregator,
    ) {
        // Create shared references for parallel execution
        let call_graph = Arc::clone(&self.call_graph);
        let metrics_arc = Arc::new(metrics.to_vec());
        let debt_items_opt = debt_items.map(|d| d.to_vec());

        // Use thread-safe containers for results
        let data_flow_result = Arc::new(Mutex::new(None));
        let purity_result = Arc::new(Mutex::new(None));
        let test_funcs_result = Arc::new(Mutex::new(None));
        let debt_agg_result = Arc::new(Mutex::new(None));

        let timings = Arc::new(Mutex::new(self.timings.clone()));

        // Suppress old progress spinners - unified system already shows "4/4 Resolving dependencies"
        // These sub-tasks are handled silently by the unified progress system
        let (df_progress, purity_progress, test_progress, debt_progress) = (
            indicatif::ProgressBar::hidden(),
            indicatif::ProgressBar::hidden(),
            indicatif::ProgressBar::hidden(),
            indicatif::ProgressBar::hidden(),
        );

        let df_progress = Arc::new(df_progress);
        let purity_progress = Arc::new(purity_progress);
        let test_progress = Arc::new(test_progress);
        let debt_progress = Arc::new(debt_progress);

        // Execute all 4 initialization steps in parallel
        rayon::scope(|s| {
            // Task 1: Data flow graph creation
            self.spawn_data_flow_task(
                s,
                Arc::clone(&call_graph),
                Arc::clone(&metrics_arc),
                Arc::clone(&data_flow_result),
                Arc::clone(&timings),
                Arc::clone(&df_progress),
            );

            // Task 2: Purity analysis
            self.spawn_purity_task(
                s,
                Arc::clone(&metrics_arc),
                Arc::clone(&purity_result),
                Arc::clone(&timings),
                Arc::clone(&purity_progress),
            );

            // Task 3: Test detection
            self.spawn_test_detection_task(
                s,
                Arc::clone(&call_graph),
                Arc::clone(&test_funcs_result),
                Arc::clone(&timings),
                Arc::clone(&test_progress),
            );

            // Task 4: Debt aggregation
            self.spawn_debt_aggregation_task(
                s,
                Arc::clone(&metrics_arc),
                debt_items_opt,
                Arc::clone(&debt_agg_result),
                Arc::clone(&timings),
                Arc::clone(&debt_progress),
            );
        });

        // Extract results - parking_lot::Mutex never panics on poisoning
        // The tasks always complete before scope exits, so these should be Some
        let data_flow = data_flow_result
            .lock()
            .take()
            .expect("data flow analysis task completed but produced no result");
        let purity = purity_result
            .lock()
            .take()
            .expect("purity analysis task completed but produced no result");
        let test_funcs = test_funcs_result
            .lock()
            .take()
            .expect("test detection task completed but produced no result");
        let debt_agg = debt_agg_result
            .lock()
            .take()
            .expect("debt aggregation task completed but produced no result");

        // Update timings - parking_lot::Mutex::lock() never fails
        let t = timings.lock();
        self.timings = t.clone();

        (data_flow, purity, test_funcs, debt_agg)
    }

    fn spawn_data_flow_task<'a>(
        &self,
        scope: &rayon::Scope<'a>,
        call_graph: Arc<CallGraph>,
        metrics: Arc<Vec<FunctionMetrics>>,
        result: Arc<Mutex<Option<DataFlowGraph>>>,
        timings: Arc<Mutex<AnalysisPhaseTimings>>,
        progress: Arc<indicatif::ProgressBar>,
    ) {
        // Clone extracted data for the spawned task
        let extracted_data = self.extracted_data.clone();

        scope.spawn(move |_| {
            progress.tick();
            let start = Instant::now();
            let mut data_flow = DataFlowGraph::from_call_graph((*call_graph).clone());

            // Spec 214: Use extraction adapters to populate data flow from extracted data
            let (purity_count, mutation_count, io_count, dep_count, trans_count) =
                if let Some(ref extracted) = extracted_data {
                    progress.set_message("Populating from extracted data (spec 214)...");
                    let stats = crate::extraction::adapters::data_flow::populate_data_flow(
                        &mut data_flow,
                        extracted,
                    );
                    (
                        stats.purity_entries,
                        stats.purity_entries, // Mutations counted as part of purity
                        stats.io_operations,
                        stats.variable_deps,
                        stats.transformations,
                    )
                } else {
                    // Fallback: Extract all files first, then populate via adapter
                    progress.set_message("Extracting file data (fallback path)...");

                    // Collect unique file paths from metrics
                    let file_paths: HashSet<PathBuf> = metrics.iter().map(|m| m.file.clone()).collect();

                    // Extract all data from files using the unified extractor
                    let fallback_extracted: HashMap<PathBuf, ExtractedFileData> = file_paths
                        .into_iter()
                        .filter(|p| p.extension().map(|e| e == "rs").unwrap_or(false))
                        .filter_map(|path| {
                            std::fs::read_to_string(&path)
                                .ok()
                                .and_then(|content| {
                                    crate::extraction::UnifiedFileExtractor::extract(&path, &content).ok()
                                })
                                .map(|data| (path, data))
                        })
                        .collect();

                    progress.set_message("Populating from extracted data (fallback)...");
                    let stats = crate::extraction::adapters::data_flow::populate_data_flow(
                        &mut data_flow,
                        &fallback_extracted,
                    );
                    (
                        stats.purity_entries,
                        stats.purity_entries, // Mutations counted as part of purity
                        stats.io_operations,
                        stats.variable_deps,
                        stats.transformations,
                    )
                };

            // Populate purity info from metrics as fallback (matches sequential behavior)
            // This ensures consistent scoring when source files aren't available (e.g., in tests)
            progress.set_message("Populating purity analysis from metrics...");
            for metric in metrics.iter() {
                let func_id = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line);
                let purity_info = crate::data_flow::PurityInfo {
                    is_pure: metric.is_pure.unwrap_or(false),
                    confidence: metric.purity_confidence.unwrap_or(0.0),
                    impurity_reasons: if !metric.is_pure.unwrap_or(false) {
                        vec!["Function may have side effects".to_string()]
                    } else {
                        vec![]
                    },
                };
                data_flow.set_purity_info(func_id, purity_info);
            }

            // parking_lot::Mutex::lock() never fails (no poisoning)
            timings.lock().data_flow_creation = start.elapsed();

            // parking_lot::Mutex::lock() never fails (no poisoning)
            *result.lock() = Some(data_flow);
            progress.finish_with_message(format!(
                "Data flow complete: {} functions, {} mutations, {} I/O ops, {} deps, {} transforms",
                purity_count,
                mutation_count,
                io_count,
                dep_count,
                trans_count
            ));
        });
    }

    fn spawn_purity_task<'a>(
        &self,
        scope: &rayon::Scope<'a>,
        metrics: Arc<Vec<FunctionMetrics>>,
        result: Arc<Mutex<Option<HashMap<String, bool>>>>,
        timings: Arc<Mutex<AnalysisPhaseTimings>>,
        progress: Arc<indicatif::ProgressBar>,
    ) {
        scope.spawn(move |_| {
            progress.tick();
            let start = Instant::now();
            let purity_map = transformations::metrics_to_purity_map(&metrics);
            // parking_lot::Mutex::lock() never fails (no poisoning)
            timings.lock().purity_analysis = start.elapsed();
            *result.lock() = Some(purity_map);
            progress.finish_with_message("Purity analysis complete");
        });
    }

    fn spawn_test_detection_task<'a>(
        &self,
        scope: &rayon::Scope<'a>,
        call_graph: Arc<CallGraph>,
        result: Arc<Mutex<Option<HashSet<FunctionId>>>>,
        timings: Arc<Mutex<AnalysisPhaseTimings>>,
        progress: Arc<indicatif::ProgressBar>,
    ) {
        scope.spawn(move |_| {
            progress.tick();
            let start = Instant::now();
            let detector = OptimizedTestDetector::new(call_graph);
            let test_funcs = detector.find_all_test_only_functions();
            // parking_lot::Mutex::lock() never fails (no poisoning)
            timings.lock().test_detection = start.elapsed();
            *result.lock() = Some(test_funcs);
            progress.finish_with_message("Test detection complete");
        });
    }

    fn spawn_debt_aggregation_task<'a>(
        &self,
        scope: &rayon::Scope<'a>,
        metrics: Arc<Vec<FunctionMetrics>>,
        debt_items: Option<Vec<crate::core::DebtItem>>,
        result: Arc<Mutex<Option<DebtAggregator>>>,
        timings: Arc<Mutex<AnalysisPhaseTimings>>,
        progress: Arc<indicatif::ProgressBar>,
    ) {
        scope.spawn(move |_| {
            progress.tick();
            let start = Instant::now();
            let mut debt_aggregator = DebtAggregator::new();

            if let Some(debt_items) = debt_items {
                let function_mappings = transformations::create_function_mappings(&metrics);
                debt_aggregator.aggregate_debt(debt_items, &function_mappings);
            }

            // parking_lot::Mutex::lock() never fails (no poisoning)
            timings.lock().debt_aggregation = start.elapsed();
            *result.lock() = Some(debt_aggregator);
            progress.finish_with_message("Debt aggregation complete");
        });
    }

    fn report_phase1_completion(&self, phase1_time: Duration) {
        log::debug!(
            "Phase 1 complete in {:?} (DF: {:?}, Purity: {:?}, Test: {:?}, Debt: {:?})",
            phase1_time,
            self.timings.data_flow_creation,
            self.timings.purity_analysis,
            self.timings.test_detection,
            self.timings.debt_aggregation,
        );
    }

    /// Execute phase 2: Parallel function processing using functional pipeline
    #[allow(clippy::too_many_arguments)]
    pub fn execute_phase2_parallel(
        &mut self,
        metrics: &[FunctionMetrics],
        test_only_functions: &HashSet<FunctionId>,
        debt_aggregator: &DebtAggregator,
        data_flow_graph: &DataFlowGraph,
        coverage_data: Option<&LcovData>,
        framework_exclusions: &HashSet<FunctionId>,
        function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    ) -> Vec<UnifiedDebtItem> {
        let start = Instant::now();

        // Subtask 1: Score functions (main computational loop with progress) - PARALLEL
        let total_metrics = metrics.len();
        if let Some(manager) = ProgressManager::global() {
            manager.tui_update_subtask(
                5,
                1,
                crate::tui::app::StageStatus::Active,
                Some((0, total_metrics)),
            );
        }

        // Suppress old progress bar - unified system already shows "4/4 Resolving dependencies"
        let progress: Option<indicatif::ProgressBar> = None;

        // Pre-create shared detectors once to avoid per-metric regex compilation (spec 196)
        // These are Sync types that can be safely shared across threads
        let context_detector = ContextDetector::new();
        let recommendation_engine = ContextRecommendationEngine::new();

        // Create analysis context for the pipeline
        let context = FunctionAnalysisContext {
            call_graph: &self.call_graph,
            debt_aggregator,
            data_flow_graph,
            coverage_data,
            framework_exclusions,
            function_pointer_used_functions,
            risk_analyzer: self.risk_analyzer.as_ref(),
            project_path: &self.project_path,
            context_detector: &context_detector,
            recommendation_engine: &recommendation_engine,
        };

        // Functional pipeline for processing metrics with progress tracking
        let items: Vec<UnifiedDebtItem> = self.process_metrics_pipeline(
            metrics,
            test_only_functions,
            &context,
            progress.as_ref(),
        );

        self.timings.function_analysis = start.elapsed();

        if let Some(manager) = ProgressManager::global() {
            manager.tui_update_subtask(
                5,
                1,
                crate::tui::app::StageStatus::Completed,
                Some((total_metrics, total_metrics)),
            );
            std::thread::sleep(std::time::Duration::from_millis(150));
        }

        // Finish progress bar with completion message
        if let Some(pb) = progress {
            pb.finish_with_message(format!(
                "Function analysis complete ({} items in {:?})",
                items.len(),
                self.timings.function_analysis
            ));
        }

        items
    }

    /// Process metrics through a functional pipeline
    fn process_metrics_pipeline(
        &self,
        metrics: &[FunctionMetrics],
        test_only_functions: &HashSet<FunctionId>,
        context: &FunctionAnalysisContext,
        progress: Option<&indicatif::ProgressBar>,
    ) -> Vec<UnifiedDebtItem> {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let total_metrics = metrics.len();
        let processed_count = AtomicUsize::new(0);
        // Throttle TUI updates (~50-100 total updates)
        let update_interval = (total_metrics / 100).max(1);

        metrics
            .par_iter()
            .progress_with(
                progress
                    .cloned()
                    .unwrap_or_else(indicatif::ProgressBar::hidden),
            )
            .flat_map(|metric| {
                let result = self.process_single_metric(metric, test_only_functions, context);

                // Update TUI progress (throttled)
                let current = processed_count.fetch_add(1, Ordering::Relaxed) + 1;
                if current % update_interval == 0 || current == total_metrics {
                    if let Some(manager) = crate::progress::ProgressManager::global() {
                        manager.tui_update_subtask(
                            5,
                            1,
                            crate::tui::app::StageStatus::Active,
                            Some((current, total_metrics)),
                        );
                    }
                }

                result
            })
            .collect()
    }

    /// Process a single metric through the filtering and transformation pipeline (spec 228)
    fn process_single_metric(
        &self,
        metric: &FunctionMetrics,
        test_only_functions: &HashSet<FunctionId>,
        context: &FunctionAnalysisContext,
    ) -> Vec<UnifiedDebtItem> {
        // Get callee count for triviality check
        let func_id = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line);
        let callee_count = self.call_graph.get_callees(&func_id).len();

        // Apply filtering predicates
        if !predicates::should_process_metric(metric, test_only_functions, callee_count) {
            return Vec::new();
        }

        // Transform metric to debt items (spec 228: returns Vec for multi-debt)
        self.metric_to_debt_items(metric, context)
    }

    /// Transform a metric into debt items (spec 228: multi-debt support)
    fn metric_to_debt_items(
        &self,
        metric: &FunctionMetrics,
        context: &FunctionAnalysisContext,
    ) -> Vec<UnifiedDebtItem> {
        // Clone risk analyzer for thread-safe parallel execution
        let risk_analyzer_clone = context.risk_analyzer.cloned();
        // Returns Vec<UnifiedDebtItem> - one per debt type found (spec 228)
        // Uses shared detectors from context to avoid per-metric regex compilation (spec 196)
        crate::builders::unified_analysis::create_debt_item_from_metric_with_aggregator(
            metric,
            context.call_graph,
            context.coverage_data,
            context.framework_exclusions,
            context.function_pointer_used_functions,
            context.debt_aggregator,
            Some(context.data_flow_graph),
            risk_analyzer_clone.as_ref(),
            context.project_path,
            context.context_detector,
            context.recommendation_engine,
        )
    }

    /// Execute phase 3: Parallel file analysis
    pub fn execute_phase3_parallel(
        &mut self,
        metrics: &[FunctionMetrics],
        coverage_data: Option<&LcovData>,
        no_god_object: bool,
    ) -> Vec<(FileDebtItem, Vec<FunctionMetrics>)> {
        let start = Instant::now();

        // Group functions by file
        let mut files_map: HashMap<PathBuf, Vec<&FunctionMetrics>> = HashMap::new();
        for metric in metrics {
            files_map
                .entry(metric.file.clone())
                .or_default()
                .push(metric);
        }

        let total_files = files_map.len();

        // Initialize TUI progress tracking (design consistency - DESIGN.md:179)
        // Subtask 2: File analysis (stage 5 = debt scoring)
        if let Some(manager) = crate::progress::ProgressManager::global() {
            manager.tui_update_subtask(
                5,
                2,
                crate::tui::app::StageStatus::Active,
                Some((0, total_files)),
            );
        }

        // Shared progress counter for parallel processing
        let processed_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let last_update = std::sync::Arc::new(std::sync::Mutex::new(Instant::now()));

        // Suppress old progress bar - unified system already shows subtask progress
        let progress = indicatif::ProgressBar::hidden();

        // Analyze files in parallel with TUI progress updates
        // Store both file items and raw functions for god object aggregation
        let file_data: Vec<(FileDebtItem, Vec<FunctionMetrics>)> = files_map
            .par_iter()
            .progress_with(progress.clone())
            .filter_map(|(file_path, functions)| {
                let result =
                    self.analyze_file_parallel(file_path, functions, coverage_data, no_god_object);

                // Update progress (throttled to maintain 60 FPS - DESIGN.md:179)
                let current =
                    processed_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;

                if let Ok(mut last) = last_update.try_lock() {
                    if current % 10 == 0 || last.elapsed() > std::time::Duration::from_millis(100) {
                        if let Some(manager) = crate::progress::ProgressManager::global() {
                            manager.tui_update_subtask(
                                5,
                                2,
                                crate::tui::app::StageStatus::Active,
                                Some((current, total_files)),
                            );
                        }
                        *last = Instant::now();
                    }
                }

                // Return both the file item and the raw functions
                result.map(|item| {
                    let raw_functions: Vec<FunctionMetrics> =
                        functions.iter().map(|&f| f.clone()).collect();
                    (item, raw_functions)
                })
            })
            .collect();

        self.timings.file_analysis = start.elapsed();

        progress.finish_and_clear();

        // Mark file analysis subtask complete
        if let Some(manager) = crate::progress::ProgressManager::global() {
            manager.tui_update_subtask(
                5,
                2,
                crate::tui::app::StageStatus::Completed,
                Some((total_files, total_files)),
            );
        }

        file_data
    }

    fn analyze_file_parallel(
        &self,
        file_path: &Path,
        functions: &[&FunctionMetrics],
        coverage_data: Option<&LcovData>,
        no_god_object: bool,
    ) -> Option<FileDebtItem> {
        // Transform to owned functions
        let functions_owned: Vec<FunctionMetrics> = functions.iter().map(|&f| f.clone()).collect();

        // Pure: aggregate function metrics
        let mut file_metrics =
            file_analysis::aggregate_file_metrics(&functions_owned, coverage_data);

        // Spec 195: Use cached line count from Phase 1 if available
        let cached_line_count = self.line_count_index.get(file_path).copied();

        // Early filtering: Skip god object analysis for small files (spec 195)
        // Uses cached line count when available to avoid unnecessary file I/O
        let estimated_lines = cached_line_count.unwrap_or_else(|| {
            // Fallback: estimate from function metrics (less accurate but avoids I/O)
            functions_owned.iter().map(|f| f.length).sum::<usize>()
        });
        let skip_god_object_analysis =
            no_god_object || (estimated_lines < 500 && file_metrics.function_count < 20);

        // Spec 214: Use extracted data when available (avoids file I/O)
        let extracted_file_data = self
            .extracted_data
            .as_ref()
            .and_then(|data| data.get(file_path));

        if let Some(extracted) = extracted_file_data {
            // Use extracted data - no file I/O needed
            file_metrics.total_lines = extracted.total_lines;
            file_metrics.uncovered_lines =
                ((1.0 - file_metrics.coverage_percent) * extracted.total_lines as f64) as usize;

            // Use god object adapter with extracted data
            file_metrics.god_object_analysis = if skip_god_object_analysis {
                None
            } else {
                crate::extraction::adapters::god_object::analyze_god_object(file_path, extracted)
            };
        } else {
            // Fallback: read file content when extracted data not available
            let needs_file_read = cached_line_count.is_none() || !skip_god_object_analysis;

            if needs_file_read {
                if let Ok(content) = std::fs::read_to_string(file_path) {
                    let actual_line_count = content.lines().count();
                    file_metrics.total_lines = actual_line_count;

                    // Recalculate uncovered lines based on actual line count
                    file_metrics.uncovered_lines =
                        ((1.0 - file_metrics.coverage_percent) * actual_line_count as f64) as usize;

                    // Handle god object detection with accurate line count
                    file_metrics.god_object_analysis = if skip_god_object_analysis {
                        None
                    } else {
                        let analysis_result =
                            file_analysis::analyze_god_object(&content, file_path, coverage_data);

                        let analyzed = analysis_result.unwrap_or(None);

                        // Use heuristic fallback if:
                        // 1. Analysis failed (analyzed is None), OR
                        // 2. Analysis succeeded but said not god object, BUT heuristic thresholds are met
                        // This ensures simple god objects (many low-complexity methods) are caught
                        // (Spec 212: Uses shared heuristics from organization::god_object::heuristics)
                        if analyzed.as_ref().is_some_and(|a| a.is_god_object) {
                            // Analysis found a god object, use it
                            analyzed
                        } else {
                            // Try heuristic fallback with preserved analysis data
                            crate::organization::god_object::heuristics::fallback_with_preserved_analysis(
                                file_metrics.function_count,
                                actual_line_count,
                                file_metrics.total_complexity,
                                analyzed.as_ref(),
                            ).or(analyzed)
                        }
                    };
                } else {
                    // Fallback to estimated metrics if file can't be read
                    file_metrics.god_object_analysis = if no_god_object {
                        None
                    } else {
                        self.analyze_god_object_with_io(file_path, coverage_data)
                    };
                }
            } else {
                // Spec 195: Use cached line count - no file I/O needed
                file_metrics.total_lines = cached_line_count.unwrap_or(estimated_lines);
                file_metrics.uncovered_lines = ((1.0 - file_metrics.coverage_percent)
                    * file_metrics.total_lines as f64)
                    as usize;
                file_metrics.god_object_analysis = None; // Small files skip god object analysis
            }
        } // End of else block for extracted data fallback

        // Calculate function scores from items already created in phase 2
        // Note: In parallel execution, items are not yet in unified analysis at this point
        // So function_scores will remain empty, which is consistent with the builder.build() approach
        // where items are added after file analysis. This maintains functional purity.
        file_metrics.function_scores = Vec::new();

        // Detect file context for scoring adjustments (spec 166/168)
        use crate::analysis::FileContextDetector;
        use crate::core::Language;
        let language = Language::from_path(file_path);
        let detector = FileContextDetector::new(language);
        let file_context = detector.detect(file_path, &functions_owned);

        // Create FileDebtItem with context-aware scoring
        let item = crate::priority::FileDebtItem::from_metrics(file_metrics, Some(&file_context));

        // Include file if it meets score threshold OR is a god object (spec 207)
        // God objects should always be included as they represent architectural issues
        let has_god_object = item
            .metrics
            .god_object_analysis
            .as_ref()
            .is_some_and(|analysis| analysis.is_god_object);

        if file_analysis::should_include_file(item.score) || has_god_object {
            Some(item)
        } else {
            None
        }
    }

    /// I/O wrapper for god object analysis
    fn analyze_god_object_with_io(
        &self,
        file_path: &Path,
        coverage_data: Option<&LcovData>,
    ) -> Option<crate::organization::GodObjectAnalysis> {
        let _span = debug_span!("analyze_god_object", path = %file_path.display()).entered();

        // I/O: Read file content
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| {
                warn!(file = %file_path.display(), error = %e, "Failed to read file");
                e
            })
            .ok()?;

        // Pure: Analyze content
        file_analysis::analyze_god_object(&content, file_path, coverage_data)
            .map_err(|e| {
                warn!(file = %file_path.display(), error = %e, "Failed to analyze god object");
                e
            })
            .ok()
            .flatten() // Flatten Option<Option<GodObjectAnalysis>> to Option<GodObjectAnalysis>
    }

    /// Build the final unified analysis from parallel results
    pub fn build(
        mut self,
        data_flow_graph: DataFlowGraph,
        purity_analysis: HashMap<String, bool>,
        items: Vec<UnifiedDebtItem>,
        file_data: Vec<(FileDebtItem, Vec<FunctionMetrics>)>,
        coverage_data: Option<&LcovData>,
    ) -> (UnifiedAnalysis, AnalysisPhaseTimings) {
        let start = Instant::now();

        // Create progress spinner for final aggregation
        let agg_progress = ProgressManager::global()
            .map(|pm| pm.create_spinner("Aggregating analysis results"))
            .unwrap_or_else(indicatif::ProgressBar::hidden);

        let mut unified = UnifiedAnalysis::new((*self.call_graph).clone());
        unified.data_flow_graph = data_flow_graph;

        // Spec 201: Register all analyzed files for accurate total LOC calculation
        // Use LocCounter to get accurate line counts for all files
        use crate::metrics::loc_counter::LocCounter;
        let loc_counter = LocCounter::default();
        for (file_item, _) in &file_data {
            if let Ok(loc_count) = loc_counter.count_file(&file_item.metrics.path) {
                unified.register_analyzed_file(
                    file_item.metrics.path.clone(),
                    loc_count.physical_lines,
                );
            }
        }

        // Add purity information
        for (func_name, is_pure) in purity_analysis {
            if let Some(item) = unified
                .items
                .iter_mut()
                .find(|i| i.location.function == func_name)
            {
                item.is_pure = Some(is_pure);
            }
        }

        // Add all items
        for item in items {
            unified.add_item(item);
        }

        // Add file items and create god object UnifiedDebtItems (spec 207)
        for (file_item, raw_functions) in file_data {
            // Check if this file has god object analysis
            if let Some(ref god_analysis) = file_item.metrics.god_object_analysis {
                if god_analysis.is_god_object {
                    // Aggregate from raw metrics first for complexity (includes ALL functions, even tests)
                    use crate::priority::god_object_aggregation::{
                        aggregate_coverage_from_raw_metrics, aggregate_from_raw_metrics,
                        aggregate_god_object_metrics, extract_member_functions,
                    };

                    let mut aggregated_metrics = aggregate_from_raw_metrics(&raw_functions);

                    // Aggregate coverage from ALL raw functions (not just debt items)
                    // This ensures god objects show accurate coverage even when member
                    // functions are filtered out by complexity thresholds.
                    if let Some(lcov) = coverage_data {
                        aggregated_metrics.weighted_coverage =
                            aggregate_coverage_from_raw_metrics(&raw_functions, lcov);
                    }

                    // Enrich with contextual risk
                    // NOTE: Dependencies are already aggregated from raw metrics (complete architectural view).
                    let member_functions =
                        extract_member_functions(unified.items.iter(), &file_item.metrics.path);
                    if !member_functions.is_empty() {
                        let item_metrics = aggregate_god_object_metrics(&member_functions);

                        // Spec 248: Prefer direct file-level git analysis over member aggregation
                        aggregated_metrics.aggregated_contextual_risk = self
                            .risk_analyzer
                            .as_ref()
                            .and_then(|analyzer| {
                                crate::builders::unified_analysis::analyze_file_git_context(
                                    &file_item.metrics.path,
                                    analyzer,
                                    &self.project_path,
                                )
                            })
                            .or(item_metrics.aggregated_contextual_risk); // Fallback to member aggregation
                    } else {
                        // Spec 248: When no member functions, try direct file analysis
                        aggregated_metrics.aggregated_contextual_risk =
                            self.risk_analyzer.as_ref().and_then(|analyzer| {
                                crate::builders::unified_analysis::analyze_file_git_context(
                                    &file_item.metrics.path,
                                    analyzer,
                                    &self.project_path,
                                )
                            });
                    }
                    // Dependencies are already populated from raw metrics (complete architectural view)

                    // Enrich god_analysis with aggregated entropy and error swallowing data
                    let mut god_analysis = god_analysis.clone();
                    god_analysis.aggregated_entropy = aggregated_metrics.aggregated_entropy.clone();
                    god_analysis.aggregated_error_swallowing_count =
                        if aggregated_metrics.total_error_swallowing_count > 0 {
                            Some(aggregated_metrics.total_error_swallowing_count)
                        } else {
                            None
                        };
                    god_analysis.aggregated_error_swallowing_patterns =
                        if !aggregated_metrics.error_swallowing_patterns.is_empty() {
                            Some(aggregated_metrics.error_swallowing_patterns.clone())
                        } else {
                            None
                        };

                    // Create god object UnifiedDebtItem using same function as sequential path
                    let mut god_item = crate::builders::unified_analysis::create_god_object_debt_item(
                        &file_item.metrics.path,
                        &file_item.metrics,
                        &god_analysis,
                        aggregated_metrics,
                        coverage_data,
                    );

                    // Generate context suggestion for AI agents (spec 263)
                    use crate::priority::context::{generate_context_suggestion, ContextConfig};
                    let context_config = ContextConfig::default();
                    god_item.context_suggestion =
                        generate_context_suggestion(&god_item, &self.call_graph, &context_config);

                    unified.add_item(god_item);
                }
            }

            // Spec 201: Enrich file item with dependency metrics aggregated from function-level data
            let file_item_with_deps = enrich_file_item_with_dependencies(file_item, &unified.items);
            unified.add_file_item(file_item_with_deps);
        }

        agg_progress.set_message("Sorting by priority and calculating impact");

        // Final sorting and impact calculation
        unified.sort_by_priority();
        unified.calculate_total_impact();

        // Set coverage data availability flag (spec 108)
        unified.has_coverage_data = coverage_data.is_some();

        if let Some(lcov) = coverage_data {
            unified.overall_coverage = Some(lcov.get_overall_coverage());
        }

        agg_progress.finish_with_message(format!(
            "Analysis complete ({} function items, {} file items)",
            unified.items.len(),
            unified.file_items.len()
        ));

        self.timings.sorting = start.elapsed();
        self.timings.total = self.timings.call_graph_building
            + self.timings.trait_resolution
            + self.timings.coverage_loading
            + self.timings.data_flow_creation
            + self.timings.purity_analysis
            + self.timings.test_detection
            + self.timings.debt_aggregation
            + self.timings.function_analysis
            + self.timings.file_analysis
            + self.timings.aggregation
            + self.timings.sorting;

        if self.options.progress {
            log::debug!("Total parallel analysis time: {:?}", self.timings.total);
            log::debug!(
                "  - Call graph building: {:?}",
                self.timings.call_graph_building
            );
            log::debug!("  - Trait resolution: {:?}", self.timings.trait_resolution);
            log::debug!("  - Coverage loading: {:?}", self.timings.coverage_loading);
            log::debug!("  - Data flow: {:?}", self.timings.data_flow_creation);
            log::debug!("  - Purity: {:?}", self.timings.purity_analysis);
            log::debug!("  - Test detection: {:?}", self.timings.test_detection);
            log::debug!("  - Debt aggregation: {:?}", self.timings.debt_aggregation);
            log::debug!(
                "  - Function analysis: {:?}",
                self.timings.function_analysis
            );
            log::debug!("  - File analysis: {:?}", self.timings.file_analysis);
            log::debug!("  - Sorting: {:?}", self.timings.sorting);
        }

        (unified, self.timings)
    }
}

/// Enrich file item with dependency metrics aggregated from function-level data (spec 201)
fn enrich_file_item_with_dependencies(
    mut file_item: crate::priority::FileDebtItem,
    unified_items: &im::Vector<crate::priority::UnifiedDebtItem>,
) -> crate::priority::FileDebtItem {
    use crate::priority::god_object_aggregation::{
        aggregate_dependency_metrics, extract_member_functions,
    };

    let member_functions = extract_member_functions(unified_items.iter(), &file_item.metrics.path);
    let (callers, callees, afferent, efferent) = aggregate_dependency_metrics(&member_functions);

    // Update file metrics with aggregated dependency data
    file_item.metrics.afferent_coupling = afferent;
    file_item.metrics.efferent_coupling = efferent;
    file_item.metrics.instability =
        crate::output::unified::calculate_instability(afferent, efferent);
    file_item.metrics.dependents = callers.into_iter().take(10).collect();
    file_item.metrics.dependencies_list = callees.into_iter().take(10).collect();

    file_item
}

/// Trait for parallel analysis
pub trait ParallelAnalyzer {
    fn analyze_parallel(
        &self,
        options: ParallelUnifiedAnalysisOptions,
    ) -> Result<UnifiedAnalysis, anyhow::Error>;
}
