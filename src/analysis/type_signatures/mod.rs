//! Type Signature-Based Classification
//!
//! Classifies function responsibilities based on type signatures:
//! - Parser patterns (String → Result<T>)
//! - Formatter patterns (T → String)
//! - Validator patterns (T → Result<(), Error>)
//! - I/O patterns (error types, trait bounds)
//! - Builder patterns (Self → Self)
//! - Query patterns (&T → Option<U>)
//!
//! Spec 147: Type Signature-Based Classification

pub mod analyzer;
pub mod extractors;
pub mod normalizer;
pub mod patterns;

pub use analyzer::{
    GenericBound, Parameter, TypeBasedClassification, TypeSignature, TypeSignatureAnalyzer,
};
pub use extractors::extract_rust_signature;
pub use normalizer::{CanonicalType, TypeNormalizer};
pub use patterns::{TypeMatcher, TypePattern, TypePatternLibrary};
