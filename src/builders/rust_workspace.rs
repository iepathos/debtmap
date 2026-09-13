//! Shared bounded Rust pipeline for serial and parallel source readers.
use super::parallel_call_graph::{CallGraphPhase, CallGraphProgress};
use crate::{
    analysis::call_graph::RustCallGraphBuilder,
    analyzers::rust_resolution::workspace,
    priority::call_graph::{CallGraph, FunctionId},
};
use anyhow::Result;
use rayon::prelude::*;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

pub(crate) mod identity;

type GraphResult = (CallGraph, HashSet<FunctionId>, HashSet<FunctionId>);

pub(super) fn build(
    paths: &[PathBuf],
    graph: CallGraph,
    parallel: bool,
    mut progress: impl FnMut(CallGraphProgress),
) -> Result<GraphResult> {
    progress(CallGraphProgress {
        phase: CallGraphPhase::ParsingASTs,
        current: 0,
        total: paths.len(),
    });
    let sources = snapshots(paths, parallel);
    let mut enhancement = RustCallGraphBuilder::new();
    let mut result = Ok(());
    let mut processed = 0;
    let (resolved, _) = workspace::extract_sources(&sources, |batch| {
        if result.is_ok() {
            result = collect_enhancement(&mut enhancement, batch);
        }
        processed += batch.len();
        progress(CallGraphProgress {
            phase: CallGraphPhase::ExtractingCalls,
            current: processed,
            total: sources.len(),
        });
    });
    result?;
    let graph = identity::merge(graph, resolved);
    progress(CallGraphProgress {
        phase: CallGraphPhase::LinkingModules,
        current: 0,
        total: 0,
    });
    enhancement.merge_base_graph(graph);
    enhancement.finalize_trait_analysis()?;
    let enhanced = enhancement.build();
    let exclusions = enhanced
        .framework_patterns
        .get_exclusions()
        .into_iter()
        .filter_map(|id| enhanced.base_graph.find_function(&id))
        .collect();
    let used = enhanced
        .function_pointer_tracker
        .get_definitely_used_functions()
        .into_iter()
        .filter_map(|id| enhanced.base_graph.find_function(&id))
        .collect();
    let mut graph = enhanced.base_graph;
    graph.resolve_cross_file_calls();
    Ok((graph, exclusions, used))
}

fn snapshots(paths: &[PathBuf], parallel: bool) -> Vec<(PathBuf, String)> {
    let mut sources: Vec<_> = if parallel {
        paths
            .par_iter()
            .filter_map(|path| read_snapshot(path))
            .collect()
    } else {
        paths
            .iter()
            .filter_map(|path| read_snapshot(path))
            .collect()
    };
    sources.sort_by(|left, right| left.0.cmp(&right.0));
    sources.dedup_by(|left, right| left.0 == right.0);
    sources
}

fn read_snapshot(path: &Path) -> Option<(PathBuf, String)> {
    match crate::io::read_file(path) {
        Ok(source) => Some((path.to_path_buf(), source)),
        Err(error) => {
            log::warn!("Cannot read Rust source {}: {error}", path.display());
            None
        }
    }
}

fn collect_enhancement(
    builder: &mut RustCallGraphBuilder,
    files: &[workspace::ParsedFile],
) -> Result<()> {
    for (path, ast) in files {
        builder
            .analyze_trait_dispatch(path, ast)?
            .analyze_function_pointers(path, ast)?
            .analyze_framework_patterns(path, ast)?;
    }
    builder.analyze_cross_module(files)?;
    Ok(())
}
