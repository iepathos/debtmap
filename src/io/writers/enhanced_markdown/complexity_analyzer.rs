use crate::core::{AnalysisResults, DebtItem};
use crate::priority::UnifiedDebtItem;
use std::collections::HashMap;

/// Categorize debt items by type
pub fn categorize_debt(items: &[DebtItem]) -> HashMap<&'static str, Vec<&DebtItem>> {
    let mut categories: HashMap<&'static str, Vec<&DebtItem>> = HashMap::new();

    for item in items {
        let category = match item.debt_type {
            crate::core::DebtType::Complexity | crate::core::DebtType::TestComplexity => {
                "Complexity Issues"
            }
            crate::core::DebtType::Todo
            | crate::core::DebtType::Fixme
            | crate::core::DebtType::TestTodo => "TODOs and FIXMEs",
            crate::core::DebtType::Duplication | crate::core::DebtType::TestDuplication => {
                "Duplication"
            }
            crate::core::DebtType::CodeSmell | crate::core::DebtType::CodeOrganization => {
                "Code Quality"
            }
            crate::core::DebtType::Dependency => "Dependencies",
            crate::core::DebtType::ErrorSwallowing | crate::core::DebtType::ResourceManagement => {
                "Error Handling"
            }
            crate::core::DebtType::TestQuality => "Test Quality",
        };

        categories.entry(category).or_default().push(item);
    }

    categories
}

/// Estimate effort required to fix a debt item
pub fn estimate_effort(item: &UnifiedDebtItem) -> u32 {
    // Simple effort estimation based on type and severity - using priority's DebtType
    let base_effort = match &item.debt_type {
        crate::priority::DebtType::ComplexityHotspot { cyclomatic, .. } => match cyclomatic {
            0..=5 => 1,
            6..=10 => 2,
            11..=20 => 4,
            _ => 8,
        },
        crate::priority::DebtType::TestingGap { coverage, .. } => match coverage {
            x if x > &0.8 => 1,
            x if x > &0.5 => 2,
            x if x > &0.2 => 4,
            _ => 8,
        },
        crate::priority::DebtType::Risk { risk_score, .. } => {
            if risk_score > &8.0 {
                8
            } else if risk_score > &5.0 {
                4
            } else {
                2
            }
        }
        crate::priority::DebtType::DeadCode { .. } => 2,
        crate::priority::DebtType::Duplication { instances, .. } => {
            if instances > &5 {
                8
            } else {
                4
            }
        }
        crate::priority::DebtType::TestComplexityHotspot { .. } => 4,
        crate::priority::DebtType::TestTodo { .. } => 2,
        crate::priority::DebtType::TestDuplication { .. } => 3,
        crate::priority::DebtType::ErrorSwallowing { .. } => 3,
        crate::priority::DebtType::AllocationInefficiency { .. } => 4,
        crate::priority::DebtType::StringConcatenation { .. } => 3,
        crate::priority::DebtType::NestedLoops { depth, .. } => {
            if depth > &3 {
                8
            } else {
                4
            }
        }
        crate::priority::DebtType::BlockingIO { .. } => 5,
        crate::priority::DebtType::SuboptimalDataStructure { .. } => 6,
        crate::priority::DebtType::GodObject { .. } => 16,
        crate::priority::DebtType::FeatureEnvy { .. } => 8,
        crate::priority::DebtType::PrimitiveObsession { .. } => 4,
        crate::priority::DebtType::MagicValues { .. } => 2,
        crate::priority::DebtType::AssertionComplexity { .. } => 4,
        crate::priority::DebtType::FlakyTestPattern { .. } => 6,
        crate::priority::DebtType::AsyncMisuse { .. } => 8,
        crate::priority::DebtType::ResourceLeak { .. } => 10,
        crate::priority::DebtType::CollectionInefficiency { .. } => 4,
    };

    base_effort * 2 // Account for testing and review
}

/// Extract module dependencies from analysis (simplified)
pub fn extract_module_dependencies(items: &[UnifiedDebtItem]) -> HashMap<String, Vec<String>> {
    let mut deps: HashMap<String, Vec<String>> = HashMap::new();

    for item in items {
        let module = item
            .location
            .file
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        for _callee in &item.downstream_callees {
            // Simplified: assume callees are in different modules
            let target_module = "dependency".to_string();
            if module != target_module {
                deps.entry(module.clone()).or_default().push(target_module);
            }
        }
    }

    // Deduplicate
    for dependencies in deps.values_mut() {
        dependencies.sort();
        dependencies.dedup();
    }

    deps
}

/// Get top complex functions from analysis results
pub fn get_top_complex_functions(results: &AnalysisResults, limit: usize) -> Vec<String> {
    let mut functions: Vec<_> = results
        .complexity
        .metrics
        .iter()
        .map(|m| (m.name.clone(), m.cyclomatic))
        .collect();

    functions.sort_by(|a, b| b.1.cmp(&a.1));
    functions.truncate(limit);

    functions
        .into_iter()
        .map(|(name, complexity)| format!("{} (complexity: {})", name, complexity))
        .collect()
}
