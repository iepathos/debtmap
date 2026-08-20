//! Import-aware Python call candidates for the shared call graph.

use crate::core::Language;
use crate::extraction::{ExtractedFileData, ImportInfo, ImportKind};
use crate::priority::call_graph::FunctionId;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct PythonImportIndex {
    bindings: HashMap<PathBuf, HashMap<String, Vec<FunctionId>>>,
}

impl PythonImportIndex {
    pub(super) fn from_extracted(files: &[(&PathBuf, &ExtractedFileData)]) -> Self {
        let bindings = files
            .iter()
            .filter(|(path, _)| Language::from_path(path) == Language::Python)
            .filter_map(|(path, file)| {
                let imports = import_bindings(file, files);
                (!imports.is_empty()).then(|| ((*path).clone(), imports))
            })
            .collect();
        Self { bindings }
    }

    pub(super) fn candidates(&self, file: &Path, call: &str) -> Vec<FunctionId> {
        self.bindings
            .get(file)
            .and_then(|bindings| bindings.get(call))
            .cloned()
            .unwrap_or_default()
    }
}

fn import_bindings(
    file: &ExtractedFileData,
    files: &[(&PathBuf, &ExtractedFileData)],
) -> HashMap<String, Vec<FunctionId>> {
    file.imports
        .iter()
        .fold(HashMap::new(), |mut bindings, import| {
            add_import(&mut bindings, import, files);
            bindings
        })
}

fn add_import(
    bindings: &mut HashMap<String, Vec<FunctionId>>,
    import: &ImportInfo,
    files: &[(&PathBuf, &ExtractedFileData)],
) {
    match import.kind {
        ImportKind::Symbol => add_symbol_import(bindings, import, files),
        ImportKind::Module => add_module_import(bindings, import, files),
        ImportKind::Unknown | ImportKind::Glob => {}
    }
}

fn add_symbol_import(
    bindings: &mut HashMap<String, Vec<FunctionId>>,
    import: &ImportInfo,
    files: &[(&PathBuf, &ExtractedFileData)],
) {
    let Some((module, symbol)) = import.path.rsplit_once('.') else {
        return;
    };
    let binding = import.alias.as_deref().unwrap_or(symbol);
    for target in module_functions(module, files)
        .into_iter()
        .filter(|target| target.name == symbol)
    {
        add_candidate(bindings, binding, target);
    }
}

fn add_module_import(
    bindings: &mut HashMap<String, Vec<FunctionId>>,
    import: &ImportInfo,
    files: &[(&PathBuf, &ExtractedFileData)],
) {
    let binding = import.alias.as_deref().unwrap_or(&import.path);
    for target in module_functions(&import.path, files) {
        add_candidate(bindings, &format!("{binding}.{}", target.name), target);
    }
}

fn module_functions(module: &str, files: &[(&PathBuf, &ExtractedFileData)]) -> Vec<FunctionId> {
    files
        .iter()
        .filter(|(path, _)| path_matches_module(path, module))
        .flat_map(|(path, file)| {
            file.functions
                .iter()
                .filter(|function| !function.qualified_name.contains('.'))
                .map(|function| {
                    FunctionId::new(
                        (*path).clone(),
                        function.qualified_name.clone(),
                        function.line,
                    )
                })
        })
        .collect()
}

fn path_matches_module(path: &Path, module: &str) -> bool {
    let mut path_parts: Vec<String> = path
        .components()
        .filter_map(|component| component.as_os_str().to_str().map(str::to_string))
        .collect();
    let Some(file) = path_parts.pop() else {
        return false;
    };
    let stem = Path::new(&file)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if stem != "__init__" {
        path_parts.push(stem.to_string());
    }
    has_module_suffix(&path_parts, module)
}

fn has_module_suffix(path_parts: &[String], module: &str) -> bool {
    let module_parts: Vec<_> = module
        .trim_start_matches('.')
        .split('.')
        .filter(|part| !part.is_empty())
        .collect();
    !module_parts.is_empty()
        && path_parts.len() >= module_parts.len()
        && path_parts[path_parts.len() - module_parts.len()..]
            .iter()
            .map(String::as_str)
            .eq(module_parts)
}

fn add_candidate(bindings: &mut HashMap<String, Vec<FunctionId>>, name: &str, target: FunctionId) {
    let candidates = bindings.entry(name.to_string()).or_default();
    if !candidates.contains(&target) {
        candidates.push(target);
        candidates.sort();
    }
}
