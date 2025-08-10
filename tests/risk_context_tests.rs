use anyhow::Result;
use debtmap::risk::context::{
    AnalysisTarget, Context, ContextAggregator, ContextDetails, ContextMap, ContextProvider,
    ContextualRisk, Impact, Priority,
};
use std::path::PathBuf;

struct MockProvider {
    name: String,
    weight: f64,
    should_fail: bool,
}

impl ContextProvider for MockProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn gather(&self, _target: &AnalysisTarget) -> Result<Context> {
        if self.should_fail {
            anyhow::bail!("Mock provider failure")
        }

        Ok(Context {
            provider: self.name.clone(),
            weight: self.weight,
            contribution: 0.5,
            details: ContextDetails::Business {
                priority: Priority::Medium,
                impact: Impact::Performance,
                annotations: vec!["test".to_string()],
            },
        })
    }

    fn weight(&self) -> f64 {
        self.weight
    }

    fn explain(&self, _context: &Context) -> String {
        format!("{} explanation", self.name)
    }
}

#[test]
fn test_context_aggregator_default() {
    let aggregator = ContextAggregator::default();
    let target = AnalysisTarget {
        root_path: PathBuf::from("/test"),
        file_path: PathBuf::from("/test/file.rs"),
        function_name: "test_fn".to_string(),
        line_range: (1, 10),
    };

    let mut aggregator = aggregator;
    let context_map = aggregator.analyze(&target);

    // Default aggregator has no providers, so context map should be empty
    assert_eq!(context_map.total_contribution(), 0.0);
}

#[test]
fn test_context_aggregator_new() {
    let aggregator = ContextAggregator::new();
    let target = AnalysisTarget {
        root_path: PathBuf::from("/test"),
        file_path: PathBuf::from("/test/file.rs"),
        function_name: "test_fn".to_string(),
        line_range: (1, 10),
    };

    let mut aggregator = aggregator;
    let context_map = aggregator.analyze(&target);

    // New aggregator has no providers, so context map should be empty
    assert_eq!(context_map.total_contribution(), 0.0);
}

#[test]
fn test_context_aggregator_with_provider() {
    let provider = Box::new(MockProvider {
        name: "test_provider".to_string(),
        weight: 1.0,
        should_fail: false,
    });

    let mut aggregator = ContextAggregator::new().with_provider(provider);

    let target = AnalysisTarget {
        root_path: PathBuf::from("/test"),
        file_path: PathBuf::from("/test/file.rs"),
        function_name: "test_fn".to_string(),
        line_range: (1, 10),
    };

    let context_map = aggregator.analyze(&target);

    // Should have one context from the provider
    assert!(context_map.get("test_provider").is_some());
    assert_eq!(context_map.total_contribution(), 0.5); // contribution * weight = 0.5 * 1.0
}

#[test]
fn test_context_aggregator_with_failing_provider() {
    let provider = Box::new(MockProvider {
        name: "failing_provider".to_string(),
        weight: 1.0,
        should_fail: true,
    });

    let mut aggregator = ContextAggregator::new().with_provider(provider);

    let target = AnalysisTarget {
        root_path: PathBuf::from("/test"),
        file_path: PathBuf::from("/test/file.rs"),
        function_name: "test_fn".to_string(),
        line_range: (1, 10),
    };

    let context_map = aggregator.analyze(&target);

    // Failing provider should not add context
    assert!(context_map.get("failing_provider").is_none());
    assert_eq!(context_map.total_contribution(), 0.0);
}

#[test]
fn test_context_aggregator_cache() {
    let provider = Box::new(MockProvider {
        name: "cached_provider".to_string(),
        weight: 1.0,
        should_fail: false,
    });

    let mut aggregator = ContextAggregator::new().with_provider(provider);

    let target = AnalysisTarget {
        root_path: PathBuf::from("/test"),
        file_path: PathBuf::from("/test/file.rs"),
        function_name: "test_fn".to_string(),
        line_range: (1, 10),
    };

    // First call
    let context_map1 = aggregator.analyze(&target);
    // Second call should use cache
    let context_map2 = aggregator.analyze(&target);

    // Both should be identical
    assert_eq!(
        context_map1.total_contribution(),
        context_map2.total_contribution()
    );
}

#[test]
fn test_context_aggregator_clear_cache() {
    let provider = Box::new(MockProvider {
        name: "cache_clear_provider".to_string(),
        weight: 1.0,
        should_fail: false,
    });

    let mut aggregator = ContextAggregator::new().with_provider(provider);

    let target = AnalysisTarget {
        root_path: PathBuf::from("/test"),
        file_path: PathBuf::from("/test/file.rs"),
        function_name: "test_fn".to_string(),
        line_range: (1, 10),
    };

    let _ = aggregator.analyze(&target);
    aggregator.clear_cache();
    // After clearing cache, should recompute
    let context_map = aggregator.analyze(&target);

    assert!(context_map.get("cache_clear_provider").is_some());
}

#[test]
fn test_context_map_default() {
    let context_map = ContextMap::default();
    assert_eq!(context_map.total_contribution(), 0.0);
    assert!(context_map.get("nonexistent").is_none());
}

#[test]
fn test_context_map_new() {
    let context_map = ContextMap::new();
    assert_eq!(context_map.total_contribution(), 0.0);
    assert!(context_map.get("nonexistent").is_none());
}

#[test]
fn test_context_map_add_and_get() {
    let mut context_map = ContextMap::new();

    let context = Context {
        provider: "test".to_string(),
        weight: 2.0,
        contribution: 0.5,
        details: ContextDetails::Historical {
            change_frequency: 0.3,
            bug_density: 0.1,
            age_days: 100,
            author_count: 3,
        },
    };

    context_map.add("test".to_string(), context.clone());

    assert!(context_map.get("test").is_some());
    assert_eq!(context_map.get("test").unwrap().provider, "test");
    assert_eq!(context_map.total_contribution(), 1.0); // 0.5 * 2.0
}

#[test]
fn test_context_map_total_contribution() {
    let mut context_map = ContextMap::new();

    context_map.add(
        "provider1".to_string(),
        Context {
            provider: "provider1".to_string(),
            weight: 1.0,
            contribution: 0.5,
            details: ContextDetails::CriticalPath {
                entry_points: vec!["main".to_string()],
                path_weight: 0.8,
                is_user_facing: true,
            },
        },
    );

    context_map.add(
        "provider2".to_string(),
        Context {
            provider: "provider2".to_string(),
            weight: 2.0,
            contribution: 0.3,
            details: ContextDetails::DependencyChain {
                depth: 3,
                propagated_risk: 0.7,
                dependents: vec!["mod1".to_string()],
                blast_radius: 5,
            },
        },
    );

    // Total = (0.5 * 1.0) + (0.3 * 2.0) = 0.5 + 0.6 = 1.1
    assert_eq!(context_map.total_contribution(), 1.1);
}

#[test]
fn test_context_map_iter() {
    let mut context_map = ContextMap::new();

    context_map.add(
        "provider1".to_string(),
        Context {
            provider: "provider1".to_string(),
            weight: 1.0,
            contribution: 0.5,
            details: ContextDetails::Business {
                priority: Priority::High,
                impact: Impact::Revenue,
                annotations: vec!["critical".to_string()],
            },
        },
    );

    let items: Vec<_> = context_map.iter().collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].0, "provider1");
}

#[test]
fn test_contextual_risk_new() {
    let mut context_map = ContextMap::new();

    context_map.add(
        "test".to_string(),
        Context {
            provider: "test".to_string(),
            weight: 1.0,
            contribution: 0.2,
            details: ContextDetails::Business {
                priority: Priority::Low,
                impact: Impact::UserExperience,
                annotations: vec![],
            },
        },
    );

    let risk = ContextualRisk::new(5.0, &context_map);

    assert_eq!(risk.base_risk, 5.0);
    // contextual_risk = 5.0 * (1.0 + 0.2) = 6.0
    assert_eq!(risk.contextual_risk, 6.0);
    assert_eq!(risk.contexts.len(), 1);
    assert!(risk.explanation.contains("Base risk: 5.0"));
    assert!(risk.explanation.contains("test: +0.2"));
}

#[test]
fn test_contextual_risk_capped_at_10() {
    let mut context_map = ContextMap::new();

    context_map.add(
        "high_impact".to_string(),
        Context {
            provider: "high_impact".to_string(),
            weight: 2.0,
            contribution: 2.0,
            details: ContextDetails::Business {
                priority: Priority::Critical,
                impact: Impact::Security,
                annotations: vec!["security-critical".to_string()],
            },
        },
    );

    let risk = ContextualRisk::new(8.0, &context_map);

    assert_eq!(risk.base_risk, 8.0);
    // Would be 8.0 * (1.0 + 4.0) = 40.0, but capped at 10.0
    assert_eq!(risk.contextual_risk, 10.0);
}

#[test]
fn test_priority_equality() {
    assert_eq!(Priority::Critical, Priority::Critical);
    assert_ne!(Priority::Critical, Priority::High);
    assert_eq!(Priority::Medium, Priority::Medium);
    assert_ne!(Priority::Low, Priority::High);
}

#[test]
fn test_impact_equality() {
    assert_eq!(Impact::Revenue, Impact::Revenue);
    assert_ne!(Impact::Revenue, Impact::Security);
    assert_eq!(Impact::Compliance, Impact::Compliance);
    assert_ne!(Impact::Performance, Impact::UserExperience);
}
