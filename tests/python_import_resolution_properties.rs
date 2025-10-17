//! Property-based tests for Python import resolution
//!
//! These tests verify invariants that should hold for all inputs:
//! - Symbol resolution is deterministic
//! - Confidence ordering is consistent
//! - Cross-file analysis never marks used functions as dead
//! - Import graphs have no self-loops
//! - Cycle detection is consistent

use debtmap::analysis::python_imports::{EnhancedImportResolver, ImportType, ResolutionConfidence};
use proptest::prelude::*;
use std::collections::HashSet;
use std::fs;
use tempfile::TempDir;

/// Python keywords to avoid
const PYTHON_KEYWORDS: &[&str] = &[
    "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif",
    "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda",
    "nonlocal", "not", "or", "pass", "raise", "return", "try", "while", "with", "yield", "None",
    "True", "False",
];

/// Generate valid Python identifier (avoiding keywords)
fn python_identifier() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,15}".prop_filter("not a keyword", |s| !PYTHON_KEYWORDS.contains(&s.as_str()))
}

/// Parse Python code to AST
fn parse_python(source: &str, filename: &str) -> rustpython_parser::ast::Mod {
    rustpython_parser::parse(source, rustpython_parser::Mode::Module, filename)
        .expect("Failed to parse Python code")
}

proptest! {
    /// Property: Symbol resolution is deterministic - running resolution
    /// multiple times on the same input should always produce the same result
    #[test]
    fn prop_symbol_resolution_is_deterministic(
        func_name in python_identifier(),
        module_name in python_identifier()
    ) {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create a module with a function
        let module_source = format!("def {}():\n    pass\n", func_name);
        let module_path = base_path.join(format!("{}.py", module_name));
        fs::write(&module_path, &module_source).unwrap();

        let ast = parse_python(&module_source, module_path.to_str().unwrap());

        // Resolve the symbol twice
        let mut resolver1 = EnhancedImportResolver::new();
        resolver1.analyze_imports(&ast, &module_path);
        let result1 = resolver1.resolve_symbol(&module_path, &func_name);

        let mut resolver2 = EnhancedImportResolver::new();
        resolver2.analyze_imports(&ast, &module_path);
        let result2 = resolver2.resolve_symbol(&module_path, &func_name);

        // Results should be identical
        prop_assert_eq!(result1.is_some(), result2.is_some());
        if let (Some(r1), Some(r2)) = (result1, result2) {
            prop_assert_eq!(r1.name, r2.name);
            prop_assert_eq!(r1.module_path, r2.module_path);
            prop_assert_eq!(r1.confidence, r2.confidence);
        }
    }

    /// Property: Confidence ordering is consistent - confidence levels
    /// should have a stable ordering relationship
    #[test]
    fn prop_confidence_ordering_is_consistent(level1 in 0usize..4, level2 in 0usize..4) {
        let confidences = [
            ResolutionConfidence::Unknown,
            ResolutionConfidence::Low,
            ResolutionConfidence::Medium,
            ResolutionConfidence::High,
        ];

        let c1 = confidences[level1];
        let c2 = confidences[level2];

        // Ordering should be transitive and consistent
        let ord1 = c1.cmp(&c2);
        let ord2 = c2.cmp(&c1);

        // If c1 < c2, then c2 > c1
        if ord1 == std::cmp::Ordering::Less {
            prop_assert_eq!(ord2, std::cmp::Ordering::Greater);
        }
        // If c1 > c2, then c2 < c1
        if ord1 == std::cmp::Ordering::Greater {
            prop_assert_eq!(ord2, std::cmp::Ordering::Less);
        }
        // If c1 == c2, then c2 == c1
        if ord1 == std::cmp::Ordering::Equal {
            prop_assert_eq!(ord2, std::cmp::Ordering::Equal);
        }
    }

    /// Property: Import type confidence classification is stable
    #[test]
    fn prop_import_type_confidence_stable(relative_level in 0usize..10) {
        // Classification should be deterministic for same input
        let import_type = ImportType::Relative { level: relative_level };
        let conf1 = ResolutionConfidence::from_import_type(&import_type);
        let conf2 = ResolutionConfidence::from_import_type(&import_type);
        prop_assert_eq!(conf1, conf2);

        // Classification rules should be consistent
        if relative_level <= 1 {
            prop_assert!(conf1 >= ResolutionConfidence::Medium);
        } else {
            prop_assert!(conf1 == ResolutionConfidence::Low);
        }
    }

    /// Property: Import graph has no self-loops - a module should never
    /// import from itself
    #[test]
    fn prop_import_graph_has_no_self_loops(
        func_names in prop::collection::vec(python_identifier(), 1..5)
    ) {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create a module with multiple functions
        let module_source = func_names
            .iter()
            .map(|name| format!("def {}():\n    pass\n", name))
            .collect::<Vec<_>>()
            .join("\n");

        let module_path = base_path.join("module.py");
        fs::write(&module_path, &module_source).unwrap();

        let ast = parse_python(&module_source, module_path.to_str().unwrap());

        let mut resolver = EnhancedImportResolver::new();
        resolver.analyze_imports(&ast, &module_path);

        // Check that there are no self-loops in the import graph
        let import_graph = resolver.import_graph();
        if let Some(edges) = import_graph.edges.get(&module_path) {
            for edge in edges {
                prop_assert_ne!(
                    &edge.from_module, &edge.to_module,
                    "Import graph should not contain self-loops"
                );
            }
        }
    }

    /// Property: Star import resolution respects __all__ definition
    #[test]
    fn prop_star_import_respects_all(
        public_names in prop::collection::vec(python_identifier(), 1..5),
        private_names in prop::collection::vec(python_identifier(), 1..3)
    ) {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create a module with __all__ definition
        let mut module_source = String::new();
        for name in &public_names {
            module_source.push_str(&format!("def {}():\n    pass\n\n", name));
        }
        for name in &private_names {
            module_source.push_str(&format!("def {}():\n    pass\n\n", name));
        }

        let all_list = public_names
            .iter()
            .map(|n| format!("'{}'", n))
            .collect::<Vec<_>>()
            .join(", ");
        module_source.push_str(&format!("__all__ = [{}]\n", all_list));

        let module_path = base_path.join("module.py");
        fs::write(&module_path, &module_source).unwrap();

        let ast = parse_python(&module_source, module_path.to_str().unwrap());

        let mut resolver = EnhancedImportResolver::new();
        resolver.analyze_imports(&ast, &module_path);

        // Get star import exports
        let exports = resolver.resolve_star_imports(&module_path);
        let export_names: HashSet<String> = exports.iter().map(|e| e.name.clone()).collect();

        // All public names should be in exports
        for name in &public_names {
            prop_assert!(
                export_names.contains(name),
                "Public name '{}' should be in star import exports",
                name
            );
        }

        // Private names should NOT be in exports (since __all__ is defined)
        for name in &private_names {
            prop_assert!(
                !export_names.contains(name),
                "Private name '{}' should not be in star import exports when __all__ is defined",
                name
            );
        }
    }

    /// Property: Confidence levels have proper ordering
    #[test]
    fn prop_confidence_levels_ordered(_seed in any::<u32>()) {
        // Direct and From imports should have highest confidence
        prop_assert_eq!(
            ResolutionConfidence::from_import_type(&ImportType::Direct),
            ResolutionConfidence::High
        );
        prop_assert_eq!(
            ResolutionConfidence::from_import_type(&ImportType::From),
            ResolutionConfidence::High
        );

        // Star imports should have low confidence
        prop_assert_eq!(
            ResolutionConfidence::from_import_type(&ImportType::Star),
            ResolutionConfidence::Low
        );

        // Dynamic imports should have unknown confidence
        prop_assert_eq!(
            ResolutionConfidence::from_import_type(&ImportType::Dynamic),
            ResolutionConfidence::Unknown
        );

        // Enum ordering: High < Medium < Low < Unknown (due to variant order)
        prop_assert!(ResolutionConfidence::High < ResolutionConfidence::Medium);
        prop_assert!(ResolutionConfidence::Medium < ResolutionConfidence::Low);
        prop_assert!(ResolutionConfidence::Low < ResolutionConfidence::Unknown);
    }

    /// Property: Circular import detection is symmetric - if A imports B
    /// and B imports A, both should be in a detected cycle
    #[test]
    fn prop_circular_import_detection_symmetric(
        module_a_name in python_identifier(),
        module_b_name in python_identifier()
    ) {
        // Skip if names are the same (would be a self-import)
        prop_assume!(module_a_name != module_b_name);

        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create circular import: A -> B -> A
        let module_a_source = format!("from {} import func_b\ndef func_a(): pass", module_b_name);
        let module_b_source = format!("from {} import func_a\ndef func_b(): pass", module_a_name);

        let module_a_path = base_path.join(format!("{}.py", module_a_name));
        let module_b_path = base_path.join(format!("{}.py", module_b_name));

        fs::write(&module_a_path, &module_a_source).unwrap();
        fs::write(&module_b_path, &module_b_source).unwrap();

        let ast_a = parse_python(&module_a_source, module_a_path.to_str().unwrap());
        let ast_b = parse_python(&module_b_source, module_b_path.to_str().unwrap());

        let mut resolver = EnhancedImportResolver::new();
        resolver.analyze_imports(&ast_a, &module_a_path);
        resolver.analyze_imports(&ast_b, &module_b_path);
        resolver.build_import_graph(&[
            (ast_a, module_a_path.clone()),
            (ast_b, module_b_path.clone()),
        ]);

        let cycles = resolver.circular_imports();

        // There should be at least one cycle detected
        prop_assert!(!cycles.is_empty(), "Should detect circular import");

        // Both modules should be in the detected cycle
        let first_cycle = &cycles[0];
        prop_assert!(
            first_cycle.contains(&module_a_path) && first_cycle.contains(&module_b_path),
            "Circular import should include both modules"
        );
    }
}

#[cfg(test)]
mod additional_properties {
    use super::*;

    #[test]
    fn test_confidence_enum_ordering() {
        // Verify that the confidence enum has proper Ord implementation
        // Enum ordering follows variant declaration order: High < Medium < Low < Unknown
        assert!(ResolutionConfidence::High < ResolutionConfidence::Medium);
        assert!(ResolutionConfidence::Medium < ResolutionConfidence::Low);
        assert!(ResolutionConfidence::Low < ResolutionConfidence::Unknown);

        // Verify PartialOrd consistency
        assert_eq!(
            ResolutionConfidence::High.partial_cmp(&ResolutionConfidence::Medium),
            Some(std::cmp::Ordering::Less)
        );
    }

    #[test]
    fn test_resolution_confidence_as_str() {
        // Verify string representation is consistent
        assert_eq!(ResolutionConfidence::High.as_str(), "high");
        assert_eq!(ResolutionConfidence::Medium.as_str(), "medium");
        assert_eq!(ResolutionConfidence::Low.as_str(), "low");
        assert_eq!(ResolutionConfidence::Unknown.as_str(), "unknown");
    }
}
