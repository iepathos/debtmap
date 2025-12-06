use crate::{
    analyzers::FileAnalyzer,
    core::FunctionMetrics,
    data_flow::DataFlowGraph,
    organization::god_object::{DetectionType, GodObjectAnalysis, GodObjectConfidence},
    priority::{
        call_graph::{CallGraph, FunctionId},
        debt_aggregator::{DebtAggregator, FunctionId as AggregatorFunctionId},
        file_metrics::{FileDebtItem, FileDebtMetrics},
        UnifiedAnalysis, UnifiedAnalysisUtils, UnifiedDebtItem,
    },
    progress::ProgressManager,
    risk::lcov::LcovData,
};
use indicatif::ParallelProgressIterator;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

/// Convert GodObjectIndicators to GodObjectAnalysis for god object debt item creation (spec 207)
fn convert_indicators_to_analysis(file_metrics: &FileDebtMetrics) -> GodObjectAnalysis {
    let indicators = &file_metrics.god_object_indicators;

    GodObjectAnalysis {
        is_god_object: indicators.is_god_object,
        method_count: indicators.methods_count,
        field_count: indicators.fields_count,
        responsibility_count: indicators.responsibilities,
        lines_of_code: file_metrics.total_lines,
        complexity_sum: file_metrics.total_complexity,
        god_object_score: indicators.god_object_score,
        recommended_splits: Vec::new(), // Type mismatch - will be populated from file indicators
        confidence: GodObjectConfidence::Definite,
        responsibilities: indicators.responsibility_names.clone(),
        purity_distribution: None,
        module_structure: None, // Type mismatch - using file-level data instead
        detection_type: indicators.detection_type.clone().unwrap_or(DetectionType::GodFile),
        visibility_breakdown: None,
        domain_count: indicators.domain_count,
        domain_diversity: indicators.domain_diversity,
        struct_ratio: indicators.struct_ratio,
        analysis_method: Default::default(), // Type mismatch - using default
        cross_domain_severity: None, // Type mismatch - using None
        domain_diversity_metrics: None, // Type mismatch - using None
    }
}

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

    /// Extract full purity analysis for functions by re-analyzing
    /// This is necessary because FunctionMetrics only stores boolean purity, not full CFG analysis
    pub fn extract_purity_analysis(
        metrics: &[FunctionMetrics],
    ) -> HashMap<FunctionId, crate::analyzers::purity_detector::PurityAnalysis> {
        use crate::analyzers::purity_detector::PurityDetector;
        use std::fs;

        metrics
            .par_iter()
            .filter_map(|m| {
                // Read file and parse to get AST
                let content = fs::read_to_string(&m.file)
                    .map_err(|e| {
                        eprintln!("Warning: Failed to read file {}: {}", m.file.display(), e);
                        e
                    })
                    .ok()?;
                let file_ast = syn::parse_file(&content).ok()?;

                // Find the function in the AST by name and line number
                for item in &file_ast.items {
                    if let syn::Item::Fn(item_fn) = item {
                        if let Some(ident_span) =
                            item_fn.sig.ident.span().start().line.checked_sub(1)
                        {
                            if ident_span == m.line {
                                // Run purity analysis
                                let mut detector = PurityDetector::new();
                                let analysis = detector.is_pure_function(item_fn);
                                let func_id =
                                    FunctionId::new(m.file.clone(), m.name.clone(), m.line);
                                return Some((func_id, analysis));
                            }
                        }
                    }
                }
                None
            })
            .collect()
    }
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
    use crate::priority::file_metrics::{FileDebtMetrics, GodObjectIndicators};

    /// Pure function to aggregate function metrics into file metrics
    pub fn aggregate_file_metrics(
        functions: &[FunctionMetrics],
        coverage_data: Option<&LcovData>,
    ) -> FileDebtMetrics {
        let file_analyzer = UnifiedFileAnalyzer::new(coverage_data.cloned());
        file_analyzer.aggregate_functions(functions)
    }

    /// Pure function to analyze god object indicators from file content
    pub fn analyze_god_object_indicators(
        content: &str,
        file_path: &Path,
        coverage_data: Option<&LcovData>,
    ) -> Result<GodObjectIndicators, String> {
        let file_analyzer = UnifiedFileAnalyzer::new(coverage_data.cloned());
        file_analyzer
            .analyze_file(file_path, content)
            .map(|analyzed| analyzed.god_object_indicators)
            .map_err(|e| format!("Failed to analyze god object: {}", e))
    }

    /// Pure function to create default god object indicators
    pub fn default_god_object_indicators() -> GodObjectIndicators {
        GodObjectIndicators {
            methods_count: 0,
            fields_count: 0,
            responsibilities: 0,
            is_god_object: false,
            god_object_score: 0.0,
            responsibility_names: Vec::new(),
            recommended_splits: Vec::new(),
            module_structure: None,
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: crate::priority::file_metrics::SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
            detection_type: None,
        }
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
}

impl ParallelUnifiedAnalysisBuilder {
    pub fn new(call_graph: CallGraph, options: ParallelUnifiedAnalysisOptions) -> Self {
        Self {
            call_graph: Arc::new(call_graph),
            options,
            timings: AnalysisPhaseTimings::default(),
            risk_analyzer: None,
            project_path: PathBuf::from("."),
        }
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

        // Subtask 0: Initialize (data flow graph, purity, test detection) - PARALLEL
        if let Some(manager) = ProgressManager::global() {
            manager.tui_update_subtask(6, 0, crate::tui::app::StageStatus::Active, None);
        }

        // Execute parallel initialization tasks
        let (data_flow, purity, test_funcs, debt_agg) =
            self.execute_phase1_tasks(metrics, debt_items);

        let phase1_time = start.elapsed();
        self.report_phase1_completion(phase1_time);

        if let Some(manager) = ProgressManager::global() {
            manager.tui_update_subtask(6, 0, crate::tui::app::StageStatus::Completed, None);
            std::thread::sleep(std::time::Duration::from_millis(150));
        }

        // Subtask 1: Aggregate debt (included in phase 1)
        if let Some(manager) = ProgressManager::global() {
            manager.tui_update_subtask(6, 1, crate::tui::app::StageStatus::Active, None);
        }
        if let Some(manager) = ProgressManager::global() {
            manager.tui_update_subtask(6, 1, crate::tui::app::StageStatus::Completed, None);
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

        // Extract results
        let data_flow = data_flow_result.lock().unwrap().take().unwrap();
        let purity = purity_result.lock().unwrap().take().unwrap();
        let test_funcs = test_funcs_result.lock().unwrap().take().unwrap();
        let debt_agg = debt_agg_result.lock().unwrap().take().unwrap();

        // Update timings
        if let Ok(t) = timings.lock() {
            self.timings = t.clone();
        }

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
        scope.spawn(move |_| {
            progress.tick();
            let start = Instant::now();
            let mut data_flow = DataFlowGraph::from_call_graph((*call_graph).clone());

            // Extract full purity analysis to populate CFG data
            progress.set_message("Building data flow graph...");
            let purity_results = transformations::extract_purity_analysis(&metrics);

            progress.set_message("Analyzing mutations and escape analysis...");
            for (func_id, purity) in &purity_results {
                crate::data_flow::population::populate_from_purity_analysis(
                    &mut data_flow,
                    func_id,
                    purity,
                );
            }

            // Populate I/O operations from metrics
            progress.set_message("Detecting I/O operations...");
            let io_count =
                crate::data_flow::population::populate_io_operations(&mut data_flow, &metrics);

            // Populate variable dependencies
            progress.set_message("Analyzing variable dependencies...");
            let dep_count = crate::data_flow::population::populate_variable_dependencies(
                &mut data_flow,
                &metrics,
            );

            // Populate data transformations
            progress.set_message("Detecting data transformations...");
            let trans_count = crate::data_flow::population::populate_data_transformations(
                &mut data_flow,
                &metrics,
            );

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

            if let Ok(mut t) = timings.lock() {
                t.data_flow_creation = start.elapsed();
            }

            // Count mutations from purity results
            let mutation_count: usize = purity_results
                .values()
                .map(|p| p.total_mutations)
                .sum();

            if let Ok(mut r) = result.lock() {
                *r = Some(data_flow);
            }
            progress.finish_with_message(format!(
                "Data flow complete: {} functions, {} mutations, {} I/O ops, {} deps, {} transforms",
                purity_results.len(),
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
            if let Ok(mut t) = timings.lock() {
                t.purity_analysis = start.elapsed();
            }
            if let Ok(mut r) = result.lock() {
                *r = Some(purity_map);
            }
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
            if let Ok(mut t) = timings.lock() {
                t.test_detection = start.elapsed();
            }
            if let Ok(mut r) = result.lock() {
                *r = Some(test_funcs);
            }
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

            if let Ok(mut t) = timings.lock() {
                t.debt_aggregation = start.elapsed();
            }
            if let Ok(mut r) = result.lock() {
                *r = Some(debt_aggregator);
            }
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

        // Subtask 2: Score functions (main computational loop with progress) - PARALLEL
        let total_metrics = metrics.len();
        if let Some(manager) = ProgressManager::global() {
            manager.tui_update_subtask(
                6,
                2,
                crate::tui::app::StageStatus::Active,
                Some((0, total_metrics)),
            );
        }

        // Suppress old progress bar - unified system already shows "4/4 Resolving dependencies"
        let progress: Option<indicatif::ProgressBar> = None;

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
                6,
                2,
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
        metrics
            .par_iter()
            .progress_with(
                progress
                    .cloned()
                    .unwrap_or_else(indicatif::ProgressBar::hidden),
            )
            .filter_map(|metric| self.process_single_metric(metric, test_only_functions, context))
            .collect()
    }

    /// Process a single metric through the filtering and transformation pipeline
    fn process_single_metric(
        &self,
        metric: &FunctionMetrics,
        test_only_functions: &HashSet<FunctionId>,
        context: &FunctionAnalysisContext,
    ) -> Option<UnifiedDebtItem> {
        // Get callee count for triviality check
        let func_id = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line);
        let callee_count = self.call_graph.get_callees(&func_id).len();

        // Apply filtering predicates
        if !predicates::should_process_metric(metric, test_only_functions, callee_count) {
            return None;
        }

        // Transform metric to debt item (spec 201: may return None for clean dispatchers)
        self.metric_to_debt_item(metric, context)
    }

    /// Transform a metric into a debt item (pure transformation)
    fn metric_to_debt_item(
        &self,
        metric: &FunctionMetrics,
        context: &FunctionAnalysisContext,
    ) -> Option<UnifiedDebtItem> {
        // Clone risk analyzer for thread-safe parallel execution
        let risk_analyzer_clone = context.risk_analyzer.cloned();
        // Returns None for clean dispatchers (spec 201)
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
        )
    }

    /// Execute phase 3: Parallel file analysis
    pub fn execute_phase3_parallel(
        &mut self,
        metrics: &[FunctionMetrics],
        coverage_data: Option<&LcovData>,
        no_god_object: bool,
    ) -> Vec<FileDebtItem> {
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
        if let Some(manager) = crate::progress::ProgressManager::global() {
            manager.tui_update_subtask(
                6,
                3,
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
        let file_items: Vec<FileDebtItem> = files_map
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
                                6,
                                3,
                                crate::tui::app::StageStatus::Active,
                                Some((current, total_files)),
                            );
                        }
                        *last = Instant::now();
                    }
                }

                result
            })
            .collect();

        self.timings.file_analysis = start.elapsed();

        progress.finish_and_clear();

        file_items
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

        // Read file content to get accurate line count
        if let Ok(content) = std::fs::read_to_string(file_path) {
            let actual_line_count = content.lines().count();
            file_metrics.total_lines = actual_line_count;

            // Recalculate uncovered lines based on actual line count
            file_metrics.uncovered_lines =
                ((1.0 - file_metrics.coverage_percent) * actual_line_count as f64) as usize;

            // Handle god object detection with accurate line count
            file_metrics.god_object_indicators = if no_god_object {
                file_analysis::default_god_object_indicators()
            } else {
                file_analysis::analyze_god_object_indicators(&content, file_path, coverage_data)
                    .unwrap_or_else(|_| {
                        let mut indicators = file_analysis::default_god_object_indicators();
                        // Update based on actual line count
                        if actual_line_count > 2000 || file_metrics.function_count > 50 {
                            indicators.is_god_object = true;
                            indicators.god_object_score =
                                (file_metrics.function_count as f64 / 50.0).min(2.0);
                            indicators.methods_count = file_metrics.function_count;
                        }
                        indicators
                    })
            };
        } else {
            // Fallback to estimated metrics if file can't be read
            file_metrics.god_object_indicators = if no_god_object {
                file_analysis::default_god_object_indicators()
            } else {
                self.analyze_god_object_with_io(file_path, coverage_data)
                    .unwrap_or_else(file_analysis::default_god_object_indicators)
            };
        }

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

        if file_analysis::should_include_file(item.score) {
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
    ) -> Option<crate::priority::file_metrics::GodObjectIndicators> {
        // I/O: Read file content
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| {
                eprintln!(
                    "Warning: Failed to read file {}: {}",
                    file_path.display(),
                    e
                );
                e
            })
            .ok()?;

        // Pure: Analyze content
        file_analysis::analyze_god_object_indicators(&content, file_path, coverage_data)
            .map_err(|e| {
                eprintln!(
                    "Warning: Failed to analyze god object indicators for {}: {}",
                    file_path.display(),
                    e
                );
                e
            })
            .ok()
    }

    /// Build the final unified analysis from parallel results
    pub fn build(
        mut self,
        data_flow_graph: DataFlowGraph,
        purity_analysis: HashMap<String, bool>,
        items: Vec<UnifiedDebtItem>,
        file_items: Vec<FileDebtItem>,
        coverage_data: Option<&LcovData>,
    ) -> (UnifiedAnalysis, AnalysisPhaseTimings) {
        let start = Instant::now();

        // Create progress spinner for final aggregation
        let agg_progress = ProgressManager::global()
            .map(|pm| pm.create_spinner("Aggregating analysis results"))
            .unwrap_or_else(indicatif::ProgressBar::hidden);

        let mut unified = UnifiedAnalysis::new((*self.call_graph).clone());
        unified.data_flow_graph = data_flow_graph;

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
        for file_item in file_items {
            // Check if this file has god object indicators
            if file_item.metrics.god_object_indicators.is_god_object {
                // Convert GodObjectIndicators to GodObjectAnalysis for god item creation
                let god_analysis = convert_indicators_to_analysis(&file_item.metrics);

                // Create god object UnifiedDebtItem using same function as sequential path
                let god_item = crate::builders::unified_analysis::create_god_object_debt_item(
                    &file_item.metrics.path,
                    &file_item.metrics,
                    &god_analysis,
                );
                unified.add_item(god_item);
            }

            unified.add_file_item(file_item);
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

/// Trait for parallel analysis
pub trait ParallelAnalyzer {
    fn analyze_parallel(
        &self,
        options: ParallelUnifiedAnalysisOptions,
    ) -> Result<UnifiedAnalysis, anyhow::Error>;
}
