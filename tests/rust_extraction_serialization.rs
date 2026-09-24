//! Extraction records preserve optional fields in current binary formats.
use debtmap::extraction::{ExtractedFileData, UnifiedFileExtractor};
use std::path::{Path, PathBuf};

fn assert_postcard_round_trip(data: ExtractedFileData) {
    let bytes = postcard::to_allocvec(&data).unwrap();
    let restored: ExtractedFileData = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(
        serde_json::to_value(restored).unwrap(),
        serde_json::to_value(data).unwrap()
    );
}

#[test]
fn current_empty_other_language_records_round_trip_through_postcard() {
    assert_postcard_round_trip(ExtractedFileData::empty(PathBuf::from("sample.py")));
}

#[test]
fn current_rust_snapshots_and_source_absent_functions_round_trip_through_postcard() {
    let mut data = UnifiedFileExtractor::extract(
        Path::new("src/lib.rs"),
        "struct A; impl A { fn bar(&self) {} } fn caller(a: &A) { a.bar(); }",
    )
    .unwrap();
    assert_postcard_round_trip(data.clone());
    data.rust_source = None;
    assert_postcard_round_trip(data);
}

#[test]
fn current_nonempty_python_records_round_trip_through_postcard() {
    let data = UnifiedFileExtractor::extract(
        Path::new("sample.py"),
        "def helper():\n    return 1\n\ndef caller():\n    return helper()\n",
    )
    .unwrap();
    assert_eq!(data.functions.len(), 2);
    assert_postcard_round_trip(data);
}
