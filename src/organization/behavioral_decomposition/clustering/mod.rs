//! Clustering algorithms for behavioral method grouping.
//!
//! This module provides various clustering strategies for grouping methods
//! based on their behavioral characteristics and call patterns:
//!
//! - **Call graph analysis**: Builds adjacency matrix from method calls
//! - **Community detection**: Groups methods by call graph connectivity
//! - **Hybrid clustering**: Combines name-based categorization with call graph analysis
//! - **Production-ready clustering**: Comprehensive pipeline with test filtering and size balancing
//!
//! # Module Structure
//!
//! ```text
//! clustering/
//! ├── mod.rs          # Public API (this file)
//! ├── call_graph.rs   # Build method call adjacency matrix
//! ├── cohesion.rs     # Cohesion calculation utilities
//! ├── community.rs    # Community detection algorithm
//! ├── hybrid.rs       # Hybrid name + call-graph clustering
//! ├── pipeline.rs     # Production pipeline with warnings
//! └── refinement.rs   # Cluster refinement and patterns
//! ```
//!
//! # Design Principles
//!
//! 1. **Pure Functions**: All clustering algorithms are pure
//! 2. **Warnings as Data**: No logging in core; return `ClusteringResult`
//! 3. **Single Responsibility**: Each module handles one aspect
//! 4. **Composable Pipeline**: Production clustering is composed of steps
//!
//! # Usage
//!
//! ```rust,ignore
//! use debtmap::organization::behavioral_decomposition::clustering;
//!
//! // Build call graph
//! let adjacency = clustering::build_method_call_adjacency_matrix(&impl_blocks);
//!
//! // Apply clustering
//! let result = clustering::apply_production_ready_clustering(&methods, &adjacency);
//!
//! // Handle warnings at I/O boundary
//! for warning in &result.warnings {
//!     eprintln!("Warning: {:?}", warning);
//! }
//!
//! // Use clusters
//! for cluster in result.clusters {
//!     println!("Cluster {}: {} methods",
//!         cluster.category.display_name(),
//!         cluster.methods.len()
//!     );
//! }
//! ```

mod call_graph;
mod cohesion;
mod community;
mod hybrid;
mod pipeline;
mod refinement;

// Re-export call graph functions
pub use call_graph::{
    build_method_call_adjacency_matrix, build_method_call_adjacency_matrix_with_functions,
};

// Re-export community detection
pub use community::apply_community_detection;

// Re-export hybrid clustering
pub use hybrid::apply_hybrid_clustering;

// Re-export production pipeline
pub use pipeline::{apply_production_ready_clustering, ClusteringResult, ClusteringWarning};
