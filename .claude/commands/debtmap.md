---
name: fix-tech-debt
description: Analyze tech debt with debtmap, fix the top priority item, test, and commit
---

# Fix Top Priority Tech Debt

Use debtmap to analyze the repository and identify tech debt, then fix the highest priority item.

## Process

1. **Initial analysis** - Run `debtmap analyze .` to identify issues and capture baseline debt score
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
7. **Final analysis** - Run `debtmap analyze .` again to measure debt score improvement
8. **Commit** - Create a clear commit message including the debt score change

## Important Instructions

**IMPORTANT**: When making ANY commits, do NOT include attribution text like "🤖 Generated with Claude Code" or "Co-Authored-By: Claude" in commit messages. Keep commits clean and focused on the actual changes.

## Steps

```bash
# First, analyze the codebase and capture initial debt score
echo "Initial analysis:"
debtmap analyze . | tee /tmp/debtmap_initial.txt
INITIAL_SCORE=$(grep "Total debt score:" /tmp/debtmap_initial.txt | sed -E 's/.*Total debt score: ([0-9]+).*/\1/')
echo "Initial debt score: $INITIAL_SCORE"

# After fixing, verify everything works
cargo test
cargo clippy -- -D warnings
cargo fmt --check

# Run debtmap again to get the new debt score
echo "Final analysis:"
debtmap analyze . | tee /tmp/debtmap_final.txt
FINAL_SCORE=$(grep "Total debt score:" /tmp/debtmap_final.txt | sed -E 's/.*Total debt score: ([0-9]+).*/\1/')
echo "Final debt score: $FINAL_SCORE"

# Calculate the change
SCORE_CHANGE=$((INITIAL_SCORE - FINAL_SCORE))
if [ $SCORE_CHANGE -gt 0 ]; then
    CHANGE_MESSAGE="Reduced debt score by $SCORE_CHANGE (from $INITIAL_SCORE to $FINAL_SCORE)"
elif [ $SCORE_CHANGE -lt 0 ]; then
    CHANGE_MESSAGE="Increased debt score by $((-SCORE_CHANGE)) (from $INITIAL_SCORE to $FINAL_SCORE)"
else
    CHANGE_MESSAGE="Debt score unchanged at $FINAL_SCORE"
fi

# Commit with clear message about tech debt reduction
git add -A
git commit -m "fix: [description of tech debt fixed]

- [Specific improvements made]
- [Impact on code quality]
- $CHANGE_MESSAGE

Tech debt category: [from debtmap analysis]"
```

## Success Criteria

- [ ] Initial debtmap analysis completed and baseline score captured
- [ ] Top priority issue identified and fixed
- [ ] All tests passing
- [ ] No clippy warnings
- [ ] Code formatted properly
- [ ] Final debtmap analysis shows debt score improvement
- [ ] Changes committed with descriptive message including debt score change