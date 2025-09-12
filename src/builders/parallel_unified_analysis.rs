use crate::{
    analyzers::FileAnalyzer,
    core::FunctionMetrics,
    data_flow::DataFlowGraph,
    priority::{
        call_graph::{CallGraph, FunctionId},
        debt_aggregator::{DebtAggregator, FunctionId as AggregatorFunctionId},
        file_metrics::FileDebtItem,
        UnifiedAnalysis, UnifiedDebtItem,
    },
    risk::lcov::LcovData,
};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

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
                let func_id = AggregatorFunctionId {
                    file: m.file.clone(),
                    name: m.name.clone(),
                    start_line: m.line,
                    end_line: m.line + m.length,
                };
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

    /// Pure predicate for determining if progress should be shown
    pub fn should_show_progress(quiet_mode: bool, progress_enabled: bool) -> bool {
        !quiet_mode && progress_enabled
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

        let func_id = FunctionId {
            file: metric.file.clone(),
            name: metric.name.clone(),
            line: metric.line,
        };

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
    use crate::priority::file_metrics::{FileDebtMetrics, FileImpact, GodObjectIndicators};

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
        }
    }

    /// Pure function to determine if file should be included based on score
    pub fn should_include_file(score: f64) -> bool {
        score > 50.0
    }

    /// Pure function to calculate file impact
    pub fn calculate_file_impact(file_metrics: &FileDebtMetrics, score: f64) -> FileImpact {
        FileImpact {
            complexity_reduction: file_metrics.avg_complexity
                * file_metrics.function_count as f64
                * 0.2,
            maintainability_improvement: score / 10.0,
            test_effort: file_metrics.uncovered_lines as f64 * 0.1,
        }
    }

    /// Pure transformation: create FileDebtItem from metrics
    pub fn create_file_debt_item(file_metrics: FileDebtMetrics, score: f64) -> FileDebtItem {
        let recommendation = file_metrics.generate_recommendation();
        let impact = calculate_file_impact(&file_metrics, score);

        FileDebtItem {
            metrics: file_metrics,
            score,
            priority_rank: 0,
            recommendation,
            impact,
        }
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
}

impl ParallelUnifiedAnalysisBuilder {
    pub fn new(call_graph: CallGraph, options: ParallelUnifiedAnalysisOptions) -> Self {
        // Configure rayon thread pool if specified
        if let Some(jobs) = options.jobs {
            rayon::ThreadPoolBuilder::new()
                .num_threads(jobs)
                .build_global()
                .ok();
        }

        Self {
            call_graph: Arc::new(call_graph),
            options,
            timings: AnalysisPhaseTimings::default(),
        }
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
        let quiet_mode = std::env::var("DEBTMAP_QUIET").is_ok();
        let show_progress =
            transformations::should_show_progress(quiet_mode, self.options.progress);

        if show_progress {
            eprintln!(" ✓");
            eprintln!("🚀 Starting parallel phase 1 (initialization)...");
        }

        // Execute parallel initialization tasks
        let (data_flow, purity, test_funcs, debt_agg) =
            self.execute_phase1_tasks(metrics, debt_items);

        let phase1_time = start.elapsed();
        if show_progress {
            self.report_phase1_completion(phase1_time);
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

        // Execute all 4 initialization steps in parallel
        rayon::scope(|s| {
            // Task 1: Data flow graph creation
            self.spawn_data_flow_task(
                s,
                Arc::clone(&call_graph),
                Arc::clone(&data_flow_result),
                Arc::clone(&timings),
            );

            // Task 2: Purity analysis
            self.spawn_purity_task(
                s,
                Arc::clone(&metrics_arc),
                Arc::clone(&purity_result),
                Arc::clone(&timings),
            );

            // Task 3: Test detection
            self.spawn_test_detection_task(
                s,
                Arc::clone(&call_graph),
                Arc::clone(&test_funcs_result),
                Arc::clone(&timings),
            );

            // Task 4: Debt aggregation
            self.spawn_debt_aggregation_task(
                s,
                Arc::clone(&metrics_arc),
                debt_items_opt,
                Arc::clone(&debt_agg_result),
                Arc::clone(&timings),
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
        result: Arc<Mutex<Option<DataFlowGraph>>>,
        timings: Arc<Mutex<AnalysisPhaseTimings>>,
    ) {
        scope.spawn(move |_| {
            let start = Instant::now();
            let data_flow = DataFlowGraph::from_call_graph((*call_graph).clone());
            if let Ok(mut t) = timings.lock() {
                t.data_flow_creation = start.elapsed();
            }
            if let Ok(mut r) = result.lock() {
                *r = Some(data_flow);
            }
        });
    }

    fn spawn_purity_task<'a>(
        &self,
        scope: &rayon::Scope<'a>,
        metrics: Arc<Vec<FunctionMetrics>>,
        result: Arc<Mutex<Option<HashMap<String, bool>>>>,
        timings: Arc<Mutex<AnalysisPhaseTimings>>,
    ) {
        scope.spawn(move |_| {
            let start = Instant::now();
            let purity_map = transformations::metrics_to_purity_map(&metrics);
            if let Ok(mut t) = timings.lock() {
                t.purity_analysis = start.elapsed();
            }
            if let Ok(mut r) = result.lock() {
                *r = Some(purity_map);
            }
        });
    }

    fn spawn_test_detection_task<'a>(
        &self,
        scope: &rayon::Scope<'a>,
        call_graph: Arc<CallGraph>,
        result: Arc<Mutex<Option<HashSet<FunctionId>>>>,
        timings: Arc<Mutex<AnalysisPhaseTimings>>,
    ) {
        scope.spawn(move |_| {
            let start = Instant::now();
            let detector = OptimizedTestDetector::new(call_graph);
            let test_funcs = detector.find_all_test_only_functions();
            if let Ok(mut t) = timings.lock() {
                t.test_detection = start.elapsed();
            }
            if let Ok(mut r) = result.lock() {
                *r = Some(test_funcs);
            }
        });
    }

    fn spawn_debt_aggregation_task<'a>(
        &self,
        scope: &rayon::Scope<'a>,
        metrics: Arc<Vec<FunctionMetrics>>,
        debt_items: Option<Vec<crate::core::DebtItem>>,
        result: Arc<Mutex<Option<DebtAggregator>>>,
        timings: Arc<Mutex<AnalysisPhaseTimings>>,
    ) {
        scope.spawn(move |_| {
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
        });
    }

    fn report_phase1_completion(&self, phase1_time: Duration) {
        eprintln!(
            "✅ Phase 1 complete in {:?} (DF: {:?}, Purity: {:?}, Test: {:?}, Debt: {:?})",
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
        let quiet_mode = std::env::var("DEBTMAP_QUIET").is_ok();
        let show_progress =
            transformations::should_show_progress(quiet_mode, self.options.progress);

        if show_progress {
            eprintln!("🚀 Starting parallel phase 2 (function analysis)...");
        }

        // Create analysis context for the pipeline
        let context = FunctionAnalysisContext {
            call_graph: &self.call_graph,
            debt_aggregator,
            data_flow_graph,
            coverage_data,
            framework_exclusions,
            function_pointer_used_functions,
        };

        // Functional pipeline for processing metrics
        let items: Vec<UnifiedDebtItem> =
            self.process_metrics_pipeline(metrics, test_only_functions, &context);

        self.timings.function_analysis = start.elapsed();

        if show_progress {
            eprintln!(
                "✅ Phase 2 complete in {:?} ({} items processed)",
                self.timings.function_analysis,
                items.len()
            );
        }

        items
    }

    /// Process metrics through a functional pipeline
    fn process_metrics_pipeline(
        &self,
        metrics: &[FunctionMetrics],
        test_only_functions: &HashSet<FunctionId>,
        context: &FunctionAnalysisContext,
    ) -> Vec<UnifiedDebtItem> {
        metrics
            .par_iter()
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
        let func_id = FunctionId {
            file: metric.file.clone(),
            name: metric.name.clone(),
            line: metric.line,
        };
        let callee_count = self.call_graph.get_callees(&func_id).len();

        // Apply filtering predicates
        if !predicates::should_process_metric(metric, test_only_functions, callee_count) {
            return None;
        }

        // Transform metric to debt item
        Some(self.metric_to_debt_item(metric, context))
    }

    /// Transform a metric into a debt item (pure transformation)
    fn metric_to_debt_item(
        &self,
        metric: &FunctionMetrics,
        context: &FunctionAnalysisContext,
    ) -> UnifiedDebtItem {
        crate::builders::unified_analysis::create_debt_item_from_metric_with_aggregator(
            metric,
            context.call_graph,
            context.coverage_data,
            context.framework_exclusions,
            context.function_pointer_used_functions,
            context.debt_aggregator,
            Some(context.data_flow_graph),
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
        let quiet_mode = std::env::var("DEBTMAP_QUIET").is_ok();

        if !quiet_mode && self.options.progress {
            eprintln!("🚀 Starting parallel phase 3 (file analysis)...");
        }

        // Group functions by file
        let mut files_map: HashMap<PathBuf, Vec<&FunctionMetrics>> = HashMap::new();
        for metric in metrics {
            files_map
                .entry(metric.file.clone())
                .or_default()
                .push(metric);
        }

        // Analyze files in parallel
        let file_items: Vec<FileDebtItem> = files_map
            .par_iter()
            .filter_map(|(file_path, functions)| {
                self.analyze_file_parallel(file_path, functions, coverage_data, no_god_object)
            })
            .collect();

        self.timings.file_analysis = start.elapsed();

        if !quiet_mode && self.options.progress {
            eprintln!(
                "✅ Phase 3 complete in {:?} ({} file items)",
                self.timings.file_analysis,
                file_items.len()
            );
        }

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

        // Pure: calculate score and decide if to include
        let score = file_metrics.calculate_score();

        if file_analysis::should_include_file(score) {
            Some(file_analysis::create_file_debt_item(file_metrics, score))
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
        let content = std::fs::read_to_string(file_path).ok()?;

        // Pure: Analyze content
        file_analysis::analyze_god_object_indicators(&content, file_path, coverage_data).ok()
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
        let quiet_mode = std::env::var("DEBTMAP_QUIET").is_ok();

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

        // Add file items
        for file_item in file_items {
            unified.add_file_item(file_item);
        }

        // Final sorting and impact calculation
        unified.sort_by_priority();
        unified.calculate_total_impact();

        if let Some(lcov) = coverage_data {
            unified.overall_coverage = Some(lcov.get_overall_coverage());
        }

        self.timings.sorting = start.elapsed();
        self.timings.total = self.timings.data_flow_creation
            + self.timings.purity_analysis
            + self.timings.test_detection
            + self.timings.debt_aggregation
            + self.timings.function_analysis
            + self.timings.file_analysis
            + self.timings.aggregation
            + self.timings.sorting;

        if !quiet_mode && self.options.progress {
            eprintln!("⏱️  Total parallel analysis time: {:?}", self.timings.total);
            eprintln!("  - Data flow: {:?}", self.timings.data_flow_creation);
            eprintln!("  - Purity: {:?}", self.timings.purity_analysis);
            eprintln!("  - Test detection: {:?}", self.timings.test_detection);
            eprintln!("  - Debt aggregation: {:?}", self.timings.debt_aggregation);
            eprintln!(
                "  - Function analysis: {:?}",
                self.timings.function_analysis
            );
            eprintln!("  - File analysis: {:?}", self.timings.file_analysis);
            eprintln!("  - Sorting: {:?}", self.timings.sorting);
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
