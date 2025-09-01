# Using Claude Code, prodigy cook, and debtmap for Automated Technical Debt Reduction

This guide explains how to use `prodigy cook` with the `workflows/debtmap.yml` workflow to automatically reduce technical debt through iterative Claude-powered improvements.

## What is prodigy?

`prodigy` (Prodigy) is an AI workflow automation tool that enables declarative workflows using Claude. It creates isolated git worktrees for each session and orchestrates multi-step improvement cycles.

## The Debtmap Workflow

### Command

```bash
prodigy cook workflows/debtmap.yml -wyn 5
```

Options:
- `-w` - Create a worktree (isolated git branch for changes)
- `-y` - Auto-confirm (skip confirmation prompts)
- `-n 5` - Run 5 iterations maximum

### What Happens

When you run this command, prodigy:

1. **Creates an isolated worktree** at `~/.prodigy/worktrees/debtmap/session-[uuid]`
2. **Executes the workflow** for up to 5 iterations
3. Each iteration runs three steps:
   - **Claude debtmap analysis** - Identifies and fixes top priority technical debt
   - **Test validation** - Runs `just test` to ensure all tests pass
   - **Linting** - Applies formatting and linting fixes

### Workflow Structure (`workflows/debtmap.yml`)

```yaml
- claude: "/debtmap"
  commit_required: true

- shell: "just test"
  on_failure:
    claude: "/prodigy-debug-test-failure --spec $ARG --output ${shell.output}"
    max_attempts: 3
    fail_workflow: false  # Continue workflow even if tests can't be fixed

# Run linting and formatting after implementation
- claude: "/prodigy-lint"
```

## The Claude Commands

### `/debtmap` Command (`.claude/commands/debtmap.md`)

This command:
1. **Generates coverage** using `cargo tarpaulin`
2. **Analyzes technical debt** with coverage-based risk scoring
3. **Selects top priority item** based on complexity and coverage
4. **Implements the fix** using functional programming patterns
5. **Validates the fix** with tests, clippy, and formatting
6. **Measures improvement** by comparing debt scores
7. **Commits changes** with detailed debt reduction metrics

### Key Features

- **Risk-based prioritization**: Uses complexity × (100 - coverage%) to identify highest risk code
- **Functional patterns**: Applies Rust idioms (iterators, pattern matching, Result/Option)
- **Automated testing**: Ensures no regressions with comprehensive test runs
- **Score tracking**: Measures actual debt reduction in each commit

## Example Session Output

```bash
$ prodigy cook workflows/debtmap.yml -wyn 5
ℹ️  Created worktree at: /Users/glen/.prodigy/worktrees/debtmap/session-76d7d5a2
ℹ️  Executing workflow: default (max 5 iterations)
🔄 Starting improvement loop
🔄 Starting iteration 1/5
🔄 Executing step 1/3: claude: /debtmap
✅ ✓ claude: /debtmap created commits
🔄 Executing step 2/3: test: just test
✅ ✓ Tests passed on attempt 1
🔄 Executing step 3/3: claude: /prodigy-lint
ℹ️  ✓ Iteration 1 completed in 5m 43s
```

## What Gets Fixed

In each iteration, the workflow targets:

1. **High complexity functions** (cyclomatic complexity > 10)
2. **Untested complex code** (low coverage on complex functions)
3. **Code duplication** (repeated blocks > 50 lines)
4. **Deep nesting** (indentation levels > 4)
5. **Long functions** (> 100 lines)

## Monitoring Progress

### During Execution

The workflow provides real-time feedback:
- Initial debt score before changes
- Specific issue being addressed
- Test results after each fix
- Final debt score showing improvement

### After Completion

Review changes in the worktree:

```bash
# Navigate to the worktree
cd ~/.prodigy/worktrees/debtmap/session-[uuid]

# Review all commits made
git log --oneline

# See detailed changes
git show

# Check overall debt reduction
debtmap analyze . --coverage-file target/coverage/lcov.info
```

## Integration Strategy

### Daily Workflow

1. **Morning run**: Start the day with debt reduction
   ```bash
   prodigy cook workflows/debtmap.yml -wyn 5
   ```

2. **Review changes**: Examine what was fixed
   ```bash
   cd ~/.prodigy/worktrees/debtmap/session-*
   git log --stat
   ```

3. **Merge improvements**: If satisfied, merge to main
   ```bash
   git checkout main
   git merge session-[uuid]
   ```

### Continuous Improvement

Schedule regular runs:
```bash
# Add to crontab for nightly runs
0 2 * * * cd /path/to/project && prodigy cook workflows/debtmap.yml -wyn 10
```

## Customization

### Adjusting Iterations

- **Quick improvement**: `-n 1` for single fix
- **Sprint debt reduction**: `-n 10` for comprehensive cleanup
- **Overnight run**: `-n 20` for major refactoring

### Modifying Workflow

Edit `workflows/debtmap.yml` to:
- Add validation steps
- Include additional linters
- Run integration tests
- Deploy to staging

Example enhanced workflow:
```yaml
- claude: "/debtmap"
  commit_required: true

- shell: "cargo test"
  on_failure:
    claude: "/fix-tests"
    max_attempts: 3

- shell: "cargo clippy -- -D warnings"
  
- claude: "/prodigy-lint"

- shell: "cargo bench"
  continue_on_failure: true
```

## Safety Features

### Worktree Isolation

- Changes are made in isolated worktrees
- Original branch remains untouched
- Easy rollback if needed

### Test Gates

- Every iteration must pass tests
- Failed tests trigger automatic fix attempts
- Workflow continues even if individual fixes fail

### Commit Tracking

Each commit includes:
- Specific debt category addressed
- Debt score change metrics
- Detailed description of improvements

## Best Practices

### 1. Start Small

Begin with fewer iterations to understand the impact:
```bash
prodigy cook workflows/debtmap.yml -wyn 1
```

### 2. Review Before Merging

Always review the automated changes:
```bash
# Diff against main
git diff main..HEAD

# Run comprehensive tests
cargo test --all-features
cargo tarpaulin --out html
```

### 3. Track Metrics

Monitor debt reduction over time:
```bash
# Before prodigy run
debtmap analyze . > before.txt

# After prodigy run
debtmap analyze . > after.txt

# Compare
diff before.txt after.txt
```

### 4. Combine with Manual Review

Use prodigy for automated fixes, then:
- Review architectural decisions
- Add documentation for complex changes
- Update tests for edge cases

## Troubleshooting

### Workflow Fails

If the workflow fails:
```bash
# Check logs in worktree
cd ~/.prodigy/worktrees/debtmap/session-*
git status
cargo test
```

### Tests Don't Pass

The workflow includes automatic test fixing:
- Up to 3 attempts per test failure
- Continues workflow even if unfixable
- Review manual fixes needed

### No Improvements Found

If debt score doesn't improve:
- Check coverage generation succeeded
- Review threshold settings
- Consider focusing on specific modules

## Advanced Usage

### Targeting Specific Modules

Create a custom workflow for focused improvement:
```yaml
- claude: "/debtmap --focus src/core"
  commit_required: true
  
- shell: "cargo test core::"
  
- claude: "/prodigy-lint src/core"
```

### Parallel Workflows

Run multiple improvement streams:
```bash
# Terminal 1: Fix complexity
prodigy cook workflows/complexity.yml -wyn 5

# Terminal 2: Improve coverage
prodigy cook workflows/coverage.yml -wyn 5

# Terminal 3: Remove duplication
prodigy cook workflows/duplication.yml -wyn 5
```

### CI Integration

Add to GitHub Actions:
```yaml
name: Weekly Debt Reduction
on:
  schedule:
    - cron: '0 0 * * 0'  # Weekly on Sunday
  workflow_dispatch:

jobs:
  reduce-debt:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run prodigy debt reduction
        run: |
          prodigy cook workflows/debtmap.yml -wyn 10
      - name: Create PR
        run: |
          gh pr create --title "Weekly debt reduction" \
            --body "Automated technical debt improvements"
```

## Results and Benefits

Using this workflow typically achieves:

- **30-50% complexity reduction** in targeted functions
- **20-40% coverage improvement** for critical paths
- **60-80% duplication elimination** across modules
- **Consistent code style** through automated formatting
- **Incremental improvements** without breaking changes

## Summary

The `prodigy cook workflows/debtmap.yml -wyn 5` command provides:
1. **Automated debt reduction** through Claude-powered analysis
2. **Safe iteration** in isolated worktrees
3. **Validated improvements** with comprehensive testing
4. **Measurable progress** through debt score tracking
5. **Continuous improvement** without manual intervention

This creates a powerful feedback loop where code quality improves automatically while maintaining stability and test coverage.
