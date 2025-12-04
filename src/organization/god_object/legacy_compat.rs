//! Legacy Compatibility Layer
//!
//! This module provides backward compatibility for deprecated functions from
//! god_object_analysis.rs that are still needed by external tests.
//!
//! These functions will be removed in v0.9.0.

#![allow(deprecated)]

use crate::analysis::multi_signal_aggregation::AggregatedClassification;
use std::collections::HashMap;

/// Re-export from the deprecated god_object_analysis module.
///
/// This function groups methods by responsibility using domain pattern detection.
/// It is used by tests and will be removed in v0.9.0.
#[deprecated(
    since = "0.8.0",
    note = "Use god_object classifier and recommender modules directly. This re-export will be removed in 0.9.0"
)]
pub fn group_methods_by_responsibility_with_domain_patterns(
    methods: &[(String, Option<String>)],
    language: crate::analysis::io_detection::Language,
    structures: &[String],
) -> (
    HashMap<String, Vec<String>>,
    HashMap<String, AggregatedClassification>,
) {
    crate::organization::god_object_analysis::group_methods_by_responsibility_with_domain_patterns(
        methods, language, structures,
    )
}

/// Re-export from the deprecated god_object_analysis module.
#[deprecated(
    since = "0.8.0",
    note = "Use domain_diversity module directly. This re-export will be removed in 0.9.0"
)]
#[allow(dead_code)]
pub fn calculate_domain_diversity_from_structs(
    structs: &[super::core_types::StructMetrics],
    is_god_object: bool,
) -> Result<crate::organization::DomainDiversityMetrics, anyhow::Error> {
    crate::organization::god_object_analysis::calculate_domain_diversity_from_structs(
        structs,
        is_god_object,
    )
}

/// Re-export from the deprecated god_object_analysis module.
#[deprecated(
    since = "0.8.0",
    note = "Use recommender module directly. This re-export will be removed in 0.9.0"
)]
#[allow(dead_code)]
pub fn suggest_splits_by_struct_grouping(
    structs: &[super::core_types::StructMetrics],
    ownership: Option<&crate::organization::struct_ownership::StructOwnershipAnalyzer>,
    file_path: Option<&std::path::Path>,
    ast: Option<&syn::File>,
) -> Vec<super::split_types::ModuleSplit> {
    crate::organization::god_object_analysis::suggest_splits_by_struct_grouping(
        structs, ownership, file_path, ast,
    )
}
