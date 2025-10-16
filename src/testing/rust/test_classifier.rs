use super::RustTestType;
use quote::ToTokens;
use std::path::Path;
use syn::ItemFn;

/// Classifies Rust tests by type and detects test context
pub struct TestClassifier;

impl TestClassifier {
    pub fn new() -> Self {
        Self
    }

    /// Detect if a function is a test
    pub fn is_test_function(&self, func: &ItemFn) -> bool {
        self.has_test_attribute(func)
            || self.is_benchmark_test(func)
            || self.has_test_name_pattern(&func.sig.ident.to_string())
    }

    /// Classify the type of test
    pub fn classify_test_type(&self, func: &ItemFn, file_path: &Path) -> Option<RustTestType> {
        if !self.is_test_function(func) {
            return None;
        }

        // Check for benchmark tests
        if self.is_benchmark_test(func) {
            return Some(RustTestType::BenchmarkTest);
        }

        // Check for property tests
        if self.is_property_test(func) {
            return Some(RustTestType::PropertyTest);
        }

        // Check for integration tests based on file path
        if self.is_integration_test_path(file_path) {
            return Some(RustTestType::IntegrationTest);
        }

        // Default to unit test
        Some(RustTestType::UnitTest)
    }

    /// Check if function has test attributes
    fn has_test_attribute(&self, func: &ItemFn) -> bool {
        func.attrs.iter().any(|attr| {
            // Check for #[test]
            if attr.path().is_ident("test") {
                return true;
            }

            // Check for tokio::test, async_std::test, etc.
            if let Some(last_segment) = attr.path().segments.last() {
                if last_segment.ident == "test" {
                    return true;
                }
            }

            // Check for #[cfg(test)]
            if attr.path().is_ident("cfg") {
                let tokens = attr.meta.to_token_stream().to_string();
                if tokens.contains("test") {
                    return true;
                }
            }

            false
        })
    }

    /// Check if function name suggests it's a test
    fn has_test_name_pattern(&self, name: &str) -> bool {
        const TEST_PREFIXES: &[&str] = &["test_", "it_", "should_"];
        const MOCK_PATTERNS: &[&str] = &["mock", "stub", "fake"];

        let name_lower = name.to_lowercase();

        // Check test prefixes
        if TEST_PREFIXES.iter().any(|prefix| name.starts_with(prefix)) {
            return true;
        }

        // Check mock patterns (used in tests)
        if MOCK_PATTERNS
            .iter()
            .any(|pattern| name_lower.contains(pattern))
        {
            return true;
        }

        false
    }

    /// Check if it's a benchmark test
    fn is_benchmark_test(&self, func: &ItemFn) -> bool {
        func.attrs.iter().any(|attr| {
            attr.path().is_ident("bench")
                || attr.path().segments.iter().any(|seg| seg.ident == "bench")
        })
    }

    /// Check if it's a property test (proptest or quickcheck)
    fn is_property_test(&self, func: &ItemFn) -> bool {
        // Check for proptest macro
        let has_proptest = func.attrs.iter().any(|attr| {
            attr.path().segments.iter().any(|seg| {
                let ident_str = seg.ident.to_string();
                ident_str.contains("proptest") || ident_str.contains("quickcheck")
            })
        });

        if has_proptest {
            return true;
        }

        // Check function body for proptest! macro invocation
        let tokens = quote::quote!(#func).to_string();
        tokens.contains("proptest!") || tokens.contains("quickcheck!")
    }

    /// Check if the file path indicates an integration test
    fn is_integration_test_path(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Integration tests are in tests/ directory at project root
        path_str.contains("/tests/")
            || path_str.contains("\\tests\\")
            || path_str.starts_with("tests/")
            || path_str.starts_with("tests\\")
    }
}

impl Default for TestClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use syn::parse_quote;

    #[test]
    fn test_detect_standard_test() {
        let classifier = TestClassifier::new();
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_something() {
                assert_eq!(1, 1);
            }
        };
        assert!(classifier.is_test_function(&func));
    }

    #[test]
    fn test_detect_tokio_test() {
        let classifier = TestClassifier::new();
        let func: ItemFn = parse_quote! {
            #[tokio::test]
            async fn test_async() {
                assert_eq!(1, 1);
            }
        };
        assert!(classifier.is_test_function(&func));
    }

    #[test]
    fn test_detect_test_by_name() {
        let classifier = TestClassifier::new();
        let func: ItemFn = parse_quote! {
            fn test_something() {
                assert_eq!(1, 1);
            }
        };
        assert!(classifier.is_test_function(&func));
    }

    #[test]
    fn test_classify_unit_test() {
        let classifier = TestClassifier::new();
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_unit() {
                assert_eq!(1, 1);
            }
        };
        let path = PathBuf::from("src/lib.rs");
        assert_eq!(
            classifier.classify_test_type(&func, &path),
            Some(RustTestType::UnitTest)
        );
    }

    #[test]
    fn test_classify_integration_test() {
        let classifier = TestClassifier::new();
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_integration() {
                assert_eq!(1, 1);
            }
        };
        let path = PathBuf::from("tests/integration_test.rs");
        assert_eq!(
            classifier.classify_test_type(&func, &path),
            Some(RustTestType::IntegrationTest)
        );
    }

    #[test]
    fn test_classify_benchmark_test() {
        let classifier = TestClassifier::new();
        let func: ItemFn = parse_quote! {
            #[bench]
            fn bench_something(b: &mut Bencher) {
                b.iter(|| 2 + 2);
            }
        };
        let path = PathBuf::from("benches/bench.rs");
        assert_eq!(
            classifier.classify_test_type(&func, &path),
            Some(RustTestType::BenchmarkTest)
        );
    }

    #[test]
    fn test_non_test_function() {
        let classifier = TestClassifier::new();
        let func: ItemFn = parse_quote! {
            fn regular_function() {
                println!("not a test");
            }
        };
        assert!(!classifier.is_test_function(&func));
    }
}
