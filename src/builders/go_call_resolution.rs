//! Conservative same-package Go call resolution for the shared priority graph.

use crate::core::{FunctionMetrics, Language};
use crate::priority::CallGraph;
use crate::priority::call_graph::{CallEdgeProvenance, CallType, FunctionId, ResolutionOutcome};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

type PackageSymbol = (PathBuf, String);

#[derive(Debug, Default)]
struct SymbolIndex {
    by_package: HashMap<PackageSymbol, Vec<FunctionId>>,
    by_import_path: HashMap<(String, String), Vec<FunctionId>>,
    imports_by_file: HashMap<PathBuf, HashMap<String, String>>,
}

pub(super) fn add_resolved_calls(graph: &mut CallGraph, metrics: &[FunctionMetrics]) {
    add_framework_evidence(graph, metrics);
    let index = symbol_index(metrics);
    let mut callers: Vec<_> = metrics
        .iter()
        .filter(|metric| is_production_go(metric))
        .collect();
    callers.sort_by_key(|metric| function_id(metric));

    for caller in callers {
        add_caller_edges(graph, caller, &index);
    }
}

fn add_framework_evidence(graph: &mut CallGraph, metrics: &[FunctionMetrics]) {
    let mut sources = HashMap::new();
    for metric in metrics
        .iter()
        .filter(|metric| Language::from_path(&metric.file) == Language::Go)
    {
        let source = sources
            .entry(metric.file.clone())
            .or_insert_with(|| std::fs::read_to_string(&metric.file).unwrap_or_default());
        let evidence = crate::analysis::role_policy::framework_evidence_for_source(
            Language::Go,
            source,
            &metric.name,
        );
        graph.add_role_evidence(&function_id(metric), evidence);
    }
}

fn symbol_index(metrics: &[FunctionMetrics]) -> SymbolIndex {
    metrics
        .iter()
        .filter(|metric| is_production_go(metric))
        .fold(SymbolIndex::default(), |mut index, metric| {
            index
                .imports_by_file
                .entry(metric.file.clone())
                .or_insert_with(|| {
                    crate::analyzers::go::imports::import_aliases_for_file(&metric.file)
                });
            let candidates = index
                .by_package
                .entry(symbol_key(&metric.file, &metric.name))
                .or_default();
            candidates.push(function_id(metric));
            candidates.sort();
            candidates.dedup();
            if !metric.name.contains('.') {
                index_importable_symbol(&mut index, metric);
            }
            index
        })
}

fn index_importable_symbol(index: &mut SymbolIndex, metric: &FunctionMetrics) {
    let Some(directory) = metric.file.parent() else {
        return;
    };
    let Some(import_path) = crate::analyzers::go::imports::package_import_path(directory) else {
        return;
    };
    let candidates = index
        .by_import_path
        .entry((import_path, metric.name.clone()))
        .or_default();
    candidates.push(function_id(metric));
    candidates.sort();
    candidates.dedup();
}

fn add_caller_edges(graph: &mut CallGraph, caller: &FunctionMetrics, index: &SymbolIndex) {
    let mut calls = caller.call_dependencies.clone().unwrap_or_default();
    calls.sort();
    calls.dedup();
    for call in calls {
        let outcome = resolve_call(caller, &call, index);
        graph.add_resolution(function_id(caller), CallType::Direct, outcome);
    }
}

fn resolve_call(caller: &FunctionMetrics, call: &str, index: &SymbolIndex) -> ResolutionOutcome {
    let (candidates, provenance, confidence) = imported_candidates(caller, call, index)
        .unwrap_or_else(|| {
            (
                index
                    .by_package
                    .get(&symbol_key(&caller.file, call))
                    .cloned()
                    .unwrap_or_default(),
                if call.contains('.') {
                    CallEdgeProvenance::TypeResolution
                } else {
                    CallEdgeProvenance::AstDirect
                },
                if call.contains('.') { 95 } else { 100 },
            )
        });
    outcome(candidates, call, provenance, confidence)
}

fn imported_candidates(
    caller: &FunctionMetrics,
    call: &str,
    index: &SymbolIndex,
) -> Option<(Vec<FunctionId>, CallEdgeProvenance, u8)> {
    let (alias, function) = call.split_once('.')?;
    let imports = index.imports_by_file.get(&caller.file)?;
    let import_path = imports.get(alias)?;
    let candidates = index
        .by_import_path
        .get(&(import_path.clone(), function.to_string()))
        .cloned()
        .unwrap_or_default();
    Some((candidates, CallEdgeProvenance::ImportResolution, 100))
}

fn outcome(
    mut candidates: Vec<FunctionId>,
    call: &str,
    provenance: CallEdgeProvenance,
    confidence: u8,
) -> ResolutionOutcome {
    candidates.sort();
    match candidates.as_slice() {
        [target] => ResolutionOutcome::Resolved {
            target: target.clone(),
            provenance,
            confidence,
            call_site: None,
        },
        [] => ResolutionOutcome::Unresolved {
            query: call.to_string(),
        },
        _ => ResolutionOutcome::Ambiguous { candidates },
    }
}

fn symbol_key(file: &Path, name: &str) -> PackageSymbol {
    (
        file.parent().unwrap_or_else(|| Path::new("")).to_path_buf(),
        name.to_string(),
    )
}

fn function_id(metric: &FunctionMetrics) -> FunctionId {
    FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line)
}

fn is_production_go(metric: &FunctionMetrics) -> bool {
    Language::from_path(&metric.file) == Language::Go
        && crate::analysis::role_policy::roles_for_metric(metric).is_production()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn resolves_unique_same_package_calls_with_evidence() {
        let mut caller = metric("pkg/app.go", "run", 1);
        caller.call_dependencies = Some(vec!["helper".into(), "Worker.Validate".into()]);
        let metrics = vec![
            caller.clone(),
            metric("pkg/helpers.go", "helper", 10),
            metric("pkg/worker.go", "Worker.Validate", 20),
            metric("other/helpers.go", "helper", 30),
        ];
        let mut graph = crate::builders::call_graph::build_initial_call_graph(&metrics);

        add_resolved_calls(&mut graph, &metrics);

        let caller_id = function_id(&caller);
        let callees = graph.get_callees_exact(&caller_id);
        assert_eq!(callees.len(), 2);
        let evidence: Vec<_> = graph.edge_evidence().collect();
        assert_eq!(evidence.len(), 2);
        assert!(evidence.iter().all(|edge| edge.confidence >= 95));
        assert!(evidence.iter().all(|edge| edge.call_site.is_none()));
    }

    #[test]
    fn does_not_guess_cross_package_targets() {
        let mut caller = metric("pkg/app.go", "run", 1);
        caller.call_dependencies = Some(vec!["helper".into()]);
        let metrics = vec![caller.clone(), metric("other/helpers.go", "helper", 30)];
        let mut graph = crate::builders::call_graph::build_initial_call_graph(&metrics);

        add_resolved_calls(&mut graph, &metrics);

        assert!(graph.get_callees_exact(&function_id(&caller)).is_empty());
        assert_eq!(graph.edge_evidence().count(), 0);
    }

    #[test]
    fn resolves_explicit_import_alias_without_name_guessing() {
        let project = tempdir().unwrap();
        fs::write(project.path().join("go.mod"), "module example.com/app\n").unwrap();
        let app = project.path().join("cmd/app.go");
        let library = project.path().join("lib/helpers.go");
        fs::create_dir_all(app.parent().unwrap()).unwrap();
        fs::create_dir_all(library.parent().unwrap()).unwrap();
        fs::write(&app, "package main\nimport util \"example.com/app/lib\"\n").unwrap();
        fs::write(&library, "package lib\n").unwrap();
        let mut caller = metric(app.to_str().unwrap(), "run", 1);
        caller.call_dependencies = Some(vec!["util.Helper".into(), "other.Helper".into()]);
        let callee = metric(library.to_str().unwrap(), "Helper", 1);
        let metrics = vec![caller.clone(), callee.clone()];
        let mut graph = crate::builders::call_graph::build_initial_call_graph(&metrics);

        add_resolved_calls(&mut graph, &metrics);

        assert_eq!(
            graph.get_callees_exact(&function_id(&caller)),
            vec![function_id(&callee)]
        );
        let evidence = graph.edge_evidence().next().unwrap();
        assert_eq!(evidence.provenance, CallEdgeProvenance::ImportResolution);
        assert_eq!(evidence.confidence, 100);
    }

    #[test]
    fn http_registration_marks_handler_as_framework_managed() {
        let project = tempdir().unwrap();
        let file = project.path().join("server.go");
        fs::write(
            &file,
            "package main\nfunc health() {}\nfunc main() { http.HandleFunc(\"/health\", health) }\n",
        )
        .unwrap();
        let handler = metric(file.to_str().unwrap(), "health", 2);
        let metrics = vec![handler.clone()];
        let mut graph = crate::builders::call_graph::build_initial_call_graph(&metrics);

        add_resolved_calls(&mut graph, &metrics);

        assert!(
            graph
                .get_roles(&function_id(&handler))
                .unwrap()
                .is_framework_managed
        );
    }

    fn metric(file: &str, name: &str, line: usize) -> FunctionMetrics {
        FunctionMetrics::new(name.to_string(), PathBuf::from(file), line)
    }
}
