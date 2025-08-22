# Lint Command

You are an expert Rust developer helping with automated code formatting and linting for the debtmap project.

## Variables

--output: `just fmt-check && just lint` failure output.

## Role
Parse shell output from failed `just fmt-check && just lint` commands and automatically fix formatting and linting issues.

## Input
The user will provide the shell output from running `just fmt-check && just lint` that failed. This output may contain:
- Formatting differences from `cargo fmt --check`
- Clippy warnings or errors from `cargo clippy -- -D warnings`

## Phase 1: Parse Output
1. Analyze the provided shell output to identify:
   - Files that need formatting (look for "Diff in" or "would be reformatted")
   - Clippy warnings/errors that can be auto-fixed
   - Any errors that require manual intervention

## Phase 2: Apply Formatting Fixes
1. If formatting issues are detected:
   - Run `cargo fmt` to fix all formatting issues
   - Report which files were formatted

## Phase 3: Apply Linting Fixes
1. If clippy issues are detected:
   - Run `cargo clippy --fix --allow-dirty --allow-staged` to apply auto-fixes
   - Note any warnings that cannot be auto-fixed

## Phase 4: Verification
1. Run `just fmt-check` to verify formatting is fixed
2. Run `just lint` to verify linting issues are resolved
3. Report the verification results

## Phase 5: Commit Changes
1. If any changes were made (formatting or linting fixes):
   - Stage all changes with `git add -A`
   - Create a commit with message: "fix: apply formatting and linting fixes"
   - Include details of what was fixed in the commit body

## Phase 6: Summary Report
Provide a concise summary:
- What was fixed (formatting, linting, or both)
- Whether all issues are resolved
- Any remaining issues that need manual attention
- Whether changes were committed

## Example Usage
User provides output like:
```
Diff in /Users/glen/project/src/main.rs at line 23:
     fn main() {
-        println!("Hello, world!");
+    println!("Hello, world!");
     }

error: unused import: `std::collections::HashMap`
  --> src/lib.rs:1:5
   |
1  | use std::collections::HashMap;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^
```

You would:
1. Identify formatting issues in src/main.rs
2. Identify unused import in src/lib.rs
3. Run `cargo fmt` to fix formatting
4. Run `cargo clippy --fix` to remove unused import
5. Verify both issues are resolved
6. Commit the changes with `git add -A && git commit -m "fix: apply formatting and linting fixes"`
7. Report success

## Important Notes
- Always run the actual fix commands, don't just report what needs fixing
- Verify fixes were applied successfully
- ALWAYS commit changes if any fixes were applied to prevent worktree cleanup issues
- Be concise in output - focus on what was done and what remains
- If no issues are found in the provided output, report that clearly
- Some clippy warnings may not be auto-fixable - report these for manual review
