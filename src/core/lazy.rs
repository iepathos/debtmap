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

impl<T> Default for TransformationPipeline<T> {
    fn default() -> Self {
        Self::new()
    }
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
        // Safe: Either value was already Some, or we just set it via generator
        // The only edge case is if force() is called after generator was already taken
        // and value is still None. In that case, we panic with a clear message.
        self.value
            .as_ref()
            .expect("Lazy::force called but generator was already consumed and value is None")
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
    fn test_lazy_pipeline_take() {
        let numbers = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let result: Vec<i32> = LazyPipeline::new(numbers.into_iter()).take(3).collect();

        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_lazy_pipeline_skip() {
        let numbers = vec![1, 2, 3, 4, 5];
        let result: Vec<i32> = LazyPipeline::new(numbers.into_iter()).skip(2).collect();

        assert_eq!(result, vec![3, 4, 5]);
    }

    #[test]
    fn test_lazy_pipeline_flat_map() {
        let numbers = vec![1, 2, 3];
        let result: Vec<i32> = LazyPipeline::new(numbers.into_iter())
            .flat_map(|x| vec![x, x * 2])
            .collect();

        assert_eq!(result, vec![1, 2, 2, 4, 3, 6]);
    }

    #[test]
    fn test_lazy_pipeline_fold() {
        let numbers = vec![1, 2, 3, 4, 5];
        let sum = LazyPipeline::new(numbers.into_iter()).fold(0, |acc, x| acc + x);

        assert_eq!(sum, 15);
    }

    #[test]
    fn test_lazy_pipeline_any() {
        let numbers = vec![1, 2, 3, 4, 5];
        let has_three = LazyPipeline::new(numbers.into_iter()).any(|x| x == 3);

        assert!(has_three);
    }

    #[test]
    fn test_lazy_pipeline_all() {
        let numbers = vec![2, 4, 6, 8];
        let all_even = LazyPipeline::new(numbers.into_iter()).all(|x| x % 2 == 0);

        assert!(all_even);
    }

    #[test]
    fn test_lazy_pipeline_count() {
        let numbers = vec![1, 2, 3, 4, 5];
        let count = LazyPipeline::new(numbers.into_iter())
            .filter(|&x| x > 2)
            .count();

        assert_eq!(count, 3);
    }

    #[test]
    fn test_transformation_pipeline() {
        let pipeline = TransformationPipeline::new()
            .add_transformation(|x| x + 1)
            .add_transformation(|x| x * 2);

        let result = pipeline.apply(5);
        assert_eq!(result, 12); // (5 + 1) * 2
    }

    #[test]
    fn test_transformation_pipeline_apply_all() {
        let pipeline = TransformationPipeline::new().add_transformation(|x| x + 10);

        let values = vec![1, 2, 3];
        let results = pipeline.apply_all(values);
        assert_eq!(results, vec![11, 12, 13]);
    }

    #[test]
    fn test_transformation_pipeline_empty() {
        let pipeline = TransformationPipeline::<i32>::new();
        let result = pipeline.apply(42);
        assert_eq!(result, 42); // No transformations, returns input
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

    #[test]
    fn test_lazy_value_multiple_force() {
        let mut lazy = Lazy::new(|| 100);

        assert_eq!(*lazy.force(), 100);
        assert_eq!(*lazy.force(), 100); // Should return cached value
        assert!(lazy.is_evaluated());
    }

    #[test]
    fn test_lazy_pipeline_complex_chain() {
        let numbers = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let result: Vec<i32> = LazyPipeline::new(numbers.into_iter())
            .filter(|&x| x % 2 == 0)
            .map(|x| x * x)
            .take(3)
            .collect();

        assert_eq!(result, vec![4, 16, 36]);
    }
}
