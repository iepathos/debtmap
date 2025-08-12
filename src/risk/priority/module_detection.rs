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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_base_dependencies_entry_point() {
        let deps = get_base_dependencies(&ModuleType::EntryPoint);
        assert_eq!(deps, vec!["cli", "core", "io"]);
    }

    #[test]
    fn test_get_base_dependencies_core() {
        let deps = get_base_dependencies(&ModuleType::Core);
        assert_eq!(deps, vec!["models", "utils"]);
    }

    #[test]
    fn test_get_base_dependencies_api() {
        let deps = get_base_dependencies(&ModuleType::Api);
        assert_eq!(deps, vec!["core", "models", "io"]);
    }

    #[test]
    fn test_get_base_dependencies_model() {
        let deps = get_base_dependencies(&ModuleType::Model);
        assert_eq!(deps, vec!["utils"]);
    }

    #[test]
    fn test_get_base_dependencies_io() {
        let deps = get_base_dependencies(&ModuleType::IO);
        assert_eq!(deps, vec!["models", "utils"]);
    }

    #[test]
    fn test_get_base_dependencies_default_cases() {
        // Test Utility, Test, and Unknown types which should return empty vectors
        let utility_deps = get_base_dependencies(&ModuleType::Utility);
        assert_eq!(utility_deps, Vec::<String>::new());

        let test_deps = get_base_dependencies(&ModuleType::Test);
        assert_eq!(test_deps, Vec::<String>::new());

        let unknown_deps = get_base_dependencies(&ModuleType::Unknown);
        assert_eq!(unknown_deps, Vec::<String>::new());
    }

    #[test]
    fn test_get_base_dependents_core() {
        let dependents = get_base_dependents(&ModuleType::Core);
        assert_eq!(dependents, vec!["api", "main", "transformers"]);
    }

    #[test]
    fn test_get_base_dependents_api() {
        let dependents = get_base_dependents(&ModuleType::Api);
        assert_eq!(dependents, vec!["main", "service"]);
    }

    #[test]
    fn test_get_base_dependents_model() {
        let dependents = get_base_dependents(&ModuleType::Model);
        assert_eq!(dependents, vec!["core", "api", "io"]);
    }

    #[test]
    fn test_get_base_dependents_io() {
        let dependents = get_base_dependents(&ModuleType::IO);
        assert_eq!(dependents, vec!["api", "main"]);
    }

    #[test]
    fn test_get_base_dependents_utility() {
        let dependents = get_base_dependents(&ModuleType::Utility);
        assert_eq!(dependents, vec!["core", "models", "api", "io", "main"]);
    }

    #[test]
    fn test_get_base_dependents_default_cases() {
        // Test EntryPoint, Test, and Unknown types which should return empty vectors
        let entry_point_deps = get_base_dependents(&ModuleType::EntryPoint);
        assert_eq!(entry_point_deps, Vec::<String>::new());

        let test_deps = get_base_dependents(&ModuleType::Test);
        assert_eq!(test_deps, Vec::<String>::new());

        let unknown_deps = get_base_dependents(&ModuleType::Unknown);
        assert_eq!(unknown_deps, Vec::<String>::new());
    }

    #[test]
    fn test_add_path_specific_dependencies_analyzers() {
        let mut dependencies = Vec::new();
        let mut dependents = Vec::new();

        add_path_specific_dependencies("src/analyzers/rust.rs", &mut dependencies, &mut dependents);

        assert!(dependencies.contains(&"syntax".to_string()));
        assert!(dependents.contains(&"main".to_string()));
    }

    #[test]
    fn test_add_path_specific_dependencies_risk() {
        let mut dependencies = Vec::new();
        let mut dependents = Vec::new();

        add_path_specific_dependencies("src/risk/priority.rs", &mut dependencies, &mut dependents);

        assert!(dependencies.contains(&"complexity".to_string()));
        assert!(dependencies.contains(&"debt".to_string()));
        assert!(dependents.contains(&"reporting".to_string()));
    }

    #[test]
    fn test_add_path_specific_dependencies_complexity() {
        let mut dependencies = Vec::new();
        let mut dependents = Vec::new();

        add_path_specific_dependencies(
            "src/complexity/metrics.rs",
            &mut dependencies,
            &mut dependents,
        );

        assert!(dependencies.contains(&"ast".to_string()));
        assert!(dependents.contains(&"risk".to_string()));
        assert!(dependents.contains(&"reporting".to_string()));
    }

    #[test]
    fn test_add_path_specific_dependencies_output_and_writers() {
        // Test "output" path
        let mut dependencies = Vec::new();
        let mut dependents = Vec::new();

        add_path_specific_dependencies("src/io/output.rs", &mut dependencies, &mut dependents);

        assert!(dependencies.contains(&"models".to_string()));
        assert!(dependents.contains(&"cli".to_string()));

        // Test "writers" path
        dependencies.clear();
        dependents.clear();

        add_path_specific_dependencies(
            "src/io/writers/json.rs",
            &mut dependencies,
            &mut dependents,
        );

        assert!(dependencies.contains(&"models".to_string()));
        assert!(dependents.contains(&"cli".to_string()));
    }
}
