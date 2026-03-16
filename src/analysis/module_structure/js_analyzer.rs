//! JavaScript/TypeScript module structure analysis
//!
//! Provides parser-backed analysis of JavaScript and TypeScript source files.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::types::{
    ComponentDependencyGraph, FunctionCounts, ModuleComponent, ModuleFacadeInfo, ModuleStructure,
    OrganizationQuality, PathDeclaration,
};
use crate::analyzers::typescript::dependencies::extract_dependencies;
use crate::analyzers::typescript::parser::{node_line, node_text, parse_source};
use crate::analyzers::typescript::visitor::class_analysis::extract_classes;
use crate::analyzers::typescript::visitor::function_analysis::extract_functions;
use crate::core::ast::JsLanguageVariant;
use tree_sitter::Node;

/// Analyze a JavaScript source file to extract module structure
pub fn analyze_javascript_file(content: &str, file_path: &Path) -> ModuleStructure {
    analyze_script_file(content, file_path, JsLanguageVariant::JavaScript)
}

/// Analyze a TypeScript source file to extract module structure
///
/// TypeScript analysis is similar to JavaScript with type annotations.
pub fn analyze_typescript_file(content: &str, file_path: &Path) -> ModuleStructure {
    analyze_script_file(content, file_path, JsLanguageVariant::TypeScript)
}

fn analyze_script_file(
    content: &str,
    file_path: &Path,
    variant: JsLanguageVariant,
) -> ModuleStructure {
    let Ok(ast) = parse_source(content, file_path, variant) else {
        return empty_structure(content);
    };

    let classes = extract_classes(&ast);
    let functions = extract_functions(&ast, true);
    let dependencies = build_dependency_graph(
        file_path,
        &components_for_graph(&classes, &functions),
        &extract_dependencies(&ast),
    );
    let facade_info = Some(detect_js_module_facade(&ast));

    let mut components = Vec::new();
    let mut counts = FunctionCounts::new();

    for class_info in classes {
        components.push(ModuleComponent::Struct {
            name: class_info.name,
            fields: class_info.property_count,
            methods: class_info.method_count,
            public: class_info.is_exported,
            line_range: (class_info.line, class_info.line),
        });
    }

    for func in functions {
        let public = func.is_exported;
        let is_method = matches!(
            func.kind,
            crate::analyzers::typescript::types::FunctionKind::ClassMethod
                | crate::analyzers::typescript::types::FunctionKind::Constructor
                | crate::analyzers::typescript::types::FunctionKind::Getter
                | crate::analyzers::typescript::types::FunctionKind::Setter
        );

        if is_method {
            counts.impl_methods += 1;
        } else {
            counts.module_level_functions += 1;
            if public {
                counts.public_functions += 1;
            } else {
                counts.private_functions += 1;
            }
        }

        components.push(ModuleComponent::ModuleLevelFunction {
            name: func.name,
            public,
            lines: func.length,
            complexity: func.cyclomatic,
        });
    }

    let responsibility_count = (components.len() / 5).max(1);

    ModuleStructure {
        total_lines: content.lines().count(),
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

fn components_for_graph(
    classes: &[crate::analyzers::typescript::visitor::class_analysis::ClassInfo],
    functions: &[crate::analyzers::typescript::types::JsFunctionMetrics],
) -> Vec<String> {
    let class_names = classes.iter().map(|class_info| class_info.name.clone());
    let function_names = functions
        .iter()
        .filter(|function| {
            !matches!(
                function.kind,
                crate::analyzers::typescript::types::FunctionKind::ClassMethod
                    | crate::analyzers::typescript::types::FunctionKind::Constructor
                    | crate::analyzers::typescript::types::FunctionKind::Getter
                    | crate::analyzers::typescript::types::FunctionKind::Setter
            )
        })
        .map(|function| function.name.clone());

    class_names.chain(function_names).collect()
}

fn build_dependency_graph(
    file_path: &Path,
    local_components: &[String],
    dependencies: &[crate::core::Dependency],
) -> ComponentDependencyGraph {
    let module_name = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("module")
        .to_string();
    let mut components = vec![module_name.clone()];
    components.extend(local_components.iter().cloned());

    let dependency_names: Vec<String> = dependencies
        .iter()
        .map(|dependency| dependency.name.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    components.extend(dependency_names.iter().cloned());
    components.sort();
    components.dedup();

    let edges = dependency_names
        .iter()
        .map(|dependency| (module_name.clone(), dependency.clone()))
        .collect::<Vec<_>>();

    let mut coupling_scores = HashMap::new();
    let fan_out = dependency_names.len() as f64;
    coupling_scores.insert(
        module_name.clone(),
        if fan_out == 0.0 {
            1.0
        } else {
            1.0 / (1.0 + fan_out)
        },
    );

    for component in local_components {
        coupling_scores.insert(component.clone(), 1.0);
    }

    for dependency in dependency_names {
        coupling_scores.insert(dependency, 1.0);
    }

    ComponentDependencyGraph {
        components,
        edges,
        coupling_scores,
    }
}

fn detect_js_module_facade(ast: &crate::core::ast::TypeScriptAst) -> ModuleFacadeInfo {
    let mut reexports = Vec::new();
    collect_reexports(&ast.tree.root_node(), ast, &mut reexports);

    let unique_paths: Vec<PathDeclaration> =
        reexports
            .into_iter()
            .fold(Vec::new(), |mut acc, declaration| {
                if !acc
                    .iter()
                    .any(|existing| existing.file_path == declaration.file_path)
                {
                    acc.push(declaration);
                }
                acc
            });

    let submodule_count = unique_paths.len();
    let export_statement_count = count_export_statements(&ast.tree.root_node());
    let local_declaration_count = count_local_declarations(&ast.tree.root_node());
    let reexport_ratio = submodule_count as f64 / export_statement_count.max(1) as f64;
    let locality_score = 1.0 / (1.0 + local_declaration_count as f64);
    let submodule_score = (submodule_count.min(10) as f64) / 10.0;
    let facade_score = ((reexport_ratio + locality_score + submodule_score) / 3.0).min(1.0);
    let organization_quality = classify_organization_quality(submodule_count, facade_score);

    ModuleFacadeInfo {
        is_facade: submodule_count >= 3 && facade_score >= 0.5,
        submodule_count,
        path_declarations: unique_paths,
        facade_score,
        organization_quality,
    }
}

fn collect_reexports(
    node: &Node,
    ast: &crate::core::ast::TypeScriptAst,
    reexports: &mut Vec<PathDeclaration>,
) {
    if node.kind() == "export_statement" {
        if let Some(source) = node.child_by_field_name("source") {
            let file_path = node_text(&source, &ast.source)
                .trim_matches(|character| character == '"' || character == '\'' || character == '`')
                .to_string();
            let module_name = Path::new(&file_path)
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or(file_path.as_str())
                .to_string();
            reexports.push(PathDeclaration {
                module_name,
                file_path,
                line: node_line(&node),
            });
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_reexports(&child, ast, reexports);
    }
}

fn count_export_statements(node: &Node) -> usize {
    let child_count = node
        .children(&mut node.walk())
        .map(|child| count_export_statements(&child))
        .sum::<usize>();

    usize::from(node.kind() == "export_statement") + child_count
}

fn count_local_declarations(node: &Node) -> usize {
    let is_local_declaration = matches!(
        node.kind(),
        "function_declaration"
            | "class_declaration"
            | "lexical_declaration"
            | "variable_declaration"
    );
    let child_count = node
        .children(&mut node.walk())
        .map(|child| count_local_declarations(&child))
        .sum::<usize>();

    usize::from(is_local_declaration) + child_count
}

fn classify_organization_quality(submodule_count: usize, facade_score: f64) -> OrganizationQuality {
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
    fn test_parser_backed_javascript_module_structure_preserves_names() {
        let source = r#"
export function helper() { return 1; }
class Greeter {
  greet() { return "hi"; }
}
"#;
        let structure = analyze_javascript_file(source, Path::new("greeter.js"));

        assert_eq!(structure.function_counts.module_level_functions, 1);
        assert_eq!(structure.function_counts.impl_methods, 1);
        assert!(structure
            .components
            .iter()
            .any(|component| matches!(component, ModuleComponent::ModuleLevelFunction { name, .. } if name == "helper")));
        assert!(structure
            .components
            .iter()
            .any(|component| matches!(component, ModuleComponent::Struct { name, methods, .. } if name == "Greeter" && *methods == 1)));
    }

    #[test]
    fn test_typescript_module_structure_includes_dependency_graph_and_facade_info() {
        let source = r#"
export { foo } from "./foo";
export { bar } from "./bar";
export * from "./baz";
import { helper } from "./helper";

export function run() {
  return helper();
}
"#;
        let structure = analyze_typescript_file(source, Path::new("index.ts"));

        assert!(structure
            .dependencies
            .edges
            .iter()
            .any(|(_, dependency)| dependency == "./helper"));
        assert!(structure
            .dependencies
            .edges
            .iter()
            .any(|(_, dependency)| dependency == "./foo"));
        let facade_info = structure
            .facade_info
            .as_ref()
            .expect("facade info should be populated for parsed JS/TS files");
        assert!(facade_info.is_facade);
        assert_eq!(facade_info.submodule_count, 3);
        assert_eq!(facade_info.organization_quality, OrganizationQuality::Poor);
    }
}
