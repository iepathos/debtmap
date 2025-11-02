//! Type Extractors
//!
//! Language-specific type signature extraction

pub mod rust;

pub use rust::extract_rust_signature;
