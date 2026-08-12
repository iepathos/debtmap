//! Collection types used by Debtmap's analysis graph and result models.
//!
//! These aliases keep collection choices explicit at module boundaries while relying on the
//! maintained standard library rather than an external persistent-collection implementation.

pub use std::collections::{HashMap, HashSet};

/// Ordered, contiguous collection used for result and graph edge lists.
pub type Vector<T> = Vec<T>;
