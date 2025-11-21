//! Benchmarks for semantic naming performance
//!
//! Validates that semantic naming adds <10% overhead to god object analysis.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use debtmap::organization::semantic_naming::SemanticNameGenerator;

/// Benchmark basic name generation with varying method counts
fn bench_name_generation(c: &mut Criterion) {
    let generator = SemanticNameGenerator::new();

    c.bench_function("semantic_naming_small_group", |b| {
        let methods = vec![
            "format_output".to_string(),
            "format_summary".to_string(),
            "format_report".to_string(),
        ];

        b.iter(|| {
            let _result = generator.generate_names(black_box(&methods), None);
        });
    });

    c.bench_function("semantic_naming_medium_group", |b| {
        let methods = vec![
            "format_output".to_string(),
            "format_summary".to_string(),
            "format_report".to_string(),
            "validate_index".to_string(),
            "validate_data".to_string(),
            "validate_config".to_string(),
            "parse_input".to_string(),
            "parse_json".to_string(),
            "calculate_total".to_string(),
            "calculate_average".to_string(),
        ];

        b.iter(|| {
            let _result = generator.generate_names(black_box(&methods), None);
        });
    });

    c.bench_function("semantic_naming_large_group", |b| {
        let methods = vec![
            "format_output".to_string(),
            "format_summary".to_string(),
            "format_report".to_string(),
            "format_error".to_string(),
            "format_warning".to_string(),
            "validate_index".to_string(),
            "validate_data".to_string(),
            "validate_config".to_string(),
            "validate_input".to_string(),
            "validate_schema".to_string(),
            "parse_input".to_string(),
            "parse_json".to_string(),
            "parse_xml".to_string(),
            "parse_csv".to_string(),
            "calculate_total".to_string(),
            "calculate_average".to_string(),
            "calculate_median".to_string(),
            "calculate_variance".to_string(),
            "serialize_data".to_string(),
            "serialize_config".to_string(),
        ];

        b.iter(|| {
            let _result = generator.generate_names(black_box(&methods), None);
        });
    });
}

/// Benchmark with responsibility descriptions
fn bench_name_generation_with_description(c: &mut Criterion) {
    let generator = SemanticNameGenerator::new();

    c.bench_function("semantic_naming_with_description", |b| {
        let methods = vec![
            "format_coverage_status".to_string(),
            "format_coverage_factor".to_string(),
            "calculate_coverage_percentage".to_string(),
        ];
        let description = "Manage coverage data and its transformations";

        b.iter(|| {
            let _result =
                generator.generate_names(black_box(&methods), Some(black_box(description)));
        });
    });
}

/// Benchmark multiple splits (simulates real god object splitting workload)
fn bench_multiple_splits(c: &mut Criterion) {
    let generator = SemanticNameGenerator::new();

    c.bench_function("semantic_naming_multiple_splits", |b| {
        let splits = vec![
            vec!["format_output".to_string(), "format_summary".to_string()],
            vec!["validate_index".to_string(), "validate_data".to_string()],
            vec!["parse_input".to_string(), "parse_config".to_string()],
            vec![
                "calculate_coverage".to_string(),
                "calculate_total_size".to_string(),
            ],
            vec![
                "serialize_report".to_string(),
                "serialize_metrics".to_string(),
            ],
        ];

        b.iter(|| {
            for methods in &splits {
                let _result = generator.generate_names(black_box(methods), None);
            }
        });
    });
}

/// Benchmark pattern recognition specifically
fn bench_pattern_recognition(c: &mut Criterion) {
    use debtmap::organization::semantic_naming::PatternRecognizer;

    let recognizer = PatternRecognizer::new();

    c.bench_function("pattern_recognition", |b| {
        let methods = vec![
            "format_output".to_string(),
            "format_summary".to_string(),
            "format_report".to_string(),
            "format_error".to_string(),
        ];

        b.iter(|| {
            let _result = recognizer.recognize_pattern(black_box(&methods));
        });
    });
}

/// Benchmark domain term extraction
fn bench_domain_extraction(c: &mut Criterion) {
    use debtmap::organization::semantic_naming::DomainTermExtractor;

    let extractor = DomainTermExtractor::new();

    c.bench_function("domain_term_extraction", |b| {
        let methods = vec![
            "format_coverage_status".to_string(),
            "format_coverage_factor".to_string(),
            "calculate_coverage_percentage".to_string(),
            "validate_coverage_data".to_string(),
        ];

        b.iter(|| {
            let _result = extractor.generate_domain_name(black_box(&methods));
        });
    });
}

/// Benchmark specificity scoring
fn bench_specificity_scoring(c: &mut Criterion) {
    use debtmap::organization::semantic_naming::SpecificityScorer;

    let scorer = SpecificityScorer::new();

    c.bench_function("specificity_scoring", |b| {
        let names = vec!["formatting", "coverage", "validation", "format_coverage"];

        b.iter(|| {
            for name in &names {
                let _score = scorer.calculate_specificity(black_box(name));
            }
        });
    });
}

criterion_group!(
    benches,
    bench_name_generation,
    bench_name_generation_with_description,
    bench_multiple_splits,
    bench_pattern_recognition,
    bench_domain_extraction,
    bench_specificity_scoring
);
criterion_main!(benches);
