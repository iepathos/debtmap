//! Integration tests for macro expansion functionality

use debtmap::expansion::{ExpansionConfig, MacroExpander, MacroExpansion};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Test that expansion config can be created with defaults
#[test]
fn test_expansion_config_defaults() {
    let config = ExpansionConfig::default();
    assert!(config.enabled); // Now enabled by default for accuracy
    assert!(config.fallback_on_error);
    assert!(config.parallel);
    assert_eq!(config.timeout_secs, 60);
}

/// Test that expander can be created
#[test]
fn test_create_expander() {
    let config = ExpansionConfig::default();
    let result = MacroExpander::new(config);
    // This might fail if cargo is not in PATH, which is OK for CI
    if result.is_ok() {
        assert!(!result.unwrap().is_cache_valid(Path::new("nonexistent.rs")));
    }
}

/// Test cache directory creation
#[test]
fn test_cache_directory_creation() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join(".debtmap/cache/expanded");

    let config = ExpansionConfig {
        cache_dir: cache_dir.clone(),
        ..Default::default()
    };

    if let Ok(_expander) = MacroExpander::new(config) {
        assert!(cache_dir.exists());
    }
}

/// Test source map parsing
#[test]
fn test_source_map_creation() {
    use debtmap::expansion::SourceMap;

    let expanded_code = r#"
#[line = 10]
fn foo() {
    println!("hello");
}
#[line = 15]
fn bar() {
    format!("world");
}
"#;

    let source_map = SourceMap::from_expanded(expanded_code, Path::new("test.rs")).unwrap();

    // Check that mappings were created
    assert!(!source_map.mappings().is_empty());
}

/// Test expansion with a simple Rust file
#[test]
fn test_expand_simple_file() {
    // Create a temporary Rust project
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    // Create Cargo.toml
    let cargo_toml = r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    // Create a simple Rust file with macros
    let rust_code = r#"
fn main() {
    println!("Hello, world!");
    let result = format!("{} {}", "foo", "bar");
    assert_eq!(result, "foo bar");
}

fn helper() -> String {
    format!("helper")
}
"#;
    let lib_path = src_dir.join("lib.rs");
    fs::write(&lib_path, rust_code).unwrap();

    // Try to expand (might fail if cargo-expand is not installed)
    let config = ExpansionConfig {
        enabled: true,
        ..Default::default()
    };

    if let Ok(mut expander) = MacroExpander::new(config) {
        // Check if cargo-expand is available
        if expander.expand_file(&lib_path).is_ok() {
            // If expansion succeeded, cache should be valid
            assert!(expander.is_cache_valid(&lib_path));
        }
    }
}

/// Test that fallback works when expansion fails
#[test]
fn test_fallback_on_error() {
    let config = ExpansionConfig {
        enabled: true,
        fallback_on_error: true,
        ..Default::default()
    };

    // Try to expand a non-existent file
    if let Ok(mut expander) = MacroExpander::new(config) {
        let result = expander.expand_file(Path::new("nonexistent.rs"));
        // Should fail but not panic
        assert!(result.is_err());
    }
}

/// Test cache clearing
#[test]
fn test_clear_cache() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join(".debtmap/cache/expanded");

    let config = ExpansionConfig {
        cache_dir,
        ..Default::default()
    };

    if let Ok(mut expander) = MacroExpander::new(config) {
        // Clear cache should succeed even if empty
        assert!(expander.clear_cache().is_ok());
    }
}
