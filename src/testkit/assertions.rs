//! Assertion macros for testing debtmap operations.
//!
//! This module extends stillwater's validation assertions with additional
//! macros for working with `Result` types and error content checking.
//!
//! # Stillwater Provides (for Validation)
//!
//! - `assert_success!` - Assert a Validation is successful
//! - `assert_failure!` - Assert a Validation has failures
//! - `assert_validation_errors!` - Compare validation error vectors
//!
//! # Debtmap Extends (for Result)
//!
//! - [`crate::assert_result_ok!`] - Assert Result is Ok and extract value
//! - [`crate::assert_result_err!`] - Assert Result is Err and extract error
//! - [`crate::assert_contains_error!`] - Assert error message contains pattern
//! - [`crate::assert_validation_error_count!`] - Assert validation has N errors
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::testkit::{assert_result_ok, assert_result_err, assert_contains_error};
//!
//! // Assert success and get value
//! let result: Result<i32, String> = Ok(42);
//! let value = assert_result_ok!(result);
//! assert_eq!(value, 42);
//!
//! // Assert failure and get error
//! let result: Result<i32, String> = Err("oops".to_string());
//! let err = assert_result_err!(result);
//! assert_eq!(err, "oops");
//!
//! // Assert error contains pattern
//! let result: Result<i32, String> = Err("File not found: test.rs".to_string());
//! assert_contains_error!(result, "not found");
//! ```

/// Assert that a Result is Ok and extract the value.
///
/// If the Result is Err, panics with a message showing the error.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::testkit::assert_result_ok;
///
/// let result: Result<i32, &str> = Ok(42);
/// let value = assert_result_ok!(result);
/// assert_eq!(value, 42);
/// ```
#[macro_export]
macro_rules! assert_result_ok {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(e) => panic!(
                "Expected Ok, got Err: {:?}\n  at {}:{}:{}",
                e,
                file!(),
                line!(),
                column!()
            ),
        }
    };
    ($result:expr, $($msg:tt)+) => {
        match $result {
            Ok(value) => value,
            Err(e) => panic!(
                "{}: Expected Ok, got Err: {:?}\n  at {}:{}:{}",
                format!($($msg)+),
                e,
                file!(),
                line!(),
                column!()
            ),
        }
    };
}

/// Assert that a Result is Err and extract the error.
///
/// If the Result is Ok, panics with a message showing the value.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::testkit::assert_result_err;
///
/// let result: Result<i32, String> = Err("error".to_string());
/// let err = assert_result_err!(result);
/// assert_eq!(err, "error");
/// ```
#[macro_export]
macro_rules! assert_result_err {
    ($result:expr) => {
        match $result {
            Ok(value) => panic!(
                "Expected Err, got Ok: {:?}\n  at {}:{}:{}",
                value,
                file!(),
                line!(),
                column!()
            ),
            Err(e) => e,
        }
    };
    ($result:expr, $($msg:tt)+) => {
        match $result {
            Ok(value) => panic!(
                "{}: Expected Err, got Ok: {:?}\n  at {}:{}:{}",
                format!($($msg)+),
                value,
                file!(),
                line!(),
                column!()
            ),
            Err(e) => e,
        }
    };
}

/// Assert that an error message contains a specific pattern.
///
/// This macro first asserts the Result is Err, then checks if the
/// error's Display representation contains the pattern.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::testkit::assert_contains_error;
///
/// let result: Result<i32, String> = Err("File not found: test.rs".to_string());
/// assert_contains_error!(result, "not found");
/// ```
#[macro_export]
macro_rules! assert_contains_error {
    ($result:expr, $pattern:expr) => {{
        let err = $crate::assert_result_err!($result);
        let err_str = err.to_string();
        assert!(
            err_str.contains($pattern),
            "Error '{}' does not contain '{}'\n  at {}:{}:{}",
            err_str,
            $pattern,
            file!(),
            line!(),
            column!()
        );
        err
    }};
}

/// Assert that a Validation has a specific number of errors.
///
/// Returns the error vector if the assertion passes.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::testkit::assert_validation_error_count;
/// use stillwater::Validation;
///
/// let validation: Validation<i32, Vec<String>> = Validation::Failure(vec![
///     "error 1".to_string(),
///     "error 2".to_string(),
/// ]);
/// let errors = assert_validation_error_count!(validation, 2);
/// ```
#[macro_export]
macro_rules! assert_validation_error_count {
    ($validation:expr, $count:expr) => {
        match $validation {
            stillwater::Validation::Success(value) => panic!(
                "Expected {} validation errors, got success with: {:?}\n  at {}:{}:{}",
                $count,
                value,
                file!(),
                line!(),
                column!()
            ),
            stillwater::Validation::Failure(errors) => {
                assert_eq!(
                    errors.len(),
                    $count,
                    "Expected {} errors, got {}: {:?}\n  at {}:{}:{}",
                    $count,
                    errors.len(),
                    errors,
                    file!(),
                    line!(),
                    column!()
                );
                errors
            }
        }
    };
}

/// Assert that a Validation is successful and extract the value.
///
/// This is a convenience wrapper similar to stillwater's assert_success!
/// but with better error messages.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::testkit::assert_validation_ok;
/// use stillwater::Validation;
///
/// let validation: Validation<i32, Vec<String>> = Validation::Success(42);
/// let value = assert_validation_ok!(validation);
/// assert_eq!(value, 42);
/// ```
#[macro_export]
macro_rules! assert_validation_ok {
    ($validation:expr) => {
        match $validation {
            stillwater::Validation::Success(value) => value,
            stillwater::Validation::Failure(errors) => panic!(
                "Expected validation success, got {} errors: {:?}\n  at {}:{}:{}",
                errors.len(),
                errors,
                file!(),
                line!(),
                column!()
            ),
        }
    };
}

/// Assert that a Validation has failures and extract the errors.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::testkit::assert_validation_err;
/// use stillwater::Validation;
///
/// let validation: Validation<i32, Vec<String>> = Validation::Failure(vec!["error".to_string()]);
/// let errors = assert_validation_err!(validation);
/// assert_eq!(errors.len(), 1);
/// ```
#[macro_export]
macro_rules! assert_validation_err {
    ($validation:expr) => {
        match $validation {
            stillwater::Validation::Success(value) => panic!(
                "Expected validation failure, got success: {:?}\n  at {}:{}:{}",
                value,
                file!(),
                line!(),
                column!()
            ),
            stillwater::Validation::Failure(errors) => errors,
        }
    };
}

// Note: Macros are exported at crate root via #[macro_export]
// They can be used as `debtmap::assert_result_ok!` or via `use debtmap::*`

#[cfg(test)]
mod tests {

    #[test]
    fn test_assert_result_ok_success() {
        let result: Result<i32, String> = Ok(42);
        let value = assert_result_ok!(result);
        assert_eq!(value, 42);
    }

    #[test]
    #[should_panic(expected = "Expected Ok, got Err")]
    fn test_assert_result_ok_failure() {
        let result: Result<i32, String> = Err("error".to_string());
        let _ = assert_result_ok!(result);
    }

    #[test]
    fn test_assert_result_ok_with_message() {
        let result: Result<i32, String> = Ok(42);
        let value = assert_result_ok!(result, "custom message");
        assert_eq!(value, 42);
    }

    #[test]
    fn test_assert_result_err_success() {
        let result: Result<i32, String> = Err("error".to_string());
        let err = assert_result_err!(result);
        assert_eq!(err, "error");
    }

    #[test]
    #[should_panic(expected = "Expected Err, got Ok")]
    fn test_assert_result_err_failure() {
        let result: Result<i32, String> = Ok(42);
        let _ = assert_result_err!(result);
    }

    #[test]
    fn test_assert_contains_error_success() {
        let result: Result<i32, String> = Err("File not found: test.rs".to_string());
        let _ = assert_contains_error!(result, "not found");
    }

    #[test]
    #[should_panic(expected = "does not contain")]
    fn test_assert_contains_error_pattern_mismatch() {
        let result: Result<i32, String> = Err("Permission denied".to_string());
        assert_contains_error!(result, "not found");
    }

    #[test]
    fn test_assert_validation_error_count() {
        use stillwater::Validation;

        let validation: Validation<i32, Vec<String>> =
            Validation::Failure(vec!["e1".to_string(), "e2".to_string()]);
        let errors = assert_validation_error_count!(validation, 2);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    #[should_panic(expected = "Expected 3 errors")]
    fn test_assert_validation_error_count_mismatch() {
        use stillwater::Validation;

        let validation: Validation<i32, Vec<String>> =
            Validation::Failure(vec!["e1".to_string(), "e2".to_string()]);
        let _ = assert_validation_error_count!(validation, 3);
    }

    #[test]
    fn test_assert_validation_ok() {
        use stillwater::Validation;

        let validation: Validation<i32, Vec<String>> = Validation::Success(42);
        let value = assert_validation_ok!(validation);
        assert_eq!(value, 42);
    }

    #[test]
    #[should_panic(expected = "Expected validation success")]
    fn test_assert_validation_ok_failure() {
        use stillwater::Validation;

        let validation: Validation<i32, Vec<String>> =
            Validation::Failure(vec!["error".to_string()]);
        let _ = assert_validation_ok!(validation);
    }

    #[test]
    fn test_assert_validation_err() {
        use stillwater::Validation;

        let validation: Validation<i32, Vec<String>> =
            Validation::Failure(vec!["error".to_string()]);
        let errors = assert_validation_err!(validation);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    #[should_panic(expected = "Expected validation failure")]
    fn test_assert_validation_err_success() {
        use stillwater::Validation;

        let validation: Validation<i32, Vec<String>> = Validation::Success(42);
        let _ = assert_validation_err!(validation);
    }
}
