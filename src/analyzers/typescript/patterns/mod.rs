//! Pattern detection for TypeScript/JavaScript
//!
//! Detects various coding patterns including async/await, promises,
//! callbacks, and functional programming patterns.

pub mod async_await;
pub mod callback;
pub mod functional;
pub mod promise;

pub use async_await::detect_async_patterns;
pub use callback::detect_callback_patterns;
pub use functional::detect_functional_chains;
pub use promise::detect_promise_patterns;
