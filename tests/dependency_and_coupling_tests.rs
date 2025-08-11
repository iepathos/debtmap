use debtmap::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_duplication_detection() {
    let temp_dir = TempDir::new().unwrap();

    // Create two files with duplicate code
    let file1 = temp_dir.path().join("file1.rs");
    let file2 = temp_dir.path().join("file2.rs");

    let duplicate_code = r#"
fn calculate_sum(a: i32, b: i32) -> i32 {
    let result = a + b;
    println!("Sum: {}", result);
    result
}

fn another_function() {
    println!("This is unique");
}
"#;

    fs::write(&file1, duplicate_code).unwrap();
    fs::write(&file2, duplicate_code).unwrap();

    let files = vec![
        (file1.clone(), fs::read_to_string(&file1).unwrap()),
        (file2.clone(), fs::read_to_string(&file2).unwrap()),
    ];

    let duplications = detect_duplication(files, 3, 0.8);
    assert!(!duplications.is_empty(), "Should detect duplicate code");

    // Verify duplication details
    let dup = &duplications[0];
    assert_eq!(dup.locations.len(), 2, "Should find duplication in 2 files");
    assert!(dup.lines >= 3, "Should detect at least 3 duplicate lines");
}

#[test]
fn test_circular_dependency_detection() {
    let mut graph = DependencyGraph::new();

    // Create a circular dependency: A -> B -> C -> A
    graph.add_dependency("module_a".to_string(), "module_b".to_string());
    graph.add_dependency("module_b".to_string(), "module_c".to_string());
    graph.add_dependency("module_c".to_string(), "module_a".to_string());

    let circular_deps = graph.detect_circular_dependencies();
    assert!(
        !circular_deps.is_empty(),
        "Should detect circular dependency"
    );

    // Verify the cycle contains all three modules
    let cycle = &circular_deps[0].cycle;
    assert!(cycle.contains(&"module_a".to_string()));
    assert!(cycle.contains(&"module_b".to_string()));
    assert!(cycle.contains(&"module_c".to_string()));
}

#[test]
fn test_self_dependency() {
    let mut graph = DependencyGraph::new();

    // Create a self-dependency
    graph.add_dependency("module_self".to_string(), "module_self".to_string());

    let circular_deps = graph.detect_circular_dependencies();
    assert!(!circular_deps.is_empty(), "Should detect self-dependency");
}

#[test]
fn test_coupling_metrics() {
    let modules = vec![
        ModuleDependency {
            module: "module_a".to_string(),
            dependencies: vec!["module_b".to_string(), "module_c".to_string()],
            dependents: vec!["module_d".to_string()],
        },
        ModuleDependency {
            module: "module_b".to_string(),
            dependencies: vec!["module_c".to_string()],
            dependents: vec!["module_a".to_string()],
        },
    ];

    let metrics = calculate_coupling_metrics(&modules);

    // Check module_a metrics
    let module_a_metrics = metrics.get("module_a").unwrap();
    assert_eq!(module_a_metrics.efferent_coupling, 2);
    assert_eq!(module_a_metrics.afferent_coupling, 1);
    assert!((module_a_metrics.instability - 0.666).abs() < 0.01);

    // Check module_b metrics
    let module_b_metrics = metrics.get("module_b").unwrap();
    assert_eq!(module_b_metrics.efferent_coupling, 1);
    assert_eq!(module_b_metrics.afferent_coupling, 1);
    assert_eq!(module_b_metrics.instability, 0.5);
}

#[test]
fn test_dependency_graph_operations() {
    let mut graph = DependencyGraph::new();

    // Add modules and dependencies
    graph.add_module("core".to_string());
    graph.add_dependency("ui".to_string(), "core".to_string());
    graph.add_dependency("api".to_string(), "core".to_string());
    graph.add_dependency("ui".to_string(), "api".to_string());

    // Calculate coupling metrics
    let metrics = graph.calculate_coupling_metrics();

    // Find core module metrics
    let core_metrics = metrics.iter().find(|m| m.module == "core").unwrap();
    assert_eq!(
        core_metrics.dependents.len(),
        2,
        "Core should have 2 dependents"
    );
    assert_eq!(
        core_metrics.dependencies.len(),
        0,
        "Core should have no dependencies"
    );

    // Find ui module metrics
    let ui_metrics = metrics.iter().find(|m| m.module == "ui").unwrap();
    assert_eq!(
        ui_metrics.dependencies.len(),
        2,
        "UI should depend on 2 modules"
    );
}

#[test]
fn test_build_module_dependency_map_empty() {
    use debtmap::debt::coupling::build_module_dependency_map;

    let file_dependencies = vec![];
    let result = build_module_dependency_map(&file_dependencies);

    assert_eq!(result.len(), 0, "Empty input should produce empty output");
}

#[test]
fn test_build_module_dependency_map_single_file_no_deps() {
    use debtmap::debt::coupling::build_module_dependency_map;
    use std::path::PathBuf;

    let file_dependencies = vec![(PathBuf::from("src/main.rs"), vec![])];

    let result = build_module_dependency_map(&file_dependencies);

    assert_eq!(result.len(), 1, "Should have one module");
    let module = &result[0];
    assert_eq!(module.module, "main");
    assert_eq!(module.dependencies.len(), 0);
    assert_eq!(module.dependents.len(), 0);
}

#[test]
fn test_build_module_dependency_map_with_imports() {
    use debtmap::debt::coupling::build_module_dependency_map;
    use debtmap::{Dependency, DependencyKind};
    use std::path::PathBuf;

    let file_dependencies = vec![
        (
            PathBuf::from("src/ui.rs"),
            vec![
                Dependency {
                    name: "core::utils".to_string(),
                    kind: DependencyKind::Import,
                },
                Dependency {
                    name: "api::client".to_string(),
                    kind: DependencyKind::Import,
                },
            ],
        ),
        (PathBuf::from("src/core.rs"), vec![]),
        (
            PathBuf::from("src/api.rs"),
            vec![Dependency {
                name: "core::config".to_string(),
                kind: DependencyKind::Import,
            }],
        ),
    ];

    let result = build_module_dependency_map(&file_dependencies);

    // Find the ui module
    let ui_module = result.iter().find(|m| m.module == "ui").unwrap();
    assert_eq!(ui_module.dependencies.len(), 2);
    assert!(ui_module.dependencies.contains(&"core".to_string()));
    assert!(ui_module.dependencies.contains(&"api".to_string()));

    // Find the core module - should have ui and api as dependents
    let core_module = result.iter().find(|m| m.module == "core").unwrap();
    assert_eq!(core_module.dependencies.len(), 0);
    assert_eq!(core_module.dependents.len(), 2);
    assert!(core_module.dependents.contains(&"ui".to_string()));
    assert!(core_module.dependents.contains(&"api".to_string()));

    // Find the api module
    let api_module = result.iter().find(|m| m.module == "api").unwrap();
    assert_eq!(api_module.dependencies.len(), 1);
    assert!(api_module.dependencies.contains(&"core".to_string()));
    assert_eq!(api_module.dependents.len(), 1);
    assert!(api_module.dependents.contains(&"ui".to_string()));
}

#[test]
fn test_build_module_dependency_map_filters_non_import_deps() {
    use debtmap::debt::coupling::build_module_dependency_map;
    use debtmap::{Dependency, DependencyKind};
    use std::path::PathBuf;

    let file_dependencies = vec![(
        PathBuf::from("src/main.rs"),
        vec![
            Dependency {
                name: "std::collections".to_string(),
                kind: DependencyKind::Import,
            },
            Dependency {
                name: "serde".to_string(),
                kind: DependencyKind::Package, // Should be filtered out
            },
            Dependency {
                name: "utils".to_string(),
                kind: DependencyKind::Module,
            },
        ],
    )];

    let result = build_module_dependency_map(&file_dependencies);

    let main_module = result.iter().find(|m| m.module == "main").unwrap();
    assert_eq!(main_module.dependencies.len(), 2);
    assert!(main_module.dependencies.contains(&"std".to_string()));
    assert!(main_module.dependencies.contains(&"utils".to_string()));
    // serde should not be included as it's a Package dependency
}

#[test]
fn test_build_module_dependency_map_bidirectional_deps() {
    use debtmap::debt::coupling::build_module_dependency_map;
    use debtmap::{Dependency, DependencyKind};
    use std::path::PathBuf;

    let file_dependencies = vec![
        (
            PathBuf::from("src/module_a.rs"),
            vec![Dependency {
                name: "module_b::function".to_string(),
                kind: DependencyKind::Import,
            }],
        ),
        (
            PathBuf::from("src/module_b.rs"),
            vec![Dependency {
                name: "module_a::types".to_string(),
                kind: DependencyKind::Import,
            }],
        ),
    ];

    let result = build_module_dependency_map(&file_dependencies);

    // Both modules should appear in each other's dependencies and dependents
    let module_a = result.iter().find(|m| m.module == "module_a").unwrap();
    assert_eq!(module_a.dependencies.len(), 1);
    assert!(module_a.dependencies.contains(&"module_b".to_string()));
    assert_eq!(module_a.dependents.len(), 1);
    assert!(module_a.dependents.contains(&"module_b".to_string()));

    let module_b = result.iter().find(|m| m.module == "module_b").unwrap();
    assert_eq!(module_b.dependencies.len(), 1);
    assert!(module_b.dependencies.contains(&"module_a".to_string()));
    assert_eq!(module_b.dependents.len(), 1);
    assert!(module_b.dependents.contains(&"module_a".to_string()));
}

#[test]
fn test_build_module_dependency_map_complex_paths() {
    use debtmap::debt::coupling::build_module_dependency_map;
    use debtmap::{Dependency, DependencyKind};
    use std::path::PathBuf;

    let file_dependencies = vec![
        (
            PathBuf::from("src/components/header.rs"),
            vec![Dependency {
                name: "utils::helpers::format".to_string(),
                kind: DependencyKind::Import,
            }],
        ),
        (PathBuf::from("src/utils/helpers.rs"), vec![]),
        (
            PathBuf::from("/absolute/path/to/footer.rs"),
            vec![Dependency {
                name: "header".to_string(),
                kind: DependencyKind::Module,
            }],
        ),
    ];

    let result = build_module_dependency_map(&file_dependencies);

    // Check that module names are extracted correctly from paths
    assert!(result.iter().any(|m| m.module == "header"));
    assert!(result.iter().any(|m| m.module == "helpers"));
    assert!(result.iter().any(|m| m.module == "footer"));

    // Verify dependency relationships
    let header_module = result.iter().find(|m| m.module == "header").unwrap();
    assert!(header_module.dependencies.contains(&"utils".to_string()));
    assert!(header_module.dependents.contains(&"footer".to_string()));
}
