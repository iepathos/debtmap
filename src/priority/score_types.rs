//! Score types module.
//!
//! This module previously contained `Score0To100` and `Score0To1` newtype
//! wrappers for score types. These have been removed in favor of using
//! plain `f64` values throughout the codebase (spec 261).
//!
//! Scores now have no upper bound - they can exceed 100 for severe debt items.
//! This preserves relative priority information. Negative values are floored
//! to 0 using `.max(0.0)` at the point of score calculation.
//!
//! # Migration Guide
//!
//! - `Score0To100::new(x)` → `x.max(0.0)`
//! - `score.value()` → `score` (direct f64)
//! - `score.normalize()` → `score / 100.0`

// Module is kept for backwards compatibility but contains no types.
// Re-exports are removed from mod.rs.
