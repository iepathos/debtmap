---
number: 139
title: Struct-Ownership-Based God Object Splitting (SUPERSEDED)
category: optimization
priority: high
status: superseded
dependencies: []
created: 2025-01-23
superseded_by: [143, 144, 145]
---

# Specification 139: Struct-Ownership-Based God Object Splitting

**Status**: ⚠️ **SUPERSEDED** - This spec has been split into three focused specifications:
- **Spec 143**: Struct Ownership and Domain Classification (Phase 1) - **IMPLEMENT THIS FIRST**
- **Spec 144**: Call Graph Integration for Cohesion Scoring (Phase 2)
- **Spec 145**: Multi-Language God Object Support (Phase 3)

---

## Reason for Split

The original spec was too ambitious for a single implementation cycle. The evaluation identified:

1. **Scope Too Large**: 7 functional requirements + 3 NFRs + 40+ acceptance criteria
2. **Mixed Concerns**: Struct ownership, cohesion scoring, and multi-language support are independent features
3. **Implementation Risk**: Estimated 3-4 weeks for experienced developer (too long)
4. **Dependency Confusion**: Call graph integration and multi-language support don't depend on each other

## How the Specs Were Split

### Phase 1: Foundation (Spec 143) - HIGH PRIORITY
**What**: Struct ownership tracking and domain-based classification for Rust
**Why First**: Solves the core problem (bad recommendations for config.rs)
**Estimated Time**: 1-2 weeks
**Key Deliverables**:
- Struct ownership analyzer for Rust
- Enhanced domain classification (10+ patterns)
- Size validation (5-40 methods)
- Priority assignment (High/Medium/Low)
- Integration test on config.rs

**Start Here**: Read and implement Spec 143 first.

### Phase 2: Quality Metrics (Spec 144) - MEDIUM PRIORITY
**What**: Call graph integration for cohesion scoring and dependency analysis
**Why Second**: Adds quantitative validation to Phase 1 recommendations
**Estimated Time**: 1 week
**Key Deliverables**:
- Cohesion score calculation (0.0-1.0)
- Dependency tracking (dependencies_in/out)
- Circular dependency detection
- Quality-based priority refinement

**Dependencies**: Requires Spec 143 to be complete

### Phase 3: Multi-Language (Spec 145) - LOW PRIORITY
**What**: Extend to Python, JavaScript, TypeScript
**Why Third**: Broader impact but not critical for initial value
**Estimated Time**: 2 weeks
**Key Deliverables**:
- Python class ownership tracking
- JavaScript/TypeScript class ownership tracking
- Language-specific domain patterns
- Unified multi-language interface

**Dependencies**: Requires Spec 143; Spec 144 optional but recommended

## Migration Guide for Readers

**If you are implementing this work**:
1. ✅ Read Spec 143 in full - this is your starting point
2. ✅ Implement Spec 143 completely (including all tests)
3. ⏸️ Pause and validate: Does config.rs get good recommendations?
4. ✅ Read and implement Spec 144 (cohesion scoring)
5. ⏸️ Pause and validate: Do cohesion scores make sense?
6. ✅ Read and implement Spec 145 (multi-language) - only if needed

**If you are reviewing this work**:
- Refer to the individual specs (143, 144, 145) for acceptance criteria
- Each spec has its own success metrics and test strategy
- Phases can be merged/deployed independently

## Original Problem Statement

(Preserved for historical context)

**Current Problem**: Debtmap's god object detection correctly identifies large files (e.g., `config.rs` with 2459 lines, 201 functions), but generates poor quality refactoring recommendations that would harm code quality rather than improve it.

**Real-World Example from config.rs**:
```
Current Recommendation (BAD):
├─ config_core_operations.rs - Core Operations (151 methods, ~3020 lines)
└─ config_data_access.rs - Data Access (21 methods, ~420 lines)
```

**Why This is Wrong**:
1. **Still a god object**: 151 methods in one module violates single responsibility
2. **Naive grouping**: Groups `get_config()`, `get_ignore_patterns()`, `get_error_handling_config()` together just because they all start with "get"
3. **Ignores struct ownership**: Doesn't consider that methods belong to different structs (ScoringWeights, ThresholdsConfig, ErrorHandlingConfig, etc.)
4. **No size validation**: Recommends modules that exceed reasonable bounds (>40 methods)

## Implementation Roadmap

```
v0.3.0 (Spec 143)
├─ Struct ownership tracking (Rust)
├─ Domain-based classification
├─ Size validation (5-40 methods)
└─ config.rs test case passes

v0.3.1 (Spec 144)
├─ Cohesion score calculation
├─ Dependency analysis
├─ Circular dependency detection
└─ Quality-based priority

v0.4.0 (Spec 145)
├─ Python class ownership
├─ JavaScript/TypeScript class ownership
├─ Language-specific domain patterns
└─ Unified multi-language interface
```

## Key Decisions from Evaluation

### ✅ Kept in Phase 1 (Spec 143)
- Struct ownership tracking (Rust-only)
- Domain classification with 10+ patterns
- Size validation (5-40 methods)
- Basic ModuleSplit fields: `structs_to_move`, `method_count`, `warning`, `priority`
- Simple 2-level splitting for oversized modules (not recursive)

### ⏸️ Deferred to Phase 2 (Spec 144)
- Cohesion score calculation (requires call graph)
- Dependency analysis (`dependencies_in/out`)
- Circular dependency detection
- Quality-based priority refinement

### ⏸️ Deferred to Phase 3 (Spec 145)
- Python class method tracking
- TypeScript/JavaScript support
- Language-specific domain patterns
- Configurable domain patterns (hard-coded in Phase 1)

### ❌ Removed Entirely
- Recursive splitting (replaced with simple 2-level approach)
- Machine learning patterns (too complex)
- Interactive refinement (deferred to v0.6.0+)

## References

**Primary Specs** (read these instead):
- [Spec 143: Struct Ownership Foundation](./143-struct-ownership-god-object-foundation.md)
- [Spec 144: Call Graph Cohesion Scoring](./144-call-graph-cohesion-scoring.md)
- [Spec 145: Multi-Language Support](./145-multi-language-god-object-support.md)

**Related Work**:
- src/organization/god_object_analysis.rs - Current implementation
- src/organization/god_object_detector.rs - Detection logic
- src/config.rs - Real-world test case (2459 lines, 27 structs)

## Notes

This spec remains in the repository for historical context and to document the evaluation process. All implementation work should reference Specs 143, 144, and 145.

The split was based on a comprehensive evaluation that assessed:
- ✅ Technical feasibility
- ✅ Alignment with codebase principles
- ✅ Implementation risk
- ✅ Dependencies and ordering
- ✅ Value delivery timeline

**Bottom Line**: Start with Spec 143. It solves 80% of the problem in 20% of the time.
