use anyhow::Result;
use clap::Parser;
use debtmap::cli::{Cli, Commands};
use debtmap::core::injection::{AppContainer, AppContainerBuilder};
use debtmap::formatting::{ColorMode, EmojiMode, FormattingConfig};
use std::path::Path;
use std::sync::Arc;

// Default implementations for DI container components
mod default_implementations {
    use anyhow::Result;
    use debtmap::core::traits::{
        Cache, CacheStats, ConfigProvider, Formatter, PriorityCalculator, PriorityFactor, Scorer,
    };
    use debtmap::core::types::{AnalysisResult, DebtCategory, DebtItem, Severity};
    use std::collections::HashMap;

    pub struct DefaultDebtScorer;

    impl DefaultDebtScorer {
        pub fn new() -> Self {
            Self
        }
    }

    impl Scorer for DefaultDebtScorer {
        type Item = DebtItem;

        fn score(&self, item: &Self::Item) -> f64 {
            let base_score = match item.category {
                DebtCategory::Complexity => 10.0,
                DebtCategory::Testing => 8.0,
                DebtCategory::Documentation => 6.0,
                DebtCategory::Organization => 7.0,
                DebtCategory::Performance => 9.0,
                DebtCategory::Security => 10.0,
                DebtCategory::Maintainability => 7.0,
                _ => 5.0,
            };

            let severity_multiplier = match item.severity {
                Severity::Critical => 3.0,
                Severity::Major => 2.0,
                Severity::Warning => 1.5,
                Severity::Info => 1.0,
            };

            base_score * severity_multiplier * item.effort.max(0.1)
        }

        fn methodology(&self) -> &str {
            "Default scoring based on category, severity, and estimated hours"
        }
    }

    pub struct DefaultCache {
        storage: std::sync::Mutex<HashMap<String, Vec<u8>>>,
        hits: std::sync::atomic::AtomicUsize,
        misses: std::sync::atomic::AtomicUsize,
    }

    impl DefaultCache {
        pub fn new() -> Result<Self> {
            Ok(Self {
                storage: std::sync::Mutex::new(HashMap::new()),
                hits: std::sync::atomic::AtomicUsize::new(0),
                misses: std::sync::atomic::AtomicUsize::new(0),
            })
        }
    }

    impl Cache for DefaultCache {
        type Key = String;
        type Value = Vec<u8>;

        fn get(&self, key: &Self::Key) -> Option<Self::Value> {
            let storage = self.storage.lock().unwrap();
            if let Some(value) = storage.get(key) {
                self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Some(value.clone())
            } else {
                self.misses
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                None
            }
        }

        fn set(&mut self, key: Self::Key, value: Self::Value) {
            let mut storage = self.storage.lock().unwrap();
            storage.insert(key, value);
        }

        fn clear(&mut self) {
            let mut storage = self.storage.lock().unwrap();
            storage.clear();
            self.hits.store(0, std::sync::atomic::Ordering::Relaxed);
            self.misses.store(0, std::sync::atomic::Ordering::Relaxed);
        }

        fn stats(&self) -> CacheStats {
            let storage = self.storage.lock().unwrap();
            let memory_usage: usize = storage.values().map(|v| v.len()).sum();

            CacheStats {
                hits: self.hits.load(std::sync::atomic::Ordering::Relaxed),
                misses: self.misses.load(std::sync::atomic::Ordering::Relaxed),
                entries: storage.len(),
                memory_usage,
            }
        }
    }

    pub struct DefaultConfigProvider {
        config: std::sync::RwLock<HashMap<String, String>>,
    }

    impl DefaultConfigProvider {
        pub fn new() -> Self {
            let mut config = HashMap::new();
            // Load default configuration values
            config.insert("complexity_threshold".to_string(), "10".to_string());
            config.insert("max_file_size".to_string(), "1000000".to_string());
            config.insert("enable_caching".to_string(), "true".to_string());
            config.insert("parallel_processing".to_string(), "true".to_string());

            Self {
                config: std::sync::RwLock::new(config),
            }
        }
    }

    impl ConfigProvider for DefaultConfigProvider {
        fn get(&self, key: &str) -> Option<String> {
            let config = self.config.read().unwrap();
            config.get(key).cloned()
        }

        fn set(&mut self, key: String, value: String) {
            let mut config = self.config.write().unwrap();
            config.insert(key, value);
        }

        fn load_from_file(&self, _path: &std::path::Path) -> Result<()> {
            // In production, would read from actual config file
            // For now, just return Ok to satisfy the trait
            Ok(())
        }
    }

    pub struct DefaultPriorityCalculator;

    impl DefaultPriorityCalculator {
        pub fn new() -> Self {
            Self
        }
    }

    impl PriorityCalculator for DefaultPriorityCalculator {
        type Item = DebtItem;

        fn calculate_priority(&self, item: &Self::Item) -> f64 {
            let severity_weight = match item.severity {
                Severity::Critical => 1.0,
                Severity::Major => 0.75,
                Severity::Warning => 0.5,
                Severity::Info => 0.25,
            };

            let category_weight = match item.category {
                DebtCategory::Complexity => 0.9,
                DebtCategory::Testing => 0.85,
                DebtCategory::Performance => 0.8,
                DebtCategory::Organization => 0.7,
                DebtCategory::Documentation => 0.6,
                DebtCategory::Security => 0.95,
                _ => 0.5,
            };

            let effort_factor = 1.0 / (1.0 + item.effort);

            f64::min(
                severity_weight * 0.5 + category_weight * 0.3 + effort_factor * 0.2,
                1.0,
            )
        }

        fn get_factors(&self, item: &Self::Item) -> Vec<PriorityFactor> {
            vec![
                PriorityFactor {
                    name: "severity".to_string(),
                    weight: 0.5,
                    value: match item.severity {
                        Severity::Critical => 1.0,
                        Severity::Major => 0.75,
                        Severity::Warning => 0.5,
                        Severity::Info => 0.25,
                    },
                    description: format!("Severity: {:?}", item.severity),
                },
                PriorityFactor {
                    name: "category".to_string(),
                    weight: 0.3,
                    value: match item.category {
                        DebtCategory::Complexity => 0.9,
                        DebtCategory::Testing => 0.85,
                        _ => 0.5,
                    },
                    description: format!("Category: {:?}", item.category),
                },
                PriorityFactor {
                    name: "effort".to_string(),
                    weight: 0.2,
                    value: 1.0 / (1.0 + item.effort),
                    description: format!("Estimated effort: {} hours", item.effort),
                },
            ]
        }
    }

    pub struct JsonFormatter;

    impl JsonFormatter {
        pub fn new() -> Self {
            Self
        }
    }

    impl Formatter for JsonFormatter {
        type Report = AnalysisResult;

        fn format(&self, report: &Self::Report) -> Result<String> {
            serde_json::to_string_pretty(report)
                .map_err(|e| anyhow::anyhow!("JSON formatting error: {}", e))
        }

        fn format_name(&self) -> &str {
            "json"
        }
    }

    pub struct MarkdownFormatter;

    impl MarkdownFormatter {
        pub fn new() -> Self {
            Self
        }
    }

    impl Formatter for MarkdownFormatter {
        type Report = AnalysisResult;

        fn format(&self, report: &Self::Report) -> Result<String> {
            let mut output = String::new();
            output.push_str("# Code Analysis Report\n\n");
            output.push_str("## Summary\n\n");
            output.push_str(&format!("- Total Files: {}\n", report.metrics.total_files));
            output.push_str(&format!(
                "- Total Functions: {}\n",
                report.metrics.total_functions
            ));
            output.push_str(&format!("- Total Lines: {}\n", report.metrics.total_lines));
            output.push_str(&format!(
                "- Average Complexity: {:.2}\n",
                report.metrics.average_complexity
            ));
            output.push_str(&format!("- Debt Score: {:.2}\n\n", report.total_score));

            if !report.debt_items.is_empty() {
                output.push_str("## Technical Debt Items\n\n");
                for item in &report.debt_items {
                    output.push_str(&format!(
                        "- **{:?}** ({:?}): {}\n",
                        item.category, item.severity, item.description
                    ));
                }
            }

            Ok(output)
        }

        fn format_name(&self) -> &str {
            "markdown"
        }
    }

    pub struct TerminalFormatter;

    impl TerminalFormatter {
        pub fn new() -> Self {
            Self
        }
    }

    impl Formatter for TerminalFormatter {
        type Report = AnalysisResult;

        fn format(&self, report: &Self::Report) -> Result<String> {
            let mut output = String::new();
            output.push_str("═══════════════════════════════════════\n");
            output.push_str("         Code Analysis Report          \n");
            output.push_str("═══════════════════════════════════════\n\n");

            output.push_str(&format!(
                "Total Files:      {}\n",
                report.metrics.total_files
            ));
            output.push_str(&format!(
                "Total Functions:  {}\n",
                report.metrics.total_functions
            ));
            output.push_str(&format!(
                "Total Lines:      {}\n",
                report.metrics.total_lines
            ));
            output.push_str(&format!(
                "Avg Complexity:   {:.2}\n",
                report.metrics.average_complexity
            ));
            output.push_str(&format!("Debt Score:       {:.2}\n", report.total_score));

            if !report.debt_items.is_empty() {
                output.push_str("\n───────────────────────────────────────\n");
                output.push_str("Technical Debt Summary:\n");
                output.push_str(&format!("  {} items found\n", report.debt_items.len()));

                // Create severity counts
                let mut critical_count = 0;
                let mut major_count = 0;
                let mut warning_count = 0;
                let mut info_count = 0;

                for item in &report.debt_items {
                    match item.severity {
                        Severity::Critical => critical_count += 1,
                        Severity::Major => major_count += 1,
                        Severity::Warning => warning_count += 1,
                        Severity::Info => info_count += 1,
                    }
                }

                if critical_count > 0 {
                    output.push_str(&format!("  Critical: {}\n", critical_count));
                }
                if major_count > 0 {
                    output.push_str(&format!("  Major: {}\n", major_count));
                }
                if warning_count > 0 {
                    output.push_str(&format!("  Warning: {}\n", warning_count));
                }
                if info_count > 0 {
                    output.push_str(&format!("  Info: {}\n", info_count));
                }
            }

            Ok(output)
        }

        fn format_name(&self) -> &str {
            "terminal"
        }
    }
}

// Create and configure the dependency injection container
fn create_app_container() -> Result<AppContainer> {
    use default_implementations::*;

    // Build the container with all required dependencies
    // We need to create instances directly since the factory returns Box<dyn Analyzer>
    // But our AppContainerBuilder expects concrete types implementing the new trait
    use debtmap::core::injection::{
        JavaScriptAnalyzerAdapter, PythonAnalyzerAdapter, RustAnalyzerAdapter,
        TypeScriptAnalyzerAdapter,
    };

    let container = AppContainerBuilder::new()
        .with_rust_analyzer(RustAnalyzerAdapter::new())
        .with_python_analyzer(PythonAnalyzerAdapter::new())
        .with_js_analyzer(JavaScriptAnalyzerAdapter::new())
        .with_ts_analyzer(TypeScriptAnalyzerAdapter::new())
        .with_debt_scorer(DefaultDebtScorer::new())
        .with_cache(DefaultCache::new()?)
        .with_config(DefaultConfigProvider::new())
        .with_priority_calculator(DefaultPriorityCalculator::new())
        .with_json_formatter(JsonFormatter::new())
        .with_markdown_formatter(MarkdownFormatter::new())
        .with_terminal_formatter(TerminalFormatter::new())
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build container: {}", e))?;

    Ok(container)
}

// Main orchestrator function
fn main() -> Result<()> {
    let cli = Cli::parse();

    // Create the dependency injection container once at startup
    let _container = Arc::new(create_app_container()?);

    match cli.command {
        command @ Commands::Analyze { .. } => handle_analyze_command(command)?,
        Commands::Init { force } => {
            debtmap::commands::init::init_config(force)?;
            Ok(())
        }
        Commands::Validate {
            path,
            config,
            coverage_file,
            format,
            output,
            enable_context,
            context_providers,
            disable_context,
            max_debt_density,
            top,
            tail,
            summary: _, // Validate command doesn't use summary yet
            semantic_off,
            explain_score: _,
            verbosity,
        } => {
            let validate_config = debtmap::commands::validate::ValidateConfig {
                path,
                config,
                coverage_file,
                format,
                output,
                enable_context,
                context_providers,
                disable_context,
                max_debt_density,
                top,
                tail,
                semantic_off,
                verbosity,
            };
            debtmap::commands::validate::validate_project(validate_config)?;
            Ok(())
        }
        Commands::Compare {
            before,
            after,
            plan,
            target_location,
            format,
            output,
        } => {
            handle_compare_command(
                &before,
                &after,
                plan.as_deref(),
                target_location,
                format,
                output.as_deref(),
            )?;
            Ok(())
        }
    }
}

// Pure function to handle analyze command with all its complexity
fn handle_analyze_command(command: Commands) -> Result<Result<()>> {
    if let Commands::Analyze {
        path,
        format,
        json_format,
        output,
        threshold_complexity,
        threshold_duplication,
        languages,
        coverage_file,
        enable_context,
        context_providers,
        disable_context,
        top,
        tail,
        summary,
        semantic_off,
        explain_score: _,
        verbosity,
        verbose_macro_warnings,
        show_macro_stats,
        group_by_category,
        min_priority,
        filter_categories,
        no_context_aware,
        threshold_preset,
        plain,
        no_parallel,
        jobs,
        use_cache,
        no_cache,
        clear_cache,
        force_cache_rebuild,
        cache_stats,
        migrate_cache,
        cache_location,
        multi_pass,
        show_attribution,
        detail_level,
        aggregate_only,
        no_aggregation,
        aggregation_method,
        min_problematic,
        no_god_object,
        max_files,
        validate_loc,
        no_public_api_detection,
        public_api_threshold,
        no_pattern_detection,
        patterns,
        pattern_threshold,
        show_pattern_warnings,
    } = command
    {
        // Apply side effects first
        let cache_location_path = cache_location.as_ref().map(std::path::PathBuf::from);
        apply_environment_setup(&cache_location_path, no_context_aware)?;

        // Handle early returns for cache operations
        if let Some(result) = handle_cache_operations(cache_stats, migrate_cache, &path)? {
            return Ok(result);
        }

        // Build configuration from pure data transformation
        let formatting_config = create_formatting_config(plain);
        let config = build_analyze_config(
            path,
            format,
            json_format,
            output,
            threshold_complexity,
            threshold_duplication,
            languages,
            coverage_file,
            enable_context,
            context_providers,
            disable_context,
            top,
            tail,
            summary,
            semantic_off,
            verbosity,
            verbose_macro_warnings,
            show_macro_stats,
            group_by_category,
            min_priority,
            filter_categories,
            no_context_aware,
            threshold_preset,
            formatting_config,
            no_parallel,
            jobs,
            use_cache,
            no_cache,
            clear_cache,
            force_cache_rebuild,
            cache_location,
            multi_pass,
            show_attribution,
            detail_level,
            aggregate_only,
            no_aggregation,
            aggregation_method,
            min_problematic,
            no_god_object,
            max_files,
            validate_loc,
            no_public_api_detection,
            public_api_threshold,
            no_pattern_detection,
            patterns,
            pattern_threshold,
            show_pattern_warnings,
        );

        Ok(debtmap::commands::analyze::handle_analyze(config))
    } else {
        Err(anyhow::anyhow!("Invalid command"))
    }
}

// Pure function to check cache operations
fn handle_cache_operations(
    cache_stats: bool,
    migrate_cache: bool,
    path: &Path,
) -> Result<Option<Result<()>>> {
    if cache_stats {
        return Ok(Some(handle_cache_stats(path)));
    }
    if migrate_cache {
        return Ok(Some(handle_cache_migration(path)?));
    }
    Ok(None)
}

// Side effect handler for cache stats
fn handle_cache_stats(path: &std::path::Path) -> Result<()> {
    let cache = debtmap::cache::SharedCache::new(Some(path))?;
    let stats = cache.get_stats();
    println!("Cache Statistics:");
    println!("  Entries: {}", stats.entry_count);
    println!("  Size: {} bytes", stats.total_size);
    Ok(())
}

// Side effect handler for cache migration
fn handle_cache_migration(path: &std::path::Path) -> Result<Result<()>> {
    use debtmap::cache::CacheStrategy;

    println!("Migrating cache to shared location");

    // Get the shared cache location
    let dst_location =
        debtmap::cache::CacheLocation::resolve_with_strategy(Some(path), CacheStrategy::Shared)?;

    // Create a new cache at the shared location
    let _dst_cache = debtmap::cache::SharedCache::new_with_cache_dir(
        Some(path),
        dst_location.get_cache_path().to_path_buf(),
    )?;

    // For now, just report that we've set up the cache in the shared location
    println!(
        "Cache now configured at shared location: {:?}",
        dst_location.get_cache_path()
    );
    Ok(Ok(()))
}

// Side effect function for environment setup (I/O at edges)
fn apply_environment_setup(
    cache_location: &Option<std::path::PathBuf>,
    no_context_aware: bool,
) -> Result<()> {
    if let Some(ref location) = cache_location {
        std::env::set_var("DEBTMAP_CACHE_DIR", location);
    }

    if !no_context_aware {
        std::env::set_var("DEBTMAP_CONTEXT_AWARE", "true");
    }

    Ok(())
}

// Pure function to check condition
fn should_use_cache(use_cache: bool, no_cache: bool) -> bool {
    !no_cache || use_cache
}

// Pure function to determine parallel mode
fn should_use_parallel(no_parallel: bool) -> bool {
    !no_parallel
}

// Pure function to get worker count
fn get_worker_count(jobs: usize) -> usize {
    if jobs == 0 {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    } else {
        jobs
    }
}

// Pure functions for data transformation
fn convert_min_priority(priority: Option<String>) -> Option<String> {
    priority
}

fn convert_filter_categories(categories: Option<Vec<String>>) -> Option<Vec<String>> {
    categories.filter(|v| !v.is_empty())
}

fn convert_context_providers(providers: Option<Vec<String>>) -> Option<Vec<String>> {
    providers.filter(|v| !v.is_empty())
}

fn convert_disable_context(disable_context: Option<Vec<String>>) -> Option<Vec<String>> {
    disable_context
}

fn convert_languages(languages: Option<Vec<String>>) -> Option<Vec<String>> {
    languages.filter(|v| !v.is_empty())
}

fn convert_cache_location(location: &Option<String>) -> Option<String> {
    location.clone()
}

fn convert_threshold_preset(
    preset: Option<debtmap::cli::ThresholdPreset>,
) -> Option<debtmap::cli::ThresholdPreset> {
    preset
}

fn convert_output_format(format: debtmap::cli::OutputFormat) -> debtmap::cli::OutputFormat {
    format
}

// Pure function to create formatting configuration
fn create_formatting_config(plain: bool) -> FormattingConfig {
    if plain {
        FormattingConfig::new(ColorMode::Never, EmojiMode::Never)
    } else {
        FormattingConfig::from_env()
    }
}

// Pure function to build analyze configuration
#[allow(clippy::too_many_arguments)]
fn build_analyze_config(
    path: std::path::PathBuf,
    format: debtmap::cli::OutputFormat,
    json_format: debtmap::cli::JsonFormat,
    output: Option<std::path::PathBuf>,
    threshold_complexity: u32,
    threshold_duplication: usize,
    languages: Option<Vec<String>>,
    coverage_file: Option<std::path::PathBuf>,
    enable_context: bool,
    context_providers: Option<Vec<String>>,
    disable_context: Option<Vec<String>>,
    top: Option<usize>,
    tail: Option<usize>,
    summary: bool,
    semantic_off: bool,
    verbosity: u8,
    verbose_macro_warnings: bool,
    show_macro_stats: bool,
    group_by_category: bool,
    min_priority: Option<String>,
    filter_categories: Option<Vec<String>>,
    no_context_aware: bool,
    threshold_preset: Option<debtmap::cli::ThresholdPreset>,
    formatting_config: FormattingConfig,
    no_parallel: bool,
    jobs: usize,
    use_cache: bool,
    no_cache: bool,
    clear_cache: bool,
    force_cache_rebuild: bool,
    cache_location: Option<String>,
    multi_pass: bool,
    show_attribution: bool,
    detail_level: Option<String>,
    aggregate_only: bool,
    no_aggregation: bool,
    aggregation_method: Option<String>,
    min_problematic: Option<usize>,
    no_god_object: bool,
    max_files: Option<usize>,
    validate_loc: bool,
    no_public_api_detection: bool,
    public_api_threshold: f32,
    no_pattern_detection: bool,
    patterns: Option<Vec<String>>,
    pattern_threshold: f32,
    show_pattern_warnings: bool,
) -> debtmap::commands::analyze::AnalyzeConfig {
    debtmap::commands::analyze::AnalyzeConfig {
        path,
        format: convert_output_format(format),
        json_format,
        output,
        threshold_complexity,
        threshold_duplication,
        languages: convert_languages(languages),
        coverage_file,
        enable_context,
        context_providers: convert_context_providers(context_providers),
        disable_context: convert_disable_context(disable_context),
        top,
        tail,
        summary,
        semantic_off,
        verbosity,
        verbose_macro_warnings,
        show_macro_stats,
        group_by_category,
        min_priority: convert_min_priority(min_priority),
        filter_categories: convert_filter_categories(filter_categories),
        no_context_aware,
        threshold_preset: convert_threshold_preset(threshold_preset),
        formatting_config,
        parallel: should_use_parallel(no_parallel),
        jobs: get_worker_count(jobs),
        use_cache: should_use_cache(use_cache, no_cache),
        no_cache,
        clear_cache,
        force_cache_rebuild,
        cache_stats: false,
        migrate_cache: false,
        cache_location: convert_cache_location(&cache_location),
        multi_pass,
        show_attribution,
        detail_level,
        aggregate_only,
        no_aggregation,
        aggregation_method,
        min_problematic,
        no_god_object,
        max_files,
        validate_loc,
        no_public_api_detection,
        public_api_threshold,
        no_pattern_detection,
        patterns,
        pattern_threshold,
        show_pattern_warnings,
    }
}

/// Pure function to convert UnifiedJsonOutput to UnifiedAnalysis
/// Splits merged DebtItem enum into separate function and file vectors
fn json_to_analysis(
    json: debtmap::output::json::UnifiedJsonOutput,
) -> debtmap::priority::UnifiedAnalysis {
    use debtmap::priority::{call_graph::CallGraph, DebtItem, UnifiedAnalysis};
    use im::Vector;

    let mut items = Vector::new();
    let mut file_items = Vector::new();

    // Split DebtItems into function and file items
    for item in json.items {
        match item {
            DebtItem::Function(func) => items.push_back(*func),
            DebtItem::File(file) => file_items.push_back(*file),
        }
    }

    // Create UnifiedAnalysis with empty call graph and data flow graph
    // These aren't serialized in JSON output anyway
    let call_graph = CallGraph::new();

    UnifiedAnalysis {
        items,
        file_items,
        total_impact: json.total_impact,
        total_debt_score: json.total_debt_score,
        debt_density: json.debt_density,
        total_lines_of_code: json.total_lines_of_code,
        call_graph: call_graph.clone(),
        data_flow_graph: debtmap::data_flow::DataFlowGraph::from_call_graph(call_graph),
        overall_coverage: json.overall_coverage,
        has_coverage_data: json.overall_coverage.is_some(),
    }
}

// Handle compare command
fn handle_compare_command(
    before: &Path,
    after: &Path,
    plan: Option<&Path>,
    target_location: Option<String>,
    format: debtmap::cli::OutputFormat,
    output: Option<&Path>,
) -> Result<()> {
    use debtmap::comparison::{Comparator, PlanParser};
    use debtmap::output::json::UnifiedJsonOutput;
    use std::fs;

    // Extract target location from plan or use explicit location
    let target = if let Some(plan_path) = plan {
        Some(PlanParser::extract_target_location(plan_path)?)
    } else {
        target_location
    };

    // Load JSON output and convert to UnifiedAnalysis
    let before_content = fs::read_to_string(before)?;
    let before_json: UnifiedJsonOutput = serde_json::from_str(&before_content)?;
    let before_results = json_to_analysis(before_json);

    let after_content = fs::read_to_string(after)?;
    let after_json: UnifiedJsonOutput = serde_json::from_str(&after_content)?;
    let after_results = json_to_analysis(after_json);

    // Perform comparison
    let comparator = Comparator::new(before_results, after_results, target);
    let comparison = comparator.compare()?;

    // Output results
    let output_str = match format {
        debtmap::cli::OutputFormat::Json => serde_json::to_string_pretty(&comparison)?,
        debtmap::cli::OutputFormat::Markdown => format_comparison_markdown(&comparison),
        debtmap::cli::OutputFormat::Terminal => {
            print_comparison_terminal(&comparison);
            return Ok(());
        }
    };

    // Write to file or stdout
    if let Some(output_path) = output {
        fs::write(output_path, output_str)?;
    } else {
        println!("{}", output_str);
    }

    Ok(())
}

// Format comparison as markdown
fn format_comparison_markdown(comparison: &debtmap::comparison::ComparisonResult) -> String {
    use debtmap::comparison::{DebtTrend, TargetStatus};

    let mut md = String::new();

    md.push_str("# Debtmap Comparison Report\n\n");
    md.push_str(&format!(
        "**Date**: {}\n\n",
        comparison.metadata.comparison_date
    ));

    if let Some(target) = &comparison.target_item {
        md.push_str("## Target Item Analysis\n\n");

        let status_icon = match target.status {
            TargetStatus::Resolved => "✅",
            TargetStatus::Improved => "✅",
            TargetStatus::Unchanged => "⚠️",
            TargetStatus::Regressed => "❌",
            TargetStatus::NotFoundBefore | TargetStatus::NotFound => "❓",
        };

        md.push_str(&format!(
            "{} **Status**: {:?}\n\n",
            status_icon, target.status
        ));
        md.push_str(&format!("**Location**: `{}`\n\n", target.location));

        md.push_str("### Before\n");
        md.push_str(&format!("- **Score**: {:.1}\n", target.before.score));
        md.push_str(&format!(
            "- **Complexity**: Cyclomatic {}, Cognitive {}\n",
            target.before.cyclomatic_complexity, target.before.cognitive_complexity
        ));
        md.push_str(&format!("- **Coverage**: {:.1}%\n", target.before.coverage));
        md.push_str(&format!(
            "- **Function Length**: {} lines\n\n",
            target.before.function_length
        ));

        if let Some(after_metrics) = &target.after {
            md.push_str("### After\n");
            md.push_str(&format!("- **Score**: {:.1}\n", after_metrics.score));
            md.push_str(&format!(
                "- **Complexity**: Cyclomatic {}, Cognitive {}\n",
                after_metrics.cyclomatic_complexity, after_metrics.cognitive_complexity
            ));
            md.push_str(&format!("- **Coverage**: {:.1}%\n", after_metrics.coverage));
            md.push_str(&format!(
                "- **Function Length**: {} lines\n\n",
                after_metrics.function_length
            ));
        }

        md.push_str("### Improvements\n");
        md.push_str(&format!(
            "- Score reduced by **{:.1}%**\n",
            target.improvements.score_reduction_pct
        ));
        md.push_str(&format!(
            "- Complexity reduced by **{:.1}%**\n",
            target.improvements.complexity_reduction_pct
        ));
        md.push_str(&format!(
            "- Coverage improved by **{:.1}%**\n\n",
            target.improvements.coverage_improvement_pct
        ));
    }

    md.push_str("## Project Health\n\n");

    let trend_icon = match comparison.summary.overall_debt_trend {
        DebtTrend::Improving => "📉",
        DebtTrend::Stable => "➡️",
        DebtTrend::Regressing => "📈",
    };

    md.push_str(&format!(
        "### Overall Trend: {} {:?}\n\n",
        trend_icon, comparison.summary.overall_debt_trend
    ));
    md.push_str(&format!(
        "- Total debt: {:.1} → {:.1} ({:+.1}%)\n",
        comparison.project_health.before.total_debt_score,
        comparison.project_health.after.total_debt_score,
        comparison.project_health.changes.debt_score_change_pct
    ));
    md.push_str(&format!(
        "- Critical items: {} → {} ({:+})\n",
        comparison.project_health.before.critical_items,
        comparison.project_health.after.critical_items,
        comparison.project_health.changes.critical_items_change
    ));

    if !comparison.regressions.is_empty() {
        md.push_str(&format!(
            "\n⚠️ {} new critical item(s) detected\n\n",
            comparison.regressions.len()
        ));

        md.push_str("### Regressions\n\n");
        for reg in &comparison.regressions {
            md.push_str(&format!("- `{}` (score: {:.1})\n", reg.location, reg.score));
        }
    } else {
        md.push_str("\n✅ No new critical items introduced\n");
    }

    md.push_str("\n## Summary\n\n");
    if comparison.summary.target_improved {
        md.push_str("✅ Target item significantly improved\n");
    }
    if comparison.summary.new_critical_count == 0 {
        md.push_str("✅ No regressions detected\n");
    }
    match comparison.summary.overall_debt_trend {
        DebtTrend::Improving => md.push_str("✅ Overall project health improved\n"),
        DebtTrend::Stable => md.push_str("➡️ Overall project health stable\n"),
        DebtTrend::Regressing => md.push_str("⚠️ Overall project health declined\n"),
    }

    md
}

// Print comparison to terminal
fn print_comparison_terminal(comparison: &debtmap::comparison::ComparisonResult) {
    println!("{}", format_comparison_markdown(comparison));
}
