## Stage 1: Restore Analyzer Signal
**Goal**: Ensure Python analysis preserves extracted dependencies, classes, and debt signals instead of returning empty placeholders.
**Success Criteria**: Python `FileMetrics` includes extracted dependencies/classes/debt items; extraction failures surface as explicit analysis debt instead of silent empty output.
**Tests**: Adapter unit tests for Python conversion; analyzer tests covering preserved metadata and failure signaling.
**Status**: Complete

## Stage 2: Remove Dead Toggles
**Goal**: Make Rust enhanced detection and TypeScript functional analysis configuration flags materially affect analysis.
**Success Criteria**: Rust enhanced detection can be disabled through the analyzer path; TypeScript functional analysis populates additional metrics only when enabled.
**Tests**: Unit tests for Rust analyzer flag behavior and TypeScript functional-analysis behavior.
**Status**: Complete

## Stage 3: Replace Placeholder Module Analysis
**Goal**: Replace JS/TS and Python module-structure text scanning with parser-backed analysis.
**Success Criteria**: Module structure reports real function/class names and separates class methods from module-level functions for Python and JS/TS.
**Tests**: Module-structure tests for Python and JS/TS files with classes, methods, and exported/top-level functions.
**Status**: Complete

## Stage 4: Verify End-to-End
**Goal**: Run targeted tests for the touched analyzer paths and update plan status.
**Success Criteria**: Relevant cargo tests pass and the plan reflects completed stages.
**Tests**: Targeted `cargo test` filters for Python analyzer/adapters, TypeScript analyzer/module structure, and Rust analyzer behavior.
**Status**: Complete
