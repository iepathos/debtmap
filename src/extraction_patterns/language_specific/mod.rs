pub mod javascript_patterns;
pub mod rust_patterns;

#[cfg(test)]
mod rust_patterns_test;

pub use javascript_patterns::JavaScriptPatternMatcher;
pub use rust_patterns::RustPatternMatcher;
