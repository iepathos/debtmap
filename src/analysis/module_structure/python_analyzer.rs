//! Python module structure analysis
//!
//! Provides basic text-based analysis of Python source files.
//! Detects classes and function definitions through pattern matching.

use std::path::Path;

use super::types::{ComponentDependencyGraph, FunctionCounts, ModuleComponent, ModuleStructure};

/// Analyze a Python source file to extract module structure
pub fn analyze_python_file(content: &str, _file_path: &Path) -> ModuleStructure {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    let (components, public_count, private_count) = extract_python_components(&lines);
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

/// Extract components from Python source lines
fn extract_python_components(lines: &[&str]) -> (Vec<ModuleComponent>, usize, usize) {
    let mut components = Vec::new();
    let mut public_count = 0;
    let mut private_count = 0;

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if let Some(comp) = try_extract_class(trimmed, idx) {
            components.push(comp);
        }

        if let Some((comp, is_public)) = try_extract_function(trimmed) {
            if is_public {
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
    if !line.starts_with("class ") {
        return None;
    }

    let name = line
        .strip_prefix("class ")
        .and_then(|s| s.split(['(', ':']).next())
        .unwrap_or("Unknown")
        .trim()
        .to_string();

    let public = !name.starts_with('_');

    Some(ModuleComponent::Struct {
        name,
        fields: 0,
        methods: 0,
        public,
        line_range: (idx, idx),
    })
}

/// Try to extract a function definition from a line
fn try_extract_function(line: &str) -> Option<(ModuleComponent, bool)> {
    if !line.starts_with("def ") {
        return None;
    }

    let name = line
        .strip_prefix("def ")
        .and_then(|s| s.split('(').next())
        .unwrap_or("unknown")
        .trim()
        .to_string();

    let public = !name.starts_with('_');

    let comp = ModuleComponent::ModuleLevelFunction {
        name,
        public,
        lines: 5, // Estimate
        complexity: 1,
    };

    Some((comp, public))
}
