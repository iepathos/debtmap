# Solidity Parsing And Analysis Baseline

Recorded on 2026-07-25 after the single-pass structural extraction refactor.
These results are a local comparison baseline, not a CI performance threshold.

## Environment

- Build profile: Cargo `dev` (unoptimized Debtmap code, optimized dependencies)
- Command: `cargo bench --profile dev --bench solidity_parsing_bench -- --noplot`
- Rust: `rustc 1.89.0 (29483883e 2025-08-04)`
- Host: Apple M2 Pro, Darwin 24.5.0 arm64

## Fixtures

| Fixture | Lines | Bytes |
| --- | ---: | ---: |
| Small token | 14 | 353 |
| Medium pool | 86 | 2,823 |
| Large protocol | 245 | 8,530 |

## Criterion Estimates

The values below are the middle estimates from Criterion's reported confidence
intervals.

| Phase | Small | Medium | Large |
| --- | ---: | ---: | ---: |
| Parse | 18.827 µs | 222.61 µs | 666.65 µs |
| Structural extraction | 25.251 µs | 211.10 µs | 594.79 µs |
| Analyze pre-parsed AST | 1.9958 ms | 7.2253 ms | 21.739 ms |
| Parse and analyze | 2.0045 ms | 7.4960 ms | 22.493 ms |

The benchmark intentionally separates parsing, extraction, analysis of an
already parsed AST, and end-to-end parse plus analysis. This keeps future
changes to grammar parsing distinguishable from changes to Solidity analysis
passes.

Compared with the first run before signature extraction was limited to
signature nodes, structural extraction improved by about 12% for the small
fixture and 27% for both the medium and large fixtures. Pre-parsed analysis
also improved by roughly 1-2%.
