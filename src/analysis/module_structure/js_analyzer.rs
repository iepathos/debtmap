//! JavaScript/TypeScript module structure analysis
//!
//! Provides basic text-based analysis of JavaScript and TypeScript source files.
//! Detects classes and function definitions through pattern matching.

use std::path::Path;

use super::types::{ComponentDependencyGraph, FunctionCounts, ModuleComponent, ModuleStructure};

/// Analyze a JavaScript source file to extract module structure
pub fn analyze_javascript_file(content: &str, _file_path: &Path) -> ModuleStructure {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    let (components, public_count, private_count) = extract_js_components(&lines);
    let responsibility_count = (components.len() / 5).max(1);

    ModuleStructure {
        total_lines,
        components,
        function_counts: FunctionCounts {
            module_level_functions: public_count + private_count,
            impl_methods: 0,
            trait_methods: 0,
            nested_module_functions: 0,
            public_functions: public_count,
            private_functions: private_count,
        },
        responsibility_count,
        public_api_surface: public_count,
        dependencies: ComponentDependencyGraph::new(),
        facade_info: None,
    }
}

/// Analyze a TypeScript source file to extract module structure
///
/// TypeScript analysis is similar to JavaScript with type annotations.
/// For now, delegates to JavaScript analyzer.
pub fn analyze_typescript_file(content: &str, file_path: &Path) -> ModuleStructure {
    analyze_javascript_file(content, file_path)
}

/// Extract components from JavaScript source lines
fn extract_js_components(lines: &[&str]) -> (Vec<ModuleComponent>, usize, usize) {
    let mut components = Vec::new();
    let mut public_count = 0;
    let mut private_count = 0;

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if let Some(comp) = try_extract_class(trimmed, idx) {
            components.push(comp);
        }

        if let Some((comp, is_export)) = try_extract_function(trimmed) {
            if is_export {
                public_count += 1;
            } else {
                private_count += 1;
            }
            components.push(comp);
        }
    }

    (components, public_count, private_count)
}

/// Try to extract a class definition from a line
fn try_extract_class(line: &str, idx: usize) -> Option<ModuleComponent> {
    if !line.starts_with("class ") && !line.contains(" class ") {
        return None;
    }

    let name = line
        .split_whitespace()
        .skip_while(|&s| s != "class")
        .nth(1)
        .unwrap_or("Unknown")
        .split(['{', ' '])
        .next()
        .unwrap_or("Unknown")
        .to_string();

    Some(ModuleComponent::Struct {
        name,
        fields: 0,
        methods: 0,
        public: true,
        line_range: (idx, idx),
    })
}

/// Try to extract a function definition from a line
fn try_extract_function(line: &str) -> Option<(ModuleComponent, bool)> {
    let is_function = line.starts_with("function ")
        || line.starts_with("export function ")
        || line.contains("= function")
        || (line.contains("=> ") && (line.starts_with("const ") || line.starts_with("let ")));

    if !is_function {
        return None;
    }

    let is_export = line.starts_with("export ");

    let comp = ModuleComponent::ModuleLevelFunction {
        name: "function".to_string(),
        public: is_export,
        lines: 5,
        complexity: 1,
    };

    Some((comp, is_export))
}
