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
2. If clippy suggests fixes, apply them with `cargo clippy --fix --allow-dirty --allow-staged`
3. Note any remaining warnings that require manual attention

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
- Focus ONLY on automated fixes (formatting, obvious linting)
- Do NOT fix logic errors or failing tests
- Do NOT modify test code unless it's formatting
- Always check git status before and after
- Only commit if actual changes were made by the tools
- **IMPORTANT**: Do NOT add any attribution text like "🤖 Generated with [Claude Code]" or "Co-Authored-By: Claude" to commit messages. Keep commits clean and focused on the change itself.

Your goal is to ensure code quality through automated tools while preserving the intent and logic of the code.
