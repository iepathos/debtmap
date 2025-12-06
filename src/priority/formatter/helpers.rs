//! Helper utilities for formatting (deprecated, will be removed in spec 203)
//!
//! This module contains helper functions that will be replaced by more focused
//! formatting modules. Use these only for backward compatibility.

use crate::priority::{DebtType, FunctionRole, UnifiedDebtItem};

/// Format impact metrics (temporary, will be moved in spec 203)
pub fn format_impact(impact: &crate::priority::ImpactMetrics) -> String {
    let mut parts = Vec::new();

    if impact.coverage_improvement > 0.0 {
        // Show function-level coverage improvement
        if impact.coverage_improvement >= 100.0 {
            parts.push("Full test coverage".to_string());
        } else if impact.coverage_improvement >= 50.0 {
            parts.push(format!(
                "+{}% function coverage",
                impact.coverage_improvement as i32
            ));
        } else {
            // For complex functions that need refactoring first
            parts.push("Partial coverage after refactor".to_string());
        }
    }

    if impact.complexity_reduction > 0.0 {
        parts.push(format!(
            "-{} complexity",
            impact.complexity_reduction as i32
        ));
    }

    if impact.risk_reduction > 0.0 {
        parts.push(format!("-{:.1} risk", impact.risk_reduction));
    }

    if impact.lines_reduction > 0 {
        parts.push(format!("-{} LOC", impact.lines_reduction));
    }

    if parts.is_empty() {
        "Improved maintainability".to_string()
    } else {
        parts.join(", ")
    }
}

/// Extract complexity info from item (temporary, will be removed in spec 203)
pub fn extract_complexity_info(item: &UnifiedDebtItem) -> (u32, u32, u32, u32, usize) {
    // Always show complexity metrics from the item itself, regardless of debt type
    let cyclomatic = item.cyclomatic_complexity;
    let cognitive = item.cognitive_complexity;
    let branch_count = cyclomatic; // Use cyclomatic as proxy for branch count

    (
        cyclomatic,
        cognitive,
        branch_count,
        item.nesting_depth,
        item.function_length,
    )
}

/// Extract dependency info from item (temporary, will be removed in spec 203)
pub fn extract_dependency_info(item: &UnifiedDebtItem) -> (usize, usize) {
    (item.upstream_dependencies, item.downstream_dependencies)
}

/// Format debt type as string
pub fn format_debt_type(debt_type: &DebtType) -> &'static str {
    match debt_type {
        DebtType::TestingGap { .. } => "TEST GAP",
        DebtType::ComplexityHotspot { .. } => "COMPLEXITY",
        DebtType::DeadCode { .. } => "DEAD CODE",
        DebtType::Duplication { .. } => "DUPLICATION",
        DebtType::Risk { .. } => "RISK",
        DebtType::TestComplexityHotspot { .. } => "TEST COMPLEXITY",
        DebtType::TestTodo { .. } => "TEST TODO",
        DebtType::TestDuplication { .. } => "TEST DUPLICATION",
        DebtType::ErrorSwallowing { .. } => "ERROR SWALLOWING",
        // Resource Management debt types
        DebtType::AllocationInefficiency { .. } => "ALLOCATION",
        DebtType::StringConcatenation { .. } => "STRING CONCAT",
        DebtType::NestedLoops { .. } => "NESTED LOOPS",
        DebtType::BlockingIO { .. } => "BLOCKING I/O",
        DebtType::SuboptimalDataStructure { .. } => "DATA STRUCTURE",
        // Organization debt types
        DebtType::GodObject { .. } => "GOD OBJECT",
        DebtType::GodModule { .. } => "GOD MODULE",
        DebtType::FeatureEnvy { .. } => "FEATURE ENVY",
        DebtType::PrimitiveObsession { .. } => "PRIMITIVE OBSESSION",
        DebtType::MagicValues { .. } => "MAGIC VALUES",
        // Testing quality debt types
        DebtType::AssertionComplexity { .. } => "ASSERTION COMPLEXITY",
        DebtType::FlakyTestPattern { .. } => "FLAKY TEST",
        // Resource management debt types
        DebtType::AsyncMisuse { .. } => "ASYNC MISUSE",
        DebtType::ResourceLeak { .. } => "RESOURCE LEAK",
        DebtType::CollectionInefficiency { .. } => "COLLECTION INEFFICIENCY",
        // Type organization (Spec 187)
        DebtType::ScatteredType { .. } => "SCATTERED TYPE",
        DebtType::OrphanedFunctions { .. } => "ORPHANED FUNCTIONS",
        DebtType::UtilitiesSprawl { .. } => "UTILITIES SPRAWL",
        // Default for legacy variants
        _ => "OTHER",
    }
}

/// Format function role as string
pub fn format_role(role: FunctionRole) -> &'static str {
    match role {
        FunctionRole::PureLogic => "PureLogic",
        FunctionRole::Orchestrator => "Orchestrator",
        FunctionRole::IOWrapper => "IOWrapper",
        FunctionRole::EntryPoint => "EntryPoint",
        FunctionRole::PatternMatch => "PatternMatch",
        FunctionRole::Debug => "Debug",
        FunctionRole::Unknown => "Unknown",
    }
}
