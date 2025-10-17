use debtmap::analyzers::{python::PythonAnalyzer, Analyzer};
use debtmap::core::Language;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_pattern_extraction_observer_pattern() {
    // Read the observer pattern fixture
    let fixture_path = PathBuf::from("tests/fixtures/pattern_extraction/observer.py");
    let content = fs::read_to_string(&fixture_path).expect("Failed to read fixture");

    // Parse and analyze
    let analyzer = PythonAnalyzer::new();
    let ast = analyzer
        .parse(&content, fixture_path.clone())
        .expect("Failed to parse Python code");
    let metrics = analyzer.analyze(&ast);

    // Verify language
    assert_eq!(metrics.language, Language::Python);

    // Verify classes were extracted
    assert!(
        metrics.classes.is_some(),
        "Classes should be extracted from the file"
    );
    let classes = metrics.classes.as_ref().unwrap();
    assert!(
        classes.len() >= 5,
        "Should extract at least 5 classes (Observer, Subject, ConcreteObserver, EventManager, Configuration)"
    );

    // Verify Observer class has ABC base and abstractmethod
    let observer = classes
        .iter()
        .find(|c| c.name == "Observer")
        .expect("Should find Observer class");
    assert!(
        observer.base_classes.contains(&"ABC".to_string()),
        "Observer should inherit from ABC"
    );
    assert!(
        observer.is_abstract,
        "Observer should be marked as abstract"
    );
    assert!(
        observer.methods.iter().any(|m| m.is_abstract),
        "Observer should have abstract methods"
    );

    // Verify ConcreteObserver has dataclass decorator
    let concrete_observer = classes
        .iter()
        .find(|c| c.name == "ConcreteObserver")
        .expect("Should find ConcreteObserver class");
    assert!(
        concrete_observer
            .decorators
            .contains(&"dataclass".to_string()),
        "ConcreteObserver should have @dataclass decorator"
    );
    assert!(
        concrete_observer
            .base_classes
            .contains(&"Observer".to_string()),
        "ConcreteObserver should inherit from Observer"
    );

    // Verify EventManager inherits from Subject
    let event_manager = classes
        .iter()
        .find(|c| c.name == "EventManager")
        .expect("Should find EventManager class");
    assert!(
        event_manager.base_classes.contains(&"Subject".to_string()),
        "EventManager should inherit from Subject"
    );

    // Verify module scope analysis
    assert!(
        metrics.module_scope.is_some(),
        "Module scope should be analyzed"
    );
    let module_scope = metrics.module_scope.as_ref().unwrap();

    // Verify singleton instances were detected
    assert!(
        !module_scope.singleton_instances.is_empty(),
        "Should detect module-level singleton instances"
    );

    // Verify event_manager singleton
    let event_manager_singleton = module_scope
        .singleton_instances
        .iter()
        .find(|s| s.variable_name == "event_manager");
    assert!(
        event_manager_singleton.is_some(),
        "Should detect 'event_manager' singleton"
    );
    let event_manager_singleton = event_manager_singleton.unwrap();
    assert_eq!(
        event_manager_singleton.class_name, "EventManager",
        "event_manager should be instance of EventManager"
    );

    // Verify config singleton
    let config_singleton = module_scope
        .singleton_instances
        .iter()
        .find(|s| s.variable_name == "config");
    assert!(
        config_singleton.is_some(),
        "Should detect 'config' singleton"
    );
    let config_singleton = config_singleton.unwrap();
    assert_eq!(
        config_singleton.class_name, "Configuration",
        "config should be instance of Configuration"
    );

    // Verify module-level assignments
    assert!(
        module_scope.assignments.len() >= 2,
        "Should detect at least 2 module-level assignments (event_manager, config)"
    );
}

#[test]
fn test_pattern_extraction_preserves_function_metrics() {
    // Ensure AST extraction doesn't break existing function analysis
    let fixture_path = PathBuf::from("tests/fixtures/pattern_extraction/observer.py");
    let content = fs::read_to_string(&fixture_path).expect("Failed to read fixture");

    let analyzer = PythonAnalyzer::new();
    let ast = analyzer
        .parse(&content, fixture_path.clone())
        .expect("Failed to parse Python code");
    let metrics = analyzer.analyze(&ast);

    // Verify functions were analyzed
    assert!(
        !metrics.complexity.functions.is_empty(),
        "Should analyze functions"
    );

    // Verify specific methods exist
    let method_names: Vec<String> = metrics
        .complexity
        .functions
        .iter()
        .map(|f| f.name.clone())
        .collect();

    assert!(
        method_names.iter().any(|n| n.contains("on_event")),
        "Should find on_event method"
    );
    assert!(
        method_names.iter().any(|n| n.contains("attach")),
        "Should find attach method"
    );
    assert!(
        method_names.iter().any(|n| n.contains("notify")),
        "Should find notify method"
    );

    // Verify complexity metrics are still calculated
    assert!(
        metrics.complexity.cyclomatic_complexity > 0,
        "Should calculate cyclomatic complexity"
    );
}

#[test]
fn test_abstract_method_detection() {
    let fixture_path = PathBuf::from("tests/fixtures/pattern_extraction/observer.py");
    let content = fs::read_to_string(&fixture_path).expect("Failed to read fixture");

    let analyzer = PythonAnalyzer::new();
    let ast = analyzer
        .parse(&content, fixture_path.clone())
        .expect("Failed to parse Python code");
    let metrics = analyzer.analyze(&ast);

    let classes = metrics.classes.as_ref().unwrap();

    // Find Observer class and verify abstract method
    let observer = classes.iter().find(|c| c.name == "Observer").unwrap();
    let on_event = observer
        .methods
        .iter()
        .find(|m| m.name == "on_event")
        .expect("Should find on_event method");

    assert!(
        on_event.decorators.contains(&"abstractmethod".to_string()),
        "on_event should have @abstractmethod decorator"
    );
    assert!(
        on_event.is_abstract,
        "on_event should be marked as abstract"
    );
}
