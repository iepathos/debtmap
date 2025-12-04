# Architectural Analysis

Debtmap provides comprehensive architectural analysis capabilities based on Robert C. Martin's software engineering principles. These tools help identify structural issues, coupling problems, and architectural anti-patterns in your codebase.

## Overview

Architectural analysis examines module-level relationships and dependencies to identify:

- **Circular Dependencies** - Modules that create dependency cycles
- **Coupling Metrics** - Afferent and efferent coupling measurements
- **Bidirectional Dependencies** - Inappropriate intimacy between modules
- **Stable Dependencies Principle Violations** - Unstable modules being depended upon
- **Zone of Pain** - Rigid, concrete implementations heavily depended upon
- **Zone of Uselessness** - Overly abstract, unstable modules
- **Code Duplication** - Identical or similar code blocks across files

These analyses help you maintain clean architecture and identify refactoring opportunities.

## Circular Dependency Detection

Circular dependencies occur when modules form a dependency cycle (A depends on B, B depends on C, C depends on A). These violations break architectural boundaries and make code harder to understand, test, and maintain.

### How It Works

Debtmap builds a **dependency graph** from module imports and uses **depth-first search (DFS)** with recursion stack tracking to detect cycles:

1. Parse all files to extract import/module dependencies
2. Build a directed graph where nodes are modules and edges are dependencies
3. Run DFS from each unvisited module
4. Track visited nodes and recursion stack
5. When a node is reached that's already in the recursion stack, a cycle is detected

**Implementation:** `src/debt/circular.rs:44-66` (detect_circular_dependencies)

### Example

```rust
// Module A (src/auth.rs)
use crate::user::User;
use crate::session::validate_session;

// Module B (src/user.rs)
use crate::session::Session;

// Module C (src/session.rs)
use crate::auth::authenticate; // Creates cycle: auth → session → auth
```

**Debtmap detects:**
```
Circular dependency detected: auth → session → auth
```

### Refactoring Recommendations

To break circular dependencies:

1. **Extract Interface** - Create a trait that both modules depend on
2. **Dependency Inversion** - Introduce an abstraction layer
3. **Move Shared Code** - Extract common functionality to a new module
4. **Remove Dependency** - Inline or duplicate small amounts of code

## Coupling Metrics

Coupling metrics measure how interconnected modules are. Debtmap calculates two primary metrics:

### Afferent Coupling (Ca)

**Afferent coupling** is the number of modules that depend on this module. High afferent coupling means many modules rely on this code.

```rust
pub struct CouplingMetrics {
    pub module: String,
    pub afferent_coupling: usize, // Number depending on this module
    pub efferent_coupling: usize, // Number this module depends on
    pub instability: f64,         // Calculated from Ca and Ce
    pub abstractness: f64,        // Ratio of abstract types
}
```

**Implementation:** `src/debt/coupling.rs:6-30`

### Efferent Coupling (Ce)

**Efferent coupling** is the number of modules this module depends on. High efferent coupling means this module has many dependencies.

**Note on Abstractness:** The `abstractness` field in `CouplingMetrics` requires advanced type analysis to calculate properly. The current implementation uses a placeholder value (0.0) as full abstractness calculation would need semantic analysis of trait definitions, abstract types, and implementation ratios. This is similar to the cohesion analysis limitation documented below (see "Cohesion Analysis" section).

**Source:** `src/debt/coupling.rs:44`

### Example Coupling Analysis

```
Module: api_handler
  Afferent coupling (Ca): 8  // 8 modules depend on api_handler
  Efferent coupling (Ce): 3  // api_handler depends on 3 modules
  Instability: 0.27          // Relatively stable
```

High afferent or efferent coupling (typically >5) indicates potential maintainability issues.

## Instability Metric

The **instability metric** measures how resistant a module is to change. It's calculated as:

```
I = Ce / (Ca + Ce)
```

**Interpretation:**
- **I = 0.0** - Maximally stable (no dependencies, many dependents)
- **I = 1.0** - Maximally unstable (many dependencies, no dependents)

**Implementation:** `src/debt/coupling.rs:16-24` (calculate_instability)

### Stability Guidelines

- **Stable modules (I < 0.3)** - Hard to change but depended upon; should contain stable abstractions
- **Balanced modules (0.3 ≤ I ≤ 0.7)** - Normal modules with both dependencies and dependents
- **Unstable modules (I > 0.7)** - Change frequently; should have few or no dependents

### Example

```rust
// Stable module (I = 0.1)
// core/types.rs - defines fundamental types, depended on by 20 modules
pub struct User { ... }
pub struct Session { ... }

// Unstable module (I = 0.9)
// handlers/admin_dashboard.rs - depends on 10 modules, no dependents
use crate::auth::*;
use crate::database::*;
use crate::templates::*;
// ... 7 more imports
```

## Stable Dependencies Principle

The **Stable Dependencies Principle (SDP)** states: *Depend in the direction of stability*. Modules should depend on modules that are more stable than themselves.

### SDP Violations

Debtmap flags violations when a module has:
- **Instability > 0.8** (very unstable)
- **Afferent coupling > 2** (multiple modules depend on it)

This means an unstable, frequently changing module is being depended upon by multiple other modules - a recipe for maintenance problems.

**Implementation:** `src/debt/coupling.rs:69-76`

### Example Violation

```
Module 'temp_utils' violates Stable Dependencies Principle
(instability: 0.85, depended on by 5 modules)

Problem: This module changes frequently but is heavily depended upon.
Solution: Extract stable interface or reduce dependencies on this module.
```

### Fixing SDP Violations

1. **Increase stability** - Reduce the module's dependencies
2. **Reduce afferent coupling** - Extract interface, use dependency injection
3. **Split module** - Separate stable and unstable parts

## Bidirectional Dependencies

Bidirectional dependencies (also called **inappropriate intimacy**) occur when two modules depend on each other:

```
Module A depends on Module B
Module B depends on Module A
```

This creates tight coupling and makes both modules harder to change, test, or reuse independently.

**Implementation:** `src/debt/coupling.rs:98-117` (detect_inappropriate_intimacy)

### Example

```rust
// order.rs
use crate::customer::Customer;

pub struct Order {
    customer: Customer,
}

// customer.rs
use crate::order::Order; // Bidirectional dependency!

pub struct Customer {
    orders: Vec<Order>,
}
```

**Debtmap detects:**
```
Inappropriate intimacy detected between 'order' and 'customer'
```

### Refactoring Recommendations

1. **Create Mediator** - Introduce a third module to manage the relationship
2. **Break into Separate Modules** - Split concerns more clearly
3. **Use Events** - Replace direct dependencies with event-driven communication
4. **Dependency Inversion** - Introduce interfaces/traits both depend on

## Zone of Pain Detection

The **zone of pain** contains modules with:
- **Low abstractness (< 0.2)** - Concrete implementations, no abstractions
- **Low instability (< 0.2)** - Stable, hard to change
- **High afferent coupling (> 3)** - Many modules depend on them

These modules are rigid concrete implementations that are heavily used but hard to change - causing pain when modifications are needed.

**Implementation:** `src/debt/coupling.rs:125-138`

### Example

```
Module 'database_client' is in the zone of pain (rigid and hard to change)
  Abstractness: 0.1  (all concrete implementation)
  Instability: 0.15  (very stable, many dependents)
  Afferent coupling: 12 (12 modules depend on it)

Problem: This concrete database client is used everywhere.
Any change to its implementation requires updating many modules.
```

### Refactoring Recommendations

1. **Extract Interfaces** - Create a `DatabaseClient` trait
2. **Introduce Abstractions** - Define abstract operations others depend on
3. **Break into Smaller Modules** - Separate concerns to reduce coupling
4. **Use Dependency Injection** - Pass implementations via interfaces

## Zone of Uselessness Detection

The **zone of uselessness** contains modules with:
- **High abstractness (> 0.8)** - Mostly abstract, few concrete implementations
- **High instability (> 0.8)** - Frequently changing

These modules are overly abstract and unstable, providing little stable value to the system.

**Implementation:** `src/debt/coupling.rs:141-153`

### Example

```
Module 'base_processor' is in the zone of uselessness
(too abstract and unstable)
  Abstractness: 0.9  (mostly traits and interfaces)
  Instability: 0.85  (changes frequently)

Problem: This module defines many abstractions but provides little
concrete value. It changes often, breaking implementations.
```

### Refactoring Recommendations

1. **Add Concrete Implementations** - Make the module useful by implementing functionality
2. **Remove if Unused** - Delete if no real value is provided
3. **Stabilize Interfaces** - Stop changing abstractions frequently
4. **Merge with Implementations** - Combine abstract and concrete code

## Distance from Main Sequence

The **main sequence** represents the ideal balance between abstractness and instability. Modules should lie on the line:

```
A + I = 1
```

Where:
- **A** = Abstractness (ratio of abstract types to total types)
- **I** = Instability (Ce / (Ca + Ce))

**Distance** from the main sequence:

```
D = |A + I - 1|
```

**Implementation:** `src/debt/coupling.rs:119-123`

### Interpretation

- **D ≈ 0.0** - Module is on the main sequence (ideal)
- **D > 0.5** - Module is far from ideal
  - High D with low A and I → Zone of Pain
  - High D with high A and I → Zone of Uselessness

### Visual Representation

```
Abstractness
    1.0 ┤        Zone of Uselessness
        │      ╱
        │    ╱
    0.5 ┤  ╱ Main Sequence
        │╱
        ╱
    0.0 ┤──────────────────────────
        0.0    0.5              1.0
                 Instability

        Zone of Pain
```

## Code Duplication Detection

Debtmap detects code duplication using **hash-based chunk comparison**:

1. **Extract chunks** - Split files into fixed-size chunks (default: 50 lines)
2. **Normalize** - Remove whitespace and comments
3. **Calculate hash** - Compute SHA-256 hash for each normalized chunk
4. **Match duplicates** - Find chunks with identical hashes
5. **Merge adjacent** - Consolidate consecutive duplicate blocks

**Note:** The minimum chunk size is configurable via the `--threshold-duplication` flag or in `.debtmap.toml` (default: 50 lines).

**Implementation:** `src/debt/duplication.rs:6-44` (detect_duplication)

### Algorithm Details

```rust
pub fn detect_duplication(
    files: Vec<(PathBuf, String)>,
    min_lines: usize,           // Default: 50
    _similarity_threshold: f64, // Currently unused (exact matching)
) -> Vec<DuplicationBlock>
```

The algorithm:
1. Extracts overlapping chunks from each file
2. Normalizes by trimming whitespace and removing comments
3. Calculates SHA-256 hash for each normalized chunk
4. Groups chunks by hash
5. Returns groups with 2+ locations (duplicates found)

### Example Output

```
Code duplication detected:
  Hash: a3f2b9c1...
  Lines: 50
  Locations:
    - src/handlers/user.rs:120-169
    - src/handlers/admin.rs:85-134
    - src/handlers/guest.rs:200-249

Recommendation: Extract common validation logic to shared module
```

## Duplication Configuration

Configure duplication detection in `.debtmap.toml`:

```toml
# Minimum lines for duplication detection
threshold_duplication = 50  # Default value

# Smaller values catch more duplications but increase noise
# threshold_duplication = 30  # More sensitive

# Larger values only catch major duplications
# threshold_duplication = 100  # Less noise
```

**Configuration reference:** `src/cli.rs:69` (threshold_duplication flag definition)

**Implementation:** `src/debt/duplication.rs:6-10`

### Current Limitations

- **Exact matching only** - Currently uses hash-based exact matching
- **similarity_threshold parameter** - Defined in function signature but not implemented yet
- **Future enhancement** - Fuzzy matching for near-duplicates using similarity algorithms (e.g., Levenshtein distance, token-based similarity)

The `similarity_threshold` parameter exists for future extensibility but is currently unused. All duplication detection uses exact hash matching. Track progress on fuzzy matching in the project's issue tracker or roadmap.

## Refactoring Recommendations

Debtmap provides specific refactoring recommendations for each architectural issue:

### For Circular Dependencies

1. **Extract Interface** - Create shared abstraction both modules use
2. **Dependency Inversion** - Introduce interfaces to reverse dependency direction
3. **Move Shared Code** - Extract to new module both can depend on
4. **Event-Driven** - Replace direct calls with event publishing/subscribing

### For High Coupling

1. **Facade Pattern** - Provide simplified interface hiding complex dependencies
2. **Reduce Dependencies** - Remove unnecessary imports and calls
3. **Dependency Injection** - Pass dependencies via constructors/parameters
4. **Interface Segregation** - Split large interfaces into focused ones

### For Zone of Pain

1. **Introduce Abstractions** - Extract traits/interfaces for flexibility
2. **Adapter Pattern** - Wrap concrete implementations with adapters
3. **Strategy Pattern** - Make algorithms pluggable via interfaces

### For Zone of Uselessness

1. **Add Concrete Implementations** - Provide useful functionality
2. **Remove Unused Code** - Delete if providing no value
3. **Stabilize Interfaces** - Stop changing abstractions frequently

### For Bidirectional Dependencies

1. **Create Mediator** - Third module manages relationship
2. **Break into Separate Modules** - Clearer separation of concerns
3. **Observer Pattern** - One-way communication via observers

### For Code Duplication

1. **Extract Common Code** - Create shared function/module
2. **Use Inheritance/Composition** - Share via traits or composition
3. **Parameterize Differences** - Extract variable parts as parameters
4. **Template Method** - Define algorithm structure, vary specific steps

## Examples and Use Cases

### Running Architectural Analysis

```bash
# Architectural analysis runs automatically with standard analysis
debtmap analyze .

# Duplication detection with custom chunk size
debtmap analyze . --threshold-duplication 30

# Note: Circular dependencies, coupling metrics, and SDP violations
# are analyzed automatically. There are no separate flags to enable
# or disable specific architectural checks.
```

### Example: Circular Dependency

**Before:**
```
src/auth.rs → src/session.rs → src/user.rs → src/auth.rs

Circular dependency detected: auth → session → user → auth
```

**After refactoring:**
```
src/auth.rs → src/auth_interface.rs ← src/session.rs
                      ↑
              src/user.rs

No circular dependencies found.
```

### Example: Coupling Metrics Table

```
Module Analysis Results:

Module              Ca    Ce    Instability  Issues
-------------------------------------------------
core/types          15     0       0.00      None
api/handlers         2     8       0.80      High Ce
database/client      8     2       0.20      None
utils/temp          5    12       0.71      SDP violation
auth/session        3     3       0.50      None
```

### Example: Zone of Pain

**Module:** `legacy_db_client`

```
Metrics:
  Abstractness: 0.05 (all concrete code)
  Instability: 0.12 (depended on by 25 modules)
  Afferent coupling: 25
  Distance from main sequence: 0.83

Status: Zone of Pain - rigid and hard to change

Refactoring steps:
1. Extract interface DatabaseClient trait
2. Create adapter wrapping legacy implementation
3. Gradually migrate dependents to use trait
4. Introduce alternative implementations
```

## Interpreting Results

### Prioritization

Address architectural issues in this order:

1. **Circular Dependencies** (Highest Priority)
   - Break architectural boundaries
   - Make testing impossible
   - Cause build issues

2. **Bidirectional Dependencies** (High Priority)
   - Create tight coupling
   - Prevent independent testing
   - Block modular changes

3. **Zone of Pain Issues** (Medium-High Priority)
   - Indicate rigid architecture
   - Block future changes
   - High risk for bugs

4. **SDP Violations** (Medium Priority)
   - Cause ripple effects
   - Increase maintenance cost
   - Unstable foundation

5. **High Coupling** (Medium Priority)
   - Maintainability risk
   - Testing difficulty
   - Change amplification

6. **Code Duplication** (Lower Priority)
   - Maintenance burden
   - Bug multiplication
   - Inconsistency risk

### Decision Flowchart

```
Is there a circular dependency?
├─ YES → Break immediately (extract interface, DI)
└─ NO  → Continue

Is there bidirectional dependency?
├─ YES → Refactor (mediator, event-driven)
└─ NO  → Continue

Is module in zone of pain?
├─ YES → Introduce abstractions
└─ NO  → Continue

Is SDP violated?
├─ YES → Stabilize or reduce afferent coupling
└─ NO  → Continue

Is coupling > threshold?
├─ YES → Reduce dependencies
└─ NO  → Continue

Is there significant duplication?
├─ YES → Extract common code
└─ NO  → Architecture is good!
```

## Integration with Debt Categories

Architectural analysis results are integrated with debtmap's debt categorization system:

### Debt Type Mapping

Architectural issues are mapped to existing DebtType enum variants:

- **Duplication** - Duplicated code blocks found
- **Dependency** - Used for circular dependencies and coupling issues
- **CodeOrganization** - May be used for architectural violations (SDP, zone issues)

**Note:** The DebtType enum does not have dedicated variants for CircularDependency, HighCoupling, or ArchitecturalViolation. Architectural issues are mapped to existing general-purpose debt types.

**Reference:** `src/core/mod.rs:220-236` for actual DebtType enum definition

### Tiered Prioritization

Architectural issues are assigned priority tiers:

- **Tier 1 (Critical)** - Circular dependencies, bidirectional dependencies
- **Tier 2 (High)** - Zone of pain, SDP violations
- **Tier 3 (Medium)** - High coupling, large duplications
- **Tier 4 (Low)** - Small duplications, minor coupling issues

**Reference:** See [Tiered Prioritization](tiered-prioritization.md) for complete priority assignment logic

## Cohesion Analysis

**Note:** Module cohesion analysis is currently a simplified placeholder implementation.

**Current status:** `src/debt/coupling.rs:82-95` (analyze_module_cohesion)

The function exists but provides basic cohesion calculation. Full cohesion analysis (measuring how well module elements belong together) is planned for a future release.

### Future Enhancement

Full cohesion analysis would measure:
- Functional cohesion (functions operating on related data)
- Sequential cohesion (output of one function feeds another)
- Communicational cohesion (functions operating on same data structures)

## Configuration

### Configurable Parameters

Configure duplication detection in `.debtmap.toml` or via CLI:

```toml
# Minimum lines for duplication detection
threshold_duplication = 50  # Default value
```

Or via command line:

```bash
debtmap analyze . --threshold-duplication 50
```

**Configuration reference:** `src/cli.rs:69` (threshold_duplication flag definition)

### Hardcoded Thresholds

**Note:** Most architectural thresholds are currently hardcoded in the implementation and cannot be configured. These thresholds are based on industry-standard metrics from Robert C. Martin's research and empirical software engineering studies:

- **Coupling threshold:** 5 (modules with >5 dependencies are flagged)
- **Instability threshold:** 0.8 (for SDP violations)
- **SDP afferent threshold:** 2 (minimum dependents for SDP violations)
- **Zone of pain thresholds:**
  - Abstractness < 0.2
  - Instability < 0.2
  - Afferent coupling > 3
- **Zone of uselessness thresholds:**
  - Abstractness > 0.8
  - Instability > 0.8

These values represent widely-accepted boundaries in software architecture literature. While they work well for most projects, configurable thresholds may be added in a future release to support domain-specific tuning.

**Source:** `src/debt/coupling.rs:70-76, 130, 145` (hardcoded threshold definitions)

See [Configuration](configuration.md) for complete options.

## Troubleshooting

### "No circular dependencies detected but build fails"

**Cause:** Circular dependencies at the package/crate level, not module level.

**Solution:** Use `cargo tree` to analyze package-level dependencies.

### "Too many coupling warnings"

**Cause:** Default threshold of 5 may be too strict for your codebase.

**Solution:** The coupling threshold is currently hardcoded at 5 in the implementation (`src/debt/coupling.rs:62`). To adjust it, you would need to modify the source code. Consider using suppression patterns to exclude specific modules if needed. See [Suppression Patterns](suppression-patterns.md).

### "Duplication detected in generated code"

**Cause:** Code generation tools create similar patterns.

**Solution:** Use suppression patterns to exclude generated files. See [Suppression Patterns](suppression-patterns.md).

### "Zone of pain false positives"

**Cause:** Utility modules are intentionally stable and concrete.

**Solution:** This is often correct - utility modules should be stable. Consider whether the module should be more abstract.

## Further Reading

### Robert C. Martin's Principles

The architectural metrics in debtmap are based on:

- **Clean Architecture** by Robert C. Martin
- **Agile Software Development: Principles, Patterns, and Practices** by Robert C. Martin
- Stable Dependencies Principle (SDP)
- Stable Abstractions Principle (SAP)
- Main Sequence distance metric

### Related Topics

- [Analysis Guide](analysis-guide/index.md) - Complete analysis workflow
- [Configuration](configuration.md) - Configuration options
- [Entropy Analysis](entropy-analysis.md) - Complexity vs. entropy
- [Scoring Strategies](scoring-strategies.md) - How debt is scored
- [Tiered Prioritization](tiered-prioritization.md) - Priority assignment
