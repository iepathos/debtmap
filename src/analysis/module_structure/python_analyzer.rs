//! Python module structure analysis
//!
//! Provides parser-backed analysis of Python source files.

use std::path::Path;

use super::types::{ComponentDependencyGraph, FunctionCounts, ModuleComponent, ModuleStructure};
use crate::analyzers::python::parser::parse_source;
use crate::extraction::python::PythonExtractor;

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

    ModuleStructure {
        total_lines: extracted.total_lines,
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
}
