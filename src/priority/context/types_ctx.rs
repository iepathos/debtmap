//! Type context extraction (Spec 263).

use super::types::{ContextRelationship, FileRange, RelatedContext};
use std::path::{Path, PathBuf};

pub fn extract_type_contexts(_file: &Path, _function: &str) -> Vec<RelatedContext> {
    Vec::new()
}

#[allow(dead_code)]
fn create_type_context(
    file: PathBuf,
    type_name: String,
    start_line: u32,
    end_line: u32,
) -> RelatedContext {
    RelatedContext {
        range: FileRange {
            file,
            start_line,
            end_line,
            symbol: Some(type_name.clone()),
        },
        relationship: ContextRelationship::TypeDefinition,
        reason: format!("Type definition for {}", type_name),
    }
}
