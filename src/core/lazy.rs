use std::marker::PhantomData;

/// A lazy evaluation pipeline for composable transformations
pub struct LazyPipeline<T, I>
where
    I: Iterator<Item = T>,
{
    source: I,
    _phantom: PhantomData<T>,
}

impl<T, I> LazyPipeline<T, I>
where
    I: Iterator<Item = T>,
{
    /// Create a new lazy pipeline from an iterator
    pub fn new(source: I) -> Self {
        Self {
            source,
            _phantom: PhantomData,
        }
    }

    /// Map transformation
    pub fn map<U, F>(self, f: F) -> LazyPipeline<U, impl Iterator<Item = U>>
    where
        F: FnMut(T) -> U,
    {
        LazyPipeline::new(self.source.map(f))
    }

    /// Filter transformation
    pub fn filter<F>(self, predicate: F) -> LazyPipeline<T, impl Iterator<Item = T>>
    where
        F: FnMut(&T) -> bool,
    {
        LazyPipeline::new(self.source.filter(predicate))
    }

    /// Flat map transformation
    pub fn flat_map<U, F, II>(self, f: F) -> LazyPipeline<U, impl Iterator<Item = U>>
    where
        F: FnMut(T) -> II,
        II: IntoIterator<Item = U>,
    {
        LazyPipeline::new(self.source.flat_map(f))
    }

    /// Take first n elements
    pub fn take(self, n: usize) -> LazyPipeline<T, impl Iterator<Item = T>> {
        LazyPipeline::new(self.source.take(n))
    }

    /// Skip first n elements
    pub fn skip(self, n: usize) -> LazyPipeline<T, impl Iterator<Item = T>> {
        LazyPipeline::new(self.source.skip(n))
    }

    /// Evaluate the pipeline and collect results
    pub fn collect<C: FromIterator<T>>(self) -> C {
        self.source.collect()
    }

    /// Evaluate the pipeline and fold into a single value
    pub fn fold<B, F>(self, init: B, f: F) -> B
    where
        F: FnMut(B, T) -> B,
    {
        self.source.fold(init, f)
    }

    /// Check if any element matches predicate
    pub fn any<F>(mut self, predicate: F) -> bool
    where
        F: FnMut(T) -> bool,
    {
        self.source.any(predicate)
    }

    /// Check if all elements match predicate
    pub fn all<F>(mut self, predicate: F) -> bool
    where
        F: FnMut(T) -> bool,
    {
        self.source.all(predicate)
    }

    /// Count elements
    pub fn count(self) -> usize {
        self.source.count()
    }
}

/// Builder for composing analysis transformations
pub struct TransformationPipeline<T> {
    transformations: Vec<fn(T) -> T>,
}

impl<T> TransformationPipeline<T> {
    /// Create a new transformation pipeline
    pub fn new() -> Self {
        Self {
            transformations: Vec::new(),
        }
    }

    /// Add a transformation to the pipeline
    pub fn add_transformation(mut self, f: fn(T) -> T) -> Self {
        self.transformations.push(f);
        self
    }

    /// Apply all transformations to a value
    pub fn apply(&self, value: T) -> T {
        self.transformations
            .iter()
            .fold(value, |acc, transform| transform(acc))
    }

    /// Apply transformations to an iterator of values
    pub fn apply_all<I>(&self, values: I) -> Vec<T>
    where
        I: IntoIterator<Item = T>,
    {
        values.into_iter().map(|value| self.apply(value)).collect()
    }
}

/// Lazy value evaluation
pub struct Lazy<T> {
    value: Option<T>,
    generator: Option<Box<dyn FnOnce() -> T>>,
}

impl<T> Lazy<T> {
    /// Create a new lazy value
    pub fn new<F>(generator: F) -> Self
    where
        F: FnOnce() -> T + 'static,
    {
        Self {
            value: None,
            generator: Some(Box::new(generator)),
        }
    }

    /// Force evaluation and get the value
    pub fn force(&mut self) -> &T {
        if self.value.is_none() {
            if let Some(generator) = self.generator.take() {
                self.value = Some(generator());
            }
        }
        self.value.as_ref().unwrap()
    }

    /// Check if the value has been evaluated
    pub fn is_evaluated(&self) -> bool {
        self.value.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lazy_pipeline() {
        let numbers = vec![1, 2, 3, 4, 5];
        let result: Vec<i32> = LazyPipeline::new(numbers.into_iter())
            .filter(|&x| x > 2)
            .map(|x| x * 2)
            .collect();

        assert_eq!(result, vec![6, 8, 10]);
    }

    #[test]
    fn test_lazy_value() {
        let mut lazy = Lazy::new(|| {
            println!("Computing expensive value");
            42
        });

        assert!(!lazy.is_evaluated());
        assert_eq!(*lazy.force(), 42);
        assert!(lazy.is_evaluated());
    }
}
