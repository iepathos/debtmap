# PRODIGY Lint Command

You are an expert Rust developer helping with automated code formatting and linting for the prodigy project as part of the git-native improvement flow.

## Role
Format and lint Rust code to ensure quality standards, then commit any automated fixes.

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

## Phase 3: Linting & Analysis (CRITICAL - Follow Flow Exactly)

### Step 3.1: Initial Clippy Check
1. Run `cargo clippy -- -D warnings` to catch common issues
2. Capture the full error output if errors are found
3. **If no errors**: Skip to Phase 4 (Documentation Check)
4. **If errors found**: Proceed to Step 3.2

### Step 3.2: Attempt Auto-Fix
1. Run `cargo clippy --fix --allow-dirty --allow-staged`
2. Run `git status` to check if files were modified
3. **MANDATORY**: Proceed to Step 3.3 (Verification)

### Step 3.3: Verify Auto-Fix Results
1. Run `cargo clippy -- -D warnings` again to get current error state
2. **Decision Point**:
   - **If no errors**: Auto-fix succeeded → Skip to Phase 4 (Documentation Check)
   - **If errors persist**: Auto-fix failed or incomplete → **PROCEED TO Step 3.4 (Manual Fixes)**

### Step 3.4: Manual Fix Loop (REQUIRED when auto-fix doesn't resolve all errors)

**IMPORTANT**: You MUST attempt manual fixes for any remaining clippy errors. Do not skip this step.

For each remaining clippy error, identify the error type and apply the appropriate manual fix:

**Supported Error Types:**
- **too_many_arguments** (>7 parameters) → Create parameter struct [See line 272 for detailed example]
- **result_large_err** (>128 bytes) → Box the large variant [See line 98]
- **large_enum_variant** → Box the large variant [See line 282]
- **type_complexity** → Extract type alias [See line 306]
- **unused_variables** (test code only) → Prefix with underscore

**Manual Fix Process:**

```
For each clippy error:
a. Parse error to identify: error type, file, line number, affected function/type
b. Read the affected file to understand context
c. Apply appropriate fix strategy (see detailed examples in sections below)
d. Run `cargo check` to verify compilation
e. Run `cargo clippy -- -D warnings` to verify error resolved
f. Track attempts:
   - If resolved: Continue to next error
   - If persists: Increment counter for THIS specific error
   - If 2 attempts on SAME error: STOP and report (go to Step 3.5)
g. After fixing all errors OR hitting unfixable error: Proceed to Step 3.5
```

### Step 3.5: Verify All Fixes
1. Run `cargo check` to ensure project compiles
2. Run `cargo clippy -- -D warnings` one final time
3. **Decision Point**:
   - **If clippy clean**: Success → Proceed to Step 3.6
   - **If same errors persist after 2 attempts**: Report unfixable errors and STOP
   - **If compilation fails**: Report errors and STOP

### Step 3.6: Commit Lint Fixes
1. Run `git status` to see what was modified
2. **If changes exist**:
   - Run `git add .`
   - Run `git commit -m "style: apply clippy fixes"`
   - Do NOT add attribution text
3. **If no changes**: Proceed to Phase 4

## Phase 4: Documentation Check
1. Run `cargo doc --no-deps` to check documentation builds
2. Fix any documentation warnings if possible

## Phase 5: Format Commit (Only if changes were made in Phase 2)
1. Check `git status` to see what files were modified by formatting
2. If files were modified by formatting only (not already committed in 3.6):
   - Stage all changes: `git add .`
   - Commit with message: `style: apply automated formatting`
3. If no changes were made, do not create an empty commit

## Phase 6: Summary Report
Provide a brief summary:
- What formatting/linting was applied
- Whether clippy is clean
- Whether commits were made
- Any manual issues that need attention (unfixable errors)

## Automation Mode
When `PRODIGY_AUTOMATION=true` environment variable is set:
- Run all phases automatically
- Only output errors and the final summary
- Exit with appropriate status codes

## Example Output (Automation Mode)
```
✓ Formatting: No changes needed
✓ Linting: Fixed 1 clippy error (too_many_arguments)
✓ Clippy: Clean (no warnings)
✓ Committed: style: apply clippy fixes

Manual attention required:
- None
```

## Error Handling
- If cargo fmt fails: Report error but continue
- If clippy fails after 2 fix attempts: Report unfixable errors and stop
- If compilation fails: Report errors and stop
- If git operations fail: Report error and exit

## Important Notes
- **CRITICAL**: You MUST attempt manual fixes when auto-fix doesn't resolve all clippy errors. This is not optional.
- Focus on automated fixes AND common structural refactorings (boxing variants, parameter structs)
- Do NOT fix logic errors or failing tests
- Do NOT modify test code unless it's formatting or unused variable warnings
- Always check git status before and after changes
- Only commit if actual changes were made by the tools
- **CRITICAL**: Do NOT add attribution text like "🤖 Generated with [Claude Code]" or "Co-Authored-By: Claude" to commit messages

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

**Solution**: Refactor parameters into a config struct

**Detailed Example** (with lifetimes):
```rust
// Before (9 parameters - clippy error)
fn analyze_domains_and_recommend_splits(
    per_struct_metrics: &[StructMetrics],
    total_methods: usize,
    lines_of_code: usize,
    is_god_object: bool,
    path: &Path,
    all_methods: &[String],
    field_tracker: Option<&FieldAccessTracker>,
    responsibility_groups: &HashMap<String, Vec<String>>,
    ast: &syn::File,
) -> (Vec<ModuleSplit>, SplitAnalysisMethod, Option<RecommendationSeverity>, usize, f64, f64) {
    // Original function body uses parameters directly
    let struct_count = per_struct_metrics.len();
    let domain_count = count_distinct_domains(per_struct_metrics);
    // ... more code using all the parameters
}

// After (fixed with parameter struct)
struct DomainAnalysisParams<'a> {
    per_struct_metrics: &'a [StructMetrics],
    total_methods: usize,
    lines_of_code: usize,
    is_god_object: bool,
    path: &'a Path,
    all_methods: &'a [String],
    field_tracker: Option<&'a FieldAccessTracker>,
    responsibility_groups: &'a HashMap<String, Vec<String>>,
    ast: &'a syn::File,
}

fn analyze_domains_and_recommend_splits(
    params: DomainAnalysisParams,
) -> (Vec<ModuleSplit>, SplitAnalysisMethod, Option<RecommendationSeverity>, usize, f64, f64) {
    // Destructure params at the start to preserve original function body
    let per_struct_metrics = params.per_struct_metrics;
    let total_methods = params.total_methods;
    let lines_of_code = params.lines_of_code;
    let is_god_object = params.is_god_object;
    let path = params.path;
    let all_methods = params.all_methods;
    let field_tracker = params.field_tracker;
    let responsibility_groups = params.responsibility_groups;
    let ast = params.ast;

    // Original function body (unchanged)
    let struct_count = per_struct_metrics.len();
    let domain_count = count_distinct_domains(per_struct_metrics);
    // ... rest of original code
}

// Update call sites:
// Old:
let result = analyze_domains_and_recommend_splits(
    &per_struct_metrics,
    total_methods,
    lines_of_code,
    is_god_object,
    path,
    &all_methods,
    field_tracker.as_ref(),
    &responsibility_groups,
    ast,
);

// New:
let result = analyze_domains_and_recommend_splits(DomainAnalysisParams {
    per_struct_metrics: &per_struct_metrics,
    total_methods,
    lines_of_code,
    is_god_object,
    path,
    all_methods: &all_methods,
    field_tracker: field_tracker.as_ref(),
    responsibility_groups: &responsibility_groups,
    ast,
});
```

**Step-by-Step Fix Process:**
1. Parse clippy output to identify function name, file, line number
2. Read the file and locate the function definition
3. Count parameters to confirm (>7 means clippy error)
4. Create a new struct above the function:
   - Struct name: `{FunctionName}Params` (e.g., `DomainAnalysisParams`)
   - Add lifetime `<'a>` if ANY parameters have references
   - One field per parameter with same name and type
5. Replace function signature: `fn func_name(params: StructName<'a>)`
6. At start of function body, destructure params:
   ```rust
   let param1 = params.param1;
   let param2 = params.param2;
   // ... for each parameter
   ```
   This preserves the original function body unchanged
7. Find ALL call sites: `grep -rn "function_name(" --include="*.rs"`
8. For each call site, replace:
   ```rust
   // Old: function_name(arg1, arg2, arg3, ...)
   // New: function_name(StructName { param1: arg1, param2: arg2, ... })
   ```
9. Run `cargo check` to verify compilation
10. Run `cargo clippy -- -D warnings` to verify error resolved
11. If errors remain, check for:
    - Missed call sites (use grep again)
    - Incorrect field names in struct construction
    - Missing or incorrect lifetime parameters

**Common Pitfalls:**
- Forgetting lifetime parameter `<'a>` when references are present
- Missing call sites (always use grep to find ALL occurrences)
- Not destructuring params in function body (breaks all variable usage)
- Mismatched field names between struct definition and usage

## Workflow for Manual Fixes
1. **Identify the pattern**: Read clippy error message carefully
2. **Find all occurrences**: Use `grep` to find all usage sites
3. **Apply the fix**: Refactor using patterns above
4. **Verify compilation**: Run `cargo check` after each change
5. **Verify clippy**: Run `cargo clippy -- -D warnings` again
6. **Commit changes**: Stage and commit with descriptive message

## When to Stop
- **Stop attempting fixes** if:
  - The same clippy warning persists after 2 fix attempts
  - Compilation fails after refactoring
  - The warning requires deep architectural changes
  - You're unsure about the correct fix
- **Report the issue** and exit with error status to trigger manual review

Your goal is to ensure code quality through automated tools AND smart structural refactoring while preserving the intent and logic of the code.
