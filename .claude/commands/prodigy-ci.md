---
name: prodigy-ci
description: Run CI checks and automatically fix any issues until all checks pass
---

Run `just ci` and automatically fix any issues encountered until all CI checks pass successfully.

This command will:
1. Run the full CI pipeline
2. Identify and fix any compilation errors
3. Fix any failing tests by addressing root causes
4. Fix any linting issues
5. Fix any formatting issues
6. Continue iterating until all checks pass
7. Create a commit for any fixes made (without Claude attribution)

The CI pipeline includes:
- Running all tests
- Checking code formatting
- Running clippy linter
- Building in release mode
- Checking documentation

## Commit Behavior

When fixes are made during CI:
1. Track all files that were modified to fix CI issues
2. Analyze the types of fixes made (compilation, tests, linting, formatting)
3. Generate an appropriate commit message based on fixes:
   - `fix: resolve compilation errors` for build fixes
   - `fix: correct failing tests` for test fixes
   - `style: apply linting and formatting fixes` for style issues
   - `fix: resolve CI pipeline issues` for mixed fixes
4. Stage and commit changes automatically
5. **IMPORTANT**: Do NOT add any attribution text like "🤖 Generated with [Claude Code]" or "Co-Authored-By: Claude" to commit messages

## Example Commit Messages

- `fix: resolve type errors in rust_call_graph module`
- `style: apply cargo fmt and clippy suggestions`
- `fix: correct test assertions for macro parsing`
- `fix: resolve multiple CI pipeline issues`

The commit will only be created if files were actually modified during the CI fix process.
