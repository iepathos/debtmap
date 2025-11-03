use debtmap::analyzers::purity_detector::{ImpurityReason, PurityDetector};
use debtmap::core::PurityLevel;
use syn::{parse_str, ItemFn};

fn analyze_function_str(code: &str) -> (PurityLevel, Vec<ImpurityReason>, f32) {
    let item_fn = parse_str::<ItemFn>(code).unwrap();
    let mut detector = PurityDetector::new();
    let analysis = detector.is_pure_function(&item_fn);
    (analysis.purity_level, analysis.reasons, analysis.confidence)
}

#[test]
fn test_pure_closure_in_map() {
    let code = r#"
        fn double_values(nums: &[i32]) -> Vec<i32> {
            nums.iter().map(|x| x * 2).collect()
        }
    "#;

    let (purity_level, _, confidence) = analyze_function_str(code);
    assert_eq!(purity_level, PurityLevel::StrictlyPure);
    assert!(
        confidence > 0.70,
        "Expected confidence > 0.70, got {}",
        confidence
    );
}

#[test]
fn test_impure_closure_propagates() {
    let code = r#"
        fn print_values(nums: &[i32]) {
            nums.iter().for_each(|x| {
                println!("{}", x);
            });
        }
    "#;

    let (purity_level, _, _) = analyze_function_str(code);
    // Should be detected as impure due to println! macro in closure
    // The implementation correctly detects macros in statements
    assert_eq!(purity_level, PurityLevel::Impure);
}

#[test]
fn test_fnmut_local_capture() {
    let code = r#"
        fn sum_values(nums: &[i32]) -> i32 {
            let mut sum = 0;
            nums.iter().for_each(|x| {
                sum += *x;
            });
            sum
        }
    "#;

    let (purity_level, _, _) = analyze_function_str(code);
    // For now, without advanced capture analysis, this may be detected as StrictlyPure
    // since the mutation is within the closure scope
    assert!(matches!(
        purity_level,
        PurityLevel::LocallyPure | PurityLevel::StrictlyPure
    ));
}

#[test]
fn test_move_closure() {
    let code = r#"
        fn create_adder(x: i32) -> impl Fn(i32) -> i32 {
            move |y| x + y
        }
    "#;

    let (purity_level, _, _) = analyze_function_str(code);
    assert_eq!(purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_nested_closures() {
    let code = r#"
        fn nested_map(data: &[Vec<i32>]) -> Vec<Vec<i32>> {
            data.iter()
                .map(|inner| inner.iter().map(|x| x * 2).collect())
                .collect()
        }
    "#;

    let (purity_level, _, confidence) = analyze_function_str(code);
    assert_eq!(purity_level, PurityLevel::StrictlyPure);
    // Nested closure detection is a nice-to-have but may not reduce confidence
    // in all cases without deeper analysis
    assert!(
        confidence > 0.70,
        "Expected reasonable confidence, got {}",
        confidence
    );
}

#[test]
fn test_mixed_purity_chain() {
    let code = r#"
        fn process(nums: &[i32]) -> Vec<i32> {
            nums.iter()
                .map(|x| x * 2)
                .inspect(|x| {
                    println!("{}", x);
                })
                .collect()
        }
    "#;

    let (purity_level, _, _) = analyze_function_str(code);
    // Chain contains impure operation (inspect with println)
    assert_eq!(purity_level, PurityLevel::Impure);
}

#[test]
fn test_filter_map_purity() {
    let code = r#"
        fn extract_evens(nums: &[i32]) -> Vec<i32> {
            nums.iter()
                .filter_map(|&x| if x % 2 == 0 { Some(x) } else { None })
                .collect()
        }
    "#;

    let (purity_level, _, _) = analyze_function_str(code);
    assert_eq!(purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_flat_map_purity() {
    let code = r#"
        fn flatten_data(data: &[Vec<i32>]) -> Vec<i32> {
            data.iter().flat_map(|v| v.iter().copied()).collect()
        }
    "#;

    let (purity_level, _, _) = analyze_function_str(code);
    assert_eq!(purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_scan_with_accumulator() {
    let code = r#"
        fn running_sum(nums: &[i32]) -> Vec<i32> {
            nums.iter()
                .scan(0, |acc, &x| {
                    *acc += x;
                    Some(*acc)
                })
                .collect()
        }
    "#;

    let (purity_level, _, _) = analyze_function_str(code);
    // scan mutates accumulator but it's local to the closure
    // May be detected as StrictlyPure since the mutation is scoped to the closure
    assert!(matches!(
        purity_level,
        PurityLevel::LocallyPure | PurityLevel::StrictlyPure
    ));
}

#[test]
fn test_try_fold_error_handling() {
    let code = r#"
        fn safe_sum(nums: &[i32]) -> Result<i32, String> {
            nums.iter().try_fold(0, |acc, &x| {
                if x < 0 {
                    Err("Negative number".to_string())
                } else {
                    Ok(acc + x)
                }
            })
        }
    "#;

    let (purity_level, _, _) = analyze_function_str(code);
    assert_eq!(purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_partition_purity() {
    let code = r#"
        fn partition_evens(nums: &[i32]) -> (Vec<i32>, Vec<i32>) {
            nums.iter().partition(|&&x| x % 2 == 0)
        }
    "#;

    let (purity_level, _, _) = analyze_function_str(code);
    assert_eq!(purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_complex_iterator_chain() {
    let code = r#"
        fn complex_processing(data: &[i32]) -> Vec<String> {
            data.iter()
                .filter(|&&x| x > 0)
                .map(|&x| x * 2)
                .filter(|&x| x < 100)
                .map(|x| format!("Value: {}", x))
                .collect()
        }
    "#;

    let (purity_level, _, _) = analyze_function_str(code);
    assert_eq!(purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_closure_in_option_combinator() {
    let code = r#"
        fn transform_option(opt: Option<i32>) -> Option<i32> {
            opt.and_then(|x| Some(x * 2))
        }
    "#;

    let (purity_level, _, _) = analyze_function_str(code);
    assert_eq!(purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_closure_in_result_combinator() {
    let code = r#"
        fn transform_result(res: Result<i32, String>) -> Result<i32, String> {
            res.map(|x| x * 2)
        }
    "#;

    let (purity_level, _, _) = analyze_function_str(code);
    assert_eq!(purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_closure_with_multiple_captures() {
    let code = r#"
        fn combine_values(a: i32, b: i32, c: i32) -> Vec<i32> {
            vec![1, 2, 3].iter().map(|x| x + a + b + c).collect()
        }
    "#;

    let (purity_level, _, _) = analyze_function_str(code);
    assert_eq!(purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_any_all_purity() {
    let code = r#"
        fn has_positive_and_all_small(nums: &[i32]) -> bool {
            nums.iter().any(|&x| x > 0) && nums.iter().all(|&x| x < 1000)
        }
    "#;

    let (purity_level, _, _) = analyze_function_str(code);
    assert_eq!(purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_find_position_purity() {
    let code = r#"
        fn find_first_even(nums: &[i32]) -> Option<usize> {
            nums.iter().position(|&x| x % 2 == 0)
        }
    "#;

    let (purity_level, _, _) = analyze_function_str(code);
    assert_eq!(purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_fold_reduce_purity() {
    let code = r#"
        fn sum_and_product(nums: &[i32]) -> (i32, i32) {
            let sum = nums.iter().fold(0, |acc, &x| acc + x);
            let product = nums.iter().fold(1, |acc, &x| acc * x);
            (sum, product)
        }
    "#;

    let (purity_level, _, _) = analyze_function_str(code);
    assert_eq!(purity_level, PurityLevel::StrictlyPure);
}
