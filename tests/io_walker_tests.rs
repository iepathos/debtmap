use debtmap::core::Language;
use debtmap::io::walker::FileWalker;
use std::fs;
use tempfile::TempDir;

/// Helper function to create a test directory structure
fn create_test_directory() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create various test files
    fs::create_dir_all(base_path.join("src")).unwrap();
    fs::write(base_path.join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(base_path.join("src/lib.rs"), "pub mod utils;").unwrap();
    fs::write(base_path.join("src/test.py"), "def test(): pass").unwrap();
    fs::write(base_path.join("src/app.js"), "console.log('hello');").unwrap();
    fs::write(base_path.join("src/types.ts"), "interface User {}").unwrap();

    // Create some files that should be ignored
    fs::write(base_path.join("src/README.md"), "# Readme").unwrap();
    fs::write(base_path.join("src/config.toml"), "[config]").unwrap();

    // Create nested directories
    fs::create_dir_all(base_path.join("src/nested")).unwrap();
    fs::write(base_path.join("src/nested/deep.rs"), "fn deep() {}").unwrap();

    // Create hidden directory and .gitignore
    fs::create_dir_all(base_path.join(".git")).unwrap();
    fs::write(base_path.join(".git/config"), "[core]").unwrap();
    fs::write(base_path.join(".gitignore"), "target/\n*.log").unwrap();

    temp_dir
}

#[test]
fn test_walk_finds_all_supported_files() {
    let temp_dir = create_test_directory();
    let walker = FileWalker::new(temp_dir.path().to_path_buf());

    let files = walker.walk().unwrap();

    // Should find: main.rs, lib.rs, test.py, app.js, types.ts, nested/deep.rs
    assert_eq!(files.len(), 6);

    // Verify file extensions are correct
    let extensions: Vec<String> = files
        .iter()
        .filter_map(|f| f.extension())
        .map(|e| e.to_string_lossy().to_string())
        .collect();

    assert!(extensions.contains(&"rs".to_string()));
    assert!(extensions.contains(&"py".to_string()));
    assert!(extensions.contains(&"js".to_string()));
    assert!(extensions.contains(&"ts".to_string()));
}

#[test]
fn test_walk_filters_by_language() {
    let temp_dir = create_test_directory();
    let walker =
        FileWalker::new(temp_dir.path().to_path_buf()).with_languages(vec![Language::Rust]);

    let files = walker.walk().unwrap();

    // Should only find Rust files: main.rs, lib.rs, nested/deep.rs
    assert_eq!(files.len(), 3);

    // All files should be Rust files
    for file in &files {
        assert_eq!(file.extension().unwrap().to_string_lossy(), "rs");
    }
}

#[test]
fn test_walk_with_ignore_patterns() {
    let temp_dir = create_test_directory();
    let walker = FileWalker::new(temp_dir.path().to_path_buf())
        .with_languages(vec![Language::Rust])
        .with_ignore_patterns(vec!["**/nested/**".to_string()]);

    let files = walker.walk().unwrap();

    // Should find main.rs and lib.rs but not nested/deep.rs
    assert_eq!(files.len(), 2);

    // Verify nested files are excluded
    let has_nested = files.iter().any(|f| f.to_string_lossy().contains("nested"));
    assert!(!has_nested);
}

#[test]
fn test_walk_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let walker = FileWalker::new(temp_dir.path().to_path_buf());

    let files = walker.walk().unwrap();

    // Should return empty vector for empty directory
    assert_eq!(files.len(), 0);
}
