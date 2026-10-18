//! Conservative Solidity call resolution for the shared priority graph.

use crate::analyzers::solidity::call_graph::{
    SolidityBatchSnapshot, compute_call_edges_with, parse_snapshots,
};
use crate::core::ast::SolidityAst;
use crate::core::{FunctionMetrics, Language};
use crate::priority::CallGraph;
use crate::priority::call_graph::{CallEdgeProvenance, CallType, FunctionId, ResolutionOutcome};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub(super) fn add_resolved_calls(graph: &mut CallGraph, metrics: &[FunctionMetrics]) {
    let solidity: Vec<_> = metrics
        .iter()
        .filter(|metric| Language::from_path(&metric.file) == Language::Solidity)
        .collect();
    let sources = read_sources(&solidity);
    add_framework_evidence(graph, &solidity, &sources);
    let functions = production_functions(&solidity);
    let snapshots = snapshots(&functions);
    let asts = parse_snapshots(&snapshots);
    let edges = compute_call_edges_with(&snapshots, &asts);
    let targets = target_index(&functions);
    let imports = import_index(&functions, &asts);
    let callers = caller_index(&functions);

    for ((caller_file, caller_name), callees) in sorted_edges(edges) {
        let Some([caller]) = callers
            .get(&(caller_file.clone(), caller_name))
            .map(Vec::as_slice)
        else {
            continue;
        };
        for callee in callees {
            let candidates = targets.get(&callee).cloned().unwrap_or_default();
            graph.add_resolution(
                caller.clone(),
                CallType::Direct,
                resolution(
                    candidates,
                    &callee,
                    &caller_file,
                    imports.get(&caller_file).map(Vec::as_slice).unwrap_or(&[]),
                ),
            );
        }
    }
}

fn read_sources(metrics: &[&FunctionMetrics]) -> HashMap<PathBuf, String> {
    let files: HashSet<_> = metrics.iter().map(|metric| &metric.file).collect();
    files
        .into_par_iter()
        .map(|file| {
            (
                file.clone(),
                std::fs::read_to_string(file).unwrap_or_default(),
            )
        })
        .collect()
}

fn add_framework_evidence(
    graph: &mut CallGraph,
    metrics: &[&FunctionMetrics],
    sources: &HashMap<PathBuf, String>,
) {
    let evidence: Vec<_> = metrics
        .par_iter()
        .map(|metric| {
            crate::analysis::role_policy::framework_evidence_for_source(
                Language::Solidity,
                sources
                    .get(&metric.file)
                    .map(String::as_str)
                    .unwrap_or_default(),
                &metric.name,
            )
        })
        .collect();
    for (metric, evidence) in metrics.iter().zip(evidence) {
        graph.add_role_evidence(&function_id(metric), evidence);
    }
}

fn production_functions<'a>(metrics: &[&'a FunctionMetrics]) -> Vec<&'a FunctionMetrics> {
    metrics
        .iter()
        .copied()
        .filter(|metric| crate::analysis::role_policy::roles_for_metric(metric).is_production())
        .collect()
}

fn caller_index(functions: &[&FunctionMetrics]) -> HashMap<(PathBuf, String), Vec<FunctionId>> {
    functions.iter().fold(HashMap::new(), |mut index, metric| {
        index
            .entry((metric.file.clone(), metric.name.clone()))
            .or_default()
            .push(function_id(metric));
        index
    })
}

fn snapshots(functions: &[&FunctionMetrics]) -> Vec<SolidityBatchSnapshot> {
    let mut by_file: HashMap<PathBuf, Vec<String>> = HashMap::new();
    for function in functions {
        by_file
            .entry(function.file.clone())
            .or_default()
            .push(function.name.clone());
    }
    let mut snapshots: Vec<_> = by_file
        .into_iter()
        .map(|(path, mut functions)| {
            functions.sort();
            functions.dedup();
            SolidityBatchSnapshot {
                path,
                language: Language::Solidity,
                functions,
            }
        })
        .collect();
    snapshots.sort_by(|left, right| left.path.cmp(&right.path));
    snapshots
}

fn target_index(functions: &[&FunctionMetrics]) -> HashMap<String, Vec<FunctionId>> {
    functions.iter().fold(HashMap::new(), |mut index, metric| {
        let candidates = index.entry(metric.name.clone()).or_default();
        candidates.push(function_id(metric));
        candidates.sort();
        candidates.dedup();
        index
    })
}

fn import_index(
    functions: &[&FunctionMetrics],
    asts: &[SolidityAst],
) -> HashMap<PathBuf, Vec<PathBuf>> {
    let resolver =
        crate::analyzers::solidity::remappings::SolidityImportResolver::from_analyzed_files(
            &unique_files(functions),
        );
    asts.par_iter()
        .map(|ast| (ast.path.clone(), imports_for_ast(ast, &resolver)))
        .collect()
}

fn unique_files(functions: &[&FunctionMetrics]) -> Vec<PathBuf> {
    let mut files = functions
        .iter()
        .map(|function| function.file.clone())
        .collect::<Vec<_>>();
    files.sort();
    files.dedup();
    files
}

fn imports_for_ast(
    ast: &SolidityAst,
    resolver: &crate::analyzers::solidity::remappings::SolidityImportResolver,
) -> Vec<PathBuf> {
    crate::analyzers::solidity::dependencies::extract_dependencies(ast)
        .into_iter()
        .filter(|dependency| dependency.kind == crate::core::DependencyKind::Import)
        .map(|dependency| PathBuf::from(resolver.resolve(&dependency.name, &ast.path)))
        .collect()
}

fn sorted_edges(
    edges: HashMap<(PathBuf, String), Vec<String>>,
) -> Vec<((PathBuf, String), Vec<String>)> {
    let mut edges: Vec<_> = edges.into_iter().collect();
    edges.sort_by(|left, right| left.0.cmp(&right.0));
    edges
}

fn resolution(
    candidates: Vec<FunctionId>,
    query: &str,
    caller_file: &Path,
    imports: &[PathBuf],
) -> ResolutionOutcome {
    let same_file = candidates_for_file(&candidates, caller_file);
    if same_file.len() == 1 {
        return resolved(same_file[0].clone(), CallEdgeProvenance::TypeResolution);
    }
    let imported = candidates
        .iter()
        .filter(|candidate| {
            imports
                .iter()
                .any(|path| paths_match(&candidate.file, path))
        })
        .cloned()
        .collect::<Vec<_>>();
    if imported.len() == 1 {
        return resolved(imported[0].clone(), CallEdgeProvenance::ImportResolution);
    }
    resolution_from_unique(candidates, query)
}

fn candidates_for_file(candidates: &[FunctionId], file: &Path) -> Vec<FunctionId> {
    candidates
        .iter()
        .filter(|candidate| candidate.file == file)
        .cloned()
        .collect()
}

fn paths_match(candidate: &Path, imported: &Path) -> bool {
    candidate == imported || candidate.ends_with(imported) || imported.ends_with(candidate)
}

fn resolved(target: FunctionId, provenance: CallEdgeProvenance) -> ResolutionOutcome {
    ResolutionOutcome::Resolved {
        target,
        provenance,
        confidence: 95,
        call_site: None,
    }
}

fn resolution_from_unique(mut candidates: Vec<FunctionId>, query: &str) -> ResolutionOutcome {
    candidates.sort();
    match candidates.as_slice() {
        [target] => resolved(target.clone(), CallEdgeProvenance::TypeResolution),
        [] => ResolutionOutcome::Unresolved {
            query: query.to_string(),
        },
        _ => ResolutionOutcome::Ambiguous { candidates },
    }
}

fn function_id(metric: &FunctionMetrics) -> FunctionId {
    FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line)
        .with_column(metric.column)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn duplicate_contract_names_resolve_through_explicit_import_path() {
        let project = tempdir().unwrap();
        let first = project.path().join("root_a/Token.sol");
        let second = project.path().join("root_b/Token.sol");
        let caller = project.path().join("root_a/Caller.sol");
        fs::create_dir_all(first.parent().unwrap()).unwrap();
        fs::create_dir_all(second.parent().unwrap()).unwrap();
        fs::write(
            &first,
            "pragma solidity 0.8.20; contract Token { function helper() public {} }",
        )
        .unwrap();
        fs::write(
            &second,
            "pragma solidity 0.8.20; contract Token { function helper() public {} }",
        )
        .unwrap();
        fs::write(
            &caller,
            "pragma solidity 0.8.20; import './Token.sol'; contract Caller { function run(address token) public { Token(token).helper(); } }",
        )
        .unwrap();
        let caller_metric = metric(&caller, "Caller.run", 1);
        let first_metric = metric(&first, "Token.helper", 1);
        let second_metric = metric(&second, "Token.helper", 1);
        let metrics = vec![caller_metric.clone(), first_metric.clone(), second_metric];
        let mut graph = crate::builders::call_graph::build_initial_call_graph(&metrics);
        add_resolved_calls(&mut graph, &metrics);

        assert_eq!(
            graph.get_callees_exact(&function_id(&caller_metric)),
            vec![function_id(&first_metric)]
        );
        assert_eq!(
            graph.edge_evidence().next().unwrap().provenance,
            CallEdgeProvenance::ImportResolution
        );
    }

    #[test]
    fn foundry_hook_is_framework_managed_test_evidence() {
        let project = tempdir().unwrap();
        let file = project.path().join("Vault.t.sol");
        fs::write(
            &file,
            "import 'forge-std/Test.sol'; contract VaultTest { function setUp() public {} }",
        )
        .unwrap();
        let hook = metric(&file, "VaultTest.setUp", 1);
        let metrics = vec![hook.clone()];
        let mut graph = crate::builders::call_graph::build_initial_call_graph(&metrics);

        add_resolved_calls(&mut graph, &metrics);

        let roles = graph.get_roles(&function_id(&hook)).unwrap();
        assert!(roles.is_test);
        assert!(roles.is_framework_managed);
    }

    fn metric(file: &Path, name: &str, line: usize) -> FunctionMetrics {
        FunctionMetrics::new(name.to_string(), file.to_path_buf(), line)
    }
}
