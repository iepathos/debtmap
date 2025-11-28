//! Effect-based wrappers for call graph analysis (Spec 207).
//!
//! This module provides effect-based interfaces for Rust-specific call graph
//! analysis, enabling configuration access via the Reader pattern and supporting
//! testability with `DebtmapTestEnv`.
//!
//! # Architecture
//!
//! The call graph module follows a "pure core, effects shell" pattern:
//!
//! - **Pure functions**: Graph building, trait resolution, pattern detection
//! - **Effect wrappers**: Configuration access and post-analysis operations
//!
//! # Note on syn::File
//!
//! Since `syn::File` is not `Send`, AST parsing and call graph construction
//! must happen outside of async effects. Use `build_call_graph_result` for
//! synchronous graph construction, then wrap results in effects for composition.
//!
//! # Example
//!
//! ```rust,ignore
//! use crate::analysis::call_graph::effects::{build_call_graph_result, analyze_dead_code_effect};
//!
//! // Build call graph synchronously
//! let graph = build_call_graph_result(&path, &ast, &config)?;
//!
//! // Analyze dead code as an effect
//! let effect = analyze_dead_code_effect(graph);
//! let analysis = run_effect(effect, config)?;
//! ```

use super::{AnalysisConfig, DeadCodeAnalysis, RustCallGraph, RustCallGraphBuilder};
use crate::analysis::effects::{lift_pure, query_config};
use crate::effects::AnalysisEffect;
use crate::env::RealEnv;
use crate::errors::AnalysisError;
use im::HashSet;
use stillwater::Effect;
use syn::File;

use crate::priority::call_graph::FunctionId;

/// Pure function to build a call graph.
///
/// This is the core logic without effect wrapping. Since `syn::File` is not
/// `Send`, call graph construction must happen synchronously.
fn build_call_graph_pure(
    file_path: &std::path::Path,
    ast: &File,
    config: &AnalysisConfig,
) -> anyhow::Result<RustCallGraph> {
    let mut builder = RustCallGraphBuilder::with_config(config.clone());

    builder.analyze_basic_calls(file_path, ast)?;
    builder.analyze_trait_dispatch(file_path, ast)?;
    builder.analyze_function_pointers(file_path, ast)?;
    builder.analyze_framework_patterns(file_path, ast)?;

    Ok(builder.build())
}

/// Analyze dead code using the call graph as an effect.
///
/// This wraps the dead code analysis in an effect for composition with
/// other effect-based operations.
pub fn analyze_dead_code_effect(graph: RustCallGraph) -> AnalysisEffect<Vec<DeadCodeAnalysis>> {
    let analysis = graph.analyze_dead_code();
    lift_pure(analysis)
}

/// Get live functions from a call graph as an effect.
pub fn get_live_functions_effect(graph: RustCallGraph) -> AnalysisEffect<HashSet<FunctionId>> {
    let live = graph.get_live_functions();
    lift_pure(live)
}

/// Get potential dead code from a call graph as an effect.
pub fn get_dead_code_effect(graph: RustCallGraph) -> AnalysisEffect<HashSet<FunctionId>> {
    let dead = graph.get_potential_dead_code();
    lift_pure(dead)
}

/// Query the analysis config from environment.
pub fn get_analysis_config_effect(
) -> impl Effect<Output = AnalysisConfig, Error = AnalysisError, Env = RealEnv> {
    query_config(get_analysis_config_from_debtmap_config)
}

// Helper function to extract analysis config from debtmap config
fn get_analysis_config_from_debtmap_config(
    config: &crate::config::DebtmapConfig,
) -> AnalysisConfig {
    // Use defaults for now; could be extended to read from config
    let analysis = config.analysis.as_ref();

    AnalysisConfig {
        enable_trait_analysis: analysis
            .and_then(|a| a.enable_trait_analysis)
            .unwrap_or(true),
        enable_function_pointer_tracking: analysis
            .and_then(|a| a.enable_function_pointer_tracking)
            .unwrap_or(true),
        enable_framework_patterns: analysis
            .and_then(|a| a.enable_framework_patterns)
            .unwrap_or(true),
        enable_cross_module_analysis: analysis
            .and_then(|a| a.enable_cross_module_analysis)
            .unwrap_or(true),
        max_analysis_depth: analysis.and_then(|a| a.max_analysis_depth).unwrap_or(10),
    }
}

// =============================================================================
// Backwards Compatibility Wrappers
// =============================================================================

/// Build a call graph (backwards-compatible wrapper).
///
/// This function builds a call graph synchronously since `syn::File` cannot
/// be used in async contexts (not `Send`).
pub fn build_call_graph_result(
    file_path: &std::path::Path,
    ast: &File,
    config: &crate::config::DebtmapConfig,
) -> anyhow::Result<RustCallGraph> {
    let analysis_config = get_analysis_config_from_debtmap_config(config);
    build_call_graph_pure(file_path, ast, &analysis_config)
}

/// Analyze dead code (backwards-compatible wrapper).
pub fn analyze_dead_code_result(graph: &RustCallGraph) -> Vec<DeadCodeAnalysis> {
    graph.analyze_dead_code()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DebtmapConfig;
    use crate::env::RealEnv;
    use quote::quote;
    use syn::parse2;

    fn create_test_ast() -> File {
        let tokens = quote! {
            fn main() {
                helper();
            }

            fn helper() {
                println!("Hello");
            }
        };
        parse2(tokens).unwrap()
    }

    #[tokio::test]
    async fn test_analyze_dead_code_effect() {
        let env = RealEnv::default();
        let graph = RustCallGraph::new();

        let effect = analyze_dead_code_effect(graph);
        let result = effect.run(&env).await.unwrap();

        // Empty graph has no dead code
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_get_analysis_config_effect() {
        let env = RealEnv::default();

        let effect = get_analysis_config_effect();
        let config = effect.run(&env).await.unwrap();

        // Default config should have analysis enabled
        assert!(config.enable_trait_analysis);
        assert!(config.enable_framework_patterns);
    }

    #[test]
    fn test_backwards_compat_build_call_graph() {
        use std::path::Path;

        let ast = create_test_ast();
        let path = Path::new("test.rs");
        let config = DebtmapConfig::default();

        let result = build_call_graph_result(path, &ast, &config);
        assert!(result.is_ok());
    }
}
