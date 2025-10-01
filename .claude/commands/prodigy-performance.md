# Performance Analysis Command

Analyzes code for performance bottlenecks and generates optimization specs.

## Usage

```
/prodigy-performance [focus]
```

Examples:
- `/prodigy-performance` - General performance analysis
- `/prodigy-performance memory` - Focus on memory usage
- `/prodigy-performance startup` - Optimize startup time
- `/prodigy-performance hot-paths` - Focus on frequently executed code

## What This Command Does

1. **Performance Analysis**
   - Identifies computational bottlenecks
   - Detects memory allocation hotspots
   - Finds inefficient algorithms
   - Locates unnecessary work

2. **Generates Optimization Spec**
   - Creates detailed optimization plan
   - Prioritizes by impact
   - Suggests specific improvements
   - Includes benchmarking strategy

3. **Commits Performance Plan**
   - Git commit with optimization spec
   - Ready for implementation
   - Maintains functionality guarantees

## Analysis Areas

- **Algorithm Complexity**: O(n²) → O(n log n) improvements
- **Memory Usage**: Reduce allocations, improve cache locality
- **I/O Operations**: Batch operations, async patterns
- **Concurrency**: Parallelize independent work
- **Caching**: Add strategic caches
- **Data Structures**: Use more efficient structures

## Performance Metrics

- Execution time
- Memory usage
- CPU utilization
- I/O operations
- Cache efficiency
- Startup time

## Output Format

Generates and commits:

```
perf: generate optimization spec for {focus} perf-{timestamp}
```

## Optimization Principles

1. Measure before optimizing
2. Optimize the hot path first
3. Maintain code readability
4. Document performance tradeoffs
5. Include benchmarks