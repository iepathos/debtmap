use crate::debt::suppression_audit::SuppressionAudit;
use crate::{
    builders::unified_analysis_phases::phases::scoring::{
        PreparedScoringInput, ScoringExecution, SuppressionContextCache,
        score_metrics_with_policy_audited,
    },
    config::AnalysisPolicy,
    core::FunctionMetrics,
    data_flow::DataFlowGraph,
    extraction::ExtractedFileData,
    priority::{
        UnifiedAnalysis, UnifiedAnalysisUtils, UnifiedDebtItem,
        call_graph::{CallGraph, FunctionId},
        debt_aggregator::DebtAggregator,
        file_metrics::FileDebtItem,
    },
    progress::ProgressManager,
    risk::lcov::LcovData,
    time_span,
};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use indicatif::ParallelProgressIterator;
use parking_lot::Mutex;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn clone_function_metrics(functions: &[&FunctionMetrics]) -> Vec<FunctionMetrics> {
    functions.iter().map(|&function| function.clone()).collect()
}

fn should_emit_file_item(item: &FileDebtItem) -> bool {
    let has_god_object = item
        .metrics
        .god_object_analysis
        .as_ref()
        .is_some_and(|analysis| analysis.is_god_object);

    crate::builders::unified_analysis_phases::phases::file_analysis::should_include_file(item.score)
        || has_god_object
}

/// Options for parallel unified analysis
#[derive(Debug, Clone)]
pub struct ParallelUnifiedAnalysisOptions {
    pub parallel: bool,
    pub jobs: Option<usize>,
    pub batch_size: usize,
    pub progress: bool,
    /// Reference time for analysis (for determinism)
    pub reference_time: DateTime<Utc>,
}

impl Default for ParallelUnifiedAnalysisOptions {
    fn default() -> Self {
        Self {
            parallel: true,
            jobs: None,
            batch_size: 100,
            progress: true,
            reference_time: Utc::now(),
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
    pub prepare_scoring: Duration,
    pub score_functions: Duration,
    pub function_analysis: Duration,
    pub analyze_files: Duration,
    pub file_analysis: Duration,
    pub finalize_files: Duration,
    pub calculate_impact: Duration,
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
            prepare_scoring: Duration::from_secs(0),
            score_functions: Duration::from_secs(0),
            function_analysis: Duration::from_secs(0),
            analyze_files: Duration::from_secs(0),
            file_analysis: Duration::from_secs(0),
            finalize_files: Duration::from_secs(0),
            calculate_impact: Duration::from_secs(0),
            aggregation: Duration::from_secs(0),
            sorting: Duration::from_secs(0),
            total: Duration::from_secs(0),
        }
    }
}

/// Optimized test detector with lock-free caching
///
/// Uses DashMap for concurrent cache access without lock contention.
/// This improves parallel scoring performance by 5-10% on large codebases.
pub struct OptimizedTestDetector {
    call_graph: Arc<CallGraph>,
    test_roots: HashSet<FunctionId>,
    reachability_cache: DashMap<FunctionId, bool>,
}

impl OptimizedTestDetector {
    pub fn new(call_graph: Arc<CallGraph>) -> Self {
        let test_roots = Self::find_test_roots(&call_graph);
        Self {
            call_graph,
            test_roots,
            reachability_cache: DashMap::new(),
        }
    }

    fn find_test_roots(call_graph: &Arc<CallGraph>) -> HashSet<FunctionId> {
        let mut test_roots = HashSet::new();

        // Find all functions that are test roots (have no callers and are test functions)
        for func_id in call_graph.get_all_functions() {
            let callers = call_graph.get_callers(func_id);
            if callers.is_empty() && Self::is_test_function(func_id) {
                test_roots.insert(func_id.clone());
            }
        }

        test_roots
    }

    fn is_test_function(func_id: &FunctionId) -> bool {
        let file = func_id.file.to_string_lossy();

        func_id.name.starts_with("test_")
            || func_id.name.contains("::test")
            || file.contains("/tests/")
            || file.contains("_test.rs")
    }

    pub fn is_test_only(&self, func_id: &FunctionId) -> bool {
        // Check cache first (lock-free read via DashMap)
        if let Some(result) = self.reachability_cache.get(func_id) {
            return *result;
        }

        // If it's a test root, it's test-only
        if self.test_roots.contains(func_id) {
            self.reachability_cache.insert(func_id.clone(), true);
            return true;
        }

        // Check if all callers are test-only
        let callers = self.call_graph.get_callers(func_id);
        if callers.is_empty() {
            // No callers and not a test root means it's not test-only
            self.reachability_cache.insert(func_id.clone(), false);
            return false;
        }

        // Use BFS to check if reachable from non-test code
        let is_test_only = self.is_reachable_only_from_tests(func_id);

        // Cache the result
        self.reachability_cache
            .insert(func_id.clone(), is_test_only);

        is_test_only
    }

    fn is_reachable_only_from_tests(&self, func_id: &FunctionId) -> bool {
        let mut visited = HashSet::new();
        let mut queue = vec![func_id.clone()];

        while let Some(current) = queue.pop() {
            if self.reaches_non_test_root(current, &mut visited, &mut queue) {
                return false;
            }
        }

        true
    }

    fn reaches_non_test_root(
        &self,
        current: FunctionId,
        visited: &mut HashSet<FunctionId>,
        queue: &mut Vec<FunctionId>,
    ) -> bool {
        if !visited.insert(current.clone()) {
            return false;
        }

        let callers = self.call_graph.get_callers(&current);
        if callers.is_empty() {
            return !self.test_roots.contains(&current);
        }

        Self::enqueue_unvisited_callers(callers, visited, queue);
        false
    }

    fn enqueue_unvisited_callers(
        callers: Vec<FunctionId>,
        visited: &HashSet<FunctionId>,
        queue: &mut Vec<FunctionId>,
    ) {
        for caller in callers
            .into_iter()
            .filter(|caller| !visited.contains(caller))
        {
            queue.push(caller);
        }
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
    /// Immutable source snapshot shared by suppression and file analysis.
    file_content_index: HashMap<PathBuf, Option<String>>,
    suppression_contexts: SuppressionContextCache,
    suppression_audit: SuppressionAudit,
    analysis_policy: AnalysisPolicy,
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
            file_content_index: HashMap::new(),
            suppression_contexts: HashMap::new(),
            suppression_audit: SuppressionAudit::default(),
            analysis_policy: AnalysisPolicy::from_config(crate::config::get_config()),
            extracted_data: None,
        }
    }

    /// Use a pre-resolved immutable policy for every analysis phase.
    pub fn with_analysis_policy(mut self, policy: AnalysisPolicy) -> Self {
        self.analysis_policy = policy;
        self
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

    fn prepare_file_facts(&mut self, metrics: &[FunctionMetrics]) {
        let mut paths: Vec<_> = metrics
            .iter()
            .map(|metric| metric.file.clone())
            .filter(|path| !self.file_content_index.contains_key(path))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        paths.sort();

        let read = |path: &PathBuf| (path.clone(), std::fs::read_to_string(path).ok());
        let facts: Vec<_> = if self.options.parallel {
            paths.par_iter().map(read).collect()
        } else {
            paths.iter().map(read).collect()
        };
        for (path, content) in facts {
            self.register_file_facts(path, content);
        }
    }

    fn register_file_facts(&mut self, path: PathBuf, content: Option<String>) {
        let extracted_lines = self
            .extracted_data
            .as_ref()
            .and_then(|files| files.get(&path))
            .map(|file| file.total_lines);
        if let Some(lines) =
            extracted_lines.or_else(|| content.as_ref().map(|text| text.lines().count()))
        {
            self.line_count_index.entry(path.clone()).or_insert(lines);
        }
        if let Some(text) = content.as_deref() {
            let context = crate::debt::suppression::parse_suppression_comments(
                text,
                crate::core::Language::from_path(&path),
                &path,
            );
            if context.has_directives() {
                self.suppression_contexts.insert(path.clone(), context);
            }
        }
        self.file_content_index.insert(path, content);
    }

    /// Set the risk analyzer for contextual risk analysis
    pub fn with_risk_analyzer(mut self, risk_analyzer: crate::risk::RiskAnalyzer) -> Self {
        // Ensure analyzer uses same reference time as overall analysis (Spec 214)
        let analyzer = risk_analyzer.with_reference_time(self.options.reference_time);
        self.risk_analyzer = Some(analyzer);
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
        if !self.options.parallel {
            return self.execute_phase1_sequential(metrics, debt_items);
        }

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

    fn execute_phase1_sequential(
        &mut self,
        metrics: &[FunctionMetrics],
        debt_items: Option<&[crate::core::DebtItem]>,
    ) -> (
        DataFlowGraph,
        HashMap<String, bool>,
        HashSet<FunctionId>,
        DebtAggregator,
    ) {
        let started = Instant::now();
        let data_flow =
            crate::builders::unified_analysis_phases::phases::preparation::build_data_flow_graph(
                metrics,
                &self.call_graph,
                self.extracted_data.as_deref(),
            );
        self.timings.data_flow_creation = started.elapsed();

        let started = Instant::now();
        let purity =
            crate::builders::unified_analysis_phases::phases::scoring::metrics_to_purity_map(
                metrics,
            );
        self.timings.purity_analysis = started.elapsed();

        let started = Instant::now();
        let tests =
            crate::builders::unified_analysis_phases::phases::call_graph::find_test_only_functions(
                &self.call_graph,
            );
        self.timings.test_detection = started.elapsed();

        let started = Instant::now();
        let aggregator =
            crate::builders::unified_analysis_phases::phases::scoring::setup_debt_aggregator(
                metrics, debt_items,
            );
        self.timings.debt_aggregation = started.elapsed();
        (data_flow, purity, tests, aggregator)
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
            progress.set_message("Preparing shared data-flow facts...");
            let data_flow = crate::builders::unified_analysis_phases::phases::preparation::build_data_flow_graph(
                &metrics,
                &call_graph,
                extracted_data.as_deref(),
            );

            // parking_lot::Mutex::lock() never fails (no poisoning)
            timings.lock().data_flow_creation = start.elapsed();

            // parking_lot::Mutex::lock() never fails (no poisoning)
            *result.lock() = Some(data_flow);
            progress.finish_with_message("Data-flow preparation complete");
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
            let purity_map =
                crate::builders::unified_analysis_phases::phases::scoring::metrics_to_purity_map(
                    &metrics,
                );
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
            let test_funcs = crate::builders::unified_analysis_phases::phases::call_graph::find_test_only_functions(&call_graph);
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
            let debt_aggregator =
                crate::builders::unified_analysis_phases::phases::scoring::setup_debt_aggregator(
                    &metrics,
                    debt_items.as_deref(),
                );

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
        let prepare_start = Instant::now();
        {
            time_span!("prepare_scoring", parent: "debt_scoring");
            self.prepare_file_facts(metrics);
        }
        self.timings.prepare_scoring = prepare_start.elapsed();

        // Subtask 1: score functions through the shared scheduling kernel.
        let total_metrics = metrics.len();
        if let Some(manager) = ProgressManager::global() {
            manager.tui_update_subtask(
                5,
                1,
                crate::tui::app::StageStatus::Active,
                Some((0, total_metrics)),
            );
        }

        let input = PreparedScoringInput {
            call_graph: &self.call_graph,
            test_only_functions,
            debt_aggregator,
            data_flow: Some(data_flow_graph),
            coverage_data,
            framework_exclusions,
            function_pointer_used_functions,
            risk_analyzer: self.risk_analyzer.as_ref(),
            project_path: &self.project_path,
            file_line_counts: &self.line_count_index,
            suppression_contexts: &self.suppression_contexts,
        };
        let execution = if self.options.parallel {
            ScoringExecution::Parallel
        } else {
            ScoringExecution::Sequential
        };
        let scoring_start = Instant::now();
        let outcome = {
            time_span!("score_functions", parent: "debt_scoring");
            score_metrics_with_policy_audited(metrics, &input, execution, &self.analysis_policy)
        };
        self.suppression_audit = outcome.audit;
        self.timings.score_functions = scoring_start.elapsed();

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

        outcome.emitted
    }

    /// Execute phase 3: Parallel file analysis
    pub fn execute_phase3_parallel(
        &mut self,
        metrics: &[FunctionMetrics],
        coverage_data: Option<&LcovData>,
        no_god_object: bool,
    ) -> Vec<(FileDebtItem, Vec<FunctionMetrics>)> {
        time_span!("analyze_files", parent: "debt_scoring");
        let start = Instant::now();
        self.prepare_file_facts(metrics);

        // Group functions by file
        let mut files_map: HashMap<PathBuf, Vec<&FunctionMetrics>> = HashMap::new();
        for metric in metrics {
            files_map
                .entry(metric.file.clone())
                .or_default()
                .push(metric);
        }

        let mut files: Vec<_> = files_map.into_iter().collect();
        files.sort_by(|a, b| a.0.cmp(&b.0));
        let total_files = files.len();

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

        let analyze = |(file_path, functions): &(PathBuf, Vec<&FunctionMetrics>)| {
            let result = self.analyze_file(file_path, functions, coverage_data, no_god_object);
            record_file_progress(&processed_count, &last_update, total_files);
            result.map(|item| (item, clone_function_metrics(functions)))
        };
        let mut file_data: Vec<(FileDebtItem, Vec<FunctionMetrics>)> = if self.options.parallel {
            files
                .par_iter()
                .progress_with(progress.clone())
                .filter_map(analyze)
                .collect()
        } else {
            files.iter().filter_map(analyze).collect()
        };

        // Sort file_data by path to ensure deterministic order (Spec 214 fix)
        // This ensures god objects are added in a stable order for duplicate checks.
        file_data.sort_by(|a, b| a.0.metrics.path.cmp(&b.0.metrics.path));

        self.timings.file_analysis = start.elapsed();
        self.timings.analyze_files = self.timings.file_analysis;

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

    fn analyze_file(
        &self,
        file_path: &Path,
        functions: &[&FunctionMetrics],
        coverage_data: Option<&LcovData>,
        no_god_object: bool,
    ) -> Option<FileDebtItem> {
        let functions_owned = clone_function_metrics(functions);
        let extracted = self
            .extracted_data
            .as_ref()
            .and_then(|data| data.get(file_path));
        let file_content = self
            .file_content_index
            .get(file_path)
            .and_then(|content| content.as_deref());
        let mut processed =
            crate::builders::unified_analysis_phases::phases::file_analysis::process_file_metrics_with_facts(
                file_path.to_path_buf(),
                functions_owned,
                crate::builders::unified_analysis_phases::phases::file_analysis::FileAnalysisFacts {
                    content: file_content,
                    extracted,
                    line_count: self.line_count_index.get(file_path).copied(),
                },
                coverage_data,
                no_god_object,
                &self.project_path,
            );
        processed.file_metrics.function_scores.clear();
        let item =
            crate::builders::unified_analysis_phases::phases::file_analysis::create_file_debt_item(
                processed.file_metrics,
                Some(&processed.file_context),
            );

        if should_emit_file_item(&item) {
            Some(item)
        } else {
            None
        }
    }

    /// Build the final unified analysis from parallel results
    pub fn build(
        mut self,
        data_flow_graph: DataFlowGraph,
        _purity_analysis: HashMap<String, bool>,
        items: Vec<UnifiedDebtItem>,
        file_data: Vec<(FileDebtItem, Vec<FunctionMetrics>)>,
        coverage_data: Option<&LcovData>,
    ) -> (UnifiedAnalysis, AnalysisPhaseTimings) {
        let start = Instant::now();
        let total_file_items = file_data.len();

        let agg_progress = create_final_aggregation_progress(total_file_items);
        let mut unified = self.initialize_unified_analysis(data_flow_graph, &file_data);
        unified.suppression_audit = self.suppression_audit.clone();

        add_unified_items(&mut unified, items);
        let finalize_start = Instant::now();
        {
            time_span!("finalize_files", parent: "debt_scoring");
            self.add_finalized_file_items(&mut unified, file_data, coverage_data);
            apply_analysis_policy(&mut unified, &self.analysis_policy);
        }
        self.timings.finalize_files = finalize_start.elapsed();

        agg_progress.set_message("Sorting by priority and calculating impact");
        let sorting_start = Instant::now();
        {
            time_span!("sort_items", parent: "debt_scoring");
            unified.sort_by_priority();
        }
        self.timings.sorting = sorting_start.elapsed();
        let impact_start = Instant::now();
        {
            time_span!("calculate_impact", parent: "debt_scoring");
            unified.calculate_total_impact();
            apply_coverage_summary(&mut unified, coverage_data);
        }
        self.timings.calculate_impact = impact_start.elapsed();
        complete_finalization_subtask(total_file_items);
        finish_aggregation_progress(&agg_progress, &unified);

        self.record_final_timing(start.elapsed());
        self.log_timing_summary();

        (unified, self.timings)
    }

    fn initialize_unified_analysis(
        &self,
        data_flow_graph: DataFlowGraph,
        _file_data: &[(FileDebtItem, Vec<FunctionMetrics>)],
    ) -> UnifiedAnalysis {
        let mut unified = UnifiedAnalysis::new((*self.call_graph).clone());
        unified.data_flow_graph = data_flow_graph;
        register_analyzed_files(&mut unified, &self.line_count_index);
        unified
    }

    fn add_finalized_file_items(
        &self,
        unified: &mut UnifiedAnalysis,
        file_data: Vec<(FileDebtItem, Vec<FunctionMetrics>)>,
        coverage_data: Option<&LcovData>,
    ) {
        let total_file_items = file_data.len();

        for (index, (file_item, raw_functions)) in file_data.into_iter().enumerate() {
            let content = self
                .file_content_index
                .get(&file_item.metrics.path)
                .and_then(|content| content.as_deref());
            let finalized = crate::builders::unified_analysis::finalize_file_item_with_content(
                unified,
                file_item,
                &raw_functions,
                coverage_data,
                self.risk_analyzer.as_ref(),
                &self.project_path,
                &self.call_graph,
                content,
            );
            unified.add_file_item(finalized);
            update_finalization_subtask(index + 1, total_file_items);
        }
    }

    fn record_final_timing(&mut self, elapsed: Duration) {
        self.timings.aggregation = elapsed;
        self.timings.total = total_analysis_duration(&self.timings);
    }

    fn log_timing_summary(&self) {
        if !self.options.progress {
            return;
        }

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
        log::debug!("  - Prepare scoring: {:?}", self.timings.prepare_scoring);
        log::debug!("  - Score functions: {:?}", self.timings.score_functions);
        log::debug!(
            "  - Function analysis: {:?}",
            self.timings.function_analysis
        );
        log::debug!("  - File analysis: {:?}", self.timings.file_analysis);
        log::debug!("  - Finalize files: {:?}", self.timings.finalize_files);
        log::debug!("  - Calculate impact: {:?}", self.timings.calculate_impact);
        log::debug!("  - Sorting: {:?}", self.timings.sorting);
    }
}

fn record_file_progress(
    processed: &std::sync::atomic::AtomicUsize,
    last_update: &std::sync::Mutex<Instant>,
    total_files: usize,
) {
    let current = processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    let Ok(mut last) = last_update.try_lock() else {
        return;
    };
    if current % 10 != 0 && last.elapsed() <= Duration::from_millis(100) {
        return;
    }
    if let Some(manager) = ProgressManager::global() {
        manager.tui_update_subtask(
            5,
            2,
            crate::tui::app::StageStatus::Active,
            Some((current, total_files)),
        );
    }
    *last = Instant::now();
}

fn create_final_aggregation_progress(total_file_items: usize) -> indicatif::ProgressBar {
    let progress = ProgressManager::global()
        .map(|pm| pm.create_spinner("Aggregating analysis results"))
        .unwrap_or_else(indicatif::ProgressBar::hidden);

    if let Some(manager) = ProgressManager::global() {
        manager.tui_update_subtask(
            5,
            3,
            crate::tui::app::StageStatus::Active,
            Some((0, total_file_items.max(1))),
        );
    }

    progress
}

fn register_analyzed_files(
    unified: &mut UnifiedAnalysis,
    line_count_index: &HashMap<PathBuf, usize>,
) {
    for (path, line_count) in line_count_index {
        if *line_count > 0 {
            unified.register_analyzed_file(path.clone(), *line_count);
        }
    }
}

fn add_unified_items(unified: &mut UnifiedAnalysis, items: Vec<UnifiedDebtItem>) {
    for item in items {
        unified.add_item(item);
    }
}

fn apply_analysis_policy(unified: &mut UnifiedAnalysis, policy: &AnalysisPolicy) {
    unified.items = unified
        .items
        .iter()
        .filter(|item| {
            policy.allows_debt_type(
                crate::core::Language::from_path(&item.location.file),
                &item.debt_type,
            )
        })
        .cloned()
        .collect();
}

fn apply_coverage_summary(unified: &mut UnifiedAnalysis, coverage_data: Option<&LcovData>) {
    unified.has_coverage_data = coverage_data.is_some();

    if let Some(lcov) = coverage_data {
        unified.overall_coverage = Some(lcov.get_overall_coverage());
    }
}

fn complete_finalization_subtask(total_file_items: usize) {
    if let Some(manager) = ProgressManager::global() {
        manager.tui_update_subtask(
            5,
            3,
            crate::tui::app::StageStatus::Completed,
            Some((total_file_items.max(1), total_file_items.max(1))),
        );
    }
}

fn finish_aggregation_progress(progress: &indicatif::ProgressBar, unified: &UnifiedAnalysis) {
    progress.finish_with_message(format!(
        "Analysis complete ({} function items, {} file items)",
        unified.items.len(),
        unified.file_items.len()
    ));
}

fn total_analysis_duration(timings: &AnalysisPhaseTimings) -> Duration {
    timings.call_graph_building
        + timings.trait_resolution
        + timings.coverage_loading
        + timings.data_flow_creation
        + timings.purity_analysis
        + timings.test_detection
        + timings.debt_aggregation
        + timings.function_analysis
        + timings.file_analysis
        + timings.aggregation
}

fn update_finalization_subtask(current: usize, total: usize) {
    let Some(manager) = ProgressManager::global() else {
        return;
    };

    let should_refresh = current == total || current == 1 || current % 10 == 0;
    if should_refresh {
        manager.tui_update_subtask(
            5,
            3,
            crate::tui::app::StageStatus::Active,
            Some((current, total.max(1))),
        );
    }
}

/// Trait for parallel analysis
pub trait ParallelAnalyzer {
    fn analyze_parallel(
        &self,
        options: ParallelUnifiedAnalysisOptions,
    ) -> Result<UnifiedAnalysis, anyhow::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::call_graph::CallType;

    fn function_id(file: &str, name: &str, line: usize) -> FunctionId {
        FunctionId::new(PathBuf::from(file), name.to_string(), line)
    }

    fn graph_with_functions(functions: &[FunctionId]) -> CallGraph {
        let mut graph = CallGraph::new();
        for func in functions.iter() {
            graph.add_function(func.clone(), false, false, 1, 10);
        }
        graph
    }

    #[test]
    fn test_only_detector_marks_helper_called_only_by_tests() {
        let test = function_id("tests/integration.rs", "test_parses_input", 10);
        let helper = function_id("src/parser.rs", "build_fixture", 20);
        let mut graph = graph_with_functions(&[test.clone(), helper.clone()]);
        graph.add_call_parts(test, helper.clone(), CallType::Direct);

        let detector = OptimizedTestDetector::new(Arc::new(graph));

        assert!(detector.is_test_only(&helper));
    }

    #[test]
    fn test_only_detector_rejects_helper_reachable_from_production_root() {
        let test = function_id("tests/integration.rs", "test_parses_input", 10);
        let production = function_id("src/main.rs", "main", 1);
        let helper = function_id("src/parser.rs", "build_fixture", 20);
        let mut graph = graph_with_functions(&[test.clone(), production.clone(), helper.clone()]);
        graph.add_call_parts(test, helper.clone(), CallType::Direct);
        graph.add_call_parts(production, helper.clone(), CallType::Direct);

        let detector = OptimizedTestDetector::new(Arc::new(graph));

        assert!(!detector.is_test_only(&helper));
    }

    #[test]
    fn parallel_phase_uses_canonical_test_only_classification() {
        let attributed_test = function_id("src/parser.rs", "checks_input", 10);
        let helper = function_id("src/parser.rs", "build_fixture", 20);
        let mut graph = CallGraph::new();
        graph.add_function(attributed_test.clone(), false, true, 2, 5);
        graph.add_function(helper.clone(), false, false, 5, 20);
        graph.add_call_parts(attributed_test, helper.clone(), CallType::Direct);
        let expected = graph.find_test_only_functions().into_iter().collect();
        let mut builder = ParallelUnifiedAnalysisBuilder::new(
            graph,
            ParallelUnifiedAnalysisOptions {
                progress: false,
                ..ParallelUnifiedAnalysisOptions::default()
            },
        );

        let (_, _, actual, _) = builder.execute_phase1_parallel(&[], None);

        assert!(actual.contains(&helper));
        assert_eq!(actual, expected);
    }
}
