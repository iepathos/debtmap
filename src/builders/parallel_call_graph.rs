use crate::{
    config,
    core::Language,
    io,
    priority::{
        call_graph::{CallGraph, FunctionId},
        parallel_call_graph::ParallelConfig,
    },
};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Call graph construction phases for progress tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallGraphPhase {
    DiscoveringFiles,
    ParsingASTs,
    ExtractingCalls,
    LinkingModules,
}

/// Progress information for call graph construction
#[derive(Debug, Clone)]
pub struct CallGraphProgress {
    pub phase: CallGraphPhase,
    pub current: usize,
    pub total: usize,
}

/// Parallel call graph builder for Rust projects
pub struct ParallelCallGraphBuilder;

impl Default for ParallelCallGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ParallelCallGraphBuilder {
    pub fn new() -> Self {
        Self
    }

    pub fn with_config(_config: ParallelConfig) -> Self {
        // Config is no longer used as thread pool is configured globally
        Self
    }

    /// Build call graph with parallel processing
    pub fn build_parallel<F>(
        &self,
        project_path: &Path,
        base_graph: CallGraph,
        progress_callback: F,
    ) -> Result<(CallGraph, HashSet<FunctionId>, HashSet<FunctionId>)>
    where
        F: FnMut(CallGraphProgress) + Send + Sync,
    {
        self.build_parallel_with_files(project_path, base_graph, None, progress_callback)
    }

    /// Build call graph with parallel processing, using optional pre-discovered files
    ///
    /// If `rust_files` is provided, skips file discovery and uses the given files.
    /// This avoids redundant filesystem walking when files were already discovered.
    ///
    /// Spec 210: Uses batched processing to prevent proc-macro2 SourceMap overflow.
    /// Files are processed in batches of ~200 files, with SourceMap reset between batches.
    pub fn build_parallel_with_files<F>(
        &self,
        project_path: &Path,
        base_graph: CallGraph,
        rust_files: Option<&[PathBuf]>,
        mut progress_callback: F,
    ) -> Result<(CallGraph, HashSet<FunctionId>, HashSet<FunctionId>)>
    where
        F: FnMut(CallGraphProgress) + Send + Sync,
    {
        let discovered_files: Vec<PathBuf>;
        let rust_files = match rust_files {
            Some(files) => {
                // Skip discover phase - files already known from stage 0
                log::info!("Using {} pre-discovered Rust files", files.len());
                files
            }
            None => {
                // Phase 1: Discover files (only when not pre-discovered)
                progress_callback(CallGraphProgress {
                    phase: CallGraphPhase::DiscoveringFiles,
                    current: 0,
                    total: 0,
                });

                let config = config::get_config();
                discovered_files = io::walker::find_project_files_with_config(
                    project_path,
                    vec![Language::Rust],
                    config,
                )
                .context("Failed to find Rust files for call graph")?;
                log::info!("Discovered {} Rust files", discovered_files.len());

                // Mark discover phase complete
                progress_callback(CallGraphProgress {
                    phase: CallGraphPhase::DiscoveringFiles,
                    current: discovered_files.len(),
                    total: discovered_files.len(),
                });

                &discovered_files
            }
        };

        super::rust_workspace::build(rust_files, base_graph, true, progress_callback)
    }
}

/// Parallel processing entry point for call graph construction
pub fn build_call_graph_parallel<F>(
    project_path: &Path,
    base_graph: CallGraph,
    num_threads: Option<usize>,
    progress_callback: F,
) -> Result<(CallGraph, HashSet<FunctionId>, HashSet<FunctionId>)>
where
    F: FnMut(CallGraphProgress) + Send + Sync,
{
    build_call_graph_parallel_with_files(
        project_path,
        base_graph,
        num_threads,
        None,
        progress_callback,
    )
}

/// Parallel processing entry point with optional pre-discovered files
///
/// If `rust_files` is provided, skips file discovery and uses the given files.
/// This avoids redundant filesystem walking when files were already discovered.
pub fn build_call_graph_parallel_with_files<F>(
    project_path: &Path,
    base_graph: CallGraph,
    num_threads: Option<usize>,
    rust_files: Option<&[PathBuf]>,
    progress_callback: F,
) -> Result<(CallGraph, HashSet<FunctionId>, HashSet<FunctionId>)>
where
    F: FnMut(CallGraphProgress) + Send + Sync,
{
    let mut config = ParallelConfig::default();

    if let Some(threads) = num_threads {
        config = config.with_threads(threads);
    }

    let builder = ParallelCallGraphBuilder::with_config(config);
    builder.build_parallel_with_files(project_path, base_graph, rust_files, progress_callback)
}

// ============================================================================
// Spec 213: Call Graph Building from Extracted Data
// ============================================================================

use crate::extraction::{ExtractedFileData, ExtractedFunctionData};
use std::collections::HashMap;

/// Build call graph from pre-extracted file data (spec 213).
///
/// Resolves Rust snapshots together against complete declarations. Other languages
/// and legacy records use their available extracted call information.
///
/// # Arguments
///
/// * `base_graph` - Base call graph from function metrics
/// * `extracted` - Pre-extracted file data from unified extraction phase
///
/// # Returns
///
/// Tuple of (CallGraph, framework_exclusions, function_pointer_used)
pub fn build_call_graph_from_extracted(
    base_graph: CallGraph,
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> (CallGraph, HashSet<FunctionId>, HashSet<FunctionId>) {
    use crate::priority::call_graph::CallType as GraphCallType;

    let sorted_extracted = extracted_files_sorted(extracted);
    let crate::analyzers::rust_resolution::cached::EnhancedExtraction {
        graph: rust_graph,
        available: rust_snapshots,
        framework_exclusions,
        function_pointer_used,
    } = crate::analyzers::rust_resolution::cached::extract_with_enhancement(extracted);
    let callee_index = CalleeResolutionIndex::from_sorted_extracted(&sorted_extracted);
    let mut final_graph = base_graph;

    for (path, file_data) in &sorted_extracted {
        for func in &file_data.functions {
            let function_id = extracted_function_id(path, func);
            if final_graph.get_function_info(&function_id).is_some()
                || rust_graph.find_function(&function_id).is_some()
            {
                continue;
            }
            let facts = crate::analysis::role_policy::evidence_for_facts(
                crate::analysis::role_policy::RoleFacts {
                    path,
                    language: crate::core::Language::from_path(path),
                    name: &func.qualified_name,
                    is_test: func.is_test,
                    in_test_module: func.in_test_module,
                    visibility: func.visibility.as_deref(),
                },
            );
            let evidence =
                crate::analysis::role_policy::merge_evidence(&facts, &func.role_evidence);
            final_graph.add_function_with_evidence(
                function_id,
                evidence,
                func.cyclomatic,
                func.length,
            );
        }
    }

    let mut final_graph = super::rust_workspace::identity::merge(final_graph, rust_graph);
    for (path, file_data) in sorted_extracted {
        if rust_snapshots.contains(path) {
            continue;
        }
        for func in &file_data.functions {
            let caller = extracted_function_id(path, func);
            for (ordinal, call) in func.calls.iter().enumerate() {
                if rust_call_requires_source(path, call) {
                    record_missing_rust_source(&mut final_graph, caller.clone(), call, ordinal);
                    continue;
                }
                let outcome = resolve_callee_from_extracted(call, &caller, path, &callee_index);
                final_graph.add_resolution(caller.clone(), GraphCallType::Direct, outcome);
            }
        }
    }

    log::info!(
        "Call graph from extracted data: {} nodes in {} files",
        final_graph.node_count(),
        extracted.len()
    );

    (final_graph, framework_exclusions, function_pointer_used)
}

struct CalleeResolutionIndex {
    same_file_functions: HashMap<PathBuf, HashMap<String, Vec<FunctionId>>>,
    qualified_functions: HashMap<String, Vec<FunctionId>>,
    method_functions: HashMap<String, Vec<FunctionId>>,
    python_imports: super::python_call_resolution::PythonImportIndex,
}

fn extracted_files_sorted(
    extracted: &HashMap<PathBuf, ExtractedFileData>,
) -> Vec<(&PathBuf, &ExtractedFileData)> {
    let mut files: Vec<_> = extracted.iter().collect();
    files.sort_by(|left, right| left.0.cmp(right.0));
    files
}

impl CalleeResolutionIndex {
    fn from_sorted_extracted(sorted_extracted: &[(&PathBuf, &ExtractedFileData)]) -> Self {
        let python_imports =
            super::python_call_resolution::PythonImportIndex::from_extracted(sorted_extracted);
        let mut index = sorted_extracted
            .iter()
            .fold(Self::empty(), |mut index, item| {
                index.add_file_functions(item.0, &item.1.functions);
                index
            });
        index.python_imports = python_imports;
        index
    }

    fn empty() -> Self {
        Self {
            same_file_functions: HashMap::new(),
            qualified_functions: HashMap::new(),
            method_functions: HashMap::new(),
            python_imports: super::python_call_resolution::PythonImportIndex::default(),
        }
    }

    fn add_file_functions(&mut self, path: &Path, functions: &[ExtractedFunctionData]) {
        let file_functions = self
            .same_file_functions
            .entry(path.to_path_buf())
            .or_default();
        for func in functions {
            let function_id = extracted_function_id(path, func);
            add_same_file_function(file_functions, func, &function_id);
            add_candidate(
                &mut self.qualified_functions,
                &func.qualified_name,
                &function_id,
            );
            add_candidate(&mut self.method_functions, &func.name, &function_id);
        }
    }
}

fn extracted_function_id(path: &Path, func: &ExtractedFunctionData) -> FunctionId {
    FunctionId::new(path.to_path_buf(), func.qualified_name.clone(), func.line)
        .with_column(func.column)
}

fn add_same_file_function(
    file_functions: &mut HashMap<String, Vec<FunctionId>>,
    func: &ExtractedFunctionData,
    function_id: &FunctionId,
) {
    add_candidate(file_functions, &func.qualified_name, function_id);
    add_candidate(file_functions, &func.name, function_id);
}

fn add_candidate(
    functions: &mut HashMap<String, Vec<FunctionId>>,
    key: &str,
    function_id: &FunctionId,
) {
    let candidates = functions.entry(key.to_string()).or_default();
    if !candidates.contains(function_id) {
        candidates.push(function_id.clone());
        candidates.sort();
    }
}

/// Resolve a callee name to a FunctionId using extracted data.
fn resolve_callee_from_extracted(
    call: &crate::extraction::CallSite,
    caller: &FunctionId,
    caller_file: &Path,
    index: &CalleeResolutionIndex,
) -> crate::priority::call_graph::ResolutionOutcome {
    use crate::extraction::CallType;
    use crate::priority::call_graph::ResolutionOutcome;

    if rust_call_requires_source(caller_file, call) {
        return ResolutionOutcome::Unresolved {
            query: call.callee_name.clone(),
        };
    }

    match call.call_type {
        CallType::Direct | CallType::StaticMethod | CallType::TraitMethod => {
            resolve_direct_callee(call, caller_file, index)
        }
        CallType::Method => {
            if crate::core::Language::from_path(caller_file) == crate::core::Language::Python {
                return resolve_python_method(call, caller, caller_file, index);
            }
            if crate::analyzers::call_graph::CallResolver::is_common_library_method(
                &call.callee_name,
            ) {
                return ResolutionOutcome::Ignored {
                    reason: format!("common library method {}", call.callee_name),
                };
            }
            let candidates = index
                .method_functions
                .get(&call.callee_name)
                .cloned()
                .unwrap_or_default();
            outcome_for_candidates(
                candidates,
                call,
                caller_file,
                crate::priority::call_graph::CallEdgeProvenance::NameHeuristic,
                60,
            )
        }
        CallType::Closure | CallType::FunctionPointer => ResolutionOutcome::Ignored {
            reason: "dynamic callable".to_string(),
        },
    }
}

fn rust_call_requires_source(path: &Path, call: &crate::extraction::CallSite) -> bool {
    use crate::extraction::CallType;
    Language::from_path(path) == Language::Rust
        && matches!(
            call.call_type,
            CallType::Method | CallType::StaticMethod | CallType::TraitMethod
        )
}

fn record_missing_rust_source(
    graph: &mut CallGraph,
    caller: FunctionId,
    call: &crate::extraction::CallSite,
    ordinal: usize,
) {
    use crate::priority::call_graph::{CallSite, CallType, UncertainCall, UncertaintyReason};
    graph.record_uncertain_call(UncertainCall {
        call_ordinal: Some(ordinal),
        call_site: CallSite {
            file: caller.file.clone(),
            line: call.line,
            column: None,
        },
        lexical_module: caller.module_path.clone(),
        caller,
        call_type: CallType::Direct,
        query: call.callee_name.clone(),
        receiver: None,
        candidates: Vec::new(),
        reason: UncertaintyReason::UnavailableDefinition,
    });
}

fn resolve_python_method(
    call: &crate::extraction::CallSite,
    caller: &FunctionId,
    caller_file: &Path,
    index: &CalleeResolutionIndex,
) -> crate::priority::call_graph::ResolutionOutcome {
    use crate::priority::call_graph::{CallEdgeProvenance, ResolutionOutcome};

    let imported = index
        .python_imports
        .candidates(caller_file, &call.callee_name);
    if !imported.is_empty() {
        return outcome_for_candidates(
            imported,
            call,
            caller_file,
            CallEdgeProvenance::ImportResolution,
            90,
        );
    }
    let Some((receiver, method)) = call.callee_name.rsplit_once('.') else {
        return ResolutionOutcome::Unresolved {
            query: call.callee_name.clone(),
        };
    };
    let qualified_name = if receiver == "self" {
        caller
            .name
            .rsplit_once('.')
            .map(|(owner, _)| format!("{owner}.{method}"))
    } else if !receiver.contains('.') && receiver.starts_with(char::is_uppercase) {
        Some(call.callee_name.clone())
    } else {
        None
    };
    let Some(qualified_name) = qualified_name else {
        return ResolutionOutcome::Unresolved {
            query: call.callee_name.clone(),
        };
    };
    let candidates = index
        .same_file_functions
        .get(caller_file)
        .and_then(|functions| functions.get(&qualified_name))
        .cloned()
        .unwrap_or_default();
    outcome_for_candidates(
        candidates,
        call,
        caller_file,
        CallEdgeProvenance::TypeResolution,
        95,
    )
}

fn resolve_direct_callee(
    call: &crate::extraction::CallSite,
    caller_file: &Path,
    index: &CalleeResolutionIndex,
) -> crate::priority::call_graph::ResolutionOutcome {
    let local = index
        .same_file_functions
        .get(caller_file)
        .and_then(|functions| functions.get(&call.callee_name))
        .cloned()
        .unwrap_or_default();
    if !local.is_empty() {
        return outcome_for_candidates(
            local,
            call,
            caller_file,
            crate::priority::call_graph::CallEdgeProvenance::AstDirect,
            100,
        );
    }
    let imported = index
        .python_imports
        .candidates(caller_file, &call.callee_name);
    if !imported.is_empty() {
        return outcome_for_candidates(
            imported,
            call,
            caller_file,
            crate::priority::call_graph::CallEdgeProvenance::ImportResolution,
            90,
        );
    }
    if crate::core::Language::from_path(caller_file) == crate::core::Language::Python {
        return crate::priority::call_graph::ResolutionOutcome::Unresolved {
            query: call.callee_name.clone(),
        };
    }
    let global = index
        .qualified_functions
        .get(&call.callee_name)
        .cloned()
        .unwrap_or_default();
    outcome_for_candidates(
        global,
        call,
        caller_file,
        crate::priority::call_graph::CallEdgeProvenance::ImportResolution,
        85,
    )
}

fn outcome_for_candidates(
    mut candidates: Vec<FunctionId>,
    call: &crate::extraction::CallSite,
    caller_file: &Path,
    provenance: crate::priority::call_graph::CallEdgeProvenance,
    confidence: u8,
) -> crate::priority::call_graph::ResolutionOutcome {
    use crate::priority::call_graph::{CallSite, ResolutionOutcome};
    candidates.sort();
    match candidates.as_slice() {
        [target] => ResolutionOutcome::Resolved {
            target: target.clone(),
            provenance,
            confidence,
            call_site: Some(CallSite {
                file: caller_file.to_path_buf(),
                line: call.line,
                column: None,
            }),
        },
        [] => ResolutionOutcome::Unresolved {
            query: call.callee_name.clone(),
        },
        _ => ResolutionOutcome::Ambiguous { candidates },
    }
}

#[cfg(test)]
mod extracted_call_resolution_tests {
    use super::*;
    use crate::extraction::{
        CallType, ExtractedFileData, ExtractedFunctionData, ImportInfo, ImportKind,
    };

    #[test]
    fn direct_calls_prefer_same_file_before_qualified_index() {
        let caller = PathBuf::from("src/caller.rs");
        let other = PathBuf::from("src/other.rs");
        let extracted = extracted_files(vec![
            (
                caller.clone(),
                vec![function("helper", "local::helper", 10)],
            ),
            (other, vec![function("helper", "helper", 20)]),
        ]);
        let index = CalleeResolutionIndex::from_sorted_extracted(&sorted(&extracted));

        let caller_id = FunctionId::new(caller.clone(), "entry".to_string(), 1);
        let outcome = resolve_callee_from_extracted(
            &call("helper", CallType::Direct, 6),
            &caller_id,
            &caller,
            &index,
        );
        let crate::priority::call_graph::ResolutionOutcome::Resolved {
            target: resolved, ..
        } = outcome
        else {
            panic!("expected resolved call, got {outcome:?}");
        };

        assert_eq!(resolved.file, caller);
        assert_eq!(resolved.name, "local::helper");
        assert_eq!(resolved.line, 10);
    }

    #[test]
    fn ambiguous_method_names_produce_no_resolved_target() {
        let first = PathBuf::from("src/a.rs");
        let second = PathBuf::from("src/b.rs");
        let mut entry = function("entry", "entry", 1);
        entry.calls = vec![call("run", CallType::Method, 2)];
        let extracted = extracted_files(vec![
            (second, vec![function("run", "Second::run", 20)]),
            (
                first.clone(),
                vec![entry, function("run", "First::run", 10)],
            ),
        ]);
        let index = CalleeResolutionIndex::from_sorted_extracted(&sorted(&extracted));

        let caller_id = FunctionId::new(first.clone(), "entry".to_string(), 1);
        let outcome = resolve_callee_from_extracted(
            &call("run", CallType::Method, 6),
            &caller_id,
            &first,
            &index,
        );

        assert!(matches!(
            outcome,
            crate::priority::call_graph::ResolutionOutcome::Unresolved { .. }
        ));
        let (graph, _, _) = build_call_graph_from_extracted(CallGraph::new(), &extracted);
        let entry_id = FunctionId::new(first, "entry".to_string(), 1);
        assert!(graph.get_callees_exact(&entry_id).is_empty());
        assert_eq!(graph.edge_evidence().count(), 0);
    }

    #[test]
    fn missing_rust_context_does_not_promote_common_method_names() {
        let caller = PathBuf::from("src/builders/parallel_unified_analysis.rs");
        let support = PathBuf::from("src/support.rs");
        let extracted = extracted_files(vec![
            (caller.clone(), vec![function("entry", "entry", 5)]),
            (
                support,
                vec![
                    function("filter", "LazyPipeline::filter", 10),
                    function("map", "LazyPipeline::map", 20),
                    function("take", "LazyPipeline::take", 30),
                    function("get", "PurityCache::get", 40),
                ],
            ),
        ]);
        let index = CalleeResolutionIndex::from_sorted_extracted(&sorted(&extracted));

        for method in ["filter", "map", "take", "get"] {
            let caller_id = FunctionId::new(caller.clone(), "entry".to_string(), 1);
            let outcome = resolve_callee_from_extracted(
                &call(method, CallType::Method, 6),
                &caller_id,
                &caller,
                &index,
            );

            assert!(
                matches!(
                    outcome,
                    crate::priority::call_graph::ResolutionOutcome::Unresolved { .. }
                ),
                "method {method} without source context must remain unresolved, got {outcome:?}"
            );
        }
    }

    #[test]
    fn legacy_extracted_rust_preserves_direct_edges_and_method_uncertainty() {
        let caller = PathBuf::from("src/caller.rs");
        let helper = PathBuf::from("src/helper.rs");
        let mut entry = function("entry", "entry", 5);
        entry.calls = vec![
            crate::extraction::CallSite {
                callee_name: "local_helper".to_string(),
                call_type: CallType::Direct,
                line: 6,
            },
            crate::extraction::CallSite {
                callee_name: "Helper::remote".to_string(),
                call_type: CallType::StaticMethod,
                line: 7,
            },
            crate::extraction::CallSite {
                callee_name: "run".to_string(),
                call_type: CallType::Method,
                line: 8,
            },
        ];

        let extracted = extracted_files(vec![
            (
                caller.clone(),
                vec![entry, function("local_helper", "local_helper", 20)],
            ),
            (
                helper.clone(),
                vec![
                    function("remote", "Helper::remote", 10),
                    function("run", "Helper::run", 30),
                ],
            ),
        ]);

        let (graph, _, _) = build_call_graph_from_extracted(CallGraph::new(), &extracted);
        let entry_id = FunctionId::new(caller.clone(), "entry".to_string(), 5);
        let callees = graph.get_callees_exact(&entry_id);
        let callee_names: Vec<_> = callees.iter().map(|id| id.name.as_str()).collect();

        assert_eq!(callees.len(), 1);
        assert!(callee_names.contains(&"local_helper"));
        assert!(!callee_names.contains(&"Helper::remote"));
        assert!(!callee_names.contains(&"Helper::run"));
        assert_eq!(graph.uncertain_calls().count(), 2);
        let evidence: Vec<_> = graph.edge_evidence().collect();
        assert_eq!(evidence.len(), 1);
        assert!(evidence.iter().all(|edge| edge.confidence > 0));
        assert!(evidence.iter().all(|edge| edge.call_site.is_some()));
    }

    #[test]
    fn extracted_nodes_do_not_overwrite_base_roles() {
        let path = PathBuf::from("src/entry.py");
        let function_id = FunctionId::new(path.clone(), "main".to_string(), 1);
        let roles = crate::analysis::role_policy::CodeRoles {
            is_test: false,
            is_entry_point: true,
            is_framework_managed: true,
            is_public_api: true,
        };
        let mut base_graph = CallGraph::new();
        base_graph.add_function_with_roles(function_id.clone(), roles, 1, 2);
        let extracted = extracted_files(vec![(path, vec![function("main", "main", 1)])]);

        let (graph, _, _) = build_call_graph_from_extracted(base_graph, &extracted);

        assert_eq!(graph.nodes[&function_id].roles, roles);
    }

    #[test]
    fn python_self_calls_resolve_only_within_the_callers_class() {
        let path = PathBuf::from("src/service.py");
        let mut run = function("run", "First.run", 1);
        run.calls = vec![
            call("self.validate", CallType::Method, 2),
            call("service.validate", CallType::Method, 3),
        ];
        let extracted = extracted_files(vec![(
            path.clone(),
            vec![
                run,
                function("validate", "First.validate", 10),
                function("validate", "Second.validate", 20),
            ],
        )]);

        let (graph, _, _) = build_call_graph_from_extracted(CallGraph::new(), &extracted);
        let caller = FunctionId::new(path.clone(), "First.run".to_string(), 1);
        let callees = graph.get_callees_exact(&caller);
        let evidence: Vec<_> = graph.edge_evidence().collect();

        assert_eq!(
            callees,
            vec![FunctionId::new(path, "First.validate".into(), 10)]
        );
        assert_eq!(evidence.len(), 1);
        assert_eq!(
            evidence[0].provenance,
            crate::priority::call_graph::CallEdgeProvenance::TypeResolution
        );
        assert_eq!(evidence[0].confidence, 95);
        assert_eq!(evidence[0].call_site.as_ref().unwrap().line, 2);
    }

    #[test]
    fn python_import_aliases_resolve_to_unique_module_symbols() {
        let caller_path = PathBuf::from("src/app.py");
        let helper_path = PathBuf::from("src/helpers.py");
        let mut caller_file = ExtractedFileData::empty(caller_path.clone());
        caller_file.functions = vec![function("entry", "entry", 1)];
        caller_file.imports = vec![
            ImportInfo {
                path: "helpers.work".to_string(),
                alias: Some("run".to_string()),
                is_glob: false,
                kind: ImportKind::Symbol,
            },
            ImportInfo {
                path: "helpers".to_string(),
                alias: Some("support".to_string()),
                is_glob: false,
                kind: ImportKind::Module,
            },
        ];
        let extracted = HashMap::from([
            (caller_path.clone(), caller_file),
            (
                helper_path.clone(),
                extracted_file(helper_path.clone(), vec![function("work", "work", 10)]),
            ),
        ]);
        let index = CalleeResolutionIndex::from_sorted_extracted(&sorted(&extracted));
        let caller = FunctionId::new(caller_path.clone(), "entry".to_string(), 1);

        for (name, call_type) in [
            ("run", CallType::Direct),
            ("support.work", CallType::Method),
        ] {
            let outcome = resolve_callee_from_extracted(
                &call(name, call_type, 2),
                &caller,
                &caller_path,
                &index,
            );
            let crate::priority::call_graph::ResolutionOutcome::Resolved {
                target,
                provenance,
                confidence,
                ..
            } = outcome
            else {
                panic!("expected imported call to resolve, got {outcome:?}");
            };

            assert_eq!(
                target,
                FunctionId::new(helper_path.clone(), "work".into(), 10)
            );
            assert_eq!(
                provenance,
                crate::priority::call_graph::CallEdgeProvenance::ImportResolution
            );
            assert_eq!(confidence, 90);
        }

        let unimported = resolve_callee_from_extracted(
            &call("work", CallType::Direct, 3),
            &caller,
            &caller_path,
            &index,
        );
        assert!(matches!(
            unimported,
            crate::priority::call_graph::ResolutionOutcome::Unresolved { .. }
        ));
    }

    fn extracted_files(
        files: Vec<(PathBuf, Vec<ExtractedFunctionData>)>,
    ) -> HashMap<PathBuf, ExtractedFileData> {
        files
            .into_iter()
            .map(|(path, functions)| {
                let mut file_data = ExtractedFileData::empty(path.clone());
                file_data.functions = functions;
                (path, file_data)
            })
            .collect()
    }

    fn extracted_file(path: PathBuf, functions: Vec<ExtractedFunctionData>) -> ExtractedFileData {
        let mut file = ExtractedFileData::empty(path);
        file.functions = functions;
        file
    }

    fn function(name: &str, qualified_name: &str, line: usize) -> ExtractedFunctionData {
        let mut function = ExtractedFunctionData::minimal(name, line);
        function.qualified_name = qualified_name.to_string();
        function
    }

    fn call(name: &str, call_type: CallType, line: usize) -> crate::extraction::CallSite {
        crate::extraction::CallSite {
            callee_name: name.to_string(),
            call_type,
            line,
        }
    }

    fn sorted(
        extracted: &HashMap<PathBuf, ExtractedFileData>,
    ) -> Vec<(&PathBuf, &ExtractedFileData)> {
        let mut sorted: Vec<_> = extracted.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));
        sorted
    }
}
