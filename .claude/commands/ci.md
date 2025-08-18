---
name: ci
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

The CI pipeline includes:
- Running all tests
- Checking code formatting
- Running clippy linter
- Building in release mode
- Checking documentation