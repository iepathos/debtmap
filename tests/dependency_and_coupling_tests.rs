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
