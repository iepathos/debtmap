//! Extension traits for Option types to simplify error handling.
//!
//! This module provides ergonomic methods for converting `Option<T>` values
//! into `Result<T, AnalysisError>` with contextual error messages.
//!
//! # Design Philosophy
//!
//! "Errors Should Tell Stories" - When something fails, explain what was expected
//! and what was actually found. These extension methods make it easy to add
//! context to error messages without verbose boilerplate.
//!
//! # Example
//!
//! ```rust
//! use debtmap::utils::option_ext::OptionExt;
//!
//! fn get_first_item<T: Clone>(items: &[T]) -> Result<T, debtmap::errors::AnalysisError> {
//!     items.first().cloned().ok_or_validation("Expected non-empty list")
//! }
//! ```

use crate::errors::AnalysisError;

/// Extension trait for `Option<T>` to provide ergonomic error conversion.
///
/// This trait adds methods to convert `Option<T>` to `Result<T, AnalysisError>`
/// with appropriate error context.
pub trait OptionExt<T> {
    /// Convert to Result with a validation error message.
    ///
    /// Use this when the None case indicates invalid data or a violated invariant.
    ///
    /// # Example
    ///
    /// ```rust
    /// use debtmap::utils::option_ext::OptionExt;
    ///
    /// let items: Vec<i32> = vec![];
    /// let result = items.first().ok_or_validation("List must not be empty");
    /// assert!(result.is_err());
    /// ```
    fn ok_or_validation(self, msg: impl Into<String>) -> Result<T, AnalysisError>;

    /// Convert to Result with a lazy validation error message.
    ///
    /// Use this when building the error message is expensive and should only
    /// happen on failure.
    ///
    /// # Example
    ///
    /// ```rust
    /// use debtmap::utils::option_ext::OptionExt;
    ///
    /// let map = std::collections::HashMap::<String, i32>::new();
    /// let key = "missing";
    /// let result = map.get(key).ok_or_validation_with(|| {
    ///     format!("Key '{}' not found in map with {} entries", key, map.len())
    /// });
    /// assert!(result.is_err());
    /// ```
    fn ok_or_validation_with<F>(self, f: F) -> Result<T, AnalysisError>
    where
        F: FnOnce() -> String;

    /// Convert to Result with a context error message.
    ///
    /// Use this for internal logic errors that shouldn't happen but need handling.
    ///
    /// # Example
    ///
    /// ```rust
    /// use debtmap::utils::option_ext::OptionExt;
    ///
    /// fn get_first_char(s: &str) -> Result<char, debtmap::errors::AnalysisError> {
    ///     s.chars().next().ok_or_context("String was unexpectedly empty")
    /// }
    /// ```
    fn ok_or_context(self, msg: impl Into<String>) -> Result<T, AnalysisError>;

    /// Convert to Result with a lazy context error message.
    ///
    /// Use this when building the error message is expensive.
    fn ok_or_context_with<F>(self, f: F) -> Result<T, AnalysisError>
    where
        F: FnOnce() -> String;

    /// Convert to Result with an analysis error message.
    ///
    /// Use this for analysis algorithm failures.
    fn ok_or_analysis(self, msg: impl Into<String>) -> Result<T, AnalysisError>;

    /// Convert to Result with a lazy analysis error message.
    fn ok_or_analysis_with<F>(self, f: F) -> Result<T, AnalysisError>
    where
        F: FnOnce() -> String;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_validation(self, msg: impl Into<String>) -> Result<T, AnalysisError> {
        self.ok_or_else(|| AnalysisError::validation(msg.into()))
    }

    fn ok_or_validation_with<F>(self, f: F) -> Result<T, AnalysisError>
    where
        F: FnOnce() -> String,
    {
        self.ok_or_else(|| AnalysisError::validation(f()))
    }

    fn ok_or_context(self, msg: impl Into<String>) -> Result<T, AnalysisError> {
        self.ok_or_else(|| AnalysisError::other(msg.into()))
    }

    fn ok_or_context_with<F>(self, f: F) -> Result<T, AnalysisError>
    where
        F: FnOnce() -> String,
    {
        self.ok_or_else(|| AnalysisError::other(f()))
    }

    fn ok_or_analysis(self, msg: impl Into<String>) -> Result<T, AnalysisError> {
        self.ok_or_else(|| AnalysisError::analysis(msg.into()))
    }

    fn ok_or_analysis_with<F>(self, f: F) -> Result<T, AnalysisError>
    where
        F: FnOnce() -> String,
    {
        self.ok_or_else(|| AnalysisError::analysis(f()))
    }
}

/// Extension trait for Result types to add context to errors.
pub trait ResultExt<T, E> {
    /// Add context to an error, wrapping it in an AnalysisError.
    ///
    /// This is similar to anyhow's `context()` but produces an `AnalysisError`.
    fn with_context<F>(self, f: F) -> Result<T, AnalysisError>
    where
        F: FnOnce() -> String;
}

impl<T, E: std::fmt::Display> ResultExt<T, E> for Result<T, E> {
    fn with_context<F>(self, f: F) -> Result<T, AnalysisError>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| AnalysisError::other(format!("{}: {}", f(), e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ok_or_validation_some() {
        let opt = Some(42);
        let result = opt.ok_or_validation("Expected value");
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_ok_or_validation_none() {
        let opt: Option<i32> = None;
        let result = opt.ok_or_validation("Expected value");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Expected value"));
        assert_eq!(err.category(), "Validation");
    }

    #[test]
    fn test_ok_or_validation_with_lazy() {
        let opt: Option<i32> = None;
        let expensive_computation_called = std::cell::Cell::new(false);
        let result = opt.ok_or_validation_with(|| {
            expensive_computation_called.set(true);
            "Computed error message".to_string()
        });
        assert!(result.is_err());
        assert!(expensive_computation_called.get());
    }

    #[test]
    fn test_ok_or_validation_with_not_called_on_some() {
        let opt = Some(42);
        let expensive_computation_called = std::cell::Cell::new(false);
        let result = opt.ok_or_validation_with(|| {
            expensive_computation_called.set(true);
            "Should not be computed".to_string()
        });
        assert_eq!(result.unwrap(), 42);
        assert!(!expensive_computation_called.get());
    }

    #[test]
    fn test_ok_or_context() {
        let opt: Option<i32> = None;
        let result = opt.ok_or_context("Internal error");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Internal error"));
        // Other category (catch-all)
        assert_eq!(err.category(), "Error");
    }

    #[test]
    fn test_ok_or_analysis() {
        let opt: Option<i32> = None;
        let result = opt.ok_or_analysis("Algorithm failed");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Algorithm failed"));
        assert_eq!(err.category(), "Analysis");
    }

    #[test]
    fn test_result_with_context() {
        let result: Result<i32, &str> = Err("original error");
        let wrapped = result.with_context(|| "While processing data".to_string());
        assert!(wrapped.is_err());
        let err = wrapped.unwrap_err();
        assert!(err.to_string().contains("While processing data"));
        assert!(err.to_string().contains("original error"));
    }

    #[test]
    fn test_result_with_context_ok() {
        let result: Result<i32, &str> = Ok(42);
        let wrapped = result.with_context(|| "Should not be called".to_string());
        assert_eq!(wrapped.unwrap(), 42);
    }
}
