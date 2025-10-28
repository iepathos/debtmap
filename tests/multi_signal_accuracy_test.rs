//! Multi-Signal Responsibility Classification Accuracy Tests
//!
//! Validates that the multi-signal aggregation achieves >85% classification accuracy
//! against a manually curated ground truth corpus.

use debtmap::analysis::io_detection::Language;
use debtmap::analysis::multi_signal_aggregation::{
    ResponsibilityAggregator, ResponsibilityCategory, SignalSet,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
struct GroundTruthCorpus {
    test_cases: Vec<TestCase>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TestCase {
    id: String,
    function_name: String,
    language: String,
    code: String,
    expected_category: String,
    minimum_confidence: f64,
    rationale: String,
}

impl TestCase {
    /// Parse expected category string into ResponsibilityCategory
    fn parse_expected_category(&self) -> ResponsibilityCategory {
        match self.expected_category.as_str() {
            "FileIO" => ResponsibilityCategory::FileIO,
            "NetworkIO" => ResponsibilityCategory::NetworkIO,
            "DatabaseIO" => ResponsibilityCategory::DatabaseIO,
            "ConfigurationIO" => ResponsibilityCategory::ConfigurationIO,
            "HttpRequestHandler" => ResponsibilityCategory::HttpRequestHandler,
            "CliHandler" => ResponsibilityCategory::CliHandler,
            "TestFunction" => ResponsibilityCategory::TestFunction,
            "PureComputation" => ResponsibilityCategory::PureComputation,
            "Validation" => ResponsibilityCategory::Validation,
            "Transformation" => ResponsibilityCategory::Transformation,
            "Parsing" => ResponsibilityCategory::Parsing,
            "Formatting" => ResponsibilityCategory::Formatting,
            "Orchestration" => ResponsibilityCategory::Orchestration,
            "Coordination" => ResponsibilityCategory::Coordination,
            "ErrorHandling" => ResponsibilityCategory::ErrorHandling,
            _ => ResponsibilityCategory::Unknown,
        }
    }

    /// Parse language string into Language enum
    fn parse_language(&self) -> Language {
        match self.language.to_lowercase().as_str() {
            "rust" => Language::Rust,
            "python" => Language::Python,
            "javascript" | "js" => Language::JavaScript,
            "typescript" | "ts" => Language::TypeScript,
            _ => Language::Rust, // Default fallback
        }
    }
}

/// Load ground truth corpus from JSON file
fn load_ground_truth() -> GroundTruthCorpus {
    let corpus_path = Path::new("tests/responsibility_ground_truth.json");
    let contents = fs::read_to_string(corpus_path)
        .expect("Failed to read ground truth corpus");
    serde_json::from_str(&contents)
        .expect("Failed to parse ground truth corpus")
}

/// Classify a test case using multi-signal aggregation
fn classify_test_case(
    aggregator: &ResponsibilityAggregator,
    test_case: &TestCase,
) -> ResponsibilityCategory {
    let language = test_case.parse_language();

    // Collect available signals
    let mut signals = SignalSet::default();

    // Always collect I/O and purity signals
    signals.io_signal = aggregator.collect_io_signal(&test_case.code, language);
    signals.purity_signal = aggregator.collect_purity_signal(&test_case.code, language);
    signals.name_signal = Some(aggregator.collect_name_signal(&test_case.function_name));

    // Aggregate signals
    let result = aggregator.aggregate(&signals);
    result.primary
}

#[test]
fn test_multi_signal_accuracy() {
    let corpus = load_ground_truth();
    let aggregator = ResponsibilityAggregator::new();

    let mut correct = 0;
    let mut total = 0;
    let mut failed_cases = Vec::new();

    for test_case in &corpus.test_cases {
        total += 1;
        let expected = test_case.parse_expected_category();
        let actual = classify_test_case(&aggregator, test_case);

        if actual == expected {
            correct += 1;
            println!("✓ {}: {} (correct)", test_case.id, expected.as_str());
        } else {
            println!(
                "✗ {}: expected {}, got {}",
                test_case.id,
                expected.as_str(),
                actual.as_str()
            );
            failed_cases.push((test_case.id.clone(), expected, actual));
        }
    }

    let accuracy = (correct as f64 / total as f64) * 100.0;
    println!("\n=== Accuracy Results ===");
    println!("Correct: {}/{}", correct, total);
    println!("Accuracy: {:.2}%", accuracy);

    if !failed_cases.is_empty() {
        println!("\nFailed cases:");
        for (id, expected, actual) in failed_cases {
            println!("  - {}: expected {}, got {}", id, expected.as_str(), actual.as_str());
        }
    }

    assert!(
        accuracy >= 85.0,
        "Accuracy {:.2}% is below the 85% threshold",
        accuracy
    );
}

#[test]
fn test_individual_signal_strengths() {
    let corpus = load_ground_truth();
    let aggregator = ResponsibilityAggregator::new();

    println!("\n=== Individual Signal Performance ===");

    // Test I/O detection signal only
    let mut io_correct = 0;
    for test_case in &corpus.test_cases {
        if let Some(io_signal) = aggregator.collect_io_signal(&test_case.code, test_case.parse_language()) {
            if io_signal.category == test_case.parse_expected_category() {
                io_correct += 1;
            }
        }
    }
    println!("I/O Detection alone: {:.2}%", (io_correct as f64 / corpus.test_cases.len() as f64) * 100.0);

    // Test purity analysis signal only
    let mut purity_correct = 0;
    for test_case in &corpus.test_cases {
        if let Some(purity_signal) = aggregator.collect_purity_signal(&test_case.code, test_case.parse_language()) {
            if purity_signal.category == test_case.parse_expected_category() {
                purity_correct += 1;
            }
        }
    }
    println!("Purity Analysis alone: {:.2}%", (purity_correct as f64 / corpus.test_cases.len() as f64) * 100.0);

    // Test name-based signal only
    let mut name_correct = 0;
    for test_case in &corpus.test_cases {
        let name_signal = aggregator.collect_name_signal(&test_case.function_name);
        if name_signal.category == test_case.parse_expected_category() {
            name_correct += 1;
        }
    }
    println!("Name-based alone: {:.2}%", (name_correct as f64 / corpus.test_cases.len() as f64) * 100.0);
}

#[test]
fn test_confidence_levels() {
    let corpus = load_ground_truth();
    let aggregator = ResponsibilityAggregator::new();

    println!("\n=== Confidence Level Analysis ===");

    let mut high_confidence_correct = 0;
    let mut high_confidence_total = 0;

    for test_case in &corpus.test_cases {
        let language = test_case.parse_language();
        let mut signals = SignalSet::default();

        signals.io_signal = aggregator.collect_io_signal(&test_case.code, language);
        signals.purity_signal = aggregator.collect_purity_signal(&test_case.code, language);
        signals.name_signal = Some(aggregator.collect_name_signal(&test_case.function_name));

        let result = aggregator.aggregate(&signals);

        if result.confidence >= 0.70 {
            high_confidence_total += 1;
            if result.primary == test_case.parse_expected_category() {
                high_confidence_correct += 1;
            }
        }
    }

    if high_confidence_total > 0 {
        let high_conf_accuracy = (high_confidence_correct as f64 / high_confidence_total as f64) * 100.0;
        println!("High confidence (>=0.70) accuracy: {:.2}%", high_conf_accuracy);
        println!("High confidence cases: {}/{}", high_confidence_total, corpus.test_cases.len());
    }
}

#[test]
fn test_category_specific_accuracy() {
    let corpus = load_ground_truth();
    let aggregator = ResponsibilityAggregator::new();

    println!("\n=== Category-Specific Accuracy ===");

    let mut category_stats: std::collections::HashMap<ResponsibilityCategory, (usize, usize)> =
        std::collections::HashMap::new();

    for test_case in &corpus.test_cases {
        let expected = test_case.parse_expected_category();
        let actual = classify_test_case(&aggregator, test_case);

        let (total, correct) = category_stats.entry(expected).or_insert((0, 0));
        *total += 1;
        if actual == expected {
            *correct += 1;
        }
    }

    for (category, (total, correct)) in category_stats {
        let accuracy = (correct as f64 / total as f64) * 100.0;
        println!("{}: {}/{} ({:.2}%)", category.as_str(), correct, total, accuracy);
    }
}
