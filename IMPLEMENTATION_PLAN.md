# Fix Skipped Tests Implementation Plan

## Stage 1: Fix Macro Parsing in CallGraphExtractor ✅
**Goal**: Fix the 7 ignored macro-related tests in rust_call_graph.rs
**Success Criteria**: All macro tests pass without being ignored
**Tests**: 
- test_format_macro_with_function_calls ✅
- test_println_macro_with_expressions ✅ 
- test_assert_macro_with_function_calls ✅
- test_hashmap_macro_with_function_calls ✅
- test_macro_stats_tracking ✅
- test_nested_macros ✅
- test_logging_macros ✅
**Status**: Completed

### Implemented Solutions:
1. Added `visit_stmt` to handle statement-level macros (println!, assert!, etc.)
2. Fixed `parse_format_macro` to visit ALL arguments including format string
3. Added special handling for hashmap-like macros with key-value pairs
4. Fixed test functions to use correct FunctionId lookups
5. Corrected logging test expectations (call graph deduplicates callees)

## Stage 2: Optimize Slow Integration Test
**Goal**: Make test_comprehensive_false_positive_patterns run faster
**Success Criteria**: Test runs in under 5 seconds
**Tests**: test_comprehensive_false_positive_patterns
**Status**: Not Started

### Approach:
1. Split into individual unit tests
2. Use in-memory analysis instead of subprocess calls
3. Cache compilation results where possible

## Stage 3: Verify and Clean Up
**Goal**: Ensure all tests pass and remove #[ignore] attributes
**Success Criteria**: 
- All 8 tests run and pass
- No #[ignore] attributes remain
- CI pipeline passes
**Status**: Not Started