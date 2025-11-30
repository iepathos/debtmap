# Debtmap Evaluation Against Stillwater Philosophy

**Date:** 2025-11-30
**Evaluator:** Claude Code
**Philosophy Reference:** `/Users/glen/memento-mori/stillwater/PHILOSOPHY.md`

---

## Executive Summary

Debtmap demonstrates **strong adherence** to functional programming principles described in the Stillwater philosophy. The codebase exhibits:

- ✅ **Excellent I/O separation** with clear trait boundaries
- ✅ **Strong functional core** with pure computation logic
- ✅ **Effect system integration** using Stillwater's Effect types
- ⚠️ **Some functions exceed complexity guidelines** (>20 lines)
- ⚠️ **Several large files** that could be split into smaller modules
- ⚠️ **Mixed I/O/logic** in a few utility functions

**Overall Grade:** B+ (Strong functional design with room for refinement)

---

## 1. Pure Core, Imperative Shell

### Stillwater Principle
> "Like a still pond with water flowing through it, your application should have pure business logic that doesn't change and effects that move data in and out."

### Debtmap Adherence: ✅ EXCELLENT

**Evidence:**

#### Pure Core Examples

**`src/complexity/pure.rs:111-150`** - Perfect pure functions:
```rust
pub fn calculate_cyclomatic_pure(file: &File) -> u32 {
    file.items.iter().map(count_item_branches).sum()
}

pub fn count_function_branches(block: &Block) -> u32 {
    1 + block.stmts.iter().map(count_stmt_branches).sum::<u32>()
}
```

**Why this is exemplary:**
- Takes parsed AST as input (I/O already done)
- Returns simple `u32` (no `Result` needed)
- Deterministic: same input → same output
- Zero side effects
- Testable in microseconds

**`src/debt/mod.rs:20-83`** - Pure transformations:
```rust
pub fn categorize_debt(items: &[DebtItem]) -> HashMap<DebtType, Vec<DebtItem>> {
    items.iter().fold(HashMap::new(), |mut acc, item| {
        acc.entry(item.debt_type).or_default().push(item.clone());
        acc
    })
}

pub fn prioritize_debt(items: &[DebtItem]) -> Vec<DebtItem> {
    let mut sorted = items.to_vec();
    sorted.sort_by_key(|item| std::cmp::Reverse(item.priority));
    sorted
}
```

**Strengths:**
- Uses `fold` for aggregation (functional)
- Single responsibility per function
- No I/O mixing
- Clear data transformations

#### Imperative Shell Examples

**`src/io/traits.rs:40-317`** - Clean I/O abstraction:
```rust
pub trait FileSystem: Send + Sync {
    fn read_to_string(&self, path: &Path) -> Result<String, AnalysisError>;
    fn write(&self, path: &Path, content: &str) -> Result<(), AnalysisError>;
    fn exists(&self, path: &Path) -> bool;
}
```

**`src/io/real.rs:40-67`** - Production implementation:
```rust
impl FileSystem for RealFileSystem {
    fn read_to_string(&self, path: &Path) -> Result<String, AnalysisError> {
        std::fs::read_to_string(path)
            .map_err(|e| AnalysisError::Io { path: path.to_owned(), source: e })
    }
}
```

**Strengths:**
- I/O completely isolated behind trait
- Enables dependency injection
- Testable with mock implementations
- Clear separation from business logic

#### Effect System Integration

**`src/effects.rs:1-100+`** - Stillwater Effect usage:
```rust
pub fn asks_config<Env, F, T>(f: F) -> impl Effect<...>
where Env: AnalysisEnv + Clone
{
    ask_env(move |env: &Env| f(env.config()))
}
```

**`src/resources.rs:94-150`** - Bracket pattern:
```rust
pub fn with_lock_file<T, F, Eff>(lock_path: PathBuf, effect_fn: F) -> AnalysisEffect<T>
{
    bracket(
        // Acquire: Create the lock file
        from_fn(move |_env: &RealEnv| { ... }),
        // Use: Run the provided effect
        move |_lock: LockFile| effect_fn(),
        // Release: Remove the lock file (runs even on error)
        |lock: LockFile| { from_fn(move |_env: &RealEnv| { ... }) },
    )
}
```

**Strengths:**
- Guarantees cleanup even on errors
- I/O isolated in acquire/release
- Business logic in effect_fn
- Zero-cost abstraction

### Areas for Improvement: ⚠️

**`src/analysis_utils.rs:14-66`** - Mixed concerns:
```rust
pub fn collect_file_metrics(files: &[PathBuf]) -> Vec<FileMetrics> {
    // I/O: Read environment variable
    let (total_files, files_to_process) = match std::env::var("DEBTMAP_MAX_FILES") { ... }

    // Side effect: Print warning
    if files.len() > max_files {
        eprintln!("[WARN] Processing limited to {} files...", max_files);
    }

    // I/O: Create progress bar
    let progress = ProgressManager::global().map(|pm| { ... });

    // Pure computation: Parallel analysis
    let results: Vec<FileMetrics> = files_to_process.par_iter() ...
}
```

**Recommendation:**
```rust
// Pure: Determine files to process
fn determine_files_to_process(files: &[PathBuf], max: Option<usize>) -> &[PathBuf] {
    match max {
        Some(0) | None => files,
        Some(max_files) => &files[..max_files.min(files.len())],
    }
}

// Pure: Analyze files (no progress tracking)
fn analyze_files_parallel(files: &[PathBuf]) -> Vec<FileMetrics> {
    files.par_iter()
        .filter_map(|path| analyze_single_file(path))
        .collect()
}

// I/O wrapper: Add progress tracking
fn with_progress_tracking<T>(
    total: usize,
    effect: AnalysisEffect<T>
) -> AnalysisEffect<T> {
    // Stillwater effect for progress management
}
```

---

## 2. Fail Fast vs Fail Completely

### Stillwater Principle
> "Validation usually stops at the first error. User submits a form with 5 fields, gets 'email invalid' error, fixes it, submits again, gets 'password too weak', etc. Frustrating!"

### Debtmap Adherence: ⚠️ PARTIAL

**Current Approach:** Debtmap primarily uses **fail-fast** with `Result<T, E>`:

**`src/analyzers/mod.rs:56-66`**
```rust
pub fn analyze_file(content: String, path: PathBuf, analyzer: &dyn Analyzer)
    -> Result<FileMetrics>
{
    analyzer
        .parse(&content, path.clone())?      // Fails fast here
        .map(transform_ast)?                 // Or here
        .map(|ast| analyzer.analyze(&ast))?  // Or here
}
```

**When this is appropriate:**
- ✅ File analysis: later steps depend on parsing success
- ✅ Sequential operations with dependencies
- ✅ Single file processing

**Where Validation would help:**

**`src/core/config.rs`** (hypothetical) - Configuration validation:
```rust
// Current (fail-fast):
fn validate_config(cfg: &Config) -> Result<ValidConfig, ConfigError> {
    validate_max_complexity(cfg.max_complexity)?;  // Stops here
    validate_output_format(cfg.format)?;           // Never reached
    validate_file_patterns(cfg.patterns)?;         // Never reached
    Ok(ValidConfig::from(cfg))
}

// Better (fail-completely):
fn validate_config(cfg: &Config) -> Validation<ValidConfig, Vec<ConfigError>> {
    Validation::all((
        validate_max_complexity(cfg.max_complexity),
        validate_output_format(cfg.format),
        validate_file_patterns(cfg.patterns),
    ))
    .map(|(max_complexity, format, patterns)| {
        ValidConfig { max_complexity, format, patterns }
    })
}
// Returns: Err(vec![ComplexityError, FormatError, PatternError])
```

**Recommendation:**
- ✅ Keep `Result` for sequential file processing
- ➕ Add `Validation` for configuration and multi-field input validation
- ➕ Consider `Validation` for batch file analysis error reporting

---

## 3. Errors Should Tell Stories

### Stillwater Principle
> "Deep call stacks lose context. Use `.context()` to add breadcrumbs."

### Debtmap Adherence: ✅ GOOD

**Evidence:**

**`src/io/real.rs:40-67`** - Contextual errors:
```rust
fn read_to_string(&self, path: &Path) -> Result<String, AnalysisError> {
    std::fs::read_to_string(path)
        .map_err(|e| AnalysisError::Io {
            path: path.to_owned(),  // Context: which file
            source: e                // Original error preserved
        })
}
```

**`src/analyzers/batch.rs:149-200`** - Error context in analysis:
```rust
pub fn analyze_files_effect(files: Vec<PathBuf>) -> AnalysisEffect<Vec<FileMetrics>> {
    traverse(files, |path| {
        read_file_effect(&path)
            .context(format!("Reading file: {}", path.display()))
            .and_then(|content| {
                parse_file_effect(content, path.clone())
                    .context(format!("Parsing file: {}", path.display()))
            })
            .and_then(|ast| {
                analyze_ast_effect(ast)
                    .context("Analyzing AST")
            })
    })
}
```

**Strengths:**
- File paths included in errors
- Operation context added at each layer
- Original errors preserved
- Stack trace equivalent in error messages

### Areas for Improvement: ⚠️

Some error handling could be more contextual:

**`src/utils/analysis_helpers.rs:55-70`**
```rust
pub fn prepare_files_for_duplication_check(files: &[PathBuf])
    -> Vec<(PathBuf, String)>
{
    files.iter()
        .filter_map(|path| match io::read_file(path) {
            Ok(content) => Some((path.clone(), content)),
            Err(e) => {
                log::debug!("Skipping file {} for duplication check: {}",
                    path.display(), e);  // Lost context
                None
            }
        })
        .collect()
}
```

**Better:**
```rust
Err(e) => {
    log::debug!(
        "Skipping file {} for duplication check: {:?}",
        path.display(),
        e.chain()  // Show full error chain
    );
    None
}
```

---

## 4. Composition Over Complexity

### Stillwater Principle
> "Build complex behavior from simple, composable pieces. Each piece does one thing, is easily testable, and has clear types."

### Debtmap Adherence: ✅ EXCELLENT

**Evidence:**

**`src/analysis_utils.rs:68-98`** - Functional pipelines:
```rust
pub fn extract_all_functions(file_metrics: &[FileMetrics]) -> Vec<FunctionMetrics> {
    file_metrics
        .iter()
        .flat_map(|m| &m.complexity.functions)
        .cloned()
        .collect()
}

pub fn extract_file_contexts(file_metrics: &[FileMetrics])
    -> HashMap<PathBuf, FileContext>
{
    file_metrics
        .iter()
        .map(|m| {
            let detector = FileContextDetector::new(m.language);
            let context = detector.detect(&m.path, &m.complexity.functions);
            (m.path.clone(), context)
        })
        .collect()
}
```

**Strengths:**
- Single responsibility per function
- Pure transformations
- Iterator combinators (`flat_map`, `map`, `collect`)
- Composable and reusable

**`src/complexity/mod.rs:72-86`** - Tiny composable functions:
```rust
pub fn combine_complexity(a: u32, b: u32) -> u32 { a + b }
pub fn max_complexity(a: u32, b: u32) -> u32 { a.max(b) }
pub fn average_complexity(values: &[u32]) -> f64 {
    if values.is_empty() { 0.0 }
    else { values.iter().sum::<u32>() as f64 / values.len() as f64 }
}
```

**Strengths:**
- 1-6 lines each
- Single purpose
- Easily tested
- Building blocks for complex analysis

### Areas for Improvement: ⚠️

**Large Files Violate Composition:**

1. **`god_object_detector.rs`** - 4,363 lines
   - Should be split into:
     - `god_object/detector.rs` - Detection logic
     - `god_object/classifier.rs` - Classification rules
     - `god_object/recommender.rs` - Recommendations
     - `god_object/mod.rs` - Public API

2. **`formatter.rs`** - 3,094 lines
   - Should be split into:
     - `format/rules.rs` - Pure formatting rules
     - `format/output.rs` - I/O operations
     - `format/types.rs` - Data types
     - `format/mod.rs` - Composition

**Large Functions:**

**`src/main.rs:564-714`** - `handle_analyze_command` (150+ lines)
```rust
fn handle_analyze_command(command: Commands) -> Result<Result<()>> {
    if let Commands::Analyze {
        // 50+ parameters destructured
    } = command {
        // Environment setup
        // Configuration building
        // Validation
        // Execution
        // Output formatting
        // All in one function!
    }
}
```

**Better decomposition:**
```rust
fn handle_analyze_command(command: Commands) -> Result<Result<()>> {
    let params = extract_analyze_params(command)?;
    let config = build_analyze_config(params)?;
    let env = setup_environment(&config)?;
    let results = run_analysis(&config, &env)?;
    format_and_output(results, &config)
}

// Each helper function: 5-15 lines, single responsibility
```

---

## 5. Types Guide, Don't Restrict

### Stillwater Principle
> "Use types to make wrong code hard to write, but keep them simple. Effect<T, E, Env> tells you what it produces, how it fails, and what it needs."

### Debtmap Adherence: ✅ EXCELLENT

**Evidence:**

**`src/effects.rs`** - Clear type signatures:
```rust
pub type AnalysisEffect<T> = BoxedEffect<T, AnalysisError, RealEnv>;

pub fn asks_config<Env, F, T>(f: F) -> impl Effect<T, E, Env>
where
    Env: AnalysisEnv + Clone,
    F: FnOnce(&Config) -> T
{
    ask_env(move |env: &Env| f(env.config()))
}
```

**Benefits:**
- `AnalysisEffect<T>` tells you: produces `T`, can fail with `AnalysisError`, needs `RealEnv`
- Can't run without environment (compile error)
- Can't forget to handle errors
- Clear what resources are needed

**`src/core/mod.rs:377-390`** - Type-driven parsing:
```rust
impl Language {
    pub fn from_extension(ext: &str) -> Option<Language> {
        match ext {
            "rs" => Some(Language::Rust),
            "py" => Some(Language::Python),
            "js" | "jsx" => Some(Language::JavaScript),
            "ts" | "tsx" => Some(Language::TypeScript),
            _ => None,
        }
    }
}
```

**Benefits:**
- Returns `Option<Language>` (explicit about failure)
- Exhaustive pattern matching (no forgotten extensions)
- Type-safe (can't mix up language IDs)

**`src/resources.rs:94-150`** - Resource type safety:
```rust
pub fn with_lock_file<T, F, Eff>(
    lock_path: PathBuf,
    effect_fn: F
) -> AnalysisEffect<T>
where
    F: FnOnce() -> Eff + Send + 'static,
    Eff: Effect<T, AnalysisError, RealEnv> + Send + 'static,
    T: Send + 'static,
{
    bracket(/* acquire */, effect_fn, /* release */)
}
```

**Benefits:**
- Type system enforces cleanup (can't forget to release)
- Send bounds ensure thread safety
- Effect type ensures proper error handling

### Simple Types, Not Complex:

**Good:** Most types are straightforward:
```rust
pub struct FileMetrics {
    pub path: PathBuf,
    pub language: Language,
    pub complexity: ComplexityMetrics,
}
```

**Avoid:** No heavy type machinery like:
- ❌ Complex GATs (Generic Associated Types)
- ❌ HKT simulation with macro magic
- ❌ Deep trait hierarchies

---

## 6. Pragmatism Over Purity

### Stillwater Principle
> "We're not trying to be Haskell. We're trying to be better Rust."

### Debtmap Adherence: ✅ EXCELLENT

**Evidence:**

**Works with Rust ecosystem:**
```rust
// Uses standard Result with ? operator
pub fn analyze_file(path: &Path) -> Result<FileMetrics, AnalysisError> {
    let content = std::fs::read_to_string(path)?;  // Standard library
    let ast = parse_content(&content)?;            // ? operator works
    Ok(analyze_ast(&ast))
}

// Integrates with rayon for parallelism
pub fn analyze_files(files: &[PathBuf]) -> Vec<FileMetrics> {
    files.par_iter()                               // rayon parallel iterator
        .filter_map(|path| analyze_file(path).ok())
        .collect()
}
```

**Pragmatic choices:**
- ✅ Uses `Result<T, E>` (not custom monad)
- ✅ Uses `rayon` for parallelism (not custom concurrency)
- ✅ Uses `serde` for serialization (not custom derive)
- ✅ Uses `clap` for CLI parsing (not custom parser)
- ✅ Integrates with `tokio` for async (where needed)

**Zero-cost abstractions:**
```rust
// Effect system compiles to zero-cost
pub fn asks_config<Env, F, T>(f: F) -> impl Effect<T, E, Env>
{
    ask_env(move |env: &Env| f(env.config()))
}
// No boxing unless explicitly .boxed()
// No runtime overhead
// Inlines completely
```

**Not fighting the borrow checker:**
```rust
// Good: Works with ownership
pub fn categorize_debt(items: &[DebtItem]) -> HashMap<DebtType, Vec<DebtItem>> {
    items.iter().fold(HashMap::new(), |mut acc, item| {
        acc.entry(item.debt_type).or_default().push(item.clone());
        acc
    })
}

// Not: Fighting with lifetimes
// (No complex lifetime gymnastics in codebase)
```

---

## Architecture Alignment

### Stillwater Mental Model: The Pond

```
  Stream In              Stream Out
     (I/O)                 (I/O)
       ↓                     ↑
    ┌─────────────────────┐
    │                     │
    │   Still  Water     │ ← Pure logic happens here
    │                     │   (calm, predictable)
    │   (Your Business)   │
    │                     │
    └─────────────────────┘
```

### Debtmap Implementation:

```
  Files In                 Reports Out
  (I/O Layer)              (I/O Layer)
       ↓                       ↑
    ┌─────────────────────────────┐
    │  src/io/                    │
    ├─────────────────────────────┤
    │  Still Water Core:          │
    │  - src/complexity/pure.rs   │ ← Pure calculations
    │  - src/debt/mod.rs          │ ← Pure transformations
    │  - src/analyzers/           │ ← AST analysis
    │                             │
    │  Effect Shell:              │
    │  - src/effects.rs           │ ← Effect composition
    │  - src/resources.rs         │ ← Resource management
    └─────────────────────────────┘
```

**Alignment:** ✅ EXCELLENT

The architecture matches the Stillwater mental model:
- **I/O at boundaries:** `src/io/` module for all file operations
- **Pure core:** `complexity/pure.rs`, `debt/mod.rs` have zero I/O
- **Effect shell:** `effects.rs` composes I/O operations
- **Resources managed:** `resources.rs` uses bracket pattern

---

## Summary Scorecard

| Principle | Grade | Notes |
|-----------|-------|-------|
| **Pure Core, Imperative Shell** | A | Excellent separation, effect system integration |
| **Fail Fast vs Fail Completely** | B | Uses Result well, could add Validation for config |
| **Errors Should Tell Stories** | A- | Good context, could improve in utilities |
| **Composition Over Complexity** | B+ | Good pipelines, but some large files/functions |
| **Types Guide, Don't Restrict** | A | Clear types, not overly complex |
| **Pragmatism Over Purity** | A | Works with Rust ecosystem, zero-cost abstractions |

**Overall:** **B+ (Strong Functional Design)**

---

## Specific Recommendations

### High Priority (Do Now)

1. **Split large files into modules:**
   ```bash
   src/god_object_detector.rs (4,363 lines)
   → src/god_object/detector.rs
   → src/god_object/classifier.rs
   → src/god_object/recommender.rs
   → src/god_object/mod.rs
   ```

2. **Refactor `handle_analyze_command` (main.rs:564-714):**
   - Extract: `extract_analyze_params`
   - Extract: `setup_environment`
   - Extract: `run_analysis`
   - Extract: `format_and_output`
   - Each function: 5-20 lines

3. **Separate I/O from logic in `collect_file_metrics`:**
   ```rust
   // Pure
   fn determine_files_to_process(files: &[PathBuf], max: Option<usize>) -> &[PathBuf]

   // Pure
   fn analyze_files_parallel(files: &[PathBuf]) -> Vec<FileMetrics>

   // I/O wrapper
   fn with_progress_tracking<T>(effect: AnalysisEffect<T>) -> AnalysisEffect<T>
   ```

### Medium Priority (Next Sprint)

4. **Add Validation for config:**
   ```rust
   fn validate_config(cfg: &Config)
       -> Validation<ValidConfig, Vec<ConfigError>>
   {
       Validation::all((
           validate_max_complexity(cfg.max_complexity),
           validate_output_format(cfg.format),
           validate_file_patterns(cfg.patterns),
       ))
   }
   ```

5. **Remove/gate debug statements:**
   - Audit 464 print statements
   - Remove from core analysis code
   - Gate behind `#[cfg(debug_assertions)]` or feature flag

6. **Split `formatter.rs` (3,094 lines):**
   ```
   src/format/rules.rs      (pure formatting logic)
   src/format/output.rs     (I/O operations)
   src/format/types.rs      (data structures)
   src/format/mod.rs        (composition)
   ```

### Low Priority (Future)

7. **Extract more pure functions from analyzers:**
   - Look for 30+ line methods
   - Extract calculation logic
   - Keep I/O in thin wrappers

8. **Consider parallel validation:**
   - Use `Validation` for batch error reporting
   - Example: Multiple file validation errors in single pass

9. **Document architectural patterns:**
   - Add ARCHITECTURE.md
   - Document Pure Core / Imperative Shell
   - Link to Stillwater philosophy

---

## Conclusion

Debtmap is a **strong example** of functional programming principles in Rust. The codebase demonstrates:

- ✅ Clear separation of I/O and business logic
- ✅ Extensive use of pure functions
- ✅ Effect system for composable operations
- ✅ Functional pipelines with iterator combinators
- ✅ Immutable data structures
- ✅ Resource safety with bracket pattern

The primary areas for improvement are:
- ⚠️ Reducing function and file size
- ⚠️ Further separating mixed I/O/logic in utilities
- ⚠️ Adding Validation for comprehensive error reporting

With these refinements, debtmap would be an **A-grade exemplar** of the Stillwater philosophy in practice.

---

**Next Steps:**

1. Run `/prodigy-debtmap` to identify highest-priority technical debt
2. Use `refactor-assistant` agent to split large functions
3. Use `file-ops` agent to reorganize large files into modules
4. Re-evaluate after refactoring

---

*"The stillness at the center of your code is where truth lives. Effects are just water flowing around it."*
— Stillwater Philosophy
