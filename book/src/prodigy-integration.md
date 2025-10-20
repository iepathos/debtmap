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
# Install Prodigy from GitHub repository
cargo install --git https://github.com/iepathos/prodigy prodigy

# Or if available on crates.io:
cargo install prodigy

# Verify installation
prodigy --version
```

**Requirements:**
- Rust 1.70 or later
- Git (for worktree management)
- Anthropic API key for Claude access

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
# Run with auto-confirm, 5 iterations
prodigy run workflows/debtmap.yml -yn 5

# Run with custom iteration count
prodigy run workflows/debtmap.yml -yn 10

# Run single iteration for testing
prodigy run workflows/debtmap.yml -yn 1
```

**Command Flags:**
- `-y` (`--yes`) - Auto-confirm workflow steps (skip prompts)
- `-n 5` (`--max-iterations 5`) - Run workflow for up to 5 iterations

**Note**: Worktrees are managed separately via the `prodigy worktree` command. In MapReduce mode, Prodigy automatically creates isolated worktrees for each parallel agent.

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

## Useful Prodigy Commands

Beyond `prodigy run`, several commands help manage workflows and sessions:

### Resume Interrupted Workflows

```bash
# Resume an interrupted sequential workflow
prodigy resume <SESSION_ID>

# Resume an interrupted MapReduce job
prodigy resume-job <JOB_ID>

# List all sessions to find the SESSION_ID
prodigy sessions
```

**When to use**: If a workflow is interrupted (Ctrl-C, system crash, network issues), you can resume from the last checkpoint rather than starting over.

### View Checkpoints

```bash
# List all available checkpoints
prodigy checkpoints

# List checkpoints for specific session
prodigy checkpoints --session <SESSION_ID>
```

**When to use**: To see available restore points for interrupted workflows.

### Manage Worktrees

```bash
# List all Prodigy worktrees
prodigy worktree list

# Clean up old worktrees
prodigy worktree clean

# Remove specific worktree
prodigy worktree remove <SESSION_ID>
```

**When to use**: MapReduce workflows create many worktrees. Clean them up periodically to save disk space.

### Monitor MapReduce Progress

```bash
# View progress of running MapReduce job
prodigy progress <JOB_ID>

# View events and logs from MapReduce job
prodigy events <JOB_ID>

# Filter events by type
prodigy events <JOB_ID> --type agent_started
prodigy events <JOB_ID> --type agent_completed
prodigy events <JOB_ID> --type agent_failed
```

**When to use**: Monitor long-running MapReduce jobs to see how many agents have completed, which are still running, and which have failed.

### Manage Dead Letter Queue

```bash
# View failed MapReduce items in DLQ
prodigy dlq list <JOB_ID>

# Retry failed items from DLQ
prodigy dlq retry <JOB_ID> <ITEM_ID>

# Remove items from DLQ
prodigy dlq remove <JOB_ID> <ITEM_ID>
```

**When to use**: When some MapReduce agents fail, their items go to the Dead Letter Queue. You can retry them individually or investigate why they failed.

### Session Management

```bash
# List all workflow sessions
prodigy sessions

# Clean up old sessions
prodigy clean
```

**When to use**: View history of workflow runs and clean up old data.

## Workflow Configuration

Prodigy workflows are defined as YAML lists of steps. Each step can be either a `shell` command or a `claude` slash command.

### Workflow Step Types

#### Shell Commands

Execute shell commands directly:

```yaml
# Simple shell command
- shell: "cargo test"

# With timeout (in seconds)
- shell: "just coverage-lcov"
  timeout: 900  # 15 minutes

# With error handling
- shell: "just test"
  on_failure:
    claude: "/prodigy-debug-test-failure --output ${shell.output}"
    max_attempts: 5
    fail_workflow: true
```

**Shell Command Fields:**
- `shell`: Command to execute (string)
- `timeout`: Maximum execution time in seconds (optional)
- `on_failure`: Error handler configuration (optional)
  - `claude`: Slash command to run on failure
  - `max_attempts`: Maximum retry attempts
  - `fail_workflow`: If true, fail entire workflow after max attempts

#### Claude Commands

Execute Claude Code slash commands:

```yaml
# Simple Claude command
- claude: "/prodigy-debtmap-plan --before .prodigy/debtmap-before.json --output .prodigy/IMPLEMENTATION_PLAN.md"

# With output capture (makes command output available in ${shell.output})
- claude: "/prodigy-debtmap-plan --before .prodigy/debtmap-before.json --output .prodigy/IMPLEMENTATION_PLAN.md"
  capture_output: true

# With commit requirement (workflow fails if no git commit made)
- claude: "/prodigy-debtmap-implement --plan .prodigy/IMPLEMENTATION_PLAN.md"
  commit_required: true

# With timeout and validation
- claude: "/prodigy-debtmap-implement --plan .prodigy/IMPLEMENTATION_PLAN.md"
  commit_required: true
  timeout: 1800  # 30 minutes
  validate:
    commands:
      - shell: "cargo test"
    result_file: ".prodigy/validation.json"
    threshold: 75
```

**Claude Command Fields:**
- `claude`: Slash command to execute (string)
- `capture_output`: If true, command output is available in `${shell.output}` variable (optional)
- `commit_required`: If true, workflow fails if command doesn't create a git commit (optional)
- `timeout`: Maximum execution time in seconds (optional)
- `validate`: Validation configuration (optional, see Step-Level Validation below)

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

## MapReduce Workflows

Prodigy supports MapReduce workflows for processing multiple items in parallel. This is powerful for large-scale refactoring where you want to fix many debt items simultaneously.

### When to Use MapReduce

- Processing multiple independent debt items simultaneously (e.g., refactor 10 high-complexity functions in parallel)
- Applying the same fix pattern across many files
- Large-scale codebase cleanup tasks
- Situations where sequential iteration would be too slow

### MapReduce vs Sequential Workflows

**Sequential Workflow** (`-n 5`):
- Runs entire workflow N times in sequence
- Fixes one item per iteration
- Each iteration re-analyzes the codebase
- Total time: N × workflow_duration

**MapReduce Workflow**:
- Processes multiple items in parallel in a single run
- Setup phase runs once
- Map phase spawns N parallel agents (each in isolated worktree)
- Reduce phase aggregates results
- Total time: setup + max(map_agent_durations) + reduce

### Complete MapReduce Example

Create `workflows/debtmap-reduce.yml`:

```yaml
name: debtmap-parallel-elimination
mode: mapreduce

# Setup phase: Analyze the codebase and generate debt items
setup:
  timeout: 900  # 15 minutes for coverage generation
  commands:
    # Generate coverage data with tarpaulin
    - shell: "just coverage-lcov"

    # Run debtmap with coverage data to establish baseline
    - shell: "debtmap analyze src --lcov target/coverage/lcov.info --output .prodigy/debtmap-before.json --format json"

# Map phase: Process each debt item in parallel with planning and validation
map:
  # Input configuration - debtmap-before.json contains items array
  input: .prodigy/debtmap-before.json
  json_path: "$.items[*]"

  # Commands to execute for each debt item
  agent_template:
    # Phase 1: Create implementation plan
    - claude: "/prodigy-debtmap-plan --item '${item}' --output .prodigy/plan-${item_id}.md"
      capture_output: true
      validate:
        commands:
          - claude: "/prodigy-validate-debtmap-plan --item '${item}' --plan .prodigy/plan-${item_id}.md --output .prodigy/validation-${item_id}.json"
        result_file: ".prodigy/validation-${item_id}.json"
        threshold: 75
        on_incomplete:
          commands:
            - claude: "/prodigy-revise-debtmap-plan --gaps ${validation.gaps} --plan .prodigy/plan-${item_id}.md"
          max_attempts: 3
          fail_workflow: false

    # Phase 2: Execute the plan
    - claude: "/prodigy-debtmap-implement --plan .prodigy/plan-${item_id}.md"
      commit_required: true
      validate:
        commands:
          - shell: "just coverage-lcov"
          - shell: "debtmap analyze src --lcov target/coverage/lcov.info --output .prodigy/debtmap-after-${item_id}.json --format json"
          - shell: "debtmap compare --before .prodigy/debtmap-before.json --after .prodigy/debtmap-after-${item_id}.json --plan .prodigy/plan-${item_id}.md --output .prodigy/comparison-${item_id}.json --format json"
          - claude: "/prodigy-validate-debtmap-improvement --comparison .prodigy/comparison-${item_id}.json --output .prodigy/debtmap-validation-${item_id}.json"
        result_file: ".prodigy/debtmap-validation-${item_id}.json"
        threshold: 75
        on_incomplete:
          commands:
            - claude: "/prodigy-complete-debtmap-fix --plan .prodigy/plan-${item_id}.md --validation .prodigy/debtmap-validation-${item_id}.json --attempt ${validation.attempt_number}"
              commit_required: true
            - shell: "just coverage-lcov"
            - shell: "debtmap analyze src --lcov target/coverage/lcov.info --output .prodigy/debtmap-after-${item_id}.json --format json"
            - shell: "debtmap compare --before .prodigy/debtmap-before.json --after .prodigy/debtmap-after-${item_id}.json --plan .prodigy/plan-${item_id}.md --output .prodigy/comparison-${item_id}.json --format json"
          max_attempts: 5
          fail_workflow: true

    # Phase 3: Verify tests pass
    - shell: "just test"
      on_failure:
        claude: "/prodigy-debug-test-failure --output ${shell.output}"
        max_attempts: 5
        fail_workflow: true

    # Phase 4: Check formatting and linting
    - shell: "just fmt-check && just lint"
      on_failure:
        claude: "/prodigy-lint ${shell.output}"
        max_attempts: 5
        fail_workflow: true

  # Parallelization settings
  max_parallel: 5  # Run up to 5 agents in parallel

  # Filter and sort items
  filter: "File.score >= 10 OR Function.unified_score.final_score >= 10"
  sort_by: "File.score DESC, Function.unified_score.final_score DESC"
  max_items: 10  # Limit to 10 items per run

# Reduce phase: Aggregate results and verify overall improvements
reduce:
  # Phase 1: Run final tests across all changes
  - shell: "just test"
    on_failure:
      claude: "/prodigy-debug-test-failure --output ${shell.output}"
      max_attempts: 5
      fail_workflow: true

  # Phase 2: Check formatting and linting
  - shell: "just fmt-check && just lint"
    on_failure:
      claude: "/prodigy-lint ${shell.output}"
      max_attempts: 5
      fail_workflow: true

  # Phase 3: Re-run debtmap to measure cumulative improvements
  - shell: "just coverage-lcov"
  - shell: "debtmap analyze src --lcov target/coverage/lcov.info --output .prodigy/debtmap-after.json --format json"

  # Phase 4: Create final commit with summary
  - write_file:
      path: ".prodigy/map-results.json"
      content: "${map.results}"
      format: json
      create_dirs: true

  - claude: |
      /prodigy-compare-debt-results \
        --before .prodigy/debtmap-before.json \
        --after .prodigy/debtmap-after.json \
        --map-results-file .prodigy/map-results.json \
        --successful ${map.successful} \
        --failed ${map.failed} \
        --total ${map.total}
    commit_required: true
```

### Running MapReduce Workflows

```bash
# Run MapReduce workflow (single execution processes multiple items in parallel)
prodigy run workflows/debtmap-reduce.yml

# Run with auto-confirm
prodigy run workflows/debtmap-reduce.yml -y
```

**Note**: MapReduce workflows don't typically use `-n` for iterations. Instead, they process multiple items in a single run through parallel map agents.

### MapReduce Configuration Options

#### Top-Level Fields

- `name`: Workflow name (string)
- `mode: mapreduce`: Enables MapReduce mode (required)
- `setup`: Commands to run once before map phase
- `map`: Map phase configuration
- `reduce`: Commands to run after all map agents complete

#### Setup Phase Fields

- `timeout`: Maximum time in seconds for setup phase
- `commands`: List of shell or claude commands to run

#### Map Phase Fields

- `input`: Path to JSON file containing items to process
- `json_path`: JSONPath expression to extract items array (e.g., `$.items[*]`)
- `agent_template`: List of commands to run for each item (each item gets its own agent in an isolated worktree)
- `max_parallel`: Maximum number of agents to run concurrently
- `filter`: Expression to filter which items to process (e.g., `"score >= 10"`)
- `sort_by`: Expression to sort items (e.g., `"score DESC"`)
- `max_items`: Limit total items processed

#### MapReduce-Specific Variables

Available in `agent_template` commands:

- `${item}`: The full JSON object for current item
- `${item_id}`: Unique ID for current item (auto-generated)
- `${validation.gaps}`: List of validation gaps from failed validation
- `${validation.attempt_number}`: Current retry attempt number (1, 2, 3, etc.)
- `${shell.output}`: Output from previous shell command
- `${map.results}`: All map agent results (available in reduce phase)
- `${map.successful}`: Count of successful map agents (reduce phase)
- `${map.failed}`: Count of failed map agents (reduce phase)
- `${map.total}`: Total number of map agents (reduce phase)

### MapReduce Architecture

```
┌─────────────────────────────────────────────────────────┐
│ Setup Phase (main worktree)                            │
│ - Generate coverage data                               │
│ - Run debtmap analysis                                 │
│ - Output: .prodigy/debtmap-before.json                 │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│ Map Phase (parallel worktrees)                         │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │ Agent 1      │  │ Agent 2      │  │ Agent 3      │ │
│  │ Item #1      │  │ Item #2      │  │ Item #3      │ │
│  │ Worktree A   │  │ Worktree B   │  │ Worktree C   │ │
│  │              │  │              │  │              │ │
│  │ Plan → Fix   │  │ Plan → Fix   │  │ Plan → Fix   │ │
│  │ → Validate   │  │ → Validate   │  │ → Validate   │ │
│  │ → Test       │  │ → Test       │  │ → Test       │ │
│  │ → Commit     │  │ → Commit     │  │ → Commit     │ │
│  └──────────────┘  └──────────────┘  └──────────────┘ │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐                   │
│  │ Agent 4      │  │ Agent 5      │                   │
│  │ Item #4      │  │ Item #5      │                   │
│  │ Worktree D   │  │ Worktree E   │                   │
│  │              │  │              │                   │
│  │ Plan → Fix   │  │ Plan → Fix   │                   │
│  │ → Validate   │  │ → Validate   │                   │
│  │ → Test       │  │ → Test       │                   │
│  │ → Commit     │  │ → Commit     │                   │
│  └──────────────┘  └──────────────┘                   │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│ Reduce Phase (main worktree)                           │
│ - Merge all agent worktrees                            │
│ - Run final tests on merged code                       │
│ - Run final linting                                    │
│ - Re-analyze with debtmap                              │
│ - Generate summary commit                              │
└─────────────────────────────────────────────────────────┘
```

**Key Concepts:**
- **Isolation**: Each map agent works in its own git worktree
- **Parallelism**: Multiple agents process different items simultaneously
- **Validation**: Each agent validates its changes independently
- **Merging**: Reduce phase merges all successful agent worktrees
- **Final Validation**: Reduce phase ensures merged code passes all tests

## Iteration Strategy

### How Iterations Work

When you run `prodigy run workflows/debtmap.yml -yn 5`, the workflow executes up to 5 times:

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
prodigy run workflows/debtmap.yml -yn 1

# Standard run (5 iterations)
prodigy run workflows/debtmap.yml -yn 5

# Deep cleanup (10+ iterations)
prodigy run workflows/debtmap.yml -yn 20
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
        run: prodigy run workflows/debtmap.yml -yn 5

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
    - prodigy run workflows/debtmap.yml -yn 5
  artifacts:
    paths:
      - .prodigy/debtmap-*.json
      - .prodigy/comparison.json
```

### Important CI Considerations

- **API Keys**: Store `ANTHROPIC_API_KEY` as a secret
- **Worktrees**: MapReduce mode creates isolated worktrees automatically for parallel processing
- **Dependencies**: Install `prodigy`, `debtmap`, and `just` (or your build tool)
- **Timeout**: CI jobs may need extended timeout for multiple iterations
- **Review**: Always create a PR for human review before merging automated changes

## Best Practices

### 1. Start Small

Begin with low iteration counts:
```bash
# First run: 1 iteration to test workflow
prodigy run workflows/debtmap.yml -yn 1

# Standard run: 3-5 iterations
prodigy run workflows/debtmap.yml -yn 5
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

### MapReduce-Specific Troubleshooting

#### Resuming Failed MapReduce Jobs

**Issue**: MapReduce job was interrupted or failed

**Solution**:
```bash
# Find the job ID from recent sessions
prodigy sessions

# Resume the MapReduce job from checkpoint
prodigy resume-job <JOB_ID>
```

The job will resume from where it left off, skipping already-completed items.

#### Checking MapReduce Progress

**Issue**: Want to monitor long-running MapReduce job

**Solution**:
```bash
# View overall progress
prodigy progress <JOB_ID>

# View detailed events
prodigy events <JOB_ID>

# Filter for specific event types
prodigy events <JOB_ID> --type agent_completed
prodigy events <JOB_ID> --type agent_failed
```

**Output example**:
```
MapReduce Job: job-abc123
Status: running
Progress: 7/10 items (70%)
- Completed: 5
- Running: 2
- Failed: 3
```

#### Managing Failed MapReduce Items

**Issue**: Some agents failed, items in Dead Letter Queue

**Solution**:
```bash
# View failed items
prodigy dlq list <JOB_ID>

# Review why an item failed (check events)
prodigy events <JOB_ID> --item <ITEM_ID>

# Retry specific failed item
prodigy dlq retry <JOB_ID> <ITEM_ID>

# Remove unfixable items from DLQ
prodigy dlq remove <JOB_ID> <ITEM_ID>
```

**Common failure reasons**:
- Validation threshold not met after max_attempts
- Tests fail and can't be fixed automatically
- Merge conflicts with other agents' changes
- Timeout exceeded for complex refactoring

#### Cleaning Up MapReduce Worktrees

**Issue**: Disk space consumed by many MapReduce worktrees

**Solution**:
```bash
# List all worktrees
prodigy worktree list

# Clean up completed job worktrees
prodigy worktree clean

# Remove specific session's worktrees
prodigy worktree remove <SESSION_ID>

# Manual cleanup (if Prodigy commands don't work)
rm -rf ~/.prodigy/worktrees/session-xxx
```

**When to clean**:
- After successful job completion and merge
- When disk space is low
- After abandoned or failed jobs

#### MapReduce Merge Conflicts

**Issue**: Reduce phase fails due to merge conflicts between agent worktrees

**Possible Causes**:
- Multiple agents modified overlapping code
- Agents made conflicting architectural changes
- Shared dependencies updated differently

**Solution**:
```bash
# Review which agents succeeded
prodigy events <JOB_ID> --type agent_completed

# Check merge conflicts
cd ~/.prodigy/worktrees/session-xxx
git status

# Manually resolve conflicts
# Edit conflicting files
git add .
git commit -m "Resolve MapReduce merge conflicts"

# Resume the job
prodigy resume-job <JOB_ID>
```

**Prevention**:
- Use `filter` to ensure agents work on independent items
- Reduce `max_parallel` to minimize conflicts
- Design debt items to be truly independent

#### Understanding MapReduce Variables

If you're debugging workflow files, these variables are available:

**In map phase (agent_template)**:
- `${item}`: Full JSON of current item being processed
- `${item_id}`: Unique ID for current item
- `${validation.gaps}`: Validation gaps from validation result
- `${validation.attempt_number}`: Current retry attempt (1, 2, 3...)
- `${shell.output}`: Output from previous shell command

**In reduce phase**:
- `${map.results}`: All map agent results as JSON
- `${map.successful}`: Count of successful agents
- `${map.failed}`: Count of failed agents
- `${map.total}`: Total number of agents

**Example debug command**:
```yaml
# In agent_template, log the item being processed
- shell: "echo 'Processing item: ${item_id}' >> .prodigy/debug.log"
```

## Example Workflows

### Full Repository Cleanup

For comprehensive debt reduction, use a higher iteration count:

```bash
# Run 10 iterations for deeper cleanup
prodigy run workflows/debtmap.yml -yn 10

# Run 20 iterations for major refactoring
prodigy run workflows/debtmap.yml -yn 20
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
prodigy run workflows/add-tests.yml -yn 5
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
