---
number: 201
title: Invert Multi-Pass Analysis Default Behavior
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-12-03
---

# Specification 201: Invert Multi-Pass Analysis Default Behavior

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

Multi-pass analysis is a core feature of debtmap that distinguishes between genuine logical complexity and formatting artifacts by performing two analyses (raw and normalized) and comparing the results. This provides significant value:

- Separates signal (logical complexity) from noise (formatting)
- Provides actionable attribution showing complexity sources
- Generates targeted refactoring recommendations
- Validates refactoring effectiveness

Currently, multi-pass analysis is opt-in via `--multi-pass` flag. However, since this analysis provides substantially more valuable insights and the performance overhead is acceptable (typically 15-25%), it should be enabled by default.

**Current State:**
```bash
# Users must explicitly enable multi-pass
debtmap analyze . --multi-pass

# Default behavior is single-pass (less informative)
debtmap analyze .
```

**Desired State:**
```bash
# Multi-pass is default (more informative)
debtmap analyze .

# Users can opt-out if needed for performance
debtmap analyze . --no-multi-pass
# or
debtmap analyze . --single-pass
```

## Objective

Invert the multi-pass analysis flag logic so that multi-pass analysis runs by default, with users able to opt-out via `--no-multi-pass` or `--single-pass` flags when needed for performance or compatibility reasons.

## Requirements

### Functional Requirements

1. **Default Behavior Change**
   - Multi-pass analysis MUST run by default when no flags are specified
   - Single-pass analysis MUST only run when explicitly requested

2. **CLI Flag Updates**
   - Remove the `--multi-pass` flag (or deprecate with a message)
   - Add `--no-multi-pass` flag to disable multi-pass analysis
   - Add `--single-pass` as an alias for `--no-multi-pass`
   - Both flags should be functionally equivalent

3. **Backward Compatibility**
   - If `--multi-pass` is still present (deprecation path), it should be a no-op with a warning
   - Existing scripts using default behavior will get enhanced output (breaking change documented)
   - Configuration file settings should continue to work

4. **Environment Variable Support**
   - Add `DEBTMAP_SINGLE_PASS=1` environment variable to disable multi-pass
   - Environment variable should have same effect as `--no-multi-pass`

5. **Documentation Updates**
   - Update book/src/multi-pass-analysis.md to reflect new default
   - Update book/src/cli-reference.md with new flags
   - Update book/src/getting-started.md examples
   - Update book/src/troubleshooting.md for performance issues

### Non-Functional Requirements

1. **Performance**
   - Default overhead should remain ≤25% compared to legacy single-pass
   - Performance warnings should trigger if overhead exceeds threshold
   - Users with performance constraints can easily opt-out

2. **User Experience**
   - Default behavior provides maximum insight without configuration
   - Opt-out path is clear and well-documented
   - Migration guide helps users adapt existing workflows

3. **Testing**
   - All existing tests must pass with new default
   - Add tests for `--no-multi-pass` flag
   - Add tests for `--single-pass` alias
   - Add tests for environment variable behavior

## Acceptance Criteria

- [ ] `debtmap analyze .` runs multi-pass analysis by default
- [ ] `--no-multi-pass` flag disables multi-pass analysis
- [ ] `--single-pass` flag works as alias for `--no-multi-pass`
- [ ] `DEBTMAP_SINGLE_PASS=1` environment variable disables multi-pass
- [ ] `--multi-pass` flag either removed or deprecated with warning
- [ ] CLI help text updated to reflect new behavior
- [ ] All documentation updated (multi-pass-analysis.md, cli-reference.md, getting-started.md)
- [ ] All existing tests pass with new default
- [ ] New tests added for opt-out flags and environment variable
- [ ] Performance characteristics remain within acceptable bounds
- [ ] Backward compatibility handled gracefully

## Technical Details

### Implementation Approach

**1. CLI Argument Changes (src/cli.rs)**

Current:
```rust
/// Enable multi-pass analysis with attribution
#[arg(long = "multi-pass")]
multi_pass: bool,
```

New:
```rust
/// Disable multi-pass analysis (use single-pass for performance)
#[arg(long = "no-multi-pass", visible_alias = "single-pass")]
no_multi_pass: bool,
```

**2. Default Value Logic**

In `AnalyzeConfig` construction, invert the logic:

Current:
```rust
multi_pass: args.multi_pass,  // false by default
```

New:
```rust
multi_pass: !args.no_multi_pass,  // true by default, disabled by flag
```

**3. Environment Variable Support**

Add environment variable check:
```rust
let single_pass_env = std::env::var("DEBTMAP_SINGLE_PASS")
    .ok()
    .and_then(|v| v.parse::<bool>().ok())
    .unwrap_or(false);

multi_pass: !args.no_multi_pass && !single_pass_env,
```

**4. Deprecation Path (Optional)**

If keeping `--multi-pass` for backward compatibility:
```rust
/// Deprecated: Multi-pass is now default. Use --no-multi-pass to disable.
#[arg(long = "multi-pass", hide = true)]
multi_pass_deprecated: bool,

// In initialization:
if multi_pass_deprecated {
    eprintln!("Warning: --multi-pass is deprecated (now default behavior). Use --no-multi-pass to disable.");
}
```

### Architecture Changes

No significant architectural changes required. The multi-pass analyzer infrastructure remains unchanged; only the default invocation behavior changes.

### Data Structures

No changes to existing data structures. The `AnalyzeConfig.multi_pass` field retains its boolean type but with inverted default semantics.

### APIs and Interfaces

**CLI Interface Changes:**
- Remove: `--multi-pass`
- Add: `--no-multi-pass`
- Add: `--single-pass` (alias)

**Environment Variables:**
- Add: `DEBTMAP_SINGLE_PASS`

**Configuration File:**
No changes needed. Existing config files should continue to work.

## Dependencies

**Prerequisites:** None

**Affected Components:**
- `src/cli.rs` - CLI argument definitions
- `src/commands/analyze.rs` - Config construction logic
- `book/src/multi-pass-analysis.md` - Documentation
- `book/src/cli-reference.md` - CLI reference docs
- `book/src/getting-started.md` - Getting started examples
- `book/src/examples.md` - Example commands

**External Dependencies:** None

## Testing Strategy

### Unit Tests

**Test new CLI flag parsing:**
```rust
#[test]
fn test_no_multi_pass_flag() {
    let args = vec!["debtmap", "analyze", ".", "--no-multi-pass"];
    let cli = Cli::parse_from(args);
    match cli.command {
        Commands::Analyze { no_multi_pass, .. } => {
            assert!(no_multi_pass);
        }
        _ => panic!("Expected Analyze command"),
    }
}

#[test]
fn test_single_pass_alias() {
    let args = vec!["debtmap", "analyze", ".", "--single-pass"];
    let cli = Cli::parse_from(args);
    match cli.command {
        Commands::Analyze { no_multi_pass, .. } => {
            assert!(no_multi_pass);
        }
        _ => panic!("Expected Analyze command"),
    }
}

#[test]
fn test_default_enables_multi_pass() {
    let args = vec!["debtmap", "analyze", "."];
    let cli = Cli::parse_from(args);
    match cli.command {
        Commands::Analyze { no_multi_pass, .. } => {
            assert!(!no_multi_pass);  // multi-pass is default
        }
        _ => panic!("Expected Analyze command"),
    }
}
```

### Integration Tests

**Test environment variable:**
```rust
#[test]
fn test_single_pass_env_var() {
    std::env::set_var("DEBTMAP_SINGLE_PASS", "1");
    // Run analysis and verify single-pass behavior
    std::env::remove_var("DEBTMAP_SINGLE_PASS");
}
```

**Test actual analysis behavior:**
- Verify default analysis includes attribution data
- Verify `--no-multi-pass` produces single-pass output
- Verify performance characteristics

### Performance Tests

- Benchmark default multi-pass overhead
- Verify opt-out restores single-pass performance
- Ensure overhead stays ≤25%

### User Acceptance

**User Story 1: Default rich analysis**
```bash
# User runs default analysis and gets comprehensive results
debtmap analyze src/
# Output includes attribution, insights, recommendations
```

**User Story 2: Performance-constrained opt-out**
```bash
# User in CI with tight time budget opts out
debtmap analyze . --no-multi-pass
# Fast single-pass analysis completes quickly
```

**User Story 3: Environment-based configuration**
```bash
# CI pipeline sets environment variable
export DEBTMAP_SINGLE_PASS=1
debtmap analyze .
# Single-pass runs without flag in command
```

## Documentation Requirements

### Code Documentation

- Update CLI arg documentation in `src/cli.rs`
- Add migration notes in CHANGELOG.md
- Update code comments referencing multi-pass behavior

### User Documentation

**book/src/multi-pass-analysis.md:**
- Update introduction to note multi-pass is default
- Change examples to show opt-out syntax
- Add migration section for existing users
- Update "When to Use" sections

**book/src/cli-reference.md:**
- Remove `--multi-pass` flag documentation
- Add `--no-multi-pass` and `--single-pass` documentation
- Add `DEBTMAP_SINGLE_PASS` environment variable

**book/src/getting-started.md:**
- Update examples to reflect new defaults
- Add note about multi-pass being default
- Mention opt-out for performance needs

**book/src/troubleshooting.md:**
- Add "Analysis is slow" section recommending `--no-multi-pass`
- Document performance trade-offs

### Architecture Updates

No ARCHITECTURE.md updates needed - this is a UX change, not architectural.

## Implementation Notes

### Migration Strategy

**For End Users:**
1. Existing workflows using default behavior get enhanced output (acceptable breaking change)
2. Users who need old behavior use `--no-multi-pass`
3. Migration guide in release notes explains changes

**For CI/CD:**
1. Pipelines may see increased runtime (15-25%)
2. Can opt-out with `--no-multi-pass` or `DEBTMAP_SINGLE_PASS=1`
3. Document in release notes

### Performance Considerations

**Default Overhead:**
- Multi-pass adds ~15-25% overhead
- Acceptable for default behavior given value add
- Users with performance constraints can opt-out

**Monitoring:**
- Performance tracking remains available
- Warnings trigger if overhead exceeds 25%
- Users get clear feedback about performance impact

### Edge Cases

**Conflicting Flags:**
- `--no-multi-pass --attribution` should warn that attribution requires multi-pass
- Disable attribution if multi-pass is off

**Environment Variable Precedence:**
- Command-line flag should override environment variable
- `--no-multi-pass` takes precedence over default

## Migration and Compatibility

### Breaking Changes

**Default Behavior:**
- Users running `debtmap analyze` will see different output
- Analysis will take longer (15-25% overhead)
- Output format includes attribution data

**Mitigation:**
- Document clearly in release notes
- Provide `--no-multi-pass` for quick opt-out
- Explain benefits of new default

### Compatibility Considerations

**Backward Compatibility:**
- Existing scripts work but get different output
- Performance-sensitive workflows need update
- Config files continue to work

**Forward Compatibility:**
- New flag names are stable
- Environment variable provides flexibility
- Future enhancements build on multi-pass foundation

### Migration Guide

**For Users:**
```bash
# Before (v0.x): Multi-pass was opt-in
debtmap analyze . --multi-pass

# After (v1.0): Multi-pass is default
debtmap analyze .

# If you need single-pass for performance:
debtmap analyze . --no-multi-pass
# or
debtmap analyze . --single-pass
```

**For CI/CD:**
```yaml
# Option 1: Set environment variable
env:
  DEBTMAP_SINGLE_PASS: 1
steps:
  - run: debtmap analyze .

# Option 2: Use command flag
steps:
  - run: debtmap analyze . --no-multi-pass
```

## Risks and Challenges

### Risk: Performance Impact on CI

**Description:** CI pipelines may see 15-25% increase in analysis time

**Mitigation:**
- Document opt-out clearly
- Provide environment variable for easy CI configuration
- Include migration guide in release notes

**Severity:** Low (easily mitigated)

### Risk: Output Format Changes

**Description:** Tools parsing debtmap output may break

**Mitigation:**
- Multi-pass output is already available, so parsers should handle it
- JSON output format remains stable
- Document changes in release notes

**Severity:** Low (JSON format is stable)

### Risk: User Confusion

**Description:** Users may not understand why default changed

**Mitigation:**
- Clear documentation explaining benefits
- Migration guide with examples
- Troubleshooting section for performance issues

**Severity:** Low (good documentation prevents confusion)

## Success Metrics

**Adoption:**
- % of users using default (multi-pass) vs opt-out
- Target: >80% use default behavior

**Performance:**
- Average overhead stays ≤25%
- Target: 15-25% typical overhead

**User Satisfaction:**
- GitHub issues related to confusion
- Target: <5 issues in first 3 months

**Code Quality:**
- All tests pass
- No performance regressions beyond expected overhead
- Clean deprecation path

## Future Enhancements

**Spec 84 Integration:**
When Spec 84 (Detailed AST-Based Source Mapping) is implemented, the enhanced attribution will make multi-pass analysis even more valuable as the default.

**Adaptive Performance:**
Future enhancement could automatically detect CI environments and adjust defaults:
```rust
// Pseudo-code for future enhancement
let is_ci = std::env::var("CI").is_ok();
let default_multi_pass = !is_ci || user_preference;
```

**Progressive Enhancement:**
Could implement "quick multi-pass" mode that trades some accuracy for speed:
```rust
/// Quick multi-pass mode (reduced overhead)
#[arg(long = "quick-multi-pass")]
quick_multi_pass: bool,
```

## Related Specifications

- **Spec 84**: Detailed AST-Based Source Mapping - Will enhance multi-pass attribution
- **Spec 81**: Advanced Memory Tracking - Related to performance monitoring
- **Spec 82**: Enhanced Insight Generation - Benefits from multi-pass attribution

## References

- [Multi-Pass Analysis Documentation](../book/src/multi-pass-analysis.md)
- [CLI Reference](../book/src/cli-reference.md)
- [Performance Considerations](../book/src/multi-pass-analysis.md#performance-considerations)
