---
name: fix-tech-debt
description: Analyze tech debt with debtmap, fix the top priority item, test, and commit
---

# Fix Top Priority Tech Debt

Use debtmap to analyze the repository and identify tech debt, then fix the highest priority item.

## Process

1. **Analyze tech debt** - Run `debtmap analyze .` to identify issues
2. **Prioritize** - Select the top priority item from the analysis
3. **Plan fix** - Create implementation plan for the fix
4. **Implement** - Fix the issue using functional programming patterns where appropriate for idiomatic Rust:
   - Prefer iterators over loops
   - Use pattern matching over if-else chains
   - Favor immutability and ownership patterns
   - Use Result/Option for error handling
   - Prefer pure functions without side effects
5. **Test** - Run all tests to ensure nothing breaks
6. **Verify** - Run cargo clippy and cargo fmt
7. **Commit** - Create a clear commit message describing the tech debt fix

## Steps

```bash
# First, analyze the codebase
debtmap analyze .

# After fixing, verify everything works
cargo test
cargo clippy -- -D warnings
cargo fmt --check

# Commit with clear message about tech debt reduction
git add -A
git commit -m "fix: [description of tech debt fixed]

- [Specific improvements made]
- [Impact on code quality]

Tech debt category: [from debtmap analysis]"
```

## Success Criteria

- [ ] Debtmap analysis completed
- [ ] Top priority issue identified and fixed
- [ ] All tests passing
- [ ] No clippy warnings
- [ ] Code formatted properly
- [ ] Changes committed with descriptive message