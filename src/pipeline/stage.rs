//! Pipeline stage abstractions for composable analysis workflows.
//!
//! This module defines the core `Stage` trait that enables type-safe composition
//! of analysis operations. Stages can be pure transformations or effects that
//! perform I/O.

use crate::errors::AnalysisError;
use std::marker::PhantomData;

/// A pipeline stage that transforms data.
///
/// Stages are the building blocks of analysis pipelines. Each stage has:
/// - An input type (what data it expects)
/// - An output type (what data it produces)
/// - An error type (how it can fail)
///
/// # Type Safety
///
/// The type system ensures stages can only be composed when their types align:
/// ```rust,ignore
/// stage1  // Input: A, Output: B
///   .then(stage2)  // Input: B, Output: C - OK!
///   .then(stage3)  // Input: D, Output: E - Compile error!
/// ```
pub trait Stage {
    type Input;
    type Output;
    type Error;

    /// Execute this stage with the given input.
    fn execute(&self, input: Self::Input) -> Result<Self::Output, Self::Error>;

    /// Get the stage name for progress reporting.
    fn name(&self) -> &str;
}

/// A pure stage that performs no I/O.
///
/// Pure stages are simple transformations of data. They:
/// - Have no side effects
/// - Are deterministic (same input → same output)
/// - Can be easily tested
/// - Can be run in parallel
///
/// # Example
///
/// ```rust,ignore
/// let stage = PureStage::new("Calculate Metrics", |ast| {
///     calculate_complexity(&ast)
/// });
/// ```
pub struct PureStage<F, I, O> {
    name: String,
    func: F,
    _phantom: PhantomData<(I, O)>,
}

impl<F, I, O> PureStage<F, I, O>
where
    F: Fn(I) -> O,
{
    /// Create a new pure stage with a name and transformation function.
    pub fn new(name: impl Into<String>, func: F) -> Self {
        Self {
            name: name.into(),
            func,
            _phantom: PhantomData,
        }
    }
}

impl<F, I, O> Stage for PureStage<F, I, O>
where
    F: Fn(I) -> O,
{
    type Input = I;
    type Output = O;
    type Error = std::convert::Infallible;

    fn execute(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
        Ok((self.func)(input))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// A fallible stage that can fail with an error.
///
/// Fallible stages perform computations that may fail, but don't perform I/O.
/// They're useful for validation, parsing, or other operations that can error.
///
/// # Example
///
/// ```rust,ignore
/// let stage = FallibleStage::new("Parse AST", |source| {
///     parse_source(&source)
/// });
/// ```
pub struct FallibleStage<F, I, O, E> {
    name: String,
    func: F,
    _phantom: PhantomData<(I, O, E)>,
}

impl<F, I, O, E> FallibleStage<F, I, O, E>
where
    F: Fn(I) -> Result<O, E>,
{
    /// Create a new fallible stage with a name and function.
    pub fn new(name: impl Into<String>, func: F) -> Self {
        Self {
            name: name.into(),
            func,
            _phantom: PhantomData,
        }
    }
}

impl<F, I, O, E> Stage for FallibleStage<F, I, O, E>
where
    F: Fn(I) -> Result<O, E>,
{
    type Input = I;
    type Output = O;
    type Error = E;

    fn execute(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
        (self.func)(input)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Type-erased stage for dynamic dispatch.
///
/// This trait allows stages of different types to be stored in collections.
/// It's used internally by the pipeline builder.
pub(crate) trait AnyStage: Send + Sync {
    fn execute_any(
        &self,
        input: Box<dyn std::any::Any>,
    ) -> Result<Box<dyn std::any::Any>, AnalysisError>;
    fn name(&self) -> &str;
}

impl<S> AnyStage for S
where
    S: Stage + Send + Sync,
    S::Input: 'static,
    S::Output: 'static,
    S::Error: Into<AnalysisError>,
{
    fn execute_any(
        &self,
        input: Box<dyn std::any::Any>,
    ) -> Result<Box<dyn std::any::Any>, AnalysisError> {
        let typed_input = input
            .downcast::<S::Input>()
            .map_err(|_| AnalysisError::other("Type mismatch in pipeline stage input"))?;

        let output = self.execute(*typed_input).map_err(|e| e.into())?;
        Ok(Box::new(output))
    }

    fn name(&self) -> &str {
        Stage::name(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pure_stage_execution() {
        let stage = PureStage::new("Double", |x: i32| x * 2);
        let result = stage.execute(21).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_fallible_stage_success() {
        let stage = FallibleStage::new("Parse", |s: String| {
            s.parse::<i32>().map_err(|_| "Parse error")
        });
        let result = stage.execute("42".to_string()).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_fallible_stage_failure() {
        let stage = FallibleStage::new("Parse", |s: String| {
            s.parse::<i32>().map_err(|_| "Parse error")
        });
        let result = stage.execute("not a number".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_stage_name() {
        let stage = PureStage::new("Test Stage", |x: i32| x);
        assert_eq!(Stage::name(&stage), "Test Stage");
    }
}
