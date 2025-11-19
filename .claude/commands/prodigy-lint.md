# PRODIGY Lint Command

You are an expert Rust developer helping with automated code formatting, linting, and testing for the prodigy project as part of the git-native improvement flow.

## Role
Format, lint, and test Rust code to ensure quality standards, then commit any automated fixes.

## Context Files (Read these to understand the project)
- `.prodigy/PROJECT.md` - Project overview and goals
- `ARCHITECTURE.md` - Technical architecture
- `Cargo.toml` - Dependencies and project config
- `src/` - Source code structure

## Phase 1: Assessment
1. Check current git status to see if there are uncommitted changes
2. Identify the project type (should be Rust based on Cargo.toml)
3. Determine available linting/formatting tools

## Phase 2: Automated Formatting
1. Run `cargo fmt` to format all Rust code
2. Check if any files were modified by formatting

## Phase 3: Linting & Analysis
1. Run `cargo clippy -- -D warnings` to catch common issues
2. If clippy reports errors:
   a. Capture the error output and identify error types
   b. Run `cargo clippy --fix --allow-dirty --allow-staged`
   c. Check git diff to see if any files were modified
   d. If NO files were modified, auto-fix cannot resolve these issues
   e. Run clippy again to get fresh error list
3. If clippy warnings remain after auto-fix, attempt manual fixes for common patterns:
   - **result_large_err**: Box large error enum variants (>128 bytes) [See manual fix strategies below]
   - **large_enum_variant**: Box the largest variant
   - **type_complexity**: Extract complex types into type aliases
   - **too_many_arguments**: Refactor into config structs
4. After manual fixes:
   a. Run `cargo check` to ensure compilation succeeds
   b. Run `cargo nextest run` to ensure tests still pass
   c. Run `cargo clippy -- -D warnings` again to verify resolution
5. If the SAME warning persists after 2 manual fix attempts, stop and report
6. Note any remaining warnings that cannot be automatically resolved

## Phase 4: Testing
1. Run `cargo nextest run` to ensure all tests pass
2. If tests fail:
   - Report which tests are failing
   - Do NOT attempt to fix test failures (that's for implement-spec)
   - Continue with the workflow

## Phase 5: Documentation Check
1. Run `cargo doc --no-deps` to check documentation builds
2. Fix any documentation warnings if possible

## Phase 6: Git Commit (Only if changes were made)
1. Check `git status` to see what files were modified by the automated tools
2. If files were modified by formatting/linting:
   - Stage all changes: `git add .`
   - Commit with message: `style: apply automated formatting and lint fixes`
3. If no changes were made, do not create an empty commit

## Phase 7: Summary Report
Provide a brief summary:
- What formatting/linting was applied
- Whether tests passed
- Whether a commit was made
- Any manual issues that need attention

## Automation Mode
When `PRODIGY_AUTOMATION=true` environment variable is set:
- Run all phases automatically
- Only output errors and the final summary
- Exit with appropriate status codes

## Example Output (Automation Mode)
```
✓ Formatting: 3 files updated
✓ Linting: 2 issues auto-fixed  
✓ Tests: All 15 tests passed
✓ Committed: style: apply automated formatting and lint fixes
```

## Error Handling
- If cargo fmt fails: Report error but continue
- If clippy fails: Report error but continue  
- If tests fail: Report but continue (don't exit)
- If git operations fail: Report error and exit

## Important Notes
- Focus on automated fixes AND common structural refactorings (like boxing variants)
- Do NOT fix logic errors or failing tests
- Do NOT modify test code unless it's formatting
- Always check git status before and after
- Only commit if actual changes were made by the tools
- **IMPORTANT**: Do NOT add any attribution text like "🤖 Generated with [Claude Code]" or "Co-Authored-By: Claude" to commit messages. Keep commits clean and focused on the change itself.

## Manual Fix Strategies for Common Clippy Warnings

### result_large_err (Error variant too large)
**Problem**: Error enum variants exceed 128 bytes, causing performance issues.

**Solution**: Box the large variant
```rust
// Before (clippy warning)
pub enum MyError {
    LargeVariant {
        field1: String,
        field2: String,
        field3: String,
        // ... many fields (>128 bytes total)
    },
}

// After (fixed)
pub enum MyError {
    LargeVariant(Box<LargeVariantDetails>),
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("...")]
pub struct LargeVariantDetails {
    pub field1: String,
    pub field2: String,
    pub field3: String,
    // ... many fields
}

// Update construction sites:
// Old: MyError::LargeVariant { field1, field2, field3 }
// New: MyError::LargeVariant(Box::new(LargeVariantDetails { field1, field2, field3 }))

// Update match sites:
// Old: MyError::LargeVariant { field1, .. } => ...
// New: MyError::LargeVariant(details) => ... // access via details.field1
```

**Steps**:
1. Identify the large variant from clippy output
2. Create a new struct with the variant's fields
3. Replace the variant with a boxed struct
4. Update all construction sites (find with `grep -r "VariantName {"`)
5. Update all pattern matching sites (find with `grep -r "VariantName {"`)
6. Run clippy again to verify the fix

**Real Example** (from prodigy codebase):
```rust
// Before - clippy error: result_large_err
pub enum ExecutionError {
    CommitValidationFailed {
        agent_id: String,
        item_id: String,
        step_index: usize,
        command: String,
        base_commit: String,
        worktree_path: String,  // Large variant: 128+ bytes
    },
}

// After - fixed
pub enum ExecutionError {
    CommitValidationFailed(Box<CommitValidationError>),
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("Command '{command}' (step {step_index}) did not create required commits")]
pub struct CommitValidationError {
    pub agent_id: String,
    pub item_id: String,
    pub step_index: usize,
    pub command: String,
    pub base_commit: String,
    pub worktree_path: String,
}

// Update construction sites:
// Old:
return Err(ExecutionError::CommitValidationFailed {
    agent_id: handle.config.id.clone(),
    item_id: handle.config.item_id.clone(),
    // ... more fields
});

// New:
return Err(ExecutionError::CommitValidationFailed(Box::new(
    CommitValidationError {
        agent_id: handle.config.id.clone(),
        item_id: handle.config.item_id.clone(),
        // ... more fields
    }
)));
```

### large_enum_variant (Variant size mismatch)
**Problem**: One variant is significantly larger than others.

**Solution**: Box the large variant
```rust
// Before
pub enum Message {
    Small(u32),
    Large { huge_data: Vec<u8> },  // Much larger than Small
}

// After
pub enum Message {
    Small(u32),
    Large(Box<LargeData>),
}

pub struct LargeData {
    pub huge_data: Vec<u8>,
}
```

### type_complexity (Type too complex)
**Problem**: Type signature is too complex (nested generics, long types).

**Solution**: Extract into type alias
```rust
// Before
fn process(data: HashMap<String, Vec<Result<Option<Data>, Error>>>) -> Result<(), Error> { ... }

// After
type ProcessingMap = HashMap<String, Vec<Result<Option<Data>, Error>>>;
fn process(data: ProcessingMap) -> Result<(), Error> { ... }
```

### too_many_arguments (Function has too many arguments)
**Problem**: Function has more than 7 parameters.

**Solution**: Refactor into a config struct
```rust
// Before
fn create_agent(id: String, name: String, config: Config, timeout: u64, retries: u32) { ... }

// After
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub config: Config,
    pub timeout: u64,
    pub retries: u32,
}

fn create_agent(config: AgentConfig) { ... }
```

## Workflow for Manual Fixes
1. **Identify the pattern**: Read clippy error message carefully
2. **Find all occurrences**: Use `grep` to find all usage sites
3. **Apply the fix**: Refactor using patterns above
4. **Verify compilation**: Run `cargo check` after each change
5. **Run tests**: Ensure no behavioral changes with `cargo nextest run`
6. **Verify clippy**: Run `cargo clippy -- -D warnings` again
7. **Commit changes**: Stage and commit with descriptive message

## When to Stop
- **Stop attempting fixes** if:
  - The same clippy warning persists after 2 fix attempts
  - Tests start failing after refactoring
  - The warning requires deep architectural changes
  - You're unsure about the correct fix
- **Report the issue** and exit with error status to trigger manual review

Your goal is to ensure code quality through automated tools AND smart structural refactoring while preserving the intent and logic of the code.
