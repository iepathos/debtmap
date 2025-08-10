use std::path::Path;

#[derive(Clone, Debug, PartialEq)]
pub enum ModuleType {
    EntryPoint,
    Core,
    Api,
    Model,
    IO,
    Utility,
    Test,
    Unknown,
}

pub struct ModuleTypeDetector;

impl ModuleTypeDetector {
    pub fn is_entry_point(file_name: &str) -> bool {
        matches!(file_name, "main.rs" | "lib.rs")
    }

    pub fn from_path_patterns(path_str: &str) -> ModuleType {
        const PATTERNS: &[(&[&str], ModuleType)] = &[
            (&["test"], ModuleType::Test),
            (&["core"], ModuleType::Core),
            (&["api", "handler"], ModuleType::Api),
            (&["model"], ModuleType::Model),
            (&["io", "output"], ModuleType::IO),
            (&["util", "helper"], ModuleType::Utility),
        ];

        PATTERNS
            .iter()
            .find(|(keywords, _)| keywords.iter().any(|k| path_str.contains(k)))
            .map(|(_, module_type)| module_type.clone())
            .unwrap_or(ModuleType::Unknown)
    }
}

pub fn determine_module_type(path: &Path) -> ModuleType {
    let path_str = path.to_string_lossy().to_lowercase();

    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        if ModuleTypeDetector::is_entry_point(file_name) {
            return ModuleType::EntryPoint;
        }
    }

    ModuleTypeDetector::from_path_patterns(&path_str)
}

fn get_base_dependencies(module_type: &ModuleType) -> Vec<String> {
    match module_type {
        ModuleType::EntryPoint => vec!["cli", "core", "io"],
        ModuleType::Core => vec!["models", "utils"],
        ModuleType::Api => vec!["core", "models", "io"],
        ModuleType::Model => vec!["utils"],
        ModuleType::IO => vec!["models", "utils"],
        _ => vec![],
    }
    .into_iter()
    .map(String::from)
    .collect()
}

fn get_base_dependents(module_type: &ModuleType) -> Vec<String> {
    match module_type {
        ModuleType::Core => vec!["api", "main", "transformers"],
        ModuleType::Api => vec!["main", "service"],
        ModuleType::Model => vec!["core", "api", "io"],
        ModuleType::IO => vec!["api", "main"],
        ModuleType::Utility => vec!["core", "models", "api", "io", "main"],
        _ => vec![],
    }
    .into_iter()
    .map(String::from)
    .collect()
}

fn add_path_specific_dependencies(
    path_str: &str,
    dependencies: &mut Vec<String>,
    dependents: &mut Vec<String>,
) {
    const PATH_RULES: &[(&str, &[&str], &[&str])] = &[
        ("analyzers", &["syntax"], &["main"]),
        ("risk", &["complexity", "debt"], &["reporting"]),
        ("complexity", &["ast"], &["risk", "reporting"]),
    ];

    for (pattern, deps, depts) in PATH_RULES {
        if path_str.contains(pattern) {
            dependencies.extend(deps.iter().map(|s| s.to_string()));
            dependents.extend(depts.iter().map(|s| s.to_string()));
        }
    }

    if path_str.contains("output") || path_str.contains("writers") {
        dependencies.push("models".to_string());
        dependents.push("cli".to_string());
    }
}

pub fn infer_module_relationships(
    path: &Path,
    module_type: &ModuleType,
) -> (Vec<String>, Vec<String>) {
    let path_str = path.to_string_lossy();
    let mut dependencies = get_base_dependencies(module_type);
    let mut dependents = get_base_dependents(module_type);

    add_path_specific_dependencies(&path_str, &mut dependencies, &mut dependents);

    (dependencies, dependents)
}
