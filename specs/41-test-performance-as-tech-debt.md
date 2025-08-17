# Spec 41: Test Performance as Tech Debt

## Finding
The performance detector correctly identifies blocking I/O in test loops. These are NOT false positives - they represent real tech debt that impacts test suite performance.

## Current Issues in tests/core_cache_tests.rs

### Line 161, 196, 310: Sequential File I/O in Loops
```rust
for i in 0..3 {
    let test_file = temp_dir.path().join(format!("test{i}.rs"));
    std::fs::write(&test_file, format!("fn test{i}() {{}}")).unwrap();  // BLOCKING I/O
    // ... process file
}
```

**Problem**: Files are created sequentially, blocking on each write operation.

**Impact**: 
- Each `fs::write` blocks for disk I/O
- With N files, total time = N × write_time
- Across entire test suite, this compounds

## Recommended Solutions

### Solution 1: Batch File Creation (Simple)
```rust
// Pre-create all test data
let test_files: Vec<_> = (0..3)
    .map(|i| (
        temp_dir.path().join(format!("test{i}.rs")),
        format!("fn test{i}() {{}}")
    ))
    .collect();

// Write all files (still sequential but separated from processing)
for (path, content) in &test_files {
    std::fs::write(path, content).unwrap();
}

// Process files
for (path, _) in test_files {
    cache.get_or_compute(&path, || ...).unwrap();
}
```
**Benefit**: Clearer separation of setup and test logic

### Solution 2: Parallel File Creation (Better)
```rust
use rayon::prelude::*;

// Create files in parallel
let test_files: Vec<_> = (0..3)
    .into_par_iter()
    .map(|i| {
        let path = temp_dir.path().join(format!("test{i}.rs"));
        std::fs::write(&path, format!("fn test{i}() {{}}")).unwrap();
        path
    })
    .collect();

// Process files
for path in test_files {
    cache.get_or_compute(&path, || ...).unwrap();
}
```
**Benefit**: Parallel I/O can be 2-3x faster for multiple files

### Solution 3: Test Fixtures Module (Best for large test suites)
```rust
mod fixtures {
    use std::path::Path;
    use tempfile::TempDir;
    
    pub struct TestFiles {
        pub dir: TempDir,
        pub files: Vec<PathBuf>,
    }
    
    impl TestFiles {
        pub fn create_batch(patterns: &[(&str, &str)]) -> Result<Self> {
            let dir = TempDir::new()?;
            
            // Parallel creation
            let files = patterns
                .par_iter()
                .map(|(name, content)| {
                    let path = dir.path().join(name);
                    std::fs::write(&path, content)?;
                    Ok(path)
                })
                .collect::<Result<Vec<_>>>()?;
                
            Ok(Self { dir, files })
        }
    }
}
```

## Decision: Keep Detection, Document as Tech Debt

1. **These are valid performance issues** - test performance impacts developer productivity
2. **The detector is working correctly** - it found legitimate blocking I/O in loops
3. **Solutions exist** but require refactoring test code

## Recommended Action

1. **Keep the performance detection for tests** - it's finding real issues
2. **Document these as tech debt** with lower priority than production code
3. **Consider adding a "test performance" category** to distinguish from production performance
4. **Gradually refactor tests** to use parallel I/O or fixture modules

## Configuration Option

Add configuration to control test performance detection:

```toml
# .debtmap.toml
[performance.tests]
enabled = true  # Detect performance issues in tests
severity_reduction = 1  # Reduce severity by 1 level (Critical -> High, High -> Medium)
```

This allows teams to:
- Keep visibility into test performance debt
- Prioritize it appropriately relative to production code
- Gradually improve test suite performance