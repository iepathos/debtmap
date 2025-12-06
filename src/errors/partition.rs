//! Helper traits for partitioning Result iterators.
//!
//! Provides `.partition_result()` extension for both sequential
//! and parallel iterators.

use rayon::prelude::*;

/// Extension trait for partitioning Result iterators.
pub trait PartitionResult<T, E>: Iterator<Item = Result<T, E>> + Sized {
    /// Partitions iterator into successes and failures.
    ///
    /// Similar to `Iterator::partition`, but for Results.
    fn partition_result(self) -> (Vec<T>, Vec<E>) {
        self.fold((Vec::new(), Vec::new()), |(mut oks, mut errs), result| {
            match result {
                Ok(val) => oks.push(val),
                Err(err) => errs.push(err),
            }
            (oks, errs)
        })
    }
}

/// Implement for all Result iterators.
impl<T, E, I> PartitionResult<T, E> for I where I: Iterator<Item = Result<T, E>> {}

/// Extension trait for parallel Result iterators.
pub trait ParPartitionResult<T, E>: ParallelIterator<Item = Result<T, E>> {
    /// Partitions parallel iterator into successes and failures.
    fn partition_result(self) -> (Vec<T>, Vec<E>)
    where
        T: Send,
        E: Send,
    {
        self.fold(
            || (Vec::new(), Vec::new()),
            |(mut oks, mut errs), result| {
                match result {
                    Ok(val) => oks.push(val),
                    Err(err) => errs.push(err),
                }
                (oks, errs)
            },
        )
        .reduce(
            || (Vec::new(), Vec::new()),
            |(mut oks1, mut errs1), (oks2, errs2)| {
                oks1.extend(oks2);
                errs1.extend(errs2);
                (oks1, errs1)
            },
        )
    }
}

impl<T, E, I> ParPartitionResult<T, E> for I
where
    I: ParallelIterator<Item = Result<T, E>>,
    T: Send,
    E: Send,
{
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_result_sequential() {
        let results = vec![Ok(1), Err("e1"), Ok(2), Ok(3), Err("e2")];

        let (successes, failures): (Vec<i32>, Vec<&str>) = results.into_iter().partition_result();

        assert_eq!(successes, vec![1, 2, 3]);
        assert_eq!(failures, vec!["e1", "e2"]);
    }

    #[test]
    fn test_partition_result_parallel() {
        let results: Vec<Result<i32, &str>> = vec![Ok(1), Err("e1"), Ok(2), Ok(3), Err("e2")];

        let (successes, failures) = results.into_par_iter().partition_result();

        assert_eq!(successes.len(), 3);
        assert_eq!(failures.len(), 2);
        assert!(successes.contains(&1));
        assert!(successes.contains(&2));
        assert!(successes.contains(&3));
    }

    #[test]
    fn test_partition_result_all_success() {
        let results = vec![Ok(1), Ok(2), Ok(3)];
        let (successes, failures): (Vec<i32>, Vec<&str>) = results.into_iter().partition_result();

        assert_eq!(successes, vec![1, 2, 3]);
        assert!(failures.is_empty());
    }

    #[test]
    fn test_partition_result_all_failures() {
        let results: Vec<Result<i32, &str>> = vec![Err("e1"), Err("e2"), Err("e3")];
        let (successes, failures) = results.into_iter().partition_result();

        assert!(successes.is_empty());
        assert_eq!(failures, vec!["e1", "e2", "e3"]);
    }
}
