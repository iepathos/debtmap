//! Benchmark for evidence formatting performance (spec 148)
//!
//! Verifies that evidence formatting overhead is <2% as required by spec

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use debtmap::analysis::multi_signal_aggregation::{
    AggregatedClassification, ResponsibilityCategory, SignalEvidence, SignalType,
};
use debtmap::output::evidence_formatter::EvidenceFormatter;

/// Create sample evidence for benchmarking
fn create_sample_evidence() -> AggregatedClassification {
    AggregatedClassification {
        primary: ResponsibilityCategory::FileIO,
        confidence: 0.85,
        evidence: vec![
            SignalEvidence {
                signal_type: SignalType::IoDetection,
                category: ResponsibilityCategory::FileIO,
                confidence: 0.90,
                weight: 0.35,
                contribution: 0.315,
                description: "Multiple file operations detected".to_string(),
            },
            SignalEvidence {
                signal_type: SignalType::CallGraph,
                category: ResponsibilityCategory::FileIO,
                confidence: 0.85,
                weight: 0.25,
                contribution: 0.2125,
                description: "Calls file-related functions".to_string(),
            },
            SignalEvidence {
                signal_type: SignalType::TypeSignatures,
                category: ResponsibilityCategory::FileIO,
                confidence: 0.80,
                weight: 0.20,
                contribution: 0.16,
                description: "File/Path types in signature".to_string(),
            },
            SignalEvidence {
                signal_type: SignalType::Purity,
                category: ResponsibilityCategory::FileIO,
                confidence: 0.75,
                weight: 0.15,
                contribution: 0.1125,
                description: "Detected I/O operations".to_string(),
            },
        ],
        alternatives: vec![
            (ResponsibilityCategory::Orchestration, 0.62),
            (ResponsibilityCategory::ErrorHandling, 0.55),
        ],
    }
}

/// Benchmark evidence formatting at minimal verbosity
fn bench_evidence_minimal(c: &mut Criterion) {
    let evidence = create_sample_evidence();
    let formatter = EvidenceFormatter::new(0);

    c.bench_function("evidence_format_minimal", |b| {
        b.iter(|| {
            let _output = formatter.format_evidence(black_box(&evidence));
        })
    });
}

/// Benchmark evidence formatting at standard verbosity
fn bench_evidence_standard(c: &mut Criterion) {
    let evidence = create_sample_evidence();
    let formatter = EvidenceFormatter::new(1);

    c.bench_function("evidence_format_standard", |b| {
        b.iter(|| {
            let _output = formatter.format_evidence(black_box(&evidence));
        })
    });
}

/// Benchmark evidence formatting at verbose level
fn bench_evidence_verbose(c: &mut Criterion) {
    let evidence = create_sample_evidence();
    let formatter = EvidenceFormatter::new(2);

    c.bench_function("evidence_format_verbose", |b| {
        b.iter(|| {
            let _output = formatter.format_evidence(black_box(&evidence));
        })
    });
}

/// Benchmark baseline (no evidence formatting) for comparison
fn bench_baseline_no_evidence(c: &mut Criterion) {
    c.bench_function("baseline_no_evidence", |b| {
        b.iter(|| {
            // Simulate minimal work that would be done without evidence formatting
            let _s = format!("File I/O [85% confidence]");
        })
    });
}

/// Benchmark with multiple evidence items
fn bench_evidence_many_signals(c: &mut Criterion) {
    let mut evidence = create_sample_evidence();
    // Add more signals
    for i in 0..10 {
        evidence.evidence.push(SignalEvidence {
            signal_type: SignalType::Name,
            category: ResponsibilityCategory::Unknown,
            confidence: 0.50,
            weight: 0.05,
            contribution: 0.025,
            description: format!("Additional signal {}", i),
        });
    }

    let formatter = EvidenceFormatter::new(1);

    c.bench_function("evidence_format_many_signals", |b| {
        b.iter(|| {
            let _output = formatter.format_evidence(black_box(&evidence));
        })
    });
}

criterion_group!(
    benches,
    bench_baseline_no_evidence,
    bench_evidence_minimal,
    bench_evidence_standard,
    bench_evidence_verbose,
    bench_evidence_many_signals,
);
criterion_main!(benches);
