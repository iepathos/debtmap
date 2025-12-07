# Why AI "Fails" at Complex Tasks - Code Examples

This directory contains runnable Rust code examples demonstrating the concepts from the blog post "Why AI 'Fails' at Complex Tasks: It's Your Technical Debt, Not AI Limitations".

## Examples

### ETL (Extract-Transform-Load) Examples

**`etl_bad.rs`** - Demonstrates poor separation of concerns:
- Database queries mixed with transformations
- Side effects scattered throughout
- Business logic intertwined with I/O
- High cognitive complexity requiring understanding of entire pipeline

**`etl_good.rs`** - Demonstrates clean architecture:
- Pure transformation functions (no I/O)
- Clear separation of business logic and infrastructure
- Easy to test and reason about locally
- Includes comprehensive unit tests

### Event Handling Examples

**`events_bad.rs`** - Demonstrates tangled event handling:
- Event infrastructure, business logic, and side effects mixed
- Requires understanding event bus, email service, analytics, database schema
- Nested event publishing increases complexity
- Hard to test and modify

**`events_good.rs`** - Demonstrates clean event handling:
- Pure domain logic separated from infrastructure
- Domain events as first-class types
- Infrastructure wrapper handles I/O concerns
- Easy to test with simple unit tests

## Running the Examples

```bash
# Run individual examples
cargo run --bin etl_bad
cargo run --bin etl_good
cargo run --bin events_bad
cargo run --bin events_good

# Run tests (for good examples)
cargo test
```

## Analyzing with Debtmap

These examples are designed to be analyzed with debtmap to demonstrate the difference in technical debt scores:

```bash
# Analyze all examples
debtmap analyze .

# Compare bad vs good scores
debtmap analyze etl_bad.rs
debtmap analyze etl_good.rs
debtmap analyze events_bad.rs
debtmap analyze events_good.rs
```

The "bad" examples should show:
- Higher cognitive complexity scores
- Lower test coverage (0% for bad examples)
- Higher overall risk scores

The "good" examples should show:
- Lower cognitive complexity (thanks to pure functions)
- Higher test coverage (includes unit tests)
- Lower overall risk scores

## Key Takeaways

1. **ETL Example**: Shows how separating data transformation (pure functions) from I/O reduces cognitive load
2. **Event Example**: Shows how domain events can abstract away infrastructure concerns
3. **Both Examples**: Demonstrate that "complex domains" can have clean code when properly structured

## Related Blog Post

See the full blog post at: https://entropicdrift.com/blog/why-ai-fails-complex-tasks/
