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
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

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

        if !quiet_mode && self.options.progress {
            eprintln!("  🚀 Starting parallel phase 1 (initialization)...");
        }

        // Create shared references for parallel execution
        let call_graph = Arc::clone(&self.call_graph);
        let metrics = Arc::new(metrics.to_vec());
        let debt_items_opt = debt_items.map(|d| d.to_vec());

        // Use thread-safe containers for results
        let data_flow_result = Arc::new(Mutex::new(None));
        let purity_result = Arc::new(Mutex::new(None));
        let test_funcs_result = Arc::new(Mutex::new(None));
        let debt_agg_result = Arc::new(Mutex::new(None));

        let timings = Arc::new(Mutex::new(self.timings.clone()));

        // Clone all Arcs upfront for parallel tasks
        let timings1 = Arc::clone(&timings);
        let timings2 = Arc::clone(&timings);
        let timings3 = Arc::clone(&timings);
        let timings4 = Arc::clone(&timings);

        // Execute all 4 initialization steps in parallel using scoped threads
        rayon::scope(|s| {
            let data_flow_result = Arc::clone(&data_flow_result);
            let call_graph_clone = Arc::clone(&call_graph);
            s.spawn(move |_| {
                let start = Instant::now();
                let result = DataFlowGraph::from_call_graph((*call_graph_clone).clone());
                if let Ok(mut t) = timings1.lock() {
                    t.data_flow_creation = start.elapsed();
                }
                if let Ok(mut r) = data_flow_result.lock() {
                    *r = Some(result);
                }
            });

            let purity_result = Arc::clone(&purity_result);
            let metrics_clone = Arc::clone(&metrics);
            s.spawn(move |_| {
                let start = Instant::now();
                let result: HashMap<String, bool> = metrics_clone
                    .par_iter()
                    .map(|m| (m.name.clone(), m.is_pure.unwrap_or(false)))
                    .collect();
                if let Ok(mut t) = timings2.lock() {
                    t.purity_analysis = start.elapsed();
                }
                if let Ok(mut r) = purity_result.lock() {
                    *r = Some(result);
                }
            });

            let test_funcs_result = Arc::clone(&test_funcs_result);
            let call_graph_clone = Arc::clone(&call_graph);
            s.spawn(move |_| {
                let start = Instant::now();
                let detector = OptimizedTestDetector::new(call_graph_clone);
                let result = detector.find_all_test_only_functions();
                if let Ok(mut t) = timings3.lock() {
                    t.test_detection = start.elapsed();
                }
                if let Ok(mut r) = test_funcs_result.lock() {
                    *r = Some(result);
                }
            });

            let debt_agg_result = Arc::clone(&debt_agg_result);
            let metrics_clone = Arc::clone(&metrics);
            let debt_items_clone = debt_items_opt.clone();
            s.spawn(move |_| {
                let start = Instant::now();
                let mut debt_aggregator = DebtAggregator::new();

                if let Some(debt_items) = debt_items_clone {
                    let function_mappings: Vec<(AggregatorFunctionId, usize, usize)> =
                        metrics_clone
                            .par_iter()
                            .map(|m| {
                                let func_id = AggregatorFunctionId {
                                    file: m.file.clone(),
                                    name: m.name.clone(),
                                    start_line: m.line,
                                    end_line: m.line + m.length,
                                };
                                (func_id, m.line, m.line + m.length)
                            })
                            .collect();

                    debt_aggregator.aggregate_debt(debt_items, &function_mappings);
                }

                if let Ok(mut t) = timings4.lock() {
                    t.debt_aggregation = start.elapsed();
                }
                if let Ok(mut r) = debt_agg_result.lock() {
                    *r = Some(debt_aggregator);
                }
            });
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

        let phase1_time = start.elapsed();
        if !quiet_mode && self.options.progress {
            eprintln!(
                "  ✅ Phase 1 complete in {:?} (DF: {:?}, Purity: {:?}, Test: {:?}, Debt: {:?})",
                phase1_time,
                self.timings.data_flow_creation,
                self.timings.purity_analysis,
                self.timings.test_detection,
                self.timings.debt_aggregation,
            );
        }

        (data_flow, purity, test_funcs, debt_agg)
    }

    /// Execute phase 2: Parallel function processing
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

        if !quiet_mode && self.options.progress {
            eprintln!("  🚀 Starting parallel phase 2 (function analysis)...");
        }

        // Process functions in parallel batches
        let items: Vec<UnifiedDebtItem> = metrics
            .par_iter()
            .filter_map(|metric| {
                if self.should_skip_metric(metric, test_only_functions) {
                    None
                } else {
                    Some(self.create_debt_item_parallel(
                        metric,
                        debt_aggregator,
                        data_flow_graph,
                        coverage_data,
                        framework_exclusions,
                        function_pointer_used_functions,
                    ))
                }
            })
            .collect();

        self.timings.function_analysis = start.elapsed();

        if !quiet_mode && self.options.progress {
            eprintln!(
                "  ✅ Phase 2 complete in {:?} ({} items processed)",
                self.timings.function_analysis,
                items.len()
            );
        }

        items
    }

    fn should_skip_metric(
        &self,
        metric: &FunctionMetrics,
        test_only_functions: &HashSet<FunctionId>,
    ) -> bool {
        if metric.is_test || metric.in_test_module {
            return true;
        }

        if metric.name.contains("<closure@") {
            return true;
        }

        let func_id = FunctionId {
            file: metric.file.clone(),
            name: metric.name.clone(),
            line: metric.line,
        };

        if test_only_functions.contains(&func_id) {
            return true;
        }

        // Skip trivial functions
        if metric.cyclomatic == 1 && metric.cognitive == 0 && metric.length <= 3 {
            let callees = self.call_graph.get_callees(&func_id);
            if callees.len() == 1 {
                return true;
            }
        }

        false
    }

    fn create_debt_item_parallel(
        &self,
        metric: &FunctionMetrics,
        debt_aggregator: &DebtAggregator,
        data_flow_graph: &DataFlowGraph,
        coverage_data: Option<&LcovData>,
        framework_exclusions: &HashSet<FunctionId>,
        function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    ) -> UnifiedDebtItem {
        // Use the existing function from the original implementation
        crate::builders::unified_analysis::create_debt_item_from_metric_with_aggregator(
            metric,
            &self.call_graph,
            coverage_data,
            framework_exclusions,
            function_pointer_used_functions,
            debt_aggregator,
            Some(data_flow_graph),
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
            eprintln!("  🚀 Starting parallel phase 3 (file analysis)...");
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
                "  ✅ Phase 3 complete in {:?} ({} file items)",
                self.timings.file_analysis,
                file_items.len()
            );
        }

        file_items
    }

    fn analyze_file_parallel(
        &self,
        file_path: &PathBuf,
        functions: &[&FunctionMetrics],
        coverage_data: Option<&LcovData>,
        no_god_object: bool,
    ) -> Option<FileDebtItem> {
        use crate::analyzers::file_analyzer::UnifiedFileAnalyzer;
        use crate::priority::file_metrics::FileImpact;

        let functions_owned: Vec<FunctionMetrics> = functions.iter().map(|&f| f.clone()).collect();
        let file_analyzer = UnifiedFileAnalyzer::new(coverage_data.cloned());

        let mut file_metrics = file_analyzer.aggregate_functions(&functions_owned);

        // Run god object detection if enabled
        if !no_god_object {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                if let Ok(analyzed) = file_analyzer.analyze_file(file_path, &content) {
                    file_metrics.god_object_indicators = analyzed.god_object_indicators;
                }
            }
        } else {
            file_metrics.god_object_indicators =
                crate::priority::file_metrics::GodObjectIndicators {
                    methods_count: 0,
                    fields_count: 0,
                    responsibilities: 0,
                    is_god_object: false,
                    god_object_score: 0.0,
                };
        }

        let score = file_metrics.calculate_score();

        // Only add file items with significant scores
        if score > 50.0 {
            let recommendation = file_metrics.generate_recommendation();

            Some(FileDebtItem {
                metrics: file_metrics.clone(),
                score,
                priority_rank: 0,
                recommendation,
                impact: FileImpact {
                    complexity_reduction: file_metrics.avg_complexity
                        * file_metrics.function_count as f64
                        * 0.2,
                    maintainability_improvement: score / 10.0,
                    test_effort: file_metrics.uncovered_lines as f64 * 0.1,
                },
            })
        } else {
            None
        }
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
            eprintln!(
                "  ⏱️  Total parallel analysis time: {:?}",
                self.timings.total
            );
            eprintln!("    - Data flow: {:?}", self.timings.data_flow_creation);
            eprintln!("    - Purity: {:?}", self.timings.purity_analysis);
            eprintln!("    - Test detection: {:?}", self.timings.test_detection);
            eprintln!(
                "    - Debt aggregation: {:?}",
                self.timings.debt_aggregation
            );
            eprintln!(
                "    - Function analysis: {:?}",
                self.timings.function_analysis
            );
            eprintln!("    - File analysis: {:?}", self.timings.file_analysis);
            eprintln!("    - Sorting: {:?}", self.timings.sorting);
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
