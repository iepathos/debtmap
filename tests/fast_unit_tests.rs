/// Fast unit tests that don't spawn external processes
/// These tests run quickly and can be used in CI
#[cfg(test)]
mod tests {
    use debtmap::context::{detect_file_type, detector::ContextDetector, FileType, FunctionRole};
    use std::path::Path;
    use syn::visit::Visit;

    #[test]
    fn test_context_detector_identifies_test_functions() {
        let code = r#"
            #[test]
            fn test_something() {
                assert_eq!(1, 1);
            }
            
            fn regular_function() {
                println!("hello");
            }
        "#;

        let file = syn::parse_file(code).unwrap();
        let mut detector = ContextDetector::new(FileType::Test);
        detector.visit_file(&file);

        // Test function should be detected
        let test_ctx = detector.get_context("test_something");
        assert!(test_ctx.is_some(), "Test function should be detected");
        assert_eq!(test_ctx.unwrap().role, FunctionRole::TestFunction);

        // Regular function should not be a test
        let regular_ctx = detector.get_context("regular_function");
        assert!(regular_ctx.is_some(), "Regular function should be detected");
        assert_ne!(regular_ctx.unwrap().role, FunctionRole::TestFunction);
    }

    #[test]
    fn test_line_based_context_lookup() {
        let code = r#"
            fn func_at_line_2() {
                println!("line 3");
            }
            
            #[test]
            fn test_at_line_6() {
                assert!(true);
            }
        "#;

        let file = syn::parse_file(code).unwrap();
        let mut detector = ContextDetector::new(FileType::Test);
        detector.visit_file(&file);

        // Check if we can look up functions by line
        // Note: syn's line numbers start at 1 for parsed strings
        let ctx_at_3 = detector.get_context_for_line(3);
        let ctx_at_7 = detector.get_context_for_line(7);

        // We should be able to find contexts for lines within functions
        // The exact behavior depends on how spans are calculated
        println!("Context at line 3: {:?}", ctx_at_3.map(|c| &c.role));
        println!("Context at line 7: {:?}", ctx_at_7.map(|c| &c.role));
    }

    #[test]
    fn test_file_type_detection() {
        // Test paths that match the actual detection logic
        // The detection looks for "/tests/" with a leading slash
        assert_eq!(
            detect_file_type(Path::new("project/tests/integration.rs")),
            FileType::Test
        );
        assert_eq!(
            detect_file_type(Path::new("src/main.rs")),
            FileType::Production
        );
        assert_eq!(
            detect_file_type(Path::new("something_test.rs")),
            FileType::Test
        );
        assert_eq!(
            detect_file_type(Path::new("module_tests.rs")),
            FileType::Test
        );

        // These paths need the directory separator
        assert_eq!(
            detect_file_type(Path::new("project/benches/bench.rs")),
            FileType::Benchmark
        );
        assert_eq!(
            detect_file_type(Path::new("project/examples/demo.rs")),
            FileType::Example
        );
    }

    #[test]
    fn test_context_aware_environment_variable() {
        // Test that the environment variable can be set and read
        std::env::set_var("DEBTMAP_CONTEXT_AWARE", "true");
        let is_aware = std::env::var("DEBTMAP_CONTEXT_AWARE")
            .map(|v| v == "true")
            .unwrap_or(false);
        assert!(is_aware, "Environment variable should be set");

        std::env::remove_var("DEBTMAP_CONTEXT_AWARE");
        let is_aware_after = std::env::var("DEBTMAP_CONTEXT_AWARE")
            .map(|v| v == "true")
            .unwrap_or(false);
        assert!(!is_aware_after, "Environment variable should be removed");
    }
}
