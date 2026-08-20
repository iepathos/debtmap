//! Conservative project-level JavaScript and TypeScript call resolution.

use super::call_graph::{
    CallShape, ExtractedCall, FunctionWithCalls, extract_functions_with_calls,
};
use super::module_links::{
    ImportBinding, ImportName, ReExport, extract_imports, extract_reexports,
};
use crate::core::ast::TypeScriptAst;
use crate::priority::call_graph::{
    CallEdgeProvenance, CallGraph, CallSite, CallType, FunctionId, ResolutionOutcome,
};
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
struct Module<'a> {
    ast: &'a TypeScriptAst,
    functions: Vec<FunctionWithCalls>,
    imports: Vec<ImportBinding>,
    reexports: Vec<ReExport>,
}

pub(crate) fn extract_project_call_graph(asts: &[TypeScriptAst]) -> CallGraph {
    let modules = asts.iter().map(module_from_ast).collect::<Vec<_>>();
    let mut graph = graph_with_functions(&modules);
    for module in &modules {
        add_module_edges(&mut graph, module, &modules);
    }
    graph
}

fn module_from_ast(ast: &TypeScriptAst) -> Module<'_> {
    Module {
        ast,
        functions: extract_functions_with_calls(ast),
        imports: extract_imports(ast),
        reexports: extract_reexports(ast),
    }
}

fn graph_with_functions(modules: &[Module<'_>]) -> CallGraph {
    modules.iter().flat_map(|module| &module.functions).fold(
        CallGraph::new(),
        |mut graph, function| {
            graph.add_function(
                function_id(function),
                function.is_exported,
                function.is_test,
                1,
                10,
            );
            graph
        },
    )
}

fn add_module_edges(graph: &mut CallGraph, module: &Module<'_>, modules: &[Module<'_>]) {
    for caller in &module.functions {
        let owner = caller.name.split_once("::").map(|(owner, _)| owner);
        for call in sorted_calls(caller) {
            graph.add_resolution(
                function_id(caller),
                CallType::Direct,
                resolve_call(module, modules, &call, owner),
            );
        }
    }
}

fn sorted_calls(caller: &FunctionWithCalls) -> Vec<ExtractedCall> {
    let mut calls = caller.calls.clone();
    calls.sort_by(|a, b| (a.line, a.column, &a.shape).cmp(&(b.line, b.column, &b.shape)));
    calls
}

fn resolve_call(
    module: &Module<'_>,
    modules: &[Module<'_>],
    call: &ExtractedCall,
    owner: Option<&str>,
) -> ResolutionOutcome {
    let (candidates, provenance, confidence) = match &call.shape {
        CallShape::Identifier(name) => resolve_identifier(module, modules, name),
        CallShape::Member { receiver, property } if receiver == "this" => owner
            .map(|owner| {
                (
                    local_candidates(module, &format!("{owner}::{property}")),
                    CallEdgeProvenance::TypeResolution,
                    95,
                )
            })
            .unwrap_or_else(|| (Vec::new(), CallEdgeProvenance::TypeResolution, 95)),
        CallShape::Member { receiver, property } => (
            resolve_namespace_member(module, modules, receiver, property),
            CallEdgeProvenance::ImportResolution,
            100,
        ),
    };
    outcome(candidates, call, &module.ast.path, provenance, confidence)
}

fn resolve_identifier(
    module: &Module<'_>,
    modules: &[Module<'_>],
    name: &str,
) -> (Vec<FunctionId>, CallEdgeProvenance, u8) {
    let local = local_candidates(module, name);
    if !local.is_empty() {
        return (local, CallEdgeProvenance::AstDirect, 100);
    }
    (
        module
            .imports
            .iter()
            .filter(|binding| binding.local == name)
            .filter_map(|binding| match &binding.imported {
                ImportName::Named(imported) => Some((&binding.source, imported)),
                ImportName::Namespace => None,
            })
            .flat_map(|(source, imported)| exported_candidates(module, modules, source, imported))
            .collect(),
        CallEdgeProvenance::ImportResolution,
        100,
    )
}

fn resolve_namespace_member(
    module: &Module<'_>,
    modules: &[Module<'_>],
    receiver: &str,
    property: &str,
) -> Vec<FunctionId> {
    module
        .imports
        .iter()
        .filter(|binding| binding.local == receiver && binding.imported == ImportName::Namespace)
        .flat_map(|binding| exported_candidates(module, modules, &binding.source, property))
        .collect()
}

fn local_candidates(module: &Module<'_>, name: &str) -> Vec<FunctionId> {
    module
        .functions
        .iter()
        .filter(|function| function.name == name)
        .map(function_id)
        .collect()
}

fn exported_candidates(
    origin: &Module<'_>,
    modules: &[Module<'_>],
    source: &str,
    name: &str,
) -> Vec<FunctionId> {
    let mut visited = HashSet::new();
    source_modules(origin, modules, source)
        .into_iter()
        .flat_map(|module| resolve_export(module, modules, name, &mut visited))
        .collect()
}

fn resolve_export(
    module: &Module<'_>,
    modules: &[Module<'_>],
    name: &str,
    visited: &mut HashSet<(PathBuf, String)>,
) -> Vec<FunctionId> {
    if !visited.insert((module.ast.path.clone(), name.to_string())) {
        return Vec::new();
    }
    let mut candidates = module
        .functions
        .iter()
        .filter(|function| function.is_exported && function.name == name)
        .map(function_id)
        .collect::<Vec<_>>();
    for export in module
        .reexports
        .iter()
        .filter(|export| export.exported == name)
    {
        for target in source_modules(module, modules, &export.source) {
            candidates.extend(resolve_export(target, modules, &export.imported, visited));
        }
    }
    candidates
}

fn source_modules<'a>(
    origin: &Module<'_>,
    modules: &'a [Module<'_>],
    source: &str,
) -> Vec<&'a Module<'a>> {
    if !source.starts_with('.') {
        return Vec::new();
    }
    let base = lexical_path(
        &origin
            .ast
            .path
            .parent()
            .unwrap_or(Path::new(""))
            .join(source),
    );
    modules
        .iter()
        .filter(|module| module_matches(&base, &module.ast.path))
        .collect()
}

fn module_matches(base: &Path, candidate: &Path) -> bool {
    let candidate = lexical_path(candidate);
    candidate == base
        || lexical_path(&candidate.with_extension("")) == base
        || (candidate.file_stem().and_then(|name| name.to_str()) == Some("index")
            && candidate.parent() == Some(base))
}

fn lexical_path(path: &Path) -> PathBuf {
    path.components()
        .fold(PathBuf::new(), |mut clean, component| {
            match component {
                Component::ParentDir => {
                    clean.pop();
                }
                Component::CurDir => {}
                _ => clean.push(component.as_os_str()),
            }
            clean
        })
}

fn outcome(
    mut candidates: Vec<FunctionId>,
    call: &ExtractedCall,
    file: &Path,
    provenance: CallEdgeProvenance,
    confidence: u8,
) -> ResolutionOutcome {
    candidates.sort();
    candidates.dedup();
    match candidates.as_slice() {
        [target] => ResolutionOutcome::Resolved {
            target: target.clone(),
            provenance,
            confidence,
            call_site: Some(CallSite {
                file: file.to_path_buf(),
                line: call.line,
                column: Some(call.column),
            }),
        },
        [] => ResolutionOutcome::Unresolved {
            query: format!("{:?}", call.shape),
        },
        _ => ResolutionOutcome::Ambiguous { candidates },
    }
}

fn function_id(function: &FunctionWithCalls) -> FunctionId {
    FunctionId::new(function.file.clone(), function.name.clone(), function.line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::typescript::parser::parse_source;
    use crate::core::ast::JsLanguageVariant;

    #[test]
    fn resolves_named_import_through_reexport() {
        let asts = vec![
            ast(
                "src/main.ts",
                "import { renamed } from './barrel'; export function run() { renamed(); }",
            ),
            ast(
                "src/barrel.ts",
                "export { helper as renamed } from './helper';",
            ),
            ast("src/helper.ts", "export function helper() { return 1; }"),
        ];

        let graph = extract_project_call_graph(&asts);
        let run = find(&graph, "run");
        let callees = graph.get_callees(run);

        assert_eq!(callees.len(), 1);
        assert_eq!(callees[0].name, "helper");
        assert_eq!(
            graph.edge_evidence().next().unwrap().provenance,
            CallEdgeProvenance::ImportResolution
        );
    }

    #[test]
    fn keeps_direct_provenance_for_same_module_calls() {
        let asts = vec![ast(
            "src/main.ts",
            "function helper() {} function run() { helper(); }",
        )];

        let graph = extract_project_call_graph(&asts);
        let evidence = graph.edge_evidence().next().unwrap();

        assert_eq!(evidence.provenance, CallEdgeProvenance::AstDirect);
        assert_eq!(evidence.confidence, 100);
        assert_eq!(evidence.call_site.as_ref().unwrap().line, 1);
    }

    #[test]
    fn resolves_namespace_import_but_not_dynamic_receiver() {
        let asts = vec![
            ast(
                "src/main.js",
                "import * as util from './util.js'; function run(service) { util.help(); service.help(); }",
            ),
            ast("src/util.js", "export function help() { return 1; }"),
        ];

        let graph = extract_project_call_graph(&asts);
        let callees = graph.get_callees(find(&graph, "run"));

        assert_eq!(callees.len(), 1);
        assert_eq!(callees[0].name, "help");
    }

    #[test]
    fn duplicate_module_candidates_are_ambiguous() {
        let asts = vec![
            ast(
                "src/main.ts",
                "import { helper } from './lib'; function run() { helper(); }",
            ),
            ast("src/lib.ts", "export function helper() {}"),
            ast("src/lib.js", "export function helper() {}"),
        ];

        let graph = extract_project_call_graph(&asts);

        assert!(graph.get_callees(find(&graph, "run")).is_empty());
        assert_eq!(graph.edge_evidence().count(), 0);
    }

    fn ast(path: &str, source: &str) -> TypeScriptAst {
        parse_source(source, Path::new(path), JsLanguageVariant::TypeScript).unwrap()
    }

    fn find<'a>(graph: &'a CallGraph, name: &str) -> &'a FunctionId {
        graph
            .get_all_functions()
            .find(|function| function.name == name)
            .unwrap()
    }
}
