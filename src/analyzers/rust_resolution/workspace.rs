//! Snapshot-based workspace resolution with bounded AST lifetimes.
//!
//! Parsed syntax trees cannot leave the thread that parsed them, so each worker
//! collects declarations for a contiguous slice of the sorted sources and keeps
//! the trees that fit the parse cache. Bodies of those trees are resolved by the
//! same worker; the remaining sources are shared through a queue of small
//! chunks. Results are merged in source order and match a single-threaded run.
use super::{
    body::Body,
    index::{DeclarationCollector, WorkspaceIndex},
};
use crate::priority::call_graph::CallGraph;
use foldhash::{HashSet, HashSetExt};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread,
};
use syn::visit::Visit;

pub(crate) const BATCH_SIZE: usize = 200;
/// Files per shared body chunk; small enough to even out uneven resolution cost.
const QUEUE_CHUNK: usize = 8;
const WORKER_STACK_SIZE: usize = 16 * 1024 * 1024;
pub(crate) type ParsedFile = (PathBuf, syn::File);
type Source = (PathBuf, String);
type Declarations = (DeclarationCollector, HashSet<PathBuf>);

/// Capture declarations first; each body sees the same complete immutable index.
pub(crate) fn extract_sources(
    sources: &[Source],
    mut on_batch: impl FnMut(&[ParsedFile]),
) -> (CallGraph, HashSet<PathBuf>) {
    let known: HashSet<_> = sources.iter().map(|(file, _)| file.clone()).collect();
    let parts = partitions(sources, rayon::current_num_threads());
    let budget = crate::core::parsing::parse_cache_bytes() / parts.len().max(1);
    let splits: Vec<_> = parts
        .iter()
        .scan(0, |start, part| {
            let split = split(part, *start, budget);
            *start += part.len();
            Some(split)
        })
        .collect();
    let queue = Queue::new(&splits);
    thread::scope(|scope| {
        let workers: Vec<_> = splits
            .iter()
            .map(|split| Worker::spawn(scope, split, &queue, &known))
            .collect();
        let declarations = workers
            .iter()
            .filter_map(|worker| worker.declarations.recv().ok());
        let (index, available) = merge_declarations(&known, declarations);
        let index = Arc::new(index);
        for worker in &workers {
            let _ = worker.index.send(Arc::clone(&index));
        }
        for_each_parsed_batch(sources, &mut on_batch);
        let mut parts: Vec<_> = workers.into_iter().flat_map(Worker::join).collect();
        parts.sort_by_key(|(start, _)| *start);
        let graph = parts
            .into_iter()
            .fold(definitions(&index), |mut graph, (_, part)| {
                graph.merge(part);
                graph
            });
        super::report_counts(&graph);
        (graph, available)
    })
}

/// A partition's uncached prefix and cached suffix, with the suffix's source position.
struct Split<'a> {
    reparsed: &'a [Source],
    retained: &'a [Source],
    retained_start: usize,
}

fn split(part: &[Source], start: usize, budget: usize) -> Split<'_> {
    // At least half of every partition goes through the queue to balance workers.
    let half = part.iter().map(|(_, text)| text.len()).sum::<usize>() / 2;
    let at = retained_from(part, budget.min(half));
    let (reparsed, retained) = part.split_at(at);
    Split {
        reparsed,
        retained,
        retained_start: start + at,
    }
}

/// Uncached sources of every partition, in source order, handed out chunk by chunk.
struct Queue<'a> {
    chunks: Vec<(usize, &'a [Source])>,
    next: AtomicUsize,
}

impl<'a> Queue<'a> {
    fn new(splits: &[Split<'a>]) -> Self {
        let chunks = splits
            .iter()
            .flat_map(|split| {
                let start = split.retained_start - split.reparsed.len();
                split
                    .reparsed
                    .chunks(QUEUE_CHUNK)
                    .enumerate()
                    .map(move |(i, chunk)| (start + i * QUEUE_CHUNK, chunk))
            })
            .collect();
        Self {
            chunks,
            next: AtomicUsize::new(0),
        }
    }

    fn pop(&self) -> Option<(usize, &'a [Source])> {
        self.chunks
            .get(self.next.fetch_add(1, Ordering::Relaxed))
            .copied()
    }
}

/// Split sorted sources into at most `workers` contiguous slices of similar size.
fn partitions(sources: &[Source], workers: usize) -> Vec<&[Source]> {
    let total: usize = sources.iter().map(|(_, text)| text.len()).sum();
    let target = total.div_ceil(workers.max(1)).max(1);
    let mut parts = Vec::new();
    let (mut start, mut size) = (0, 0);
    for (end, (_, text)) in sources.iter().enumerate() {
        size += text.len();
        if size >= target {
            parts.push(&sources[start..=end]);
            (start, size) = (end + 1, 0);
        }
    }
    if start < sources.len() {
        parts.push(&sources[start..]);
    }
    parts
}

/// First file of the longest suffix whose source fits in the retention budget.
fn retained_from(files: &[Source], budget: usize) -> usize {
    let mut used = 0;
    files
        .iter()
        .rposition(|(_, text)| {
            used += text.len();
            used > budget
        })
        .map(|position| position + 1)
        .unwrap_or(0)
}

/// Resolved graphs keyed by the source position of their first file.
type Resolved = Vec<(usize, CallGraph)>;

struct Worker<'scope> {
    declarations: mpsc::Receiver<Declarations>,
    index: mpsc::Sender<Arc<WorkspaceIndex>>,
    handle: thread::ScopedJoinHandle<'scope, Resolved>,
}

impl<'scope> Worker<'scope> {
    fn spawn<'env>(
        scope: &'scope thread::Scope<'scope, 'env>,
        split: &'env Split<'env>,
        queue: &'env Queue<'env>,
        known: &'env HashSet<PathBuf>,
    ) -> Self {
        let (declarations_tx, declarations) = mpsc::channel();
        let (index, index_rx) = mpsc::channel();
        let handle = thread::Builder::new()
            .name("rust-resolution".into())
            .stack_size(WORKER_STACK_SIZE)
            .spawn_scoped(scope, move || {
                resolve_partition(split, queue, known, declarations_tx, index_rx)
            })
            .expect("failed to spawn Rust resolution worker");
        Self {
            declarations,
            index,
            handle,
        }
    }

    fn join(self) -> Resolved {
        drop(self.index);
        self.handle
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    }
}

fn resolve_partition(
    split: &Split<'_>,
    queue: &Queue<'_>,
    known: &HashSet<PathBuf>,
    declarations: mpsc::Sender<Declarations>,
    index: mpsc::Receiver<Arc<WorkspaceIndex>>,
) -> Resolved {
    let (collected, retained) = collect_partition(split.reparsed, split.retained, known);
    if declarations.send(collected).is_err() {
        return Vec::new();
    }
    let Ok(index) = index.recv() else {
        return Vec::new();
    };
    resolve_bodies(&index, (split.retained_start, retained), queue)
}

/// Parsed trees for `retained` stay alive on this thread for the body pass.
fn collect_partition(
    reparsed: &[Source],
    retained: &[Source],
    known: &HashSet<PathBuf>,
) -> (Declarations, Vec<ParsedFile>) {
    let mut collector = DeclarationCollector::new(known.clone());
    let mut available = HashSet::new();
    let mut collect = |parsed: &[ParsedFile]| {
        available.extend(parsed.iter().map(|(file, _)| file.clone()));
        collector.collect(parsed);
    };
    for_each_parsed_batch(reparsed, &mut collect);
    let retained = parse_batch(retained);
    collect(&retained);
    ((collector, available), retained)
}

/// Cached trees first, then shared chunks; graphs carry only edges, uncertainty and effects.
fn resolve_bodies(
    index: &WorkspaceIndex,
    (retained_start, retained): (usize, Vec<ParsedFile>),
    queue: &Queue<'_>,
) -> Resolved {
    let mut graph = definitions(index);
    let mut resolved = Vec::new();
    if !retained.is_empty() {
        analyze(index, &retained, &mut graph);
        resolved.push((retained_start, take_resolutions(&mut graph)));
    }
    drop(retained);
    crate::core::parsing::reset_span_locations();
    while let Some((start, chunk)) = queue.pop() {
        let parsed = parse_batch(chunk);
        analyze(index, &parsed, &mut graph);
        resolved.push((start, take_resolutions(&mut graph)));
        drop(parsed);
        crate::core::parsing::reset_span_locations();
    }
    resolved
}

/// Moves recorded results out, leaving only the definitions resolution checks against.
fn take_resolutions(graph: &mut CallGraph) -> CallGraph {
    let mut seeded = CallGraph::new();
    seeded.nodes = std::mem::take(&mut graph.nodes);
    std::mem::replace(graph, seeded)
}

fn merge_declarations(
    known: &HashSet<PathBuf>,
    parts: impl Iterator<Item = Declarations>,
) -> (WorkspaceIndex, HashSet<PathBuf>) {
    let (collector, available) = parts.fold(
        (DeclarationCollector::new(known.clone()), HashSet::new()),
        |(mut collector, mut available), (part, files)| {
            collector.absorb(part);
            available.extend(files);
            (collector, available)
        },
    );
    (collector.finish(), available)
}

fn for_each_parsed_batch(sources: &[Source], mut visit: impl FnMut(&[ParsedFile])) {
    for batch in sources.chunks(BATCH_SIZE) {
        let parsed = parse_batch(batch);
        visit(&parsed);
        drop(parsed);
        crate::core::parsing::reset_span_locations();
    }
}

fn parse_batch(sources: &[Source]) -> Vec<ParsedFile> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sources(sizes: &[usize]) -> Vec<Source> {
        sizes
            .iter()
            .enumerate()
            .map(|(i, size)| (PathBuf::from(format!("src/f{i}.rs")), "x".repeat(*size)))
            .collect()
    }

    #[test]
    fn partitions_are_contiguous_and_bounded() {
        let files = sources(&[5, 1, 1, 1, 9, 2, 3]);
        for workers in 1..=8 {
            let parts = partitions(&files, workers);
            assert!(parts.len() <= workers);
            assert_eq!(parts.concat(), files);
        }
    }

    #[test]
    fn retained_suffix_fits_budget() {
        let files = sources(&[4, 4, 4]);
        assert_eq!(retained_from(&files, 0), 3);
        assert_eq!(retained_from(&files, 7), 2);
        assert_eq!(retained_from(&files, 8), 1);
        assert_eq!(retained_from(&files, 100), 0);
    }

    #[test]
    fn partitioned_resolution_matches_single_thread() {
        let files: Vec<Source> = (0..12)
            .map(|i| {
                let next = (i + 1) % 12;
                let source = format!(
                    "pub struct S{i};\nimpl S{i} {{ pub fn run(&self) {{ crate::f{next}::helper(); }} }}\n\
                     pub fn helper() {{ let s = S{i}; s.run(); unknown.call(); }}\n"
                );
                (PathBuf::from(format!("src/f{i}.rs")), source)
            })
            .collect();
        let pool = |threads| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .expect("test pool")
        };
        let run = |threads, cache_mb| {
            crate::core::parsing::set_parse_cache_mb(cache_mb);
            let graph = pool(threads).install(|| extract_sources(&files, |_| {}).0);
            let mut nodes: Vec<_> = graph.get_all_functions().cloned().collect();
            nodes.sort();
            format!(
                "{nodes:?}\n{:?}\n{:?}\n{:?}",
                graph.edge_evidence().collect::<Vec<_>>(),
                graph.uncertain_calls().collect::<Vec<_>>(),
                graph.effect_assessments().collect::<Vec<_>>()
            )
        };
        let expected = run(1, 0);
        assert_eq!(run(4, 0), expected);
        assert_eq!(run(4, 64), expected);
        assert_eq!(run(3, 64), expected);
        crate::core::parsing::set_parse_cache_mb(crate::core::parsing::DEFAULT_PARSE_CACHE_MB);
    }
}
