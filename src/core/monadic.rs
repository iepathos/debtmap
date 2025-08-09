use anyhow::{Context as AnyhowContext, Result};
use std::fmt::Display;

/// Extension trait for monadic Result operations
pub trait ResultExt<T> {
    /// Chain async-like operations
    fn and_then_async<F, U>(self, f: F) -> Result<U>
    where
        F: FnOnce(T) -> Result<U>;

    /// Provide alternative on error
    fn or_else_with<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>;

    /// Add context to error
    fn map_err_context<C>(self, context: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static;

    /// Map the Ok value
    fn map_ok<F, U>(self, f: F) -> Result<U>
    where
        F: FnOnce(T) -> U;

    /// Tap into the Ok value without consuming
    fn tap<F>(self, f: F) -> Result<T>
    where
        F: FnOnce(&T);

    /// Tap into error without consuming
    fn tap_err<F>(self, f: F) -> Result<T>
    where
        F: FnOnce(&anyhow::Error);
}

impl<T> ResultExt<T> for Result<T> {
    fn and_then_async<F, U>(self, f: F) -> Result<U>
    where
        F: FnOnce(T) -> Result<U>,
    {
        self.and_then(f)
    }

    fn or_else_with<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>,
    {
        self.or_else(|_| f())
    }

    fn map_err_context<C>(self, context: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static,
    {
        self.context(context)
    }

    fn map_ok<F, U>(self, f: F) -> Result<U>
    where
        F: FnOnce(T) -> U,
    {
        self.map(f)
    }

    fn tap<F>(self, f: F) -> Result<T>
    where
        F: FnOnce(&T),
    {
        if let Ok(ref value) = self {
            f(value);
        }
        self
    }

    fn tap_err<F>(self, f: F) -> Result<T>
    where
        F: FnOnce(&anyhow::Error),
    {
        if let Err(ref e) = self {
            f(e);
        }
        self
    }
}

/// Extension trait for Option monadic operations
pub trait OptionExt<T> {
    /// Convert None to an error
    fn ok_or_error<E>(self, error: E) -> Result<T>
    where
        E: Into<anyhow::Error>;

    /// Chain operations on Some
    fn and_then_some<F, U>(self, f: F) -> Option<U>
    where
        F: FnOnce(T) -> Option<U>;

    /// Provide alternative on None
    fn or_else_some<F>(self, f: F) -> Option<T>
    where
        F: FnOnce() -> Option<T>;

    /// Filter with predicate
    fn filter_some<F>(self, predicate: F) -> Option<T>
    where
        F: FnOnce(&T) -> bool;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_error<E>(self, error: E) -> Result<T>
    where
        E: Into<anyhow::Error>,
    {
        self.ok_or_else(|| error.into())
    }

    fn and_then_some<F, U>(self, f: F) -> Option<U>
    where
        F: FnOnce(T) -> Option<U>,
    {
        self.and_then(f)
    }

    fn or_else_some<F>(self, f: F) -> Option<T>
    where
        F: FnOnce() -> Option<T>,
    {
        self.or_else(f)
    }

    fn filter_some<F>(self, predicate: F) -> Option<T>
    where
        F: FnOnce(&T) -> bool,
    {
        self.filter(predicate)
    }
}

/// Applicative functor for parallel computations
pub struct Applicative<T> {
    values: Vec<T>,
}

impl<T> Applicative<T> {
    /// Create new applicative
    pub fn new(values: Vec<T>) -> Self {
        Self { values }
    }

    /// Apply function to all values
    pub fn apply<F, U>(self, f: F) -> Applicative<U>
    where
        F: Fn(T) -> U,
    {
        Applicative::new(self.values.into_iter().map(f).collect())
    }

    /// Apply function that may fail
    pub fn apply_result<F, U>(self, f: F) -> Result<Applicative<U>>
    where
        F: Fn(T) -> Result<U>,
    {
        let results: Result<Vec<U>> = self.values.into_iter().map(f).collect();
        results.map(Applicative::new)
    }

    /// Extract values
    pub fn unwrap(self) -> Vec<T> {
        self.values
    }
}

/// Kleisli composition for Result types
pub fn compose_results<A, B, C, F, G>(f: F, g: G) -> impl Fn(A) -> Result<C>
where
    F: Fn(A) -> Result<B>,
    G: Fn(B) -> Result<C>,
{
    move |a| f(a).and_then(&g)
}

/// Lift a pure function into Result context
pub fn lift_result<T, U, F>(f: F) -> impl Fn(T) -> Result<U>
where
    F: Fn(T) -> U,
{
    move |t| Ok(f(t))
}

/// Sequence a vector of Results into a Result of vector
pub fn sequence_results<T>(results: Vec<Result<T>>) -> Result<Vec<T>> {
    results.into_iter().collect()
}

/// Traverse with a function that returns Result
pub fn traverse_results<T, U, F>(values: Vec<T>, f: F) -> Result<Vec<U>>
where
    F: Fn(T) -> Result<U>,
{
    values.into_iter().map(f).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn test_result_ext() {
        let result: Result<i32> = Ok(42);

        let chained = result
            .map_ok(|x| x * 2)
            .and_then_async(|x| Ok(x + 1))
            .tap(|x| println!("Value: {x}"));

        assert_eq!(chained.unwrap(), 85);
    }

    #[test]
    fn test_option_ext() {
        let value = Some(42);

        let result = value
            .and_then_some(|x| Some(x * 2))
            .filter_some(|&x| x > 50)
            .ok_or_error(anyhow!("Value too small"));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 84);
    }

    #[test]
    fn test_kleisli_composition() {
        let add_one = |x: i32| -> Result<i32> { Ok(x + 1) };
        let double = |x: i32| -> Result<i32> { Ok(x * 2) };

        let composed = compose_results(add_one, double);
        assert_eq!(composed(5).unwrap(), 12); // (5 + 1) * 2
    }

    #[test]
    fn test_sequence() {
        let results = vec![Ok(1), Ok(2), Ok(3)];
        let sequenced = sequence_results(results);
        assert_eq!(sequenced.unwrap(), vec![1, 2, 3]);

        let with_error = vec![Ok(1), Err(anyhow!("error")), Ok(3)];
        let sequenced_err = sequence_results(with_error);
        assert!(sequenced_err.is_err());
    }
}
