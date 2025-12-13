// Specific debt type recommendation generators
// This module contains recommendation generators for known debt patterns
// like code smells, resource issues, test problems, and performance issues.

/// Generate recommendation for resource management debt
pub fn generate_resource_management_recommendation(
    resource_type: &str,
    detail1: &str,
    detail2: &str,
) -> (String, String, Vec<String>) {
    match resource_type {
        "allocation" => (
            format!("Optimize allocation pattern: {}", detail1),
            format!("Resource impact: {}", detail2),
            vec![
                "Use object pooling where appropriate".to_string(),
                "Consider pre-allocation strategies".to_string(),
                "Profile memory usage patterns".to_string(),
                "Review data structure choices".to_string(),
            ],
        ),
        "blocking_io" => (
            format!("Optimize {} operation", detail1),
            format!("Context: {}", detail2),
            vec![
                "Consider async/await pattern".to_string(),
                "Use appropriate I/O libraries".to_string(),
                "Consider background processing".to_string(),
                "Add proper error handling".to_string(),
            ],
        ),
        "basic" => (
            format!("Optimize {} resource issue", detail1),
            format!("Resource impact ({}): {}", detail2, detail1),
            vec![
                "Profile and identify resource bottlenecks".to_string(),
                "Apply resource optimization techniques".to_string(),
                "Monitor resource usage before and after changes".to_string(),
                "Consider algorithmic improvements".to_string(),
            ],
        ),
        _ => (
            "Optimize resource usage".to_string(),
            "Resource issue detected".to_string(),
            vec!["Monitor and profile resource usage".to_string()],
        ),
    }
}

/// Generate recommendation for string concatenation in loops
pub fn generate_string_concat_recommendation(
    loop_type: &str,
    iterations: &Option<u32>,
) -> (String, String, Vec<String>) {
    let iter_info = iterations.map_or("unknown".to_string(), |i| i.to_string());
    (
        format!("Use StringBuilder for {} loop concatenation", loop_type),
        format!(
            "String concatenation in {} (≈{} iterations)",
            loop_type, iter_info
        ),
        vec![
            "Replace += with StringBuilder/StringBuffer".to_string(),
            "Pre-allocate capacity if known".to_string(),
            "Consider string formatting alternatives".to_string(),
            "Benchmark performance improvement".to_string(),
        ],
    )
}

/// Generate recommendation for nested loops
pub fn generate_nested_loops_recommendation(
    depth: u32,
    complexity_estimate: &str,
) -> (String, String, Vec<String>) {
    (
        format!("Reduce {}-level nested loop complexity", depth),
        format!("Complexity estimate: {}", complexity_estimate),
        vec![
            "Extract inner loops into functions".to_string(),
            "Consider algorithmic improvements".to_string(),
            "Use iterators for cleaner code".to_string(),
            "Profile actual performance impact".to_string(),
        ],
    )
}

/// Generate recommendation for data structure improvements
pub fn generate_data_structure_recommendation(
    current: &str,
    recommended: &str,
) -> (String, String, Vec<String>) {
    (
        format!("Replace {} with {}", current, recommended),
        format!(
            "Data structure {} is suboptimal for access patterns",
            current
        ),
        vec![
            format!("Refactor to use {}", recommended),
            "Update related algorithms".to_string(),
            "Test performance before/after".to_string(),
            "Update documentation".to_string(),
        ],
    )
}

/// Generate recommendation for god object pattern
pub fn generate_god_object_recommendation(
    responsibility_count: u32,
    complexity_score: f64,
) -> (String, String, Vec<String>) {
    (
        format!(
            "Split {} responsibilities into focused classes",
            responsibility_count
        ),
        format!("God object with complexity {:.1}", complexity_score),
        vec![
            "Identify single responsibility principle violations".to_string(),
            "Extract cohesive functionality into separate classes".to_string(),
            "Use composition over inheritance".to_string(),
            "Refactor incrementally with tests".to_string(),
        ],
    )
}

/// Generate recommendation for feature envy pattern
pub fn generate_feature_envy_recommendation(
    external_class: &str,
    usage_ratio: f64,
) -> (String, String, Vec<String>) {
    (
        format!("Move method closer to {} class", external_class),
        format!(
            "Method uses {}% external data",
            (usage_ratio * 100.0) as u32
        ),
        vec![
            format!("Consider moving method to {}", external_class),
            "Extract shared functionality".to_string(),
            "Review class responsibilities".to_string(),
            "Maintain cohesion after refactoring".to_string(),
        ],
    )
}

/// Generate recommendation for primitive obsession pattern
pub fn generate_primitive_obsession_recommendation(
    primitive_type: &str,
    domain_concept: &str,
) -> (String, String, Vec<String>) {
    (
        format!(
            "Create {} domain type instead of {}",
            domain_concept, primitive_type
        ),
        format!(
            "Primitive obsession: {} used for {}",
            primitive_type, domain_concept
        ),
        vec![
            format!("Create {} value object", domain_concept),
            "Add validation and behavior to type".to_string(),
            "Replace primitive usage throughout codebase".to_string(),
            "Add type safety and domain logic".to_string(),
        ],
    )
}

/// Generate recommendation for magic values
pub fn generate_magic_values_recommendation(
    value: &str,
    occurrences: u32,
) -> (String, String, Vec<String>) {
    (
        format!("Extract '{}' into named constant", value),
        format!("Magic value '{}' appears {} times", value, occurrences),
        vec![
            format!(
                "Define const {} = '{}'",
                value.to_uppercase().replace(' ', "_"),
                value
            ),
            "Replace all occurrences with named constant".to_string(),
            "Add documentation explaining value meaning".to_string(),
            "Group related constants in module".to_string(),
        ],
    )
}

/// Generate recommendation for complex assertions in tests
pub fn generate_assertion_complexity_recommendation(
    assertion_count: u32,
    complexity_score: f64,
) -> (String, String, Vec<String>) {
    (
        format!("Simplify {} complex assertions", assertion_count),
        format!("Test assertion complexity: {:.1}", complexity_score),
        vec![
            "Split complex assertions into multiple simple ones".to_string(),
            "Use custom assertion helpers".to_string(),
            "Add descriptive assertion messages".to_string(),
            "Consider table-driven test patterns".to_string(),
        ],
    )
}

/// Generate recommendation for flaky test patterns
pub fn generate_flaky_test_recommendation(
    pattern_type: &str,
    reliability_impact: &str,
) -> (String, String, Vec<String>) {
    (
        format!("Fix {} flaky test pattern", pattern_type),
        format!("Reliability impact: {}", reliability_impact),
        vec![
            "Identify and eliminate non-deterministic behavior".to_string(),
            "Use test doubles to isolate dependencies".to_string(),
            "Add proper test cleanup and setup".to_string(),
            "Consider parallel test safety".to_string(),
        ],
    )
}

/// Generate recommendation for async/await misuse
pub fn generate_async_misuse_recommendation(
    pattern: &str,
    performance_impact: &str,
) -> (String, String, Vec<String>) {
    (
        format!("Fix async pattern: {}", pattern),
        format!("Resource impact: {}", performance_impact),
        vec![
            "Use proper async/await patterns".to_string(),
            "Avoid blocking async contexts".to_string(),
            "Configure async runtime appropriately".to_string(),
            "Add timeout and cancellation handling".to_string(),
        ],
    )
}

/// Generate recommendation for resource leaks
pub fn generate_resource_leak_recommendation(
    resource_type: &str,
    cleanup_missing: &str,
) -> (String, String, Vec<String>) {
    (
        format!("Add {} resource cleanup", resource_type),
        format!("Missing cleanup: {}", cleanup_missing),
        vec![
            "Implement Drop trait for automatic cleanup".to_string(),
            "Use RAII patterns for resource management".to_string(),
            "Add try-finally or defer patterns".to_string(),
            "Test resource cleanup in error scenarios".to_string(),
        ],
    )
}

/// Generate recommendation for collection inefficiencies
pub fn generate_collection_inefficiency_recommendation(
    collection_type: &str,
    inefficiency_type: &str,
) -> (String, String, Vec<String>) {
    (
        format!("Optimize {} usage", collection_type),
        format!("Inefficiency: {}", inefficiency_type),
        vec![
            "Review collection access patterns".to_string(),
            "Consider alternative data structures".to_string(),
            "Pre-allocate capacity where possible".to_string(),
            "Monitor collection resource usage".to_string(),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_resource_management_allocation() {
        let (action, rationale, steps) = generate_resource_management_recommendation(
            "allocation",
            "frequent allocations",
            "high memory churn",
        );

        assert!(action.contains("Optimize allocation pattern"));
        assert!(rationale.contains("Resource impact"));
        assert_eq!(steps.len(), 4);
        assert!(steps.iter().any(|s| s.contains("object pooling")));
    }

    #[test]
    fn test_generate_resource_management_blocking_io() {
        let (action, rationale, steps) =
            generate_resource_management_recommendation("blocking_io", "file read", "main thread");

        assert!(action.contains("Optimize"));
        assert!(rationale.contains("Context"));
        assert_eq!(steps.len(), 4);
        assert!(steps.iter().any(|s| s.contains("async/await")));
    }

    #[test]
    fn test_generate_resource_management_basic() {
        let (action, rationale, steps) =
            generate_resource_management_recommendation("basic", "memory", "moderate");

        assert!(action.contains("Optimize"));
        assert!(rationale.contains("Resource impact"));
        assert_eq!(steps.len(), 4);
    }

    #[test]
    fn test_generate_resource_management_unknown() {
        let (action, rationale, steps) =
            generate_resource_management_recommendation("unknown_type", "", "");

        assert_eq!(action, "Optimize resource usage");
        assert_eq!(rationale, "Resource issue detected");
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn test_generate_string_concat_recommendation() {
        let (action, rationale, steps) = generate_string_concat_recommendation("for", &Some(100));

        assert!(action.contains("StringBuilder"));
        assert!(rationale.contains("100 iterations"));
        assert_eq!(steps.len(), 4);
    }

    #[test]
    fn test_generate_string_concat_unknown_iterations() {
        let (action, rationale, steps) = generate_string_concat_recommendation("while", &None);

        assert!(action.contains("StringBuilder"));
        assert!(rationale.contains("unknown"));
        assert_eq!(steps.len(), 4);
    }

    #[test]
    fn test_generate_nested_loops_recommendation() {
        let (action, rationale, steps) = generate_nested_loops_recommendation(3, "O(n^3)");

        assert!(action.contains("3-level"));
        assert!(rationale.contains("O(n^3)"));
        assert_eq!(steps.len(), 4);
        assert!(steps.iter().any(|s| s.contains("Extract inner loops")));
    }

    #[test]
    fn test_generate_data_structure_recommendation() {
        let (action, rationale, steps) = generate_data_structure_recommendation("Vec", "HashMap");

        assert!(action.contains("Replace Vec with HashMap"));
        assert!(rationale.contains("suboptimal"));
        assert_eq!(steps.len(), 4);
        assert!(steps.iter().any(|s| s.contains("HashMap")));
    }

    #[test]
    fn test_generate_god_object_recommendation() {
        let (action, rationale, steps) = generate_god_object_recommendation(5, 150.0);

        assert!(action.contains("Split 5 responsibilities"));
        assert!(rationale.contains("complexity 150.0"));
        assert_eq!(steps.len(), 4);
        assert!(steps.iter().any(|s| s.contains("single responsibility")));
    }

    #[test]
    fn test_generate_feature_envy_recommendation() {
        let (action, rationale, steps) = generate_feature_envy_recommendation("UserService", 0.75);

        assert!(action.contains("UserService"));
        assert!(rationale.contains("75%"));
        assert_eq!(steps.len(), 4);
    }

    #[test]
    fn test_generate_primitive_obsession_recommendation() {
        let (action, rationale, steps) =
            generate_primitive_obsession_recommendation("String", "EmailAddress");

        assert!(action.contains("EmailAddress"));
        assert!(action.contains("String"));
        assert!(rationale.contains("Primitive obsession"));
        assert_eq!(steps.len(), 4);
    }

    #[test]
    fn test_generate_magic_values_recommendation() {
        let (action, rationale, steps) = generate_magic_values_recommendation("admin", 5);

        assert!(action.contains("admin"));
        assert!(rationale.contains("5 times"));
        assert_eq!(steps.len(), 4);
        assert!(steps.iter().any(|s| s.contains("ADMIN")));
    }

    #[test]
    fn test_generate_assertion_complexity_recommendation() {
        let (action, rationale, steps) = generate_assertion_complexity_recommendation(10, 25.5);

        assert!(action.contains("10 complex assertions"));
        assert!(rationale.contains("25.5"));
        assert_eq!(steps.len(), 4);
    }

    #[test]
    fn test_generate_flaky_test_recommendation() {
        let (action, rationale, steps) =
            generate_flaky_test_recommendation("timing-dependent", "high");

        assert!(action.contains("timing-dependent"));
        assert!(rationale.contains("high"));
        assert_eq!(steps.len(), 4);
    }

    #[test]
    fn test_generate_async_misuse_recommendation() {
        let (action, rationale, steps) =
            generate_async_misuse_recommendation("blocking in async", "severe");

        assert!(action.contains("blocking in async"));
        assert!(rationale.contains("severe"));
        assert_eq!(steps.len(), 4);
    }

    #[test]
    fn test_generate_resource_leak_recommendation() {
        let (action, rationale, steps) =
            generate_resource_leak_recommendation("file handle", "close() not called");

        assert!(action.contains("file handle"));
        assert!(rationale.contains("close()"));
        assert_eq!(steps.len(), 4);
        assert!(steps.iter().any(|s| s.contains("Drop trait")));
    }

    #[test]
    fn test_generate_collection_inefficiency_recommendation() {
        let (action, rationale, steps) =
            generate_collection_inefficiency_recommendation("Vec", "linear search");

        assert!(action.contains("Vec"));
        assert!(rationale.contains("linear search"));
        assert_eq!(steps.len(), 4);
    }
}
