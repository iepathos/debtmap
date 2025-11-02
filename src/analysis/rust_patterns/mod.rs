//! Rust-Specific Responsibility Pattern Detection
//!
//! This module implements AST-based detection of Rust language patterns to enhance
//! responsibility classification accuracy for Rust code:
//!
//! - **Trait Implementations**: Standard traits (Display, From, Drop, etc.)
//! - **Async/Concurrency**: async fn, tokio::spawn, channels, mutexes
//! - **Error Handling**: ? operator, Result types, unwrap/panic anti-patterns
//! - **Builder Patterns**: Chainable methods, constructors, finalization
//!
//! All pattern detection uses `syn::visit::Visit` for accurate AST traversal,
//! avoiding false positives from comments or string literals.

pub mod async_detector;
pub mod builder_detector;
pub mod context;
pub mod detector;
pub mod error_detector;
pub mod trait_detector;

// Re-export primary types
pub use context::{ImplContext, RustFunctionContext};
pub use detector::{
    RustPattern, RustPatternDetector, RustPatternResult, RustSpecificClassification,
};
