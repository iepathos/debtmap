use super::*;

fn near_duplicate_files() -> Vec<(PathBuf, String)> {
    vec![
        (
            PathBuf::from("a.rs"),
            "alpha bravo charlie\ndelta echo foxtrot\ngolf hotel india".into(),
        ),
        (
            PathBuf::from("b.rs"),
            "alpha bravo charlie\ndelta echo foxtrot\ngolf hotel juliet".into(),
        ),
    ]
}

#[test]
fn similarity_threshold_changes_near_duplicate_detection() {
    assert_eq!(detect_duplication(near_duplicate_files(), 3, 0.8).len(), 1);
    assert!(detect_duplication(near_duplicate_files(), 3, 0.81).is_empty());
    assert!(detect_duplication(near_duplicate_files(), 3, 1.0).is_empty());
}

#[test]
fn exact_detection_is_deterministic() {
    let mut files = near_duplicate_files();
    files[1].1 = files[0].1.clone();
    let first = detect_duplication(files.clone(), 3, 1.0);
    let second = detect_duplication(files, 3, 1.0);

    assert_eq!(first, second);
    assert_eq!(first[0].locations[0].file, PathBuf::from("a.rs"));
}

#[test]
fn fuzzy_detection_is_independent_of_input_order() {
    let files = near_duplicate_files();
    let mut reversed = files.clone();
    reversed.reverse();

    assert_eq!(
        detect_duplication(files, 3, 0.8),
        detect_duplication(reversed, 3, 0.8)
    );
}

#[test]
fn invalid_public_thresholds_fail_closed() {
    assert!(detect_duplication(near_duplicate_files(), 0, 0.8).is_empty());
    assert!(detect_duplication(near_duplicate_files(), 3, 0.0).is_empty());
    assert!(detect_duplication(near_duplicate_files(), 3, -0.1).is_empty());
    assert!(detect_duplication(near_duplicate_files(), 3, 1.1).is_empty());
    assert!(detect_duplication(near_duplicate_files(), 3, f64::NAN).is_empty());
}
