use crate::{core::*, debt, risk};
use anyhow::{Context, Result};
use im::Vector;
use std::path::Path;

pub fn analyze_risk_with_coverage(
    results: &AnalysisResults,
    lcov_path: &Path,
    project_path: &Path,
    enable_context: bool,
    context_providers: Option<Vec<String>>,
    disable_context: Option<Vec<String>>,
) -> Result<Option<risk::RiskInsight>> {
    let lcov_data = risk::lcov::parse_lcov_file(lcov_path).context("Failed to parse LCOV file")?;
    let debt_score = debt::total_debt_score(&results.technical_debt.items) as f64;
    let debt_threshold = 100.0;

    let mut analyzer = risk::RiskAnalyzer::default().with_debt_context(debt_score, debt_threshold);

    if let Some(aggregator) = build_context_aggregator(
        project_path,
        enable_context,
        context_providers,
        disable_context,
    ) {
        analyzer = analyzer.with_context_aggregator(aggregator);
    }

    let mut function_risks = Vector::new();
    let has_context = analyzer.has_context();

    for func in &results.complexity.metrics {
        let complexity_metrics = ComplexityMetrics::from_function(func);
        let coverage = lcov_data.get_function_coverage_with_line(&func.file, &func.name, func.line);

        let risk = if has_context {
            // Use context-aware analysis when context providers are enabled
            let (risk_with_ctx, _) = analyzer.analyze_function_with_context(
                func.file.clone(),
                func.name.clone(),
                (func.line, func.line + func.length),
                &complexity_metrics,
                coverage,
                func.is_test,
                project_path.to_path_buf(),
            );
            risk_with_ctx
        } else {
            analyzer.analyze_function(
                func.file.clone(),
                func.name.clone(),
                (func.line, func.line + func.length),
                &complexity_metrics,
                coverage,
                func.is_test,
            )
        };

        function_risks.push_back(risk);
    }

    let insights = risk::insights::generate_risk_insights(function_risks, &analyzer);
    Ok(Some(insights))
}

pub fn analyze_risk_without_coverage(
    results: &AnalysisResults,
    enable_context: bool,
    context_providers: Option<Vec<String>>,
    disable_context: Option<Vec<String>>,
    project_path: &Path,
) -> Result<Option<risk::RiskInsight>> {
    let debt_score = debt::total_debt_score(&results.technical_debt.items) as f64;
    let debt_threshold = 100.0;

    let mut analyzer = risk::RiskAnalyzer::default().with_debt_context(debt_score, debt_threshold);

    if let Some(aggregator) = build_context_aggregator(
        project_path,
        enable_context,
        context_providers,
        disable_context,
    ) {
        analyzer = analyzer.with_context_aggregator(aggregator);
    }

    let mut function_risks = Vector::new();
    let has_context = analyzer.has_context();

    for func in &results.complexity.metrics {
        let complexity_metrics = ComplexityMetrics::from_function(func);

        let risk = if has_context {
            // Use context-aware analysis when context providers are enabled
            let (risk_with_ctx, _) = analyzer.analyze_function_with_context(
                func.file.clone(),
                func.name.clone(),
                (func.line, func.line + func.length),
                &complexity_metrics,
                None,
                func.is_test,
                project_path.to_path_buf(),
            );
            risk_with_ctx
        } else {
            analyzer.analyze_function(
                func.file.clone(),
                func.name.clone(),
                (func.line, func.line + func.length),
                &complexity_metrics,
                None,
                func.is_test,
            )
        };

        function_risks.push_back(risk);
    }

    let insights = risk::insights::generate_risk_insights(function_risks, &analyzer);
    Ok(Some(insights))
}

pub fn build_context_aggregator(
    project_path: &Path,
    enable_context: bool,
    context_providers: Option<Vec<String>>,
    disable_context: Option<Vec<String>>,
) -> Option<risk::context::ContextAggregator> {
    if !enable_context {
        return None;
    }

    let enabled_providers = context_providers.unwrap_or_else(get_default_providers);
    let disabled = disable_context.unwrap_or_default();

    let aggregator = enabled_providers
        .into_iter()
        .filter(|name| !disabled.contains(name))
        .fold(
            risk::context::ContextAggregator::new(),
            |acc, provider_name| add_provider_to_aggregator(acc, &provider_name, project_path),
        );

    Some(aggregator)
}

fn get_default_providers() -> Vec<String> {
    vec![
        "critical_path".to_string(),
        "dependency".to_string(),
        "git_history".to_string(),
    ]
}

fn add_provider_to_aggregator(
    aggregator: risk::context::ContextAggregator,
    provider_name: &str,
    project_path: &Path,
) -> risk::context::ContextAggregator {
    // Map provider names to subsection indices (spec 219)
    // 0 = critical_path, 1 = dependency, 2 = git_history
    let subsection_index = match provider_name {
        "critical_path" => Some(0),
        "dependency" => Some(1),
        "git_history" => Some(2),
        _ => None,
    };

    // Update subsection to Active state
    if let Some(index) = subsection_index {
        if let Some(manager) = crate::progress::ProgressManager::global() {
            manager.tui_update_subtask(6, index, crate::tui::app::StageStatus::Active, None);
        }
    }

    let result = match create_provider(provider_name, project_path) {
        Some(provider) => aggregator.with_provider(provider),
        None => {
            eprintln!("Warning: Unknown context provider: {provider_name}");
            aggregator
        }
    };

    // Update subsection to Completed state and add visibility pause (spec 219)
    if let Some(index) = subsection_index {
        if let Some(manager) = crate::progress::ProgressManager::global() {
            manager.tui_update_subtask(6, index, crate::tui::app::StageStatus::Completed, None);
            // 150ms visibility pause for user feedback
            std::thread::sleep(std::time::Duration::from_millis(150));
        }
    }

    result
}

fn create_provider(
    provider_name: &str,
    project_path: &Path,
) -> Option<Box<dyn risk::context::ContextProvider>> {
    match provider_name {
        "critical_path" => Some(create_critical_path_provider()),
        "dependency" => Some(create_dependency_provider()),
        "git_history" => create_git_history_provider(project_path),
        _ => None,
    }
}

fn create_critical_path_provider() -> Box<dyn risk::context::ContextProvider> {
    let analyzer = risk::context::critical_path::CriticalPathAnalyzer::new();
    Box::new(risk::context::critical_path::CriticalPathProvider::new(
        analyzer,
    ))
}

fn create_dependency_provider() -> Box<dyn risk::context::ContextProvider> {
    let graph = risk::context::dependency::DependencyGraph::new();
    Box::new(risk::context::dependency::DependencyRiskProvider::new(
        graph,
    ))
}

fn create_git_history_provider(
    project_path: &Path,
) -> Option<Box<dyn risk::context::ContextProvider>> {
    risk::context::git_history::GitHistoryProvider::new(project_path.to_path_buf())
        .ok()
        .map(|provider| Box::new(provider) as Box<dyn risk::context::ContextProvider>)
}
