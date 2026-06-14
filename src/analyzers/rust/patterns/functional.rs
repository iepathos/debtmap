//! Functional composition analysis
//!
//! Analyzes functional programming patterns in functions.

use crate::analysis::functional_composition::{
    CompositionMetrics, FunctionalAnalysisConfig, analyze_composition,
};

/// Perform functional composition analysis (spec 111)
pub fn analyze_functional_composition(
    enabled: bool,
    item_fn: &syn::ItemFn,
) -> Option<CompositionMetrics> {
    if !enabled {
        return None;
    }

    let config = std::env::var("DEBTMAP_FUNCTIONAL_ANALYSIS_PROFILE")
        .ok()
        .and_then(|p| match p.as_str() {
            "strict" => Some(FunctionalAnalysisConfig::strict()),
            "balanced" => Some(FunctionalAnalysisConfig::balanced()),
            "lenient" => Some(FunctionalAnalysisConfig::lenient()),
            _ => None,
        })
        .unwrap_or_else(FunctionalAnalysisConfig::balanced);

    Some(analyze_composition(item_fn, &config))
}
