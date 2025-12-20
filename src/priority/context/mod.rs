//! Context window suggestions for AI agents (Spec 263).

pub mod callees;
pub mod callers;
pub mod generator;
pub mod limits;
pub mod tests;
pub mod tests_ctx;
pub mod types;
pub mod types_ctx;

pub use generator::{generate_context_suggestion, ContextConfig};
pub use types::{ContextRelationship, ContextSuggestion, FileRange, RelatedContext};
