# Evidence for Exponential Scaling

## Source Definitions Found
- ScalingConfig struct: src/priority/scoring/scaling.rs:16-28
- Default exponents: src/priority/scoring/scaling.rs:30-43
- apply_exponential_scaling function: src/priority/scoring/scaling.rs:45-78
- apply_risk_boosts function: src/priority/scoring/scaling.rs:80-109
- calculate_final_score function: src/priority/scoring/scaling.rs:119-173
- Default scoring weights: src/config/scoring.rs:186-198
- DebtType enum (GodObject): src/priority/debt_types.rs:145-154

## Test Examples Found
- test_exponential_scaling_god_object: src/priority/scoring/scaling.rs:271-293
- test_exponential_scaling_creates_separation: src/priority/scoring/scaling.rs:295-325
- test_risk_boosts_multiply: src/priority/scoring/scaling.rs:327-352
- test_calculate_final_score_integration: src/priority/scoring/scaling.rs:354-383
- test_error_swallowing_boost: src/priority/scoring/scaling.rs:456-510
- Property tests for monotonicity: src/priority/scoring/scaling.rs:513-694

## Documentation References
- Parent content source: book/src/scoring-strategies.md:1281-1575
- SUMMARY.md reference: book/src/SUMMARY.md:57

## Validation Results
- All config fields verified against ScalingConfig struct definition
- All exponent values match Default::default() implementation
- All risk boost multipliers verified against apply_risk_boosts() function
- Thresholds verified (total_deps > 15, cyclomatic > 30/20/15, coverage < 10%)
- Error swallowing boost documented (was previously missing)
- Default scoring weights corrected to 50%/35%/15%

## Issues Fixed
1. CRITICAL: Created missing subsection file (was referenced in SUMMARY.md but didn't exist)
2. HIGH: Fixed outdated configuration section - now accurately states exponents are not TOML-configurable
3. MEDIUM: Corrected pattern-specific exponents table to match actual DebtType handling
4. MEDIUM: Fixed risk multipliers table (10+ callers → >15 total deps, removed incorrect low-coverage boost)
5. MEDIUM: Added missing error_swallowing_boost documentation (1.15x)
6. LOW: Added god_module_exponent documentation (1.4)
7. MEDIUM: Corrected scoring weights to 50%/35%/15% instead of 40%/40%/20%

## Content Metrics
- Lines of content: 249 (minimum: 50)
- Level-2 headings: 15 (minimum: 3)
- Code examples: 9 (minimum: 2)
- Source references: 12 (minimum: 1)

## Quality Gates
- Meets minimum content requirements
- All critical issues resolved
- All high severity issues resolved
- All examples grounded in codebase
- All source references validated

Status: READY TO COMMIT
