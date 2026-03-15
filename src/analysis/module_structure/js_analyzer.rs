//! JavaScript/TypeScript module structure analysis
//!
//! Provides parser-backed analysis of JavaScript and TypeScript source files.

use std::path::Path;

use super::types::{ComponentDependencyGraph, FunctionCounts, ModuleComponent, ModuleStructure};
use crate::analyzers::typescript::parser::parse_source;
use crate::analyzers::typescript::visitor::class_analysis::extract_classes;
use crate::analyzers::typescript::visitor::function_analysis::extract_functions;
use crate::core::ast::JsLanguageVariant;

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
        dependencies: ComponentDependencyGraph::new(),
        facade_info: None,
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
}
