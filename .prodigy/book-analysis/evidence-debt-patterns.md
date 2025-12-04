# Evidence for Debt Patterns

## Source Definitions Found
- DebtType enum: src/priority/mod.rs:158-288
- DebtCategory enum: src/priority/mod.rs:290-296
- Category mapping function: src/priority/mod.rs:309-347
- ScatteredType struct: src/organization/codebase_type_analyzer.rs:30-48
- OrphanedFunctionGroup struct: src/organization/codebase_type_analyzer.rs:58-71
- UtilitiesModule struct: src/organization/codebase_type_analyzer.rs:74-80

## Key Findings
- **Actual debt type count**: 27 (not 25 as documented)
- **Missing types**: ScatteredType, OrphanedFunctions, UtilitiesSprawl
- **Category mappings verified**:
  - ComplexityHotspot → CodeQuality (line 340)
  - DeadCode → CodeQuality (line 341)
  - MagicValues → CodeQuality (line 345)
  - ScatteredType → Architecture (line 317)
  - OrphanedFunctions → Architecture (line 318)
  - UtilitiesSprawl → Architecture (line 319)

## Type Organization Debt Details
- ScatteredType: Detects types with methods scattered across multiple files
  - Fields: type_name, total_methods, file_count, severity
  - Severity levels: Low (2 files), Medium (3-5 files), High (6+ files)
  - Source: src/organization/codebase_type_analyzer.rs:30-48

- OrphanedFunctions: Detects functions that should be methods on a type
  - Fields: target_type, function_count, file_count
  - Source: src/organization/codebase_type_analyzer.rs:58-71

- UtilitiesSprawl: Detects utility modules with poor cohesion
  - Fields: function_count, distinct_types
  - Source: src/organization/codebase_type_analyzer.rs:74-80

## Validation Results
✓ All 27 debt types verified in DebtType enum
✓ All category mappings verified against implementation
✓ All field names match source code exactly
✓ Type organization debt patterns fully documented
✓ No broken references

## Discovery Notes
- Test directories found: ./proptest-regressions, ./specs, ./tests
- Source verified in: src/priority/mod.rs, src/organization/codebase_type_analyzer.rs
- Recommendation generation: src/priority/scoring/debt_item.rs:628-677
