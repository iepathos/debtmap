//! Python module structure analysis
//!
//! Provides parser-backed analysis of Python source files.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::types::{
    ComponentDependencyGraph, FunctionCounts, ModuleComponent, ModuleFacadeInfo, ModuleStructure,
    OrganizationQuality, PathDeclaration,
};
use crate::analyzers::python::parser::parse_source;
use crate::extraction::python::PythonExtractor;
use crate::extraction::ImportInfo;

/// Analyze a Python source file to extract module structure
pub fn analyze_python_file(content: &str, file_path: &Path) -> ModuleStructure {
    let Ok(ast) = parse_source(content, file_path) else {
        return empty_structure(content);
    };

    let Ok(extracted) = PythonExtractor::extract(&ast) else {
        return empty_structure(content);
    };

    let mut components = Vec::new();
    let mut counts = FunctionCounts::new();

    for class_data in &extracted.structs {
        let method_count = extracted
            .impls
            .iter()
            .find(|impl_data| impl_data.type_name == class_data.name)
            .map(|impl_data| impl_data.methods.len())
            .unwrap_or(0);

        components.push(ModuleComponent::Struct {
            name: class_data.name.clone(),
            fields: class_data.fields.len(),
            methods: method_count,
            public: class_data.is_public,
            line_range: (class_data.line, class_data.line),
        });
    }

    for func in &extracted.functions {
        let public = func.visibility.is_some();
        let component = ModuleComponent::ModuleLevelFunction {
            name: func.qualified_name.clone(),
            public,
            lines: func.length,
            complexity: func.cyclomatic,
        };

        if func.qualified_name.contains('.') {
            counts.impl_methods += 1;
        } else {
            counts.module_level_functions += 1;
            if public {
                counts.public_functions += 1;
            } else {
                counts.private_functions += 1;
            }
        }

        components.push(component);
    }

    let responsibility_count = (components.len() / 5).max(1);
    let dependencies = build_dependency_graph(file_path, &components, &extracted.imports);
    let facade_info = Some(detect_python_module_facade(
        file_path,
        &extracted.imports,
        components.len(),
    ));

    ModuleStructure {
        total_lines: extracted.total_lines,
        components,
        function_counts: counts.clone(),
        responsibility_count,
        public_api_surface: counts.public_functions,
        dependencies,
        facade_info,
    }
}

fn empty_structure(content: &str) -> ModuleStructure {
    ModuleStructure {
        total_lines: content.lines().count(),
        components: vec![],
        function_counts: FunctionCounts::new(),
        responsibility_count: 0,
        public_api_surface: 0,
        dependencies: ComponentDependencyGraph::new(),
        facade_info: None,
    }
}

fn build_dependency_graph(
    file_path: &Path,
    components: &[ModuleComponent],
    imports: &[ImportInfo],
) -> ComponentDependencyGraph {
    let module_name = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("module")
        .to_string();
    let mut graph_components = vec![module_name.clone()];
    graph_components.extend(components.iter().map(ModuleComponent::name));

    let import_names: Vec<String> = imports
        .iter()
        .map(|import| import.path.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    graph_components.extend(import_names.iter().cloned());
    graph_components.sort();
    graph_components.dedup();

    let edges = import_names
        .iter()
        .map(|dependency| (module_name.clone(), dependency.clone()))
        .collect::<Vec<_>>();

    let mut coupling_scores = HashMap::new();
    let fan_out = import_names.len() as f64;
    coupling_scores.insert(
        module_name.clone(),
        if fan_out == 0.0 {
            1.0
        } else {
            1.0 / (1.0 + fan_out)
        },
    );

    for component in components {
        coupling_scores.insert(component.name(), 1.0);
    }

    for dependency in import_names {
        coupling_scores.insert(dependency, 1.0);
    }

    ComponentDependencyGraph {
        components: graph_components,
        edges,
        coupling_scores,
    }
}

fn detect_python_module_facade(
    file_path: &Path,
    imports: &[ImportInfo],
    component_count: usize,
) -> ModuleFacadeInfo {
    let path_declarations = imports
        .iter()
        .map(|import| normalize_python_module_path(&import.path))
        .filter(|module_path| module_path.contains('.') || !module_path.is_empty())
        .map(|import| PathDeclaration {
            module_name: import
                .rsplit('.')
                .next()
                .unwrap_or(import.as_str())
                .to_string(),
            file_path: import,
            line: 1,
        })
        .collect::<Vec<_>>();

    let submodule_count = path_declarations
        .iter()
        .map(|declaration| declaration.file_path.clone())
        .collect::<HashSet<_>>()
        .len();
    let facade_score = calculate_python_facade_score(file_path, submodule_count, component_count);

    ModuleFacadeInfo {
        is_facade: submodule_count >= 3 && facade_score >= 0.5,
        submodule_count,
        path_declarations,
        facade_score,
        organization_quality: classify_python_organization_quality(submodule_count, facade_score),
    }
}

fn normalize_python_module_path(path: &str) -> String {
    let trimmed = path.trim_start_matches('.').trim_end_matches(".*");
    let mut segments: Vec<&str> = trimmed
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect();

    if segments.len() > 1 {
        segments.pop();
    }

    segments.join(".")
}

fn calculate_python_facade_score(
    file_path: &Path,
    submodule_count: usize,
    component_count: usize,
) -> f64 {
    let init_bonus = if file_path.file_name().and_then(|name| name.to_str()) == Some("__init__.py")
    {
        0.4
    } else {
        0.0
    };
    let submodule_score = (submodule_count.min(10) as f64) / 10.0;
    let locality_score = 1.0 / (1.0 + component_count as f64);

    (init_bonus + submodule_score + locality_score).min(1.0)
}

fn classify_python_organization_quality(
    submodule_count: usize,
    facade_score: f64,
) -> OrganizationQuality {
    if submodule_count >= 10 && facade_score >= 0.8 {
        OrganizationQuality::Excellent
    } else if submodule_count >= 5 && facade_score >= 0.6 {
        OrganizationQuality::Good
    } else if submodule_count >= 3 && facade_score >= 0.5 {
        OrganizationQuality::Poor
    } else {
        OrganizationQuality::Monolithic
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_backed_python_module_structure_counts_methods_separately() {
        let source = r#"
class Service:
    def process(self, item):
        return item

def helper():
    return 1
"#;
        let structure = analyze_python_file(source, Path::new("service.py"));

        assert_eq!(structure.function_counts.module_level_functions, 1);
        assert_eq!(structure.function_counts.impl_methods, 1);
        assert!(structure
            .components
            .iter()
            .any(|component| matches!(component, ModuleComponent::Struct { name, .. } if name == "Service")));
        assert!(structure
            .components
            .iter()
            .any(|component| matches!(component, ModuleComponent::ModuleLevelFunction { name, .. } if name == "Service.process")));
    }

    #[test]
    fn test_python_module_structure_includes_dependency_graph_and_facade_info() {
        let source = r#"
from users import UserService
from billing import BillingService
from reports import ReportService
"#;
        let structure = analyze_python_file(source, Path::new("__init__.py"));

        assert!(structure
            .dependencies
            .edges
            .iter()
            .any(|(_, dependency)| dependency == "users.UserService"));
        let facade_info = structure
            .facade_info
            .as_ref()
            .expect("facade info should be populated for parsed Python files");
        assert!(facade_info.is_facade);
        assert_eq!(facade_info.submodule_count, 3);
        assert_eq!(facade_info.organization_quality, OrganizationQuality::Poor);
    }
}
