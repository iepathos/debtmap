use anyhow::{anyhow, Result};
use debtmap::core::monadic::ResultExt;

#[test]
fn test_map_err_context_adds_context_on_error() {
    let result: Result<i32> = Err(anyhow!("original error"));
    let with_context = result.map_err_context("additional context");

    assert!(with_context.is_err());
    let error_string = format!("{:?}", with_context.unwrap_err());
    assert!(error_string.contains("additional context"));
    assert!(error_string.contains("original error"));
}

#[test]
fn test_map_err_context_preserves_ok_value() {
    let result: Result<i32> = Ok(42);
    let with_context = result.map_err_context("this context won't be used");

    assert!(with_context.is_ok());
    assert_eq!(with_context.unwrap(), 42);
}

#[test]
fn test_map_err_context_with_formatted_message() {
    let value = 10;
    let result: Result<i32> = Err(anyhow!("failed"));
    let with_context = result.map_err_context(format!("Processing value {value}"));

    assert!(with_context.is_err());
    let error_string = format!("{:?}", with_context.unwrap_err());
    assert!(error_string.contains("Processing value 10"));
}

#[test]
fn test_and_then_async_chains_operations() {
    let result: Result<i32> = Ok(5);
    let chained = result
        .and_then_async(|x| Ok(x * 2))
        .and_then_async(|x| Ok(x + 3));

    assert!(chained.is_ok());
    assert_eq!(chained.unwrap(), 13); // (5 * 2) + 3
}

#[test]
fn test_and_then_async_stops_on_first_error() {
    let result: Result<i32> = Ok(5);
    let chained = result
        .and_then_async(|_| Err(anyhow!("first error")))
        .and_then_async(|x: i32| Ok(x + 3)); // This won't execute

    assert!(chained.is_err());
    let error_string = format!("{:?}", chained.unwrap_err());
    assert!(error_string.contains("first error"));
}

#[test]
fn test_or_else_with_provides_alternative() {
    let result: Result<i32> = Err(anyhow!("original error"));
    let with_alternative = result.or_else_with(|| Ok(100));

    assert!(with_alternative.is_ok());
    assert_eq!(with_alternative.unwrap(), 100);
}

#[test]
fn test_or_else_with_preserves_ok() {
    let result: Result<i32> = Ok(42);
    let with_alternative = result.or_else_with(|| Ok(100));

    assert!(with_alternative.is_ok());
    assert_eq!(with_alternative.unwrap(), 42); // Original value preserved
}

#[test]
fn test_or_else_with_can_also_fail() {
    let result: Result<i32> = Err(anyhow!("original error"));
    let with_alternative = result.or_else_with(|| Err(anyhow!("alternative error")));

    assert!(with_alternative.is_err());
    let error_string = format!("{:?}", with_alternative.unwrap_err());
    assert!(error_string.contains("alternative error"));
}

#[test]
fn test_map_ok_transforms_value() {
    let result: Result<i32> = Ok(10);
    let mapped = result.map_ok(|x| x.to_string());

    assert!(mapped.is_ok());
    assert_eq!(mapped.unwrap(), "10");
}

#[test]
fn test_map_ok_preserves_error() {
    let result: Result<i32> = Err(anyhow!("error"));
    let mapped = result.map_ok(|x| x.to_string());

    assert!(mapped.is_err());
    let error_string = format!("{:?}", mapped.unwrap_err());
    assert!(error_string.contains("error"));
}

#[test]
fn test_tap_inspects_ok_value() {
    let mut side_effect = 0;
    let result: Result<i32> = Ok(42);

    let tapped = result.tap(|&x| {
        side_effect = x * 2;
    });

    assert!(tapped.is_ok());
    assert_eq!(tapped.unwrap(), 42); // Original value unchanged
    assert_eq!(side_effect, 84); // Side effect occurred
}

#[test]
fn test_tap_skips_on_error() {
    let mut side_effect = 0;
    let result: Result<i32> = Err(anyhow!("error"));

    let tapped = result.tap(|&x| {
        side_effect = x * 2;
    });

    assert!(tapped.is_err());
    assert_eq!(side_effect, 0); // Side effect didn't occur
}

#[test]
fn test_tap_err_inspects_error() {
    let mut error_message = String::new();
    let result: Result<i32> = Err(anyhow!("test error"));

    let tapped = result.tap_err(|e| {
        error_message = format!("{e}");
    });

    assert!(tapped.is_err());
    assert!(error_message.contains("test error")); // Side effect occurred
}

#[test]
fn test_tap_err_skips_on_ok() {
    let mut error_message = String::new();
    let result: Result<i32> = Ok(42);

    let tapped = result.tap_err(|e| {
        error_message = format!("{e}");
    });

    assert!(tapped.is_ok());
    assert_eq!(tapped.unwrap(), 42);
    assert_eq!(error_message, ""); // Side effect didn't occur
}

#[test]
fn test_chaining_multiple_extensions() {
    let result: Result<i32> = Ok(5);

    let final_result = result
        .map_ok(|x| x * 2)
        .and_then_async(|x| Ok(x + 1))
        .tap(|&x| assert_eq!(x, 11))
        .map_err_context("Failed to process number");

    assert!(final_result.is_ok());
    assert_eq!(final_result.unwrap(), 11);
}

#[test]
fn test_error_handling_chain() {
    let result: Result<i32> = Err(anyhow!("initial error"));

    let final_result = result
        .map_err_context("step 1 failed")
        .or_else_with(|| Ok(10))
        .and_then_async(|x| Ok(x * 2))
        .tap(|&x| assert_eq!(x, 20));

    assert!(final_result.is_ok());
    assert_eq!(final_result.unwrap(), 20);
}
