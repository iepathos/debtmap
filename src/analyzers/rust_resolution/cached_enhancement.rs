//! Enhancement consumes each active snapshot batch and final canonical identities.

use crate::analysis::call_graph::RustCallGraphBuilder;
use crate::priority::call_graph::{CallGraph, FunctionId};
use std::collections::HashSet;
use std::path::PathBuf;

pub(crate) struct EnhancedExtraction {
    pub graph: CallGraph,
    pub available: HashSet<PathBuf>,
    pub framework_exclusions: HashSet<FunctionId>,
    pub function_pointer_used: HashSet<FunctionId>,
}

pub(super) fn finish(
    mut builder: RustCallGraphBuilder,
    graph: CallGraph,
    available: HashSet<PathBuf>,
) -> EnhancedExtraction {
    builder.merge_base_graph(graph);
    if let Err(error) = builder.finalize_trait_analysis() {
        log::warn!("Cannot finalize cached Rust enhancements: {error:#}");
    }
    let enhanced = builder.build();
    let existing = |id: &FunctionId| enhanced.base_graph.get_function_info(id).is_some();
    let framework_exclusions = enhanced
        .framework_patterns
        .get_exclusions()
        .into_iter()
        .filter(existing)
        .collect();
    let function_pointer_used = enhanced
        .function_pointer_tracker
        .get_definitely_used_functions()
        .into_iter()
        .filter_map(|id| enhanced.base_graph.find_function(&id))
        .collect();
    EnhancedExtraction {
        graph: enhanced.base_graph,
        available,
        framework_exclusions,
        function_pointer_used,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enhancement_metadata_retains_no_thread_local_ast_or_spans() {
        fn owned<T: Send + Sync>() {}
        owned::<RustCallGraphBuilder>();
    }
}
