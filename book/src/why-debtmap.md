# Why Debtmap?

Technical debt analysis tools are everywhere. So why another one? Debtmap takes a fundamentally different approach to code quality analysis—one that reduces false positives and gives you actionable insights instead of just flagging "complex" code.

## The Problem with Traditional Static Analysis

Most static analysis tools flag code as "complex" based purely on metrics like cyclomatic complexity or lines of code. The problem? Not all complexity is equal.

Consider this common pattern:

```rust
fn validate_config(config: &Config) -> Result<()> {
    if config.output_dir.is_none() {
        return Err(anyhow!("output_dir required"))
    }
    if config.max_workers.is_none() {
        return Err(anyhow!("max_workers required"))
    }
    if config.timeout_secs.is_none() {
        return Err(anyhow!("timeout_secs required"))
    }
    if config.log_level.is_none() {
        return Err(anyhow!("log_level required"))
    }
    if config.cache_dir.is_none() {
        return Err(anyhow!("cache_dir required"))
    }
    // ... 15 more similar checks
    Ok(())
}
```

**Traditional tools say:** "Cyclomatic complexity: 20 - CRITICAL! Refactor immediately!"

**Reality:** This is a simple validation function with a repetitive pattern. Yes, it has 20 branches, but they're all identical in structure. An experienced developer can read and understand this in seconds.

Now compare with this function:

```rust
fn reconcile_state(current: &State, desired: &State) -> Vec<Action> {
    let mut actions = vec![];

    match (current.mode, desired.mode) {
        (Mode::Active, Mode::Standby) => {
            if current.has_active_connections() {
                actions.push(Action::DrainConnections);
                actions.push(Action::WaitForDrain);
            }
            actions.push(Action::TransitionToStandby);
        }
        (Mode::Standby, Mode::Active) => {
            if desired.requires_warmup() {
                actions.push(Action::Warmup);
            }
            actions.push(Action::TransitionToActive);
        }
        (Mode::Active, Mode::Maintenance) => {
            // Complex state transitions based on multiple conditions
            if current.has_pending_operations() {
                if desired.force_maintenance {
                    actions.push(Action::AbortPending);
                } else {
                    actions.push(Action::FinishPending);
                }
            }
            actions.push(Action::TransitionToMaintenance);
        }
        // ... more complex state transitions
        _ => {}
    }

    actions
}
```

**Traditional tools say:** "Cyclomatic complexity: 8 - moderate"

**Reality:** This function involves complex state machine logic with conditional transitions, side effects, and non-obvious control flow. It's genuinely complex and error-prone.

**The key insight:** Traditional metrics treat both functions equally, but they're fundamentally different in terms of cognitive load and risk.

## Debtmap's Unique Approach

### 1. Entropy-Based Complexity Analysis

Debtmap uses information theory to distinguish between genuinely complex code and repetitive pattern-based code.

**How it works:**
- Calculate the **variety** of code patterns in a function
- High variety (many different patterns) = high entropy = genuinely complex
- Low variety (repetitive patterns) = low entropy = simple despite high branch count

**Applied to our examples:**

```
validate_config():
- Cyclomatic complexity: 20
- Pattern entropy: 0.3 (low - all branches identical)
- Entropy-adjusted complexity: 5
- Assessment: Low risk despite high branch count

reconcile_state():
- Cyclomatic complexity: 8
- Pattern entropy: 0.85 (high - diverse conditional logic)
- Entropy-adjusted complexity: 9
- Assessment: High risk - genuinely complex logic
```

This approach **significantly reduces false positives** compared to traditional cyclomatic complexity metrics by recognizing that repetitive patterns are easier to understand than diverse, complex logic.

### 2. Coverage-Risk Correlation

Debtmap is the only Rust analysis tool that natively combines code complexity with test coverage to compute risk scores.

**Why this matters:**
- Complex code with good tests = managed risk
- Simple code without tests = unmanaged risk (but low priority)
- Complex code without tests = CRITICAL gap

**Example:**

```rust
// Function A: Complex but well-tested
fn parse_query(sql: &str) -> Result<Query> {
    // Complexity: 15, Coverage: 95%
    // Risk Score: 3.2 (moderate - complexity managed by tests)
}

// Function B: Moderate complexity, no tests
fn apply_migrations(db: &mut Database) -> Result<()> {
    // Complexity: 8, Coverage: 0%
    // Risk Score: 8.9 (critical - untested with moderate complexity)
}
```

Debtmap integrates with LCOV coverage data to automatically prioritize Function B over Function A, even though A is more complex. This is because the risk is about **untested complexity**, not just complexity alone.

**What makes this unique:**

Debtmap is the only Rust-focused tool that natively combines complexity analysis with LCOV coverage data to compute risk scores. While other tools support coverage reporting, they don't correlate it with complexity metrics to prioritize technical debt and testing efforts.

### 3. Actionable Recommendations

Most tools tell you **what** is wrong. Debtmap tells you **what to do about it** and **what impact it will have**.

**Compare:**

**SonarQube:**
```
Function 'process_request' has complexity 15 (threshold: 10)
Severity: Major
```

**Debtmap:**
```
#1 SCORE: 8.9 [CRITICAL]
├─ TEST GAP: ./src/handlers.rs:127 process_request()
├─ ACTION: Add 8 unit tests for full coverage
├─ IMPACT: -5.2 risk reduction
├─ WHY: Complex logic (cyclo=15) with 0% coverage
└─ SUGGEST: Extract validation to separate functions, test each independently
```

Debtmap tells you:
- **Specific location** (file:line)
- **Quantified gap** (8 missing tests)
- **Expected impact** (-5.2 risk reduction)
- **Rationale** (complexity + no coverage)
- **Refactoring suggestions** (extract functions)

### 4. Context-Aware Analysis

Debtmap understands that not all code needs the same level of scrutiny.

**Entry Points:** Main functions, CLI handlers, and framework integration points are typically tested via integration tests, not unit tests. Debtmap's analysis accounts for this:

```rust
// Entry point - flagged as low priority for unit test coverage
fn main() {
    // Debtmap: "Integration test coverage expected - low priority"
}

// Core business logic - flagged as high priority
fn calculate_risk_score(metrics: &Metrics) -> f64 {
    // Debtmap: "High complexity + low coverage = CRITICAL"
}
```

**Call Graph Analysis:** Debtmap traces function dependencies to prioritize functions called by many untested paths:

```
parse_input() [untested]
  ├─ called by: main() [integration tested]
  └─ called by: process_batch() [untested]

Priority: HIGH (called from untested code path)
```

### 5. Performance

Debtmap is written in Rust and uses parallel processing for analysis. Being a native Rust binary with no JVM overhead, it's designed for fast local development workflow integration.

**Typical analysis time:**
- Small project (~10k LOC): 1-2 seconds
- Medium project (~50k LOC): 5-8 seconds
- Large project (~200k LOC): 20-30 seconds

This speed means you can run debtmap in your local development workflow without breaking flow, not just in CI.

## What Problem Does Debtmap Solve?

Debtmap addresses a gap that existing tools don't fill: **quantified technical debt prioritization with actionable refactoring guidance**.

### The Gap in Existing Tools

| Tool Type | What It Does | What It Doesn't Do |
|-----------|--------------|-------------------|
| **Linters** (clippy, ESLint) | Find code style issues and common mistakes | Don't quantify risk or prioritize by impact |
| **Complexity Analyzers** (SonarQube, CodeClimate) | Flag complex code | Don't correlate with test coverage or provide refactoring impact estimates |
| **Coverage Tools** (tarpaulin, codecov) | Show what code is tested | Don't identify which untested code is most risky |

**Note:** Debtmap is not a security scanner. Use tools like `cargo-audit` and `cargo-geiger` for security vulnerability detection. Debtmap focuses on technical debt prioritization, though complex untested code can sometimes harbor security issues.

**What Debtmap uniquely provides:**

1. **Quantified Debt Scoring** - Not just "this is complex," but "this scores 8.9/10 on risk"
2. **Coverage-Risk Correlation** - Identifies untested complex code, not just complex code
3. **Impact Quantification** - "Adding 6 tests will reduce risk by 3.7 points"
4. **Actionable Recommendations** - Specific refactoring suggestions with effort estimates
5. **Dependency-Aware Prioritization** - Prioritizes code that impacts many other functions

### Debtmap vs Traditional Tools

**SonarQube / CodeClimate:**
- **They say:** "Function has complexity 15 (threshold exceeded)"
- **Debtmap says:** "Add 8 tests (-5.2 risk). Extract validation logic to reduce complexity by 60%"

**Coverage Tools (tarpaulin, codecov):**
- **They say:** "67% line coverage, 54% branch coverage"
- **Debtmap says:** "3 critical gaps: untested complex functions that are called from 12+ code paths"

**Linters (clippy):**
- **They say:** "Consider using Iterator::any() instead of a for loop"
- **Debtmap says:** "This function has high cognitive complexity (12) and is called by 8 untested modules - prioritize adding tests before refactoring"

### When to Use Debtmap

**Use Debtmap when you need to:**
- Decide which technical debt to tackle first (limited time/resources)
- Identify critical testing gaps (high-complexity, zero-coverage code)
- Quantify the impact of refactoring efforts
- Reduce false positives from repetitive validation code
- Prioritize refactoring based on risk, not just complexity
- Get specific, actionable recommendations with effort estimates

**Use other tools for different needs:**
- **clippy** - Catch Rust idiom violations and common mistakes
- **tarpaulin** - Generate LCOV coverage data (Debtmap analyzes it)
- **SonarQube** - Multi-language analysis with centralized dashboards

**Security is a separate concern:**
- **cargo-audit** - Find known vulnerabilities in dependencies
- **cargo-geiger** - Detect unsafe code usage
- Debtmap doesn't scan for security issues, though complex code may harbor security risks

### Recommended Workflow

Debtmap works **alongside** existing tools, not instead of them:

```bash
# 1. Local development loop (before commit)
cargo fmt                    # Format code
cargo clippy                 # Check idioms and common issues
cargo test                   # Run tests
debtmap analyze .            # Identify new technical debt

# 2. CI/CD pipeline (PR validation)
cargo test --all-features    # Full test suite
cargo clippy -- -D warnings  # Fail on warnings
debtmap validate .           # Enforce debt thresholds

# 3. Weekly planning (prioritize work)
cargo tarpaulin --out lcov   # Generate coverage
debtmap analyze . --lcov lcov.info --top 20
# Review top 20 debt items, plan sprint work

# 4. Monthly review (track trends)
debtmap analyze . --format json --output debt-$(date +%Y%m).json
debtmap compare --before debt-202410.json --after debt-202411.json
```

### The Bottom Line

**Debtmap isn't a replacement for linters or coverage tools.** It solves a different problem: turning raw complexity and coverage data into **prioritized, actionable technical debt recommendations**.

If you're asking "Where should I focus my refactoring efforts?" or "Which code needs tests most urgently?", that's what Debtmap is built for.

## Key Differentiators

1. **Entropy analysis** - Reduces false positives from repetitive code
2. **Native coverage integration** - Built-in LCOV support for risk scoring
3. **Actionable recommendations** - Specific steps with quantified impact
4. **Context-aware** - Understands entry points, call graphs, and testing patterns
5. **Fast** - Rust performance for local development workflow
6. **Tiered prioritization** - Critical/High/Moderate/Low classification with clear rationale

## Next Steps

Ready to try it? Head to [Getting Started](getting-started.md) to install debtmap and run your first analysis.

Want to understand how it works under the hood? See [Architecture](architecture.md) for the analysis pipeline.

Have questions? Check the [FAQ](faq.md) for common questions and answers.
