//! Purity detection for Rust functions
//!
//! This module provides static analysis to determine whether Rust functions are pure
//! (no side effects) or impure. It classifies functions into purity levels:
//!
//! - `StrictlyPure`: No side effects whatsoever
//! - `LocallyPure`: May have local mutations but no external effects
//! - `ReadOnly`: May read external state but doesn't modify it
//! - `Impure`: Has side effects or modifies external state
//!
//! # Example
//!
//! ```ignore
//! use debtmap::analyzers::purity_detector::PurityDetector;
//!
//! let code = r#"
//!     fn add(a: i32, b: i32) -> i32 {
//!         a + b
//!     }
//! "#;
//! let item_fn: syn::ItemFn = syn::parse_str(code).unwrap();
//! let mut detector = PurityDetector::new();
//! let analysis = detector.is_pure_function(&item_fn);
//! assert!(analysis.is_pure);
//! ```

mod confidence;
mod constants;
mod detector;
mod io_detection;
mod macro_handling;
mod mutation_scope;
mod path_classification;
mod types;
mod unsafe_analysis;

// Re-export main types for backward compatibility
pub use detector::PurityDetector;
pub use path_classification::{is_known_pure_call, is_known_pure_method};
pub use types::{ImpurityReason, LocalMutation, MutationScope, PurityAnalysis, UpvalueMutation};

// Re-export constants for external use
pub use constants::KNOWN_PURE_STD_FUNCTIONS;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::PurityLevel;
    use syn::ItemFn;

    fn analyze_function_str(code: &str) -> PurityAnalysis {
        let item_fn = syn::parse_str::<ItemFn>(code).unwrap();
        let mut detector = PurityDetector::new();
        detector.is_pure_function(&item_fn)
    }

    #[test]
    fn test_pure_function() {
        let analysis = analyze_function_str(
            r#"
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }
            "#,
        );
        assert!(analysis.is_pure);
        assert!(analysis.reasons.is_empty());
    }

    #[test]
    fn test_function_with_print() {
        let analysis = analyze_function_str(
            r#"
            fn debug_add(a: i32, b: i32) -> i32 {
                println!("Adding {} + {}", a, b);
                a + b
            }
            "#,
        );
        assert!(!analysis.is_pure);
        assert!(analysis.reasons.contains(&ImpurityReason::IOOperations));
    }

    #[test]
    fn test_function_with_mutable_param() {
        let analysis = analyze_function_str(
            r#"
            fn increment(x: &mut i32) {
                *x += 1;
            }
            "#,
        );
        assert!(!analysis.is_pure);
        assert!(analysis
            .reasons
            .contains(&ImpurityReason::MutableParameters));
    }

    #[test]
    fn test_function_with_unsafe_read_is_pure() {
        let analysis = analyze_function_str(
            r#"
            fn dangerous() -> i32 {
                unsafe {
                    std::ptr::null::<i32>().read()
                }
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
        assert!(analysis.confidence < 0.90);
    }

    #[test]
    fn test_local_mutation_is_locally_pure() {
        let analysis = analyze_function_str(
            r#"
            fn process_data(input: Vec<i32>) -> Vec<i32> {
                let mut result = Vec::new();
                for item in input {
                    result.push(item * 2);
                }
                result
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::LocallyPure);
        assert!(analysis.confidence > 0.85);
    }

    #[test]
    fn test_builder_pattern_is_locally_pure() {
        let analysis = analyze_function_str(
            r#"
            fn with_value(mut self, value: u32) -> Self {
                self.value = value;
                self
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::LocallyPure);
    }

    #[test]
    fn test_strictly_pure_function() {
        let analysis = analyze_function_str(
            r#"
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    // Tests for spec 259: Fix constants false positive in purity analysis

    #[test]
    fn test_std_max_constant_is_pure() {
        let analysis = analyze_function_str(
            r#"
            fn is_valid(x: i32) -> bool {
                x < std::i32::MAX
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_core_constant_is_pure() {
        let analysis = analyze_function_str(
            r#"
            fn min_val() -> u64 {
                core::u64::MIN
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_float_constants_are_pure() {
        let analysis = analyze_function_str(
            r#"
            fn is_infinite(x: f64) -> bool {
                x == std::f64::INFINITY || x == std::f64::NEG_INFINITY
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_float_math_constants_are_pure() {
        let analysis = analyze_function_str(
            r#"
            fn get_pi() -> f64 {
                std::f64::consts::PI
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_enum_variant_is_pure() {
        let analysis = analyze_function_str(
            r#"
            fn default_option() -> Option<i32> {
                Option::None
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_screaming_case_constant_is_pure() {
        let analysis = analyze_function_str(
            r#"
            fn get_max() -> usize {
                config::MAX_SIZE
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
        assert!(analysis.confidence < 0.98);
    }

    #[test]
    fn test_unknown_path_is_conservative() {
        let analysis = analyze_function_str(
            r#"
            fn get_value() -> i32 {
                external_crate::get_value
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::ReadOnly);
    }

    #[test]
    fn test_multiple_constants_are_pure() {
        let analysis = analyze_function_str(
            r#"
            fn range_check(x: i32) -> bool {
                x >= std::i32::MIN && x <= std::i32::MAX
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_external_mutation_is_impure() {
        let analysis = analyze_function_str(
            r#"
            fn increment(&mut self) {
                self.count += 1;
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }

    // Tests for spec 160a: macro classification fix

    #[test]
    #[cfg(not(debug_assertions))]
    fn test_debug_assert_pure_in_release() {
        let analysis = analyze_function_str(
            r#"
            fn check_bounds(x: usize) -> bool {
                debug_assert!(x < 100);
                debug_assert_eq!(x, x);
                x < 100
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    #[cfg(debug_assertions)]
    fn test_debug_assert_impure_in_debug() {
        let analysis = analyze_function_str(
            r#"
            fn check_bounds(x: usize) -> bool {
                debug_assert!(x < 100);
                x < 100
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }

    #[test]
    fn test_io_macros_always_impure() {
        let test_cases = vec![
            r#"fn f() { println!("test"); }"#,
            r#"fn f() { eprintln!("error"); }"#,
            r#"fn f() { dbg!(42); }"#,
            r#"fn f() { print!("no newline"); }"#,
        ];

        for code in test_cases {
            let analysis = analyze_function_str(code);
            assert_eq!(
                analysis.purity_level,
                PurityLevel::Impure,
                "Failed for: {}",
                code
            );
        }
    }

    #[test]
    fn test_expression_macros() {
        let analysis = analyze_function_str(
            r#"
            fn example() -> i32 {
                let x = dbg!(42);
                x
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }

    #[test]
    fn test_pure_macros() {
        let analysis = analyze_function_str(
            r#"
            fn create_list() -> Vec<i32> {
                vec![1, 2, 3]
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_no_substring_false_positives() {
        let analysis = analyze_function_str(
            r#"
            fn example() -> i32 {
                42
            }
            "#,
        );
        assert_ne!(analysis.purity_level, PurityLevel::Impure);
    }

    #[test]
    fn test_assert_always_impure() {
        let analysis = analyze_function_str(
            r#"
            fn validate(x: i32) -> i32 {
                assert!(x > 0);
                x
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }

    // Tests for spec 160b: macro definition collection

    #[test]
    fn test_purity_with_custom_macros() {
        use crate::analyzers::macro_definition_collector::*;
        use dashmap::DashMap;
        use std::sync::Arc;

        let code = r#"
            macro_rules! my_logger {
                ($($arg:tt)*) => {
                    eprintln!($($arg)*);
                };
            }

            fn example() {
                my_logger!("test");
            }
        "#;

        let ast = syn::parse_file(code).unwrap();
        let definitions = Arc::new(DashMap::new());
        collect_definitions(&ast, std::path::Path::new("test.rs"), definitions.clone());

        assert!(definitions.contains_key("my_logger"));

        let mut detector = PurityDetector::with_macro_definitions(definitions);

        let func_code = r#"
            fn example() {
                my_logger!("test");
            }
        "#;
        let item_fn = syn::parse_str::<ItemFn>(func_code).unwrap();
        let analysis = detector.is_pure_function(&item_fn);

        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }

    #[test]
    fn test_known_vs_unknown_macro_confidence() {
        use crate::analyzers::macro_definition_collector::*;
        use dashmap::DashMap;
        use std::sync::Arc;

        let definitions = Arc::new(DashMap::new());
        definitions.insert(
            "my_macro".to_string(),
            MacroDefinition {
                name: "my_macro".to_string(),
                body: String::new(),
                source_file: std::path::PathBuf::from("test.rs"),
                line: 1,
            },
        );

        let mut detector_with_def = PurityDetector::with_macro_definitions(definitions);
        let func_with_known = r#"
            fn test() {
                my_macro!();
            }
        "#;
        let item_fn = syn::parse_str::<ItemFn>(func_with_known).unwrap();
        let analysis_with_def = detector_with_def.is_pure_function(&item_fn);

        let mut detector_without_def = PurityDetector::new();
        let func_with_unknown = r#"
            fn test() {
                unknown_macro!();
            }
        "#;
        let item_fn2 = syn::parse_str::<ItemFn>(func_with_unknown).unwrap();
        let analysis_without_def = detector_without_def.is_pure_function(&item_fn2);

        assert!(analysis_with_def.confidence < 1.0);
        assert!(analysis_without_def.confidence < 1.0);
    }

    // Tests for spec 160c: custom macro heuristic analysis

    #[test]
    fn test_end_to_end_custom_macro_analysis() {
        use crate::analyzers::macro_definition_collector::*;
        use dashmap::DashMap;
        use std::sync::Arc;

        let code = r#"
            macro_rules! my_logger {
                ($($arg:tt)*) => {
                    eprintln!("[LOG] {}", format!($($arg)*));
                };
            }

            fn process_data(data: &str) {
                my_logger!("Processing: {}", data);
            }
        "#;

        let ast = syn::parse_file(code).unwrap();

        let definitions = Arc::new(DashMap::new());
        collect_definitions(&ast, std::path::Path::new("test.rs"), definitions.clone());

        let mut detector = PurityDetector::with_macro_definitions(definitions);

        let func_code = r#"
            fn process_data(data: &str) {
                my_logger!("Processing: {}", data);
            }
        "#;
        let item_fn = syn::parse_str::<ItemFn>(func_code).unwrap();
        let analysis = detector.is_pure_function(&item_fn);

        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }

    #[test]
    fn test_custom_pure_macro_detection() {
        use crate::analyzers::macro_definition_collector::*;
        use dashmap::DashMap;
        use std::sync::Arc;

        let code = r#"
            macro_rules! make_vec {
                ($($elem:expr),*) => {
                    vec![$($elem),*]
                };
            }

            fn create_list() -> Vec<i32> {
                make_vec![1, 2, 3]
            }
        "#;

        let ast = syn::parse_file(code).unwrap();
        let definitions = Arc::new(DashMap::new());
        collect_definitions(&ast, std::path::Path::new("test.rs"), definitions.clone());

        let mut detector = PurityDetector::with_macro_definitions(definitions);

        let func_code = r#"
            fn create_list() -> Vec<i32> {
                make_vec![1, 2, 3]
            }
        "#;
        let item_fn = syn::parse_str::<ItemFn>(func_code).unwrap();
        let analysis = detector.is_pure_function(&item_fn);

        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_conditional_custom_macro() {
        use crate::analyzers::macro_definition_collector::*;
        use dashmap::DashMap;
        use std::sync::Arc;

        let code = r#"
            macro_rules! debug_check {
                ($val:expr) => {
                    debug_assert!($val > 0);
                    $val
                };
            }

            fn validate(x: i32) -> i32 {
                debug_check!(x)
            }
        "#;

        let ast = syn::parse_file(code).unwrap();
        let definitions = Arc::new(DashMap::new());
        collect_definitions(&ast, std::path::Path::new("test.rs"), definitions.clone());

        let mut detector = PurityDetector::with_macro_definitions(definitions);

        let func_code = r#"
            fn validate(x: i32) -> i32 {
                debug_check!(x)
            }
        "#;
        let item_fn = syn::parse_str::<ItemFn>(func_code).unwrap();
        let analysis = detector.is_pure_function(&item_fn);

        #[cfg(debug_assertions)]
        assert_eq!(analysis.purity_level, PurityLevel::Impure);

        #[cfg(not(debug_assertions))]
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_nested_custom_macros() {
        use crate::analyzers::macro_definition_collector::*;
        use dashmap::DashMap;
        use std::sync::Arc;

        let code = r#"
            macro_rules! log_and_return {
                ($val:expr) => {
                    {
                        println!("Returning: {}", $val);
                        $val
                    }
                };
            }

            fn compute(x: i32) -> i32 {
                log_and_return!(x * 2)
            }
        "#;

        let ast = syn::parse_file(code).unwrap();
        let definitions = Arc::new(DashMap::new());
        collect_definitions(&ast, std::path::Path::new("test.rs"), definitions.clone());

        let mut detector = PurityDetector::with_macro_definitions(definitions);

        let func_code = r#"
            fn compute(x: i32) -> i32 {
                log_and_return!(x * 2)
            }
        "#;
        let item_fn = syn::parse_str::<ItemFn>(func_code).unwrap();
        let analysis = detector.is_pure_function(&item_fn);

        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }

    // Tests for spec 161: refined unsafe block analysis

    #[test]
    fn test_transmute_is_pure_unsafe() {
        let analysis = analyze_function_str(
            r#"
            fn bytes_to_u32(bytes: [u8; 4]) -> u32 {
                unsafe { std::mem::transmute(bytes) }
            }
            "#,
        );

        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
        assert!(analysis.confidence > 0.80);
        assert!(analysis.confidence < 0.90);
    }

    #[test]
    fn test_ffi_call_is_impure() {
        let analysis = analyze_function_str(
            r#"
            fn call_external() {
                extern "C" { fn external_func(); }
                unsafe { external_func(); }
            }
            "#,
        );

        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }

    #[test]
    fn test_pointer_read_is_pure_unsafe() {
        let analysis = analyze_function_str(
            r#"
            fn read_ptr(ptr: *const i32) -> i32 {
                unsafe { ptr.read() }
            }
            "#,
        );

        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
        assert!(analysis.confidence > 0.80);
    }

    #[test]
    fn test_pointer_write_is_impure() {
        let analysis = analyze_function_str(
            r#"
            fn write_ptr(ptr: *mut i32, value: i32) {
                unsafe { ptr.write(value); }
            }
            "#,
        );

        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }

    #[test]
    fn test_pointer_arithmetic_is_pure_unsafe() {
        let analysis = analyze_function_str(
            r#"
            fn offset_ptr(ptr: *const i32, offset: isize) -> *const i32 {
                unsafe { ptr.offset(offset) }
            }
            "#,
        );

        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
        assert!(analysis.confidence > 0.80);
    }

    #[test]
    fn test_mutable_static_is_impure() {
        let analysis = analyze_function_str(
            r#"
            fn access_static() -> i32 {
                static mut COUNTER: i32 = 0;
                unsafe {
                    COUNTER += 1;
                    COUNTER
                }
            }
            "#,
        );

        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }

    // Tests for spec 261: Known pure std function detection

    #[test]
    fn test_is_known_pure_call_option_map() {
        assert!(is_known_pure_call("map", Some("Option")));
        assert!(is_known_pure_call("and_then", Some("Option")));
        assert!(is_known_pure_call("unwrap_or", Some("Option")));
    }

    #[test]
    fn test_is_known_pure_call_result_methods() {
        assert!(is_known_pure_call("map", Some("Result")));
        assert!(is_known_pure_call("map_err", Some("Result")));
        assert!(is_known_pure_call("and_then", Some("Result")));
        assert!(is_known_pure_call("is_ok", Some("Result")));
    }

    #[test]
    fn test_is_known_pure_call_iterator_methods() {
        assert!(is_known_pure_call("map", Some("Iterator")));
        assert!(is_known_pure_call("filter", Some("Iterator")));
        assert!(is_known_pure_call("fold", Some("Iterator")));
        assert!(is_known_pure_call("collect", Some("Iterator")));
        assert!(is_known_pure_call("sum", Some("Iterator")));
    }

    #[test]
    fn test_is_known_pure_call_string_methods() {
        assert!(is_known_pure_call("len", Some("str")));
        assert!(is_known_pure_call("is_empty", Some("str")));
        assert!(is_known_pure_call("contains", Some("str")));
        assert!(is_known_pure_call("trim", Some("str")));
    }

    #[test]
    fn test_is_known_pure_call_vec_methods() {
        assert!(is_known_pure_call("len", Some("Vec")));
        assert!(is_known_pure_call("is_empty", Some("Vec")));
        assert!(is_known_pure_call("iter", Some("Vec")));
        assert!(is_known_pure_call("get", Some("Vec")));
    }

    #[test]
    fn test_is_known_pure_call_clone_default() {
        assert!(is_known_pure_call("clone", Some("Clone")));
        assert!(is_known_pure_call("default", Some("Default")));
    }

    #[test]
    fn test_is_known_pure_method_without_receiver() {
        assert!(is_known_pure_method("map"));
        assert!(is_known_pure_method("filter"));
        assert!(is_known_pure_method("collect"));
        assert!(is_known_pure_method("len"));
        assert!(is_known_pure_method("is_empty"));
        assert!(is_known_pure_method("clone"));
    }

    #[test]
    fn test_is_known_pure_method_unknown() {
        assert!(!is_known_pure_method("println"));
        assert!(!is_known_pure_method("write"));
        assert!(!is_known_pure_method("push"));
        assert!(!is_known_pure_method("insert"));
    }

    #[test]
    fn test_is_known_pure_call_std_mem() {
        assert!(is_known_pure_call("size_of", Some("std::mem")));
        assert!(is_known_pure_call("align_of", Some("std::mem")));
    }
}
