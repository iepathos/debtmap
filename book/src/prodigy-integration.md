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
cargo install prodigy

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
# Sequential workflow. Fix top technical debt item

# Phase 1: Generate coverage data
- shell: "just coverage-lcov"

# Phase 2: Analyze tech debt and capture baseline
- shell: "debtmap analyze . --lcov target/coverage/lcov.info --output .prodigy/debtmap-before.json --format json"

# Phase 3: Create implementation plan (PLANNING PHASE)
- claude: "/prodigy-debtmap-plan --before .prodigy/debtmap-before.json --output .prodigy/IMPLEMENTATION_PLAN.md"
  capture_output: true
  validate:
    commands:
      - claude: "/prodigy-validate-debtmap-plan --before .prodigy/debtmap-before.json --plan .prodigy/IMPLEMENTATION_PLAN.md --output .prodigy/plan-validation.json"
    result_file: ".prodigy/plan-validation.json"
    threshold: 75
    on_incomplete:
      commands:
        - claude: "/prodigy-revise-debtmap-plan --gaps ${validation.gaps} --plan .prodigy/IMPLEMENTATION_PLAN.md"
      max_attempts: 3
      fail_workflow: false

# Phase 4: Execute the plan (IMPLEMENTATION PHASE)
- claude: "/prodigy-debtmap-implement --plan .prodigy/IMPLEMENTATION_PLAN.md"
  commit_required: true
  validate:
    commands:
      - shell: "debtmap analyze . --lcov target/coverage/lcov.info --output .prodigy/debtmap-after.json --format json"
      - shell: "debtmap compare --before .prodigy/debtmap-before.json --after .prodigy/debtmap-after.json --plan .prodigy/IMPLEMENTATION_PLAN.md --output .prodigy/comparison.json --format json"
      - claude: "/prodigy-validate-debtmap-improvement --comparison .prodigy/comparison.json --output .prodigy/debtmap-validation.json"
    result_file: ".prodigy/debtmap-validation.json"
    threshold: 75
    on_incomplete:
      commands:
        - claude: "/prodigy-complete-debtmap-fix --gaps ${validation.gaps} --plan .prodigy/IMPLEMENTATION_PLAN.md"
          commit_required: true
        - shell: "just coverage-lcov"
        - shell: "debtmap analyze . --lcov target/coverage/lcov.info --output .prodigy/debtmap-after.json --format json"
        - shell: "debtmap compare --before .prodigy/debtmap-before.json --after .prodigy/debtmap-after.json --plan .prodigy/IMPLEMENTATION_PLAN.md --output .prodigy/comparison.json --format json"
      max_attempts: 5
      fail_workflow: true

# Phase 5: Run tests with automatic fixing
- shell: "just test"
  on_failure:
    claude: "/prodigy-debug-test-failure --output ${shell.output}"
    max_attempts: 5
    fail_workflow: true

# Phase 6: Run linting and formatting
- shell: "just fmt-check && just lint"
  on_failure:
    claude: "/prodigy-lint ${shell.output}"
    max_attempts: 5
    fail_workflow: true
```

### 2. Run Workflow

```bash
# Run with worktree, auto-confirm, 5 iterations
prodigy cook workflows/debtmap.yml -wyn 5

# Run with custom iteration count
prodigy cook workflows/debtmap.yml -wyn 10

# Run single iteration for testing
prodigy cook workflows/debtmap.yml -wyn 1
```

**Command Flags:**
- `-w` - Create an isolated git worktree for changes
- `-y` - Auto-confirm workflow steps (skip prompts)
- `-n 5` - Run workflow for up to 5 iterations

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

Prodigy workflows are defined as YAML lists of steps. Each step can be either a `shell` command or a `claude` slash command.

### Workflow Step Types

#### Shell Commands

Execute shell commands directly:

```yaml
- shell: "cargo test"
```

With error handling:

```yaml
- shell: "just test"
  on_failure:
    claude: "/prodigy-debug-test-failure --output ${shell.output}"
    max_attempts: 5
    fail_workflow: true
```

#### Claude Commands

Execute Claude Code slash commands:

```yaml
- claude: "/prodigy-debtmap-plan --before .prodigy/debtmap-before.json --output .prodigy/IMPLEMENTATION_PLAN.md"
  capture_output: true
  commit_required: true
```

### Step-Level Validation

Steps can include validation that must pass:

```yaml
- claude: "/prodigy-debtmap-implement --plan .prodigy/IMPLEMENTATION_PLAN.md"
  commit_required: true
  validate:
    commands:
      - shell: "cargo test"
      - shell: "cargo clippy -- -D warnings"
    result_file: ".prodigy/validation.json"
    threshold: 75
    on_incomplete:
      commands:
        - claude: "/prodigy-complete-debtmap-fix --gaps ${validation.gaps} --plan .prodigy/IMPLEMENTATION_PLAN.md"
          commit_required: true
      max_attempts: 5
      fail_workflow: true
```

**Validation Options:**
- `commands`: List of commands to run for validation
- `result_file`: JSON file containing validation results
- `threshold`: Minimum score (0-100) required to pass
- `on_incomplete`: Actions to take if validation score < threshold
- `max_attempts`: Maximum retry attempts
- `fail_workflow`: Whether to fail entire workflow if validation never passes

### Error Handling

Use `on_failure` to handle command failures:

```yaml
- shell: "just fmt-check && just lint"
  on_failure:
    claude: "/prodigy-lint ${shell.output}"
    max_attempts: 5
    fail_workflow: true
```

**Error Handling Options:**
- `claude`: Slash command to fix the failure
- `max_attempts`: Maximum fix attempts
- `fail_workflow`: If true, workflow fails after max_attempts; if false, continues to next step

### Coverage Integration

Generate and use coverage data in workflows:

```yaml
# Generate coverage
- shell: "just coverage-lcov"

# Use coverage in analysis
- shell: "debtmap analyze . --lcov target/coverage/lcov.info --output .prodigy/debtmap-before.json --format json"
```

## Claude Slash Commands

Prodigy workflows use Claude Code slash commands to perform analysis, planning, and implementation. The key commands used in the debtmap workflow are:

### Planning Commands

#### `/prodigy-debtmap-plan`

Creates an implementation plan for the top priority debt item.

```yaml
- claude: "/prodigy-debtmap-plan --before .prodigy/debtmap-before.json --output .prodigy/IMPLEMENTATION_PLAN.md"
  capture_output: true
```

**Parameters:**
- `--before`: Path to debtmap analysis JSON file
- `--output`: Path to write implementation plan

#### `/prodigy-validate-debtmap-plan`

Validates that the implementation plan is complete and addresses the debt item.

```yaml
- claude: "/prodigy-validate-debtmap-plan --before .prodigy/debtmap-before.json --plan .prodigy/IMPLEMENTATION_PLAN.md --output .prodigy/plan-validation.json"
```

**Parameters:**
- `--before`: Original debtmap analysis
- `--plan`: Implementation plan to validate
- `--output`: Validation results JSON (with score 0-100)

#### `/prodigy-revise-debtmap-plan`

Revises an incomplete plan based on validation gaps.

```yaml
- claude: "/prodigy-revise-debtmap-plan --gaps ${validation.gaps} --plan .prodigy/IMPLEMENTATION_PLAN.md"
```

**Parameters:**
- `--gaps`: List of missing items from validation
- `--plan`: Plan file to update

### Implementation Commands

#### `/prodigy-debtmap-implement`

Executes the implementation plan.

```yaml
- claude: "/prodigy-debtmap-implement --plan .prodigy/IMPLEMENTATION_PLAN.md"
  commit_required: true
```

**Parameters:**
- `--plan`: Path to implementation plan

#### `/prodigy-validate-debtmap-improvement`

Validates that the implementation successfully addressed the debt item.

```yaml
- claude: "/prodigy-validate-debtmap-improvement --comparison .prodigy/comparison.json --output .prodigy/debtmap-validation.json"
```

**Parameters:**
- `--comparison`: Debtmap comparison results (before vs after)
- `--output`: Validation results JSON (with score 0-100)

#### `/prodigy-complete-debtmap-fix`

Completes a partial fix based on validation gaps.

```yaml
- claude: "/prodigy-complete-debtmap-fix --gaps ${validation.gaps} --plan .prodigy/IMPLEMENTATION_PLAN.md"
  commit_required: true
```

**Parameters:**
- `--gaps`: Validation gaps to address
- `--plan`: Original implementation plan

### Testing and Quality Commands

#### `/prodigy-debug-test-failure`

Automatically fixes failing tests.

```yaml
- shell: "just test"
  on_failure:
    claude: "/prodigy-debug-test-failure --output ${shell.output}"
    max_attempts: 5
```

**Parameters:**
- `--output`: Test failure output from shell command

#### `/prodigy-lint`

Fixes linting and formatting issues.

```yaml
- shell: "just fmt-check && just lint"
  on_failure:
    claude: "/prodigy-lint ${shell.output}"
    max_attempts: 5
```

**Parameters:**
- Shell output with linting errors

## Target Selection

Target selection happens through the debtmap analysis and slash commands, not through workflow configuration:

### How Targets Are Selected

1. **Debtmap analyzes** the codebase and scores all items by complexity, coverage, and risk
2. **Planning command** (`/prodigy-debtmap-plan`) selects the highest priority item
3. **Implementation command** (`/prodigy-debtmap-implement`) fixes that specific item
4. **Next iteration** re-analyzes and selects the next highest priority item

### Factors in Prioritization

- **Complexity score**: Functions with cyclomatic complexity > 10
- **Coverage percentage**: Lower coverage increases priority
- **Risk score**: Complexity × (100 - coverage%)
- **Debt type**: Complexity, TestGap, Duplication, GodObject, DeepNesting

### Customizing Target Selection

To focus on specific debt types or modules, modify the slash commands or create custom commands in `.claude/commands/`

## Map-Reduce Workflows

Prodigy supports map-reduce workflows for processing multiple items in parallel. This is useful for large-scale refactoring tasks.

### When to Use Map-Reduce

- Processing multiple independent debt items simultaneously
- Applying the same fix pattern across many files
- Large-scale codebase cleanup tasks

### Map-Reduce Structure

The exact syntax for map-reduce workflows in Prodigy may differ from sequential workflows. Consult the Prodigy documentation for current map-reduce syntax and examples.

**Key Concepts:**
- **Map phase**: Process items in parallel using multiple agents
- **Reduce phase**: Aggregate results and ensure consistency
- **Isolation**: Each map agent works in its own worktree
- **Validation**: All changes must pass validation before merging

## Iteration Strategy

### How Iterations Work

When you run `prodigy cook workflows/debtmap.yml -wyn 5`, the workflow executes up to 5 times:

1. **Iteration 1**:
   - Analyze codebase with debtmap
   - Select highest priority item
   - Create implementation plan
   - Execute plan and validate
   - Run tests and linting

2. **Iteration 2**:
   - Re-analyze codebase (scores updated based on Iteration 1 changes)
   - Select next highest priority item
   - Repeat plan/implement/validate cycle

3. **Continue** until iteration limit reached or workflow completes without finding issues

### Controlling Iterations

Iterations are controlled via the `-n` flag:

```bash
# Single iteration (testing)
prodigy cook workflows/debtmap.yml -wyn 1

# Standard run (5 iterations)
prodigy cook workflows/debtmap.yml -wyn 5

# Deep cleanup (10+ iterations)
prodigy cook workflows/debtmap.yml -wyn 20
```

### What Happens Each Iteration

Each iteration runs the **entire workflow from start to finish**:

1. Generate coverage data
2. Analyze technical debt
3. Create implementation plan
4. Execute plan
5. Validate improvement
6. Run tests (with auto-fixing)
7. Run linting (with auto-fixing)

The workflow continues to the next iteration automatically if all steps succeed.

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

Prodigy validates changes at the workflow step level, not as a standalone configuration.

### Step-Level Validation

Validation is attached to specific workflow steps:

```yaml
- claude: "/prodigy-debtmap-implement --plan .prodigy/IMPLEMENTATION_PLAN.md"
  commit_required: true
  validate:
    commands:
      - shell: "debtmap analyze . --lcov target/coverage/lcov.info --output .prodigy/debtmap-after.json --format json"
      - shell: "debtmap compare --before .prodigy/debtmap-before.json --after .prodigy/debtmap-after.json --plan .prodigy/IMPLEMENTATION_PLAN.md --output .prodigy/comparison.json --format json"
      - claude: "/prodigy-validate-debtmap-improvement --comparison .prodigy/comparison.json --output .prodigy/debtmap-validation.json"
    result_file: ".prodigy/debtmap-validation.json"
    threshold: 75
    on_incomplete:
      commands:
        - claude: "/prodigy-complete-debtmap-fix --gaps ${validation.gaps} --plan .prodigy/IMPLEMENTATION_PLAN.md"
          commit_required: true
        - shell: "just coverage-lcov"
        - shell: "debtmap analyze . --lcov target/coverage/lcov.info --output .prodigy/debtmap-after.json --format json"
        - shell: "debtmap compare --before .prodigy/debtmap-before.json --after .prodigy/debtmap-after.json --plan .prodigy/IMPLEMENTATION_PLAN.md --output .prodigy/comparison.json --format json"
      max_attempts: 5
      fail_workflow: true
```

### Validation Process

1. **Commands run**: Execute validation commands (shell or claude)
2. **Check result file**: Read JSON file specified in `result_file`
3. **Compare to threshold**: Score must be >= threshold (0-100 scale)
4. **On incomplete**: If score < threshold, run `on_incomplete` commands
5. **Retry**: Repeat up to `max_attempts` times
6. **Fail or continue**: If `fail_workflow: true`, stop workflow; otherwise continue

### Validation Result Format

The `result_file` JSON should contain:

```json
{
  "score": 85,
  "passed": true,
  "gaps": [],
  "details": "All debt improvement criteria met"
}
```

### Test Validation with Auto-Fix

Tests are validated with automatic fixing on failure:

```yaml
- shell: "just test"
  on_failure:
    claude: "/prodigy-debug-test-failure --output ${shell.output}"
    max_attempts: 5
    fail_workflow: true
```

If tests fail, Prodigy automatically attempts to fix them up to 5 times before failing the workflow.

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
  workflow_dispatch:

jobs:
  reduce-debt:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Install Prodigy
        run: cargo install prodigy

      - name: Install dependencies
        run: |
          cargo install debtmap
          cargo install just

      - name: Run Prodigy workflow
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
        run: prodigy cook workflows/debtmap.yml -wyn 5

      - name: Create PR
        uses: peter-evans/create-pull-request@v5
        with:
          title: "chore: automated debt reduction via Prodigy"
          body: |
            Automated technical debt reduction using Prodigy workflow.

            This PR was generated by the weekly debt reduction workflow.
            Review changes carefully before merging.
          branch: prodigy-debt-reduction
```

### GitLab CI

```yaml
prodigy-debt-reduction:
  stage: quality
  rules:
    - if: '$CI_PIPELINE_SOURCE == "schedule"'
  script:
    - cargo install prodigy
    - cargo install debtmap
    - cargo install just
    - prodigy cook workflows/debtmap.yml -wyn 5
  artifacts:
    paths:
      - .prodigy/debtmap-*.json
      - .prodigy/comparison.json
```

### Important CI Considerations

- **API Keys**: Store `ANTHROPIC_API_KEY` as a secret
- **Worktrees**: The `-w` flag creates isolated worktrees automatically
- **Dependencies**: Install `prodigy`, `debtmap`, and `just` (or your build tool)
- **Timeout**: CI jobs may need extended timeout for multiple iterations
- **Review**: Always create a PR for human review before merging automated changes

## Best Practices

### 1. Start Small

Begin with low iteration counts:
```bash
# First run: 1 iteration to test workflow
prodigy cook workflows/debtmap.yml -wyn 1

# Standard run: 3-5 iterations
prodigy cook workflows/debtmap.yml -wyn 5
```

### 2. Focus on High-Priority Items

The debtmap analysis automatically prioritizes by:
- Complexity score (cyclomatic complexity)
- Coverage percentage (lower coverage = higher priority)
- Risk score (complexity × (100 - coverage%))

To focus on specific areas, create custom slash commands in `.claude/commands/` that filter by:
- Module/file patterns
- Specific debt types (Complexity, TestGap, Duplication)
- Score thresholds

### 3. Validate Thoroughly

Use comprehensive validation in your workflow:

```yaml
- shell: "just test"
  on_failure:
    claude: "/prodigy-debug-test-failure --output ${shell.output}"
    max_attempts: 5
    fail_workflow: true

- shell: "just fmt-check && just lint"
  on_failure:
    claude: "/prodigy-lint ${shell.output}"
    max_attempts: 5
    fail_workflow: true
```

### 4. Review Before Merging

Always review Prodigy's changes:
```bash
# Find your worktree
ls ~/.prodigy/worktrees/

# Check changes
cd ~/.prodigy/worktrees/session-xxx
git diff main

# Review commit history
git log --oneline

# Run full test suite
cargo test --all-features
```

### 5. Monitor Progress

Track debt reduction over iterations:
```bash
# Compare before and after
debtmap compare --before .prodigy/debtmap-before.json --after .prodigy/debtmap-after.json

# View detailed metrics
cat .prodigy/comparison.json | jq
```

## Troubleshooting

### Workflow Fails to Start

**Issue**: "Prodigy not found" or "API key missing"

**Solution**:
```bash
# Install Prodigy
cargo install prodigy

# Set API key
export ANTHROPIC_API_KEY="your-key"

# Verify installation
prodigy --version
```

### Validation Failures

**Issue**: Validation score below threshold

**Solution**: Check validation results:
```bash
# View validation details
cat .prodigy/debtmap-validation.json

# Check what gaps remain
cat .prodigy/debtmap-validation.json | jq '.gaps'

# Review comparison results
cat .prodigy/comparison.json
```

The workflow will automatically retry up to `max_attempts` times with `/prodigy-complete-debtmap-fix`.

### Test Failures

**Issue**: Tests fail after implementation

**Solution**: The workflow includes automatic test fixing:
```yaml
- shell: "just test"
  on_failure:
    claude: "/prodigy-debug-test-failure --output ${shell.output}"
    max_attempts: 5
    fail_workflow: true
```

If tests still fail after 5 attempts, review manually:
```bash
# Check test output
just test

# Review recent changes
git diff HEAD~1
```

### No Items Processed

**Issue**: Workflow completes but doesn't find debt to fix

**Possible Causes**:
1. Codebase has very low debt scores (below selection threshold)
2. Coverage data not generated properly
3. Debtmap analysis found no high-priority items

**Solution**:
```bash
# Check debtmap analysis results
cat .prodigy/debtmap-before.json | jq '.items | sort_by(-.unified_score.final_score) | .[0:5]'

# Verify coverage was generated
ls -lh target/coverage/lcov.info

# Run debtmap manually to see what's detected
debtmap analyze . --lcov target/coverage/lcov.info
```

### Workflow Hangs or Times Out

**Issue**: Workflow takes too long or appears stuck

**Possible Causes**:
- Large codebase with many files
- Complex refactoring requiring extensive analysis
- Network issues with Claude API

**Solution**:
- Reduce iteration count for testing (`-n 1`)
- Check Claude API connectivity
- Monitor worktree for progress: `cd ~/.prodigy/worktrees/session-xxx && git log`

## Example Workflows

### Full Repository Cleanup

For comprehensive debt reduction, use a higher iteration count:

```bash
# Run 10 iterations for deeper cleanup
prodigy cook workflows/debtmap.yml -wyn 10

# Run 20 iterations for major refactoring
prodigy cook workflows/debtmap.yml -wyn 20
```

The workflow automatically:
1. Selects highest priority items each iteration
2. Addresses different debt types (Complexity, TestGap, Duplication)
3. Validates all changes with tests and linting
4. Commits only successful improvements

### Custom Workflow for Specific Focus

Create a custom workflow file for focused improvements:

**`workflows/add-tests.yml`** - Focus on test coverage:
```yaml
# Generate coverage
- shell: "just coverage-lcov"

# Analyze with focus on test gaps
- shell: "debtmap analyze . --lcov target/coverage/lcov.info --output .prodigy/debtmap-before.json --format json"

# Create plan (slash command will prioritize TestGap items)
- claude: "/prodigy-debtmap-plan --before .prodigy/debtmap-before.json --output .prodigy/IMPLEMENTATION_PLAN.md"

# ... rest of standard workflow steps
```

Run with:
```bash
prodigy cook workflows/add-tests.yml -wyn 5
```

### Targeted Module Cleanup

Create a custom slash command to focus on specific modules:

**`.claude/commands/refactor-module.md`**:
```markdown
# /refactor-module

Refactor the highest complexity item in the specified module.

Arguments: --module <module_name>

... implementation details ...
```

Then create a workflow using this command for targeted refactoring.

## See Also

- [Debtmap CLI Reference](./cli-reference.md) - Debtmap command options
- [Configuration](./configuration.md) - Debtmap configuration
- [Tiered Prioritization](./tiered-prioritization.md) - Understanding priority tiers
- [Prodigy Documentation](https://github.com/iepathos/prodigy) - Full Prodigy reference
