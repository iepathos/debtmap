//! Solidity parsing, extraction, and analysis benchmarks.

use std::hint::black_box;
use std::path::Path;
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use debtmap::analyzers::solidity::extraction::extract_solidity;
use debtmap::analyzers::solidity::orchestration::analyze_solidity_file;
use debtmap::analyzers::solidity::parser::parse_source;
use debtmap::config::SolidityLanguageConfig;
use debtmap::core::ast::SolidityAst;

const COMPLEXITY_THRESHOLD: u32 = 10;

struct Fixture {
    name: &'static str,
    file_name: &'static str,
    source: &'static str,
}

const FIXTURES: [Fixture; 3] = [
    Fixture {
        name: "small",
        file_name: "SmallToken.sol",
        source: include_str!("fixtures/solidity/small_token.sol"),
    },
    Fixture {
        name: "medium",
        file_name: "MediumPool.sol",
        source: include_str!("fixtures/solidity/medium_pool.sol"),
    },
    Fixture {
        name: "large",
        file_name: "LargeProtocol.sol",
        source: include_str!("fixtures/solidity/large_protocol.sol"),
    },
];

fn parse_fixture(fixture: &Fixture) -> SolidityAst {
    parse_source(black_box(fixture.source), Path::new(fixture.file_name))
        .expect("benchmark fixture should parse")
}

fn configure_group(group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>) {
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(1));
}

fn benchmark_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("solidity_parse");
    configure_group(&mut group);
    for fixture in &FIXTURES {
        group.throughput(Throughput::Bytes(fixture.source.len() as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(fixture.name),
            fixture,
            |b, input| {
                b.iter(|| black_box(parse_fixture(input)));
            },
        );
    }
    group.finish();
}

fn benchmark_extract(c: &mut Criterion) {
    let mut group = c.benchmark_group("solidity_extract");
    configure_group(&mut group);
    for fixture in &FIXTURES {
        let ast = parse_fixture(fixture);
        group.throughput(Throughput::Bytes(fixture.source.len() as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(fixture.name),
            &ast,
            |b, input| {
                b.iter(|| black_box(extract_solidity(input)));
            },
        );
    }
    group.finish();
}

fn benchmark_analyze(c: &mut Criterion) {
    let config = SolidityLanguageConfig::default();
    let mut group = c.benchmark_group("solidity_analyze");
    configure_group(&mut group);
    for fixture in &FIXTURES {
        let ast = parse_fixture(fixture);
        group.throughput(Throughput::Bytes(fixture.source.len() as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(fixture.name),
            &ast,
            |b, input| {
                b.iter(|| black_box(analyze_solidity_file(input, COMPLEXITY_THRESHOLD, &config)));
            },
        );
    }
    group.finish();
}

fn benchmark_parse_and_analyze(c: &mut Criterion) {
    let config = SolidityLanguageConfig::default();
    let mut group = c.benchmark_group("solidity_parse_and_analyze");
    configure_group(&mut group);
    for fixture in &FIXTURES {
        group.throughput(Throughput::Bytes(fixture.source.len() as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(fixture.name),
            fixture,
            |b, input| {
                b.iter(|| {
                    let ast = parse_fixture(input);
                    black_box(analyze_solidity_file(&ast, COMPLEXITY_THRESHOLD, &config))
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    benchmark_parse,
    benchmark_extract,
    benchmark_analyze,
    benchmark_parse_and_analyze
);
criterion_main!(benches);
