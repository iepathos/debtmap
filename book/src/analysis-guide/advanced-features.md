# Advanced Features

## Advanced Features

### Purity Detection

Debtmap detects pure functions - those without side effects that always return the same output for the same input.

**What makes a function pure:**
- No I/O operations (file, network, database)
- No mutable global state
- No random number generation
- No system calls
- Deterministic output

**Purity detection is optional:**
- Both `is_pure` and `purity_confidence` are `Option` types
- May be `None` for some functions or languages where detection is not available
- Rust has the most comprehensive purity detection support

**Confidence scoring (when available):**
- **0.9-1.0**: Very confident (no side effects detected)
- **0.7-0.8**: Likely pure (minimal suspicious patterns)
- **0.5-0.6**: Uncertain (some suspicious patterns)
- **0.0-0.4**: Likely impure (side effects detected)

**Example:**
```rust
// Pure: confidence = 0.95
fn calculate_total(items: &[Item]) -> f64 {
    items.iter().map(|i| i.price).sum()
}

// Impure: confidence = 0.1 (I/O detected)
fn save_total(items: &[Item]) -> Result<()> {
    let total = items.iter().map(|i| i.price).sum();
    write_to_file(total)  // Side effect!
}
```

**Benefits:**
- Pure functions are easier to test
- Can be safely cached or memoized
- Safe to parallelize
- Easier to reason about

### Data Flow Analysis

Debtmap builds a comprehensive `DataFlowGraph` that extends basic call graph analysis with variable dependencies, data transformations, I/O operations, and purity tracking.

#### Call Graph Foundation

**Upstream callers** - Who calls this function
- Indicates impact radius
- More callers = higher impact if it breaks

**Downstream callees** - What this function calls
- Indicates dependencies
- More callees = more integration testing needed

**Example:**
```json
{
  "name": "process_payment",
  "upstream_callers": [
    "handle_checkout",
    "process_subscription",
    "handle_refund"
  ],
  "downstream_callees": [
    "validate_payment_method",
    "calculate_fees",
    "record_transaction",
    "send_receipt"
  ]
}
```

#### Variable Dependency Tracking

`DataFlowGraph` tracks which variables each function depends on:

```rust
pub struct DataFlowGraph {
    // Maps function_id -> set of variable names used
    variable_dependencies: HashMap<String, HashSet<String>>,
    // ...
}
```

**What it tracks:**
- Local variables accessed in function body
- Function parameters
- Captured variables (closures)
- Mutable vs immutable references

**Benefits:**
- Identify functions coupled through shared state
- Detect potential side effect chains
- Guide refactoring to reduce coupling

**Example output:**
```json
{
  "function": "calculate_total",
  "variable_dependencies": ["items", "tax_rate", "discount", "total"],
  "parameter_count": 3,
  "local_var_count": 1
}
```

#### Data Transformation Patterns

`DataFlowGraph` identifies common functional programming patterns:

```rust
pub enum TransformationType {
    Map,        // Transform each element
    Filter,     // Select subset of elements
    Reduce,     // Aggregate to single value
    FlatMap,    // Transform and flatten
    Unknown,    // Other transformations
}
```

**Pattern detection:**
- Recognizes iterator chains (`.map()`, `.filter()`, `.fold()`)
- Identifies functional vs imperative data flow
- Tracks input/output variable relationships

**Example:**
```rust
// Detected as: Filter → Map → Reduce pattern
fn total_active_users(users: &[User]) -> f64 {
    users.iter()
        .filter(|u| u.active)      // Filter transformation
        .map(|u| u.balance)        // Map transformation
        .sum()                      // Reduce transformation
}
```

**Transformation metadata:**
```json
{
  "function": "total_active_users",
  "input_vars": ["users"],
  "output_vars": ["sum_result"],
  "transformation_type": "Reduce",
  "is_functional_style": true,
  "pipeline_length": 3
}
```

#### I/O Operation Detection

Tracks functions performing I/O operations for purity and performance analysis:

**I/O categories tracked:**
- **File I/O**: `std::fs`, `File::open`, `read_to_string`
- **Network I/O**: HTTP requests, socket operations
- **Database I/O**: SQL queries, ORM operations
- **System calls**: Process spawning, environment access
- **Blocking operations**: `thread::sleep`, synchronous I/O in async

**Example detection:**
```rust
// Detected I/O operations: FileRead, FileWrite
fn save_config(config: &Config, path: &Path) -> Result<()> {
    let json = serde_json::to_string(config)?;  // No I/O
    std::fs::write(path, json)?;                 // FileWrite detected
    Ok(())
}
```

**I/O metadata:**
```json
{
  "function": "save_config",
  "io_operations": ["FileWrite"],
  "is_blocking": true,
  "affects_purity": true,
  "async_safe": false
}
```

#### Purity Analysis Integration

`DataFlowGraph` integrates with purity detection to provide comprehensive side effect analysis:

**Side effect tracking:**
- I/O operations (file, network, console)
- Global state mutations
- Random number generation
- System time access
- Non-deterministic behavior

**Purity confidence factors:**
- **1.0**: Pure mathematical function, no side effects
- **0.8**: Pure with deterministic data transformations
- **0.5**: Mixed - some suspicious patterns
- **0.2**: Likely impure - I/O detected
- **0.0**: Definitely impure - multiple side effects

**Example analysis:**
```json
{
  "function": "calculate_discount",
  "is_pure": true,
  "purity_confidence": 0.95,
  "side_effects": [],
  "deterministic": true,
  "safe_to_parallelize": true,
  "safe_to_cache": true
}
```

#### Modification Impact Analysis

`DataFlowGraph` calculates the impact of modifying a function:

```rust
pub struct ModificationImpact {
    pub function_name: String,
    pub affected_functions: Vec<String>,  // Upstream callers
    pub dependency_count: usize,          // Downstream callees
    pub has_side_effects: bool,
    pub risk_level: RiskLevel,
}
```

**Risk level calculation:**
- **Critical**: Many upstream callers + side effects + low test coverage
- **High**: Many callers OR side effects with moderate coverage
- **Medium**: Few callers with side effects OR many callers with good coverage
- **Low**: Few callers, no side effects, or well-tested

**Example impact analysis:**
```json
{
  "function": "validate_payment_method",
  "modification_impact": {
    "affected_functions": [
      "process_payment",
      "refund_payment",
      "update_payment_method",
      "validate_subscription"
    ],
    "affected_count": 4,
    "dependency_count": 8,
    "has_side_effects": true,
    "io_operations": ["DatabaseRead", "NetworkCall"],
    "risk_level": "High",
    "recommendation": "Comprehensive testing required - 4 functions depend on this, performs I/O"
  }
}
```

**Using modification impact:**
```bash
# Analyze impact before refactoring
debtmap analyze . --format json | jq '.functions[] | select(.name == "validate_payment_method") | .modification_impact'
```

**Impact analysis uses:**
- **Refactoring planning**: Understand blast radius before changes
- **Test prioritization**: Focus tests on high-impact functions
- **Code review**: Flag high-risk changes for extra scrutiny
- **Dependency management**: Identify tightly coupled components

#### DataFlowGraph Methods

Key methods for data flow analysis:

```rust
// Add function with its dependencies
pub fn add_function(&mut self, function_id: String, callees: Vec<String>)

// Track variable dependencies
pub fn add_variable_dependency(&mut self, function_id: String, var_name: String)

// Record I/O operations
pub fn add_io_operation(&mut self, function_id: String, io_type: IoType)

// Calculate modification impact
pub fn calculate_modification_impact(&self, function_id: &str) -> ModificationImpact

// Get all functions affected by a change
pub fn get_affected_functions(&self, function_id: &str) -> Vec<String>

// Find functions with side effects
pub fn find_functions_with_side_effects(&self) -> Vec<String>
```

**Integration in analysis pipeline:**
1. Parser builds initial call graph
2. DataFlowGraph extends with variable/I/O tracking
3. Purity analyzer adds side effect information
4. Modification impact calculated for each function
5. Results used in prioritization and risk scoring

**Connection to Unified Scoring:**

The dependency analysis from DataFlowGraph directly feeds into the **unified scoring system's dependency factor** (20% weight):

- **Dependency Factor Calculation**: Functions with high upstream caller count or on critical paths from entry points receive higher dependency scores (8-10)
- **Isolated Utilities**: Functions with few or no callers score lower (1-3) on dependency factor
- **Impact Prioritization**: This helps prioritize functions where bugs have wider impact across the codebase
- **Modification Risk**: The modification impact analysis uses dependency data to calculate blast radius when changes are made

**Example:**
```
Function: validate_payment_method
  Upstream callers: 4 (high impact)
  → Dependency Factor: 8.0

Function: format_currency_string
  Upstream callers: 0 (utility)
  → Dependency Factor: 1.5

Both have same complexity, but validate_payment_method gets higher unified score
due to its critical role in the call graph.
```

This integration ensures that the unified scoring system considers not just internal function complexity and test coverage, but also the function's importance in the broader codebase architecture.

### Entropy-Based Complexity

Advanced pattern detection to reduce false positives.

**Token Classification:**
```rust
enum TokenType {
    Variable,     // Weight: 1.0
    Method,       // Weight: 1.5 (more important)
    Literal,      // Weight: 0.5 (less important)
    Keyword,      // Weight: 0.8
    Operator,     // Weight: 0.6
}
```

**Shannon Entropy Calculation:**
```
H(X) = -Σ p(x) × log₂(p(x))
```
where p(x) is the probability of each token type.

**Dampening Decision:**
```rust
if entropy_score.token_entropy < 0.4
   && entropy_score.pattern_repetition > 0.6
   && entropy_score.branch_similarity > 0.7
{
    // Apply dampening
    effective_complexity = base_complexity × (1 - dampening_factor);
}
```

**Output explanation:**
```
Function: validate_input
  Cyclomatic: 15 → Effective: 5
  Reasoning:
    - High pattern repetition detected (85%)
    - Low token entropy indicates simple patterns (0.32)
    - Similar branch structures found (92% similarity)
    - Complexity reduced by 67% due to pattern-based code
```

### Entropy Analysis Caching

`EntropyAnalyzer` includes an LRU-style cache for performance optimization when analyzing large codebases or performing repeated analysis.

#### Cache Structure

```rust
struct CacheEntry {
    score: EntropyScore,
    timestamp: Instant,
    hit_count: usize,
}
```

**Cache configuration:**
- **Default size**: 1000 entries
- **Eviction policy**: LRU (Least Recently Used)
- **Memory per entry**: ~128 bytes
- **Total memory overhead**: ~128 KB for default size

#### Cache Statistics

The analyzer tracks cache performance:

```rust
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
    pub evictions: usize,
    pub hit_rate: f64,
    pub memory_bytes: usize,
}
```

**Example stats output:**
```json
{
  "entropy_cache_stats": {
    "hits": 3427,
    "misses": 1573,
    "evictions": 573,
    "hit_rate": 0.685,
    "memory_bytes": 128000
  }
}
```

**Hit rate interpretation:**
- **> 0.7**: Excellent - many repeated analyses, cache is effective
- **0.4-0.7**: Good - moderate reuse, typical for incremental analysis
- **< 0.4**: Low - mostly unique functions, cache less helpful

#### Performance Benefits

**Typical performance gains:**
- **Cold analysis**: 100ms baseline (no cache benefit)
- **Incremental analysis**: 30-40ms (~60-70% faster) for unchanged functions
- **Re-analysis**: 15-20ms (~80-85% faster) for recently analyzed functions

**Best for:**
- **Watch mode**: Analyzing on file save (repeated analysis of same files)
- **CI/CD**: Comparing feature branch to main (overlap in functions)
- **Large codebases**: Many similar functions benefit from pattern caching

**Memory estimation:**
```
Total cache memory = entry_count × 128 bytes

Examples:
- 1,000 entries: ~128 KB (default)
- 5,000 entries: ~640 KB (large projects)
- 10,000 entries: ~1.25 MB (very large)
```

#### Cache Management

**Automatic eviction:**
- When cache reaches size limit, oldest entries evicted
- Hit count influences retention (frequently accessed stay longer)
- Timestamp used for LRU ordering

**Cache invalidation:**
- Function source changes invalidate entry
- Cache cleared between major analysis runs
- No manual invalidation needed

**Configuration (if exposed in future):**
```toml
[entropy.cache]
enabled = true
size = 1000           # Number of entries
ttl_seconds = 3600    # Optional: expire after 1 hour
```

### Context-Aware Analysis

Debtmap adjusts analysis based on code context:

**Pattern Recognition:**
- Validation patterns (repetitive checks)
- Dispatcher patterns (routing logic)
- Builder patterns (fluent APIs)
- Configuration parsers (key-value processing)

**Adjustment Strategies:**
- Reduce false positives for recognized patterns
- Apply appropriate thresholds by pattern type
- Consider pattern confidence in scoring

**Example:**
```rust
// Recognized as "validation_pattern"
// Complexity dampening applied
fn validate_user_input(input: &UserInput) -> Result<()> {
    if input.name.is_empty() { return Err(Error::EmptyName); }
    if input.email.is_empty() { return Err(Error::EmptyEmail); }
    if input.age < 13 { return Err(Error::TooYoung); }
    // ... more similar validations
    Ok(())
}
```

### Coverage Integration

Debtmap parses LCOV coverage data for risk analysis:

**LCOV Support:**
- Standard format from most coverage tools
- Line-level coverage tracking
- Function-level aggregation

**Coverage Index:**
- O(1) exact name lookups (~0.5μs)
- O(log n) line-based fallback (~5-8μs)
- ~200 bytes per function
- Thread-safe (`Arc<CoverageIndex>`)

#### Performance Characteristics

**Index Build Performance:**
- Index construction: O(n), approximately 20-30ms for 5,000 functions
- Memory usage: ~200 bytes per record (~2MB for 5,000 functions)
- Scales linearly with function count

**Lookup Performance:**
- Exact match (function name): O(1) average, ~0.5μs per lookup
- Line-based fallback: O(log n), ~5-8μs per lookup
- Cache-friendly data structure for hot paths

**Analysis Overhead:**
- Coverage integration overhead: ~2.5x baseline analysis time
- Target overhead: ≤3x (maintained through optimizations)
- Example timing: 53ms baseline → 130ms with coverage (2.45x overhead)
- Overhead includes index build + lookups + coverage propagation

**When to use coverage integration:**
- **Skip coverage** (faster iteration): For rapid development iteration or quick local checks, omit `--lcov` to get baseline results 2.5x faster
- **Include coverage** (comprehensive analysis): Use coverage integration for final validation, sprint planning, and CI/CD gates where comprehensive risk analysis is needed

**Thread Safety:**
- Coverage index wrapped in `Arc&lt;CoverageIndex&gt;` for lock-free parallel access
- Multiple analyzer threads can query coverage simultaneously
- No contention on reads, suitable for parallel analysis pipelines

**Memory Footprint:**
```
Total memory = (function_count × 200 bytes) + index overhead

Examples:
- 1,000 functions: ~200 KB
- 5,000 functions: ~2 MB
- 10,000 functions: ~4 MB
```

**Scalability:**
- Tested with codebases up to 10,000 functions
- Performance remains predictable and acceptable
- Memory usage stays bounded and reasonable

**Generating coverage:**
```bash
# Rust (using cargo-tarpaulin)
cargo tarpaulin --out lcov --output-dir target/coverage

# Or using cargo-llvm-cov
cargo llvm-cov --lcov --output-path target/coverage/lcov.info
```

**Using with Debtmap:**
```bash
debtmap analyze . --lcov target/coverage/lcov.info
```

**Coverage dampening:**
When coverage data is provided, debt scores are dampened for well-tested code:
```
final_score = base_score × (1 - coverage_percentage)
```

This ensures well-tested complex code gets lower priority than untested simple code.

