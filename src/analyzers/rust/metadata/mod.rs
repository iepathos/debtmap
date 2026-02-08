//! Metadata extraction for Rust functions
//!
//! Contains modules for extracting and classifying function metadata.

pub mod classification;
pub mod extraction;
pub mod test_detection;

pub use classification::classify_function_role;
pub use extraction::{extract_function_metadata, extract_visibility};
pub use test_detection::{
    classify_test_file, has_test_attribute, has_test_name_pattern, is_test_function,
};
