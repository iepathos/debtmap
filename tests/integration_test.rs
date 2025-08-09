use debtmap::*;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_todo_fixme_detection() {
    // debtmap:ignore-start -- Test fixture data
    let content = r#"
    // TODO: Implement this feature
    // FIXME: This is broken
    // HACK: Temporary workaround
    // XXX: This needs review
    // BUG: Known issue here
    // OPTIMIZE: Can be improved
    // REFACTOR: Needs cleanup
    "#;
    // debtmap:ignore-end

    let path = PathBuf::from("test.rs");
    let items = find_todos_and_fixmes(content, &path);

    assert_eq!(items.len(), 7, "Should find all debt markers");

    // Check that different markers have appropriate priorities
    let has_high_priority = items.iter().any(|item| item.priority == Priority::High);
    let has_medium_priority = items.iter().any(|item| item.priority == Priority::Medium);

    assert!(
        has_high_priority,
        "Should have high priority items (FIXME, BUG)"
    );
    assert!(
        has_medium_priority,
        "Should have medium priority items (TODO)"
    );

    // Verify specific items
    let todo_item = items.iter().find(|i| i.message.contains("TODO")).unwrap();
    assert_eq!(todo_item.priority, Priority::Medium);
    assert_eq!(todo_item.debt_type, DebtType::Todo);

    let fixme_item = items.iter().find(|i| i.message.contains("FIXME")).unwrap();
    assert_eq!(fixme_item.priority, Priority::High);
    assert_eq!(fixme_item.debt_type, DebtType::Fixme);
}

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
fn test_code_smell_long_parameter_list() {
    let func = FunctionMetrics {
        name: "test_function".to_string(),
        file: PathBuf::from("test.rs"),
        line: 10,
        cyclomatic: 5,
        cognitive: 8,
        nesting: 2,
        length: 30,
    };

    // Test with 7 parameters (over threshold of 5)
    let smell = detect_long_parameter_list(&func, 7);
    assert!(smell.is_some(), "Should detect long parameter list");

    let smell = smell.unwrap();
    assert_eq!(smell.smell_type, SmellType::LongParameterList);
    assert_eq!(smell.severity, Priority::Medium);

    // Test with 3 parameters (under threshold)
    let no_smell = detect_long_parameter_list(&func, 3);
    assert!(
        no_smell.is_none(),
        "Should not detect smell for short parameter list"
    );
}

#[test]
fn test_code_smell_long_method() {
    let mut func = FunctionMetrics {
        name: "test_function".to_string(),
        file: PathBuf::from("test.rs"),
        line: 10,
        cyclomatic: 5,
        cognitive: 8,
        nesting: 2,
        length: 100, // Over threshold of 50
    };

    let smell = detect_long_method(&func);
    assert!(smell.is_some(), "Should detect long method");

    let smell = smell.unwrap();
    assert_eq!(smell.smell_type, SmellType::LongMethod);
    assert_eq!(smell.severity, Priority::Medium);

    // Test with short method
    func.length = 30;
    let no_smell = detect_long_method(&func);
    assert!(
        no_smell.is_none(),
        "Should not detect smell for short method"
    );
}

#[test]
fn test_code_smell_deep_nesting() {
    let mut func = FunctionMetrics {
        name: "test_function".to_string(),
        file: PathBuf::from("test.rs"),
        line: 10,
        cyclomatic: 5,
        cognitive: 8,
        nesting: 6, // Over threshold of 4
        length: 30,
    };

    let smell = detect_deep_nesting(&func);
    assert!(smell.is_some(), "Should detect deep nesting");

    let smell = smell.unwrap();
    assert_eq!(smell.smell_type, SmellType::DeepNesting);
    assert_eq!(smell.severity, Priority::Medium);

    // Test with shallow nesting
    func.nesting = 2;
    let no_smell = detect_deep_nesting(&func);
    assert!(
        no_smell.is_none(),
        "Should not detect smell for shallow nesting"
    );
}

#[test]
fn test_complexity_metrics() {
    let functions = vec![
        FunctionMetrics {
            name: "simple".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic: 2,
            cognitive: 3,
            nesting: 1,
            length: 10,
        },
        FunctionMetrics {
            name: "complex".to_string(),
            file: PathBuf::from("test.rs"),
            line: 20,
            cyclomatic: 15,
            cognitive: 20,
            nesting: 5,
            length: 50,
        },
        FunctionMetrics {
            name: "medium".to_string(),
            file: PathBuf::from("test.rs"),
            line: 80,
            cyclomatic: 8,
            cognitive: 10,
            nesting: 3,
            length: 30,
        },
    ];

    let avg = calculate_average_complexity(&functions);
    assert_eq!(
        avg, 8.333333333333334,
        "Should calculate correct average complexity"
    );

    let max = find_max_complexity(&functions);
    assert_eq!(max, 15, "Should find maximum complexity");

    let high_count = count_high_complexity(&functions, 10);
    assert_eq!(
        high_count, 1,
        "Should count high complexity functions correctly"
    );
}

#[test]
fn test_function_is_complex() {
    let func = FunctionMetrics {
        name: "test".to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic: 15,
        cognitive: 8,
        nesting: 2,
        length: 30,
    };

    assert!(
        func.is_complex(10),
        "Should be complex when cyclomatic > threshold"
    );
    assert!(
        !func.is_complex(20),
        "Should not be complex when both metrics < threshold"
    );
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
fn test_output_json_format() {
    use chrono::Utc;

    let results = AnalysisResults {
        project_path: PathBuf::from("/test/project"),
        timestamp: Utc::now(),
        complexity: ComplexityReport {
            metrics: vec![FunctionMetrics {
                name: "test_func".to_string(),
                file: PathBuf::from("test.rs"),
                line: 10,
                cyclomatic: 5,
                cognitive: 7,
                nesting: 2,
                length: 25,
            }],
            summary: ComplexitySummary {
                total_functions: 1,
                average_complexity: 5.0,
                max_complexity: 5,
                high_complexity_count: 0,
            },
        },
        technical_debt: TechnicalDebtReport {
            items: vec![DebtItem {
                id: "test-1".to_string(),
                debt_type: DebtType::Todo,
                priority: Priority::Medium,
                file: PathBuf::from("test.rs"),
                line: 5,
                message: "TODO: Implement feature".to_string(), // debtmap:ignore -- Test fixture
                context: None,
            }],
            by_type: {
                let mut map = HashMap::new();
                map.insert(DebtType::Todo, vec![]);
                map
            },
            priorities: vec![Priority::Medium],
            duplications: vec![],
        },
        dependencies: DependencyReport {
            modules: vec![],
            circular: vec![],
        },
        duplications: vec![],
    };

    let mut writer = create_writer(OutputFormat::Json);
    let result = writer.write_results(&results);
    assert!(result.is_ok(), "JSON output should succeed");
}

#[test]
fn test_output_markdown_format() {
    use chrono::Utc;

    let results = AnalysisResults {
        project_path: PathBuf::from("/test/project"),
        timestamp: Utc::now(),
        complexity: ComplexityReport {
            metrics: vec![],
            summary: ComplexitySummary {
                total_functions: 10,
                average_complexity: 5.5,
                max_complexity: 15,
                high_complexity_count: 2,
            },
        },
        technical_debt: TechnicalDebtReport {
            items: vec![],
            by_type: HashMap::new(),
            priorities: vec![],
            duplications: vec![],
        },
        dependencies: DependencyReport {
            modules: vec![],
            circular: vec![],
        },
        duplications: vec![],
    };

    let mut writer = create_writer(OutputFormat::Markdown);
    let result = writer.write_results(&results);
    assert!(result.is_ok(), "Markdown output should succeed");
}

#[test]
fn test_code_smell_detection_multiple() {
    let func = FunctionMetrics {
        name: "bad_function".to_string(),
        file: PathBuf::from("test.rs"),
        line: 10,
        cyclomatic: 15,
        cognitive: 20,
        nesting: 6,
        length: 100,
    };

    let smells = analyze_function_smells(&func, 7);

    // Should detect multiple smells
    assert!(smells.len() >= 3, "Should detect multiple code smells");

    // Check specific smells are detected
    let has_long_params = smells
        .iter()
        .any(|s| matches!(s.smell_type, SmellType::LongParameterList));
    let has_long_method = smells
        .iter()
        .any(|s| matches!(s.smell_type, SmellType::LongMethod));
    let has_deep_nesting = smells
        .iter()
        .any(|s| matches!(s.smell_type, SmellType::DeepNesting));

    assert!(has_long_params, "Should detect long parameter list");
    assert!(has_long_method, "Should detect long method");
    assert!(has_deep_nesting, "Should detect deep nesting");
}

#[test]
fn test_module_smell_detection() {
    let path = PathBuf::from("large_module.rs");

    // Test large module detection
    let smells = analyze_module_smells(&path, 500);
    assert!(!smells.is_empty(), "Should detect large module");

    let smell = &smells[0];
    assert_eq!(smell.smell_type, SmellType::LargeClass);
    assert_eq!(smell.severity, Priority::Medium);

    // Test normal size module
    let no_smells = analyze_module_smells(&path, 200);
    assert!(
        no_smells.is_empty(),
        "Should not detect smell for normal size module"
    );
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
fn test_debt_item_creation() {
    let item = DebtItem {
        id: "test-debt-1".to_string(),
        debt_type: DebtType::Complexity,
        priority: Priority::High,
        file: PathBuf::from("complex.rs"),
        line: 42,
        message: "Function has high complexity".to_string(),
        context: Some("fn complex_function() { ... }".to_string()),
    };

    assert_eq!(item.debt_type, DebtType::Complexity);
    assert_eq!(item.priority, Priority::High);
    assert_eq!(item.line, 42);
    assert!(item.context.is_some());
}

#[test]
fn test_priority_ordering() {
    let low = Priority::Low;
    let medium = Priority::Medium;
    let high = Priority::High;
    let critical = Priority::Critical;

    assert!(low < medium);
    assert!(medium < high);
    assert!(high < critical);
    assert!(critical > low);
}

#[test]
fn test_language_detection() {
    assert_eq!(Language::from_extension("rs"), Language::Rust);
    assert_eq!(Language::from_extension("py"), Language::Python);
    assert_eq!(Language::from_extension("unknown"), Language::Unknown);
}

#[test]
fn test_code_smell_to_debt_item() {
    let smell = CodeSmell {
        smell_type: SmellType::LongMethod,
        location: PathBuf::from("test.rs"),
        line: 100,
        message: "Method is too long".to_string(),
        severity: Priority::High,
    };

    let debt_item = smell.to_debt_item();

    assert_eq!(debt_item.debt_type, DebtType::CodeSmell);
    assert_eq!(debt_item.priority, Priority::High);
    assert_eq!(debt_item.line, 100);
    assert_eq!(debt_item.message, "Method is too long");
}

#[test]
fn test_suppression_block_comments() {
    let content = r#"
// debtmap:ignore-start
// TODO: This should be suppressed
// FIXME: This too
// debtmap:ignore-end
// TODO: This should not be suppressed
"#;

    let path = PathBuf::from("test.rs");
    let items = find_todos_and_fixmes_with_suppression(
        content,
        &path,
        Some(&parse_suppression_comments(content, Language::Rust, &path)),
    );

    assert_eq!(items.len(), 1, "Should only find one non-suppressed TODO");
    assert!(items[0].message.contains("This should not be suppressed"));
}

#[test]
fn test_suppression_line_comments() {
    // debtmap:ignore-start -- Test fixture data
    let content = r#"
// TODO: Not suppressed
// TODO: Suppressed // debtmap:ignore
// FIXME: Also not suppressed
"#;
    // debtmap:ignore-end

    let path = PathBuf::from("test.rs");
    let items = find_todos_and_fixmes_with_suppression(
        content,
        &path,
        Some(&parse_suppression_comments(content, Language::Rust, &path)),
    );

    assert_eq!(items.len(), 2, "Should find two non-suppressed items");
    assert!(!items.iter().any(|i| i.message.contains("Suppressed")));
}

#[test]
fn test_suppression_next_line() {
    let content = r#"
// debtmap:ignore-next-line
// TODO: This should be suppressed
// TODO: This should not be suppressed
"#;

    let path = PathBuf::from("test.rs");
    let items = find_todos_and_fixmes_with_suppression(
        content,
        &path,
        Some(&parse_suppression_comments(content, Language::Rust, &path)),
    );

    assert_eq!(items.len(), 1, "Should only find one non-suppressed TODO");
    assert!(items[0].message.contains("This should not be suppressed"));
}

#[test]
fn test_type_specific_suppression() {
    // debtmap:ignore-start -- Test fixture data
    let content = r#"
// debtmap:ignore-start[todo]
// TODO: Suppressed
// FIXME: Not suppressed  
// debtmap:ignore-end
"#;
    // debtmap:ignore-end

    let path = PathBuf::from("test.rs");
    let items = find_todos_and_fixmes_with_suppression(
        content,
        &path,
        Some(&parse_suppression_comments(content, Language::Rust, &path)),
    );

    assert_eq!(items.len(), 1, "Should only find FIXME");
    assert!(items[0].debt_type == DebtType::Fixme);
}

#[test]
fn test_suppression_with_reason() {
    let content = r#"
// debtmap:ignore-start -- Test fixture data
// TODO: Test TODO
// FIXME: Test FIXME
// debtmap:ignore-end
"#;

    let path = PathBuf::from("test.rs");
    let suppression = parse_suppression_comments(content, Language::Rust, &path);
    let items = find_todos_and_fixmes_with_suppression(content, &path, Some(&suppression));

    assert_eq!(items.len(), 0, "All items should be suppressed");
    assert_eq!(
        suppression.active_blocks[0].reason,
        Some("Test fixture data".to_string())
    );
}

#[test]
fn test_python_suppression() {
    // debtmap:ignore-start -- Test fixture data
    let content = r#"
# debtmap:ignore-start
# TODO: Python TODO
# FIXME: Python FIXME
# debtmap:ignore-end
# TODO: Not suppressed
"#;
    // debtmap:ignore-end

    let path = PathBuf::from("test.py");
    let items = find_todos_and_fixmes_with_suppression(
        content,
        &path,
        Some(&parse_suppression_comments(
            content,
            Language::Python,
            &path,
        )),
    );

    assert_eq!(items.len(), 1, "Should only find one non-suppressed TODO");
    assert!(items[0].message.contains("Not suppressed"));
}

#[test]
fn test_code_smell_suppression() {
    let content = r#"
// debtmap:ignore-next-line[codesmell]
                                                                                                    // This is a very long line that would normally trigger a code smell
// This is a normal line
"#;

    let path = PathBuf::from("test.rs");
    let items = find_code_smells_with_suppression(
        content,
        &path,
        Some(&parse_suppression_comments(content, Language::Rust, &path)),
    );

    assert_eq!(items.len(), 0, "Long line should be suppressed");
}

#[test]
fn test_wildcard_suppression() {
    // debtmap:ignore-start -- Test fixture data
    let content = r#"
// debtmap:ignore[*]
// TODO: Suppressed
// FIXME: Also suppressed
"#;
    // debtmap:ignore-end

    let path = PathBuf::from("test.rs");
    let suppression = parse_suppression_comments(content, Language::Rust, &path);

    assert!(suppression.is_suppressed(2, &DebtType::Todo));
    assert!(suppression.is_suppressed(2, &DebtType::Fixme));
    assert!(suppression.is_suppressed(2, &DebtType::CodeSmell));
}
