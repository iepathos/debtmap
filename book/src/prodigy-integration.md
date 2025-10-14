# Prodigy Integration

Debtmap integrates with [Prodigy](https://github.com/iepathos/prodigy) to provide fully automated technical debt reduction through AI-driven workflows. This chapter explains how to set up and use Prodigy workflows to automatically refactor code, add tests, and improve codebase quality.

## What is Prodigy?

Prodigy is an AI-powered workflow automation system that uses Claude to execute complex multi-step tasks. When integrated with Debtmap, it can:

- **Automatically refactor** high-complexity functions identified by Debtmap
- **Add unit tests** for untested code
- **Fix code duplication** by extracting shared logic
- **Improve code organization** by addressing architectural issues
- **Validate improvements** with automated testing

All changes are made in isolated git worktrees, validated with tests and linting, and only committed if all checks pass.

## Benefits

### Automated Debt Reduction

Instead of manually addressing each technical debt item, Prodigy can:
1. Analyze Debtmap's output
2. Select high-priority items
3. Generate refactoring plans
4. Execute refactorings automatically
5. Validate with tests
6. Commit clean changes

### Iterative Improvement

Prodigy supports **iterative workflows**:
- Run analysis → fix top items → re-analyze → fix more
- Configurable iteration count (default: 5 iterations)
- Each iteration focuses on highest-priority remaining items

### Safe Experimentation

All changes happen in **isolated git worktrees**:
- Original branch remains untouched
- Failed attempts don't affect main codebase
- Easy to review before merging
- Automatic cleanup after workflow

## Prerequisites

### Install Prodigy

```bash
# Install Prodigy CLI
cargo install prodigy-cli

# Verify installation
prodigy --version
```

### Configure Claude API

```bash
# Set Claude API key
export ANTHROPIC_API_KEY="your-api-key-here"

# Or in ~/.prodigy/config.toml:
[api]
anthropic_key = "your-api-key-here"
```

### Ensure Debtmap is Installed

```bash
# Install Debtmap
cargo install debtmap

# Verify installation
debtmap --version
```

## Quick Start

### 1. Initialize Workflow

Create a workflow file `workflows/debtmap.yml`:

```yaml
name: Debtmap Debt Reduction
description: Automatically refactor high-complexity code and add tests

config:
  analysis_command: "debtmap analyze . --lcov target/coverage/lcov.info --format json --output debtmap-report.json"
  validation_command: "cargo test && cargo clippy"
  iterations: 5

steps:
  - name: analyze
    description: "Run Debtmap analysis with coverage"
    command: "debtmap analyze . --lcov target/coverage/lcov.info --format json --output debtmap-report.json"

  - name: identify_targets
    description: "Select top priority items from analysis"
    command: "jq '.items | sort_by(-.unified_score.final_score) | .[0:3]' debtmap-report.json"

  - name: refactor
    description: "Refactor high-complexity functions"
    agent: refactor-assistant
    targets: "{{ steps.identify_targets.output }}"

  - name: test
    description: "Add unit tests for untested code"
    agent: test-runner
    targets: "{{ steps.identify_targets.output }}"

  - name: validate
    description: "Run tests and linting"
    command: "cargo test && cargo clippy"
```

### 2. Run Workflow

```bash
# Run with default settings (5 iterations)
prodigy run workflows/debtmap.yml

# Run with custom iteration count
prodigy run workflows/debtmap.yml -n 10

# Run with dry-run mode
prodigy run workflows/debtmap.yml --dry-run
```

### 3. Review Results

Prodigy creates a detailed report:
```
📊 WORKFLOW SUMMARY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Iterations: 5
Items Fixed: 12
Tests Added: 8
Complexity Reduced: 145 → 78 (-46%)
Coverage Improved: 45% → 72% (+27%)

✅ All validations passed
```

## Workflow Configuration

### Basic Configuration

```yaml
config:
  # Analysis command (generates debtmap-report.json)
  analysis_command: "debtmap analyze . --format json --output debtmap-report.json"

  # Validation command (must pass before committing)
  validation_command: "cargo test && cargo clippy"

  # Number of iterations (default: 5)
  iterations: 5

  # Target types to focus on
  target_types:
    - complexity     # High-complexity functions
    - test_gap       # Untested code
    - duplication    # Code duplication

  # Priority thresholds
  min_priority_score: 7.0    # Only address items with score >= 7.0
```

### Coverage Integration

To include coverage-based prioritization:

```yaml
config:
  # Generate coverage before analysis
  setup_commands:
    - "cargo tarpaulin --out lcov --output-dir target/coverage"

  # Pass coverage file to debtmap
  analysis_command: "debtmap analyze . --lcov target/coverage/lcov.info --format json --output debtmap-report.json"
```

### Advanced Configuration

```yaml
config:
  # Map-reduce parallel processing
  parallel_agents: 3          # Run 3 agents in parallel

  # Retry failed items
  retry_failed: true
  max_retries: 2

  # Validation options
  validation:
    run_tests: true
    run_linting: true
    run_formatting: true

  # Git options
  create_pr: true             # Create pull request after workflow
  pr_title: "chore: automated debt reduction via Prodigy"
```

## Workflow Targets

Prodigy can target different types of technical debt:

### High Complexity Functions

```yaml
steps:
  - name: refactor_complexity
    description: "Refactor high-complexity functions"
    agent: refactor-assistant
    targets:
      filter: "debt_type == 'Complexity'"
      min_score: 8.0
      max_count: 5
```

### Untested Code

```yaml
steps:
  - name: add_tests
    description: "Add tests for untested functions"
    agent: test-runner
    targets:
      filter: "debt_type == 'TestGap'"
      min_score: 7.0
      max_count: 10
```

### Code Duplication

```yaml
steps:
  - name: fix_duplication
    description: "Extract duplicated code"
    agent: refactor-assistant
    targets:
      filter: "debt_type == 'Duplication'"
      min_lines: 25
      max_count: 3
```

### God Objects

```yaml
steps:
  - name: split_god_objects
    description: "Split god objects into smaller modules"
    agent: refactor-assistant
    targets:
      filter: "debt_type == 'GodObject'"
      min_score: 8.5
```

## Map-Reduce Workflows

For large codebases, use **map-reduce parallel processing**:

### Map Phase

```yaml
map:
  agent: refactor-assistant
  parallel: 5                 # 5 agents working in parallel
  targets:
    filter: "unified_score.final_score >= 7.0"
    max_count: 20
  task: "Refactor this function to reduce complexity below 10"
```

### Reduce Phase

```yaml
reduce:
  agent: code-reviewer
  task: "Review all refactored code for consistency and quality"
  validation:
    - "cargo test --all"
    - "cargo clippy --all-targets -- -D warnings"
```

## Iteration Strategy

### How Iterations Work

1. **Iteration 1**: Analyze, fix top 3-5 items, validate
2. **Iteration 2**: Re-analyze (scores updated), fix next 3-5 items, validate
3. **Continue** until iteration limit or no high-priority items remain

### Configuring Iterations

```yaml
config:
  iterations: 5               # Total iterations

  # Items per iteration
  targets_per_iteration: 3

  # Stop early if no items above threshold
  early_stop: true
  early_stop_threshold: 6.0
```

### Example Output

```
Iteration 1:
  - Fixed: parse_expression() (9.2 → 5.1)
  - Fixed: calculate_score() (8.8 → 4.2)
  - Fixed: apply_weights() (8.5 → 5.8)
  ✓ Tests pass

Iteration 2:
  - Fixed: normalize_results() (7.5 → 3.9)
  - Fixed: aggregate_data() (7.2 → 4.1)
  ✓ Tests pass

Iteration 3:
  - No items above threshold (6.0)
  ✓ Early stop

Final Results:
  Items fixed: 5
  Average complexity: 15.2 → 8.6
```

## Validation

Prodigy validates all changes before committing:

### Default Validation

```yaml
validation:
  commands:
    - "cargo test"            # All tests must pass
    - "cargo clippy"          # No clippy warnings
    - "cargo fmt -- --check"  # Code must be formatted
```

### Custom Validation

```yaml
validation:
  commands:
    - "cargo test --all-features"
    - "cargo clippy --all-targets -- -D warnings"
    - "cargo deny check"
    - "cargo doc --no-deps"

  # Require coverage threshold
  coverage_threshold: 80.0

  # Re-run debtmap and ensure improvement
  debt_regression_check: true
```

### Validation Failures

If validation fails:
- Changes are **not committed**
- Detailed error log is saved
- Workflow continues with next item
- Failed items can be retried (if `retry_failed: true`)

## Output and Metrics

### Workflow Report

```json
{
  "workflow": "debtmap-debt-reduction",
  "iterations": 5,
  "items_processed": 12,
  "items_fixed": 10,
  "items_failed": 2,
  "metrics": {
    "complexity_before": 145,
    "complexity_after": 78,
    "complexity_reduction": -46.2,
    "coverage_before": 45.3,
    "coverage_after": 72.1,
    "coverage_improvement": 26.8
  },
  "changes": [
    {
      "file": "src/parser.rs",
      "function": "parse_expression",
      "before_score": 9.2,
      "after_score": 5.1,
      "improvements": ["Reduced complexity", "Added tests"]
    }
  ]
}
```

### Commit Messages

Prodigy generates descriptive commit messages:

```
refactor(parser): reduce complexity in parse_expression

- Extract nested conditionals to helper functions
- Add unit tests for edge cases
- Coverage: 0% → 85%
- Complexity: 22 → 8

Generated by Prodigy workflow: debtmap-debt-reduction
Iteration: 1/5
```

## Integration with CI/CD

### GitHub Actions

```yaml
name: Prodigy Debt Reduction

on:
  schedule:
    - cron: '0 0 * * 0'  # Weekly on Sunday

jobs:
  reduce-debt:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Prodigy
        run: cargo install prodigy-cli

      - name: Run Prodigy workflow
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
        run: prodigy run workflows/debtmap.yml -n 5

      - name: Create PR
        uses: peter-evans/create-pull-request@v5
        with:
          title: "chore: automated debt reduction"
          body: "Automated technical debt reduction via Prodigy"
```

### GitLab CI

```yaml
prodigy-debt-reduction:
  stage: quality
  rules:
    - if: '$CI_PIPELINE_SOURCE == "schedule"'
  script:
    - cargo install prodigy-cli
    - prodigy run workflows/debtmap.yml -n 5
  artifacts:
    reports:
      metrics: prodigy-report.json
```

## Best Practices

### 1. Start Small

Begin with low iteration counts:
```bash
# First run: 1-2 iterations
prodigy run workflows/debtmap.yml -n 2
```

### 2. Focus on High-Priority Items

```yaml
config:
  min_priority_score: 8.0    # Only critical items
  targets_per_iteration: 3   # Small batches
```

### 3. Validate Thoroughly

```yaml
validation:
  commands:
    - "cargo test --all-features"
    - "cargo clippy --all-targets -- -D warnings"
    - "cargo fmt -- --check"
  coverage_threshold: 75.0
```

### 4. Review Before Merging

Always review Prodigy's changes:
```bash
# Check worktree changes
cd .prodigy/worktrees/session-xxx
git diff master

# Review commit history
git log --oneline
```

### 5. Use Dry-Run Mode

Test workflows without making changes:
```bash
prodigy run workflows/debtmap.yml --dry-run
```

## Troubleshooting

### Workflow Fails to Start

**Issue**: "Prodigy not found" or "API key missing"

**Solution**:
```bash
# Install Prodigy
cargo install prodigy-cli

# Set API key
export ANTHROPIC_API_KEY="your-key"
```

### Validation Failures

**Issue**: Tests fail after refactoring

**Solution**: Check validation logs:
```bash
# View detailed logs
cat .prodigy/logs/validation-errors.log

# Run validation manually
cargo test
cargo clippy
```

### No Items Processed

**Issue**: Workflow completes but fixes nothing

**Solution**: Lower thresholds:
```yaml
config:
  min_priority_score: 5.0    # Lower from 7.0
  early_stop: false          # Don't stop early
```

### Agent Timeout

**Issue**: "Agent timeout after 300 seconds"

**Solution**: Increase timeout:
```yaml
config:
  agent_timeout: 600         # 10 minutes
```

## Example Workflows

### Full Repository Cleanup

```yaml
name: Full Repository Debt Reduction
config:
  iterations: 10
  targets_per_iteration: 5
  validation_command: "cargo test --all && cargo clippy --all-targets -- -D warnings"

steps:
  - name: analyze
    command: "debtmap analyze . --lcov coverage.lcov --format json -o report.json"

  - name: refactor
    agent: refactor-assistant
    targets:
      filter: "debt_type == 'Complexity' && unified_score.final_score >= 7.0"

  - name: test
    agent: test-runner
    targets:
      filter: "debt_type == 'TestGap' && unified_score.final_score >= 7.0"

  - name: deduplicate
    agent: refactor-assistant
    targets:
      filter: "debt_type == 'Duplication'"
```

### Focused Testing Sprint

```yaml
name: Test Coverage Improvement
config:
  iterations: 5
  focus: testing

steps:
  - name: analyze
    command: "debtmap analyze . --lcov coverage.lcov --format json -o report.json"

  - name: add_tests
    agent: test-runner
    targets:
      filter: "debt_type == 'TestGap'"
      min_score: 6.0
      max_count: 10
```

## See Also

- [Debtmap CLI Reference](./cli-reference.md) - Debtmap command options
- [Configuration](./configuration.md) - Debtmap configuration
- [Tiered Prioritization](./tiered-prioritization.md) - Understanding priority tiers
- [Prodigy Documentation](https://github.com/iepathos/prodigy) - Full Prodigy reference
