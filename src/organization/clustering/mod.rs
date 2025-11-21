//! Advanced responsibility clustering for god object detection
//!
//! This module implements multi-signal clustering that groups methods based on:
//! - Call graph connectivity (40% weight)
//! - Data dependencies (25% weight)
//! - Naming patterns (20% weight)
//! - Behavioral patterns (10% weight)
//! - Architectural layer (5% weight)
//!
//! The clustering algorithm uses hierarchical agglomerative clustering with
//! quality validation to ensure coherent method groupings.

mod hierarchical;
mod quality_metrics;
mod similarity;
mod unclustered_handler;

pub use hierarchical::{Cluster, HierarchicalClustering};
pub use quality_metrics::{calculate_silhouette_score, ClusterQuality};
pub use similarity::{
    CallGraphProvider, ClusteringSimilarityCalculator, FieldAccessProvider, SimilarityWeights,
};
pub use unclustered_handler::UnclusteredMethodHandler;

/// Method information for clustering
#[derive(Debug, Clone)]
pub struct Method {
    pub name: String,
    pub is_pure: bool,
    pub visibility: Visibility,
    pub complexity: u32,
    pub has_io: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Crate,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArchitecturalLayer {
    Api,
    Core,
    Internal,
    Utility,
}

impl ArchitecturalLayer {
    pub fn is_adjacent_to(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Api, Self::Core)
                | (Self::Core, Self::Api)
                | (Self::Core, Self::Internal)
                | (Self::Internal, Self::Core)
        )
    }
}
