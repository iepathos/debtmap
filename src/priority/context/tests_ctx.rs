//! Test context detection (Spec 263).

use super::types::{ContextRelationship, FileRange, RelatedContext};
use std::path::{Path, PathBuf};

pub fn detect_test_files(source_file: &Path) -> Vec<PathBuf> {
    let mut test_files = Vec::new();

    if let Some(stem) = source_file.file_stem() {
        if let Some(parent) = source_file.parent() {
            let test_name = format!("{}_test.rs", stem.to_string_lossy());
            test_files.push(parent.join(test_name));
        }
    }

    if let Some(_file_name) = source_file.file_name() {
        let test_path = source_file
            .to_string_lossy()
            .replace("/src/", "/tests/")
            .replace("\\src\\", "\\tests\\");
        if test_path != source_file.to_string_lossy() {
            test_files.push(PathBuf::from(test_path));
        }
    }

    test_files
}

pub fn extract_test_contexts(source_file: &Path, function_name: &str) -> Vec<RelatedContext> {
    let test_files = detect_test_files(source_file);

    test_files
        .into_iter()
        .map(|test_file| create_test_context(test_file, function_name))
        .collect()
}

fn create_test_context(test_file: PathBuf, function_name: &str) -> RelatedContext {
    RelatedContext {
        range: FileRange {
            file: test_file.clone(),
            start_line: 1,
            end_line: 100,
            symbol: None,
        },
        relationship: ContextRelationship::TestCode,
        reason: format!("Tests for {}", function_name),
    }
}
