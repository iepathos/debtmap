//! Debt item deduplication logic (spec 231)
//!
//! Provides deduplication of debt items by (file, line, function) key to prevent
//! duplicate entries in the output.

use super::types::UnifiedDebtItemOutput;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

/// Unique key for debt item deduplication (spec 231)
///
/// Items are considered duplicates if they have the same file path, line number,
/// and function name (for function items) or just file path (for file items).
#[derive(Debug, Clone)]
pub struct DebtItemKey {
    pub file: String,
    pub line: Option<usize>,
    pub function: Option<String>,
}

impl PartialEq for DebtItemKey {
    fn eq(&self, other: &Self) -> bool {
        self.file == other.file && self.line == other.line && self.function == other.function
    }
}

impl Eq for DebtItemKey {}

impl Hash for DebtItemKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.file.hash(state);
        self.line.hash(state);
        self.function.hash(state);
    }
}

impl From<&UnifiedDebtItemOutput> for DebtItemKey {
    fn from(item: &UnifiedDebtItemOutput) -> Self {
        match item {
            UnifiedDebtItemOutput::File(f) => DebtItemKey {
                file: f.location.file.clone(),
                line: None,
                function: None,
            },
            UnifiedDebtItemOutput::Function(f) => DebtItemKey {
                file: f.location.file.clone(),
                line: f.location.line,
                function: f.location.function.clone(),
            },
        }
    }
}

/// Deduplicate debt items by (file, line, function) key (spec 231)
///
/// Removes duplicate items that have the same location. Keeps the first occurrence
/// of each unique item. Logs when duplicates are removed for debugging.
pub fn deduplicate_items(items: Vec<UnifiedDebtItemOutput>) -> Vec<UnifiedDebtItemOutput> {
    let mut seen: HashSet<DebtItemKey> = HashSet::new();
    let mut result = Vec::with_capacity(items.len());
    let mut duplicate_count = 0;

    for item in items {
        let key = DebtItemKey::from(&item);

        if seen.insert(key.clone()) {
            result.push(item);
        } else {
            duplicate_count += 1;
            // Log duplicate removal for debugging (only in debug builds or when RUST_LOG is set)
            #[cfg(debug_assertions)]
            eprintln!(
                "Warning: Removed duplicate debt item: file={}, line={:?}, function={:?}",
                key.file, key.line, key.function
            );
        }
    }

    if duplicate_count > 0 {
        // Always log summary when duplicates are found
        eprintln!(
            "Warning: Removed {} duplicate debt items from output",
            duplicate_count
        );
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::unified::dependencies::{Dependencies, RecommendationOutput};
    use crate::output::unified::file_item::{
        FileDebtItemOutput, FileImpactOutput, FileMetricsOutput,
    };
    use crate::output::unified::func_item::{
        FunctionDebtItemOutput, FunctionImpactOutput, FunctionMetricsOutput,
    };
    use crate::output::unified::location::UnifiedLocation;
    use crate::output::unified::priority::Priority;
    use crate::priority::{DebtType, FunctionRole};

    /// Helper to create a function debt item for testing
    fn create_test_function_item(
        file: &str,
        line: usize,
        function: &str,
        score: f64,
    ) -> UnifiedDebtItemOutput {
        UnifiedDebtItemOutput::Function(Box::new(FunctionDebtItemOutput {
            score,
            category: "TestCategory".to_string(),
            priority: Priority::from_score(score),
            location: UnifiedLocation {
                file: file.to_string(),
                line: Some(line),
                function: Some(function.to_string()),
                file_context_label: None,
            },
            metrics: FunctionMetricsOutput {
                cyclomatic_complexity: 10,
                cognitive_complexity: 15,
                length: 50,
                nesting_depth: 3,
                coverage: Some(0.8),
                uncovered_lines: None,
                entropy_score: None,
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 10,
                cognitive: 15,
            },
            function_role: FunctionRole::PureLogic,
            purity_analysis: None,
            dependencies: Dependencies {
                upstream_count: 0,
                downstream_count: 0,
                upstream_callers: vec![],
                downstream_callees: vec![],
            },
            recommendation: RecommendationOutput {
                action: "Test action".to_string(),
                priority: None,
                implementation_steps: vec![],
            },
            impact: FunctionImpactOutput {
                coverage_improvement: 0.0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            scoring_details: None,
            adjusted_complexity: None,
            complexity_pattern: None,
            pattern_type: None,
            pattern_confidence: None,
            pattern_details: None,
        }))
    }

    /// Helper to create a file debt item for testing
    fn create_test_file_item(file: &str, score: f64) -> UnifiedDebtItemOutput {
        UnifiedDebtItemOutput::File(Box::new(FileDebtItemOutput {
            score,
            category: "Architecture".to_string(),
            priority: Priority::from_score(score),
            location: UnifiedLocation {
                file: file.to_string(),
                line: None,
                function: None,
                file_context_label: None,
            },
            metrics: FileMetricsOutput {
                lines: 500,
                functions: 20,
                classes: 1,
                avg_complexity: 8.0,
                max_complexity: 15,
                total_complexity: 160,
                coverage: 0.7,
                uncovered_lines: 150,
            },
            god_object_indicators: None,
            dependencies: None,
            anti_patterns: None,
            cohesion: None,
            recommendation: RecommendationOutput {
                action: "Refactor file".to_string(),
                priority: None,
                implementation_steps: vec![],
            },
            impact: FileImpactOutput {
                complexity_reduction: 10.0,
                maintainability_improvement: 0.2,
                test_effort: 5.0,
            },
            scoring_details: None,
        }))
    }

    #[test]
    fn test_deduplication_removes_duplicate_functions() {
        let items = vec![
            create_test_function_item("a.rs", 10, "foo", 50.0),
            create_test_function_item("a.rs", 10, "foo", 45.0), // Duplicate
            create_test_function_item("b.rs", 20, "bar", 30.0),
        ];

        let result = deduplicate_items(items);

        assert_eq!(result.len(), 2);
        // Should keep first occurrence (score 50.0)
        assert_eq!(result[0].score(), 50.0);
        assert_eq!(result[1].score(), 30.0);
    }

    #[test]
    fn test_deduplication_preserves_unique_items() {
        let items = vec![
            create_test_function_item("a.rs", 10, "foo", 50.0),
            create_test_function_item("a.rs", 20, "bar", 45.0), // Different line
            create_test_function_item("b.rs", 10, "foo", 30.0), // Different file
        ];

        let result = deduplicate_items(items);

        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_deduplication_handles_file_items() {
        let items = vec![
            create_test_file_item("a.rs", 50.0),
            create_test_file_item("a.rs", 45.0), // Duplicate
            create_test_file_item("b.rs", 30.0),
        ];

        let result = deduplicate_items(items);

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_deduplication_mixed_item_types() {
        let items = vec![
            create_test_function_item("a.rs", 10, "foo", 50.0),
            create_test_file_item("a.rs", 45.0), // Different type, should not be duplicate
            create_test_function_item("a.rs", 10, "foo", 30.0), // Duplicate function
        ];

        let result = deduplicate_items(items);

        // Function and file items have different keys (file item has no line/function)
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_deduplication_empty_input() {
        let items: Vec<UnifiedDebtItemOutput> = vec![];
        let result = deduplicate_items(items);
        assert!(result.is_empty());
    }

    #[test]
    fn test_deduplication_single_item() {
        let items = vec![create_test_function_item("a.rs", 10, "foo", 50.0)];
        let result = deduplicate_items(items);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_debt_item_key_equality() {
        let key1 = DebtItemKey {
            file: "a.rs".to_string(),
            line: Some(10),
            function: Some("foo".to_string()),
        };
        let key2 = DebtItemKey {
            file: "a.rs".to_string(),
            line: Some(10),
            function: Some("foo".to_string()),
        };
        let key3 = DebtItemKey {
            file: "a.rs".to_string(),
            line: Some(10),
            function: Some("bar".to_string()), // Different function
        };

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }
}
