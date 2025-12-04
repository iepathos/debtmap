//! Metrics Calculation for God Object Detection
//!
//! This module provides pure functions for calculating various metrics used in god object
//! detection. All functions are stateless and side-effect-free, making them easy to test
//! and reason about.

use super::TypeVisitor;
use crate::analysis::FunctionCounts;
use crate::organization::{
    aggregate_weighted_complexity, calculate_avg_complexity, calculate_complexity_weight,
    calculate_god_object_score, calculate_god_object_score_weighted,
    group_methods_by_responsibility, DetectionType, FunctionComplexityInfo,
    FunctionVisibilityBreakdown, GodObjectThresholds, PurityAnalyzer, PurityDistribution,
    PurityLevel, StructMetrics,
};
use std::collections::HashMap;

/// Build metrics for each struct in the file
///
/// Spec 134 Phase 3: This returns ALL methods (including tests) per struct.
/// This is intentional for per-struct breakdown. For file-level god object
/// detection, use the filtered counts from analyze_comprehensive.
pub fn build_per_struct_metrics(visitor: &TypeVisitor) -> Vec<StructMetrics> {
    visitor
        .types
        .values()
        .map(|type_analysis| {
            let responsibilities = group_methods_by_responsibility(&type_analysis.methods);
            StructMetrics {
                name: type_analysis.name.clone(),
                method_count: type_analysis.method_count, // All methods (including tests)
                field_count: type_analysis.field_count,
                responsibilities: responsibilities.keys().cloned().collect(),
                line_span: (
                    type_analysis.location.line,
                    type_analysis
                        .location
                        .end_line
                        .unwrap_or(type_analysis.location.line),
                ),
            }
        })
        .collect()
}

/// Calculate weighted metrics based on complexity and purity
///
/// Returns (weighted_method_count, avg_complexity, purity_weighted_count, purity_distribution)
pub fn calculate_weighted_metrics(
    visitor: &TypeVisitor,
    detection_type: &DetectionType,
) -> (f64, f64, f64, Option<PurityDistribution>) {
    // Calculate complexity-weighted metrics
    // Spec 130: For God Class, use production functions only; for God File, use all
    let relevant_complexity: Vec<_> = match detection_type {
        DetectionType::GodClass => {
            // Filter to production functions only (exclude tests)
            visitor
                .function_complexity
                .iter()
                .filter(|fc| !fc.is_test)
                .cloned()
                .collect()
        }
        DetectionType::GodFile | DetectionType::GodModule => {
            // Include all functions (production + tests)
            visitor.function_complexity.clone()
        }
    };

    let weighted_method_count = aggregate_weighted_complexity(&relevant_complexity);
    let avg_complexity = calculate_avg_complexity(&relevant_complexity);

    // Calculate purity-weighted metrics
    let (purity_weighted_count, purity_distribution) = if !visitor.function_items.is_empty() {
        // Filter function items based on detection type
        let relevant_items: Vec<_> = match detection_type {
            DetectionType::GodClass => {
                // Production functions only
                visitor
                    .function_items
                    .iter()
                    .filter(|item| {
                        !visitor
                            .function_complexity
                            .iter()
                            .find(|fc| item.sig.ident == fc.name)
                            .map(|fc| fc.is_test)
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect()
            }
            DetectionType::GodFile | DetectionType::GodModule => {
                // All functions
                visitor.function_items.clone()
            }
        };
        calculate_purity_weights(&relevant_items, &relevant_complexity)
    } else {
        (weighted_method_count, None)
    };

    (
        weighted_method_count,
        avg_complexity,
        purity_weighted_count,
        purity_distribution,
    )
}

/// Calculates the final god object score using the best available metrics
///
/// # Arguments
/// * `purity_weighted_count` - Purity-based weighted count
/// * `weighted_method_count` - Complexity-weighted method count
/// * `total_methods` - Raw method count
/// * `total_fields` - Number of fields
/// * `responsibility_count` - Number of responsibilities
/// * `lines_of_code` - Estimated lines of code
/// * `avg_complexity` - Average complexity
/// * `purity_distribution` - Optional purity distribution
/// * `has_complexity_data` - Whether complexity data is available
/// * `thresholds` - God object thresholds
/// * `module_structure` - Optional module structure for facade detection (Spec 170)
///
/// # Returns
/// Tuple of (god_object_score, is_god_object)
#[allow(clippy::too_many_arguments)]
pub fn calculate_final_god_object_score(
    purity_weighted_count: f64,
    weighted_method_count: f64,
    total_methods: usize,
    total_fields: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    avg_complexity: f64,
    purity_distribution: &Option<PurityDistribution>,
    has_complexity_data: bool,
    thresholds: &GodObjectThresholds,
    module_structure: Option<&crate::analysis::ModuleStructure>,
) -> (f64, bool) {
    // Use purity-weighted scoring if available, otherwise fall back to complexity weighting or raw count
    let mut god_object_score = if purity_distribution.is_some() {
        calculate_god_object_score_weighted(
            purity_weighted_count,
            total_fields,
            responsibility_count,
            lines_of_code,
            avg_complexity,
            thresholds,
        )
    } else if has_complexity_data {
        calculate_god_object_score_weighted(
            weighted_method_count,
            total_fields,
            responsibility_count,
            lines_of_code,
            avg_complexity,
            thresholds,
        )
    } else {
        calculate_god_object_score(
            total_methods,
            total_fields,
            responsibility_count,
            lines_of_code,
            thresholds,
        )
    };

    // Apply facade scoring adjustment (Spec 170)
    if let Some(structure) = module_structure {
        if let Some(facade_info) = &structure.facade_info {
            god_object_score = crate::priority::scoring::adjust_score_for_facade(
                god_object_score,
                facade_info,
                total_methods,
                lines_of_code,
            );
        }
    }

    // With complexity weighting, use the god_object_score to determine if it's a god object
    // rather than just the confidence level (which still uses raw counts)
    let is_god_object = god_object_score >= 70.0;

    (god_object_score, is_god_object)
}

/// Calculate purity-weighted function contributions
///
/// Combines complexity weighting with purity weighting to produce a total weight
/// for each function. Pure functions contribute less to god object score.
pub fn calculate_purity_weights(
    function_items: &[syn::ItemFn],
    function_complexity: &[FunctionComplexityInfo],
) -> (f64, Option<PurityDistribution>) {
    if function_items.is_empty() {
        return (0.0, None);
    }

    // Build a map of function names to complexity for quick lookup
    let complexity_map: HashMap<String, u32> = function_complexity
        .iter()
        .map(|f| (f.name.clone(), f.cyclomatic_complexity))
        .collect();

    let mut pure_count = 0;
    let mut probably_pure_count = 0;
    let mut impure_count = 0;
    let mut pure_weight = 0.0;
    let mut probably_pure_weight = 0.0;
    let mut impure_weight = 0.0;

    // Analyze each function for purity and calculate combined weights
    for func in function_items {
        let name = func.sig.ident.to_string();
        let purity_level = PurityAnalyzer::analyze(func);
        let complexity = complexity_map.get(&name).copied().unwrap_or(1);

        let complexity_weight = calculate_complexity_weight(complexity);
        let purity_weight_multiplier = purity_level.weight_multiplier();
        let total_weight = complexity_weight * purity_weight_multiplier;

        match purity_level {
            PurityLevel::Pure => {
                pure_count += 1;
                pure_weight += total_weight;
            }
            PurityLevel::ProbablyPure => {
                probably_pure_count += 1;
                probably_pure_weight += total_weight;
            }
            PurityLevel::Impure => {
                impure_count += 1;
                impure_weight += total_weight;
            }
        }
    }

    let total_weighted = pure_weight + probably_pure_weight + impure_weight;
    let distribution = PurityDistribution {
        pure_count,
        probably_pure_count,
        impure_count,
        pure_weight_contribution: pure_weight,
        probably_pure_weight_contribution: probably_pure_weight,
        impure_weight_contribution: impure_weight,
    };

    (total_weighted, Some(distribution))
}

/// Calculate visibility breakdown from TypeVisitor data (Spec 134)
pub fn calculate_visibility_breakdown(
    visitor: &TypeVisitor,
    method_names: &[String],
) -> FunctionVisibilityBreakdown {
    let mut breakdown = FunctionVisibilityBreakdown::new();

    // Count visibility for function items (standalone functions)
    for func_item in &visitor.function_items {
        let name = func_item.sig.ident.to_string();
        if !method_names.contains(&name) {
            continue;
        }

        match &func_item.vis {
            syn::Visibility::Public(_) => breakdown.public += 1,
            syn::Visibility::Restricted(r) => {
                if let Some(ident) = r.path.get_ident() {
                    if ident == "crate" {
                        breakdown.pub_crate += 1;
                    } else if ident == "super" {
                        breakdown.pub_super += 1;
                    } else {
                        breakdown.private += 1;
                    }
                } else {
                    breakdown.private += 1;
                }
            }
            syn::Visibility::Inherited => breakdown.private += 1,
        }
    }

    // Count visibility for impl methods (Spec 134 Phase 2: Fixed)
    for type_info in visitor.types.values() {
        for method_name in &type_info.methods {
            if !method_names.contains(method_name) {
                continue;
            }

            // Look up the tracked visibility for this method
            if let Some(vis) = visitor.method_visibility.get(method_name) {
                match vis {
                    syn::Visibility::Public(_) => breakdown.public += 1,
                    syn::Visibility::Restricted(r) => {
                        if let Some(ident) = r.path.get_ident() {
                            if ident == "crate" {
                                breakdown.pub_crate += 1;
                            } else if ident == "super" {
                                breakdown.pub_super += 1;
                            } else {
                                breakdown.private += 1;
                            }
                        } else {
                            breakdown.private += 1;
                        }
                    }
                    syn::Visibility::Inherited => breakdown.private += 1,
                }
            } else {
                // Fallback: if visibility not tracked, assume private (conservative)
                breakdown.private += 1;
            }
        }
    }

    breakdown
}

/// Integrate visibility breakdown into FunctionCounts (Spec 140)
///
/// This bridges the new visibility tracking system (Spec 134) with the existing
/// ModuleStructure.function_counts field used by output formatters.
///
/// Maps visibility levels to public/private counts:
/// - public_functions = breakdown.public
/// - private_functions = breakdown.private + pub_crate + pub_super
/// - total = breakdown.total()
///
/// Falls back to original counts if visibility_breakdown is unavailable.
pub fn integrate_visibility_into_counts(
    original_counts: &FunctionCounts,
    breakdown: &FunctionVisibilityBreakdown,
    _total_methods: usize,
) -> FunctionCounts {
    FunctionCounts {
        module_level_functions: original_counts.module_level_functions,
        impl_methods: original_counts.impl_methods,
        trait_methods: original_counts.trait_methods,
        nested_module_functions: original_counts.nested_module_functions,
        // Use visibility breakdown for public/private counts
        public_functions: breakdown.public,
        private_functions: breakdown.private + breakdown.pub_crate + breakdown.pub_super,
    }
}
