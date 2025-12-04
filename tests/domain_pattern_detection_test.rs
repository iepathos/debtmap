//! Integration tests for domain pattern detection (Spec 175)
#![allow(deprecated)]

use debtmap::organization::{
    cluster_methods_by_domain, group_methods_by_responsibility_with_domain_patterns, DomainPattern,
    DomainPatternDetector,
};
use debtmap::{
    analysis::io_detection::Language,
    organization::domain_patterns::{FileContext, MethodInfo},
};
use std::collections::HashSet;

#[test]
fn test_observer_pattern_cluster_detection() {
    let detector = DomainPatternDetector::new();

    // Simulate methods from python_type_tracker with observer pattern
    let methods = vec![
        MethodInfo {
            name: "register_observer_interfaces".to_string(),
            body: "self.observer_registry.register(interface)".to_string(),
            doc_comment: Some("Register observer interface".to_string()),
        },
        MethodInfo {
            name: "detect_observer_dispatch".to_string(),
            body: "self.observer_registry.dispatch_event()".to_string(),
            doc_comment: None,
        },
        MethodInfo {
            name: "populate_observer_registry".to_string(),
            body: "self.observer_registry.populate()".to_string(),
            doc_comment: None,
        },
        MethodInfo {
            name: "notify_all_observers".to_string(),
            body: "self.observer_registry.notify_all()".to_string(),
            doc_comment: None,
        },
    ];

    let context = FileContext {
        methods: methods.clone(),
        structures: ["ObserverRegistry".to_string()].into_iter().collect(),
        call_edges: vec![],
    };

    let clusters = cluster_methods_by_domain(&methods, &context, &detector);

    // Should detect observer pattern cluster
    assert!(
        clusters.contains_key(&DomainPattern::ObserverPattern),
        "Observer pattern cluster should be detected. Found clusters: {:?}",
        clusters.keys().collect::<Vec<_>>()
    );

    let observer_cluster = &clusters[&DomainPattern::ObserverPattern];
    assert!(
        observer_cluster.len() >= 3,
        "Observer cluster should have at least 3 methods, got {}",
        observer_cluster.len()
    );
}

#[test]
fn test_callback_pattern_detection() {
    let detector = DomainPatternDetector::new();

    let methods = vec![
        MethodInfo {
            name: "check_for_callback_patterns".to_string(),
            body: "self.callback_tracker.check()".to_string(),
            doc_comment: None,
        },
        MethodInfo {
            name: "extract_callback_expr".to_string(),
            body: "self.callback_tracker.extract()".to_string(),
            doc_comment: None,
        },
        MethodInfo {
            name: "check_for_event_bindings".to_string(),
            body: "self.callback_tracker.check_bindings()".to_string(),
            doc_comment: None,
        },
    ];

    let context = FileContext {
        methods: methods.clone(),
        structures: ["CallbackTracker".to_string()].into_iter().collect(),
        call_edges: vec![],
    };

    let clusters = cluster_methods_by_domain(&methods, &context, &detector);

    // Should detect callback pattern cluster
    assert!(
        clusters.contains_key(&DomainPattern::CallbackPattern),
        "Callback pattern cluster should be detected"
    );

    let callback_cluster = &clusters[&DomainPattern::CallbackPattern];
    assert_eq!(callback_cluster.len(), 3);
}

#[test]
fn test_domain_pattern_integration_with_responsibility_classification() {
    // Test that domain pattern detection integrates correctly with responsibility classification

    let methods = vec![
        // Observer pattern methods
        (
            "register_observer".to_string(),
            Some("self.observer_registry.register(obs)".to_string()),
        ),
        (
            "notify_observers".to_string(),
            Some("self.observer_registry.notify_all()".to_string()),
        ),
        (
            "unregister_observer".to_string(),
            Some("self.observer_registry.remove(obs)".to_string()),
        ),
        // Non-pattern method
        (
            "format_output".to_string(),
            Some("format!(\"{}\", data)".to_string()),
        ),
        (
            "parse_input".to_string(),
            Some("parse(input_str)".to_string()),
        ),
    ];

    let structures = vec!["ObserverRegistry".to_string()];

    let (groups, _evidence) =
        group_methods_by_responsibility_with_domain_patterns(&methods, Language::Rust, &structures);

    // Should have observer pattern group
    assert!(
        groups.contains_key("Observer Pattern Detection"),
        "Should have Observer Pattern Detection group. Groups: {:?}",
        groups.keys().collect::<Vec<_>>()
    );

    let observer_group = &groups["Observer Pattern Detection"];
    assert_eq!(
        observer_group.len(),
        3,
        "Observer pattern group should have 3 methods"
    );

    // Non-pattern methods should be classified separately
    assert!(
        groups.contains_key("Formatting") || groups.contains_key("Formatting/Serialization"),
        "Should have formatting group"
    );
    assert!(
        groups.contains_key("Parsing") || groups.contains_key("Parsing/Deserialization"),
        "Should have parsing group"
    );
}

#[test]
fn test_minimum_cluster_size_threshold() {
    let detector = DomainPatternDetector::new();

    // Only 2 observer methods (below MIN_DOMAIN_CLUSTER_SIZE of 3)
    let methods = vec![
        MethodInfo {
            name: "register_observer".to_string(),
            body: "self.observer_registry.register(obs)".to_string(),
            doc_comment: None,
        },
        MethodInfo {
            name: "notify_observers".to_string(),
            body: "self.observer_registry.notify_all()".to_string(),
            doc_comment: None,
        },
    ];

    let context = FileContext {
        methods: methods.clone(),
        structures: ["ObserverRegistry".to_string()].into_iter().collect(),
        call_edges: vec![],
    };

    let clusters = cluster_methods_by_domain(&methods, &context, &detector);

    // Should NOT create cluster with only 2 methods
    assert!(
        clusters.is_empty(),
        "Should not create cluster with only 2 methods (below threshold of 3)"
    );
}

#[test]
fn test_mixed_patterns_separation() {
    let detector = DomainPatternDetector::new();

    let methods = vec![
        // Observer pattern methods
        MethodInfo {
            name: "register_observer".to_string(),
            body: "self.observer_registry.add(obs)".to_string(),
            doc_comment: None,
        },
        MethodInfo {
            name: "notify_observers".to_string(),
            body: "self.observer_registry.notify()".to_string(),
            doc_comment: None,
        },
        MethodInfo {
            name: "unregister_observer".to_string(),
            body: "self.observer_registry.remove(obs)".to_string(),
            doc_comment: None,
        },
        // Builder pattern methods
        MethodInfo {
            name: "with_config".to_string(),
            body: "self.builder.with_config(config)".to_string(),
            doc_comment: None,
        },
        MethodInfo {
            name: "with_options".to_string(),
            body: "self.builder.with_options(opts)".to_string(),
            doc_comment: None,
        },
        MethodInfo {
            name: "build".to_string(),
            body: "self.builder.build()".to_string(),
            doc_comment: None,
        },
    ];

    let context = FileContext {
        methods: methods.clone(),
        structures: ["ObserverRegistry".to_string(), "Builder".to_string()]
            .into_iter()
            .collect(),
        call_edges: vec![],
    };

    let clusters = cluster_methods_by_domain(&methods, &context, &detector);

    // Should detect both patterns as separate clusters
    assert!(
        clusters.contains_key(&DomainPattern::ObserverPattern),
        "Should detect observer pattern"
    );
    assert!(
        clusters.contains_key(&DomainPattern::BuilderPattern),
        "Should detect builder pattern"
    );

    assert_eq!(clusters[&DomainPattern::ObserverPattern].len(), 3);
    assert_eq!(clusters[&DomainPattern::BuilderPattern].len(), 3);
}

#[test]
fn test_pattern_confidence_threshold() {
    let detector = DomainPatternDetector::new();

    // Method with weak pattern signals (only name matches, no structure access)
    let method = MethodInfo {
        name: "observer_helper".to_string(),
        body: "println!(\"helper\")".to_string(), // No structure access
        doc_comment: None,
    };

    let context = FileContext {
        methods: vec![method.clone()],
        structures: HashSet::new(), // No relevant structures
        call_edges: vec![],
    };

    let result = detector.detect_method_domain(&method, &context);

    // Should not match due to weak signals (only name keyword, no structure access)
    // With WEIGHT_NAME_KEYWORDS = 0.30 and threshold = 0.60, name alone isn't enough
    if let Some(matched) = result {
        assert!(
            matched.confidence < 0.60,
            "Weak signals should not meet confidence threshold"
        );
    }
}

#[test]
fn test_all_patterns_have_valid_definitions() {
    let patterns = DomainPattern::all_patterns();

    assert_eq!(patterns.len(), 6, "Should have 6 domain patterns defined");

    for pattern in patterns {
        // Each pattern should have keywords
        assert!(
            !pattern.keywords().is_empty(),
            "Pattern {:?} should have keywords",
            pattern
        );

        // Each pattern should have a module name
        let module_name = pattern.module_name();
        assert!(
            !module_name.is_empty(),
            "Pattern {:?} should have module name",
            pattern
        );

        // Each pattern should have a description
        let description = pattern.description();
        assert!(
            !description.is_empty(),
            "Pattern {:?} should have description",
            pattern
        );
    }
}
