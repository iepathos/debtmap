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
    _methods: &[(String, Option<String>)],
    _language: crate::analysis::io_detection::Language,
    _structures: &[String],
) -> (
    HashMap<String, Vec<String>>,
    HashMap<String, AggregatedClassification>,
) {
    panic!("This function has been removed. Use god_object::classifier and recommender modules directly.")
}

/// Re-export from the deprecated god_object_analysis module.
#[deprecated(
    since = "0.8.0",
    note = "Use domain_diversity module directly. This re-export will be removed in 0.9.0"
)]
#[allow(dead_code)]
pub fn calculate_domain_diversity_from_structs(
    _structs: &[super::core_types::StructMetrics],
    _is_god_object: bool,
) -> Result<crate::organization::DomainDiversityMetrics, anyhow::Error> {
    panic!("This function has been removed. Use domain_diversity module directly.")
}

/// Re-export from the deprecated god_object_analysis module.
#[deprecated(
    since = "0.8.0",
    note = "Use recommender module directly. This re-export will be removed in 0.9.0"
)]
#[allow(dead_code)]
pub fn suggest_splits_by_struct_grouping(
    _structs: &[super::core_types::StructMetrics],
    _ownership: Option<&crate::organization::struct_ownership::StructOwnershipAnalyzer>,
    _file_path: Option<&std::path::Path>,
    _ast: Option<&syn::File>,
) -> Vec<super::split_types::ModuleSplit> {
    panic!("This function has been removed. Use recommender module directly.")
}
