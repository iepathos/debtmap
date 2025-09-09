//! Unit tests for pure functions extracted during refactoring

#[cfg(test)]
mod tests {
    use super::super::*;
    use im::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_meets_delegation_criteria() {
        // Low complexity with multiple callees should meet criteria
        assert!(CallGraph::meets_delegation_criteria(2, 3));
        assert!(CallGraph::meets_delegation_criteria(3, 2));

        // High complexity should not meet criteria
        assert!(!CallGraph::meets_delegation_criteria(4, 3));

        // Too few callees should not meet criteria
        assert!(!CallGraph::meets_delegation_criteria(2, 1));
    }

    #[test]
    fn test_calculate_average_callee_complexity() {
        let mut nodes = HashMap::new();

        // Create test functions with different complexities
        let func1 = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func1".to_string(),
            line: 10,
        };
        let func2 = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func2".to_string(),
            line: 20,
        };

        nodes.insert(
            func1.clone(),
            FunctionNode {
                id: func1.clone(),
                is_entry_point: false,
                is_test: false,
                complexity: 5,
                _lines: 30,
            },
        );

        nodes.insert(
            func2.clone(),
            FunctionNode {
                id: func2.clone(),
                is_entry_point: false,
                is_test: false,
                complexity: 10,
                _lines: 50,
            },
        );

        let callees = vec![func1, func2];
        let avg = CallGraph::calculate_average_callee_complexity(&callees, &nodes);

        assert_eq!(avg, 7.5);
    }

    #[test]
    fn test_indicates_delegation() {
        // Callees significantly more complex than orchestrator
        assert!(CallGraph::indicates_delegation(2, 10.0));
        assert!(CallGraph::indicates_delegation(3, 5.0));

        // Callees not significantly more complex
        assert!(!CallGraph::indicates_delegation(5, 6.0));
        assert!(!CallGraph::indicates_delegation(10, 10.0));
    }

    #[test]
    fn test_entry_point_criticality_factor() {
        assert_eq!(CallGraph::entry_point_criticality_factor(true), 2.0);
        assert_eq!(CallGraph::entry_point_criticality_factor(false), 1.0);
    }

    #[test]
    fn test_dependency_count_criticality_factor() {
        assert_eq!(CallGraph::dependency_count_criticality_factor(0), 1.0);
        assert_eq!(CallGraph::dependency_count_criticality_factor(2), 1.0);
        assert_eq!(CallGraph::dependency_count_criticality_factor(3), 1.2);
        assert_eq!(CallGraph::dependency_count_criticality_factor(6), 1.5);
    }

    #[test]
    fn test_has_entry_point_caller() {
        let func1 = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "entry".to_string(),
            line: 10,
        };
        let func2 = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "regular".to_string(),
            line: 20,
        };

        let callers = vec![func1.clone(), func2.clone()];

        // Test with entry point in callers
        assert!(CallGraph::has_entry_point_caller(&callers, |id| id == &func1));

        // Test without entry point in callers
        assert!(!CallGraph::has_entry_point_caller(&callers, |id| id.name == "not_present"));
    }

    #[test]
    fn test_entry_point_caller_criticality_factor() {
        assert_eq!(CallGraph::entry_point_caller_criticality_factor(true), 1.3);
        assert_eq!(CallGraph::entry_point_caller_criticality_factor(false), 1.0);
    }

    #[test]
    fn test_functions_in_file() {
        let mut nodes = HashMap::new();
        let file1 = PathBuf::from("file1.rs");
        let file2 = PathBuf::from("file2.rs");

        let func1 = FunctionId {
            file: file1.clone(),
            name: "func1".to_string(),
            line: 10,
        };
        let func2 = FunctionId {
            file: file1.clone(),
            name: "func2".to_string(),
            line: 20,
        };
        let func3 = FunctionId {
            file: file2.clone(),
            name: "func3".to_string(),
            line: 30,
        };

        nodes.insert(
            func1.clone(),
            FunctionNode {
                id: func1.clone(),
                is_entry_point: false,
                is_test: false,
                complexity: 1,
                _lines: 10,
            },
        );
        nodes.insert(
            func2.clone(),
            FunctionNode {
                id: func2.clone(),
                is_entry_point: false,
                is_test: false,
                complexity: 1,
                _lines: 10,
            },
        );
        nodes.insert(
            func3.clone(),
            FunctionNode {
                id: func3.clone(),
                is_entry_point: false,
                is_test: false,
                complexity: 1,
                _lines: 10,
            },
        );

        let functions = CallGraph::functions_in_file(&nodes, &file1);
        assert_eq!(functions.len(), 2);

        let functions = CallGraph::functions_in_file(&nodes, &file2);
        assert_eq!(functions.len(), 1);
    }

    #[test]
    fn test_find_best_line_match() {
        let func1 = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func1".to_string(),
            line: 10,
        };
        let func2 = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func2".to_string(),
            line: 30,
        };
        let func3 = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func3".to_string(),
            line: 50,
        };

        let functions = vec![&func1, &func2, &func3];

        // Line 25 should match func1 (line 10 is closest before 25)
        let result = CallGraph::find_best_line_match(&functions, 25);
        assert_eq!(result.unwrap().line, 10);

        // Line 40 should match func2 (line 30 is closest before 40)
        let result = CallGraph::find_best_line_match(&functions, 40);
        assert_eq!(result.unwrap().line, 30);

        // Line 5 should not match anything (before all functions)
        let result = CallGraph::find_best_line_match(&functions, 5);
        assert!(result.is_none());
    }

    #[test]
    fn test_is_production_entry_point() {
        let node_entry = FunctionNode {
            id: FunctionId {
                file: PathBuf::from("test.rs"),
                name: "entry".to_string(),
                line: 10,
            },
            is_entry_point: true,
            is_test: false,
            complexity: 1,
            _lines: 10,
        };

        let node_test = FunctionNode {
            id: FunctionId {
                file: PathBuf::from("test.rs"),
                name: "test".to_string(),
                line: 20,
            },
            is_entry_point: false,
            is_test: true,
            complexity: 1,
            _lines: 10,
        };

        let node_regular = FunctionNode {
            id: FunctionId {
                file: PathBuf::from("test.rs"),
                name: "regular".to_string(),
                line: 30,
            },
            is_entry_point: false,
            is_test: false,
            complexity: 1,
            _lines: 10,
        };

        // Entry point should be production entry point
        assert!(CallGraph::is_production_entry_point(&node_entry, &[]));

        // Test function should not be production entry point
        assert!(!CallGraph::is_production_entry_point(&node_test, &[]));

        // Regular function with no callers should be production entry point
        assert!(CallGraph::is_production_entry_point(&node_regular, &[]));

        // Regular function with callers should not be production entry point
        let callers = vec![FunctionId {
            file: PathBuf::from("test.rs"),
            name: "caller".to_string(),
            line: 40,
        }];
        assert!(!CallGraph::is_production_entry_point(
            &node_regular,
            &callers
        ));
    }
}
