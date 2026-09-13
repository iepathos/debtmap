//! Snapshot-based workspace resolution with bounded AST lifetimes.
use super::{
    body::Body,
    index::{DeclarationCollector, WorkspaceIndex},
};
use crate::priority::call_graph::CallGraph;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use syn::visit::Visit;

pub(crate) const BATCH_SIZE: usize = 200;
pub(crate) type ParsedFile = (PathBuf, syn::File);

/// Capture declarations first; each body sees the same complete immutable index.
pub(crate) fn extract_sources(
    sources: &[(PathBuf, String)],
    mut on_batch: impl FnMut(&[ParsedFile]),
) -> (CallGraph, HashSet<PathBuf>) {
    let (index, available) = collect_declarations(sources);
    let mut graph = definitions(&index);
    for batch in sources.chunks(BATCH_SIZE) {
        let parsed = parse_batch(batch);
        analyze(&index, &parsed, &mut graph);
        on_batch(&parsed);
        drop(parsed);
        crate::core::parsing::reset_span_locations();
    }
    super::report_counts(&graph);
    (graph, available)
}

fn collect_declarations(sources: &[(PathBuf, String)]) -> (WorkspaceIndex, HashSet<PathBuf>) {
    let mut collector =
        DeclarationCollector::new(sources.iter().map(|(file, _)| file.clone()).collect());
    let mut available = HashSet::new();
    for batch in sources.chunks(BATCH_SIZE) {
        let parsed = parse_batch(batch);
        available.extend(parsed.iter().map(|(file, _)| file.clone()));
        collector.collect(&parsed);
        drop(parsed);
        crate::core::parsing::reset_span_locations();
    }
    (collector.finish(), available)
}

fn parse_batch(sources: &[(PathBuf, String)]) -> Vec<ParsedFile> {
    sources
        .iter()
        .filter_map(|(file, source)| match syn::parse_file(source) {
            Ok(ast) => Some((file.clone(), ast)),
            Err(error) => {
                log::warn!("Cannot resolve Rust source {}: {error}", file.display());
                None
            }
        })
        .collect()
}

pub(super) fn definitions(index: &WorkspaceIndex) -> CallGraph {
    let mut graph = CallGraph::new();
    for call in index.callables().iter().filter(|call| call.has_body) {
        graph.add_function(call.id.clone(), false, call.is_test, 0, 0);
    }
    graph
}

pub(super) fn analyze(index: &WorkspaceIndex, files: &[ParsedFile], graph: &mut CallGraph) {
    for (file, ast) in files {
        BodyVisitor { index, graph, file }.visit_file(ast);
    }
}

struct BodyVisitor<'a> {
    index: &'a WorkspaceIndex,
    graph: &'a mut CallGraph,
    file: &'a Path,
}

impl BodyVisitor<'_> {
    fn analyze(&mut self, signature: &syn::Signature, block: &syn::Block) {
        let start = signature.ident.span().start();
        if let Some(callable) = self.index.callable_at(self.file, start.line, start.column) {
            Body::new(self.index, callable, self.graph).analyze(signature, block);
        }
    }
}

impl<'ast> Visit<'ast> for BodyVisitor<'_> {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.analyze(&item.sig, &item.block);
    }
    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.analyze(&item.sig, &item.block);
    }
    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if let Some(block) = &item.default {
            self.analyze(&item.sig, block);
        }
    }
}
