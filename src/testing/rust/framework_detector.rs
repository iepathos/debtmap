use super::RustTestFramework;
use syn::ItemFn;

/// Detects which test framework is being used
pub struct FrameworkDetector;

impl FrameworkDetector {
    pub fn new() -> Self {
        Self
    }

    /// Detect the test framework used by a function
    pub fn detect_framework(&self, func: &ItemFn) -> RustTestFramework {
        // Check attributes for framework markers
        for attr in &func.attrs {
            // Criterion benchmark
            if attr.path().is_ident("bench") {
                return RustTestFramework::Criterion;
            }

            // Check for proptest
            let path_str = attr
                .path()
                .segments
                .iter()
                .map(|seg| seg.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");

            if path_str.contains("proptest") {
                return RustTestFramework::Proptest;
            }

            if path_str.contains("quickcheck") {
                return RustTestFramework::Quickcheck;
            }

            if path_str.contains("rstest") {
                return RustTestFramework::Rstest;
            }
        }

        // Check function body for framework-specific patterns
        let tokens = quote::quote!(#func).to_string();

        if tokens.contains("proptest!") {
            return RustTestFramework::Proptest;
        }

        if tokens.contains("quickcheck!") {
            return RustTestFramework::Quickcheck;
        }

        // Check function parameters for criterion signature
        if self.has_criterion_signature(func) {
            return RustTestFramework::Criterion;
        }

        // Default to standard library tests
        RustTestFramework::Std
    }

    /// Check if function has criterion benchmark signature
    fn has_criterion_signature(&self, func: &ItemFn) -> bool {
        // Criterion benchmarks typically have a parameter of type &mut Bencher or &mut Criterion
        func.sig.inputs.iter().any(|arg| {
            if let syn::FnArg::Typed(pat_type) = arg {
                let type_str = quote::quote!(#pat_type.ty).to_string();
                type_str.contains("Bencher") || type_str.contains("Criterion")
            } else {
                false
            }
        })
    }

    /// Check if framework is property-based testing
    pub fn is_property_testing(&self, framework: &RustTestFramework) -> bool {
        matches!(
            framework,
            RustTestFramework::Proptest | RustTestFramework::Quickcheck
        )
    }

    /// Check if framework is benchmarking
    pub fn is_benchmarking(&self, framework: &RustTestFramework) -> bool {
        matches!(framework, RustTestFramework::Criterion)
    }
}

impl Default for FrameworkDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_detect_std_test() {
        let detector = FrameworkDetector::new();
        let func: ItemFn = parse_quote! {
            #[test]
            fn test_something() {
                assert_eq!(1, 1);
            }
        };
        assert_eq!(detector.detect_framework(&func), RustTestFramework::Std);
    }

    #[test]
    fn test_detect_criterion() {
        let detector = FrameworkDetector::new();
        let func: ItemFn = parse_quote! {
            #[bench]
            fn bench_something(b: &mut Bencher) {
                b.iter(|| 2 + 2);
            }
        };
        assert_eq!(
            detector.detect_framework(&func),
            RustTestFramework::Criterion
        );
    }

    #[test]
    fn test_is_property_testing() {
        let detector = FrameworkDetector::new();
        assert!(detector.is_property_testing(&RustTestFramework::Proptest));
        assert!(detector.is_property_testing(&RustTestFramework::Quickcheck));
        assert!(!detector.is_property_testing(&RustTestFramework::Std));
    }

    #[test]
    fn test_is_benchmarking() {
        let detector = FrameworkDetector::new();
        assert!(detector.is_benchmarking(&RustTestFramework::Criterion));
        assert!(!detector.is_benchmarking(&RustTestFramework::Std));
    }
}
