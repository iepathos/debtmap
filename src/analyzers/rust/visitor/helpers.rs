//! Visitor helper functions
//!
//! Utility functions for the function visitor.

use crate::analyzers::rust::types::FunctionContext;
use std::path::PathBuf;

/// Get line number from a span
pub fn get_line_number(span: syn::__private::Span) -> usize {
    span.start().line
}

/// Create function context from visitor state
pub fn create_function_context(
    name: String,
    file: PathBuf,
    line: usize,
    is_trait_method: bool,
    in_test_module: bool,
    impl_type_name: Option<String>,
    trait_name: Option<String>,
) -> FunctionContext {
    FunctionContext {
        name,
        file,
        line,
        is_trait_method,
        in_test_module,
        impl_type_name,
        trait_name,
    }
}
