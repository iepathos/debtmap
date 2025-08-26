---
number: 63
title: Clear Debt Type Labeling System
category: optimization
priority: high
status: draft
dependencies: [35]
created: 2025-08-26
---

# Specification 63: Clear Debt Type Labeling System

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [35 - Debt Pattern Unified Scoring Integration]

## Context

The current debt type labeling system uses a `DebtType::Risk` variant that is ambiguous and misleading. This variant is used as a catch-all for functions with moderate complexity (cyclomatic 6-10), but the name "RISK" suggests it should be about coverage or security risks. Users have reported confusion when seeing functions labeled as "RISK" when the actual issue is moderate complexity.

The `identify_risk_factors` function can include various factors:
- Moderate complexity (cyclomatic > 5)
- Cognitive complexity (> 8)
- Long functions (> 50 lines)
- Low coverage (< 50%)

However, the primary categorization logic uses complexity thresholds:
- Functions with cyclomatic > 10 become `ComplexityHotspot`
- Functions with cyclomatic 6-10 become `Risk`
- Functions with low coverage but low complexity also become `Risk`

This creates confusion where a function with good coverage but moderate complexity shows as "RISK", making users think it's about coverage when it's actually about complexity.

## Objective

Replace the ambiguous `DebtType::Risk` variant with clear, specific debt type labels that immediately communicate the nature of the technical debt. Each label should be self-explanatory and actionable, helping developers understand what needs to be fixed without examining the details.

## Requirements

### Functional Requirements

1. **Replace ambiguous Risk variant**: Remove `DebtType::Risk` and replace with specific variants
2. **Create coverage-specific variant**: Add `DebtType::CoverageGap` for low coverage issues
3. **Create moderate complexity variant**: Add `DebtType::ModerateComplexity` for cyclomatic 6-10
4. **Handle compound issues**: Implement priority-based selection for functions with multiple issues
5. **Maintain backward compatibility**: Update all existing code paths that use `DebtType::Risk`
6. **Update formatters**: Ensure all output formats display the new labels correctly

### Non-Functional Requirements

1. **Clear naming**: Each debt type name must immediately convey the issue type
2. **Consistent thresholds**: Use consistent, documented thresholds for categorization
3. **Performance**: No regression in analysis performance
4. **Extensibility**: Design should allow easy addition of new debt types

## Acceptance Criteria

- [ ] `DebtType::Risk` variant is completely removed from the codebase
- [ ] New `DebtType::CoverageGap` variant exists with coverage percentage and urgency fields
- [ ] New `DebtType::ModerateComplexity` variant exists with complexity metrics
- [ ] Functions with < 50% coverage are labeled as "COVERAGE GAP" in output
- [ ] Functions with cyclomatic 6-10 are labeled as "MODERATE COMPLEXITY" in output
- [ ] Functions with cyclomatic > 10 remain labeled as "COMPLEXITY HOTSPOT"
- [ ] Priority system correctly selects primary issue when multiple problems exist
- [ ] All formatters (terminal, markdown, JSON) display new labels correctly
- [ ] Existing tests are updated and all pass
- [ ] New tests validate the categorization logic
- [ ] Documentation is updated with new debt type definitions

## Technical Details

### Implementation Approach

1. **Update DebtType enum** in `src/priority/mod.rs`:
   - Remove `Risk` variant
   - Add `CoverageGap { coverage_pct: f64, urgency: f64 }`
   - Add `ModerateComplexity { cyclomatic: usize, cognitive: usize }`

2. **Create prioritized categorization** in `src/priority/unified_scorer.rs`:
   - Implement priority-based selection for primary debt type
   - Priority order: Security > Coverage < 30% > Complexity > 15 > Coverage < 50% > Complexity > 10 > Moderate Complexity

3. **Update all pattern matching** across the codebase:
   - Replace all `DebtType::Risk` matches with appropriate new variants
   - Update scoring calculations
   - Update formatters

### Architecture Changes

The debt categorization will use a priority-based system:

```rust
fn categorize_debt(func: &FunctionMetrics, coverage: Option<f64>) -> Option<DebtType> {
    // Priority 1: Security issues (existing)
    if has_security_issue(func) {
        return Some(DebtType::Security { ... });
    }
    
    // Priority 2: Critical coverage gaps
    if let Some(cov) = coverage {
        if cov < 0.3 {
            return Some(DebtType::CoverageGap {
                coverage_pct: cov * 100.0,
                urgency: calculate_coverage_urgency(cov, func),
            });
        }
    }
    
    // Priority 3: High complexity
    if func.cyclomatic > 15 {
        return Some(DebtType::ComplexityHotspot { ... });
    }
    
    // Priority 4: Moderate coverage gaps
    if let Some(cov) = coverage {
        if cov < 0.5 {
            return Some(DebtType::CoverageGap {
                coverage_pct: cov * 100.0,
                urgency: calculate_coverage_urgency(cov, func),
            });
        }
    }
    
    // Priority 5: Moderate complexity
    if func.cyclomatic > 10 {
        return Some(DebtType::ComplexityHotspot { ... });
    }
    
    if func.cyclomatic >= 6 {
        return Some(DebtType::ModerateComplexity {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        });
    }
    
    None
}
```

### Data Structures

```rust
pub enum DebtType {
    // Security issues
    BasicSecurity { ... },
    HardcodedSecrets { ... },
    SqlInjectionRisk { ... },
    UnsafeCode { ... },
    WeakCryptography { ... },
    
    // Coverage issues
    CoverageGap {
        coverage_pct: f64,
        urgency: f64,
    },
    
    // Complexity issues
    ModerateComplexity {
        cyclomatic: usize,
        cognitive: usize,
    },
    ComplexityHotspot {
        cyclomatic: usize,
        cognitive: usize,
        nesting: usize,
    },
    
    // Performance issues
    NestedLoops { ... },
    BlockingIO { ... },
    AllocationInefficiency { ... },
    
    // Code quality issues
    DeadCode { ... },
    Duplication { ... },
    ErrorSwallowing { ... },
    
    // Test-specific issues
    TestComplexityHotspot { ... },
    TestTodo { ... },
    TestDuplication { ... },
}
```

### APIs and Interfaces

The public API remains unchanged. The categorization logic is internal to the unified scorer. Output formats will show the new, clearer labels.

## Dependencies

- **Prerequisites**: Spec 35 (Debt Pattern Unified Scoring Integration) must be implemented
- **Affected Components**:
  - `src/priority/mod.rs` - DebtType enum definition
  - `src/priority/unified_scorer.rs` - Categorization logic
  - `src/priority/formatter.rs` - Terminal output formatting
  - `src/priority/formatter_markdown.rs` - Markdown output formatting
  - `src/priority/formatter_verbosity.rs` - Verbose output formatting
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**:
  - Test categorization logic with various complexity and coverage combinations
  - Verify priority-based selection when multiple issues exist
  - Test edge cases (exact threshold values)
  
- **Integration Tests**:
  - Create test files with known complexity and coverage values
  - Verify correct labels appear in all output formats
  - Test with real-world codebases to validate categorization
  
- **Performance Tests**:
  - Ensure no performance regression in analysis
  - Verify memory usage remains constant
  
- **User Acceptance**:
  - Run on multiple real projects
  - Verify labels are clear and actionable
  - Confirm no false positives or incorrect categorizations

## Documentation Requirements

- **Code Documentation**:
  - Document all new DebtType variants with clear descriptions
  - Add examples showing when each variant is used
  - Document the priority-based categorization logic
  
- **User Documentation**:
  - Update README with new debt type definitions
  - Provide examples of each debt type in output
  - Explain the prioritization system for compound issues
  
- **Architecture Updates**:
  - Update ARCHITECTURE.md with new debt categorization system
  - Document the rationale for priority ordering

## Implementation Notes

1. **Migration path**: Use compiler to find all uses of `DebtType::Risk` and update systematically
2. **Testing approach**: Create comprehensive test suite before making changes
3. **Rollout**: Can be done in a single commit as it's an internal refactor
4. **Backward compatibility**: JSON output structure should remain compatible, only label strings change
5. **Future extensibility**: Design allows easy addition of new debt types without breaking existing code

## Migration and Compatibility

During the prototype phase, breaking changes are allowed. However, this change should be mostly transparent to users:

1. **CLI interface**: No changes needed
2. **Output format**: Structure remains the same, only labels change
3. **Configuration**: No changes to configuration files
4. **API**: Library API remains unchanged

The main visible change will be in the output labels:
- `"RISK"` → `"COVERAGE GAP"` or `"MODERATE COMPLEXITY"`
- More specific and actionable labels throughout

This change will make the tool's output significantly clearer and more actionable, reducing user confusion and improving the developer experience.