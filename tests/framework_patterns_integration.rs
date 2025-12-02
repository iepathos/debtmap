//! Integration tests for framework pattern detection
//!
//! This test suite validates that framework patterns are correctly detected
//! across Rust, Python, and JavaScript/TypeScript codebases.

use debtmap::analysis::framework_patterns_multi::{
    detector::{Attribute, Decorator, FileContext, FunctionAst, FunctionCall, Parameter},
    patterns::Language,
    FrameworkDetector,
};
use std::path::Path;

#[test]
fn test_axum_handler_detection() {
    let config_path = Path::new("framework_patterns.toml");

    // Skip if config doesn't exist (for CI environments)
    if !config_path.exists() {
        eprintln!("Skipping test: framework_patterns.toml not found");
        return;
    }

    let detector = FrameworkDetector::from_config(config_path)
        .expect("Failed to load framework patterns config");

    let mut function = FunctionAst::new("get_user".to_string());
    function.is_async = true;
    function.parameters.push(Parameter {
        name: "user_id".to_string(),
        type_annotation: "Path<u32>".to_string(),
    });
    function.return_type = Some("Json<User>".to_string());

    let mut file_context = FileContext::new(Language::Rust, "handlers.rs".into());
    file_context.add_import("use axum::extract::Path;".to_string());
    file_context.add_import("use axum::response::Json;".to_string());

    let matches = detector.detect_framework_patterns(&function, &file_context);

    assert!(!matches.is_empty(), "Should detect Axum framework");
    assert_eq!(matches[0].category, "HTTP Request Handler");
    assert!(matches[0].framework.contains("Axum"));
}

#[test]
fn test_pytest_fixture_detection() {
    let config_path = Path::new("framework_patterns.toml");

    if !config_path.exists() {
        eprintln!("Skipping test: framework_patterns.toml not found");
        return;
    }

    let detector = FrameworkDetector::from_config(config_path)
        .expect("Failed to load framework patterns config");

    let mut function = FunctionAst::new("database".to_string());
    function.decorators.push(Decorator {
        name: "@pytest.fixture".to_string(),
    });

    let mut file_context = FileContext::new(Language::Python, "conftest.py".into());
    file_context.add_import("import pytest".to_string());

    let matches = detector.detect_framework_patterns(&function, &file_context);

    assert!(!matches.is_empty(), "Should detect pytest framework");
    assert_eq!(matches[0].category, "Test Function");
}

#[test]
fn test_rust_test_detection() {
    let config_path = Path::new("framework_patterns.toml");

    if !config_path.exists() {
        eprintln!("Skipping test: framework_patterns.toml not found");
        return;
    }

    let detector = FrameworkDetector::from_config(config_path)
        .expect("Failed to load framework patterns config");

    let mut function = FunctionAst::new("test_addition".to_string());
    function.attributes.push(Attribute {
        text: "#[test]".to_string(),
    });

    let file_context = FileContext::new(Language::Rust, "lib.rs".into());

    let matches = detector.detect_framework_patterns(&function, &file_context);

    assert!(!matches.is_empty(), "Should detect Rust test");
    assert_eq!(matches[0].category, "Test Function");
}

#[test]
fn test_clap_cli_parser_detection() {
    let config_path = Path::new("framework_patterns.toml");

    if !config_path.exists() {
        eprintln!("Skipping test: framework_patterns.toml not found");
        return;
    }

    let detector = FrameworkDetector::from_config(config_path)
        .expect("Failed to load framework patterns config");

    let mut function = FunctionAst::new("Args".to_string());
    function.derives.push("Parser".to_string());

    let mut file_context = FileContext::new(Language::Rust, "main.rs".into());
    file_context.add_import("use clap::Parser;".to_string());

    let matches = detector.detect_framework_patterns(&function, &file_context);

    assert!(!matches.is_empty(), "Should detect Clap CLI");
    assert_eq!(matches[0].category, "CLI Argument Parsing");
}

#[test]
fn test_fastapi_handler_detection() {
    let config_path = Path::new("framework_patterns.toml");

    if !config_path.exists() {
        eprintln!("Skipping test: framework_patterns.toml not found");
        return;
    }

    let detector = FrameworkDetector::from_config(config_path)
        .expect("Failed to load framework patterns config");

    let mut function = FunctionAst::new("create_user".to_string());
    function.decorators.push(Decorator {
        name: "@app.post".to_string(),
    });

    let mut file_context = FileContext::new(Language::Python, "main.py".into());
    file_context.add_import("from fastapi import FastAPI".to_string());

    let matches = detector.detect_framework_patterns(&function, &file_context);

    assert!(!matches.is_empty(), "Should detect FastAPI framework");
    assert_eq!(matches[0].category, "HTTP Request Handler");
}

#[test]
fn test_diesel_query_detection() {
    let config_path = Path::new("framework_patterns.toml");

    if !config_path.exists() {
        eprintln!("Skipping test: framework_patterns.toml not found");
        return;
    }

    let detector = FrameworkDetector::from_config(config_path)
        .expect("Failed to load framework patterns config");

    let mut function = FunctionAst::new("get_all_users".to_string());
    function.calls.push(FunctionCall {
        name: ".load(".to_string(),
    });

    let mut file_context = FileContext::new(Language::Rust, "db.rs".into());
    file_context.add_import("use diesel::prelude::*;".to_string());

    let matches = detector.detect_framework_patterns(&function, &file_context);

    assert!(!matches.is_empty(), "Should detect Diesel framework");
    assert_eq!(matches[0].category, "Database Query");
}

#[test]
fn test_confidence_calculation() {
    let config_path = Path::new("framework_patterns.toml");

    if !config_path.exists() {
        eprintln!("Skipping test: framework_patterns.toml not found");
        return;
    }

    let detector = FrameworkDetector::from_config(config_path)
        .expect("Failed to load framework patterns config");

    let mut function = FunctionAst::new("get_user".to_string());
    function.is_async = true;
    function.parameters.push(Parameter {
        name: "user_id".to_string(),
        type_annotation: "Path<u32>".to_string(),
    });
    function.return_type = Some("Json<User>".to_string());

    let mut file_context = FileContext::new(Language::Rust, "handlers.rs".into());
    file_context.add_import("use axum::extract::Path;".to_string());
    file_context.add_import("use axum::response::Json;".to_string());

    let matches = detector.detect_framework_patterns(&function, &file_context);

    assert!(!matches.is_empty());
    // Confidence should be between 0.5 and 1.0
    assert!(matches[0].confidence >= 0.5 && matches[0].confidence <= 1.0);
}

#[test]
fn test_no_false_positives() {
    let config_path = Path::new("framework_patterns.toml");

    if !config_path.exists() {
        eprintln!("Skipping test: framework_patterns.toml not found");
        return;
    }

    let detector = FrameworkDetector::from_config(config_path)
        .expect("Failed to load framework patterns config");

    let function = FunctionAst::new("normal_function".to_string());
    let file_context = FileContext::new(Language::Rust, "lib.rs".into());

    let matches = detector.detect_framework_patterns(&function, &file_context);

    assert!(
        matches.is_empty(),
        "Should not detect framework for normal function"
    );
}
