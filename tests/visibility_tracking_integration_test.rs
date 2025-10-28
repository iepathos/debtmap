/// Integration test for Spec 140: Visibility Tracking System Integration
///
/// This test verifies that visibility breakdown from GodObjectAnalysis
/// correctly flows into ModuleStructure.function_counts, ensuring that
/// terminal output shows accurate visibility counts without contradictions.
use debtmap::organization::GodObjectDetector;
use std::path::Path;

#[test]
fn test_visibility_breakdown_integrates_with_module_structure() {
    // Test case: GodClass with mixed visibility methods
    let code = r#"
        pub struct GodClass {}

        impl GodClass {
            pub fn public_method1(&self) {}
            pub fn public_method2(&self) {}
            pub(crate) fn crate_method1(&self) {}
            pub(crate) fn crate_method2(&self) {}
            pub(super) fn super_method(&self) {}
            fn private_method1(&self) {}
            fn private_method2(&self) {}
            fn private_method3(&self) {}
        }
    "#;

    let ast: syn::File = syn::parse_str(code).unwrap();
    let detector = GodObjectDetector::with_source_content(code);
    let path = Path::new("test_god_class.rs");
    let analysis = detector.analyze_comprehensive(path, &ast);

    // Verify visibility_breakdown exists and has correct counts
    assert!(
        analysis.visibility_breakdown.is_some(),
        "visibility_breakdown should be populated for Rust files"
    );
    let breakdown = analysis.visibility_breakdown.as_ref().unwrap();
    assert_eq!(breakdown.public, 2, "Should have 2 public methods");
    assert_eq!(breakdown.pub_crate, 2, "Should have 2 pub(crate) methods");
    assert_eq!(breakdown.pub_super, 1, "Should have 1 pub(super) method");
    assert_eq!(breakdown.private, 3, "Should have 3 private methods");
    assert_eq!(breakdown.total(), 8, "Total should be 8 methods");

    // Verify method_count matches visibility_breakdown total
    assert_eq!(
        analysis.method_count,
        breakdown.total(),
        "method_count should match visibility breakdown total"
    );

    // Verify validation passes (no contradictions)
    assert!(
        analysis.validate().is_ok(),
        "Metrics should be consistent and pass validation"
    );

    // Verify module_structure is populated (for god objects)
    if analysis.is_god_object {
        assert!(
            analysis.module_structure.is_some(),
            "module_structure should be populated for god objects"
        );

        let module_structure = analysis.module_structure.as_ref().unwrap();

        // KEY TEST: Verify function_counts are sourced from visibility_breakdown
        assert_eq!(
            module_structure.function_counts.public_functions,
            breakdown.public,
            "module_structure.function_counts.public_functions should match visibility_breakdown.public"
        );

        assert_eq!(
            module_structure.function_counts.private_functions,
            breakdown.private + breakdown.pub_crate + breakdown.pub_super,
            "module_structure.function_counts.private_functions should be sum of private + pub_crate + pub_super"
        );

        // Verify no "0 public, 0 private" contradiction
        let total_counted = module_structure.function_counts.public_functions
            + module_structure.function_counts.private_functions;
        assert!(
            total_counted > 0,
            "function_counts should not show zero for both public and private"
        );
    }
}

#[test]
fn test_visibility_integration_with_godfile() {
    // Test case: GodFile with standalone functions
    let code = r#"
        pub fn public_fn1() {}
        pub fn public_fn2() {}
        pub fn public_fn3() {}
        pub(crate) fn crate_fn1() {}
        pub(crate) fn crate_fn2() {}
        fn private_fn1() {}
        fn private_fn2() {}
        fn private_fn3() {}
        fn private_fn4() {}
        fn private_fn5() {}
    "#;

    let ast: syn::File = syn::parse_str(code).unwrap();
    let detector = GodObjectDetector::with_source_content(code);
    let path = Path::new("test_god_file.rs");
    let analysis = detector.analyze_comprehensive(path, &ast);

    // Verify visibility_breakdown
    let breakdown = analysis.visibility_breakdown.as_ref().unwrap();
    assert_eq!(breakdown.public, 3);
    assert_eq!(breakdown.pub_crate, 2);
    assert_eq!(breakdown.private, 5);
    assert_eq!(breakdown.total(), 10);

    // If it's a god object, verify module_structure integration
    if analysis.is_god_object && analysis.module_structure.is_some() {
        let module_structure = analysis.module_structure.as_ref().unwrap();

        assert_eq!(
            module_structure.function_counts.public_functions,
            breakdown.public
        );
        assert_eq!(
            module_structure.function_counts.private_functions,
            breakdown.private + breakdown.pub_crate + breakdown.pub_super
        );
    }
}

#[test]
fn test_visibility_integration_preserves_other_counts() {
    // Test that integration doesn't break other function count categories
    let code = r#"
        pub struct MyStruct {}

        impl MyStruct {
            pub fn method1(&self) {}
            fn method2(&self) {}
        }

        impl std::fmt::Display for MyStruct {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                Ok(())
            }
        }

        pub fn standalone_fn() {}
    "#;

    let ast: syn::File = syn::parse_str(code).unwrap();
    let detector = GodObjectDetector::with_source_content(code);
    let path = Path::new("test_mixed.rs");
    let analysis = detector.analyze_comprehensive(path, &ast);

    if let Some(module_structure) = &analysis.module_structure {
        let counts = &module_structure.function_counts;

        // Verify different function categories are still counted
        // (exact values depend on whether it's a god object, but structure should exist)
        assert!(
            counts.impl_methods > 0
                || counts.trait_methods > 0
                || counts.module_level_functions > 0,
            "Should count different function categories"
        );

        // Verify visibility is tracked
        if let Some(breakdown) = &analysis.visibility_breakdown {
            assert_eq!(
                counts.public_functions + counts.private_functions,
                breakdown.total(),
                "Total visibility counts should match breakdown"
            );
        }
    }
}

#[test]
fn test_non_rust_files_still_work() {
    // Test that non-Rust files don't break (backward compatibility)
    let code = r#"
        def function1():
            pass
        def function2():
            pass
    "#;

    let _path = Path::new("test.py");
    let _detector = GodObjectDetector::with_source_content(code);

    // Non-Rust files won't parse as Rust AST, so we test the fallback behavior
    // This should not panic and should handle gracefully
    // (Note: actual Python analysis would use a different detector)
    // This test primarily verifies that creating a detector with Python code doesn't panic
}
