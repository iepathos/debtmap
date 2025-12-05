---
number: 204
title: Lock-Free Context Sharing for Parallel Risk Analysis
category: parallel
priority: high
status: draft
dependencies: [202]
created: 2025-12-04
---

# Specification 204: Lock-Free Context Sharing for Parallel Risk Analysis

**Category**: parallel
**Priority**: high
**Status**: draft
**Dependencies**: Spec 202 (Contextual Risk in Priority Scoring)

## Context

Spec 202 successfully wired contextual risk through the priority scoring pipeline, but exposed a critical bug in parallel execution: `RiskAnalyzer::clone()` discards the `context_aggregator` field, causing all context providers (git history, dependency analysis, critical path) to be silently lost when cloned for thread-safe parallel execution.

### Current Behavior

```rust
// src/risk/mod.rs:106-114
impl Clone for RiskAnalyzer {
    fn clone(&self) -> Self {
        Self {
            strategy: self.strategy.box_clone(),
            debt_score: self.debt_score,
            debt_threshold: self.debt_threshold,
            context_aggregator: None, // ❌ BUG: Discarded!
        }
    }
}
```

This causes the parallel unified analysis path (`parallel_unified_analysis.rs:696`) to create risk analyzers without context:

```rust
let mut risk_analyzer_clone = context.risk_analyzer.cloned(); // Loses context!
```

Result: Zero contextual risk data appears in output despite `--context` flag.

### Root Cause

`ContextAggregator` uses `&mut self` for caching and contains trait objects (`Vec<Box<dyn ContextProvider>>`), making it non-trivially clonable. The current implementation chose to discard it rather than share it.

### Architectural Constraint

Context providers (`git_history`, `critical_path`, `dependency`) are **pure** - all `gather()` methods take `&self`. The only mutation is the cache in `ContextAggregator::analyze()`.

## Objective

Enable lock-free, thread-safe sharing of `ContextAggregator` across parallel analysis workers while preserving the pure functional interface of context providers and maintaining zero-cost parallelism.

## Requirements

### Functional Requirements

1. **Preserve Context in Clone**: `RiskAnalyzer::clone()` must preserve the `context_aggregator` field
2. **Thread-Safe Sharing**: Multiple parallel workers must safely access the same aggregator
3. **Cache Coherency**: Concurrent cache reads/writes must not cause data races
4. **Pure Provider Interface**: No changes to `ContextProvider::gather(&self)` signatures
5. **Backward Compatibility**: No breaking changes to public APIs

### Non-Functional Requirements

1. **Lock-Free Performance**: No mutex/lock contention in hot path (analyze function)
2. **Memory Efficiency**: Shared aggregator via `Arc`, not duplicated per worker
3. **Zero-Cost Abstraction**: No performance regression vs current (non-context) baseline
4. **Stillwater Alignment**: Follow "pure core, imperative shell" pattern
5. **Minimal Code Changes**: Surgical fix, not architectural rewrite

## Acceptance Criteria

- [ ] `RiskAnalyzer::clone()` preserves `context_aggregator` field using `Arc`
- [ ] `ContextAggregator` uses `DashMap` for lock-free concurrent cache access
- [ ] `ContextAggregator::analyze()` signature changes from `&mut self` to `&self`
- [ ] Parallel analysis successfully populates `contextual_risk` field in `UnifiedDebtItem`
- [ ] Running `debtmap analyze . --context --top 5` displays "CONTEXT RISK" sections
- [ ] Git history metrics (change frequency, bug density, etc.) appear in output
- [ ] Existing unit tests pass without modification
- [ ] New integration test validates context data flows through parallel path
- [ ] No performance regression in parallel analysis benchmarks
- [ ] Debug logging removed from production code

## Technical Details

### Implementation Approach

**Phase 1: Interior Mutability for Cache**

Change `ContextAggregator` to use `DashMap` (already a debtmap dependency) for lock-free caching:

```rust
// src/risk/context/mod.rs
use dashmap::DashMap;
use std::sync::Arc;

pub struct ContextAggregator {
    providers: Vec<Box<dyn ContextProvider>>,
    cache: Arc<DashMap<String, ContextMap>>, // Lock-free concurrent HashMap
}

impl ContextAggregator {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            cache: Arc::new(DashMap::new()),
        }
    }

    // Signature change: &mut self -> &self
    pub fn analyze(&self, target: &AnalysisTarget) -> ContextMap {
        let cache_key = format!("{}:{}", target.file_path.display(), target.function_name);

        // Check cache (lock-free read)
        if let Some(cached) = self.cache.get(&cache_key) {
            return cached.clone();
        }

        // Gather context from providers
        let mut context_map = ContextMap::new();
        for provider in &self.providers {
            match provider.gather(target) {
                Ok(context) => {
                    context_map.add(provider.name().to_string(), context);
                }
                Err(e) => {
                    log::debug!("Context provider {} failed: {}", provider.name(), e);
                }
            }
        }

        // Insert into cache (lock-free write)
        self.cache.insert(cache_key, context_map.clone());
        context_map
    }
}

impl Clone for ContextAggregator {
    fn clone(&self) -> Self {
        Self {
            providers: Vec::new(), // Don't clone providers (heavy)
            cache: Arc::clone(&self.cache), // Share cache via Arc
        }
    }
}
```

**Phase 2: Arc-Wrapped Sharing in RiskAnalyzer**

```rust
// src/risk/mod.rs
pub struct RiskAnalyzer {
    strategy: Box<dyn RiskCalculator>,
    debt_score: Option<f64>,
    debt_threshold: Option<f64>,
    context_aggregator: Option<Arc<ContextAggregator>>, // Wrapped in Arc
}

impl Clone for RiskAnalyzer {
    fn clone(&self) -> Self {
        Self {
            strategy: self.strategy.box_clone(),
            debt_score: self.debt_score,
            debt_threshold: self.debt_threshold,
            context_aggregator: self.context_aggregator.clone(), // Arc::clone is cheap!
        }
    }
}

impl RiskAnalyzer {
    pub fn with_context_aggregator(mut self, aggregator: ContextAggregator) -> Self {
        self.context_aggregator = Some(Arc::new(aggregator));
        self
    }

    pub fn analyze_function_with_context(&mut self, ...) -> (FunctionRisk, Option<ContextualRisk>) {
        // ...
        let contextual_risk = if let Some(ref aggregator) = self.context_aggregator {
            // aggregator is Arc<ContextAggregator>, deref to &ContextAggregator
            let target = AnalysisTarget { /* ... */ };
            let context_map = aggregator.analyze(&target); // Now &self instead of &mut self
            // ...
        }
        // ...
    }
}
```

**Phase 3: Update Call Sites**

Update signature change propagation:

```rust
// src/utils/risk_analyzer.rs - No changes needed (already &self after construction)
// src/builders/parallel_unified_analysis.rs - No changes needed (clone now works)
```

### Architecture Changes

**Before (Broken)**:
```
ContextAggregator (non-cloneable)
  └─ context_aggregator: None on clone
      └─ Parallel workers get no context ❌
```

**After (Fixed)**:
```
Arc<ContextAggregator>
  ├─ Worker 1: Arc::clone (cheap pointer copy)
  ├─ Worker 2: Arc::clone (cheap pointer copy)
  ├─ Worker 3: Arc::clone (cheap pointer copy)
  └─ Shared DashMap cache (lock-free)
      └─ All workers get context ✓
```

### Data Structures

**Key Changes**:
1. `ContextAggregator::cache`: `HashMap` → `Arc<DashMap<String, ContextMap>>`
2. `RiskAnalyzer::context_aggregator`: `Option<ContextAggregator>` → `Option<Arc<ContextAggregator>>`

**DashMap Benefits**:
- Lock-free concurrent reads (most common operation)
- Sharded internal locks minimize contention on writes
- Drop-in replacement for `HashMap` with concurrent API
- Already a dependency (used elsewhere in debtmap)

### Performance Characteristics

| Operation | Before | After | Notes |
|-----------|--------|-------|-------|
| Cache read (hit) | O(1) | O(1) | Lock-free, zero contention |
| Cache write | O(1) with &mut | O(1) lock-free | Sharded locks, minimal contention |
| RiskAnalyzer clone | O(1) | O(1) | Arc::clone is pointer copy |
| Memory overhead | N analyzers × aggregator | Arc shared | ~1MB saved per worker |

## Dependencies

- **Prerequisites**:
  - Spec 202 (Contextual Risk Priority Scoring infrastructure)
  - DashMap crate (already in `Cargo.toml`)

- **Affected Components**:
  - `src/risk/context/mod.rs` - ContextAggregator implementation
  - `src/risk/mod.rs` - RiskAnalyzer Clone implementation
  - `src/builders/parallel_unified_analysis.rs` - Already correct, will work after fix
  - `src/priority/scoring/construction.rs` - Remove debug logging

- **External Dependencies**:
  - `dashmap` (already present in Cargo.toml)
  - `std::sync::Arc` (standard library)

## Testing Strategy

### Unit Tests

**Test Context Cloning** (`src/risk/mod.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_analyzer_clone_preserves_context() {
        let aggregator = ContextAggregator::new()
            .with_provider(Box::new(MockProvider::new()));

        let analyzer = RiskAnalyzer::default()
            .with_context_aggregator(aggregator);

        let cloned = analyzer.clone();

        assert!(cloned.has_context());
    }

    #[test]
    fn test_context_aggregator_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let aggregator = Arc::new(ContextAggregator::new());
        let handles: Vec<_> = (0..10).map(|i| {
            let agg = Arc::clone(&aggregator);
            thread::spawn(move || {
                let target = AnalysisTarget { /* ... */ };
                agg.analyze(&target) // Concurrent access
            })
        }).collect();

        for handle in handles {
            handle.join().unwrap();
        }
        // No panics = success
    }
}
```

### Integration Tests

**End-to-End Context Flow**:
```rust
#[test]
fn test_parallel_analysis_with_context() {
    let output = Command::new("target/debug/debtmap")
        .args(&["analyze", "src/risk", "--context", "--top", "3"])
        .output()
        .expect("Failed to run debtmap");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify contextual risk appears in output
    assert!(stdout.contains("CONTEXT RISK:"));
    assert!(stdout.contains("git_history"));
}
```

### Performance Tests

**Parallel Analysis Benchmark**:
```rust
#[bench]
fn bench_parallel_analysis_with_context(b: &mut Bencher) {
    b.iter(|| {
        // Run parallel analysis on test codebase
        unified_analysis::perform_unified_analysis_with_options(
            UnifiedAnalysisOptions {
                enable_context: true,
                // ... other options
            }
        )
    });
}
```

**Expected**: No regression vs non-context baseline (< 5% overhead).

### User Acceptance

Manual validation:
```bash
# Before fix: No contextual risk in output
$ cargo run -- analyze . --context --top 5 | grep "CONTEXT RISK"
# (empty)

# After fix: Contextual risk data appears
$ cargo run -- analyze . --context --top 5 | grep "CONTEXT RISK"
├─ CONTEXT RISK: base: 45.2, contextual: 67.8 (1.50x multiplier)
   └─ git_history +15.3 impact (changes/mo: 12.4, bug density: 8.2%, age: 234d, authors: 5)
```

## Documentation Requirements

### Code Documentation

1. **Update ContextAggregator docs** (`src/risk/context/mod.rs`):
   ```rust
   /// Thread-safe aggregator for context providers.
   ///
   /// Uses lock-free DashMap for caching to enable safe concurrent access
   /// from parallel analysis workers. The aggregator itself is wrapped in
   /// Arc for cheap cloning across threads.
   ///
   /// # Thread Safety
   ///
   /// Safe to share across threads via Arc. The internal cache uses DashMap
   /// for lock-free concurrent access, avoiding contention in hot paths.
   ```

2. **Update RiskAnalyzer::clone() docs** (`src/risk/mod.rs`):
   ```rust
   /// Clone the risk analyzer, preserving context aggregator.
   ///
   /// The context aggregator is wrapped in Arc, so cloning is cheap (just
   /// an atomic reference count increment) and preserves the shared cache.
   ```

### User Documentation

Update README.md section on `--context` flag to note parallel execution support.

### Architecture Updates

Add to ARCHITECTURE.md:
- Section on lock-free context sharing via DashMap
- Diagram showing Arc-shared aggregator in parallel execution
- Performance characteristics of concurrent cache access

## Implementation Notes

### Stillwater Philosophy Alignment

This fix exemplifies Stillwater principles:

1. **Pure Core, Imperative Shell**:
   - ✓ Context providers remain pure (`&self`)
   - ✓ Cache is isolated imperative shell
   - ✓ No mixing of concerns

2. **Interior Mutability**:
   - ✓ DashMap provides safe interior mutability
   - ✓ Preserves immutable interface (`&self`)
   - ✓ Thread-safe without explicit locks

3. **Pragmatism Over Purity**:
   - ✓ Uses Rust idioms (Arc, DashMap)
   - ✓ Leverages existing dependencies
   - ✓ No heavyweight abstractions

### Gotchas

1. **Provider Cloning**: Don't clone providers in `ContextAggregator::clone()` (they're heavy). Only share the cache via Arc.

2. **Cache Invalidation**: DashMap entries persist for lifetime of aggregator. This is acceptable since:
   - Analysis is single-shot (not long-running server)
   - Cache memory bounded by unique (file, function) pairs
   - Improves performance (fewer git operations)

3. **Arc Cycles**: Be careful not to create reference cycles. Current design is acyclic (analyzer → aggregator).

### Alternative Approaches Considered

| Approach | Pros | Cons | Decision |
|----------|------|------|----------|
| Mutex<HashMap> | Simple | Lock contention | ❌ Rejected (performance) |
| RwLock<HashMap> | Read-biased | Writer starvation | ❌ Rejected (complexity) |
| DashMap | Lock-free reads | Slightly more memory | ✅ **Selected** |
| Recreate aggregator | No sharing | Expensive (git ops) | ❌ Rejected (performance) |

## Migration and Compatibility

### Breaking Changes

**None**. This is an internal fix. Public APIs unchanged:
- `RiskAnalyzer` - Same construction and usage
- `ContextAggregator` - Same construction (signature change internal only)
- CLI flags - Same behavior

### Migration Steps

1. Update `ContextAggregator` to use DashMap
2. Update `RiskAnalyzer` to wrap aggregator in Arc
3. Remove debug logging from `construction.rs` and `mod.rs`
4. Run test suite
5. Manual validation with `--context` flag
6. Commit fix

### Rollback Plan

If issues discovered:
1. Revert commit
2. Add `--no-parallel` flag as workaround (fall back to sequential)
3. Debug and fix properly

## Success Metrics

- ✅ Zero test failures
- ✅ Contextual risk data appears in CLI output with `--context`
- ✅ No performance regression (< 5% overhead)
- ✅ No increase in memory usage (Arc sharing)
- ✅ All context providers (git_history, critical_path, dependency) working
- ✅ Cache hit rate > 60% on typical codebases (measured via debug mode)

## Timeline

- **Phase 1** (1-2 hours): DashMap cache refactor + Arc wrapping
- **Phase 2** (30 min): Remove debug logging, clean up
- **Phase 3** (1 hour): Testing and validation
- **Total**: ~3 hours

## Related Issues

- Spec 202: Contextual Risk in Priority Scoring (prerequisite)
- Issue: Parallel execution silently drops context (this spec fixes it)
